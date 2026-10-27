use super::*;
pub(super) fn run() -> Value {
    let r = registry(1);
    let caps = r.capabilities(vec![]).unwrap();
    let script = compile("Text(\"effect\")", Scope::NodeAction, &r).unwrap();
    let conditions=[(r#""ALLOW""#,r#""allow""#),(r#""Ä""#,r#""ä""#),(r#""ALLOW""#,r#""ALLOW""#),("RAIN and snow","rain and snow"),("rain and snow","rain"),(r#""a\"B""#,r#""a\"b""#)].map(|(antecedent,attestation)| {
        let mut ctx=context(); ctx.conditions=vec![(attestation.into(),true)];
        let p=prepare(Input{id:"synthetic-input".into(),text:format!("if {antecedent} then go")},&ctx,&tables(false),Budget::default(),&mut FixedDraw(0)).unwrap();
        let a=last(&p);
        json!({"antecedent":antecedent,"attestation":attestation,"prepared":prepared(&p),"guard":format!("{:?}",execute_for_action(&script,a,&r,&caps,ScriptBudget::default()).map(|_|"executed"))})
    });
    let invalid = [
        (
            UtilityKind::Grammar,
            "halt",
            UtilityEffect::Role(Role::Negation),
        ),
        (
            UtilityKind::Conjunction,
            "don't",
            UtilityEffect::Conjunction(Connective::And),
        ),
        (
            UtilityKind::Conjunction,
            "if",
            UtilityEffect::Conjunction(Connective::And),
        ),
    ]
    .map(|(kind, phrase, effect)| {
        let snapshots = UtilityKind::ALL
            .into_iter()
            .map(|k| UtilitySnapshot {
                population: "parity".into(),
                version: "v1".into(),
                kind: k,
                rows: if k == kind {
                    vec![UtilityRow {
                        reference: "control".into(),
                        phrase: phrase.into(),
                        effect: effect.clone(),
                    }]
                } else {
                    vec![]
                },
            })
            .collect();
        format!(
            "{:?}",
            UtilityTables::new("parity", "v1", snapshots).unwrap_err()
        )
    });
    let population = Population::new("budget", "probe", "text");
    let index = RouteIndex::new(population.clone(), vec![Scope::NodeAction], &r).unwrap();
    let key = text_v3::encode(first(&prep("hello")))
        .unwrap()
        .pattern_key()
        .clone();
    let entry = |id, ast| EntryRecord {
        population: population.clone(),
        node: NodeRef(id + 10000),
        reference: EntryRef(id),
        key: key.clone(),
        original: format!("target{id}"),
        ast,
        resource: None,
    };
    let mut table = HybridTable::default();
    for id in 1..=257 {
        table.insert(entry(id, ActionAst::default()), None).unwrap();
    }
    let q = |t: &HybridTable, text: &str, steps| {
        candidates(
            t,
            &index,
            &population,
            Scope::NodeAction,
            &prep(text),
            &r,
            &caps,
            RouteBudget {
                work_steps: steps,
                ..RouteBudget::default()
            },
        )
    };
    let empty = format!("{:?}", q(&table, "", 1).unwrap_err());
    let mut selected = HybridTable::default();
    selected
        .insert(
            entry(
                1,
                ActionAst {
                    bind: vec![],
                    alternatives: (0..12000)
                        .map(|_| Alternative {
                            action: vec![],
                            inhibited: true,
                            weight: 1.0,
                        })
                        .collect(),
                },
            ),
            None,
        )
        .unwrap();
    let large = format!("{:?}", q(&selected, "hello", 100).unwrap_err());
    let complete = q(&selected, "hello", 100000).unwrap();
    let a = prep("alpha, beta gamma delta epsilon zeta");
    json!({"conditions":conditions,"invalid_controls":invalid,"empty_table_records":257,"empty_query_error":empty,"selected_alternatives_error":large,"complete_alternative_count":complete[0].target_ast.alternatives.len(),"complexity":{"getter":first(&a).complexity_token_count(),"key":key_value(first(&a))}})
}
fn key_value(a: &PreparedAction) -> Value {
    super::key(a)
}
