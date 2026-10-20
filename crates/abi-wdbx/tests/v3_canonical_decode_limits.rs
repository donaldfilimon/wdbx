//! Exact resource and depth boundaries for the bounded canonical reader.
use abi_wdbx::v3::commitment::{
    CanonicalDecodeError as E, CanonicalDecodeLimits as L, CanonicalValue as V, EpisodeCommitment,
};
fn decode(bytes: &[u8], limits: L) -> Result<V, E> {
    V::from_canonical_bytes(bytes, limits)
}
fn episode(count: usize) -> Vec<u8> {
    EpisodeCommitment::new(1, V::Map(vec![]), V::Map(vec![]), vec![[1; 32]; count])
        .canonical_bytes()
        .unwrap()
}
#[test]
fn input_and_global_value_caps_include_root_keys_and_values() {
    assert!(
        decode(
            &[0],
            L {
                max_input_bytes: 1,
                max_value_count: 1,
                ..L::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        decode(
            &[0],
            L {
                max_input_bytes: 0,
                ..L::default()
            }
        ),
        Err(E::InputLimit)
    );
    assert_eq!(
        decode(
            &[0],
            L {
                max_value_count: 0,
                ..L::default()
            }
        ),
        Err(E::ValueLimit)
    );
    // Map + key array + its scalar + value array + its scalar = five values.
    let bytes = [0xa1, 0x81, 0, 0x81, 0];
    assert!(
        decode(
            &bytes,
            L {
                max_value_count: 5,
                max_input_bytes: 5,
                ..L::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        decode(
            &bytes,
            L {
                max_value_count: 4,
                ..L::default()
            }
        ),
        Err(E::ValueLimit)
    );
    assert_eq!(
        decode(
            &bytes,
            L {
                max_input_bytes: 4,
                ..L::default()
            }
        ),
        Err(E::InputLimit)
    );
    let bytes = episode(0);
    assert!(
        EpisodeCommitment::from_canonical_bytes(
            &bytes,
            L {
                max_value_count: 11,
                ..L::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        EpisodeCommitment::from_canonical_bytes(
            &bytes,
            L {
                max_value_count: 10,
                ..L::default()
            }
        ),
        Err(E::ValueLimit)
    );
}
#[test]
fn collection_caps_accept_exact_and_reject_next() {
    for bytes in [&[0x82, 0, 0][..], &[0xa2, 0, 0, 1, 0][..]] {
        assert!(
            decode(
                bytes,
                L {
                    max_collection_entries: 2,
                    ..L::default()
                }
            )
            .is_ok()
        );
        assert_eq!(
            decode(
                bytes,
                L {
                    max_collection_entries: 1,
                    ..L::default()
                }
            ),
            Err(E::CollectionLimit)
        );
    }
}

#[test]
fn string_caps_and_zero_collection_caps_accept_exact_and_reject_next() {
    for bytes in [&[0x42, 0, 0][..], &[0x62, 0xc3, 0xa9][..]] {
        assert!(
            decode(
                bytes,
                L {
                    max_string_bytes: 2,
                    ..L::default()
                }
            )
            .is_ok()
        );
        assert_eq!(
            decode(
                bytes,
                L {
                    max_string_bytes: 1,
                    ..L::default()
                }
            ),
            Err(E::StringLimit)
        );
    }
    for bytes in [&[0x40][..], &[0x60][..]] {
        assert!(
            decode(
                bytes,
                L {
                    max_string_bytes: 0,
                    ..L::default()
                }
            )
            .is_ok()
        );
    }
    for bytes in [&[0x80][..], &[0xa0][..]] {
        assert!(
            decode(
                bytes,
                L {
                    max_collection_entries: 0,
                    ..L::default()
                }
            )
            .is_ok()
        );
    }
    assert_eq!(
        decode(
            &[0x81, 0],
            L {
                max_collection_entries: 0,
                ..L::default()
            }
        ),
        Err(E::CollectionLimit)
    );
    assert_eq!(
        decode(
            &[0x41, 0],
            L {
                max_string_bytes: 0,
                ..L::default()
            }
        ),
        Err(E::StringLimit)
    );
}

#[test]
fn parent_caps_apply_before_children_only_in_episode_context() {
    let bytes = episode(2);
    assert!(
        EpisodeCommitment::from_canonical_bytes(
            &bytes,
            L {
                max_parent_count: 2,
                ..L::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        EpisodeCommitment::from_canonical_bytes(
            &bytes,
            L {
                max_parent_count: 1,
                ..L::default()
            }
        ),
        Err(E::ParentLimit)
    );
    assert!(
        EpisodeCommitment::from_canonical_bytes(
            &episode(0),
            L {
                max_parent_count: 0,
                ..L::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        EpisodeCommitment::from_canonical_bytes(
            &episode(1),
            L {
                max_parent_count: 0,
                ..L::default()
            }
        ),
        Err(E::ParentLimit)
    );
    assert!(
        decode(
            &bytes,
            L {
                max_parent_count: 0,
                ..L::default()
            }
        )
        .is_ok()
    );
    // Root parents declare two items without providing children: parent policy wins
    // before any attempt to reserve or parse those declared children.
    let mut truncated = episode(0);
    *truncated.last_mut().unwrap() = 0x82;
    assert_eq!(
        EpisodeCommitment::from_canonical_bytes(
            &truncated,
            L {
                max_parent_count: 1,
                ..L::default()
            }
        ),
        Err(E::ParentLimit)
    );
}
#[test]
fn giant_declared_lengths_fail_with_tiny_input_even_under_large_policy() {
    let generous = L {
        max_input_bytes: usize::MAX,
        max_value_count: usize::MAX,
        max_collection_entries: usize::MAX,
        max_string_bytes: usize::MAX,
        max_parent_count: usize::MAX,
    };
    for major in [0x5b, 0x7b, 0x9b, 0xbb] {
        let mut bytes = vec![major];
        bytes.extend_from_slice(&u64::MAX.to_be_bytes());
        assert!(decode(&bytes, generous).is_err());
    }
    for major in [0x58, 0x78, 0x98, 0xb8] {
        assert!(decode(&[major, 24], generous).is_err());
    }
    assert_eq!(
        decode(
            &[0x98, 24],
            L {
                max_value_count: 1,
                ..L::default()
            }
        ),
        Err(E::ValueLimit)
    );
    assert_eq!(
        decode(
            &[0xb8, 24],
            L {
                max_value_count: 48,
                ..L::default()
            }
        ),
        Err(E::ValueLimit)
    );
}
fn nested(depth: usize, leaf: V) -> V {
    (0..depth).fold(leaf, |v, _| V::Array(vec![v]))
}
#[test]
fn root_zero_depth_boundary_counts_leaves_empty_containers_and_map_keys() {
    for leaf in [V::Null, V::Array(vec![]), V::Map(vec![])] {
        let bytes = nested(32, leaf).canonical_bytes().unwrap();
        assert!(decode(&bytes, L::default()).is_ok());
        let mut too_deep = vec![0x81];
        too_deep.extend(bytes);
        assert_eq!(decode(&too_deep, L::default()), Err(E::NestingLimit));
    }
    let mut map_key = vec![0xa1];
    map_key.extend(vec![0x81; 31]);
    map_key.extend([0xf6, 0]);
    assert!(decode(&map_key, L::default()).is_ok());
    map_key.insert(1, 0x81);
    assert_eq!(decode(&map_key, L::default()), Err(E::NestingLimit));
    let header = V::Map(vec![(V::Unsigned(0), nested(30, V::Null))]);
    let bytes = EpisodeCommitment::new(1, header, V::Map(vec![]), vec![])
        .canonical_bytes()
        .unwrap();
    assert!(EpisodeCommitment::from_canonical_bytes(&bytes, L::default()).is_ok());
    // Header's map value begins at depth two, so one extra array makes leaf 33.
    let mut bad = bytes;
    let pos = bad.iter().position(|byte| *byte == 0x81).unwrap();
    bad.insert(pos, 0x81);
    assert_eq!(
        EpisodeCommitment::from_canonical_bytes(&bad, L::default()),
        Err(E::NestingLimit)
    );
}

#[test]
fn operational_defaults_bound_real_inputs_at_exact_thresholds() {
    // Independent string cap: raise only input cap so its overhead cannot hide it.
    let string_policy = L {
        max_input_bytes: 1_048_582,
        ..L::default()
    };
    assert!(
        decode(
            &V::Bytes(vec![0; 1_048_576]).canonical_bytes().unwrap(),
            string_policy
        )
        .is_ok()
    );
    assert_eq!(
        decode(
            &V::Bytes(vec![0; 1_048_577]).canonical_bytes().unwrap(),
            string_policy
        ),
        Err(E::StringLimit)
    );
    // Exactly one MiB total input, including the five-byte string prefix.
    assert!(
        decode(
            &V::Bytes(vec![0; 1_048_571]).canonical_bytes().unwrap(),
            L::default()
        )
        .is_ok()
    );
    assert_eq!(
        decode(
            &V::Bytes(vec![0; 1_048_572]).canonical_bytes().unwrap(),
            L::default()
        ),
        Err(E::InputLimit)
    );
    assert!(
        decode(
            &V::Array(vec![V::Null; 16_384]).canonical_bytes().unwrap(),
            L::default()
        )
        .is_ok()
    );
    assert_eq!(
        decode(
            &V::Array(vec![V::Null; 16_385]).canonical_bytes().unwrap(),
            L::default()
        ),
        Err(E::CollectionLimit)
    );
    // 1 root + 4 nested arrays + 65,531 leaves = 65,536 values.
    let mut groups = vec![V::Array(vec![V::Null; 16_383]); 3];
    groups.push(V::Array(vec![V::Null; 16_382]));
    assert!(
        decode(
            &V::Array(groups.clone()).canonical_bytes().unwrap(),
            L::default()
        )
        .is_ok()
    );
    let V::Array(last) = groups.last_mut().unwrap() else {
        unreachable!()
    };
    last.push(V::Null);
    assert_eq!(
        decode(&V::Array(groups).canonical_bytes().unwrap(), L::default()),
        Err(E::ValueLimit)
    );
    assert!(EpisodeCommitment::from_canonical_bytes(&episode(4_096), L::default()).is_ok());
    assert_eq!(
        EpisodeCommitment::from_canonical_bytes(&episode(4_097), L::default()),
        Err(E::ParentLimit)
    );
}
