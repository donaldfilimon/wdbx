//! Retention classes and time-to-live expiry for the v3 episode ledger.
//!
//! A guild policy may configure a retention time-to-live
//! ([`GuildEpisodePolicy::retention_ttl_ms`](super::super::types::GuildEpisodePolicy))
//! and may configure a *per-class* window
//! ([`GuildEpisodePolicy::retention_windows`](super::super::types::GuildEpisodePolicy)).
//! The store binds that window when a memory candidate is admitted, reports
//! the deadline it bound, and emits the `forgets` candidates that close the
//! expired ones. `durable` candidates never expire.
//!
//! # Which window a candidate gets
//!
//! The window is the policy's, never the candidate's. A candidate carries no
//! window of its own, because a candidate-chosen window would have to be
//! bound into the candidate's canonical digest, and the frozen v3 record
//! format cannot carry it: a new field on `MemoryCandidate` would change
//! every candidate digest and break the pinned goldens and the cross-language
//! witness, and a new `EpisodeEvent` variant would change the canonical event
//! encoder, the witness, and every pinned golden with it. The remaining open
//! item is a *candidate-selected* window, and it needs a new event variant
//! and a coordinated golden regeneration.
//!
//! The precedence inside the policy is fixed
//! ([`GuildEpisodePolicy::retention_ttl_for`](super::super::types::GuildEpisodePolicy)):
//! an explicit per-class window wins, the guild's single time-to-live is the
//! fallback, and a class with neither never expires.
//!
//! # What the deadline is anchored to
//!
//! The deadline is anchored to the candidate's *arrival*: the moment the
//! canonical writer admitted it. The deadline is therefore
//! `arrival + ttl`, and it is bound once, at admission.
//!
//! The v3 record format carries no arrival timestamp. `StoredRecord` has no
//! time field at all, and the format is frozen: adding one would change
//! every episode digest and break the pinned goldens and the cross-language
//! witness. The deadline consequently cannot be derived from the record
//! itself, and the store does not guess one.
//!
//! It is committed instead in a *derived index* beside the ledger,
//! `episodes.v1.retention.jsonl`: one line per TTL-bound candidate, holding
//! the candidate's episode digest and the millisecond at which its window
//! closes. The index holds no episode content, no payload, and no second
//! canonical episode. It is the store's own record of a decision the store
//! took at admission, and it is written by the same single writer, in the
//! same order, before the ledger line it describes: a candidate that reaches
//! the ledger always has its deadline.
//!
//! # What is missing means
//!
//! A candidate with no line in the index has no deadline: it never expires,
//! and the store never emits a `forgets` candidate for it. That is the
//! fail-safe direction. A record whose deadline cannot be determined is never
//! destroyed on a guessed time; it is kept. A candidate admitted before the
//! guild configured a TTL, or one whose index line was lost, is kept.
//!
//! # What the gate refuses
//!
//! The write gate, not a background job, is the enforcement point for new
//! writes: it refuses a `session` or `operational` candidate whose guild TTL
//! is zero, and it refuses nothing else about the retention window. A zero
//! TTL is the only way a deadline can be closed at admission, because the
//! deadline is bound from the store's own clock at the instant of admission
//! (`arrival + 0` is the arrival itself). A positive TTL is admitted, and it
//! may expire immediately afterwards. The gate never compares a candidate's
//! deadline against a caller-supplied clock, and it never reads a clock at
//! all. See `store::validate`.

use super::super::types::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeSource, EpisodeWrite, EvidenceLevel,
    GuildEpisodePolicy, MemoryCandidate, MemoryClass, RetentionClass,
};
use super::filesystem::{owner_file, read_bounded_file_if_present, sync_directory};
use super::{EpisodeStore, EpisodeStoreError, MAX_LEDGER_BYTES, bounded_identifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// The service identity that records a retention expiry.
///
/// It is a service actor, like every other memory-candidate writer: the
/// expiry engine is not a person and never resolves a memory edge.
const EXPIRY_SERVICE: &str = "wdbx-retention-expiry";

/// The retention state of one admitted memory candidate.
///
/// Content-free by construction: a class, a deadline, and a flag. It never
/// carries a payload, a participant, or an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetentionState {
    /// The retention class the candidate was admitted with.
    pub class: RetentionClass,
    /// The millisecond at which the candidate's retention window closes.
    ///
    /// `None` means the candidate has no deadline: it is `durable`, or the
    /// guild configured no TTL, or the store holds no arrival stamp for it.
    pub expires_at: Option<u64>,
    /// True when the deadline has already passed.
    pub expired: bool,
}

