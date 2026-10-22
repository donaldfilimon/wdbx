//! Bounded reference construction.
use super::*;
pub(super) fn binding(value: C) -> DResult<ScopeBinding> {
    let mut f = Fields::new(value, &["tenant", "scope"], &[])?;
    Ok(ScopeBinding::new((
        token(f.take("tenant")?, TenantRef::new)?,
        scope(f.take("scope")?)?,
    )))
}
pub(super) fn version(value: C) -> DResult<VersionRef> {
    let mut f = Fields::new(value, &["component", "revision", "digest"], &[])?;
    Ok(VersionRef::new((
        token(f.take("component")?, ComponentRef::new)?,
        token(f.take("revision")?, OpaqueRef::new)?,
        digest(f.take("digest")?)?,
    )))
}
pub(super) fn local(value: C) -> DResult<LocalEpisodeRef> {
    let mut f = Fields::new(value, &["binding", "domain", "digest"], &[])?;
    Ok(LocalEpisodeRef::new((
        binding(f.take("binding")?)?,
        episode_domain(f.take("domain")?)?,
        digest(f.take("digest")?)?,
    )))
}
pub(super) fn artifact(value: C) -> DResult<ArtifactRef> {
    let mut f = Fields::new(
        value,
        &["binding", "digest", "byte_length", "kind", "format"],
        &[],
    )?;
    Ok(ArtifactRef::new((
        binding(f.take("binding")?)?,
        digest(f.take("digest")?)?,
        unsigned(f.take("byte_length")?)?,
        artifact_kind(f.take("kind")?)?,
        version(f.take("format")?)?,
    )))
}
pub(super) fn foreign(value: C) -> DResult<ForeignEvidenceRef> {
    let mut f = Fields::new(
        value,
        &[
            "origin_organization",
            "origin_binding",
            "domain",
            "digest",
            "import_receipt",
        ],
        &[],
    )?;
    Ok(ForeignEvidenceRef::new((
        token(f.take("origin_organization")?, OrganizationRef::new)?,
        binding(f.take("origin_binding")?)?,
        episode_domain(f.take("domain")?)?,
        digest(f.take("digest")?)?,
        local(f.take("import_receipt")?)?,
    )))
}
pub(super) fn actor(value: C) -> DResult<Actor> {
    let mut f = Fields::new(value, &["principal", "kind"], &[])?;
    Ok(Actor::new((
        token(f.take("principal")?, PrincipalRef::new)?,
        actor_kind(f.take("kind")?)?,
    )))
}
pub(super) fn resource(value: C) -> DResult<ResourceReading> {
    let mut f = Fields::new(value, &["resource", "amount"], &[])?;
    Ok(ResourceReading::new((
        resource_kind(f.take("resource")?)?,
        unsigned(f.take("amount")?)?,
    )))
}
pub(super) fn action(value: C) -> DResult<ActionDescriptor> {
    let mut f = Fields::new(
        value,
        &["capability", "action", "parameter_classes", "phase"],
        &[],
    )?;
    Ok(ActionDescriptor::new((
        version(f.take("capability")?)?,
        token(f.take("action")?, ActionRef::new)?,
        list(f.take("parameter_classes")?, 32, |v| {
            token(v, ParameterClassRef::new)
        })?,
        phase(f.take("phase")?)?,
    )))
}
pub(super) fn consent(value: C) -> DResult<ConsentRef> {
    let mut f = Fields::new(value, &["binding", "epoch", "policy", "attestation"], &[])?;
    Ok(ConsentRef::new((
        binding(f.take("binding")?)?,
        unsigned(f.take("epoch")?)?,
        version(f.take("policy")?)?,
        local(f.take("attestation")?)?,
    )))
}
pub(super) fn object_key(value: C) -> DResult<ObjectKeyRef> {
    let mut f = Fields::new(value, &["binding", "key_id"], &[])?;
    Ok(ObjectKeyRef::new((
        binding(f.take("binding")?)?,
        token(f.take("key_id")?, OpaqueRef::new)?,
    )))
}
pub(super) fn link(value: C) -> DResult<LifecycleLink> {
    let mut f = Fields::new(value, &["relation", "target", "basis"], &[])?;
    Ok(LifecycleLink::new((
        relation(f.take("relation")?)?,
        local(f.take("target")?)?,
        local(f.take("basis")?)?,
    )))
}
pub(super) fn scope(value: C) -> DResult<Scope> {
    let (tag, fields) = variant(value)?;
    Ok(match tag {
        0 => {
            let mut f = Fields::new(fields, &["guild"], &[])?;
            Scope::Guild(token(f.take("guild")?, GuildRef::new)?)
        }
        1 => {
            let mut f = Fields::new(fields, &["guild", "member"], &[])?;
            Scope::GuildMember {
                guild: token(f.take("guild")?, GuildRef::new)?,
                member: token(f.take("member")?, PrincipalRef::new)?,
            }
        }
        2 => {
            let mut f = Fields::new(fields, &["user"], &[])?;
            Scope::DirectMessage {
                user: token(f.take("user")?, PrincipalRef::new)?,
            }
        }
        3 => {
            let mut f = Fields::new(fields, &["organization"], &[])?;
            Scope::Organization(token(f.take("organization")?, OrganizationRef::new)?)
        }
        _ => return Err(E::UnknownVariant),
    })
}
pub(super) fn consulted(value: C) -> DResult<ConsultedRef> {
    let (tag, fields) = variant(value)?;
    match tag {
        0 => {
            let mut f = Fields::new(fields, &["reference"], &[])?;
            Ok(ConsultedRef::Local(local(f.take("reference")?)?))
        }
        1 => {
            let mut f = Fields::new(fields, &["reference"], &[])?;
            Ok(ConsultedRef::Foreign(foreign(f.take("reference")?)?))
        }
        _ => Err(E::UnknownVariant),
    }
}
pub(super) fn retention(value: C) -> DResult<RetentionIntent> {
    let values = array(value)?;
    let len = values.len();
    if !(1..=2).contains(&len) {
        return Err(E::InvalidShape);
    }
    let mut values = values.into_iter();
    let tag = unsigned(next(&mut values)?)?;
    let fields = values.next();
    if fields.as_ref().is_some_and(|f| !matches!(f, C::Map(_))) {
        return Err(E::InvalidShape);
    }
    if tag > 4 {
        return Err(E::UnknownVariant);
    }
    if tag == 0 {
        return if len == 1 {
            Ok(RetentionIntent::Ephemeral)
        } else {
            Err(E::InvalidShape)
        };
    }
    let fields = fields.ok_or(E::InvalidShape)?;
    Ok(match tag {
        1 => {
            let mut f = Fields::new(fields, &["session", "expires_at"], &[])?;
            RetentionIntent::Session {
                session: token(f.take("session")?, OpaqueRef::new)?,
                expires_at: utc(f.take("expires_at")?)?,
            }
        }
        2 => {
            let mut f = Fields::new(fields, &["expires_at"], &[])?;
            RetentionIntent::Operational {
                expires_at: utc(f.take("expires_at")?)?,
            }
        }
        3 => {
            let mut f = Fields::new(fields, &["privacy_basis"], &[])?;
            RetentionIntent::Durable {
                privacy_basis: local(f.take("privacy_basis")?)?,
            }
        }
        4 => {
            let mut f = Fields::new(fields, &["obligation"], &[])?;
            RetentionIntent::MandatoryIncident {
                obligation: local(f.take("obligation")?)?,
            }
        }
        _ => return Err(E::UnknownVariant),
    })
}
pub(super) fn measurement(value: C) -> DResult<Measurement> {
    let mut f = Fields::new(value, &["unit", "value"], &[])?;
    Ok(Measurement::new(
        measurement_unit(f.take("unit")?)?,
        SignedMicros::new(signed(f.take("value")?)?),
    )?)
}
pub(super) fn monotonic(value: C) -> DResult<MonotonicRange> {
    let mut f = Fields::new(value, &["clock", "start", "end"], &[])?;
    Ok(MonotonicRange::new((
        token(f.take("clock")?, ClockRef::new)?,
        MonoNanos::new(unsigned(f.take("start")?)?),
        MonoNanos::new(unsigned(f.take("end")?)?),
    ))?)
}
pub(super) fn posterior(value: C) -> DResult<RegimeDistribution> {
    let values = array(value)?;
    if !(1..=32).contains(&values.len()) {
        return Err(CompleteEpisodeError::InvalidPosterior.into());
    }
    let entries = list(C::Array(values), 32, |v| {
        let values = array(v)?;
        if values.len() != 2 {
            return Err(E::InvalidShape);
        }
        let mut values = values.into_iter();
        Ok((
            token(next(&mut values)?, RegimeRef::new)?,
            ppm(next(&mut values)?)?,
        ))
    })?;
    Ok(RegimeDistribution::new(entries)?)
}
pub(super) fn signer(value: C) -> DResult<crate::v3::episode::SignerKeyId> {
    let value = text(value)?;
    // The constructor checks its fixed spelling before allocating its owned string.
    crate::v3::episode::SignerKeyId::parse(&value)
        .ok_or(CompleteEpisodeError::InvalidIdentifier.into())
}
