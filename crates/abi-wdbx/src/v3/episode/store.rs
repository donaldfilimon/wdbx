//! Durable append-only episode ledger and write gate.

use super::signing::{
    EpisodeSignature, EpisodeSigner, SignatureStatus, SignerKeyId, check_signature,
};
use super::types::{
    ActorRef, EpisodeEvent, EpisodeReceipt, EpisodeSource, EpisodeWrite, MemoryEdge,
    MemoryEdgeKind, MemoryEdgeState, OpenContradiction, RetentionClass, StorePolicy,
    TerminalStatus,
};
use crate::v3::commitment::CanonicalCborError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

mod admission_gate;
mod authority;
mod canonical;
pub(crate) mod deletion;
pub(crate) mod expiry;
mod filesystem;
pub(crate) mod gc;
pub(crate) mod holds;
pub(crate) mod lineage;
pub(crate) mod query;
pub(crate) mod quota;
mod validate;

pub use deletion::{
    DerivedCopyKind, ObjectKeyDestructionDecision, ObjectKeyDestructionReceipt,
    ObjectKeyDestructionRequest, ObjectKeyDestructionState, ObjectKeyRef, RedactionDecision,
    RedactionReceipt, RedactionRequest, RedactionState,
};
pub use expiry::RetentionState;
pub use gc::{GcPlan, GcReport};
pub use holds::{
    Hold, HoldDecision, HoldKind, HoldReceipt, HoldReleaseReceipt, HoldReleaseRequest, HoldRequest,
    HoldState, HoldStatus, RevocationDecision, RevocationReceipt, RevocationRequest,
    RevocationState,
};
pub use lineage::SupersessionLineage;
pub use quota::{
    QuotaAdjustmentDecision, QuotaAdjustmentReceipt, QuotaAdjustmentRecord, QuotaAdjustmentRequest,
    QuotaResetReceipt, QuotaState,
};

use filesystem::{
    acquire_writer_lock, owner_file, prepare_directory, read_bounded_ledger,
    reject_symlink_if_present, sync_directory,
};
use validate::{
    payload_bytes, validate_new_write, validate_store_policy, validate_stored_record,
    validate_transition,
};

const LEDGER_FILE: &str = "episodes.v1.jsonl";
const LOCK_FILE: &str = "episodes.v1.lock";
const RETENTION_FILE: &str = "episodes.v1.retention.jsonl";
const REDACTION_FILE: &str = "episodes.v1.redaction.jsonl";
const OBJECT_KEY_FILE: &str = "episodes.v1.object_keys.jsonl";
const RECLAIMED_FILE: &str = "episodes.v1.reclaimed.jsonl";
const HOLDS_FILE: &str = "episodes.v1.holds.jsonl";
const REVOCATION_FILE: &str = "episodes.v1.revocations.jsonl";
const QUOTA_FILE: &str = "episodes.v1.quotas.jsonl";
const MAX_LEDGER_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECORD_BYTES: usize = 1024 * 1024;
const MAX_TOKEN_COST: u64 = 1_000_000;
const MAX_RECEIPTS: usize = 2_048;