/// One committed retention decision.
///
/// The checksum is SHA-256 over the line's own fields. It is a checksum, not
/// a signature: it detects a corrupted or half-written line, exactly as the
/// episode digest detects a mutated ledger line. It proves nothing about who
/// wrote the line.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RetentionStamp {
    /// Episode digest of the candidate the deadline belongs to.
    digest: [u8; 32],
    /// The millisecond at which the candidate's retention window closes.
    expires_at_ms: u64,
    /// SHA-256 over the digest and the deadline.
    checksum: [u8; 32],
}

/// The store's retention index: the committed deadline of every TTL-bound
/// candidate.
///
/// The index is derived state beside the ledger. Losing it does not lose an
/// episode: the ledger stays the canonical record, and the affected
/// candidates simply never expire. A store that never bound a TTL has no
/// index file at all: the file is created with the first deadline, not with
/// the store.
#[derive(Debug)]
pub(super) struct RetentionIndex {
    path: PathBuf,
    file: Option<File>,
    deadlines: BTreeMap<[u8; 32], u64>,
}

impl RetentionIndex {
    /// Open the index, replaying its complete lines.
    ///
    /// An incomplete final line is discarded and truncated, exactly as the
    /// ledger discards an incomplete final line. A complete line that does
    /// not parse, or whose checksum does not match, is corruption: the store
    /// refuses to open rather than silently forgetting a deadline.
    pub(super) fn open(path: &Path) -> Result<Self, EpisodeStoreError> {
        let Some(raw) = read_bounded_file_if_present(path)? else {
            return Ok(Self {
                path: path.to_path_buf(),
                file: None,
                deadlines: BTreeMap::new(),
            });
        };
        let (deadlines, valid_bytes) = decode_deadlines(&raw)?;
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
            deadlines,
        })
    }

    /// The deadline committed for one candidate, if any.
    pub(super) fn deadline(&self, digest: &[u8; 32]) -> Option<u64> {
        self.deadlines.get(digest).copied()
    }

    /// The number of bytes the index holds right now.
    ///
    /// An index that was never written holds nothing: the file is created
    /// with the first deadline, not with the store.
    pub(super) fn byte_len(&self) -> u64 {
        self.file
            .as_ref()
            .and_then(|file| file.metadata().ok())
            .map_or(0, |metadata| metadata.len())
    }

    /// An index with no deadlines and nowhere to write one.
    #[cfg(test)]
    pub(super) fn empty() -> Self {
        Self {
            path: PathBuf::new(),
            file: None,
            deadlines: BTreeMap::new(),
        }
    }

    /// Commit the deadline of one candidate.
    ///
    /// The line is written and synchronized before the ledger line it
    /// describes, so a candidate that reaches the ledger always has its
    /// deadline. A failure here fails the append: the store never admits a
    /// TTL-bound candidate it cannot later expire.
    ///
    /// The ceiling is charged at the greatest size a line can ever have, so
    /// the write gate's projection of this write and this check are the same
    /// arithmetic on the same number. The decision, the preview and the
    /// append therefore reach the same verdict.
    pub(super) fn commit(
        &mut self,
        digest: [u8; 32],
        expires_at_ms: u64,
    ) -> Result<(), EpisodeStoreError> {
        let stamp = RetentionStamp {
            digest,
            expires_at_ms,
            checksum: stamp_checksum(&digest, expires_at_ms),
        };
        let mut line = serde_json::to_vec(&stamp).map_err(|_| EpisodeStoreError::Corrupt)?;
        line.push(b'\n');
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
        if current.saturating_add(max_stamp_line_bytes()) > MAX_LEDGER_BYTES {
            return Err(EpisodeStoreError::StorageBudget);
        }
        if file.write_all(&line).is_err() || file.sync_data().is_err() {
            return Err(EpisodeStoreError::Io);
        }
        self.deadlines.insert(digest, expires_at_ms);
        Ok(())
    }
}

