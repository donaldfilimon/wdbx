use crate::{
    identity::*,
    language_v3::{script::Scope, *},
    text_v3::TextError,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimilarityBypass {
    RequireFullScan,
    Declared,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WildcardMode {
    Prefix,
    Contains,
}
/// Supplied typed grammar over producer tokens. This does not parse language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrammarAtom {
    Literal(String),
    Quoted(String),
    Number(String),
    Operator { name: String, allowed: Vec<String> },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteKind {
    Grammar(Vec<GrammarAtom>),
    Wildcard {
        mode: WildcardMode,
        literal: String,
        bypass: SimilarityBypass,
    },
    /// Program runs on preserved original UTF-8 bytes through the producer Tape
    /// handler. Matching requires exact output bytes. Not a dynamic script binder.
    Automaton {
        handler: String,
        program: String,
        expected: Vec<u8>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRule {
    pub id: String,
    pub entries: Vec<EntryRef>,
    pub scopes: Vec<Scope>,
    pub kind: RouteKind,
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RouteError {
    #[error("population mismatch")]
    PopulationMismatch,
    #[error("undeclared scope")]
    UndeclaredScope,
    #[error("missing concrete entry")]
    MissingEntry,
    #[error("invalid or duplicate route rule")]
    InvalidRule,
    #[error("route index limit exceeded")]
    RuleLimit,
    #[error("query budget exceeded")]
    BudgetExceeded,
    #[error("invalid query budget")]
    InvalidBudget,
    #[error("stale route registry")]
    StaleRegistry,
    #[error(transparent)]
    Language(#[from] LanguageError),
    #[error(transparent)]
    Text(#[from] TextError),
}
#[derive(Debug, Clone, Copy)]
pub struct RouteBudget {
    /// Aggregate logical work, including worst-case Tape reservations, not time.
    pub work_steps: usize,
    /// Aggregate worst-case Tape output reservations (1024 bytes per invocation).
    pub output_bytes: usize,
    pub origins: usize,
    pub tape_steps: usize,
}
impl Default for RouteBudget {
    fn default() -> Self {
        Self {
            work_steps: 1_000_000,
            output_bytes: 65536,
            origins: 65536,
            tape_steps: 10000,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomatonStatus {
    Matched,
    Blocked,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidatePath {
    Structural(PatternKey),
    Grammar(String),
    Wildcard(String),
    Automaton {
        rule: String,
        status: AutomatonStatus,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureValue {
    Number(CanonicalDecimal),
    Operator(String),
    Text(String),
    Bytes(Vec<u8>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCapture {
    pub name: String,
    pub span: Span,
    pub value: CaptureValue,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraints {
    pub inhibited: bool,
    pub uncertain_negation: bool,
    pub unresolved_frame: bool,
    pub conditional: ConditionalDisposition,
    pub executable: bool,
}
impl From<&PreparedAction> for Constraints {
    fn from(a: &PreparedAction) -> Self {
        Self {
            inhibited: a.inhibited(),
            uncertain_negation: a.uncertain_negation(),
            unresolved_frame: a.unresolved_frame(),
            conditional: a.conditional(),
            executable: a.executable(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateOrigin {
    pub origin: Origin,
    pub span: Span,
    pub token_spans: Vec<Span>,
    pub constraints: Constraints,
    pub path: CandidatePath,
    pub captures: Vec<RouteCapture>,
    pub bypass: SimilarityBypass,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub entry: EntryRef,
    pub node: NodeRef,
    pub original: String,
    /// Target alternatives retain their inhibition and exact weights as supplied
    /// to the identity table; a route does not authorize any alternative.
    pub target_ast: ActionAst,
    pub origins: Vec<CandidateOrigin>,
}
