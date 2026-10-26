//! Private-path admission and the exclusive typed writer lock.
//!
//! Every path is absolute, never a symlink, and owner-only: directories carry no
//! group/other permission bits, and configuration, key and request files are
//! owner-only regular files read under a size bound.

use crate::error::TypedError;
use std::fs::{File, Metadata, OpenOptions, TryLockError};
use std::io::Read as _;
use std::path::Path;
use std::time::{Duration, Instant};

const LOCK_FILE: &str = "typed.lock";
const LOCK_WAIT: Duration = Duration::from_secs(2);

pub(crate) fn absolute(path: &str) -> Result<&Path, TypedError> {
    let path = Path::new(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(TypedError::new("UnsafePath", "paths must be absolute"))
    }
}

#[cfg(unix)]
#[allow(
    clippy::verbose_bit_mask,
    reason = "a group/other permission mask reads clearer than trailing_zeros"
)]
fn owner_only(metadata: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    metadata.permissions().mode() & 0o077 == 0
}

#[cfg(not(unix))]
fn owner_only(_: &Metadata) -> bool {
    // Owner-only admission is defined only for Unix permission bits.
    false
}

fn metadata(path: &Path) -> Result<Metadata, TypedError> {
    std::fs::symlink_metadata(path)
        .map_err(|error| TypedError::new("UnsafePath", format!("{}: {error}", path.display())))
}

pub(crate) fn private_directory(path: &Path) -> Result<(), TypedError> {
    let found = metadata(path)?;
    if found.file_type().is_symlink() || !found.is_dir() || !owner_only(&found) {
        return Err(TypedError::new(
            "UnsafePath",
            format!("{} must be an owner-only directory", path.display()),
        ));
    }
    Ok(())
}

pub(crate) fn read_private_file(path: &Path, limit: u64) -> Result<Vec<u8>, TypedError> {
    let found = metadata(path)?;
    if found.file_type().is_symlink() || !found.is_file() || !owner_only(&found) {
        return Err(TypedError::new(
            "UnsafePath",
            format!("{} must be an owner-only regular file", path.display()),
        ));
    }
    let file = File::open(path)
        .map_err(|error| TypedError::new("UnsafePath", format!("{}: {error}", path.display())))?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| TypedError::new("UnsafePath", format!("{}: {error}", path.display())))?;
    if bytes.len() as u64 > limit {
        return Err(TypedError::new(
            "InputTooLarge",
            format!("{} exceeds {limit} bytes", path.display()),
        ));
    }
    Ok(bytes)
}

/// Held for the whole read-check-commit window; released when dropped.
pub(crate) struct TypedLock {
    _file: File,
}

pub(crate) fn acquire_lock(root: &Path) -> Result<TypedLock, TypedError> {
    let path = root.join(LOCK_FILE);
    if let Ok(found) = std::fs::symlink_metadata(&path)
        && (found.file_type().is_symlink() || !found.is_file())
    {
        return Err(TypedError::new(
            "UnsafePath",
            "typed lock is not a regular file",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let file = options
        .open(&path)
        .map_err(|error| TypedError::new("StoreUnavailable", format!("typed lock: {error}")))?;
    let deadline = Instant::now() + LOCK_WAIT;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(TypedLock { _file: file }),
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(TryLockError::WouldBlock) => return Err(TypedError::writer_busy()),
            Err(TryLockError::Error(error)) => {
                return Err(TypedError::new(
                    "StoreUnavailable",
                    format!("typed lock: {error}"),
                ));
            }
        }
    }
}
