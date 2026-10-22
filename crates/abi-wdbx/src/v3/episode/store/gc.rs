//! Garbage collection of expired and forgotten memory records.
//!
//! # What is reclaimed, and what is kept
//!
//! The episode ledger is append-only, and a forgotten candidate stays in it
//! as a tombstone forever. That is correct: the tombstone is the minimal
//! anti-resurrection metadata, and a record that was erased must never
//! become live again under any later operation. So the tombstone is
//! **retained forever**, and this module never proposes reclaiming it.
//!
//! What may be reclaimed is the *payload-bearing record* of a candidate that
//! is already forgotten, whose retention deadline has passed, and which no
//! open quarantine or contradiction is reviewing. Such a record carries a
//! payload commitment and a payload size and nothing else; dropping it
//! returns its bytes to the guild's storage budget and destroys the store's
//! commitment to a payload the guild already forgot.
//!
//! The plan never proposes reclaiming:
//!
//! - a tombstone, which is retained forever;
//! - a live record, which the expiry engine must forget first;
//! - a record under an open quarantine or contradiction, which is an active
//!   review;
//! - a record under a live hold, which an authority froze;
//! - a record whose deadline is unknown, which is never destroyed on a
//!   guessed time.
//!
//! # How the rewrite keeps the ledger honest
//!
//! The v3 ledger is not one global hash chain: `previous_digest` chains the
//! events of a single *operation*, and a memory candidate is a single-event
//! operation whose `previous_digest` is `None`. Reclaiming a memory record
//! therefore breaks no chain: no surviving record names it as a parent.
//!
//! The canonical digest covers the header, the payload, and the parents of a
//! record — never its ledger `sequence`. The `sequence` is a position, not an
//! identity: the rewrite re-positions the surviving records so the ledger
//! stays contiguous (`1..n`), exactly as the store's own replay validation
//! requires. **Every surviving record keeps its digest byte for byte**, so
//! the pinned goldens and the cross-language witness do not move.
//!
//! Reclaiming a record does leave the ledger holding a tombstone whose
//! target has no record of its own. The tombstone still replays, because the
//! store remembers, content-free, which candidates it reclaimed: the
//! reclaimed index (`episodes.v1.reclaimed.jsonl`) holds the digest, the
//! member-scope flag, and the retention class of every reclaimed candidate,
//! and the replay rebuilds the candidate's admission from it. The index is
//! written *before* the ledger rewrite it describes, so a crash never leaves
//! a tombstone the store cannot replay.
//!
//! # Crash safety
//!
//! The rewrite writes a new ledger beside the old one and renames it over the
//! old one, after synchronizing it, and then synchronizes the directory. An
//! interrupted rewrite leaves the original ledger intact and openable: the
//! new bytes never touch the old file until the rename, and the rename is
//! atomic. The store holds the writer lock for its whole lifetime, so no
//! other writer can race the rewrite.

use super::super::types::{EpisodeEvent, RetentionClass};
use super::expiry::RetentionIndex;
use super::filesystem::{owner_file, read_bounded_file_if_present, sync_directory};
use super::validate::validate_stored_record;
use super::{
    EpisodeStore, EpisodeStoreError, GuildMemories, LedgerState, MAX_LEDGER_BYTES, StoredRecord,
    apply_record, bounded_identifier, serialized_line,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The temporary ledger the rewrite builds before it replaces the live one.
const GC_TEMP_FILE: &str = "episodes.v1.gc.tmp";

/// The plan of one garbage-collection round.
///
/// The plan is the decision side: it mutates nothing, appends nothing, and
/// consumes no request identifier. It separates the records that MAY be
/// reclaimed from the tombstones that are RETAINED forever.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GcPlan {
    /// The guild the plan covers.
    pub guild_ref: String,
    /// The millisecond the plan was computed at.
    pub now_ms: u64,
    /// The payload-bearing records that may be reclaimed, in ledger order.
    pub reclaimable: Vec<[u8; 32]>,
    /// The tombstone records that are retained forever, in ledger order.
    pub retained_tombstones: Vec<[u8; 32]>,
}

impl GcPlan {
    /// How many records the plan may reclaim.
    #[must_use]
    pub fn reclaimable_count(&self) -> usize {
        self.reclaimable.len()
    }

