use super::{math::*, *};
use crate::{
    language_v3::Span,
    logical_charge_v1::LogicalChargeV1,
    scanner_v3::{self, Evidence, Rational},
};
#[derive(Debug, Clone, Copy)]
pub struct Atp {
    pub valence: Rational,
    pub intensity: Rational,
}
impl Default for Atp {
    fn default() -> Self {
        Self {
            valence: whole(0),
            intensity: whole(0),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Policy {
    pub strength: u8,
    pub threshold: i16,
    pub atp: Atp,
    pub jitter: bool,
    pub cap: Rational,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            strength: 1,
            threshold: 50,
            atp: Atp::default(),
            jitter: true,
            cap: whole(1),
        }
    }
}
impl Policy {
    pub fn brainstorm() -> Self {
        Self {
            cap: whole(5),
            ..Self::default()
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitterReason {
    Disabled,
    Endpoint,
    MaximumQualified,
    ZeroCap,
    Eligible,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    BelowThreshold,
    ZeroStrength,
    InputInhibited,
    TargetInhibited,
    UncertainNegation,
    Unresolved,
    Refuted,
    BlockedAutomaton,
    NoAlternative,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreTrace {
    pub random: DrawIdentity,
    pub scan: Rational,
    pub baseline: Rational,
    pub delta: Rational,
    pub effective: Rational,
    pub baseline_qualified: bool,
    pub qualified: bool,
    pub jitter_reason: JitterReason,
    pub refusals: Vec<Refusal>,
    pub draws: Vec<DrawRecord>,
}
pub(super) fn validate_policy(p: Policy) -> Result<(), ScoringError> {
    if p.strength > 100
        || !(-100..=100).contains(&p.threshold)
        || p.atp.valence.denominator() > 1_000_000
        || p.atp.intensity.denominator() > 1_000_000
        || p.cap.denominator() > 1_000_000
        || !between(p.atp.valence, -1, 1)?
        || !between(p.atp.intensity, 0, 1)?
        || !between(p.cap, 0, 5)?
    {
        return Err(ScoringError::InvalidPolicy);
    }
    Ok(())
}
/// Explicitly numeric mechanism helper. This is not a scanner or executable permit.
pub fn numeric_policy(
    scan: Rational,
    p: Policy,
    tape: &mut DrawTape<'_>,
) -> Result<ScoreTrace, ScoringError> {
    let mut position = tape.position;
    let out = score_at(scan, p, tape, &mut position)?;
    tape.position = position;
    Ok(out)
}
/// Checked balance for synthetic numeric tests; S/D are subtracted before clamp.
pub fn numeric_balance(
    similarity: Rational,
    dissimilarity: Rational,
    p: Policy,
    tape: &mut DrawTape<'_>,
) -> Result<ScoreTrace, ScoringError> {
    numeric_policy(
        scanner_v3::reduce_totals(similarity, dissimilarity)?.confidence,
        p,
        tape,
    )
}
pub(super) fn score_at(
    scan: Rational,
    p: Policy,
    tape: &DrawTape<'_>,
    position: &mut usize,
) -> Result<ScoreTrace, ScoringError> {
    validate_policy(p)?;
    if !between(scan, -100, 100)? {
        return Err(ScoringError::InvalidEvidence);
    }
    let negative = if p.atp.valence.numerator() < 0 {
        mul(p.atp.valence, whole(-1))?
    } else {
        whole(0)
    };
    let baseline = clamp(scan.checked_sub(mul(whole(10), mul(p.atp.intensity, negative)?)?)?)?;
    let baseline_qualified = !cmp(baseline, whole(i128::from(p.threshold)))?.is_lt();
    let jitter_reason = if !p.jitter {
        JitterReason::Disabled
    } else if baseline == whole(100) || baseline == whole(-100) {
        JitterReason::Endpoint
    } else if p.strength == 100 && baseline_qualified {
        JitterReason::MaximumQualified
    } else if p.cap == whole(0) {
        JitterReason::ZeroCap
    } else {
        JitterReason::Eligible
    };
    let mut draws = vec![];
    let delta = if jitter_reason == JitterReason::Eligible {
        tape.ensure(*position, 2)?;
        let magnitude = tape.draw(position, DrawPurpose::JitterMagnitude)?;
        let sign = tape.draw(position, DrawPurpose::JitterSign)?;
        let delta = mul(
            p.cap,
            Rational::new(i128::from(magnitude.value), i128::from(u32::MAX))?,
        )?;
        let delta = mul(delta, whole(if sign.value & 1 == 0 { -1 } else { 1 }))?;
        draws.extend([magnitude, sign]);
        delta
    } else {
        whole(0)
    };
    let effective = clamp(baseline.checked_add(delta)?)?;
    let mut refusals = vec![];
    if cmp(effective, whole(i128::from(p.threshold)))?.is_lt() {
        refusals.push(Refusal::BelowThreshold)
    }
    if p.strength == 0 {
        refusals.push(Refusal::ZeroStrength)
    }
    Ok(ScoreTrace {
        random: tape.identity(),
        scan,
        baseline,
        delta,
        effective,
        baseline_qualified,
        qualified: refusals.is_empty(),
        jitter_reason,
        refusals,
        draws,
    })
}
/// Validate an externally constructible evidence value, without treating it as
/// authentication. Executable contributions always call the actual scanner.
pub fn validate_evidence(e: &Evidence) -> Result<(), ScoringError> {
    if e.version != "text-scanner-v3" {
        return Err(ScoringError::InvalidEvidence);
    }
    let spans = e
        .components
        .iter()
        .try_fold(0usize, |sum, c| {
            sum.checked_add(c.left_spans.len())
                .and_then(|n| n.checked_add(c.right_spans.len()))
        })
        .ok_or(ScoringError::BudgetExceeded)?;
    if spans > 1_000_000
        || spans
            .checked_mul(Span::UNIT_BYTES)
            .is_none_or(|bytes| bytes > 16 * 1024 * 1024)
    {
        return Err(ScoringError::BudgetExceeded);
    }
    let expected = [
        ("lemma-multiset", 40),
        ("ordered-bigram-multiset", 20),
        ("spo", 20),
        ("operands-operators", 20),
    ];
    let mut s = whole(0);
    let mut d = whole(0);
    for (c, (name, weight)) in e.components.iter().zip(expected) {
        if c.name != name
            || c.weight != weight
            || !between(c.similarity, 0, weight)?
            || !between(c.dissimilarity, 0, weight)?
            || c.similarity.checked_add(c.dissimilarity)? != whole(weight)
        {
            return Err(ScoringError::InvalidEvidence);
        }
        use scanner_v3::Reason;
        let valid = match c.reason {
            Reason::Match => c.similarity == whole(weight),
            Reason::Partial => c.similarity.numerator() > 0 && c.dissimilarity.numerator() > 0,
            Reason::Missing
            | Reason::Mismatch
            | Reason::NegationConflict
            | Reason::UncertainNegation => c.similarity == whole(0),
        };
        if !valid {
            return Err(ScoringError::InvalidEvidence);
        }
        if c.left_spans
            .iter()
            .chain(&c.right_spans)
            .any(|s| s.start > s.end)
        {
            return Err(ScoringError::InvalidEvidence);
        }
        s = s.checked_add(c.similarity)?;
        d = d.checked_add(c.dissimilarity)?;
    }
    if scanner_v3::reduce_totals(s, d)? != e.totals {
        return Err(ScoringError::InvalidEvidence);
    }
    for c in [&e.left_constraints, &e.right_constraints] {
        validate_constraints(c)?
    }
    Ok(())
}
pub(super) fn validate_constraints(
    c: &scanner_v3::routes::Constraints,
) -> Result<(), ScoringError> {
    use crate::language_v3::ConditionalDisposition::*;
    if c.executable
        != (!c.inhibited && !c.unresolved_frame && matches!(c.conditional, None | Satisfied))
    {
        return Err(ScoringError::InvalidEvidence);
    }
    Ok(())
}
