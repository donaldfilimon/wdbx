//! Deterministic threat and serialized-writer interleaving tests.
use super::tests::{fixture, revoke};
use super::*;
use crate::v3::commitment::CanonicalDecodeLimits;
use crate::v3::complete_episode::crypto::profile::{CryptoLimits, parse_p1};
use crate::v3::complete_episode::crypto::trust::{
    AdminAuthorization, Operation, TrustCommand, TrustState,
};
use crate::v3::complete_episode::crypto::verify::{KeySnapshot, sign_p1, verify_p1};
use crate::v3::complete_episode::crypto::verify_tests::{expected, key};
use crate::v3::complete_episode::{
    CompleteEpisode, CompleteEpisodeInput, DecodedCompleteEpisode, EpisodeId, RequestId, UtcMillis,
};

fn report_bytes(bytes: &[u8], container: &[u8], trust: &TrustSnapshot) -> VerificationReport {
    let mut configured = expected(bytes);
    configured.namespace = trust.namespace().clone();
    verify_p1(
        parse_p1(container, bytes, CryptoLimits::default()).unwrap(),
        &configured,
        &trust.verification_keys().unwrap(),
    )
}
fn again(report: &VerificationReport, trust: &TrustSnapshot) -> VerificationReport {
    report_bytes(report.payload(), report.container(), trust)
}
fn changed(
    report: &VerificationReport,
    trust: &TrustSnapshot,
    change: impl FnOnce(&mut CompleteEpisodeInput),
) -> VerificationReport {
    let (mut input, _) = DecodedCompleteEpisode::from_canonical_bytes(
        report.payload(),
        CanonicalDecodeLimits::default(),
    )
    .unwrap()
    .into_parts();
    change(&mut input);
    let bytes = CompleteEpisode::try_new(input)
        .unwrap()
        .canonical_bytes()
        .unwrap();
    let container = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    report_bytes(&bytes, &container, trust)
}
fn context_for(
    report: &VerificationReport,
    trust: &TrustSnapshot,
    writer: u64,
) -> AdmissionContext {
    let (_, _, mut context) = fixture();
    context.expected = ExpectedAdmission {
        profile: report.profile(),
        domain: report.domain(),
        version: report.version(),
        contract: report.identity().contract.digest,
        identity: report.identity().clone(),
        namespace: report.namespace().clone(),
        signer: report.signer().clone(),
        payload_digest: report.payload_digest(),
        container_digest: report.container_digest(),
    };
    context.authority.as_mut().unwrap().namespace = report.namespace().clone();
    context.authority.as_mut().unwrap().actor = report.origin_actor().clone();
    context.authority.as_mut().unwrap().binding = report.identity().binding.clone();
    context.authority.as_mut().unwrap().payload_digest = report.payload_digest();
    context.expected_trust_revision = trust.revision();
    context.expected_writer_revision = writer;
    context.current_writer_revision = writer;
    context
}
fn commit(
    simulator: &mut AdmissionSimulator,
    report: VerificationReport,
    context: &AdmissionContext,
    trust: &TrustSnapshot,
) -> Result<SimulatedOutcome, AdmissionReason> {
    let decision = evaluate_admission(&report, context, trust);
    simulator.simulate_commit(report, &decision, context, trust)
}
fn next_id(old: [u8; 16]) -> [u8; 16] {
    let mut next = old;
    next[0] ^= 1;
    next
}

