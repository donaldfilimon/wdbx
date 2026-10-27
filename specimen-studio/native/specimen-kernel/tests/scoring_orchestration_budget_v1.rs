//! Public boundaries for canonical logical-charge-v1; no target layout oracle.
#[path = "orchestration_v3/support.rs"]
mod support;
use specimen_kernel::{language::Expr, orchestration_v3::*, scanner_v3::Rational, scoring_v3::*};
use support::*;

fn request(f: &Fixture) -> EntryRequest<'_> {
    let mut r = f.request(1);
    r.policy.threshold = -100;
    r
}
fn contribution(f: &Fixture) -> Contribution {
    evaluate_entry(
        request(f),
        &mut DrawTape::new(0, "test", &[], 0).unwrap(),
        ScoringBudget::default(),
    )
    .unwrap()
    .contribution
    .unwrap()
}
// Independent v1 ledger for source=target="hello", input="input", action="é":
// fixed reserve 2048; two prepared actions at 340 each (two Token160 each,
// original/input/source/lemma UTF-8=20); population18; key14; original5;
// Alternative40+Expr48+literal2; two Alternative40 reserves; two origins5;
// Span16; wildcard "test"4; parent6. Total2971. Work counts items, not
// metadata bytes:2158. The UTF-8 literal has two bytes and one scalar.
const ENTRY_PAYLOAD: usize = 2971;
const ENTRY_WORK: usize = 2158;

#[test]
fn scoring_entry_payload_minus_one_refuses_and_exact_charge_admits_without_draws() {
    let f = Fixture::new("hello", "hello", "é", 1);
    for (payload_bytes, accepted) in [(ENTRY_PAYLOAD - 1, false), (ENTRY_PAYLOAD, true)] {
        let mut tape = DrawTape::new(0, "test", &[], 0).unwrap();
        let result = evaluate_entry(
            request(&f),
            &mut tape,
            ScoringBudget {
                payload_bytes,
                ..ScoringBudget::default()
            },
        );
        if accepted {
            let c = result.unwrap().contribution.unwrap();
            assert!(matches!(c.action(), [Expr::Text { value }] if value == "é"));
        } else {
            assert!(matches!(result, Err(ScoringError::BudgetExceeded)));
        }
        assert_eq!(tape.consumed(), 0);
    }
}

#[test]
fn scoring_work_still_counts_items_independently_of_logical_metadata_bytes() {
    let f = Fixture::new("hello", "hello", "é", 1);
    for (work, accepted) in [(ENTRY_WORK - 1, false), (ENTRY_WORK, true)] {
        let result = evaluate_entry(
            request(&f),
            &mut DrawTape::new(0, "test", &[], 0).unwrap(),
            ScoringBudget {
                work,
                ..ScoringBudget::default()
            },
        );
        assert_eq!(result.is_ok(), accepted);
        if !accepted {
            assert!(matches!(result, Err(ScoringError::BudgetExceeded)));
        }
    }
}

#[test]
fn scoring_batch_uses_fixed_entry_request_reserve() {
    let f = Fixture::new("hello", "hello", "é", 1);
    // One EntryRequest384 plus the independently itemized entry payload2971.
    for (payload_bytes, accepted) in [(3354, false), (3355, true)] {
        let mut tape = DrawTape::new(0, "test", &[], 0).unwrap();
        let result = evaluate_entries(
            &[request(&f)],
            &mut tape,
            ScoringBudget {
                payload_bytes,
                ..ScoringBudget::default()
            },
        );
        assert_eq!(result.is_ok(), accepted);
        if !accepted {
            assert!(matches!(result, Err(ScoringError::BudgetExceeded)));
        }
        assert_eq!(tape.consumed(), 0);
    }
}

#[test]
fn scoring_arbitration_uses_fixed_contribution_and_nested_metadata_reserves() {
    let f = Fixture::new("hello", "hello", "é", 1);
    let c = contribution(&f);
    // Contribution2048 + three traversals of248: record/source strings10,
    // AST90, origins10, path4, parent6, population18, key14, token span16,
    // weight8, two stream names8, four evidence spans64.
    for (payload_bytes, accepted) in [(2791, false), (2792, true)] {
        let mut tape = DrawTape::new(0, "test", &[], 0).unwrap();
        let result = arbitrate(
            std::slice::from_ref(&c),
            &mut tape,
            ScoringBudget {
                payload_bytes,
                ..ScoringBudget::default()
            },
        );
        if accepted {
            assert_eq!(result.unwrap().contributions[0].id(), 1);
        } else {
            assert!(matches!(result, Err(ScoringError::BudgetExceeded)));
        }
        assert_eq!(tape.consumed(), 0);
    }
}

