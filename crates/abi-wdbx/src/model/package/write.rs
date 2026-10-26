//! One bounded output snapshot, then the same production validation boundary.
use super::{Result, cancelled, components::validate_model, err, require, sha, validate_framing};
use crate::model::{
    PackageBytes, ValidatedModelState,
    contract::{ModelComponents, Version},
    error::ErrorKind as E,
    limits::LoadLimits,
};
use std::io::Write;
mod preflight;
mod schema;
/// Optional annotation is opaque and computationally inert.
#[derive(Debug)]
pub struct Annotation {
    /// Kind >=9, in strictly increasing order.
    pub kind: u32,
    /// Unknown optional versions are retained exactly.
    pub version: Version,
    /// Nonempty opaque payload, charged as metadata.
    pub bytes: Vec<u8>,
}
/// Encode raw records canonically, then validate all schemas before returning.
/// Consumes records so no duplicate decoded model survives validation.
pub fn build_package(
    components: ModelComponents,
    annotations: &[Annotation],
    limits: LoadLimits,
) -> Result<ValidatedModelState> {
    build_with_reservation(components, annotations, limits, |bytes| {
        let mut out = Vec::new();
        out.try_reserve_exact(bytes)
            .map_err(|_| err(E::CapacityExceeded, "writer-allocation"))?;
        Ok(out)
    })
}
// Keep fallible output reservation behind completed preflight. This boundary also
// allows a memory owner to provide an already reserved buffer without a copy.
fn build_with_reservation(
    components: ModelComponents,
    annotations: &[Annotation],
    limits: LoadLimits,
    reserve: impl FnOnce(usize) -> Result<Vec<u8>>,
) -> Result<ValidatedModelState> {
    let limits = limits.effective();
    require(
        annotations.len() <= 24,
        E::CapacityExceeded,
        "annotation-count",
    )?;
    require(
        annotations
            .iter()
            .all(|a| a.kind >= 9 && !a.bytes.is_empty())
            && annotations.windows(2).all(|w| w[0].kind < w[1].kind),
        E::NonCanonical,
        "annotation-order",
    )?;
    let plan = preflight::calculate(&components, annotations, limits)?;
    let count = 8 + annotations.len();
    let directory_end = 96 + 80 * count;
    let out = reserve(plan.output_len)?;
    require(
        out.is_empty(),
        E::NonCanonical,
        "writer-reserved-buffer-not-empty",
    )?;
    let extra = out
        .capacity()
        .checked_sub(plan.output_len)
        .ok_or_else(|| err(E::CapacityExceeded, "writer-allocation-capacity"))?;
    require(
        plan.peak
            .checked_add(extra as u64)
            .is_some_and(|peak| peak <= limits.loading_bytes),
        E::CapacityExceeded,
        "writer-peak-budget",
    )?;
    let mut e = Encoder {
        out,
        limit: plan.output_len,
        measured: None,
        metadata: 0,
        metadata_limit: limits.metadata_bytes,
    };
    e.zeroes(directory_end)?;
    for kind in 1..=8 {
        let start = e.out.len();
        schema::component(kind, &components, &mut e)?;
        e.entry(
            (kind - 1) as usize,
            kind,
            Version { major: 1, minor: 0 },
            1,
            start,
        )?;
    }
    for (i, a) in annotations.iter().enumerate() {
        let start = e.out.len();
        e.bytes(&a.bytes)?;
        e.entry(i + 8, a.kind, a.version, 0, start)?;
    }
    drop(components);
    require(
        e.out.len() == plan.output_len,
        E::NonCanonical,
        "writer-measured-size",
    )?;
    let digest = sha(&e.out[96..directory_end]);
    let total = e.out.len() as u64;
    let header = &mut e.out[..96];
    header[..8].copy_from_slice(b"WDBXMOD1");
    header[8..10].copy_from_slice(&1u16.to_le_bytes());
    header[12..16].copy_from_slice(&96u32.to_le_bytes());
    header[16..24].copy_from_slice(&total.to_le_bytes());
    header[24..28].copy_from_slice(
        &u32::try_from(count)
            .expect("bounded component count")
            .to_le_bytes(),
    );
    header[32..64].copy_from_slice(&digest);
    // Borrowed annotation buffers remain live during independent decoding.
    let mut decoder_limits = limits;
    decoder_limits.loading_bytes = limits
        .loading_bytes
        .checked_sub(plan.borrowed_annotations)
        .ok_or_else(|| err(E::CapacityExceeded, "writer-peak-budget"))?;
    validate_model(
        validate_framing(PackageBytes { bytes: e.out }, decoder_limits)?,
        decoder_limits,
    )
}
/// Re-export exactly the captured artifact, preserving signed zeros and annotations.
/// I/O is synchronous; caller supplies deadlines for blocking writer implementations.
pub fn export_exact(
    model: &ValidatedModelState,
    mut writer: impl Write,
    mut cancel: impl FnMut() -> bool,
) -> Result<()> {
    for mut bytes in model.framed.snapshot.as_bytes().chunks(65536) {
        while !bytes.is_empty() {
            cancelled(&mut cancel)?;
            let n = writer
                .write(bytes)
                .map_err(|_| err(E::Io, "export-write"))?;
            require(n > 0, E::Io, "export-write-zero")?;
            bytes = &bytes[n..];
        }
    }
    cancelled(&mut cancel)?;
    writer.flush().map_err(|_| err(E::Io, "export-flush"))
}
pub(super) struct Encoder {
    out: Vec<u8>,
    limit: usize,
    measured: Option<usize>,
    metadata: u64,
    metadata_limit: u64,
}
impl Encoder {
    fn measure(limits: LoadLimits) -> Result<Self> {
        Ok(Self {
            out: Vec::new(),
            limit: super::size(limits.artifact_bytes)?,
            measured: Some(0),
            metadata: 0,
            metadata_limit: limits.metadata_bytes,
        })
    }
    fn len(&self) -> usize {
        self.measured.unwrap_or(self.out.len())
    }
    fn reserve(&self, n: usize) -> Result<()> {
        let end = self
            .len()
            .checked_add(n)
            .ok_or_else(|| err(E::CapacityExceeded, "writer-size"))?;
        require(end <= self.limit, E::CapacityExceeded, "writer-size")?;
        if self.measured.is_none() {
            require(
                end <= self.out.capacity(),
                E::CapacityExceeded,
                "writer-unplanned-growth",
            )?;
        }
        Ok(())
    }
    fn raw(&mut self, b: &[u8]) -> Result<()> {
        self.reserve(b.len())?;
        if let Some(n) = &mut self.measured {
            *n += b.len();
        } else {
            self.out.extend_from_slice(b);
        }
        Ok(())
    }
    fn bytes(&mut self, b: &[u8]) -> Result<()> {
        self.metadata = self
            .metadata
            .checked_add(b.len() as u64)
            .ok_or_else(|| err(E::CapacityExceeded, "writer-metadata-sum"))?;
        require(
            self.metadata <= self.metadata_limit,
            E::CapacityExceeded,
            "writer-metadata-budget",
        )?;
        self.raw(b)
    }
    fn numeric(&mut self, b: &[u8]) -> Result<()> {
        self.raw(b)
    }
    fn zeroes(&mut self, n: usize) -> Result<()> {
        self.reserve(n)?;
        if let Some(count) = &mut self.measured {
            *count += n;
        } else {
            self.out.resize(self.out.len() + n, 0);
        }
        Ok(())
    }
    fn entry(
        &mut self,
        index: usize,
        kind: u32,
        version: Version,
        flags: u32,
        offset: usize,
    ) -> Result<()> {
        let length = self
            .out
            .len()
            .checked_sub(offset)
            .ok_or_else(|| err(E::NonCanonical, "writer-offset"))?;
        let digest = sha(&self.out[offset..]);
        let entry = &mut self.out[96 + index * 80..96 + (index + 1) * 80];
        entry[..4].copy_from_slice(&kind.to_le_bytes());
        entry[4..6].copy_from_slice(&version.major.to_le_bytes());
        entry[6..8].copy_from_slice(&version.minor.to_le_bytes());
        entry[8..12].copy_from_slice(&flags.to_le_bytes());
        entry[16..24].copy_from_slice(&(offset as u64).to_le_bytes());
        entry[24..32].copy_from_slice(&(length as u64).to_le_bytes());
        entry[32..64].copy_from_slice(&digest);
        Ok(())
    }
    fn u8(&mut self, v: u8) -> Result<()> {
        self.bytes(&[v])
    }
    fn u16(&mut self, v: u16) -> Result<()> {
        self.bytes(&v.to_le_bytes())
    }
    fn u32(&mut self, v: u32) -> Result<()> {
        self.bytes(&v.to_le_bytes())
    }
    fn u64(&mut self, v: u64) -> Result<()> {
        self.bytes(&v.to_le_bytes())
    }
    fn f64(&mut self, v: f64) -> Result<()> {
        self.bytes(&v.to_le_bytes())
    }
    fn count(&mut self, n: usize) -> Result<()> {
        self.u32(u32::try_from(n).map_err(|_| err(E::CapacityExceeded, "writer-count"))?)
    }
    fn blob(&mut self, b: &[u8]) -> Result<()> {
        self.count(b.len())?;
        self.bytes(b)
    }
    fn vocabulary_blob(&mut self, b: &[u8]) -> Result<()> {
        self.count(b.len())?;
        self.numeric(b)
    }
    fn string(&mut self, s: &str) -> Result<()> {
        self.blob(s.as_bytes())
    }
    fn opt<const N: usize>(&mut self, b: Option<[u8; N]>) -> Result<()> {
        if let Some(b) = b {
            self.u8(1)?;
            self.bytes(&b)
        } else {
            self.u8(0)
        }
    }
}
#[cfg(test)]
mod tests;
