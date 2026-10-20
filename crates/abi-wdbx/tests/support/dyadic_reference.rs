//! Exact represented-input arithmetic for the bounded test corpus only.
//! Integers represent multiples of 2^-1074; squared keys share scale 2^-2148.
//! Every finite f64 needs at most 2098 bits; this helper is not production code.
use abi_wdbx::spatial3d::{Point3, Radius};
use num_bigint::BigInt;
fn scaled(value: f64) -> BigInt {
    assert!(value.is_finite());
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as usize;
    let fraction = bits & ((1u64 << 52) - 1);
    let magnitude = if exponent == 0 {
        BigInt::from(fraction)
    } else {
        BigInt::from(fraction | (1u64 << 52)) << (exponent - 1)
    };
    if bits >> 63 == 0 {
        magnitude
    } else {
        -magnitude
    }
}
pub fn squared_distance(a: Point3, b: Point3) -> BigInt {
    a.coordinates()
        .into_iter()
        .zip(b.coordinates())
        .map(|(x, y)| {
            let delta = scaled(x) - scaled(y);
            &delta * &delta
        })
        .sum()
}
pub fn in_radius(point: Point3, center: Point3, radius: Radius) -> bool {
    let r = scaled(radius.get());
    squared_distance(point, center) <= &r * &r
}
