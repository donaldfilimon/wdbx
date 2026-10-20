//! Exact ten-group shape construction.
use super::*;
pub(super) fn identity(value: C) -> DResult<Identity> {
    let mut f = Fields::new(
        value,
        &["episode_id", "request_id", "binding", "contract"],
        &[],
    )?;
    Ok(Identity::new((
        EpisodeId::new(fixed(f.take("episode_id")?)?)?,
        RequestId::new(fixed(f.take("request_id")?)?)?,
        binding(f.take("binding")?)?,
        version(f.take("contract")?)?,
    )))
}
pub(super) fn time(value: C) -> DResult<Time> {
    let mut f = Fields::new(value, &["acquired_at", "received_at"], &["monotonic"])?;
    Ok(Time::new((
        utc(f.take("acquired_at")?)?,
        utc(f.take("received_at")?)?,
        f.optional("monotonic", monotonic)?,
    )))
}
pub(super) fn origin(value: C) -> DResult<Origin> {
    let mut f = Fields::new(value, &["source_node", "actor", "signer"], &[])?;
    Ok(Origin::new((
        token(f.take("source_node")?, NodeRef::new)?,
        actor(f.take("actor")?)?,
        signer(f.take("signer")?)?,
    )))
}
pub(super) fn context(value: C) -> DResult<Context> {
    let mut f = Fields::new(
        value,
        &[
            "task_id",
            "regime",
            "regime_posterior",
            "habituation",
            "resources",
        ],
        &["summary", "reservoir"],
    )?;
    Ok(Context::new((
        TaskId::new(fixed(f.take("task_id")?)?)?,
        token(f.take("regime")?, RegimeRef::new)?,
        estimate(f.take("regime_posterior")?, posterior)?,
        estimate(f.take("habituation")?, ppm)?,
        f.optional("summary", artifact)?,
        list(f.take("resources")?, 16, resource)?,
        f.optional("reservoir", artifact)?,
    )))
}
pub(super) fn decision(value: C) -> DResult<Decision> {
    let mut f = Fields::new(
        value,
        &[
            "route",
            "prediction",
            "actions",
            "uncertainty",
            "risk",
            "novelty",
        ],
        &[],
    )?;
    Ok(Decision::new((
        route(f.take("route")?)?,
        estimate(f.take("prediction")?, artifact)?,
        list(f.take("actions")?, 128, action)?,
        estimate(f.take("uncertainty")?, ppm)?,
        estimate(f.take("risk")?, ppm)?,
        estimate(f.take("novelty")?, ppm)?,
    )))
}
pub(super) fn outcome(value: C) -> DResult<Outcome> {
    let mut f = Fields::new(
        value,
        &["observed", "utility", "safety_effect", "costs", "rollback"],
        &[],
    )?;
    Ok(Outcome::new((
        estimate(f.take("observed")?, artifact)?,
        estimate(f.take("utility")?, measurement)?,
        safety(f.take("safety_effect")?)?,
        list(f.take("costs")?, 16, resource)?,
        rollback(f.take("rollback")?)?,
    )))
}
pub(super) fn authority(value: C) -> DResult<Authority> {
    let mut f = Fields::new(
        value,
        &[
            "policy",
            "safety_policy",
            "capabilities",
            "models",
            "calibrations",
            "approvals",
        ],
        &["consent"],
    )?;
    Ok(Authority::new((
        version(f.take("policy")?)?,
        version(f.take("safety_policy")?)?,
        list(f.take("capabilities")?, 32, version)?,
        list(f.take("models")?, 32, version)?,
        list(f.take("calibrations")?, 32, version)?,
        list(f.take("approvals")?, 64, local)?,
        f.optional("consent", consent)?,
    )))
}
pub(super) fn dimensions(value: C) -> DResult<RecordedEvidence> {
    let mut f = Fields::new(
        value,
        &[
            "cryptographic",
            "source",
            "calibration",
            "outcome",
            "compatibility",
            "regime",
            "lifecycle",
            "confidence",
        ],
        &[],
    )?;
    Ok(RecordedEvidence::new((
        estimate(f.take("cryptographic")?, artifact)?,
        estimate(f.take("source")?, artifact)?,
        estimate(f.take("calibration")?, artifact)?,
        estimate(f.take("outcome")?, artifact)?,
        estimate(f.take("compatibility")?, artifact)?,
        estimate(f.take("regime")?, artifact)?,
        estimate(f.take("lifecycle")?, artifact)?,
        estimate(f.take("confidence")?, artifact)?,
    )))
}
pub(super) fn evidence(value: C) -> DResult<Evidence> {
    let mut f = Fields::new(
        value,
        &["maturity", "source_refs", "observation_refs", "dimensions"],
        &[],
    )?;
    Ok(Evidence::new((
        maturity(f.take("maturity")?)?,
        list(f.take("source_refs")?, 256, artifact)?,
        list(f.take("observation_refs")?, 256, artifact)?,
        dimensions(f.take("dimensions")?)?,
    )))
}
pub(super) fn lifecycle(value: C) -> DResult<Lifecycle> {
    let mut f = Fields::new(value, &["retention", "holds", "edges"], &["deletion_key"])?;
    Ok(Lifecycle::new((
        retention(f.take("retention")?)?,
        list(f.take("holds")?, 64, local)?,
        f.optional("deletion_key", object_key)?,
        list(f.take("edges")?, 64, link)?,
    )))
}
pub(super) fn provenance(value: C) -> DResult<Provenance> {
    let mut f = Fields::new(
        value,
        &["causal_parents", "consulted_parents", "records"],
        &[],
    )?;
    Ok(Provenance::new((
        list(f.take("causal_parents")?, 64, local)?,
        list(f.take("consulted_parents")?, 256, consulted)?,
        list(f.take("records")?, 256, artifact)?,
    )))
}
