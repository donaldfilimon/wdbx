use specimen_kernel::language_v3::*;

fn tables() -> UtilityTables {
    UtilityTables::new(
        "synthetic",
        "v1",
        UtilityKind::ALL
            .into_iter()
            .map(|kind| UtilitySnapshot {
                population: "synthetic".into(),
                version: "v1".into(),
                kind,
                rows: vec![],
            })
            .collect(),
    )
    .unwrap()
}
fn context() -> ContextSnapshot {
    ContextSnapshot {
        population: "synthetic".into(),
        utility_version: "v1".into(),
        tokens: vec![],
        conditions: vec![],
    }
}
fn prep(s: &str) -> Result<Prepared, LanguageError> {
    prepare(
        Input {
            id: "input-1".into(),
            text: s.into(),
        },
        &context(),
        &tables(),
        Budget::default(),
        &mut FixedDraw(0),
    )
}

#[test]
fn s19_numeric_grammar_preserves_exact_operands_and_source_spans() {
    let words = prep("two plus two").unwrap();
    let symbols = prep("2+2").unwrap();
    let a = &words.clauses()[0].variants()[0];
    let b = &symbols.clauses()[0].variants()[0];
    assert_eq!(
        a.captures()
            .iter()
            .map(|x| x.decimal().coefficient())
            .collect::<Vec<_>>(),
        vec![2, 2]
    );
    assert_eq!(a.operators(), b.operators());
    assert_eq!(a.captures()[0].span(), Span { start: 0, end: 3 });
    assert_eq!(a.captures()[1].span(), Span { start: 9, end: 12 });
    assert_eq!(a.arithmetic_value().unwrap(), 4.0);
    assert_eq!(
        prep("I have two birds").unwrap().clauses()[0].variants()[0]
            .captures()
            .len(),
        0
    );
}
#[test]
fn decimals_precedence_unary_and_errors_are_bounded() {
    for (s, want) in [
        ("-2^2", -4.0),
        ("2^3^2", 512.0),
        ("1.20 + .3 * 2", 1.8),
        ("(-2)^2", 4.0),
    ] {
        assert!(
            (prep(s).unwrap().clauses()[0].variants()[0]
                .arithmetic_value()
                .unwrap()
                - want)
                .abs()
                < 1e-12
        );
    }
    let p = prep("1.200 + 0.00").unwrap();
    let cs = p.clauses()[0].variants()[0].captures();
    assert_eq!(
        (cs[0].decimal().coefficient(), cs[0].decimal().scale()),
        (12, 1)
    );
    assert_eq!(
        (cs[1].decimal().coefficient(), cs[1].decimal().scale()),
        (0, 0)
    );
    assert_eq!(prep("1/0").unwrap_err(), LanguageError::DivisionByZero);
    assert_eq!(
        prep("9999999999999999999 + 1").unwrap_err(),
        LanguageError::Decimal
    );
    assert_eq!(prep("10^1000").unwrap_err(), LanguageError::NonFinite);
}
#[test]
fn s20_quotes_negation_independent_clauses_and_conditionals() {
    let p = prep("\"don't calculate two\"; don't calculate 2+2; hello").unwrap();
    assert!(!p.clauses()[0].variants()[0].inhibited());
    assert!(p.clauses()[0].variants()[0].tokens()[0].quoted());
    assert!(p.clauses()[1].variants()[0].inhibited());
    assert!(!p.clauses()[2].variants()[0].inhibited());
    let p = prep("if rain then calculate 2+2 and hello or bye but wait").unwrap();
    assert!(
        matches!(p.ast(), SemanticAst::IfThen(_,body) if matches!(body.as_ref(),SemanticAst::But(..)))
    );
    assert_eq!(
        p.clauses()[1].variants()[0].conditional(),
        ConditionalDisposition::Unresolved
    );
    assert!(!p.clauses()[1].variants()[0].executable());
    assert!(prep("perhaps don't calculate").unwrap().clauses()[0].variants()[0].inhibited());
    assert_eq!(
        prep("\"unterminated").unwrap_err(),
        LanguageError::UnclosedQuote
    );
}
fn row(reference: &str, phrase: &str, effect: UtilityEffect) -> UtilityRow {
    UtilityRow {
        reference: reference.into(),
        phrase: phrase.into(),
        effect,
    }
}
fn custom(rows: Vec<(UtilityKind, UtilityRow)>) -> UtilityTables {
    let mut snapshots = UtilityKind::ALL
        .into_iter()
        .map(|kind| UtilitySnapshot {
            population: "synthetic".into(),
            version: "v1".into(),
            kind,
            rows: vec![],
        })
        .collect::<Vec<_>>();
    for (kind, row) in rows {
        snapshots
            .iter_mut()
            .find(|s| s.kind == kind)
            .unwrap()
            .rows
            .push(row);
    }
    UtilityTables::new("synthetic", "v1", snapshots).unwrap()
}
#[test]
fn all_nine_utilities_have_observable_semantics_and_safe_fanout() {
    let t = custom(vec![
        (
            UtilityKind::Dictionary,
            row(
                "d",
                "bright",
                UtilityEffect::Dictionary {
                    lemma: "bright".into(),
                    definition: "luminous".into(),
                },
            ),
        ),
        (
            UtilityKind::Thesaurus,
            row(
                "t",
                "bright",
                UtilityEffect::Equivalent {
                    replacement: "radiant".into(),
                    weight: 1.0,
                },
            ),
        ),
        (
            UtilityKind::AntiThesaurus,
            row(
                "a",
                "bank",
                UtilityEffect::Sense {
                    lemma: "riverbank".into(),
                    required_context: vec!["river".into()],
                    weight: 1.0,
                },
            ),
        ),
        (
            UtilityKind::Chargebook,
            row(
                "c",
                "bright",
                UtilityEffect::Charge {
                    valence: 0.5,
                    intensity: 0.7,
                },
            ),
        ),
        (
            UtilityKind::VerbFrames,
            row(
                "v",
                "give",
                UtilityEffect::Frame {
                    subject: true,
                    object: true,
                    numeric_object: false,
                },
            ),
        ),
        (
            UtilityKind::Conjunction,
            row("j", "also", UtilityEffect::Conjunction(Connective::And)),
        ),
        (
            UtilityKind::Negation,
            row("n", "never", UtilityEffect::Negation { uncertain: false }),
        ),
        (
            UtilityKind::Morphology,
            row(
                "m",
                "s",
                UtilityEffect::Suffix {
                    replacement: "".into(),
                },
            ),
        ),
        (
            UtilityKind::Grammar,
            row("g", "give", UtilityEffect::Role(Role::Predicate)),
        ),
    ]);
    let mut c = context();
    c.tokens.push("river".into());
    let p = prepare(
        Input {
            id: "x".into(),
            text: "never bright birds give bank also hello".into(),
        },
        &c,
        &t,
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap();
    let a = &p.clauses()[0].variants()[0];
    assert!(a.inhibited());
    assert!(a.tokens().iter().any(|t| t.lemma() == "bird"));
    assert!(a.definitions().iter().any(|x| x.definition() == "luminous"));
    assert_eq!(a.charges()[0].valence(), 0.5);
    assert!(p.clauses()[0].variants().len() > 1);
    assert!(p.clauses()[0].variants().iter().all(|a| a.inhibited()));
    assert!(
        p.clauses()[0]
            .variants()
            .iter()
            .any(|a| a.tokens().iter().any(|t| t.lemma() == "riverbank"))
    );
    assert!(matches!(a.spo().predicate(), Slot::Resolved(s) if s=="give"));
    assert_eq!(p.clauses()[0].variants()[1].origin().input_id(), "x");
}
#[test]
fn longest_phrase_context_ties_and_missing_frames_stay_explicit() {
    let t = custom(vec![
        (
            UtilityKind::Dictionary,
            row(
                "a",
                "red",
                UtilityEffect::Dictionary {
                    lemma: "r".into(),
                    definition: "short".into(),
                },
            ),
        ),
        (
            UtilityKind::Dictionary,
            row(
                "b",
                "red fox",
                UtilityEffect::Dictionary {
                    lemma: "fox".into(),
                    definition: "long".into(),
                },
            ),
        ),
        (
            UtilityKind::AntiThesaurus,
            row(
                "s2",
                "bank",
                UtilityEffect::Sense {
                    lemma: "shore".into(),
                    required_context: vec![],
                    weight: 1.0,
                },
            ),
        ),
        (
            UtilityKind::AntiThesaurus,
            row(
                "s1",
                "bank",
                UtilityEffect::Sense {
                    lemma: "finance".into(),
                    required_context: vec![],
                    weight: 1.0,
                },
            ),
        ),
        (
            UtilityKind::VerbFrames,
            row(
                "v",
                "give",
                UtilityEffect::Frame {
                    subject: true,
                    object: true,
                    numeric_object: false,
                },
            ),
        ),
    ]);
    let p = prepare(
        Input {
            id: "x".into(),
            text: "red fox bank; give".into(),
        },
        &context(),
        &t,
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap();
    assert_eq!(
        p.clauses()[0].variants()[0].definitions()[0].definition(),
        "long"
    );
    assert!(
        p.clauses()[0]
            .variants()
            .iter()
            .any(|a| a.tokens().iter().any(|t| t.lemma() == "finance"))
    );
    assert!(
        p.clauses()[0]
            .variants()
            .iter()
            .any(|a| a.tokens().iter().any(|t| t.lemma() == "shore"))
    );
    assert!(!p.clauses()[1].variants()[0].executable());
    assert!(matches!(
        p.clauses()[1].variants()[0].spo().object(),
        Slot::Unresolved
    ));
}
#[test]
fn conditional_action_ast_retains_condition_and_uncertain_negation_scope() {
    let p = prep("if rain then hello").unwrap();
    let a = &p.clauses()[1].variants()[0];
    assert!(
        matches!(a.ast(),SemanticAst::IfThen(cond,_) if matches!(cond.as_ref(),SemanticAst::Action(tokens) if tokens[0].lemma()=="rain"))
    );
    assert_eq!(a.complexity_token_count(), 3);
    let p = prep("maybe don't calculate 2+2 and calculate 3+3; hello").unwrap();
    assert!(p.clauses()[1].variants()[0].inhibited());
    assert!(p.clauses()[1].variants()[0].uncertain_negation());
    assert!(!p.clauses()[2].variants()[0].inhibited());
    let mut c = context();
    c.conditions.push(("rain".into(), true));
    let p = prepare(
        Input {
            id: "c".into(),
            text: "if rain then hello".into(),
        },
        &c,
        &tables(),
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap();
    assert!(p.clauses()[1].variants()[0].executable());
}
#[test]
fn deterministic_host_draws_original_caps_zero_weights_and_protected_spans() {
    let t = custom(vec![
        (
            UtilityKind::Thesaurus,
            row(
                "a",
                "hello",
                UtilityEffect::Equivalent {
                    replacement: "hi".into(),
                    weight: 1.0,
                },
            ),
        ),
        (
            UtilityKind::Thesaurus,
            row(
                "b",
                "hello",
                UtilityEffect::Equivalent {
                    replacement: "greetings".into(),
                    weight: 3.0,
                },
            ),
        ),
        (
            UtilityKind::Thesaurus,
            row(
                "z",
                "world",
                UtilityEffect::Equivalent {
                    replacement: "earth".into(),
                    weight: 0.0,
                },
            ),
        ),
    ]);
    let b = Budget {
        variants_per_clause: 2,
        ..Budget::default()
    };
    let run = |draw| {
        prepare(
            Input {
                id: "f".into(),
                text: "hello \"hello two\" world".into(),
            },
            &context(),
            &t,
            b,
            &mut FixedDraw(draw),
        )
        .unwrap()
    };
    let first = run(0);
    let last = run(u64::MAX);
    assert_eq!(first.clauses()[0].variants()[1].tokens()[0].lemma(), "hi");
    assert_eq!(
        last.clauses()[0].variants()[1].tokens()[0].lemma(),
        "greetings"
    );
    assert_eq!(
        first.clauses()[0].variants()[1].tokens()[1].lemma(),
        "hello two"
    );
    assert_eq!(
        first.clauses()[0].variants()[1].tokens()[0].span(),
        Span { start: 0, end: 5 }
    );
    assert_eq!(
        first.clauses()[0].variants()[1].tokens()[0].source(),
        "hello"
    );
    let z = prepare(
        Input {
            id: "z".into(),
            text: "world".into(),
        },
        &context(),
        &t,
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap();
    assert_eq!(z.draws_used(), 0);
    assert_eq!(z.clauses()[0].variants().len(), 1);
    let b = Budget {
        draws: 0,
        ..Budget::default()
    };
    assert_eq!(
        prepare(
            Input {
                id: "z".into(),
                text: "hello".into()
            },
            &context(),
            &t,
            b,
            &mut FixedDraw(0)
        )
        .unwrap_err(),
        LanguageError::DrawBudget
    );
    let many = custom(
        (0..20)
            .map(|i| {
                (
                    UtilityKind::Thesaurus,
                    row(
                        &format!("r{i:02}"),
                        "hello",
                        UtilityEffect::Equivalent {
                            replacement: format!("word{}", char::from(b'a' + i)),
                            weight: 1.0,
                        },
                    ),
                )
            })
            .collect(),
    );
    let p = prepare(
        Input {
            id: "caps".into(),
            text: vec!["hello"; 16].join(";"),
        },
        &context(),
        &many,
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap();
    assert_eq!(
        p.clauses()
            .iter()
            .map(|c| c.variants().len())
            .sum::<usize>(),
        256
    );
    assert!(prep(&vec!["x"; 64].join(";")).is_ok());
}
#[test]
fn table_rows_are_validated_before_lookup_and_unicode_spans_survive() {
    let make = |r| {
        let mut s = UtilityKind::ALL
            .into_iter()
            .map(|kind| UtilitySnapshot {
                population: "synthetic".into(),
                version: "v1".into(),
                kind,
                rows: vec![],
            })
            .collect::<Vec<_>>();
        s[1].rows = r;
        UtilityTables::new("synthetic", "v1", s)
    };
    for weight in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(
            make(vec![row(
                "x",
                "hello",
                UtilityEffect::Equivalent {
                    replacement: "hi".into(),
                    weight
                }
            )])
            .is_err()
        );
    }
    assert!(
        make(vec![row(
            "x",
            "hello",
            UtilityEffect::Equivalent {
                replacement: "not".into(),
                weight: 1.0
            }
        )])
        .is_err()
    );
    assert!(
        make(vec![row(
            "x",
            "\"broken",
            UtilityEffect::Equivalent {
                replacement: "hi".into(),
                weight: 1.0
            }
        )])
        .is_err()
    );
    assert!(
        make(vec![
            row(
                "x",
                "hello",
                UtilityEffect::Equivalent {
                    replacement: "hi".into(),
                    weight: 1.0
                }
            ),
            row(
                "x",
                "world",
                UtilityEffect::Equivalent {
                    replacement: "earth".into(),
                    weight: 1.0
                }
            )
        ])
        .is_err()
    );
    assert!(
        make(
            (0..4097)
                .map(|i| row(
                    &format!("r{i}"),
                    "hello",
                    UtilityEffect::Equivalent {
                        replacement: "hi".into(),
                        weight: 1.0
                    }
                ))
                .collect()
        )
        .is_err()
    );
    let p = prep("Élan \"NOT 二\"").unwrap();
    let ts = p.clauses()[0].variants()[0].tokens();
    assert_eq!(ts[0].lemma(), "élan");
    assert_eq!(ts[1].span(), Span { start: 6, end: 15 });
    assert_eq!(&p.original()[6..15], "\"NOT 二\"");
}
#[test]
fn incomplete_conditionals_do_not_become_executable_plain_actions() {
    assert_eq!(prep("if rain").unwrap_err(), LanguageError::Syntax);
    assert_eq!(
        prep("if rain then if snow then hello").unwrap_err(),
        LanguageError::Syntax
    );
}
#[test]
fn utility_substitutions_cannot_introduce_custom_inhibitors_or_drop_numeric_frames() {
    let t = custom(vec![
        (
            UtilityKind::Thesaurus,
            row(
                "t",
                "hello",
                UtilityEffect::Equivalent {
                    replacement: "halt".into(),
                    weight: 1.0,
                },
            ),
        ),
        (
            UtilityKind::Negation,
            row("n", "halt", UtilityEffect::Negation { uncertain: true }),
        ),
    ]);
    assert_eq!(
        prepare(
            Input {
                id: "u".into(),
                text: "hello".into()
            },
            &context(),
            &t,
            Budget::default(),
            &mut FixedDraw(0)
        )
        .unwrap_err(),
        LanguageError::UnsafeSubstitution
    );
    let t = custom(vec![(
        UtilityKind::VerbFrames,
        row(
            "f",
            "count",
            UtilityEffect::Frame {
                subject: false,
                object: true,
                numeric_object: true,
            },
        ),
    )]);
    let p = prepare(
        Input {
            id: "u".into(),
            text: "count two; count \"two\"".into(),
        },
        &context(),
        &t,
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap();
    assert_eq!(
        p.clauses()[0].variants()[0].captures()[0]
            .decimal()
            .coefficient(),
        2
    );
    assert!(p.clauses()[0].variants()[0].executable());
    assert!(!p.clauses()[1].variants()[0].executable());
}
#[test]
fn compound_condition_requires_context_for_entire_antecedent() {
    let mut c = context();
    c.conditions.push(("wind".into(), true));
    let p = prepare(
        Input {
            id: "c".into(),
            text: "if rain and wind then hello".into(),
        },
        &c,
        &tables(),
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap();
    assert!(!p.clauses()[2].variants()[0].executable());
    assert!(
        matches!(p.ast(),SemanticAst::IfThen(cond,_) if matches!(cond.as_ref(),SemanticAst::And(..)))
    );
}
#[test]
fn repeated_utility_matches_do_not_allocate_unbounded_candidate_products() {
    let t = custom(vec![
        (
            UtilityKind::Thesaurus,
            row(
                "a",
                "x",
                UtilityEffect::Equivalent {
                    replacement: "alpha".into(),
                    weight: 1.0,
                },
            ),
        ),
        (
            UtilityKind::Thesaurus,
            row(
                "b",
                "x",
                UtilityEffect::Equivalent {
                    replacement: "beta".into(),
                    weight: 1.0,
                },
            ),
        ),
    ]);
    assert_eq!(
        prepare(
            Input {
                id: "work".into(),
                text: vec!["x"; 2050].join(" ")
            },
            &context(),
            &t,
            Budget::default(),
            &mut FixedDraw(0)
        )
        .unwrap_err(),
        LanguageError::StepBudget
    );
}

#[test]
fn review_duplicate_then_cannot_escape_unresolved_condition() {
    assert!(matches!(
        prep("if rain then hello then calculate 2+2"),
        Err(LanguageError::Syntax)
    ));
    assert!(matches!(
        prep("hello then calculate 2+2"),
        Err(LanguageError::Syntax)
    ));
}
fn review_rewrite_refused(text: &str, rows: Vec<(UtilityKind, UtilityRow)>) {
    let mut snapshots = UtilityKind::ALL
        .into_iter()
        .map(|kind| UtilitySnapshot {
            population: "synthetic".into(),
            version: "v1".into(),
            kind,
            rows: vec![],
        })
        .collect::<Vec<_>>();
    for (kind, row) in rows {
        snapshots
            .iter_mut()
            .find(|s| s.kind == kind)
            .unwrap()
            .rows
            .push(row);
    }
    match UtilityTables::new("synthetic", "v1", snapshots) {
        Err(LanguageError::InvalidTables) => {}
        Ok(t) => assert!(matches!(
            prepare(
                Input {
                    id: "review".into(),
                    text: text.into()
                },
                &context(),
                &t,
                Budget::default(),
                &mut FixedDraw(0)
            ),
            Err(LanguageError::UnsafeSubstitution)
        )),
        Err(e) => panic!("unexpected table error: {e}"),
    }
}
#[test]
fn review_builtin_contraction_rewrites_refused() {
    for replacement in ["can't", "dont", "don’t"] {
        review_rewrite_refused(
            "hello",
            vec![(
                UtilityKind::Thesaurus,
                row(
                    "e",
                    "hello",
                    UtilityEffect::Equivalent {
                        replacement: replacement.into(),
                        weight: 1.0,
                    },
                ),
            )],
        );
    }
}
#[test]
fn review_custom_multiword_rewrites_refused() {
    review_rewrite_refused(
        "hello",
        vec![
            (
                UtilityKind::Thesaurus,
                row(
                    "e",
                    "hello",
                    UtilityEffect::Equivalent {
                        replacement: "halt now".into(),
                        weight: 1.0,
                    },
                ),
            ),
            (
                UtilityKind::Negation,
                row(
                    "n",
                    "halt now",
                    UtilityEffect::Negation { uncertain: false },
                ),
            ),
        ],
    );
}
#[test]
fn review_dictionary_safety_introduction_refused() {
    review_rewrite_refused(
        "hello",
        vec![
            (
                UtilityKind::Dictionary,
                row(
                    "d",
                    "hello",
                    UtilityEffect::Dictionary {
                        lemma: "halt".into(),
                        definition: "stop".into(),
                    },
                ),
            ),
            (
                UtilityKind::Negation,
                row("n", "halt", UtilityEffect::Negation { uncertain: false }),
            ),
        ],
    );
}
#[test]
fn review_morphology_safety_introduction_refused() {
    review_rewrite_refused(
        "halts",
        vec![
            (
                UtilityKind::Morphology,
                row(
                    "m",
                    "s",
                    UtilityEffect::Suffix {
                        replacement: "".into(),
                    },
                ),
            ),
            (
                UtilityKind::Negation,
                row("n", "halt", UtilityEffect::Negation { uncertain: false }),
            ),
        ],
    );
}
#[test]
fn review_cross_boundary_safety_phrase_refused() {
    review_rewrite_refused(
        "hello now",
        vec![
            (
                UtilityKind::Thesaurus,
                row(
                    "e",
                    "hello",
                    UtilityEffect::Equivalent {
                        replacement: "halt".into(),
                        weight: 1.0,
                    },
                ),
            ),
            (
                UtilityKind::Negation,
                row(
                    "n",
                    "halt now",
                    UtilityEffect::Negation { uncertain: false },
                ),
            ),
        ],
    );
}
#[test]
fn review_grammar_resolved_slots_satisfy_required_frame() {
    let t = custom(vec![
        (
            UtilityKind::Grammar,
            row("s", "alice", UtilityEffect::Role(Role::Subject)),
        ),
        (
            UtilityKind::Grammar,
            row("o", "gift", UtilityEffect::Role(Role::Object)),
        ),
        (
            UtilityKind::VerbFrames,
            row(
                "f",
                "give",
                UtilityEffect::Frame {
                    subject: true,
                    object: true,
                    numeric_object: false,
                },
            ),
        ),
    ]);
    let p = prepare(
        Input {
            id: "review".into(),
            text: "alice give gift".into(),
        },
        &context(),
        &t,
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap();
    let a = &p.clauses()[0].variants()[0];
    assert_eq!(a.spo().subject(), &Slot::Resolved("alice".into()));
    assert_eq!(a.spo().object(), &Slot::Resolved("gift".into()));
    assert!(!a.unresolved_frame());
    assert!(a.executable());
}
#[test]
fn review_multiword_replacements_preserve_exact_original_source_slice() {
    for (source, phrase) in [
        ("red\tfox", "red fox"),
        ("red   fox", "red fox"),
        ("éclat\t\t狐", "éclat 狐"),
    ] {
        let t = custom(vec![(
            UtilityKind::Thesaurus,
            row(
                "e",
                phrase,
                UtilityEffect::Equivalent {
                    replacement: "vulpine".into(),
                    weight: 1.0,
                },
            ),
        )]);
        let p = prepare(
            Input {
                id: "review".into(),
                text: source.into(),
            },
            &context(),
            &t,
            Budget::default(),
            &mut FixedDraw(0),
        )
        .unwrap();
        let token = &p.clauses()[0].variants()[1].tokens()[0];
        assert_eq!(
            token.source(),
            &p.original()[token.span().start..token.span().end]
        );
        assert_eq!(token.source(), source);
    }
}
