use super::ScoringError;
use crate::scanner_v3::{Rational, ScanError};
use std::cmp::Ordering;
pub(super) fn whole(n: i128) -> Rational {
    Rational::new(n, 1).expect("positive denominator")
}
fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}
pub(super) fn mul(a: Rational, b: Rational) -> Result<Rational, ScoringError> {
    let g = gcd(a.numerator().unsigned_abs(), b.denominator() as u128) as i128;
    let h = gcd(b.numerator().unsigned_abs(), a.denominator() as u128) as i128;
    Ok(Rational::new(
        (a.numerator() / g)
            .checked_mul(b.numerator() / h)
            .ok_or(ScanError::Overflow)?,
        (a.denominator() / h)
            .checked_mul(b.denominator() / g)
            .ok_or(ScanError::Overflow)?,
    )?)
}
pub(super) fn cmp(a: Rational, b: Rational) -> Result<Ordering, ScoringError> {
    let g = gcd(a.denominator() as u128, b.denominator() as u128) as i128;
    Ok(a.numerator()
        .checked_mul(b.denominator() / g)
        .ok_or(ScanError::Overflow)?
        .cmp(
            &b.numerator()
                .checked_mul(a.denominator() / g)
                .ok_or(ScanError::Overflow)?,
        ))
}
pub(super) fn between(v: Rational, lo: i128, hi: i128) -> Result<bool, ScoringError> {
    Ok(!cmp(v, whole(lo))?.is_lt() && !cmp(v, whole(hi))?.is_gt())
}
pub(super) fn clamp(v: Rational) -> Result<Rational, ScoringError> {
    Ok(if cmp(v, whole(-100))?.is_lt() {
        whole(-100)
    } else if cmp(v, whole(100))?.is_gt() {
        whole(100)
    } else {
        v
    })
}
