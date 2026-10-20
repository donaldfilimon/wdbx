use super::*;
use sha2::Digest;

#[test]
fn identifiers_ranges_and_redaction() {
    assert!(OpaqueRef::new("").is_err());
    assert!(OpaqueRef::new("synthetic/secret").is_err());
    let token = OpaqueRef::new("synthetic-secret").unwrap();
    assert!(!format!("{token:?}").contains("synthetic-secret"));
    assert!(EpisodeId::new([0; 16]).is_err());
    assert!(UnitPpm::new(1_000_001).is_err());
    assert!(Measurement::new(MeasurementUnit::Count, SignedMicros::new(-1)).is_err());
    assert!(Measurement::new(MeasurementUnit::Bytes, SignedMicros::new(1)).is_err());
}

#[test]
fn unknown_zero_and_tag_shape_are_distinct() {
    let unknown = encode_test(&Estimate::<UnitPpm>::Unknown);
    let zero = encode_test(&Estimate::Known(UnitPpm::new(0).unwrap()));
    assert_eq!(unknown, vec![0x81, 0]);
    assert_eq!(zero, vec![0x82, 1, 0]);
    assert_eq!(encode_test(&Route::NoModel), vec![0x81, 0]);
    assert_eq!(encode_test(&SignedMicros::new(-1)), vec![0x20]);
}

mod fixtures;
use fixtures::*;

#[test]
fn complete_minimal_and_populated_roundtrip() {
    for input in [minimal(), populated()] {
        let episode = CompleteEpisode::try_new(input).unwrap();
        let bytes = episode.canonical_bytes().unwrap();
        assert_eq!(
            episode.digest().unwrap(),
            <[u8; 32]>::from(sha2::Sha256::digest(&bytes))
        );
        assert!(
            super::super::commitment::EpisodeCommitment::from_canonical_bytes(
                &bytes,
                super::super::commitment::CanonicalDecodeLimits::default()
            )
            .is_err()
        );
        assert!(!format!("{episode:?}").contains("synthetic"));
    }
}

#[test]
fn scope_kind_and_duplicate_validation() {
    let mut input = populated();
    input.authority.approvals[0].binding = binding("other-tenant");
    assert_eq!(
        CompleteEpisode::try_new(input).unwrap_err(),
        CompleteEpisodeError::ScopeMismatch
    );
    let mut input = populated();
    input.context.summary.as_mut().unwrap().kind = ArtifactKind::Prediction;
    assert_eq!(
        CompleteEpisode::try_new(input).unwrap_err(),
        CompleteEpisodeError::InvalidArtifactKind
    );
    let mut input = populated();
    input
        .provenance
        .causal_parents
        .push(input.provenance.causal_parents[0].clone());
    assert_eq!(
        CompleteEpisode::try_new(input).unwrap_err(),
        CompleteEpisodeError::DuplicateReference
    );
}

mod boundaries;
mod collections;
mod coverage;
mod references;

fn bytes(input: CompleteEpisodeInput) -> Vec<u8> {
    CompleteEpisode::try_new(input)
        .unwrap()
        .canonical_bytes()
        .unwrap()
}
fn known_mut<T>(value: &mut Estimate<T>) -> &mut T {
    match value {
        Estimate::Known(value) => value,
        Estimate::Unknown => panic!("fixture requires Known"),
    }
}

mod diagnostics;

// Local regression pins only, provisional until independent witness and ABI review.
#[test]
fn draft_vectors() {
    for (input, expected) in [
        (minimal(), include_str!("tests/provisional-minimal.hex")),
        (populated(), include_str!("tests/provisional-populated.hex")),
    ] {
        let actual = bytes(input)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert_eq!(actual, expected.trim());
    }
}

mod draft_witness;

mod draft_adapter;

mod draft_corpus;

mod draft_shape;

mod draft_mapping;

mod decode_bounds;
mod decode_malformed;
mod decode_normalize;
mod decode_roundtrip;
mod decode_semantics;
