//! Structural validation only: no reference resolution or authority inference.
use super::*;
use CompleteEpisodeError as E;

fn limit<T>(values: &[T], maximum: usize) -> Result<()> {
    if values.len() > maximum {
        return Err(E::CollectionLimit);
    }
    Ok(())
}
fn unique<T: PartialEq>(values: &[T]) -> Result<()> {
    for (i, value) in values.iter().enumerate() {
        if values[..i].contains(value) {
            return Err(E::DuplicateReference);
        }
    }
    Ok(())
}
fn unique_by<'a, T: 'a, K: PartialEq>(values: &'a [T], key: impl Fn(&'a T) -> K) -> Result<()> {
    for (i, value) in values.iter().enumerate() {
        if values[..i].iter().any(|v| key(v) == key(value)) {
            return Err(E::DuplicateReference);
        }
    }
    Ok(())
}
fn same(binding: &ScopeBinding, expected: &ScopeBinding) -> Result<()> {
    if binding != expected {
        return Err(E::ScopeMismatch);
    }
    Ok(())
}
fn known<T>(estimate: &Estimate<T>) -> Option<&T> {
    match estimate {
        Estimate::Unknown => None,
        Estimate::Known(v) => Some(v),
    }
}
fn collection_limits(input: &CompleteEpisodeInput) -> Result<()> {
    // Every caller-owned list is bounded before traversal, duplicate comparison,
    // sorting, cloning, or canonical mapping, including lists nested in actions.
    limit(&input.decision.actions, 128)?;
    for action in &input.decision.actions {
        limit(&action.parameter_classes, 32)?;
    }
    for values in [&input.context.resources, &input.outcome.costs] {
        limit(values, 16)?;
    }
    for values in [
        &input.authority.capabilities,
        &input.authority.models,
        &input.authority.calibrations,
    ] {
        limit(values, 32)?;
    }
    for values in [
        &input.authority.approvals,
        &input.lifecycle.holds,
        &input.provenance.causal_parents,
    ] {
        limit(values, 64)?;
    }
    limit(&input.lifecycle.edges, 64)?;
    limit(&input.provenance.consulted_parents, 256)?;
    for values in [
        &input.evidence.source_refs,
        &input.evidence.observation_refs,
        &input.provenance.records,
    ] {
        limit(values, 256)?;
    }
    Ok(())
}
pub(super) fn validate(input: &CompleteEpisodeInput) -> Result<()> {
    collection_limits(input)?;
    if let Some(posterior) = known(&input.context.regime_posterior) {
        posterior.validate()?;
    }
    if let Some(range) = &input.time.monotonic
        && range.start.0 > range.end.0
    {
        return Err(E::InvalidRange);
    }
    if let Some(utility) = known(&input.outcome.utility) {
        utility.validate()?;
    }
    match &input.lifecycle.retention {
        RetentionIntent::Session { expires_at, .. }
        | RetentionIntent::Operational { expires_at }
            if expires_at.0 <= input.time.received_at.0 =>
        {
            return Err(E::InvalidRange);
        }
        _ => {}
    }
    for action in &input.decision.actions {
        unique(&action.parameter_classes)?;
    }
    for values in [&input.context.resources, &input.outcome.costs] {
        unique_by(values, |v| v.resource)?;
    }
    for values in [
        &input.authority.capabilities,
        &input.authority.models,
        &input.authority.calibrations,
    ] {
        unique_by(values, |v| &v.component)?;
    }
    for values in [
        &input.authority.approvals,
        &input.lifecycle.holds,
        &input.provenance.causal_parents,
    ] {
        unique(values)?;
    }
    unique(&input.lifecycle.edges)?;
    unique(&input.provenance.consulted_parents)?;
    for values in [
        &input.evidence.source_refs,
        &input.evidence.observation_refs,
        &input.provenance.records,
    ] {
        unique(values)?;
    }
    let binding = &input.identity.binding;
    if let Some(consent) = &input.authority.consent {
        same(&consent.binding, binding)?;
    }
    if let Some(key) = &input.lifecycle.deletion_key {
        same(&key.binding, binding)?;
    }
    for reference in &input.provenance.consulted_parents {
        if let ConsultedRef::Foreign(foreign) = reference
            && let Scope::Organization(organization) = &foreign.origin_binding.scope
            && organization != &foreign.origin_organization
        {
            return Err(E::ScopeMismatch);
        }
    }
    walk(
        input,
        |reference| same(&reference.binding, binding),
        |artifact, kind| {
            same(&artifact.binding, binding)?;
            if artifact.byte_length > (1_u64 << 40) {
                return Err(E::InvalidRange);
            }
            if kind.is_some_and(|kind| kind != artifact.kind) {
                return Err(E::InvalidArtifactKind);
            }
            Ok(())
        },
    )
}

/// Central inventory of local references and separately scoped artifact slots.
/// `None` kind permits every defined kind in `source_refs`, intentionally.
fn walk(
    input: &CompleteEpisodeInput,
    mut local: impl FnMut(&LocalEpisodeRef) -> Result<()>,
    mut artifact: impl FnMut(&ArtifactRef, Option<ArtifactKind>) -> Result<()>,
) -> Result<()> {
    for (value, kind) in [
        (input.context.summary.as_ref(), ArtifactKind::StateSummary),
        (
            input.context.reservoir.as_ref(),
            ArtifactKind::ReservoirEmbedding,
        ),
        (known(&input.decision.prediction), ArtifactKind::Prediction),
        (known(&input.outcome.observed), ArtifactKind::Outcome),
    ] {
        if let Some(value) = value {
            artifact(value, Some(kind))?;
        }
    }
    let dims = &input.evidence.dimensions;
    for value in [
        &dims.cryptographic,
        &dims.source,
        &dims.calibration,
        &dims.outcome,
        &dims.compatibility,
        &dims.regime,
        &dims.lifecycle,
        &dims.confidence,
    ] {
        if let Some(value) = known(value) {
            artifact(value, Some(ArtifactKind::Evidence))?;
        }
    }
    for value in &input.evidence.source_refs {
        artifact(value, None)?;
    }
    for value in &input.evidence.observation_refs {
        artifact(value, Some(ArtifactKind::Observation))?;
    }
    for value in &input.provenance.records {
        artifact(value, Some(ArtifactKind::Provenance))?;
    }
    for values in [
        &input.authority.approvals,
        &input.lifecycle.holds,
        &input.provenance.causal_parents,
    ] {
        for value in values {
            local(value)?;
        }
    }
    if let Some(consent) = &input.authority.consent {
        local(&consent.attestation)?;
    }
    for link in &input.lifecycle.edges {
        local(&link.target)?;
        local(&link.basis)?;
    }
    match &input.lifecycle.retention {
        RetentionIntent::Durable { privacy_basis } => local(privacy_basis)?,
        RetentionIntent::MandatoryIncident { obligation } => local(obligation)?,
        _ => {}
    }
    for reference in &input.provenance.consulted_parents {
        match reference {
            ConsultedRef::Local(value) => local(value)?,
            ConsultedRef::Foreign(value) => local(&value.import_receipt)?,
        }
    }
    Ok(())
}
pub(super) fn reject_self_reference(input: &CompleteEpisodeInput, digest: [u8; 32]) -> Result<()> {
    walk(
        input,
        |reference| {
            if reference.domain == EpisodeDomain::CompleteV1 && reference.digest.0 == digest {
                return Err(E::SelfReference);
            }
            Ok(())
        },
        |artifact, _| {
            if artifact.digest.0 == digest {
                return Err(E::SelfReference);
            }
            Ok(())
        },
    )
}
