use specimen_kernel::language_v3::*;

fn prep(text: &str, numeric_frame: bool) -> Result<Prepared, LanguageError> {
    let tables = UtilityTables::new(
        "numeric-synthetic",
        "v1",
        UtilityKind::ALL
            .into_iter()
            .map(|kind| UtilitySnapshot {
                population: "numeric-synthetic".into(),
                version: "v1".into(),
                kind,
                rows: if numeric_frame && kind == UtilityKind::VerbFrames {
                    vec![UtilityRow {
                        reference: "count-numeric-object".into(),
                        phrase: "count".into(),
                        effect: UtilityEffect::Frame {
                            subject: false,
                            object: true,
                            numeric_object: true,
                        },
                    }]
                } else {
                    vec![]
                },
            })
            .collect(),
    )
    .unwrap();
    prepare(
        Input {
            id: "numeric-input".into(),
            text: text.into(),
        },
        &ContextSnapshot {
            population: "numeric-synthetic".into(),
            utility_version: "v1".into(),
            tokens: vec![],
            conditions: vec![],
        },
        &tables,
        Budget::default(),
        &mut FixedDraw(0),
    )
}

#[test]
fn seven_ordinary_number_words_remain_lexical_without_numeric_evidence() {
    let source = "one two three four five six seven";
    let prepared = prep(source, false).unwrap();
    let action = &prepared.clauses()[0].variants()[0];
    assert_eq!(prepared.original(), source);
    assert_eq!(action.original(), source);
    assert_eq!(
        action.tokens().iter().map(Token::lemma).collect::<Vec<_>>(),
        ["one", "two", "three", "four", "five", "six", "seven"]
    );
    assert!(
        action
            .tokens()
            .iter()
            .all(|token| { token.kind() == TokenKind::Word && token.decimal().is_none() })
    );
    assert!(action.captures().is_empty());
    assert!(action.operators().is_empty());
    assert_eq!(action.arithmetic_value(), None);
    assert!(!action.routes().contains(&RouteHint::Arithmetic));
}

#[test]
fn single_and_adjacent_number_words_need_arithmetic_intent_or_numeric_frame() {
    for source in ["zero", "one", "ten", "one two", "two three four"] {
        let prepared = prep(source, false).unwrap();
        let action = &prepared.clauses()[0].variants()[0];
        assert!(
            action
                .tokens()
                .iter()
                .all(|token| { token.kind() == TokenKind::Word && token.decimal().is_none() }),
            "{source}"
        );
        assert!(action.captures().is_empty(), "{source}");
        assert!(action.operators().is_empty(), "{source}");
        assert_eq!(action.arithmetic_value(), None, "{source}");
    }
}

#[test]
fn explicit_word_arithmetic_keeps_operand_order_operators_and_value() {
    for source in ["two plus two", "calculate two plus two", "two + two"] {
        let prepared = prep(source, false).unwrap();
        let action = &prepared.clauses()[0].variants()[0];
        assert_eq!(action.arithmetic_value(), Some(4.0));
        assert_eq!(action.operators(), ["+"]);
        assert_eq!(
            action
                .captures()
                .iter()
                .map(|capture| (
                    capture.source(),
                    capture.decimal().coefficient(),
                    capture.decimal().scale()
                ))
                .collect::<Vec<_>>(),
            [("two", 2, 0), ("two", 2, 0)]
        );
        for token in action.tokens() {
            assert_eq!(
                token.source(),
                &source[token.span().start..token.span().end]
            );
        }
    }
}

#[test]
fn explicit_numeric_frame_captures_number_word_but_keeps_quotes_opaque() {
    let prepared = prep("count two; count \"two\"", true).unwrap();
    let numeric = &prepared.clauses()[0].variants()[0];
    assert_eq!(numeric.captures().len(), 1);
    assert_eq!(numeric.captures()[0].source(), "two");
    assert_eq!(numeric.captures()[0].decimal().coefficient(), 2);
    assert_eq!(numeric.arithmetic_value(), None);
    assert!(numeric.executable());
    let quoted = &prepared.clauses()[1].variants()[0];
    assert!(quoted.captures().is_empty());
    assert!(quoted.tokens()[1].quoted());
    assert!(!quoted.executable());
}

#[test]
fn malformed_explicit_arithmetic_still_returns_typed_failure() {
    for source in [
        "two plus",
        "two plus two three",
        "two + * two",
        "(two plus two",
        "calculate one two",
    ] {
        assert_eq!(
            prep(source, false).unwrap_err(),
            LanguageError::Syntax,
            "{source}"
        );
    }
    assert_eq!(
        prep("two divided zero", false).unwrap_err(),
        LanguageError::DivisionByZero
    );
    // The direct math API and existing command prefix already express numeric intent.
    assert_eq!(evaluate_math("two", 500).unwrap(), 2.0);
    assert_eq!(
        prep("calculate two", false).unwrap().clauses()[0].variants()[0].arithmetic_value(),
        Some(2.0)
    );
}

#[test]
fn quotes_negation_and_utf8_source_bytes_survive_numeric_recognition() {
    let source = "\"é One plus TWO\"; don't calculate two plus two; One\tTwo";
    let prepared = prep(source, false).unwrap();
    assert_eq!(prepared.original(), source);
    let quoted = &prepared.clauses()[0].variants()[0];
    assert!(quoted.tokens()[0].quoted());
    assert_eq!(quoted.tokens()[0].lemma(), "é One plus TWO");
    assert!(quoted.captures().is_empty());
    let blocked = &prepared.clauses()[1].variants()[0];
    assert!(blocked.inhibited());
    assert!(!blocked.executable());
    assert_eq!(blocked.arithmetic_value(), Some(4.0));
    let ordinary = &prepared.clauses()[2].variants()[0];
    assert_eq!(
        ordinary
            .tokens()
            .iter()
            .map(Token::lemma)
            .collect::<Vec<_>>(),
        ["one", "two"]
    );
    assert!(ordinary.captures().is_empty());
    for clause in prepared.clauses() {
        for token in clause.variants()[0].tokens() {
            assert_eq!(
                token.source(),
                &source[token.span().start..token.span().end]
            );
        }
    }
}