#[test]
fn historical_retry_after_retire_or_revoke_uses_fresh_read_access_not_write_permission() {
    for revoked in [false, true] {
        let (mut trust, report, context) = fixture();
        let old = trust.snapshot().unwrap();
        let duplicate = again(&report, &old);
        let decision = evaluate_admission(&report, &context, &old);
        let mut sim = AdmissionSimulator::new();
        let result = sim
            .simulate_commit(report, &decision, &context, &old)
            .unwrap();
        let SimulatedOutcome::Accepted(receipt) = result else {
            panic!("fresh acceptance");
        };
        if revoked {
            revoke(&mut trust, &duplicate);
        } else {
            let auth = AdminAuthorization::for_synthetic_host_policy(
                "synthetic-local",
                "synthetic-admin",
                vec![duplicate.identity().binding.clone()],
            )
            .unwrap();
            trust
                .apply(
                    TrustCommand::new("retire", Operation::Retire(duplicate.signer().clone()))
                        .unwrap(),
                    &auth,
                    trust.revision(),
                )
                .unwrap();
        }
        let current = trust.snapshot().unwrap();
        let replay = again(&duplicate, &current);
        let mut read = context_for(&replay, &current, 1);
        read.authority.as_mut().unwrap().may_write = false;
        read.budget.max_records = 0;
        read.budget.max_retained_bytes = 0;
        let before = sim.snapshot_for_test();
        assert_eq!(
            sim.simulate_commit(replay, &decision, &read, &current)
                .unwrap(),
            SimulatedOutcome::HistoricalReplay(receipt)
        );
        assert_eq!(sim.snapshot_for_test(), before);
        read.authority.as_mut().unwrap().may_read_history = false;
        assert_eq!(
            sim.simulate_commit(again(&duplicate, &current), &decision, &read, &current),
            Err(AdmissionReason::Unauthorized)
        );
        assert_eq!(sim.snapshot_for_test(), before);
        let fresh = changed(&duplicate, &current, |input| {
            input.identity.request_id =
                RequestId::new(next_id(input.identity.request_id.0)).unwrap();
            input.identity.episode_id =
                EpisodeId::new(next_id(input.identity.episode_id.0)).unwrap();
        });
        let fresh_context = context_for(&fresh, &current, 1);
        assert!(commit(&mut sim, fresh, &fresh_context, &current).is_err());
        assert_eq!(sim.snapshot_for_test(), before);
    }
}

#[test]
fn changed_signature_payload_and_either_identity_collision_never_replay() {
    let (trust, report, context) = fixture();
    let current = trust.snapshot().unwrap();
    let duplicate = again(&report, &current);
    let mut sim = AdmissionSimulator::new();
    commit(&mut sim, report, &context, &current).unwrap();
    let before = sim.snapshot_for_test();
    for change in 0..4 {
        let candidate = if change == 0 {
            let mut container = duplicate.container().to_vec();
            *container.last_mut().unwrap() ^= 1;
            report_bytes(duplicate.payload(), &container, &current)
        } else {
            changed(&duplicate, &current, |input| match change {
                1 => {
                    input.origin.source_node =
                        crate::v3::complete_episode::NodeRef::new("changed-source").unwrap()
                }
                2 => {
                    input.identity.request_id =
                        RequestId::new(next_id(input.identity.request_id.0)).unwrap()
                }
                _ => {
                    input.identity.episode_id =
                        EpisodeId::new(next_id(input.identity.episode_id.0)).unwrap()
                }
            })
        };
        let context = context_for(&candidate, &current, 1);
        assert_eq!(
            commit(&mut sim, candidate, &context, &current),
            Err(AdmissionReason::ReplayConflict)
        );
        assert_eq!(sim.snapshot_for_test(), before);
    }
}

#[test]
fn replay_cannot_substitute_a_different_trust_namespace_for_same_signed_bytes() {
    let (trust, report, context) = fixture();
    let current = trust.snapshot().unwrap();
    let duplicate = again(&report, &current);
    let mut sim = AdmissionSimulator::new();
    commit(&mut sim, report, &context, &current).unwrap();
    let before = sim.snapshot_for_test();
    let keys = KeySnapshot::new(
        OpaqueRef::new("other-namespace").unwrap(),
        current.revision(),
        vec![key(1).verifying_key()],
    )
    .unwrap();
    let mut configured = expected(duplicate.payload());
    configured.namespace = OpaqueRef::new("other-namespace").unwrap();
    let candidate = verify_p1(
        parse_p1(
            duplicate.container(),
            duplicate.payload(),
            CryptoLimits::default(),
        )
        .unwrap(),
        &configured,
        &keys,
    );
    let context = context_for(&candidate, &current, 1);
    assert_eq!(
        commit(&mut sim, candidate, &context, &current),
        Err(AdmissionReason::VerificationBindingMismatch)
    );
    assert_eq!(sim.snapshot_for_test(), before);
}