    /// How many tombstones the plan retains forever.
    #[must_use]
    pub fn retained_tombstone_count(&self) -> usize {
        self.retained_tombstones.len()
    }
}

/// The report of one applied garbage-collection round.
///
/// Content-free: digests and counts. It never carries a payload, an event,
/// or a path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GcReport {
    /// The guild the round covered.
    pub guild_ref: String,
    /// The records the round reclaimed, in ledger order.
    pub reclaimed: Vec<[u8; 32]>,
    /// The plan's records the round did not reclaim.
    ///
    /// A digest lands here when it is no longer eligible — it was already
    /// reclaimed, or it is not a forgotten record of this guild. The round
    /// never reclaims a record the plan could not justify.
    pub skipped: Vec<[u8; 32]>,
    /// How many tombstones the round retained.
    pub retained_tombstones: usize,
    /// The ledger's record count before the rewrite.
    pub records_before: usize,
    /// The ledger's record count after the rewrite.
    pub records_after: usize,
    /// The ledger's size in bytes before the rewrite.
    pub bytes_before: u64,
    /// The ledger's size in bytes after the rewrite.
    pub bytes_after: u64,
}

/// One reclaimed candidate, as the store remembers it.
///
/// The store keeps the candidate's admission and nothing else: it is not
/// content, and it is the minimum the ledger's tombstones need to keep
/// replaying.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ReclaimedCandidate {
    /// Whether the candidate was isolated to one member of the guild.
    member_scoped: bool,
    /// The retention class the candidate was admitted with.
    retention: RetentionClass,
}

/// One committed reclamation.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ReclaimedStamp {
    /// The guild the candidate belonged to.
    guild_ref: String,
    /// The reclaimed candidate's episode digest.
    digest: [u8; 32],
    /// Whether the candidate was isolated to one member of the guild.
    member_scoped: bool,
    /// The retention class the candidate was admitted with.
    retention: RetentionClass,
    /// SHA-256 over the line's own fields.
    checksum: [u8; 32],
}

impl ReclaimedStamp {
    /// The checksum over one reclamation line's own fields.
    fn checksum_of(
        guild_ref: &str,
        digest: &[u8; 32],
        member_scoped: bool,
        retention: RetentionClass,
    ) -> [u8; 32] {
        let mut hasher = Sha256::new();
        for part in [
            guild_ref.as_bytes(),
            digest.as_slice(),
            &[u8::from(member_scoped)][..],
            retention.label().as_bytes(),
        ] {
            hasher.update(u64::try_from(part.len()).unwrap_or(u64::MAX).to_le_bytes());
            hasher.update(part);
        }
        hasher.finalize().into()
    }
}

/// The reclamations the index holds: guild, digest, and admission.
type ReclaimedCandidates = BTreeMap<String, BTreeMap<[u8; 32], ReclaimedCandidate>>;

/// The store's reclaimed index: the candidates the store has reclaimed.
///
/// The index is derived state beside the ledger, in the same shape as the
/// retention index: one line per reclamation, a checksum over the line's own
/// fields, and no episode content. A store that never reclaimed anything has
/// no index file at all.
#[derive(Debug)]
pub(super) struct ReclaimedIndex {
    path: PathBuf,
    file: Option<File>,
    entries: ReclaimedCandidates,
}

impl Clone for ReclaimedIndex {
    /// Clone the index's entries, not its open file.
    ///
    /// The clone is a read-only view used to replay a rewritten ledger
    /// before it is written; the writer keeps the original index and its
    /// file.
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            file: None,
            entries: self.entries.clone(),
        }
    }
}

