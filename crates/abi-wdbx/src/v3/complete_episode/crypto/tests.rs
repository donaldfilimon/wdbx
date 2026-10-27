use super::profile::*;
use crate::v3::commitment::CanonicalDecodeLimits;
use sha2::{Digest, Sha256};
fn payload() -> Vec<u8> {
    let text = include_str!("../tests/provisional-minimal.hex").trim();
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
        .collect()
}
fn protected() -> Vec<u8> {
    let mut v = vec![0xa2, 1, 0x32, 4, 0x58, 72];
    v.extend_from_slice(format!("ed25519:{}", "0".repeat(64)).as_bytes());
    v
}
fn frame(header: &[u8]) -> Vec<u8> {
    let mut v = vec![0xd2, 0x84, 0x58, header.len() as u8];
    v.extend_from_slice(header);
    v.extend_from_slice(&[0xa0, 0xf6, 0x58, 64]);
    v.extend_from_slice(&[7; 64]);
    v
}
#[test]
fn exact_p1_preserves_originals_and_sig_structure() {
    let payload = payload();
    let header = protected();
    let container = frame(&header);
    let parsed = parse_p1(&container, &payload, CryptoLimits::default()).unwrap();
    assert_eq!(parsed.container(), container);
    assert_eq!(parsed.payload(), payload);
    assert_eq!(parsed.protected(), header);
    assert_eq!(parsed.signature(), &[7; 64]);
    let structure = parsed.sig_structure();
    assert_eq!(&structure[..12], b"\x84\x6aSignature1");
    assert!(
        structure
            .windows(PROFILE_ID.len())
            .any(|v| v == PROFILE_ID.as_bytes())
    );
    assert!(structure.ends_with(&payload));
    // Independently built using Python's standard-library byte encoding and
    // hashlib, with the frozen minimal payload and this synthetic header.
    assert_eq!(structure.len(), 1215);
    assert_eq!(
        Sha256::digest(structure)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "eab22dc037177ff98caac7c22cba397014bdb041d756bd03f8f9c3ae8521f302"
    );
    assert_eq!(format!("{parsed:?}"), "ParsedP1 { .. }");
}
#[test]
fn every_container_truncation_and_trailing_bytes_fail() {
    let payload = payload();
    let container = frame(&protected());
    for end in 0..container.len() {
        assert!(
            parse_p1(&container[..end], &payload, CryptoLimits::default()).is_err(),
            "{end}"
        );
    }
    let mut extra = container;
    extra.push(0);
    assert!(parse_p1(&extra, &payload, CryptoLimits::default()).is_err());
}
#[test]
fn profile_shape_is_closed() {
    let payload = payload();
    let original = frame(&protected());
    for (index, byte) in [
        (0, 0xd3),
        (1, 0x83),
        (2, 0x5f),
        (82, 0xa1),
        (83, 0x40),
        (84, 0x59),
    ] {
        let mut v = original.clone();
        v[index] = byte;
        assert!(
            parse_p1(&v, &payload, CryptoLimits::default()).is_err(),
            "{index}"
        );
    }
    let mut header = protected();
    header[2] = 0x27; // legacy -8
    assert_eq!(
        parse_p1(&frame(&header), &payload, CryptoLimits::default()).unwrap_err(),
        ProfileError::UnsupportedAlgorithm
    );
    header = protected();
    header[0] = 0xa3;
    assert!(parse_p1(&frame(&header), &payload, CryptoLimits::default()).is_err());
    header = protected();
    header[1] = 4;
    assert!(parse_p1(&frame(&header), &payload, CryptoLimits::default()).is_err());
    let mut nonminimal = vec![0xd8, 18];
    nonminimal.extend_from_slice(&original[1..]);
    assert_eq!(
        parse_p1(&nonminimal, &payload, CryptoLimits::default()).unwrap_err(),
        ProfileError::NonCanonicalEncoding
    );
}
#[test]
fn bounds_and_zero_caller_limits_are_respected() {
    let payload = payload();
    let container = frame(&protected());
    let base = CryptoLimits::default();
    for limits in [
        CryptoLimits {
            max_container_bytes: 0,
            ..base
        },
        CryptoLimits {
            max_protected_bytes: 0,
            ..base
        },
        CryptoLimits {
            max_payload_bytes: 0,
            ..base
        },
        CryptoLimits {
            max_combined_bytes: 0,
            ..base
        },
        CryptoLimits {
            max_sig_structure_bytes: 0,
            ..base
        },
    ] {
        assert_eq!(
            parse_p1(&container, &payload, limits).unwrap_err(),
            ProfileError::ResourceLimit
        );
    }
    let mut limits = base;
    limits.payload_limits.max_value_count = 0;
    assert_eq!(
        parse_p1(&container, &payload, limits).unwrap_err(),
        ProfileError::ResourceLimit
    );
    let limits = CryptoLimits {
        max_container_bytes: container.len(),
        max_protected_bytes: protected().len(),
        max_payload_bytes: payload.len(),
        max_combined_bytes: container.len() + payload.len(),
        ..base
    };
    assert!(parse_p1(&container, &payload, limits).is_ok());
    for limits in [
        CryptoLimits {
            max_container_bytes: container.len() - 1,
            ..base
        },
        CryptoLimits {
            max_payload_bytes: payload.len() - 1,
            ..base
        },
        CryptoLimits {
            max_combined_bytes: container.len() + payload.len() - 1,
            ..base
        },
    ] {
        assert_eq!(
            parse_p1(&container, &payload, limits).unwrap_err(),
            ProfileError::ResourceLimit
        );
    }
    assert_eq!(
        parse_p1(&vec![0; 257], &payload, CryptoLimits::default()).unwrap_err(),
        ProfileError::ResourceLimit
    );
    assert_eq!(
        parse_p1(&container, &vec![0; 1_048_577], CryptoLimits::default()).unwrap_err(),
        ProfileError::ResourceLimit
    );
}
#[test]
fn malformed_payload_and_static_diagnostics() {
    let container = frame(&protected());
    for payload in [vec![], vec![0xa0], vec![0xc0, 0xa0]] {
        assert!(parse_p1(&container, &payload, CryptoLimits::default()).is_err());
    }
    for error in [
        ProfileError::ResourceLimit,
        ProfileError::MalformedCbor,
        ProfileError::NonCanonicalEncoding,
        ProfileError::InvalidHeaderSet,
        ProfileError::InvalidLength,
        ProfileError::InvalidPayload,
        ProfileError::UnsupportedAlgorithm,
    ] {
        assert!(!error.to_string().contains("ed25519"));
        assert!(error.to_string().len() < 70);
    }
}

