//! One bounded output snapshot, then the same production validation boundary.
use super::{Result, cancelled, components::validate_model, err, require, sha, validate_framing};
use crate::model::{
    PackageBytes, ValidatedModelState,
    contract::{ModelComponents, Version},
    error::ErrorKind as E,
    limits::{LOADING_OVERHEAD_BYTES, LoadLimits},
};
use std::io::Write;
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
    let count = 8 + annotations.len();
    let directory_end = 96 + 80 * count;
    let mut e = Encoder {
        out: Vec::new(),
        limit: super::size(
            limits
                .artifact_bytes
                .min(limits.loading_bytes.saturating_sub(LOADING_OVERHEAD_BYTES)),
        )?,
    };
    e.zeroes(directory_end)?;
    let mut entries: Vec<(u32, Version, u32, usize, usize)> = Vec::new();
    entries
        .try_reserve_exact(count)
        .map_err(|_| err(E::CapacityExceeded, "writer-directory"))?;
    for kind in 1..=8 {
        let start = e.out.len();
        schema::component(kind, &components, &mut e)?;
        entries.push((
            kind,
            Version { major: 1, minor: 0 },
            1,
            start,
            e.out.len() - start,
        ));
    }
    for a in annotations {
        let start = e.out.len();
        e.bytes(&a.bytes)?;
        entries.push((a.kind, a.version, 0, start, e.out.len() - start));
    }
    drop(components);
    for (i, (kind, version, flags, offset, length)) in entries.into_iter().enumerate() {
        let digest = sha(&e.out[offset..offset + length]);
        let entry = &mut e.out[96 + i * 80..96 + (i + 1) * 80];
        entry[..4].copy_from_slice(&kind.to_le_bytes());
        entry[4..6].copy_from_slice(&version.major.to_le_bytes());
        entry[6..8].copy_from_slice(&version.minor.to_le_bytes());
        entry[8..12].copy_from_slice(&flags.to_le_bytes());
        entry[16..24].copy_from_slice(&(offset as u64).to_le_bytes());
        entry[24..32].copy_from_slice(&(length as u64).to_le_bytes());
        entry[32..64].copy_from_slice(&digest);
    }
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
    validate_model(
        validate_framing(PackageBytes { bytes: e.out }, limits)?,
        limits,
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
}
impl Encoder {
    fn reserve(&mut self, n: usize) -> Result<()> {
        let end = self
            .out
            .len()
            .checked_add(n)
            .ok_or_else(|| err(E::CapacityExceeded, "writer-size"))?;
        require(end <= self.limit, E::CapacityExceeded, "writer-size")?;
        if end > self.out.capacity() {
            let capacity = end.saturating_add(65535) / 65536 * 65536;
            let target = capacity.min(self.limit).max(end);
            self.out
                .try_reserve_exact(target - self.out.len())
                .map_err(|_| err(E::CapacityExceeded, "writer-allocation"))?;
        }
        Ok(())
    }
    fn bytes(&mut self, b: &[u8]) -> Result<()> {
        self.reserve(b.len())?;
        self.out.extend_from_slice(b);
        Ok(())
    }
    fn zeroes(&mut self, n: usize) -> Result<()> {
        self.reserve(n)?;
        self.out.resize(self.out.len() + n, 0);
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
