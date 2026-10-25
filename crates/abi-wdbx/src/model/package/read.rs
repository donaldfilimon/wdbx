//! Capture once; source paths and mutable descriptors are not validation state.
use super::{Result, cancelled, err, header, require};
use crate::model::{PackageBytes, error::ErrorKind as E, limits::LoadLimits};
use std::io::Read;
/// Read exactly one bounded owned snapshot plus one EOF probe.
/// Interrupted reads fail explicitly; arbitrary blocking I/O deadlines belong to the caller.
pub fn read_bounded_snapshot(reader: impl Read, limits: LoadLimits) -> Result<PackageBytes> {
    read_bounded_snapshot_with_cancel(reader, limits, || false)
}
/// Cancellation is checked before every bounded read, including the EOF probe.
pub fn read_bounded_snapshot_with_cancel(
    mut reader: impl Read,
    limits: LoadLimits,
    mut cancel: impl FnMut() -> bool,
) -> Result<PackageBytes> {
    let mut header = [0; 96];
    read_into(&mut reader, &mut header, &mut cancel)?;
    let (total, _) = header::inspect(&header, limits)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(total)
        .map_err(|_| err(E::CapacityExceeded, "snapshot-allocation"))?;
    bytes.extend_from_slice(&header);
    bytes.resize(total, 0);
    read_into(&mut reader, &mut bytes[96..], &mut cancel)?;
    cancelled(&mut cancel)?;
    let mut probe = [0; 1];
    let n = reader
        .read(&mut probe)
        .map_err(|_| err(E::Io, "eof-probe"))?;
    require(n == 0, E::TrailingBytes, "artifact-trailing")?;
    Ok(PackageBytes { bytes })
}
fn read_into(
    reader: &mut impl Read,
    mut out: &mut [u8],
    cancel: &mut impl FnMut() -> bool,
) -> Result<()> {
    while !out.is_empty() {
        cancelled(cancel)?;
        let len = out.len().min(65536);
        let n = reader
            .read(&mut out[..len])
            .map_err(|_| err(E::Io, "snapshot-read"))?;
        require(n > 0, E::Truncated, "snapshot-read")?;
        out = &mut out[n..];
    }
    Ok(())
}
