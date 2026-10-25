use specimen_kernel::{language_v3::*, scanner_v3::*, text_v3};
fn prep(text: &str) -> Prepared {
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
            id: "input".into(),
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
fn action(p: &Prepared) -> &PreparedAction {
    &p.clauses()[0].variants()[0]
}
fn r(n: i128, d: i128) -> Rational {
    Rational::new(n, d).unwrap()
}
#[test]
fn s06_direct_checked_i128_reducer_subtracts_before_single_clamp() {
    for big in [i64::MAX as i128 + 999, 1_i128 << 100] {
        for delta in [-150, -50, 0, 50, 150] {
            let evidence = reduce_totals(r(big + delta, 1), r(big, 1)).unwrap();
            assert_eq!(evidence.difference, r(delta, 1));
            assert_eq!(evidence.confidence, r(delta.clamp(-100, 100), 1));
            assert_eq!(evidence.similarity, r(big + delta, 1));
        }
    }
    assert_eq!(
        reduce_totals(r(1001, 3), r(1000, 3)).unwrap().confidence,
        r(1, 3)
    );
    assert_eq!(r(-7, 3).trunc(), -2);
    assert_eq!(r(7, 3).trunc(), 2);
    assert_eq!(r(2, 6), r(1, 3));
    assert_eq!(
        r(i128::MAX, 1).checked_add(r(1, 1)),
        Err(ScanError::Overflow)
    );
    assert_eq!(
        reduce_totals(r(i128::MIN, 1), r(1, 1)),
        Err(ScanError::Overflow)
    );
    assert_eq!(
        r(i128::MIN, 1).checked_sub(r(i128::MIN, 1)).unwrap(),
        r(0, 1)
    );
    assert_eq!(r(-1, 3).checked_add(r(1, 6)).unwrap(), r(-1, 6));
    assert_eq!(
        reduce_totals(r(-(1_i128 << 100) + 25, 1), r(-(1_i128 << 100), 1))
            .unwrap()
            .confidence,
        r(25, 1)
    );
    assert_eq!(Rational::new(1, 0), Err(ScanError::InvalidDenominator));
    assert_eq!(Rational::new(1, -1), Err(ScanError::InvalidDenominator));
}
#[test]
fn profile_absence_consequences_are_visible_and_modality_is_explicit() {
    for (text, score) in [("red blue", 20), ("opaque", -20)] {
        let p = prep(text);
        let e = scan(action(&p), action(&p)).unwrap();
        assert_eq!(e.totals.confidence, r(score, 1));
        assert_eq!(e.components.map(|c| c.weight), [40, 20, 20, 20]);
        assert_eq!(e.version, "text-scanner-v3");
    }
    let p = prep("opaque");
    assert_eq!(
        scan_modality("image", action(&p), action(&p)),
        Err(ScanError::UnsupportedModality)
    );
}
#[test]
fn s05_same_low_key_scans_distinct_original_numeric_evidence() {
    let a = prep("2+3");
    let b = prep("8+9");
    assert_eq!(
        text_v3::encode(action(&a)).unwrap().pattern_key(),
        text_v3::encode(action(&b)).unwrap().pattern_key()
    );
    let same = scan(action(&a), action(&a)).unwrap();
    let different = scan(action(&a), action(&b)).unwrap();
    assert_eq!(same.components[3].similarity, r(20, 1));
    assert_eq!(different.components[3].similarity, r(0, 1));
    assert_eq!(different.components[3].reason, Reason::Mismatch);
    assert_ne!(same.totals.confidence, different.totals.confidence);
    assert!(!different.components[3].left_spans.is_empty());
}
#[test]
fn multiset_repeats_bigrams_operand_order_and_exact_decimals() {
    let a = prep("red red blue");
    let b = prep("red blue blue");
    let e = scan(action(&a), action(&b)).unwrap();
    assert_eq!(e.components[0].similarity, r(20, 1));
    assert_eq!(e.components[1].similarity, r(20, 3));
    let c = prep("blue red red");
    assert_ne!(
        scan(action(&a), action(&c)).unwrap().components[1].similarity,
        r(20, 1)
    );
    let a = prep("1.20+0.3");
    let b = prep("1.2+.30");
    let c = prep(".3+1.2");
    assert_eq!(
        scan(action(&a), action(&b)).unwrap().components[3].similarity,
        r(20, 1)
    );
    assert_eq!(
        scan(action(&a), action(&c)).unwrap().components[3].similarity,
        r(0, 1)
    );
    let d = prep("1.20000000000000001+.3");
    assert_eq!(
        scan(action(&a), action(&d)).unwrap().components[3].similarity,
        r(0, 1)
    );
}
#[test]
fn resolved_spo_thirds_and_negation_conflict_are_independent_components() {
    fn roles(text: &str) -> Prepared {
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
            .find(|s| s.kind == UtilityKind::Grammar)
            .unwrap()
            .rows = vec![
            UtilityRow {
                reference: "subject".into(),
                phrase: "alice".into(),
                effect: UtilityEffect::Role(Role::Subject),
            },
            UtilityRow {
                reference: "predicate".into(),
                phrase: "sees".into(),
                effect: UtilityEffect::Role(Role::Predicate),
            },
            UtilityRow {
                reference: "object".into(),
                phrase: "bird".into(),
                effect: UtilityEffect::Role(Role::Object),
            },
        ];
        let tables = UtilityTables::new("p", "v", snapshots).unwrap();
        prepare(
            Input {
                id: "roles".into(),
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
    let a = roles("alice sees bird");
    let b = roles("alice sees stone");
    let n = roles("not alice sees bird");
    assert_eq!(
        scan(action(&a), action(&a)).unwrap().components[2].similarity,
        r(20, 1)
    );
    assert_eq!(
        scan(action(&a), action(&b)).unwrap().components[2].similarity,
        r(40, 3)
    );
    let conflict = scan(action(&a), action(&n)).unwrap();
    assert_eq!(conflict.components[2].similarity, r(0, 1));
    assert_eq!(conflict.components[2].dissimilarity, r(20, 1));
    assert_eq!(conflict.components[2].reason, Reason::NegationConflict);
    assert_eq!(
        conflict
            .totals
            .similarity
            .checked_add(conflict.totals.dissimilarity)
            .unwrap(),
        r(100, 1)
    );
}
#[test]
fn conditional_antecedent_leaves_are_scanned_once_without_inventing_roles() {
    let rain = prep("if rain then hello");
    let sun = prep("if sun then hello");
    let a = &rain.clauses()[1].variants()[0];
    let b = &sun.clauses()[1].variants()[0];
    let same = scan(a, a).unwrap();
    let different = scan(a, b).unwrap();
    assert_eq!(different.components[0].similarity, r(40, 3));
    assert_eq!(different.components[1].similarity, r(0, 1));
    assert_eq!(same.components[0].similarity, r(40, 1));
    assert_eq!(different.components[0].left_spans.len(), 2);
    assert_eq!(different.components[2].similarity, r(0, 1));
    assert_eq!(different.components[3].similarity, r(0, 1));
    assert_ne!(same.totals.confidence, different.totals.confidence);
}
#[test]
fn punctuation_is_excluded_from_lexical_scanner_features() {
    let plain = prep("red blue");
    let punctuated = prep("red, blue");
    let e = scan(action(&plain), action(&punctuated)).unwrap();
    assert_eq!(e.components[0].similarity, r(40, 1));
    assert_eq!(e.components[1].similarity, r(20, 1));
}

fn conditional_roles(text: &str) -> Prepared {
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
        .find(|s| s.kind == UtilityKind::Grammar)
        .unwrap()
        .rows = [
        ("alice", Role::Subject),
        ("sees", Role::Predicate),
        ("bird", Role::Object),
    ]
    .into_iter()
    .map(|(phrase, role)| UtilityRow {
        reference: phrase.into(),
        phrase: phrase.into(),
        effect: UtilityEffect::Role(role),
    })
    .collect();
    let tables = UtilityTables::new("p", "v", snapshots).unwrap();
    prepare(
        Input {
            id: "conditional-roles".into(),
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
fn consequent(p: &Prepared) -> &PreparedAction {
    &p.clauses().last().unwrap().variants()[0]
}
#[test]
fn shared_negated_antecedent_cannot_hide_opposite_body_spo_polarity() {
    let positive = conditional_roles("if not rain then alice sees bird");
    let negative = conditional_roles("if not rain then not alice sees bird");
    let same = scan(consequent(&positive), consequent(&positive)).unwrap();
    let opposite = scan(consequent(&positive), consequent(&negative)).unwrap();
    assert_eq!(opposite.components[2].similarity, r(0, 1));
    assert_eq!(opposite.components[2].dissimilarity, r(20, 1));
    assert_eq!(opposite.components[2].reason, Reason::NegationConflict);
    for i in [0, 1, 3] {
        assert_eq!(
            opposite.components[i].similarity,
            same.components[i].similarity
        );
        assert_eq!(
            opposite.components[i].dissimilarity,
            same.components[i].dissimilarity
        );
    }
    assert_eq!(
        same.totals
            .difference
            .checked_sub(opposite.totals.difference)
            .unwrap(),
        r(40, 1)
    );
}
#[test]
fn opposite_antecedent_polarity_cannot_penalize_identical_positive_body_spo() {
    let positive = conditional_roles("if rain then alice sees bird");
    let negative = conditional_roles("if not rain then alice sees bird");
    let evidence = scan(consequent(&positive), consequent(&negative)).unwrap();
    assert_eq!(evidence.components[2].similarity, r(20, 1));
    assert_eq!(evidence.components[2].dissimilarity, r(0, 1));
    assert_eq!(evidence.components[2].reason, Reason::Match);
    assert_eq!(
        evidence.left_constraints.conditional,
        ConditionalDisposition::Unresolved
    );
    assert_eq!(
        evidence.right_constraints.conditional,
        ConditionalDisposition::Unresolved
    );
    assert!(!evidence.left_constraints.executable && !evidence.right_constraints.executable);
}
#[test]
fn uncertain_body_scope_contributes_absence_without_inventing_explicit_conflict() {
    let positive = conditional_roles("if rain then alice sees bird");
    let uncertain = conditional_roles("if rain then maybe not alice sees bird");
    let evidence = scan(consequent(&positive), consequent(&uncertain)).unwrap();
    assert_eq!(evidence.components[2].similarity, r(0, 1));
    assert_eq!(evidence.components[2].dissimilarity, r(20, 1));
    assert_eq!(evidence.components[2].reason, Reason::UncertainNegation);
    assert!(evidence.right_constraints.uncertain_negation);
    assert!(evidence.right_constraints.inhibited);
    assert!(!evidence.right_constraints.executable);
    assert_eq!(
        evidence
            .totals
            .similarity
            .checked_add(evidence.totals.dissimilarity)
            .unwrap(),
        r(100, 1)
    );
}

#[test]
fn uncertain_antecedent_scope_does_not_change_known_body_role_agreement() {
    let positive = conditional_roles("if rain then alice sees bird");
    let uncertain = conditional_roles("if maybe not rain then alice sees bird");
    let evidence = scan(consequent(&positive), consequent(&uncertain)).unwrap();
    assert_eq!(evidence.components[2].similarity, r(20, 1));
    assert_eq!(evidence.components[2].dissimilarity, r(0, 1));
    assert_eq!(evidence.components[2].reason, Reason::Match);
    assert_eq!(
        evidence.right_constraints.conditional,
        ConditionalDisposition::Unresolved
    );
}
