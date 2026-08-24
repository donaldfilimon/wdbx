//! Positive golden and boundary tests for the v3 canonical commitment profile.

use abi_wdbx::v3::commitment::{
    CanonicalCborError, CanonicalValue, EpisodeCommitment, PROFILE_NAME,
};

fn fixture(header: CanonicalValue, parents: Vec<[u8; 32]>) -> EpisodeCommitment {
    EpisodeCommitment::new(
        1,
        header,
        CanonicalValue::Map(vec![(
            CanonicalValue::Unsigned(1),
            CanonicalValue::Text("synthetic".into()),
        )]),
        parents,
    )
}

#[test]
fn profile_and_envelope_bytes_are_exact() {
    let commitment = fixture(CanonicalValue::Map(Vec::new()), Vec::new());
    let bytes = commitment.canonical_bytes().expect("canonical envelope");

    assert_eq!(PROFILE_NAME, "abbey-cbor-episode-v1");
    assert_eq!(
        hex(&bytes),
        "a5007561626265792d63626f722d657069736f64652d7631010102a003a1016973796e7468657469630480"
    );
}

#[test]
fn parent_permutations_have_identical_bytes_and_digest() {
    let low = [0x11; 32];
    let high = [0xee; 32];
    let forward = fixture(CanonicalValue::Map(Vec::new()), vec![low, high]);
    let reverse = fixture(CanonicalValue::Map(Vec::new()), vec![high, low]);

    assert_eq!(
        forward.canonical_bytes().expect("forward bytes"),
        reverse.canonical_bytes().expect("reverse bytes")
    );
    assert_eq!(
        forward.digest().expect("forward digest"),
        reverse.digest().expect("reverse digest")
    );
}

#[test]
fn duplicate_parents_are_preserved_not_silently_deduplicated() {
    let parent = [0x44; 32];
    let once = fixture(CanonicalValue::Map(Vec::new()), vec![parent]);
    let twice = fixture(CanonicalValue::Map(Vec::new()), vec![parent, parent]);

    assert_ne!(
        once.canonical_bytes().expect("one parent"),
        twice.canonical_bytes().expect("two repeated parents")
    );
    assert_ne!(
        once.digest().expect("one-parent digest"),
        twice.digest().expect("two-parent digest")
    );
}

#[test]
fn positive_golden_cbor_fixtures_match_exact_bytes_and_digests() {
    let empty = fixture(CanonicalValue::Map(Vec::new()), Vec::new());
    assert_golden(
        &empty,
        include_str!("golden/abbey-cbor-episode-v1/empty-parents.hex"),
        include_str!("golden/abbey-cbor-episode-v1/empty-parents.sha256"),
    );

    let two_parents = fixture(
        CanonicalValue::Map(Vec::new()),
        vec![[0xee; 32], [0x11; 32]],
    );
    assert_golden(
        &two_parents,
        include_str!("golden/abbey-cbor-episode-v1/two-parents.hex"),
        include_str!("golden/abbey-cbor-episode-v1/two-parents.sha256"),
    );
}

#[test]
fn map_keys_follow_rfc_8949_length_first_order() {
    let header = CanonicalValue::Map(vec![
        (CanonicalValue::Text("aa".into()), CanonicalValue::Null),
        (CanonicalValue::Unsigned(24), CanonicalValue::Null),
        (CanonicalValue::Text("b".into()), CanonicalValue::Null),
        (CanonicalValue::Unsigned(1), CanonicalValue::Null),
    ]);
    let bytes = fixture(header, Vec::new())
        .canonical_bytes()
        .expect("ordered map");

    assert!(
        hex(&bytes).contains("02a401f61818f66162f6626161f6"),
        "map keys must sort by encoded length and then lexical bytes"
    );
}

#[test]
fn absent_and_present_zero_are_distinct() {
    let absent = fixture(CanonicalValue::Map(Vec::new()), Vec::new());
    let present_zero = fixture(
        CanonicalValue::Map(vec![(
            CanonicalValue::Unsigned(7),
            CanonicalValue::Unsigned(0),
        )]),
        Vec::new(),
    );

    assert_ne!(
        absent.canonical_bytes().expect("absent bytes"),
        present_zero.canonical_bytes().expect("zero bytes")
    );
    assert_ne!(
        absent.digest().expect("absent digest"),
        present_zero.digest().expect("zero digest")
    );
}

#[test]
fn duplicate_encoded_map_keys_fail_closed_without_content() {
    let commitment = fixture(
        CanonicalValue::Map(vec![
            (
                CanonicalValue::Text("private-first".into()),
                CanonicalValue::Null,
            ),
            (
                CanonicalValue::Text("private-first".into()),
                CanonicalValue::Bool(true),
            ),
        ]),
        Vec::new(),
    );

    let error = commitment.canonical_bytes().expect_err("duplicate key");
    assert_eq!(error, CanonicalCborError::DuplicateMapKey);
    assert!(!error.to_string().contains("private-first"));
}

#[test]
fn header_and_payload_must_be_maps_and_schema_version_is_nonzero() {
    assert_eq!(
        EpisodeCommitment::new(
            1,
            CanonicalValue::Null,
            CanonicalValue::Map(Vec::new()),
            Vec::new(),
        )
        .canonical_bytes(),
        Err(CanonicalCborError::HeaderMustBeMap)
    );
    assert_eq!(
        EpisodeCommitment::new(
            1,
            CanonicalValue::Map(Vec::new()),
            CanonicalValue::Null,
            Vec::new(),
        )
        .canonical_bytes(),
        Err(CanonicalCborError::PayloadMustBeMap)
    );
    assert_eq!(
        EpisodeCommitment::new(
            0,
            CanonicalValue::Map(Vec::new()),
            CanonicalValue::Map(Vec::new()),
            Vec::new(),
        )
        .canonical_bytes(),
        Err(CanonicalCborError::ZeroSchemaVersion)
    );
}

fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;

    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            write!(&mut output, "{byte:02x}").expect("write to string");
            output
        },
    )
}

fn assert_golden(commitment: &EpisodeCommitment, bytes_hex: &str, digest_hex: &str) {
    let expected_bytes = decode_hex(bytes_hex.trim());
    assert_eq!(
        commitment.canonical_bytes().expect("canonical bytes"),
        expected_bytes
    );
    assert_eq!(
        hex(&commitment.digest().expect("canonical digest")),
        digest_hex.trim()
    );
}

fn decode_hex(input: &str) -> Vec<u8> {
    assert_eq!(input.len() % 2, 0, "golden hex must have byte pairs");
    input
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let pair = core::str::from_utf8(pair).expect("golden hex is ASCII");
            u8::from_str_radix(pair, 16).expect("golden hex byte")
        })
        .collect()
}
