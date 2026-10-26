//! Opt-in exact scoring and scoped contribution elections. These pure plans do
//! not authorize host effects, execute legacy trees, or update durable ATP.
mod arbitration;
mod draws;
pub use arbitration::*;
mod contribution;
pub use contribution::*;
mod budget;
pub use budget::ScoringBudget;
mod election;
mod math;
pub use election::*;
mod policy;
use crate::scanner_v3::ScanError;
pub use draws::*;
pub use policy::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ScoringError {
    #[error(transparent)]
    Scan(#[from] ScanError),
    #[error("invalid exact policy configuration")]
    InvalidPolicy,
    #[error("inconsistent scanner evidence")]
    InvalidEvidence,
    #[error("nonfinite or negative weight")]
    InvalidWeight,
    #[error("invalid scoring budget")]
    InvalidBudget,
    #[error("scoring budget exceeded")]
    BudgetExceeded,
    #[error("draw input exhausted")]
    DrawExhausted,
    #[error("stale or inconsistent prepared binding")]
    BindingMismatch,
    #[error("invalid contribution metadata or references")]
    InvalidContribution,
}
