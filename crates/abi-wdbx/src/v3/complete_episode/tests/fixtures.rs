use super::*;
use crate::v3::episode::SignerKeyId;
use crate::v3::episode::{ActorKind, EvidenceLevel};

pub(super) fn binding(tenant: &str) -> ScopeBinding {
    ScopeBinding {
        tenant: TenantRef::new(tenant).unwrap(),
        scope: Scope::Guild(GuildRef::new("synthetic-guild").unwrap()),
    }
}
pub(super) fn version(name: &str) -> VersionRef {
    VersionRef {
        component: ComponentRef::new(name).unwrap(),
        revision: OpaqueRef::new("v1").unwrap(),
        digest: Digest32::new([7; 32]),
    }
}
pub(super) fn local(id: u8) -> LocalEpisodeRef {
    LocalEpisodeRef {
        binding: binding("synthetic-tenant"),
        domain: EpisodeDomain::CompleteV1,
        digest: Digest32::new([id; 32]),
    }
}
pub(super) fn artifact(kind: ArtifactKind, id: u8) -> ArtifactRef {
    ArtifactRef {
        binding: binding("synthetic-tenant"),
        digest: Digest32::new([id; 32]),
        byte_length: 24,
        kind,
        format: version("format"),
    }
}
pub(super) fn uuid() -> [u8; 16] {
    [1, 2, 3, 4, 5, 6, 0x40, 8, 0x80, 10, 11, 12, 13, 14, 15, 16]
}
pub(super) fn minimal() -> CompleteEpisodeInput {
    CompleteEpisodeInput {
        identity: Identity {
            episode_id: EpisodeId::new(uuid()).unwrap(),
            request_id: RequestId::new(uuid()).unwrap(),
            binding: binding("synthetic-tenant"),
            contract: version("contract"),
        },
        time: Time {
            acquired_at: UtcMillis::new(100),
            received_at: UtcMillis::new(99),
            monotonic: None,
        },
        origin: Origin {
            source_node: NodeRef::new("synthetic-node").unwrap(),
            actor: Actor {
                principal: PrincipalRef::new("synthetic-principal").unwrap(),
                kind: ActorKind::Service,
            },
            signer: SignerKeyId::parse(&format!("ed25519:{}", "ab".repeat(32))).unwrap(),
        },
        context: Context {
            task_id: TaskId::new(uuid()).unwrap(),
            regime: RegimeRef::new("synthetic-regime").unwrap(),
            regime_posterior: Estimate::Unknown,
            habituation: Estimate::Unknown,
            summary: None,
            resources: vec![],
            reservoir: None,
        },
        decision: Decision {
            route: Route::NoModel,
            prediction: Estimate::Unknown,
            actions: vec![],
            uncertainty: Estimate::Unknown,
            risk: Estimate::Unknown,
            novelty: Estimate::Unknown,
        },
        outcome: Outcome {
            observed: Estimate::Unknown,
            utility: Estimate::Unknown,
            safety_effect: SafetyEffect::Unknown,
            costs: vec![],
            rollback: RollbackState::Unknown,
        },
        authority: Authority {
            policy: version("policy"),
            safety_policy: version("safety"),
            capabilities: vec![],
            models: vec![],
            calibrations: vec![],
            approvals: vec![],
            consent: None,
        },
        evidence: Evidence {
            maturity: EvidenceLevel::C0,
            source_refs: vec![],
            observation_refs: vec![],
            dimensions: RecordedEvidence {
                cryptographic: Estimate::Unknown,
                source: Estimate::Unknown,
                calibration: Estimate::Unknown,
                outcome: Estimate::Unknown,
                compatibility: Estimate::Unknown,
                regime: Estimate::Unknown,
                lifecycle: Estimate::Unknown,
                confidence: Estimate::Unknown,
            },
        },
        lifecycle: Lifecycle {
            retention: RetentionIntent::Ephemeral,
            holds: vec![],
            deletion_key: None,
            edges: vec![],
        },
        provenance: Provenance {
            causal_parents: vec![],
            consulted_parents: vec![],
            records: vec![],
        },
    }
}
pub(super) fn populated() -> CompleteEpisodeInput {
    let mut input = minimal();
    input.time.monotonic = Some(
        MonotonicRange::new((
            ClockRef::new("synthetic-clock").unwrap(),
            MonoNanos::new(10),
            MonoNanos::new(10),
        ))
        .unwrap(),
    );
    input.context.regime_posterior = Estimate::Known(
        RegimeDistribution::new(vec![
            (RegimeRef::new("z").unwrap(), UnitPpm::new(0).unwrap()),
            (
                RegimeRef::new("a").unwrap(),
                UnitPpm::new(1_000_000).unwrap(),
            ),
        ])
        .unwrap(),
    );
    input.context.habituation = Estimate::Known(UnitPpm::new(0).unwrap());
    input.context.summary = Some(artifact(ArtifactKind::StateSummary, 1));
    input.context.reservoir = Some(artifact(ArtifactKind::ReservoirEmbedding, 2));
    input.context.resources = vec![
        ResourceReading {
            resource: ResourceKind::Tokens,
            amount: 24,
        },
        ResourceReading {
            resource: ResourceKind::CpuMicros,
            amount: 256,
        },
    ];
    input.decision.prediction = Estimate::Known(artifact(ArtifactKind::Prediction, 3));
    input.decision.route = Route::LocalModel;
    input.decision.actions = vec![ActionDescriptor {
        capability: version("capability"),
        action: ActionRef::new("run").unwrap(),
        parameter_classes: vec![
            ParameterClassRef::new("a").unwrap(),
            ParameterClassRef::new("z").unwrap(),
        ],
        phase: ActionPhase::Proposed,
    }];
    input.decision.uncertainty = Estimate::Known(UnitPpm::new(100).unwrap());
    input.decision.risk = Estimate::Known(UnitPpm::new(200).unwrap());
    input.decision.novelty = Estimate::Known(UnitPpm::new(300).unwrap());
    input.outcome.observed = Estimate::Known(artifact(ArtifactKind::Outcome, 4));
    input.outcome.utility = Estimate::Known(
        Measurement::new(MeasurementUnit::Unitless, SignedMicros::new(-1)).unwrap(),
    );
    input.outcome.safety_effect = SafetyEffect::NoViolationObserved;
    input.outcome.costs = input.context.resources.clone();
    input.outcome.rollback = RollbackState::NotAttempted;
    input.authority.capabilities = vec![version("capability"), version("other")];
    input.authority.models = vec![version("model")];
    input.authority.calibrations = vec![version("calibration")];
    input.authority.approvals = vec![local(10), local(11)];
    input.authority.consent = Some(ConsentRef {
        binding: binding("synthetic-tenant"),
        epoch: 1,
        policy: version("consent"),
        attestation: local(12),
    });
    input.evidence.source_refs = vec![
        artifact(ArtifactKind::Model, 5),
        artifact(ArtifactKind::Authority, 6),
    ];
    input.evidence.observation_refs = vec![artifact(ArtifactKind::Observation, 7)];
    let dim = Estimate::Known(artifact(ArtifactKind::Evidence, 8));
    input.evidence.dimensions = RecordedEvidence {
        cryptographic: dim.clone(),
        source: dim.clone(),
        calibration: dim.clone(),
        outcome: dim.clone(),
        compatibility: dim.clone(),
        regime: dim.clone(),
        lifecycle: dim.clone(),
        confidence: dim,
    };
    input.lifecycle.retention = RetentionIntent::Operational {
        expires_at: UtcMillis::new(100),
    };
    input.lifecycle.holds = vec![local(13)];
    input.lifecycle.deletion_key = Some(ObjectKeyRef {
        binding: binding("synthetic-tenant"),
        key_id: OpaqueRef::new("synthetic-key").unwrap(),
    });
    input.lifecycle.edges = vec![
        LifecycleLink {
            relation: LifecycleRelation::Contradicts,
            target: local(14),
            basis: local(15),
        },
        LifecycleLink {
            relation: LifecycleRelation::Resolves,
            target: local(14),
            basis: local(15),
        },
    ];
    input.provenance.causal_parents = vec![local(16), local(17)];
    input.provenance.consulted_parents = vec![
        ConsultedRef::Local(local(16)),
        ConsultedRef::Foreign(ForeignEvidenceRef {
            origin_organization: OrganizationRef::new("foreign-org").unwrap(),
            origin_binding: ScopeBinding {
                tenant: TenantRef::new("foreign-tenant").unwrap(),
                scope: Scope::Organization(OrganizationRef::new("foreign-org").unwrap()),
            },
            domain: EpisodeDomain::LegacyEpisodeV1,
            digest: Digest32::new([18; 32]),
            import_receipt: local(19),
        }),
    ];
    input.provenance.records = vec![artifact(ArtifactKind::Provenance, 9)];
    input
}