/// Closed durable-episode failures. No raw event content is included.
#[derive(Debug, Error)]
pub enum EpisodeStoreError {
    /// The store path or a ledger operation failed.
    #[error("episode_store_io")]
    Io,
    /// Another writer owns the ledger.
    #[error("episode_store_writer_busy")]
    WriterBusy,
    /// The ledger contains an invalid or mutated complete record.
    #[error("episode_store_corrupt")]
    Corrupt,
    /// A canonical value was outside the deterministic profile.
    #[error("episode_commitment_invalid")]
    Canonical(#[from] CanonicalCborError),
    /// A bounded identifier, collection, counter, or policy is invalid.
    #[error("episode_input_invalid")]
    InvalidInput,
    /// Contract revision, digest, guild, policy, or consent epoch is stale.
    #[error("episode_binding_stale")]
    StaleBinding,
    /// Guild learning and durable writes are not opted in.
    #[error("episode_learning_disabled")]
    LearningDisabled,
    /// `QUIET` suppresses response, learning, and the durable write.
    #[error("episode_quiet")]
    Quiet,
    /// The request or operation identifier was replayed.
    #[error("episode_replay")]
    Replay,
    /// The requested lifecycle transition or authority identity is invalid.
    #[error("episode_transition_invalid")]
    InvalidTransition,
    /// The caller-supplied commitment prediction does not match WDBX.
    #[error("episode_commitment_mismatch")]
    CommitmentMismatch,
    /// The guild token budget is exhausted.
    #[error("episode_token_budget_exhausted")]
    TokenBudget,
    /// The guild storage budget or whole-ledger size ceiling is exhausted.
    #[error("episode_storage_budget_exhausted")]
    StorageBudget,
}

/// Durable, single-writer, append-only episode store.
///
/// Appends synchronize ledger data. Unix initialization also synchronizes
/// directory entries; other platforms retain file-only synchronization and
/// provide no directory-entry crash-durability guarantee.
#[derive(Debug)]
pub struct EpisodeStore {
    _writer_lock: File,
    ledger: File,
    /// The directory the store owns, for the ledger rewrite of a
    /// garbage-collection round.
    directory: PathBuf,
    policy: StorePolicy,
    records: Vec<StoredRecord>,
    state: LedgerState,
    poisoned: bool,
    signer: Option<EpisodeSigner>,
    /// The committed retention deadlines of the TTL-bound candidates.
    retention: expiry::RetentionIndex,
    /// The committed redaction requests for derived copies.
    redaction: deletion::RedactionIndex,
    /// The committed object-key destruction requests.
    object_keys: deletion::ObjectKeyIndex,
    /// The candidates the store has reclaimed from the ledger.
    reclaimed: gc::ReclaimedIndex,
    /// The committed holds on memory candidates.
    holds: holds::HoldIndex,
    /// The committed candidate revocations.
    revocations: holds::RevocationIndex,
    /// The adjusted budgets of the guilds whose quota was administered.
    quotas: quota::QuotaIndex,
    /// The principals that some guild's authority map names.
    ///
    /// A named principal is scoped to the guilds that name it, so the store
    /// derives the set from the policy when it opens. See
    /// [`authority::scoped_principals`].
    scoped_principals: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct StoredRecord {
    sequence: u64,
    request_id: String,
    operation_id: String,
    contract_revision: u64,
    contract_digest: [u8; 32],
    guild_ref: String,
    consent_epoch: Option<u64>,
    source_type: EpisodeSource,
    policy_version: String,
    evidence_level: super::types::EvidenceLevel,
    event: EpisodeEvent,
    token_cost: u64,
    previous_digest: Option<[u8; 32]>,
    episode_digest: [u8; 32],
    /// Detached writer signature over `episode_digest`. Kept outside the
    /// committed envelope and omitted when absent, so an unsigned record
    /// serializes byte-for-byte as it did before signing existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    signature: Option<EpisodeSignature>,
}

#[derive(Debug, Default)]
struct LedgerState {
    operations: BTreeMap<String, OperationState>,
    request_ids: BTreeSet<String>,
    guild_usage: BTreeMap<String, GuildUsage>,
    memories: BTreeMap<String, GuildMemories>,
}

/// Memory-candidate edges per guild.
///
/// Rebuilt on open from the ledger, so `supersedes`/`forgets` references and
/// memory-edge rules are checked identically on the live and the replay path.
#[derive(Clone, Debug, Default)]
struct GuildMemories {
    /// Admitted candidate digests, with each candidate's `member_scoped`.
    admitted: BTreeMap<[u8; 32], bool>,
    /// The retention class each admitted candidate was admitted with.
    retention: BTreeMap<[u8; 32], RetentionClass>,
    /// The committed retention deadline of each TTL-bound candidate.
    expires_at: BTreeMap<[u8; 32], u64>,
    /// Zero-payload forgetting records are durable tombstones, not live memories.
    tombstones: BTreeSet<[u8; 32]>,
    forgotten: BTreeSet<[u8; 32]>,
    /// Open quarantine per candidate: target digest to edge digest.
    quarantined: BTreeMap<[u8; 32], [u8; 32]>,
    /// Open contradiction per ordered candidate pair, to edge digest.
    contradictions: BTreeMap<([u8; 32], [u8; 32]), [u8; 32]>,
    /// Open `quarantines`/`contradicts` edge episodes and what each holds.
    open_edges: BTreeMap<[u8; 32], OpenEdge>,
    /// Superseded candidate to the candidate that supersedes it.
    ///
    /// Two maps, because the two directions are not symmetric. A candidate
    /// names at most one `supersedes` target, so `supersedes` (candidate to
    /// target) is exact. Two corrections of one memory are two edges, so
    /// several candidates may supersede the same target and `superseded_by`
    /// (target to candidate) keeps the most recent one. Both are rebuilt from
    /// the ledger on open, so the live and the replay path answer alike.
    superseded_by: BTreeMap<[u8; 32], [u8; 32]>,
    /// Superseding candidate to the candidate it supersedes.
    supersedes: BTreeMap<[u8; 32], [u8; 32]>,
    /// The candidates this guild revoked.
    ///
    /// A revoked candidate is withdrawn by authority: it is not live, no
    /// later `supersedes` may name it, and it is never resurrected. The set
    /// is not rebuilt from the ledger — a revocation is not an episode — but
    /// from the store's revocation index, after the ledger has been replayed.
    revoked: BTreeSet<[u8; 32]>,
}

#[derive(Clone, Copy, Debug)]
enum OpenEdge {
    Quarantine([u8; 32]),
    Contradiction([u8; 32], [u8; 32]),
}

impl GuildMemories {
    fn is_live(&self, digest: &[u8; 32]) -> bool {
        self.admitted.contains_key(digest)
            && !self.tombstones.contains(digest)
            && !self.forgotten.contains(digest)
            && !self.revoked.contains(digest)
    }
}

#[derive(Clone, Debug)]
struct OperationState {
    contract_revision: u64,
    contract_digest: [u8; 32],
    guild_ref: String,
    consent_epoch: Option<u64>,
    source_type: EpisodeSource,
    policy_version: String,
    evidence_level: super::types::EvidenceLevel,
    requested_by: ActorRef,
    proposed_by: ActorRef,
    approved_by: Option<ActorRef>,
    stage: Stage,
    last_digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Proposed,
    Approved,
    Executed,
    /// Compensation was attempted; the flag records exact restoration.
    Compensated(bool),
    Terminal,
}

#[derive(Clone, Copy, Debug, Default)]
struct GuildUsage {
    tokens: u64,
    bytes: u64,
}

impl EpisodeStore {
    /// Open and replay-verify a store, truncating only an incomplete final line.
    ///
    /// Writer-lock contention is retried for up to 50 ms to tolerate transient
    /// inherited descriptors; persistent contention returns `WriterBusy`.
    ///
    /// A store opened this way appends unsigned records. Existing signed
    /// records remain readable; use [`Self::signature_status`] to check them.
    pub fn open(
        directory: impl AsRef<Path>,
        policy: StorePolicy,
    ) -> Result<Self, EpisodeStoreError> {
        Self::open_inner(directory.as_ref(), policy, None)
    }

