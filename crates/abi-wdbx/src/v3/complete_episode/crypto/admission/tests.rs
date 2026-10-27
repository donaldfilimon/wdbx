//! Slice 4 private admission tests; no durable acceptance or real authority.
use super::*;
use crate::v3::complete_episode::crypto::profile::{CryptoLimits, PROFILE_ID, parse_p1};
use crate::v3::complete_episode::crypto::trust::{
    AdminAuthorization, KeyPurpose, KeyRegistration, Operation, RevocationReason, TrustCommand,
    TrustState,
};
use crate::v3::complete_episode::crypto::verify::{VerificationReport, sign_p1, verify_p1};
use crate::v3::complete_episode::crypto::verify_tests::{expected, key, payload};
use crate::v3::complete_episode::{Actor, OpaqueRef, PrincipalRef};
use crate::v3::episode::{ActorKind, SignerKeyId};

pub(super) fn fixture() -> (TrustState, VerificationReport, AdmissionContext) {
    let bytes = payload(&key(1));
    let binding = expected(&bytes).binding;
    let auth = AdminAuthorization::for_synthetic_host_policy(
        "synthetic-local",
        "synthetic-admin",
        vec![binding.clone()],
    )
    .unwrap();
    let mut trust = TrustState::new("synthetic-local").unwrap();
    let public = key(1).verifying_key();
    let id = SignerKeyId::for_key(&public);
    let registration = KeyRegistration::new(
        public.as_bytes(),
        id.clone(),
        "canonical-writer",
        KeyPurpose::P1CanonicalWriter,
        PROFILE_ID,
        vec![binding],
    )
    .unwrap();
    trust
        .apply(
            TrustCommand::new("register", Operation::Register(Box::new(registration))).unwrap(),
            &auth,
            0,
        )
        .unwrap();
    trust
        .apply(
            TrustCommand::new("activate", Operation::Activate(id)).unwrap(),
            &auth,
            1,
        )
        .unwrap();
    let snapshot = trust.snapshot().unwrap();
    let keys = snapshot.verification_keys().unwrap();
    let container = sign_p1(&bytes, &key(1), CryptoLimits::default()).unwrap();
    let report = verify_p1(
        parse_p1(&container, &bytes, CryptoLimits::default()).unwrap(),
        &expected(&bytes),
        &keys,
    );
    let context = AdmissionContext {
        expected: ExpectedAdmission {
            profile: report.profile(),
            domain: report.domain(),
            version: report.version(),
            contract: report.identity().contract.digest,
            identity: report.identity().clone(),
            namespace: report.namespace().clone(),
            signer: report.signer().clone(),
            payload_digest: report.payload_digest(),
            container_digest: report.container_digest(),
        },
        expected_trust_revision: snapshot.revision(),
        expected_policy_epoch: 3,
        expected_consent_epoch: 4,
        expected_writer_revision: 0,
        current_writer_revision: 0,
        authority: Some(SyntheticHostAuthority::for_synthetic_host_policy(
            report.namespace().clone(),
            OpaqueRef::new("canonical-writer").unwrap(),
            report.origin_actor().clone(),
            report.identity().binding.clone(),
            report.payload_digest(),
            (true, true),
        )),
        policy: Some(CurrentPolicy {
            epoch: 3,
            policy: report.authority().policy.clone(),
            safety_policy: report.authority().safety_policy.clone(),
        }),
        consent: Some(CurrentConsent::NotRequired { epoch: 4 }),
        budget: HostBudget {
            max_records: 4096,
            max_retained_bytes: 64 * 1024 * 1024,
        },
    };
    (trust, report, context)
}
pub(super) fn revoke(trust: &mut TrustState, report: &VerificationReport) {
    let auth = AdminAuthorization::for_synthetic_host_policy(
        "synthetic-local",
        "synthetic-admin",
        vec![report.identity().binding.clone()],
    )
    .unwrap();
    trust
        .apply(
            TrustCommand::new(
                "revoke",
                Operation::Revoke(
                    report.signer().clone(),
                    RevocationReason::SuspectedCompromise,
                    Some(0),
                ),
            )
            .unwrap(),
            &auth,
            trust.revision(),
        )
        .unwrap();
}

#[test]
fn pure_evaluation_never_changes_the_simulator_and_acceptance_retains_originals() {
    let (trust, report, context) = fixture();
    let current = trust.snapshot().unwrap();
    let mut simulator = AdmissionSimulator::new();
    let before = simulator.snapshot_for_test();
    let decision = evaluate_admission(&report, &context, &current);
    assert_eq!(decision.class(), DecisionClass::Eligible);
    assert_eq!(simulator.snapshot_for_test(), before);
    let bytes = report.payload().to_vec();
    let container = report.container().to_vec();
    assert!(matches!(
        simulator.simulate_commit(report, &decision, &context, &current),
        Ok(SimulatedOutcome::Accepted(_))
    ));
    assert_eq!(simulator.revision(), 1);
    assert_eq!(simulator.record_count(), 1);
    assert_eq!(
        simulator.originals_for_test(0),
        Some((bytes.as_slice(), container.as_slice()))
    );
}

