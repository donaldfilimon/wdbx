//! The outbox's durable file format.
//!
//! # The format
//!
//! The outbox lives in one file, `outbox.v1.jsonl`, beside its writer lock
//! `outbox.v1.lock`. It is a JSON Lines file: a **header line**, then one
//! line per operation, in enqueue order.
//!
//! ```text
//! {"header":{"format":"outbox.v1","count":2,"records_checksum":[32 bytes]}}
//! {"record":{...},"checksum":[32 bytes]}
//! {"record":{...},"checksum":[32 bytes]}
//! ```
//!
//! The header names the format, the number of operations the file holds, and
//! a checksum over the record set. The `record` member of an operation line
//! is the operation itself: its identifier, its dependency, its binding, its
//! kind tag, its proposal, its status, its reason, its recorded conflict, its
//! asset state, its enqueue stamp, and its sequence number. The `checksum`
//! member is SHA-256 over the record's own canonical JSON bytes.
//!
//! # The discipline
//!
//! The file follows the discipline the repository already uses for its
//! derived sidecars, with one difference that the whole-file replacement
//! makes necessary:
//!
//! - the checksum is over the line's own fields, so a corrupted or
//!   half-written line is *detected* rather than believed. It is a checksum,
//!   not a signature: it proves the bytes are the bytes the outbox wrote, and
//!   nothing about who wrote them;
//! - the header's count and record-set checksum make a truncation
//!   *detectable*: a file that holds fewer operations than its header names
//!   is corruption, and the outbox refuses to open rather than opening with
//!   fewer operations than were queued;
//! - a trailing fragment is corruption too, and is **not** discarded. Every
//!   write replaces the whole file atomically, so a torn tail cannot arise
//!   from the outbox's own code path: its presence is evidence of damage,
//!   not of a crash;
//! - a complete line that does not parse, or whose checksum does not match,
//!   is corruption: the outbox refuses to open rather than silently dropping
//!   an operation;
//! - every write replaces the whole file atomically — the bytes are written
//!   to a temporary file, synchronized, renamed over the target, and the
//!   directory entry is synchronized in turn. An interrupted write leaves
//!   the previous outbox intact and openable.
//!
//! # What the file does not hold
//!
//! The file holds proposals, never episodes. A proposed episode write is
//! stored as the write the canonical store would compute its own commitment
//! from, and it carries no canonical episode digest: the outbox does not
//! know the digest, and a digest it invented would be a second canonical
//! episode. The file holds no payload, no rendered episode, and no field
//! value — an edit commits to its value by digest.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::error::OutboxError;
use super::types::{OperationId, OutboxOperation};

/// The file name of the outbox beside its writer lock.
pub const OUTBOX_FILE: &str = "outbox.v1.jsonl";

/// The file name of the outbox's writer lock.
pub const OUTBOX_LOCK: &str = "outbox.v1.lock";

/// The greatest number of bytes the outbox file may occupy.
pub const MAX_OUTBOX_BYTES: u64 = 64 * 1024 * 1024;

/// The greatest number of bytes one line may occupy.
pub const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;

/// The format tag the outbox file's header carries.
pub(crate) const OUTBOX_FORMAT: &str = "outbox.v1";

/// The header of the outbox file.
///
/// The header is what makes a truncation detectable: it names how many
/// operations the file holds, and it carries a checksum over the record set.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct OutboxHeader {
    /// The format tag.
    format: String,
    /// The number of operation lines the file holds.
    count: u64,
    /// SHA-256 over the record checksums, in file order.
    records_checksum: [u8; 32],
}

/// The header line of the outbox file.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct HeaderLine {
    /// The header.
    header: OutboxHeader,
}

/// One committed outbox line.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct OutboxLine {
    /// The operation the line holds.
    record: OutboxOperation,
    /// SHA-256 over the record's own canonical JSON bytes.
    checksum: [u8; 32],
}