#[test]
fn stale_epochs_signed_policy_and_missing_or_wrong_consent_refuse_without_mutation() {
    for field in 0..9 {
        let (trust, report, mut context) = fixture();
        let current = trust.snapshot().unwrap();
        match field {
            0 => context.expected_trust_revision += 1,
            1 => context.expected_policy_epoch += 1,
            2 => context.expected_consent_epoch += 1,
            3 => context.expected_writer_revision += 1,
            4 => {
                context.policy.as_mut().unwrap().policy.revision = OpaqueRef::new("stale").unwrap()
            }
            5 => {
                context.policy.as_mut().unwrap().safety_policy.revision =
                    OpaqueRef::new("stale").unwrap()
            }
            6 => {
                context.consent = Some(CurrentConsent::Required {
                    epoch: 4,
                    policy: report.authority().policy.clone(),
                })
            }
            7 => {
                context.authority.as_mut().unwrap().writer_principal =
                    OpaqueRef::new("other-writer").unwrap()
            }
            _ => context.current_writer_revision = 1,
        }
        let mut sim = AdmissionSimulator::new();
        let before = sim.snapshot_for_test();
        assert!(commit(&mut sim, report, &context, &current).is_err());
        assert_eq!(sim.snapshot_for_test(), before);
    }
}

#[test]
fn competing_write_and_changed_current_decision_state_cannot_commit_old_eligibility() {
    let (trust, report, context) = fixture();
    let current = trust.snapshot().unwrap();
    let first = again(&report, &current);
    let second = changed(&report, &current, |input| {
        input.identity.request_id = RequestId::new(next_id(input.identity.request_id.0)).unwrap();
        input.identity.episode_id = EpisodeId::new(next_id(input.identity.episode_id.0)).unwrap();
    });
    let second_context = context_for(&second, &current, 0);
    let old_decision = evaluate_admission(&second, &second_context, &current);
    let mut sim = AdmissionSimulator::new();
    commit(&mut sim, first, &context, &current).unwrap();
    let before = sim.snapshot_for_test();
    assert_eq!(
        sim.simulate_commit(second, &old_decision, &second_context, &current),
        Err(AdmissionReason::StaleWriterRevision)
    );
    assert_eq!(sim.snapshot_for_test(), before);
    let candidate = changed(&report, &current, |input| {
        input.identity.request_id = RequestId::new(next_id(input.identity.request_id.0)).unwrap();
        input.identity.episode_id = EpisodeId::new(next_id(input.identity.episode_id.0)).unwrap();
    });
    let current_context = context_for(&candidate, &current, 1);
    assert_eq!(
        sim.simulate_commit(candidate, &old_decision, &current_context, &current),
        Err(AdmissionReason::StaleWriterRevision)
    );
    assert_eq!(sim.snapshot_for_test(), before);
}

#[test]
fn invalid_or_unverifiable_math_cannot_create_or_replay_acceptance() {
    let (trust, report, context) = fixture();
    let current = trust.snapshot().unwrap();
    let empty = KeySnapshot::new(report.namespace().clone(), 0, vec![]).unwrap();
    let candidate = verify_p1(
        parse_p1(
            report.container(),
            report.payload(),
            CryptoLimits::default(),
        )
        .unwrap(),
        &expected(report.payload()),
        &empty,
    );
    assert_eq!(
        evaluate_admission(&candidate, &context, &current).class(),
        DecisionClass::Indeterminate
    );
    let mut sim = AdmissionSimulator::new();
    let before = sim.snapshot_for_test();
    assert!(commit(&mut sim, candidate, &context, &current).is_err());
    assert_eq!(sim.snapshot_for_test(), before);
    let duplicate = again(&report, &current);
    commit(&mut sim, report, &context, &current).unwrap();
    let before = sim.snapshot_for_test();
    let candidate = verify_p1(
        parse_p1(
            duplicate.container(),
            duplicate.payload(),
            CryptoLimits::default(),
        )
        .unwrap(),
        &expected(duplicate.payload()),
        &empty,
    );
    let mut context = context_for(&candidate, &current, 1);
    context.authority.as_mut().unwrap().may_write = false;
    assert_eq!(
        commit(&mut sim, candidate, &context, &current),
        Err(AdmissionReason::VerificationUnavailable)
    );
    assert_eq!(sim.snapshot_for_test(), before);
}

