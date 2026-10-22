//! Draining the outbox on reconnect.
//!
//! # The shape of a drain
//!
//! A drain is the reconnect path, and it runs in four explicit steps:
//!
//! 1. **Reauthorize.** Every live operation is re-validated against the
//!    *current* server policy and consent, and against the canonical store's
//!    own write gate. Each operation gets its own verdict
//!    ([`ReauthorizeOutcome`]). An operation the current policy refuses is
//!    not sent, and it is not dropped: it stays in the outbox with a
//!    terminal status and a reason.
//! 2. **Merge.** The edit operations that were authorized are merged. Edits
//!    over disjoint fields of one object merge automatically; edits over the
//!    same field, or over incompatible versions of one object, are recorded
//!    as conflicts and both branches are preserved.
//! 3. **Submit.** The proposals are handed to the caller's
//!    [`OperationSink`], which stands in for the server. The outbox has no
//!    transport of its own.
//! 4. **Record.** The verdicts are recorded, and the outbox is written back
//!    durably.
//!
//! # Dependencies
//!
//! An operation may depend on another operation. A dependent operation is
//! submitted only once its dependency is acknowledged, so a proposal never
//! overtakes the proposal it depends on. A dependent operation whose
//! dependency is not acknowledged stays pending with a
//! [`OperationReason::DependencyPending`]; it is unsent work, and it is
//! preserved.
//!
//! # Crashes
//!
//! The outbox records a submission's outcome only after the sink has
//! answered. A crash between the send and the record therefore leaves the
//! operation live, and the next drain resubmits it under the *same*
//! operation identifier. The identifier is the idempotency key, so a server
//! that deduplicates on it never applies one proposal twice. This slice
//! cannot make that deduplication happen; it can only make the retry
//! carry the same key.

use super::reauthorize::ReauthorizeOutcome;
use super::types::{OperationId, OperationPayload, OperationReason, OperationStatus};
/// What one submission carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submission {
    /// Every operation the submission covers, in identifier order.
    ///
    /// One operation submits alone. Several operations submit together when
    /// their edits merged into one object.
    pub contributors: Vec<OperationId>,
    /// The proposal to submit.
    pub payload: OperationPayload,
}

/// The server's verdict on one submitted proposal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmissionVerdict {
    /// The server accepted the proposal and bound it to a canonical episode.
    Accepted,
    /// The server refused the proposal, for a content-free reason.
    Refused(super::types::RefusalReason),
    /// The transport failed, so the outcome is unknown.
    Unavailable,
}

/// Where a drained proposal goes.
///
/// The outbox has no transport: the caller supplies the sink that stands in
/// for the server, and the outbox records what the sink reports. A sink
/// that talks to a real server must map the server's own duplicate-replay
/// refusal onto [`SubmissionVerdict::Accepted`] when it can confirm the
/// operation is already bound, because a resubmission after a crash is not
/// a new proposal.
pub trait OperationSink {
    /// The server's verdict on one submitted proposal.
    fn submit(&mut self, submission: &Submission) -> SubmissionVerdict;
}

impl<T: OperationSink + ?Sized> OperationSink for &mut T {
    fn submit(&mut self, submission: &Submission) -> SubmissionVerdict {
        (**self).submit(submission)
    }
}

/// The outcome of one operation in a drain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationOutcome {
    /// The operation the outcome is about.
    pub operation_id: OperationId,
    /// The reauthorization verdict the current policy gave.
    pub reauthorization: ReauthorizeOutcome,
    /// Whether the operation was handed to the sink.
    pub submitted: bool,
    /// The status the operation ended the drain in.
    pub status: OperationStatus,
    /// The content-free reason the operation ended the drain in.
    pub reason: Option<OperationReason>,
}

/// The report of one drain.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DrainReport {
    /// The per-operation outcomes, in operation order.
    pub outcomes: Vec<OperationOutcome>,
    /// The merge the drain applied, in canonical order.
    pub merge: super::merge::MergePlan,
    /// The number of proposals handed to the sink.
    pub submitted: usize,
    /// The number of proposals the server accepted.
    pub accepted: usize,
    /// The number of proposals the server refused.
    pub refused: usize,
    /// The number of proposals whose outcome is unknown.
    pub unavailable: usize,
}

/// The counts a drain accumulates while it submits.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DrainCounts {
    /// The number of proposals handed to the sink.
    pub submitted: usize,
    /// The number of proposals the server accepted.
    pub accepted: usize,
    /// The number of proposals the server refused.
    pub refused: usize,
    /// The number of proposals whose outcome is unknown.
    pub unavailable: usize,
}
