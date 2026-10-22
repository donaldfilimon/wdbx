//! Manual test-only fixture construction. No production serde representation.
use super::super::*;
use crate::v3::episode::{ActorKind, EvidenceLevel, SignerKeyId};
use serde_json::Value;
type R<T> = std::result::Result<T, &'static str>;
fn text(v: &Value) -> R<&str> {
    v.as_str().ok_or("text")
}
fn number(v: &Value) -> R<u64> {
    v.as_u64().ok_or("integer")
}
fn arr(v: &Value) -> R<&Vec<Value>> {
    v.as_array().ok_or("array")
}
fn items<T>(v: &Value, f: impl Fn(&Value) -> R<T>) -> R<Vec<T>> {
    arr(v)?.iter().map(f).collect()
}
fn optional<T>(v: &Value, key: &str, f: impl Fn(&Value) -> R<T>) -> R<Option<T>> {
    v.get(key).map(f).transpose()
}
fn hex<const N: usize>(v: &Value) -> R<[u8; N]> {
    let s = text(v)?;
    if s.len() != N * 2
        || !s
            .bytes()
            .all(|x| x.is_ascii_digit() || (b'a'..=b'f').contains(&x))
    {
        return Err("hex");
    }
    let mut result = [0; N];
    for (i, b) in result.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|_| "hex")?;
    }
    Ok(result)
}
macro_rules! leaf {
    ($name:ident,$ty:ident) => {
        fn $name(v: &Value) -> R<$ty> {
            $ty::new(text(v)?).map_err(|_| "token")
        }
    };
}
leaf!(opaque, OpaqueRef);
leaf!(tenant, TenantRef);
leaf!(guild, GuildRef);
leaf!(principal, PrincipalRef);
leaf!(organization, OrganizationRef);
leaf!(component, ComponentRef);
leaf!(node, NodeRef);
leaf!(regime, RegimeRef);
leaf!(clock, ClockRef);
leaf!(action_ref, ActionRef);
leaf!(parameter, ParameterClassRef);
fn digest(v: &Value) -> R<Digest32> {
    Ok(Digest32::new(hex(v)?))
}
fn utc(v: &Value) -> R<UtcMillis> {
    Ok(UtcMillis::new(number(v)?))
}
fn ppm(v: &Value) -> R<UnitPpm> {
    UnitPpm::new(number(v)?.try_into().map_err(|_| "ppm")?).map_err(|_| "ppm")
}
fn estimate<T>(v: &Value, f: impl Fn(&Value) -> R<T>) -> R<Estimate<T>> {
    if v.get("unknown") == Some(&Value::Bool(true)) {
        Ok(Estimate::Unknown)
    } else {
        Ok(Estimate::Known(f(v.get("known").ok_or("estimate")?)?))
    }
}
macro_rules! enumeration { ($f:ident,$ty:ident,$($variant:ident),+) => { fn $f(v:&Value)->R<$ty> { match text(v)? { $(stringify!($variant)=>Ok($ty::$variant),)+ _=>Err("variant") } } }; }
enumeration!(domain, EpisodeDomain, CompleteV1, LegacyEpisodeV1);
enumeration!(
    route,
    Route,
    NoModel,
    LocalModel,
    CloudModel,
    Specialist,
    Deterministic
);
enumeration!(phase, ActionPhase, Proposed, Performed);
enumeration!(
    unit,
    MeasurementUnit,
    Unitless,
    Count,
    Milliseconds,
    Bytes,
    Microjoules
);
enumeration!(
    resource,
    ResourceKind,
    Tokens,
    CpuMicros,
    MemoryBytes,
    NetworkBytes,
    EnergyMicrojoules
);
enumeration!(
    kind,
    ArtifactKind,
    StateSummary,
    Prediction,
    Outcome,
    Observation,
    Model,
    Calibration,
    Provenance,
    ReservoirEmbedding,
    Evidence,
    Authority
);
enumeration!(
    safety,
    SafetyEffect,
    Unknown,
    NoViolationObserved,
    InterlockTriggered,
    ViolationObserved
);
enumeration!(
    rollback,
    RollbackState,
    NotApplicable,
    NotAttempted,
    Attempted,
    ExactRestoreObserved,
    Failed,
    Unknown
);
enumeration!(
    relation,
    LifecycleRelation,
    Supersedes,
    Contradicts,
    Revokes,
    Quarantines,
    Resolves,
    Redacts,
    Forgets
);
enumeration!(
    actor_kind,
    ActorKind,
    HumanSubject,
    OrganizationOwner,
    GuildOwner,
    GuildAdministrator,
    GuildManager,
    Service
);
enumeration!(maturity, EvidenceLevel, C0, C1, C2, C3, C4, C5, C6, C7);
fn binding(v: &Value) -> R<ScopeBinding> {
    let s = &v["scope"];
    let scope = match text(&s["variant"])? {
        "Guild" => Scope::Guild(guild(&s["guild"])?),
        "GuildMember" => Scope::GuildMember {
            guild: guild(&s["guild"])?,
            member: principal(&s["member"])?,
        },
        "DirectMessage" => Scope::DirectMessage {
            user: principal(&s["user"])?,
        },
        "Organization" => Scope::Organization(organization(&s["organization"])?),
        _ => return Err("scope"),
    };
    Ok(ScopeBinding::new((tenant(&v["tenant"])?, scope)))
}
fn version(v: &Value) -> R<VersionRef> {
    Ok(VersionRef::new((
        component(&v["component"])?,
        opaque(&v["revision"])?,
        digest(&v["digest"])?,
    )))
}
fn local(v: &Value) -> R<LocalEpisodeRef> {
    Ok(LocalEpisodeRef::new((
        binding(&v["binding"])?,
        domain(&v["domain"])?,
        digest(&v["digest"])?,
    )))
}
fn artifact(v: &Value) -> R<ArtifactRef> {
    Ok(ArtifactRef::new((
        binding(&v["binding"])?,
        digest(&v["digest"])?,
        number(&v["byte_length"])?,
        kind(&v["kind"])?,
        version(&v["format"])?,
    )))
}
fn consulted(v: &Value) -> R<ConsultedRef> {
    let r = &v["reference"];
    match text(&v["variant"])? {
        "Local" => Ok(ConsultedRef::Local(local(r)?)),
        "Foreign" => Ok(ConsultedRef::Foreign(ForeignEvidenceRef::new((
            organization(&r["origin_organization"])?,
            binding(&r["origin_binding"])?,
            domain(&r["domain"])?,
            digest(&r["digest"])?,
            local(&r["import_receipt"])?,
        )))),
        _ => Err("consulted"),
    }
}
fn reading(v: &Value) -> R<ResourceReading> {
    Ok(ResourceReading::new((
        resource(&v["resource"])?,
        number(&v["amount"])?,
    )))
}
fn action(v: &Value) -> R<ActionDescriptor> {
    Ok(ActionDescriptor::new((
        version(&v["capability"])?,
        action_ref(&v["action"])?,
        items(&v["parameter_classes"], parameter)?,
        phase(&v["phase"])?,
    )))
}
fn measurement(v: &Value) -> R<Measurement> {
    Measurement::new(
        unit(&v["unit"])?,
        SignedMicros::new(v["value"].as_i64().ok_or("signed")?),
    )
    .map_err(|_| "measurement")
}
fn monotonic(v: &Value) -> R<MonotonicRange> {
    MonotonicRange::new((
        clock(&v["clock"])?,
        MonoNanos::new(number(&v["start"])?),
        MonoNanos::new(number(&v["end"])?),
    ))
    .map_err(|_| "monotonic")
}
fn posterior(v: &Value) -> R<RegimeDistribution> {
    RegimeDistribution::new(items(v, |p| {
        let a = arr(p)?;
        if a.len() != 2 {
            return Err("posterior");
        }
        Ok((regime(&a[0])?, ppm(&a[1])?))
    })?)
    .map_err(|_| "posterior")
}
fn consent(v: &Value) -> R<ConsentRef> {
    Ok(ConsentRef::new((
        binding(&v["binding"])?,
        number(&v["epoch"])?,
        version(&v["policy"])?,
        local(&v["attestation"])?,
    )))
}
fn retention(v: &Value) -> R<RetentionIntent> {
    match text(&v["variant"])? {
        "Ephemeral" => Ok(RetentionIntent::Ephemeral),
        "Session" => Ok(RetentionIntent::Session {
            session: opaque(&v["session"])?,
            expires_at: utc(&v["expires_at"])?,
        }),
        "Operational" => Ok(RetentionIntent::Operational {
            expires_at: utc(&v["expires_at"])?,
        }),
        "Durable" => Ok(RetentionIntent::Durable {
            privacy_basis: local(&v["privacy_basis"])?,
        }),
        "MandatoryIncident" => Ok(RetentionIntent::MandatoryIncident {
            obligation: local(&v["obligation"])?,
        }),
        _ => Err("retention"),
    }
}
fn key(v: &Value) -> R<ObjectKeyRef> {
    Ok(ObjectKeyRef::new((
        binding(&v["binding"])?,
        opaque(&v["key_id"])?,
    )))
}
fn edge(v: &Value) -> R<LifecycleLink> {
    Ok(LifecycleLink::new((
        relation(&v["relation"])?,
        local(&v["target"])?,
        local(&v["basis"])?,
    )))
}
fn dimensions(v: &Value) -> R<RecordedEvidence> {
    Ok(RecordedEvidence::new((
        estimate(&v["cryptographic"], artifact)?,
        estimate(&v["source"], artifact)?,
        estimate(&v["calibration"], artifact)?,
        estimate(&v["outcome"], artifact)?,
        estimate(&v["compatibility"], artifact)?,
        estimate(&v["regime"], artifact)?,
        estimate(&v["lifecycle"], artifact)?,
        estimate(&v["confidence"], artifact)?,
    )))
}
pub(super) fn input(fixture: &Value) -> R<CompleteEpisodeInput> {
    super::draft_shape::validate(fixture)?;
    let identity = &fixture["identity"];
    let time = &fixture["time"];
    let origin = &fixture["origin"];
    let context = &fixture["context"];
    let decision = &fixture["decision"];
    let outcome = &fixture["outcome"];
    let authority = &fixture["authority"];
    let evidence = &fixture["evidence"];
    let lifecycle = &fixture["lifecycle"];
    let provenance = &fixture["provenance"];
    Ok(CompleteEpisodeInput::new((
        Identity::new((
            EpisodeId::new(hex(&identity["episode_id"])?).map_err(|_| "uuid")?,
            RequestId::new(hex(&identity["request_id"])?).map_err(|_| "uuid")?,
            binding(&identity["binding"])?,
            version(&identity["contract"])?,
        )),
        Time::new((
            utc(&time["acquired_at"])?,
            utc(&time["received_at"])?,
            optional(time, "monotonic", monotonic)?,
        )),
        Origin::new((
            node(&origin["source_node"])?,
            Actor::new((
                principal(&origin["actor"]["principal"])?,
                actor_kind(&origin["actor"]["kind"])?,
            )),
            SignerKeyId::parse(text(&origin["signer"])?).ok_or("signer")?,
        )),
        Context::new((
            TaskId::new(hex(&context["task_id"])?).map_err(|_| "uuid")?,
            regime(&context["regime"])?,
            estimate(&context["regime_posterior"], posterior)?,
            estimate(&context["habituation"], ppm)?,
            optional(context, "summary", artifact)?,
            items(&context["resources"], reading)?,
            optional(context, "reservoir", artifact)?,
        )),
        Decision::new((
            route(&decision["route"])?,
            estimate(&decision["prediction"], artifact)?,
            items(&decision["actions"], action)?,
            estimate(&decision["uncertainty"], ppm)?,
            estimate(&decision["risk"], ppm)?,
            estimate(&decision["novelty"], ppm)?,
        )),
        Outcome::new((
            estimate(&outcome["observed"], artifact)?,
            estimate(&outcome["utility"], measurement)?,
            safety(&outcome["safety_effect"])?,
            items(&outcome["costs"], reading)?,
            rollback(&outcome["rollback"])?,
        )),
        Authority::new((
            version(&authority["policy"])?,
            version(&authority["safety_policy"])?,
            items(&authority["capabilities"], version)?,
            items(&authority["models"], version)?,
            items(&authority["calibrations"], version)?,
            items(&authority["approvals"], local)?,
            optional(authority, "consent", consent)?,
        )),
        Evidence::new((
            maturity(&evidence["maturity"])?,
            items(&evidence["source_refs"], artifact)?,
            items(&evidence["observation_refs"], artifact)?,
            dimensions(&evidence["dimensions"])?,
        )),
        Lifecycle::new((
            retention(&lifecycle["retention"])?,
            items(&lifecycle["holds"], local)?,
            optional(lifecycle, "deletion_key", key)?,
            items(&lifecycle["edges"], edge)?,
        )),
        Provenance::new((
            items(&provenance["causal_parents"], local)?,
            items(&provenance["consulted_parents"], consulted)?,
            items(&provenance["records"], artifact)?,
        )),
    )))
}