impl ReclaimedIndex {
    /// Open the index, replaying its complete lines.
    ///
    /// An incomplete final line is discarded and truncated, exactly as the
    /// ledger discards an incomplete final line. A complete line that does
    /// not parse, or whose checksum does not match, is corruption.
    pub(super) fn open(path: &Path) -> Result<Self, EpisodeStoreError> {
        let Some(raw) = read_bounded_file_if_present(path)? else {
            return Ok(Self {
                path: path.to_path_buf(),
                file: None,
                entries: BTreeMap::new(),
            });
        };
        let (entries, valid_bytes) = decode_reclaimed(&raw)?;
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
            entries,
        })
    }

    /// The admission the store remembers for one reclaimed candidate.
    fn candidate(&self, guild_ref: &str, digest: &[u8; 32]) -> Option<ReclaimedCandidate> {
        self.entries
            .get(guild_ref)
            .and_then(|candidates| candidates.get(digest))
            .copied()
    }

    /// Whether the index already names one candidate.
    fn contains(&self, guild_ref: &str, digest: &[u8; 32]) -> bool {
        self.candidate(guild_ref, digest).is_some()
    }

    /// Remember one candidate's admission in the index.
    fn insert(&mut self, guild_ref: &str, digest: [u8; 32], candidate: ReclaimedCandidate) {
        self.entries
            .entry(guild_ref.to_string())
            .or_default()
            .insert(digest, candidate);
    }

    /// Rebuild the admission of every reclaimed candidate.
    ///
    /// The reclaimed candidates have no record of their own any more, but the
    /// ledger's tombstones still name them, and a tombstone replays only
    /// when its target is live at that point of the replay. The index is
    /// what makes that true: it restores the admission, and the tombstone
    /// restores the forgetting.
    pub(super) fn inject(&self, state: &mut LedgerState, retention: &RetentionIndex) {
        for (guild_ref, candidates) in &self.entries {
            let memories = state.memories.entry(guild_ref.clone()).or_default();
            for (digest, candidate) in candidates {
                memories.admitted.insert(*digest, candidate.member_scoped);
                memories.retention.insert(*digest, candidate.retention);
                if let Some(deadline) = retention.deadline(digest) {
                    memories.expires_at.insert(*digest, deadline);
                }
            }
        }
    }

    /// Check that no reclaimed candidate became live again.
    ///
    /// A reclaimed candidate is a forgotten candidate. If the ledger ever
    /// showed it as live again — a tombstone lost, a forgotten digest
    /// resurrected — the store refuses to open rather than serve a ledger
    /// that contradicts its own reclamation.
    pub(super) fn verify(&self, state: &LedgerState) -> Result<(), EpisodeStoreError> {
        for (guild_ref, candidates) in &self.entries {
            let Some(memories) = state.memories.get(guild_ref) else {
                return Err(EpisodeStoreError::Corrupt);
            };
            for digest in candidates.keys() {
                if memories.is_live(digest) {
                    return Err(EpisodeStoreError::Corrupt);
                }
            }
        }
        Ok(())
    }

    /// Commit one reclamation.
    ///
    /// The line is written and synchronized before the ledger rewrite it
    /// describes, so a crash never leaves a tombstone the store cannot
    /// replay.
    pub(super) fn commit(
        &mut self,
        guild_ref: &str,
        digest: [u8; 32],
        member_scoped: bool,
        retention: RetentionClass,
    ) -> Result<(), EpisodeStoreError> {
        let stamp = ReclaimedStamp {
            guild_ref: guild_ref.to_string(),
            digest,
            member_scoped,
            retention,
            checksum: ReclaimedStamp::checksum_of(guild_ref, &digest, member_scoped, retention),
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
        self.entries
            .entry(guild_ref.to_string())
            .or_default()
            .insert(
                digest,
                ReclaimedCandidate {
                    member_scoped,
                    retention,
                },
            );
        Ok(())
    }
}

/// Read the complete index lines, and the byte length of the complete prefix.
fn decode_reclaimed(raw: &[u8]) -> Result<(ReclaimedCandidates, usize), EpisodeStoreError> {
    let mut entries: ReclaimedCandidates = BTreeMap::new();
    let mut start = 0_usize;
    while let Some(relative_end) = raw[start..].iter().position(|byte| *byte == b'\n') {
        let end = start + relative_end;
        let line = &raw[start..end];
        if line.is_empty() {
            return Err(EpisodeStoreError::Corrupt);
        }
        let stamp: ReclaimedStamp =
            serde_json::from_slice(line).map_err(|_| EpisodeStoreError::Corrupt)?;
        if stamp.checksum
            != ReclaimedStamp::checksum_of(
                &stamp.guild_ref,
                &stamp.digest,
                stamp.member_scoped,
                stamp.retention,
            )
        {
            return Err(EpisodeStoreError::Corrupt);
        }
        entries.entry(stamp.guild_ref).or_default().insert(
            stamp.digest,
            ReclaimedCandidate {
                member_scoped: stamp.member_scoped,
                retention: stamp.retention,
            },
        );
        start = end + 1;
    }
    Ok((entries, start))
}

