//! Closed episode write and receipt vocabulary.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Maximum number of bounded voice state transitions retained in one event.
pub const MAX_VOICE_TRANSITIONS: usize = 32;

/// Identity classes admitted to constitutional episode events.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    /// A human request subject.
    HumanSubject,
    /// An organization owner.
    OrganizationOwner,
    /// A guild owner.
    GuildOwner,
    /// A guild administrator.
    GuildAdministrator,
    /// A guild manager.
    GuildManager,
    /// A service identity such as Abbey.
    Service,
}

impl ActorKind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::HumanSubject => "human_subject",
            Self::OrganizationOwner => "organization_owner",
            Self::GuildOwner => "guild_owner",
            Self::GuildAdministrator => "guild_administrator",
            Self::GuildManager => "guild_manager",
            Self::Service => "service",
        }
    }
}

/// Bounded, opaque authority identity. It is never a Discord participant list.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActorRef {
    /// Opaque principal reference.
    pub principal_id: String,
    /// Closed principal class.
    pub kind: ActorKind,
}

/// Source class bound into each canonical episode commitment.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeSource {
    /// A typed operator or service proposal.
    Proposal,
    /// A Discord guild effect with no voice content.
    DiscordGuild,
    /// Content-free counters from a consented Discord voice epoch.
    DiscordVoice,
    /// A local runtime observation.
    LocalRuntime,
}

impl EpisodeSource {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Proposal => "proposal",
            Self::DiscordGuild => "discord_guild",
            Self::DiscordVoice => "discord_voice",
            Self::LocalRuntime => "local_runtime",
        }
    }
}

/// Constitutional evidence maturity bound into the event commitment.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceLevel {
    /// Specified only.
    C0,
    /// Source and contract evidence.
    C1,
    /// Deterministic replay evidence.
    C2,
    /// Baseline and ablation evidence.
    C3,
    /// Shadow evidence.
    C4,
    /// Isolated canary evidence.
    C5,
    /// Live witnessed evidence.
    C6,
    /// Sustained production evidence.
    C7,
}

impl EvidenceLevel {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::C0 => "C0",
            Self::C1 => "C1",
            Self::C2 => "C2",
            Self::C3 => "C3",
            Self::C4 => "C4",
            Self::C5 => "C5",
            Self::C6 => "C6",
            Self::C7 => "C7",
        }
    }
}

/// Consent authorization state retained without participant identity.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationState {
    /// Epoch is awaiting unanimous consent.
    Pending,
    /// Current manager and all current participants are authorized.
    Authorized,
    /// Authorization is paused pending a new attestation.
    Paused,
    /// The epoch is closed.
    Closed,
}

impl AuthorizationState {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Authorized => "authorized",
            Self::Paused => "paused",
            Self::Closed => "closed",
        }
    }
}

/// Content-free decoded-attribution result.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttributionResult {
    /// Every decoded stream was attributable inside the active epoch.
    Attributed,
    /// Attribution was ambiguous and media stopped.
    Ambiguous,
    /// Attribution was unavailable and media stopped.
    Unavailable,
}

impl AttributionResult {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Attributed => "attributed",
            Self::Ambiguous => "ambiguous",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Bounded media-stage outcome with no media or text payload.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaOutcome {
    /// Stage was not attempted.
    NotAttempted,
    /// Stage completed.
    Succeeded,
    /// Stage failed closed.
    Failed,
    /// Stage was cancelled.
    Cancelled,
}

impl MediaOutcome {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::NotAttempted => "not_attempted",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Content-free consent and playback transitions.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VoiceTransition {
    /// Consent epoch opened.
    Opened,
    /// An attestation was accepted.
    Attested,
    /// Media paused.
    Paused,
    /// Authorized media resumed.
    Resumed,
    /// Participant change closed the epoch.
    ParticipantChangeClosed,
    /// Epoch closed for another bounded reason.
    Closed,
}

impl VoiceTransition {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Opened => "opened",
            Self::Attested => "attested",
            Self::Paused => "paused",
            Self::Resumed => "resumed",
            Self::ParticipantChangeClosed => "participant_change_closed",
            Self::Closed => "closed",
        }
    }
}

/// Content-free terminal reason for a voice interaction.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminalReason {
    /// The bounded interaction completed normally.
    Completed,
    /// Participant membership changed.
    ParticipantChange,
    /// Consent was lost or unavailable.
    ConsentLost,
    /// Manager authorization was lost.
    ManagerDeauthorized,
    /// Provider health degraded.
    ProviderDegraded,
    /// An explicit stop was requested.
    ExplicitStop,
    /// A bounded media stage failed.
    MediaFailure,
}

impl TerminalReason {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::ParticipantChange => "participant_change",
            Self::ConsentLost => "consent_lost",
            Self::ManagerDeauthorized => "manager_deauthorized",
            Self::ProviderDegraded => "provider_degraded",
            Self::ExplicitStop => "explicit_stop",
            Self::MediaFailure => "media_failure",
        }
    }
}

