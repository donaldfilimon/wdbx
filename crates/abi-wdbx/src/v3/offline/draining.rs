//! The drain: reauthorize, merge, submit, record.
//!
//! The steps and their rationale are in the parent module's documentation
//! ([`super::drain`]).

use std::collections::BTreeSet;

use crate::v3::episode::{EpisodeStore, StorePolicy};

use super::drain::{
    DrainCounts, DrainReport, OperationOutcome, OperationSink, Submission, SubmissionVerdict,
};
use super::error::OutboxError;
use super::merge::merge_edits;
use super::outbox::{OfflineOutbox, SubmissionGroup, build_groups, reason_for};
use super::reauthorize::reauthorize;
use super::types::{OperationId, OperationPayload, OperationReason, OperationStatus};

impl OfflineOutbox {
    /// Drain the outbox against the *current* server policy.
    ///
    /// This is the reconnect path, and it is deliberately explicit: the
    /// caller hands the current policy and the canonical store, and the
    /// drain answers a per-operation reauthorization verdict for every live
    /// operation. There is no path that submits a queued operation without
    /// reauthorizing it first.
    ///
    /// The canonical store must be the store of the same scope, opened under
    /// the same current policy. The two answers are combined
    /// conservatively: a proposal is authorized only when the current policy
    /// accepts it *and* the store's own write gate accepts it, so a
    /// mismatch can only ever refuse, never admit.
    ///
    /// An operation the current policy refuses is **not dropped**. It stays
    /// in the outbox with a terminal status and a content-free reason, and an
    /// authorized caller resolves it with [`OfflineOutbox::resolve`]. An
    /// operation that conflicts with another is preserved the same way: the
    /// conflict is recorded, and the outbox never picks a winner.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::Io`] when the outbox cannot be written.
    pub fn drain(
        &mut self,
        current: &StorePolicy,
        store: &EpisodeStore,
        sink: &mut dyn OperationSink,
    ) -> Result<DrainReport, OutboxError> {
        let candidates: Vec<usize> = self
            .operations
            .iter()
            .enumerate()
            .filter(|(_, operation)| operation.is_live())
            .map(|(index, _)| index)
            .collect();

        // 1. Reauthorize every live operation against the current policy.
        let mut outcomes: Vec<Option<OperationOutcome>> = vec![None; self.operations.len()];
        let mut authorized: Vec<usize> = Vec::new();
        for &index in &candidates {
            let verdict = reauthorize(&self.operations[index], current, store);
            outcomes[index] = Some(OperationOutcome {
                operation_id: self.operations[index].id.clone(),
                reauthorization: verdict,
                submitted: false,
                status: self.operations[index].status,
                reason: self.operations[index].reason,
            });
            if verdict.is_authorized() {
                authorized.push(index);
            } else {
                let operation = &mut self.operations[index];
                operation.status = OperationStatus::Rejected;
                operation.reason = Some(reason_for(verdict));
            }
        }

        // 2. Merge the authorized edits. A conflict is recorded, and both of
        //    its branches are preserved.
        let edits: Vec<&super::types::OutboxOperation> = authorized
            .iter()
            .filter_map(|index| self.operations.get(*index))
            .filter(|operation| matches!(operation.payload, OperationPayload::Edit(_)))
            .collect();
        let plan = merge_edits(&edits);
        let affected = self.apply_conflicts(&plan);
        for index in affected {
            if let Some(outcome) = outcomes[index].as_mut() {
                outcome.status = self.operations[index].status;
                outcome.reason = self.operations[index].reason;
            }
        }
        let branches: BTreeSet<OperationId> = plan
            .conflicts
            .iter()
            .flat_map(|conflict| conflict.operations.iter().cloned())
            .collect();
        let authorized: Vec<usize> = authorized
            .into_iter()
            .filter(|index| !branches.contains(&self.operations[*index].id))
            .collect();

        // 3. Submit, in dependency order.
        let groups = build_groups(&self.operations, &authorized, &plan);
        let mut counts = DrainCounts::default();
        let mut pending = groups;
        loop {
            if pending.is_empty() {
                break;
            }
            let mut deferred: Vec<SubmissionGroup> = Vec::new();
            let mut progressed = false;
            for group in pending {
                if !self.dependencies_satisfied(&group.members) {
                    deferred.push(group);
                    continue;
                }
                self.submit_group(&group, sink, &mut outcomes, &mut counts);
                progressed = true;
            }
            pending = deferred;
            if !progressed {
                break;
            }
        }

        // 4. A deferred group is unsent work: it stays, with a reason.
        for group in &pending {
            for member in &group.members {
                let operation = &mut self.operations[*member];
                operation.reason = Some(OperationReason::DependencyPending);
                if let Some(outcome) = outcomes[*member].as_mut() {
                    outcome.status = operation.status;
                    outcome.reason = operation.reason;
                }
            }
        }

        self.persist()?;
        let reported: Vec<OperationOutcome> = candidates
            .iter()
            .filter_map(|index| outcomes[*index].clone())
            .collect();
        Ok(DrainReport {
            outcomes: reported,
            merge: plan,
            submitted: counts.submitted,
            accepted: counts.accepted,
            refused: counts.refused,
            unavailable: counts.unavailable,
        })
    }

    /// Hand one proposal to the sink and record the verdict.
    fn submit_group(
        &mut self,
        group: &SubmissionGroup,
        sink: &mut dyn OperationSink,
        outcomes: &mut [Option<OperationOutcome>],
        counts: &mut DrainCounts,
    ) {
        let contributors: Vec<OperationId> = group
            .members
            .iter()
            .filter_map(|index| self.operations.get(*index))
            .map(|operation| operation.id.clone())
            .collect();
        let submission = Submission {
            contributors,
            payload: group.payload.clone(),
        };
        let verdict = sink.submit(&submission);
        let (status, reason) = match verdict {
            SubmissionVerdict::Accepted => (
                OperationStatus::Acknowledged,
                Some(OperationReason::Authorized),
            ),
            SubmissionVerdict::Refused(reason) => (
                OperationStatus::Rejected,
                Some(OperationReason::Refused(reason)),
            ),
            SubmissionVerdict::Unavailable => (
                OperationStatus::InFlight,
                Some(OperationReason::Unavailable),
            ),
        };
        for member in &group.members {
            let operation = &mut self.operations[*member];
            operation.status = status;
            operation.reason = reason;
            if let Some(outcome) = outcomes[*member].as_mut() {
                outcome.submitted = true;
                outcome.status = status;
                outcome.reason = reason;
            }
        }
        counts.submitted += 1;
        match verdict {
            SubmissionVerdict::Accepted => counts.accepted += 1,
            SubmissionVerdict::Refused(_) => counts.refused += 1,
            SubmissionVerdict::Unavailable => counts.unavailable += 1,
        }
    }
}
