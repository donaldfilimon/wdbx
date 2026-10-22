use super::*;
type VersionList = fn(&mut CompleteEpisodeInput) -> &mut Vec<VersionRef>;
type LocalList = fn(&mut CompleteEpisodeInput) -> &mut Vec<LocalEpisodeRef>;
type ArtifactList = fn(&mut CompleteEpisodeInput) -> &mut Vec<ArtifactRef>;

fn permutable() -> CompleteEpisodeInput {
    let mut input = populated();
    input.authority.models.push(version("model-two"));
    input
        .authority
        .calibrations
        .push(version("calibration-two"));
    input.lifecycle.holds.push(local(88));
    input
        .evidence
        .observation_refs
        .push(artifact(ArtifactKind::Observation, 88));
    input
        .provenance
        .records
        .push(artifact(ArtifactKind::Provenance, 88));
    input
}

#[test]
fn all_set_reversals_and_sequence_order() {
    let original = bytes(permutable());
    macro_rules! reverse {($($path:ident).+) => {{let mut input=permutable();input.$($path).+.reverse();assert_eq!(bytes(input),original,stringify!($($path).+));}};}
    reverse!(context.resources);
    reverse!(outcome.costs);
    reverse!(authority.capabilities);
    reverse!(authority.models);
    reverse!(authority.calibrations);
    reverse!(authority.approvals);
    reverse!(evidence.source_refs);
    reverse!(evidence.observation_refs);
    reverse!(lifecycle.holds);
    reverse!(lifecycle.edges);
    reverse!(provenance.causal_parents);
    reverse!(provenance.consulted_parents);
    reverse!(provenance.records);
    let mut input = permutable();
    input.decision.actions[0].parameter_classes.reverse();
    assert_eq!(bytes(input), original);
    let mut input = permutable();
    known_mut(&mut input.context.regime_posterior).0.reverse();
    assert_eq!(bytes(input), original);
    let mut input = permutable();
    let mut action = input.decision.actions[0].clone();
    action.phase = ActionPhase::Performed;
    input.decision.actions.push(action);
    let ordered = bytes(input.clone());
    input.decision.actions.reverse();
    assert_ne!(bytes(input), ordered);
    let mut input = permutable();
    input
        .decision
        .actions
        .push(input.decision.actions[0].clone());
    assert!(CompleteEpisode::try_new(input).is_ok());
    let mut input = permutable();
    input.provenance.consulted_parents.push(ConsultedRef::Local(
        input.provenance.causal_parents.pop().unwrap(),
    ));
    assert_ne!(bytes(input), original);
}
#[test]
fn every_set_rejects_duplicates() {
    macro_rules! duplicate {($($path:ident).+) => {{let mut input=populated();let value=input.$($path).+[0].clone();input.$($path).+.push(value);assert_eq!(CompleteEpisode::try_new(input).unwrap_err(),CompleteEpisodeError::DuplicateReference,stringify!($($path).+));}};}
    duplicate!(context.resources);
    duplicate!(outcome.costs);
    duplicate!(authority.capabilities);
    duplicate!(authority.models);
    duplicate!(authority.calibrations);
    duplicate!(authority.approvals);
    duplicate!(evidence.source_refs);
    duplicate!(evidence.observation_refs);
    duplicate!(lifecycle.holds);
    duplicate!(lifecycle.edges);
    duplicate!(provenance.causal_parents);
    duplicate!(provenance.consulted_parents);
    duplicate!(provenance.records);
    let mut input = populated();
    let parameter = input.decision.actions[0].parameter_classes[0].clone();
    input.decision.actions[0].parameter_classes.push(parameter);
    assert_eq!(
        CompleteEpisode::try_new(input).unwrap_err(),
        CompleteEpisodeError::DuplicateReference
    );
    let mut input = populated();
    let mut v = input.authority.capabilities[0].clone();
    v.digest = Digest32::new([99; 32]);
    input.authority.capabilities.push(v);
    assert_eq!(
        CompleteEpisode::try_new(input).unwrap_err(),
        CompleteEpisodeError::DuplicateReference
    );
    let mut input = populated();
    let mut value = input.context.resources[0].clone();
    value.amount += 1;
    input.context.resources.push(value);
    assert_eq!(
        CompleteEpisode::try_new(input).unwrap_err(),
        CompleteEpisodeError::DuplicateReference
    );
}
fn reference(index: usize) -> LocalEpisodeRef {
    let mut result = local(0);
    result.digest = Digest32::new(index_digest(index));
    result
}
fn index_digest(index: usize) -> [u8; 32] {
    let mut value = [0; 32];
    value[..8].copy_from_slice(&(index as u64).to_be_bytes());
    value
}
#[test]
fn exact_cardinality_caps_and_cap_plus_one() {
    macro_rules! bounded {
        ($cap:expr,$field:expr,$value:expr) => {{
            let mut input = populated();
            let field: fn(&mut CompleteEpisodeInput) -> &mut _ = $field;
            *field(&mut input) = (0..$cap).map($value).collect::<Vec<_>>();
            assert!(CompleteEpisode::try_new(input.clone()).is_ok());
            field(&mut input).push(($value)($cap));
            assert_eq!(
                CompleteEpisode::try_new(input).unwrap_err(),
                CompleteEpisodeError::CollectionLimit
            );
        }};
    }
    bounded!(128, |i| &mut i.decision.actions, |_| populated()
        .decision
        .actions
        .remove(0));
    bounded!(32, |i| &mut i.decision.actions[0].parameter_classes, |n| {
        ParameterClassRef::new(&format!("p{n}")).unwrap()
    });
    for field in [
        (|i: &mut CompleteEpisodeInput| &mut i.authority.capabilities) as VersionList,
        |i: &mut CompleteEpisodeInput| &mut i.authority.models,
        |i: &mut CompleteEpisodeInput| &mut i.authority.calibrations,
    ] {
        bounded!(32, field, |n| version(&format!("v{n}")));
    }
    for field in [
        (|i: &mut CompleteEpisodeInput| &mut i.authority.approvals) as LocalList,
        |i: &mut CompleteEpisodeInput| &mut i.lifecycle.holds,
        |i: &mut CompleteEpisodeInput| &mut i.provenance.causal_parents,
    ] {
        bounded!(64, field, reference);
    }
    bounded!(64, |i| &mut i.lifecycle.edges, |n| LifecycleLink {
        relation: LifecycleRelation::Supersedes,
        target: reference(n),
        basis: local(0)
    });
    bounded!(256, |i| &mut i.provenance.consulted_parents, |n| {
        ConsultedRef::Local(reference(n))
    });
    for (field, kind) in [
        (
            (|i: &mut CompleteEpisodeInput| &mut i.evidence.source_refs) as ArtifactList,
            ArtifactKind::Evidence,
        ),
        (
            |i: &mut CompleteEpisodeInput| &mut i.evidence.observation_refs,
            ArtifactKind::Observation,
        ),
        (
            |i: &mut CompleteEpisodeInput| &mut i.provenance.records,
            ArtifactKind::Provenance,
        ),
    ] {
        bounded!(256, field, |n| ArtifactRef {
            digest: Digest32::new(index_digest(n)),
            ..artifact(kind, 0)
        });
    }
    let readings = [
        ResourceKind::Tokens,
        ResourceKind::CpuMicros,
        ResourceKind::MemoryBytes,
        ResourceKind::NetworkBytes,
        ResourceKind::EnergyMicrojoules,
    ]
    .into_iter()
    .map(|resource| ResourceReading {
        resource,
        amount: u64::MAX,
    })
    .collect::<Vec<_>>();
    let mut input = populated();
    input.context.resources = readings.clone();
    input.outcome.costs = readings;
    assert!(CompleteEpisode::try_new(input.clone()).is_ok());
    input.context.resources = vec![
        ResourceReading {
            resource: ResourceKind::Tokens,
            amount: 0
        };
        17
    ];
    assert_eq!(
        CompleteEpisode::try_new(input).unwrap_err(),
        CompleteEpisodeError::CollectionLimit
    );
}

