#![allow(dead_code)]
use specimen_kernel::{
    identity::*, language::Expr, language_v3::*, orchestration_v3::*, scanner_v3::routes::*,
    scoring_v3::*, text_v3,
};
pub struct Fixture {
    pub source: Prepared,
    pub target: Prepared,
    pub record: EntryRecord,
    pub origin: CandidateOrigin,
    pub binding: TargetBinding,
}
impl Fixture {
    pub fn new(source: &str, target: &str, action: &str, entry: u128) -> Self {
        let prepare_one = |text: &str| {
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
                    text: text.into(),
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
        let source = prepare_one(source);
        let target = prepare_one(target);
        let s = &source.clauses()[0].variants()[0];
        let t = &target.clauses()[0].variants()[0];
        let record = EntryRecord {
            population: Population::new("tenant", "specimen", "text"),
            node: NodeRef(entry),
            reference: EntryRef(entry),
            key: text_v3::encode(t).unwrap().pattern_key().clone(),
            original: t.original().into(),
            ast: ActionAst {
                bind: vec![],
                alternatives: vec![Alternative {
                    action: vec![Expr::Text {
                        value: action.into(),
                    }],
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
    pub fn request(&self, id: u128) -> EntryRequest<'_> {
        EntryRequest {
            id,
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
                ..Policy::default()
            },
        }
    }
    pub fn comparison(&self) -> Comparison<'_> {
        Comparison {
            record: &self.record,
            source: &self.source.clauses()[0].variants()[0],
            target: &self.target.clauses()[0].variants()[0],
            origin: &self.origin,
            parent_lineage: "parent",
            dependencies: &[],
            attachment_group: None,
            kind: ContributionKind::Action,
        }
    }
    pub fn vote(&self, id: u128) -> Option<Contribution> {
        evaluate_entry(
            self.request(id),
            &mut DrawTape::new(0, "test", &[], 0).unwrap(),
            ScoringBudget::default(),
        )
        .unwrap()
        .contribution
    }
}
pub fn scope() -> Population {
    Population::new("tenant", "specimen", "text")
}
pub fn job<'a>(id: u128, comparison: &'a Comparison<'a>, priority: Priority) -> Job<'a> {
    Job {
        id,
        comparison,
        priority,
    }
}
pub fn evaluate(id: u128, c: &Comparison<'_>) -> Option<Contribution> {
    evaluate_policy(
        id,
        c,
        Policy {
            jitter: false,
            ..Policy::default()
        },
    )
}
pub fn evaluate_policy(id: u128, c: &Comparison<'_>, policy: Policy) -> Option<Contribution> {
    let binding = TargetBinding {
        node: c.record.node,
        entry: c.record.reference,
        origin: c.target.origin().clone(),
    };
    evaluate_entry(
        EntryRequest {
            id,
            source: c.source,
            target: c.target,
            record: c.record,
            origin: c.origin,
            target_binding: &binding,
            parent_lineage: c.parent_lineage,
            inhibitions: &[],
            dependencies: c.dependencies,
            attachment_group: c.attachment_group,
            kind: c.kind,
            policy,
        },
        &mut DrawTape::new(0, "test", &[], 0).unwrap(),
        ScoringBudget::default(),
    )
    .unwrap()
    .contribution
}
pub fn issuer<'a>(c: &'a Comparison<'a>) -> (Broker<'a>, CompletedIssuer) {
    issuer_policy(
        c,
        Policy {
            jitter: false,
            ..Policy::default()
        },
    )
}
pub fn issuer_policy<'a>(c: &'a Comparison<'a>, policy: Policy) -> (Broker<'a>, CompletedIssuer) {
    let p = scope();
    let mut b = Broker::new(p.clone(), 1, Limits::default()).unwrap();
    let h1 = b.begin(&p, 1, 1, CycleLimit::default()).unwrap();
    b.admit(&p, 1, &h1, &[job(1, c, Priority::Foreground)])
        .unwrap();
    let l = b.dispatch(&p, 1, 1).unwrap().remove(0);
    let value = evaluate_policy(1, c, policy);
    assert!(
        value.is_some(),
        "issuer constraints {:?}",
        c.origin.constraints
    );
    let proof = b.complete(&p, 1, &l, value).unwrap().unwrap();
    (b, proof)
}
pub fn attached<'a>(f: &'a Fixture, deps: &'a [u128], group: u128) -> Comparison<'a> {
    let mut c = f.comparison();
    c.dependencies = deps;
    c.attachment_group = Some(group);
    c
}
