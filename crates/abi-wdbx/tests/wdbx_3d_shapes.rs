use abi_wdbx::spatial3d::{Aabb3, Point3, Radius, Sphere3, compare_squared_distances};
use std::cmp::Ordering;
#[path = "support/dyadic_reference.rs"]
mod reference;
fn p(x: f64, y: f64, z: f64) -> Point3 {
    Point3::try_new(x, y, z).unwrap()
}
fn s(center: Point3, radius: f64) -> Sphere3 {
    Sphere3::new(center, Radius::try_new(radius).unwrap())
}
#[test]
fn closed_boundaries_and_exact_near_ties() {
    let origin = p(0., 0., 0.);
    let unit = s(origin, 1.);
    assert!(unit.contains_point(p(1., 0., 0.)));
    let outside = p(1., f64::from_bits(1), 0.);
    assert!(!unit.contains_point(outside));
    assert_eq!(
        compare_squared_distances(origin, outside, p(1., 0., 0.)),
        Ordering::Greater
    );
    assert!(!unit.contains_point(p(f64::from_bits(1f64.to_bits() + 1), 0., 0.)));
    assert!(unit.intersects(&s(p(2., 0., 0.), 1.)));
    let tiny = 2f64.powi(-53);
    assert_eq!(1. + tiny, 1.);
    let rounded_down_center = p(1., 2f64.powi(-27), 0.);
    assert!(!unit.contains_point(rounded_down_center));
    assert!(unit.intersects(&s(rounded_down_center, tiny)));
    // Radius addition rounds upward to 1+2^-52, falsely admitting this separation.
    let r = 3. * 2f64.powi(-54);
    assert!(!unit.intersects(&s(p(1. + 2f64.powi(-52), 0., 0.), r)));
    let a = Aabb3::try_new(origin, p(1., 1., 1.)).unwrap();
    let b = Aabb3::try_new(p(1., 1., 1.), p(2., 2., 2.)).unwrap();
    assert!(a.intersects(&b));
    assert!(a.contains_point(p(1., 1., 1.)));
    assert!(s(p(2., 0., 0.), 1.).intersects_aabb(&a));
    assert!(
        !s(p(2., f64::from_bits(1), 0.), 1.)
            .intersects_aabb(&Aabb3::try_new(origin, origin).unwrap())
    );
    assert!(s(origin, 0.).contains_point(p(-0., 0., -0.)));
    assert!(s(origin, 0.).intersects(&s(origin, 0.)));
    assert!(Aabb3::try_new(p(1., 0., 0.), origin).is_err());
    assert_eq!(a.min(), origin);
    assert_eq!(a.max(), p(1., 1., 1.));
    assert_eq!(unit.center(), origin);
    assert_eq!(unit.radius().get(), 1.);
    for bad in [f64::NAN, f64::INFINITY, -f64::INFINITY, f64::MAX] {
        assert!(Point3::try_new(bad, 0., 0.).is_err());
        assert!(Radius::try_new(bad).is_err());
    }
    assert!(Radius::try_new(-1.).is_err());
    assert!(s(p(1e150, 0., 0.), 1e150).contains_point(origin));
    assert!(!s(p(-1e150, 0., 0.), 1e150).contains_point(p(1e150, 0., 0.)));
}
// Universal fixed scale, independent of the production per-operation alignment.
fn oracle_scaled(v: f64) -> num_bigint::BigInt {
    let b = v.to_bits();
    let e = ((b >> 52) & 2047) as usize;
    let m = b & ((1u64 << 52) - 1);
    let n = if e == 0 {
        num_bigint::BigInt::from(m)
    } else {
        num_bigint::BigInt::from(m + (1u64 << 52)) << (e - 1)
    };
    if v.is_sign_negative() { -n } else { n }
}
#[test]
fn independent_oracle_corpus() {
    let adversarial = [
        0.,
        -0.,
        f64::from_bits(1),
        -f64::from_bits(1),
        1.,
        -1.,
        1e150,
        -1e150,
        2f64.powi(-53),
        1. + 2f64.powi(-52),
    ];
    let mut seed = 0xcafe_babe_u64;
    let mut next = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        if seed & 3 == 0 {
            adversarial[(seed >> 8) as usize % adversarial.len()]
        } else {
            ((seed >> 16) % 20001) as f64 / 128. - 78.
        }
    };
    for _ in 0..600 {
        let a = p(next(), next(), next());
        let b = p(next(), next(), next());
        let c = p(next(), next(), next());
        let ra = Radius::try_new(next().abs()).unwrap();
        let rb = Radius::try_new(next().abs()).unwrap();
        let sa = Sphere3::new(a, ra);
        let sb = Sphere3::new(b, rb);
        assert_eq!(sa.contains_point(b), reference::in_radius(b, a, ra));
        let sum = oracle_scaled(ra.get()) + oracle_scaled(rb.get());
        let hit = reference::squared_distance(a, b) <= &sum * &sum;
        assert_eq!(sa.intersects(&sb), hit);
        assert_eq!(sb.intersects(&sa), hit);
        assert_eq!(
            compare_squared_distances(a, b, c),
            reference::squared_distance(a, b).cmp(&reference::squared_distance(a, c))
        );
        let lo = p(b.x().min(c.x()), b.y().min(c.y()), b.z().min(c.z()));
        let hi = p(b.x().max(c.x()), b.y().max(c.y()), b.z().max(c.z()));
        let bounds = Aabb3::try_new(lo, hi).unwrap();
        // Independent distance-to-interval formulation at the universal scale.
        let distance: num_bigint::BigInt = (0..3)
            .map(|i| {
                let x = oracle_scaled(a.coordinates()[i]);
                let l = oracle_scaled(lo.coordinates()[i]);
                let h = oracle_scaled(hi.coordinates()[i]);
                let delta = if x < l {
                    l - x
                } else if x > h {
                    x - h
                } else {
                    num_bigint::BigInt::from(0)
                };
                &delta * &delta
            })
            .sum();
        let r = oracle_scaled(ra.get());
        assert_eq!(sa.intersects_aabb(&bounds), distance <= &r * &r);
        assert_eq!(
            bounds.contains_point(a),
            (0..3).all(|i| a.coordinates()[i] >= lo.coordinates()[i]
                && a.coordinates()[i] <= hi.coordinates()[i])
        );
        let point_box = Aabb3::try_new(a, a).unwrap();
        assert_eq!(bounds.intersects(&point_box), bounds.contains_point(a));
        assert_eq!(point_box.intersects(&bounds), bounds.contains_point(a));
    }
}
