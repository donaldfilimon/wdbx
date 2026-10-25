//! Private synthetic in-memory admission simulator. Host fixtures do not
//! authenticate credentials or load policy. No disk, durable ordering, trusted
//! clock, custody, real erasure or production append capability is implemented.
use super::super::{Actor, Digest32, Identity, OpaqueRef, ScopeBinding, VersionRef};
use super::trust::{KeyStatus, RevocationReason, TrustSnapshot};
use super::verify::{Integrity, VerificationReport};
use crate::v3::episode::SignerKeyId;
mod evaluate;
mod simulator;
pub(super) use evaluate::evaluate_admission;
pub(super) use simulator::AdmissionSimulator;
#[cfg(test)]
mod scenarios;
#[cfg(test)]
mod tests;
const RECORD_CAP: usize = 4096;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct ExpectedAdmission {
    pub profile: &'static str,
    pub domain: &'static str,
    pub version: u64,
    pub contract: Digest32,
    pub identity: Identity,
    pub namespace: OpaqueRef,
    pub signer: SignerKeyId,
    pub payload_digest: [u8; 32],
    pub container_digest: [u8; 32],
}
redacted!(ExpectedAdmission);
/// Explicit synthetic host policy input, never decoded from a payload.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct SyntheticHostAuthority {
    pub namespace: OpaqueRef,
    pub actor: Actor,
    pub binding: ScopeBinding,
    pub payload_digest: [u8; 32],
    pub writer_principal: OpaqueRef,
    pub may_write: bool,
    pub may_read_history: bool,
}
redacted!(SyntheticHostAuthority);
impl SyntheticHostAuthority {
    pub(super) fn for_synthetic_host_policy(
        namespace: OpaqueRef,
        writer_principal: OpaqueRef,
        actor: Actor,
        binding: ScopeBinding,
        payload_digest: [u8; 32],
        rights: (bool, bool),
    ) -> Self {
        Self {
            namespace,
            writer_principal,
            actor,
            binding,
            payload_digest,
            may_write: rights.0,
            may_read_history: rights.1,
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct CurrentPolicy {
    pub epoch: u64,
    pub policy: VersionRef,
    pub safety_policy: VersionRef,
}
redacted!(CurrentPolicy);
#[derive(Clone, PartialEq, Eq)]
pub(super) enum CurrentConsent {
    NotRequired { epoch: u64 },
    Required { epoch: u64, policy: VersionRef },
}
redacted!(CurrentConsent);
impl CurrentConsent {
    fn epoch(&self) -> u64 {
        match self {
            Self::NotRequired { epoch } | Self::Required { epoch, .. } => *epoch,
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct HostBudget {
    pub max_records: usize,
    pub max_retained_bytes: usize,
}
redacted!(HostBudget);
#[derive(Clone, PartialEq, Eq)]
pub(super) struct AdmissionContext {
    pub expected: ExpectedAdmission,
    pub expected_trust_revision: u64,
    pub expected_policy_epoch: u64,
    pub expected_consent_epoch: u64,
    pub expected_writer_revision: u64,
    pub current_writer_revision: u64,
    pub authority: Option<SyntheticHostAuthority>,
    pub policy: Option<CurrentPolicy>,
    pub consent: Option<CurrentConsent>,
    pub budget: HostBudget,
}
redacted!(AdmissionContext);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DecisionClass {
    Eligible,
    Rejected,
    Indeterminate,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum AdmissionReason {
    #[error("verification binding mismatch")]
    VerificationBindingMismatch,
    #[error("missing synthetic authority evidence")]
    MissingAuthority,
    #[error("verification evidence unavailable")]
    VerificationUnavailable,
    #[error("invalid verification report")]
    InvalidVerification,
    #[error("synthetic actor unauthorized")]
    Unauthorized,
    #[error("stale trust revision")]
    StaleTrustRevision,
    #[error("stale policy revision")]
    StalePolicyRevision,
    #[error("stale consent revision")]
    StaleConsentRevision,
    #[error("stale writer revision")]
    StaleWriterRevision,
    #[error("key is not active")]
    KeyNotActive,
    #[error("key is revoked")]
    KeyRevoked,
    #[error("scope grant denied")]
    ScopeDenied,
    #[error("admission replay conflict")]
    ReplayConflict,
    #[error("admission quota exceeded")]
    Quota,
    #[error("deleted admission")]
    Deleted,
    #[error("original bytes unavailable")]
    MissingOriginalBytes,
    #[error("admission revision exhausted")]
    RevisionOverflow,
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct AdmissionDecision {
    class: DecisionClass,
    reason: Option<AdmissionReason>,
    context: Option<AdmissionContext>,
    trust_digest: [u8; 32],
    compromise_warning: bool,
}
redacted!(AdmissionDecision);
impl AdmissionDecision {
    pub(super) fn class(&self) -> DecisionClass {
        self.class
    }
    pub(super) fn reason(&self) -> Option<AdmissionReason> {
        self.reason
    }
    /// Current metadata only; it establishes no signing or acceptance time.
    pub(super) fn compromise_warning(&self) -> bool {
        self.compromise_warning
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct SimulatedReceipt {
    revision: u64,
    expected: ExpectedAdmission,
    trust_revision: u64,
    trust_digest: [u8; 32],
    verification_revision: u64,
    verification_digest: [u8; 32],
    policy_epoch: u64,
    consent_epoch: u64,
}
redacted!(SimulatedReceipt);
impl SimulatedReceipt {
    pub(super) fn revision(&self) -> u64 {
        self.revision
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) enum SimulatedOutcome {
    Accepted(SimulatedReceipt),
    HistoricalReplay(SimulatedReceipt),
}
redacted!(SimulatedOutcome);
