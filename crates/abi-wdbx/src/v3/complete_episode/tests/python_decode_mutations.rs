use super::DecodedCompleteEpisode;
use super::decode_malformed::{field, group, tree};
use super::decode_normalize::normalize;
use super::fixtures::populated;
use super::python_decode::{Limits, agree, execute, reject};
use super::{bytes, draft_adapter};
use crate::v3::commitment::CanonicalValue as C;

// Path helpers used by the moved tests.
use super::python_decode::{descend, leaves};

#[test]
fn exhaustive_leaf_mutations_agree() {
    let original = tree(populated());
    let mut paths = vec![];
    leaves(&original, vec![], &mut paths);
    let mut corpus = Vec::new();
    for (leaf_index, path) in paths.into_iter().enumerate() {
        for replacement in [C::Bool(false), C::Null] {
            let mut v = original.clone();
            *descend(&mut v, &path) = replacement;
            corpus.push(reject(
                &format!("leaf/{}/bool-or-null@{leaf_index}", path.len()),
                v.canonical_bytes().unwrap(),
                Limits::default(),
                Some("InvalidShape"),
            ));
        }
        if matches!(descend(&mut original.clone(), &path), C::Bytes(_)) {
            let mut v = original.clone();
            *descend(&mut v, &path) = C::Bytes(vec![0; 3]);
            corpus.push(reject(
                &format!("leaf/{}/bytes-width@{leaf_index}", path.len()),
                v.canonical_bytes().unwrap(),
                Limits::default(),
                Some("InvalidShape"),
            ));
        }
    }
    execute(corpus);
}

#[test]
fn relational_slot_mutations_agree() {
    use super::decode_malformed::{field, group, items};
    let mut corpus = Vec::new();

    {
        let mut v = tree(populated());
        *field(
            field(
                &mut items(field(group(&mut v, 8), "approvals"))[0],
                "binding",
            ),
            "tenant",
        ) = C::Text("other-tenant".into());
        corpus.push(reject(
            "relational/approval-tenant-mismatch",
            v.canonical_bytes().unwrap(),
            Limits::default(),
            Some("InvalidValue:ScopeMismatch"),
        ));
    }

    {
        let mut v = tree(populated());
        *field(field(group(&mut v, 5), "summary"), "kind") = C::Array(vec![C::Unsigned(1)]);
        corpus.push(reject(
            "relational/summary-invalid-kind",
            v.canonical_bytes().unwrap(),
            Limits::default(),
            Some("InvalidValue:InvalidArtifactKind"),
        ));
    }

    {
        let mut v = tree(populated());
        let a = items(field(group(&mut v, 11), "causal_parents"));
        a.push(a[0].clone());
        corpus.push(reject(
            "relational/duplicate-causal-parent",
            v.canonical_bytes().unwrap(),
            Limits::default(),
            Some("InvalidValue:DuplicateReference"),
        ));
    }

    {
        let mut v = tree(populated());
        let a = items(field(group(&mut v, 11), "consulted_parents"));
        a.push(a[0].clone());
        corpus.push(reject(
            "relational/duplicate-consulted-parent",
            v.canonical_bytes().unwrap(),
            Limits::default(),
            Some("InvalidValue:DuplicateReference"),
        ));
    }

    {
        let mut v = tree(populated());
        let retention = field(group(&mut v, 10), "retention");
        if let C::Array(parts) = retention
            && let C::Map(m) = &mut parts[1]
        {
            for (k, val) in m.iter_mut() {
                if matches!(k, C::Text(t) if t == "expires_at") {
                    *val = C::Unsigned(0);
                }
            }
        }
        corpus.push(reject(
            "relational/retention-expiry-not-greater",
            v.canonical_bytes().unwrap(),
            Limits::default(),
            None,
        ));
    }

    execute(corpus);
}

#[test]
fn draft_corpus_breadth_agrees() {
    let mut corpus = Vec::new();
    for v in super::draft_corpus::vocabulary_cases() {
        let input = draft_adapter::input(&v).expect("draft corpus fixture");
        let _ = normalize(input.clone());
        let data = bytes(input);
        corpus.push(agree("draft-corpus/vocabulary", data, Limits::default()));
    }
    for v in super::draft_corpus::set_reversal_cases() {
        let input = draft_adapter::input(&v).expect("draft corpus fixture");
        let data = bytes(input);
        corpus.push(agree("draft-corpus/set-reversal", data, Limits::default()));
    }
    if corpus.is_empty() {
        return;
    }
    execute(corpus);
}

#[test]
fn ingress_category_census_agrees() {
    let mut corpus = Vec::new();

    // UnsupportedDomain: wrong domain text.
    {
        let mut v = tree(populated());
        *group(&mut v, 0) = C::Text("wrong-domain".into());
        corpus.push(reject(
            "ingress/unsupported-domain",
            v.canonical_bytes().unwrap(),
            Limits::default(),
            Some("UnsupportedDomain"),
        ));
    }

    // UnsupportedVersion: version 2.
    {
        let mut v = tree(populated());
        *group(&mut v, 1) = C::Unsigned(2);
        corpus.push(reject(
            "ingress/unsupported-version",
            v.canonical_bytes().unwrap(),
            Limits::default(),
            Some("UnsupportedVersion"),
        ));
    }

    // UnknownVariant: route tag 99.
    {
        let mut v = tree(populated());
        *field(group(&mut v, 6), "route") = C::Array(vec![C::Unsigned(99)]);
        corpus.push(reject(
            "ingress/unknown-variant",
            v.canonical_bytes().unwrap(),
            Limits::default(),
            Some("UnknownVariant"),
        ));
    }

    // ParentLimit: populated with max_parent_count 0 (only if Rust rejects).
    {
        let data = bytes(populated());
        let limits = Limits {
            max_parent_count: Some(0),
            ..Limits::default()
        };
        let rust_result = DecodedCompleteEpisode::from_canonical_bytes(&data, limits.rust());
        if rust_result.is_err() {
            corpus.push(reject("ingress/parent-limit", data, limits, None));
        }
    }

    if corpus.is_empty() {
        return;
    }
    execute(corpus);
}
