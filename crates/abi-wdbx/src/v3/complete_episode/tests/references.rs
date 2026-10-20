use super::*;

// Independent test inventory: changing production walk alone cannot hide a slot.
pub(super) fn artifacts(input: &mut CompleteEpisodeInput) -> Vec<&mut ArtifactRef> {
    let dims = &mut input.evidence.dimensions;
    let mut result = vec![
        input.context.summary.as_mut().unwrap(),
        input.context.reservoir.as_mut().unwrap(),
        known_mut(&mut input.decision.prediction),
        known_mut(&mut input.outcome.observed),
        known_mut(&mut dims.cryptographic),
        known_mut(&mut dims.source),
        known_mut(&mut dims.calibration),
        known_mut(&mut dims.outcome),
        known_mut(&mut dims.compatibility),
        known_mut(&mut dims.regime),
        known_mut(&mut dims.lifecycle),
        known_mut(&mut dims.confidence),
    ];
    result.extend(input.evidence.source_refs.iter_mut());
    result.extend(input.evidence.observation_refs.iter_mut());
    result.extend(input.provenance.records.iter_mut());
    result
}
pub(super) fn locals(input: &mut CompleteEpisodeInput) -> Vec<&mut LocalEpisodeRef> {
    let mut result = vec![];
    result.extend(input.authority.approvals.iter_mut());
    result.push(&mut input.authority.consent.as_mut().unwrap().attestation);
    result.extend(input.lifecycle.holds.iter_mut());
    for edge in &mut input.lifecycle.edges {
        result.push(&mut edge.target);
        result.push(&mut edge.basis);
    }
    result.extend(input.provenance.causal_parents.iter_mut());
    for reference in &mut input.provenance.consulted_parents {
        result.push(match reference {
            ConsultedRef::Local(v) => v,
            ConsultedRef::Foreign(v) => &mut v.import_receipt,
        });
    }
    match &mut input.lifecycle.retention {
        RetentionIntent::Durable { privacy_basis } => result.push(privacy_basis),
        RetentionIntent::MandatoryIncident { obligation } => result.push(obligation),
        _ => {}
    }
    result
}
pub(super) fn rebind(input: &mut CompleteEpisodeInput, binding: ScopeBinding) {
    input.identity.binding = binding.clone();
    for value in artifacts(input) {
        value.binding = binding.clone();
    }
    for value in locals(input) {
        value.binding = binding.clone();
    }
    input.authority.consent.as_mut().unwrap().binding = binding.clone();
    input.lifecycle.deletion_key.as_mut().unwrap().binding = binding;
}
fn bindings() -> Vec<ScopeBinding> {
    vec![
        binding("other-tenant"),
        ScopeBinding {
            tenant: TenantRef::new("synthetic-tenant").unwrap(),
            scope: Scope::Guild(GuildRef::new("other-guild").unwrap()),
        },
        ScopeBinding {
            tenant: TenantRef::new("synthetic-tenant").unwrap(),
            scope: Scope::GuildMember {
                guild: GuildRef::new("synthetic-guild").unwrap(),
                member: PrincipalRef::new("member").unwrap(),
            },
        },
        ScopeBinding {
            tenant: TenantRef::new("synthetic-tenant").unwrap(),
            scope: Scope::DirectMessage {
                user: PrincipalRef::new("user").unwrap(),
            },
        },
        ScopeBinding {
            tenant: TenantRef::new("synthetic-tenant").unwrap(),
            scope: Scope::Organization(OrganizationRef::new("org").unwrap()),
        },
    ]
}
#[test]
fn every_nested_scope_and_kind_is_checked() {
    for binding in bindings() {
        for slot in 0..artifacts(&mut populated()).len() {
            let mut input = populated();
            artifacts(&mut input)[slot].binding = binding.clone();
            assert_eq!(
                CompleteEpisode::try_new(input).unwrap_err(),
                CompleteEpisodeError::ScopeMismatch,
                "artifact slot {slot}"
            );
        }
        for slot in 0..locals(&mut populated()).len() {
            let mut input = populated();
            locals(&mut input)[slot].binding = binding.clone();
            assert_eq!(
                CompleteEpisode::try_new(input).unwrap_err(),
                CompleteEpisodeError::ScopeMismatch,
                "local slot {slot}"
            );
        }
        for consent in [true, false] {
            let mut input = populated();
            if consent {
                input.authority.consent.as_mut().unwrap().binding = binding.clone();
            } else {
                input.lifecycle.deletion_key.as_mut().unwrap().binding = binding.clone();
            }
            assert_eq!(
                CompleteEpisode::try_new(input).unwrap_err(),
                CompleteEpisodeError::ScopeMismatch
            );
        }
        for retention in [
            RetentionIntent::Durable {
                privacy_basis: LocalEpisodeRef {
                    binding: binding.clone(),
                    ..local(44)
                },
            },
            RetentionIntent::MandatoryIncident {
                obligation: LocalEpisodeRef {
                    binding: binding.clone(),
                    ..local(44)
                },
            },
        ] {
            let mut input = populated();
            input.lifecycle.retention = retention;
            assert_eq!(
                CompleteEpisode::try_new(input).unwrap_err(),
                CompleteEpisodeError::ScopeMismatch
            );
        }
        let mut input = populated();
        rebind(&mut input, binding);
        assert_ne!(bytes(input), bytes(populated()));
    }
    // First twelve fixed-kind slots, then two unrestricted source slots, then
    // observation and provenance. Check every closed kind against that matrix.
    let kinds = [
        ArtifactKind::StateSummary,
        ArtifactKind::Prediction,
        ArtifactKind::Outcome,
        ArtifactKind::Observation,
        ArtifactKind::Model,
        ArtifactKind::Calibration,
        ArtifactKind::Provenance,
        ArtifactKind::ReservoirEmbedding,
        ArtifactKind::Evidence,
        ArtifactKind::Authority,
    ];
    for slot in 0..16 {
        for kind in kinds {
            let mut input = populated();
            let original = artifacts(&mut input)[slot].kind;
            artifacts(&mut input)[slot].kind = kind;
            let result = CompleteEpisode::try_new(input);
            if slot == 12 || slot == 13 || kind == original {
                assert!(result.is_ok());
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    CompleteEpisodeError::InvalidArtifactKind
                );
            }
        }
    }
}

