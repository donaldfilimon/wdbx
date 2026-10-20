//! Concrete container/omission assertions independent of enum discriminants.
use super::draft_witness::{compare, fixture};
use crate::v3::commitment::{CanonicalDecodeLimits, CanonicalValue as C};
use serde_json::{Value, json};
fn tree(v: &Value) -> C {
    let episode =
        super::super::CompleteEpisode::try_new(super::draft_adapter::input(v).unwrap()).unwrap();
    C::from_canonical_bytes(
        &episode.canonical_bytes().unwrap(),
        CanonicalDecodeLimits::default(),
    )
    .unwrap()
}
fn field(v: &C, key: C) -> &C {
    match v {
        C::Map(entries) => entries
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
            .unwrap(),
        _ => panic!("map expected"),
    }
}
fn named<'a>(v: &'a C, key: &str) -> &'a C {
    field(v, C::Text(key.into()))
}
fn tagged(tag: u64) -> C {
    C::Array(vec![C::Unsigned(tag)])
}
#[test]
fn wrapper_shapes_posterior_and_optional_distinctions() {
    let minimal = fixture(false);
    let populated = fixture(true);
    let min = tree(&minimal);
    let pop = tree(&populated);
    let context = field(&min, C::Unsigned(5));
    assert_eq!(named(context, "habituation"), &tagged(0));
    assert_eq!(named(context, "resources"), &C::Array(vec![]));
    match context {
        C::Map(entries) => assert!(!entries.iter().any(|(k, _)| *k == C::Text("summary".into()))),
        _ => panic!("map"),
    }
    assert_eq!(named(field(&min, C::Unsigned(6)), "route"), &tagged(0));
    assert_ne!(tagged(0), C::Unsigned(0));
    let ctx = field(&pop, C::Unsigned(5));
    assert_eq!(
        named(ctx, "habituation"),
        &C::Array(vec![C::Unsigned(1), C::Unsigned(0)])
    );
    assert_eq!(
        named(ctx, "regime_posterior"),
        &C::Array(vec![
            C::Unsigned(1),
            C::Array(vec![
                C::Array(vec![C::Text("a".into()), C::Unsigned(1_000_000)]),
                C::Array(vec![C::Text("z".into()), C::Unsigned(0)])
            ])
        ])
    );
    let binding = named(field(&pop, C::Unsigned(2)), "binding");
    assert_eq!(
        named(binding, "scope"),
        &C::Array(vec![
            C::Unsigned(0),
            C::Map(vec![(
                C::Text("guild".into()),
                C::Text("synthetic-guild".into())
            )])
        ])
    );
    let retention = named(field(&pop, C::Unsigned(10)), "retention");
    assert_eq!(
        retention,
        &C::Array(vec![
            C::Unsigned(2),
            C::Map(vec![(C::Text("expires_at".into()), C::Unsigned(100))])
        ])
    );
    let mut zero = minimal.clone();
    zero["context"]["habituation"] = json!({"known":0});
    assert_ne!(tree(&zero), min);
    let mut present = minimal.clone();
    present["time"]["monotonic"] = json!({"clock":"synthetic-clock","start":0,"end":0});
    assert_ne!(tree(&present), min);
    compare(&[minimal, populated, zero, present]);
}

#[test]
fn complete_domain_substitution_changes_bytes_and_digest() {
    use crate::v3::commitment::EpisodeCommitment;
    use sha2::{Digest, Sha256};

    for populated in [false, true] {
        let input = fixture(populated);
        let episode =
            super::super::CompleteEpisode::try_new(super::draft_adapter::input(&input).unwrap())
                .unwrap();
        let actual_bytes = episode.canonical_bytes().unwrap();
        let complete = tree(&input);
        assert_eq!(
            field(&complete, C::Unsigned(0)),
            &C::Text("wdbx-complete-episode-v1".into())
        );
        assert_eq!(complete.canonical_bytes().unwrap(), actual_bytes);
        assert_eq!(
            episode.digest().unwrap(),
            <[u8; 32]>::from(Sha256::digest(&actual_bytes))
        );

        // Synthetic substitution into the same complete map, not a valid old episode.
        let mut substituted = complete.clone();
        let C::Map(ref mut entries) = substituted else {
            panic!("complete map")
        };
        assert_eq!(entries.len(), 12);
        let (_, domain) = entries
            .iter_mut()
            .find(|(key, _)| *key == C::Unsigned(0))
            .unwrap();
        let saved_domain = domain.clone();
        *domain = C::Text("abbey-cbor-episode-v1".into());
        let substituted_bytes = substituted.canonical_bytes().unwrap();
        assert_ne!(actual_bytes, substituted_bytes);
        assert_ne!(
            Sha256::digest(&actual_bytes),
            Sha256::digest(&substituted_bytes)
        );

        let C::Map(ref mut entries) = substituted else {
            panic!("complete map")
        };
        *entries
            .iter_mut()
            .find(|(key, _)| *key == C::Unsigned(0))
            .unwrap() = (C::Unsigned(0), saved_domain);
        assert_eq!(substituted, complete, "all non-domain content is unchanged");
        for bytes in [&actual_bytes, &substituted_bytes] {
            let generic = C::from_canonical_bytes(bytes, CanonicalDecodeLimits::default()).unwrap();
            assert_eq!(&generic.canonical_bytes().unwrap(), bytes);
            assert!(
                EpisodeCommitment::from_canonical_bytes(bytes, CanonicalDecodeLimits::default())
                    .is_err()
            );
        }
        // Generic canonical syntax only; neither substitution is a typed decoder witness.
    }
}
