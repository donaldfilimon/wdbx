//! Synthetic, pure language/text/scanner integration witness; no runtime cutover.
use serde_json::{Value, json};
use specimen_kernel::{
    identity::*,
    language_v3::{script::*, *},
    scanner_v3::{self, routes::*, *},
    text_v3,
};

fn tables(rich: bool) -> UtilityTables {
    let mut snapshots = UtilityKind::ALL
        .into_iter()
        .map(|kind| UtilitySnapshot {
            population: "parity".into(),
            version: "v1".into(),
            kind,
            rows: vec![],
        })
        .collect::<Vec<_>>();
    let mut add = |kind, reference: &str, phrase: &str, effect| {
        snapshots
            .iter_mut()
            .find(|s| s.kind == kind)
            .unwrap()
            .rows
            .push(UtilityRow {
                reference: reference.into(),
                phrase: phrase.into(),
                effect,
            });
    };
    if rich {
        add(
            UtilityKind::Dictionary,
            "dictionary",
            "bright",
            UtilityEffect::Dictionary {
                lemma: "bright".into(),
                definition: "luminous".into(),
            },
        );
        add(
            UtilityKind::Thesaurus,
            "equivalent-a",
            "bright",
            UtilityEffect::Equivalent {
                replacement: "radiant".into(),
                weight: 1.0,
            },
        );
        add(
            UtilityKind::Thesaurus,
            "equivalent-b",
            "bright",
            UtilityEffect::Equivalent {
                replacement: "shining".into(),
                weight: 3.0,
            },
        );
        add(
            UtilityKind::AntiThesaurus,
            "sense",
            "bank",
            UtilityEffect::Sense {
                lemma: "riverbank".into(),
                required_context: vec!["river".into()],
                weight: 1.0,
            },
        );
        add(
            UtilityKind::Chargebook,
            "charge",
            "bright",
            UtilityEffect::Charge {
                valence: 0.5,
                intensity: 0.75,
            },
        );
        add(
            UtilityKind::VerbFrames,
            "frame",
            "give",
            UtilityEffect::Frame {
                subject: true,
                object: true,
                numeric_object: false,
            },
        );
        add(
            UtilityKind::Conjunction,
            "conjunction",
            "also",
            UtilityEffect::Conjunction(Connective::And),
        );
        add(
            UtilityKind::Negation,
            "negation",
            "never",
            UtilityEffect::Negation { uncertain: false },
        );
        add(
            UtilityKind::Morphology,
            "morphology",
            "s",
            UtilityEffect::Suffix {
                replacement: "".into(),
            },
        );
        for (reference, phrase, role) in [
            ("grammar", "give", Role::Predicate),
            ("subject", "alice", Role::Subject),
            ("predicate", "see", Role::Predicate),
            ("object", "bird", Role::Object),
        ] {
            add(
                UtilityKind::Grammar,
                reference,
                phrase,
                UtilityEffect::Role(role),
            );
        }
    }
    UtilityTables::new("parity", "v1", snapshots).unwrap()
}
fn context() -> ContextSnapshot {
    ContextSnapshot {
        population: "parity".into(),
        utility_version: "v1".into(),
        tokens: vec!["river".into()],
        conditions: vec![],
    }
}
fn prepare_with(
    text: &str,
    rich: bool,
    draw: u64,
    budget: Budget,
) -> Result<Prepared, LanguageError> {
    prepare(
        Input {
            id: "synthetic-input".into(),
            text: text.into(),
        },
        &context(),
        &tables(rich),
        budget,
        &mut FixedDraw(draw),
    )
}
fn prep(text: &str) -> Prepared {
    prepare_with(text, false, 0, Budget::default()).unwrap()
}
fn rich(text: &str) -> Prepared {
    prepare_with(text, true, 0, Budget::default()).unwrap()
}
fn first(p: &Prepared) -> &PreparedAction {
    &p.clauses()[0].variants()[0]
}
fn last(p: &Prepared) -> &PreparedAction {
    &p.clauses().last().unwrap().variants()[0]
}
fn span(s: Span) -> Value {
    json!([s.start, s.end])
}
fn decimal(n: &CanonicalDecimal) -> Value {
    json!({"coefficient":n.coefficient().to_string(),"scale":n.scale()})
}
fn token(t: &Token) -> Value {
    json!({"source":t.source(),"lemma":t.lemma(),"span":span(t.span()),"kind":format!("{:?}",t.kind()),"role":format!("{:?}",t.role()),"quoted":t.quoted(),"decimal":t.decimal().map(decimal),"rules":t.rule_refs()})
}
fn ast(tree: &SemanticAst) -> Value {
    match tree {
        SemanticAst::Action(ts) => {
            json!({"tag":"action","tokens":ts.iter().map(token).collect::<Vec<_>>()})
        }
        SemanticAst::And(a, b) => binary("and", a, b),
        SemanticAst::Or(a, b) => binary("or", a, b),
        SemanticAst::But(a, b) => binary("but", a, b),
        SemanticAst::IfThen(a, b) => binary("if-then", a, b),
        SemanticAst::NegatedAction { uncertain, action } => {
            json!({"tag":"negated","uncertain":uncertain,"child":ast(action)})
        }
    }
}
fn binary(tag: &str, a: &SemanticAst, b: &SemanticAst) -> Value {
    json!({"tag":tag,"children":[ast(a),ast(b)]})
}
fn slot(s: &Slot) -> Value {
    match s {
        Slot::Resolved(s) => json!(s),
        Slot::Unresolved => Value::Null,
    }
}
fn origin(o: &Origin) -> Value {
    json!({"input":o.input_id(),"clause":o.clause_id(),"action":o.action_id(),"fanout":o.fanout_id(),"rules":o.rule_refs()})
}
fn constraints(c: &Constraints) -> Value {
    json!({"inhibited":c.inhibited,"uncertain":c.uncertain_negation,"unresolved_frame":c.unresolved_frame,"condition":format!("{:?}",c.conditional),"executable":c.executable})
}
fn action(a: &PreparedAction) -> Value {
    json!({"original":a.original(),"span":span(a.span()),"tokens":a.tokens().iter().map(token).collect::<Vec<_>>(),"ast":ast(a.ast()),
        "captures":a.captures().iter().map(|c|json!({"source":c.source(),"span":span(c.span()),"decimal":decimal(c.decimal())})).collect::<Vec<_>>(),
        "operators":a.operators(),"value":a.arithmetic_value(),"spo":[slot(a.spo().subject()),slot(a.spo().predicate()),slot(a.spo().object())],
        "constraints":constraints(&a.into()),"origin":origin(a.origin()),"routes":a.routes().iter().map(|r|format!("{r:?}")).collect::<Vec<_>>(),
        "definitions":a.definitions().iter().map(|d|json!({"rule":d.rule_ref(),"definition":d.definition(),"span":span(d.span())})).collect::<Vec<_>>(),
        "charges":a.charges().iter().map(|c|json!({"rule":c.rule_ref(),"valence":c.valence(),"intensity":c.intensity(),"span":span(c.span())})).collect::<Vec<_>>()})
}
fn prepared(p: &Prepared) -> Value {
    json!({"original":p.original(),"ast":ast(p.ast()),"draws":p.draws_used(),"actions":p.clauses().iter().flat_map(|c|c.variants().iter().map(action)).collect::<Vec<_>>()})
}
fn fraction(r: Rational) -> Value {
    json!([r.numerator().to_string(), r.denominator().to_string()])
}
fn totals(t: Totals) -> Value {
    json!({"similarity":fraction(t.similarity),"dissimilarity":fraction(t.dissimilarity),"difference":fraction(t.difference),"confidence":fraction(t.confidence)})
}
fn evidence(a: &PreparedAction, b: &PreparedAction) -> Value {
    let e = scanner_v3::scan(a, b).unwrap();
    json!({"version":e.version,"left":action(a),"right":action(b),"components":e.components.iter().map(|c|json!({"name":c.name,"weight":c.weight.to_string(),"s":fraction(c.similarity),"d":fraction(c.dissimilarity),"reason":format!("{:?}",c.reason),"left_spans":c.left_spans.iter().copied().map(span).collect::<Vec<_>>(),"right_spans":c.right_spans.iter().copied().map(span).collect::<Vec<_>>()})).collect::<Vec<_>>(),"totals":totals(e.totals)})
}
fn key(a: &PreparedAction) -> Value {
    let k = text_v3::encode(a).unwrap();
    json!({"action":action(a),"class":k.resolution().as_str(),"count":k.token_count(),"transform":k.transform(),"preimage":k.preimage(),"digest":k.pattern_key().digest,"encoder":k.pattern_key().encoder_version,"modality":k.pattern_key().modality})
}
fn registry(revision: u64) -> Registry {
    let scopes = vec![Scope::NodeAction];
    let declarations = [
        ("Text", Handler::Text, vec![]),
        ("Math", Handler::Math, vec![]),
        ("Tape", Handler::Tape, vec![]),
        (
            "LookUp",
            Handler::LookUp,
            vec!["resource".into(), "memory".into()],
        ),
        ("Parent", Handler::Concat, vec!["resource".into()]),
        ("Bind", Handler::Bind, vec![]),
    ]
    .into_iter()
    .map(|(name, handler, reads)| Declaration {
        name: name.into(),
        aliases: vec![],
        scopes: scopes.clone(),
        reads,
        handler,
    })
    .collect();
    Registry::new(
        "parity-registry",
        revision,
        vec!["resource".into(), "memory".into()],
        declarations,
    )
    .unwrap()
}
fn script_plan(source: &str) -> Result<EffectPlan, LanguageError> {
    let r = registry(1);
    execute_script(
        &compile(source, Scope::NodeAction, &r)?,
        &r,
        &r.capabilities(vec!["resource".into(), "memory".into()])
            .unwrap(),
        ScriptBudget::default(),
    )
}
fn scripts() -> Value {
    let r = registry(1);
    let compiled = compile(
        "Parent(LookUp(\"resource\",\"key\"))",
        Scope::NodeAction,
        &r,
    )
    .unwrap();
    let caps = r
        .capabilities(vec!["resource".into(), "memory".into()])
        .unwrap();
    let newer = registry(2);
    let bind = script_plan("Bind(\"2+2\")").unwrap();
    json!({"nested_denial":format!("{:?}",script_plan("Parent(LookUp(\"memory\",\"key\"))").unwrap_err()),
        "stale":format!("{:?}",execute_script(&compiled,&newer,&caps,ScriptBudget::default()).unwrap_err()),
        "runaway":format!("{:?}",script_plan("Tape(\"+[]\",\"\")").unwrap_err()),
        "tape":format!("{:?}",script_plan("Tape(\",.,.\",\"é\")").unwrap().value()),
        "inert_effects":script_plan("Text(\"LookUp(unknown,key)\")").unwrap().effects().len(),
        "bind_value":format!("{:?}",bind.value()),"bind_trace":bind.trace().iter().map(|x|format!("{x:?}")).collect::<Vec<_>>(),
        "allowed_effects":format!("{:?}",execute_script(&compiled,&r,&caps,ScriptBudget::default()).unwrap().effects())})
}
fn add(table: &mut HybridTable, pop: &Population, id: u128, text: &str) {
    table
        .insert(
            EntryRecord {
                population: pop.clone(),
                node: NodeRef(100 + id),
                reference: EntryRef(id),
                key: text_v3::encode(first(&prep(text)))
                    .unwrap()
                    .pattern_key()
                    .clone(),
                original: text.into(),
                ast: ActionAst::default(),
                resource: None,
            },
            None,
        )
        .unwrap();
}
fn candidate(c: &Candidate) -> Value {
    json!({"entry":c.entry.0.to_string(),"original":c.original,"origins":c.origins.iter().map(|o|{
        let path=match &o.path {
            CandidatePath::Structural(_) => json!({"kind":"structural"}),
            CandidatePath::Grammar(id) => json!({"kind":"grammar","rule":id}),
            CandidatePath::Wildcard(id) => json!({"kind":"wildcard","rule":id}),
            CandidatePath::Automaton {rule,status} => json!({"kind":"automaton","rule":rule,"status":format!("{status:?}")}),
        };
        json!({"origin":origin(&o.origin),"span":span(o.span),"constraints":constraints(&o.constraints),"path":path,"bypass":format!("{:?}",o.bypass),"captures":o.captures.iter().map(|c|{
            let value=match &c.value { CaptureValue::Number(n)=>json!({"number":decimal(n)}),CaptureValue::Operator(s)=>json!({"operator":s}),CaptureValue::Text(s)=>json!({"text":s}),CaptureValue::Bytes(b)=>json!({"bytes":b}) };
            json!({"name":c.name,"span":span(c.span),"value":value})
        }).collect::<Vec<_>>()})
    }).collect::<Vec<_>>()})
}
fn routes() -> Value {
    let pop = Population::new("parity", "synthetic", "text");
    let mut table = HybridTable::default();
    add(&mut table, &pop, 10, "separate target");
    let r = registry(1);
    let caps = r.capabilities(vec![]).unwrap();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
    let kinds = vec![
        (
            "grammar",
            RouteKind::Grammar(vec![
                GrammarAtom::Number("left".into()),
                GrammarAtom::Operator {
                    name: "operator".into(),
                    allowed: vec!["+".into()],
                },
                GrammarAtom::Number("right".into()),
            ]),
        ),
        (
            "wildcard",
            RouteKind::Wildcard {
                mode: WildcardMode::Prefix,
                literal: "2".into(),
                bypass: SimilarityBypass::Declared,
            },
        ),
        (
            "automaton",
            RouteKind::Automaton {
                handler: "Tape".into(),
                program: ",.".into(),
                expected: vec![b'2'],
            },
        ),
    ];
    index
        .register(
            &table,
            &r,
            kinds
                .into_iter()
                .map(|(id, kind)| RouteRule {
                    id: id.into(),
                    entries: vec![EntryRef(10)],
                    scopes: vec![Scope::NodeAction],
                    kind,
                })
                .collect(),
        )
        .unwrap();
    let query = prep("2+3");
    let hits = candidates(
        &table,
        &index,
        &pop,
        Scope::NodeAction,
        &query,
        &r,
        &caps,
        RouteBudget::default(),
    )
    .unwrap();
    let repeated = prep("2+3; 2+3");
    let repeated_hits = candidates(
        &table,
        &index,
        &pop,
        Scope::NodeAction,
        &repeated,
        &r,
        &caps,
        RouteBudget::default(),
    )
    .unwrap();
    let blocked = prep("not 2+3");
    let blocked_hits = candidates(
        &table,
        &index,
        &pop,
        Scope::NodeAction,
        &blocked,
        &r,
        &caps,
        RouteBudget::default(),
    )
    .unwrap();
    json!({"query":prepared(&query),"hits":hits.iter().map(candidate).collect::<Vec<_>>(),"repeated":repeated_hits.iter().map(candidate).collect::<Vec<_>>(),"blocked":blocked_hits.iter().map(candidate).collect::<Vec<_>>(),"rules":index.rule_count()})
}
fn shared_low() -> Value {
    let pop = Population::new("parity", "synthetic", "text");
    let mut table = HybridTable::default();
    add(&mut table, &pop, 1, "2+3");
    add(&mut table, &pop, 2, "8+9");
    let a = prep("2+3");
    let b = prep("8+9");
    let k = text_v3::encode(first(&a)).unwrap();
    let refs = table.find_all(&pop, k.pattern_key());
    let originals = refs
        .iter()
        .map(|id| {
            table
                .get(&pop, NodeRef(100 + id.0), *id)
                .unwrap()
                .original
                .clone()
        })
        .collect::<Vec<_>>();
    json!({"refs":refs.iter().map(|r|r.0.to_string()).collect::<Vec<_>>(),"originals":originals,"keys":[key(first(&a)),key(first(&b))],"scans":[evidence(first(&a),first(&a)),evidence(first(&a),first(&b))]})
}
/// Executes real immutable APIs with synthetic inputs and fixed caller draws.
/// All i128 evidence is encoded as decimal strings, never lossy JSON numbers.
pub fn run() -> String {
    let utility = rich("never bright birds give bank also hello");
    let weighted = |draw| {
        prepare_with(
            "bright",
            true,
            draw,
            Budget {
                variants_per_clause: 2,
                ..Budget::default()
            },
        )
        .unwrap()
    };
    let quoted = rich("\"never also two 界\"; never bright birds give bank; give");
    let unframed = prep("one two three four five six seven");
    let decimal_input = prep("1.200 + 0.00");
    let numeric = [prep("two plus two"), prep("2+2")];
    let conditional = [
        rich("if not rain then alice sees bird"),
        rich("if not rain then not alice sees bird"),
        rich("if rain then alice sees bird"),
        rich("if maybe not rain then alice sees bird"),
        rich("if rain then maybe not alice sees bird"),
    ];
    let boundaries = [6, 7, 16, 17]
        .into_iter()
        .map(|n| key(first(&prep(&vec!["word"; n].join(" ")))))
        .collect::<Vec<_>>();
    let repeated_a = prep("red red blue");
    let repeated_b = prep("red blue blue");
    let partial_a = rich("alice sees bird");
    let partial_b = rich("alice sees stone");
    let huge_s = Rational::new(10_i128.pow(30) + 1, 3).unwrap();
    let huge_d = Rational::new(10_i128.pow(30), 3).unwrap();
    let mut huge = totals(reduce_totals(huge_s, huge_d).unwrap());
    huge["overflow"] = json!(format!(
        "{:?}",
        reduce_totals(
            Rational::new(i128::MAX, 1).unwrap(),
            Rational::new(-1, 1).unwrap()
        )
        .unwrap_err()
    ));
    let fail = |s: &str| {
        format!(
            "{:?}",
            prepare_with(s, false, 0, Budget::default()).unwrap_err()
        )
    };
    json!({"schema":"task8-language-mechanism-probe-v1","utility":prepared(&utility),"weighted":[prepared(&weighted(0)),prepared(&weighted(u64::MAX))],"quoted":prepared(&quoted),"unframed":prepared(&unframed),"numeric":numeric.iter().map(prepared).collect::<Vec<_>>(),"decimal":prepared(&decimal_input),"boundaries":boundaries,
        "control_keys":[key(last(&conditional[0])),key(last(&conditional[4])),key(first(&quoted)),key(first(&prep("1+2+3+4"))),key(first(&prep("1+2+3+4+5+6+7+8+9")))],
        "errors":{"division":fail("1/0"),"decimal":fail("9999999999999999999+1"),"quote":fail("\"unterminated"),"syntax":fail("two plus"),"nonfinite":fail("10^1000"),"work":format!("{:?}",prepare_with("bright",true,0,Budget { work_steps:1,..Budget::default() }).unwrap_err())},
        "scripts":scripts(),"scans":{"antecedent_body_conflict":evidence(last(&conditional[0]),last(&conditional[1])),"antecedent_only":evidence(last(&conditional[0]),last(&conditional[2])),"uncertain_antecedent":evidence(last(&conditional[2]),last(&conditional[3])),"uncertain_body":evidence(last(&conditional[2]),last(&conditional[4])),"numeric":evidence(first(&numeric[0]),first(&numeric[1])),"repeated":evidence(first(&repeated_a),first(&repeated_b)),"partial_roles":evidence(first(&partial_a),first(&partial_b))},"huge":huge,"shared_low":shared_low(),"routes":routes()}).to_string()
}
