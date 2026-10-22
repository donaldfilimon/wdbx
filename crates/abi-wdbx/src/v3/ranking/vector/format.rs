//! The binding index's durable file format.
//!
//! # The format
//!
//! The index lives in one file, `episodes.v1.vectors.jsonl`, beside its
//! writer lock `episodes.v1.vectors.lock`. It is a JSON Lines file: a header
//! line, then one line per binding.
//!
//! ```text
//! {"header":{"version":1,"records":2,"checksum":[32 bytes]}}
//! {"scope":"guild","vector_id":1,"episode_digest":[32 bytes],"checksum":[32 bytes]}
//! ```
//!
//! The header records the format version, how many binding lines the file
//! holds, and a checksum over the whole record set. The binding line carries
//! the scope, the vector identifier and the episode digest — and nothing else:
//! no payload, no actor, no event — with a checksum over the line's own
//! fields.
//!
//! # Why the header exists
//!
//! The index is written by whole-file atomic replacement, so a torn tail
//! cannot arise from the writer's own code path: the bytes are written to a
//! temporary file, synchronized, and renamed over the target. A file that
//! does not end in a complete line is therefore *damage*, not a crash tail,
//! and it is refused rather than trimmed.
//!
//! A truncation at a line boundary removes whole records with no visible
//! trace, so the header is what makes it detectable: the record count and the
//! record-set checksum are written on every commit, and a file whose records
//! do not match its header is refused rather than believed. The header is
//! what turns "the index opened with fewer bindings" into a refusal.
//!
//! # The discipline
//!
//! The file follows the discipline the repository already uses for its
//! derived sidecars:
//!
//! - the checksum is over the line's own fields, so a corrupted or
//!   half-written line is *detected* rather than believed. It is a checksum,
//!   not a signature: it proves the bytes are the bytes the index wrote, and
//!   nothing about who wrote them;
//! - a complete line that does not parse, or whose checksum does not match,
//!   is corruption: the index refuses to open rather than silently dropping
//!   a binding;
//! - a line the *writer* could never produce is corruption too: a scope that
//!   is not a bounded identifier, an identifier bound to two digests, or a
//!   digest bound to two identifiers are all refused, so the loader accepts
//!   exactly the states the writer can create;
//! - every write replaces the whole file atomically. An interrupted write
//!   leaves the previous index intact and openable.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use super::VectorBindingError;

/// The format version the header records.
pub(super) const FORMAT_VERSION: u32 = 1;

/// The greatest length of a scope identifier, in bytes.
pub(super) const MAX_SCOPE_BYTES: usize = 128;

/// The bindings of one scope: a vector identifier to an episode digest.
pub(super) type ScopeBindings = BTreeMap<u64, [u8; 32]>;

/// Every scope's bindings.
pub(super) type AllBindings = BTreeMap<String, ScopeBindings>;

/// The header's own record count and checksum over the record set.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct BindingHeader {
    /// The format version the file was written in.
    version: u32,
    /// How many binding lines the file holds.
    records: u64,
    /// SHA-256 over the record set.
    checksum: [u8; 32],
}

/// The first line of the file.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct HeaderLine {
    /// The header itself.
    header: BindingHeader,
}

/// One committed binding line.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct BindingStamp {
    /// The scope the binding belongs to.
    scope: String,
    /// The vector identifier the binding names.
    vector_id: u64,
    /// The episode digest the binding names.
    episode_digest: [u8; 32],
    /// SHA-256 over the line's own fields.
    checksum: [u8; 32],
}

