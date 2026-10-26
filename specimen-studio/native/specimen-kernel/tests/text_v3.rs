use sha2::{Digest, Sha256};
use specimen_kernel::{language_v3::*, text_v3::*};

fn prep(text: &str) -> Prepared {
    prep_id("synthetic", text)
}
fn prep_id(id: &str, text: &str) -> Prepared {
    let tables = UtilityTables::new(
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
    .unwrap();
    prepare(
        Input {
            id: id.into(),
            text: text.into(),
        },
        &ContextSnapshot {
            population: "synthetic".into(),
            utility_version: "v1".into(),
            tokens: vec![],
            conditions: vec![],
        },
        &tables,
        Budget::default(),
        &mut FixedDraw(0),
    )
    .unwrap()
}
fn key(p: &Prepared) -> TextKey {
    encode_prepared(p).unwrap()
}

#[test]
fn exact_ast_token_boundaries_and_full_digest_framing() {
    for (n, class) in [
        (6, Resolution::Low),
        (7, Resolution::Medium),
        (16, Resolution::Medium),
        (17, Resolution::High),
    ] {
        let p = prep(&vec!["word"; n].join(" "));
        let k = key(&p);
        assert_eq!(k.token_count(), n);
        assert_eq!(k.resolution(), class);
        assert_eq!(k.pattern_key().encoder_version, "text-v3");
        assert_eq!(k.pattern_key().modality, "text");
        let mut expected = vec![];
        for bytes in [k.transform(), class.as_str().as_bytes(), b"text"] {
            expected.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
            expected.extend_from_slice(bytes);
        }
        assert_eq!(k.preimage(), expected);
        assert_eq!(
            k.pattern_key().digest.as_slice(),
            Sha256::digest(&expected).as_slice()
        );
    }
}
#[test]
fn low_numeric_coarsening_retains_quotes_negation_and_controls() {
    assert_eq!(
        key(&prep("2+3")).pattern_key(),
        key(&prep("8+9")).pattern_key()
    );
    assert_eq!(
        key(&prep("red blue red")).pattern_key(),
        key(&prep("blue red")).pattern_key()
    );
    for (a, b) in [
        ("2+3", "\"2\" + \"3\""),
        ("hello", "not hello"),
        ("hello and world", "hello or world"),
        ("\"not\"", "not"),
    ] {
        assert_ne!(
            key(&prep(a)).pattern_key(),
            key(&prep(b)).pattern_key(),
            "{a} vs {b}"
        );
    }
}
#[test]
fn medium_order_roles_operators_and_high_complete_values_survive() {
    assert_ne!(
        key(&prep("alpha beta gamma delta epsilon zeta eta")).pattern_key(),
        key(&prep("eta zeta epsilon delta gamma beta alpha")).pattern_key()
    );
    assert_ne!(
        key(&prep("1+2+3+4")).pattern_key(),
        key(&prep("1-2+3+4")).pattern_key()
    );
    let a = key(&prep(
        "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho",
    ));
    let b = key(&prep(
        "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi sigma",
    ));
    assert_eq!(a.resolution(), Resolution::High);
    assert_ne!(a.pattern_key(), b.pattern_key());
}
#[test]
fn ids_whitespace_and_utf8_spans_are_excluded_original_limit_is_characters() {
    assert_eq!(
        key(&prep_id("a", "Élan  blue")).pattern_key(),
        key(&prep_id("b", "Élan\tblue")).pattern_key()
    );
    assert_eq!(key(&prep("Élan blue")), key(&prep("Élan blue")));
    let p = prep(&"界".repeat(2000));
    assert!(encode(&p.clauses()[0].variants()[0]).is_ok());
    let p = prep(&"界".repeat(2001));
    assert_eq!(
        encode(&p.clauses()[0].variants()[0]).unwrap_err(),
        TextError::PatternTooLong
    );
}
#[test]
fn punctuation_does_not_count_as_lexical_ast_complexity() {
    let p = prep("alpha, beta gamma delta epsilon zeta");
    let k = key(&p);
    assert_eq!(k.token_count(), 6);
    assert_eq!(k.resolution(), Resolution::Low);
    let n = key(&prep("not alpha beta gamma delta epsilon"));
    assert_eq!(n.token_count(), 6);
}
#[test]
fn medium_roles_change_key_but_host_rule_refs_do_not() {
    let run = |role, reference: &str| {
        let mut snapshots = UtilityKind::ALL
            .into_iter()
            .map(|kind| UtilitySnapshot {
                population: "synthetic".into(),
                version: "v1".into(),
                kind,
                rows: vec![],
            })
            .collect::<Vec<_>>();
        snapshots
            .iter_mut()
            .find(|s| s.kind == UtilityKind::Grammar)
            .unwrap()
            .rows = vec![UtilityRow {
            reference: reference.into(),
            phrase: "alpha".into(),
            effect: UtilityEffect::Role(role),
        }];
        let tables = UtilityTables::new("synthetic", "v1", snapshots).unwrap();
        prepare(
            Input {
                id: reference.into(),
                text: "alpha beta gamma delta epsilon zeta eta".into(),
            },
            &ContextSnapshot {
                population: "synthetic".into(),
                utility_version: "v1".into(),
                tokens: vec![],
                conditions: vec![],
            },
            &tables,
            Budget::default(),
            &mut FixedDraw(0),
        )
        .unwrap()
    };
    assert_eq!(key(&run(Role::Subject, "a")), key(&run(Role::Subject, "b")));
    assert_ne!(key(&run(Role::Subject, "a")), key(&run(Role::Object, "a")));
}
#[test]
fn high_complete_numeric_ast_and_low_quoted_literal_content_are_retained() {
    let a = key(&prep("1+2+3+4+5+6+7+8+9"));
    let b = key(&prep("1+2+3+4+5+6+7+8+10"));
    assert_eq!(a.resolution(), Resolution::High);
    assert_ne!(a.pattern_key(), b.pattern_key());
    assert_ne!(key(&prep("\"ABC\"")), key(&prep("\"abc\"")));
    assert_ne!(key(&prep("\"two\"")), key(&prep("2+0")));
}
#[test]
fn low_and_medium_lexical_transforms_ignore_punctuation() {
    for (plain, punctuated) in [
        ("red blue", "red, blue"),
        (
            "alpha beta gamma delta epsilon zeta eta",
            "alpha, beta gamma delta epsilon zeta eta",
        ),
    ] {
        assert_eq!(
            key(&prep(plain)).pattern_key(),
            key(&prep(punctuated)).pattern_key()
        );
    }
}
