use super::*;
use crate::v3::episode::{ActorKind, EvidenceLevel, SignerKeyId};

#[test]
fn every_group_leaf_changes_the_commitment() {
    let original = bytes(populated());
    macro_rules! change {($($path:ident).+ = $value:expr) => {{let mut input=populated();input.$($path).+ = $value;assert_ne!(bytes(input),original,stringify!($($path).+));}};}
    let mut id = uuid();
    id[15] += 1;
    change!(identity.episode_id = EpisodeId::new(id).unwrap());
    change!(identity.request_id = RequestId::new(id).unwrap());
    change!(identity.contract.component = ComponentRef::new("other").unwrap());
    change!(identity.contract.revision = OpaqueRef::new("other").unwrap());
    change!(identity.contract.digest = Digest32::new([55; 32]));
    change!(time.acquired_at = UtcMillis::new(101));
    change!(time.received_at = UtcMillis::new(98));
    change!(time.monotonic = None);
    change!(origin.source_node = NodeRef::new("other").unwrap());
    change!(origin.actor.principal = PrincipalRef::new("other").unwrap());
    change!(origin.actor.kind = ActorKind::HumanSubject);
    change!(origin.signer = SignerKeyId::parse(&format!("ed25519:{}", "cd".repeat(32))).unwrap());
    change!(context.task_id = TaskId::new(id).unwrap());
    change!(context.regime = RegimeRef::new("other").unwrap());
    change!(context.regime_posterior = Estimate::Unknown);
    change!(context.habituation = Estimate::Unknown);
    change!(context.summary = None);
    change!(context.reservoir = None);
    change!(context.resources = vec![]);
    change!(decision.route = Route::CloudModel);
    change!(decision.prediction = Estimate::Unknown);
    change!(decision.actions = vec![]);
    change!(decision.uncertainty = Estimate::Known(UnitPpm::new(101).unwrap()));
    change!(decision.risk = Estimate::Known(UnitPpm::new(201).unwrap()));
    change!(decision.novelty = Estimate::Known(UnitPpm::new(301).unwrap()));
    change!(outcome.observed = Estimate::Unknown);
    change!(
        outcome.utility = Estimate::Known(
            Measurement::new(MeasurementUnit::Milliseconds, SignedMicros::new(-1)).unwrap()
        )
    );
    change!(outcome.safety_effect = SafetyEffect::ViolationObserved);
    change!(outcome.costs = vec![]);
    change!(outcome.rollback = RollbackState::Failed);
    change!(authority.capabilities = vec![]);
    change!(authority.models = vec![]);
    change!(authority.calibrations = vec![]);
    change!(authority.approvals = vec![]);
    change!(authority.consent = None);
    change!(evidence.maturity = EvidenceLevel::C7);
    change!(evidence.source_refs = vec![]);
    change!(evidence.observation_refs = vec![]);
    change!(lifecycle.retention = RetentionIntent::Ephemeral);
    change!(lifecycle.holds = vec![]);
    change!(lifecycle.deletion_key = None);
    change!(lifecycle.edges = vec![]);
    change!(provenance.causal_parents = vec![]);
    change!(provenance.consulted_parents = vec![]);
    change!(provenance.records = vec![]);
    let mutations: Vec<fn(&mut CompleteEpisodeInput)> = vec![
        |i| i.time.monotonic.as_mut().unwrap().clock = ClockRef::new("other").unwrap(),
        |i| i.time.monotonic.as_mut().unwrap().start = MonoNanos::new(9),
        |i| i.time.monotonic.as_mut().unwrap().end = MonoNanos::new(11),
        |i| known_mut(&mut i.context.regime_posterior).0[0].0 = RegimeRef::new("other").unwrap(),
        |i| {
            let p = known_mut(&mut i.context.regime_posterior);
            p.0[0].1 = UnitPpm::new(1).unwrap();
            p.0[1].1 = UnitPpm::new(999_999).unwrap();
        },
        |i| i.context.habituation = Estimate::Known(UnitPpm::new(1).unwrap()),
        |i| i.context.resources[0].resource = ResourceKind::MemoryBytes,
        |i| i.context.resources[0].amount += 1,
        |i| i.outcome.costs[0].resource = ResourceKind::MemoryBytes,
        |i| i.outcome.costs[0].amount += 1,
        |i| i.decision.actions[0].action = ActionRef::new("other").unwrap(),
        |i| i.decision.actions[0].parameter_classes[0] = ParameterClassRef::new("other").unwrap(),
        |i| i.decision.actions[0].phase = ActionPhase::Performed,
        |i| known_mut(&mut i.outcome.utility).value = SignedMicros::new(i64::MIN),
        |i| i.authority.consent.as_mut().unwrap().epoch += 1,
        |i| i.lifecycle.deletion_key.as_mut().unwrap().key_id = OpaqueRef::new("other").unwrap(),
        |i| i.lifecycle.edges[0].relation = LifecycleRelation::Forgets,
        |i| {
            i.lifecycle.retention = RetentionIntent::Operational {
                expires_at: UtcMillis::new(101),
            }
        },
    ];
    for (n, mutate) in mutations.into_iter().enumerate() {
        let mut input = populated();
        mutate(&mut input);
        assert_ne!(bytes(input), original, "mutation {n}");
    }
    for slot in 0..references::artifacts(&mut populated()).len() {
        for leaf in 0..5 {
            let mut input = populated();
            let mut slots = references::artifacts(&mut input);
            let a = &mut *slots[slot];
            match leaf {
                0 => a.digest = Digest32::new([99; 32]),
                1 => a.byte_length += 1,
                2 => a.format.component = ComponentRef::new("other").unwrap(),
                3 => a.format.revision = OpaqueRef::new("other").unwrap(),
                _ => a.format.digest = Digest32::new([99; 32]),
            }
            assert_ne!(bytes(input), original, "artifact {slot} leaf {leaf}");
        }
    }
    for slot in 0..references::locals(&mut populated()).len() {
        for domain in [true, false] {
            let mut input = populated();
            let mut slots = references::locals(&mut input);
            let a = &mut *slots[slot];
            if domain {
                a.domain = EpisodeDomain::LegacyEpisodeV1;
            } else {
                a.digest = Digest32::new([99; 32]);
            }
            assert_ne!(bytes(input), original, "local {slot} domain={domain}");
        }
    }
    for slot in 0..8 {
        for leaf in 0..3 {
            let mut input = populated();
            let v = match slot {
                0 => &mut input.authority.policy,
                1 => &mut input.authority.safety_policy,
                2 => &mut input.authority.capabilities[0],
                3 => &mut input.authority.models[0],
                4 => &mut input.authority.calibrations[0],
                5 => &mut input.authority.consent.as_mut().unwrap().policy,
                6 => &mut input.decision.actions[0].capability,
                _ => &mut input.identity.contract,
            };
            match leaf {
                0 => v.component = ComponentRef::new("changed").unwrap(),
                1 => v.revision = OpaqueRef::new("changed").unwrap(),
                _ => v.digest = Digest32::new([99; 32]),
            }
            assert_ne!(bytes(input), original, "version {slot} leaf {leaf}");
        }
    }
}