#[test]
fn exact_structure_budget_and_oversized_declared_lengths() {
    let payload = payload();
    let container = frame(&protected());
    for (cap, accepted) in [(1214, false), (1215, true)] {
        let limits = CryptoLimits {
            max_sig_structure_bytes: cap,
            ..CryptoLimits::default()
        };
        assert_eq!(parse_p1(&container, &payload, limits).is_ok(), accepted);
    }
    let oversized = [vec![0xd2, 0x84, 0x5b], u64::MAX.to_be_bytes().to_vec()].concat();
    assert_eq!(
        parse_p1(&oversized, &payload, CryptoLimits::default()).unwrap_err(),
        ProfileError::ResourceLimit
    );
    let mut extra_header = protected();
    extra_header.push(0);
    assert!(parse_p1(&frame(&extra_header), &payload, CryptoLimits::default()).is_err());
    let limits = CryptoLimits {
        max_payload_bytes: usize::MAX,
        ..CryptoLimits::default()
    };
    assert_eq!(
        parse_p1(&container, &vec![0; 1_048_577], limits).unwrap_err(),
        ProfileError::ResourceLimit
    );
}

#[test]
fn malformed_small_corpus_is_panic_free() {
    let payload = payload();
    for byte in 0..=255 {
        for len in 0..=8 {
            let _ = parse_p1(&vec![byte; len], &payload, CryptoLimits::default());
        }
    }
    let mut payload_with_tail = payload.clone();
    payload_with_tail.push(0);
    assert!(
        parse_p1(
            &frame(&protected()),
            &payload_with_tail,
            CryptoLimits::default()
        )
        .is_err()
    );
}

