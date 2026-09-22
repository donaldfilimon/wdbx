//! Canonical episode digest derivation for [`super::StoredRecord`].
//!
//! Mirrored byte-for-byte by `episode_digest` and `canonical_event` in
//! `tools/abbey_cbor_episode_v1.py`; the `GOLDEN_DIGEST` constants in
//! `tests/v3_memory_candidate.rs` and `tests/v3_memory_edge.rs` pin the output.

use super::super::types::{ActorRef, EpisodeEvent, MemoryCandidate, MemoryEdge, VoiceEvidence};
use super::{EpisodeStoreError, StoredRecord};
use crate::v3::commitment::{CanonicalValue, EpisodeCommitment};

impl StoredRecord {
    pub(super) fn computed_digest(&self) -> Result<[u8; 32], EpisodeStoreError> {
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
        EpisodeEvent::MemoryCandidate {
            recorded_by,
            candidate,
        } => CanonicalValue::Map(vec![
            text_entry("recorded_by", canonical_actor(recorded_by)),
            text_entry("candidate", canonical_memory_candidate(candidate)),
        ]),
        EpisodeEvent::MemoryEdge { recorded_by, edge } => CanonicalValue::Map(vec![
            text_entry("recorded_by", canonical_actor(recorded_by)),
            text_entry("edge", canonical_memory_edge(edge)),
        ]),
    }
}

fn canonical_memory_edge(edge: &MemoryEdge) -> CanonicalValue {
    CanonicalValue::Map(vec![
        text_entry("kind", CanonicalValue::Text(edge.kind.label().into())),
        text_entry("target", CanonicalValue::Bytes(edge.target.to_vec())),
        text_entry(
            "counterpart",
            edge.counterpart.map_or(CanonicalValue::Null, |bytes| {
                CanonicalValue::Bytes(bytes.to_vec())
            }),
        ),
        text_entry("reason", CanonicalValue::Text(edge.reason.label().into())),
    ])
}

fn canonical_memory_candidate(candidate: &MemoryCandidate) -> CanonicalValue {
    let digest = |value: Option<[u8; 32]>| {
        value.map_or(CanonicalValue::Null, |bytes| {
            CanonicalValue::Bytes(bytes.to_vec())
        })
    };
    CanonicalValue::Map(vec![
        text_entry(
            "class",
            CanonicalValue::Text(candidate.class.label().into()),
        ),
        text_entry(
            "retention",
            CanonicalValue::Text(candidate.retention.label().into()),
        ),
        text_entry(
            "payload_commitment",
            CanonicalValue::Bytes(candidate.payload_commitment.to_vec()),
        ),
        text_entry(
            "payload_bytes",
            CanonicalValue::Unsigned(candidate.payload_bytes),
        ),
        text_entry(
            "dimension",
            candidate.dimension.map_or(CanonicalValue::Null, |width| {
                CanonicalValue::Unsigned(u64::from(width))
            }),
        ),
        text_entry(
            "embedding_version",
            candidate
                .embedding_version
                .clone()
                .map_or(CanonicalValue::Null, CanonicalValue::Text),
        ),
        text_entry(
            "member_scoped",
            CanonicalValue::Bool(candidate.member_scoped),
        ),
        text_entry("supersedes", digest(candidate.supersedes)),
        text_entry("forgets", digest(candidate.forgets)),
    ])
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
