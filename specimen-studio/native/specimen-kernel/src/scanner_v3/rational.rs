use super::ScanError;
/// Reduced signed rational. Denominator is always positive; no float rounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rational {
    numerator: i128,
    denominator: i128,
}
impl Rational {
    pub fn new(numerator: i128, denominator: i128) -> Result<Self, ScanError> {
        if denominator <= 0 {
            return Err(ScanError::InvalidDenominator);
        }
        let g = gcd(numerator.unsigned_abs(), denominator as u128) as i128;
        Ok(Self {
            numerator: numerator / g,
            denominator: denominator / g,
        })
    }
    pub fn numerator(self) -> i128 {
        self.numerator
    }
    pub fn denominator(self) -> i128 {
        self.denominator
    }
    /// Optional display projection only: truncates towards zero, never used to scan.
    pub fn trunc(self) -> i128 {
        self.numerator / self.denominator
    }
    pub fn checked_add(self, other: Self) -> Result<Self, ScanError> {
        self.combine(other, false)
    }
    pub fn checked_sub(self, other: Self) -> Result<Self, ScanError> {
        self.combine(other, true)
    }
    fn combine(self, other: Self, subtract: bool) -> Result<Self, ScanError> {
        let g = gcd(self.denominator as u128, other.denominator as u128) as i128;
        let a = self
            .numerator
            .checked_mul(other.denominator / g)
            .ok_or(ScanError::Overflow)?;
        let b = other
            .numerator
            .checked_mul(self.denominator / g)
            .ok_or(ScanError::Overflow)?;
        let n = if subtract {
            a.checked_sub(b)
        } else {
            a.checked_add(b)
        }
        .ok_or(ScanError::Overflow)?;
        // Reduce by the shared denominator gcd before multiplying denominators.
        let common = gcd(n.unsigned_abs(), g as u128) as i128;
        let d = (self.denominator / g)
            .checked_mul(other.denominator / common)
            .ok_or(ScanError::Overflow)?;
        Self::new(n / common, d)
    }
    fn clamp(self) -> Self {
        if self.trunc() > 100 || (self.trunc() == 100 && self.numerator % self.denominator > 0) {
            Self::whole(100)
        } else if self.trunc() < -100
            || (self.trunc() == -100 && self.numerator % self.denominator < 0)
        {
            Self::whole(-100)
        } else {
            self
        }
    }
    pub(super) const fn whole(n: i128) -> Self {
        Self {
            numerator: n,
            denominator: 1,
        }
    }
}
fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Totals {
    pub similarity: Rational,
    pub dissimilarity: Rational,
    pub difference: Rational,
    pub confidence: Rational,
}
/// Shared reducer for scanner and direct extreme-value witnesses. S and D are
/// retained exactly; only their checked difference is clamped, once.
pub fn reduce_totals(similarity: Rational, dissimilarity: Rational) -> Result<Totals, ScanError> {
    let difference = similarity.checked_sub(dissimilarity)?;
    Ok(Totals {
        similarity,
        dissimilarity,
        difference,
        confidence: difference.clamp(),
    })
}
