//! Pure, content-free admission evaluation over immutable verification/trust.
use super::*;

pub(super) fn binding_matches(report: &VerificationReport, expected: &ExpectedAdmission) -> bool {
    report.profile() == expected.profile
        && report.domain() == expected.domain
        && report.version() == expected.version
        && report.identity() == &expected.identity
        && report.identity().contract.digest == expected.contract
        && report.namespace() == &expected.namespace
        && report.signer() == &expected.signer
        && report.payload_digest() == expected.payload_digest
        && report.container_digest() == expected.container_digest
}
pub(super) fn authority_matches(
    report: &VerificationReport,
    auth: &SyntheticHostAuthority,
) -> bool {
    auth.namespace == *report.namespace()
        && auth.actor == *report.origin_actor()
        && auth.binding == report.identity().binding
        && auth.payload_digest == report.payload_digest()
}
pub(super) fn mathematical_reason(
    report: &VerificationReport,
) -> Option<(DecisionClass, AdmissionReason)> {
    match report.integrity() {
        Integrity::Valid => None,
        Integrity::Invalid => Some((
            DecisionClass::Rejected,
            AdmissionReason::InvalidVerification,
        )),
        Integrity::Unverifiable => Some((
            DecisionClass::Indeterminate,
            AdmissionReason::VerificationUnavailable,
        )),
    }
}
pub(in crate::v3::complete_episode::crypto) fn evaluate_admission(
    report: &VerificationReport,
    context: &AdmissionContext,
    trust: &TrustSnapshot,
) -> AdmissionDecision {
    let warning = trust.namespace() == report.namespace()
        && matches!(
            trust.revocation(report.signer()),
            Some((RevocationReason::SuspectedCompromise, _))
        );
    let refuse = |class, reason| AdmissionDecision {
        class,
        reason: Some(reason),
        context: None,
        trust_digest: trust.digest(),
        compromise_warning: warning,
    };
    let reject = |reason| refuse(DecisionClass::Rejected, reason);
    if !binding_matches(report, &context.expected) {
        return reject(AdmissionReason::VerificationBindingMismatch);
    }
    if let Some((class, reason)) = mathematical_reason(report) {
        return refuse(class, reason);
    }
    let (Some(auth), Some(policy), Some(consent)) =
        (&context.authority, &context.policy, &context.consent)
    else {
        return refuse(
            DecisionClass::Indeterminate,
            AdmissionReason::MissingAuthority,
        );
    };
    if !authority_matches(report, auth) || !auth.may_write {
        return reject(AdmissionReason::Unauthorized);
    }
    if trust.namespace() != report.namespace() {
        return reject(AdmissionReason::VerificationBindingMismatch);
    }
    if context.expected_trust_revision != trust.revision() {
        return reject(AdmissionReason::StaleTrustRevision);
    }
    if context.expected_policy_epoch != policy.epoch
        || policy.policy != report.authority().policy
        || policy.safety_policy != report.authority().safety_policy
    {
        return reject(AdmissionReason::StalePolicyRevision);
    }
    if context.expected_consent_epoch != consent.epoch() {
        return reject(AdmissionReason::StaleConsentRevision);
    }
    let consent_matches = match (consent, &report.authority().consent) {
        (CurrentConsent::NotRequired { .. }, None) => true,
        (CurrentConsent::Required { epoch, policy }, Some(signed)) => {
            signed.epoch == *epoch
                && signed.policy == *policy
                && signed.binding == report.identity().binding
        }
        _ => false,
    };
    if !consent_matches {
        return reject(AdmissionReason::StaleConsentRevision);
    }
    if context.expected_writer_revision != context.current_writer_revision {
        return reject(AdmissionReason::StaleWriterRevision);
    }
    match trust.status(report.signer()) {
        Some(KeyStatus::Active) => (),
        Some(KeyStatus::Revoked) => return reject(AdmissionReason::KeyRevoked),
        _ => return reject(AdmissionReason::KeyNotActive),
    }
    if !trust.permits(
        report.signer(),
        &auth.writer_principal,
        &report.identity().binding,
    ) {
        return reject(AdmissionReason::ScopeDenied);
    }
    AdmissionDecision {
        class: DecisionClass::Eligible,
        reason: None,
        context: Some(context.clone()),
        trust_digest: trust.digest(),
        compromise_warning: warning,
    }
}
