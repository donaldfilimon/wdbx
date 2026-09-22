//! Durable append-only episode ledger and write gate.

use super::signing::{
    EpisodeSignature, EpisodeSigner, SignatureStatus, SignerKeyId, check_signature,
};
use super::types::{
    ActorRef, EpisodeEvent, EpisodeReceipt, EpisodeSource, EpisodeWrite, MemoryEdge,
    MemoryEdgeKind, MemoryEdgeState, OpenContradiction, StorePolicy, TerminalStatus,
};
use crate::v3::commitment::CanonicalCborError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

mod canonical;
mod validate;

use validate::{
    payload_bytes, validate_new_write, validate_store_policy, validate_stored_record,
    validate_transition,
};

const LEDGER_FILE: &str = "episodes.v1.jsonl";
const LOCK_FILE: &str = "episodes.v1.lock";
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
    /// The guild storage budget is exhausted.
    #[error("episode_storage_budget_exhausted")]
    StorageBudget,
}

/// Durable, single-writer, append-only episode store.
#[derive(Debug)]
pub struct EpisodeStore {
    _writer_lock: File,
    ledger: File,
    policy: StorePolicy,
    records: Vec<StoredRecord>,
    state: LedgerState,
    poisoned: bool,
    signer: Option<EpisodeSigner>,
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
    forgotten: BTreeSet<[u8; 32]>,
    /// Open quarantine per candidate: target digest to edge digest.
    quarantined: BTreeMap<[u8; 32], [u8; 32]>,
    /// Open contradiction per ordered candidate pair, to edge digest.
    contradictions: BTreeMap<([u8; 32], [u8; 32]), [u8; 32]>,
    /// Open `quarantines`/`contradicts` edge episodes and what each holds.
    open_edges: BTreeMap<[u8; 32], OpenEdge>,
}

#[derive(Clone, Copy, Debug)]
enum OpenEdge {
    Quarantine([u8; 32]),
    Contradiction([u8; 32], [u8; 32]),
}

impl GuildMemories {
    fn is_live(&self, digest: &[u8; 32]) -> bool {
        self.admitted.contains_key(digest) && !self.forgotten.contains(digest)
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
    Compensated,
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
        reject_symlink_if_present(&lock_path)?;
        reject_symlink_if_present(&ledger_path)?;

        let writer_lock = owner_file(&lock_path)?;
        match writer_lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(EpisodeStoreError::WriterBusy),
            Err(TryLockError::Error(_)) => return Err(EpisodeStoreError::Io),
        }
        let raw = read_bounded_ledger(&ledger_path)?;
        let (records, valid_bytes) = decode_complete_records(&raw)?;
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

        let mut state = LedgerState::default();
        let mut offset = 0_usize;
        for record in &records {
            let line_bytes = serialized_line(record)?.len();
            validate_stored_record(record, &state, offset)?;
            if let Some(signer) = &signer {
                validate_own_signature(record, signer)?;
            }
            apply_record(record, line_bytes, &mut state)?;
            offset = offset.saturating_add(line_bytes);
        }
        let ledger = owner_file(&ledger_path)?;
        Ok(Self {
            _writer_lock: writer_lock,
            ledger,
            policy,
            records,
            state,
            poisoned: false,
            signer,
        })
    }

    /// Identifier of the key this store signs new records with, if any.
    #[must_use]
    pub fn signer_key_id(&self) -> Option<&SignerKeyId> {
        self.signer.as_ref().map(EpisodeSigner::key_id)
    }

    /// Compute the exact commitment WDBX would append without mutating the store.
    pub fn preview_commitment(&self, write: &EpisodeWrite) -> Result<[u8; 32], EpisodeStoreError> {
        if self.poisoned {
            return Err(EpisodeStoreError::Io);
        }
        let record = self.prepare_record(write)?;
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
        record.signature = self
            .signer
            .as_ref()
            .map(|signer| signer.sign_digest(&record.episode_digest));
        if write
            .expected_commitment
            .is_some_and(|expected| expected != record.episode_digest)
        {
            return Err(EpisodeStoreError::CommitmentMismatch);
        }
        let line = serialized_line(&record)?;
        let guild_policy = self
            .policy
            .guilds
            .get(&record.guild_ref)
            .ok_or(EpisodeStoreError::StaleBinding)?;
        let usage = self
            .state
            .guild_usage
            .get(&record.guild_ref)
            .copied()
            .unwrap_or_default();
        if usage.tokens.saturating_add(record.token_cost) > guild_policy.token_budget {
            return Err(EpisodeStoreError::TokenBudget);
        }
        let line_bytes = u64::try_from(line.len()).map_err(|_| EpisodeStoreError::InvalidInput)?;
        if usage
            .bytes
            .saturating_add(line_bytes)
            .saturating_add(payload_bytes(&record.event))
            > guild_policy.storage_budget_bytes
        {
            return Err(EpisodeStoreError::StorageBudget);
        }

        if self.ledger.write_all(&line).is_err() || self.ledger.sync_data().is_err() {
            self.poisoned = true;
            return Err(EpisodeStoreError::Io);
        }
        apply_record(&record, line.len(), &mut self.state)?;
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
    /// Bytes count every serialized ledger line plus the `payload_bytes` of
    /// every admitted memory candidate, so a guild's memory footprint is
    /// bounded by the same policy that bounds its ledger.
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
        Ok(record)
    }
}

fn apply_record(
    record: &StoredRecord,
    line_bytes: usize,
    state: &mut LedgerState,
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
            if let Some(forgotten) = candidate.forgets {
                memories.forgotten.insert(forgotten);
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
                EpisodeEvent::Compensation { .. } => Stage::Compensated,
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

fn prepare_directory(path: &Path) -> Result<PathBuf, EpisodeStoreError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(EpisodeStoreError::Io);
        }
    } else {
        fs::create_dir_all(path).map_err(|_| EpisodeStoreError::Io)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| EpisodeStoreError::Io)?;
    }
    Ok(path.to_path_buf())
}

fn reject_symlink_if_present(path: &Path) -> Result<(), EpisodeStoreError> {
    if let Ok(metadata) = fs::symlink_metadata(path)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(EpisodeStoreError::Io);
    }
    Ok(())
}

fn owner_file(path: &Path) -> Result<File, EpisodeStoreError> {
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .map_err(|_| EpisodeStoreError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| EpisodeStoreError::Io)?;
    }
    Ok(file)
}

fn read_bounded_ledger(path: &Path) -> Result<Vec<u8>, EpisodeStoreError> {
    if !path.exists() {
        let file = owner_file(path)?;
        file.sync_data().map_err(|_| EpisodeStoreError::Io)?;
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| EpisodeStoreError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > MAX_LEDGER_BYTES
    {
        return Err(EpisodeStoreError::Io);
    }
    fs::read(path).map_err(|_| EpisodeStoreError::Io)
}
