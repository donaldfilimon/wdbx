use super::{Connective, LanguageError, Role, tokenize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UtilityKind {
    Dictionary,
    Thesaurus,
    AntiThesaurus,
    Chargebook,
    VerbFrames,
    Conjunction,
    Negation,
    Morphology,
    Grammar,
}
impl UtilityKind {
    pub const ALL: [Self; 9] = [
        Self::Dictionary,
        Self::Thesaurus,
        Self::AntiThesaurus,
        Self::Chargebook,
        Self::VerbFrames,
        Self::Conjunction,
        Self::Negation,
        Self::Morphology,
        Self::Grammar,
    ];
}
#[derive(Debug, Clone)]
pub enum UtilityEffect {
    Dictionary {
        lemma: String,
        definition: String,
    },
    Equivalent {
        replacement: String,
        weight: f64,
    },
    Sense {
        lemma: String,
        required_context: Vec<String>,
        weight: f64,
    },
    Charge {
        valence: f64,
        intensity: f64,
    },
    Frame {
        subject: bool,
        object: bool,
        numeric_object: bool,
    },
    Conjunction(Connective),
    Negation {
        uncertain: bool,
    },
    Suffix {
        replacement: String,
    },
    Role(Role),
}
impl UtilityEffect {
    fn kind(&self) -> UtilityKind {
        match self {
            Self::Dictionary { .. } => UtilityKind::Dictionary,
            Self::Equivalent { .. } => UtilityKind::Thesaurus,
            Self::Sense { .. } => UtilityKind::AntiThesaurus,
            Self::Charge { .. } => UtilityKind::Chargebook,
            Self::Frame { .. } => UtilityKind::VerbFrames,
            Self::Conjunction(_) => UtilityKind::Conjunction,
            Self::Negation { .. } => UtilityKind::Negation,
            Self::Suffix { .. } => UtilityKind::Morphology,
            Self::Role(_) => UtilityKind::Grammar,
        }
    }
}
#[derive(Debug, Clone)]
pub struct UtilityRow {
    pub reference: String,
    pub phrase: String,
    pub effect: UtilityEffect,
}
#[derive(Debug, Clone)]
pub struct UtilitySnapshot {
    pub population: String,
    pub version: String,
    pub kind: UtilityKind,
    pub rows: Vec<UtilityRow>,
}
#[derive(Debug, Clone)]
pub struct UtilityTables {
    population: String,
    version: String,
    pub(super) rows: Vec<UtilityRow>,
    phrases: Vec<Vec<String>>,
    safety_match_cost: usize,
}
impl UtilityTables {
    pub fn new(
        population: &str,
        version: &str,
        snapshots: Vec<UtilitySnapshot>,
    ) -> Result<Self, LanguageError> {
        let mut kinds = BTreeSet::new();
        let mut refs = BTreeSet::new();
        let mut rows = Vec::new();
        if population.is_empty() || version.is_empty() || snapshots.len() != 9 {
            return Err(LanguageError::InvalidTables);
        }
        for s in snapshots {
            if s.population != population || s.version != version || !kinds.insert(s.kind) {
                return Err(LanguageError::InvalidTables);
            }
            for row in s.rows {
                if row.effect.kind() != s.kind
                    || row.reference.is_empty()
                    || row.reference.len() > 256
                    || !refs.insert(row.reference.clone())
                    || row.phrase.trim().is_empty()
                    || row.phrase.chars().count() > 256
                {
                    return Err(LanguageError::InvalidTables);
                }
                let phrase =
                    tokenize::tokenize(&row.phrase, 0).map_err(|_| LanguageError::InvalidTables)?;
                if phrase.is_empty() || phrase.iter().any(|t| t.kind() != super::TokenKind::Word) {
                    return Err(LanguageError::InvalidTables);
                }
                let valid = match &row.effect {
                    UtilityEffect::Dictionary { lemma, definition } => {
                        safe_lexical(lemma)
                            && !definition.is_empty()
                            && definition.chars().count() <= 2000
                    }
                    UtilityEffect::Equivalent {
                        replacement,
                        weight,
                    } => safe_lexical(replacement) && valid_weight(*weight),
                    UtilityEffect::Sense {
                        lemma,
                        required_context,
                        weight,
                    } => {
                        safe_lexical(lemma)
                            && valid_weight(*weight)
                            && required_context.len() <= 256
                            && required_context.iter().all(|s| safe_lexical(s))
                    }
                    UtilityEffect::Charge { valence, intensity } => {
                        valence.is_finite()
                            && (-1.0..=1.0).contains(valence)
                            && intensity.is_finite()
                            && (0.0..=1.0).contains(intensity)
                    }
                    UtilityEffect::Suffix { replacement } => {
                        replacement.chars().all(char::is_alphabetic)
                            && row.phrase.chars().all(char::is_alphabetic)
                    }
                    _ => true,
                };
                if !valid {
                    return Err(LanguageError::InvalidTables);
                }
                rows.push(row);
                if rows.len() > 4096 {
                    return Err(LanguageError::InvalidTables);
                }
            }
        }
        rows.sort_by(|a, b| a.reference.cmp(&b.reference));
        let phrases = rows
            .iter()
            .map(|r| {
                tokenize::tokenize(&r.phrase, 0)
                    .map(|ts| ts.into_iter().map(|t| t.lemma).collect::<Vec<_>>())
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Reserve the worst-case row visits, lexical byte comparisons and
        // changed-region checks for both safety kinds before each matcher pass.
        let safety_words = rows
            .iter()
            .zip(&phrases)
            .filter(|(r, _)| {
                matches!(
                    r.effect,
                    UtilityEffect::Negation { .. } | UtilityEffect::Conjunction(_)
                )
            })
            .flat_map(|(_, p)| p.iter())
            .try_fold(0usize, |sum, word| {
                sum.checked_add(word.len() + 1)
                    .ok_or(LanguageError::InvalidTables)
            })?;
        let safety_match_cost = rows
            .len()
            .checked_mul(2)
            .and_then(|v| safety_words.checked_mul(2).and_then(|n| v.checked_add(n)))
            .ok_or(LanguageError::InvalidTables)?;
        Ok(Self {
            safety_match_cost,
            phrases,
            population: population.into(),
            version: version.into(),
            rows,
        })
    }
    pub fn population(&self) -> &str {
        &self.population
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn rows(&self) -> &[UtilityRow] {
        &self.rows
    }
    /// Check all rewrite paths against one vocabulary, including phrases spanning
    /// the changed region and unchanged neighboring lexical tokens.
    pub(super) fn validate_rewrite(
        &self,
        tokens: &[super::Token],
        range: std::ops::Range<usize>,
        replacement: &str,
        work: &mut PreparationWork,
    ) -> Result<Vec<super::Token>, LanguageError> {
        work.charge(
            replacement
                .len()
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .ok_or(LanguageError::StepBudget)?,
        )?;
        if !safe_lexical(replacement) {
            return Err(LanguageError::UnsafeSubstitution);
        }
        let replacement = tokenize::tokenize(replacement, 0)?;
        let mut view = Vec::new();
        let mut changed = Vec::new();
        let mut append = |token: &super::Token, is_changed: bool| -> Result<(), LanguageError> {
            work.charge(
                token
                    .source()
                    .len()
                    .checked_add(token.lemma().len())
                    .and_then(|n| n.checked_add(1))
                    .ok_or(LanguageError::StepBudget)?,
            )?;
            let lexical = if token.kind() == super::TokenKind::Word {
                tokenize::tokenize(token.lemma(), 0)?
            } else {
                vec![token.clone()]
            };
            work.charge(lexical.len())?;
            for t in lexical {
                view.push(t);
                changed.push(is_changed)
            }
            Ok(())
        };
        for token in &tokens[..range.start] {
            append(token, false)?
        }
        for token in &replacement {
            append(token, true)?
        }
        for token in &tokens[range.end..] {
            append(token, false)?
        }
        for at in 0..view.len() {
            work.charge(self.safety_match_cost)?;
            for kind in [UtilityKind::Negation, UtilityKind::Conjunction] {
                for (_, count) in self.matches(kind, &view, at) {
                    if changed[at..at + count].iter().any(|v| *v) {
                        return Err(LanguageError::UnsafeSubstitution);
                    }
                }
            }
        }
        Ok(replacement)
    }
    pub(super) fn matches<'a>(
        &'a self,
        kind: UtilityKind,
        tokens: &[super::Token],
        at: usize,
    ) -> Vec<(&'a UtilityRow, usize)> {
        let mut found = Vec::new();
        let mut longest = 0;
        for (r, p) in self
            .rows
            .iter()
            .zip(&self.phrases)
            .filter(|(r, _)| r.effect.kind() == kind)
        {
            if p.len() < longest || at + p.len() > tokens.len() {
                continue;
            }
            if p.iter()
                .zip(&tokens[at..at + p.len()])
                .all(|(a, b)| !b.quoted() && a == b.lemma())
            {
                if p.len() > longest {
                    found.clear();
                    longest = p.len()
                }
                found.push((r, p.len()));
            }
        }
        found
    }
}
fn valid_weight(w: f64) -> bool {
    w.is_finite() && (0.0..=1e12).contains(&w)
}
pub(super) fn safe_lexical(s: &str) -> bool {
    if s.is_empty() || s.chars().count() > 256 {
        return false;
    }
    let Ok(tokens) = tokenize::tokenize(s, 0) else {
        return false;
    };
    !tokens.is_empty()
        && tokens.iter().all(|t| {
            t.kind() == super::TokenKind::Word
                && !is_negator(t.lemma())
                && !matches!(
                    t.lemma(),
                    "not"
                        | "no"
                        | "never"
                        | "don't"
                        | "cannot"
                        | "and"
                        | "or"
                        | "but"
                        | "if"
                        | "then"
                        | "zero"
                        | "one"
                        | "two"
                        | "three"
                        | "four"
                        | "five"
                        | "six"
                        | "seven"
                        | "eight"
                        | "nine"
                        | "ten"
                        | "plus"
                        | "minus"
                        | "times"
                        | "divided"
                        | "modulo"
                        | "power"
                )
        })
}

pub(super) fn is_negator(s: &str) -> bool {
    matches!(
        s,
        "not" | "no" | "never" | "don't" | "dont" | "cannot" | "can't" | "don’t"
    )
}

/// One invocation-wide allowance; a rewrite never creates or resets it.
pub(super) struct PreparationWork {
    remaining: usize,
}
impl PreparationWork {
    pub(super) fn new(remaining: usize) -> Self {
        Self { remaining }
    }
    fn charge(&mut self, cost: usize) -> Result<(), LanguageError> {
        self.remaining = self
            .remaining
            .checked_sub(cost)
            .ok_or(LanguageError::StepBudget)?;
        Ok(())
    }
}
