//! The two derived indexes behind the holds and the revocations.
//!
//! The indexes are the store's own durable record of a governance decision:
//! one line per decision, a checksum over the line's own fields, no episode
//! content, written by the same single writer that appends the ledger. The
//! format is the one the retention, redaction, object-key, and reclaimed
//! indexes already use, and the reasons for it are in the parent module's
//! documentation.

use super::super::super::types::{ActorKind, ActorRef};
use super::super::filesystem::{owner_file, read_bounded_file_if_present, sync_directory};
use super::super::{EpisodeStoreError, LedgerState, MAX_LEDGER_BYTES};
use super::{HoldKind, HoldStatus};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// One committed hold, as the index remembers it.
#[derive(Clone, Debug)]
pub(in crate::v3::episode::store) struct HoldRecord {
    /// The authority that placed the hold.
    pub(in crate::v3::episode::store) authority: ActorRef,
    /// Whether the hold is in force.
    pub(in crate::v3::episode::store) status: HoldStatus,
    /// The authority that released the hold, when it was released.
    pub(in crate::v3::episode::store) released_by: Option<ActorRef>,
}

/// One committed hold line.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct HoldStamp {
    /// The guild the target belongs to.
    guild_ref: String,
    /// The candidate the hold names.
    digest: [u8; 32],
    /// The kind of freeze.
    kind: HoldKind,
    /// The principal of the authority that placed the hold.
    principal_id: String,
    /// The class of the authority that placed the hold.
    authority_kind: ActorKind,
    /// Whether the hold is in force.
    status: HoldStatus,
    /// The principal of the authority that released the hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    released_by: Option<String>,
    /// The class of the authority that released the hold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    released_by_kind: Option<ActorKind>,
    /// SHA-256 over the line's own fields.
    checksum: [u8; 32],
}

/// One committed revocation line.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RevocationStamp {
    /// The guild the target belongs to.
    guild_ref: String,
    /// The candidate the revocation names.
    digest: [u8; 32],
    /// The principal of the authority that revoked the candidate.
    principal_id: String,
    /// The class of the authority that revoked the candidate.
    authority_kind: ActorKind,
    /// SHA-256 over the line's own fields.
    checksum: [u8; 32],
}

/// SHA-256 over the fields of one index line.
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

/// The holds the index holds: guild, target, and kind.
type HoldTargets = BTreeMap<String, BTreeMap<[u8; 32], BTreeMap<HoldKind, HoldRecord>>>;

/// The revocations the index holds: guild and target.
type Revocations = BTreeMap<String, BTreeMap<[u8; 32], ActorRef>>;

/// The store's hold index: the committed holds on memory candidates.
///
/// The index is derived state beside the ledger. A store that never placed a
/// hold has no index file at all: the file is created with the first hold,
/// not with the store.
#[derive(Debug)]
pub(in crate::v3::episode::store) struct HoldIndex {
    path: PathBuf,
    file: Option<File>,
    holds: HoldTargets,
}

