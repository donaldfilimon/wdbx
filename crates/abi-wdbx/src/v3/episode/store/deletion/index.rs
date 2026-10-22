//! The two derived indexes behind the destruction requests.
//!
//! The indexes are the store's own durable record of a governance decision:
//! one line per request, a checksum over the line's own fields, no episode
//! content, written by the same single writer that appends the ledger. The
//! format is the one the retention, hold, revocation, and quota indexes
//! already use, and the reasons for it are in the parent module's
//! documentation.

use super::super::super::types::{ActorKind, ActorRef};
use super::super::filesystem::{read_bounded_file_if_present, sync_directory};
use super::super::{EpisodeStoreError, MAX_LEDGER_BYTES};
use super::DerivedCopyKind;
use super::ObjectKeyRef;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The format marker of the redaction sidecar's header line.
const REDACTION_FORMAT: &str = "wdbx-redaction-index-v1";
/// The format marker of the object-key sidecar's header line.
const OBJECT_KEY_FORMAT: &str = "wdbx-object-key-index-v1";

/// One committed redaction request.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RedactionStamp {
    /// The guild the target belongs to.
    guild_ref: String,
    /// The forgotten candidate whose derived copies must go.
    digest: [u8; 32],
    /// The kind of derived copy to destroy.
    kind: DerivedCopyKind,
    /// The principal of the authority that issued the order.
    principal_id: String,
    /// The class of the authority that issued the order.
    authority_kind: ActorKind,
    /// SHA-256 over the line's own fields.
    checksum: [u8; 32],
}

/// One committed object-key destruction request.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ObjectKeyStamp {
    /// The guild the target belongs to.
    guild_ref: String,
    /// The forgotten candidate whose payload the key holds.
    digest: [u8; 32],
    /// The tenant that owns the object key.
    tenant: String,
    /// The opaque object-key identifier.
    key_id: String,
    /// The principal of the authority that issued the order.
    principal_id: String,
    /// The class of the authority that issued the order.
    authority_kind: ActorKind,
    /// SHA-256 over the line's own fields.
    checksum: [u8; 32],
}

/// The header line of a destruction sidecar.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SidecarHeader {
    /// The format marker.
    format: String,
    /// The number of request lines the file holds.
    records: u64,
    /// SHA-256 over the request lines' bytes.
    checksum: [u8; 32],
}

