//! Private typed drafts. Group constructors defer relational validation to `try_new`.
use super::primitives::*;
use super::{CompleteEpisodeError as E, Result};
use crate::v3::episode::{ActorKind, EvidenceLevel, SignerKeyId};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum EpisodeDomain {
    CompleteV1,
    LegacyEpisodeV1,
}
redacted!(EpisodeDomain);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    NoModel,
    LocalModel,
    CloudModel,
    Specialist,
    Deterministic,
}
redacted!(Route);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionPhase {
    Proposed,
    Performed,
}
redacted!(ActionPhase);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MeasurementUnit {
    Unitless,
    Count,
    Milliseconds,
    Bytes,
    Microjoules,
}
redacted!(MeasurementUnit);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceKind {
    Tokens,
    CpuMicros,
    MemoryBytes,
    NetworkBytes,
    EnergyMicrojoules,
}
redacted!(ResourceKind);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactKind {
    StateSummary,
    Prediction,
    Outcome,
    Observation,
    Model,
    Calibration,
    Provenance,
    ReservoirEmbedding,
    Evidence,
    Authority,
}
redacted!(ArtifactKind);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SafetyEffect {
    Unknown,
    NoViolationObserved,
    InterlockTriggered,
    ViolationObserved,
}
redacted!(SafetyEffect);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RollbackState {
    NotApplicable,
    NotAttempted,
    Attempted,
    ExactRestoreObserved,
    Failed,
    Unknown,
}
redacted!(RollbackState);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LifecycleRelation {
    Supersedes,
    Contradicts,
    Revokes,
    Quarantines,
    Resolves,
    Redacts,
    Forgets,
}
redacted!(LifecycleRelation);

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Scope {
    Guild(GuildRef),
    GuildMember {
        guild: GuildRef,
        member: PrincipalRef,
    },
    DirectMessage {
        user: PrincipalRef,
    },
    Organization(OrganizationRef),
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum ConsultedRef {
    Local(LocalEpisodeRef),
    Foreign(ForeignEvidenceRef),
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum RetentionIntent {
    Ephemeral,
    Session {
        session: OpaqueRef,
        expires_at: UtcMillis,
    },
    Operational {
        expires_at: UtcMillis,
    },
    Durable {
        privacy_basis: LocalEpisodeRef,
    },
    MandatoryIncident {
        obligation: LocalEpisodeRef,
    },
}
redacted!(Scope, ConsultedRef, RetentionIntent);

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ScopeBinding {
    pub(super) tenant: TenantRef,
    pub(super) scope: Scope,
}
redacted!(ScopeBinding);
impl ScopeBinding {
    pub(crate) fn new(fields: (TenantRef, Scope)) -> Self {
        let (tenant, scope) = fields;
        Self { tenant, scope }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct VersionRef {
    pub(super) component: ComponentRef,
    pub(super) revision: OpaqueRef,
    pub(super) digest: Digest32,
}
redacted!(VersionRef);
impl VersionRef {
    pub(crate) fn new(fields: (ComponentRef, OpaqueRef, Digest32)) -> Self {
        let (component, revision, digest) = fields;
        Self {
            component,
            revision,
            digest,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct LocalEpisodeRef {
    pub(super) binding: ScopeBinding,
    pub(super) domain: EpisodeDomain,
    pub(super) digest: Digest32,
}
redacted!(LocalEpisodeRef);
impl LocalEpisodeRef {
    pub(crate) fn new(fields: (ScopeBinding, EpisodeDomain, Digest32)) -> Self {
        let (binding, domain, digest) = fields;
        Self {
            binding,
            domain,
            digest,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ArtifactRef {
    pub(super) binding: ScopeBinding,
    pub(super) digest: Digest32,
    pub(super) byte_length: u64,
    pub(super) kind: ArtifactKind,
    pub(super) format: VersionRef,
}
redacted!(ArtifactRef);
impl ArtifactRef {
    pub(crate) fn new(fields: (ScopeBinding, Digest32, u64, ArtifactKind, VersionRef)) -> Self {
        let (binding, digest, byte_length, kind, format) = fields;
        Self {
            binding,
            digest,
            byte_length,
            kind,
            format,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ForeignEvidenceRef {
    pub(super) origin_organization: OrganizationRef,
    pub(super) origin_binding: ScopeBinding,
    pub(super) domain: EpisodeDomain,
    pub(super) digest: Digest32,
    pub(super) import_receipt: LocalEpisodeRef,
}
redacted!(ForeignEvidenceRef);
impl ForeignEvidenceRef {
    pub(crate) fn new(
        fields: (
            OrganizationRef,
            ScopeBinding,
            EpisodeDomain,
            Digest32,
            LocalEpisodeRef,
        ),
    ) -> Self {
        let (origin_organization, origin_binding, domain, digest, import_receipt) = fields;
        Self {
            origin_organization,
            origin_binding,
            domain,
            digest,
            import_receipt,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Actor {
    pub(super) principal: PrincipalRef,
    pub(super) kind: ActorKind,
}
redacted!(Actor);
impl Actor {
    pub(crate) fn new(fields: (PrincipalRef, ActorKind)) -> Self {
        let (principal, kind) = fields;
        Self { principal, kind }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Measurement {
    pub(super) unit: MeasurementUnit,
    pub(super) value: SignedMicros,
}
redacted!(Measurement);

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct MonotonicRange {
    pub(super) clock: ClockRef,
    pub(super) start: MonoNanos,
    pub(super) end: MonoNanos,
}
redacted!(MonotonicRange);

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ResourceReading {
    pub(super) resource: ResourceKind,
    pub(super) amount: u64,
}
redacted!(ResourceReading);
impl ResourceReading {
    pub(crate) fn new(fields: (ResourceKind, u64)) -> Self {
        let (resource, amount) = fields;
        Self { resource, amount }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ActionDescriptor {
    pub(super) capability: VersionRef,
    pub(super) action: ActionRef,
    pub(super) parameter_classes: Vec<ParameterClassRef>,
    pub(super) phase: ActionPhase,
}
redacted!(ActionDescriptor);
impl ActionDescriptor {
    pub(crate) fn new(
        fields: (VersionRef, ActionRef, Vec<ParameterClassRef>, ActionPhase),
    ) -> Self {
        let (capability, action, parameter_classes, phase) = fields;
        Self {
            capability,
            action,
            parameter_classes,
            phase,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ConsentRef {
    pub(super) binding: ScopeBinding,
    pub(super) epoch: u64,
    pub(super) policy: VersionRef,
    pub(super) attestation: LocalEpisodeRef,
}
redacted!(ConsentRef);
impl ConsentRef {
    pub(crate) fn new(fields: (ScopeBinding, u64, VersionRef, LocalEpisodeRef)) -> Self {
        let (binding, epoch, policy, attestation) = fields;
        Self {
            binding,
            epoch,
            policy,
            attestation,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ObjectKeyRef {
    pub(super) binding: ScopeBinding,
    pub(super) key_id: OpaqueRef,
}
redacted!(ObjectKeyRef);
impl ObjectKeyRef {
    pub(crate) fn new(fields: (ScopeBinding, OpaqueRef)) -> Self {
        let (binding, key_id) = fields;
        Self { binding, key_id }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct LifecycleLink {
    pub(super) relation: LifecycleRelation,
    pub(super) target: LocalEpisodeRef,
    pub(super) basis: LocalEpisodeRef,
}
redacted!(LifecycleLink);
impl LifecycleLink {
    pub(crate) fn new(fields: (LifecycleRelation, LocalEpisodeRef, LocalEpisodeRef)) -> Self {
        let (relation, target, basis) = fields;
        Self {
            relation,
            target,
            basis,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Identity {
    pub(super) episode_id: EpisodeId,
    pub(super) request_id: RequestId,
    pub(super) binding: ScopeBinding,
    pub(super) contract: VersionRef,
}
redacted!(Identity);
impl Identity {
    pub(crate) fn new(fields: (EpisodeId, RequestId, ScopeBinding, VersionRef)) -> Self {
        let (episode_id, request_id, binding, contract) = fields;
        Self {
            episode_id,
            request_id,
            binding,
            contract,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Time {
    pub(super) acquired_at: UtcMillis,
    pub(super) received_at: UtcMillis,
    pub(super) monotonic: Option<MonotonicRange>,
}
redacted!(Time);
impl Time {
    pub(crate) fn new(fields: (UtcMillis, UtcMillis, Option<MonotonicRange>)) -> Self {
        let (acquired_at, received_at, monotonic) = fields;
        Self {
            acquired_at,
            received_at,
            monotonic,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Origin {
    pub(super) source_node: NodeRef,
    pub(super) actor: Actor,
    pub(super) signer: SignerKeyId,
}
redacted!(Origin);
impl Origin {
    pub(crate) fn new(fields: (NodeRef, Actor, SignerKeyId)) -> Self {
        let (source_node, actor, signer) = fields;
        Self {
            source_node,
            actor,
            signer,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Context {
    pub(super) task_id: TaskId,
    pub(super) regime: RegimeRef,
    pub(super) regime_posterior: Estimate<RegimeDistribution>,
    pub(super) habituation: Estimate<UnitPpm>,
    pub(super) summary: Option<ArtifactRef>,
    pub(super) resources: Vec<ResourceReading>,
    pub(super) reservoir: Option<ArtifactRef>,
}
redacted!(Context);
pub(crate) type ContextFields = (
    TaskId,
    RegimeRef,
    Estimate<RegimeDistribution>,
    Estimate<UnitPpm>,
    Option<ArtifactRef>,
    Vec<ResourceReading>,
    Option<ArtifactRef>,
);
impl Context {
    pub(crate) fn new(fields: ContextFields) -> Self {
        let (task_id, regime, regime_posterior, habituation, summary, resources, reservoir) =
            fields;
        Self {
            task_id,
            regime,
            regime_posterior,
            habituation,
            summary,
            resources,
            reservoir,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Decision {
    pub(super) route: Route,
    pub(super) prediction: Estimate<ArtifactRef>,
    pub(super) actions: Vec<ActionDescriptor>,
    pub(super) uncertainty: Estimate<UnitPpm>,
    pub(super) risk: Estimate<UnitPpm>,
    pub(super) novelty: Estimate<UnitPpm>,
}
redacted!(Decision);
pub(crate) type DecisionFields = (
    Route,
    Estimate<ArtifactRef>,
    Vec<ActionDescriptor>,
    Estimate<UnitPpm>,
    Estimate<UnitPpm>,
    Estimate<UnitPpm>,
);
impl Decision {
    pub(crate) fn new(fields: DecisionFields) -> Self {
        let (route, prediction, actions, uncertainty, risk, novelty) = fields;
        Self {
            route,
            prediction,
            actions,
            uncertainty,
            risk,
            novelty,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub(super) observed: Estimate<ArtifactRef>,
    pub(super) utility: Estimate<Measurement>,
    pub(super) safety_effect: SafetyEffect,
    pub(super) costs: Vec<ResourceReading>,
    pub(super) rollback: RollbackState,
}
redacted!(Outcome);
impl Outcome {
    pub(crate) fn new(
        fields: (
            Estimate<ArtifactRef>,
            Estimate<Measurement>,
            SafetyEffect,
            Vec<ResourceReading>,
            RollbackState,
        ),
    ) -> Self {
        let (observed, utility, safety_effect, costs, rollback) = fields;
        Self {
            observed,
            utility,
            safety_effect,
            costs,
            rollback,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Authority {
    pub(super) policy: VersionRef,
    pub(super) safety_policy: VersionRef,
    pub(super) capabilities: Vec<VersionRef>,
    pub(super) models: Vec<VersionRef>,
    pub(super) calibrations: Vec<VersionRef>,
    pub(super) approvals: Vec<LocalEpisodeRef>,
    pub(super) consent: Option<ConsentRef>,
}
redacted!(Authority);
pub(crate) type AuthorityFields = (
    VersionRef,
    VersionRef,
    Vec<VersionRef>,
    Vec<VersionRef>,
    Vec<VersionRef>,
    Vec<LocalEpisodeRef>,
    Option<ConsentRef>,
);
impl Authority {
    pub(crate) fn new(fields: AuthorityFields) -> Self {
        let (policy, safety_policy, capabilities, models, calibrations, approvals, consent) =
            fields;
        Self {
            policy,
            safety_policy,
            capabilities,
            models,
            calibrations,
            approvals,
            consent,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct RecordedEvidence {
    pub(super) cryptographic: Estimate<ArtifactRef>,
    pub(super) source: Estimate<ArtifactRef>,
    pub(super) calibration: Estimate<ArtifactRef>,
    pub(super) outcome: Estimate<ArtifactRef>,
    pub(super) compatibility: Estimate<ArtifactRef>,
    pub(super) regime: Estimate<ArtifactRef>,
    pub(super) lifecycle: Estimate<ArtifactRef>,
    pub(super) confidence: Estimate<ArtifactRef>,
}
redacted!(RecordedEvidence);
pub(crate) type RecordedEvidenceFields = (
    Estimate<ArtifactRef>,
    Estimate<ArtifactRef>,
    Estimate<ArtifactRef>,
    Estimate<ArtifactRef>,
    Estimate<ArtifactRef>,
    Estimate<ArtifactRef>,
    Estimate<ArtifactRef>,
    Estimate<ArtifactRef>,
);
impl RecordedEvidence {
    pub(crate) fn new(fields: RecordedEvidenceFields) -> Self {
        let (
            cryptographic,
            source,
            calibration,
            outcome,
            compatibility,
            regime,
            lifecycle,
            confidence,
        ) = fields;
        Self {
            cryptographic,
            source,
            calibration,
            outcome,
            compatibility,
            regime,
            lifecycle,
            confidence,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Evidence {
    pub(super) maturity: EvidenceLevel,
    pub(super) source_refs: Vec<ArtifactRef>,
    pub(super) observation_refs: Vec<ArtifactRef>,
    pub(super) dimensions: RecordedEvidence,
}
redacted!(Evidence);
impl Evidence {
    pub(crate) fn new(
        fields: (
            EvidenceLevel,
            Vec<ArtifactRef>,
            Vec<ArtifactRef>,
            RecordedEvidence,
        ),
    ) -> Self {
        let (maturity, source_refs, observation_refs, dimensions) = fields;
        Self {
            maturity,
            source_refs,
            observation_refs,
            dimensions,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Lifecycle {
    pub(super) retention: RetentionIntent,
    pub(super) holds: Vec<LocalEpisodeRef>,
    pub(super) deletion_key: Option<ObjectKeyRef>,
    pub(super) edges: Vec<LifecycleLink>,
}
redacted!(Lifecycle);
impl Lifecycle {
    pub(crate) fn new(
        fields: (
            RetentionIntent,
            Vec<LocalEpisodeRef>,
            Option<ObjectKeyRef>,
            Vec<LifecycleLink>,
        ),
    ) -> Self {
        let (retention, holds, deletion_key, edges) = fields;
        Self {
            retention,
            holds,
            deletion_key,
            edges,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Provenance {
    pub(super) causal_parents: Vec<LocalEpisodeRef>,
    pub(super) consulted_parents: Vec<ConsultedRef>,
    pub(super) records: Vec<ArtifactRef>,
}
redacted!(Provenance);
impl Provenance {
    pub(crate) fn new(fields: (Vec<LocalEpisodeRef>, Vec<ConsultedRef>, Vec<ArtifactRef>)) -> Self {
        let (causal_parents, consulted_parents, records) = fields;
        Self {
            causal_parents,
            consulted_parents,
            records,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CompleteEpisodeInput {
    pub(super) identity: Identity,
    pub(super) time: Time,
    pub(super) origin: Origin,
    pub(super) context: Context,
    pub(super) decision: Decision,
    pub(super) outcome: Outcome,
    pub(super) authority: Authority,
    pub(super) evidence: Evidence,
    pub(super) lifecycle: Lifecycle,
    pub(super) provenance: Provenance,
}
redacted!(CompleteEpisodeInput);
impl CompleteEpisodeInput {
    pub(crate) fn new(
        fields: (
            Identity,
            Time,
            Origin,
            Context,
            Decision,
            Outcome,
            Authority,
            Evidence,
            Lifecycle,
            Provenance,
        ),
    ) -> Self {
        let (
            identity,
            time,
            origin,
            context,
            decision,
            outcome,
            authority,
            evidence,
            lifecycle,
            provenance,
        ) = fields;
        Self {
            identity,
            time,
            origin,
            context,
            decision,
            outcome,
            authority,
            evidence,
            lifecycle,
            provenance,
        }
    }
}

impl Measurement {
    pub(crate) fn new(unit: MeasurementUnit, value: SignedMicros) -> Result<Self> {
        let value = Self { unit, value };
        value.validate()?;
        Ok(value)
    }
    pub(super) fn validate(&self) -> Result<()> {
        if matches!(self.unit, MeasurementUnit::Count | MeasurementUnit::Bytes)
            && (self.value.0 < 0 || self.value.0 % 1_000_000 != 0)
        {
            return Err(E::InvalidRange);
        }
        Ok(())
    }
}
impl MonotonicRange {
    pub(crate) fn new(fields: (ClockRef, MonoNanos, MonoNanos)) -> Result<Self> {
        let (clock, start, end) = fields;
        if start.0 > end.0 {
            return Err(E::InvalidRange);
        }
        Ok(Self { clock, start, end })
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct RegimeDistribution(pub(super) Vec<(RegimeRef, UnitPpm)>);
redacted!(RegimeDistribution);
impl RegimeDistribution {
    pub(crate) fn new(entries: Vec<(RegimeRef, UnitPpm)>) -> Result<Self> {
        let value = Self(entries);
        value.validate()?;
        Ok(value)
    }
    pub(super) fn validate(&self) -> Result<()> {
        if self.0.is_empty() || self.0.len() > 32 {
            return Err(E::InvalidPosterior);
        }
        let mut sum = 0u64;
        for (i, (regime, probability)) in self.0.iter().enumerate() {
            if self.0[..i].iter().any(|(r, _)| r == regime) {
                return Err(E::InvalidPosterior);
            }
            sum = sum
                .checked_add(u64::from(probability.0))
                .ok_or(E::InvalidPosterior)?;
        }
        if sum != 1_000_000 {
            return Err(E::InvalidPosterior);
        }
        Ok(())
    }
}
