//! The binding index's writer exclusion.
//!
//! The index is derived state beside the ledger, and it is written by whole
//! file replacement. Two writers on one file would therefore lose each
//! other's bindings: the second writer's snapshot predates the first
//! writer's binding, and the replacement deletes it. The index takes the
//! same exclusion the episode store and the offline outbox take — an owner
//! file, and a lock held for the index's lifetime — so a second writer is
//! refused as [`VectorBindingError::WriterBusy`] instead of racing.
//!
//! The lock is advisory and it is not an adversarial fence: it excludes the
//! repository's own writers, not an actor who rewrites the file wholesale.
//! The merge on every write is what keeps the "no lost update" property true
//! even when the lock is bypassed.

use std::fs::{self, File, OpenOptions};
use std::path::Path;
use std::time::{Duration, Instant};

use super::VectorBindingError;

/// How long a writer waits for a lock a fork may still be holding.
const LOCK_RETRY_MILLIS: u64 = 50;

/// Create the file the index owns, with owner-only permissions.
///
/// A path that is a symlink, or that is not a regular file, is refused.
pub(super) fn owner_file(path: &Path) -> Result<File, VectorBindingError> {
    reject_symlink_if_present(path)?;
    let mut options = OpenOptions::new();
    options.create(true).read(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path).map_err(|_| VectorBindingError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| VectorBindingError::Io)?;
    }
    Ok(file)
}

/// Acquire the index's writer lock, tolerating a fork that briefly retains
/// an inherited descriptor.
///
/// The retry is bounded: transient contention is retried, and persistent
/// contention is [`VectorBindingError::WriterBusy`]. The lock is never
/// released while the index lives: the caller holds the file for the
/// index's lifetime.
pub(super) fn acquire_writer_lock(file: &File) -> Result<(), VectorBindingError> {
    let deadline = Instant::now() + Duration::from_millis(LOCK_RETRY_MILLIS);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err(VectorBindingError::WriterBusy);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(std::fs::TryLockError::Error(_)) => return Err(VectorBindingError::Io),
        }
    }
}

/// Refuse a path that is a symlink or is not a regular file.
fn reject_symlink_if_present(path: &Path) -> Result<(), VectorBindingError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                Err(VectorBindingError::Io)
            } else {
                Ok(())
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(VectorBindingError::Io),
    }
}