    /// Open a store whose canonical writer signs every record it appends.
    ///
    /// Replay additionally verifies every existing record that claims this
    /// signer's key id; one that fails is corruption, exactly as a mutated
    /// digest is. Records signed under other keys, and unsigned records, are
    /// accepted and left for [`Self::signature_status`] to report.
    ///
    /// Note that a ledger containing signed records cannot be opened by a
    /// build that predates episode signing: that build's record decoder
    /// rejects the unknown `signature` member.
    pub fn open_with_signer(
        directory: impl AsRef<Path>,
        policy: StorePolicy,
        signer: EpisodeSigner,
    ) -> Result<Self, EpisodeStoreError> {
        Self::open_inner(directory.as_ref(), policy, Some(signer))
    }

    fn open_inner(
        directory: &Path,
        policy: StorePolicy,
        signer: Option<EpisodeSigner>,
    ) -> Result<Self, EpisodeStoreError> {
        validate_store_policy(&policy)?;
        let directory = prepare_directory(directory)?;
        let lock_path = directory.join(LOCK_FILE);
        let ledger_path = directory.join(LEDGER_FILE);
        let retention_path = directory.join(RETENTION_FILE);
        let redaction_path = directory.join(REDACTION_FILE);
        let object_key_path = directory.join(OBJECT_KEY_FILE);
        let reclaimed_path = directory.join(RECLAIMED_FILE);
        let holds_path = directory.join(HOLDS_FILE);
        let revocation_path = directory.join(REVOCATION_FILE);
        let quota_path = directory.join(QUOTA_FILE);
        reject_symlink_if_present(&lock_path)?;
        reject_symlink_if_present(&ledger_path)?;
        reject_symlink_if_present(&retention_path)?;
        reject_symlink_if_present(&redaction_path)?;
        reject_symlink_if_present(&object_key_path)?;
        reject_symlink_if_present(&reclaimed_path)?;
        reject_symlink_if_present(&holds_path)?;
        reject_symlink_if_present(&revocation_path)?;
        reject_symlink_if_present(&quota_path)?;

        let writer_lock = owner_file(&lock_path)?;
        acquire_writer_lock(&writer_lock)?;
        let raw = read_bounded_ledger(&ledger_path)?;
        let (records, valid_bytes) = decode_complete_records(&raw)?;
        // The retention index is read before the ledger is replayed: a
        // candidate's deadline is part of the state the replay rebuilds.
        let retention = expiry::RetentionIndex::open(&retention_path)?;
        // The deletion-lifecycle indexes are read for the same reason: the
        // requests and the reclamations are part of the state the store
        // answers from.
        let redaction = deletion::RedactionIndex::open(&redaction_path)?;
        let object_keys = deletion::ObjectKeyIndex::open(&object_key_path)?;
        let reclaimed = gc::ReclaimedIndex::open(&reclaimed_path)?;
        // The holds and the revocations are read for the same reason: they are
        // decisions the store answers from, not episodes it replays.
        let holds = holds::HoldIndex::open(&holds_path)?;
        let revocations = holds::RevocationIndex::open(&revocation_path)?;
        // The quota index is read for the same reason: the adjusted budgets
        // are part of the state the write gate judges a write against.
        let mut quotas = quota::QuotaIndex::open(&quota_path)?;
        // The principals some guild's authority map names are scoped to the
        // guilds that name them. The set is derived from the policy, so the
        // store's authority answers never depend on a clock or on the
        // ledger.
        let scoped_principals = authority::scoped_principals(&policy);

        let mut state = LedgerState::default();
        // A reclaimed candidate has no record of its own any more, but the
        // ledger's tombstones still name it. The index restores the
        // admission the tombstone replays against.
        reclaimed.inject(&mut state, &retention);
        let mut offset = 0_usize;
        for (record, line) in records
            .iter()
            .zip(raw.split_inclusive(|byte| *byte == b'\n'))
        {
            // Transport JSON need not use our serializer's whitespace or escape
            // spelling. Charge the bytes actually persisted, including newline.
            let line_bytes = line.len();
            validate_stored_record(record, &state, offset)?;
            if let Some(signer) = &signer {
                validate_own_signature(record, signer)?;
            }
            apply_record(record, line_bytes, &mut state, &retention)?;
            offset = offset.saturating_add(line_bytes);
        }
        // A reclaimed candidate must never be live again.
        reclaimed.verify(&state)?;
        // A revoked candidate is not live either. The revocations are applied
        // after the replay, never before it: a ledger written before the
        // revocation must still replay, and the replay's own liveness checks
        // are about the ledger's consistency, not about a later decision.
        revocations.inject(&mut state);
        // A host policy lowered below the recorded usage leaves the store
        // over its own budget. The store surfaces that state instead of
        // refusing to open, which would brick the store.
        quotas.detect_over_budget(&policy, &state.guild_usage);
        // A failed replay must not repair the file. Only a fully verified
        // prefix permits discarding an incomplete final line.
        if valid_bytes != raw.len() {
            let repair = OpenOptions::new()
                .write(true)
                .open(&ledger_path)
                .map_err(|_| EpisodeStoreError::Io)?;
            repair
                .set_len(u64::try_from(valid_bytes).map_err(|_| EpisodeStoreError::Io)?)
                .map_err(|_| EpisodeStoreError::Io)?;
            repair.sync_data().map_err(|_| EpisodeStoreError::Io)?;
        }
        let ledger = owner_file(&ledger_path)?;
        // On Unix, fence the ledger name (including retries after failed sync)
        // before writes. Other platforms retain file-only synchronization.
        sync_directory(&directory)?;
        Ok(Self {
            _writer_lock: writer_lock,
            ledger,
            directory,
            policy,
            records,
            state,
            poisoned: false,
            signer,
            retention,
            redaction,
            object_keys,
            reclaimed,
            holds,
            revocations,
            quotas,
            scoped_principals,
        })
    }