#[test]
fn every_evidence_dimension_and_absence_are_independent() {
    for slot in 0..8 {
        let mut input = populated();
        let dims = &mut input.evidence.dimensions;
        let fields = [
            &mut dims.cryptographic,
            &mut dims.source,
            &mut dims.calibration,
            &mut dims.outcome,
            &mut dims.compatibility,
            &mut dims.regime,
            &mut dims.lifecycle,
            &mut dims.confidence,
        ];
        *fields.into_iter().nth(slot).unwrap() = Estimate::Unknown;
        assert_ne!(bytes(input), bytes(populated()));
    }
    let mut zero = minimal();
    zero.context.habituation = Estimate::Known(UnitPpm::new(0).unwrap());
    assert_ne!(bytes(zero), bytes(minimal()));
    let mut known = minimal();
    known.decision.prediction = Estimate::Known(artifact(ArtifactKind::Prediction, 0));
    assert_ne!(bytes(known), bytes(minimal()));
    let mut known = minimal();
    known.outcome.observed = Estimate::Known(artifact(ArtifactKind::Outcome, 0));
    assert_ne!(bytes(known), bytes(minimal()));
}

#[test]
fn retention_data_and_foreign_descriptor_leaves() {
    let mut previous = vec![];
    for retention in [
        RetentionIntent::Ephemeral,
        RetentionIntent::Session {
            session: OpaqueRef::new("s1").unwrap(),
            expires_at: UtcMillis::new(100),
        },
        RetentionIntent::Session {
            session: OpaqueRef::new("s2").unwrap(),
            expires_at: UtcMillis::new(100),
        },
        RetentionIntent::Session {
            session: OpaqueRef::new("s2").unwrap(),
            expires_at: UtcMillis::new(101),
        },
        RetentionIntent::Operational {
            expires_at: UtcMillis::new(100),
        },
        RetentionIntent::Durable {
            privacy_basis: local(1),
        },
        RetentionIntent::Durable {
            privacy_basis: local(2),
        },
        RetentionIntent::MandatoryIncident {
            obligation: local(1),
        },
        RetentionIntent::MandatoryIncident {
            obligation: local(2),
        },
    ] {
        let mut input = populated();
        input.lifecycle.retention = retention;
        let bytes = bytes(input);
        assert!(!previous.contains(&bytes));
        previous.push(bytes);
    }
    for leaf in 0..5 {
        let mut input = populated();
        let ConsultedRef::Foreign(f) = &mut input.provenance.consulted_parents[1] else {
            unreachable!()
        };
        match leaf {
            0 => {
                f.origin_organization = OrganizationRef::new("changed").unwrap();
                f.origin_binding.scope = Scope::Organization(f.origin_organization.clone());
            }
            1 => f.origin_binding.tenant = TenantRef::new("changed").unwrap(),
            2 => f.domain = EpisodeDomain::CompleteV1,
            3 => f.digest = Digest32::new([99; 32]),
            _ => f.origin_binding.scope = Scope::Guild(GuildRef::new("foreign-guild").unwrap()),
        }
        assert_ne!(bytes(input), bytes(populated()));
    }
}

