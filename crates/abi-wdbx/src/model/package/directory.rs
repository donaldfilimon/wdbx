//! Canonical directory and component integrity on the captured bytes only.
use super::{
    Result, err,
    header::{self, u16_at, u32_at, u64_at},
    require, sha,
};
use crate::model::{
    FramedPackage, PackageBytes,
    contract::{DirectoryEntry, PACKAGE_DOMAIN, Version},
    error::ErrorKind as E,
    ids::{ArtifactSha256, Digest, PackageDigest},
    limits::LoadLimits,
};
use sha2::{Digest as _, Sha256};
/// Framing validity grants neither semantic validity nor resume eligibility.
pub fn validate_framing(snapshot: PackageBytes, limits: LoadLimits) -> Result<FramedPackage> {
    let raw = snapshot.as_bytes();
    let (total, count) = header::inspect(raw, limits)?;
    require(raw.len() >= total, E::Truncated, "artifact")?;
    require(raw.len() == total, E::TrailingBytes, "artifact")?;
    let dir_end = 96 + count * 80;
    require(
        sha(&raw[96..dir_end]) == raw[32..64],
        E::IntegrityMismatch,
        "directory",
    )?;
    let mut directory = Vec::new();
    directory
        .try_reserve_exact(count)
        .map_err(|_| err(E::CapacityExceeded, "directory-allocation"))?;
    let mut end = dir_end as u64;
    let mut previous = 0;
    let mut present = 0u16;
    for entry in raw[96..dir_end].as_chunks::<80>().0 {
        let kind = u32_at(entry, 0);
        let flags = u32_at(entry, 8);
        let version = Version {
            major: u16_at(entry, 4),
            minor: u16_at(entry, 6),
        };
        require(
            kind > previous
                && u32_at(entry, 12) == 0
                && entry[64..].iter().all(|b| *b == 0)
                && flags <= 1,
            E::NonCanonical,
            "directory-entry",
        )?;
        if kind <= 8 {
            require(flags == 1, E::MissingComponent, "required-flag")?;
            require(
                version.major == 1 && version.minor == 0,
                E::UnsupportedComponentVersion,
                "component-version",
            )?;
            present |= 1 << kind;
        } else {
            require(
                flags == 0,
                E::UnsupportedRequiredComponent,
                "component-kind",
            )?;
        }
        let offset = u64_at(entry, 16);
        let length = u64_at(entry, 24);
        require(
            length > 0 && offset == end && length <= total as u64 - end,
            E::NonCanonical,
            "component-range",
        )?;
        end = end
            .checked_add(length)
            .ok_or_else(|| err(E::CapacityExceeded, "component-end"))?;
        directory.push(DirectoryEntry {
            kind,
            version,
            flags,
            reserved: 0,
            offset,
            length,
            component_sha256: Digest(entry[32..64].try_into().expect("fixed digest")),
            reserved_tail: [0; 16],
        });
        previous = kind;
    }
    require(present == 0x1fe, E::MissingComponent, "required-kinds")?;
    require(end == total as u64, E::NonCanonical, "payload-end")?;
    for e in &directory {
        let start = super::size(e.offset)?;
        let end = super::size(e.offset + e.length)?;
        require(
            sha(&raw[start..end]) == e.component_sha256.0,
            E::IntegrityMismatch,
            "component-hash",
        )
        .map_err(|mut error| {
            error.component = Some(e.kind);
            error.offset = Some(e.offset);
            error
        })?;
    }
    let artifact_sha256 = ArtifactSha256(sha(raw));
    let mut hash = Sha256::new();
    hash.update(PACKAGE_DOMAIN);
    hash.update(raw);
    let package_digest = PackageDigest(hash.finalize().into());
    Ok(FramedPackage {
        snapshot,
        directory,
        artifact_sha256,
        package_digest,
    })
}
