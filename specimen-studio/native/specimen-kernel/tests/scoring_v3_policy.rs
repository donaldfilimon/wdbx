use specimen_kernel::{scanner_v3::Rational, scoring_v3::*};
fn r(n: i128, d: i128) -> Rational {
    Rational::new(n, d).unwrap()
}
fn policy() -> Policy {
    Policy {
        strength: 1,
        threshold: 50,
        atp: Atp {
            valence: r(-1, 1),
            intensity: r(1, 2),
        },
        jitter: false,
        cap: r(1, 1),
    }
}
#[test]
fn s07_atp_negative_valence_dampens_before_inclusive_gate() {
    let score = numeric_policy(
        r(55, 1),
        policy(),
        &mut DrawTape::new(1, "test", &[], 0).unwrap(),
    )
    .unwrap();
    assert_eq!(score.baseline, r(50, 1));
    assert!(score.qualified);
}
#[test]
fn exact_threshold_neighbors_and_strength_gate() {
    for (n, want) in [
        (49_999_999_999_999_999_999i128, false),
        (50_000_000_000_000_000_000, true),
        (50_000_000_000_000_000_001, true),
    ] {
        let p = Policy {
            jitter: false,
            ..Policy::default()
        };
        let s = numeric_policy(
            r(n, 1_000_000_000_000_000_000),
            p,
            &mut DrawTape::new(7, "threshold", &[], 0).unwrap(),
        )
        .unwrap();
        assert_eq!(s.qualified, want);
    }
    let mut p = Policy {
        jitter: false,
        ..Policy::default()
    };
    p.strength = 0;
    let s = numeric_policy(
        r(100, 1),
        p,
        &mut DrawTape::new(7, "strength", &[], 0).unwrap(),
    )
    .unwrap();
    assert_eq!(s.refusals, vec![Refusal::ZeroStrength]);
    assert_eq!(s.effective, r(100, 1));
}
#[test]
fn s07_endpoints_are_tested_after_atp() {
    let mut p = policy();
    p.jitter = true;
    let mut t = DrawTape::new(9, "atp", &[u32::MAX, 1], 2).unwrap();
    let s = numeric_policy(r(100, 1), p, &mut t).unwrap();
    assert_eq!(s.baseline, r(95, 1));
    assert_eq!(s.effective, r(96, 1));
    assert_eq!(t.consumed(), 2);
    for v in [-100, 100] {
        let mut t = DrawTape::new(9, "endpoints", &[], 0).unwrap();
        let s = numeric_policy(r(v, 1), Policy::default(), &mut t).unwrap();
        assert_eq!(s.jitter_reason, JitterReason::Endpoint);
        assert_eq!(s.effective, r(v, 1));
    }
}
#[test]
fn s08_maximum_strength_only_protects_qualified_baselines() {
    let p = Policy {
        strength: 100,
        ..Policy::default()
    };
    let mut t = DrawTape::new(1, "max", &[u32::MAX, 1], 2).unwrap();
    let a = numeric_policy(r(50, 1), p, &mut t).unwrap();
    assert_eq!(a.jitter_reason, JitterReason::MaximumQualified);
    assert_eq!(t.consumed(), 0);
    let b = numeric_policy(r(49, 1), p, &mut t).unwrap();
    assert_eq!(b.baseline, r(49, 1));
    assert_eq!(b.effective, r(50, 1));
    assert!(b.qualified);
    assert_eq!(t.consumed(), 2);
}
#[test]
fn jitter_cap_sign_independence_no_drift_and_trace() {
    let values = [u32::MAX, 0, u32::MAX, 1];
    let mut t = DrawTape::new(123, "jitter", &values, 4).unwrap();
    let a = numeric_policy(r(50, 1), Policy::default(), &mut t).unwrap();
    let b = numeric_policy(r(50, 1), Policy::default(), &mut t).unwrap();
    assert_eq!(a.effective, r(49, 1));
    assert_eq!(b.baseline, r(50, 1));
    assert_eq!(b.effective, r(51, 1));
    assert_eq!(b.draws[0].index, 2);
    assert_eq!(b.draws[1].purpose, DrawPurpose::JitterSign);
    assert_eq!(b.draws[1].seed, 123);
    let mut t = DrawTape::new(1, "brainstorm", &[u32::MAX, 1], 2).unwrap();
    assert_eq!(
        numeric_policy(r(98, 1), Policy::brainstorm(), &mut t)
            .unwrap()
            .effective,
        r(100, 1)
    );
}
#[test]
fn invalid_policies_and_draw_exhaustion_are_atomic() {
    for p in [
        Policy {
            strength: 101,
            ..Policy::default()
        },
        Policy {
            threshold: 101,
            ..Policy::default()
        },
        Policy {
            cap: r(6, 1),
            ..Policy::default()
        },
        Policy {
            cap: r(1, 1_000_001),
            ..Policy::default()
        },
        Policy {
            atp: Atp {
                valence: r(-2, 1),
                intensity: r(1, 1),
            },
            ..Policy::default()
        },
    ] {
        let mut t = DrawTape::new(0, "invalid", &[1, 1], 2).unwrap();
        assert_eq!(
            numeric_policy(r(50, 1), p, &mut t),
            Err(ScoringError::InvalidPolicy)
        );
        assert_eq!(t.consumed(), 0);
    }
    let mut t = DrawTape::new(0, "short", &[1], 2).unwrap();
    assert_eq!(
        numeric_policy(r(50, 1), Policy::default(), &mut t),
        Err(ScoringError::DrawExhausted)
    );
    assert_eq!(t.consumed(), 0);
    let mut t = DrawTape::new(0, "budget", &[1, 1], 1).unwrap();
    assert_eq!(
        numeric_policy(r(50, 1), Policy::default(), &mut t),
        Err(ScoringError::BudgetExceeded)
    );
    assert_eq!(t.consumed(), 0);
    assert!(Rational::new(1, 0).is_err());
    assert!(Rational::new(1, -1).is_err());
}
#[test]
fn numeric_balance_subtracts_before_clamp_and_positive_atp_does_not_boost() {
    let mut t = DrawTape::new(0, "balance", &[], 0).unwrap();
    let p = Policy {
        jitter: false,
        atp: Atp {
            valence: r(1, 1),
            intensity: r(1, 1),
        },
        ..Policy::default()
    };
    assert_eq!(
        numeric_balance(r(1000, 1), r(950, 1), p, &mut t)
            .unwrap()
            .effective,
        r(50, 1)
    );
    let p = Policy {
        cap: r(0, 1),
        ..Policy::default()
    };
    assert_eq!(
        numeric_policy(r(60, 1), p, &mut t).unwrap().jitter_reason,
        JitterReason::ZeroCap
    );
}
#[test]
fn arithmetic_overflow_is_explicit_and_never_commits_partial_draws() {
    let mut t = DrawTape::new(0, "overflow", &[u32::MAX, 1], 2).unwrap();
    let p = Policy {
        atp: Atp::default(),
        ..Policy::default()
    };
    assert!(matches!(
        numeric_policy(r(i128::MAX, i128::MAX - 1), p, &mut t),
        Err(ScoringError::Scan(_))
    ));
    assert_eq!(t.consumed(), 0);
    assert!(matches!(
        numeric_balance(r(i128::MAX, 1), r(-1, 1), p, &mut t),
        Err(ScoringError::Scan(_))
    ));
    assert_eq!(t.consumed(), 0);
}
#[test]
fn zero_magnitude_still_uses_an_independent_sign_draw() {
    let mut t = DrawTape::new(44, "grid", &[0, 1], 2).unwrap();
    let score = numeric_policy(r(50, 1), Policy::default(), &mut t).unwrap();
    assert_eq!(score.delta, r(0, 1));
    assert_eq!(t.consumed(), 2);
    assert_eq!(score.draws[0].purpose, DrawPurpose::JitterMagnitude);
    assert_eq!(score.draws[1].value, 1);
}
#[test]
fn explicit_seeded_helper_is_repeatable_bounded_and_has_no_ambient_state() {
    let a = seeded_draws(123, 4).unwrap();
    assert_eq!(
        a,
        vec![3_034_356_692, 4_194_450_665, 3_692_221_201, 2_949_776_396]
    );
    assert_eq!(seeded_draws(123, 4).unwrap(), a);
    assert_ne!(seeded_draws(124, 4).unwrap(), a);
    assert_eq!(seeded_draws(0, 100_001), Err(ScoringError::BudgetExceeded));
}
#[test]
fn no_draw_exceptions_still_retain_seed_and_stream_identity() {
    let mut tape = DrawTape::new(77, "endpoint-identity", &[], 0).unwrap();
    let score = numeric_policy(r(100, 1), Policy::default(), &mut tape).unwrap();
    assert_eq!(score.random.seed, 77);
    assert_eq!(score.random.stream, "endpoint-identity");
    assert!(score.draws.is_empty());
}
