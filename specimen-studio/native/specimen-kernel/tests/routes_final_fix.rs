use specimen_kernel::{
    identity::*,
    language::Expr,
    language_v3::{script::*, *},
    scanner_v3::routes::*,
    text_v3,
};
fn prepared(text: &str) -> Prepared {
    let t = UtilityTables::new(
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
            id: "id".into(),
            text: text.into(),
        },
        &ContextSnapshot {
            population: "p".into(),
            utility_version: "v".into(),
            tokens: vec![],
            conditions: vec![],
        },
        &t,
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap()
}
fn registry() -> Registry {
    Registry::new("r", 1, vec![], vec![]).unwrap()
}
fn pop() -> Population {
    Population::new("t", "s", "n")
}
fn entry(id: u128, key: PatternKey, ast: ActionAst) -> EntryRecord {
    EntryRecord {
        population: pop(),
        node: NodeRef(id + 10000),
        reference: EntryRef(id),
        key,
        original: format!("target{id}"),
        ast,
        resource: None,
    }
}
fn key() -> PatternKey {
    text_v3::encode(&prepared("hello").clauses()[0].variants()[0])
        .unwrap()
        .pattern_key()
        .clone()
}
fn query(
    t: &HybridTable,
    i: &RouteIndex,
    p: &Prepared,
    steps: usize,
) -> Result<Vec<Candidate>, RouteError> {
    let r = registry();
    candidates(
        t,
        i,
        &pop(),
        Scope::NodeAction,
        p,
        &r,
        &r.capabilities(vec![]).unwrap(),
        RouteBudget {
            work_steps: steps,
            ..RouteBudget::default()
        },
    )
}
#[test]
fn empty_query_large_table_requires_entry_inspection_budget() {
    let mut t = HybridTable::default();
    for id in 1..=257 {
        t.insert(entry(id, key(), ActionAst::default()), None)
            .unwrap();
    }
    let i = RouteIndex::new(pop(), vec![Scope::NodeAction], &registry()).unwrap();
    let p = prepared("");
    assert_eq!(query(&t, &i, &p, 1), Err(RouteError::BudgetExceeded));
    assert!(query(&t, &i, &p, 100000).unwrap().is_empty());
}
#[test]
fn selected_unbounded_alternative_vector_is_charged_before_clone() {
    let mut t = HybridTable::default();
    let ast = ActionAst {
        bind: vec![],
        alternatives: (0..12000)
            .map(|_| Alternative {
                action: vec![],
                inhibited: true,
                weight: 1.0,
            })
            .collect(),
    };
    t.insert(entry(1, key(), ast), None).unwrap();
    let i = RouteIndex::new(pop(), vec![Scope::NodeAction], &registry()).unwrap();
    assert_eq!(
        query(&t, &i, &prepared("hello"), 100),
        Err(RouteError::BudgetExceeded)
    );
    let out = query(&t, &i, &prepared("hello"), 100000).unwrap();
    assert_eq!(out[0].target_ast.alternatives.len(), 12000);
}
#[test]
fn nested_target_payloads_and_shared_origins_have_exact_budget_boundary() {
    let mut t = HybridTable::default();
    for id in 1..=30 {
        let ast = ActionAst {
            bind: vec![Expr::Call {
                name: "Text".into(),
                args: vec![vec![Expr::Text {
                    value: "a".repeat(80),
                }]],
            }],
            alternatives: vec![Alternative {
                action: vec![Expr::Text {
                    value: "b".repeat(80),
                }],
                inhibited: id % 2 == 0,
                weight: 0.5,
            }],
        };
        t.insert(entry(id, key(), ast), None).unwrap();
    }
    let r = registry();
    let mut i = RouteIndex::new(pop(), vec![Scope::NodeAction], &r).unwrap();
    i.register(
        &t,
        &r,
        vec![RouteRule {
            id: "shared".into(),
            entries: vec![EntryRef(1), EntryRef(30)],
            scopes: vec![Scope::NodeAction],
            kind: RouteKind::Wildcard {
                mode: WildcardMode::Prefix,
                literal: "hello".into(),
                bypass: SimilarityBypass::Declared,
            },
        }],
    )
    .unwrap();
    let p = prepared("hello");
    let full = query(&t, &i, &p, 100000).unwrap();
    assert_eq!(
        full.iter().map(|c| c.entry).collect::<Vec<_>>(),
        (1..=30).map(EntryRef).collect::<Vec<_>>()
    );
    assert_eq!(full[0].origins.len(), 2);
    assert_eq!(full[29].origins.len(), 2);
    assert_eq!(full[0].original, "target1");
    assert!(full[1].target_ast.alternatives[0].inhibited);
    let (mut lo, mut hi) = (1, 100000);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if query(&t, &i, &p, mid).is_ok() {
            hi = mid
        } else {
            lo = mid + 1
        }
    }
    assert_eq!(query(&t, &i, &p, lo).unwrap(), full);
    assert_eq!(query(&t, &i, &p, lo - 1), Err(RouteError::BudgetExceeded));
    // Nested strings, vectors and selected original bytes must cost more than flat references.
    assert!(lo > 30 * 160);
}