/// SHA-256 over the fields of one binding line.
///
/// The parts are length-prefixed so no two different field sequences can
/// produce the same input. The checksum is a checksum, not a signature: it
/// detects a corrupted or half-written line, exactly as the episode digest
/// detects a mutated ledger line. It proves nothing about who wrote it.
pub(super) fn line_checksum(scope: &str, vector_id: u64, digest: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(u64::try_from(scope.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(scope.as_bytes());
    hasher.update(vector_id.to_le_bytes());
    hasher.update(digest);
    hasher.finalize().into()
}

/// SHA-256 over the whole record set, in the index's own order.
///
/// The bindings are walked in the index's canonical order — scope, then
/// vector identifier — so two runs holding the same bindings derive the same
/// checksum.
pub(super) fn record_set_checksum(bindings: &AllBindings) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for (scope, entries) in bindings {
        for (vector_id, digest) in entries {
            hasher.update(line_checksum(scope, *vector_id, digest));
        }
    }
    hasher.finalize().into()
}

/// How many bindings the index holds, over every scope.
pub(super) fn count(bindings: &AllBindings) -> u64 {
    let mut total = 0_u64;
    for entries in bindings.values() {
        total = total.saturating_add(u64::try_from(entries.len()).unwrap_or(u64::MAX));
    }
    total
}

/// Encode the whole index: the header line, then one line per binding.
///
/// The lines are written in the index's own order — scope, then vector
/// identifier — so two runs that hold the same bindings write the same bytes.
pub(super) fn encode(bindings: &AllBindings) -> Result<Vec<u8>, VectorBindingError> {
    let header = HeaderLine {
        header: BindingHeader {
            version: FORMAT_VERSION,
            records: count(bindings),
            checksum: record_set_checksum(bindings),
        },
    };
    let mut body = serde_json::to_vec(&header).map_err(|_| VectorBindingError::Corrupt)?;
    body.push(b'\n');
    for (scope, entries) in bindings {
        for (vector_id, digest) in entries {
            let stamp = BindingStamp {
                scope: scope.clone(),
                vector_id: *vector_id,
                episode_digest: *digest,
                checksum: line_checksum(scope, *vector_id, digest),
            };
            let mut line = serde_json::to_vec(&stamp).map_err(|_| VectorBindingError::Corrupt)?;
            line.push(b'\n');
            body.extend_from_slice(&line);
        }
    }
    Ok(body)
}

/// Decode the whole index.
///
/// The loader accepts exactly the states the writer can produce: a bounded
/// scope, an identifier bound to at most one digest, a digest bound to at
/// most one identifier, and a record set that matches the header's count and
/// checksum. Anything else is corruption.
pub(super) fn decode(raw: &[u8]) -> Result<AllBindings, VectorBindingError> {
    if raw.is_empty() || !raw.ends_with(b"\n") {
        // An empty file is not an index, and a file that does not end in a
        // complete line carries a fragment: the writer replaces the whole
        // file atomically, so a fragment is damage, not a crash tail.
        return Err(VectorBindingError::Corrupt);
    }
    let body = &raw[..raw.len() - 1];
    let mut lines = body.split(|byte| *byte == b'\n');
    let header_line = lines.next().ok_or(VectorBindingError::Corrupt)?;
    let header: HeaderLine =
        serde_json::from_slice(header_line).map_err(|_| VectorBindingError::Corrupt)?;
    if header.header.version != FORMAT_VERSION {
        return Err(VectorBindingError::Corrupt);
    }
    let mut bindings: AllBindings = BTreeMap::new();
    let mut digests: BTreeMap<String, BTreeMap<[u8; 32], u64>> = BTreeMap::new();
    let mut records = 0_u64;
    for line in lines {
        if line.is_empty() {
            return Err(VectorBindingError::Corrupt);
        }
        let stamp: BindingStamp =
            serde_json::from_slice(line).map_err(|_| VectorBindingError::Corrupt)?;
        if stamp.checksum != line_checksum(&stamp.scope, stamp.vector_id, &stamp.episode_digest) {
            return Err(VectorBindingError::Corrupt);
        }
        if !bounded_scope(&stamp.scope) {
            return Err(VectorBindingError::Corrupt);
        }
        let entries = bindings.entry(stamp.scope.clone()).or_default();
        if let Some(existing) = entries.get(&stamp.vector_id)
            && *existing != stamp.episode_digest
        {
            // Two complete lines name one identifier and disagree: the index
            // contradicts itself, and that is corruption.
            return Err(VectorBindingError::Corrupt);
        }
        let scope_digests = digests.entry(stamp.scope.clone()).or_default();
        if let Some(existing) = scope_digests.get(&stamp.episode_digest)
            && *existing != stamp.vector_id
        {
            // Two complete lines name one digest and disagree: the mapping is
            // one-to-one, so the index contradicts itself.
            return Err(VectorBindingError::Corrupt);
        }
        scope_digests.insert(stamp.episode_digest, stamp.vector_id);
        entries.insert(stamp.vector_id, stamp.episode_digest);
        records = records.saturating_add(1);
    }
    if records != header.header.records {
        // The header says how many records the file holds. Fewer means the
        // file was truncated at a line boundary.
        return Err(VectorBindingError::Corrupt);
    }
    if record_set_checksum(&bindings) != header.header.checksum {
        return Err(VectorBindingError::Corrupt);
    }
    Ok(bindings)
}

/// Merge a local snapshot with the bindings already on disk.
///
/// The merge is the union of the two, and it refuses to invent a winner: a
/// vector identifier that names two different digests is an
/// [`VectorBindingError::IdConflict`], and a digest that two identifiers
/// name is a [`VectorBindingError::DigestConflict`]. A binding the disk
/// holds and the snapshot does not is *kept*, which is what makes a write
/// lose nothing even when the writer lock is bypassed.
pub(super) fn merge(
    local: &AllBindings,
    disk: Option<&AllBindings>,
) -> Result<AllBindings, VectorBindingError> {
    let mut merged: AllBindings = BTreeMap::new();
    if let Some(disk) = disk {
        for (scope, entries) in disk {
            merged
                .entry(scope.clone())
                .or_default()
                .extend(entries.iter().map(|(id, digest)| (*id, *digest)));
        }
    }
    for (scope, entries) in local {
        let target = merged.entry(scope.clone()).or_default();
        for (vector_id, digest) in entries {
            if let Some(existing) = target.get(vector_id)
                && *existing != *digest
            {
                return Err(VectorBindingError::IdConflict);
            }
            target.insert(*vector_id, *digest);
        }
    }
    for entries in merged.values() {
        let mut named: BTreeMap<[u8; 32], u64> = BTreeMap::new();
        for (vector_id, digest) in entries {
            if let Some(existing) = named.get(digest)
                && *existing != *vector_id
            {
                return Err(VectorBindingError::DigestConflict);
            }
            named.insert(*digest, *vector_id);
        }
    }
    Ok(merged)
}

/// Whether the scope is a bounded identifier.
pub(super) fn bounded_scope(scope: &str) -> bool {
    !scope.is_empty() && scope.len() <= MAX_SCOPE_BYTES && !scope.chars().any(char::is_control)
}