    /// Identifier of the key this store signs new records with, if any.
    #[must_use]
    pub fn signer_key_id(&self) -> Option<&SignerKeyId> {
        self.signer.as_ref().map(EpisodeSigner::key_id)
    }

    /// Compute the exact commitment WDBX would append without mutating the store.
    ///
    /// The preview runs the same write gate as [`Self::propose_write`],
    /// including the budget checks and the retention index's own size
    /// ceiling, so a write this refuses is one the store would not append.
    /// It still appends nothing and consumes no request identifier.
    pub fn preview_commitment(&self, write: &EpisodeWrite) -> Result<[u8; 32], EpisodeStoreError> {
        if self.poisoned {
            return Err(EpisodeStoreError::Io);
        }
        let mut record = self.prepare_record(write)?;
        // The appended line carries the signature, so the preview measures
        // the same line the append path writes. The signature is detached
        // and never moves the commitment.
        self.sign_record(&mut record);
        let line = serialized_line(&record)?;
        let line_bytes = u64::try_from(line.len()).map_err(|_| EpisodeStoreError::InvalidInput)?;
        self.check_quotas(&record, line_bytes)?;
        Ok(record.episode_digest)
    }

    /// Validate, canonicalize, hash, durably append, and return a sanitized receipt.
    pub fn propose_write(
        &mut self,
        write: &EpisodeWrite,
    ) -> Result<EpisodeReceipt, EpisodeStoreError> {
        if self.poisoned {
            return Err(EpisodeStoreError::Io);
        }
        let mut record = self.prepare_record(write)?;
        self.sign_record(&mut record);
        if write
            .expected_commitment
            .is_some_and(|expected| expected != record.episode_digest)
        {
            return Err(EpisodeStoreError::CommitmentMismatch);
        }
        let line = serialized_line(&record)?;
        let line_bytes = u64::try_from(line.len()).map_err(|_| EpisodeStoreError::InvalidInput)?;
        self.check_quotas(&record, line_bytes)?;
        // The retention deadline is committed before the ledger line it
        // describes, so a candidate that reaches the ledger always carries
        // the deadline its TTL bound. A failure here fails the append.
        self.bind_retention(&record)?;

        if self.ledger.write_all(&line).is_err() || self.ledger.sync_data().is_err() {
            self.poisoned = true;
            return Err(EpisodeStoreError::Io);
        }
        apply_record(&record, line.len(), &mut self.state, &self.retention)?;
        let receipt = receipt(&record);
        self.records.push(record);
        Ok(receipt)
    }