#[test]
fn erasure_retains_tombstones_and_both_identity_indexes_without_resurrection() {
    let (trust, report, context) = fixture();
    let current = trust.snapshot().unwrap();
    let duplicate = again(&report, &current);
    let mut sim = AdmissionSimulator::new();
    commit(&mut sim, report, &context, &current).unwrap();
    sim.simulate_host_erasure(&context.expected, 1).unwrap();
    assert_eq!(sim.revision(), 2);
    assert_eq!(sim.originals_for_test(0), None);
    let before = sim.snapshot_for_test();
    sim.simulate_host_erasure(&context.expected, 2).unwrap();
    assert_eq!(sim.snapshot_for_test(), before);
    let context = context_for(&duplicate, &current, 2);
    assert_eq!(
        commit(&mut sim, duplicate, &context, &current),
        Err(AdmissionReason::Deleted)
    );
    assert_eq!(sim.snapshot_for_test(), before);
}

#[test]
fn failed_budget_attempt_reserves_no_ids_and_caps_never_evict_history() {
    let (trust, report, mut context) = fixture();
    let current = trust.snapshot().unwrap();
    let duplicate = again(&report, &current);
    let needed = report.payload().len() + report.container().len();
    context.budget.max_retained_bytes = needed - 1;
    let mut sim = AdmissionSimulator::new();
    let before = sim.snapshot_for_test();
    assert_eq!(
        commit(&mut sim, report, &context, &current),
        Err(AdmissionReason::Quota)
    );
    assert_eq!(sim.snapshot_for_test(), before);
    context.budget.max_retained_bytes = needed;
    assert!(commit(&mut sim, duplicate, &context, &current).is_ok());
    let (_, template, _) = fixture();
    for index in 1u32..4096 {
        let candidate = changed(&template, &current, |input| {
            input.identity.request_id.0[..4].copy_from_slice(&index.to_be_bytes());
            input.identity.episode_id.0[..4].copy_from_slice(&index.to_be_bytes());
        });
        let mut next = context.clone();
        next.expected.identity = candidate.identity().clone();
        next.expected.payload_digest = candidate.payload_digest();
        next.expected.container_digest = candidate.container_digest();
        next.authority.as_mut().unwrap().payload_digest = candidate.payload_digest();
        next.expected_writer_revision = sim.revision();
        next.current_writer_revision = sim.revision();
        next.budget.max_retained_bytes = 64 * 1024 * 1024;
        assert!(commit(&mut sim, candidate, &next, &current).is_ok());
    }
    assert_eq!(sim.record_count(), 4096);
    let before = sim.snapshot_for_test();
    let (_, report, _) = fixture();
    let candidate = changed(&report, &current, |input| {
        input.identity.request_id = RequestId::new(next_id(input.identity.request_id.0)).unwrap();
        input.identity.episode_id = EpisodeId::new(next_id(input.identity.episode_id.0)).unwrap();
    });
    let mut context = context_for(&candidate, &current, sim.revision());
    context.budget.max_records = usize::MAX;
    assert_eq!(
        commit(&mut sim, candidate, &context, &current),
        Err(AdmissionReason::Quota)
    );
    assert_eq!(sim.snapshot_for_test(), before);
}

