//! The non-mutating half of the write gate: a structured admission decision
//! that runs exactly the checks [`EpisodeStore::propose_write`] runs and
//! appends nothing.

use super::super::admission::{AdmissionDecision, classify_write};
use super::super::types::EpisodeWrite;
use super::{EpisodeStore, EpisodeStoreError, serialized_line};

impl EpisodeStore {
    /// Decide whether a write would be admitted, without appending it.
    ///
    /// The decision runs the same validation, the same commitment check, and
    /// the same quota checks as [`Self::propose_write`], in the same order,
    /// so it accepts exactly the writes `propose_write` accepts and refuses
    /// exactly the writes it refuses.
    ///
    /// It appends nothing, consumes no request identifier, and leaves the
    /// ledger bytes untouched. An accepted decision therefore carries no
    /// receipt; a refused write returns the store error the append path
    /// would have returned, and [`super::super::admission::AdmissionReason`]
    /// classifies that error.
    ///
    /// A caller that wants the verdict *without* an error uses
    /// [`Self::decide_write_outcome`], which never fails.
    pub fn decide_write(
        &self,
        write: &EpisodeWrite,
    ) -> Result<AdmissionDecision, EpisodeStoreError> {
        if self.poisoned {
            return Err(EpisodeStoreError::Io);
        }
        let mut record = self.prepare_record(write)?;
        // The appended line carries the signature, so the decision measures
        // the same line the append path writes. The signature is detached
        // and never moves the commitment.
        self.sign_record(&mut record);
        if write
            .expected_commitment
            .is_some_and(|expected| expected != record.episode_digest)
        {
            return Err(EpisodeStoreError::CommitmentMismatch);
        }
        let line = serialized_line(&record)?;
        let line_bytes = u64::try_from(line.len()).map_err(|_| EpisodeStoreError::InvalidInput)?;
        self.check_quotas(&record, line_bytes)?;
        Ok(AdmissionDecision::accepted(classify_write(write)))
    }

    /// The admission outcome of one write, in one content-free value.
    ///
    /// This never fails. The answer names the class the write would be
    /// admitted as and the reason it was admitted or refused, so a caller
    /// inspects one value instead of matching on an error string. A refused
    /// write carries the reason its store error maps to
    /// ([`super::super::admission::AdmissionReason`]); an accepted one
    /// carries [`super::super::admission::AdmissionReason::Accepted`].
    ///
    /// It agrees with [`Self::decide_write`] on every outcome: the same
    /// checks run, in the same order, and the refusal reason is the one the
    /// store error maps to.
    ///
    /// Like the decision it is built on, it appends nothing, consumes no
    /// request identifier, and leaves the ledger bytes untouched.
    #[must_use]
    pub fn decide_write_outcome(&self, write: &EpisodeWrite) -> AdmissionDecision {
        let class = classify_write(write);
        match self.decide_write(write) {
            Ok(decision) => decision,
            Err(error) => AdmissionDecision::refused(class, &error),
        }
    }
}
