//! Write-gate validation for the episode ledger: store policy, bounded
//! identifiers, guild policy and voice consent binding, event shape, replay
//! rejection, lifecycle transitions, and memory-edge admission.
//!
//! The same checks run on the write path (`InvalidInput`/`Replay`/...) and on
//! replay of an existing ledger (`Corrupt`).

use super::super::types::{
    ActorKind, EpisodeEvent, EpisodeSource, EpisodeWrite, GuildEpisodePolicy,
    MAX_VOICE_TRANSITIONS, MemoryCandidate, MemoryClass, MemoryEdge, MemoryEdgeKind, StorePolicy,
    TerminalStatus, VoiceEvidence,
};
use super::{
    EpisodeStoreError, LedgerState, MAX_LEDGER_BYTES, MAX_RECEIPTS, MAX_TOKEN_COST, Stage,
    StoredRecord, bounded_identifier, valid_actor,
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
        })
    {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(())
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
    Ok(())
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
        TerminalStatus::Compensated => stage == Stage::Compensated,
        TerminalStatus::Failed | TerminalStatus::Expired | TerminalStatus::Revoked => {
            matches!(
                stage,
                Stage::Proposed | Stage::Approved | Stage::Executed | Stage::Compensated
            )
        }
    }
}