#[test]
fn broker_admission_work_boundary_is_atomic() {
    let f = Fixture::new("hello", "hello", "é", 1);
    let c = f.comparison();
    let p = scope();
    // Job160 + comparison178 + identity64 + binding28 + populations36.
    for (operation_work, accepted) in [(465, false), (466, true)] {
        let mut b = Broker::new(
            p.clone(),
            1,
            Limits {
                operation_work,
                ..Limits::default()
            },
        )
        .unwrap();
        let h = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
        let before = b.counts();
        let result = b.admit(&p, 1, &h, &[job(1, &c, Priority::Foreground)]);
        if accepted {
            result.unwrap();
            assert_eq!(b.counts().pending, 1);
            assert_eq!(b.counts().owned_bytes, before.owned_bytes + 160);
        } else {
            assert_eq!(result.unwrap_err(), Error::Backpressure);
            assert_eq!(b.counts(), before);
            assert!(b.dispatch(&p, 1, 1).unwrap().is_empty());
        }
    }
}

// Contribution2048 + record127 + origins/source15 + token span16 + parent6
// + weight8 + streams8 + evidence spans64 + path4 =2296 retained logical bytes.
// Broker274 + cycle256 + job160 =690 before completion. R=4C+4J+E=2304.
const COMPLETION_BYTES: usize = 2296;
#[test]
fn completion_owned_boundary_preserves_the_lease_on_refusal() {
    let f = Fixture::new("hello", "hello", "é", 1);
    let c = f.comparison();
    let p = scope();
    for (owned_bytes, accepted) in [(2985, false), (2986, true)] {
        let mut b = Broker::new(
            p.clone(),
            1,
            Limits {
                owned_bytes,
                ..Limits::default()
            },
        )
        .unwrap();
        let h = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
        b.admit(&p, 1, &h, &[job(1, &c, Priority::Foreground)])
            .unwrap();
        let lease = b.dispatch(&p, 1, 1).unwrap().remove(0);
        let before = b.counts();
        let result = b.complete(&p, 1, &lease, Some(contribution(&f)));
        if accepted {
            assert!(result.unwrap().is_some());
            assert_eq!(b.counts().owned_bytes, 690 + COMPLETION_BYTES);
            assert_eq!(b.counts().active, 0);
        } else {
            assert_eq!(result.unwrap_err(), Error::Backpressure);
            assert_eq!(b.counts(), before);
            b.complete(&p, 1, &lease, None).unwrap();
            assert_eq!(b.counts().active, 0);
        }
    }
}

#[test]
fn completion_cleanup_headroom_is_four_cycles_plus_four_jobs_plus_retained_bytes() {
    let f = Fixture::new("hello", "hello", "é", 1);
    let c = f.comparison();
    let p = scope();
    for (operation_work, accepted) in [(2303, false), (2304, true)] {
        let mut b = Broker::new(
            p.clone(),
            1,
            Limits {
                operation_work,
                ..Limits::default()
            },
        )
        .unwrap();
        let h = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
        b.admit(&p, 1, &h, &[job(1, &c, Priority::Foreground)])
            .unwrap();
        let lease = b.dispatch(&p, 1, 1).unwrap().remove(0);
        let before = b.counts();
        let result = b.complete(&p, 1, &lease, Some(contribution(&f)));
        if accepted {
            assert!(result.unwrap().is_some());
            assert_eq!(b.counts().owned_bytes, 690 + COMPLETION_BYTES);
            // Exactly filled cleanup reservation cannot admit one more cycle.
            let after = b.counts();
            assert_eq!(
                b.begin(&p, 1, 2, CycleLimit::default()).unwrap_err(),
                Error::Backpressure
            );
            assert_eq!(b.counts(), after);
            b.restart(&p, 1, 2).unwrap();
            b.retire_cycle(&p, 2, &h).unwrap();
            assert_eq!(b.counts().owned_bytes, 274);
        } else {
            assert_eq!(result.unwrap_err(), Error::Backpressure);
            assert_eq!(b.counts(), before);
            // The unconsumed lease can still complete a no-vote result.
            b.complete(&p, 1, &lease, None).unwrap();
        }
    }
}

#[test]
fn overflowing_policy_and_overlarge_budget_refuse_without_consuming_draws() {
    let mut tape = DrawTape::new(0, "test", &[u32::MAX, 1], 2).unwrap();
    assert!(matches!(
        numeric_policy(
            Rational::new(i128::MAX, i128::MAX - 1).unwrap(),
            Policy::default(),
            &mut tape
        ),
        Err(ScoringError::Scan(_))
    ));
    assert_eq!(tape.consumed(), 0);
    let f = Fixture::new("hello", "hello", "é", 1);
    assert!(matches!(
        evaluate_entry(
            request(&f),
            &mut tape,
            ScoringBudget {
                payload_bytes: usize::MAX,
                ..ScoringBudget::default()
            }
        ),
        Err(ScoringError::InvalidBudget)
    ));
    assert_eq!(tape.consumed(), 0);
}
