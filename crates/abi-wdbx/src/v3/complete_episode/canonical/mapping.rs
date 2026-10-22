//! Exact provisional field mapping; each enum tag is explicit.
use super::*;
impl Wire for TenantRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        self.0.wire::<B>()
    }
}
impl Wire for OrganizationRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        self.0.wire::<B>()
    }
}
impl Wire for PrincipalRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        self.0.wire::<B>()
    }
}
impl Wire for GuildRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        self.0.wire::<B>()
    }
}
impl Wire for NodeRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        self.0.wire::<B>()
    }
}
impl Wire for RegimeRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        self.0.wire::<B>()
    }
}
impl Wire for ClockRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        self.0.wire::<B>()
    }
}
impl Wire for ComponentRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        self.0.wire::<B>()
    }
}
impl Wire for ActionRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        self.0.wire::<B>()
    }
}
impl Wire for ParameterClassRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        self.0.wire::<B>()
    }
}
impl Wire for EpisodeId {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::bytes(&self.0)
    }
}
impl Wire for RequestId {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::bytes(&self.0)
    }
}
impl Wire for TaskId {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::bytes(&self.0)
    }
}
impl Wire for Digest32 {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::bytes(&self.0)
    }
}
impl Wire for UtcMillis {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::unsigned(self.0)
    }
}
impl Wire for MonoNanos {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::unsigned(self.0)
    }
}
impl Wire for UnitPpm {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::unsigned(u64::from(self.0))
    }
}
impl Wire for SignedMicros {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::signed(self.0)
    }
}
impl Wire for EpisodeDomain {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let tag = match self {
            Self::CompleteV1 => 0,
            Self::LegacyEpisodeV1 => 1,
        };
        B::array(vec![B::unsigned(tag)?])
    }
}
impl Wire for Route {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let tag = match self {
            Self::NoModel => 0,
            Self::LocalModel => 1,
            Self::CloudModel => 2,
            Self::Specialist => 3,
            Self::Deterministic => 4,
        };
        B::array(vec![B::unsigned(tag)?])
    }
}
impl Wire for ActionPhase {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let tag = match self {
            Self::Proposed => 0,
            Self::Performed => 1,
        };
        B::array(vec![B::unsigned(tag)?])
    }
}
impl Wire for MeasurementUnit {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let tag = match self {
            Self::Unitless => 0,
            Self::Count => 1,
            Self::Milliseconds => 2,
            Self::Bytes => 3,
            Self::Microjoules => 4,
        };
        B::array(vec![B::unsigned(tag)?])
    }
}
impl Wire for ResourceKind {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let tag = match self {
            Self::Tokens => 0,
            Self::CpuMicros => 1,
            Self::MemoryBytes => 2,
            Self::NetworkBytes => 3,
            Self::EnergyMicrojoules => 4,
        };
        B::array(vec![B::unsigned(tag)?])
    }
}
impl Wire for ArtifactKind {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let tag = match self {
            Self::StateSummary => 0,
            Self::Prediction => 1,
            Self::Outcome => 2,
            Self::Observation => 3,
            Self::Model => 4,
            Self::Calibration => 5,
            Self::Provenance => 6,
            Self::ReservoirEmbedding => 7,
            Self::Evidence => 8,
            Self::Authority => 9,
        };
        B::array(vec![B::unsigned(tag)?])
    }
}
impl Wire for SafetyEffect {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let tag = match self {
            Self::Unknown => 0,
            Self::NoViolationObserved => 1,
            Self::InterlockTriggered => 2,
            Self::ViolationObserved => 3,
        };
        B::array(vec![B::unsigned(tag)?])
    }
}
impl Wire for RollbackState {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let tag = match self {
            Self::NotApplicable => 0,
            Self::NotAttempted => 1,
            Self::Attempted => 2,
            Self::ExactRestoreObserved => 3,
            Self::Failed => 4,
            Self::Unknown => 5,
        };
        B::array(vec![B::unsigned(tag)?])
    }
}
impl Wire for LifecycleRelation {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let tag = match self {
            Self::Supersedes => 0,
            Self::Contradicts => 1,
            Self::Revokes => 2,
            Self::Quarantines => 3,
            Self::Resolves => 4,
            Self::Redacts => 5,
            Self::Forgets => 6,
        };
        B::array(vec![B::unsigned(tag)?])
    }
}
impl Wire for ScopeBinding {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("tenant", self.tenant.wire::<B>()?),
            ("scope", self.scope.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for VersionRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("component", self.component.wire::<B>()?),
            ("revision", self.revision.wire::<B>()?),
            ("digest", self.digest.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for LocalEpisodeRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("binding", self.binding.wire::<B>()?),
            ("domain", self.domain.wire::<B>()?),
            ("digest", self.digest.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for ArtifactRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("binding", self.binding.wire::<B>()?),
            ("digest", self.digest.wire::<B>()?),
            ("byte_length", self.byte_length.wire::<B>()?),
            ("kind", self.kind.wire::<B>()?),
            ("format", self.format.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for ForeignEvidenceRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("origin_organization", self.origin_organization.wire::<B>()?),
            ("origin_binding", self.origin_binding.wire::<B>()?),
            ("domain", self.domain.wire::<B>()?),
            ("digest", self.digest.wire::<B>()?),
            ("import_receipt", self.import_receipt.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for Actor {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("principal", self.principal.wire::<B>()?),
            ("kind", self.kind.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for Measurement {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("unit", self.unit.wire::<B>()?),
            ("value", self.value.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for MonotonicRange {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("clock", self.clock.wire::<B>()?),
            ("start", self.start.wire::<B>()?),
            ("end", self.end.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for ResourceReading {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("resource", self.resource.wire::<B>()?),
            ("amount", self.amount.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for ActionDescriptor {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("capability", self.capability.wire::<B>()?),
            ("action", self.action.wire::<B>()?),
            (
                "parameter_classes",
                parameters::<B>(&self.parameter_classes)?,
            ),
            ("phase", self.phase.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for ConsentRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("binding", self.binding.wire::<B>()?),
            ("epoch", self.epoch.wire::<B>()?),
            ("policy", self.policy.wire::<B>()?),
            ("attestation", self.attestation.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for ObjectKeyRef {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("binding", self.binding.wire::<B>()?),
            ("key_id", self.key_id.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for LifecycleLink {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("relation", self.relation.wire::<B>()?),
            ("target", self.target.wire::<B>()?),
            ("basis", self.basis.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for Identity {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("episode_id", self.episode_id.wire::<B>()?),
            ("request_id", self.request_id.wire::<B>()?),
            ("binding", self.binding.wire::<B>()?),
            ("contract", self.contract.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for Time {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let mut fields = vec![
            ("acquired_at", self.acquired_at.wire::<B>()?),
            ("received_at", self.received_at.wire::<B>()?),
        ];
        if let Some(value) = &self.monotonic {
            fields.push(("monotonic", value.wire::<B>()?));
        }
        B::map(fields)
    }
}
impl Wire for Origin {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("source_node", self.source_node.wire::<B>()?),
            ("actor", self.actor.wire::<B>()?),
            ("signer", self.signer.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for Context {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let mut fields = vec![
            ("task_id", self.task_id.wire::<B>()?),
            ("regime", self.regime.wire::<B>()?),
            ("regime_posterior", self.regime_posterior.wire::<B>()?),
            ("habituation", self.habituation.wire::<B>()?),
            ("resources", self.resources.wire::<B>()?),
        ];
        if let Some(value) = &self.summary {
            fields.push(("summary", value.wire::<B>()?));
        }
        if let Some(value) = &self.reservoir {
            fields.push(("reservoir", value.wire::<B>()?));
        }
        B::map(fields)
    }
}
impl Wire for Decision {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("route", self.route.wire::<B>()?),
            ("prediction", self.prediction.wire::<B>()?),
            ("actions", sequence::<B, _>(&self.actions)?),
            ("uncertainty", self.uncertainty.wire::<B>()?),
            ("risk", self.risk.wire::<B>()?),
            ("novelty", self.novelty.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for Outcome {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("observed", self.observed.wire::<B>()?),
            ("utility", self.utility.wire::<B>()?),
            ("safety_effect", self.safety_effect.wire::<B>()?),
            ("costs", self.costs.wire::<B>()?),
            ("rollback", self.rollback.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for Authority {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let mut fields = vec![
            ("policy", self.policy.wire::<B>()?),
            ("safety_policy", self.safety_policy.wire::<B>()?),
            ("capabilities", versions::<B>(&self.capabilities)?),
            ("models", versions::<B>(&self.models)?),
            ("calibrations", versions::<B>(&self.calibrations)?),
            ("approvals", self.approvals.wire::<B>()?),
        ];
        if let Some(value) = &self.consent {
            fields.push(("consent", value.wire::<B>()?));
        }
        B::map(fields)
    }
}
impl Wire for RecordedEvidence {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("cryptographic", self.cryptographic.wire::<B>()?),
            ("source", self.source.wire::<B>()?),
            ("calibration", self.calibration.wire::<B>()?),
            ("outcome", self.outcome.wire::<B>()?),
            ("compatibility", self.compatibility.wire::<B>()?),
            ("regime", self.regime.wire::<B>()?),
            ("lifecycle", self.lifecycle.wire::<B>()?),
            ("confidence", self.confidence.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for Evidence {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("maturity", self.maturity.wire::<B>()?),
            ("source_refs", self.source_refs.wire::<B>()?),
            ("observation_refs", self.observation_refs.wire::<B>()?),
            ("dimensions", self.dimensions.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for Lifecycle {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let mut fields = vec![
            ("retention", self.retention.wire::<B>()?),
            ("holds", self.holds.wire::<B>()?),
            ("edges", self.edges.wire::<B>()?),
        ];
        if let Some(value) = &self.deletion_key {
            fields.push(("deletion_key", value.wire::<B>()?));
        }
        B::map(fields)
    }
}
impl Wire for Provenance {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        let fields = vec![
            ("causal_parents", self.causal_parents.wire::<B>()?),
            ("consulted_parents", self.consulted_parents.wire::<B>()?),
            ("records", self.records.wire::<B>()?),
        ];
        B::map(fields)
    }
}
impl Wire for CompleteEpisodeInput {
    fn wire<B: Builder>(&self) -> Result<B::Value> {
        B::envelope(vec![
            B::text(COMPLETE_EPISODE_DOMAIN)?,
            B::unsigned(1)?,
            self.identity.wire::<B>()?,
            self.time.wire::<B>()?,
            self.origin.wire::<B>()?,
            self.context.wire::<B>()?,
            self.decision.wire::<B>()?,
            self.outcome.wire::<B>()?,
            self.authority.wire::<B>()?,
            self.evidence.wire::<B>()?,
            self.lifecycle.wire::<B>()?,
            self.provenance.wire::<B>()?,
        ])
    }
}
