#[path = "orchestration_v3/support.rs"]
mod support;
use specimen_kernel::{
    identity::*,
    language_v3::Span,
    orchestration_v3::*,
    scanner_v3::{Rational, routes::*},
};
use support::*;
fn edge(mode: AttachmentMode) -> Attachment {
    Attachment {
        from: NodeRef(1),
        to: NodeRef(2),
        direction: Direction::Directed,
        mode,
        group: 10,
    }
}
#[test]
fn i1_oversized_selected_and_declined_descriptors_are_atomic_and_retryable() {
    for captures in [false, true] {
        for (mode, draw) in [
            (AttachmentMode::Hard, 0),
            (AttachmentMode::Soft(Rational::new(0, 1).unwrap()), 0),
            (AttachmentMode::Soft(Rational::new(1, 2).unwrap()), 0),
            (AttachmentMode::Soft(Rational::new(1, 2).unwrap()), u32::MAX),
        ] {
            let cap = RouteCapture {
                name: "number".into(),
                span: Span { start: 0, end: 1 },
                value: CaptureValue::Text("2".into()),
            };
            let mut a = Fixture::new("2 + 3", "2 + 3", "issuer", 1);
            a.origin.captures = vec![cap.clone()];
            let mut bad = Fixture::new("2 + 3", "2 + 3", "target", 2);
            bad.origin.captures = if captures {
                vec![cap.clone(); 100001]
            } else {
                vec![cap.clone()]
            };
            let mut good = Fixture::new("2 + 3", "2 + 3", "target", 2);
            good.origin.captures = vec![cap];
            let huge = vec![1; 100001];
            let ca = a.comparison();
            let mut bad_c = attached(&bad, &[1], 10);
            if !captures {
                bad_c.dependencies = &huge;
            }
            let good_c = attached(&good, &[1], 10);
            let (mut b, proof) = issuer(&ca);
            let before = b.counts();
            let values = [draw];
            let mut tape = AffinityTape::new(1, "fix1", &values, 1).unwrap();
            assert_eq!(
                b.attach(
                    &scope(),
                    1,
                    &proof,
                    edge(mode),
                    Span { start: 0, end: 5 },
                    &[job(2, &bad_c, Priority::Foreground)],
                    &mut tape
                )
                .unwrap_err(),
                Error::Backpressure
            );
            assert_eq!(b.counts(), before);
            assert_eq!(tape.consumed(), 0);
            let receipt = b
                .attach(
                    &scope(),
                    1,
                    &proof,
                    edge(mode),
                    Span { start: 0, end: 5 },
                    &[job(2, &good_c, Priority::Foreground)],
                    &mut tape,
                )
                .unwrap();
            assert_eq!(
                tape.consumed(),
                usize::from(receipt.trace.unwrap().draw.is_some())
            );
        }
    }
}
#[test]
fn i2_lower_work_limit_reserves_cleanup_before_accepting_more_cycles() {
    let p = scope();
    let mut b = Broker::new(
        p.clone(),
        1,
        Limits {
            operation_work: 3,
            ..Limits::default()
        },
    )
    .unwrap();
    let before = b.counts();
    assert_eq!(
        b.begin(&p, 1, 1, CycleLimit::default()).unwrap_err(),
        Error::Backpressure
    );
    assert_eq!(b.counts(), before);
    b.restart(&p, 1, 2).unwrap();
    assert_eq!(b.counts(), before);
}
#[test]
fn i1_nested_capture_matching_uses_one_lowered_work_meter_and_retry() {
    for mode in [
        AttachmentMode::Hard,
        AttachmentMode::Soft(Rational::new(0, 1).unwrap()),
    ] {
        let caps = (0..64)
            .map(|n| RouteCapture {
                name: format!("capture-{n:02}"),
                span: Span { start: 0, end: 1 },
                value: CaptureValue::Text("2".into()),
            })
            .collect::<Vec<_>>();
        let mut a = Fixture::new("2 + 3", "2 + 3", "issuer", 1);
        a.origin.captures = caps.clone();
        let mut bad = Fixture::new("2 + 3", "2 + 3", "target", 2);
        bad.origin.captures = vec![caps[63].clone()];
        let mut good = Fixture::new("2 + 3", "2 + 3", "target", 2);
        good.origin.captures = vec![caps[0].clone()];
        let ca = a.comparison();
        let bad_c = attached(&bad, &[1], 10);
        let good_c = attached(&good, &[1], 10);
        let p = scope();
        let mut b = Broker::new(
            p.clone(),
            1,
            Limits {
                operation_work: 20000,
                ..Limits::default()
            },
        )
        .unwrap();
        let h = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
        b.admit(&p, 1, &h, &[job(1, &ca, Priority::Foreground)])
            .unwrap();
        let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
        let proof = b.complete(&p, 1, &l, evaluate(1, &ca)).unwrap().unwrap();
        let before = b.counts();
        let mut tape = AffinityTape::new(1, "fix1", &[0], 1).unwrap();
        assert_eq!(
            b.attach(
                &p,
                1,
                &proof,
                edge(mode),
                Span { start: 0, end: 5 },
                &[job(2, &bad_c, Priority::Foreground)],
                &mut tape
            )
            .unwrap_err(),
            Error::Backpressure
        );
        assert_eq!(b.counts(), before);
        assert_eq!(tape.consumed(), 0);
        b.attach(
            &p,
            1,
            &proof,
            edge(mode),
            Span { start: 0, end: 5 },
            &[job(2, &good_c, Priority::Foreground)],
            &mut tape,
        )
        .unwrap();
    }
}
#[test]
fn i1_decline_validates_identity_and_scope_without_reserving_a_full_queue() {
    let a = Fixture::new("2 + 3", "2 + 3", "issuer", 1);
    let t = Fixture::new("2 + 3", "2 + 3", "target", 2);
    let mut other = Fixture::new("2 + 3", "2 + 3", "target", 2);
    other.record.population.tenant = "other".into();
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
    let co = attached(&other, &[1], 10);
    let p = scope();
    let mut b = Broker::new(
        p.clone(),
        1,
        Limits {
            pending: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    let h = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.admit(&p, 1, &h, &[job(1, &ca, Priority::Foreground)])
        .unwrap();
    let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
    let proof = b.complete(&p, 1, &l, evaluate(1, &ca)).unwrap().unwrap();
    b.admit(&p, 1, &h, &[job(3, &ca, Priority::Maintenance)])
        .unwrap();
    let before = b.counts();
    let mut tape = AffinityTape::new(1, "fix1", &[], 0).unwrap();
    let soft = edge(AttachmentMode::Soft(Rational::new(0, 1).unwrap()));
    assert_eq!(
        b.attach(
            &p,
            1,
            &proof,
            soft,
            Span { start: 0, end: 5 },
            &[job(0, &ct, Priority::Foreground)],
            &mut tape
        )
        .unwrap_err(),
        Error::Identity
    );
    assert_eq!(
        b.attach(
            &p,
            1,
            &proof,
            soft,
            Span { start: 0, end: 5 },
            &[job(2, &co, Priority::Foreground)],
            &mut tape
        )
        .unwrap_err(),
        Error::Scope
    );
    assert_eq!(b.counts(), before);
    assert_eq!(
        b.attach(
            &p,
            1,
            &proof,
            soft,
            Span { start: 0, end: 5 },
            &[job(2, &ct, Priority::Foreground)],
            &mut tape
        )
        .unwrap()
        .disposition,
        AttachmentDisposition::AffinityDeclined
    );
    assert_eq!(b.counts().pending, 1);
    assert_eq!(tape.consumed(), 0);
}
#[test]
fn i2_retained_cancelled_records_keep_cleanup_reservation_until_retirement() {
    let p = scope();
    let mut b = Broker::new(
        p.clone(),
        1,
        Limits {
            operation_work: 8,
            ..Limits::default()
        },
    )
    .unwrap();
    let first = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    let second = b.begin(&p, 1, 2, CycleLimit::default()).unwrap();
    assert_eq!(
        b.begin(&p, 1, 3, CycleLimit::default()).unwrap_err(),
        Error::Backpressure
    );
    b.restart(&p, 1, 2).unwrap();
    assert_eq!(
        b.begin(&p, 2, 3, CycleLimit::default()).unwrap_err(),
        Error::Backpressure
    );
    b.retire_cycle(&p, 2, &first).unwrap();
    b.begin(&p, 2, 3, CycleLimit::default()).unwrap();
    assert_eq!(
        b.status(&p, 2, &second).unwrap(),
        CycleStatus::Partial(CancelReason::Restart)
    );
}
#[test]
fn i2_restart_many_small_cycles_has_one_bounded_cleanup_and_keeps_composed_metadata() {
    use specimen_kernel::scoring_v3::*;
    let f = Fixture::new("2 + 3", "2 + 3", "action", 1);
    let c = f.comparison();
    let p = scope();
    let mut b = Broker::new(
        p.clone(),
        1,
        Limits {
            operation_work: 24000,
            ..Limits::default()
        },
    )
    .unwrap();
    let retained = b.begin(&p, 1, 9000, CycleLimit::default()).unwrap();
    b.admit(&p, 1, &retained, &[job(1, &c, Priority::Foreground)])
        .unwrap();
    let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
    b.complete(&p, 1, &l, evaluate(1, &c)).unwrap();
    b.close_discovery(&p, 1, &retained).unwrap();
    b.seal(&p, 1, &retained).unwrap();
    let plan = b
        .compose(
            &p,
            1,
            &retained,
            &mut DrawTape::new(1, "fix1", &[], 0).unwrap(),
            ScoringBudget::default(),
        )
        .unwrap();
    assert_eq!(plan.nodes.len(), 1);
    let retained_extra = b.counts().owned_bytes;
    let owned_payload =
        retained_extra - (p.tenant.len() + p.specimen.len() + p.node_type.len() + 256) - 256 - 160;
    assert!(4 * 2001 + 4 * 2001 + owned_payload <= 24000);
    let mut handles = vec![];
    for id in 1..=2000 {
        let h = b.begin(&p, 1, id, CycleLimit::default()).unwrap();
        b.admit(&p, 1, &h, &[job(id, &c, Priority::Foreground)])
            .unwrap();
        handles.push(h);
    }
    let leases = b.dispatch(&p, 1, 3).unwrap();
    let before = b.counts();
    assert_eq!(before.pending, 1997);
    b.restart(&p, 1, 2).unwrap();
    assert_eq!(b.counts().pending, 0);
    assert_eq!(b.counts().active, 3);
    assert_eq!(b.counts().quarantined, 3);
    assert_eq!(b.counts().owned_bytes, before.owned_bytes);
    assert_eq!(b.status(&p, 2, &retained).unwrap(), CycleStatus::Composed);
    assert_eq!(
        b.status(&p, 2, &handles[1999]).unwrap(),
        CycleStatus::Partial(CancelReason::Restart)
    );
    assert!(b.dispatch(&p, 2, 1000).unwrap().is_empty());
    for lease in &leases {
        assert_eq!(b.complete(&p, 2, lease, None).unwrap_err(), Error::Revoked);
        b.acknowledge_fence(&p, 2, 1, lease).unwrap();
    }
    for h in &handles {
        b.retire_cycle(&p, 2, h).unwrap();
    }
    assert_eq!(b.counts().owned_bytes, retained_extra);
    assert_eq!(plan.nodes[0].contribution.id(), 1);
}
#[test]
fn i2_completion_cleanup_headroom_failure_preserves_active_work() {
    let f = Fixture::new("2 + 3", "2 + 3", "a", 1);
    let c = f.comparison();
    let p = scope();
    let mut b = Broker::new(
        p.clone(),
        1,
        Limits {
            operation_work: 8192,
            ..Limits::default()
        },
    )
    .unwrap();
    let h = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.admit(&p, 1, &h, &[job(1, &c, Priority::Foreground)])
        .unwrap();
    let lease = b.dispatch(&p, 1, 1).unwrap().remove(0);
    let contribution = evaluate(1, &c).unwrap();
    let mut retained = vec![];
    for id in 2..3000 {
        match b.begin(&p, 1, id, CycleLimit::default()) {
            Ok(h) => retained.push(h),
            Err(Error::Backpressure) => break,
            Err(e) => panic!("unexpected {e:?}"),
        }
    }
    assert_eq!(retained.len(), (8192 - 8) / 4);
    let before = b.counts();
    assert_eq!(
        b.complete(&p, 1, &lease, Some(contribution.clone()))
            .unwrap_err(),
        Error::Backpressure
    );
    assert_eq!(b.counts(), before);
    assert_eq!(b.counts().active, 1);
    for h in retained {
        b.cancel(&p, 1, &h, CancelReason::Requested).unwrap();
        b.retire_cycle(&p, 1, &h).unwrap();
    }
    b.complete(&p, 1, &lease, Some(contribution)).unwrap();
    assert_eq!(b.counts().active, 0);
}
#[test]
fn i2_attachment_cleanup_headroom_failure_preserves_rng_visit_and_retry() {
    let a = Fixture::new("2 + 3", "2 + 3", "issuer", 1);
    let t = Fixture::new("2 + 3", "2 + 3", "target", 2);
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
    let p = scope();
    let mut b = Broker::new(
        p.clone(),
        1,
        Limits {
            operation_work: 8192,
            ..Limits::default()
        },
    )
    .unwrap();
    let h = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.admit(&p, 1, &h, &[job(1, &ca, Priority::Foreground)])
        .unwrap();
    let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
    let proof = b.complete(&p, 1, &l, evaluate(1, &ca)).unwrap().unwrap();
    let mut retained = vec![];
    for id in 2..3000 {
        match b.begin(&p, 1, id, CycleLimit::default()) {
            Ok(h) => retained.push(h),
            Err(Error::Backpressure) => break,
            Err(e) => panic!("unexpected {e:?}"),
        }
    }
    let before = b.counts();
    let mut tape = AffinityTape::new(1, "fix1", &[0], 1).unwrap();
    let soft = edge(AttachmentMode::Soft(Rational::new(1, 2).unwrap()));
    assert_eq!(
        b.attach(
            &p,
            1,
            &proof,
            soft,
            Span { start: 0, end: 5 },
            &[job(2, &ct, Priority::Foreground)],
            &mut tape
        )
        .unwrap_err(),
        Error::Backpressure
    );
    assert_eq!(b.counts(), before);
    assert_eq!(tape.consumed(), 0);
    for h in retained.into_iter().take(128) {
        b.cancel(&p, 1, &h, CancelReason::Requested).unwrap();
        b.retire_cycle(&p, 1, &h).unwrap();
    }
    b.attach(
        &p,
        1,
        &proof,
        soft,
        Span { start: 0, end: 5 },
        &[job(2, &ct, Priority::Foreground)],
        &mut tape,
    )
    .unwrap();
    assert_eq!(tape.consumed(), 1);
    assert_eq!(b.counts().pending, 1);
}