/// SHA-256 over the fields of one sidecar line.
///
/// The parts are length-prefixed so no two different field sequences can
/// produce the same input. The checksum is a checksum, not a signature: it
/// detects a corrupted or half-written line, exactly as the episode digest
/// detects a mutated ledger line. It proves nothing about who wrote it.
fn line_checksum(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(u64::try_from(part.len()).unwrap_or(u64::MAX).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize().into()
}

/// The redaction requests the index holds: guild, target, and kinds.
type RedactionRequests = BTreeMap<String, BTreeMap<[u8; 32], BTreeSet<DerivedCopyKind>>>;

/// The object-key requests the index holds: guild, key, and target.
type ObjectKeyRequests = BTreeMap<String, BTreeMap<(String, String), [u8; 32]>>;

/// The store's redaction index: the committed redaction requests.
///
/// The index is derived state beside the ledger. A store that never asked
/// for a redaction has no index file at all: the file is created with the
/// first request, not with the store.
#[derive(Debug)]
pub(in crate::v3::episode::store) struct RedactionIndex {
    path: PathBuf,
    /// The request lines, in the order the store recorded them.
    lines: Vec<RedactionStamp>,
    /// The kinds of copy each target's requests cover.
    requests: RedactionRequests,
}

impl RedactionIndex {
    /// Open the index, replaying its complete lines.
    ///
    /// A file that does not parse, whose header does not match its request
    /// lines, whose lines fail their own checksums, or that ends in a
    /// fragment is corruption: the store refuses to open rather than
    /// silently forgetting a destruction order.
    pub(in crate::v3::episode::store) fn open(path: &Path) -> Result<Self, EpisodeStoreError> {
        let Some(raw) = read_bounded_file_if_present(path)? else {
            return Ok(Self {
                path: path.to_path_buf(),
                lines: Vec::new(),
                requests: RedactionRequests::new(),
            });
        };
        let (lines, requests) = decode_redactions(&raw)?;
        Ok(Self {
            path: path.to_path_buf(),
            lines,
            requests,
        })
    }

    /// The kinds of derived copy a request covers for one target.
    pub(in crate::v3::episode::store) fn kinds(
        &self,
        guild_ref: &str,
        digest: &[u8; 32],
    ) -> BTreeSet<DerivedCopyKind> {
        self.requests
            .get(guild_ref)
            .and_then(|targets| targets.get(digest))
            .cloned()
            .unwrap_or_default()
    }

    /// Commit one redaction request.
    ///
    /// The line is written and synchronized before the receipt that reports
    /// it is returned, so a receipt never claims a request the store did not
    /// durably record.
    pub(in crate::v3::episode::store) fn commit(
        &mut self,
        guild_ref: &str,
        digest: [u8; 32],
        kind: DerivedCopyKind,
        actor: &ActorRef,
    ) -> Result<(), EpisodeStoreError> {
        let stamp = RedactionStamp {
            guild_ref: guild_ref.to_string(),
            digest,
            kind,
            principal_id: actor.principal_id.clone(),
            authority_kind: actor.kind,
            checksum: redaction_checksum(guild_ref, &digest, kind, actor),
        };
        write_sidecar(&self.path, REDACTION_FORMAT, &self.lines, &stamp)?;
        self.lines.push(stamp);
        self.requests
            .entry(guild_ref.to_string())
            .or_default()
            .entry(digest)
            .or_default()
            .insert(kind);
        Ok(())
    }
}

/// Read the index's request lines and the coverage they state.
fn decode_redactions(
    raw: &[u8],
) -> Result<(Vec<RedactionStamp>, RedactionRequests), EpisodeStoreError> {
    let mut lines = Vec::new();
    let mut requests = RedactionRequests::new();
    for line in sidecar_lines(raw, REDACTION_FORMAT)? {
        let stamp: RedactionStamp =
            serde_json::from_slice(line).map_err(|_| EpisodeStoreError::Corrupt)?;
        let actor = ActorRef {
            principal_id: stamp.principal_id.clone(),
            kind: stamp.authority_kind,
        };
        if stamp.checksum != redaction_checksum(&stamp.guild_ref, &stamp.digest, stamp.kind, &actor)
        {
            return Err(EpisodeStoreError::Corrupt);
        }
        requests
            .entry(stamp.guild_ref.clone())
            .or_default()
            .entry(stamp.digest)
            .or_default()
            .insert(stamp.kind);
        lines.push(stamp);
    }
    Ok((lines, requests))
}

/// The store's object-key index: the committed destruction requests.
#[derive(Debug)]
pub(in crate::v3::episode::store) struct ObjectKeyIndex {
    path: PathBuf,
    /// The request lines, in the order the store recorded them.
    lines: Vec<ObjectKeyStamp>,
    /// The target each key was bound to.
    requests: ObjectKeyRequests,
}

impl ObjectKeyIndex {
    /// Open the index, replaying its complete lines.
    pub(in crate::v3::episode::store) fn open(path: &Path) -> Result<Self, EpisodeStoreError> {
        let Some(raw) = read_bounded_file_if_present(path)? else {
            return Ok(Self {
                path: path.to_path_buf(),
                lines: Vec::new(),
                requests: ObjectKeyRequests::new(),
            });
        };
        let (lines, requests) = decode_object_keys(&raw)?;
        Ok(Self {
            path: path.to_path_buf(),
            lines,
            requests,
        })
    }

    /// The target a destruction request bound one key to.
    pub(in crate::v3::episode::store) fn target(
        &self,
        guild_ref: &str,
        key: &ObjectKeyRef,
    ) -> Option<[u8; 32]> {
        self.requests
            .get(guild_ref)
            .and_then(|keys| keys.get(&(key.tenant.clone(), key.key_id.clone())))
            .copied()
    }

    /// Commit one object-key destruction request.
    pub(in crate::v3::episode::store) fn commit(
        &mut self,
        guild_ref: &str,
        digest: [u8; 32],
        key: &ObjectKeyRef,
        actor: &ActorRef,
    ) -> Result<(), EpisodeStoreError> {
        let stamp = ObjectKeyStamp {
            guild_ref: guild_ref.to_string(),
            digest,
            tenant: key.tenant.clone(),
            key_id: key.key_id.clone(),
            principal_id: actor.principal_id.clone(),
            authority_kind: actor.kind,
            checksum: object_key_checksum(guild_ref, &digest, key, actor),
        };
        write_sidecar(&self.path, OBJECT_KEY_FORMAT, &self.lines, &stamp)?;
        self.lines.push(stamp);
        self.requests
            .entry(guild_ref.to_string())
            .or_default()
            .insert((key.tenant.clone(), key.key_id.clone()), digest);
        Ok(())
    }
}

/// Read the index's request lines and the bindings they state.
fn decode_object_keys(
    raw: &[u8],
) -> Result<(Vec<ObjectKeyStamp>, ObjectKeyRequests), EpisodeStoreError> {
    let mut lines = Vec::new();
    let mut requests = ObjectKeyRequests::new();
    for line in sidecar_lines(raw, OBJECT_KEY_FORMAT)? {
        let stamp: ObjectKeyStamp =
            serde_json::from_slice(line).map_err(|_| EpisodeStoreError::Corrupt)?;
        let actor = ActorRef {
            principal_id: stamp.principal_id.clone(),
            kind: stamp.authority_kind,
        };
        let key = ObjectKeyRef {
            tenant: stamp.tenant.clone(),
            key_id: stamp.key_id.clone(),
        };
        if stamp.checksum != object_key_checksum(&stamp.guild_ref, &stamp.digest, &key, &actor) {
            return Err(EpisodeStoreError::Corrupt);
        }
        requests
            .entry(stamp.guild_ref.clone())
            .or_default()
            .insert((stamp.tenant.clone(), stamp.key_id.clone()), stamp.digest);
        lines.push(stamp);
    }
    Ok((lines, requests))
}

/// The request lines of one sidecar, after its header has been checked.
///
/// The file is a header line and one line per request. A header whose count
/// or checksum does not match the request lines is a truncated file, and is
/// refused rather than believed; a trailing fragment is damage rather than a
/// crash tail, because the writer only ever writes whole lines.
fn sidecar_lines<'a>(raw: &'a [u8], format: &str) -> Result<Vec<&'a [u8]>, EpisodeStoreError> {
    let Some(end) = raw.iter().position(|byte| *byte == b'\n') else {
        return Err(EpisodeStoreError::Corrupt);
    };
    let header: SidecarHeader =
        serde_json::from_slice(&raw[..end]).map_err(|_| EpisodeStoreError::Corrupt)?;
    let body = &raw[end + 1..];
    if header.format != format || header.records == 0 || !body.ends_with(b"\n") {
        return Err(EpisodeStoreError::Corrupt);
    }
    let lines: Vec<&[u8]> = body
        .split_inclusive(|byte| *byte == b'\n')
        .map(|line| &line[..line.len() - 1])
        .collect();
    let digest: [u8; 32] = Sha256::digest(body).into();
    if u64::try_from(lines.len()).map_err(|_| EpisodeStoreError::Corrupt)? != header.records
        || digest != header.checksum
    {
        return Err(EpisodeStoreError::Corrupt);
    }
    Ok(lines)
}

