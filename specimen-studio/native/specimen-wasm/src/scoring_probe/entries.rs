use super::{numeric::SEED, project::*};
use serde_json::{Value, json};
use specimen_kernel::{
    identity::*,
    language::Expr,
    language_v3::*,
    scanner_v3::{self, routes::*},
    scoring_v3::*,
    text_v3,
};
pub const BASE: u128 = 1267650600228229401496703205376;
pub fn prepared_text(text: &str, fanout: bool) -> Prepared {
    let mut snapshots = UtilityKind::ALL
        .into_iter()
        .map(|kind| UtilitySnapshot {
            population: "parity".into(),
            version: "v1".into(),
            kind,
            rows: vec![],
        })
        .collect::<Vec<_>>();
    if fanout {
        snapshots
            .iter_mut()
            .find(|s| s.kind == UtilityKind::Thesaurus)
            .unwrap()
            .rows
            .push(UtilityRow {
                reference: "authored-fanout".into(),
                phrase: "bright".into(),
                effect: UtilityEffect::Equivalent {
                    replacement: "radiant".into(),
                    weight: 1.0,
                },
            });
    }
    prepare(
        Input {
            id: "authored-input".into(),
            text: text.into(),
        },
        &ContextSnapshot {
            population: "parity".into(),
            utility_version: "v1".into(),
            tokens: vec![],
            conditions: vec![],
        },
        &UtilityTables::new("parity", "v1", snapshots).unwrap(),
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap()
}
pub struct Fixture {
    pub source: Prepared,
    pub target: Prepared,
    pub variant: usize,
    pub record: EntryRecord,
    pub origin: CandidateOrigin,
    pub binding: TargetBinding,
    pub parent: String,
    pub facts: Vec<InhibitionFact>,
    pub deps: Vec<u128>,
    pub group: Option<u128>,
    pub kind: ContributionKind,
    pub policy: Policy,
}
impl Fixture {
    pub fn new(source: &str, target: &str, action: &str, node: u128, entry: u128) -> Self {
        let source = prepared_text(source, false);
        let target = prepared_text(target, false);
        let s = &source.clauses()[0].variants()[0];
        let t = &target.clauses()[0].variants()[0];
        let record = EntryRecord {
            population: scope(),
            node: NodeRef(node),
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
        let origin = Self::origin(s);
        let binding = TargetBinding {
            node: record.node,
            entry: record.reference,
            origin: t.origin().clone(),
        };
        Self {
            source,
            target,
            variant: 0,
            record,
            origin,
            binding,
            parent: "parent".into(),
            facts: vec![],
            deps: vec![],
            group: None,
            kind: ContributionKind::Action,
            policy: Policy {
                jitter: false,
                ..Policy::default()
            },
        }
    }
    fn origin(s: &PreparedAction) -> CandidateOrigin {
        CandidateOrigin {
            origin: s.origin().clone(),
            span: s.span(),
            token_spans: s.tokens().iter().map(Token::span).collect(),
            constraints: s.into(),
            path: CandidatePath::Wildcard("authored-route".into()),
            captures: vec![],
            bypass: SimilarityBypass::RequireFullScan,
        }
    }
    pub fn s(&self) -> &PreparedAction {
        &self.source.clauses()[0].variants()[self.variant]
    }
    pub fn t(&self) -> &PreparedAction {
        &self.target.clauses()[0].variants()[0]
    }
    pub fn request(&self, id: u128) -> EntryRequest<'_> {
        EntryRequest {
            id,
            source: self.s(),
            target: self.t(),
            record: &self.record,
            origin: &self.origin,
            target_binding: &self.binding,
            parent_lineage: &self.parent,
            inhibitions: &self.facts,
            dependencies: &self.deps,
            attachment_group: self.group,
            kind: self.kind,
            policy: self.policy,
        }
    }
    pub fn input(&self, id: u128) -> Value {
        json!({"id":id.to_string(),"source":prepared(self.s()),"target":prepared(self.t()),"population":{"tenant":self.record.population.tenant,"specimen":self.record.population.specimen,"node_type":self.record.population.node_type},"node":self.record.node.0.to_string(),"entry":self.record.reference.0.to_string(),"record_original":self.record.original,"record_key":format!("{:?}",self.record.key),"binding":{"node":self.binding.node.0.to_string(),"entry":self.binding.entry.0.to_string(),"origin":origin(&self.binding.origin)},"route":format!("{:?}",self.origin.path),"bypass":format!("{:?}",self.origin.bypass),"origin":origin(&self.origin.origin),"span":span(self.origin.span),"token_spans":self.origin.token_spans.iter().copied().map(span).collect::<Vec<_>>(),"guards":constraints(&self.origin.constraints),"captures":captures(&self.origin.captures),"parent":self.parent,"inhibitions":self.facts.iter().map(|f|json!({"domain":format!("{:?}",f.domain),"code":f.code})).collect::<Vec<_>>(),"dependencies":self.deps.iter().map(ToString::to_string).collect::<Vec<_>>(),"group":self.group.map(|g|g.to_string()),"kind":format!("{:?}",self.kind),"policy":policy(self.policy),"alternatives":self.record.ast.alternatives.iter().map(|a|json!({"action":a.action,"inhibited":a.inhibited,"weight_bits":format!("{:016x}",a.weight.to_bits())})).collect::<Vec<_>>()})
    }
    pub fn comparison(&self) -> specimen_kernel::orchestration_v3::Comparison<'_> {
        specimen_kernel::orchestration_v3::Comparison {
            record: &self.record,
            source: self.s(),
            target: self.t(),
            origin: &self.origin,
            parent_lineage: &self.parent,
            dependencies: &self.deps,
            attachment_group: self.group,
            kind: self.kind,
        }
    }
}
pub fn scope() -> Population {
    Population::new("parity-tenant", "parity-specimen", "text")
}
pub fn evaluate(id: u128, c: &specimen_kernel::orchestration_v3::Comparison<'_>) -> EntryOutcome {
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
            policy: Policy {
                jitter: false,
                ..Policy::default()
            },
        },
        &mut DrawTape::new(SEED, "broker-score", &[], 0).unwrap(),
        ScoringBudget::default(),
    )
    .unwrap()
}
pub fn run() -> Value {
    let mut fixtures = (0..12)
        .map(|i| {
            Fixture::new(
                "calculate bright 2 + 3",
                "calculate bright 2 + 3",
                if i == 2 { "complement" } else { "same" },
                BASE + 1,
                BASE + 10 + i,
            )
        })
        .collect::<Vec<_>>();
    // Same node, distinct concrete entries; two actual prepared fanout siblings.
    for (i, f) in fixtures.iter_mut().take(2).enumerate() {
        f.source = prepared_text("calculate bright 2 + 3", true);
        f.variant = i;
        f.origin = Fixture::origin(f.s());
        f.origin.bypass = SimilarityBypass::Declared;
        f.origin.captures = vec![RouteCapture {
            name: "literal".into(),
            span: Span {
                start: i,
                end: i + 1,
            },
            value: CaptureValue::Text("same".into()),
        }];
    }
    fixtures[3].parent = "different-parent".into();
    fixtures[4].facts.push(InhibitionFact {
        domain: InhibitionDomain::Input,
        code: "declared".into(),
    });
    fixtures[5].facts.push(InhibitionFact {
        domain: InhibitionDomain::Target,
        code: "declared".into(),
    });
    fixtures[6].origin.captures.push(RouteCapture {
        name: "value".into(),
        span: Span { start: 0, end: 1 },
        value: CaptureValue::Number(CanonicalDecimal::parse("2.50").unwrap()),
    });
    fixtures[7].origin.captures.push(RouteCapture {
        name: "value".into(),
        span: Span { start: 0, end: 1 },
        value: CaptureValue::Bytes(vec![50, 46, 53]),
    });
    fixtures[8].group = Some(BASE + 99);
    fixtures[9].kind = ContributionKind::IntermediateFact;
    fixtures[10].deps = vec![BASE + 112]; // exempts last otherwise-redundant parent
    fixtures[11].record.ast.alternatives[0].action = vec![Expr::Call {
        name: "same".into(),
        args: vec![vec![Expr::Text {
            value: "nested".into(),
        }]],
    }];
    let requests = fixtures
        .iter()
        .enumerate()
        .map(|(i, f)| f.request(BASE + 101 + i as u128))
        .collect::<Vec<_>>();
    let mut tape = DrawTape::new(SEED, "entry-batch", &[], 0).unwrap();
    let outcomes = evaluate_entries(&requests, &mut tape, ScoringBudget::default()).unwrap();
    let batch_consumed = tape.consumed();
    let observed=outcomes.iter().enumerate().map(|(i,o)|json!({"input":fixtures[i].input(BASE+101+i as u128),"evidence":evidence(&scanner_v3::scan(fixtures[i].s(),fixtures[i].t()).unwrap()),"score":score(&o.score),"election":o.election.as_ref().map(election),"contribution":o.contribution.as_ref().map(contribution)})).collect::<Vec<_>>();
    let values = outcomes
        .into_iter()
        .map(|o| o.contribution.unwrap())
        .collect::<Vec<_>>();
    let ds = [u32::MAX, 0, 0, 0];
    let mut tape = DrawTape::new(SEED, "redundancy", &ds, 4).unwrap();
    let arbitration = arbitrate(&values, &mut tape, ScoringBudget::default()).unwrap();
    let mut refusals = vec![];
    for (name, source, bypass, blocked, stale) in [
        ("low-no-bypass", "unrelated words", false, false, false),
        ("low-bypass", "unrelated words", true, false, false),
        ("guard-bypass", "if missing then 2 + 3", true, false, false),
        ("blocked-automaton", "2 + 3", false, true, false),
        ("stale-binding", "2 + 3", false, false, true),
    ] {
        let mut f = Fixture::new(source, "2 + 3", "refusal", BASE + 80, BASE + 81);
        if bypass {
            f.origin.bypass = SimilarityBypass::Declared;
        }
        if blocked {
            f.origin.path = CandidatePath::Automaton {
                rule: "blocked".into(),
                status: AutomatonStatus::Blocked,
            };
        }
        if stale {
            f.binding.entry = EntryRef(BASE + 999);
        }
        let mut t = DrawTape::new(SEED, "refusal", &[], 0).unwrap();
        let result = evaluate_entry(f.request(BASE + 90), &mut t, ScoringBudget::default());
        let output = match result {
            Ok(o) => {
                json!({"score":score(&o.score),"contribution":o.contribution.as_ref().map(contribution),"error":Value::Null})
            }
            Err(e) => {
                json!({"score":Value::Null,"contribution":Value::Null,"error":format!("{e:?}")})
            }
        };
        refusals.push(json!({"id":name,"input":f.input(BASE+90),"evidence":evidence(&scanner_v3::scan(f.s(),f.t()).unwrap()),"output":output,"after":t.consumed(),"context_owner":"probe-authored-input; error-discriminant-from-kernel"}));
    }
    json!({"seed":SEED.to_string(),"batch_stream":"entry-batch","batch_draws":[],"batch_consumed":batch_consumed,"evaluated":observed,"arbitration":{"stream":"redundancy","draws":ds,"consumed":tape.consumed(),"groups":arbitration.groups.iter().map(super::project::arbitration).collect::<Vec<_>>(),"survivors":arbitration.contributions.iter().map(contribution).collect::<Vec<_>>()},"refusals":refusals})
}