#[test]
fn member_and_dm_identity_are_part_of_every_binding() {
    for member in [true, false] {
        let make = |name| ScopeBinding {
            tenant: TenantRef::new("synthetic-tenant").unwrap(),
            scope: if member {
                Scope::GuildMember {
                    guild: GuildRef::new("synthetic-guild").unwrap(),
                    member: PrincipalRef::new(name).unwrap(),
                }
            } else {
                Scope::DirectMessage {
                    user: PrincipalRef::new(name).unwrap(),
                }
            },
        };
        let mut base = populated();
        rebind(&mut base, make("first"));
        for slot in 0..artifacts(&mut base.clone()).len() {
            let mut input = base.clone();
            artifacts(&mut input)[slot].binding = make("second");
            assert_eq!(
                CompleteEpisode::try_new(input).unwrap_err(),
                CompleteEpisodeError::ScopeMismatch
            );
        }
        for slot in 0..locals(&mut base.clone()).len() {
            let mut input = base.clone();
            locals(&mut input)[slot].binding = make("second");
            assert_eq!(
                CompleteEpisode::try_new(input).unwrap_err(),
                CompleteEpisodeError::ScopeMismatch
            );
        }
    }
}

#[test]
fn self_reference_inventory_is_complete_without_fabricating_hash_fixed_point() {
    let digest = [200; 32];
    assert!(validate::reject_self_reference(&populated(), digest).is_ok());
    for slot in 0..artifacts(&mut populated()).len() {
        let mut input = populated();
        artifacts(&mut input)[slot].digest = Digest32::new(digest);
        assert_eq!(
            validate::reject_self_reference(&input, digest).unwrap_err(),
            CompleteEpisodeError::SelfReference
        );
    }
    for slot in 0..locals(&mut populated()).len() {
        let mut input = populated();
        locals(&mut input)[slot].digest = Digest32::new(digest);
        assert_eq!(
            validate::reject_self_reference(&input, digest).unwrap_err(),
            CompleteEpisodeError::SelfReference
        );
        locals(&mut input)[slot].domain = EpisodeDomain::LegacyEpisodeV1;
        assert!(validate::reject_self_reference(&input, digest).is_ok());
    }
    for retention in [
        RetentionIntent::Durable {
            privacy_basis: local(200),
        },
        RetentionIntent::MandatoryIncident {
            obligation: local(200),
        },
    ] {
        let mut input = populated();
        input.lifecycle.retention = retention;
        assert_eq!(
            validate::reject_self_reference(&input, digest).unwrap_err(),
            CompleteEpisodeError::SelfReference
        );
    }
}

#[test]
fn foreign_origin_consistency_and_distinct_import_claims() {
    let mut input = populated();
    let ConsultedRef::Foreign(foreign) = &mut input.provenance.consulted_parents[1] else {
        unreachable!()
    };
    foreign.origin_organization = OrganizationRef::new("different-org").unwrap();
    assert_eq!(
        CompleteEpisode::try_new(input.clone()).unwrap_err(),
        CompleteEpisodeError::ScopeMismatch
    );
    let ConsultedRef::Foreign(foreign) = &mut input.provenance.consulted_parents[1] else {
        unreachable!()
    };
    foreign.origin_binding.scope = Scope::Guild(GuildRef::new("foreign-guild").unwrap());
    foreign.origin_binding.tenant = input.identity.binding.tenant.clone();
    // Neither same tenant token nor unknown organization membership is rejected.
    let mut other = foreign.clone();
    other.import_receipt = local(99);
    input
        .provenance
        .consulted_parents
        .push(ConsultedRef::Foreign(other));
    assert!(CompleteEpisode::try_new(input).is_ok());
}

#[test]
fn constructor_checks_computed_digest_and_propagates_self_reference_rejection() {
    let input = populated();
    let expected: [u8; 32] = Sha256::digest(encode_test(&input)).into();
    for reject in [false, true] {
        let (result, calls) = validate::self_reference_probe::run(expected, reject, || {
            CompleteEpisode::try_new(input.clone())
        });
        assert_eq!(calls, 1);
        if reject {
            assert_eq!(result.unwrap_err(), CompleteEpisodeError::SelfReference);
        } else {
            assert_eq!(result.unwrap().digest().unwrap(), expected);
        }
    }
    // The scoped probe is gone and cannot affect later constructor calls.
    assert!(CompleteEpisode::try_new(input).is_ok());
}