/// Replace one sidecar with its header and its request lines.
///
/// The header carries a checksum over every request line, so the whole file
/// is rewritten on every write: a file truncated at a line boundary must
/// fail that checksum rather than look like a shorter history. The bytes are
/// written to a temporary file in the same directory, synchronized, renamed
/// over the target, and the directory entry is synchronized, so a reader sees
/// the old file or the new one and never a mixture.
fn write_sidecar<T: Serialize>(
    path: &Path,
    format: &str,
    lines: &[T],
    extra: &T,
) -> Result<(), EpisodeStoreError> {
    let mut body = Vec::new();
    for line in lines.iter().chain(std::iter::once(extra)) {
        let mut bytes = serde_json::to_vec(line).map_err(|_| EpisodeStoreError::Corrupt)?;
        bytes.push(b'\n');
        body.extend_from_slice(&bytes);
    }
    let records = u64::try_from(lines.len() + 1).map_err(|_| EpisodeStoreError::Corrupt)?;
    let header = SidecarHeader {
        format: format.to_string(),
        records,
        checksum: Sha256::digest(&body).into(),
    };
    let mut bytes = serde_json::to_vec(&header).map_err(|_| EpisodeStoreError::Corrupt)?;
    bytes.push(b'\n');
    bytes.extend_from_slice(&body);
    if u64::try_from(bytes.len()).map_err(|_| EpisodeStoreError::Io)? > MAX_LEDGER_BYTES {
        return Err(EpisodeStoreError::StorageBudget);
    }
    let parent = path.parent().ok_or(EpisodeStoreError::Io)?;
    let name = path.file_name().ok_or(EpisodeStoreError::Io)?;
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
        let mut file = options
            .open(&temporary)
            .map_err(|_| EpisodeStoreError::Io)?;
        if file.write_all(&bytes).is_err() || file.sync_all().is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(EpisodeStoreError::Io);
        }
    }
    if fs::rename(&temporary, path).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(EpisodeStoreError::Io);
    }
    sync_directory(parent)
}

/// The checksum over one redaction line's own fields.
fn redaction_checksum(
    guild_ref: &str,
    digest: &[u8; 32],
    kind: DerivedCopyKind,
    actor: &ActorRef,
) -> [u8; 32] {
    line_checksum(&[
        guild_ref.as_bytes(),
        digest.as_slice(),
        kind.label().as_bytes(),
        actor.principal_id.as_bytes(),
        actor.kind.label().as_bytes(),
    ])
}

/// The checksum over one object-key line's own fields.
fn object_key_checksum(
    guild_ref: &str,
    digest: &[u8; 32],
    key: &ObjectKeyRef,
    actor: &ActorRef,
) -> [u8; 32] {
    line_checksum(&[
        guild_ref.as_bytes(),
        digest.as_slice(),
        key.tenant.as_bytes(),
        key.key_id.as_bytes(),
        actor.principal_id.as_bytes(),
        actor.kind.label().as_bytes(),
    ])
}
