//! Private filesystem helpers for the federation slice.
//!
//! The discipline is the one the episode store and the quorum shard already
//! use: a directory the peer owns, files that are never symlinks, reads that
//! are bounded, and sidecars that are replaced atomically.
//!
//! The atomic replacement writes a temporary file in the same directory,
//! synchronizes it, renames it over the target, and synchronizes the
//! directory entry. A reader therefore sees the old file or the new one,
//! never a half-written mixture. The checksum over a sidecar body is a
//! corruption detector, not a signature: it proves the bytes are the bytes
//! this peer wrote, and nothing about who wrote them.
//!
//! These helpers are path-based checks, not descriptor-relative protection
//! against a concurrent actor replacing a checked path. They are not
//! power-loss evidence.

use crate::v3::federation::error::FederationError;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Upper bound on one federation file.
pub(crate) const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Prepare a directory the peer owns.
///
/// A directory that is missing is created; a path that is a symlink or a
/// regular file is refused.
pub(crate) fn prepare_directory(path: &Path) -> Result<PathBuf, FederationError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(FederationError::Io);
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(FederationError::Io),
    }
    fs::create_dir_all(path).map_err(|_| FederationError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| FederationError::Io)?;
    }
    fs::canonicalize(path).map_err(|_| FederationError::Io)
}

/// Refuse a path that is a symlink or is not a regular file.
pub(crate) fn reject_symlink_if_present(path: &Path) -> Result<(), FederationError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(FederationError::Io)
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(FederationError::Io),
    }
}

/// Read a file whole, bounded, when it exists.
pub(crate) fn read_bounded_optional(path: &Path) -> Result<Option<Vec<u8>>, FederationError> {
    reject_symlink_if_present(path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > MAX_FILE_BYTES
            {
                return Err(FederationError::Io);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(FederationError::Io),
    }
    let mut raw = Vec::new();
    File::open(path)
        .map_err(|_| FederationError::Io)?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|_| FederationError::Io)?;
    if u64::try_from(raw.len()).map_err(|_| FederationError::Io)? > MAX_FILE_BYTES {
        return Err(FederationError::Io);
    }
    Ok(Some(raw))
}

/// Replace a file atomically.
///
/// The bytes are written to a temporary file in the same directory, the
/// temporary file is synchronized, renamed over the target, and the
/// directory entry is synchronized.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), FederationError> {
    let parent = path.parent().ok_or(FederationError::Io)?;
    let name = path.file_name().ok_or(FederationError::Io)?;
    let mut temporary = parent.to_path_buf();
    temporary.push(format!(".{}.tmp", name.to_string_lossy()));
    {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temporary)
            .map_err(|_| FederationError::Io)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))
                .map_err(|_| FederationError::Io)?;
        }
        file.write_all(bytes).map_err(|_| FederationError::Io)?;
        file.sync_all().map_err(|_| FederationError::Io)?;
    }
    fs::rename(&temporary, path).map_err(|_| FederationError::Io)?;
    sync_directory(parent)
}

/// Synchronize a directory entry on Unix; validate only elsewhere.
pub(crate) fn sync_directory(path: &Path) -> Result<(), FederationError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| FederationError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(FederationError::Io);
    }
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| FederationError::Io)
    }
    #[cfg(not(unix))]
    {
        Ok(())
    }
}

/// SHA-256 over the bytes of a sidecar body.
pub(crate) fn checksum(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

/// An append-only file of JSON lines.
///
/// Every line is synchronized before the append returns, so a line that the
/// peer has seen is durable. A crash may leave an incomplete final line,
/// which the reader discards exactly as the episode ledger does.
pub(crate) struct Appender {
    file: File,
}

impl Appender {
    /// Open (and create) an append-only file.
    pub(crate) fn open(path: &Path) -> Result<Self, FederationError> {
        reject_symlink_if_present(path)?;
        let mut options = OpenOptions::new();
        options.create(true).read(true).append(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(path).map_err(|_| FederationError::Io)?;
        Ok(Self { file })
    }

    /// Append one line and synchronize it.
    pub(crate) fn append_line(&mut self, line: &[u8]) -> Result<(), FederationError> {
        self.file.write_all(line).map_err(|_| FederationError::Io)?;
        self.file
            .write_all(b"\n")
            .map_err(|_| FederationError::Io)?;
        self.file.sync_data().map_err(|_| FederationError::Io)
    }
}

/// The complete lines of an append-only file.
///
/// An incomplete final line is discarded: the writer may have been
/// interrupted between the bytes and the newline.
pub(crate) fn complete_lines(raw: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0_usize;
    while let Some(relative_end) = raw[start..].iter().position(|byte| *byte == b'\n') {
        let end = start + relative_end;
        lines.push(&raw[start..end]);
        start = end + 1;
    }
    lines
}