/// SHA-256 over the stamp's own fields.
fn stamp_checksum(digest: &[u8; 32], expires_at_ms: u64) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(digest);
    hasher.update(expires_at_ms.to_le_bytes());
    hasher.finalize().into()
}

/// The greatest number of bytes one retention stamp line can occupy.
///
/// The line is JSON over a fixed shape: a digest, a millisecond, and a
/// checksum. The bound is computed from the widest values the shape can
/// hold, so it is an upper bound on every line the index ever writes — and
/// the write gate can charge it against the ceiling before the line is
/// written, so the decision, the preview and the append agree.
pub(super) fn max_stamp_line_bytes() -> u64 {
    static BOUND: OnceLock<u64> = OnceLock::new();
    *BOUND.get_or_init(|| {
        let stamp = RetentionStamp {
            digest: [u8::MAX; 32],
            expires_at_ms: u64::MAX,
            checksum: [u8::MAX; 32],
        };
        let line = serde_json::to_vec(&stamp).expect("a retention stamp serializes");
        u64::try_from(line.len() + 1).unwrap_or(u64::MAX)
    })
}

/// Read the complete index lines, and the byte length of the complete prefix.
fn decode_deadlines(raw: &[u8]) -> Result<(BTreeMap<[u8; 32], u64>, usize), EpisodeStoreError> {
    let mut deadlines = BTreeMap::new();
    let mut start = 0_usize;
    while let Some(relative_end) = raw[start..].iter().position(|byte| *byte == b'\n') {
        let end = start + relative_end;
        let line = &raw[start..end];
        if line.is_empty() {
            return Err(EpisodeStoreError::Corrupt);
        }
        let stamp: RetentionStamp =
            serde_json::from_slice(line).map_err(|_| EpisodeStoreError::Corrupt)?;
        if stamp.checksum != stamp_checksum(&stamp.digest, stamp.expires_at_ms) {
            return Err(EpisodeStoreError::Corrupt);
        }
        deadlines.insert(stamp.digest, stamp.expires_at_ms);
        start = end + 1;
    }
    Ok((deadlines, start))
}

/// The current Unix millisecond, when the clock can be read.
///
/// The store reads the wall clock in exactly one place: the arrival of a
/// TTL-bound candidate. It never uses the clock to order records; the ledger
/// sequence orders records.
///
/// `None` means the clock is unreadable, which happens only when the system
/// clock is before the Unix epoch. The store then knows no arrival time, so
/// it binds no deadline: a candidate whose deadline cannot be determined is
/// never expired, and never destroyed on a guessed time.
pub(super) fn now_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok())
}

/// The retention deadline of one candidate, bound at admission.
///
/// A `forgets` candidate is a tombstone, not a memory: it is never given a
/// deadline, so a tombstone never expires and never asks to be forgotten
/// again. A `durable` candidate is never given one either.
///
/// The window is the *policy's*: the per-class window when the policy names
/// one for the candidate's class, and the guild's single time-to-live
/// otherwise. The candidate never chooses its own window — see the module
/// documentation for why the frozen record format cannot carry one.
///
/// The deadline needs an arrival time. Without one — no applicable window, no
/// class that expires, or an unreadable clock — there is no deadline, and the
/// candidate is kept.
///
/// The deadline is bound from the store's own clock at the instant of
/// admission, and it is bound once. This function never compares a deadline
/// against a caller-supplied time: the only way a deadline can already be
/// closed at admission is a zero window, and the write gate refuses that
/// (`store::validate`).
pub(super) fn bound_deadline(
    candidate: &MemoryCandidate,
    policy: &GuildEpisodePolicy,
    arrival_ms: Option<u64>,
) -> Option<u64> {
    if candidate.forgets.is_some() {
        return None;
    }
    if !matches!(
        candidate.retention,
        RetentionClass::Session | RetentionClass::Operational
    ) {
        return None;
    }
    Some(arrival_ms?.saturating_add(policy.retention_ttl_for(candidate.retention)?))
}