/// Whether one candidate takes part in an open contradiction.
fn contradicted(memories: &GuildMemories, digest: &[u8; 32]) -> bool {
    memories
        .contradictions
        .keys()
        .any(|(low, high)| low == digest || high == digest)
}

/// Whether one record is a payload-bearing memory candidate.
fn payload_bearing(record: &StoredRecord) -> bool {
    matches!(
        &record.event,
        EpisodeEvent::MemoryCandidate { candidate, .. } if candidate.forgets.is_none()
    )
}

/// Split a plan's digests into the ones a round reclaims and the ones it skips.
///
/// A digest the plan names that is no longer eligible is skipped, never
/// reclaimed: the round reclaims only what the store can still justify.
fn split_plan(plan: &GcPlan, eligible: &BTreeSet<[u8; 32]>) -> (Vec<[u8; 32]>, Vec<[u8; 32]>) {
    let mut reclaimed = Vec::new();
    let mut skipped = Vec::new();
    for digest in &plan.reclaimable {
        if eligible.contains(digest) {
            reclaimed.push(*digest);
        } else {
            skipped.push(*digest);
        }
    }
    (reclaimed, skipped)
}

impl EpisodeStore {
    /// Plan one garbage-collection round for one guild.
    ///
    /// The plan is the decision side of the collection: it mutates nothing,
    /// appends nothing, and consumes no request identifier. It lists the
    /// records that may be reclaimed and, separately, the tombstones that
    /// are retained forever.
    ///
    /// A record may be reclaimed only when all three hold: its retention
    /// deadline has passed, it is already forgotten, and no open quarantine
    /// or contradiction is reviewing it. A record whose deadline the store
    /// does not know is never reclaimed: the store never destroys a record on
    /// a guessed time. A record under a live hold is never reclaimed either:
    /// the hold freezes the candidate's content until the authority that
    /// placed it releases the hold.
    pub fn gc_plan(&self, guild_ref: &str, now_ms: u64) -> Result<GcPlan, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        if !self.policy.guilds.contains_key(guild_ref) {
            return Err(EpisodeStoreError::StaleBinding);
        }
        let mut retained_tombstones = Vec::new();
        let eligible = self.reclaimable_at(guild_ref, now_ms);
        let mut reclaimable = Vec::new();
        for record in self
            .records
            .iter()
            .filter(|record| record.guild_ref == guild_ref)
        {
            let EpisodeEvent::MemoryCandidate { candidate, .. } = &record.event else {
                continue;
            };
            if candidate.forgets.is_some() {
                retained_tombstones.push(record.episode_digest);
            } else if eligible.contains(&record.episode_digest) {
                reclaimable.push(record.episode_digest);
            }
        }
        Ok(GcPlan {
            guild_ref: guild_ref.to_string(),
            now_ms,
            reclaimable,
            retained_tombstones,
        })
    }

    /// The records one guild may reclaim at one instant.
    fn reclaimable_at(&self, guild_ref: &str, now_ms: u64) -> BTreeSet<[u8; 32]> {
        let Some(memories) = self.state.memories.get(guild_ref) else {
            return BTreeSet::new();
        };
        let mut reclaimable = BTreeSet::new();
        for record in self
            .records
            .iter()
            .filter(|record| record.guild_ref == guild_ref)
        {
            if !payload_bearing(record) {
                continue;
            }
            let digest = record.episode_digest;
            // A live record is not the collector's to reclaim: the expiry
            // engine must forget it first.
            if !memories.forgotten.contains(&digest) {
                continue;
            }
            // A record whose deadline the store does not know is kept.
            let Some(deadline) = memories.expires_at.get(&digest) else {
                continue;
            };
            if *deadline > now_ms {
                continue;
            }
            // A record under review is kept: the review is about that record.
            if memories.quarantined.contains_key(&digest) || contradicted(memories, &digest) {
                continue;
            }
            // A record under a live hold is kept: the hold freezes the
            // candidate's content until the authority that placed it
            // releases the hold.
            if self.is_held(guild_ref, &digest) {
                continue;
            }
            reclaimable.insert(digest);
        }
        reclaimable
    }

    /// Apply one garbage-collection plan.
    ///
    /// This commits: the ledger is rewritten without the reclaimable
    /// records, the reclaimed index is written before it, and the store's
    /// state is rebuilt from the rewritten ledger. The surviving records keep
    /// their order and their digests byte for byte; only their ledger
    /// `sequence` is re-positioned, because the sequence is a position and
    /// the canonical digest does not cover it.
    ///
    /// The plan is re-checked against the store's state: a digest the plan
    /// names that is no longer eligible is skipped and reported, never
    /// reclaimed. A tombstone is never reclaimed, and a reclaimed candidate
    /// never becomes live again.
    ///
    /// The rewrite is crash-safe: the new ledger is written beside the old
    /// one, synchronized, and renamed over it. An interrupted rewrite leaves
    /// the original ledger intact and openable.
    pub fn apply_gc_plan(&mut self, plan: &GcPlan) -> Result<GcReport, EpisodeStoreError> {
        if self.poisoned {
            return Err(EpisodeStoreError::Io);
        }
        if !bounded_identifier(&plan.guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        if !self.policy.guilds.contains_key(&plan.guild_ref) {
            return Err(EpisodeStoreError::StaleBinding);
        }
        let eligible = self.reclaimable_at(&plan.guild_ref, plan.now_ms);
        let (reclaimed, skipped) = split_plan(plan, &eligible);
        let records_before = self.records.len();
        let bytes_before = self
            .ledger
            .metadata()
            .map_err(|_| EpisodeStoreError::Io)?
            .len();
        if reclaimed.is_empty() {
            // Nothing is reclaimable: the ledger is not rewritten, so the
            // ledger's bytes and every record's position stay exactly as
            // they are.
            return Ok(GcReport {
                guild_ref: plan.guild_ref.clone(),
                reclaimed,
                skipped,
                retained_tombstones: plan.retained_tombstones.len(),
                records_before,
                records_after: records_before,
                bytes_before,
                bytes_after: bytes_before,
            });
        }
        let survivors = self.survivors_after(&plan.guild_ref, &reclaimed)?;
        let admissions = self.admissions_of(&plan.guild_ref, &reclaimed)?;
        let (mut state, lines) = self.replay_survivors(&plan.guild_ref, &survivors, &admissions)?;
        // The revocations are not in the ledger, so the rewritten state takes
        // them from the revocation index: a revoked candidate stays revoked
        // across a collection round.
        self.inject_revocations(&mut state);
        self.commit_reclamations(&plan.guild_ref, &admissions)?;
        let bytes_after = u64::try_from(lines.iter().map(Vec::len).sum::<usize>())
            .map_err(|_| EpisodeStoreError::Io)?;
        self.rewrite_ledger(&lines)?;

        self.records = survivors;
        self.state = state;
        Ok(GcReport {
            guild_ref: plan.guild_ref.clone(),
            reclaimed,
            skipped,
            retained_tombstones: plan.retained_tombstones.len(),
            records_before,
            records_after: self.records.len(),
            bytes_before,
            bytes_after,
        })
    }

    /// The records that survive one collection round.
    ///
    /// The sequence is a position in the ledger, not part of the record's
    /// identity: the canonical digest covers the header, the payload, and
    /// the parents, never the sequence. Re-positioning the survivors
    /// therefore leaves every surviving digest unchanged.
    fn survivors_after(
        &self,
        guild_ref: &str,
        reclaimed: &[[u8; 32]],
    ) -> Result<Vec<StoredRecord>, EpisodeStoreError> {
        let remove: BTreeSet<[u8; 32]> = reclaimed.iter().copied().collect();
        let mut survivors: Vec<StoredRecord> = Vec::with_capacity(self.records.len());
        for record in &self.records {
            if record.guild_ref == guild_ref
                && remove.contains(&record.episode_digest)
                && payload_bearing(record)
            {
                continue;
            }
            survivors.push(record.clone());
        }
        for (index, record) in survivors.iter_mut().enumerate() {
            record.sequence =
                u64::try_from(index + 1).map_err(|_| EpisodeStoreError::InvalidInput)?;
        }
        Ok(survivors)
    }

    /// The admission the store remembers for every reclaimed candidate.
    ///
    /// The store keeps it so the ledger's tombstones still replay after the
    /// record they name is gone.
    fn admissions_of(
        &self,
        guild_ref: &str,
        reclaimed: &[[u8; 32]],
    ) -> Result<BTreeMap<[u8; 32], ReclaimedCandidate>, EpisodeStoreError> {
        let memories = self
            .state
            .memories
            .get(guild_ref)
            .ok_or(EpisodeStoreError::InvalidTransition)?;
        let mut admissions: BTreeMap<[u8; 32], ReclaimedCandidate> = BTreeMap::new();
        for digest in reclaimed {
            let member_scoped = memories.admitted.get(digest).copied().unwrap_or(false);
            let retention = memories
                .retention
                .get(digest)
                .copied()
                .unwrap_or(RetentionClass::Durable);
            admissions.insert(
                *digest,
                ReclaimedCandidate {
                    member_scoped,
                    retention,
                },
            );
        }
        Ok(admissions)
    }

    /// Replay the rewritten ledger before it is written.
    ///
    /// The store never publishes a ledger it cannot replay itself.
    fn replay_survivors(
        &self,
        guild_ref: &str,
        survivors: &[StoredRecord],
        admissions: &BTreeMap<[u8; 32], ReclaimedCandidate>,
    ) -> Result<(LedgerState, Vec<Vec<u8>>), EpisodeStoreError> {
        let mut index = self.reclaimed.clone();
        for (digest, candidate) in admissions {
            index.insert(guild_ref, *digest, *candidate);
        }
        let mut state = LedgerState::default();
        index.inject(&mut state, &self.retention);
        let mut lines: Vec<Vec<u8>> = Vec::with_capacity(survivors.len());
        for record in survivors {
            let line = serialized_line(record)?;
            validate_stored_record(record, &state, 0)?;
            apply_record(record, line.len(), &mut state, &self.retention)?;
            lines.push(line);
        }
        Ok((state, lines))
    }

    /// Write the reclaimed index before the ledger rewrite it describes.
    ///
    /// A crash between the two leaves the original ledger intact and a
    /// reclaimed index the store can replay: the candidate is remembered as
    /// reclaimed while its record is still there, which is exactly what the
    /// replay needs.
    fn commit_reclamations(
        &mut self,
        guild_ref: &str,
        admissions: &BTreeMap<[u8; 32], ReclaimedCandidate>,
    ) -> Result<(), EpisodeStoreError> {
        for (digest, candidate) in admissions {
            if self.reclaimed.contains(guild_ref, digest) {
                continue;
            }
            self.reclaimed.commit(
                guild_ref,
                *digest,
                candidate.member_scoped,
                candidate.retention,
            )?;
        }
        Ok(())
    }

    /// Replace the live ledger with the rewritten one.
    ///
    /// The new bytes are written beside the old ledger, synchronized, and
    /// renamed over it. The rename is atomic, so an interrupted rewrite
    /// leaves the original ledger intact and openable.
    fn rewrite_ledger(&mut self, lines: &[Vec<u8>]) -> Result<(), EpisodeStoreError> {
        let directory = self.directory.clone();
        let ledger_path = directory.join(super::LEDGER_FILE);
        let temp_path = directory.join(GC_TEMP_FILE);
        super::filesystem::reject_symlink_if_present(&temp_path)?;
        let mut file = create_private(&temp_path)?;
        for line in lines {
            if file.write_all(line).is_err() {
                return Err(EpisodeStoreError::Io);
            }
        }
        file.sync_all().map_err(|_| EpisodeStoreError::Io)?;
        drop(file);
        fs::rename(&temp_path, &ledger_path).map_err(|_| EpisodeStoreError::Io)?;
        sync_directory(&directory)?;
        self.ledger = owner_file(&ledger_path)?;
        Ok(())
    }
}

/// Create the rewritten ledger beside the live one, private to the store.
fn create_private(path: &Path) -> Result<File, EpisodeStoreError> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path).map_err(|_| EpisodeStoreError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| EpisodeStoreError::Io)?;
    }
    Ok(file)
}