/// The only durable voice evidence shape.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VoiceEvidence {
    /// Current consent epoch, repeated for receipt inspection.
    pub consent_epoch: u64,
    /// Bounded participant count without participant identifiers.
    pub participant_count: u16,
    /// Current authorization state.
    pub authorization_state: AuthorizationState,
    /// Decoded attribution outcome.
    pub attribution: AttributionResult,
    /// Speech-to-text outcome without transcript.
    pub stt: MediaOutcome,
    /// Text-to-speech outcome without generated answer.
    pub tts: MediaOutcome,
    /// Audible playback outcome.
    pub playback: MediaOutcome,
    /// Number of barge-ins observed.
    pub barge_in_count: u16,
    /// Bounded state transitions in observation order.
    pub transitions: Vec<VoiceTransition>,
    /// Content-free terminal reason.
    pub terminal_reason: TerminalReason,
}

/// Append-only operation lifecycle event.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EpisodeEvent {
    /// Abbey-authored immutable proposal.
    Proposal {
        /// Requesting identity.
        requested_by: ActorRef,
        /// Abbey service proposal author.
        proposed_by: ActorRef,
    },
    /// Human approval for the exact proposal.
    Approval {
        /// Distinct human approver.
        approved_by: ActorRef,
    },
    /// Attempted authorized execution.
    Execution {
        /// Executing service identity.
        executed_by: ActorRef,
        /// Optional content-free voice counters.
        voice: Option<VoiceEvidence>,
    },
    /// Attempted compensation.
    Compensation {
        /// Compensating service identity.
        compensated_by: ActorRef,
        /// Whether the exact expected prior-state digest was restored.
        exact_restore_observed: bool,
    },
    /// One terminal operation status.
    Terminal {
        /// Closed terminal status.
        status: TerminalStatus,
        /// Content-free reason.
        reason: TerminalReason,
    },
}

impl EpisodeEvent {
    pub(super) const fn label(&self) -> &'static str {
        match self {
            Self::Proposal { .. } => "proposal",
            Self::Approval { .. } => "approval",
            Self::Execution { .. } => "execution",
            Self::Compensation { .. } => "compensation",
            Self::Terminal { .. } => "terminal",
        }
    }
}

/// Closed terminal state for an operation chain.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminalStatus {
    /// Operation completed.
    Completed,
    /// Operation was compensated.
    Compensated,
    /// Operation failed closed.
    Failed,
    /// Operation expired before completion.
    Expired,
    /// Operation was revoked.
    Revoked,
}

impl TerminalStatus {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Compensated => "compensated",
            Self::Failed => "failed",
            Self::Expired => "expired",
            Self::Revoked => "revoked",
        }
    }
}

/// One proposed append. WDBX computes the canonical commitment itself.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EpisodeWrite {
    /// Per-request replay identifier.
    pub request_id: String,
    /// Operation chain identifier.
    pub operation_id: String,
    /// Exact ABI contract revision.
    pub contract_revision: u64,
    /// Exact ABI contract-corpus digest.
    pub contract_digest: [u8; 32],
    /// Opaque guild scope.
    pub guild_ref: String,
    /// Consent epoch for voice sources; absent otherwise.
    pub consent_epoch: Option<u64>,
    /// Closed source type.
    pub source_type: EpisodeSource,
    /// Exact policy version.
    pub policy_version: String,
    /// Evidence maturity.
    pub evidence_level: EvidenceLevel,
    /// Append-only lifecycle event.
    pub event: EpisodeEvent,
    /// Guild-budget token charge.
    pub token_cost: u64,
    /// Optional caller prediction; mismatch is rejected.
    pub expected_commitment: Option<[u8; 32]>,
    /// Higher-priority response/learning/write suppression.
    pub quiet: bool,
}

/// Per-guild default-off policy supplied by the constitutional host.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GuildEpisodePolicy {
    /// Durable learning/write opt-in.
    pub learning_enabled: bool,
    /// Exact policy version admitted for new writes.
    pub policy_version: String,
    /// Maximum cumulative token charge.
    pub token_budget: u64,
    /// Maximum cumulative serialized ledger bytes.
    pub storage_budget_bytes: u64,
    /// Current active consent epoch for voice writes.
    pub current_consent_epoch: Option<u64>,
}

/// Exact contract and guild policy snapshot for one store session.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StorePolicy {
    /// Expected ABI contract revision.
    pub contract_revision: u64,
    /// Expected ABI contract-corpus digest.
    pub contract_digest: [u8; 32],
    /// Guild policies keyed by opaque guild reference.
    pub guilds: BTreeMap<String, GuildEpisodePolicy>,
}

/// Sanitized append or retrieval receipt.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EpisodeReceipt {
    /// Monotonic ledger sequence.
    pub sequence: u64,
    /// Request replay identifier.
    pub request_id: String,
    /// Operation identifier.
    pub operation_id: String,
    /// Opaque guild scope.
    pub guild_ref: String,
    /// Canonical episode commitment.
    pub episode_digest: [u8; 32],
    /// Previous operation event commitment, if any.
    pub previous_digest: Option<[u8; 32]>,
    /// Closed event type.
    pub event_kind: String,
    /// Bound policy version.
    pub policy_version: String,
    /// Bound evidence maturity.
    pub evidence_level: EvidenceLevel,
    /// Terminal status only for terminal events.
    pub terminal_status: Option<TerminalStatus>,
    /// Receipt is content-free by construction.
    pub redacted: bool,
}