impl EpisodeStore {
    /// Report the retention state of one admitted memory candidate.
    ///
    /// `Ok(None)` means the digest is not a memory candidate admitted in this
    /// guild. The state is content-free: the retention class the candidate
    /// was admitted with, the deadline bound at its admission, and whether
    /// that deadline has passed.
    ///
    /// A candidate with no deadline reads `expires_at: None` and
    /// `expired: false`. That is the fail-safe reading: the store never
    /// destroys a record whose deadline it cannot determine.
    pub fn retention_state(
        &self,
        guild_ref: &str,
        candidate_digest: &[u8; 32],
    ) -> Result<Option<RetentionState>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        let Some(memories) = self.state.memories.get(guild_ref) else {
            return Ok(None);
        };
        let Some(class) = memories.retention.get(candidate_digest).copied() else {
            return Ok(None);
        };
        let expires_at = memories.expires_at.get(candidate_digest).copied();
        let expired =
            expires_at.is_some_and(|deadline| now_ms().is_some_and(|now| deadline <= now));
        Ok(Some(RetentionState {
            class,
            expires_at,
            expired,
        }))
    }

    /// The candidates whose retention window has closed.
    ///
    /// This is the decision side of expiry, and it mutates nothing: it
    /// appends no record and consumes no request identifier. The caller
    /// routes the result through [`Self::expiry_writes`] and the one
    /// canonical write path.
    ///
    /// A candidate is due when it holds a deadline that `now_ms` has reached
    /// or passed. A candidate that is already forgotten is not due: the
    /// tombstone is the end of that line, and forgetting it twice is
    /// refused by the write gate.
    ///
    /// A candidate with an open quarantine or an open contradiction is not
    /// due either. Those edges are an active review: the memory is still
    /// visible, and only a guild owner, administrator, manager, or
    /// organization owner resolves it. Expiring a candidate under review
    /// would destroy the very thing the review is about and would close the
    /// dispute without a decision. The candidate stays due, and is emitted
    /// once the edge is resolved.
    ///
    /// A candidate under a live hold is not due either. A hold freezes the
    /// candidate: its content is preserved until the authority that placed
    /// the hold releases it, and expiring it away would destroy the very
    /// thing the hold protects. The candidate stays due, and is emitted once
    /// the hold is released.
    pub fn due_expirations(
        &self,
        guild_ref: &str,
        now_ms: u64,
    ) -> Result<Vec<[u8; 32]>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        let Some(memories) = self.state.memories.get(guild_ref) else {
            return Ok(Vec::new());
        };
        let mut due = Vec::new();
        for (digest, deadline) in &memories.expires_at {
            if *deadline > now_ms || !memories.is_live(digest) {
                continue;
            }
            if memories.quarantined.contains_key(digest) || has_open_contradiction(memories, digest)
            {
                continue;
            }
            // A live hold freezes the candidate: it is not expired away.
            if self.is_held(guild_ref, digest) {
                continue;
            }
            due.push(*digest);
        }
        due.sort_unstable();
        Ok(due)
    }

    /// The `forgets` writes that close the expired candidates of one guild.
    ///
    /// The emitter never writes: it returns the writes, and the caller routes
    /// them through [`Self::propose_write`], the one canonical write path.
    /// Each emitted write is the ordinary forget-candidate shape: no payload,
    /// an all-zero payload commitment, no `supersedes`, the forgotten
    /// candidate's digest, and the forgotten candidate's retention class.
    ///
    /// Every emitted write needs its own request identifier, because the store
    /// refuses a replayed one. The emitter therefore appends `_0`, `_1`, ... to
    /// the identifiers the caller supplies; the caller supplies fresh
    /// identifiers for each round.
    ///
    /// The writes are proposed for the store's own guild policy, so they
    /// carry the bound contract revision, contract digest, and policy
    /// version, and they are admitted exactly like any other memory write.
    pub fn expiry_writes(
        &self,
        guild_ref: &str,
        now_ms: u64,
        next_request_id: &str,
        next_operation_id: &str,
    ) -> Result<Vec<EpisodeWrite>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        let guild = self
            .policy
            .guilds
            .get(guild_ref)
            .ok_or(EpisodeStoreError::StaleBinding)?;
        let due = self.due_expirations(guild_ref, now_ms)?;
        let Some(memories) = self.state.memories.get(guild_ref) else {
            return Ok(Vec::new());
        };
        let mut writes = Vec::with_capacity(due.len());
        for (index, target) in due.iter().enumerate() {
            let request_id = derived_identifier(next_request_id, index)?;
            let operation_id = derived_identifier(next_operation_id, index)?;
            let class = memories
                .retention
                .get(target)
                .copied()
                .unwrap_or(RetentionClass::Durable);
            let member_scoped = memories.admitted.get(target).copied().unwrap_or(false);
            let candidate = MemoryCandidate {
                class: MemoryClass::Fact,
                retention: class,
                payload_commitment: [0; 32],
                payload_bytes: 0,
                dimension: None,
                embedding_version: None,
                member_scoped,
                supersedes: None,
                forgets: Some(*target),
            };
            writes.push(EpisodeWrite {
                request_id,
                operation_id,
                contract_revision: self.policy.contract_revision,
                contract_digest: self.policy.contract_digest,
                guild_ref: guild_ref.to_string(),
                consent_epoch: None,
                source_type: EpisodeSource::LocalRuntime,
                policy_version: guild.policy_version.clone(),
                evidence_level: EvidenceLevel::C1,
                event: EpisodeEvent::MemoryCandidate {
                    recorded_by: ActorRef {
                        principal_id: EXPIRY_SERVICE.to_string(),
                        kind: ActorKind::Service,
                    },
                    candidate,
                },
                token_cost: 1,
                expected_commitment: None,
                quiet: false,
            });
        }
        Ok(writes)
    }
}

