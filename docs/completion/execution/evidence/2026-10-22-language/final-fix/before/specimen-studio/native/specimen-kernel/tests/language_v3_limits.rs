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
fn unicode_bounds_invalid_tables_and_variant_budgets_fail_deterministically() {
    assert!(prep(&"é".repeat(8000)).is_ok());
    assert_eq!(
        prep(&"é".repeat(8001)).unwrap_err(),
        LanguageError::InputLimit
    );
    assert_eq!(
        prep(&vec!["x"; 65].join(";")).unwrap_err(),
        LanguageError::ClauseLimit
    );
    assert_eq!(
        prep(&format!("{}1{}", "(".repeat(17), ")".repeat(17))).unwrap_err(),
        LanguageError::DepthLimit
    );
    let mut ss = UtilityKind::ALL
        .into_iter()
        .map(|kind| UtilitySnapshot {
            population: "synthetic".into(),
            version: "v1".into(),
            kind,
            rows: vec![],
        })
        .collect::<Vec<_>>();
    ss[0].version = "bad".into();
    assert!(UtilityTables::new("synthetic", "v1", ss).is_err());
    let t = custom(vec![(
        UtilityKind::Thesaurus,
        row(
            "t",
            "hello",
            UtilityEffect::Equivalent {
                replacement: "hi".into(),
                weight: 1.0,
            },
        ),
    )]);
    let b = Budget {
        variants_per_clause: 17,
        ..Budget::default()
    };
    assert_eq!(
        prepare(
            Input {
                id: "x".into(),
                text: "hello".into()
            },
            &context(),
            &t,
            b,
            &mut FixedDraw(0)
        )
        .unwrap_err(),
        LanguageError::InvalidBudget
    );
    let b = Budget {
        total_variants: 1,
        ..Budget::default()
    };
    assert_eq!(
        prepare(
            Input {
                id: "x".into(),
                text: "hello".into()
            },
            &context(),
            &t,
            b,
            &mut FixedDraw(0)
        )
        .unwrap_err(),
        LanguageError::VariantLimit
    );
}

#[test]
fn shared_rewrite_work_budget_is_not_reset_between_clauses() {
    let t = custom(vec![
        (
            UtilityKind::Dictionary,
            row(
                "d",
                "x",
                UtilityEffect::Dictionary {
                    lemma: "alpha".into(),
                    definition: "synthetic letter".into(),
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
    ]);
    let budget = Budget {
        work_steps: 64,
        ..Budget::default()
    };
    let run = |text: &str| {
        prepare(
            Input {
                id: "bounded-work".into(),
                text: text.into(),
            },
            &context(),
            &t,
            budget,
            &mut FixedDraw(0),
        )
    };
    assert!(run("x").is_ok());
    assert!(matches!(run("x;x"), Err(LanguageError::StepBudget)));
    assert!(matches!(run("x;x;x;x"), Err(LanguageError::StepBudget)));
}

#[test]
fn rewrite_work_allowance_is_validated_before_preparation() {
    for work_steps in [0, 1_000_001] {
        assert!(matches!(
            prepare(
                Input {
                    id: "work-limit".into(),
                    text: "hello".into()
                },
                &context(),
                &tables(),
                Budget {
                    work_steps,
                    ..Budget::default()
                },
                &mut FixedDraw(0)
            ),
            Err(LanguageError::InvalidBudget)
        ));
    }
}
