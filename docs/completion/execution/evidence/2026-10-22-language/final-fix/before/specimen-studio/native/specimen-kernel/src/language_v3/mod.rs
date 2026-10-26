//! Opt-in bounded preparation and effect planning; never changes legacy language.
mod arithmetic;
mod model;
mod prepare;
pub mod script;
mod tables;
mod tokenize;
pub use arithmetic::{CanonicalDecimal, evaluate_math};
pub use model::*;
pub use prepare::prepare;
pub use tables::*;

/// Failures carry no partial prepared tree or effect plan.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LanguageError {
    #[error("input character limit")]
    InputLimit,
    #[error("clause limit")]
    ClauseLimit,
    #[error("variant limit")]
    VariantLimit,
    #[error("AST depth limit")]
    DepthLimit,
    #[error("invalid budget")]
    InvalidBudget,
    #[error("utility snapshot invalid")]
    InvalidTables,
    #[error("population or utility version mismatch")]
    ContextMismatch,
    #[error("context limit")]
    ContextLimit,
    #[error("unclosed quote")]
    UnclosedQuote,
    #[error("syntax")]
    Syntax,
    #[error("invalid exact decimal")]
    Decimal,
    #[error("division by zero")]
    DivisionByZero,
    #[error("nonfinite arithmetic result")]
    NonFinite,
    #[error("type mismatch")]
    TypeMismatch,
    #[error("step budget")]
    StepBudget,
    #[error("draw budget or invalid host draw")]
    DrawBudget,
    #[error("unsafe utility substitution")]
    UnsafeSubstitution,
    #[error("invalid registry")]
    InvalidRegistry,
    #[error("unknown or out of scope call")]
    InvalidScope,
    #[error("undeclared subsystem contact")]
    UndeclaredContact,
    #[error("stale registry")]
    StaleRegistry,
    #[error("dynamic subsystem contact")]
    DynamicContact,
    #[error("action inhibited or unresolved")]
    ActionBlocked,
    #[error("malformed tape program")]
    TapeSyntax,
    #[error("output budget")]
    OutputBudget,
}