#[test]
fn checked_writer_overflow_and_backdated_compromise_claims_never_authorize() {
    let (mut trust, report, context) = fixture();
    let current = trust.snapshot().unwrap();
    let duplicate = again(&report, &current);
    let mut sim = AdmissionSimulator::new();
    sim.set_revision_for_test(u64::MAX);
    let mut context = context.clone();
    context.current_writer_revision = u64::MAX;
    context.expected_writer_revision = u64::MAX;
    let before = sim.snapshot_for_test();
    assert_eq!(
        commit(&mut sim, report, &context, &current),
        Err(AdmissionReason::RevisionOverflow)
    );
    assert_eq!(sim.snapshot_for_test(), before);
    revoke(&mut trust, &duplicate);
    let revoked = trust.snapshot().unwrap();
    let backdated = changed(&duplicate, &revoked, |input| {
        input.time.acquired_at = UtcMillis::new(0);
        input.time.received_at = UtcMillis::new(0);
    });
    let context = context_for(&backdated, &revoked, 0);
    let decision = evaluate_admission(&backdated, &context, &revoked);
    assert_eq!(decision.reason(), Some(AdmissionReason::KeyRevoked));
    assert!(decision.compromise_warning());
}

#[test]
fn signed_required_consent_matches_current_evidence_and_never_infers_exemption() {
    use crate::v3::complete_episode::{ConsentRef, Digest32, EpisodeDomain, LocalEpisodeRef};
    let (trust, report, _) = fixture();
    let current = trust.snapshot().unwrap();
    let candidate = changed(&report, &current, |input| {
        input.authority.consent = Some(ConsentRef::new((
            input.identity.binding.clone(),
            4,
            input.authority.policy.clone(),
            LocalEpisodeRef::new((
                input.identity.binding.clone(),
                EpisodeDomain::CompleteV1,
                Digest32::new([42; 32]),
            )),
        )));
    });
    let mut context = context_for(&candidate, &current, 0);
    assert_eq!(
        evaluate_admission(&candidate, &context, &current).reason(),
        Some(AdmissionReason::StaleConsentRevision)
    );
    context.consent = Some(CurrentConsent::Required {
        epoch: 4,
        policy: candidate.authority().policy.clone(),
    });
    assert_eq!(
        evaluate_admission(&candidate, &context, &current).class(),
        DecisionClass::Eligible
    );
    let mut wrong_policy = context.clone();
    if let Some(CurrentConsent::Required { policy, .. }) = &mut wrong_policy.consent {
        policy.revision = OpaqueRef::new("wrong-policy").unwrap();
    }
    assert_eq!(
        evaluate_admission(&candidate, &wrong_policy, &current).reason(),
        Some(AdmissionReason::StaleConsentRevision)
    );
    let mut stale = context.clone();
    stale.consent = Some(CurrentConsent::Required {
        epoch: 5,
        policy: candidate.authority().policy.clone(),
    });
    stale.expected_consent_epoch = 5;
    assert_eq!(
        evaluate_admission(&candidate, &stale, &current).reason(),
        Some(AdmissionReason::StaleConsentRevision)
    );
    let mut sim = AdmissionSimulator::new();
    assert!(commit(&mut sim, candidate, &context, &current).is_ok());
}

fn alternate_trust(
    report: &VerificationReport,
    allowed_binding: crate::v3::complete_episode::ScopeBinding,
) -> TrustSnapshot {
    use crate::v3::complete_episode::crypto::trust::{KeyPurpose, KeyRegistration};
    let mut state = TrustState::new("synthetic-local").unwrap();
    let auth = AdminAuthorization::for_synthetic_host_policy(
        "synthetic-local",
        "synthetic-admin",
        vec![allowed_binding.clone()],
    )
    .unwrap();
    let public = key(1).verifying_key();
    let registration = KeyRegistration::new(
        public.as_bytes(),
        report.signer().clone(),
        "canonical-writer",
        KeyPurpose::P1CanonicalWriter,
        super::super::profile::PROFILE_ID,
        vec![allowed_binding],
    )
    .unwrap();
    state
        .apply(
            TrustCommand::new(
                "different-register-id",
                Operation::Register(Box::new(registration)),
            )
            .unwrap(),
            &auth,
            0,
        )
        .unwrap();
    state
        .apply(
            TrustCommand::new(
                "different-activate-id",
                Operation::Activate(report.signer().clone()),
            )
            .unwrap(),
            &auth,
            1,
        )
        .unwrap();
    state.snapshot().unwrap()
}

