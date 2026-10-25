use super::{entries::*, numeric::SEED, project::*};
use serde_json::{Value, json};
use specimen_kernel::{language_v3::Span, orchestration_v3::*, scoring_v3::*};
fn counts(b: &Broker<'_>) -> Value {
    let c = b.counts();
    json!({"pending":c.pending,"active":c.active,"quarantined":c.quarantined,"owned_bytes":c.owned_bytes,"owned_within_limit":c.owned_bytes<=Limits::default().owned_bytes})
}
fn outcome<T>(r: Result<T, Error>) -> String {
    match r {
        Ok(_) => "Ok".into(),
        Err(e) => format!("{e:?}"),
    }
}
fn event(b: &Broker<'_>, op: Value, result: Value) -> Value {
    json!({"command":op,"result":result,"counts":counts(b)})
}
fn lease(l: &Lease) -> Value {
    json!({"cycle":l.cycle().to_string(),"job":l.job().to_string(),"generation":l.generation().to_string()})
}
fn plan(p: &CompletePlan<'_>) -> Value {
    json!({"cycle":p.cycle.to_string(),"generation":p.generation.to_string(),"nodes":p.nodes.iter().map(|n|json!({"contribution":contribution(&n.contribution),"attachment":n.attachment.as_ref().map(|a|json!({"issuer":a.issuer().to_string(),"group":a.group().to_string(),"depth":a.depth(),"remainder":a.remainder(),"span":span(a.span()),"parent":a.parent_lineage(),"guards":constraints(a.guards()),"captures":captures(a.inherited_captures())}))})).collect::<Vec<_>>(),"groups":p.groups.iter().map(|g|json!({"id":g.id.to_string(),"issuers":g.issuers.iter().map(ToString::to_string).collect::<Vec<_>>(),"members":g.members.iter().map(ToString::to_string).collect::<Vec<_>>()})).collect::<Vec<_>>(),"arbitration":p.arbitration.iter().map(arbitration).collect::<Vec<_>>()})
}
fn limits(l: Limits) -> Value {
    json!({"active":l.active,"pending":l.pending,"owned_bytes":l.owned_bytes,"operation_work":l.operation_work,"trace":l.trace})
}
fn fairness() -> Value {
    let f = Fixture::new("2 + 3", "2 + 3", "fairness", BASE + 1, BASE + 2);
    let c = f.comparison();
    let p = scope();
    let lim = Limits {
        active: 2,
        pending: 20,
        ..Limits::default()
    };
    let mut b = Broker::new(p.clone(), 1, lim).unwrap();
    let mut events = vec![];
    let h = b.begin(&p, 1, BASE + 1, CycleLimit::default()).unwrap();
    events.push(event(
        &b,
        json!({"op":"begin","cycle":h.id().to_string(),"generation":"1"}),
        json!("Ok"),
    ));
    let jobs = (1..=12)
        .map(|i| Job {
            id: BASE + 200 + i,
            comparison: &c,
            priority: if i <= 10 {
                Priority::Foreground
            } else {
                Priority::Maintenance
            },
        })
        .collect::<Vec<_>>();
    let r = outcome(b.admit(&p, 1, &h, &jobs));
    events.push(event(&b,json!({"op":"admit","cycle":h.id().to_string(),"jobs":jobs.iter().map(|j|json!({"id":j.id.to_string(),"priority":format!("{:?}",j.priority)})).collect::<Vec<_>>()}),json!(r)));
    let r = outcome(b.seal(&p, 1, &h));
    events.push(event(
        &b,
        json!({"op":"seal","cycle":h.id().to_string()}),
        json!(r),
    ));
    let r = outcome(b.compose(
        &p,
        1,
        &h,
        &mut DrawTape::new(SEED, "fair-compose", &[0], 1).unwrap(),
        ScoringBudget::default(),
    ));
    events.push(event(
        &b,
        json!({"op":"compose","cycle":h.id().to_string()}),
        json!(r),
    ));
    while b.counts().pending > 0 {
        let leases = b.dispatch(&p, 1, 10).unwrap();
        events.push(event(
            &b,
            json!({"op":"dispatch","maximum":10}),
            json!(leases.iter().map(lease).collect::<Vec<_>>()),
        ));
        let none = b.dispatch(&p, 1, 1).unwrap();
        events.push(event(
            &b,
            json!({"op":"dispatch","maximum":1}),
            json!(none.iter().map(lease).collect::<Vec<_>>()),
        ));
        for l in leases {
            let o = evaluate(l.job(), &c);
            let observed = json!({"score":score(&o.score),"contribution":o.contribution.as_ref().map(contribution)});
            let r = outcome(b.complete(&p, 1, &l, o.contribution));
            events.push(event(
                &b,
                json!({"op":"complete","lease":lease(&l),"evaluation":observed}),
                json!(r),
            ));
            let r = outcome(b.complete(&p, 1, &l, None));
            events.push(event(
                &b,
                json!({"op":"complete-duplicate","lease":lease(&l)}),
                json!(r),
            ));
        }
    }
    let r = outcome(b.close_discovery(&p, 1, &h));
    events.push(event(
        &b,
        json!({"op":"close","cycle":h.id().to_string()}),
        json!(r),
    ));
    let r = outcome(b.seal(&p, 1, &h));
    events.push(event(
        &b,
        json!({"op":"seal","cycle":h.id().to_string()}),
        json!(r),
    ));
    let mut tape = DrawTape::new(SEED, "fair-compose", &[0], 1).unwrap();
    let result = b
        .compose(&p, 1, &h, &mut tape, ScoringBudget::default())
        .unwrap();
    events.push(event(&b,json!({"op":"compose","cycle":h.id().to_string(),"seed":SEED.to_string(),"stream":"fair-compose","draws":[0]}),plan(&result)));
    json!({"id":"shared-cap-fairness-drain","limits":limits(lim),"input":f.input(BASE+201),"initial":{"generation":"1","counts":{"pending":0,"active":0,"quarantined":0,"owned_within_limit":true}},"events":events})
}
fn quarantine() -> Value {
    let f = Fixture::new("2 + 3", "2 + 3", "quarantine", BASE + 1, BASE + 2);
    let c = f.comparison();
    let p = scope();
    let lim = Limits {
        active: 1,
        pending: 4,
        ..Limits::default()
    };
    let mut b = Broker::new(p.clone(), 1, lim).unwrap();
    let mut es = vec![];
    let h = b.begin(&p, 1, BASE + 3, CycleLimit::default()).unwrap();
    es.push(event(
        &b,
        json!({"op":"begin","cycle":h.id().to_string(),"generation":"1"}),
        json!("Ok"),
    ));
    let js = [
        Job {
            id: BASE + 301,
            comparison: &c,
            priority: Priority::Foreground,
        },
        Job {
            id: BASE + 302,
            comparison: &c,
            priority: Priority::Maintenance,
        },
    ];
    b.admit(&p, 1, &h, &js).unwrap();
    es.push(event(&b,json!({"op":"admit","cycle":h.id().to_string(),"jobs":js.iter().map(|j|json!({"id":j.id.to_string(),"priority":format!("{:?}",j.priority)})).collect::<Vec<_>>()}),json!("Ok")));
    let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
    es.push(event(
        &b,
        json!({"op":"dispatch","maximum":1}),
        json!([lease(&l)]),
    ));
    b.cancel(&p, 1, &h, CancelReason::Requested).unwrap();
    es.push(event(
        &b,
        json!({"op":"cancel","cycle":h.id().to_string()}),
        json!("Ok"),
    ));
    let value = evaluate(l.job(), &c);
    let r = outcome(b.complete(&p, 1, &l, value.contribution));
    es.push(event(
        &b,
        json!({"op":"late-result","lease":lease(&l),"score":score(&value.score)}),
        json!(r),
    ));
    b.restart(&p, 1, 2).unwrap();
    es.push(event(
        &b,
        json!({"op":"restart","from":"1","to":"2"}),
        json!("Ok"),
    ));
    let r = outcome(b.acknowledge_fence(&p, 1, 1, &l));
    es.push(event(
        &b,
        json!({"op":"ack","owner":"1","worker":"1","lease":lease(&l)}),
        json!(r),
    ));
    let r = outcome(b.acknowledge_fence(&p, 2, 2, &l));
    es.push(event(
        &b,
        json!({"op":"ack","owner":"2","worker":"2","lease":lease(&l)}),
        json!(r),
    ));
    let h2 = b.begin(&p, 2, BASE + 4, CycleLimit::default()).unwrap();
    es.push(event(
        &b,
        json!({"op":"begin","cycle":h2.id().to_string(),"generation":"2"}),
        json!("Ok"),
    ));
    let js = [Job {
        id: BASE + 303,
        comparison: &c,
        priority: Priority::Foreground,
    }];
    b.admit(&p, 2, &h2, &js).unwrap();
    es.push(event(&b,json!({"op":"admit","cycle":h2.id().to_string(),"jobs":[{"id":(BASE+303).to_string(),"priority":"Foreground"}]}),json!("Ok")));
    let ls = b.dispatch(&p, 2, 1).unwrap();
    es.push(event(
        &b,
        json!({"op":"dispatch","maximum":1}),
        json!(ls.iter().map(lease).collect::<Vec<_>>()),
    ));
    b.acknowledge_fence(&p, 2, 1, &l).unwrap();
    es.push(event(
        &b,
        json!({"op":"ack","owner":"2","worker":"1","lease":lease(&l)}),
        json!("Ok"),
    ));
    let r = outcome(b.acknowledge_fence(&p, 2, 1, &l));
    es.push(event(
        &b,
        json!({"op":"ack","owner":"2","worker":"1","lease":lease(&l)}),
        json!(r),
    ));
    let r = outcome(b.complete(&p, 2, &l, None));
    es.push(event(
        &b,
        json!({"op":"late-result","lease":lease(&l),"score":Value::Null}),
        json!(r),
    ));
    b.retire_cycle(&p, 2, &h).unwrap();
    es.push(event(
        &b,
        json!({"op":"retire","cycle":h.id().to_string()}),
        json!("Ok"),
    ));
    let new = b.begin(&p, 2, h.id(), CycleLimit::default()).unwrap();
    es.push(event(
        &b,
        json!({"op":"begin","cycle":new.id().to_string(),"generation":"2"}),
        json!("Ok"),
    ));
    let r = outcome(b.seal(&p, 2, &h));
    es.push(event(
        &b,
        json!({"op":"stale-handle-seal","cycle":h.id().to_string(),"handle_generation":h.generation().to_string(),"current_generation":"2"}),
        json!(r),
    ));
    let l2 = b.dispatch(&p, 2, 1).unwrap().remove(0);
    es.push(event(
        &b,
        json!({"op":"dispatch","maximum":1}),
        json!([lease(&l2)]),
    ));
    let o = evaluate(l2.job(), &c);
    let r = outcome(b.complete(&p, 2, &l2, o.contribution));
    es.push(event(
        &b,
        json!({"op":"complete","lease":lease(&l2),"evaluation":{"score":score(&o.score)}}),
        json!(r),
    ));
    let mut wrong = p.clone();
    wrong.tenant = "wrong".into();
    let r = outcome(b.dispatch(&wrong, 2, 1));
    es.push(event(
        &b,
        json!({"op":"wrong-scope-dispatch","maximum":1}),
        json!(r),
    ));
    json!({"id":"cancel-restart-quarantine","limits":limits(lim),"input":f.input(BASE+301),"initial":{"generation":"1","counts":{"pending":0,"active":0,"quarantined":0,"owned_within_limit":true}},"events":es})
}
fn attachment() -> Value {
    let a = Fixture::new("2 + 3", "2 + 3", "issuer", BASE + 1, BASE + 2);
    let mut t = Fixture::new("2 + 3", "2 + 3", "target", BASE + 5, BASE + 6);
    let mut low = Fixture::new("2 + 3", "unrelated words", "low", BASE + 5, BASE + 7);
    for f in [&mut t, &mut low] {
        f.deps = vec![BASE + 401];
        f.group = Some(BASE + 90);
    }
    let ca = a.comparison();
    let ct = t.comparison();
    let cl = low.comparison();
    let p = scope();
    let lim = Limits {
        active: 2,
        pending: 8,
        ..Limits::default()
    };
    let mut b = Broker::new(p.clone(), 1, lim).unwrap();
    let h = b.begin(&p, 1, BASE + 8, CycleLimit::default()).unwrap();
    b.admit(
        &p,
        1,
        &h,
        &[Job {
            id: BASE + 401,
            comparison: &ca,
            priority: Priority::Foreground,
        }],
    )
    .unwrap();
    let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
    let issuer = evaluate(l.job(), &ca);
    let proof = b
        .complete(&p, 1, &l, issuer.contribution.clone())
        .unwrap()
        .unwrap();
    let link = Attachment {
        from: a.record.node,
        to: t.record.node,
        direction: Direction::Directed,
        mode: AttachmentMode::Hard,
        group: BASE + 90,
    };
    let mut tape = AffinityTape::new(SEED, "attachment", &[], 0).unwrap();
    let js = [
        Job {
            id: BASE + 403,
            comparison: &cl,
            priority: Priority::Foreground,
        },
        Job {
            id: BASE + 402,
            comparison: &ct,
            priority: Priority::Foreground,
        },
    ];
    let receipt = b
        .attach(
            &p,
            1,
            &proof,
            link,
            Span { start: 0, end: 5 },
            &js,
            &mut tape,
        )
        .unwrap();
    let after_attach = counts(&b);
    let early = outcome(b.close_discovery(&p, 1, &h));
    let mut evaluations = vec![];
    for l in b.dispatch(&p, 1, 2).unwrap() {
        let c = if l.job() == BASE + 402 { &ct } else { &cl };
        let o = evaluate(l.job(), c);
        evaluations.push(json!({"lease":lease(&l),"score":score(&o.score),"evidence":evidence(&specimen_kernel::scanner_v3::scan(c.source,c.target).unwrap()),"contribution":o.contribution.as_ref().map(contribution)}));
        b.complete(&p, 1, &l, o.contribution).unwrap();
    }
    b.close_discovery(&p, 1, &h).unwrap();
    b.seal(&p, 1, &h).unwrap();
    let p1 = b
        .compose(
            &p,
            1,
            &h,
            &mut DrawTape::new(SEED, "attachment-compose", &[], 0).unwrap(),
            ScoringBudget::default(),
        )
        .unwrap();
    json!({"id":"hard-attachment-no-forced-vote","limits":limits(lim),"issuer_input":a.input(BASE+401),"target_inputs":[low.input(BASE+403),t.input(BASE+402)],"issuer":issuer.contribution.as_ref().map(contribution),"link":{"from":link.from.0.to_string(),"to":link.to.0.to_string(),"group":link.group.to_string(),"mode":"Hard","remainder":[0,5],"text":"2 + 3"},"receipt":{"disposition":format!("{:?}",receipt.disposition),"selected":receipt.trace.unwrap().selected,"consumed":tape.consumed()},"after_attach":after_attach,"early_close":early,"evaluations":evaluations,"final_counts":counts(&b),"plan":plan(&p1)})
}
fn headroom() -> Value {
    let p = scope();
    let lim = Limits {
        operation_work: 8,
        ..Limits::default()
    };
    let mut b = Broker::new(p.clone(), 1, lim).unwrap();
    let h = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.begin(&p, 1, 2, CycleLimit::default()).unwrap();
    let before_owned = b.counts().owned_bytes;
    let before = counts(&b);
    let refusal = outcome(b.begin(&p, 1, 3, CycleLimit::default()));
    let after = counts(&b);
    let after_owned = b.counts().owned_bytes;
    b.restart(&p, 1, 2).unwrap();
    let retained = outcome(b.begin(&p, 2, 3, CycleLimit::default()));
    b.retire_cycle(&p, 2, &h).unwrap();
    let retry = outcome(b.begin(&p, 2, 3, CycleLimit::default()));
    json!({"id":"cleanup-reservation","limits":limits(lim),"retained_cycles":2,"retained_jobs":0,"extra":0,"before":before,"before_owned":before_owned,"refusal":refusal,"after":after,"after_owned":after_owned,"post_restart_refusal":retained,"post_retire_retry":retry,"final_counts":counts(&b)})
}
pub fn run() -> Value {
    json!({"fairness":fairness(),"quarantine":quarantine(),"attachment":attachment(),"headroom":headroom()})
}
