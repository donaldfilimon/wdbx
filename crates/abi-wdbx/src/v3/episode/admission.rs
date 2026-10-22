//! Structured admission decisions for the v3 episode write gate.
//!
//! The write gate already answers "may this write be appended?" with a closed
//! [`EpisodeStoreError`]. This module adds the other half of that answer:
//! *what kind of record would be admitted*, and *why* a write was refused, in
//! one content-free value a caller can inspect without matching on an error
//! string.
//!
//! That value is reached through
//! [`EpisodeStore::decide_write_outcome`](super::store::EpisodeStore::decide_write_outcome),
//! which never fails: it carries the class and the reason — accepted, or the
//! reason the refusal maps to — whether the write was admitted or not.
//! [`EpisodeStore::decide_write`](super::store::EpisodeStore::decide_write)
//! remains for callers that want the `Result` and the store error itself.
//!
//! It is deliberately a parallel decision type. It does not change
//! [`EpisodeStoreError`] or its labels, which the ABI gateway maps to gRPC
//! status messages, and it changes no canonical commitment: an admitted
//! episode gains no field, so every existing digest and pinned golden is
//! unaffected.

use std::fmt;

use super::store::EpisodeStoreError;
use super::types::{EpisodeEvent, EpisodeReceipt, EpisodeWrite, MemoryCandidate, RetentionClass};

/// The constitutional five-class retention vocabulary.
///
/// A decision names the class of the record a write would admit. The set is
/// closed: a consumer may not invent a sixth class, and the store never
/// derives one from transport JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdmissionClass {
    /// Never persisted. Nothing in the episode write vocabulary proposes it.
    Ephemeral,
    /// Lives only for the bounded session.
    Session,
    /// Persisted under an adapter-configured TTL.
    Operational,
    /// Persisted until it is superseded or forgotten.
    Durable,
    /// An incident record that must be retained.
    MandatoryIncident,
}

impl AdmissionClass {
    /// Stable label, for logs and transports.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ephemeral => "ephemeral",
            Self::Session => "session",
            Self::Operational => "operational",
            Self::Durable => "durable",
            Self::MandatoryIncident => "mandatory_incident",
        }
    }
}

/// Why the write gate admitted or refused a write.
///
/// Every [`EpisodeStoreError`] variant has exactly one reason, so the
/// mapping is total: a new store error cannot be silently unclassified, and
/// a caller never has to parse an error label to learn why a write was
/// refused. The open-time [`EpisodeStoreError::WriterBusy`] is included for
/// that totality even though the write gate cannot produce it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdmissionReason {
    /// The write passed every gate check and would be appended.
    Accepted,
    /// The store or ledger operation failed.
    Io,
    /// Another writer owns the ledger.
    WriterBusy,
    /// The ledger contains an invalid or mutated complete record.
    Corrupt,
    /// A canonical value was outside the deterministic profile.
    Canonical,
    /// A bounded identifier, collection, counter, or policy is invalid.
    InvalidInput,
    /// Contract revision, digest, guild, policy, or consent epoch is stale.
    StaleBinding,
    /// Guild learning and durable writes are not opted in.
    LearningDisabled,
    /// `QUIET` suppresses response, learning, and the durable write.
    Quiet,
    /// The request or operation identifier was replayed.
    Replay,
    /// The requested lifecycle transition or authority identity is invalid.
    InvalidTransition,
    /// The caller-supplied commitment prediction does not match WDBX.
    CommitmentMismatch,
    /// The guild token budget is exhausted.
    TokenBudget,
    /// The guild storage budget or whole-ledger size ceiling is exhausted.
    StorageBudget,
}

impl AdmissionReason {
    /// Stable label, for logs and transports.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Io => "io",
            Self::WriterBusy => "writer_busy",
            Self::Corrupt => "corrupt",
            Self::Canonical => "canonical",
            Self::InvalidInput => "invalid_input",
            Self::StaleBinding => "stale_binding",
            Self::LearningDisabled => "learning_disabled",
            Self::Quiet => "quiet",
            Self::Replay => "replay",
            Self::InvalidTransition => "invalid_transition",
            Self::CommitmentMismatch => "commitment_mismatch",
            Self::TokenBudget => "token_budget",
            Self::StorageBudget => "storage_budget",
        }
    }
}

impl From<&EpisodeStoreError> for AdmissionReason {
    fn from(error: &EpisodeStoreError) -> Self {
        match error {
            EpisodeStoreError::Io => Self::Io,
            EpisodeStoreError::WriterBusy => Self::WriterBusy,
            EpisodeStoreError::Corrupt => Self::Corrupt,
            EpisodeStoreError::Canonical(_) => Self::Canonical,
            EpisodeStoreError::InvalidInput => Self::InvalidInput,
            EpisodeStoreError::StaleBinding => Self::StaleBinding,
            EpisodeStoreError::LearningDisabled => Self::LearningDisabled,
            EpisodeStoreError::Quiet => Self::Quiet,
            EpisodeStoreError::Replay => Self::Replay,
            EpisodeStoreError::InvalidTransition => Self::InvalidTransition,
            EpisodeStoreError::CommitmentMismatch => Self::CommitmentMismatch,
            EpisodeStoreError::TokenBudget => Self::TokenBudget,
            EpisodeStoreError::StorageBudget => Self::StorageBudget,
        }
    }
}

