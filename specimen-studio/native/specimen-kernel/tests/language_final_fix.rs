use specimen_kernel::{
    language_v3::{script::*, *},
    text_v3,
};
fn tables(rows: Vec<(UtilityKind, &str, UtilityEffect)>) -> Result<UtilityTables, LanguageError> {
    UtilityTables::new(
        "p",
        "v",
        UtilityKind::ALL
            .into_iter()
            .map(|kind| UtilitySnapshot {
                population: "p".into(),
                version: "v".into(),
                kind,
                rows: rows
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| r.0 == kind)
                    .map(|(i, r)| UtilityRow {
                        reference: format!("r{i}"),
                        phrase: r.1.into(),
                        effect: r.2.clone(),
                    })
                    .collect(),
            })
            .collect(),
    )
}
fn prepared(text: &str, condition: &str, t: &UtilityTables) -> Prepared {
    prepare(
        Input {
            id: "final-fix".into(),
            text: text.into(),
        },
        &ContextSnapshot {
            population: "p".into(),
            utility_version: "v".into(),
            tokens: vec![],
            conditions: vec![(condition.into(), true)],
        },
        t,
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap()
}
fn registry(name: &str, alias: &str) -> Registry {
    Registry::new(
        "test-registry",
        1,
        vec![],
        vec![Declaration {
            name: name.into(),
            aliases: vec![alias.into()],
            scopes: vec![Scope::NodeAction],
            reads: vec![],
            handler: Handler::Text,
        }],
    )
    .unwrap()
}
#[test]
fn quote_opaque_full_conditions_and_execution_guard() {
    let t = tables(vec![]).unwrap();
    let r = registry("Text", "text");
    let s = compile("Text(\"effect\")", Scope::NodeAction, &r).unwrap();
    let caps = r.capabilities(vec![]).unwrap();
    for (antecedent, attestation, yes) in [
        ("\"ALLOW\"", "\"allow\"", false),
        ("\"Ä\"", "\"ä\"", false),
        ("\"é\"", "\"é\"", false),
        ("\"ALLOW\"", "\"ALLOW\"", true),
        ("RAIN", "rain", true),
        ("rain and \"ALLOW\"", "rain", false),
        ("RAIN and \"ALLOW\"", "rain and \"ALLOW\"", true),
        (r#""a\"B""#, r#""a\"b""#, false),
        (r#""a\"B""#, r#""a\"B""#, true),
        ("don't rain", "DON'T RAIN", true),
    ] {
        let p = prepared(&format!("if {antecedent} then go"), attestation, &t);
        let a = &p.clauses().last().unwrap().variants()[0];
        assert_eq!(a.executable(), yes, "{antecedent} / {attestation}");
        assert_eq!(
            a.conditional(),
            if yes {
                ConditionalDisposition::Satisfied
            } else {
                ConditionalDisposition::Unresolved
            }
        );
        assert_eq!(
            execute_for_action(&s, a, &r, &caps, ScriptBudget::default()).is_ok(),
            yes
        );
    }
}
#[test]
fn contradictory_control_tables_refuse_before_any_prepared_action() {
    let cases = vec![
        vec![(
            UtilityKind::Grammar,
            "halt",
            UtilityEffect::Role(Role::Negation),
        )],
        vec![(
            UtilityKind::Grammar,
            "halt",
            UtilityEffect::Role(Role::Connective),
        )],
        vec![(
            UtilityKind::Conjunction,
            "don't",
            UtilityEffect::Conjunction(Connective::And),
        )],
        vec![(
            UtilityKind::Conjunction,
            "please don't",
            UtilityEffect::Conjunction(Connective::And),
        )],
        vec![(
            UtilityKind::Conjunction,
            "don't please",
            UtilityEffect::Conjunction(Connective::And),
        )],
        vec![
            (
                UtilityKind::Conjunction,
                "halt now",
                UtilityEffect::Conjunction(Connective::And),
            ),
            (
                UtilityKind::Negation,
                "halt",
                UtilityEffect::Negation { uncertain: false },
            ),
        ],
        vec![
            (
                UtilityKind::Conjunction,
                "go halt",
                UtilityEffect::Conjunction(Connective::And),
            ),
            (
                UtilityKind::Negation,
                "halt now",
                UtilityEffect::Negation { uncertain: false },
            ),
        ],
        vec![
            (
                UtilityKind::Conjunction,
                "now go",
                UtilityEffect::Conjunction(Connective::And),
            ),
            (
                UtilityKind::Negation,
                "halt now",
                UtilityEffect::Negation { uncertain: false },
            ),
        ],
        vec![(
            UtilityKind::Conjunction,
            "if",
            UtilityEffect::Conjunction(Connective::And),
        )],
        vec![(
            UtilityKind::Conjunction,
            "then",
            UtilityEffect::Conjunction(Connective::Or),
        )],
        vec![(
            UtilityKind::Negation,
            "if",
            UtilityEffect::Negation { uncertain: false },
        )],
        vec![(
            UtilityKind::Negation,
            "halt and",
            UtilityEffect::Negation { uncertain: false },
        )],
        vec![(
            UtilityKind::Grammar,
            "then",
            UtilityEffect::Role(Role::Predicate),
        )],
    ];
    for rows in cases {
        assert!(matches!(tables(rows), Err(LanguageError::InvalidTables)));
    }
    // A valid custom negator remains guarded; quotes are inert.
    let t = tables(vec![(
        UtilityKind::Negation,
        "halt now",
        UtilityEffect::Negation { uncertain: false },
    )])
    .unwrap();
    let r = registry("Text", "text");
    let s = compile("Text(\"effect\")", Scope::NodeAction, &r).unwrap();
    let caps = r.capabilities(vec![]).unwrap();
    for (text, yes) in [
        ("halt now go", false),
        ("please don't go", false),
        ("\"halt now\" go", true),
    ] {
        let p = prepared(text, "", &t);
        let a = &p.clauses()[0].variants()[0];
        assert_eq!(a.executable(), yes);
        assert_eq!(
            execute_for_action(&s, a, &r, &caps, ScriptBudget::default()).is_ok(),
            yes
        );
    }
}
#[test]
fn complexity_getter_matches_text_profile() {
    let t = tables(vec![]).unwrap();
    for text in [
        "alpha, beta gamma delta epsilon zeta",
        "\"x,y\" + 2",
        "not hello",
        "if rain and snow then go",
    ] {
        let p = prepared(text, "", &t);
        for c in p.clauses() {
            for a in c.variants() {
                assert_eq!(
                    a.complexity_token_count(),
                    text_v3::encode(a).unwrap().token_count(),
                    "{text}"
                );
            }
        }
    }
}
#[test]
fn registry_call_names_and_aliases_compile() {
    for (name, alias) in [
        ("my-text", "alias-text"),
        ("Text_2", "two_2"),
        ("2Text", "2alias"),
    ] {
        let r = registry(name, alias);
        let caps = r.capabilities(vec![]).unwrap();
        for n in [name, alias] {
            let s = compile(&format!("{n}(\"x\")"), Scope::NodeAction, &r).unwrap();
            assert_eq!(
                execute_script(&s, &r, &caps, ScriptBudget::default())
                    .unwrap()
                    .value(),
                &Value::Text("x".into())
            );
        }
    }
}
