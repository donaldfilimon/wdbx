//! Fixed logical-charge-v1 attachment boundaries, independent of target layout.
#[path = "orchestration_v3/support.rs"]
mod support;
use specimen_kernel::{
    identity::NodeRef,
    language_v3::Span,
    orchestration_v3::*,
    scanner_v3::{Rational, routes::*},
    scoring_v3::Policy,
};
use support::*;

fn fixture(entry: u128) -> Fixture {
    let mut f = Fixture::new("hello", "hello", "é", entry);
    if entry == 1 {
        f.origin.captures.push(RouteCapture {
            name: "cap".into(),
            span: Span { start: 0, end: 5 },
            value: CaptureValue::Text("é".into()),
        });
    }
    f
}
fn start<'a>(c: &'a Comparison<'a>, limits: Limits) -> (Broker<'a>, CompletedIssuer) {
    let p = scope();
    let mut b = Broker::new(p.clone(), 1, limits).unwrap();
    let h = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.admit(&p, 1, &h, &[job(1, c, Priority::Foreground)])
        .unwrap();
    let lease = b.dispatch(&p, 1, 1).unwrap().remove(0);
    let value = evaluate_policy(
        1,
        c,
        Policy {
            threshold: -100,
            jitter: false,
            ..Policy::default()
        },
    );
    let proof = b.complete(&p, 1, &lease, value).unwrap().unwrap();
    // Broker274 + cycle256 + job160 + contribution2397 (the original2296
    // fixture plus RouteCapture96, name3, UTF-8 value2).
    assert_eq!(b.counts().owned_bytes, 3087);
    (b, proof)
}
fn link() -> Attachment {
    Attachment {
        from: NodeRef(1),
        to: NodeRef(2),
        direction: Direction::Directed,
        mode: AttachmentMode::Soft(Rational::new(1, 2).unwrap()),
        group: 10,
    }
}
const REMAINDER: Span = Span { start: 0, end: 5 };
// Independent retained ledger: Context320 + Visit80 + Arc reference8
// + RouteCapture96 + name"cap"3 + value"é"2 + parent"parent"6
// + stream"attach"6 =521. A scheduled job adds160 owned bytes and4 cleanup.
const EXTRA: usize = 521;

#[test]
fn attachment_operation_work_minus_one_refuses_without_rng_visit_or_lease_mutation() {
    let a = fixture(1);
    let t = fixture(2);
    let ca = a.comparison();
    let dependencies = (1..=60).collect::<Vec<u128>>();
    let large = attached(&t, &dependencies, 10);
    let retry = attached(&t, &[1], 10);
    let p = scope();
    // Descriptor466+dependencies960; headers400; inherited capture101;
    // parent6+stream6; per-job binding38+dependencies960 =>2937 work.
    // Even with the unrelated retained active job, cleanup=2934 fits below it.
    for (operation_work, accepted) in [(2936, false), (2937, true)] {
        let (mut b, proof) = start(
            &ca,
            Limits {
                operation_work,
                ..Limits::default()
            },
        );
        let h = proof.cycle_handle();
        b.admit(&p, 1, &h, &[job(99, &ca, Priority::Maintenance)])
            .unwrap();
        let unrelated = b.dispatch(&p, 1, 1).unwrap().remove(0);
        let before = b.counts();
        assert_eq!(before.active, 1);
        let mut tape = AffinityTape::new(1, "attach", &[0], 1).unwrap();
        let result = b.attach(
            &p,
            1,
            &proof,
            link(),
            REMAINDER,
            &[job(2, &large, Priority::Foreground)],
            &mut tape,
        );
        if accepted {
            assert_eq!(
                result.unwrap().disposition,
                AttachmentDisposition::Scheduled(1)
            );
        } else {
            assert_eq!(result.unwrap_err(), Error::Backpressure);
            assert_eq!(b.counts(), before);
            assert_eq!(tape.consumed(), 0);
            // Same issuer, job id, target, remainder and tape: a cheaper descriptor
            // succeeds only if neither the visit nor job identity was published.
            assert_eq!(
                b.attach(
                    &p,
                    1,
                    &proof,
                    link(),
                    REMAINDER,
                    &[job(2, &retry, Priority::Foreground)],
                    &mut tape
                )
                .unwrap()
                .disposition,
                AttachmentDisposition::Scheduled(1)
            );
        }
        assert_eq!(tape.consumed(), 1);
        assert_eq!(b.counts().owned_bytes, before.owned_bytes + EXTRA + 160);
        assert_eq!(b.counts().active, 1);
        let after = b.counts();
        assert_eq!(
            b.attach(
                &p,
                1,
                &proof,
                link(),
                REMAINDER,
                &[job(2, &retry, Priority::Foreground)],
                &mut tape
            )
            .unwrap_err(),
            Error::Repeated
        );
        assert_eq!(b.counts(), after);
        assert_eq!(tape.consumed(), 1);
        let target = b.dispatch(&p, 1, 1).unwrap().remove(0);
        assert_eq!(target.job(), 2);
        b.complete(&p, 1, &target, None).unwrap();
        // Attachment refusal/admission did not consume the unrelated active lease.
        b.complete(&p, 1, &unrelated, None).unwrap();
        assert_eq!(b.counts().active, 0);
        let after = b.counts();
        assert_eq!(
            b.complete(&p, 1, &unrelated, None).unwrap_err(),
            Error::InvalidLease
        );
        assert_eq!(b.counts(), after);
    }
}