#[test]
fn every_numeric_enum_has_explicit_singleton_tag() {
    assert_eq!(encode_test(&EpisodeDomain::CompleteV1), vec![0x81, 0]);
    assert_eq!(encode_test(&EpisodeDomain::LegacyEpisodeV1), vec![0x81, 1]);
    assert_eq!(encode_test(&Route::NoModel), vec![0x81, 0]);
    assert_eq!(encode_test(&Route::LocalModel), vec![0x81, 1]);
    assert_eq!(encode_test(&Route::CloudModel), vec![0x81, 2]);
    assert_eq!(encode_test(&Route::Specialist), vec![0x81, 3]);
    assert_eq!(encode_test(&Route::Deterministic), vec![0x81, 4]);
    assert_eq!(encode_test(&ActionPhase::Proposed), vec![0x81, 0]);
    assert_eq!(encode_test(&ActionPhase::Performed), vec![0x81, 1]);
    assert_eq!(encode_test(&MeasurementUnit::Unitless), vec![0x81, 0]);
    assert_eq!(encode_test(&MeasurementUnit::Count), vec![0x81, 1]);
    assert_eq!(encode_test(&MeasurementUnit::Milliseconds), vec![0x81, 2]);
    assert_eq!(encode_test(&MeasurementUnit::Bytes), vec![0x81, 3]);
    assert_eq!(encode_test(&MeasurementUnit::Microjoules), vec![0x81, 4]);
    assert_eq!(encode_test(&ResourceKind::Tokens), vec![0x81, 0]);
    assert_eq!(encode_test(&ResourceKind::CpuMicros), vec![0x81, 1]);
    assert_eq!(encode_test(&ResourceKind::MemoryBytes), vec![0x81, 2]);
    assert_eq!(encode_test(&ResourceKind::NetworkBytes), vec![0x81, 3]);
    assert_eq!(encode_test(&ResourceKind::EnergyMicrojoules), vec![0x81, 4]);
    assert_eq!(encode_test(&ArtifactKind::StateSummary), vec![0x81, 0]);
    assert_eq!(encode_test(&ArtifactKind::Prediction), vec![0x81, 1]);
    assert_eq!(encode_test(&ArtifactKind::Outcome), vec![0x81, 2]);
    assert_eq!(encode_test(&ArtifactKind::Observation), vec![0x81, 3]);
    assert_eq!(encode_test(&ArtifactKind::Model), vec![0x81, 4]);
    assert_eq!(encode_test(&ArtifactKind::Calibration), vec![0x81, 5]);
    assert_eq!(encode_test(&ArtifactKind::Provenance), vec![0x81, 6]);
    assert_eq!(
        encode_test(&ArtifactKind::ReservoirEmbedding),
        vec![0x81, 7]
    );
    assert_eq!(encode_test(&ArtifactKind::Evidence), vec![0x81, 8]);
    assert_eq!(encode_test(&ArtifactKind::Authority), vec![0x81, 9]);
    assert_eq!(encode_test(&SafetyEffect::Unknown), vec![0x81, 0]);
    assert_eq!(
        encode_test(&SafetyEffect::NoViolationObserved),
        vec![0x81, 1]
    );
    assert_eq!(
        encode_test(&SafetyEffect::InterlockTriggered),
        vec![0x81, 2]
    );
    assert_eq!(encode_test(&SafetyEffect::ViolationObserved), vec![0x81, 3]);
    assert_eq!(encode_test(&RollbackState::NotApplicable), vec![0x81, 0]);
    assert_eq!(encode_test(&RollbackState::NotAttempted), vec![0x81, 1]);
    assert_eq!(encode_test(&RollbackState::Attempted), vec![0x81, 2]);
    assert_eq!(
        encode_test(&RollbackState::ExactRestoreObserved),
        vec![0x81, 3]
    );
    assert_eq!(encode_test(&RollbackState::Failed), vec![0x81, 4]);
    assert_eq!(encode_test(&RollbackState::Unknown), vec![0x81, 5]);
    assert_eq!(encode_test(&LifecycleRelation::Supersedes), vec![0x81, 0]);
    assert_eq!(encode_test(&LifecycleRelation::Contradicts), vec![0x81, 1]);
    assert_eq!(encode_test(&LifecycleRelation::Revokes), vec![0x81, 2]);
    assert_eq!(encode_test(&LifecycleRelation::Quarantines), vec![0x81, 3]);
    assert_eq!(encode_test(&LifecycleRelation::Resolves), vec![0x81, 4]);
    assert_eq!(encode_test(&LifecycleRelation::Redacts), vec![0x81, 5]);
    assert_eq!(encode_test(&LifecycleRelation::Forgets), vec![0x81, 6]);
}