#[test]
fn expected_binding_substitution_matrix_never_reserves_an_identity() {
    for field in 0..12 {
        let (trust, report, mut context) = fixture();
        let current = trust.snapshot().unwrap();
        let good_decision = evaluate_admission(&report, &context, &current);
        match field {
            0 => context.expected.payload_digest[0] ^= 1,
            1 => context.expected.container_digest[0] ^= 1,
            2 => context.expected.profile = "other-profile",
            3 => context.expected.domain = "other-domain",
            4 => context.expected.version = 2,
            5 => context.expected.contract = crate::v3::complete_episode::Digest32::new([9; 32]),
            6 => context.expected.namespace = OpaqueRef::new("other-namespace").unwrap(),
            7 => context.expected.signer = SignerKeyId::for_key(&key(2).verifying_key()),
            8 => {
                context.expected.identity.request_id =
                    crate::v3::complete_episode::RequestId::new([
                        2, 2, 3, 4, 5, 6, 0x40, 8, 0x80, 10, 11, 12, 13, 14, 15, 16,
                    ])
                    .unwrap()
            }
            9 => {
                context.expected.identity.episode_id =
                    crate::v3::complete_episode::EpisodeId::new([
                        2, 2, 3, 4, 5, 6, 0x40, 8, 0x80, 10, 11, 12, 13, 14, 15, 16,
                    ])
                    .unwrap()
            }
            10 => {
                context.expected.identity.binding.tenant =
                    crate::v3::complete_episode::TenantRef::new("other-tenant").unwrap()
            }
            _ => {
                context.expected.identity.contract.revision =
                    OpaqueRef::new("other-version").unwrap()
            }
        }
        let current = trust.snapshot().unwrap();
        let mut simulator = AdmissionSimulator::new();
        let before = simulator.snapshot_for_test();
        let decision = evaluate_admission(&report, &context, &current);
        assert_eq!(
            decision.reason(),
            Some(AdmissionReason::VerificationBindingMismatch)
        );
        assert!(
            simulator
                .simulate_commit(report, &good_decision, &context, &current)
                .is_err()
        );
        assert_eq!(simulator.snapshot_for_test(), before);
    }
}

#[test]
fn missing_current_evidence_is_indeterminate_and_actor_claims_are_not_authentication() {
    for field in 0..7 {
        let (trust, report, mut context) = fixture();
        match field {
            0 => context.authority = None,
            1 => context.policy = None,
            2 => context.consent = None,
            3 => {
                context.authority.as_mut().unwrap().actor =
                    Actor::new((PrincipalRef::new("spoofed").unwrap(), ActorKind::Service))
            }
            4 => context.authority.as_mut().unwrap().actor.kind = ActorKind::HumanSubject,
            5 => context.authority.as_mut().unwrap().payload_digest[0] ^= 1,
            _ => context.authority.as_mut().unwrap().may_write = false,
        }
        let current = trust.snapshot().unwrap();
        let mut simulator = AdmissionSimulator::new();
        let before = simulator.snapshot_for_test();
        let decision = evaluate_admission(&report, &context, &current);
        assert_eq!(
            decision.class(),
            if field < 3 {
                DecisionClass::Indeterminate
            } else {
                DecisionClass::Rejected
            }
        );
        assert!(
            simulator
                .simulate_commit(report, &decision, &context, &current)
                .is_err()
        );
        assert_eq!(simulator.snapshot_for_test(), before);
    }
}

#[test]
fn revocation_between_evaluation_and_commit_cannot_use_the_old_key_snapshot() {
    let (mut trust, report, context) = fixture();
    let old = trust.snapshot().unwrap();
    let decision = evaluate_admission(&report, &context, &old);
    assert_eq!(decision.class(), DecisionClass::Eligible);
    revoke(&mut trust, &report);
    let current = trust.snapshot().unwrap();
    let mut simulator = AdmissionSimulator::new();
    let before = simulator.snapshot_for_test();
    assert!(
        simulator
            .simulate_commit(report, &decision, &context, &current)
            .is_err()
    );
    assert_eq!(simulator.snapshot_for_test(), before);
}

#[test]
fn host_zero_budget_refuses_new_work_without_reserving_ids() {
    for byte_budget in [false, true] {
        let (trust, report, mut context) = fixture();
        if byte_budget {
            context.budget.max_retained_bytes = 0;
        } else {
            context.budget.max_records = 0;
        }
        let current = trust.snapshot().unwrap();
        let decision = evaluate_admission(&report, &context, &current);
        let mut simulator = AdmissionSimulator::new();
        let before = simulator.snapshot_for_test();
        assert_eq!(
            simulator.simulate_commit(report, &decision, &context, &current),
            Err(AdmissionReason::Quota)
        );
        assert_eq!(simulator.snapshot_for_test(), before);
    }
}
