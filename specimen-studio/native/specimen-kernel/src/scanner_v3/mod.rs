//! Exact full-feature evidence, independent of candidate structural keys.
mod rational;
pub mod routes;
use crate::language_v3::*;
pub use rational::{Rational, Totals, reduce_totals};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ScanError {
    #[error("checked i128 arithmetic overflow")]
    Overflow,
    #[error("rational denominator must be positive")]
    InvalidDenominator,
    #[error("unsupported modality")]
    UnsupportedModality,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    Match,
    Partial,
    Missing,
    Mismatch,
    NegationConflict,
    UncertainNegation,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub name: &'static str,
    pub weight: i128,
    pub similarity: Rational,
    pub dissimilarity: Rational,
    pub reason: Reason,
    pub left_spans: Vec<Span>,
    pub right_spans: Vec<Span>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub version: &'static str,
    pub components: [Component; 4],
    pub totals: Totals,
    /// Guards remain evidence; scores never resolve conditions or authorize actions.
    pub left_constraints: routes::Constraints,
    pub right_constraints: routes::Constraints,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Lexeme {
    Quoted(String),
    Number(CanonicalDecimal),
    Lexical(String),
    Operator(String),
    Punctuation(String),
}
fn lexeme(t: &Token) -> Lexeme {
    if let Some(n) = t.decimal() {
        Lexeme::Number(n.clone())
    } else {
        match t.kind() {
            TokenKind::Quoted => Lexeme::Quoted(t.lemma().into()),
            TokenKind::Operator => Lexeme::Operator(t.lemma().into()),
            TokenKind::Punctuation => Lexeme::Punctuation(t.lemma().into()),
            _ => Lexeme::Lexical(t.lemma().into()),
        }
    }
}
fn bag<T: Ord>(values: impl IntoIterator<Item = T>) -> Result<BTreeMap<T, i128>, ScanError> {
    let mut bag = BTreeMap::new();
    for v in values {
        let count = bag.entry(v).or_insert(0_i128);
        *count = count.checked_add(1).ok_or(ScanError::Overflow)?;
    }
    Ok(bag)
}
fn jaccard<T: Ord>(a: BTreeMap<T, i128>, b: BTreeMap<T, i128>) -> Result<(i128, i128), ScanError> {
    let mut intersection = 0_i128;
    let mut union = 0_i128;
    for (key, count) in &a {
        intersection = intersection
            .checked_add((*count).min(*b.get(key).unwrap_or(&0)))
            .ok_or(ScanError::Overflow)?;
        union = union
            .checked_add((*count).max(*b.get(key).unwrap_or(&0)))
            .ok_or(ScanError::Overflow)?;
    }
    for (key, count) in &b {
        if !a.contains_key(key) {
            union = union.checked_add(*count).ok_or(ScanError::Overflow)?;
        }
    }
    Ok((intersection, union))
}
fn component(
    name: &'static str,
    weight: i128,
    ratio: (i128, i128),
    spans: (Vec<Span>, Vec<Span>),
    reason: Option<Reason>,
) -> Result<Component, ScanError> {
    let (n, d) = ratio;
    let similarity = if d == 0 {
        Rational::whole(0)
    } else {
        Rational::new(n.checked_mul(weight).ok_or(ScanError::Overflow)?, d)?
    };
    Ok(Component {
        name,
        weight,
        similarity,
        dissimilarity: Rational::whole(weight).checked_sub(similarity)?,
        reason: reason.unwrap_or(if d == 0 {
            Reason::Missing
        } else if n == d {
            Reason::Match
        } else if n == 0 {
            Reason::Mismatch
        } else {
            Reason::Partial
        }),
        left_spans: spans.0,
        right_spans: spans.1,
    })
}
fn leaves<'a>(tree: &'a SemanticAst, out: &mut Vec<&'a Token>) {
    match tree {
        SemanticAst::Action(tokens) => {
            out.extend(tokens.iter().filter(|t| t.kind() != TokenKind::Punctuation))
        }
        SemanticAst::And(a, b)
        | SemanticAst::Or(a, b)
        | SemanticAst::But(a, b)
        | SemanticAst::IfThen(a, b) => {
            leaves(a, out);
            leaves(b, out);
        }
        SemanticAst::NegatedAction { action, .. } => leaves(action, out),
    }
}
// SPO is body-local. Antecedent negation/uncertainty belongs to its guard,
// whereas an uncertain affected body supplies no known polarity agreement.
fn body_polarity(tree: &SemanticAst) -> Option<bool> {
    match tree {
        SemanticAst::Action(_) => Some(false),
        SemanticAst::IfThen(_, body) => body_polarity(body),
        SemanticAst::NegatedAction {
            uncertain: true, ..
        } => None,
        SemanticAst::NegatedAction {
            uncertain: false, ..
        } => Some(true),
        SemanticAst::And(a, b) | SemanticAst::Or(a, b) | SemanticAst::But(a, b) => {
            let a = body_polarity(a);
            let b = body_polarity(b);
            if a == b { a } else { None }
        }
    }
}
pub fn scan_modality(
    modality: &str,
    a: &PreparedAction,
    b: &PreparedAction,
) -> Result<Evidence, ScanError> {
    if modality != "text" {
        return Err(ScanError::UnsupportedModality);
    }
    scan(a, b)
}
/// No class/key argument exists: every comparison consumes full validated features.
pub fn scan(a: &PreparedAction, b: &PreparedAction) -> Result<Evidence, ScanError> {
    let mut left_tokens = vec![];
    let mut right_tokens = vec![];
    leaves(a.ast(), &mut left_tokens);
    leaves(b.ast(), &mut right_tokens);
    let left = left_tokens.iter().map(|t| lexeme(t)).collect::<Vec<_>>();
    let right = right_tokens.iter().map(|t| lexeme(t)).collect::<Vec<_>>();
    let spans = || {
        (
            left_tokens.iter().map(|t| t.span()).collect(),
            right_tokens.iter().map(|t| t.span()).collect(),
        )
    };
    let overlap = jaccard(bag(left.iter().cloned())?, bag(right.iter().cloned())?)?;
    let bigrams = jaccard(
        bag(left.windows(2).map(|v| (v[0].clone(), v[1].clone())))?,
        bag(right.windows(2).map(|v| (v[0].clone(), v[1].clone())))?,
    )?;
    let lhs = [a.spo().subject(), a.spo().predicate(), a.spo().object()];
    let rhs = [b.spo().subject(), b.spo().predicate(), b.spo().object()];
    let matched = lhs
        .iter()
        .zip(rhs)
        .filter(|(x, y)| matches!((x,y),(Slot::Resolved(x),Slot::Resolved(y)) if x==y))
        .count() as i128;
    let polarity_a = body_polarity(a.ast());
    let polarity_b = body_polarity(b.ast());
    let uncertain = polarity_a.is_none() || polarity_b.is_none();
    let conflict = matches!((polarity_a, polarity_b), (Some(a), Some(b)) if a != b);
    let role_spans = |a: &PreparedAction| {
        a.tokens()
            .iter()
            .filter(|t| matches!(t.role(), Role::Subject | Role::Predicate | Role::Object))
            .map(Token::span)
            .collect()
    };
    let operands = !a.captures().is_empty() && !b.captures().is_empty();
    let exact = operands
        && a.captures()
            .iter()
            .map(Capture::decimal)
            .eq(b.captures().iter().map(Capture::decimal))
        && a.operators() == b.operators();
    let components = [
        component("lemma-multiset", 40, overlap, spans(), None)?,
        component("ordered-bigram-multiset", 20, bigrams, spans(), None)?,
        component(
            "spo",
            20,
            (if conflict || uncertain { 0 } else { matched }, 3),
            (role_spans(a), role_spans(b)),
            if uncertain {
                Some(Reason::UncertainNegation)
            } else if conflict {
                Some(Reason::NegationConflict)
            } else if lhs.iter().all(|s| matches!(s, Slot::Unresolved))
                || rhs.iter().all(|s| matches!(s, Slot::Unresolved))
            {
                Some(Reason::Missing)
            } else {
                None
            },
        )?,
        component(
            "operands-operators",
            20,
            (i128::from(exact), 1),
            (
                a.captures().iter().map(Capture::span).collect(),
                b.captures().iter().map(Capture::span).collect(),
            ),
            Some(if !operands {
                Reason::Missing
            } else if exact {
                Reason::Match
            } else {
                Reason::Mismatch
            }),
        )?,
    ];
    let mut s = Rational::whole(0);
    let mut d = Rational::whole(0);
    for c in &components {
        s = s.checked_add(c.similarity)?;
        d = d.checked_add(c.dissimilarity)?;
    }
    Ok(Evidence {
        version: "text-scanner-v3",
        components,
        totals: reduce_totals(s, d)?,
        left_constraints: a.into(),
        right_constraints: b.into(),
    })
}
