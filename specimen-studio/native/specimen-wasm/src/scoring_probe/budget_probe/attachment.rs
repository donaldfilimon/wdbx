//! Exact attachment-work, retained-owned and cleanup decisions through public APIs.
use super::*;
use specimen_kernel::scanner_v3::Rational;

fn link() -> Attachment {
    Attachment {
        from: NodeRef(1),
        to: NodeRef(2),
        direction: Direction::Directed,
        mode: AttachmentMode::Soft(Rational::new(1, 2).unwrap()),
        group: 10,
    }
}
fn receipt(r: Result<AttachmentReceipt, Error>) -> Value {
    match r {
        Ok(r) => {
            json!({"error":Value::Null,"disposition":format!("{:?}",r.disposition),"trace":r.trace.map(|t|json!({"seed":t.random.seed.to_string(),"stream":t.random.stream,"draw":t.draw,"affinity":t.affinity.map(fraction),"selected":t.selected}))})
        }
        Err(e) => json!({"error":format!("{e:?}"),"disposition":Value::Null,"trace":Value::Null}),
    }
}
fn lease(l: &Lease) -> Value {
    json!({"cycle":l.cycle().to_string(),"job":l.job().to_string(),"generation":l.generation().to_string()})
}
pub(super) fn run() -> (Value, Vec<Value>) {
    let mut a = Fixture::new();
    a.origin.captures.push(RouteCapture {
        name: "cap".into(),
        span: Span { start: 0, end: 5 },
        value: CaptureValue::Text("é".into()),
    });
    let mut t = Fixture::new();
    t.record.node = NodeRef(2);
    t.record.reference = EntryRef(2);
    t.binding.node = NodeRef(2);
    t.binding.entry = EntryRef(2);
    let ca = a.comparison();
    let deps = (1..=60).collect::<Vec<u128>>();
    let mut large = t.comparison();
    large.dependencies = &deps;
    large.attachment_group = Some(10);
    let retry_deps = [1];
    let mut retry_comparison = t.comparison();
    retry_comparison.dependencies = &retry_deps;
    retry_comparison.attachment_group = Some(10);
    let fixture = json!({"issuer":a.input(),"target":t.input(),"work_dependencies":deps.iter().map(ToString::to_string).collect::<Vec<_>>(),"retry_dependencies":["1"],"link":{"from":"1","to":"2","group":"10","direction":"Directed","affinity":fraction(Rational::new(1,2).unwrap())},"remainder":[0,5],"random":{"seed":"1","stream":"attach","limit":1}});
    let mut rows = vec![];
    for (operation, kind, exact, draw) in [
        ("attachment-work", "operation_work", 2937, 0),
        ("attachment-selected-owned", "owned_bytes", 4024, 0),
        ("attachment-declined-owned", "owned_bytes", 3864, u32::MAX),
        ("attachment-selected-cleanup", "operation_work", 2934, 0),
        (
            "attachment-declined-cleanup",
            "operation_work",
            2930,
            u32::MAX,
        ),
    ] {
        for limit in [exact - 1, exact] {
            let mut lim = Limits::default();
            if kind == "owned_bytes" {
                lim.owned_bytes = limit;
            } else {
                lim.operation_work = limit;
            }
            let p = a.record.population.clone();
            let mut b = Broker::new(p.clone(), 1, lim).unwrap();
            let h = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
            b.admit(
                &p,
                1,
                &h,
                &[Job {
                    id: 1,
                    comparison: &ca,
                    priority: Priority::Foreground,
                }],
            )
            .unwrap();
            let root = b.dispatch(&p, 1, 1).unwrap().remove(0);
            let proof = b.complete(&p, 1, &root, Some(a.vote())).unwrap().unwrap();
            let mut expendable = None;
            let mut unrelated = None;
            if operation == "attachment-work" {
                b.admit(
                    &p,
                    1,
                    &h,
                    &[Job {
                        id: 99,
                        comparison: &ca,
                        priority: Priority::Maintenance,
                    }],
                )
                .unwrap();
                unrelated = Some(b.dispatch(&p, 1, 1).unwrap().remove(0));
            } else {
                expendable = Some(b.begin(&p, 1, 2, CycleLimit::default()).unwrap());
            }
            let before = counts(&b);
            let status_before = status(&b, &p, &h);
            let values = [draw];
            let mut tape = AffinityTape::new(1, "attach", &values, 1).unwrap();
            let comparison = if operation == "attachment-work" {
                &large
            } else {
                &retry_comparison
            };
            let draws_before = tape.consumed();
            let output = receipt(b.attach(
                &p,
                1,
                &proof,
                link(),
                Span { start: 0, end: 5 },
                &[Job {
                    id: 2,
                    comparison,
                    priority: Priority::Foreground,
                }],
                &mut tape,
            ));
            let after = counts(&b);
            let status_after = status(&b, &p, &h);
            let draws_after = tape.consumed();
            let mut retry = Value::Null;
            if !output["error"].is_null() {
                let retirement = if let Some(ex) = expendable.take() {
                    b.cancel(&p, 1, &ex, CancelReason::Requested).unwrap();
                    b.retire_cycle(&p, 1, &ex).unwrap();
                    Some("retire-second-cycle")
                } else {
                    None
                };
                let before_retry = counts(&b);
                let d = tape.consumed();
                let r = receipt(b.attach(
                    &p,
                    1,
                    &proof,
                    link(),
                    Span { start: 0, end: 5 },
                    &[Job {
                        id: 2,
                        comparison: &retry_comparison,
                        priority: Priority::Foreground,
                    }],
                    &mut tape,
                ));
                retry = json!({"retirement":retirement,"comparison":"one-dependency","before":before_retry,"draws_before":d,"output":r,"after":counts(&b),"draws_after":tape.consumed()});
            }
            let mut extra_cycle = Value::Null;
            if operation.ends_with("cleanup") && output["error"].is_null() {
                let prior = counts(&b);
                let e = b.begin(&p, 1, 3, CycleLimit::default()).unwrap_err();
                extra_cycle = json!({"error":format!("{e:?}"),"before":prior,"after":counts(&b)});
            }
            let repeated_before = counts(&b);
            let repeated_draws = tape.consumed();
            let repeated = receipt(b.attach(
                &p,
                1,
                &proof,
                link(),
                Span { start: 0, end: 5 },
                &[Job {
                    id: 2,
                    comparison: &retry_comparison,
                    priority: Priority::Foreground,
                }],
                &mut tape,
            ));
            let repeated = json!({"before":repeated_before,"after":counts(&b),"draws_before":repeated_draws,"draws_after":tape.consumed(),"output":repeated});
            let targets = b.dispatch(&p, 1, 1).unwrap();
            let dispatched = targets.iter().map(lease).collect::<Vec<_>>();
            for l in &targets {
                b.complete(&p, 1, l, None).unwrap();
            }
            let mut unrelated_result = Value::Null;
            if let Some(l) = &unrelated {
                let before = counts(&b);
                b.complete(&p, 1, l, None).unwrap();
                let after = counts(&b);
                let e = b.complete(&p, 1, l, None).unwrap_err();
                unrelated_result = json!({"lease":lease(l),"before":before,"after":after,"duplicate_error":format!("{e:?}"),"after_duplicate":counts(&b)});
            }
            let drained = counts(&b);
            b.restart(&p, 1, 2).unwrap();
            b.retire_cycle(&p, 2, &h).unwrap();
            if let Some(ex) = &expendable {
                b.retire_cycle(&p, 2, ex).unwrap();
            }
            rows.push(json!({"operation":operation,"limit_kind":kind,"limit":limit,"broker_limits":limits(lim),"draws":[draw],"before":before,"after":after,"status_before":status_before,"status_after":status_after,"draws_before":draws_before,"draws_after":draws_after,"output":output,"retry":retry,"extra_cycle":extra_cycle,"repeated":repeated,"dispatched":dispatched,"completion_kind":"explicit-host-no-vote","unrelated_completion":unrelated_result,"drained":drained,"after_restart_retire":counts(&b)}));
        }
    }
    (fixture, rows)
}
