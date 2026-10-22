//! The closed vocabulary of the offline outbox.
//!
//! Everything a caller can name is named here: the operation identifier, the
//! kind tag, the status vocabulary, the content-free reasons, the proposal
//! payloads, and the status surface. The vocabulary is closed, so a consumer
//! cannot invent a sixth status or a fifth operation kind and have the
//! outbox store it.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::asset::AssetState;
use super::error::OutboxError;
use super::merge::{ConflictSummary, EditProposal};

/// The greatest length of an operation identifier, in bytes.
pub const MAX_OPERATION_ID_BYTES: usize = 64;
/// The greatest length of a scope identifier, in bytes.
pub const MAX_SCOPE_BYTES: usize = 128;
/// The greatest length of a policy version, in bytes.
pub const MAX_POLICY_VERSION_BYTES: usize = 64;
/// The greatest number of operations one outbox may hold.
pub const MAX_OPERATIONS: usize = 4096;

/// A bounded operation identifier.
///
/// The identifier is the outbox's idempotency key: a resubmission after a
/// crash carries the same identifier, and a server that deduplicates on it
/// never applies one proposal twice.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct OperationId(String);

impl OperationId {
    /// A new operation identifier, when the text is a bounded identifier.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::InvalidInput`] when the text is empty, longer
    /// than [`MAX_OPERATION_ID_BYTES`], or carries a character outside the
    /// bounded identifier set.
    pub fn new(value: impl Into<String>) -> Result<Self, OutboxError> {
        let value = value.into();
        if !bounded_identifier(&value, MAX_OPERATION_ID_BYTES) {
            return Err(OutboxError::InvalidInput);
        }
        Ok(Self(value))
    }

    /// The identifier's text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OperationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Whether the value is a bounded identifier.
///
/// The set is the episode store's own: non-empty, bounded in length, and
/// drawn from lowercase ASCII, digits, underscore, hyphen, and dot. The
/// outbox never accepts a free-text identifier, so a scope or a policy
/// version can never smuggle content into the outbox file.
pub(crate) fn bounded_identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
        })
}

/// The closed kinds of offline operation.
///
/// The kind tag is stored beside the payload and is checked against it when
/// the outbox is reopened, so a record can never claim one kind and carry
/// another.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    /// A proposed canonical episode write.
    Write,
    /// A proposed erase of one remembered record.
    Delete,
    /// A large payload transferred out of band.
    Asset,
    /// A field-level edit of a mergeable configuration object.
    Edit,
}

/// The closed status vocabulary of one queued operation.
///
/// `Pending` and `InFlight` are the live statuses; the other three are
/// terminal. A terminal operation is *retained*, never reaped: it stays in
/// the outbox with its reason until an authorized caller resolves it.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    /// Enqueued, not yet submitted.
    Pending,
    /// Submitted, and the outcome is not yet known.
    InFlight,
    /// The server accepted the proposal.
    Acknowledged,
    /// The server refused the proposal.
    Rejected,
    /// The operation conflicts with another operation.
    Conflicted,
}

impl OperationStatus {
    /// Whether the status is terminal.
    ///
    /// A terminal operation is never resubmitted: the caller resolves it.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Acknowledged | Self::Rejected | Self::Conflicted)
    }
}

/// The closed, content-free reasons a current policy refuses a proposal.
///
/// Every variant names a *class* of refusal. None of them carries a scope, a
/// digest, an identifier, or any proposal content.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RefusalReason {
    /// The scope is not in the current policy.
    UnknownScope,
    /// Durable learning and writes are not enabled for the scope.
    LearningDisabled,
    /// The policy version the operation was written under is not current.
    StalePolicyVersion,
    /// The scope's token budget is exhausted.
    TokenBudgetExhausted,
    /// The scope's storage budget is exhausted.
    StorageBudgetExhausted,
    /// The proposal is not admissible in its current shape.
    InvalidProposal,
    /// The operation was already applied.
    Replay,
    /// The proposal's transition is not valid for the current ledger state.
    InvalidTransition,
    /// The server suppressed the write.
    QuietSuppressed,
    /// The canonical store could not answer the question.
    StoreUnavailable,
    /// The server refused the proposal for a reason the caller named.
    ServerRefused,
}

