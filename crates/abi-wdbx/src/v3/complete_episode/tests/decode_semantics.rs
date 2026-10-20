use super::decode_malformed::decode;
use super::decode_roundtrip::check;
use super::*;
use CompleteEpisodeDecodeError as E;
fn malformed(input: CompleteEpisodeInput, expected: CompleteEpisodeError) {
    // Encode a malformed typed tree without invoking final relational validation.
    let wire = input.wire::<super::super::canonical::Tree>().unwrap();
    assert_eq!(decode(wire).unwrap_err(), E::InvalidValue(expected));
}
#[test]
fn every_local_and_artifact_scope_and_kind_reaches_final_validation() {
    for slot in 0..references::artifacts(&mut populated()).len() {
        let mut input = populated();
        references::artifacts(&mut input)[slot].binding = binding("wrong-tenant");
        malformed(input, CompleteEpisodeError::ScopeMismatch);
    }
    for slot in 0..references::locals(&mut populated()).len() {
        let mut input = populated();
        references::locals(&mut input)[slot].binding = binding("wrong-tenant");
        malformed(input, CompleteEpisodeError::ScopeMismatch);
    }
    for consent in [true, false] {
        let mut input = populated();
        if consent {
            input.authority.consent.as_mut().unwrap().binding = binding("wrong-tenant");
        } else {
            input.lifecycle.deletion_key.as_mut().unwrap().binding = binding("wrong-tenant");
        }
        malformed(input, CompleteEpisodeError::ScopeMismatch);
    }
    for retention in [
        RetentionIntent::Durable {
            privacy_basis: LocalEpisodeRef::new((
                binding("wrong-tenant"),
                EpisodeDomain::CompleteV1,
                Digest32::new([0; 32]),
            )),
        },
        RetentionIntent::MandatoryIncident {
            obligation: LocalEpisodeRef::new((
                binding("wrong-tenant"),
                EpisodeDomain::CompleteV1,
                Digest32::new([0; 32]),
            )),
        },
    ] {
        let mut input = populated();
        input.lifecycle.retention = retention;
        malformed(input, CompleteEpisodeError::ScopeMismatch);
    }
    for slot in 0..16 {
        for kind in [
            ArtifactKind::StateSummary,
            ArtifactKind::Prediction,
            ArtifactKind::Outcome,
            ArtifactKind::Observation,
            ArtifactKind::Model,
            ArtifactKind::Calibration,
            ArtifactKind::Provenance,
            ArtifactKind::ReservoirEmbedding,
            ArtifactKind::Evidence,
            ArtifactKind::Authority,
        ] {
            let mut input = populated();
            let expected = references::artifacts(&mut input)[slot].kind;
            references::artifacts(&mut input)[slot].kind = kind;
            if slot == 12 || slot == 13 || kind == expected {
                check(input);
            } else {
                malformed(input, CompleteEpisodeError::InvalidArtifactKind);
            }
        }
    }
}
#[test]
fn scope_identity_domains_uuid_boundaries_and_known_values() {
    for scope in [
        Scope::Guild(GuildRef::new("g").unwrap()),
        Scope::GuildMember {
            guild: GuildRef::new("g").unwrap(),
            member: PrincipalRef::new("m").unwrap(),
        },
        Scope::DirectMessage {
            user: PrincipalRef::new("u").unwrap(),
        },
        Scope::Organization(OrganizationRef::new("o").unwrap()),
    ] {
        let mut input = populated();
        references::rebind(
            &mut input,
            ScopeBinding::new((TenantRef::new("tenant").unwrap(), scope.clone())),
        );
        let decoded = check(input);
        assert_eq!(decoded.identity.binding.scope, scope);
    }
    for uuid_version in [4, 7] {
        let mut id = uuid();
        id[6] = (id[6] & 15) | (uuid_version << 4);
        let mut input = minimal();
        input.identity.episode_id = EpisodeId::new(id).unwrap();
        input.identity.request_id = RequestId::new(id).unwrap();
        input.context.task_id = TaskId::new(id).unwrap();
        let decoded = check(input);
        assert_eq!(decoded.identity.episode_id.0, id);
        assert_eq!(decoded.context.task_id.0, id);
    }
    for value in [i64::MIN, -1, 0, i64::MAX] {
        let mut input = minimal();
        input.outcome.utility = Estimate::Known(
            Measurement::new(MeasurementUnit::Unitless, SignedMicros::new(value)).unwrap(),
        );
        let decoded = check(input);
        assert_eq!(
            known_mut(&mut decoded.clone().outcome.utility).value.0,
            value
        );
    }
    let mut zero = minimal();
    zero.context.habituation = Estimate::Known(UnitPpm::new(0).unwrap());
    zero.decision.prediction = Estimate::Known(artifact(ArtifactKind::Prediction, 0));
    zero.identity.contract.digest = Digest32::new([0; 32]);
    let decoded = check(zero);
    assert_eq!(
        decoded.context.habituation,
        Estimate::Known(UnitPpm::new(0).unwrap())
    );
    assert_eq!(decoded.identity.contract.digest.0, [0; 32]);
    assert_eq!(
        known_mut(&mut decoded.clone().decision.prediction).digest.0,
        [0; 32]
    );
    for domain in [EpisodeDomain::CompleteV1, EpisodeDomain::LegacyEpisodeV1] {
        let mut input = populated();
        for reference in references::locals(&mut input) {
            reference.domain = domain;
        }
        let ConsultedRef::Foreign(foreign) = &mut input.provenance.consulted_parents[1] else {
            unreachable!()
        };
        foreign.domain = domain;
        check(input);
    }
}
#[test]
fn posterior_probability_and_foreign_origin_failures() {
    for entries in [
        vec![],
        vec![(RegimeRef::new("r").unwrap(), UnitPpm::new(1).unwrap())],
        vec![(RegimeRef::new("r").unwrap(), UnitPpm::new(500_000).unwrap()); 2],
    ] {
        let mut input = minimal();
        input.context.regime_posterior = Estimate::Known(RegimeDistribution(entries));
        malformed(input, CompleteEpisodeError::InvalidPosterior);
    }
    let mut input = populated();
    let ConsultedRef::Foreign(foreign) = &mut input.provenance.consulted_parents[1] else {
        unreachable!()
    };
    foreign.origin_organization = OrganizationRef::new("wrong").unwrap();
    malformed(input.clone(), CompleteEpisodeError::ScopeMismatch);
    let ConsultedRef::Foreign(foreign) = &mut input.provenance.consulted_parents[1] else {
        unreachable!()
    };
    foreign.origin_binding.scope = Scope::Guild(GuildRef::new("unverified").unwrap());
    check(input);
    for slot in 0..references::artifacts(&mut populated()).len() {
        let mut input = populated();
        references::artifacts(&mut input)[slot].byte_length = (1u64 << 40) + 1;
        malformed(input, CompleteEpisodeError::InvalidRange);
    }
}
#[test]
fn all_labels_and_unit_variants_retain_exact_typed_identity() {
    use crate::v3::episode::{ActorKind, EvidenceLevel};
    macro_rules! each { ($path:ident.$leaf:ident, [$($value:expr),+ $(,)?]) => {$(
        let value=$value;let mut input=populated();input.$path.$leaf=value;
        assert_eq!(check(input).$path.$leaf,value);
    )+}; }
    each!(
        decision.route,
        [
            Route::NoModel,
            Route::LocalModel,
            Route::CloudModel,
            Route::Specialist,
            Route::Deterministic
        ]
    );
    each!(
        outcome.safety_effect,
        [
            SafetyEffect::Unknown,
            SafetyEffect::NoViolationObserved,
            SafetyEffect::InterlockTriggered,
            SafetyEffect::ViolationObserved
        ]
    );
    each!(
        outcome.rollback,
        [
            RollbackState::NotApplicable,
            RollbackState::NotAttempted,
            RollbackState::Attempted,
            RollbackState::ExactRestoreObserved,
            RollbackState::Failed,
            RollbackState::Unknown
        ]
    );
    each!(
        evidence.maturity,
        [
            EvidenceLevel::C0,
            EvidenceLevel::C1,
            EvidenceLevel::C2,
            EvidenceLevel::C3,
            EvidenceLevel::C4,
            EvidenceLevel::C5,
            EvidenceLevel::C6,
            EvidenceLevel::C7
        ]
    );
    for value in [
        ActorKind::HumanSubject,
        ActorKind::OrganizationOwner,
        ActorKind::GuildOwner,
        ActorKind::GuildAdministrator,
        ActorKind::GuildManager,
        ActorKind::Service,
    ] {
        let mut input = populated();
        input.origin.actor.kind = value;
        assert_eq!(check(input).origin.actor.kind, value);
    }
    for value in [ActionPhase::Proposed, ActionPhase::Performed] {
        let mut input = populated();
        input.decision.actions[0].phase = value;
        assert_eq!(check(input).decision.actions[0].phase, value);
    }
    for value in [
        MeasurementUnit::Unitless,
        MeasurementUnit::Count,
        MeasurementUnit::Milliseconds,
        MeasurementUnit::Bytes,
        MeasurementUnit::Microjoules,
    ] {
        let mut input = populated();
        input.outcome.utility =
            Estimate::Known(Measurement::new(value, SignedMicros::new(0)).unwrap());
        assert_eq!(known_mut(&mut check(input).outcome.utility).unit, value);
    }
    for value in [
        LifecycleRelation::Supersedes,
        LifecycleRelation::Contradicts,
        LifecycleRelation::Revokes,
        LifecycleRelation::Quarantines,
        LifecycleRelation::Resolves,
        LifecycleRelation::Redacts,
        LifecycleRelation::Forgets,
    ] {
        let mut input = minimal();
        input.lifecycle.edges = vec![LifecycleLink::new((value, local(1), local(2)))];
        assert_eq!(check(input).lifecycle.edges[0].relation, value);
    }
    for value in [
        RetentionIntent::Ephemeral,
        RetentionIntent::Session {
            session: OpaqueRef::new("session").unwrap(),
            expires_at: UtcMillis::new(100),
        },
        RetentionIntent::Operational {
            expires_at: UtcMillis::new(100),
        },
        RetentionIntent::Durable {
            privacy_basis: local(1),
        },
        RetentionIntent::MandatoryIncident {
            obligation: local(2),
        },
    ] {
        let mut input = populated();
        input.lifecycle.retention = value.clone();
        assert_eq!(check(input).lifecycle.retention, value);
    }
}

#[test]
fn absence_unknown_and_known_empty_artifact_remain_distinct() {
    let absent = check(minimal());
    assert!(absent.context.summary.is_none());
    assert_eq!(absent.decision.prediction, Estimate::Unknown);
    let mut input = minimal();
    let mut empty = artifact(ArtifactKind::Prediction, 0);
    empty.byte_length = 0;
    input.decision.prediction = Estimate::Known(empty.clone());
    input.context.summary = Some(artifact(ArtifactKind::StateSummary, 0));
    let present = check(input);
    assert_eq!(present.decision.prediction, Estimate::Known(empty));
    assert!(present.context.summary.is_some());
    assert_ne!(absent, present);
}
