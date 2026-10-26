#[path = "orchestration_v3/support.rs"]
mod support;
use specimen_kernel::orchestration_v3::*;
use support::*;
#[test]
fn s14_all_1501_admitted_comparisons_drain_without_truncation() {
    let f = Fixture::new("2 + 3", "2 + 3", "a", 1);
    let c = f.comparison();
    let p = scope();
    let mut b = Broker::new(p.clone(), 1, Limits::default()).unwrap();
    let h1 = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    let jobs = (1..=1501)
        .map(|id| job(id, &c, Priority::Foreground))
        .collect::<Vec<_>>();
    b.admit(&p, 1, &h1, &jobs).unwrap();
    let leases = b.dispatch(&p, 1, 1000).unwrap();
    assert_eq!(leases.len(), 1000);
    assert_eq!(b.counts().pending, 501);
    assert_eq!(b.dispatch(&p, 1, 1).unwrap().len(), 0);
    for l in leases {
        b.complete(&p, 1, &l, None).unwrap();
    }
    for l in b.dispatch(&p, 1, 1000).unwrap() {
        b.complete(&p, 1, &l, None).unwrap();
    }
    b.close_discovery(&p, 1, &h1).unwrap();
    b.seal(&p, 1, &h1).unwrap();
    assert_eq!(b.status(&p, 1, &h1).unwrap(), CycleStatus::Ready);
}
#[test]
fn s15_cancel_quarantines_until_generation_bound_fence() {
    let f = Fixture::new("2 + 3", "2 + 3", "a", 1);
    let c = f.comparison();
    let p = scope();
    let mut b = Broker::new(
        p.clone(),
        1,
        Limits {
            active: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    let h1 = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.admit(&p, 1, &h1, &[job(1, &c, Priority::Foreground)])
        .unwrap();
    let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
    b.cancel(&p, 1, &h1, CancelReason::Requested).unwrap();
    assert_eq!(b.counts().quarantined, 1);
    assert_eq!(
        b.complete(&p, 1, &l, f.vote(1)).unwrap_err(),
        Error::Revoked
    );
    let h2 = b.begin(&p, 1, 2, CycleLimit::default()).unwrap();
    b.admit(&p, 1, &h2, &[job(2, &c, Priority::Foreground)])
        .unwrap();
    assert!(b.dispatch(&p, 1, 1).unwrap().is_empty());
    assert_eq!(
        b.acknowledge_fence(&p, 1, 2, &l).unwrap_err(),
        Error::StaleGeneration
    );
    b.acknowledge_fence(&p, 1, 1, &l).unwrap();
    assert_eq!(b.counts().active, 0);
    assert_eq!(
        b.acknowledge_fence(&p, 1, 1, &l).unwrap_err(),
        Error::InvalidLease
    );
    assert_eq!(b.dispatch(&p, 1, 1).unwrap().len(), 1);
}
#[test]
fn complete_graph_preserves_admission_order_not_numeric_job_order() {
    use specimen_kernel::scoring_v3::*;
    let a = Fixture::new("2 + 3", "2 + 3", "first", 1);
    let z = Fixture::new("2 + 3", "2 + 3", "second", 2);
    let ca = a.comparison();
    let cz = z.comparison();
    let p = scope();
    let mut b = Broker::new(p.clone(), 1, Limits::default()).unwrap();
    let h1 = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.admit(
        &p,
        1,
        &h1,
        &[
            job(9, &ca, Priority::Foreground),
            job(1, &cz, Priority::Foreground),
        ],
    )
    .unwrap();
    let leases = b.dispatch(&p, 1, 2).unwrap();
    b.complete(&p, 1, &leases[1], z.vote(1)).unwrap();
    b.complete(&p, 1, &leases[0], a.vote(9)).unwrap();
    b.close_discovery(&p, 1, &h1).unwrap();
    b.seal(&p, 1, &h1).unwrap();
    let plan = b
        .compose(
            &p,
            1,
            &h1,
            &mut DrawTape::new(1, "compose", &[], 0).unwrap(),
            ScoringBudget::default(),
        )
        .unwrap();
    assert_eq!(
        plan.nodes
            .iter()
            .map(|n| n.contribution.id())
            .collect::<Vec<_>>(),
        vec![9, 1]
    );
}
#[test]
fn s14_exact_pending_boundary_atomic_overlimit_and_byte_backpressure() {
    let f = Fixture::new("2 + 3", "2 + 3", "a", 1);
    let c = f.comparison();
    let p = scope();
    let mut b = Broker::new(p.clone(), 1, Limits::default()).unwrap();
    let h1 = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    for batch in 0..100 {
        let jobs = (1..=1000)
            .map(|n| job(batch * 1000 + n, &c, Priority::Foreground))
            .collect::<Vec<_>>();
        b.admit(&p, 1, &h1, &jobs).unwrap();
    }
    assert_eq!(b.counts().pending, 100_000);
    assert_eq!(
        b.admit(&p, 1, &h1, &[job(100001, &c, Priority::Foreground)])
            .unwrap_err(),
        Error::Backpressure
    );
    assert_eq!(b.counts().pending, 100_000);
    let mut tiny = Broker::new(
        p.clone(),
        1,
        Limits {
            owned_bytes: 1024,
            ..Limits::default()
        },
    )
    .unwrap();
    let tiny_h1 = tiny.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    assert_eq!(
        tiny.admit(
            &p,
            1,
            &tiny_h1,
            &(1..10)
                .map(|n| job(n, &c, Priority::Foreground))
                .collect::<Vec<_>>()
        )
        .unwrap_err(),
        Error::Backpressure
    );
    assert_eq!(tiny.counts().pending, 0);
}
#[test]
fn s15_foreground_fifo_eight_to_one_maintenance_even_with_permanent_refill() {
    let f = Fixture::new("2 + 3", "2 + 3", "a", 1);
    let c = f.comparison();
    let p = scope();
    let mut b = Broker::new(
        p.clone(),
        1,
        Limits {
            active: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    let h1 = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.admit(
        &p,
        1,
        &h1,
        &(1..=20)
            .map(|id| job(id, &c, Priority::Foreground))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    b.admit(
        &p,
        1,
        &h1,
        &(101..=104)
            .map(|id| job(id, &c, Priority::Maintenance))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut actual = vec![];
    for n in 0..27 {
        let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
        actual.push(l.job());
        b.complete(&p, 1, &l, None).unwrap();
        b.admit(&p, 1, &h1, &[job(200 + n, &c, Priority::Foreground)])
            .unwrap();
    }
    assert_eq!(actual[8], 101);
    assert_eq!(actual[17], 102);
    assert_eq!(actual[26], 103);
    assert_eq!(&actual[..8], &[1, 2, 3, 4, 5, 6, 7, 8]);
}
#[test]
fn s15_wrong_scope_binding_duplicate_completion_and_restart_are_atomic() {
    let f = Fixture::new("2 + 3", "2 + 3", "a", 1);
    let wrong = Fixture::new("2 + 3", "2 + 3", "wrong", 2);
    let c = f.comparison();
    let p = scope();
    let mut b = Broker::new(
        p.clone(),
        1,
        Limits {
            active: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    let h1 = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    let h2 = b.begin(&p, 1, 2, CycleLimit::default()).unwrap();
    b.admit(
        &p,
        1,
        &h1,
        &[
            job(1, &c, Priority::Foreground),
            job(2, &c, Priority::Foreground),
        ],
    )
    .unwrap();
    b.admit(&p, 1, &h2, &[job(3, &c, Priority::Maintenance)])
        .unwrap();
    let leases = b.dispatch(&p, 1, 2).unwrap();
    let before = b.counts();
    let mut other = p.clone();
    other.tenant = "other".into();
    assert_eq!(
        b.complete(&other, 1, &leases[0], None).unwrap_err(),
        Error::Scope
    );
    assert_eq!(
        b.complete(&p, 1, &leases[0], wrong.vote(1)).unwrap_err(),
        Error::Binding
    );
    assert_eq!(b.counts(), before);
    b.complete(&p, 1, &leases[1], None).unwrap();
    assert_eq!(
        b.complete(&p, 1, &leases[1], None).unwrap_err(),
        Error::InvalidLease
    );
    b.cancel(&p, 1, &h1, CancelReason::Requested).unwrap();
    assert_eq!(b.counts().pending, 1);
    assert_eq!(b.counts().quarantined, 1);
    let second = b.dispatch(&p, 1, 1).unwrap().remove(0);
    assert_eq!(second.cycle(), 2);
    b.restart(&p, 1, 2).unwrap();
    assert_eq!(b.counts().quarantined, 2);
    assert_eq!(
        b.complete(&p, 2, &second, None).unwrap_err(),
        Error::Revoked
    );
    let before_ack = b.counts();
    assert_eq!(
        b.acknowledge_fence(&p, 1, 1, &leases[0]).unwrap_err(),
        Error::StaleGeneration
    );
    assert_eq!(b.counts(), before_ack);
    b.acknowledge_fence(&p, 2, 1, &leases[0]).unwrap();
    assert_eq!(b.counts().quarantined, 1);
    b.acknowledge_fence(&p, 2, 1, &second).unwrap();
    assert_eq!(b.counts().active, 0);
    b.retire_cycle(&p, 2, &h1).unwrap();
    b.retire_cycle(&p, 2, &h2).unwrap();
}
#[test]
fn s15_explicit_deadline_and_discovery_barrier_never_compose_partial() {
    use specimen_kernel::scoring_v3::*;
    let p = scope();
    let mut b = Broker::new(
        p.clone(),
        1,
        Limits {
            trace: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    let h1 = b
        .begin(
            &p,
            1,
            1,
            CycleLimit {
                deadline: Some(10),
                ..CycleLimit::default()
            },
        )
        .unwrap();
    assert_eq!(b.seal(&p, 1, &h1).unwrap_err(), Error::Incomplete);
    assert_eq!(b.poll_deadline(&p, 1, &h1, 9).unwrap(), CycleStatus::Open);
    assert_eq!(b.poll_deadline(&p, 1, &h1, 8).unwrap_err(), Error::Invalid);
    assert_eq!(
        b.poll_deadline(&p, 1, &h1, 10).unwrap(),
        CycleStatus::Partial(CancelReason::Deadline)
    );
    assert_eq!(
        b.compose(
            &p,
            1,
            &h1,
            &mut DrawTape::new(0, "none", &[], 0).unwrap(),
            ScoringBudget::default()
        )
        .unwrap_err(),
        Error::Incomplete
    );
    assert_eq!(b.close_discovery(&p, 1, &h1).unwrap_err(), Error::Closed);
}
#[test]
fn huge_metadata_and_late_invalid_batch_are_refused_before_any_acceptance() {
    let f = Fixture::new("2 + 3", "2 + 3", "a", 1);
    let c = f.comparison();
    let mut huge = Fixture::new("2 + 3", "2 + 3", "b", 2);
    huge.record.ast.alternatives[0].action = vec![specimen_kernel::language::Expr::Call {
        name: "call".into(),
        args: vec![vec![]; 5000],
    }];
    let hc = huge.comparison();
    let p = scope();
    let mut b = Broker::new(p.clone(), 1, Limits::default()).unwrap();
    let h1 = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    let before = b.counts();
    assert_eq!(
        b.admit(
            &p,
            1,
            &h1,
            &[
                job(1, &c, Priority::Foreground),
                job(2, &hc, Priority::Foreground)
            ]
        )
        .unwrap_err(),
        Error::Backpressure
    );
    assert_eq!(b.counts(), before);
    assert_eq!(
        b.admit(
            &p,
            1,
            &h1,
            &[
                job(1, &c, Priority::Foreground),
                job(1, &c, Priority::Maintenance)
            ]
        )
        .unwrap_err(),
        Error::Identity
    );
    assert_eq!(b.counts(), before);
}
#[test]
fn sealed_cycle_rejects_new_jobs_and_expired_ready_cycle_never_composes() {
    use specimen_kernel::scoring_v3::*;
    let f = Fixture::new("2 + 3", "2 + 3", "a", 1);
    let c = f.comparison();
    let p = scope();
    let mut b = Broker::new(p.clone(), 1, Limits::default()).unwrap();
    let h1 = b
        .begin(
            &p,
            1,
            1,
            CycleLimit {
                deadline: Some(5),
                ..CycleLimit::default()
            },
        )
        .unwrap();
    b.close_discovery(&p, 1, &h1).unwrap();
    b.seal(&p, 1, &h1).unwrap();
    assert_eq!(
        b.admit(&p, 1, &h1, &[job(1, &c, Priority::Foreground)])
            .unwrap_err(),
        Error::Closed
    );
    assert_eq!(
        b.poll_deadline(&p, 1, &h1, 5).unwrap(),
        CycleStatus::Partial(CancelReason::Deadline)
    );
    assert_eq!(
        b.compose(
            &p,
            1,
            &h1,
            &mut DrawTape::new(1, "c", &[], 0).unwrap(),
            ScoringBudget::default()
        )
        .unwrap_err(),
        Error::Incomplete
    );
}
#[test]
fn scoped_retirement_releases_only_drained_cancelled_metadata_and_preserves_quarantine() {
    let f = Fixture::new("2 + 3", "2 + 3", "a", 1);
    let c = f.comparison();
    let p = scope();
    let mut b = Broker::new(p.clone(), 1, Limits::default()).unwrap();
    let baseline = b.counts().owned_bytes;
    let mut handles = vec![];
    for id in [1, 2] {
        let handle = b.begin(&p, 1, id, CycleLimit::default()).unwrap();
        b.admit(&p, 1, &handle, &[job(id, &c, Priority::Foreground)])
            .unwrap();
        handles.push(handle);
    }
    let h1 = handles[0].clone();
    let h2 = handles[1].clone();
    let leases = b.dispatch(&p, 1, 2).unwrap();
    b.cancel(&p, 1, &h1, CancelReason::Requested).unwrap();
    assert_eq!(b.retire_cycle(&p, 1, &h1).unwrap_err(), Error::Revoked);
    b.acknowledge_fence(&p, 1, 1, &leases[0]).unwrap();
    b.retire_cycle(&p, 1, &h1).unwrap();
    assert_eq!(b.counts().active, 1);
    b.complete(&p, 1, &leases[1], None).unwrap();
    b.cancel(&p, 1, &h2, CancelReason::Requested).unwrap();
    b.retire_cycle(&p, 1, &h2).unwrap();
    assert_eq!(b.counts().owned_bytes, baseline);
}
#[test]
fn delayed_cycle_controls_cannot_touch_reused_identity() {
    use specimen_kernel::scoring_v3::*;
    let f = Fixture::new("2 + 3", "2 + 3", "a", 1);
    let c = f.comparison();
    let p = scope();
    let mut b = Broker::new(p.clone(), 1, Limits::default()).unwrap();
    let old = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.admit(&p, 1, &old, &[job(1, &c, Priority::Foreground)])
        .unwrap();
    let old_lease = b.dispatch(&p, 1, 1).unwrap().remove(0);
    b.complete(&p, 1, &old_lease, None).unwrap();
    b.cancel(&p, 1, &old, CancelReason::Requested).unwrap();
    b.retire_cycle(&p, 1, &old).unwrap();
    let current = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    let before = b.counts();
    assert_eq!(
        b.admit(&p, 1, &old, &[job(1, &c, Priority::Foreground)])
            .unwrap_err(),
        Error::Identity
    );
    assert_eq!(b.close_discovery(&p, 1, &old).unwrap_err(), Error::Identity);
    assert_eq!(b.seal(&p, 1, &old).unwrap_err(), Error::Identity);
    assert_eq!(
        b.cancel(&p, 1, &old, CancelReason::Requested).unwrap_err(),
        Error::Identity
    );
    assert_eq!(
        b.poll_deadline(&p, 1, &old, u64::MAX).unwrap_err(),
        Error::Identity
    );
    assert_eq!(b.retire_cycle(&p, 1, &old).unwrap_err(), Error::Identity);
    let mut tape = DrawTape::new(1, "c", &[], 0).unwrap();
    assert_eq!(
        b.compose(&p, 1, &old, &mut tape, ScoringBudget::default())
            .unwrap_err(),
        Error::Identity
    );
    assert_eq!(b.counts(), before);
    assert_eq!(b.status(&p, 1, &current).unwrap(), CycleStatus::Open);
    b.admit(&p, 1, &current, &[job(1, &c, Priority::Foreground)])
        .unwrap();
    let current_lease = b.dispatch(&p, 1, 1).unwrap().remove(0);
    assert_eq!(
        b.complete(&p, 1, &old_lease, None).unwrap_err(),
        Error::InvalidLease
    );
    assert_eq!(b.counts().active, 1);
    b.complete(&p, 1, &current_lease, None).unwrap();
    b.close_discovery(&p, 1, &current).unwrap();
    b.seal(&p, 1, &current).unwrap();
    assert!(
        b.compose(&p, 1, &current, &mut tape, ScoringBudget::default())
            .unwrap()
            .nodes
            .is_empty()
    );
    b.retire_cycle(&p, 1, &current).unwrap();
}
