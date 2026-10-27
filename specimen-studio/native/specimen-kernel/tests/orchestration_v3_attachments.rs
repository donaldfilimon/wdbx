#[path = "orchestration_v3/support.rs"]
mod support;
use specimen_kernel::{
    identity::*,
    language_v3::*,
    orchestration_v3::*,
    scanner_v3::{Rational, routes::*},
    scoring_v3::*,
};
use support::*;
fn link(mode: AttachmentMode) -> Attachment {
    Attachment {
        from: NodeRef(1),
        to: NodeRef(2),
        direction: Direction::Directed,
        mode,
        group: 10,
    }
}
#[test]
fn s12_hard_evaluates_below_threshold_without_forcing_target_vote() {
    let a = Fixture::new("2 + 3", "2 + 3", "issuer", 1);
    let t = Fixture::new("2 + 3", "unrelated", "target", 2);
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
    let (mut b, proof) = issuer(&ca);
    let h1 = proof.cycle_handle();
    let p = scope();
    let mut tape = AffinityTape::new(1, "attach", &[], 0).unwrap();
    assert_eq!(b.seal(&p, 1, &h1).unwrap_err(), Error::Incomplete);
    let r = b
        .attach(
            &p,
            1,
            &proof,
            link(AttachmentMode::Hard),
            Span { start: 0, end: 5 },
            &[job(2, &ct, Priority::Foreground)],
            &mut tape,
        )
        .unwrap();
    assert_eq!(r.disposition, AttachmentDisposition::Scheduled(1));
    assert_eq!(tape.consumed(), 0);
    assert_eq!(
        b.close_discovery(&p, 1, &h1).unwrap_err(),
        Error::Incomplete
    );
    let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
    assert!(evaluate(2, &ct).is_none());
    b.complete(&p, 1, &l, evaluate(2, &ct)).unwrap();
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
    assert_eq!(plan.nodes.len(), 1);
    assert_eq!(plan.nodes[0].contribution.id(), 1);
}
#[test]
fn s12_affinity_endpoints_and_interior_draws_are_exact() {
    for (n, d, draw, expected, consumed) in [
        (0, 1, 0, false, 0),
        (1, 1, u32::MAX, true, 0),
        (1, 2, 0, true, 1),
        (1, 2, u32::MAX, false, 1),
    ] {
        let a = Fixture::new("2 + 3", "2 + 3", "issuer", 1);
        let t = Fixture::new("2 + 3", "2 + 3", "target", 2);
        let ca = a.comparison();
        let ct = attached(&t, &[1], 10);
        let (mut b, proof) = issuer(&ca);
        let values = [draw];
        let mut tape = AffinityTape::new(7, "affinity", &values, 1).unwrap();
        let r = b
            .attach(
                &scope(),
                1,
                &proof,
                link(AttachmentMode::Soft(Rational::new(n, d).unwrap())),
                Span { start: 0, end: 5 },
                &[job(2, &ct, Priority::Foreground)],
                &mut tape,
            )
            .unwrap();
        assert_eq!(r.trace.as_ref().unwrap().selected, expected);
        assert_eq!(tape.consumed(), consumed);
        assert_eq!(b.counts().pending, usize::from(expected));
        assert_eq!(
            b.attach(
                &scope(),
                1,
                &proof,
                link(AttachmentMode::Soft(Rational::new(n, d).unwrap())),
                Span { start: 0, end: 5 },
                &[job(2, &ct, Priority::Foreground)],
                &mut tape
            )
            .unwrap_err(),
            Error::Repeated
        );
        assert_eq!(tape.consumed(), consumed);
    }
}
#[test]
fn s12_empty_remainder_unicode_wrong_scope_and_issuer_are_refused_safely() {
    let a = Fixture::new("café 2 + 3", "café 2 + 3", "issuer", 1);
    let ca = a.comparison();
    let (mut b, proof) = issuer_policy(
        &ca,
        Policy {
            threshold: -100,
            jitter: false,
            ..Policy::default()
        },
    );
    let mut tape = AffinityTape::new(1, "a", &[], 0).unwrap();
    assert_eq!(
        b.attach(
            &scope(),
            1,
            &proof,
            link(AttachmentMode::Hard),
            Span { start: 5, end: 5 },
            &[],
            &mut tape
        )
        .unwrap()
        .disposition,
        AttachmentDisposition::EmptyRemainder
    );
    assert_eq!(
        b.attach(
            &scope(),
            1,
            &proof,
            link(AttachmentMode::Hard),
            Span { start: 4, end: 5 },
            &[],
            &mut tape
        )
        .unwrap_err(),
        Error::Binding
    );
    let mut wrong = scope();
    wrong.specimen = "other".into();
    assert_eq!(
        b.attach(
            &wrong,
            1,
            &proof,
            link(AttachmentMode::Hard),
            Span { start: 5, end: 5 },
            &[],
            &mut tape
        )
        .unwrap_err(),
        Error::Scope
    );
    let mut wrong_link = link(AttachmentMode::Hard);
    wrong_link.from = NodeRef(3);
    assert_eq!(
        b.attach(
            &scope(),
            1,
            &proof,
            wrong_link,
            Span { start: 5, end: 5 },
            &[],
            &mut tape
        )
        .unwrap_err(),
        Error::Binding
    );
    let (mut other, _) = issuer_policy(
        &ca,
        Policy {
            threshold: -100,
            jitter: false,
            ..Policy::default()
        },
    );
    assert_eq!(
        other
            .attach(
                &scope(),
                1,
                &proof,
                link(AttachmentMode::Hard),
                Span { start: 5, end: 5 },
                &[],
                &mut tape
            )
            .unwrap_err(),
        Error::InvalidLease
    );
}
#[test]
fn s12_remainder_preserves_full_capture_context_and_only_whole_target_authority() {
    let mut a = Fixture::new("café 2 + 3", "café 2 + 3", "issuer", 1);
    a.origin.captures = vec![
        RouteCapture {
            name: "outside".into(),
            span: Span { start: 0, end: 5 },
            value: CaptureValue::Text("café".into()),
        },
        RouteCapture {
            name: "partial".into(),
            span: Span { start: 3, end: 7 },
            value: CaptureValue::Text("é 2".into()),
        },
        RouteCapture {
            name: "inside".into(),
            span: Span { start: 6, end: 7 },
            value: CaptureValue::Text("2".into()),
        },
    ];
    let mut t = Fixture::new("2 + 3", "2 + 3", "target", 2);
    t.origin.captures = vec![RouteCapture {
        name: "inside".into(),
        span: Span { start: 0, end: 1 },
        value: CaptureValue::Text("2".into()),
    }];
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
    let (mut b, proof) = issuer_policy(
        &ca,
        Policy {
            threshold: -100,
            jitter: false,
            ..Policy::default()
        },
    );
    let h1 = proof.cycle_handle();
    let mut tape = AffinityTape::new(1, "a", &[], 0).unwrap();
    b.attach(
        &scope(),
        1,
        &proof,
        link(AttachmentMode::Hard),
        Span { start: 6, end: 11 },
        &[job(2, &ct, Priority::Foreground)],
        &mut tape,
    )
    .unwrap();
    let l = b.dispatch(&scope(), 1, 1).unwrap().remove(0);
    b.complete(&scope(), 1, &l, evaluate(2, &ct)).unwrap();
    b.close_discovery(&scope(), 1, &h1).unwrap();
    b.seal(&scope(), 1, &h1).unwrap();
    let plan = b
        .compose(
            &scope(),
            1,
            &h1,
            &mut DrawTape::new(1, "c", &[], 0).unwrap(),
            ScoringBudget::default(),
        )
        .unwrap();
    let context = plan.nodes[1].attachment.as_ref().unwrap();
    assert_eq!(context.inherited_captures().len(), 3);
    assert_eq!(
        context
            .authoritative_captures()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        vec!["inside"]
    );
    assert_eq!(context.guards(), &ca.origin.constraints);
    assert_eq!(context.source().origin(), ca.source.origin());
    assert_eq!(context.remainder(), "2 + 3");
    assert_eq!(context.parent_lineage(), "parent");
}
#[test]
fn s13_backpressure_does_not_consume_visit_or_rng_and_retry_is_once() {
    let a = Fixture::new("2 + 3", "2 + 3", "issuer", 1);
    let t = Fixture::new("2 + 3", "2 + 3", "target", 2);
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
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
    let h1 = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.admit(&p, 1, &h1, &[job(1, &ca, Priority::Foreground)])
        .unwrap();
    let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
    let proof = b.complete(&p, 1, &l, evaluate(1, &ca)).unwrap().unwrap();
    b.admit(&p, 1, &h1, &[job(3, &ca, Priority::Maintenance)])
        .unwrap();
    let mut tape = AffinityTape::new(1, "a", &[0], 1).unwrap();
    let soft = link(AttachmentMode::Soft(Rational::new(1, 2).unwrap()));
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
    assert_eq!(tape.consumed(), 0);
    let pending = b.dispatch(&p, 1, 1).unwrap().remove(0);
    b.complete(&p, 1, &pending, None).unwrap();
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
        Error::Repeated
    );
    assert_eq!(tape.consumed(), 1);
}
#[test]
fn s13_bidirectional_revisit_stops_without_reset_or_second_draw() {
    let a = Fixture::new("2 + 3", "2 + 3", "a", 1);
    let t = Fixture::new("2 + 3", "2 + 3", "b", 2);
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
    let back = attached(&a, &[2], 10);
    let again = attached(&t, &[3], 10);
    let (mut b, first) = issuer(&ca);
    let mut tape = AffinityTape::new(1, "a", &[], 0).unwrap();
    let mut edge = link(AttachmentMode::Hard);
    edge.direction = Direction::Bidirectional;
    b.attach(
        &scope(),
        1,
        &first,
        edge,
        Span { start: 0, end: 5 },
        &[job(2, &ct, Priority::Foreground)],
        &mut tape,
    )
    .unwrap();
    let l = b.dispatch(&scope(), 1, 1).unwrap().remove(0);
    let second = b
        .complete(&scope(), 1, &l, evaluate(2, &ct))
        .unwrap()
        .unwrap();
    b.attach(
        &scope(),
        1,
        &second,
        edge,
        Span { start: 0, end: 5 },
        &[job(3, &back, Priority::Foreground)],
        &mut tape,
    )
    .unwrap();
    let l = b.dispatch(&scope(), 1, 1).unwrap().remove(0);
    let third = b
        .complete(&scope(), 1, &l, evaluate(3, &back))
        .unwrap()
        .unwrap();
    assert_eq!(
        b.attach(
            &scope(),
            1,
            &third,
            edge,
            Span { start: 0, end: 5 },
            &[job(4, &again, Priority::Foreground)],
            &mut tape
        )
        .unwrap_err(),
        Error::Repeated
    );
    assert_eq!(b.counts().pending, 0);
    assert_eq!(tape.consumed(), 0);
}
#[test]
fn s13_fifth_attachment_depth_is_refused_with_no_recursive_dispatch() {
    let fixtures = (1..=6)
        .map(|n| Fixture::new("2 + 3", "2 + 3", "step", n))
        .collect::<Vec<_>>();
    let deps = (1..=5).map(|id| vec![id]).collect::<Vec<_>>();
    let root = fixtures[0].comparison();
    let comparisons = fixtures
        .iter()
        .skip(1)
        .zip(&deps)
        .map(|(f, d)| attached(f, d, 10))
        .collect::<Vec<_>>();
    let (mut b, mut proof) = issuer(&root);
    let mut tape = AffinityTape::new(1, "a", &[], 0).unwrap();
    for (i, c) in comparisons.iter().enumerate() {
        let id = i as u128 + 2;
        let edge = Attachment {
            from: NodeRef(id - 1),
            to: NodeRef(id),
            direction: Direction::Directed,
            mode: AttachmentMode::Hard,
            group: 10,
        };
        let result = b.attach(
            &scope(),
            1,
            &proof,
            edge,
            Span { start: 0, end: 5 },
            &[job(id, c, Priority::Foreground)],
            &mut tape,
        );
        if i == 4 {
            assert_eq!(result.unwrap_err(), Error::Depth);
            assert_eq!(b.counts().pending, 0);
        } else {
            result.unwrap();
            assert_eq!(b.counts().active, 0);
            let l = b.dispatch(&scope(), 1, 1).unwrap().remove(0);
            proof = b
                .complete(&scope(), 1, &l, evaluate(id, c))
                .unwrap()
                .unwrap();
        }
    }
}
#[test]
fn s12_all_explicit_target_entries_share_pool_and_group_survives() {
    let a = Fixture::new("2 + 3", "2 + 3", "issuer", 1);
    let mut t = Fixture::new("2 + 3", "2 + 3", "second", 3);
    t.record.node = NodeRef(2);
    t.binding.node = NodeRef(2);
    let z = Fixture::new("2 + 3", "2 + 3", "first", 2);
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
    let cz = attached(&z, &[1], 10);
    let (mut b, proof) = issuer(&ca);
    let h1 = proof.cycle_handle();
    let mut tape = AffinityTape::new(1, "a", &[], 0).unwrap();
    b.attach(
        &scope(),
        1,
        &proof,
        link(AttachmentMode::Hard),
        Span { start: 0, end: 5 },
        &[
            job(2, &cz, Priority::Foreground),
            job(3, &ct, Priority::Foreground),
        ],
        &mut tape,
    )
    .unwrap();
    assert_eq!(b.counts().pending, 2);
    let leases = b.dispatch(&scope(), 1, 2).unwrap();
    for l in leases {
        b.complete(
            &scope(),
            1,
            &l,
            if l.job() == 2 {
                evaluate(2, &cz)
            } else {
                evaluate(3, &ct)
            },
        )
        .unwrap();
    }
    b.close_discovery(&scope(), 1, &h1).unwrap();
    b.seal(&scope(), 1, &h1).unwrap();
    let plan = b
        .compose(
            &scope(),
            1,
            &h1,
            &mut DrawTape::new(1, "c", &[], 0).unwrap(),
            ScoringBudget::default(),
        )
        .unwrap();
    assert_eq!(plan.nodes.len(), 3);
    assert_eq!(plan.groups.len(), 1);
    assert_eq!(plan.groups[0].members, vec![1, 2, 3]);
    assert_eq!(plan.groups[0].issuers, vec![1]);
    assert!(plan.arbitration.is_empty());
}
#[test]
fn cancellation_drops_owned_attachment_payload_without_touching_other_cycle() {
    let a = Fixture::new("2 + 3", "2 + 3", "issuer", 1);
    let t = Fixture::new("2 + 3", "2 + 3", "target", 2);
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
    let (mut b, proof) = issuer(&ca);
    let h1 = proof.cycle_handle();
    let h2 = b.begin(&scope(), 1, 2, CycleLimit::default()).unwrap();
    b.admit(&scope(), 1, &h2, &[job(9, &ca, Priority::Maintenance)])
        .unwrap();
    let mut tape = AffinityTape::new(1, "a", &[], 0).unwrap();
    b.attach(
        &scope(),
        1,
        &proof,
        link(AttachmentMode::Hard),
        Span { start: 0, end: 5 },
        &[job(2, &ct, Priority::Foreground)],
        &mut tape,
    )
    .unwrap();
    let before = b.counts().owned_bytes;
    b.cancel(&scope(), 1, &h1, CancelReason::Requested).unwrap();
    assert!(b.counts().owned_bytes < before);
    assert_eq!(b.counts().pending, 1);
    let l = b.dispatch(&scope(), 1, 1).unwrap().remove(0);
    assert_eq!(l.cycle(), 2);
    b.complete(&scope(), 1, &l, None).unwrap();
}
#[test]
fn contextual_capture_cannot_be_rebound_into_delegated_target() {
    let mut a = Fixture::new("café 2 + 3", "café 2 + 3", "issuer", 1);
    a.origin.captures = vec![RouteCapture {
        name: "outside".into(),
        span: Span { start: 0, end: 5 },
        value: CaptureValue::Text("café".into()),
    }];
    let mut t = Fixture::new("2 + 3", "2 + 3", "target", 2);
    t.origin.captures = vec![RouteCapture {
        name: "outside".into(),
        span: Span { start: 0, end: 1 },
        value: CaptureValue::Text("café".into()),
    }];
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
    let (mut b, proof) = issuer_policy(
        &ca,
        Policy {
            threshold: -100,
            jitter: false,
            ..Policy::default()
        },
    );
    let before = b.counts();
    let mut tape = AffinityTape::new(1, "a", &[], 0).unwrap();
    assert_eq!(
        b.attach(
            &scope(),
            1,
            &proof,
            link(AttachmentMode::Hard),
            Span { start: 6, end: 11 },
            &[job(2, &ct, Priority::Foreground)],
            &mut tape
        )
        .unwrap_err(),
        Error::Binding
    );
    assert_eq!(b.counts(), before);
    assert_eq!(tape.consumed(), 0);
}
#[test]
fn attachment_foreground_fifo_and_maintenance_use_one_capacity() {
    let a = Fixture::new("2 + 3", "2 + 3", "issuer", 1);
    let t = Fixture::new("2 + 3", "2 + 3", "target", 2);
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
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
    b.admit(&p, 1, &h1, &[job(1, &ca, Priority::Foreground)])
        .unwrap();
    let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
    let proof = b.complete(&p, 1, &l, evaluate(1, &ca)).unwrap().unwrap();
    b.admit(
        &p,
        1,
        &h1,
        &(10..=17)
            .map(|id| job(id, &ca, Priority::Foreground))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    b.admit(&p, 1, &h1, &[job(99, &ca, Priority::Maintenance)])
        .unwrap();
    b.attach(
        &p,
        1,
        &proof,
        link(AttachmentMode::Hard),
        Span { start: 0, end: 5 },
        &[job(2, &ct, Priority::Foreground)],
        &mut AffinityTape::new(1, "a", &[], 0).unwrap(),
    )
    .unwrap();
    let mut ids = vec![];
    for _ in 0..10 {
        let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
        ids.push(l.job());
        assert_eq!(b.counts().active, 1);
        b.complete(&p, 1, &l, None).unwrap();
    }
    assert_eq!(ids, vec![10, 11, 12, 13, 14, 15, 16, 17, 99, 2]);
}
#[test]
fn retired_cycle_receipt_cannot_authorize_reused_cycle_and_job_ids() {
    let a = Fixture::new("2 + 3", "2 + 3", "issuer", 1);
    let t = Fixture::new("2 + 3", "2 + 3", "target", 2);
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
    let (mut b, old) = issuer(&ca);
    let h1 = old.cycle_handle();
    b.cancel(&scope(), 1, &h1, CancelReason::Requested).unwrap();
    b.retire_cycle(&scope(), 1, &h1).unwrap();
    let h1 = b.begin(&scope(), 1, 1, CycleLimit::default()).unwrap();
    b.admit(&scope(), 1, &h1, &[job(1, &ca, Priority::Foreground)])
        .unwrap();
    let l = b.dispatch(&scope(), 1, 1).unwrap().remove(0);
    let new = b
        .complete(&scope(), 1, &l, evaluate(1, &ca))
        .unwrap()
        .unwrap();
    let mut tape = AffinityTape::new(1, "a", &[], 0).unwrap();
    assert_eq!(
        b.attach(
            &scope(),
            1,
            &old,
            link(AttachmentMode::Hard),
            Span { start: 0, end: 5 },
            &[job(2, &ct, Priority::Foreground)],
            &mut tape
        )
        .unwrap_err(),
        Error::InvalidLease
    );
    b.attach(
        &scope(),
        1,
        &new,
        link(AttachmentMode::Hard),
        Span { start: 0, end: 5 },
        &[job(2, &ct, Priority::Foreground)],
        &mut tape,
    )
    .unwrap();
}
