use super::*;

#[test]
fn all_primitive_boundaries() {
    for value in [
        "",
        "has space",
        "has/slash",
        "back\\slash",
        "unicode-é",
        "punct!",
        &"a".repeat(129),
    ] {
        assert_eq!(
            OpaqueRef::new(value).unwrap_err(),
            CompleteEpisodeError::InvalidIdentifier
        );
    }
    assert!(OpaqueRef::new(&"a".repeat(128)).is_ok());
    assert!(OpaqueRef::new("A-z_0.9:token").is_ok());
    for version in [4, 7] {
        let mut value = uuid();
        value[6] = version << 4;
        assert!(EpisodeId::new(value).is_ok());
        assert!(RequestId::new(value).is_ok());
        assert!(TaskId::new(value).is_ok());
    }
    for version in [0, 1, 2, 3, 5, 6, 8, 15] {
        let mut value = uuid();
        value[6] = version << 4;
        assert!(EpisodeId::new(value).is_err());
    }
    for variant in [0x00, 0x40, 0xc0, 0xff] {
        let mut value = uuid();
        value[8] = variant;
        assert!(EpisodeId::new(value).is_err());
    }
    assert!(UnitPpm::new(0).is_ok());
    assert!(UnitPpm::new(1_000_000).is_ok());
    assert!(UnitPpm::new(u32::MAX).is_err());
    for unit in [MeasurementUnit::Count, MeasurementUnit::Bytes] {
        for value in [-1, 1, 999_999, i64::MIN, i64::MAX] {
            assert!(Measurement::new(unit, SignedMicros::new(value)).is_err());
        }
        for value in [0, 1_000_000, 9_223_372_036_854_000_000] {
            assert!(Measurement::new(unit, SignedMicros::new(value)).is_ok());
        }
    }
    for unit in [
        MeasurementUnit::Unitless,
        MeasurementUnit::Milliseconds,
        MeasurementUnit::Microjoules,
    ] {
        for value in [i64::MIN, -1, 0, 1, i64::MAX] {
            assert!(Measurement::new(unit, SignedMicros::new(value)).is_ok());
        }
    }
    assert_eq!(
        encode_test(&Digest32::new([0; 32])),
        [vec![0x58, 32], vec![0; 32]].concat()
    );
}

#[test]
fn integer_shortest_forms_and_checked_size_agree() {
    for (n, expected) in [
        (0, vec![0]),
        (23, vec![23]),
        (24, vec![0x18, 24]),
        (255, vec![0x18, 255]),
        (256, vec![0x19, 1, 0]),
        (65535, vec![0x19, 255, 255]),
        (65536, vec![0x1a, 0, 1, 0, 0]),
        (u64::from(u32::MAX), vec![0x1a, 255, 255, 255, 255]),
        (u64::from(u32::MAX) + 1, vec![0x1b, 0, 0, 0, 1, 0, 0, 0, 0]),
        (u64::MAX, vec![0x1b, 255, 255, 255, 255, 255, 255, 255, 255]),
    ] {
        assert_eq!(encode_test(&n), expected);
        assert_eq!(counted_size(&n, 32).unwrap(), expected.len());
    }
    for (n, expected) in [
        (
            i64::MIN,
            vec![0x3b, 0x7f, 255, 255, 255, 255, 255, 255, 255],
        ),
        (-25, vec![0x38, 24]),
        (-24, vec![0x37]),
        (-1, vec![0x20]),
        (0, vec![0]),
        (
            i64::MAX,
            vec![0x1b, 0x7f, 255, 255, 255, 255, 255, 255, 255],
        ),
    ] {
        assert_eq!(encode_test(&SignedMicros::new(n)), expected);
        assert_eq!(
            counted_size(&SignedMicros::new(n), 32).unwrap(),
            expected.len()
        );
    }
    for input in [minimal(), populated()] {
        let size = counted_size(&input, MAX_ENCODED_SIZE).unwrap();
        assert_eq!(size, bytes(input.clone()).len());
        assert_eq!(counted_size(&input, size).unwrap(), size);
        assert_eq!(
            counted_size(&input, size - 1).unwrap_err(),
            CompleteEpisodeError::EncodedSizeLimit
        );
    }
    assert!(super::super::canonical::sum(usize::MAX, [1]).is_err());
}

#[test]
fn posterior_cardinality_and_exact_sum() {
    let entries = |n| {
        (0..n)
            .map(|i| {
                (
                    RegimeRef::new(&format!("r{i:02}")).unwrap(),
                    UnitPpm::new(if i == 0 { 1_000_000 } else { 0 }).unwrap(),
                )
            })
            .collect()
    };
    for n in [1, 32] {
        assert!(RegimeDistribution::new(entries(n)).is_ok());
    }
    for n in [0, 33] {
        assert!(RegimeDistribution::new(entries(n)).is_err());
    }
    for n in [999_999, 1_000_001] {
        let entries = vec![
            (RegimeRef::new("a").unwrap(), UnitPpm::new(n / 2).unwrap()),
            (
                RegimeRef::new("b").unwrap(),
                UnitPpm::new(n - n / 2).unwrap(),
            ),
        ];
        assert!(RegimeDistribution::new(entries).is_err());
    }
    assert!(
        RegimeDistribution::new(vec![
            (
                RegimeRef::new("a").unwrap(),
                UnitPpm::new(500_000).unwrap()
            );
            2
        ])
        .is_err()
    );
    let posterior = RegimeDistribution::new(vec![
        (RegimeRef::new("z").unwrap(), UnitPpm::new(0).unwrap()),
        (
            RegimeRef::new("a").unwrap(),
            UnitPpm::new(1_000_000).unwrap(),
        ),
    ])
    .unwrap();
    assert_eq!(
        encode_test(&posterior),
        vec![
            0x82, 0x82, 0x61, b'a', 0x1a, 0, 0x0f, 0x42, 0x40, 0x82, 0x61, b'z', 0
        ]
    );
}

#[test]
fn temporal_and_artifact_boundaries() {
    let clock = ClockRef::new("clock").unwrap();
    assert!(MonotonicRange::new((clock.clone(), MonoNanos::new(2), MonoNanos::new(1))).is_err());
    assert!(
        MonotonicRange::new((clock, MonoNanos::new(u64::MAX), MonoNanos::new(u64::MAX))).is_ok()
    );
    for retention in [
        RetentionIntent::Operational {
            expires_at: UtcMillis::new(99),
        },
        RetentionIntent::Session {
            session: OpaqueRef::new("session").unwrap(),
            expires_at: UtcMillis::new(99),
        },
    ] {
        let mut input = minimal();
        input.lifecycle.retention = retention;
        assert_eq!(
            CompleteEpisode::try_new(input).unwrap_err(),
            CompleteEpisodeError::InvalidRange
        );
    }
    let mut input = populated();
    input.context.summary.as_mut().unwrap().byte_length = 1 << 40;
    assert!(CompleteEpisode::try_new(input.clone()).is_ok());
    input.context.summary.as_mut().unwrap().byte_length += 1;
    assert_eq!(
        CompleteEpisode::try_new(input).unwrap_err(),
        CompleteEpisodeError::InvalidRange
    );
    let mut input = minimal();
    input.time.received_at = UtcMillis::new(u64::MAX);
    input.lifecycle.retention = RetentionIntent::Operational {
        expires_at: UtcMillis::new(u64::MAX),
    };
    assert_eq!(
        CompleteEpisode::try_new(input).unwrap_err(),
        CompleteEpisodeError::InvalidRange
    );
}
