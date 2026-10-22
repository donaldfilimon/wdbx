use super::*;
use crate::v3::commitment::{CanonicalDecodeLimits, CanonicalValue as C};
use CompleteEpisodeDecodeError as E;
pub(super) fn tree(input: CompleteEpisodeInput) -> C {
    C::from_canonical_bytes(&bytes(input), CanonicalDecodeLimits::default()).unwrap()
}
pub(super) fn decode(value: C) -> std::result::Result<DecodedCompleteEpisode, E> {
    DecodedCompleteEpisode::from_canonical_bytes(
        &value.canonical_bytes().unwrap(),
        CanonicalDecodeLimits::default(),
    )
}
pub(super) fn field<'a>(value: &'a mut C, key: &str) -> &'a mut C {
    let C::Map(entries) = value else {
        panic!("map fixture")
    };
    &mut entries
        .iter_mut()
        .find(|(k, _)| *k == C::Text(key.into()))
        .unwrap()
        .1
}
pub(super) fn group(value: &mut C, key: u64) -> &mut C {
    let C::Map(entries) = value else {
        panic!("map fixture")
    };
    &mut entries
        .iter_mut()
        .find(|(k, _)| *k == C::Unsigned(key))
        .unwrap()
        .1
}
pub(super) fn items(value: &mut C) -> &mut Vec<C> {
    let C::Array(values) = value else {
        panic!("array fixture")
    };
    values
}
fn error(value: C, expected: E) {
    assert_eq!(decode(value).unwrap_err(), expected);
}
fn invalid(value: CompleteEpisodeError) -> E {
    E::InvalidValue(value)
}
#[test]
fn envelope_domain_version_and_primitives() {
    for (key, replacement, expected) in [
        (0, C::Text("sentinel-domain".into()), E::UnsupportedDomain),
        (0, C::Null, E::InvalidShape),
        (1, C::Unsigned(0), E::UnsupportedVersion),
        (1, C::Unsigned(2), E::UnsupportedVersion),
        (1, C::Negative(-1), E::InvalidShape),
        (1, C::Bool(true), E::InvalidShape),
    ] {
        let mut v = tree(minimal());
        *group(&mut v, key) = replacement;
        error(v, expected);
    }
    for value in [C::Bool(true), C::Null, C::Array(vec![]), C::Unsigned(1)] {
        error(value, E::InvalidShape);
    }
    let old =
        crate::v3::commitment::EpisodeCommitment::new(1, C::Map(vec![]), C::Map(vec![]), vec![]);
    assert!(
        DecodedCompleteEpisode::from_canonical_bytes(
            &old.canonical_bytes().unwrap(),
            CanonicalDecodeLimits::default()
        )
        .is_err()
    );
}
// Walk the original tree with index paths, then mutate one map at a time. This
// covers every represented nested structure without depending on decoder helpers.
#[derive(Clone)]
enum Step {
    Map(usize),
    Array(usize),
}
fn maps(value: &C, path: Vec<Step>, output: &mut Vec<Vec<Step>>) {
    match value {
        C::Map(entries) => {
            output.push(path.clone());
            for (i, (_, v)) in entries.iter().enumerate() {
                let mut p = path.clone();
                p.push(Step::Map(i));
                maps(v, p, output);
            }
        }
        C::Array(values) => {
            for (i, v) in values.iter().enumerate() {
                let mut p = path.clone();
                p.push(Step::Array(i));
                maps(v, p, output);
            }
        }
        _ => {}
    }
}
fn at<'a>(mut value: &'a mut C, path: &[Step]) -> &'a mut C {
    for step in path {
        value = match (step, value) {
            (Step::Map(i), C::Map(v)) => &mut v[*i].1,
            (Step::Array(i), C::Array(v)) => &mut v[*i],
            _ => panic!("original tree path"),
        };
    }
    value
}
#[test]
fn every_present_map_rejects_missing_extra_and_nontext_keys() {
    let original = tree(populated());
    let mut paths = vec![];
    maps(&original, vec![], &mut paths);
    for path in paths {
        let C::Map(entries) = at(&mut original.clone(), &path).clone() else {
            unreachable!()
        };
        for (i, (key, _)) in entries.iter().enumerate() {
            // Optional members may be absent; deleting any required member must fail.
            let optional = matches!(key,C::Text(k) if ["monotonic","summary","reservoir","consent","deletion_key"].contains(&k.as_str()));
            if !optional {
                let mut v = original.clone();
                let C::Map(m) = at(&mut v, &path) else {
                    unreachable!()
                };
                m.remove(i);
                error(v, E::InvalidShape);
            }
        }
        let mut v = original.clone();
        let C::Map(m) = at(&mut v, &path) else {
            unreachable!()
        };
        m.push((C::Text("sentinel-extra".into()), C::Null));
        error(v, E::InvalidShape);
        if !path.is_empty() {
            let mut v = original.clone();
            let C::Map(m) = at(&mut v, &path) else {
                unreachable!()
            };
            m[0].0 = C::Unsigned(999);
            error(v, E::InvalidShape);
        }
    }
}
#[test]
fn option_estimate_and_variant_shapes() {
    for (g, k) in [
        (3, "monotonic"),
        (5, "summary"),
        (5, "reservoir"),
        (8, "consent"),
        (10, "deletion_key"),
    ] {
        let mut v = tree(populated());
        *field(group(&mut v, g), k) = C::Null;
        error(v, E::InvalidShape);
    }
    for replacement in [
        C::Null,
        C::Bool(false),
        C::Array(vec![]),
        C::Array(vec![C::Unsigned(2)]),
        C::Array(vec![C::Unsigned(0), C::Null]),
        C::Array(vec![C::Unsigned(1)]),
        C::Array(vec![C::Unsigned(1), C::Unsigned(0), C::Unsigned(0)]),
    ] {
        let mut v = tree(minimal());
        *field(group(&mut v, 5), "habituation") = replacement;
        error(v, E::InvalidShape);
    }
    for replacement in [
        C::Unsigned(0),
        C::Array(vec![]),
        C::Array(vec![C::Unsigned(0), C::Map(vec![])]),
        C::Array(vec![C::Bool(false)]),
    ] {
        let mut v = tree(minimal());
        *field(group(&mut v, 6), "route") = replacement;
        error(v, E::InvalidShape);
    }
    let mut v = tree(minimal());
    *field(group(&mut v, 6), "route") = C::Array(vec![C::Unsigned(99)]);
    error(v, E::UnknownVariant);
    let mut v = tree(minimal());
    *field(field(group(&mut v, 4), "actor"), "kind") = C::Text("sentinel-label".into());
    error(v, E::UnknownVariant);
    let mut v = tree(minimal());
    *field(group(&mut v, 9), "maturity") = C::Text("C8".into());
    error(v, E::UnknownVariant);
    for replacement in [
        C::Array(vec![C::Unsigned(99), C::Map(vec![])]),
        C::Array(vec![C::Unsigned(99)]),
    ] {
        let mut v = tree(minimal());
        *field(group(&mut v, 10), "retention") = replacement;
        error(v, E::UnknownVariant);
    }
    let mut v = tree(minimal());
    *field(field(group(&mut v, 2), "binding"), "scope") =
        C::Array(vec![C::Unsigned(99), C::Map(vec![])]);
    error(v, E::UnknownVariant);
}
#[test]
fn identifiers_widths_numbers_and_ranges() {
    for replacement in [
        C::Bytes(vec![0; 15]),
        C::Bytes(vec![0; 17]),
        C::Text("sentinel".into()),
    ] {
        let mut v = tree(minimal());
        *field(group(&mut v, 2), "episode_id") = replacement;
        error(v, E::InvalidShape);
    }
    let mut v = tree(minimal());
    *field(group(&mut v, 2), "episode_id") = C::Bytes(vec![0; 16]);
    error(v, invalid(CompleteEpisodeError::InvalidUuid));
    for spelling in ["", "bad/slash", &"x".repeat(129)] {
        let mut v = tree(minimal());
        *field(group(&mut v, 4), "source_node") = C::Text(spelling.into());
        error(v, invalid(CompleteEpisodeError::InvalidIdentifier));
    }
    let mut v = tree(minimal());
    *field(group(&mut v, 4), "signer") = C::Text("sentinel-signer".into());
    error(v, invalid(CompleteEpisodeError::InvalidIdentifier));
    for (n, expected) in [
        (1_000_001, invalid(CompleteEpisodeError::InvalidRange)),
        (u64::MAX, E::InvalidShape),
    ] {
        let mut v = tree(minimal());
        *field(group(&mut v, 5), "habituation") = C::Array(vec![C::Unsigned(1), C::Unsigned(n)]);
        error(v, expected);
    }
    let mut v = tree(populated());
    let utility = &mut items(field(group(&mut v, 7), "utility"))[1];
    *field(utility, "value") = C::Unsigned(u64::MAX);
    error(v, E::InvalidShape);
    let mut v = tree(populated());
    let utility = &mut items(field(group(&mut v, 7), "utility"))[1];
    *field(utility, "unit") = C::Array(vec![C::Unsigned(1)]);
    *field(utility, "value") = C::Negative(-1);
    error(v, invalid(CompleteEpisodeError::InvalidRange));
    let mut v = tree(populated());
    *field(field(group(&mut v, 3), "monotonic"), "start") = C::Unsigned(u64::MAX);
    error(v, invalid(CompleteEpisodeError::InvalidRange));
    let mut v = tree(minimal());
    *field(group(&mut v, 10), "retention") = C::Array(vec![
        C::Unsigned(2),
        C::Map(vec![(C::Text("expires_at".into()), C::Unsigned(0))]),
    ]);
    error(v, invalid(CompleteEpisodeError::InvalidRange));
}
#[test]
fn duplicate_and_unsorted_semantic_sets() {
    for (g, k) in [
        (8, "approvals"),
        (10, "holds"),
        (11, "causal_parents"),
        (9, "source_refs"),
        (9, "observation_refs"),
        (11, "records"),
        (10, "edges"),
        (11, "consulted_parents"),
        (8, "models"),
    ] {
        let mut input = populated();
        // Existing populated lists need two distinct members for ordering evidence.
        if g == 8 && k == "models" {
            input.authority.models = vec![version("z"), version("a")];
        }
        let original = tree(input);
        let mut duplicate = original.clone();
        let a = items(field(group(&mut duplicate, g), k));
        a.push(a[0].clone());
        error(duplicate, invalid(CompleteEpisodeError::DuplicateReference));
        let mut reversed = original;
        let a = items(field(group(&mut reversed, g), k));
        if a.len() > 1 {
            a.reverse();
            error(reversed, E::NonCanonicalEncoding);
        }
    }
}
#[test]
fn every_semantic_sort_is_rejected_if_wire_order_changes() {
    let mut input = populated();
    input.authority.models.push(version("model-two"));
    input
        .authority
        .calibrations
        .push(version("calibration-two"));
    input.lifecycle.holds.push(local(88));
    input
        .evidence
        .observation_refs
        .push(artifact(ArtifactKind::Observation, 88));
    input
        .provenance
        .records
        .push(artifact(ArtifactKind::Provenance, 88));
    let original = tree(input);
    for (g, k) in [
        (5, "resources"),
        (7, "costs"),
        (8, "capabilities"),
        (8, "models"),
        (8, "calibrations"),
        (8, "approvals"),
        (9, "source_refs"),
        (9, "observation_refs"),
        (10, "holds"),
        (10, "edges"),
        (11, "causal_parents"),
        (11, "consulted_parents"),
        (11, "records"),
    ] {
        let mut v = original.clone();
        let values = items(field(group(&mut v, g), k));
        assert!(values.len() > 1);
        values.reverse();
        error(v, E::NonCanonicalEncoding);
    }
    let mut v = original.clone();
    let action = &mut items(field(group(&mut v, 6), "actions"))[0];
    items(field(action, "parameter_classes")).reverse();
    error(v, E::NonCanonicalEncoding);
    let mut v = original;
    let posterior = &mut items(field(group(&mut v, 5), "regime_posterior"))[1];
    items(posterior).reverse();
    error(v, E::NonCanonicalEncoding);
}
fn leaves(value: &C, path: Vec<Step>, output: &mut Vec<Vec<Step>>) {
    match value {
        C::Map(entries) => {
            for (i, (_, v)) in entries.iter().enumerate() {
                let mut p = path.clone();
                p.push(Step::Map(i));
                leaves(v, p, output);
            }
        }
        C::Array(values) => {
            for (i, v) in values.iter().enumerate() {
                let mut p = path.clone();
                p.push(Step::Array(i));
                leaves(v, p, output);
            }
        }
        _ => output.push(path),
    }
}
#[test]
fn every_present_scalar_rejects_boolean_null_and_wrong_byte_width() {
    let original = tree(populated());
    let mut paths = vec![];
    leaves(&original, vec![], &mut paths);
    for path in paths {
        for replacement in [C::Bool(false), C::Null] {
            let mut v = original.clone();
            *at(&mut v, &path) = replacement;
            error(v, E::InvalidShape);
        }
        if matches!(at(&mut original.clone(), &path), C::Bytes(_)) {
            let mut v = original.clone();
            *at(&mut v, &path) = C::Bytes(vec![0; 3]);
            error(v, E::InvalidShape);
        }
    }
}
#[test]
fn every_unit_variant_checks_arity_before_unknown_tag() {
    type Select = fn(&mut C) -> &mut C;
    let selectors: &[Select] = &[
        |v| field(group(v, 6), "route"),
        |v| field(group(v, 7), "safety_effect"),
        |v| field(group(v, 7), "rollback"),
        |v| field(&mut items(field(group(v, 6), "actions"))[0], "phase"),
        |v| field(&mut items(field(group(v, 7), "utility"))[1], "unit"),
        |v| field(&mut items(field(group(v, 5), "resources"))[0], "resource"),
        |v| field(field(group(v, 5), "summary"), "kind"),
        |v| field(&mut items(field(group(v, 10), "edges"))[0], "relation"),
        |v| field(&mut items(field(group(v, 8), "approvals"))[0], "domain"),
    ];
    for select in selectors {
        for (replacement, expected) in [
            (C::Array(vec![C::Unsigned(99)]), E::UnknownVariant),
            (
                C::Array(vec![C::Unsigned(99), C::Map(vec![])]),
                E::InvalidShape,
            ),
            (C::Array(vec![C::Text("sentinel".into())]), E::InvalidShape),
        ] {
            let mut v = tree(populated());
            *select(&mut v) = replacement;
            error(v, expected);
        }
    }
    let selectors: &[Select] = &[
        |v| field(field(group(v, 2), "binding"), "scope"),
        |v| &mut items(field(group(v, 11), "consulted_parents"))[0],
    ];
    for select in selectors {
        for (replacement, expected) in [
            (
                C::Array(vec![C::Unsigned(99), C::Map(vec![])]),
                E::UnknownVariant,
            ),
            (C::Array(vec![C::Unsigned(99)]), E::InvalidShape),
            (C::Array(vec![C::Unsigned(99), C::Null]), E::InvalidShape),
            (
                C::Array(vec![C::Unsigned(0), C::Map(vec![])]),
                E::InvalidShape,
            ),
        ] {
            let mut v = tree(populated());
            *select(&mut v) = replacement;
            error(v, expected);
        }
    }
    for replacement in [
        C::Array(vec![C::Unsigned(0), C::Map(vec![])]),
        C::Array(vec![C::Unsigned(1)]),
        C::Array(vec![C::Unsigned(1), C::Map(vec![])]),
        C::Array(vec![C::Unsigned(99), C::Null]),
        C::Array(vec![]),
    ] {
        let mut v = tree(populated());
        *field(group(&mut v, 10), "retention") = replacement;
        error(v, E::InvalidShape);
    }
}