#[test]
fn attachment_selected_and_declined_owned_boundaries_retain_the_same_fixed_metadata() {
    let a = fixture(1);
    let t = fixture(2);
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
    let p = scope();
    // Before attach: root3087 + expendable cycle256 =3343.
    // Selected adds521+160 =>4024; declined adds521 =>3864.
    for (draw, exact, disposition, jobs) in [
        (0, 4024, AttachmentDisposition::Scheduled(1), 1),
        (u32::MAX, 3864, AttachmentDisposition::AffinityDeclined, 0),
    ] {
        for accepted in [false, true] {
            let (mut b, proof) = start(
                &ca,
                Limits {
                    owned_bytes: exact - usize::from(!accepted),
                    ..Limits::default()
                },
            );
            let expendable = b.begin(&p, 1, 2, CycleLimit::default()).unwrap();
            let before = b.counts();
            let values = [draw];
            let mut tape = AffinityTape::new(1, "attach", &values, 1).unwrap();
            let result = b.attach(
                &p,
                1,
                &proof,
                link(),
                REMAINDER,
                &[job(2, &ct, Priority::Foreground)],
                &mut tape,
            );
            if accepted {
                assert_eq!(result.unwrap().disposition, disposition);
                assert_eq!(b.counts().owned_bytes, exact);
            } else {
                assert_eq!(result.unwrap_err(), Error::Backpressure);
                assert_eq!(b.counts(), before);
                assert_eq!(tape.consumed(), 0);
                // Free256 owned bytes, preserving the root incarnation and issuer.
                b.cancel(&p, 1, &expendable, CancelReason::Requested)
                    .unwrap();
                b.retire_cycle(&p, 1, &expendable).unwrap();
                assert_eq!(
                    b.attach(
                        &p,
                        1,
                        &proof,
                        link(),
                        REMAINDER,
                        &[job(2, &ct, Priority::Foreground)],
                        &mut tape
                    )
                    .unwrap()
                    .disposition,
                    disposition
                );
                assert_eq!(b.counts().owned_bytes, exact - 256);
            }
            assert_eq!(b.counts().pending, jobs);
            assert_eq!(tape.consumed(), 1);
            let after = b.counts();
            assert_eq!(
                b.attach(
                    &p,
                    1,
                    &proof,
                    link(),
                    REMAINDER,
                    &[job(2, &ct, Priority::Foreground)],
                    &mut tape
                )
                .unwrap_err(),
                Error::Repeated
            );
            assert_eq!(b.counts(), after);
            assert_eq!(tape.consumed(), 1);
        }
    }
}

#[test]
fn attachment_cleanup_boundary_reserves_four_cycles_four_jobs_and_retained_extra() {
    let a = fixture(1);
    let t = fixture(2);
    let ca = a.comparison();
    let ct = attached(&t, &[1], 10);
    let p = scope();
    // C=2,J=1,E=2397 before; selected C=2,J=2,E=2918 =>2934.
    // Declined C=2,J=1,E=2918 =>2930. Descriptor work1049 fits both.
    for (draw, exact, disposition, jobs) in [
        (0, 2934, AttachmentDisposition::Scheduled(1), 1),
        (u32::MAX, 2930, AttachmentDisposition::AffinityDeclined, 0),
    ] {
        for accepted in [false, true] {
            let (mut b, proof) = start(
                &ca,
                Limits {
                    operation_work: exact - usize::from(!accepted),
                    ..Limits::default()
                },
            );
            let expendable = b.begin(&p, 1, 2, CycleLimit::default()).unwrap();
            let before = b.counts();
            let values = [draw];
            let mut tape = AffinityTape::new(1, "attach", &values, 1).unwrap();
            let result = b.attach(
                &p,
                1,
                &proof,
                link(),
                REMAINDER,
                &[job(2, &ct, Priority::Foreground)],
                &mut tape,
            );
            if accepted {
                assert_eq!(result.unwrap().disposition, disposition);
                let after = b.counts();
                assert_eq!(
                    b.begin(&p, 1, 3, CycleLimit::default()).unwrap_err(),
                    Error::Backpressure
                );
                assert_eq!(b.counts(), after);
            } else {
                assert_eq!(result.unwrap_err(), Error::Backpressure);
                assert_eq!(b.counts(), before);
                assert_eq!(tape.consumed(), 0);
                // Retirement releases exactly4 cleanup units; no visit/draw was committed.
                b.cancel(&p, 1, &expendable, CancelReason::Requested)
                    .unwrap();
                b.retire_cycle(&p, 1, &expendable).unwrap();
                assert_eq!(
                    b.attach(
                        &p,
                        1,
                        &proof,
                        link(),
                        REMAINDER,
                        &[job(2, &ct, Priority::Foreground)],
                        &mut tape
                    )
                    .unwrap()
                    .disposition,
                    disposition
                );
            }
            assert_eq!(b.counts().pending, jobs);
            assert_eq!(
                b.counts().owned_bytes,
                before.owned_bytes - if accepted { 0 } else { 256 } + EXTRA + jobs * 160
            );
            assert_eq!(tape.consumed(), 1);
        }
    }
}
