//! Explicit inverse vocabulary; numeric tags are never Rust discriminants.
use super::*;
use crate::v3::episode::{ActorKind, EvidenceLevel};
pub(super) fn episode_domain(value: C) -> DResult<EpisodeDomain> {
    match unit(value)? {
        0 => Ok(EpisodeDomain::CompleteV1),
        1 => Ok(EpisodeDomain::LegacyEpisodeV1),
        _ => Err(E::UnknownVariant),
    }
}
pub(super) fn route(value: C) -> DResult<Route> {
    match unit(value)? {
        0 => Ok(Route::NoModel),
        1 => Ok(Route::LocalModel),
        2 => Ok(Route::CloudModel),
        3 => Ok(Route::Specialist),
        4 => Ok(Route::Deterministic),
        _ => Err(E::UnknownVariant),
    }
}
pub(super) fn phase(value: C) -> DResult<ActionPhase> {
    match unit(value)? {
        0 => Ok(ActionPhase::Proposed),
        1 => Ok(ActionPhase::Performed),
        _ => Err(E::UnknownVariant),
    }
}
pub(super) fn measurement_unit(value: C) -> DResult<MeasurementUnit> {
    match unit(value)? {
        0 => Ok(MeasurementUnit::Unitless),
        1 => Ok(MeasurementUnit::Count),
        2 => Ok(MeasurementUnit::Milliseconds),
        3 => Ok(MeasurementUnit::Bytes),
        4 => Ok(MeasurementUnit::Microjoules),
        _ => Err(E::UnknownVariant),
    }
}
pub(super) fn resource_kind(value: C) -> DResult<ResourceKind> {
    match unit(value)? {
        0 => Ok(ResourceKind::Tokens),
        1 => Ok(ResourceKind::CpuMicros),
        2 => Ok(ResourceKind::MemoryBytes),
        3 => Ok(ResourceKind::NetworkBytes),
        4 => Ok(ResourceKind::EnergyMicrojoules),
        _ => Err(E::UnknownVariant),
    }
}
pub(super) fn artifact_kind(value: C) -> DResult<ArtifactKind> {
    match unit(value)? {
        0 => Ok(ArtifactKind::StateSummary),
        1 => Ok(ArtifactKind::Prediction),
        2 => Ok(ArtifactKind::Outcome),
        3 => Ok(ArtifactKind::Observation),
        4 => Ok(ArtifactKind::Model),
        5 => Ok(ArtifactKind::Calibration),
        6 => Ok(ArtifactKind::Provenance),
        7 => Ok(ArtifactKind::ReservoirEmbedding),
        8 => Ok(ArtifactKind::Evidence),
        9 => Ok(ArtifactKind::Authority),
        _ => Err(E::UnknownVariant),
    }
}
pub(super) fn safety(value: C) -> DResult<SafetyEffect> {
    match unit(value)? {
        0 => Ok(SafetyEffect::Unknown),
        1 => Ok(SafetyEffect::NoViolationObserved),
        2 => Ok(SafetyEffect::InterlockTriggered),
        3 => Ok(SafetyEffect::ViolationObserved),
        _ => Err(E::UnknownVariant),
    }
}
pub(super) fn rollback(value: C) -> DResult<RollbackState> {
    match unit(value)? {
        0 => Ok(RollbackState::NotApplicable),
        1 => Ok(RollbackState::NotAttempted),
        2 => Ok(RollbackState::Attempted),
        3 => Ok(RollbackState::ExactRestoreObserved),
        4 => Ok(RollbackState::Failed),
        5 => Ok(RollbackState::Unknown),
        _ => Err(E::UnknownVariant),
    }
}
pub(super) fn relation(value: C) -> DResult<LifecycleRelation> {
    match unit(value)? {
        0 => Ok(LifecycleRelation::Supersedes),
        1 => Ok(LifecycleRelation::Contradicts),
        2 => Ok(LifecycleRelation::Revokes),
        3 => Ok(LifecycleRelation::Quarantines),
        4 => Ok(LifecycleRelation::Resolves),
        5 => Ok(LifecycleRelation::Redacts),
        6 => Ok(LifecycleRelation::Forgets),
        _ => Err(E::UnknownVariant),
    }
}
pub(super) fn actor_kind(value: C) -> DResult<ActorKind> {
    match text(value)?.as_str() {
        "human_subject" => Ok(ActorKind::HumanSubject),
        "organization_owner" => Ok(ActorKind::OrganizationOwner),
        "guild_owner" => Ok(ActorKind::GuildOwner),
        "guild_administrator" => Ok(ActorKind::GuildAdministrator),
        "guild_manager" => Ok(ActorKind::GuildManager),
        "service" => Ok(ActorKind::Service),
        _ => Err(E::UnknownVariant),
    }
}
pub(super) fn maturity(value: C) -> DResult<EvidenceLevel> {
    match text(value)?.as_str() {
        "C0" => Ok(EvidenceLevel::C0),
        "C1" => Ok(EvidenceLevel::C1),
        "C2" => Ok(EvidenceLevel::C2),
        "C3" => Ok(EvidenceLevel::C3),
        "C4" => Ok(EvidenceLevel::C4),
        "C5" => Ok(EvidenceLevel::C5),
        "C6" => Ok(EvidenceLevel::C6),
        "C7" => Ok(EvidenceLevel::C7),
        _ => Err(E::UnknownVariant),
    }
}