#[test]
fn decision_binds_current_trust_digest_even_when_revisions_and_grants_are_equal() {
    let (trust, report, context) = fixture();
    let old = trust.snapshot().unwrap();
    let decision = evaluate_admission(&report, &context, &old);
    let current = alternate_trust(&report, report.identity().binding.clone());
    assert_eq!(current.revision(), old.revision());
    assert_ne!(current.digest(), old.digest());
    assert_eq!(
        evaluate_admission(&report, &context, &current).class(),
        DecisionClass::Eligible
    );
    let mut sim = AdmissionSimulator::new();
    let before = sim.snapshot_for_test();
    assert_eq!(
        sim.simulate_commit(report, &decision, &context, &current),
        Err(AdmissionReason::StaleTrustRevision)
    );
    assert_eq!(sim.snapshot_for_test(), before);
}

#[test]
fn current_scope_grants_cannot_be_inferred_from_mathematical_key_presence() {
    let (trust, report, context) = fixture();
    let old = trust.snapshot().unwrap();
    let mut other_scope = report.identity().binding.clone();
    other_scope.tenant = crate::v3::complete_episode::TenantRef::new("other-tenant").unwrap();
    let current = alternate_trust(&report, other_scope);
    assert_eq!(report.integrity(), Integrity::Valid);
    assert_eq!(old.revision(), current.revision());
    assert_eq!(
        evaluate_admission(&report, &context, &current).reason(),
        Some(AdmissionReason::ScopeDenied)
    );
}

#[test]
fn eligible_decision_cannot_transfer_to_other_verified_bytes_or_new_policy_epochs() {
    for field in 0..3 {
        let (trust, report, context) = fixture();
        let current = trust.snapshot().unwrap();
        let decision = evaluate_admission(&report, &context, &current);
        let candidate = if field == 0 {
            changed(&report, &current, |input| {
                input.origin.source_node =
                    crate::v3::complete_episode::NodeRef::new("another-source").unwrap()
            })
        } else {
            again(&report, &current)
        };
        let mut context = context_for(&candidate, &current, 0);
        match field {
            1 => {
                context.expected_policy_epoch = 5;
                context.policy.as_mut().unwrap().epoch = 5;
            }
            2 => {
                context.expected_consent_epoch = 5;
                context.consent = Some(CurrentConsent::NotRequired { epoch: 5 });
            }
            _ => (),
        };
        let mut sim = AdmissionSimulator::new();
        let before = sim.snapshot_for_test();
        let expected = match field {
            0 => AdmissionReason::VerificationBindingMismatch,
            1 => AdmissionReason::StalePolicyRevision,
            _ => AdmissionReason::StaleConsentRevision,
        };
        assert_eq!(
            sim.simulate_commit(candidate, &decision, &context, &current),
            Err(expected)
        );
        assert_eq!(sim.snapshot_for_test(), before);
    }
}

