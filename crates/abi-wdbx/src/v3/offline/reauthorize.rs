//! Reauthorizing a queued operation against the *current* server policy.
//!
//! # Why this is a separate step
//!
//! An offline write is **provisional**. It was proposed under the policy,
//! the consent epoch, and the contract revision that were current when the
//! scope went offline, and it is not a canonical episode until the server
//! accepts it. Accepting it later under the old binding would grandfather a
//! stale policy, a withdrawn consent, or an obsolete contract into the
//! ledger — exactly what the global constraint forbids.
//!
//! So the outbox never assumes an old write is still acceptable. The
//! reauthorization is an explicit step the caller runs on reconnect, and its
//! verdict is per operation. There is no "drain and hope" path.
//!
//! # The same validation
//!
//! The reauthorization does not re-implement the write gate. For a proposed
//! episode write it runs the canonical store's own non-mutating decision
//! ([`EpisodeStore::decide_write_outcome`]), which runs the same validation,
//! the same commitment check, and the same quota checks as the append path.
//! The outbox adds the checks that are about the *binding* — the contract
//! revision, the consent epoch, the policy version, and the scope — because
//! those are what a reconnect can change.
//!
//! The caller must open the canonical store under the same current policy it
//! passes here. The two answers are combined conservatively: a proposal is
//! authorized only when the current policy accepts it *and* the canonical
//! store's own gate accepts it, so a mismatch can only ever refuse, never
//! admit.
//!
//! [`EpisodeStore::decide_write_outcome`]: crate::v3::episode::EpisodeStore::decide_write_outcome

use crate::v3::episode::{AdmissionReason, EpisodeStore, EpisodeWrite, StorePolicy};
use serde::{Deserialize, Serialize};

use super::types::{OperationPayload, OutboxOperation, RefusalReason};

/// The verdict of reauthorizing one operation against the current policy.
///
/// The four verdicts are distinct on purpose. A refusal by the current
/// policy, a moved consent epoch, and a contract revision that no longer
/// exists are three different problems, and a caller that has to tell them
/// apart cannot do it by matching on an error string.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReauthorizeOutcome {
    /// The current policy accepts the operation.
    Authorized,
    /// The current policy rejects the operation.
    Refused(RefusalReason),
    /// The consent epoch moved and the operation's bound epoch is stale.
    NeedsConsent,
    /// The operation was written under a different schema revision.
    SchemaSkew,
}

impl ReauthorizeOutcome {
    /// Whether the current policy accepts the operation.
    #[must_use]
    pub fn is_authorized(self) -> bool {
        matches!(self, Self::Authorized)
    }
}

/// Reauthorize one operation against the current policy and the canonical
/// store's own gate.
pub(crate) fn reauthorize(
    operation: &OutboxOperation,
    current: &StorePolicy,
    store: &EpisodeStore,
) -> ReauthorizeOutcome {
    let binding = &operation.binding;
    // A contract revision or contract digest that is no longer current is a
    // schema skew, not a policy refusal: the operation was written against a
    // shape the server no longer serves.
    if binding.contract_revision != current.contract_revision
        || binding.contract_digest != current.contract_digest
    {
        return ReauthorizeOutcome::SchemaSkew;
    }
    let Some(guild) = current.guilds.get(&binding.guild_ref) else {
        return ReauthorizeOutcome::Refused(RefusalReason::UnknownScope);
    };
    // A bound consent epoch that is no longer the current one means the
    // consent moved. The operation stays provisional until it is re-bound.
    if binding.consent_epoch.is_some() && binding.consent_epoch != guild.current_consent_epoch {
        return ReauthorizeOutcome::NeedsConsent;
    }
    if binding.policy_version != guild.policy_version {
        return ReauthorizeOutcome::Refused(RefusalReason::StalePolicyVersion);
    }
    if !guild.learning_enabled {
        return ReauthorizeOutcome::Refused(RefusalReason::LearningDisabled);
    }
    match &operation.payload {
        OperationPayload::Write(write) | OperationPayload::Delete(write) => {
            decide_write(write, store)
        }
        // An asset transfer and a configuration edit are not canonical
        // episode writes. The asset's bytes are verified by their digest,
        // and an edit is a proposal over a mergeable object.
        OperationPayload::Asset(_) | OperationPayload::Edit(_) => ReauthorizeOutcome::Authorized,
    }
}

/// The canonical store's own verdict on one proposed write.
fn decide_write(write: &EpisodeWrite, store: &EpisodeStore) -> ReauthorizeOutcome {
    let decision = store.decide_write_outcome(write);
    match decision.reason() {
        AdmissionReason::Accepted => ReauthorizeOutcome::Authorized,
        AdmissionReason::LearningDisabled => {
            ReauthorizeOutcome::Refused(RefusalReason::LearningDisabled)
        }
        AdmissionReason::StaleBinding => {
            ReauthorizeOutcome::Refused(RefusalReason::StalePolicyVersion)
        }
        AdmissionReason::TokenBudget => {
            ReauthorizeOutcome::Refused(RefusalReason::TokenBudgetExhausted)
        }
        AdmissionReason::StorageBudget => {
            ReauthorizeOutcome::Refused(RefusalReason::StorageBudgetExhausted)
        }
        AdmissionReason::Replay => ReauthorizeOutcome::Refused(RefusalReason::Replay),
        AdmissionReason::InvalidTransition => {
            ReauthorizeOutcome::Refused(RefusalReason::InvalidTransition)
        }
        AdmissionReason::Quiet => ReauthorizeOutcome::Refused(RefusalReason::QuietSuppressed),
        AdmissionReason::InvalidInput
        | AdmissionReason::CommitmentMismatch
        | AdmissionReason::Canonical => ReauthorizeOutcome::Refused(RefusalReason::InvalidProposal),
        AdmissionReason::Io | AdmissionReason::WriterBusy | AdmissionReason::Corrupt => {
            ReauthorizeOutcome::Refused(RefusalReason::StoreUnavailable)
        }
    }
}
