//! Checked legacy-point behavior and resource qualification.
#[path = "support/spatial_oracle.rs"]
mod spatial_oracle;
use abi_wdbx::{
    spatial::{DistanceMetric, Point3D, SpatialIndex3D},
    spatial3d::{CheckedSpatialHit, Point3, QueryLimits, Radius, SpatialError},
};
use std::sync::atomic::AtomicBool;
fn p(x: f32) -> Point3D {
    Point3D { x, y: 0., z: 0. }
}
fn limits() -> QueryLimits {
    QueryLimits {
        max_work: usize::MAX,
        max_results: usize::MAX,
        max_output_bytes: usize::MAX,
    }
}
fn keys(h: &[CheckedSpatialHit<'_>]) -> Vec<(u32, usize)> {
    h.iter().map(|h| (h.id, h.insertion_ordinal)).collect()
}
#[test]
fn invalid_insert_is_atomic() {
    let mut i = SpatialIndex3D::new();
    i.insert(7, p(1.), b"preserved");
    let before = i
        .try_nearest_neighbors_checked(p(0.), 10, limits())
        .unwrap()
        .iter()
        .map(|h| (h.id, h.point, h.payload.to_vec()))
        .collect::<Vec<_>>();
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            i.try_insert(8, p(bad), b"bad"),
            Err(SpatialError::NonFinite)
        );
        assert_eq!(i.count(), 1);
    }
    let after = i
        .try_nearest_neighbors_checked(p(0.), 10, limits())
        .unwrap()
        .iter()
        .map(|h| (h.id, h.point, h.payload.to_vec()))
        .collect::<Vec<_>>();
    assert_eq!(before, after);
    assert_eq!(after.len(), 1);
    i.try_insert(8, p(-0.), b"ok").unwrap();
    assert_eq!(i.count(), 2);
    assert_eq!(
        i.try_nearest_neighbors_checked(p(0.), 1, limits()).unwrap()[0]
            .point
            .x
            .to_bits(),
        (-0f32).to_bits()
    );
}
#[test]
fn zero_k_returns_no_hits() {
    let mut i = SpatialIndex3D::new();
    i.insert(1, p(f32::NAN), b"invalid");
    let zero = QueryLimits {
        max_work: 0,
        max_results: 0,
        max_output_bytes: 0,
    };
    assert_eq!(i.try_nearest_neighbors_checked(p(0.), 0, zero).unwrap(), []);
    assert_eq!(
        i.try_nearest_neighbors_checked(p(f32::NAN), 0, zero),
        Err(SpatialError::NonFinite)
    );
}
#[test]
fn equal_distance_uses_checked_total_order() {
    let mut i = SpatialIndex3D::new();
    i.insert(9, p(-1.), b"a");
    i.insert(2, p(1.), b"b");
    assert_eq!(
        keys(&i.try_nearest_neighbors_checked(p(0.), 1, limits()).unwrap()),
        vec![(2, 1)]
    );
    assert_eq!(
        keys(&i.try_radius_search_checked(p(0.), 1., limits()).unwrap()),
        vec![(2, 1), (9, 0)]
    );
    assert_eq!(
        i.nearest_neighbors(p(0.), 1, DistanceMetric::Euclidean)[0].id,
        9
    );
}
#[test]
fn legacy_duplicates_are_retained() {
    let mut i = SpatialIndex3D::new();
    i.insert(4, p(0.), b"one");
    i.insert(4, p(0.), b"two");
    let h = i.try_radius_search_checked(p(0.), 0., limits()).unwrap();
    assert_eq!(keys(&h), vec![(4, 0), (4, 1)]);
    assert_eq!(h[1].payload, b"two");
}
#[test]
fn invalid_legacy_population_is_not_silently_skipped() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut i = SpatialIndex3D::new();
        i.insert(1, p(0.), b"");
        i.insert(2, p(bad), b"");
        assert_eq!(
            i.try_radius_search_checked(p(0.), 0., limits()),
            Err(SpatialError::NonFinite)
        );
        assert_eq!(
            i.try_nearest_neighbors_checked(p(0.), 1, limits()),
            Err(SpatialError::NonFinite)
        );
    }
}
#[test]
fn resource_caps_are_complete_or_error() {
    let mut i = SpatialIndex3D::new();
    i.insert(1, p(0.), b"abc");
    i.insert(2, p(2.), b"0123456789");
    let slot = std::mem::size_of::<CheckedSpatialHit<'_>>();
    let exact = QueryLimits {
        max_work: 4,
        max_results: 1,
        max_output_bytes: slot + 3,
    };
    assert_eq!(
        keys(&i.try_nearest_neighbors_checked(p(0.), 1, exact).unwrap()),
        vec![(1, 0)]
    );
    assert_eq!(
        keys(&i.try_radius_search_checked(p(0.), 0., exact).unwrap()),
        vec![(1, 0)]
    );
    for cap in [
        QueryLimits {
            max_work: 3,
            ..exact
        },
        QueryLimits {
            max_results: 0,
            ..exact
        },
        QueryLimits {
            max_output_bytes: slot + 2,
            ..exact
        },
    ] {
        assert_eq!(
            i.try_nearest_neighbors_checked(p(0.), 1, cap),
            Err(SpatialError::BudgetExceeded)
        );
        assert_eq!(
            i.try_radius_search_checked(p(0.), 0., cap),
            Err(SpatialError::BudgetExceeded)
        );
    }
    assert_eq!(
        i.try_radius_search_checked(p(0.), 2., exact),
        Err(SpatialError::BudgetExceeded)
    );
    assert_eq!(
        i.try_nearest_neighbors_checked(p(0.), usize::MAX, exact),
        Err(SpatialError::BudgetExceeded)
    );
    assert_eq!(
        i.try_nearest_neighbors_checked(p(0.), usize::MAX, limits())
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn invalid_queries_empty_and_cancellation() {
    let i = SpatialIndex3D::new();
    assert_eq!(
        i.try_radius_search_checked(p(0.), 0., limits()).unwrap(),
        []
    );
    assert_eq!(
        i.try_nearest_neighbors_checked(p(0.), 1, limits()).unwrap(),
        []
    );
    assert_eq!(
        i.try_radius_search_checked(p(0.), -1., limits()),
        Err(SpatialError::NegativeRadius)
    );
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            i.try_radius_search_checked(p(0.), bad, limits()),
            Err(SpatialError::NonFinite)
        );
        assert_eq!(
            i.try_radius_search_checked(p(bad), 1., limits()),
            Err(SpatialError::NonFinite)
        );
        assert_eq!(
            i.try_nearest_neighbors_checked(p(bad), 1, limits()),
            Err(SpatialError::NonFinite)
        );
    }
    let cancel = AtomicBool::new(true);
    assert_eq!(
        i.try_radius_search_checked_with_cancel(p(0.), 1., limits(), &cancel),
        Err(SpatialError::Cancelled)
    );
    assert_eq!(
        i.try_nearest_neighbors_checked_with_cancel(p(0.), 1, limits(), &cancel),
        Err(SpatialError::Cancelled)
    );
}
#[test]
fn widening_extrema_subnormals_and_closed_boundaries_match_independent_scan() {
    let mut i = SpatialIndex3D::new();
    let mut rows = Vec::new();
    let values = [
        -f32::MAX,
        f32::MAX,
        -0.,
        0.,
        f32::from_bits(1),
        1.,
        f32::from_bits(1f32.to_bits() + 1),
    ];
    for (ordinal, x) in values.into_iter().enumerate() {
        let point = p(x);
        let id = u32::try_from(10 - ordinal).unwrap();
        i.insert(id, point, b"x");
        rows.push(spatial_oracle::OraclePoint {
            point: Point3::from_legacy(point).unwrap(),
            id,
            ordinal,
        });
    }
    for center in [p(0.), p(-f32::MAX)] {
        let c = Point3::from_legacy(center).unwrap();
        for k in [1, 3, 100] {
            let expected = spatial_oracle::reference_knn(&rows, c, k);
            let actual = i
                .try_nearest_neighbors_checked(center, k, limits())
                .unwrap();
            assert_eq!(
                keys(&actual),
                expected
                    .iter()
                    .map(|h| (h.id, h.ordinal))
                    .collect::<Vec<_>>()
            );
            for (a, e) in actual.iter().zip(expected) {
                assert_eq!(a.distance_f64.to_bits(), e.distance.to_bits());
                assert!(a.distance_f64.is_finite());
            }
        }
        for radius in [-0., f32::from_bits(1), 1., f32::MAX] {
            let expected = spatial_oracle::reference_radius(
                &rows,
                c,
                Radius::try_new(f64::from(radius)).unwrap(),
            );
            assert_eq!(
                keys(
                    &i.try_radius_search_checked(center, radius, limits())
                        .unwrap()
                ),
                expected
                    .iter()
                    .map(|h| (h.id, h.ordinal))
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn bounded_topk_matches_independent_scan_across_thresholds() {
    let mut seed = 0x9e37_79b9u32;
    for size in [1, 2, 3, 7, 63, 64, 65, 127, 256] {
        let mut i = SpatialIndex3D::new();
        let mut rows = Vec::new();
        for ordinal in 0..size {
            let mut coord = || {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (f32::from(u16::try_from(seed >> 16).unwrap()) - 32768.) / 128.
            };
            let point = Point3D {
                x: coord(),
                y: coord(),
                z: coord(),
            };
            let id = u32::try_from(ordinal % 9).unwrap();
            i.insert(id, point, b"borrowed");
            rows.push(spatial_oracle::OraclePoint {
                point: Point3::from_legacy(point).unwrap(),
                id,
                ordinal,
            });
        }
        for center in [p(0.), p(18.), p(-200.)] {
            for k in [1, 2, 3, 7, 16, 100, usize::MAX] {
                let take = k.min(size);
                let cap = QueryLimits {
                    max_work: 2 * size,
                    max_results: take,
                    max_output_bytes: take * (std::mem::size_of::<CheckedSpatialHit<'_>>() + 8),
                };
                let expected =
                    spatial_oracle::reference_knn(&rows, Point3::from_legacy(center).unwrap(), k);
                let actual = i.try_nearest_neighbors_checked(center, k, cap).unwrap();
                assert_eq!(
                    keys(&actual),
                    expected
                        .iter()
                        .map(|h| (h.id, h.ordinal))
                        .collect::<Vec<_>>(),
                    "size={size} k={k}"
                );
            }
        }
    }
}

#[test]
fn rounded_display_distance_does_not_replace_squared_order_key() {
    let mut i = SpatialIndex3D::new();
    i.insert(
        1,
        Point3D {
            x: 1.,
            y: 2f32.powi(-26),
            z: 0.,
        },
        b"farther",
    );
    i.insert(9, p(1.), b"nearer");
    let hits = i.try_nearest_neighbors_checked(p(0.), 2, limits()).unwrap();
    assert_eq!(keys(&hits), vec![(9, 1), (1, 0)]);
    assert_eq!(hits[0].distance_f64, 1.);
    assert_eq!(hits[1].distance_f64, 1.);
    assert_eq!(
        keys(&i.try_radius_search_checked(p(0.), 1., limits()).unwrap()),
        vec![(9, 1)]
    );
}