#[test]
fn each_payload_decoder_limit_keeps_its_zero_semantics() {
    let payload = payload();
    let container = frame(&protected());
    let defaults = CanonicalDecodeLimits::default();
    for payload_limits in [
        CanonicalDecodeLimits {
            max_input_bytes: 0,
            ..defaults
        },
        CanonicalDecodeLimits {
            max_value_count: 0,
            ..defaults
        },
        CanonicalDecodeLimits {
            max_collection_entries: 0,
            ..defaults
        },
        CanonicalDecodeLimits {
            max_string_bytes: 0,
            ..defaults
        },
    ] {
        assert_eq!(
            parse_p1(
                &container,
                &payload,
                CryptoLimits {
                    payload_limits,
                    ..CryptoLimits::default()
                }
            )
            .unwrap_err(),
            ProfileError::ResourceLimit
        );
    }
    // Parent count belongs to the old episode envelope; complete episodes
    // deliberately retain the typed decoder's inert interpretation.
    let payload_limits = CanonicalDecodeLimits {
        max_parent_count: 0,
        ..defaults
    };
    assert!(
        parse_p1(
            &container,
            &payload,
            CryptoLimits {
                payload_limits,
                ..CryptoLimits::default()
            }
        )
        .is_ok()
    );
}

#[test]
fn protected_budget_and_closed_header_labels_are_explicit() {
    let payload = payload();
    let header = protected();
    let container = frame(&header);
    for (cap, expected) in [(header.len(), true), (header.len() - 1, false)] {
        let result = parse_p1(
            &container,
            &payload,
            CryptoLimits {
                max_protected_bytes: cap,
                ..CryptoLimits::default()
            },
        );
        if expected {
            assert!(result.is_ok());
        } else {
            assert_eq!(result.unwrap_err(), ProfileError::ResourceLimit);
        }
    }
    for second_label in [1, 5] {
        let mut changed = header.clone();
        changed[3] = second_label; // duplicate algorithm or unknown label
        assert_eq!(
            parse_p1(&frame(&changed), &payload, CryptoLimits::default()).unwrap_err(),
            ProfileError::InvalidHeaderSet
        );
    }
}

#[test]
fn oversized_caller_budgets_cannot_raise_hard_caps() {
    let limits = CryptoLimits {
        max_container_bytes: usize::MAX,
        max_protected_bytes: usize::MAX,
        max_payload_bytes: usize::MAX,
        max_combined_bytes: usize::MAX,
        max_sig_structure_bytes: usize::MAX,
        payload_limits: CanonicalDecodeLimits {
            max_input_bytes: usize::MAX,
            max_value_count: usize::MAX,
            max_collection_entries: usize::MAX,
            max_string_bytes: usize::MAX,
            max_parent_count: usize::MAX,
        },
    };
    let container = frame(&protected());
    let parsed = parse_p1(&container, &payload(), limits).unwrap();
    assert_eq!(parsed.sig_structure().len(), 1215);
    assert_eq!(
        parse_p1(&vec![0; 257], &payload(), limits).unwrap_err(),
        ProfileError::ResourceLimit
    );
    assert_eq!(
        parse_p1(&container, &vec![0; 1_048_577], limits).unwrap_err(),
        ProfileError::ResourceLimit
    );
    let oversized_header = [vec![0xd2, 0x84, 0x58, 129], vec![0; 129]].concat();
    assert_eq!(
        parse_p1(&oversized_header, &payload(), limits).unwrap_err(),
        ProfileError::ResourceLimit
    );
    // Declared collection size is rejected before allocation/typed shape.
    assert_eq!(
        parse_p1(&container, &[0x99, 0x40, 1], limits).unwrap_err(),
        ProfileError::ResourceLimit
    );
    // Total values exceed 65,536 while each collection stays within 16,384.
    let mut many_values = vec![0x99, 0x40, 0];
    for _ in 0..16_384 {
        many_values.extend_from_slice(&[0x84, 0, 0, 0, 0]);
    }
    assert_eq!(
        parse_p1(&container, &many_values, limits).unwrap_err(),
        ProfileError::ResourceLimit
    );
    // Byte-string length is bounded even without providing its bytes.
    assert_eq!(
        parse_p1(&container, &[0x5a, 0, 0x10, 0, 1], limits).unwrap_err(),
        ProfileError::ResourceLimit
    );
}

#[test]
fn depth_32_is_syntax_only_while_depth_33_exhausts_the_reader() {
    let container = frame(&protected());
    for (depth, expected) in [
        (32, ProfileError::InvalidPayload),
        (33, ProfileError::ResourceLimit),
    ] {
        let mut nested = vec![0x81; depth];
        nested.push(0);
        // These are arrays, not complete episodes. Depth 32 passes syntax
        // then fails typed shape; depth 33 fails the fixed parser depth cap.
        assert_eq!(
            parse_p1(&container, &nested, CryptoLimits::default()).unwrap_err(),
            expected
        );
    }
}
