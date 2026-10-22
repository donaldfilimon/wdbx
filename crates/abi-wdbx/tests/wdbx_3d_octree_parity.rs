//! Deterministic checked octree parity, resources, and compatibility boundaries.
#[path = "support/spatial_oracle.rs"]
mod spatial_oracle;
use abi_wdbx::spatial::{DistanceMetric, Point3D, SpatialIndex3D};
use abi_wdbx::spatial3d::*;
fn query(selection: PointSelection) -> CheckedPointQuery<'static> {
    CheckedPointQuery {
        center: Point3::try_new(0., 0., 0.).unwrap(),
        selection,
        metric: DistanceMetric::Euclidean,
        limits: QueryLimits {
            max_work: 1_000_000,
            max_results: 10000,
            max_output_bytes: 1_000_000,
        },
        build_limits: OctreeBuildLimits {
            max_nodes: 10000,
            max_scratch_bytes: 10_000_000,
        },
        cancellation: None,
    }
}
fn data(n: usize) -> SpatialIndex3D {
    let mut i = SpatialIndex3D::new();
    for j in 0..n {
        i.insert(
            j as u32,
            Point3D {
                x: j as f32,
                y: 0.,
                z: 0.,
            },
            &[7],
        );
    }
    i
}
#[test]
fn threshold_63_64_65_has_same_semantics() {
    for n in [63, 64, 65] {
        let mut i = data(n);
        let o = checked_octree_query(&mut i, &query(PointSelection::Knn(5))).unwrap();
        assert_eq!(
            o.hits.iter().map(|h| h.id).collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4]
        );
        assert_eq!(
            o.path_used,
            if n < 64 {
                PointQueryPath::CompleteScan(ScanReason::BelowThreshold)
            } else {
                PointQueryPath::Octree
            }
        );
    }
}
#[test]
fn degenerate_points_do_not_explode_tree() {
    let mut i = SpatialIndex3D::new();
    for j in 0..1000 {
        i.insert(j, Point3D::default(), &[]);
    }
    let mut q = query(PointSelection::Knn(3));
    q.build_limits.max_nodes = 1;
    let o = checked_octree_query(&mut i, &q).unwrap();
    assert_eq!(o.path_used, PointQueryPath::Octree);
    assert_eq!(o.built_nodes, 1);
}
#[test]
fn conservative_bounds_do_not_drop_boundary_hits() {
    let mut i = data(65);
    let o = checked_octree_query(
        &mut i,
        &query(PointSelection::Radius(Radius::try_new(8.).unwrap())),
    )
    .unwrap();
    assert_eq!(o.hits.len(), 9);
    assert_eq!(o.hits[8].id, 8);
}
#[test]
fn insert_after_build_invalidates_correctly() {
    let mut i = data(65);
    let q = query(PointSelection::Knn(2));
    assert_eq!(checked_octree_query(&mut i, &q).unwrap().hits[1].id, 1);
    i.insert(999, Point3D::default(), &[9]);
    let o = checked_octree_query(&mut i, &q).unwrap();
    assert_eq!(o.hits[1].id, 999);
}
#[test]
fn unsupported_cosine_pruning_falls_back() {
    let mut i = data(65);
    let mut q = query(PointSelection::Knn(2));
    q.metric = DistanceMetric::Cosine;
    let o = checked_octree_query(&mut i, &q).unwrap();
    assert_eq!(
        o.path_used,
        PointQueryPath::CompleteScan(ScanReason::UnsupportedCosineBound)
    );
}
#[test]
fn thousand_seeded_independent_cases() {
    use spatial_oracle::{OraclePoint, reference_knn, reference_radius};
    let mut accelerated = 0;
    for seed in 0..1000u64 {
        let n = [63, 64, 65][seed as usize % 3];
        let mut state = seed + 1;
        let mut index = SpatialIndex3D::new();
        let mut rows = Vec::new();
        for ordinal in 0..n {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let a = ((state >> 32) as i32 % 1024) as f32 / 16.;
            state = state.rotate_left(17);
            let b = ((state >> 32) as i32 % 1024) as f32 / 16.;
            let p = match seed % 8 {
                0 => Point3D::default(),
                1 => Point3D { x: a, y: 0., z: 0. },
                2 => Point3D { x: a, y: b, z: 0. },
                3 => Point3D {
                    x: a / 1024.,
                    y: b / 1024.,
                    z: a / 2048.,
                },
                4 => Point3D {
                    x: if ordinal % 2 == 0 {
                        f32::MAX
                    } else {
                        -f32::MAX
                    },
                    y: b,
                    z: 0.,
                },
                5 => Point3D {
                    x: f32::from_bits((ordinal % 5) as u32),
                    y: -0.,
                    z: 0.,
                },
                6 => Point3D {
                    x: if ordinal % 2 == 0 { 1. } else { -1. },
                    y: 0.,
                    z: 0.,
                },
                _ => Point3D {
                    x: a,
                    y: b,
                    z: (ordinal % 7) as f32,
                },
            };
            let id = (ordinal % 11) as u32;
            index.insert(id, p, &[(ordinal % 251) as u8]);
            rows.push(OraclePoint {
                point: Point3::from_legacy(p).unwrap(),
                id,
                ordinal,
            });
        }
        for radius in [false, true] {
            let selection = if radius {
                PointSelection::Radius(
                    Radius::try_new(if seed % 5 == 0 { 0. } else { 32. }).unwrap(),
                )
            } else {
                PointSelection::Knn((seed % 70) as usize)
            };
            let mut q = query(selection);
            if seed % 13 == 0 {
                q.center = Point3::try_new(1e150, -1e150, 1e150).unwrap();
            }
            let expected = match selection {
                PointSelection::Knn(k) => reference_knn(&rows, q.center, k),
                PointSelection::Radius(r) => reference_radius(&rows, q.center, r),
            };
            let out = checked_octree_query(&mut index, &q).unwrap();
            accelerated += usize::from(out.path_used == PointQueryPath::Octree);
            assert_eq!(
                out.hits.len(),
                expected.len(),
                "seed {seed} radius {radius}"
            );
            for (actual, want) in out.hits.iter().zip(&expected) {
                assert_eq!(
                    (actual.id, actual.insertion_ordinal, actual.distance_f64),
                    (want.id, want.ordinal, want.distance),
                    "seed {seed} radius {radius}"
                );
                assert_eq!(actual.payload, &[(want.ordinal % 251) as u8]);
            }
            assert_eq!(out.numeric_accuracy, "defined_float_profile");
            assert_eq!(out.candidate_coverage, "exhaustive");
            assert_eq!(out.completion, "complete");
            assert!(out.partial_reason.is_none() && out.continuation.is_none());
            assert_eq!(out.observed_generations, []);
            assert!(out.built_nodes <= q.build_limits.max_nodes);
            assert!(out.reserved_scratch_bytes <= q.build_limits.max_scratch_bytes);
        }
        index.insert(9000, Point3D::default(), &[99]);
        rows.push(OraclePoint {
            point: Point3::from_legacy(Point3D::default()).unwrap(),
            id: 9000,
            ordinal: n,
        });
        let q = query(PointSelection::Knn(7));
        let expected = reference_knn(&rows, q.center, 7);
        let rebuilt = checked_octree_query(&mut index, &q).unwrap();
        assert_eq!(
            rebuilt
                .hits
                .iter()
                .map(|h| (h.id, h.insertion_ordinal))
                .collect::<Vec<_>>(),
            expected
                .iter()
                .map(|h| (h.id, h.ordinal))
                .collect::<Vec<_>>(),
            "rebuild seed {seed}"
        );
    }
    assert!(accelerated > 1000, "actual accelerated cases {accelerated}");
}
#[test]
fn resource_caps_fallback_and_cumulative_work() {
    let mut index = data(65);
    let mut q = query(PointSelection::Knn(4));
    q.build_limits.max_nodes = 0;
    let o = checked_octree_query(&mut index, &q).unwrap();
    assert_eq!(
        o.path_used,
        PointQueryPath::CompleteScan(ScanReason::IndexBudget)
    );
    assert_eq!(o.work_observations, 195);
    q.limits.max_work = 130;
    assert!(matches!(
        checked_octree_query(&mut index, &q),
        Err(SpatialError::BudgetExceeded)
    ));
    q.limits.max_work = 1_000_000;
    q.build_limits.max_scratch_bytes = 0;
    let o = checked_octree_query(&mut index, &q).unwrap();
    assert_eq!(o.work_observations, 130);
    assert_eq!(o.reserved_scratch_bytes, 0);
    q.limits.max_output_bytes = 0;
    assert!(matches!(
        checked_octree_query(&mut index, &q),
        Err(SpatialError::BudgetExceeded)
    ));
}
#[test]
fn invalid_legacy_and_cancelled_queries_fail_closed() {
    use std::sync::atomic::AtomicBool;
    let mut index = data(65);
    index.insert(
        8,
        Point3D {
            x: f32::NAN,
            y: 0.,
            z: 0.,
        },
        &[],
    );
    let q = query(PointSelection::Knn(1));
    assert!(matches!(
        checked_octree_query(&mut index, &q),
        Err(SpatialError::NonFinite)
    ));
    let cancel = AtomicBool::new(true);
    let mut q = query(PointSelection::Knn(1));
    q.cancellation = Some(&cancel);
    assert!(matches!(
        checked_octree_query(&mut index, &q),
        Err(SpatialError::Cancelled)
    ));
    q.cancellation = None;
    q.selection = PointSelection::Knn(0);
    q.limits.max_work = 0;
    assert_eq!(
        checked_octree_query(&mut index, &q).unwrap().path_used,
        PointQueryPath::EmptySelection
    );
}
#[test]
fn manhattan_and_cosine_have_independent_ordering() {
    for metric in [DistanceMetric::Manhattan, DistanceMetric::Cosine] {
        let mut index = data(65);
        let mut q = query(PointSelection::Knn(65));
        q.center = Point3::try_new(0., 2., 3.).unwrap();
        q.metric = metric;
        let out = checked_octree_query(&mut index, &q).unwrap();
        let mut expected: Vec<_> = (0..65u32)
            .map(|id| {
                let distance = if metric == DistanceMetric::Manhattan {
                    f64::from(id) + 5.
                } else {
                    1.
                };
                (distance, id)
            })
            .collect();
        expected.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        assert_eq!(
            out.hits
                .iter()
                .map(|h| (h.distance_f64, h.id))
                .collect::<Vec<_>>(),
            expected
        );
        if metric == DistanceMetric::Manhattan {
            assert_eq!(out.path_used, PointQueryPath::Octree);
        }
    }
}
#[test]
fn cosine_nontrivial_normalization_and_payloads() {
    let mut index = SpatialIndex3D::new();
    for (id, p) in [
        (1, [1., 0., 0.]),
        (2, [4., 3., 0.]),
        (3, [3., 4., 0.]),
        (4, [0., 0., 0.]),
        (5, [-1., 0., 0.]),
    ] {
        index.insert(
            id,
            Point3D {
                x: p[0],
                y: p[1],
                z: p[2],
            },
            &[id as u8],
        );
    }
    for id in 6..70 {
        index.insert(
            id,
            Point3D {
                x: -1.,
                y: 0.,
                z: 0.,
            },
            &[id as u8],
        );
    }
    let mut q = query(PointSelection::Knn(5));
    q.center = Point3::try_new(1., 0., 0.).unwrap();
    q.metric = DistanceMetric::Cosine;
    let out = checked_octree_query(&mut index, &q).unwrap();
    assert_eq!(
        out.path_used,
        PointQueryPath::CompleteScan(ScanReason::UnsupportedCosineBound)
    );
    assert_eq!(
        out.hits
            .iter()
            .map(|h| (h.id, h.distance_f64))
            .collect::<Vec<_>>(),
        vec![(1, 0.), (2, 1. - 0.8), (3, 1. - 0.6), (4, 1.), (5, 2.)]
    );
    for h in out.hits {
        assert_eq!(h.payload, &[h.id as u8]);
    }
}
#[test]
fn fifty_seeded_manhattan_radius_and_knn_oracles() {
    for seed in 0..50usize {
        let mut index = SpatialIndex3D::new();
        let mut expected = Vec::new();
        for ordinal in 0..65usize {
            let p = [
                ((ordinal * 17 + seed * 3) % 21) as f32 - 10.,
                ((ordinal * 7 + seed) % 17) as f32 - 8.,
                ((ordinal * 3 + seed * 11) % 13) as f32 - 6.,
            ];
            let id = (ordinal % 9) as u32;
            index.insert(
                id,
                Point3D {
                    x: p[0],
                    y: p[1],
                    z: p[2],
                },
                &[ordinal as u8],
            );
            let distance = (f64::from(p[0]) - 1.).abs()
                + (f64::from(p[1]) + 2.).abs()
                + (f64::from(p[2]) - 3.).abs();
            expected.push((distance, id, ordinal));
        }
        expected.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        for selection in [
            PointSelection::Knn(seed % 70),
            PointSelection::Radius(Radius::try_new(12.).unwrap()),
        ] {
            let mut q = query(selection);
            q.metric = DistanceMetric::Manhattan;
            q.center = Point3::try_new(1., -2., 3.).unwrap();
            let mut want = expected.clone();
            match selection {
                PointSelection::Knn(k) => want.truncate(k),
                PointSelection::Radius(_) => want.retain(|h| h.0 <= 12.),
            }
            let out = checked_octree_query(&mut index, &q).unwrap();
            assert_eq!(
                out.hits
                    .iter()
                    .map(|h| (h.distance_f64, h.id, h.insertion_ordinal))
                    .collect::<Vec<_>>(),
                want,
                "Manhattan seed {seed}"
            );
            if !matches!(selection, PointSelection::Knn(0)) {
                assert_eq!(out.path_used, PointQueryPath::Octree);
            }
        }
    }
    let mut index = data(65);
    let mut q = query(PointSelection::Radius(Radius::try_new(3.).unwrap()));
    q.metric = DistanceMetric::Manhattan;
    q.center = Point3::try_new(1., 1., 1.).unwrap();
    let out = checked_octree_query(&mut index, &q).unwrap();
    assert_eq!(
        out.hits.iter().map(|h| h.id).collect::<Vec<_>>(),
        vec![1, 0, 2]
    );
}