impl HoldIndex {
    /// Open the index, replaying its complete lines.
    ///
    /// An incomplete final line is discarded and truncated, exactly as the
    /// ledger discards an incomplete final line. A complete line that does
    /// not parse, or whose checksum does not match, is corruption: the store
    /// refuses to open rather than silently dropping a hold.
    pub(in crate::v3::episode::store) fn open(path: &Path) -> Result<Self, EpisodeStoreError> {
        let Some(raw) = read_bounded_file_if_present(path)? else {
            return Ok(Self {
                path: path.to_path_buf(),
                file: None,
                holds: BTreeMap::new(),
            });
        };
        let (holds, valid_bytes) = decode_holds(&raw)?;
        if valid_bytes != raw.len() {
            let repair = OpenOptions::new()
                .write(true)
                .open(path)
                .map_err(|_| EpisodeStoreError::Io)?;
            repair
                .set_len(u64::try_from(valid_bytes).map_err(|_| EpisodeStoreError::Io)?)
                .map_err(|_| EpisodeStoreError::Io)?;
            repair.sync_data().map_err(|_| EpisodeStoreError::Io)?;
        }
        let file = owner_file(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            file: Some(file),
            holds,
        })
    }

    /// The record of one hold, when the index holds one.
    pub(super) fn record(
        &self,
        guild_ref: &str,
        digest: &[u8; 32],
        kind: HoldKind,
    ) -> Option<&HoldRecord> {
        self.holds
            .get(guild_ref)
            .and_then(|targets| targets.get(digest))
            .and_then(|holds| holds.get(&kind))
    }

    /// The holds one target carries, in canonical kind order.
    pub(super) fn holds_of(
        &self,
        guild_ref: &str,
        digest: &[u8; 32],
    ) -> Vec<(HoldKind, HoldRecord)> {
        self.holds
            .get(guild_ref)
            .and_then(|targets| targets.get(digest))
            .map(|holds| {
                holds
                    .iter()
                    .map(|(kind, record)| (*kind, record.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Whether a live hold names one target.
    pub(super) fn is_held(&self, guild_ref: &str, digest: &[u8; 32]) -> bool {
        self.holds_of(guild_ref, digest)
            .iter()
            .any(|(_, record)| record.status == HoldStatus::Live)
    }

    /// Commit one hold decision.
    ///
    /// The line is written and synchronized before the receipt that reports
    /// it is returned, so a receipt never claims a hold the store did not
    /// durably record.
    pub(super) fn commit(
        &mut self,
        guild_ref: &str,
        digest: [u8; 32],
        kind: HoldKind,
        authority: &ActorRef,
        status: HoldStatus,
        released_by: Option<&ActorRef>,
    ) -> Result<(), EpisodeStoreError> {
        let stamp = HoldStamp {
            guild_ref: guild_ref.to_string(),
            digest,
            kind,
            principal_id: authority.principal_id.clone(),
            authority_kind: authority.kind,
            status,
            released_by: released_by.map(|actor| actor.principal_id.clone()),
            released_by_kind: released_by.map(|actor| actor.kind),
            checksum: hold_checksum(&stamp_fields(
                guild_ref,
                &digest,
                kind,
                authority,
                status,
                released_by,
            )),
        };
        let mut line = serde_json::to_vec(&stamp).map_err(|_| EpisodeStoreError::Corrupt)?;
        line.push(b'\n');
        let line_bytes = u64::try_from(line.len()).map_err(|_| EpisodeStoreError::Io)?;
        if self.file.is_none() {
            let file = owner_file(&self.path)?;
            // Publish the directory entry before the line it will hold.
            if let Some(parent) = self.path.parent() {
                sync_directory(parent)?;
            }
            self.file = Some(file);
        }
        let file = self.file.as_mut().expect("the index file exists");
        let current = file.metadata().map_err(|_| EpisodeStoreError::Io)?.len();
        if current.saturating_add(line_bytes) > MAX_LEDGER_BYTES {
            return Err(EpisodeStoreError::StorageBudget);
        }
        if file.write_all(&line).is_err() || file.sync_data().is_err() {
            return Err(EpisodeStoreError::Io);
        }
        self.holds
            .entry(guild_ref.to_string())
            .or_default()
            .entry(digest)
            .or_default()
            .insert(
                kind,
                HoldRecord {
                    authority: authority.clone(),
                    status,
                    released_by: released_by.cloned(),
                },
            );
        Ok(())
    }
}

/// The fields one hold line's checksum covers.
fn stamp_fields<'a>(
    guild_ref: &'a str,
    digest: &'a [u8; 32],
    kind: HoldKind,
    authority: &'a ActorRef,
    status: HoldStatus,
    released_by: Option<&'a ActorRef>,
) -> Vec<&'a [u8]> {
    let principal = authority.principal_id.as_bytes();
    let authority_kind = authority.kind.label().as_bytes();
    let (released, released_kind) = match released_by {
        Some(actor) => (actor.principal_id.as_bytes(), actor.kind.label().as_bytes()),
        None => (&[][..], &[][..]),
    };
    vec![
        guild_ref.as_bytes(),
        digest.as_slice(),
        kind.label().as_bytes(),
        principal,
        authority_kind,
        status.label().as_bytes(),
        released,
        released_kind,
    ]
}

/// SHA-256 over one hold line's own fields.
fn hold_checksum(fields: &[&[u8]]) -> [u8; 32] {
    line_checksum(fields)
}

/// Read the complete hold lines, and the byte length of the complete prefix.
fn decode_holds(raw: &[u8]) -> Result<(HoldTargets, usize), EpisodeStoreError> {
    let mut holds: HoldTargets = BTreeMap::new();
    let mut start = 0_usize;
    while let Some(relative_end) = raw[start..].iter().position(|byte| *byte == b'\n') {
        let end = start + relative_end;
        let line = &raw[start..end];
        if line.is_empty() {
            return Err(EpisodeStoreError::Corrupt);
        }
        let stamp: HoldStamp =
            serde_json::from_slice(line).map_err(|_| EpisodeStoreError::Corrupt)?;
        let authority = ActorRef {
            principal_id: stamp.principal_id.clone(),
            kind: stamp.authority_kind,
        };
        let released_by = match (stamp.released_by.clone(), stamp.released_by_kind) {
            (Some(principal_id), Some(kind)) => Some(ActorRef { principal_id, kind }),
            (None, None) => None,
            _ => return Err(EpisodeStoreError::Corrupt),
        };
        let expected = hold_checksum(&stamp_fields(
            &stamp.guild_ref,
            &stamp.digest,
            stamp.kind,
            &authority,
            stamp.status,
            released_by.as_ref(),
        ));
        if stamp.checksum != expected {
            return Err(EpisodeStoreError::Corrupt);
        }
        holds
            .entry(stamp.guild_ref)
            .or_default()
            .entry(stamp.digest)
            .or_default()
            .insert(
                stamp.kind,
                HoldRecord {
                    authority,
                    status: stamp.status,
                    released_by,
                },
            );
        start = end + 1;
    }
    Ok((holds, start))
}

/// The store's revocation index: the committed candidate revocations.
///
/// The index is derived state beside the ledger. A store that never revoked
/// a candidate has no index file at all.
#[derive(Debug)]
pub(in crate::v3::episode::store) struct RevocationIndex {
    path: PathBuf,
    file: Option<File>,
    revocations: Revocations,
}

impl RevocationIndex {
    /// Open the index, replaying its complete lines.
    ///
    /// An incomplete final line is discarded and truncated. A complete line
    /// that does not parse, or whose checksum does not match, is corruption.
    pub(in crate::v3::episode::store) fn open(path: &Path) -> Result<Self, EpisodeStoreError> {
        let Some(raw) = read_bounded_file_if_present(path)? else {
            return Ok(Self {
                path: path.to_path_buf(),
                file: None,
                revocations: BTreeMap::new(),
            });
        };
        let (revocations, valid_bytes) = decode_revocations(&raw)?;
        if valid_bytes != raw.len() {
            let repair = OpenOptions::new()
                .write(true)
                .open(path)
                .map_err(|_| EpisodeStoreError::Io)?;
            repair
                .set_len(u64::try_from(valid_bytes).map_err(|_| EpisodeStoreError::Io)?)
                .map_err(|_| EpisodeStoreError::Io)?;
            repair.sync_data().map_err(|_| EpisodeStoreError::Io)?;
        }
        let file = owner_file(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            file: Some(file),
            revocations,
        })
    }

    /// The authority that revoked one candidate, when it was revoked.
    pub(super) fn authority_of(&self, guild_ref: &str, digest: &[u8; 32]) -> Option<&ActorRef> {
        self.revocations
            .get(guild_ref)
            .and_then(|targets| targets.get(digest))
    }

    /// Commit one revocation.
    ///
    /// The line is written and synchronized before the receipt that reports
    /// it is returned.
    pub(super) fn commit(
        &mut self,
        guild_ref: &str,
        digest: [u8; 32],
        authority: &ActorRef,
    ) -> Result<(), EpisodeStoreError> {
        let stamp = RevocationStamp {
            guild_ref: guild_ref.to_string(),
            digest,
            principal_id: authority.principal_id.clone(),
            authority_kind: authority.kind,
            checksum: revocation_checksum(guild_ref, &digest, authority),
        };
        let mut line = serde_json::to_vec(&stamp).map_err(|_| EpisodeStoreError::Corrupt)?;
        line.push(b'\n');
        let line_bytes = u64::try_from(line.len()).map_err(|_| EpisodeStoreError::Io)?;
        if self.file.is_none() {
            let file = owner_file(&self.path)?;
            // Publish the directory entry before the line it will hold.
            if let Some(parent) = self.path.parent() {
                sync_directory(parent)?;
            }
            self.file = Some(file);
        }
        let file = self.file.as_mut().expect("the index file exists");
        let current = file.metadata().map_err(|_| EpisodeStoreError::Io)?.len();
        if current.saturating_add(line_bytes) > MAX_LEDGER_BYTES {
            return Err(EpisodeStoreError::StorageBudget);
        }
        if file.write_all(&line).is_err() || file.sync_data().is_err() {
            return Err(EpisodeStoreError::Io);
        }
        self.revocations
            .entry(guild_ref.to_string())
            .or_default()
            .insert(digest, authority.clone());
        Ok(())
    }

    /// Rebuild the revoked candidates of the store's replayed state.
    ///
    /// The revocation is applied *after* the ledger replay, never before it:
    /// a ledger that was written before the revocation must still replay, and
    /// the replay's own liveness checks are about the ledger's consistency,
    /// not about a decision taken after it.
    pub(in crate::v3::episode::store) fn inject(&self, state: &mut LedgerState) {
        for (guild_ref, candidates) in &self.revocations {
            let memories = state.memories.entry(guild_ref.clone()).or_default();
            for digest in candidates.keys() {
                memories.revoked.insert(*digest);
            }
        }
    }
}

/// SHA-256 over one revocation line's own fields.
fn revocation_checksum(guild_ref: &str, digest: &[u8; 32], authority: &ActorRef) -> [u8; 32] {
    line_checksum(&[
        guild_ref.as_bytes(),
        digest.as_slice(),
        authority.principal_id.as_bytes(),
        authority.kind.label().as_bytes(),
    ])
}

/// Read the complete revocation lines, and the byte length of the complete
/// prefix.
fn decode_revocations(raw: &[u8]) -> Result<(Revocations, usize), EpisodeStoreError> {
    let mut revocations: Revocations = BTreeMap::new();
    let mut start = 0_usize;
    while let Some(relative_end) = raw[start..].iter().position(|byte| *byte == b'\n') {
        let end = start + relative_end;
        let line = &raw[start..end];
        if line.is_empty() {
            return Err(EpisodeStoreError::Corrupt);
        }
        let stamp: RevocationStamp =
            serde_json::from_slice(line).map_err(|_| EpisodeStoreError::Corrupt)?;
        let authority = ActorRef {
            principal_id: stamp.principal_id.clone(),
            kind: stamp.authority_kind,
        };
        if stamp.checksum != revocation_checksum(&stamp.guild_ref, &stamp.digest, &authority) {
            return Err(EpisodeStoreError::Corrupt);
        }
        revocations
            .entry(stamp.guild_ref)
            .or_default()
            .insert(stamp.digest, authority);
        start = end + 1;
    }
    Ok((revocations, start))
}