    /// Return at most 2,048 sanitized receipts for one guild in ledger order.
    pub fn retrieve(
        &self,
        guild_ref: &str,
        limit: usize,
    ) -> Result<Vec<EpisodeReceipt>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) || !(1..=MAX_RECEIPTS).contains(&limit) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        Ok(self
            .records
            .iter()
            .filter(|record| record.guild_ref == guild_ref)
            .take(limit)
            .map(receipt)
            .collect())
    }

    /// Find the receipt for one commitment anywhere in a guild's ledger.
    ///
    /// Unlike [`Self::retrieve`], this is not windowed: it scans every record
    /// the store holds, so verification does not silently stop at the first
    /// [`MAX_RECEIPTS`] receipts of a busy guild. A digest that was never
    /// appended, including the all-zero digest, is simply `Ok(None)`.
    pub fn find_receipt(
        &self,
        guild_ref: &str,
        episode_digest: &[u8; 32],
    ) -> Result<Option<EpisodeReceipt>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        Ok(self
            .records
            .iter()
            .find(|record| {
                record.guild_ref == guild_ref && record.episode_digest == *episode_digest
            })
            .map(receipt))
    }

    /// Report the signature state of one appended episode.
    ///
    /// `Ok(None)` means the digest was never appended for this guild.
    /// `resolve` maps a signer key id to its verifying key; returning `None`
    /// yields [`SignatureStatus::UnknownKey`], never `Invalid`.
    pub fn signature_status(
        &self,
        guild_ref: &str,
        episode_digest: &[u8; 32],
        resolve: impl Fn(&SignerKeyId) -> Option<ed25519_dalek::VerifyingKey>,
    ) -> Result<Option<SignatureStatus>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        Ok(self
            .records
            .iter()
            .find(|record| {
                record.guild_ref == guild_ref && record.episode_digest == *episode_digest
            })
            .map(|record| {
                check_signature(&record.episode_digest, record.signature.as_ref(), &resolve)
            }))
    }

    /// Return cumulative token and byte usage for a guild.
    ///
    /// Bytes count every persisted ledger line (including whitespace and its
    /// newline) plus the `payload_bytes` of every admitted memory candidate,
    /// so a guild's memory footprint is bounded by the same policy that bounds
    /// its ledger. Incomplete final lines discarded during recovery do not count.
    #[must_use]
    pub fn guild_usage(&self, guild_ref: &str) -> Option<(u64, u64)> {
        self.state
            .guild_usage
            .get(guild_ref)
            .map(|usage| (usage.tokens, usage.bytes))
    }

    /// Report the open memory-edge state of one admitted memory candidate.
    ///
    /// `Ok(None)` means the digest is not a memory candidate admitted in this
    /// guild. Quarantine and contradiction never hide a record; this read is
    /// how a retrieval layer learns about them.
    pub fn memory_edge_state(
        &self,
        guild_ref: &str,
        candidate_digest: &[u8; 32],
    ) -> Result<Option<MemoryEdgeState>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        let Some(memories) = self.state.memories.get(guild_ref) else {
            return Ok(None);
        };
        if !memories.admitted.contains_key(candidate_digest) {
            return Ok(None);
        }
        let mut open_contradictions: Vec<OpenContradiction> = memories
            .contradictions
            .iter()
            .filter_map(|(&(low, high), &edge)| {
                if low == *candidate_digest {
                    Some(OpenContradiction {
                        counterpart: high,
                        edge,
                    })
                } else if high == *candidate_digest {
                    Some(OpenContradiction {
                        counterpart: low,
                        edge,
                    })
                } else {
                    None
                }
            })
            .collect();
        open_contradictions.sort_by_key(|item| item.counterpart);
        Ok(Some(MemoryEdgeState {
            forgotten: memories.forgotten.contains(candidate_digest),
            open_quarantine: memories.quarantined.get(candidate_digest).copied(),
            open_contradictions,
        }))
    }

    /// Report whether a `quarantines`/`contradicts` edge episode is still open.
    ///
    /// `Ok(None)` means the digest is not such an edge in this guild,
    /// `Some(true)` that it is open, and `Some(false)` that a `resolves` edge
    /// has closed it.
    pub fn memory_edge_open(
        &self,
        guild_ref: &str,
        edge_digest: &[u8; 32],
    ) -> Result<Option<bool>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        if self
            .state
            .memories
            .get(guild_ref)
            .is_some_and(|memories| memories.open_edges.contains_key(edge_digest))
        {
            return Ok(Some(true));
        }
        let closable = self.records.iter().any(|record| {
            record.guild_ref == guild_ref
                && record.episode_digest == *edge_digest
                && matches!(
                    &record.event,
                    EpisodeEvent::MemoryEdge { edge, .. } if edge.kind != MemoryEdgeKind::Resolves
                )
        });
        Ok(closable.then_some(false))
    }

    /// Return the canonical resolution digest for a quarantine or contradiction.
    ///
    /// `Ok(None)` means the edge is open, unknown, belongs to another guild, or
    /// is not closable. Each edge can be resolved only once; later edges on the
    /// same candidate have their own resolution. Records are validated on
    /// append and replay, so this query preserves the relationship after reopen.
    pub fn memory_edge_resolution(
        &self,
        guild_ref: &str,
        edge_digest: &[u8; 32],
    ) -> Result<Option<[u8; 32]>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        Ok(self.records.iter().find_map(|record| {
            if record.guild_ref != guild_ref {
                return None;
            }
            match &record.event {
                EpisodeEvent::MemoryEdge { edge, .. }
                    if edge.kind == MemoryEdgeKind::Resolves && edge.target == *edge_digest =>
                {
                    Some(record.episode_digest)
                }
                _ => None,
            }
        }))
    }

    fn prepare_record(&self, write: &EpisodeWrite) -> Result<StoredRecord, EpisodeStoreError> {
        validate_new_write(write, &self.policy, &self.state)?;
        let sequence = u64::try_from(self.records.len())
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(EpisodeStoreError::InvalidInput)?;
        let previous_digest = self
            .state
            .operations
            .get(&write.operation_id)
            .map(|operation| operation.last_digest);
        let mut record = StoredRecord {
            sequence,
            request_id: write.request_id.clone(),
            operation_id: write.operation_id.clone(),
            contract_revision: write.contract_revision,
            contract_digest: write.contract_digest,
            guild_ref: write.guild_ref.clone(),
            consent_epoch: write.consent_epoch,
            source_type: write.source_type,
            policy_version: write.policy_version.clone(),
            evidence_level: write.evidence_level,
            event: write.event.clone(),
            token_cost: write.token_cost,
            previous_digest,
            episode_digest: [0; 32],
            signature: None,
        };
        record.episode_digest = record.computed_digest()?;
        validate_transition(&record, &self.state)?;
        // A held candidate is not silently superseded: the correction waits
        // for the explicit resolution that releases its hold.
        self.refuse_held_target(&record)?;
        Ok(record)
    }

    /// Bind the retention deadline of one record, if its class has one.
    ///
    /// The deadline is the candidate's arrival plus the guild policy's TTL,
    /// and it is bound once, here, at admission: a later policy change never
    /// rewrites the deadline of a record the ledger already holds. A
    /// `durable` candidate and a `forgets` tombstone never get one.
    fn bind_retention(&mut self, record: &StoredRecord) -> Result<(), EpisodeStoreError> {
        let Some(expires_at) = self.retention_deadline(record) else {
            return Ok(());
        };
        self.retention.commit(record.episode_digest, expires_at)
    }

    /// The retention deadline one record would be bound, if any.
    ///
    /// The deadline is the candidate's arrival plus the window the *policy*
    /// gives its retention class — the explicit per-class window when the
    /// policy names one, and the guild's single time-to-live otherwise — read
    /// from the store's own clock at this instant. It is `None` for a
    /// `durable` candidate, a `forgets` tombstone, a guild with no
    /// applicable window, and an unreadable clock.
    ///
    /// The write gate reads it too, so that the decision, the preview and
    /// the append all agree on whether this write adds a line to the
    /// retention index.
    fn retention_deadline(&self, record: &StoredRecord) -> Option<u64> {
        let EpisodeEvent::MemoryCandidate { candidate, .. } = &record.event else {
            return None;
        };
        let guild = self.policy.guilds.get(&record.guild_ref)?;
        expiry::bound_deadline(candidate, guild, expiry::now_ms())
    }

    /// The signature the store would put on one record, if it signs at all.
    ///
    /// The signature is detached: it is not covered by the episode digest,
    /// so signing a record never moves its commitment. The write gate signs
    /// the record it checks for exactly this reason: the byte budget must
    /// be charged for the same line the append path writes.
    fn sign_record(&self, record: &mut StoredRecord) {
        record.signature = self
            .signer
            .as_ref()
            .map(|signer| signer.sign_digest(&record.episode_digest));
    }
}