/// SHA-256 over one record's own canonical JSON bytes.
fn record_checksum(record: &OutboxOperation) -> Result<[u8; 32], OutboxError> {
    let bytes = serde_json::to_vec(record).map_err(|_| OutboxError::Corrupt)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hasher.finalize().into())
}

/// SHA-256 over the record checksums, in file order.
///
/// The record set's checksum covers the digests only: it never covers a
/// proposal, so the header stays content-free.
fn records_checksum(checksums: &[[u8; 32]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for checksum in checksums {
        hasher.update(checksum);
    }
    hasher.finalize().into()
}

/// Prepare the directory the outbox owns.
///
/// A missing directory is created; a path that is a symlink or a regular
/// file is refused, exactly as the episode store refuses one.
pub(crate) fn prepare_directory(path: &Path) -> Result<PathBuf, OutboxError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(OutboxError::Io);
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(OutboxError::Io),
    }
    fs::create_dir_all(path).map_err(|_| OutboxError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| OutboxError::Io)?;
    }
    fs::canonicalize(path).map_err(|_| OutboxError::Io)
}

/// Refuse a path that is a symlink or is not a regular file.
pub(crate) fn reject_symlink_if_present(path: &Path) -> Result<(), OutboxError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(OutboxError::Io)
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(OutboxError::Io),
    }
}

/// Create the file the outbox owns, with owner-only permissions.
pub(crate) fn owner_file(path: &Path) -> Result<File, OutboxError> {
    reject_symlink_if_present(path)?;
    let mut options = OpenOptions::new();
    options.create(true).read(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path).map_err(|_| OutboxError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| OutboxError::Io)?;
    }
    Ok(file)
}

/// Read the outbox file whole, bounded, when it exists.
pub(crate) fn read_bounded_optional(path: &Path) -> Result<Option<Vec<u8>>, OutboxError> {
    reject_symlink_if_present(path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > MAX_OUTBOX_BYTES
            {
                return Err(OutboxError::Io);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(OutboxError::Io),
    }
    let mut raw = Vec::new();
    File::open(path)
        .map_err(|_| OutboxError::Io)?
        .take(MAX_OUTBOX_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|_| OutboxError::Io)?;
    if u64::try_from(raw.len()).map_err(|_| OutboxError::Io)? > MAX_OUTBOX_BYTES {
        return Err(OutboxError::Io);
    }
    Ok(Some(raw))
}

/// Synchronize a directory entry, where the platform supports it.
pub(crate) fn sync_directory(path: &Path) -> Result<(), OutboxError> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| OutboxError::Io)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

/// Read the outbox's operations.
///
/// The header must name exactly the operations the file holds, and the record
/// set's checksum must match it. A file that was truncated, or that was
/// damaged in any other way, is corruption: the outbox refuses to open
/// rather than opening with fewer operations than were queued.
pub(crate) fn read(path: &Path) -> Result<Vec<OutboxOperation>, OutboxError> {
    let Some(raw) = read_bounded_optional(path)? else {
        return Ok(Vec::new());
    };
    decode(&raw)
}

