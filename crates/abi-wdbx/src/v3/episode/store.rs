//! Durable append-only episode ledger and write gate.

use super::types::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeReceipt, EpisodeSource, EpisodeWrite,
    GuildEpisodePolicy, MAX_VOICE_TRANSITIONS, StorePolicy, TerminalStatus, VoiceEvidence,
};
use crate::v3::commitment::{CanonicalCborError, CanonicalValue, EpisodeCommitment};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

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
}

#[derive(Debug, Default)]
struct LedgerState {
    operations: BTreeMap<String, OperationState>,
    request_ids: BTreeSet<String>,
    guild_usage: BTreeMap<String, GuildUsage>,
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
    pub fn open(
        directory: impl AsRef<Path>,
        policy: StorePolicy,
    ) -> Result<Self, EpisodeStoreError> {
        validate_store_policy(&policy)?;
        let directory = prepare_directory(directory.as_ref())?;
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
        })
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
        let record = self.prepare_record(write)?;
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
        if usage
            .bytes
            .saturating_add(u64::try_from(line.len()).map_err(|_| EpisodeStoreError::InvalidInput)?)
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

    /// Return cumulative token and byte usage for a guild.
    #[must_use]
    pub fn guild_usage(&self, guild_ref: &str) -> Option<(u64, u64)> {
        self.state
            .guild_usage
            .get(guild_ref)
            .map(|usage| (usage.tokens, usage.bytes))
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
        };
        record.episode_digest = record.computed_digest()?;
        validate_transition(&record, &self.state)?;
        Ok(record)
    }
}

impl StoredRecord {
    fn computed_digest(&self) -> Result<[u8; 32], EpisodeStoreError> {
        let header = CanonicalValue::Map(vec![
            text_entry("request_id", CanonicalValue::Text(self.request_id.clone())),
            text_entry(
                "operation_id",
                CanonicalValue::Text(self.operation_id.clone()),
            ),
            text_entry(
                "contract_revision",
                CanonicalValue::Unsigned(self.contract_revision),
            ),
            text_entry(
                "contract_digest",
                CanonicalValue::Bytes(self.contract_digest.to_vec()),
            ),
            text_entry("guild_ref", CanonicalValue::Text(self.guild_ref.clone())),
            text_entry(
                "consent_epoch",
                self.consent_epoch
                    .map_or(CanonicalValue::Null, CanonicalValue::Unsigned),
            ),
            text_entry(
                "source_type",
                CanonicalValue::Text(self.source_type.label().into()),
            ),
            text_entry(
                "policy_version",
                CanonicalValue::Text(self.policy_version.clone()),
            ),
            text_entry(
                "evidence_level",
                CanonicalValue::Text(self.evidence_level.label().into()),
            ),
        ]);
        let payload = CanonicalValue::Map(vec![
            text_entry(
                "event_kind",
                CanonicalValue::Text(self.event.label().into()),
            ),
            text_entry("event", canonical_event(&self.event)),
            text_entry("token_cost", CanonicalValue::Unsigned(self.token_cost)),
        ]);
        let parents = self.previous_digest.into_iter().collect();
        Ok(EpisodeCommitment::new(1, header, payload, parents).digest()?)
    }
}

