use super::*;
#[derive(Debug, Clone, PartialEq)]
pub struct ElectionTrace {
    pub random: DrawIdentity,
    pub weights: Vec<f64>,
    pub refused: Vec<usize>,
    pub draw: Option<DrawRecord>,
    pub winner: Option<usize>,
}
/// Proportional election over supplied slots. Eligibility and weights are fully
/// validated before the transactional draw. No uniform fallback is used.
pub fn elect(
    weights: &[f64],
    eligible: &[bool],
    tape: &mut DrawTape<'_>,
) -> Result<ElectionTrace, ScoringError> {
    if weights.len() > 100_000 || weights.len() != eligible.len() {
        return Err(ScoringError::BudgetExceeded);
    }
    let mut position = tape.position;
    let out = elect_at(
        weights,
        eligible,
        tape,
        &mut position,
        DrawPurpose::Alternative,
    )?;
    tape.position = position;
    Ok(out)
}
pub(super) fn elect_at(
    weights: &[f64],
    eligible: &[bool],
    tape: &DrawTape<'_>,
    position: &mut usize,
    purpose: DrawPurpose,
) -> Result<ElectionTrace, ScoringError> {
    if weights.len() != eligible.len() || weights.iter().any(|w| !w.is_finite() || *w < 0.0) {
        return Err(ScoringError::InvalidWeight);
    }
    let positives = weights
        .iter()
        .zip(eligible)
        .enumerate()
        .filter(|(_, (w, e))| **e && **w > 0.0)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let mut out = ElectionTrace {
        random: tape.identity(),
        weights: weights.to_vec(),
        refused: eligible
            .iter()
            .enumerate()
            .filter_map(|(i, e)| (!e).then_some(i))
            .collect(),
        draw: None,
        winner: None,
    };
    if positives.is_empty() {
        return Ok(out);
    }
    if positives.len() == 1 {
        out.winner = Some(positives[0]);
        return Ok(out);
    }
    let draw = tape.draw(position, purpose)?;
    if draw.value == 0 || draw.value == u32::MAX {
        out.winner = if draw.value == 0 {
            positives.first().copied()
        } else {
            positives.last().copied()
        };
        out.draw = Some(draw);
        return Ok(out);
    }
    let maximum = positives.iter().map(|i| weights[*i]).fold(0.0, f64::max);
    // Dividing first prevents overflowing a sum of finite weights. Compensated
    // summation makes a stable slot-ordered finite proportional distribution.
    let mut total = 0.0;
    let mut correction = 0.0;
    for i in &positives {
        let y = weights[*i] / maximum - correction;
        let next = total + y;
        correction = (next - total) - y;
        total = next;
    }
    let target = f64::from(draw.value) / f64::from(u32::MAX) * total;
    let mut cumulative = 0.0;
    let mut correction = 0.0;
    out.winner = positives.last().copied();
    if draw.value != u32::MAX {
        for i in &positives {
            let y = weights[*i] / maximum - correction;
            let next = cumulative + y;
            correction = (next - cumulative) - y;
            cumulative = next;
            if target < cumulative {
                out.winner = Some(*i);
                break;
            }
        }
    }
    out.draw = Some(draw);
    Ok(out)
}
