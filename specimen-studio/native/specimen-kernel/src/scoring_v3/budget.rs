use super::*;
use crate::logical_charge_v1::LogicalChargeV1;
use crate::{identity::*, language::Expr, language_v3::*, scanner_v3::routes::*};
#[derive(Debug, Clone, Copy)]
pub struct ScoringBudget {
    pub contributions: usize,
    pub work: usize,
    pub payload_bytes: usize,
    pub draws: usize,
}
impl Default for ScoringBudget {
    fn default() -> Self {
        Self {
            contributions: 100_000,
            work: 1_000_000,
            payload_bytes: 16 * 1024 * 1024,
            draws: 100_000,
        }
    }
}
pub(super) struct Work {
    pub remaining: usize,
    payload: usize,
}
impl Work {
    pub fn new(b: ScoringBudget) -> Result<Self, ScoringError> {
        if b.contributions > 100_000
            || b.work == 0
            || b.work > 1_000_000
            || b.payload_bytes == 0
            || b.payload_bytes > 16 * 1024 * 1024
            || b.draws > 100_000
        {
            return Err(ScoringError::InvalidBudget);
        }
        Ok(Self {
            remaining: b.work,
            payload: b.payload_bytes,
        })
    }
    pub fn charge(&mut self, n: usize) -> Result<(), ScoringError> {
        self.remaining = self
            .remaining
            .checked_sub(n)
            .ok_or(ScoringError::BudgetExceeded)?;
        Ok(())
    }
    pub fn bytes(&mut self, n: usize) -> Result<(), ScoringError> {
        self.charge(n)?;
        self.payload = self
            .payload
            .checked_sub(n)
            .ok_or(ScoringError::BudgetExceeded)?;
        Ok(())
    }
    pub fn text(&mut self, s: &str) -> Result<(), ScoringError> {
        if s.len() > 8000 {
            return Err(ScoringError::BudgetExceeded);
        }
        self.bytes(s.len())
    }
    pub fn label(&mut self, s: &str) -> Result<(), ScoringError> {
        if s.is_empty() {
            return Err(ScoringError::InvalidContribution);
        }
        self.text(s)
    }
    pub fn items<T: LogicalChargeV1>(&mut self, v: &[T]) -> Result<(), ScoringError> {
        self.charge(v.len())?;
        self.payload = self
            .payload
            .checked_sub(
                v.len()
                    .checked_mul(T::UNIT_BYTES)
                    .ok_or(ScoringError::BudgetExceeded)?,
            )
            .ok_or(ScoringError::BudgetExceeded)?;
        Ok(())
    }
    pub fn origin(&mut self, o: &Origin) -> Result<(), ScoringError> {
        self.label(o.input_id())?;
        self.items(o.rule_refs())?;
        for s in o.rule_refs() {
            self.text(s)?
        }
        Ok(())
    }
    pub fn path(&mut self, p: &CandidatePath) -> Result<(), ScoringError> {
        match p {
            CandidatePath::Structural(k) => self.key(k)?,
            CandidatePath::Grammar(s)
            | CandidatePath::Wildcard(s)
            | CandidatePath::Automaton { rule: s, .. } => self.label(s)?,
        }
        Ok(())
    }
    pub fn key(&mut self, k: &PatternKey) -> Result<(), ScoringError> {
        self.label(&k.encoder_version)?;
        self.label(&k.modality)?;
        self.label(&k.resolution)
    }
    pub fn captures(&mut self, cs: &[RouteCapture]) -> Result<(), ScoringError> {
        self.items(cs)?;
        for c in cs {
            self.label(&c.name)?;
            if c.span.start > c.span.end {
                return Err(ScoringError::InvalidContribution);
            }
            match &c.value {
                CaptureValue::Number(_) => {}
                CaptureValue::Operator(s) | CaptureValue::Text(s) => self.text(s)?,
                CaptureValue::Bytes(b) => {
                    if b.len() > 8000 {
                        return Err(ScoringError::BudgetExceeded);
                    }
                    self.bytes(b.len())?
                }
            }
        }
        Ok(())
    }
    pub fn action(&mut self, tree: &[Expr]) -> Result<(), ScoringError> {
        let mut nodes = 0;
        self.tree(tree, 1, &mut nodes)
    }
    fn tree(&mut self, tree: &[Expr], depth: usize, nodes: &mut usize) -> Result<(), ScoringError> {
        if depth > 16 || tree.len() > 4096usize.saturating_sub(*nodes) {
            return Err(ScoringError::BudgetExceeded);
        }
        self.items(tree)?;
        for e in tree {
            if *nodes >= 4096 {
                return Err(ScoringError::BudgetExceeded);
            }
            *nodes += 1;
            match e {
                Expr::Text { value } => self.text(value)?,
                Expr::Call { name, args } => {
                    // Argument containers are structural work even when empty.
                    // Count them so a single Expr cannot hide an unbounded fanout.
                    if args.len() > 4096usize.saturating_sub(*nodes) {
                        return Err(ScoringError::BudgetExceeded);
                    }
                    *nodes += args.len();
                    self.label(name)?;
                    self.items(args)?;
                    for a in args {
                        self.tree(a, depth + 1, nodes)?
                    }
                }
            }
        }
        Ok(())
    }
    pub fn ast(&mut self, a: &ActionAst) -> Result<(), ScoringError> {
        self.action(&a.bind)?;
        self.items(&a.alternatives)?;
        for alt in &a.alternatives {
            if !alt.weight.is_finite() || alt.weight < 0.0 {
                return Err(ScoringError::InvalidWeight);
            }
            self.action(&alt.action)?
        }
        Ok(())
    }
    pub fn prepared(&mut self, a: &PreparedAction) -> Result<(), ScoringError> {
        self.text(a.original())?;
        self.origin(a.origin())?;
        self.items(a.tokens())?;
        for t in a.tokens() {
            self.text(t.source())?;
            self.text(t.lemma())?;
            self.items(t.rule_refs())?;
            for s in t.rule_refs() {
                self.text(s)?
            }
        }
        self.items(a.captures())?;
        for c in a.captures() {
            self.text(c.source())?
        }
        self.items(a.operators())?;
        for s in a.operators() {
            self.text(s)?
        }
        self.semantic(a.ast(), 1)?;
        self.items(a.definitions())?;
        for d in a.definitions() {
            self.text(d.rule_ref())?;
            self.text(d.definition())?
        }
        self.items(a.charges())?;
        for c in a.charges() {
            if !c.valence().is_finite() || !c.intensity().is_finite() {
                return Err(ScoringError::InvalidEvidence);
            }
            self.text(c.rule_ref())?
        }
        Ok(())
    }
    fn semantic(&mut self, a: &SemanticAst, depth: usize) -> Result<(), ScoringError> {
        self.charge(1)?;
        if depth > 16 {
            return Err(ScoringError::BudgetExceeded);
        }
        match a {
            SemanticAst::Action(t) => self.items(t)?,
            SemanticAst::And(a, b)
            | SemanticAst::Or(a, b)
            | SemanticAst::But(a, b)
            | SemanticAst::IfThen(a, b) => {
                self.semantic(a, depth + 1)?;
                self.semantic(b, depth + 1)?
            }
            SemanticAst::NegatedAction { action, .. } => self.semantic(action, depth + 1)?,
        }
        Ok(())
    }
}
