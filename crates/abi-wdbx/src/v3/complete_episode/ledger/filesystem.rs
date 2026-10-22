//! Private filesystem preparation for the complete-episode ledger.
//!
//! These are path-based checks, not descriptor-relative protection against a
//! concurrent actor replacing a checked path. They mirror the v3 episode
//! ledger's own filesystem discipline: a store directory that is not a
//! directory is refused, a symlink in place of the ledger or the lock is
//! refused, the ledger is read whole under a byte ceiling, and the writer
//! lock is held for the store's lifetime.
//!
//! They do not establish power-loss evidence.

use super::CompleteEpisodeStoreError;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

/// Tolerate fork/exec-inherited descriptors without bypassing writer exclusion.
pub(super) fn acquire_writer_lock(file: &File) -> Result<(), CompleteEpisodeStoreError> {
    let deadline = Instant::now() + Duration::from_millis(50);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(TryLockError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err(CompleteEpisodeStoreError::WriterBusy);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(TryLockError::Error(_)) => return Err(CompleteEpisodeStoreError::Io),
        }
    }
}

/// Prepare the store directory, creating it when it is missing.
pub(super) fn prepare_directory(path: &Path) -> Result<PathBuf, CompleteEpisodeStoreError> {
    // Ancestor directory aliases (including macOS /var) remain valid. Only
    // the requested store itself must not be a symlink.
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(CompleteEpisodeStoreError::Io);
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => return Err(CompleteEpisodeStoreError::Io),
    }
    let absolute = std::path::absolute(path).map_err(|_| CompleteEpisodeStoreError::Io)?;
    let mut ancestor = absolute.as_path();
    let mut missing = Vec::new();
    loop {
        match fs::metadata(ancestor) {
            Ok(metadata) if metadata.is_dir() => break,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let component = ancestor
                    .components()
                    .next_back()
                    .ok_or(CompleteEpisodeStoreError::Io)?;
                if !matches!(component, Component::Normal(_) | Component::ParentDir) {
                    return Err(CompleteEpisodeStoreError::Io);
                }
                missing.push(component.as_os_str().to_os_string());
                ancestor = ancestor.parent().ok_or(CompleteEpisodeStoreError::Io)?;
            }
            Ok(_) | Err(_) => return Err(CompleteEpisodeStoreError::Io),
        }
    }
    let mut current = ancestor
        .canonicalize()
        .map_err(|_| CompleteEpisodeStoreError::Io)?;
    if let Some(parent) = current.parent() {
        sync_directory(parent)?;
    }
    for component in missing.into_iter().rev() {
        if component == ".." {
            current = current
                .join(component)
                .canonicalize()
                .map_err(|_| CompleteEpisodeStoreError::Io)?;
            continue;
        }
        let child = current.join(component);
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&child) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let metadata =
                    fs::symlink_metadata(&child).map_err(|_| CompleteEpisodeStoreError::Io)?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(CompleteEpisodeStoreError::Io);
                }
            }
            Err(_) => return Err(CompleteEpisodeStoreError::Io),
        }
        sync_directory(&current)?;
        current = child;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&current, fs::Permissions::from_mode(0o700))
            .map_err(|_| CompleteEpisodeStoreError::Io)?;
    }
    Ok(current)
}

/// Fence the ledger's directory entry on Unix; validate only elsewhere.
pub(super) fn sync_directory(path: &Path) -> Result<(), CompleteEpisodeStoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| CompleteEpisodeStoreError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(CompleteEpisodeStoreError::Io);
    }
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| CompleteEpisodeStoreError::Io)
    }
    #[cfg(not(unix))]
    {
        Ok(())
    }
}

/// Refuse a symlink, or anything that is not a regular file, in place of a
/// store file.
pub(super) fn reject_symlink_if_present(path: &Path) -> Result<(), CompleteEpisodeStoreError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(CompleteEpisodeStoreError::Io)
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(CompleteEpisodeStoreError::Io),
    }
}

/// Open a store file for append, creating it when it is missing.
pub(super) fn owner_file(path: &Path) -> Result<File, CompleteEpisodeStoreError> {
    reject_symlink_if_present(path)?;
    let mut options = OpenOptions::new();
    options.create(true).read(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(path)
        .map_err(|_| CompleteEpisodeStoreError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| CompleteEpisodeStoreError::Io)?;
    }
    Ok(file)
}

/// Read a store file whole, bounded, creating it when it does not exist.
pub(super) fn read_bounded_file(
    path: &Path,
    limit: u64,
) -> Result<Vec<u8>, CompleteEpisodeStoreError> {
    reject_symlink_if_present(path)?;
    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            owner_file(path)?
                .sync_data()
                .map_err(|_| CompleteEpisodeStoreError::Io)?;
        }
        Err(_) => return Err(CompleteEpisodeStoreError::Io),
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| CompleteEpisodeStoreError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CompleteEpisodeStoreError::Io);
    }
    if metadata.len() > limit {
        return Err(CompleteEpisodeStoreError::LimitExceeded);
    }
    // Keep the allocation bounded even if another actor grows the file after
    // the metadata check. The writer lock is not an adversarial security fence.
    let mut raw = Vec::new();
    File::open(path)
        .map_err(|_| CompleteEpisodeStoreError::Io)?
        .take(limit + 1)
        .read_to_end(&mut raw)
        .map_err(|_| CompleteEpisodeStoreError::Io)?;
    if u64::try_from(raw.len()).map_err(|_| CompleteEpisodeStoreError::Io)? > limit {
        return Err(CompleteEpisodeStoreError::LimitExceeded);
    }
    Ok(raw)
}
