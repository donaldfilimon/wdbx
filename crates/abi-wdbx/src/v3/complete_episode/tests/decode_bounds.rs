use super::decode_malformed::{decode, field, group, items, tree};
use super::*;
use crate::v3::commitment::{
    CanonicalDecodeError as CE, CanonicalDecodeLimits as Limits, CanonicalValue as C,
};
use CompleteEpisodeDecodeError as E;
fn raw(bytes: &[u8], limits: Limits) -> std::result::Result<DecodedCompleteEpisode, E> {
    DecodedCompleteEpisode::from_canonical_bytes(bytes, limits)
}
#[test]
fn independent_generic_limits_zero_and_exact_input() {
    let bytes = bytes(populated());
    assert!(
        raw(
            &bytes,
            Limits {
                max_input_bytes: bytes.len(),
                max_parent_count: 0,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    for (limits, expected) in [
        (
            Limits {
                max_input_bytes: bytes.len() - 1,
                ..Limits::default()
            },
            CE::InputLimit,
        ),
        (
            Limits {
                max_input_bytes: 0,
                ..Limits::default()
            },
            CE::InputLimit,
        ),
        (
            Limits {
                max_value_count: 0,
                ..Limits::default()
            },
            CE::ValueLimit,
        ),
        (
            Limits {
                max_collection_entries: 0,
                ..Limits::default()
            },
            CE::CollectionLimit,
        ),
        (
            Limits {
                max_string_bytes: 0,
                ..Limits::default()
            },
            CE::StringLimit,
        ),
        (
            Limits {
                max_value_count: 12,
                ..Limits::default()
            },
            CE::ValueLimit,
        ),
        (
            Limits {
                max_collection_entries: 11,
                ..Limits::default()
            },
            CE::CollectionLimit,
        ),
        (
            Limits {
                max_string_bytes: 15,
                ..Limits::default()
            },
            CE::StringLimit,
        ),
    ] {
        assert_eq!(raw(&bytes, limits).unwrap_err(), E::Canonical(expected));
    }
    // This is input-clamp evidence, not a valid encoder production-size fixture.
    assert_eq!(
        raw(
            &vec![0; MAX_ENCODED_SIZE + 1],
            Limits {
                max_input_bytes: usize::MAX,
                ..Limits::default()
            }
        )
        .unwrap_err(),
        E::Canonical(CE::InputLimit)
    );
}
#[test]
fn hostile_syntax_and_finite_nonpanic_corpus() {
    for (bytes, expected) in [
        (vec![], CE::UnexpectedEnd),
        (vec![0, 0], CE::TrailingBytes),
        (vec![0x18, 0], CE::NonMinimalArgument),
        (vec![0x58, 0], CE::NonMinimalArgument),
        (vec![0x9f], CE::InvalidAdditionalInformation),
        (vec![0x1c], CE::InvalidAdditionalInformation),
        (vec![0xc0, 0], CE::UnsupportedType),
        (vec![0xf9, 0, 0], CE::UnsupportedType),
        (vec![0x61, 0xff], CE::InvalidUtf8),
        (vec![0x58, 32, 0], CE::UnexpectedEnd),
        (vec![0xa2, 0, 0, 0, 1], CE::DuplicateMapKey),
        (vec![0xa2, 1, 0, 0, 0], CE::MapKeyOrder),
        (
            vec![0x3b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            CE::IntegerOutOfRange,
        ),
    ] {
        assert_eq!(
            raw(&bytes, Limits::default()).unwrap_err(),
            E::Canonical(expected)
        );
    }
    let mut nested = vec![0x81; 33];
    nested.push(0);
    assert_eq!(
        raw(&nested, Limits::default()).unwrap_err(),
        E::Canonical(CE::NestingLimit)
    );
    for major in [0x5b, 0x7b, 0x9b, 0xbb] {
        let mut bytes = vec![major];
        bytes.extend([0xff; 8]);
        assert!(raw(&bytes, Limits::default()).is_err());
    }
    let good = bytes(minimal());
    for cut in 0..good.len() {
        assert!(raw(&good[..cut], Limits::default()).is_err());
    }
    for byte in 0..=255u8 {
        assert!(std::panic::catch_unwind(|| raw(&[byte], Limits::default())).is_ok());
    }
    let mut trailing = good;
    trailing.push(0);
    assert_eq!(
        raw(&trailing, Limits::default()).unwrap_err(),
        E::Canonical(CE::TrailingBytes)
    );
}
#[test]
fn every_typed_list_cap_before_child_conversion() {
    for (g, k, cap) in [
        (6, "actions", 128),
        (5, "resources", 16),
        (7, "costs", 16),
        (8, "capabilities", 32),
        (8, "models", 32),
        (8, "calibrations", 32),
        (8, "approvals", 64),
        (10, "holds", 64),
        (10, "edges", 64),
        (11, "causal_parents", 64),
        (11, "consulted_parents", 256),
        (9, "source_refs", 256),
        (9, "observation_refs", 256),
        (11, "records", 256),
    ] {
        let mut v = tree(minimal());
        *field(group(&mut v, g), k) = C::Array(vec![C::Null; cap + 1]);
        assert_eq!(
            decode(v).unwrap_err(),
            E::InvalidValue(CompleteEpisodeError::CollectionLimit)
        );
    }
    let mut v = tree(populated());
    *field(
        &mut items(field(group(&mut v, 6), "actions"))[0],
        "parameter_classes",
    ) = C::Array(vec![C::Null; 33]);
    assert_eq!(
        decode(v).unwrap_err(),
        E::InvalidValue(CompleteEpisodeError::CollectionLimit)
    );
    for n in [0, 33] {
        let mut v = tree(minimal());
        *field(group(&mut v, 5), "regime_posterior") =
            C::Array(vec![C::Unsigned(1), C::Array(vec![C::Null; n])]);
        assert_eq!(
            decode(v).unwrap_err(),
            E::InvalidValue(CompleteEpisodeError::InvalidPosterior)
        );
    }
}
fn unique_local(index: usize) -> LocalEpisodeRef {
    let mut value = local(0);
    let mut digest = [0; 32];
    digest[..8].copy_from_slice(&(index as u64).to_be_bytes());
    value.digest = Digest32::new(digest);
    value
}
fn unique_artifact(kind: ArtifactKind, index: usize) -> ArtifactRef {
    let mut value = artifact(kind, 0);
    value.digest = unique_local(index).digest;
    value
}
fn cap_error(input: CompleteEpisodeInput, expected: CompleteEpisodeError) {
    let wire = input.wire::<super::super::canonical::Tree>().unwrap();
    assert_eq!(decode(wire).unwrap_err(), E::InvalidValue(expected));
}
#[test]
fn realizable_positive_typed_caps_and_cap_plus_one() {
    let mut input = populated();
    let action = input.decision.actions[0].clone();
    input.decision.actions = vec![action.clone(); 128];
    super::decode_roundtrip::check(input.clone());
    input.decision.actions.push(action);
    cap_error(input, CompleteEpisodeError::CollectionLimit);
    let mut input = populated();
    input.decision.actions[0].parameter_classes = (0..32)
        .map(|i| ParameterClassRef::new(&format!("p{i:02}")).unwrap())
        .collect();
    super::decode_roundtrip::check(input.clone());
    input.decision.actions[0]
        .parameter_classes
        .push(ParameterClassRef::new("p32").unwrap());
    cap_error(input, CompleteEpisodeError::CollectionLimit);
    for select in [
        (|i: &mut CompleteEpisodeInput| &mut i.authority.capabilities)
            as fn(&mut CompleteEpisodeInput) -> &mut Vec<VersionRef>,
        |i| &mut i.authority.models,
        |i| &mut i.authority.calibrations,
    ] {
        let mut input = populated();
        *select(&mut input) = (0..32).map(|i| version(&format!("v{i:02}"))).collect();
        super::decode_roundtrip::check(input.clone());
        select(&mut input).push(version("v32"));
        cap_error(input, CompleteEpisodeError::CollectionLimit);
    }
    for select in [
        (|i: &mut CompleteEpisodeInput| &mut i.authority.approvals)
            as fn(&mut CompleteEpisodeInput) -> &mut Vec<LocalEpisodeRef>,
        |i| &mut i.lifecycle.holds,
        |i| &mut i.provenance.causal_parents,
    ] {
        let mut input = populated();
        *select(&mut input) = (0..64).map(unique_local).collect();
        super::decode_roundtrip::check(input.clone());
        select(&mut input).push(unique_local(64));
        cap_error(input, CompleteEpisodeError::CollectionLimit);
    }
    let mut input = populated();
    input.lifecycle.edges = (0..64)
        .map(|i| LifecycleLink::new((LifecycleRelation::Supersedes, unique_local(i), local(250))))
        .collect();
    super::decode_roundtrip::check(input.clone());
    input.lifecycle.edges.push(LifecycleLink::new((
        LifecycleRelation::Supersedes,
        unique_local(64),
        local(250),
    )));
    cap_error(input, CompleteEpisodeError::CollectionLimit);
    let mut input = populated();
    input.provenance.consulted_parents = (0..256)
        .map(|i| ConsultedRef::Local(unique_local(i)))
        .collect();
    super::decode_roundtrip::check(input.clone());
    input
        .provenance
        .consulted_parents
        .push(ConsultedRef::Local(unique_local(256)));
    cap_error(input, CompleteEpisodeError::CollectionLimit);
    for (select, kind) in [
        (
            (|i: &mut CompleteEpisodeInput| &mut i.evidence.source_refs)
                as fn(&mut CompleteEpisodeInput) -> &mut Vec<ArtifactRef>,
            ArtifactKind::Evidence,
        ),
        (
            |i| &mut i.evidence.observation_refs,
            ArtifactKind::Observation,
        ),
        (|i| &mut i.provenance.records, ArtifactKind::Provenance),
    ] {
        let mut input = populated();
        *select(&mut input) = (0..256).map(|i| unique_artifact(kind, i)).collect();
        super::decode_roundtrip::check(input.clone());
        select(&mut input).push(unique_artifact(kind, 256));
        cap_error(input, CompleteEpisodeError::CollectionLimit);
    }
    let mut input = populated();
    input.context.regime_posterior = Estimate::Known(
        RegimeDistribution::new(
            (0..32)
                .map(|i| {
                    (
                        RegimeRef::new(&format!("r{i:02}")).unwrap(),
                        UnitPpm::new(if i == 0 { 1_000_000 } else { 0 }).unwrap(),
                    )
                })
                .collect(),
        )
        .unwrap(),
    );
    super::decode_roundtrip::check(input.clone());
    known_mut(&mut input.context.regime_posterior)
        .0
        .push((RegimeRef::new("r32").unwrap(), UnitPpm::new(0).unwrap()));
    cap_error(input, CompleteEpisodeError::InvalidPosterior);
    for select in [
        (|i: &mut CompleteEpisodeInput| &mut i.context.resources)
            as fn(&mut CompleteEpisodeInput) -> &mut Vec<ResourceReading>,
        |i| &mut i.outcome.costs,
    ] {
        let mut input = populated();
        *select(&mut input) = [
            ResourceKind::Tokens,
            ResourceKind::CpuMicros,
            ResourceKind::MemoryBytes,
            ResourceKind::NetworkBytes,
            ResourceKind::EnergyMicrojoules,
        ]
        .into_iter()
        .map(|kind| ResourceReading::new((kind, 0)))
        .collect();
        super::decode_roundtrip::check(input.clone());
        select(&mut input).push(ResourceReading::new((ResourceKind::Tokens, 1)));
        let wire = input.wire::<super::super::canonical::Tree>().unwrap();
        assert_eq!(
            decode(wire).unwrap_err(),
            E::InvalidValue(CompleteEpisodeError::DuplicateReference)
        );
    }
}
#[test]
fn diagnostics_are_content_free() {
    use std::error::Error;
    let mut input = populated();
    input.origin.source_node = NodeRef::new("sentinel-private-node").unwrap();
    input.time.acquired_at = UtcMillis::new(987_654_321);
    input.identity.contract.digest = Digest32::new([0xab; 32]);
    let encoded = bytes(input);
    let decoded = raw(&encoded, Limits::default()).unwrap();
    assert_eq!(format!("{decoded:?}"), "DecodedCompleteEpisode { .. }");
    let mut errors = vec![
        E::InvalidShape,
        E::UnsupportedDomain,
        E::UnsupportedVersion,
        E::UnknownVariant,
        E::NonCanonicalEncoding,
        E::InvalidValue(CompleteEpisodeError::InvalidIdentifier),
        E::Canonical(CE::InvalidUtf8),
    ];
    errors.extend(
        [
            CompleteEpisodeError::InvalidIdentifier,
            CompleteEpisodeError::InvalidUuid,
            CompleteEpisodeError::InvalidRange,
            CompleteEpisodeError::DuplicateReference,
            CompleteEpisodeError::ScopeMismatch,
            CompleteEpisodeError::CollectionLimit,
            CompleteEpisodeError::EncodedSizeLimit,
            CompleteEpisodeError::InvalidPosterior,
            CompleteEpisodeError::SelfReference,
            CompleteEpisodeError::InvalidArtifactKind,
            CompleteEpisodeError::Encoding,
        ]
        .into_iter()
        .map(E::InvalidValue),
    );
    errors.extend(
        [
            CE::InputLimit,
            CE::ValueLimit,
            CE::CollectionLimit,
            CE::StringLimit,
            CE::ParentLimit,
            CE::UnexpectedEnd,
            CE::TrailingBytes,
            CE::InvalidAdditionalInformation,
            CE::NonMinimalArgument,
            CE::UnsupportedType,
            CE::InvalidUtf8,
            CE::IntegerOutOfRange,
            CE::NestingLimit,
            CE::LengthOutOfRange,
            CE::AllocationFailure,
            CE::DuplicateMapKey,
            CE::MapKeyOrder,
            CE::InvalidEnvelope,
            CE::ProfileMismatch,
            CE::ZeroSchemaVersion,
            CE::HeaderMustBeMap,
            CE::PayloadMustBeMap,
            CE::InvalidParentDigest,
            CE::ParentOrder,
        ]
        .into_iter()
        .map(E::Canonical),
    );
    // Direct enum construction covers diagnostics, not ingress reachability.
    for error in errors {
        let mut current: Option<&dyn Error> = Some(&error);
        while let Some(e) = current {
            let text = format!("{e:?} {e}");
            for sentinel in ["synthetic", "sentinel", "987654321", "07070707"] {
                assert!(!text.contains(sentinel));
            }
            current = e.source();
        }
    }
}