fn apply_record(
    record: &StoredRecord,
    line_bytes: usize,
    state: &mut LedgerState,
    retention: &expiry::RetentionIndex,
) -> Result<(), EpisodeStoreError> {
    state.request_ids.insert(record.request_id.clone());
    let usage = state
        .guild_usage
        .entry(record.guild_ref.clone())
        .or_default();
    usage.tokens = usage.tokens.saturating_add(record.token_cost);
    usage.bytes = usage
        .bytes
        .saturating_add(u64::try_from(line_bytes).map_err(|_| EpisodeStoreError::Corrupt)?)
        .saturating_add(payload_bytes(&record.event));
    let open_operation =
        |requested_by: &ActorRef, proposed_by: &ActorRef, stage: Stage| OperationState {
            contract_revision: record.contract_revision,
            contract_digest: record.contract_digest,
            guild_ref: record.guild_ref.clone(),
            consent_epoch: record.consent_epoch,
            source_type: record.source_type,
            policy_version: record.policy_version.clone(),
            evidence_level: record.evidence_level,
            requested_by: requested_by.clone(),
            proposed_by: proposed_by.clone(),
            approved_by: None,
            stage,
            last_digest: record.episode_digest,
        };
    match &record.event {
        EpisodeEvent::Proposal {
            requested_by,
            proposed_by,
        } => {
            state.operations.insert(
                record.operation_id.clone(),
                open_operation(requested_by, proposed_by, Stage::Proposed),
            );
        }
        EpisodeEvent::MemoryCandidate {
            recorded_by,
            candidate,
        } => {
            state.operations.insert(
                record.operation_id.clone(),
                open_operation(recorded_by, recorded_by, Stage::Terminal),
            );
            let memories = state.memories.entry(record.guild_ref.clone()).or_default();
            memories
                .admitted
                .insert(record.episode_digest, candidate.member_scoped);
            memories
                .retention
                .insert(record.episode_digest, candidate.retention);
            // The deadline was committed before the record reached the
            // ledger, so the live path and the replay path read the same one.
            if let Some(deadline) = retention.deadline(&record.episode_digest) {
                memories.expires_at.insert(record.episode_digest, deadline);
            }
            if let Some(forgotten) = candidate.forgets {
                memories.tombstones.insert(record.episode_digest);
                memories.forgotten.insert(forgotten);
            }
            if let Some(superseded) = candidate.supersedes {
                // The target is an admitted candidate of this guild: the write
                // gate and the replay check both refuse any other.
                memories
                    .superseded_by
                    .insert(superseded, record.episode_digest);
                memories
                    .supersedes
                    .insert(record.episode_digest, superseded);
            }
        }
        EpisodeEvent::MemoryEdge { recorded_by, edge } => {
            state.operations.insert(
                record.operation_id.clone(),
                open_operation(recorded_by, recorded_by, Stage::Terminal),
            );
            let memories = state.memories.entry(record.guild_ref.clone()).or_default();
            apply_memory_edge(memories, edge, record.episode_digest)?;
        }
        event => {
            let operation = state
                .operations
                .get_mut(&record.operation_id)
                .ok_or(EpisodeStoreError::Corrupt)?;
            operation.stage = match event {
                EpisodeEvent::Approval { approved_by } => {
                    operation.approved_by = Some(approved_by.clone());
                    Stage::Approved
                }
                EpisodeEvent::Execution { .. } => Stage::Executed,
                EpisodeEvent::Compensation {
                    exact_restore_observed,
                    ..
                } => Stage::Compensated(*exact_restore_observed),
                EpisodeEvent::Terminal { .. } => Stage::Terminal,
                EpisodeEvent::Proposal { .. }
                | EpisodeEvent::MemoryCandidate { .. }
                | EpisodeEvent::MemoryEdge { .. } => {
                    return Err(EpisodeStoreError::Corrupt);
                }
            };
            operation.last_digest = record.episode_digest;
        }
    }
    Ok(())
}