/// The outcome of one admission decision.
///
/// Content-free by construction: it names a class, a reason, and at most a
/// sanitized receipt. It never carries a payload, a transcript, a
/// participant, or a raw event.
#[derive(Clone, PartialEq, Eq)]
pub struct AdmissionDecision {
    class: AdmissionClass,
    reason: AdmissionReason,
    receipt: Option<EpisodeReceipt>,
}

impl AdmissionDecision {
    /// An accepted write that has not been appended yet.
    ///
    /// A decision is non-mutating, so it has no receipt yet.
    #[must_use]
    pub fn accepted(class: AdmissionClass) -> Self {
        Self {
            class,
            reason: AdmissionReason::Accepted,
            receipt: None,
        }
    }

    /// An accepted write that already carries its sanitized receipt.
    #[must_use]
    pub fn admitted(class: AdmissionClass, receipt: EpisodeReceipt) -> Self {
        Self {
            class,
            reason: AdmissionReason::Accepted,
            receipt: Some(receipt),
        }
    }

    /// A refused write, with the reason the gate refused it.
    #[must_use]
    pub fn refused(class: AdmissionClass, error: &EpisodeStoreError) -> Self {
        Self {
            class,
            reason: AdmissionReason::from(error),
            receipt: None,
        }
    }

    /// The class of the record this decision admits.
    #[must_use]
    pub fn class(&self) -> AdmissionClass {
        self.class
    }

    /// Why the write was admitted or refused.
    #[must_use]
    pub fn reason(&self) -> AdmissionReason {
        self.reason
    }

    /// The sanitized receipt, when the write was already appended.
    #[must_use]
    pub fn receipt(&self) -> Option<&EpisodeReceipt> {
        self.receipt.as_ref()
    }
}

/// Content-free debug output: the class, the reason, and whether a receipt
/// exists. No identifier, digest, or label from the receipt is printed.
impl fmt::Debug for AdmissionDecision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdmissionDecision")
            .field("class", &self.class)
            .field("reason", &self.reason)
            .field(
                "receipt",
                &self.receipt.as_ref().map(|receipt| receipt.sequence),
            )
            .finish()
    }
}

/// The admission class of one memory candidate.
///
/// The candidate's requested [`RetentionClass`] is the only retention signal
/// the write carries, so it maps one to one: `session` is session-scoped,
/// `operational` is TTL-scoped, and `durable` is persisted until it is
/// superseded or forgotten.
///
/// [`AdmissionClass::Ephemeral`] and [`AdmissionClass::MandatoryIncident`]
/// are unreachable from a candidate: the closed `RetentionClass` vocabulary
/// has no such member, and an adapter may not propose one.
#[must_use]
pub fn classify(candidate: &MemoryCandidate) -> AdmissionClass {
    match candidate.retention {
        RetentionClass::Session => AdmissionClass::Session,
        RetentionClass::Operational => AdmissionClass::Operational,
        RetentionClass::Durable => AdmissionClass::Durable,
    }
}

/// The admission class of one episode event.
///
/// A memory candidate carries its own retention class. A memory edge names
/// candidates and adds no payload, and a lifecycle episode
/// (`proposal`, `approval`, `execution`, `compensation`, `terminal`) carries
/// no memory at all; both are classified [`AdmissionClass::Durable`], for
/// the reason given in [`classify_write`].
#[must_use]
pub fn classify_event(event: &EpisodeEvent) -> AdmissionClass {
    match event {
        EpisodeEvent::MemoryCandidate { candidate, .. } => classify(candidate),
        EpisodeEvent::MemoryEdge { .. }
        | EpisodeEvent::Proposal { .. }
        | EpisodeEvent::Approval { .. }
        | EpisodeEvent::Execution { .. }
        | EpisodeEvent::Compensation { .. }
        | EpisodeEvent::Terminal { .. } => AdmissionClass::Durable,
    }
}

/// The admission class of one proposed write.
///
/// A memory candidate is classified by the retention class it requests. Any
/// other episode write is classified [`AdmissionClass::Durable`]: the
/// episode ledger is append-only and durable, a lifecycle episode is
/// persisted, it has no adapter TTL, and it expires only when the guild
/// storage budget or the whole-ledger ceiling refuses further appends. It is
/// therefore never ephemeral (it is persisted), never session-scoped (a
/// session-scoped record is not persisted by the adapter), never operational
/// (no TTL governs a lifecycle episode), and never a mandatory incident (a
/// lifecycle episode is not an incident record).
#[must_use]
pub fn classify_write(write: &EpisodeWrite) -> AdmissionClass {
    classify_event(&write.event)
}