/// The identifier one emitted write carries.
///
/// The store refuses a replayed request identifier, so every emitted write
/// needs its own. The caller's identifier is the base and the write's index
/// completes it.
fn derived_identifier(base: &str, index: usize) -> Result<String, EpisodeStoreError> {
    let derived = format!("{base}_{index}");
    if !bounded_identifier(&derived, 64) {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(derived)
}

/// Whether one candidate takes part in an open contradiction.
fn has_open_contradiction(memories: &super::GuildMemories, digest: &[u8; 32]) -> bool {
    memories
        .contradictions
        .keys()
        .any(|(low, high)| low == digest || high == digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The size the write gate charges against the retention ceiling must
    /// cover every line the index can write, and must be the same number
    /// the index's own check uses.
    #[test]
    fn the_stamp_line_bound_covers_every_line() {
        let widest = RetentionStamp {
            digest: [u8::MAX; 32],
            expires_at_ms: u64::MAX,
            checksum: [u8::MAX; 32],
        };
        let mut widest_line = serde_json::to_vec(&widest).expect("stamp serializes");
        widest_line.push(b'\n');
        let narrowest = RetentionStamp {
            digest: [0; 32],
            expires_at_ms: 0,
            checksum: [0; 32],
        };
        let mut narrowest_line = serde_json::to_vec(&narrowest).expect("stamp serializes");
        narrowest_line.push(b'\n');

        let bound = max_stamp_line_bytes();
        assert!(
            bound >= u64::try_from(widest_line.len()).expect("length fits"),
            "the bound must cover the widest line: {bound} < {}",
            widest_line.len()
        );
        assert!(
            bound >= u64::try_from(narrowest_line.len()).expect("length fits"),
            "the bound must cover the narrowest line"
        );
        // The bound is the widest line itself: the gate and the index charge
        // exactly the same number.
        assert_eq!(
            bound,
            u64::try_from(widest_line.len()).expect("length fits"),
            "the gate and the index must charge the same size"
        );
    }
}
