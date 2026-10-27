//! Fixed same-limit public decision witnesses for the reviewed logical-charge-v1.
mod attachment;
use super::project::*;
use serde_json::{Value, json};
use specimen_kernel::{
    identity::*, language::Expr, language_v3::*, orchestration_v3::*, scanner_v3::routes::*,
    scoring_v3::*, text_v3,
};

struct Fixture {
    source: Prepared,
    target: Prepared,
    record: EntryRecord,
    origin: CandidateOrigin,
    binding: TargetBinding,
}
impl Fixture {
    fn new() -> Self {
        let one = || {
            let tables = UtilityTables::new(
                "p",
                "v",
                UtilityKind::ALL
                    .into_iter()
                    .map(|kind| UtilitySnapshot {
                        population: "p".into(),
                        version: "v".into(),
                        kind,
                        rows: vec![],
                    })
                    .collect(),
            )
            .unwrap();
            prepare(
                Input {
                    id: "input".into(),
                    text: "hello".into(),
                },
                &ContextSnapshot {
                    population: "p".into(),
                    utility_version: "v".into(),
                    tokens: vec![],
                    conditions: vec![],
                },
                &tables,
                Budget::default(),
                &mut FixedDraw(0),
            )
            .unwrap()
        };
        let source = one();
        let target = one();
        let s = &source.clauses()[0].variants()[0];
        let t = &target.clauses()[0].variants()[0];
        let record = EntryRecord {
            population: Population::new("tenant", "specimen", "text"),
            node: NodeRef(1),
            reference: EntryRef(1),
            key: text_v3::encode(t).unwrap().pattern_key().clone(),
            original: t.original().into(),
            ast: ActionAst {
                bind: vec![],
                alternatives: vec![Alternative {
                    action: vec![Expr::Text { value: "é".into() }],
                    inhibited: false,
                    weight: 1.0,
                }],
            },
            resource: None,
        };
        let origin = CandidateOrigin {
            origin: s.origin().clone(),
            span: s.span(),
            token_spans: s.tokens().iter().map(Token::span).collect(),
            constraints: s.into(),
            path: CandidatePath::Wildcard("test".into()),
            captures: vec![],
            bypass: SimilarityBypass::RequireFullScan,
        };
        let binding = TargetBinding {
            node: record.node,
            entry: record.reference,
            origin: t.origin().clone(),
        };
        Self {
            source,
            target,
            record,
            origin,
            binding,
        }
    }
    fn request(&self) -> EntryRequest<'_> {
        EntryRequest {
            id: 1,
            source: &self.source.clauses()[0].variants()[0],
            target: &self.target.clauses()[0].variants()[0],
            record: &self.record,
            origin: &self.origin,
            target_binding: &self.binding,
            parent_lineage: "parent",
            inhibitions: &[],
            dependencies: &[],
            attachment_group: None,
            kind: ContributionKind::Action,
            policy: Policy {
                jitter: false,
                threshold: -100,
                ..Policy::default()
            },
        }
    }
    fn comparison(&self) -> Comparison<'_> {
        let r = self.request();
        Comparison {
            record: r.record,
            source: r.source,
            target: r.target,
            origin: r.origin,
            parent_lineage: r.parent_lineage,
            dependencies: r.dependencies,
            attachment_group: r.attachment_group,
            kind: r.kind,
        }
    }
    fn vote(&self) -> Contribution {
        evaluate_entry(
            self.request(),
            &mut DrawTape::new(0, "test", &[], 0).unwrap(),
            ScoringBudget::default(),
        )
        .unwrap()
        .contribution
        .unwrap()
    }
    fn input(&self) -> Value {
        let r = self.request();
        let action = |a: &PreparedAction| {
            json!({
                "prepared":prepared(a),
                "tokens":a.tokens().iter().map(|t|json!({"source":t.source(),"lemma":t.lemma(),"rule_refs":t.rule_refs()})).collect::<Vec<_>>(),
                "semantic_tokens":match a.ast(){SemanticAst::Action(ts)=>ts.len(),_=>panic!("authored fixture must be one Action")},
                "captures":a.captures().len(),"operators":a.operators(),"definitions":a.definitions().len(),"charges":a.charges().len()
            })
        };
        json!({"source":action(r.source),"target":action(r.target),"population":{"tenant":self.record.population.tenant,"specimen":self.record.population.specimen,"node_type":self.record.population.node_type},"node":self.record.node.0.to_string(),"entry":self.record.reference.0.to_string(),"id":r.id.to_string(),"key":{"encoder_version":self.record.key.encoder_version,"modality":self.record.key.modality,"resolution":self.record.key.resolution},"original":self.record.original,"alternatives":self.record.ast.alternatives.iter().map(|a|json!({"action":a.action,"inhibited":a.inhibited,"weight_bits":format!("{:016x}",a.weight.to_bits())})).collect::<Vec<_>>(),"bind":self.record.ast.bind,"origin":super::project::origin(&self.origin.origin),"token_spans":self.origin.token_spans.iter().copied().map(span).collect::<Vec<_>>(),"route":{"kind":"Wildcard","name":"test"},"parent":"parent","captures":captures(&self.origin.captures),"inhibitions":[],"dependencies":[],"group":Value::Null,"kind":"Action","resource":Value::Null,"binding":{"node":self.binding.node.0.to_string(),"entry":self.binding.entry.0.to_string(),"origin":super::project::origin(&self.binding.origin)},"policy":policy(r.policy),"random":{"seed":"0","stream":"test","draws":[],"limit":0}})
    }
}
fn scoring(b: ScoringBudget) -> Value {
    json!({"contributions":b.contributions,"work":b.work,"payload_bytes":b.payload_bytes,"draws":b.draws})
}
fn limits(l: Limits) -> Value {
    json!({"active":l.active,"pending":l.pending,"owned_bytes":l.owned_bytes,"operation_work":l.operation_work,"trace":l.trace})
}
fn counts(b: &Broker<'_>) -> Value {
    let c = b.counts();
    json!({"pending":c.pending,"active":c.active,"quarantined":c.quarantined,"owned_bytes":c.owned_bytes})
}
fn status(b: &Broker<'_>, p: &Population, h: &CycleHandle) -> String {
    format!("{:?}", b.status(p, 1, h).unwrap())
}
fn scoring_rows(f: &Fixture) -> Vec<Value> {
    let mut rows = vec![];
    for (operation, kind, exact) in [
        ("single-payload", "payload_bytes", 2971),
        ("single-work", "work", 2158),
        ("batch-payload", "payload_bytes", 3355),
        ("arbitration-payload", "payload_bytes", 2792),
    ] {
        for limit in [exact - 1, exact] {
            let mut budget = ScoringBudget::default();
            if kind == "work" {
                budget.work = limit;
            } else {
                budget.payload_bytes = limit;
            }
            let mut tape = DrawTape::new(0, "test", &[], 0).unwrap();
            let before = tape.consumed();
            let result =
                match operation {
                    "batch-payload" => evaluate_entries(&[f.request()], &mut tape, budget)
                        .map(|mut os| os.remove(0).contribution.unwrap()),
                    "arbitration-payload" => arbitrate(&[f.vote()], &mut tape, budget)
                        .map(|mut a| a.contributions.remove(0)),
                    _ => evaluate_entry(f.request(), &mut tape, budget)
                        .map(|o| o.contribution.unwrap()),
                };
            let output = match result {
                Ok(c) => json!({"error":Value::Null,"contribution":contribution(&c)}),
                Err(e) => json!({"error":format!("{e:?}"),"contribution":Value::Null}),
            };
            rows.push(json!({"operation":operation,"limit_kind":kind,"limit":limit,"scoring_budget":scoring(budget),"draws_before":before,"draws_after":tape.consumed(),"output":output}));
        }
    }
    rows
}
fn broker_rows(f: &Fixture) -> Vec<Value> {
    let mut rows = vec![];
    for (operation, kind, exact) in [
        ("descriptor-work", "operation_work", 466),
        ("completion-owned", "owned_bytes", 2986),
        ("completion-work", "operation_work", 2304),
    ] {
        for limit in [exact - 1, exact] {
            let mut lim = Limits::default();
            if kind == "owned_bytes" {
                lim.owned_bytes = limit;
            } else {
                lim.operation_work = limit;
            }
            let p = f.record.population.clone();
            let comparison = f.comparison();
            let mut b = Broker::new(p.clone(), 1, lim).unwrap();
            let h = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
            let job = Job {
                id: 1,
                comparison: &comparison,
                priority: Priority::Foreground,
            };
            if operation != "descriptor-work" {
                b.admit(&p, 1, &h, &[job]).unwrap();
            }
            let lease = if operation == "descriptor-work" {
                None
            } else {
                Some(b.dispatch(&p, 1, 1).unwrap().remove(0))
            };
            let before = counts(&b);
            let status_before = status(&b, &p, &h);
            let result = if operation == "descriptor-work" {
                b.admit(&p, 1, &h, &[job]).map(|_| false)
            } else {
                b.complete(&p, 1, lease.as_ref().unwrap(), Some(f.vote()))
                    .map(|v| v.is_some())
            };
            let (error, proof_returned) = match result {
                Ok(proof) => (None, proof),
                Err(e) => (Some(format!("{e:?}")), false),
            };
            let after = counts(&b);
            let status_after = status(&b, &p, &h);
            let mut fallback = Value::Null;
            let mut duplicate = Value::Null;
            let mut extra_cycle = Value::Null;
            if operation == "descriptor-work" {
                let dispatched = b.dispatch(&p, 1, 1).unwrap();
                if let Some(l) = dispatched.first() {
                    b.complete(&p, 1, l, None).unwrap();
                }
                fallback = json!({"dispatched":dispatched.iter().map(|l|json!({"cycle":l.cycle().to_string(),"job":l.job().to_string(),"generation":l.generation().to_string()})).collect::<Vec<_>>(),"counts":counts(&b)});
            } else {
                let l = lease.as_ref().unwrap();
                if error.is_some() {
                    let r = b.complete(&p, 1, l, None);
                    fallback = json!({"result":if r.is_ok(){"Ok"}else{"Error"},"proof_returned":r.unwrap().is_some(),"counts":counts(&b)});
                }
                duplicate = json!({"error":format!("{:?}",b.complete(&p,1,l,None).unwrap_err()),"counts":counts(&b)});
                if operation == "completion-work" && error.is_none() {
                    let prior = counts(&b);
                    let e = b.begin(&p, 1, 2, CycleLimit::default()).unwrap_err();
                    extra_cycle =
                        json!({"error":format!("{e:?}"),"before":prior,"after":counts(&b)});
                }
            }
            b.restart(&p, 1, 2).unwrap();
            b.retire_cycle(&p, 2, &h).unwrap();
            rows.push(json!({"operation":operation,"limit_kind":kind,"limit":limit,"broker_limits":limits(lim),"generation":"1","cycle":"1","job":"1","before":before,"after":after,"status_before":status_before,"status_after":status_after,"output":{"error":error,"proof_returned":proof_returned},"fallback":fallback,"duplicate":duplicate,"extra_cycle":extra_cycle,"after_restart_retire":counts(&b)}));
        }
    }
    rows
}
pub fn run() -> Value {
    let f = Fixture::new();
    let mut rows = scoring_rows(&f);
    rows.extend(broker_rows(&f));
    let (attachment_fixture, attachment_rows) = attachment::run();
    rows.extend(attachment_rows);
    json!({"profile":"logical-charge-v1","fixture":f.input(),"attachment_fixture":attachment_fixture,"rows":rows})
}
