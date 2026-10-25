use specimen_kernel::language_v3::{script::*, *};
fn registry(revision: u64, parent_reads: Vec<&str>) -> Registry {
    let scopes = vec![
        Scope::NodeAction,
        Scope::PatternBind,
        Scope::RelationalTriple,
        Scope::Vote,
        Scope::Orchestration,
    ];
    Registry::new(
        "test-registry",
        revision,
        vec!["resource".into(), "memory".into()],
        vec![
            Declaration {
                name: "Text".into(),
                aliases: vec!["literal".into()],
                scopes: scopes.clone(),
                reads: vec![],
                handler: Handler::Text,
            },
            Declaration {
                name: "Math".into(),
                aliases: vec!["math".into()],
                scopes: scopes.clone(),
                reads: vec![],
                handler: Handler::Math,
            },
            Declaration {
                name: "Tape".into(),
                aliases: vec![],
                scopes: scopes.clone(),
                reads: vec![],
                handler: Handler::Tape,
            },
            Declaration {
                name: "LookUp".into(),
                aliases: vec![],
                scopes: scopes.clone(),
                reads: vec!["resource".into(), "memory".into()],
                handler: Handler::LookUp,
            },
            Declaration {
                name: "Parent".into(),
                aliases: vec![],
                scopes: scopes.clone(),
                reads: parent_reads.into_iter().map(str::to_string).collect(),
                handler: Handler::Concat,
            },
            Declaration {
                name: "Bind".into(),
                aliases: vec![],
                scopes,
                reads: vec![],
                handler: Handler::Bind,
            },
        ],
    )
    .unwrap()
}
fn execute(s: &str) -> Result<EffectPlan, LanguageError> {
    let r = registry(1, vec!["resource", "memory"]);
    let ast = compile(s, Scope::NodeAction, &r)?;
    let caps = r.capabilities(vec!["resource".into(), "memory".into()])?;
    execute_script(&ast, &r, &caps, ScriptBudget::default())
}
#[test]
fn scripts_math_pratt_and_literal_quotes_execute_once() {
    assert_eq!(
        execute("math(\"-2^2+2^3^2\")").unwrap().value(),
        &Value::Number(508.0)
    );
    let p = execute("Bind(\"2+2\")").unwrap();
    assert_eq!(p.value(), &Value::Number(4.0));
    assert_eq!(
        p.trace()
            .iter()
            .filter(|t| matches!(t, Trace::BindComputed))
            .count(),
        1
    );
    assert_eq!(
        p.trace()
            .iter()
            .filter(|t| matches!(t, Trace::BindExecuted))
            .count(),
        1
    );
    assert_eq!(
        execute("Text(\"LookUp(unknown,key)\")").unwrap().effects(),
        &[]
    );
    assert_eq!(
        execute("Math(\"1/0\")").unwrap_err(),
        LanguageError::DivisionByZero
    );
    assert_eq!(
        execute("Math(Text(\"2+2\"))").unwrap_err(),
        LanguageError::TypeMismatch
    );
}
#[test]
fn s21_actual_tape_execution_and_atomic_failures() {
    assert_eq!(
        execute("Tape(\"++++++++[>++++++++<-]>+.\",\"\")")
            .unwrap()
            .value(),
        &Value::Bytes(vec![65])
    );
    assert_eq!(
        execute("Tape(\",.,.\",\"é\")").unwrap().value(),
        &Value::Bytes(vec![195, 169])
    );
    assert_eq!(
        execute("Tape(\"<-.\",\"\")").unwrap().value(),
        &Value::Bytes(vec![255])
    );
    assert_eq!(
        execute("Tape(\"+[]\",\"\")").unwrap_err(),
        LanguageError::StepBudget
    );
    assert_eq!(
        execute("Tape(\"[\",\"\")").unwrap_err(),
        LanguageError::TapeSyntax
    );
    assert_eq!(
        execute("Tape(\"x\",\"\")").unwrap_err(),
        LanguageError::TapeSyntax
    );
    let s = format!("Tape(\"{}\",\"\")", ".".repeat(1025));
    assert_eq!(execute(&s).unwrap_err(), LanguageError::OutputBudget);
    let s = format!("Tape(\"{}{}\",\"\")", "[".repeat(17), "]".repeat(17));
    assert_eq!(execute(&s).unwrap_err(), LanguageError::DepthLimit);
}
#[test]
fn s46_nested_contacts_narrow_parent_and_current_registry() {
    let r = registry(1, vec!["resource"]);
    let caps = r
        .capabilities(vec!["resource".into(), "memory".into()])
        .unwrap();
    let a = compile("Parent(LookUp(\"memory\",\"key\"))", Scope::NodeAction, &r).unwrap();
    assert_eq!(
        execute_script(&a, &r, &caps, ScriptBudget::default()).unwrap_err(),
        LanguageError::UndeclaredContact
    );
    let a = compile(
        "Parent(LookUp(\"resource\",\"key\"))",
        Scope::NodeAction,
        &r,
    )
    .unwrap();
    assert_eq!(
        execute_script(&a, &r, &caps, ScriptBudget::default())
            .unwrap()
            .effects(),
        &[Effect::Read {
            subsystem: "resource".into(),
            key: "key".into()
        }]
    );
    let newer = registry(2, vec!["resource"]);
    assert_eq!(
        execute_script(&a, &newer, &caps, ScriptBudget::default()).unwrap_err(),
        LanguageError::StaleRegistry
    );
    assert_eq!(
        compile("LookUp(\"unknown\",\"x\")", Scope::NodeAction, &r).unwrap_err(),
        LanguageError::UndeclaredContact
    );
    assert_eq!(
        compile("LookUp(Text(\"resource\"),\"x\")", Scope::NodeAction, &r).unwrap_err(),
        LanguageError::DynamicContact
    );
    assert_eq!(
        compile("Math(\"2+2\")", Scope::Reserved, &r).unwrap_err(),
        LanguageError::InvalidScope
    );
    let a = compile(
        "Parent(LookUp(\"resource\",\"x\"),Math(\"1/0\"))",
        Scope::NodeAction,
        &r,
    )
    .unwrap();
    assert_eq!(
        execute_script(&a, &r, &caps, ScriptBudget::default()).unwrap_err(),
        LanguageError::DivisionByZero
    );
}
#[test]
fn execute_for_action_preserves_preparation_guards() {
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
    let c = ContextSnapshot {
        population: "p".into(),
        utility_version: "v".into(),
        tokens: vec![],
        conditions: vec![],
    };
    let p = prepare(
        Input {
            id: "i".into(),
            text: "don't calculate 2+2; calculate 2+2".into(),
        },
        &c,
        &t,
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap();
    let r = registry(1, vec![]);
    let caps = r.capabilities(vec![]).unwrap();
    let ast = compile("Math(\"2+2\")", Scope::NodeAction, &r).unwrap();
    assert_eq!(
        execute_for_action(
            &ast,
            &p.clauses()[0].variants()[0],
            &r,
            &caps,
            ScriptBudget::default()
        )
        .unwrap_err(),
        LanguageError::ActionBlocked
    );
    assert_eq!(
        execute_for_action(
            &ast,
            &p.clauses()[1].variants()[0],
            &r,
            &caps,
            ScriptBudget::default()
        )
        .unwrap()
        .value(),
        &Value::Number(4.0)
    );
}
#[test]
fn registry_content_binding_and_aggregate_budgets_cannot_be_bypassed() {
    let r = registry(1, vec!["resource"]);
    let caps = r.capabilities(vec!["resource".into()]).unwrap();
    let a = compile("Parent(Bind(\"2+2\"),Bind(\"3+3\"))", Scope::NodeAction, &r).unwrap();
    let changed = registry(1, vec!["resource", "memory"]);
    assert_eq!(
        execute_script(&a, &changed, &caps, ScriptBudget::default()).unwrap_err(),
        LanguageError::StaleRegistry
    );
    let b = ScriptBudget {
        arithmetic_steps: 4,
        ..ScriptBudget::default()
    };
    assert_eq!(
        execute_script(&a, &r, &caps, b).unwrap_err(),
        LanguageError::StepBudget
    );
    let a = compile(
        "Parent(Tape(\"+++\",\"\"),Tape(\"+++\",\"\"))",
        Scope::NodeAction,
        &r,
    )
    .unwrap();
    let b = ScriptBudget {
        tape_steps: 5,
        ..ScriptBudget::default()
    };
    assert_eq!(
        execute_script(&a, &r, &caps, b).unwrap_err(),
        LanguageError::StepBudget
    );
    let a = compile("Text(\"hello\")", Scope::NodeAction, &r).unwrap();
    let b = ScriptBudget {
        output_bytes: 4,
        ..ScriptBudget::default()
    };
    assert_eq!(
        execute_script(&a, &r, &caps, b).unwrap_err(),
        LanguageError::OutputBudget
    );
    for scope in [
        Scope::RelationalTriple,
        Scope::PatternBind,
        Scope::NodeAction,
        Scope::Vote,
        Scope::Orchestration,
    ] {
        assert!(compile("Math(\"1+1\")", scope, &r).is_ok());
    }
    assert!(r.capabilities(vec!["other".into()]).is_err());
    let nested = format!("{}\"x\"{}", "Text(".repeat(17), ")".repeat(17));
    assert_eq!(
        compile(&nested, Scope::NodeAction, &r).unwrap_err(),
        LanguageError::DepthLimit
    );
}
#[test]
fn root_quoted_literal_obeys_output_budget() {
    let r = registry(1, vec![]);
    let caps = r.capabilities(vec![]).unwrap();
    let ast = compile("\"hello\"", Scope::NodeAction, &r).unwrap();
    let b = ScriptBudget {
        output_bytes: 4,
        ..ScriptBudget::default()
    };
    assert_eq!(
        execute_script(&ast, &r, &caps, b).unwrap_err(),
        LanguageError::OutputBudget
    );
}

#[test]
fn review_mixed_sequence_output_bytes_are_counted_recursively() {
    let r = registry(1, vec![]);
    let caps = r.capabilities(vec![]).unwrap();
    for source in [
        "Parent(\"hello\",Tape(\"\",\"\"))",
        "Parent(Parent(\"hello\",Tape(\"\",\"\")),Tape(\"\",\"\"))",
    ] {
        let ast = compile(source, Scope::NodeAction, &r).unwrap();
        assert!(matches!(
            execute_script(
                &ast,
                &r,
                &caps,
                ScriptBudget {
                    output_bytes: 1,
                    ..ScriptBudget::default()
                }
            ),
            Err(LanguageError::OutputBudget)
        ));
        assert!(
            execute_script(
                &ast,
                &r,
                &caps,
                ScriptBudget {
                    output_bytes: 5,
                    ..ScriptBudget::default()
                }
            )
            .is_ok()
        );
    }
    let ast = compile("Parent(\"é\",Tape(\"+.\",\"\"))", Scope::NodeAction, &r).unwrap();
    assert!(matches!(
        execute_script(
            &ast,
            &r,
            &caps,
            ScriptBudget {
                output_bytes: 2,
                ..ScriptBudget::default()
            }
        ),
        Err(LanguageError::OutputBudget)
    ));
    assert!(
        execute_script(
            &ast,
            &r,
            &caps,
            ScriptBudget {
                output_bytes: 3,
                ..ScriptBudget::default()
            }
        )
        .is_ok()
    );
}
#[test]
fn review_exact_arithmetic_exhaustion_is_runtime_step_budget() {
    let r = registry(1, vec![]);
    let caps = r.capabilities(vec![]).unwrap();
    let ast = compile("Parent(Math(\"1\"),Math(\"1\"))", Scope::NodeAction, &r).unwrap();
    assert!(matches!(
        execute_script(
            &ast,
            &r,
            &caps,
            ScriptBudget {
                arithmetic_steps: 1,
                ..ScriptBudget::default()
            }
        ),
        Err(LanguageError::StepBudget)
    ));
}