#[test]
fn historical_lookup_rejects_current_foreign_namespace_and_reused_host_token() {
    let (trust, report, context) = fixture();
    let current = trust.snapshot().unwrap();
    let duplicate = again(&report, &current);
    let mut sim = AdmissionSimulator::new();
    commit(&mut sim, report, &context, &current).unwrap();
    let before = sim.snapshot_for_test();
    use crate::v3::complete_episode::crypto::trust::{KeyPurpose, KeyRegistration};
    let mut foreign_state = TrustState::new("foreign-namespace").unwrap();
    let admin = AdminAuthorization::for_synthetic_host_policy(
        "foreign-namespace",
        "synthetic-admin",
        vec![duplicate.identity().binding.clone()],
    )
    .unwrap();
    let public = key(1).verifying_key();
    let registration = KeyRegistration::new(
        public.as_bytes(),
        duplicate.signer().clone(),
        "canonical-writer",
        KeyPurpose::P1CanonicalWriter,
        super::super::profile::PROFILE_ID,
        vec![duplicate.identity().binding.clone()],
    )
    .unwrap();
    foreign_state
        .apply(
            TrustCommand::new(
                "foreign-register",
                Operation::Register(Box::new(registration)),
            )
            .unwrap(),
            &admin,
            0,
        )
        .unwrap();
    foreign_state
        .apply(
            TrustCommand::new(
                "foreign-revoke",
                Operation::Revoke(
                    duplicate.signer().clone(),
                    RevocationReason::SuspectedCompromise,
                    Some(0),
                ),
            )
            .unwrap(),
            &admin,
            1,
        )
        .unwrap();
    let foreign = foreign_state.snapshot().unwrap();
    let local_context = context_for(&duplicate, &current, 1);
    assert!(!evaluate_admission(&duplicate, &local_context, &foreign).compromise_warning());
    assert_eq!(
        commit(
            &mut sim,
            again(&duplicate, &current),
            &local_context,
            &foreign
        ),
        Err(AdmissionReason::VerificationBindingMismatch)
    );
    assert_eq!(sim.snapshot_for_test(), before);
    let mut reused = local_context;
    reused.authority.as_mut().unwrap().namespace = OpaqueRef::new("foreign-namespace").unwrap();
    assert_eq!(
        commit(&mut sim, duplicate, &reused, &current),
        Err(AdmissionReason::Unauthorized)
    );
    assert_eq!(sim.snapshot_for_test(), before);
}

#[test]
fn a_one_record_host_budget_refuses_a_second_identity_and_keeps_the_first() {
    let (trust, report, mut context) = fixture();
    let current = trust.snapshot().unwrap();
    let duplicate = again(&report, &current);
    context.budget.max_records = 1;
    let mut sim = AdmissionSimulator::new();
    let first = commit(&mut sim, report, &context, &current).unwrap();
    let before = sim.snapshot_for_test();
    let next = changed(&duplicate, &current, |input| {
        input.identity.request_id = RequestId::new(next_id(input.identity.request_id.0)).unwrap();
        input.identity.episode_id = EpisodeId::new(next_id(input.identity.episode_id.0)).unwrap();
    });
    let mut next_context = context_for(&next, &current, 1);
    next_context.budget.max_records = 1;
    assert_eq!(
        commit(&mut sim, next, &next_context, &current),
        Err(AdmissionReason::Quota)
    );
    assert_eq!(sim.snapshot_for_test(), before);
    let replay_context = context_for(&duplicate, &current, 1);
    let replay = commit(&mut sim, duplicate, &replay_context, &current).unwrap();
    let SimulatedOutcome::Accepted(receipt) = first else {
        panic!("first acceptance");
    };
    assert_eq!(replay, SimulatedOutcome::HistoricalReplay(receipt));
    assert_eq!(sim.snapshot_for_test(), before);
}

#[test]
fn unavailable_originals_preserve_receipt_and_indexes_without_faking_exact_replay() {
    let (trust, report, context) = fixture();
    let current = trust.snapshot().unwrap();
    let duplicate = again(&report, &current);
    let mut sim = AdmissionSimulator::new();
    let SimulatedOutcome::Accepted(receipt) = commit(&mut sim, report, &context, &current).unwrap()
    else {
        panic!("fresh acceptance");
    };
    sim.simulate_host_originals_unavailable(&context.expected, 1)
        .unwrap();
    assert_eq!(sim.revision(), 2);
    assert_eq!(sim.record_count(), 1);
    assert_eq!(sim.originals_for_test(0), None);
    assert_eq!(sim.receipt_for_test(0), Some(&receipt));
    let before = sim.snapshot_for_test();
    sim.simulate_host_originals_unavailable(&context.expected, 2)
        .unwrap();
    assert_eq!(sim.snapshot_for_test(), before);
    let replay_context = context_for(&duplicate, &current, 2);
    assert_eq!(
        commit(&mut sim, duplicate, &replay_context, &current),
        Err(AdmissionReason::MissingOriginalBytes)
    );
    assert_eq!(sim.snapshot_for_test(), before);
    assert_eq!(sim.receipt_for_test(0), Some(&receipt));
}