/// Decode the outbox file's bytes.
///
/// The file is a header line followed by one line per operation. Every
/// deviation from that shape is corruption, and nothing is discarded: a
/// trailing fragment is damage, because the writer replaces the whole file
/// atomically and can never leave one behind.
fn decode(raw: &[u8]) -> Result<Vec<OutboxOperation>, OutboxError> {
    if raw.is_empty() || !raw.ends_with(b"\n") {
        return Err(OutboxError::Corrupt);
    }
    // The file ends with a newline, so the body below it splits into whole
    // lines with no trailing empty element.
    let body = &raw[..raw.len() - 1];
    let mut lines = body.split(|byte| *byte == b'\n');
    let header_line = lines.next().ok_or(OutboxError::Corrupt)?;
    if header_line.is_empty() || header_line.len() > MAX_LINE_BYTES {
        return Err(OutboxError::Corrupt);
    }
    let header: HeaderLine =
        serde_json::from_slice(header_line).map_err(|_| OutboxError::Corrupt)?;
    if header.header.format != OUTBOX_FORMAT {
        return Err(OutboxError::Corrupt);
    }
    let mut operations: Vec<OutboxOperation> = Vec::new();
    let mut identifiers: BTreeSet<OperationId> = BTreeSet::new();
    let mut checksums: Vec<[u8; 32]> = Vec::new();
    for line in lines {
        if line.is_empty() || line.len() > MAX_LINE_BYTES {
            return Err(OutboxError::Corrupt);
        }
        let parsed: OutboxLine = serde_json::from_slice(line).map_err(|_| OutboxError::Corrupt)?;
        if parsed.checksum != record_checksum(&parsed.record)? {
            return Err(OutboxError::Corrupt);
        }
        if parsed.record.kind != parsed.record.payload.kind() {
            return Err(OutboxError::Corrupt);
        }
        if !identifiers.insert(parsed.record.id.clone()) {
            return Err(OutboxError::Corrupt);
        }
        checksums.push(parsed.checksum);
        operations.push(parsed.record);
    }
    let count = usize::try_from(header.header.count).map_err(|_| OutboxError::Corrupt)?;
    if count != operations.len() {
        return Err(OutboxError::Corrupt);
    }
    if header.header.records_checksum != records_checksum(&checksums) {
        return Err(OutboxError::Corrupt);
    }
    Ok(operations)
}

/// Replace the outbox file atomically.
///
/// The bytes are written to a temporary file in the same directory, the
/// temporary file is synchronized, renamed over the target, and the
/// directory entry is synchronized. A reader therefore sees the old file or
/// the new one, never a half-written mixture.
pub(crate) fn write_atomic(path: &Path, operations: &[OutboxOperation]) -> Result<(), OutboxError> {
    let mut checksums: Vec<[u8; 32]> = Vec::with_capacity(operations.len());
    let mut body: Vec<u8> = Vec::new();
    for operation in operations {
        let checksum = record_checksum(operation)?;
        let line = OutboxLine {
            record: operation.clone(),
            checksum,
        };
        let mut bytes = serde_json::to_vec(&line).map_err(|_| OutboxError::Corrupt)?;
        bytes.push(b'\n');
        if bytes.len() > MAX_LINE_BYTES {
            return Err(OutboxError::InvalidInput);
        }
        body.extend_from_slice(&bytes);
        checksums.push(checksum);
    }
    let header = HeaderLine {
        header: OutboxHeader {
            format: OUTBOX_FORMAT.to_string(),
            count: u64::try_from(operations.len()).unwrap_or(u64::MAX),
            records_checksum: records_checksum(&checksums),
        },
    };
    let mut whole = serde_json::to_vec(&header).map_err(|_| OutboxError::Corrupt)?;
    whole.push(b'\n');
    if whole.len() > MAX_LINE_BYTES {
        return Err(OutboxError::InvalidInput);
    }
    whole.extend_from_slice(&body);
    if u64::try_from(whole.len()).map_err(|_| OutboxError::Io)? > MAX_OUTBOX_BYTES {
        return Err(OutboxError::InvalidInput);
    }
    let parent = path.parent().ok_or(OutboxError::Io)?;
    let name = path.file_name().ok_or(OutboxError::Io)?;
    let mut temporary = parent.to_path_buf();
    temporary.push(format!(".{}.tmp", name.to_string_lossy()));
    {
        let mut options = OpenOptions::new();
        options.create(true).write(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary).map_err(|_| OutboxError::Io)?;
        if file.write_all(&whole).is_err() || file.sync_all().is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(OutboxError::Io);
        }
    }
    if fs::rename(&temporary, path).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(OutboxError::Io);
    }
    sync_directory(parent)
}
