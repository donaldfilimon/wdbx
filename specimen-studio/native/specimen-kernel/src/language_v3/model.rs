use super::CanonicalDecimal;

/// Half-open byte span into the preserved original UTF-8 input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}
#[derive(Debug, Clone)]
pub struct Input {
    pub id: String,
    pub text: String,
}
#[derive(Debug, Clone)]
pub struct ContextSnapshot {
    pub population: String,
    pub utility_version: String,
    pub tokens: Vec<String>,
    pub conditions: Vec<(String, bool)>,
}
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    pub variants_per_clause: usize,
    pub total_variants: usize,
    pub draws: usize,
    pub arithmetic_steps: usize,
    /// Shared rewrite-validation work allowance; 1..=1_000_000.
    pub work_steps: usize,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            variants_per_clause: 16,
            total_variants: 256,
            draws: 4096,
            arithmetic_steps: 500,
            work_steps: 1_000_000,
        }
    }
}
/// Host entropy only; values must be in [0, 1).
pub trait DrawSource {
    fn draw(&mut self) -> f64;
}
pub struct FixedDraw(pub u64);
impl DrawSource for FixedDraw {
    fn draw(&mut self) -> f64 {
        (self.0 >> 11) as f64 / 9_007_199_254_740_992.0
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Word,
    Quoted,
    Number,
    Operator,
    Punctuation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Subject,
    Predicate,
    Object,
    Other,
    Connective,
    Negation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub(super) source: String,
    pub(super) lemma: String,
    pub(super) span: Span,
    pub(super) kind: TokenKind,
    pub(super) role: Role,
    pub(super) decimal: Option<CanonicalDecimal>,
    pub(super) rule_refs: Vec<String>,
}
impl Token {
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn lemma(&self) -> &str {
        &self.lemma
    }
    pub fn span(&self) -> Span {
        self.span
    }
    pub fn kind(&self) -> TokenKind {
        self.kind
    }
    pub fn quoted(&self) -> bool {
        self.kind == TokenKind::Quoted
    }
    pub fn role(&self) -> Role {
        self.role
    }
    pub fn decimal(&self) -> Option<&CanonicalDecimal> {
        self.decimal.as_ref()
    }
    pub fn rule_refs(&self) -> &[String] {
        &self.rule_refs
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connective {
    And,
    Or,
    But,
    If,
    Then,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticAst {
    Action(Vec<Token>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    But(Box<Self>, Box<Self>),
    IfThen(Box<Self>, Box<Self>),
    NegatedAction { uncertain: bool, action: Box<Self> },
}
impl SemanticAst {
    /// Profile lexical count: punctuation excluded, explicit controls counted once.
    pub fn token_count(&self) -> usize {
        match self {
            Self::Action(t) => t
                .iter()
                .filter(|t| t.kind() != TokenKind::Punctuation)
                .count(),
            Self::And(a, b) | Self::Or(a, b) | Self::But(a, b) | Self::IfThen(a, b) => {
                1 + a.token_count() + b.token_count()
            }
            Self::NegatedAction { action, .. } => 1 + action.token_count(),
        }
    }
    pub fn depth(&self) -> usize {
        match self {
            Self::Action(_) => 1,
            Self::And(a, b) | Self::Or(a, b) | Self::But(a, b) | Self::IfThen(a, b) => {
                1 + a.depth().max(b.depth())
            }
            Self::NegatedAction { action, .. } => 1 + action.depth(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Slot {
    Resolved(String),
    Unresolved,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spo {
    pub(super) subject: Slot,
    pub(super) predicate: Slot,
    pub(super) object: Slot,
}
impl Spo {
    pub fn subject(&self) -> &Slot {
        &self.subject
    }
    pub fn predicate(&self) -> &Slot {
        &self.predicate
    }
    pub fn object(&self) -> &Slot {
        &self.object
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalDisposition {
    None,
    Satisfied,
    Refuted,
    Unresolved,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    pub(super) source: String,
    pub(super) span: Span,
    pub(super) decimal: CanonicalDecimal,
}
impl Capture {
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn span(&self) -> Span {
        self.span
    }
    pub fn decimal(&self) -> &CanonicalDecimal {
        &self.decimal
    }
}
#[derive(Debug, Clone)]
pub struct Definition {
    pub(super) rule_ref: String,
    pub(super) definition: String,
    pub(super) span: Span,
}
impl Definition {
    pub fn rule_ref(&self) -> &str {
        &self.rule_ref
    }
    pub fn definition(&self) -> &str {
        &self.definition
    }
    pub fn span(&self) -> Span {
        self.span
    }
}
#[derive(Debug, Clone)]
pub struct Charge {
    pub(super) rule_ref: String,
    pub(super) valence: f64,
    pub(super) intensity: f64,
    pub(super) span: Span,
}
impl Charge {
    pub fn rule_ref(&self) -> &str {
        &self.rule_ref
    }
    pub fn valence(&self) -> f64 {
        self.valence
    }
    pub fn intensity(&self) -> f64 {
        self.intensity
    }
    pub fn span(&self) -> Span {
        self.span
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    pub(super) input_id: String,
    pub(super) clause_id: usize,
    pub(super) action_id: usize,
    pub(super) fanout_id: usize,
    pub(super) rule_refs: Vec<String>,
}
impl Origin {
    pub fn input_id(&self) -> &str {
        &self.input_id
    }
    pub fn clause_id(&self) -> usize {
        self.clause_id
    }
    pub fn action_id(&self) -> usize {
        self.action_id
    }
    pub fn fanout_id(&self) -> usize {
        self.fanout_id
    }
    pub fn rule_refs(&self) -> &[String] {
        &self.rule_refs
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteHint {
    Lexical,
    Arithmetic,
    Conditional,
    Inhibited,
    Unresolved,
}
#[derive(Debug, Clone)]
pub struct PreparedAction {
    pub(super) original: String,
    pub(super) span: Span,
    pub(super) tokens: Vec<Token>,
    pub(super) ast: SemanticAst,
    pub(super) captures: Vec<Capture>,
    pub(super) operators: Vec<String>,
    pub(super) spo: Spo,
    pub(super) inhibited: bool,
    pub(super) uncertain_negation: bool,
    pub(super) conditional: ConditionalDisposition,
    pub(super) unresolved_frame: bool,
    pub(super) value: Option<f64>,
    pub(super) definitions: Vec<Definition>,
    pub(super) charges: Vec<Charge>,
    pub(super) origin: Origin,
    pub(super) routes: Vec<RouteHint>,
}
impl PreparedAction {
    pub fn original(&self) -> &str {
        &self.original
    }
    pub fn span(&self) -> Span {
        self.span
    }
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }
    pub fn ast(&self) -> &SemanticAst {
        &self.ast
    }
    pub fn captures(&self) -> &[Capture] {
        &self.captures
    }
    pub fn operators(&self) -> &[String] {
        &self.operators
    }
    pub fn spo(&self) -> &Spo {
        &self.spo
    }
    pub fn inhibited(&self) -> bool {
        self.inhibited
    }
    pub fn uncertain_negation(&self) -> bool {
        self.uncertain_negation
    }
    pub fn conditional(&self) -> ConditionalDisposition {
        self.conditional
    }
    pub fn unresolved_frame(&self) -> bool {
        self.unresolved_frame
    }
    pub fn executable(&self) -> bool {
        !self.inhibited
            && !self.unresolved_frame
            && matches!(
                self.conditional,
                ConditionalDisposition::None | ConditionalDisposition::Satisfied
            )
    }
    pub fn arithmetic_value(&self) -> Option<f64> {
        self.value
    }
    pub fn definitions(&self) -> &[Definition] {
        &self.definitions
    }
    pub fn charges(&self) -> &[Charge] {
        &self.charges
    }
    pub fn origin(&self) -> &Origin {
        &self.origin
    }
    pub fn routes(&self) -> &[RouteHint] {
        &self.routes
    }
    pub fn complexity_token_count(&self) -> usize {
        self.ast.token_count()
    }
}
#[derive(Debug, Clone)]
pub struct PreparedClause {
    pub(super) span: Span,
    pub(super) variants: Vec<PreparedAction>,
}
impl PreparedClause {
    pub fn span(&self) -> Span {
        self.span
    }
    pub fn variants(&self) -> &[PreparedAction] {
        &self.variants
    }
}
#[derive(Debug, Clone)]
pub struct Prepared {
    pub(super) input: Input,
    pub(super) clauses: Vec<PreparedClause>,
    pub(super) ast: SemanticAst,
    pub(super) draws: usize,
}
impl Prepared {
    pub fn original(&self) -> &str {
        &self.input.text
    }
    pub fn input_id(&self) -> &str {
        &self.input.id
    }
    pub fn clauses(&self) -> &[PreparedClause] {
        &self.clauses
    }
    pub fn ast(&self) -> &SemanticAst {
        &self.ast
    }
    pub fn draws_used(&self) -> usize {
        self.draws
    }
}