fn validate_store_policy(policy: &StorePolicy) -> Result<(), EpisodeStoreError> {
    if policy.contract_revision == 0
        || policy.contract_digest == [0; 32]
        || policy.guilds.len() > MAX_RECEIPTS
        || policy.guilds.iter().any(|(guild, item)| {
            !bounded_identifier(guild, 128)
                || !bounded_identifier(&item.policy_version, 64)
                || item.token_budget > 1_000_000_000
                || item.storage_budget_bytes > MAX_LEDGER_BYTES
        })
    {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(())
}

fn validate_new_write(
    write: &EpisodeWrite,
    policy: &StorePolicy,
    state: &LedgerState,
) -> Result<(), EpisodeStoreError> {
    if write.quiet {
        return Err(EpisodeStoreError::Quiet);
    }
    if !bounded_identifier(&write.request_id, 64)
        || !bounded_identifier(&write.operation_id, 64)
        || !bounded_identifier(&write.guild_ref, 128)
        || !bounded_identifier(&write.policy_version, 64)
        || write.token_cost > MAX_TOKEN_COST
        || write.contract_revision == 0
        || write.contract_digest == [0; 32]
    {
        return Err(EpisodeStoreError::InvalidInput);
    }
    if state.request_ids.contains(&write.request_id) {
        return Err(EpisodeStoreError::Replay);
    }
    let guild = policy
        .guilds
        .get(&write.guild_ref)
        .ok_or(EpisodeStoreError::StaleBinding)?;
    if !guild.learning_enabled {
        return Err(EpisodeStoreError::LearningDisabled);
    }
    if write.contract_revision != policy.contract_revision
        || write.contract_digest != policy.contract_digest
        || write.policy_version != guild.policy_version
    {
        return Err(EpisodeStoreError::StaleBinding);
    }
    validate_voice_binding(write.source_type, write.consent_epoch, &write.event, guild)?;
    Ok(())
}

fn validate_voice_binding(
    source: EpisodeSource,
    consent_epoch: Option<u64>,
    event: &EpisodeEvent,
    policy: &GuildEpisodePolicy,
) -> Result<(), EpisodeStoreError> {
    let voice = match event {
        EpisodeEvent::Execution { voice, .. } => voice.as_ref(),
        _ => None,
    };
    if source == EpisodeSource::DiscordVoice {
        let epoch = consent_epoch
            .filter(|value| *value > 0)
            .ok_or(EpisodeStoreError::StaleBinding)?;
        if policy.current_consent_epoch != Some(epoch) {
            return Err(EpisodeStoreError::StaleBinding);
        }
        if let Some(evidence) = voice {
            validate_voice_evidence(evidence, epoch)?;
        }
    } else if consent_epoch.is_some() || voice.is_some() {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(())
}

fn validate_voice_evidence(
    evidence: &VoiceEvidence,
    consent_epoch: u64,
) -> Result<(), EpisodeStoreError> {
    if evidence.consent_epoch != consent_epoch
        || evidence.participant_count == 0
        || usize::from(evidence.participant_count) > MAX_RECEIPTS
        || evidence.transitions.len() > MAX_VOICE_TRANSITIONS
        || evidence.barge_in_count > 4_096
    {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(())
}

fn validate_stored_record(
    record: &StoredRecord,
    state: &LedgerState,
    _offset: usize,
) -> Result<(), EpisodeStoreError> {
    let expected_sequence = u64::try_from(state.request_ids.len())
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or(EpisodeStoreError::Corrupt)?;
    if record.sequence != expected_sequence
        || !bounded_identifier(&record.request_id, 64)
        || !bounded_identifier(&record.operation_id, 64)
        || !bounded_identifier(&record.guild_ref, 128)
        || !bounded_identifier(&record.policy_version, 64)
        || record.contract_revision == 0
        || record.contract_digest == [0; 32]
        || record.token_cost > MAX_TOKEN_COST
        || record.computed_digest()? != record.episode_digest
    {
        return Err(EpisodeStoreError::Corrupt);
    }
    validate_historical_voice_binding(record.source_type, record.consent_epoch, &record.event)
        .map_err(|_| EpisodeStoreError::Corrupt)?;
    validate_transition(record, state).map_err(|_| EpisodeStoreError::Corrupt)
}

fn validate_historical_voice_binding(
    source: EpisodeSource,
    consent_epoch: Option<u64>,
    event: &EpisodeEvent,
) -> Result<(), EpisodeStoreError> {
    let voice = match event {
        EpisodeEvent::Execution { voice, .. } => voice.as_ref(),
        _ => None,
    };
    if source == EpisodeSource::DiscordVoice {
        let epoch = consent_epoch
            .filter(|value| *value > 0)
            .ok_or(EpisodeStoreError::InvalidInput)?;
        if let Some(evidence) = voice {
            validate_voice_evidence(evidence, epoch)?;
        }
    } else if consent_epoch.is_some() || voice.is_some() {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(())
}

fn validate_transition(
    record: &StoredRecord,
    state: &LedgerState,
) -> Result<(), EpisodeStoreError> {
    if state.request_ids.contains(&record.request_id) {
        return Err(EpisodeStoreError::Replay);
    }
    let Some(existing) = state.operations.get(&record.operation_id) else {
        return match &record.event {
            EpisodeEvent::Proposal {
                requested_by,
                proposed_by,
            } if valid_actor(requested_by)
                && valid_actor(proposed_by)
                && proposed_by.kind == ActorKind::Service
                && requested_by.principal_id != proposed_by.principal_id
                && record.previous_digest.is_none() =>
            {
                Ok(())
            }
            _ => Err(EpisodeStoreError::InvalidTransition),
        };
    };
    if matches!(record.event, EpisodeEvent::Proposal { .. }) {
        return Err(EpisodeStoreError::Replay);
    }
    if existing.stage == Stage::Terminal
        || record.previous_digest != Some(existing.last_digest)
        || existing.contract_revision != record.contract_revision
        || existing.contract_digest != record.contract_digest
        || existing.guild_ref != record.guild_ref
        || existing.consent_epoch != record.consent_epoch
        || existing.source_type != record.source_type
        || existing.policy_version != record.policy_version
        || existing.evidence_level != record.evidence_level
    {
        return Err(EpisodeStoreError::InvalidTransition);
    }
    match (&record.event, existing.stage) {
        (EpisodeEvent::Approval { approved_by }, Stage::Proposed)
            if valid_actor(approved_by)
                && approved_by.kind != ActorKind::Service
                && approved_by.principal_id != existing.requested_by.principal_id
                && approved_by.principal_id != existing.proposed_by.principal_id =>
        {
            Ok(())
        }
        (EpisodeEvent::Execution { executed_by, .. }, Stage::Approved)
            if valid_actor(executed_by) && executed_by.kind == ActorKind::Service =>
        {
            Ok(())
        }
        (EpisodeEvent::Compensation { compensated_by, .. }, Stage::Executed)
            if valid_actor(compensated_by) && compensated_by.kind == ActorKind::Service =>
        {
            Ok(())
        }
        (EpisodeEvent::Terminal { status, .. }, prior_stage)
            if terminal_follows(*status, prior_stage) =>
        {
            Ok(())
        }
        _ => Err(EpisodeStoreError::InvalidTransition),
    }
}

fn terminal_follows(status: TerminalStatus, stage: Stage) -> bool {
    match status {
        TerminalStatus::Completed => stage == Stage::Executed,
        TerminalStatus::Compensated => stage == Stage::Compensated,
        TerminalStatus::Failed | TerminalStatus::Expired | TerminalStatus::Revoked => {
            matches!(
                stage,
                Stage::Proposed | Stage::Approved | Stage::Executed | Stage::Compensated
            )
        }
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
        .saturating_add(u64::try_from(line_bytes).map_err(|_| EpisodeStoreError::Corrupt)?);
    match &record.event {
        EpisodeEvent::Proposal {
            requested_by,
            proposed_by,
        } => {
            state.operations.insert(
                record.operation_id.clone(),
                OperationState {
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
                    stage: Stage::Proposed,
                    last_digest: record.episode_digest,
                },
            );
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
                EpisodeEvent::Proposal { .. } => return Err(EpisodeStoreError::Corrupt),
            };
            operation.last_digest = record.episode_digest;
        }
    }
    Ok(())
}

fn canonical_event(event: &EpisodeEvent) -> CanonicalValue {
    match event {
        EpisodeEvent::Proposal {
            requested_by,
            proposed_by,
        } => CanonicalValue::Map(vec![
            text_entry("requested_by", canonical_actor(requested_by)),
            text_entry("proposed_by", canonical_actor(proposed_by)),
        ]),
        EpisodeEvent::Approval { approved_by } => CanonicalValue::Map(vec![text_entry(
            "approved_by",
            canonical_actor(approved_by),
        )]),
        EpisodeEvent::Execution { executed_by, voice } => CanonicalValue::Map(vec![
            text_entry("executed_by", canonical_actor(executed_by)),
            text_entry(
                "voice",
                voice.as_ref().map_or(CanonicalValue::Null, canonical_voice),
            ),
        ]),
        EpisodeEvent::Compensation {
            compensated_by,
            exact_restore_observed,
        } => CanonicalValue::Map(vec![
            text_entry("compensated_by", canonical_actor(compensated_by)),
            text_entry(
                "exact_restore_observed",
                CanonicalValue::Bool(*exact_restore_observed),
            ),
        ]),
        EpisodeEvent::Terminal { status, reason } => CanonicalValue::Map(vec![
            text_entry("status", CanonicalValue::Text(status.label().into())),
            text_entry("reason", CanonicalValue::Text(reason.label().into())),
        ]),
    }
}

fn canonical_actor(actor: &ActorRef) -> CanonicalValue {
    CanonicalValue::Map(vec![
        text_entry(
            "principal_id",
            CanonicalValue::Text(actor.principal_id.clone()),
        ),
        text_entry("kind", CanonicalValue::Text(actor.kind.label().into())),
    ])
}

fn canonical_voice(voice: &VoiceEvidence) -> CanonicalValue {
    CanonicalValue::Map(vec![
        text_entry(
            "consent_epoch",
            CanonicalValue::Unsigned(voice.consent_epoch),
        ),
        text_entry(
            "participant_count",
            CanonicalValue::Unsigned(u64::from(voice.participant_count)),
        ),
        text_entry(
            "authorization_state",
            CanonicalValue::Text(voice.authorization_state.label().into()),
        ),
        text_entry(
            "attribution",
            CanonicalValue::Text(voice.attribution.label().into()),
        ),
        text_entry("stt", CanonicalValue::Text(voice.stt.label().into())),
        text_entry("tts", CanonicalValue::Text(voice.tts.label().into())),
        text_entry(
            "playback",
            CanonicalValue::Text(voice.playback.label().into()),
        ),
        text_entry(
            "barge_in_count",
            CanonicalValue::Unsigned(u64::from(voice.barge_in_count)),
        ),
        text_entry(
            "transitions",
            CanonicalValue::Array(
                voice
                    .transitions
                    .iter()
                    .map(|transition| CanonicalValue::Text(transition.label().into()))
                    .collect(),
            ),
        ),
        text_entry(
            "terminal_reason",
            CanonicalValue::Text(voice.terminal_reason.label().into()),
        ),
    ])
}

fn text_entry(key: &str, value: CanonicalValue) -> (CanonicalValue, CanonicalValue) {
    (CanonicalValue::Text(key.into()), value)
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