/// The closed, content-free reasons one operation's status carries.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationReason {
    /// The current policy authorized the operation.
    Authorized,
    /// The current policy refused the operation.
    Refused(RefusalReason),
    /// The consent epoch moved and the operation's bound epoch is stale.
    ConsentEpochMoved,
    /// The operation was written under a different schema revision.
    SchemaSkew,
    /// The operation conflicts with another operation over one field.
    MergeConflict,
    /// The operation depends on an operation that is not acknowledged.
    DependencyPending,
    /// The transport failed, so the outcome is unknown.
    Unavailable,
    /// An authorized caller requeued the operation.
    Requeued,
}

/// The binding an operation carries: what it was written under.
///
/// The binding is what the reauthorization step compares against the
/// *current* server policy. It is content-free: a scope reference, a
/// contract revision, a contract digest, a policy version, and a consent
/// epoch.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperationBinding {
    /// The guild scope the operation belongs to.
    pub guild_ref: String,
    /// The ABI contract revision the operation was written under.
    pub contract_revision: u64,
    /// The ABI contract-corpus digest the operation was written under.
    pub contract_digest: [u8; 32],
    /// The policy version the operation was written under.
    pub policy_version: String,
    /// The consent epoch the operation was written under, when it binds one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consent_epoch: Option<u64>,
}

/// A large payload transferred out of band.
///
/// The proposal holds the asset's *commitment* and its geometry, never its
/// bytes. The chunk payload is supplied by the caller, one chunk at a time.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetProposal {
    /// SHA-256 over the asset's chunk list, in the outbox asset profile.
    pub digest: [u8; 32],
    /// The asset's length in bytes.
    pub length: u64,
    /// The transfer's chunk size in bytes.
    pub chunk_size: u64,
}

/// What one queued operation proposes.
///
/// The payload holds a *proposal*. It never holds a rendered or readable
/// episode, and it never holds a canonical episode digest: an episode write
/// is stored as the write the canonical store would compute its own
/// commitment from.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationPayload {
    /// A proposed canonical episode write.
    Write(Box<crate::v3::episode::EpisodeWrite>),
    /// A proposed erase of one remembered record.
    Delete(Box<crate::v3::episode::EpisodeWrite>),
    /// A large payload transferred out of band.
    Asset(AssetProposal),
    /// A field-level edit of a mergeable configuration object.
    Edit(EditProposal),
}

impl OperationPayload {
    /// The kind tag this payload carries.
    #[must_use]
    pub fn kind(&self) -> OperationKind {
        match self {
            Self::Write(_) => OperationKind::Write,
            Self::Delete(_) => OperationKind::Delete,
            Self::Asset(_) => OperationKind::Asset,
            Self::Edit(_) => OperationKind::Edit,
        }
    }
}

/// One durable offline operation, as the outbox holds it.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutboxOperation {
    /// The operation's identifier.
    pub id: OperationId,
    /// The operation this one depends on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<OperationId>,
    /// The binding the operation was written under.
    pub binding: OperationBinding,
    /// The operation's kind tag.
    pub kind: OperationKind,
    /// The proposal the operation carries.
    pub payload: OperationPayload,
    /// The operation's status.
    pub status: OperationStatus,
    /// The content-free reason for the status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<OperationReason>,
    /// The recorded conflict, when the operation is in conflict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict: Option<super::merge::RecordedConflict>,
    /// The asset transfer state, when the operation is an asset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset: Option<AssetState>,
    /// The millisecond at which the operation was enqueued.
    pub enqueued_at_ms: u64,
    /// The operation's sequence number in the outbox.
    pub sequence: u64,
}

impl OutboxOperation {
    /// Whether the operation is live: enqueued, and not yet resolved.
    #[must_use]
    pub fn is_live(&self) -> bool {
        !self.status.is_terminal()
    }
}

/// One operation to enqueue.
#[derive(Clone, Debug)]
pub struct NewOperation {
    /// The operation's identifier.
    pub id: OperationId,
    /// The operation this one depends on.
    pub depends_on: Option<OperationId>,
    /// The binding the operation was written under.
    pub binding: OperationBinding,
    /// The proposal the operation carries.
    pub payload: OperationPayload,
}

/// The receipt of one outbox mutation.
///
/// The receipt is content-free: it names an operation, its status, and its
/// reason. It never carries a proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationReceipt {
    /// The operation the receipt is about.
    pub id: OperationId,
    /// The operation's status after the mutation.
    pub status: OperationStatus,
    /// The content-free reason, when there is one.
    pub reason: Option<OperationReason>,
    /// The operation's sequence number.
    pub sequence: u64,
}

