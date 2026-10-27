//! Bounded serialized memory model, never a durable ledger or real erasure API.
use super::evaluate::{authority_matches, binding_matches, mathematical_reason};
use super::*;
use crate::v3::commitment::CanonicalValue as C;
use crate::v3::complete_episode::canonical::{Tree, Wire};
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq)]
struct Record {
    receipt: SimulatedReceipt,
    payload: Option<Vec<u8>>,
    container: Option<Vec<u8>>,
    deleted: bool,
}
redacted!(Record);
#[derive(Clone, Default, PartialEq, Eq)]
pub(in crate::v3::complete_episode::crypto) struct AdmissionSimulator {
    revision: u64,
    records: Vec<Record>,
    requests: BTreeMap<Vec<u8>, usize>,
    episodes: BTreeMap<Vec<u8>, usize>,
    retained_bytes: usize,
}
redacted!(AdmissionSimulator);
fn replay_key(expected: &ExpectedAdmission, id: &[u8; 16]) -> Result<Vec<u8>, AdmissionReason> {
    C::Array(vec![
        expected
            .identity
            .binding
            .wire::<Tree>()
            .map_err(|_| AdmissionReason::VerificationBindingMismatch)?,
        C::Text(expected.profile.to_owned()),
        C::Bytes(id.to_vec()),
    ])
    .canonical_bytes()
    .map_err(|_| AdmissionReason::VerificationBindingMismatch)
}
impl AdmissionSimulator {
    pub(in crate::v3::complete_episode::crypto) fn new() -> Self {
        Self::default()
    }
    pub(in crate::v3::complete_episode::crypto) fn revision(&self) -> u64 {
        self.revision
    }
    pub(in crate::v3::complete_episode::crypto) fn record_count(&self) -> usize {
        self.records.len()
    }
    pub(in crate::v3::complete_episode::crypto) fn simulate_commit(
        &mut self,
        report: VerificationReport,
        decision: &AdmissionDecision,
        context: &AdmissionContext,
        trust: &TrustSnapshot,
    ) -> Result<SimulatedOutcome, AdmissionReason> {
        if !binding_matches(&report, &context.expected) {
            return Err(AdmissionReason::VerificationBindingMismatch);
        }
        let auth = context
            .authority
            .as_ref()
            .ok_or(AdmissionReason::MissingAuthority)?;
        // Read access is checked before even consulting replay indexes. This
        // closed synthetic model requires read access to its idempotency scope.
        if !authority_matches(&report, auth) || !auth.may_read_history {
            return Err(AdmissionReason::Unauthorized);
        }
        if trust.namespace() != report.namespace() {
            return Err(AdmissionReason::VerificationBindingMismatch);
        }
        let request = replay_key(&context.expected, &context.expected.identity.request_id.0)?;
        let episode = replay_key(&context.expected, &context.expected.identity.episode_id.0)?;
        match (self.requests.get(&request), self.episodes.get(&episode)) {
            (Some(a), Some(b)) if a == b => {
                let record = &self.records[*a];
                // Replay indexes deliberately omit trust namespace; original
                // acceptance bindings must still match before revealing it.
                let mut bindings = context.expected.clone();
                bindings.payload_digest = record.receipt.expected.payload_digest;
                bindings.container_digest = record.receipt.expected.container_digest;
                if record.receipt.expected != bindings {
                    return Err(AdmissionReason::VerificationBindingMismatch);
                }
                if record.deleted {
                    return Err(AdmissionReason::Deleted);
                }
                let (Some(payload), Some(container)) = (&record.payload, &record.container) else {
                    return Err(AdmissionReason::MissingOriginalBytes);
                };
                if record.receipt.expected.payload_digest != report.payload_digest()
                    || record.receipt.expected.container_digest != report.container_digest()
                    || payload != report.payload()
                    || container != report.container()
                {
                    return Err(AdmissionReason::ReplayConflict);
                }
                if let Some((_, reason)) = mathematical_reason(&report) {
                    return Err(reason);
                }
                return Ok(SimulatedOutcome::HistoricalReplay(record.receipt.clone()));
            }
            (None, None) => (),
            _ => return Err(AdmissionReason::ReplayConflict),
        }
        let current = evaluate_admission(&report, context, trust);
        if let Some(reason) = current.reason {
            return Err(reason);
        }
        if context.current_writer_revision != self.revision {
            return Err(AdmissionReason::StaleWriterRevision);
        }
        if decision.class != DecisionClass::Eligible {
            return Err(decision
                .reason
                .unwrap_or(AdmissionReason::InvalidVerification));
        }
        let old = decision
            .context
            .as_ref()
            .ok_or(AdmissionReason::InvalidVerification)?;
        if old.expected != context.expected {
            return Err(AdmissionReason::VerificationBindingMismatch);
        }
        if decision.trust_digest != trust.digest()
            || old.expected_trust_revision != context.expected_trust_revision
        {
            return Err(AdmissionReason::StaleTrustRevision);
        }
        if old.policy != context.policy
            || old.expected_policy_epoch != context.expected_policy_epoch
        {
            return Err(AdmissionReason::StalePolicyRevision);
        }
        if old.consent != context.consent
            || old.expected_consent_epoch != context.expected_consent_epoch
        {
            return Err(AdmissionReason::StaleConsentRevision);
        }
        if old.expected_writer_revision != context.expected_writer_revision
            || old.current_writer_revision != context.current_writer_revision
        {
            return Err(AdmissionReason::StaleWriterRevision);
        }
        if old.authority != context.authority {
            return Err(AdmissionReason::Unauthorized);
        }
        let next = self
            .revision
            .checked_add(1)
            .ok_or(AdmissionReason::RevisionOverflow)?;
        let added = report
            .payload()
            .len()
            .checked_add(report.container().len())
            .ok_or(AdmissionReason::Quota)?;
        let bytes = self
            .retained_bytes
            .checked_add(added)
            .ok_or(AdmissionReason::Quota)?;
        if self.records.len() >= RECORD_CAP.min(context.budget.max_records)
            || bytes > context.budget.max_retained_bytes
        {
            return Err(AdmissionReason::Quota);
        }
        let receipt = SimulatedReceipt {
            revision: next,
            expected: context.expected.clone(),
            trust_revision: trust.revision(),
            trust_digest: trust.digest(),
            verification_revision: report.snapshot_revision(),
            verification_digest: report.snapshot_digest(),
            policy_epoch: context.expected_policy_epoch,
            consent_epoch: context.expected_consent_epoch,
        };
        let index = self.records.len();
        self.records.push(Record {
            receipt: receipt.clone(),
            payload: Some(report.payload().to_vec()),
            container: Some(report.container().to_vec()),
            deleted: false,
        });
        self.requests.insert(request, index);
        self.episodes.insert(episode, index);
        self.retained_bytes = bytes;
        self.revision = next;
        Ok(SimulatedOutcome::Accepted(receipt))
    }
    /// Synthetic host deletion fixture only. No real lifecycle authorization,
    /// erasure guarantee or owner privacy policy follows from this model.
    pub(in crate::v3::complete_episode::crypto) fn simulate_host_erasure(
        &mut self,
        expected: &ExpectedAdmission,
        expected_writer_revision: u64,
    ) -> Result<(), AdmissionReason> {
        self.remove_originals(expected, expected_writer_revision, true)
    }
    /// Trusted synthetic fault injection only: original bytes are unavailable,
    /// not deleted. Retained receipts cannot reconstruct or freshly verify them.
    pub(in crate::v3::complete_episode::crypto) fn simulate_host_originals_unavailable(
        &mut self,
        expected: &ExpectedAdmission,
        expected_writer_revision: u64,
    ) -> Result<(), AdmissionReason> {
        self.remove_originals(expected, expected_writer_revision, false)
    }
    fn remove_originals(
        &mut self,
        expected: &ExpectedAdmission,
        expected_writer_revision: u64,
        deleted: bool,
    ) -> Result<(), AdmissionReason> {
        if expected_writer_revision != self.revision {
            return Err(AdmissionReason::StaleWriterRevision);
        }
        let request = replay_key(expected, &expected.identity.request_id.0)?;
        let episode = replay_key(expected, &expected.identity.episode_id.0)?;
        let (Some(a), Some(b)) = (self.requests.get(&request), self.episodes.get(&episode)) else {
            return Err(AdmissionReason::MissingOriginalBytes);
        };
        if a != b || self.records[*a].receipt.expected != *expected {
            return Err(AdmissionReason::VerificationBindingMismatch);
        }
        let record = &mut self.records[*a];
        if record.deleted {
            return if deleted {
                Ok(())
            } else {
                Err(AdmissionReason::Deleted)
            };
        }
        if !deleted && record.payload.is_none() && record.container.is_none() {
            return Ok(());
        }
        let next = self
            .revision
            .checked_add(1)
            .ok_or(AdmissionReason::RevisionOverflow)?;
        let bytes = record.payload.as_ref().map_or(0, Vec::len)
            + record.container.as_ref().map_or(0, Vec::len);
        record.payload = None;
        record.container = None;
        record.deleted = deleted;
        self.retained_bytes -= bytes;
        self.revision = next;
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn snapshot_for_test(&self) -> Self {
        self.clone()
    }
    #[cfg(test)]
    pub(super) fn originals_for_test(&self, index: usize) -> Option<(&[u8], &[u8])> {
        let record = self.records.get(index)?;
        Some((record.payload.as_deref()?, record.container.as_deref()?))
    }
    #[cfg(test)]
    pub(super) fn receipt_for_test(&self, index: usize) -> Option<&SimulatedReceipt> {
        self.records.get(index).map(|record| &record.receipt)
    }
    #[cfg(test)]
    pub(super) fn set_revision_for_test(&mut self, revision: u64) {
        self.revision = revision;
    }
}