#[test]
fn valid_bounded_shape_exercises_production_output_cap() {
    let long = "x".repeat(128);
    let mut input = populated();
    let binding = ScopeBinding {
        tenant: TenantRef::new(&long).unwrap(),
        scope: Scope::GuildMember {
            guild: GuildRef::new(&long).unwrap(),
            member: PrincipalRef::new(&long).unwrap(),
        },
    };
    references::rebind(&mut input, binding.clone());
    let format = VersionRef {
        component: ComponentRef::new(&long).unwrap(),
        revision: OpaqueRef::new(&long).unwrap(),
        digest: Digest32::new([1; 32]),
    };
    for (field, kind) in [
        (
            (|i: &mut CompleteEpisodeInput| &mut i.evidence.source_refs) as ArtifactList,
            ArtifactKind::Evidence,
        ),
        (
            |i: &mut CompleteEpisodeInput| &mut i.evidence.observation_refs,
            ArtifactKind::Observation,
        ),
        (
            |i: &mut CompleteEpisodeInput| &mut i.provenance.records,
            ArtifactKind::Provenance,
        ),
    ] {
        *field(&mut input) = (0..256)
            .map(|n| ArtifactRef {
                binding: binding.clone(),
                digest: Digest32::new(index_digest(n)),
                byte_length: 1 << 40,
                kind,
                format: format.clone(),
            })
            .collect();
    }
    let action = ActionDescriptor {
        capability: format,
        action: ActionRef::new(&long).unwrap(),
        parameter_classes: (0..32)
            .map(|n| ParameterClassRef::new(&format!("{n:03}{}", "x".repeat(125))).unwrap())
            .collect(),
        phase: ActionPhase::Performed,
    };
    input.decision.actions = vec![action; 128];
    validate::validate(&input).unwrap();
    let size = counted_size(&input, usize::MAX).unwrap();
    assert!(size > MAX_ENCODED_SIZE, "actual checked size {size}");
    assert_eq!(
        CompleteEpisode::try_new(input).unwrap_err(),
        CompleteEpisodeError::EncodedSizeLimit
    );
    println!("valid bounded over-limit fixture counted bytes: {size}");
}
