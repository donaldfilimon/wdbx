//! Versioned opt-in structural text keys. These are projections, not entry equality.
use crate::{identity::PatternKey, language_v3::*};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Low,
    Medium,
    High,
}
impl Resolution {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TextError {
    #[error("original exceeds 2000 characters")]
    PatternTooLong,
}
/// Only constructed from immutable, validated language-v3 actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextKey {
    key: PatternKey,
    resolution: Resolution,
    count: usize,
    transform: Vec<u8>,
    preimage: Vec<u8>,
}
impl TextKey {
    pub fn pattern_key(&self) -> &PatternKey {
        &self.key
    }
    pub fn resolution(&self) -> Resolution {
        self.resolution
    }
    pub fn token_count(&self) -> usize {
        self.count
    }
    pub fn transform(&self) -> &[u8] {
        &self.transform
    }
    pub fn preimage(&self) -> &[u8] {
        &self.preimage
    }
}
/// Every scalar/tag is a byte-length framed field. Every list starts with its
/// u64 big-endian element count. No span, caller identity or rule ref is encoded.
pub fn encode(action: &PreparedAction) -> Result<TextKey, TextError> {
    encode_tree(action.original(), action.ast(), &[action])
}
/// Encode the complete original chunk tree, retaining cross-action connectives.
/// Fan-out alternatives remain separate action keys and do not alter this key.
pub fn encode_prepared(prepared: &Prepared) -> Result<TextKey, TextError> {
    let actions = prepared
        .clauses()
        .iter()
        .map(|c| &c.variants()[0])
        .collect::<Vec<_>>();
    encode_tree(prepared.original(), prepared.ast(), &actions)
}
fn encode_tree(
    original: &str,
    tree: &SemanticAst,
    actions: &[&PreparedAction],
) -> Result<TextKey, TextError> {
    if original.chars().count() > 2000 {
        return Err(TextError::PatternTooLong);
    }
    let count = lexical_count(tree);
    let resolution = if count <= 6 {
        Resolution::Low
    } else if count <= 16 {
        Resolution::Medium
    } else {
        Resolution::High
    };
    let mut transform = vec![];
    field(&mut transform, b"text-v3");
    match resolution {
        Resolution::Low => {
            let mut leaves = vec![];
            low_leaves(tree, &mut leaves);
            leaves.sort();
            leaves.dedup();
            list(&mut transform, &leaves);
            // Connective and negation topology survives the bag projection.
            control(tree, &mut transform);
        }
        Resolution::Medium => medium_ast(tree, &mut transform),
        Resolution::High => ast(tree, &mut transform),
    }
    transform.extend_from_slice(&(actions.len() as u64).to_be_bytes());
    for action in actions {
        constraints(action, &mut transform);
        if resolution == Resolution::High {
            field(&mut transform, b"spo");
            for slot in [
                action.spo().subject(),
                action.spo().predicate(),
                action.spo().object(),
            ] {
                match slot {
                    Slot::Resolved(s) => {
                        field(&mut transform, b"resolved");
                        field(&mut transform, s.as_bytes());
                    }
                    Slot::Unresolved => field(&mut transform, b"unresolved"),
                }
            }
            let captures = action
                .captures()
                .iter()
                .map(|c| decimal(c.decimal()))
                .collect::<Vec<_>>();
            list(&mut transform, &captures);
        }
        if resolution != Resolution::Low {
            let operators = action
                .operators()
                .iter()
                .map(|s| s.as_bytes().to_vec())
                .collect::<Vec<_>>();
            list(&mut transform, &operators);
        }
    }
    let mut preimage = vec![];
    for value in [
        transform.as_slice(),
        resolution.as_str().as_bytes(),
        b"text",
    ] {
        field(&mut preimage, value);
    }
    let key = PatternKey::new(
        "text-v3",
        "text",
        resolution.as_str(),
        Sha256::digest(&preimage).into(),
    );
    Ok(TextKey {
        key,
        resolution,
        count,
        transform,
        preimage,
    })
}
fn field(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    out.extend_from_slice(bytes);
}
fn list(out: &mut Vec<u8>, values: &[Vec<u8>]) {
    out.extend_from_slice(&(values.len() as u64).to_be_bytes());
    for value in values {
        field(out, value);
    }
}
fn decimal(n: &CanonicalDecimal) -> Vec<u8> {
    let mut out = vec![];
    field(&mut out, n.coefficient().to_string().as_bytes());
    field(&mut out, n.scale().to_string().as_bytes());
    out
}
fn token(t: &Token, low: bool) -> Vec<u8> {
    let mut out = vec![];
    let tag = match t.kind() {
        TokenKind::Word => "word",
        TokenKind::Quoted => "quoted",
        TokenKind::Number => "number",
        TokenKind::Operator => "operator",
        TokenKind::Punctuation => "punctuation",
    };
    field(&mut out, tag.as_bytes());
    if let Some(n) = t.decimal() {
        if low {
            field(&mut out, b"#");
        } else {
            field(&mut out, &decimal(n));
        }
    } else {
        field(&mut out, t.lemma().as_bytes());
    }
    if !low {
        field(
            &mut out,
            match t.role() {
                Role::Subject => b"subject",
                Role::Predicate => b"predicate",
                Role::Object => b"object",
                Role::Other => b"other",
                Role::Connective => b"connective",
                Role::Negation => b"negation",
            },
        );
    }
    out
}
fn low_leaves(tree: &SemanticAst, out: &mut Vec<Vec<u8>>) {
    match tree {
        SemanticAst::Action(tokens) => out.extend(
            tokens
                .iter()
                .filter(|t| t.kind() != TokenKind::Punctuation)
                .map(|t| token(t, true)),
        ),
        SemanticAst::And(a, b)
        | SemanticAst::Or(a, b)
        | SemanticAst::But(a, b)
        | SemanticAst::IfThen(a, b) => {
            low_leaves(a, out);
            low_leaves(b, out);
        }
        SemanticAst::NegatedAction { action, .. } => low_leaves(action, out),
    }
}
fn control(tree: &SemanticAst, out: &mut Vec<u8>) {
    match tree {
        SemanticAst::Action(_) => field(out, b"action"),
        SemanticAst::And(a, b) => binary(b"and", a, b, out, control),
        SemanticAst::Or(a, b) => binary(b"or", a, b, out, control),
        SemanticAst::But(a, b) => binary(b"but", a, b, out, control),
        SemanticAst::IfThen(a, b) => binary(b"if-then", a, b, out, control),
        SemanticAst::NegatedAction { uncertain, action } => {
            field(out, b"negated");
            field(
                out,
                if *uncertain {
                    b"uncertain"
                } else {
                    b"explicit"
                },
            );
            control(action, out);
        }
    }
}
fn medium_ast(tree: &SemanticAst, out: &mut Vec<u8>) {
    render_ast(tree, out, false);
}
fn ast(tree: &SemanticAst, out: &mut Vec<u8>) {
    render_ast(tree, out, true);
}
fn render_ast(tree: &SemanticAst, out: &mut Vec<u8>, complete: bool) {
    let recurse = if complete { ast } else { medium_ast };
    match tree {
        SemanticAst::Action(tokens) => {
            field(out, b"action");
            list(
                out,
                &tokens
                    .iter()
                    .filter(|t| complete || t.kind() != TokenKind::Punctuation)
                    .map(|t| token(t, false))
                    .collect::<Vec<_>>(),
            );
        }
        SemanticAst::And(a, b) => binary(b"and", a, b, out, recurse),
        SemanticAst::Or(a, b) => binary(b"or", a, b, out, recurse),
        SemanticAst::But(a, b) => binary(b"but", a, b, out, recurse),
        SemanticAst::IfThen(a, b) => binary(b"if-then", a, b, out, recurse),
        SemanticAst::NegatedAction { uncertain, action } => {
            field(out, b"negated");
            field(
                out,
                if *uncertain {
                    b"uncertain"
                } else {
                    b"explicit"
                },
            );
            recurse(action, out);
        }
    }
}
fn binary(
    tag: &[u8],
    a: &SemanticAst,
    b: &SemanticAst,
    out: &mut Vec<u8>,
    f: fn(&SemanticAst, &mut Vec<u8>),
) {
    field(out, tag);
    out.extend_from_slice(&2_u64.to_be_bytes());
    f(a, out);
    f(b, out);
}
fn constraints(a: &PreparedAction, out: &mut Vec<u8>) {
    for (tag, value) in [
        ("inhibited", a.inhibited()),
        ("uncertain-negation", a.uncertain_negation()),
        ("unresolved-frame", a.unresolved_frame()),
    ] {
        field(out, tag.as_bytes());
        field(out, if value { b"true" } else { b"false" });
    }
    field(out, b"condition");
    field(
        out,
        match a.conditional() {
            ConditionalDisposition::None => b"none",
            ConditionalDisposition::Satisfied => b"satisfied",
            ConditionalDisposition::Refuted => b"refuted",
            ConditionalDisposition::Unresolved => b"unresolved",
        },
    );
}

// Punctuation is retained by the high AST, but is not a lexical leaf in the
// selected class profile. Count the actual immutable tree, never source length.
fn lexical_count(tree: &SemanticAst) -> usize {
    match tree {
        SemanticAst::Action(tokens) => tokens
            .iter()
            .filter(|t| t.kind() != TokenKind::Punctuation)
            .count(),
        SemanticAst::And(a, b)
        | SemanticAst::Or(a, b)
        | SemanticAst::But(a, b)
        | SemanticAst::IfThen(a, b) => 1 + lexical_count(a) + lexical_count(b),
        SemanticAst::NegatedAction { action, .. } => 1 + lexical_count(action),
    }
}