/// The counts of the outbox's operations, per status.
///
/// The status surface is content-free: it counts operations and names
/// nothing else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutboxStatus {
    /// Operations that are enqueued and not yet submitted.
    pub pending: usize,
    /// Operations that were submitted and whose outcome is unknown.
    pub in_flight: usize,
    /// Operations the server accepted.
    pub acknowledged: usize,
    /// Operations the server refused.
    pub rejected: usize,
    /// Operations that conflict with another operation.
    pub conflicted: usize,
}

impl OutboxStatus {
    /// The total number of operations the outbox holds.
    #[must_use]
    pub fn total(&self) -> usize {
        self.pending + self.in_flight + self.acknowledged + self.rejected + self.conflicted
    }
}

/// The content-free view of one operation.
///
/// The view carries the operation's identity, kind, status, and reason. It
/// carries no proposal: a caller can see *what happened to* an operation
/// without reading *what it proposed*.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationView {
    /// The operation's identifier.
    pub id: OperationId,
    /// The operation's kind tag.
    pub kind: OperationKind,
    /// The operation's status.
    pub status: OperationStatus,
    /// The content-free reason for the status.
    pub reason: Option<OperationReason>,
    /// The operation this one depends on.
    pub depends_on: Option<OperationId>,
    /// The guild scope the operation belongs to.
    pub guild_ref: String,
    /// The contract revision the operation was written under.
    pub contract_revision: u64,
    /// The operation's sequence number.
    pub sequence: u64,
    /// The millisecond at which the operation was enqueued.
    pub enqueued_at_ms: u64,
    /// The recorded conflict, summarized without its digests.
    pub conflict: Option<ConflictSummary>,
}

/// The closed resolutions an authorized caller may apply to a queued
/// operation.
///
/// The vocabulary is closed on purpose. Every variant is an explicit act by
/// an authorized caller, and none of them is implicit: the outbox never
/// reaps an operation on its own.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Resolution {
    /// Keep the operation exactly as it is.
    Retain,
    /// Put the operation back in the pending set for another attempt.
    ///
    /// Only a refused or a conflicted operation may be requeued.
    Requeue,
    /// Remove a refused or a conflicted operation from the outbox.
    ///
    /// This is the removal of a *refusal*: the operation was refused or it
    /// conflicts, and the authorized caller has dealt with it.
    ///
    /// A discard is refused while another operation depends on the one being
    /// discarded: the dependent would name a dependency the outbox does not
    /// hold, and it could never be sent. The caller resolves the dependent
    /// first, and the discard then succeeds. See [`Resolution::Remove`].
    Discard,
    /// Remove the operation from the outbox, whatever its status.
    ///
    /// This is the status-independent removal path. It exists because the
    /// other resolutions do not cover every operation: an **acknowledged**
    /// operation is the success case, it is the one that accumulates
    /// fastest, and without this path it would have no way to leave the
    /// outbox at all — a long-lived client would wedge its queue at
    /// [`MAX_OPERATIONS`] and could never make room again.
    ///
    /// The removal is still an explicit, authorized act: a service identity
    /// may not remove anything, because a writer that could bury its own
    /// acknowledgement or its own refusal could defeat the audit the
    /// outbox keeps.
    ///
    /// **A removal is refused while another operation depends on the one
    /// being removed.** A dependent names the operation it depends on, and
    /// removing that operation would leave the dependent with a dependency
    /// the outbox does not hold: every later drain would defer it with
    /// [`OperationReason::DependencyPending`], for ever, because no drain
    /// could ever satisfy a dependency that is not there. The refusal is
    /// [`OutboxError::DependencyPresent`], which names neither the operation
    /// nor the dependent. The caller finds the dependents in the outbox's own
    /// content-free views and resolves them first — the leaves first — and the
    /// removal then succeeds.
    Remove,
}

/// The source of the millisecond stamps the outbox records.
///
/// The clock exists for observability only. **No outbox behaviour depends on
/// it**: an operation never expires, and the merge never reads it. The
/// default clock is the system clock; a test injects its own.
pub type Clock = Box<dyn Fn() -> u64 + Send + Sync>;

/// The system clock, in milliseconds since the Unix epoch.
pub(crate) fn system_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok())
        .unwrap_or_default()
}
