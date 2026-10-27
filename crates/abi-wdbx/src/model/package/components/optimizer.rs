//! Full post-step optimizer state; no partial/frozen subsets.
use super::{Cursor, Result};
use crate::model::{
    contract::{
        tensors::TensorId,
        training::{AdamWState, Moments, OptimizerProfile, OptimizerState, SchedulerProfile},
    },
    error::ErrorKind as E,
};
pub(super) fn parse(r: &mut Cursor<'_, '_>) -> Result<OptimizerState> {
    match r.u8()? {
        0 => {
            r.end()?;
            return Ok(OptimizerState::Absent);
        }
        1 => {}
        _ => return Err(r.error(E::NonCanonical, "state-tag")),
    }
    let a = r.u16()?;
    let b = r.u16()?;
    r.check(a == 1 && b == 1, E::InvalidOptimizer, "optimizer-profiles")?;
    let successful_step = r.u64()?;
    let learning_rate = r.f64()?;
    let beta1 = r.f64()?;
    let beta2 = r.f64()?;
    let epsilon = r.f64()?;
    let weight_decay = r.f64()?;
    let gradient_clip_norm = r.f64()?;
    r.check(
        successful_step < u64::MAX
            && learning_rate > 0.0
            && learning_rate <= 1.0
            && (0.0..1.0).contains(&beta1)
            && (0.0..1.0).contains(&beta2)
            && epsilon > 0.0
            && epsilon <= 1.0
            && (0.0..=1.0).contains(&weight_decay)
            && gradient_clip_norm > 0.0
            && gradient_clip_norm <= 1000.0,
        E::InvalidOptimizer,
        "optimizer-hyperparameters",
    )?;
    let n = r.count(16)?;
    r.check(n == 16, E::InvalidOptimizer, "moment-membership")?;
    let mut moments = r.vec(16)?;
    let mut total = 0;
    for expected in 1..=16 {
        let ident = r.u16()?;
        let scalar_count = r.u64()?;
        total += scalar_count.min(1_000_001);
        r.check(
            ident == expected && (1..=1_000_000).contains(&scalar_count) && total <= 1_000_000,
            E::InvalidOptimizer,
            "moment-count",
        )?;
        let first_moment = numbers(r, scalar_count, false, successful_step)?;
        let second_moment = numbers(r, scalar_count, true, successful_step)?;
        moments.push(Moments {
            tensor_id: TensorId::try_from(ident)?,
            scalar_count,
            first_moment,
            second_moment,
        });
    }
    r.end()?;
    Ok(OptimizerState::AdamW(AdamWState {
        optimizer_profile: OptimizerProfile::AdamWGlobalNormMatrixDecayV1,
        scheduler_profile: SchedulerProfile::ConstantLearningRateV1,
        successful_step,
        learning_rate,
        beta1,
        beta2,
        epsilon,
        weight_decay,
        gradient_clip_norm,
        moments,
    }))
}
fn numbers(r: &mut Cursor<'_, '_>, n: u64, second: bool, step: u64) -> Result<Vec<f64>> {
    let mut v = r.vec(super::super::size(n)?)?;
    for bytes in r.take(super::super::size(n * 8)?, true)?.as_chunks::<8>().0 {
        let f = f64::from_le_bytes(*bytes);
        r.check(
            f.is_finite() && (!second || f >= 0.0) && (step != 0 || f == 0.0),
            E::InvalidOptimizer,
            "moment-values",
        )?;
        v.push(f);
    }
    Ok(v)
}