fn apply_memory_edge(
    memories: &mut GuildMemories,
    edge: &MemoryEdge,
    edge_digest: [u8; 32],
) -> Result<(), EpisodeStoreError> {
    match (edge.kind, edge.counterpart) {
        (MemoryEdgeKind::Quarantines, None) => {
            memories.quarantined.insert(edge.target, edge_digest);
            memories
                .open_edges
                .insert(edge_digest, OpenEdge::Quarantine(edge.target));
        }
        (MemoryEdgeKind::Contradicts, Some(counterpart)) => {
            memories
                .contradictions
                .insert((edge.target, counterpart), edge_digest);
            memories.open_edges.insert(
                edge_digest,
                OpenEdge::Contradiction(edge.target, counterpart),
            );
        }
        (MemoryEdgeKind::Resolves, None) => {
            match memories
                .open_edges
                .remove(&edge.target)
                .ok_or(EpisodeStoreError::Corrupt)?
            {
                OpenEdge::Quarantine(target) => {
                    memories.quarantined.remove(&target);
                }
                OpenEdge::Contradiction(low, high) => {
                    memories.contradictions.remove(&(low, high));
                }
            }
        }
        _ => return Err(EpisodeStoreError::Corrupt),
    }
    Ok(())
}

/// Replay check for records claiming the opening writer's own key.
fn validate_own_signature(
    record: &StoredRecord,
    signer: &EpisodeSigner,
) -> Result<(), EpisodeStoreError> {
    let Some(signature) = &record.signature else {
        return Ok(());
    };
    if &signature.signer_key_id != signer.key_id() {
        return Ok(());
    }
    let key = signer.verifying_key();
    match check_signature(&record.episode_digest, Some(signature), |_| Some(key)) {
        SignatureStatus::Valid(_) => Ok(()),
        _ => Err(EpisodeStoreError::Corrupt),
    }
}

