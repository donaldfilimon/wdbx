use super::ScoringError;
/// Random-stream provenance survives even when an exception consumes no draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawIdentity {
    pub seed: u64,
    pub stream: String,
}
/// Every draw has a stable purpose and position within the supplied stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawPurpose {
    JitterMagnitude,
    JitterSign,
    Alternative,
    Redundancy,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawRecord {
    pub seed: u64,
    pub stream: String,
    pub index: usize,
    pub value: u32,
    pub purpose: DrawPurpose,
}
/// Bounded host draws. Failed operations never advance the cursor. Seed and
/// stream identify provenance only, not authenticated randomness.
pub struct DrawTape<'a> {
    pub(super) values: &'a [u32],
    pub(super) position: usize,
    seed: u64,
    stream: String,
    limit: usize,
}
impl<'a> DrawTape<'a> {
    pub fn new(
        seed: u64,
        stream: &str,
        values: &'a [u32],
        limit: usize,
    ) -> Result<Self, ScoringError> {
        if stream.is_empty() || stream.len() > 128 || limit > 100_000 {
            return Err(ScoringError::InvalidBudget);
        }
        Ok(Self {
            values,
            position: 0,
            seed,
            stream: stream.into(),
            limit,
        })
    }
    pub fn identity(&self) -> DrawIdentity {
        DrawIdentity {
            seed: self.seed,
            stream: self.stream.clone(),
        }
    }
    pub fn consumed(&self) -> usize {
        self.position
    }
    pub(super) fn ensure(&self, position: usize, n: usize) -> Result<(), ScoringError> {
        let end = position
            .checked_add(n)
            .ok_or(ScoringError::BudgetExceeded)?;
        if end > self.limit {
            Err(ScoringError::BudgetExceeded)
        } else if end > self.values.len() {
            Err(ScoringError::DrawExhausted)
        } else {
            Ok(())
        }
    }
    pub(super) fn draw(
        &self,
        position: &mut usize,
        purpose: DrawPurpose,
    ) -> Result<DrawRecord, ScoringError> {
        self.ensure(*position, 1)?;
        let index = *position;
        *position += 1;
        Ok(DrawRecord {
            seed: self.seed,
            stream: self.stream.clone(),
            index,
            value: self.values[index],
            purpose,
        })
    }
}
/// Explicit deterministic SplitMix64-derived u32 grid, without entropy or I/O.
/// The caller supplies state and a bounded count and owns persistence, if any.
pub fn seeded_draws(mut state: u64, count: usize) -> Result<Vec<u32>, ScoringError> {
    if count > 100_000 {
        return Err(ScoringError::BudgetExceeded);
    }
    Ok((0..count)
        .map(|_| {
            state = state.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            ((z ^ (z >> 31)) >> 32) as u32
        })
        .collect())
}
