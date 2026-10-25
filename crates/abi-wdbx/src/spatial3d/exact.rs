//! Exact dyadic arithmetic aligned to the smallest participating exponent.
use super::Point3;
use num_bigint::BigInt;

fn parts(value: f64) -> (BigInt, i32) {
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mantissa, power) = if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1u64 << 52), exponent - 1023 - 52)
    };
    let integer = BigInt::from(mantissa);
    (if bits >> 63 == 0 { integer } else { -integer }, power)
}

fn align(values: &[f64]) -> Vec<BigInt> {
    let parts: Vec<_> = values.iter().copied().map(parts).collect();
    let power = parts
        .iter()
        .filter(|(n, _)| *n != BigInt::from(0))
        .map(|(_, e)| *e)
        .min()
        .unwrap_or(0);
    parts
        .into_iter()
        .map(|(n, e)| {
            if n == BigInt::from(0) {
                n
            } else {
                n << (e - power) as usize
            }
        })
        .collect()
}
fn squared(a: &[BigInt], b: &[BigInt]) -> BigInt {
    a.iter()
        .zip(b)
        .map(|(x, y)| {
            let d = x - y;
            &d * &d
        })
        .sum()
}
pub(super) fn within(a: Point3, b: Point3, radii: &[f64]) -> bool {
    let mut inputs = a.coordinates().to_vec();
    inputs.extend(b.coordinates());
    inputs.extend(radii);
    let values = align(&inputs);
    let radius: BigInt = values[6..].iter().sum();
    squared(&values[..3], &values[3..6]) <= &radius * &radius
}
/// Compare exact squared Euclidean distances over the represented coordinates.
#[must_use]
pub fn compare_squared_distances(
    origin: Point3,
    left: Point3,
    right: Point3,
) -> std::cmp::Ordering {
    let mut inputs = origin.coordinates().to_vec();
    inputs.extend(left.coordinates());
    inputs.extend(right.coordinates());
    let values = align(&inputs);
    squared(&values[..3], &values[3..6]).cmp(&squared(&values[..3], &values[6..9]))
}