#[test]
fn reused_enum_labels_are_exact() {
    assert_eq!(
        encode_test(&ActorKind::HumanSubject),
        [vec![0x60 + 13], b"human_subject".to_vec()].concat()
    );
    assert_eq!(
        encode_test(&ActorKind::OrganizationOwner),
        [vec![0x60 + 18], b"organization_owner".to_vec()].concat()
    );
    assert_eq!(
        encode_test(&ActorKind::GuildOwner),
        [vec![0x60 + 11], b"guild_owner".to_vec()].concat()
    );
    assert_eq!(
        encode_test(&ActorKind::GuildAdministrator),
        [vec![0x60 + 19], b"guild_administrator".to_vec()].concat()
    );
    assert_eq!(
        encode_test(&ActorKind::GuildManager),
        [vec![0x60 + 13], b"guild_manager".to_vec()].concat()
    );
    assert_eq!(
        encode_test(&ActorKind::Service),
        [vec![0x60 + 7], b"service".to_vec()].concat()
    );
    assert_eq!(encode_test(&EvidenceLevel::C0), vec![0x62, b'C', b'0']);
    assert_eq!(encode_test(&EvidenceLevel::C1), vec![0x62, b'C', b'1']);
    assert_eq!(encode_test(&EvidenceLevel::C2), vec![0x62, b'C', b'2']);
    assert_eq!(encode_test(&EvidenceLevel::C3), vec![0x62, b'C', b'3']);
    assert_eq!(encode_test(&EvidenceLevel::C4), vec![0x62, b'C', b'4']);
    assert_eq!(encode_test(&EvidenceLevel::C5), vec![0x62, b'C', b'5']);
    assert_eq!(encode_test(&EvidenceLevel::C6), vec![0x62, b'C', b'6']);
    assert_eq!(encode_test(&EvidenceLevel::C7), vec![0x62, b'C', b'7']);
}
