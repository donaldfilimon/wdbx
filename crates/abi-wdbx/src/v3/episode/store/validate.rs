//! Write-gate validation for the episode ledger: store policy, bounded
//! identifiers, guild policy and voice consent binding, event shape, replay
//! rejection, lifecycle transitions, and memory-edge admission.
//!
//! The same checks run on the write path (`InvalidInput`/`Replay`/...) and on
//! replay of an existing ledger (`Corrupt`).

use super::super::types::{
    ActorKind, EpisodeEvent, EpisodeSource, EpisodeWrite, GuildEpisodePolicy,
    MAX_AUTHORIZED_PRINCIPALS, MAX_VOICE_TRANSITIONS, MemoryCandidate, MemoryClass, MemoryEdge,
    MemoryEdgeKind, RetentionClass, StorePolicy, TerminalStatus, VoiceEvidence,
};
use super::{
    EpisodeStore, EpisodeStoreError, LedgerState, MAX_LEDGER_BYTES, MAX_RECEIPTS, MAX_TOKEN_COST,
    Stage, StoredRecord, bounded_identifier, valid_actor,
};

pub(super) fn validate_store_policy(policy: &StorePolicy) -> Result<(), EpisodeStoreError> {
    if policy.contract_revision == 0
        || policy.contract_digest == [0; 32]
        || policy.guilds.len() > MAX_RECEIPTS
        || policy.guilds.iter().any(|(guild, item)| {
            !bounded_identifier(guild, 128)
                || !bounded_identifier(&item.policy_version, 64)
                || item.token_budget > 1_000_000_000
                || item.storage_budget_bytes > MAX_LEDGER_BYTES
                || !valid_authority_map(item.authorized_principals.as_deref())
        })
    {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(())
}

/// Whether one guild's authority map is well formed.
///
/// A named principal must be a bounded identifier, exactly like the
/// `principal_id` of an actor: a name that could never be presented is a
/// configuration mistake, and the store refuses it at open rather than
/// carrying a map that can never match. The list is bounded so the check
/// stays a bounded scan.
fn valid_authority_map(principals: Option<&[String]>) -> bool {
    let Some(principals) = principals else {
        return true;
    };
    principals.len() <= MAX_AUTHORIZED_PRINCIPALS
        && principals
            .iter()
            .all(|principal| bounded_identifier(principal, 64))
}

pub(super) fn validate_new_write(
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
    validate_event_shape(write.source_type, &write.event)?;
    validate_retention(&write.event, guild)?;
    Ok(())
}

/// The retention gate: a zero window is refused outright.
///
/// The window that applies to a candidate is the policy's: the explicit
/// per-class window when the policy names one for the candidate's class, and
/// the guild's single time-to-live otherwise. A zero window is the only way a
/// deadline can be closed at admission, because `arrival + 0` is the arrival
/// itself. The gate refuses it instead of admitting a candidate that is
/// already expired.
///
/// The deadline a candidate gets is bound *after* this gate, from the store's
/// own clock at the instant of admission (`store::expiry::bound_deadline`).
///
/// A positive window is admitted, and it may expire immediately afterwards:
/// the gate never compares a candidate's deadline against a caller-supplied
/// clock, and it never reads a clock at all.
///
/// A `durable` candidate never expires and is never refused here, whatever
/// the window says. A `forgets` tombstone is not a memory and is never given
/// a deadline, so it is never refused here either.
fn validate_retention(
    event: &EpisodeEvent,
    policy: &GuildEpisodePolicy,
) -> Result<(), EpisodeStoreError> {
    let EpisodeEvent::MemoryCandidate { candidate, .. } = event else {
        return Ok(());
    };
    if candidate.forgets.is_some() {
        return Ok(());
    }
    if !matches!(
        candidate.retention,
        RetentionClass::Session | RetentionClass::Operational
    ) {
        return Ok(());
    }
    match policy.retention_ttl_for(candidate.retention) {
        Some(0) => Err(EpisodeStoreError::InvalidInput),
        _ => Ok(()),
    }
}

/// Source and field rules an event must satisfy regardless of lifecycle position.
///
/// Checked on the write path (`InvalidInput`) and on replay (`Corrupt`), so a
/// mutated ledger line cannot smuggle in a candidate the gate would refuse.
fn validate_event_shape(
    source: EpisodeSource,
    event: &EpisodeEvent,
) -> Result<(), EpisodeStoreError> {
    match event {
        // Raw audio and transcripts are ephemeral; there is no content-free
        // memory of a voice epoch for these classes to carry.
        EpisodeEvent::MemoryCandidate { .. } | EpisodeEvent::MemoryEdge { .. }
            if source == EpisodeSource::DiscordVoice =>
        {
            Err(EpisodeStoreError::InvalidInput)
        }
        EpisodeEvent::MemoryCandidate { candidate, .. } => validate_memory_candidate(candidate),
        EpisodeEvent::MemoryEdge { edge, .. } => validate_memory_edge(edge),
        _ => Ok(()),
    }
}

fn validate_memory_edge(edge: &MemoryEdge) -> Result<(), EpisodeStoreError> {
    let pair_ok = match (edge.kind, edge.counterpart) {
        // Strict order also rules out a candidate contradicting itself.
        (MemoryEdgeKind::Contradicts, Some(counterpart)) => edge.target < counterpart,
        (MemoryEdgeKind::Quarantines | MemoryEdgeKind::Resolves, None) => true,
        _ => false,
    };
    if pair_ok && edge.target != [0; 32] && edge.reason.fits(edge.kind) {
        Ok(())
    } else {
        Err(EpisodeStoreError::InvalidInput)
    }
}

fn validate_memory_candidate(candidate: &MemoryCandidate) -> Result<(), EpisodeStoreError> {
    let zero = [0_u8; 32];
    let forgets = candidate.forgets.is_some();
    let shape_ok = if forgets {
        candidate.supersedes.is_none()
            && candidate.payload_bytes == 0
            && candidate.payload_commitment == zero
    } else {
        candidate.payload_bytes > 0 && candidate.payload_commitment != zero
    };
    let dimension_ok = match candidate.dimension {
        Some(width) => candidate.class == MemoryClass::Embedding && width > 0,
        None => candidate.class != MemoryClass::Embedding,
    };
    let version_ok = candidate
        .embedding_version
        .as_deref()
        .is_none_or(|version| bounded_identifier(version, 64));
    if shape_ok && dimension_ok && version_ok {
        Ok(())
    } else {
        Err(EpisodeStoreError::InvalidInput)
    }
}

/// Bytes a candidate charges beyond its own ledger line.
pub(super) fn payload_bytes(event: &EpisodeEvent) -> u64 {
    match event {
        EpisodeEvent::MemoryCandidate { candidate, .. } => candidate.payload_bytes,
        _ => 0,
    }
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

pub(super) fn validate_stored_record(
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
    validate_event_shape(record.source_type, &record.event)
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

pub(super) fn validate_transition(
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
            EpisodeEvent::MemoryCandidate {
                recorded_by,
                candidate,
            } if valid_actor(recorded_by)
                && recorded_by.kind == ActorKind::Service
                && record.previous_digest.is_none()
                && memory_edges_admitted(&record.guild_ref, candidate, state) =>
            {
                Ok(())
            }
            EpisodeEvent::MemoryEdge { recorded_by, edge }
                if valid_actor(recorded_by)
                    && edge_author_allowed(edge.kind, recorded_by.kind)
                    && record.previous_digest.is_none()
                    && memory_edge_admitted(&record.guild_ref, edge, state) =>
            {
                Ok(())
            }
            _ => Err(EpisodeStoreError::InvalidTransition),
        };
    };
    // Every operation-opening event replays an existing operation identifier.
    if matches!(
        record.event,
        EpisodeEvent::Proposal { .. }
            | EpisodeEvent::MemoryCandidate { .. }
            | EpisodeEvent::MemoryEdge { .. }
    ) {
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

/// `supersedes`/`forgets` may only name a memory candidate this guild already
/// admitted and has not forgotten. A superseded digest may be superseded again
/// (two corrections of one memory are two edges, not a rewrite) and may still
/// be forgotten; a forgotten digest is a tombstone and accepts no further edge.
fn memory_edges_admitted(
    guild_ref: &str,
    candidate: &MemoryCandidate,
    state: &LedgerState,
) -> bool {
    let Some(target) = candidate.supersedes.or(candidate.forgets) else {
        return true;
    };
    state
        .memories
        .get(guild_ref)
        .is_some_and(|memories| memories.is_live(&target))
}

/// A service flags; only human guild or organization governance resolves.
const fn edge_author_allowed(kind: MemoryEdgeKind, author: ActorKind) -> bool {
    match kind {
        MemoryEdgeKind::Quarantines | MemoryEdgeKind::Contradicts => {
            matches!(author, ActorKind::Service)
        }
        MemoryEdgeKind::Resolves => matches!(
            author,
            ActorKind::GuildOwner
                | ActorKind::GuildAdministrator
                | ActorKind::GuildManager
                | ActorKind::OrganizationOwner
        ),
    }
}

/// Graph rules for a memory edge, given a shape already validated.
///
/// `quarantines` and `contradicts` name live candidates of this guild and may
/// not duplicate an open edge; a contradiction joins candidates of equal
/// member scope. `resolves` names an open edge episode, which may still name
/// a candidate forgotten since.
fn memory_edge_admitted(guild_ref: &str, edge: &MemoryEdge, state: &LedgerState) -> bool {
    let Some(memories) = state.memories.get(guild_ref) else {
        return false;
    };
    match (edge.kind, edge.counterpart) {
        (MemoryEdgeKind::Quarantines, None) => {
            memories.is_live(&edge.target) && !memories.quarantined.contains_key(&edge.target)
        }
        (MemoryEdgeKind::Contradicts, Some(counterpart)) => {
            memories.is_live(&edge.target)
                && memories.is_live(&counterpart)
                && memories.admitted.get(&edge.target) == memories.admitted.get(&counterpart)
                && !memories
                    .contradictions
                    .contains_key(&(edge.target, counterpart))
        }
        (MemoryEdgeKind::Resolves, None) => memories.open_edges.contains_key(&edge.target),
        _ => false,
    }
}

fn terminal_follows(status: TerminalStatus, stage: Stage) -> bool {
    match status {
        TerminalStatus::Completed => stage == Stage::Executed,
        TerminalStatus::Compensated => stage == Stage::Compensated(true),
        TerminalStatus::Failed | TerminalStatus::Expired | TerminalStatus::Revoked => {
            matches!(
                stage,
                Stage::Proposed | Stage::Approved | Stage::Executed | Stage::Compensated(_)
            )
        }
    }
}

impl EpisodeStore {
    /// The three budget checks of the write gate, shared by every caller.
    ///
    /// The guild token budget, the guild storage budget, and the
    /// whole-ledger ceiling are judged against the state the store holds
    /// right now, so the append path, the commitment preview, and the
    /// non-mutating admission decision reach the same verdict for the same
    /// write. `line_bytes` is the serialized length of the line the write
    /// would append, including its newline.
    ///
    /// The two guild budgets are the *effective* ones: the host policy's
    /// budget, overridden where a quota adjustment applies
    /// (`store::quota`). An adjustment therefore changes what the store
    /// accepts, and it changes it for every path at once.
    ///
    /// The retention index has a ceiling of its own, and this check accounts
    /// for it: a TTL-bound candidate adds one line to the retention sidecar
    /// before it reaches the ledger, and that line is charged here, before
    /// it is written. Without this the decision and the preview could admit
    /// a write the append path refuses when the sidecar is full.
    pub(super) fn check_quotas(
        &self,
        record: &StoredRecord,
        line_bytes: u64,
    ) -> Result<(), EpisodeStoreError> {
        let Some((token_budget, storage_budget_bytes)) = self.effective_budgets(&record.guild_ref)
        else {
            return Err(EpisodeStoreError::StaleBinding);
        };
        let usage = self
            .state
            .guild_usage
            .get(&record.guild_ref)
            .copied()
            .unwrap_or_default();
        if usage.tokens.saturating_add(record.token_cost) > token_budget {
            return Err(EpisodeStoreError::TokenBudget);
        }
        if usage
            .bytes
            .saturating_add(line_bytes)
            .saturating_add(payload_bytes(&record.event))
            > storage_budget_bytes
        {
            return Err(EpisodeStoreError::StorageBudget);
        }
        // Replay refuses ledgers larger than this bound, even when no single
        // guild has exhausted its own budget. Never append an unreopenable file.
        let ledger_bytes = self
            .ledger
            .metadata()
            .map_err(|_| EpisodeStoreError::Io)?
            .len();
        if ledger_bytes
            .checked_add(line_bytes)
            .is_none_or(|total| total > MAX_LEDGER_BYTES)
        {
            return Err(EpisodeStoreError::StorageBudget);
        }
        // The retention sidecar has the same whole-ledger ceiling, and this
        // write would add one line to it. The line is charged at its
        // greatest possible size, so the check is an upper bound: the
        // append path can never refuse a line this check admitted.
        if let Some(projected) = self.projected_retention_bytes(record)
            && projected > MAX_LEDGER_BYTES
        {
            return Err(EpisodeStoreError::StorageBudget);
        }
        Ok(())
    }

    /// The number of bytes the retention index would hold after this write.
    ///
    /// `None` when the write binds no deadline: a `durable` candidate, a
    /// `forgets` tombstone, a guild without a TTL, or an unreadable clock.
    fn projected_retention_bytes(&self, record: &StoredRecord) -> Option<u64> {
        self.retention_deadline(record)?;
        Some(
            self.retention
                .byte_len()
                .saturating_add(super::expiry::max_stamp_line_bytes()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::episode::types::{ActorRef, EvidenceLevel, TerminalReason};

    #[test]
    fn replay_rejects_digest_correct_success_after_inexact_compensation() {
        let actor = |principal_id: &str, kind| ActorRef {
            principal_id: principal_id.into(),
            kind,
        };
        let events = [
            EpisodeEvent::Proposal {
                requested_by: actor("requester", ActorKind::HumanSubject),
                proposed_by: actor("proposer", ActorKind::Service),
            },
            EpisodeEvent::Approval {
                approved_by: actor("admin", ActorKind::GuildAdministrator),
            },
            EpisodeEvent::Execution {
                executed_by: actor("executor", ActorKind::Service),
                voice: None,
            },
            EpisodeEvent::Compensation {
                compensated_by: actor("executor", ActorKind::Service),
                exact_restore_observed: false,
            },
            // Older validation admitted this chain despite the failed restore.
            EpisodeEvent::Terminal {
                status: TerminalStatus::Compensated,
                reason: TerminalReason::Completed,
            },
        ];
        let mut state = LedgerState::default();
        let mut previous_digest = None;
        for (index, event) in events.into_iter().enumerate() {
            let mut record = StoredRecord {
                sequence: u64::try_from(index + 1).unwrap(),
                request_id: format!("request_{index}"),
                operation_id: "operation".into(),
                contract_revision: 2,
                contract_digest: [9; 32],
                guild_ref: "guild".into(),
                consent_epoch: Some(7),
                source_type: EpisodeSource::DiscordVoice,
                policy_version: "policy_v1".into(),
                evidence_level: EvidenceLevel::C2,
                event,
                token_cost: 3,
                previous_digest,
                episode_digest: [0; 32],
                signature: None,
            };
            record.episode_digest = record.computed_digest().expect("canonical digest");
            // Round-trip persisted JSON so this exercises replay's actual record shape.
            let line = super::super::serialized_line(&record).expect("ledger line");
            let decoded: StoredRecord = serde_json::from_slice(&line).expect("stored JSON");
            assert_eq!(decoded.computed_digest().unwrap(), decoded.episode_digest);
            if index == 4 {
                assert!(matches!(
                    validate_stored_record(&decoded, &state, 0),
                    Err(EpisodeStoreError::Corrupt)
                ));
            } else {
                validate_stored_record(&decoded, &state, 0).expect("valid replay prefix");
                super::super::apply_record(
                    &decoded,
                    line.len(),
                    &mut state,
                    &super::super::expiry::RetentionIndex::empty(),
                )
                .expect("rebuild replay state");
                previous_digest = Some(decoded.episode_digest);
            }
        }
    }
}