#[test]
fn radius_payload_budget_exact_and_one_byte_under_on_both_paths() {
    for fallback in [false, true] {
        let mut index = SpatialIndex3D::new();
        index.insert(9, Point3D::default(), &[4; 17]);
        index.insert(
            3,
            Point3D {
                x: 1.,
                y: 0.,
                z: 0.,
            },
            &[5; 23],
        );
        for id in 20..85 {
            index.insert(
                id,
                Point3D {
                    x: 100.,
                    y: 0.,
                    z: 0.,
                },
                &[8; 3],
            );
        }
        let mut q = query(PointSelection::Radius(Radius::try_new(1.).unwrap()));
        if fallback {
            q.build_limits.max_scratch_bytes = 0;
        }
        q.limits.max_output_bytes = 2 * std::mem::size_of::<CheckedSpatialHit<'_>>() + 17 + 23;
        let out = checked_octree_query(&mut index, &q).unwrap();
        assert_eq!(
            out.path_used,
            if fallback {
                PointQueryPath::CompleteScan(ScanReason::IndexBudget)
            } else {
                PointQueryPath::Octree
            }
        );
        assert_eq!(
            out.hits
                .iter()
                .map(|h| (h.id, h.payload.len()))
                .collect::<Vec<_>>(),
            vec![(9, 17), (3, 23)]
        );
        q.limits.max_output_bytes -= 1;
        assert!(matches!(
            checked_octree_query(&mut index, &q),
            Err(SpatialError::BudgetExceeded)
        ));
    }
}
