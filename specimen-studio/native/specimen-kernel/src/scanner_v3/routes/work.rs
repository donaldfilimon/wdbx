//! Borrowed preflights: charge vector lengths before traversal and strings before
//! comparisons/copies. No arbitrary new identity limit, timing claim or mutation.
use super::*;
use crate::language::Expr;
impl Work {
    pub(super) fn population(&mut self, p: &Population) -> Result<(), RouteError> {
        for s in [&p.tenant, &p.specimen, &p.node_type] {
            self.charge(s.len())?;
        }
        Ok(())
    }
    pub(super) fn key(&mut self, k: &PatternKey) -> Result<(), RouteError> {
        self.charge(32)?;
        for s in [&k.encoder_version, &k.modality, &k.resolution] {
            self.charge(s.len())?;
        }
        Ok(())
    }
    fn strings(&mut self, strings: &[String]) -> Result<(), RouteError> {
        self.charge(strings.len())?;
        for s in strings {
            self.charge(s.len())?;
        }
        Ok(())
    }
    fn expressions(&mut self, trees: &[Expr]) -> Result<(), RouteError> {
        self.charge(1)?; // vector header, including empty argument vectors
        self.charge(trees.len())?;
        for tree in trees {
            match tree {
                Expr::Text { value } => self.charge(value.len())?,
                Expr::Call { name, args } => {
                    self.charge(name.len())?;
                    self.charge(args.len())?;
                    for arg in args {
                        self.expressions(arg)?;
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn target_ast(&mut self, ast: &ActionAst) -> Result<(), RouteError> {
        self.charge(1)?;
        // A selected entry can hold unbounded alternatives. Refuse in O(1)
        // from the borrowed vector length before visiting any of them.
        self.charge(ast.alternatives.len())?;
        self.expressions(&ast.bind)?;
        for alternative in &ast.alternatives {
            self.expressions(&alternative.action)?;
        }
        Ok(())
    }
    pub(super) fn semantic_ast(&mut self, ast: &SemanticAst) -> Result<(), RouteError> {
        self.charge(1)?;
        match ast {
            SemanticAst::Action(tokens) => {
                self.charge(tokens.len())?;
                for t in tokens {
                    self.charge(t.lemma().len())?;
                    self.charge(t.source().len())?;
                }
            }
            SemanticAst::And(a, b)
            | SemanticAst::Or(a, b)
            | SemanticAst::But(a, b)
            | SemanticAst::IfThen(a, b) => {
                self.semantic_ast(a)?;
                self.semantic_ast(b)?;
            }
            SemanticAst::NegatedAction { action, .. } => self.semantic_ast(action)?,
        }
        Ok(())
    }
    pub(super) fn origin(
        &mut self,
        a: &PreparedAction,
        path: &CandidatePath,
        captures: &[RouteCapture],
    ) -> Result<(), RouteError> {
        self.charge(1)?;
        self.charge(a.tokens().len())?;
        self.charge(a.origin().input_id().len())?;
        self.strings(a.origin().rule_refs())?;
        match path {
            CandidatePath::Structural(k) => self.key(k)?,
            CandidatePath::Grammar(s)
            | CandidatePath::Wildcard(s)
            | CandidatePath::Automaton { rule: s, .. } => self.charge(s.len())?,
        }
        self.charge(captures.len())?;
        for c in captures {
            self.charge(c.name.len())?;
            self.charge(match &c.value {
                CaptureValue::Text(s) | CaptureValue::Operator(s) => s.len(),
                CaptureValue::Bytes(s) => s.len(),
                CaptureValue::Number(_) => 32,
            })?;
        }
        Ok(())
    }
}
