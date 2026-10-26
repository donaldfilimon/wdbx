//! Pure shared admission and completed graph plans. The host must own one broker
//! per runtime domain and actually fence revoked workers before acknowledging.
//! Borrowed comparison snapshots remain host-owned and are not measured here.
mod attachments;
mod broker;
mod budget;
mod compose;
use crate::{identity::*, language_v3::*, scanner_v3::routes::*, scoring_v3::*};
pub use attachments::*;
pub use broker::*;
pub use compose::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid limit or metadata")]
    Invalid,
    #[error("scope mismatch")]
    Scope,
    #[error("stale fencing generation")]
    StaleGeneration,
    #[error("unknown or reused cycle/job")]
    Identity,
    #[error("admission closed")]
    Closed,
    #[error("pending, active or undiscovered work remains")]
    Incomplete,
    #[error("explicit capacity backpressure")]
    Backpressure,
    #[error("invalid or already consumed lease")]
    InvalidLease,
    #[error("revoked work remains quarantined")]
    Revoked,
    #[error("contribution does not match comparison")]
    Binding,
    #[error("repeated non-progressing attachment")]
    Repeated,
    #[error("attachment depth exceeded")]
    Depth,
    #[error("missing reference or cyclic graph")]
    Graph,
    #[error(transparent)]
    Scoring(#[from] ScoringError),
}
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub active: usize,
    pub pending: usize,
    pub owned_bytes: usize,
    pub operation_work: usize,
    pub trace: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            active: 1000,
            pending: 100_000,
            owned_bytes: 16 * 1024 * 1024,
            operation_work: 1_000_000,
            trace: 100_000,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct CycleLimit {
    pub jobs: usize,
    pub deadline: Option<u64>,
}
impl Default for CycleLimit {
    fn default() -> Self {
        Self {
            jobs: 100_000,
            deadline: None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Foreground,
    Maintenance,
}
/// Immutable, host-owned binding. One job is exactly one entry/origin comparison.
/// This public descriptor declares host intent; only scorer contributions prove
/// local scoring, and neither descriptor nor broker authenticates credentials.
pub struct Comparison<'a> {
    pub record: &'a EntryRecord,
    pub source: &'a PreparedAction,
    pub target: &'a PreparedAction,
    pub origin: &'a CandidateOrigin,
    pub parent_lineage: &'a str,
    pub dependencies: &'a [u128],
    pub attachment_group: Option<u128>,
    pub kind: ContributionKind,
}
#[derive(Clone, Copy)]
pub struct Job<'a> {
    pub id: u128,
    pub comparison: &'a Comparison<'a>,
    pub priority: Priority,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelReason {
    Requested,
    Deadline,
    Restart,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleStatus {
    Open,
    Ready,
    Partial(CancelReason),
    Composed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub pending: usize,
    pub active: usize,
    pub quarantined: usize,
    pub owned_bytes: usize,
}
