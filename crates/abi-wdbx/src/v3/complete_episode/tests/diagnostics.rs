use super::*;
use std::error::Error;

#[test]
fn every_new_type_redacts_private_leaves_and_signer_containment() {
    let input = populated();
    let mut diagnostics = vec![];
    macro_rules! debug {($($value:expr),+ $(,)?)=>{$(diagnostics.push(format!("{:?}",$value));)+};}
    debug!(
        input,
        input.identity,
        input.time,
        input.origin,
        input.context,
        input.decision,
        input.outcome,
        input.authority,
        input.evidence,
        input.lifecycle,
        input.provenance,
        input.identity.episode_id,
        input.identity.request_id,
        input.identity.binding,
        input.identity.binding.tenant,
        input.identity.binding.scope,
        input.identity.contract,
        input.identity.contract.component,
        input.identity.contract.revision,
        input.identity.contract.digest,
        input.time.acquired_at,
        input.time.received_at,
        input.time.monotonic.as_ref().unwrap(),
        input.time.monotonic.as_ref().unwrap().clock,
        input.time.monotonic.as_ref().unwrap().start,
        input.origin.source_node,
        input.origin.actor,
        input.origin.actor.principal,
        input.context.task_id,
        input.context.regime,
        input.context.regime_posterior,
        known_mut(&mut input.clone().context.regime_posterior),
        input.context.habituation,
        input.context.summary.as_ref().unwrap(),
        input.context.resources[0],
        input.context.resources[0].resource,
        input.decision.route,
        input.decision.actions[0],
        input.decision.actions[0].action,
        input.decision.actions[0].phase,
        input.decision.actions[0].parameter_classes[0],
        UnitPpm::new(987_654).unwrap(),
        known_mut(&mut input.clone().outcome.utility),
        SignedMicros::new(-987_654_321),
        MeasurementUnit::Unitless,
        input.outcome.safety_effect,
        input.outcome.rollback,
        input.authority.approvals[0],
        input.authority.approvals[0].domain,
        input.authority.consent.as_ref().unwrap(),
        input.evidence.dimensions,
        input.lifecycle.retention,
        input.lifecycle.edges[0],
        input.lifecycle.edges[0].relation,
        input.lifecycle.deletion_key.as_ref().unwrap(),
        input.provenance.consulted_parents[1],
        ArtifactKind::Evidence,
        GuildRef::new("sentinel-private-guild").unwrap(),
        OrganizationRef::new("sentinel-private-org").unwrap(),
        Digest32::new([199; 32]),
        UtcMillis::new(987_654_321),
        MonoNanos::new(987_654_321),
        CompleteEpisode::try_new(input.clone()).unwrap()
    );
    if let ConsultedRef::Foreign(f) = &input.provenance.consulted_parents[1] {
        debug!(f, f.origin_organization);
    }
    for text in diagnostics {
        for sentinel in [
            "synthetic",
            "sentinel-private",
            "ed25519:",
            "abababab",
            "987654",
            "199, 199",
            "c7c7c7c7",
            "1, 2, 3, 4",
            "01020304",
        ] {
            assert!(
                !text.contains(sentinel),
                "private diagnostic escaped: {text}"
            );
        }
    }
    // The existing signer ID intentionally has its old content-bearing API.
    // Only the new containing Origin and complete input/output promise redaction.
    assert!(!format!("{:?}", input.origin).contains(input.origin.signer.as_str()));
    for error in [
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
    ] {
        assert!(error.source().is_none());
        assert!(!format!("{error}: {error:?}").contains("synthetic"));
    }
}
