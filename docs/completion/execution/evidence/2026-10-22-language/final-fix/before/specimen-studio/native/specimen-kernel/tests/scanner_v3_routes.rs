use specimen_kernel::{
    identity::*,
    language_v3::{script::*, *},
    scanner_v3::{routes::*, *},
    text_v3,
};
fn prepared(text: &str) -> Prepared {
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
            id: "original".into(),
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
}
fn population() -> Population {
    Population::new("tenant", "specimen", "text")
}
fn registry() -> Registry {
    Registry::new(
        "routes",
        1,
        vec![],
        vec![Declaration {
            name: "Tape".into(),
            aliases: vec![],
            scopes: vec![Scope::NodeAction],
            reads: vec![],
            handler: Handler::Tape,
        }],
    )
    .unwrap()
}
fn add(table: &mut HybridTable, p: &Population, id: u128, text: &str) {
    let original = prepared(text);
    table
        .insert(
            EntryRecord {
                population: p.clone(),
                node: NodeRef(id + 1000),
                reference: EntryRef(id),
                key: text_v3::encode(&original.clauses()[0].variants()[0])
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
fn rule(id: &str, entry: u128, kind: RouteKind) -> RouteRule {
    RouteRule {
        id: id.into(),
        entries: vec![EntryRef(entry)],
        scopes: vec![Scope::NodeAction],
        kind,
    }
}
fn wild(literal: &str) -> RouteKind {
    RouteKind::Wildcard {
        mode: WildcardMode::Prefix,
        literal: literal.into(),
        bypass: SimilarityBypass::Declared,
    }
}
fn query(
    table: &HybridTable,
    index: &RouteIndex,
    pop: &Population,
    text: &str,
) -> Result<Vec<Candidate>, RouteError> {
    let r = registry();
    let caps = r.capabilities(vec![]).unwrap();
    candidates(
        table,
        index,
        pop,
        Scope::NodeAction,
        &prepared(text),
        &r,
        &caps,
        RouteBudget::default(),
    )
}
#[test]
fn s05_distinct_originals_share_low_key_but_full_scanner_evidence_differs() {
    let mut table = HybridTable::default();
    let pop = population();
    add(&mut table, &pop, 1, "2+3");
    add(&mut table, &pop, 2, "8+9");
    let p = prepared("2+3");
    let a = &p.clauses()[0].variants()[0];
    let refs = table.find_all(&pop, text_v3::encode(a).unwrap().pattern_key());
    assert_eq!(refs, vec![EntryRef(1), EntryRef(2)]);
    let originals = [prepared("2+3"), prepared("8+9")];
    let evidence = originals
        .iter()
        .map(|p| scan(a, &p.clauses()[0].variants()[0]).unwrap())
        .collect::<Vec<_>>();
    assert_ne!(evidence[0].components[3], evidence[1].components[3]);
    let index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &registry()).unwrap();
    assert_eq!(
        query(&table, &index, &pop, "2+3")
            .unwrap()
            .iter()
            .map(|c| c.entry)
            .collect::<Vec<_>>(),
        refs
    );
}
#[test]
fn grammar_wildcard_and_automaton_routes_cross_literal_keys_and_keep_captures() {
    let mut table = HybridTable::default();
    let pop = population();
    for id in 1..=3 {
        add(&mut table, &pop, id, &format!("target{id}"));
    }
    let r = registry();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
    index
        .register(
            &table,
            &r,
            vec![
                rule(
                    "grammar",
                    1,
                    RouteKind::Grammar(vec![
                        GrammarAtom::Number("left".into()),
                        GrammarAtom::Operator {
                            name: "op".into(),
                            allowed: vec!["+".into()],
                        },
                        GrammarAtom::Number("right".into()),
                    ]),
                ),
                rule("wild", 2, wild("2")),
                rule(
                    "tape",
                    3,
                    RouteKind::Automaton {
                        handler: "Tape".into(),
                        program: ",.".into(),
                        expected: vec![b'2'],
                    },
                ),
            ],
        )
        .unwrap();
    let hits = query(&table, &index, &pop, "2+3").unwrap();
    assert_eq!(
        hits.iter().map(|c| c.entry).collect::<Vec<_>>(),
        vec![EntryRef(1), EntryRef(3), EntryRef(2)]
    );
    assert_eq!(
        hits[0].origins[0].captures[0].span,
        Span { start: 0, end: 1 }
    );
    assert!(matches!(
        hits[0].origins[0].captures[0].value,
        CaptureValue::Number(_)
    ));
    assert!(matches!(
        hits[1].origins[0].path,
        CandidatePath::Automaton {
            status: AutomatonStatus::Matched,
            ..
        }
    ));
    assert_eq!(hits[2].origins[0].bypass, SimilarityBypass::Declared);
    assert!(hits.iter().all(|c| c.origins[0].constraints.executable));
}
#[test]
fn population_scope_reference_and_registration_failures_are_atomic() {
    let mut table = HybridTable::default();
    let pop = population();
    add(&mut table, &pop, 1, "target");
    let other = Population::new("other", "specimen", "text");
    add(&mut table, &other, 2, "elsewhere");
    let r = registry();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
    for bad in [
        rule("missing", 99, wild("x")),
        rule("foreign", 2, wild("x")),
        RouteRule {
            scopes: vec![Scope::Vote],
            ..rule("scope", 1, wild("x"))
        },
    ] {
        assert!(
            index
                .register(&table, &r, vec![rule("good", 1, wild("x")), bad])
                .is_err()
        );
        assert_eq!(index.rule_count(), 0);
    }
    index
        .register(&table, &r, vec![rule("good", 1, wild("x"))])
        .unwrap();
    assert_eq!(
        query(&table, &index, &other, "x"),
        Err(RouteError::PopulationMismatch)
    );
    let caps = r.capabilities(vec![]).unwrap();
    assert_eq!(
        candidates(
            &table,
            &index,
            &pop,
            Scope::Vote,
            &prepared("x"),
            &r,
            &caps,
            RouteBudget::default()
        ),
        Err(RouteError::UndeclaredScope)
    );
    assert_eq!(index.rule_count(), 1);
}
#[test]
fn blocked_routes_retain_constraints_without_successful_automaton_execution() {
    let mut table = HybridTable::default();
    let pop = population();
    add(&mut table, &pop, 1, "target");
    let r = registry();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
    index
        .register(
            &table,
            &r,
            vec![
                rule(
                    "wild",
                    1,
                    RouteKind::Wildcard {
                        mode: WildcardMode::Contains,
                        literal: "hello".into(),
                        bypass: SimilarityBypass::Declared,
                    },
                ),
                rule(
                    "tape",
                    1,
                    RouteKind::Automaton {
                        handler: "Tape".into(),
                        program: "+[]".into(),
                        expected: vec![],
                    },
                ),
            ],
        )
        .unwrap();
    let hits = query(&table, &index, &pop, "not hello").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].origins.len(), 2);
    assert!(
        hits[0]
            .origins
            .iter()
            .all(|o| o.constraints.inhibited && !o.constraints.executable)
    );
    assert!(hits[0].origins.iter().any(|o| matches!(
        o.path,
        CandidatePath::Automaton {
            status: AutomatonStatus::Blocked,
            ..
        }
    )));
}
#[test]
fn runaway_and_shared_budgets_return_errors_without_partial_candidates() {
    let mut table = HybridTable::default();
    let pop = population();
    add(&mut table, &pop, 1, "target");
    let r = registry();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
    index
        .register(
            &table,
            &r,
            vec![
                rule("a-wild", 1, wild("x")),
                rule(
                    "z-runaway",
                    1,
                    RouteKind::Automaton {
                        handler: "Tape".into(),
                        program: "+[]".into(),
                        expected: vec![],
                    },
                ),
            ],
        )
        .unwrap();
    assert_eq!(
        query(&table, &index, &pop, "x"),
        Err(RouteError::Language(LanguageError::StepBudget))
    );
    let caps = r.capabilities(vec![]).unwrap();
    assert_eq!(
        candidates(
            &table,
            &index,
            &pop,
            Scope::NodeAction,
            &prepared("x"),
            &r,
            &caps,
            RouteBudget {
                work_steps: 1,
                ..RouteBudget::default()
            }
        ),
        Err(RouteError::BudgetExceeded)
    );
}
#[test]
fn duplicate_paths_and_repeated_contains_keep_distinct_capture_origins() {
    let mut table = HybridTable::default();
    let pop = population();
    add(&mut table, &pop, 1, "xx");
    let r = registry();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
    index
        .register(
            &table,
            &r,
            vec![
                rule("prefix", 1, wild("x")),
                rule(
                    "contains",
                    1,
                    RouteKind::Wildcard {
                        mode: WildcardMode::Contains,
                        literal: "x".into(),
                        bypass: SimilarityBypass::RequireFullScan,
                    },
                ),
            ],
        )
        .unwrap();
    let first = query(&table, &index, &pop, "xx").unwrap();
    let second = query(&table, &index, &pop, "xx").unwrap();
    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].origins.len(), 4);
    assert_ne!(first[0].origins[1].captures, first[0].origins[2].captures);
}
#[test]
fn wildcard_and_automaton_fixed_caps_are_checked_before_batch_publication() {
    let mut table = HybridTable::default();
    let pop = population();
    add(&mut table, &pop, 1, "target");
    let r = registry();
    for automaton in [false, true] {
        let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
        let rules = (0..65)
            .map(|i| {
                rule(
                    &format!("rule{i:03}"),
                    1,
                    if automaton {
                        RouteKind::Automaton {
                            handler: "Tape".into(),
                            program: ",.".into(),
                            expected: vec![b'x'],
                        }
                    } else {
                        wild("")
                    },
                )
            })
            .collect();
        assert_eq!(
            index.register(&table, &r, rules),
            Err(RouteError::RuleLimit)
        );
        assert_eq!(index.rule_count(), 0);
        let rules = (0..64)
            .map(|i| {
                rule(
                    &format!("rule{i:03}"),
                    1,
                    if automaton {
                        RouteKind::Automaton {
                            handler: "Tape".into(),
                            program: ",.".into(),
                            expected: vec![b'x'],
                        }
                    } else {
                        wild("")
                    },
                )
            })
            .collect();
        index.register(&table, &r, rules).unwrap();
        assert_eq!(index.rule_count(), 64);
    }
}
#[test]
fn automata_share_output_reservations_and_registry_identity_is_current() {
    let mut table = HybridTable::default();
    let pop = population();
    add(&mut table, &pop, 1, "target");
    let r = registry();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
    index
        .register(
            &table,
            &r,
            (0..2)
                .map(|i| {
                    rule(
                        &format!("tape{i}"),
                        1,
                        RouteKind::Automaton {
                            handler: "Tape".into(),
                            program: ",.".into(),
                            expected: vec![b'x'],
                        },
                    )
                })
                .collect(),
        )
        .unwrap();
    let caps = r.capabilities(vec![]).unwrap();
    assert_eq!(
        candidates(
            &table,
            &index,
            &pop,
            Scope::NodeAction,
            &prepared("x"),
            &r,
            &caps,
            RouteBudget {
                output_bytes: 1024,
                ..RouteBudget::default()
            }
        ),
        Err(RouteError::BudgetExceeded)
    );
    let changed = Registry::new("routes", 2, vec![], vec![]).unwrap();
    let caps2 = changed.capabilities(vec![]).unwrap();
    assert_eq!(
        candidates(
            &table,
            &index,
            &pop,
            Scope::NodeAction,
            &prepared("x"),
            &changed,
            &caps2,
            RouteBudget::default()
        ),
        Err(RouteError::StaleRegistry)
    );
    assert_eq!(
        index.register(&table, &changed, vec![rule("next", 1, wild("x"))]),
        Err(RouteError::StaleRegistry)
    );
    assert_eq!(index.rule_count(), 2);
}
#[test]
fn declared_scopes_do_not_leak_route_hits_and_removed_refs_refuse() {
    let mut table = HybridTable::default();
    let pop = population();
    add(&mut table, &pop, 1, "target");
    let r = registry();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction, Scope::Vote], &r).unwrap();
    index
        .register(&table, &r, vec![rule("wild", 1, wild("x"))])
        .unwrap();
    let caps = r.capabilities(vec![]).unwrap();
    assert!(
        candidates(
            &table,
            &index,
            &pop,
            Scope::Vote,
            &prepared("x"),
            &r,
            &caps,
            RouteBudget::default()
        )
        .unwrap()
        .is_empty()
    );
    table.remove(&pop, NodeRef(1001), EntryRef(1)).unwrap();
    assert_eq!(
        query(&table, &index, &pop, "x"),
        Err(RouteError::MissingEntry)
    );
}
#[test]
fn unresolved_condition_and_utf8_spans_survive_wildcard_routes() {
    let mut table = HybridTable::default();
    let pop = population();
    add(&mut table, &pop, 1, "target");
    let r = registry();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
    index
        .register(
            &table,
            &r,
            vec![rule(
                "wild",
                1,
                RouteKind::Wildcard {
                    mode: WildcardMode::Contains,
                    literal: "界".into(),
                    bypass: SimilarityBypass::Declared,
                },
            )],
        )
        .unwrap();
    let hits = query(&table, &index, &pop, "if rain then 界").unwrap();
    assert_eq!(hits.len(), 1);
    let origin = &hits[0].origins[0];
    assert_eq!(
        origin.constraints.conditional,
        ConditionalDisposition::Unresolved
    );
    assert!(!origin.constraints.executable);
    let span = origin.captures[0].span;
    assert_eq!(&"if rain then 界"[span.start..span.end], "界");
}
#[test]
fn wildcard_capture_copy_work_is_reserved_before_materializing_matches() {
    let mut table = HybridTable::default();
    let pop = population();
    add(&mut table, &pop, 1, "target");
    let r = registry();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
    index
        .register(
            &table,
            &r,
            vec![rule(
                "contains",
                1,
                RouteKind::Wildcard {
                    mode: WildcardMode::Contains,
                    literal: "x".into(),
                    bypass: SimilarityBypass::RequireFullScan,
                },
            )],
        )
        .unwrap();
    let caps = r.capabilities(vec![]).unwrap();
    assert_eq!(
        candidates(
            &table,
            &index,
            &pop,
            Scope::NodeAction,
            &prepared("xxxx"),
            &r,
            &caps,
            RouteBudget {
                work_steps: 20,
                ..RouteBudget::default()
            }
        ),
        Err(RouteError::BudgetExceeded)
    );
}
#[test]
fn typed_grammar_total_pattern_bound_is_validated_atomically() {
    let mut table = HybridTable::default();
    let pop = population();
    add(&mut table, &pop, 1, "target");
    let r = registry();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
    assert_eq!(
        index.register(
            &table,
            &r,
            vec![rule(
                "long",
                1,
                RouteKind::Grammar(vec![
                    GrammarAtom::Literal("a".repeat(1500)),
                    GrammarAtom::Literal("b".repeat(1500))
                ])
            )]
        ),
        Err(RouteError::InvalidRule)
    );
    assert_eq!(index.rule_count(), 0);
}
#[test]
fn fanout_origins_and_target_inhibition_remain_separate_through_join() {
    let mut table = HybridTable::default();
    let pop = population();
    let r = registry();
    let target = prepared("target");
    table
        .insert(
            EntryRecord {
                population: pop.clone(),
                node: NodeRef(1001),
                reference: EntryRef(1),
                key: text_v3::encode(&target.clauses()[0].variants()[0])
                    .unwrap()
                    .pattern_key()
                    .clone(),
                original: "target".into(),
                ast: ActionAst {
                    bind: vec![],
                    alternatives: vec![Alternative {
                        action: vec![],
                        inhibited: true,
                        weight: 0.25,
                    }],
                },
                resource: None,
            },
            None,
        )
        .unwrap();
    let mut snapshots = UtilityKind::ALL
        .into_iter()
        .map(|kind| UtilitySnapshot {
            population: "p".into(),
            version: "v".into(),
            kind,
            rows: vec![],
        })
        .collect::<Vec<_>>();
    snapshots
        .iter_mut()
        .find(|s| s.kind == UtilityKind::Thesaurus)
        .unwrap()
        .rows = vec![UtilityRow {
        reference: "swap".into(),
        phrase: "alpha".into(),
        effect: UtilityEffect::Equivalent {
            replacement: "beta".into(),
            weight: 1.0,
        },
    }];
    let tables = UtilityTables::new("p", "v", snapshots).unwrap();
    let p = prepare(
        Input {
            id: "fanout".into(),
            text: "alpha".into(),
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
    .unwrap();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
    index
        .register(&table, &r, vec![rule("wild", 1, wild("alpha"))])
        .unwrap();
    let caps = r.capabilities(vec![]).unwrap();
    let hits = candidates(
        &table,
        &index,
        &pop,
        Scope::NodeAction,
        &p,
        &r,
        &caps,
        RouteBudget::default(),
    )
    .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].origins.len(), 2);
    assert_eq!(hits[0].origins[0].origin.fanout_id(), 0);
    assert_ne!(hits[0].origins[1].origin.fanout_id(), 0);
    assert!(hits[0].target_ast.alternatives[0].inhibited);
    assert_eq!(hits[0].target_ast.alternatives[0].weight, 0.25);
}
#[test]
fn tape_input_uses_quote_safe_preserved_utf8_without_script_interpretation() {
    let mut table = HybridTable::default();
    let pop = population();
    add(&mut table, &pop, 1, "target");
    let r = registry();
    let mut index = RouteIndex::new(pop.clone(), vec![Scope::NodeAction], &r).unwrap();
    index
        .register(
            &table,
            &r,
            vec![rule(
                "tape",
                1,
                RouteKind::Automaton {
                    handler: "Tape".into(),
                    program: ",.".into(),
                    expected: vec![b'"'],
                },
            )],
        )
        .unwrap();
    let hits = query(&table, &index, &pop, "\"Tape(\\\"+[]\\\",\\\"\\\") 界\"").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(
        hits[0].origins[0].captures[0].value,
        CaptureValue::Bytes(vec![b'"'])
    );
}
