//! Durable sidecar files for the shard.
//!
//! The shard's own state is a *small* file beside its voters, not a second
//! ledger. It is content-free (a term, a membership, and the digests the
//! shard acknowledged), it carries a SHA-256 checksum over its own body, and
//! it is replaced atomically: a temporary file is written and synchronized,
//! renamed over the target, and the directory entry is synchronized in turn.
//!
//! A crash therefore leaves either the previous state or the next one, never
//! a half-written mixture. The checksum is a corruption detector, not a
//! signature: it proves the bytes are the bytes the shard wrote, and nothing
//! about who wrote them.

use crate::v3::replication::error::ShardError;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Upper bound on one sidecar file.
pub(super) const MAX_SIDECAR_BYTES: u64 = 16 * 1024 * 1024;

/// Prepare a directory the shard owns.
///
/// A directory that is missing is created; a path that is a symlink or a
/// regular file is refused, exactly as the episode store refuses one.
pub(super) fn prepare_directory(path: &Path) -> Result<PathBuf, ShardError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(ShardError::Io);
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(ShardError::Io),
    }
    fs::create_dir_all(path).map_err(|_| ShardError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|_| ShardError::Io)?;
    }
    fs::canonicalize(path).map_err(|_| ShardError::Io)
}

/// Read a sidecar file whole, bounded, when it exists.
///
/// A missing file is reported as absent; a symlink, a non-regular file, or a
/// file past the bound is refused.
pub(super) fn read_bounded_optional(path: &Path) -> Result<Option<Vec<u8>>, ShardError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > MAX_SIDECAR_BYTES
            {
                return Err(ShardError::Io);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(ShardError::Io),
    }
    let mut raw = Vec::new();
    File::open(path)
        .map_err(|_| ShardError::Io)?
        .take(MAX_SIDECAR_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|_| ShardError::Io)?;
    if u64::try_from(raw.len()).map_err(|_| ShardError::Io)? > MAX_SIDECAR_BYTES {
        return Err(ShardError::Io);
    }
    Ok(Some(raw))
}

/// Replace a sidecar file atomically.
///
/// The bytes are written to a temporary file in the same directory, the
/// temporary file is synchronized, renamed over the target, and the
/// directory entry is synchronized. A reader therefore sees the old file or
/// the new one.
pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), ShardError> {
    let parent = path.parent().ok_or(ShardError::Io)?;
    let name = path.file_name().ok_or(ShardError::Io)?;
    let mut temporary = parent.to_path_buf();
    temporary.push(format!(".{}.tmp", name.to_string_lossy()));
    {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temporary)
            .map_err(|_| ShardError::Io)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))
                .map_err(|_| ShardError::Io)?;
        }
        file.write_all(bytes).map_err(|_| ShardError::Io)?;
        file.sync_all().map_err(|_| ShardError::Io)?;
    }
    fs::rename(&temporary, path).map_err(|_| ShardError::Io)?;
    sync_directory(parent)
}

/// Synchronize a directory entry on Unix; validate only elsewhere.
///
/// Portable Rust cannot synchronize a directory handle on every platform,
/// so the other platforms keep file-only durability, exactly as the episode
/// store does.
pub(super) fn sync_directory(path: &Path) -> Result<(), ShardError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ShardError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ShardError::Io);
    }
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| ShardError::Io)
    }
    #[cfg(not(unix))]
    {
        Ok(())
    }
}

/// SHA-256 over the bytes of a sidecar body.
pub(super) fn checksum(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}
