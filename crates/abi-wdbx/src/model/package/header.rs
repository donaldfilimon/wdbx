//! Outer bounds before allocation or component access.
use super::{Result, require};
use crate::model::{
    contract::MAGIC,
    error::ErrorKind as E,
    limits::{LOADING_OVERHEAD_BYTES, LoadLimits, MAX_COMPONENTS, MIN_COMPONENTS},
};
pub(super) fn u16_at(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes(b[p..p + 2].try_into().expect("bounded fixed field"))
}
pub(super) fn u32_at(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(b[p..p + 4].try_into().expect("bounded fixed field"))
}
pub(super) fn u64_at(b: &[u8], p: usize) -> u64 {
    u64::from_le_bytes(b[p..p + 8].try_into().expect("bounded fixed field"))
}
pub(super) fn inspect(b: &[u8], limits: LoadLimits) -> Result<(usize, usize)> {
    require(b.len() >= 96, E::Truncated, "header")?;
    require(b[..8] == MAGIC, E::BadMagic, "magic")?;
    require(
        u16_at(b, 8) == 1 && u16_at(b, 10) == 0,
        E::UnsupportedContainerVersion,
        "container-version",
    )?;
    require(
        u32_at(b, 12) == 96 && u32_at(b, 28) == 0 && b[64..96].iter().all(|b| *b == 0),
        E::NonCanonical,
        "header",
    )?;
    let limits = limits.effective();
    let total = u64_at(b, 16);
    let count = u64::from(u32_at(b, 24));
    require(
        (96..=limits.artifact_bytes).contains(&total),
        E::CapacityExceeded,
        "artifact-bytes",
    )?;
    require(
        (MIN_COMPONENTS..=MAX_COMPONENTS).contains(&count),
        E::CapacityExceeded,
        "directory-count",
    )?;
    require(96 + count * 80 <= total, E::Truncated, "directory")?;
    require(
        total
            .checked_add(LOADING_OVERHEAD_BYTES)
            .is_some_and(|n| n <= limits.loading_bytes),
        E::CapacityExceeded,
        "snapshot-loading-budget",
    )?;
    Ok((
        usize::try_from(total).map_err(|_| super::err(E::CapacityExceeded, "platform-size"))?,
        usize::try_from(count).expect("bounded directory"),
    ))
}
