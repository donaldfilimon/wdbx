//! Restricted profile reader compatibility and malformed-input contracts.
use abi_wdbx::v3::commitment::{
    CanonicalDecodeError as E, CanonicalDecodeLimits as L, CanonicalValue as V, EpisodeCommitment,
};

fn decode(bytes: &[u8]) -> Result<V, E> {
    V::from_canonical_bytes(bytes, L::default())
}
fn unhex(input: &str) -> Vec<u8> {
    input
        .trim()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|p| u8::from_str_radix(core::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn envelope(parents: Vec<[u8; 32]>) -> Vec<u8> {
    EpisodeCommitment::new(1, V::Map(vec![]), V::Map(vec![]), parents)
        .canonical_bytes()
        .unwrap()
}

#[test]
fn golden_envelopes_reencode_and_retain_pinned_digests() {
    for (hex, digest, count) in [
        (
            include_str!("golden/abbey-cbor-episode-v1/empty-parents.hex"),
            include_str!("golden/abbey-cbor-episode-v1/empty-parents.sha256"),
            0,
        ),
        (
            include_str!("golden/abbey-cbor-episode-v1/two-parents.hex"),
            include_str!("golden/abbey-cbor-episode-v1/two-parents.sha256"),
            2,
        ),
    ] {
        let bytes = unhex(hex);
        let parsed = EpisodeCommitment::from_canonical_bytes(&bytes, L::default()).unwrap();
        assert_eq!(parsed.canonical_bytes().unwrap(), bytes);
        assert_eq!(parsed.digest().unwrap().as_slice(), unhex(digest));
        assert_eq!(parsed.schema_version(), 1);
        assert_eq!(parsed.header(), &V::Map(vec![]));
        assert!(matches!(parsed.payload(), V::Map(_)));
        assert_eq!(parsed.parent_digests().len(), count);
        for end in 0..bytes.len() {
            assert!(
                EpisodeCommitment::from_canonical_bytes(&bytes[..end], L::default()).is_err(),
                "prefix {end}"
            );
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            EpisodeCommitment::from_canonical_bytes(&trailing, L::default()),
            Err(E::TrailingBytes)
        );
    }
}

#[test]
fn scalar_boundaries_are_exact_and_text_is_not_normalized() {
    for hex in [
        "00",
        "17",
        "1818",
        "18ff",
        "190100",
        "19ffff",
        "1a00010000",
        "1affffffff",
        "1b0000000100000000",
        "1bffffffffffffffff",
        "20",
        "37",
        "3818",
        "38ff",
        "390100",
        "3b7fffffffffffffff",
        "40",
        "60",
        "f4",
        "f5",
        "f6",
        "6300c3a9",
        "6365cc81",
    ] {
        let bytes = unhex(hex);
        assert_eq!(decode(&bytes).unwrap().canonical_bytes().unwrap(), bytes);
    }
    assert_eq!(
        decode(&unhex("3b7fffffffffffffff")),
        Ok(V::Negative(i64::MIN))
    );
    assert_ne!(decode(&unhex("62c3a9")), decode(&unhex("6365cc81")));
    let distinct = [
        V::Map(vec![]),
        V::Map(vec![(V::Unsigned(0), V::Null)]),
        V::Map(vec![(V::Unsigned(0), V::Unsigned(0))]),
        V::Map(vec![(V::Unsigned(0), V::Bool(false))]),
    ];
    let bytes: Vec<_> = distinct
        .iter()
        .map(|v| v.canonical_bytes().unwrap())
        .collect();
    for (i, left) in bytes.iter().enumerate() {
        for right in &bytes[i + 1..] {
            assert_ne!(left, right);
        }
        assert_eq!(decode(left).unwrap().canonical_bytes().unwrap(), *left);
    }
    for len in [23, 24, 255, 256, 65535, 65536] {
        for value in [V::Bytes(vec![0; len]), V::Text("a".repeat(len))] {
            let bytes = value.canonical_bytes().unwrap();
            assert_eq!(decode(&bytes), Ok(value));
        }
    }
}

#[test]
fn compound_keys_and_length_first_order_are_preserved() {
    // One-byte empty array sorts before two-byte integer 24, despite 0x80 > 0x18.
    let bytes = unhex("a280f61818f6");
    assert_eq!(decode(&bytes).unwrap().canonical_bytes().unwrap(), bytes);
    let value = V::Map(vec![
        (
            V::Map(vec![(V::Array(vec![V::Null]), V::Bool(true))]),
            V::Null,
        ),
        (
            V::Array(vec![V::Bytes(vec![1]), V::Text("k".into())]),
            V::Unsigned(9),
        ),
        (V::Bool(false), V::Negative(-1)),
        (V::Null, V::Null),
    ]);
    let bytes = value.canonical_bytes().unwrap();
    assert_eq!(decode(&bytes).unwrap().canonical_bytes().unwrap(), bytes);
    for hex in ["a21818f680f6", "a201f600f6", "a280f680f6", "a181a00081a000"] {
        assert!(decode(&unhex(hex)).is_err(), "{hex}");
    }
    assert_eq!(decode(&unhex("a200f600f6")), Err(E::DuplicateMapKey));
    assert_eq!(decode(&unhex("a201f600f6")), Err(E::MapKeyOrder));
}

#[test]
fn sorted_duplicate_parents_survive_and_future_schema_is_syntactic() {
    let bytes = envelope(vec![[0x44; 32], [0x44; 32]]);
    let parsed = EpisodeCommitment::from_canonical_bytes(&bytes, L::default()).unwrap();
    assert_eq!(parsed.parent_digests(), &[[0x44; 32], [0x44; 32]]);
    assert_eq!(parsed.canonical_bytes().unwrap(), bytes);
    let future = EpisodeCommitment::new(u64::MAX, V::Map(vec![]), V::Map(vec![]), vec![])
        .canonical_bytes()
        .unwrap();
    assert_eq!(
        EpisodeCommitment::from_canonical_bytes(&future, L::default())
            .unwrap()
            .schema_version(),
        u64::MAX
    );
    let mut reversed = envelope(vec![[0x11; 32], [0xee; 32]]);
    let len = reversed.len();
    reversed[len - 66..len - 34].fill(0xee);
    reversed[len - 32..].fill(0x11);
    assert_eq!(
        EpisodeCommitment::from_canonical_bytes(&reversed, L::default()),
        Err(E::ParentOrder)
    );
}

#[test]
fn envelope_shape_failures_are_rejected() {
    let bytes = envelope(vec![]);
    // Root count and frozen key positions; header/payload/parents types, schema/profile.
    for (position, replacement) in [
        (0, 0xa4),
        (1, 0x05),
        (23, 0),
        (25, 0xf6),
        (27, 0xf6),
        (29, 0xf6),
    ] {
        let mut bad = bytes.clone();
        bad[position] = replacement;
        assert!(
            EpisodeCommitment::from_canonical_bytes(&bad, L::default()).is_err(),
            "position {position}"
        );
    }
    let mut wrong_profile = bytes.clone();
    wrong_profile[3] = b'X';
    assert_eq!(
        EpisodeCommitment::from_canonical_bytes(&wrong_profile, L::default()),
        Err(E::ProfileMismatch)
    );
    for parent in [
        V::Bytes(vec![0; 31]),
        V::Bytes(vec![0; 33]),
        V::Text("x".into()),
        V::Array(vec![]),
    ] {
        let value = V::Map(vec![
            (V::Unsigned(0), V::Text("abbey-cbor-episode-v1".into())),
            (V::Unsigned(1), V::Unsigned(1)),
            (V::Unsigned(2), V::Map(vec![])),
            (V::Unsigned(3), V::Map(vec![])),
            (V::Unsigned(4), V::Array(vec![parent])),
        ]);
        assert_eq!(
            EpisodeCommitment::from_canonical_bytes(
                &value.canonical_bytes().unwrap(),
                L::default()
            ),
            Err(E::InvalidParentDigest)
        );
    }
}

#[test]
fn malformed_syntax_is_rejected_without_private_content() {
    for hex in [
        "18",
        "1900",
        "1a000000",
        "1b00000000000000",
        "1817",
        "1900ff",
        "1a0000ffff",
        "1b00000000ffffffff",
        "581700",
        "7800",
        "9800",
        "b800",
        "61ff",
        "3b8000000000000000",
        "1c",
        "1d",
        "1e",
        "1f",
        "5f",
        "7f",
        "9f",
        "bf",
        "c0f6",
        "f7",
        "f8f4",
        "f90000",
        "fa00000000",
        "fb0000000000000000",
        "ff",
        "f0",
    ] {
        assert!(decode(&unhex(hex)).is_err(), "{hex}");
    }
    assert_eq!(decode(&unhex("1817")), Err(E::NonMinimalArgument));
    assert_eq!(decode(&unhex("61ff")), Err(E::InvalidUtf8));
    assert_eq!(
        decode(&unhex("3b8000000000000000")),
        Err(E::IntegerOutOfRange)
    );
    let private = unhex("a2677072697661746500677072697661746501");
    let error = decode(&private).unwrap_err();
    assert!(!format!("{error:?} {error}").contains("private"));
    assert_eq!(decode(&[0, 0]), Err(E::TrailingBytes));
}

#[test]
fn deterministic_mutations_never_panic_and_success_reencodes_exactly() {
    let golden = unhex(include_str!("golden/abbey-cbor-episode-v1/two-parents.hex"));
    for index in 0..golden.len() {
        for replacement in [0, 23, 24, 31, 0x80, 0xa0, 0xff] {
            let mut bytes = golden.clone();
            bytes[index] = replacement;
            if let Ok(value) = decode(&bytes) {
                assert_eq!(value.canonical_bytes().unwrap(), bytes);
            }
            if let Ok(value) = EpisodeCommitment::from_canonical_bytes(&bytes, L::default()) {
                assert_eq!(value.canonical_bytes().unwrap(), bytes);
            }
        }
    }
    let mut seed = 7_u64;
    for len in 0..128 {
        for _ in 0..16 {
            let bytes: Vec<_> = (0..len)
                .map(|_| {
                    seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                    seed.to_le_bytes()[4]
                })
                .collect();
            if let Ok(value) = decode(&bytes) {
                assert_eq!(value.canonical_bytes().unwrap(), bytes);
            }
        }
    }
}

#[test]
fn all_argument_widths_are_minimal_for_integers_and_lengths() {
    // Each literal uses a wider argument than required. No payload allocation or
    // truncation result may mask the canonical width failure.
    for major in 0..=5_u8 {
        for (info, argument) in [
            (24, vec![23]),
            (25, vec![0, 255]),
            (26, vec![0, 0, 255, 255]),
            (27, vec![0, 0, 0, 0, 255, 255, 255, 255]),
        ] {
            let mut bytes = vec![(major << 5) | info];
            bytes.extend(argument);
            assert_eq!(decode(&bytes), Err(E::NonMinimalArgument));
        }
        for info in 28..=31 {
            assert_eq!(
                decode(&[(major << 5) | info]),
                Err(E::InvalidAdditionalInformation)
            );
        }
    }
}

#[test]
fn envelope_exact_members_types_and_root_context_are_enforced() {
    fn fields() -> Vec<(V, V)> {
        vec![
            (V::Unsigned(0), V::Text("abbey-cbor-episode-v1".into())),
            (V::Unsigned(1), V::Unsigned(1)),
            (V::Unsigned(2), V::Map(vec![])),
            (V::Unsigned(3), V::Map(vec![])),
            (V::Unsigned(4), V::Array(vec![])),
        ]
    }
    let parse = |value: V| {
        EpisodeCommitment::from_canonical_bytes(&value.canonical_bytes().unwrap(), L::default())
    };
    for root in [V::Null, V::Array(vec![]), V::Unsigned(0)] {
        assert_eq!(parse(root), Err(E::InvalidEnvelope));
    }
    for missing in 0..5 {
        let mut entries = fields();
        entries.remove(missing);
        assert_eq!(parse(V::Map(entries)), Err(E::InvalidEnvelope));
    }
    let mut extra = fields();
    extra.push((V::Unsigned(5), V::Null));
    assert_eq!(parse(V::Map(extra)), Err(E::InvalidEnvelope));
    for (index, key) in [
        (0, V::Negative(-1)),
        (1, V::Text("1".into())),
        (4, V::Unsigned(5)),
    ] {
        let mut entries = fields();
        entries[index].0 = key;
        assert_eq!(parse(V::Map(entries)), Err(E::InvalidEnvelope));
    }
    for schema in [V::Negative(-1), V::Text("1".into()), V::Null] {
        let mut entries = fields();
        entries[1].1 = schema;
        assert_eq!(parse(V::Map(entries)), Err(E::InvalidEnvelope));
    }
    let mut entries = fields();
    entries[1].1 = V::Unsigned(0);
    assert_eq!(parse(V::Map(entries)), Err(E::ZeroSchemaVersion));
    for (index, value, error) in [
        (0, V::Unsigned(0), E::ProfileMismatch),
        (2, V::Null, E::HeaderMustBeMap),
        (3, V::Array(vec![]), E::PayloadMustBeMap),
        (4, V::Map(vec![]), E::InvalidEnvelope),
    ] {
        let mut entries = fields();
        entries[index].1 = value;
        assert_eq!(parse(V::Map(entries)), Err(error));
    }
    // Unsigned key 4 nested in a header has no parent-array context.
    let mut entries = fields();
    entries[2].1 = V::Map(vec![(V::Unsigned(4), V::Array(vec![V::Null]))]);
    let bytes = V::Map(entries).canonical_bytes().unwrap();
    assert!(
        EpisodeCommitment::from_canonical_bytes(
            &bytes,
            L {
                max_parent_count: 0,
                ..L::default()
            }
        )
        .is_ok()
    );
}
