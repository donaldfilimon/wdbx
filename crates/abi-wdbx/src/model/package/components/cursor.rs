//! Checked flat decoding and cumulative allocation/metadata accounting.
use super::{Result, err, require};
use crate::model::{
    error::{ErrorKind as E, ModelError},
    ids::Digest,
    limits::{LOADING_OVERHEAD_BYTES, LoadLimits},
};
pub(super) struct Budget {
    live: u64,
    metadata: u64,
    limits: LoadLimits,
}
impl Budget {
    pub fn new(snapshot: u64, limits: LoadLimits) -> Result<Self> {
        require(
            snapshot <= limits.artifact_bytes,
            E::CapacityExceeded,
            "artifact-bytes",
        )?;
        let mut b = Self {
            live: 0,
            metadata: 0,
            limits,
        };
        b.allocate(
            snapshot
                .checked_add(LOADING_OVERHEAD_BYTES)
                .ok_or_else(|| err(E::CapacityExceeded, "loading-overhead"))?,
        )?;
        Ok(b)
    }
    pub fn allocate(&mut self, n: u64) -> Result<()> {
        self.live = self
            .live
            .checked_add(n)
            .ok_or_else(|| err(E::CapacityExceeded, "allocation-sum"))?;
        require(
            self.live <= self.limits.loading_bytes,
            E::CapacityExceeded,
            "loading-budget",
        )
    }
    pub fn metadata(&mut self, n: u64) -> Result<()> {
        self.metadata = self
            .metadata
            .checked_add(n)
            .ok_or_else(|| err(E::CapacityExceeded, "metadata-sum"))?;
        require(
            self.metadata <= self.limits.metadata_bytes,
            E::CapacityExceeded,
            "metadata-budget",
        )
    }
}
pub(super) struct Cursor<'a, 'b> {
    raw: &'a [u8],
    pos: usize,
    kind: u32,
    base: u64,
    budget: &'b mut Budget,
}
impl<'a, 'b> Cursor<'a, 'b> {
    pub fn new(raw: &'a [u8], kind: u32, base: u64, budget: &'b mut Budget) -> Self {
        Self {
            raw,
            pos: 0,
            kind,
            base,
            budget,
        }
    }
    pub fn take(&mut self, n: usize, numeric: bool) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| self.error(E::CapacityExceeded, "field-end"))?;
        require(end <= self.raw.len(), E::Truncated, "component-field")
            .map_err(|_| self.error(E::Truncated, "component-field"))?;
        if !numeric {
            self.budget.metadata(n as u64)?;
        }
        let bytes = &self.raw[self.pos..end];
        self.pos = end;
        Ok(bytes)
    }
    pub fn error(&self, kind: E, field: &'static str) -> ModelError {
        ModelError::new(
            kind,
            field,
            Some(self.kind),
            Some(self.base + self.pos as u64),
        )
    }
    pub fn check(&self, ok: bool, kind: E, field: &'static str) -> Result<()> {
        if ok {
            Ok(())
        } else {
            Err(self.error(kind, field))
        }
    }
    pub fn end(&self) -> Result<()> {
        self.check(
            self.pos == self.raw.len(),
            E::NonCanonical,
            "component-trailing",
        )
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1, false)?[0])
    }
    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.take(2, false)?.try_into().expect("fixed field"),
        ))
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4, false)?.try_into().expect("fixed field"),
        ))
    }
    pub fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(
            self.take(8, false)?.try_into().expect("fixed field"),
        ))
    }
    pub fn f64(&mut self) -> Result<f64> {
        let v = f64::from_le_bytes(self.take(8, false)?.try_into().expect("fixed field"));
        self.check(v.is_finite(), E::NonCanonical, "finite-float")?;
        Ok(v)
    }
    pub fn e8<T: TryFrom<u8, Error = ModelError>>(&mut self) -> Result<T> {
        let v = self.u8()?;
        T::try_from(v).map_err(|_| self.error(E::NonCanonical, "enum"))
    }
    pub fn e16<T: TryFrom<u16, Error = ModelError>>(&mut self) -> Result<T> {
        let v = self.u16()?;
        T::try_from(v).map_err(|_| self.error(E::NonCanonical, "enum"))
    }
    pub fn count(&mut self, max: u32) -> Result<usize> {
        let n = self.u32()?;
        self.check(n <= max, E::CapacityExceeded, "array-count")?;
        usize::try_from(n).map_err(|_| self.error(E::CapacityExceeded, "platform-count"))
    }
    pub fn vec<T>(&mut self, n: usize) -> Result<Vec<T>> {
        let bytes = n
            .checked_mul(std::mem::size_of::<T>())
            .and_then(|n| n.checked_add(32))
            .ok_or_else(|| self.error(E::CapacityExceeded, "allocation-product"))?;
        self.budget.allocate(bytes as u64)?;
        let mut v = Vec::new();
        v.try_reserve_exact(n)
            .map_err(|_| self.error(E::CapacityExceeded, "allocation"))?;
        Ok(v)
    }
    pub fn blob(&mut self, max: u32, vocab: bool) -> Result<Vec<u8>> {
        let n = self.count(max)?;
        let mut out = self.vec::<u8>(n)?;
        out.extend_from_slice(self.take(n, vocab)?);
        Ok(out)
    }
    pub fn string(&mut self, max: u32) -> Result<String> {
        String::from_utf8(self.blob(max, false)?).map_err(|_| self.error(E::NonCanonical, "utf8"))
    }
    pub fn fixed<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.take(N, false)?.try_into().expect("fixed field"))
    }
    pub fn nonzero<const N: usize>(&mut self) -> Result<[u8; N]> {
        let b = self.fixed()?;
        self.check(
            b.iter().any(|b| *b != 0),
            E::InvalidProvenance,
            "nonzero-reference",
        )?;
        Ok(b)
    }
    pub fn digest(&mut self) -> Result<Digest> {
        Ok(Digest(self.nonzero()?))
    }
    pub fn opt<const N: usize>(&mut self) -> Result<Option<[u8; N]>> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.nonzero()?)),
            _ => Err(self.error(E::NonCanonical, "option-tag")),
        }
    }
    pub fn tokenizer_cap(&self) -> Result<()> {
        self.check(
            self.raw.len() as u64 <= self.budget.limits.tokenizer_bytes,
            E::CapacityExceeded,
            "tokenizer-component",
        )
    }
}