fn receipt(record: &StoredRecord) -> EpisodeReceipt {
    EpisodeReceipt {
        sequence: record.sequence,
        request_id: record.request_id.clone(),
        operation_id: record.operation_id.clone(),
        guild_ref: record.guild_ref.clone(),
        episode_digest: record.episode_digest,
        previous_digest: record.previous_digest,
        event_kind: record.event.label().into(),
        policy_version: record.policy_version.clone(),
        evidence_level: record.evidence_level,
        terminal_status: match record.event {
            EpisodeEvent::Terminal { status, .. } => Some(status),
            EpisodeEvent::MemoryCandidate { .. } | EpisodeEvent::MemoryEdge { .. } => {
                Some(TerminalStatus::Completed)
            }
            _ => None,
        },
        redacted: true,
    }
}

fn valid_actor(actor: &ActorRef) -> bool {
    bounded_identifier(&actor.principal_id, 64)
}

fn bounded_identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
        })
}

fn serialized_line(record: &StoredRecord) -> Result<Vec<u8>, EpisodeStoreError> {
    let mut line = serde_json::to_vec(record).map_err(|_| EpisodeStoreError::Corrupt)?;
    line.push(b'\n');
    if line.len() > MAX_RECORD_BYTES {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(line)
}

fn decode_complete_records(raw: &[u8]) -> Result<(Vec<StoredRecord>, usize), EpisodeStoreError> {
    let mut records = Vec::new();
    let mut start = 0_usize;
    while let Some(relative_end) = raw[start..].iter().position(|byte| *byte == b'\n') {
        let end = start + relative_end;
        let line = &raw[start..end];
        if line.is_empty() || line.len() + 1 > MAX_RECORD_BYTES {
            return Err(EpisodeStoreError::Corrupt);
        }
        let record = serde_json::from_slice(line).map_err(|_| EpisodeStoreError::Corrupt)?;
        records.push(record);
        start = end + 1;
    }
    Ok((records, start))
}
