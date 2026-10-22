//! Content-free supersession lineage reads for admitted memory candidates.
//!
//! The lineage is a projection of the `supersedes` edges already committed in
//! the ledger. It invents no edge, hides no record, and carries no payload:
//! only digests and flags. Both maps are rebuilt by `apply_record`, so the
//! live write path and the replay-on-open path answer identically.

use super::{EpisodeStore, EpisodeStoreError, bounded_identifier};

/// Supersession lineage of one memory candidate, without any content.
///
/// The two directions of a supersession edge are reported separately:
/// `superseded_by` names the candidate that replaced this one, and
/// `supersedes` names the candidate this one replaced. `superseded` is the
/// boolean form of the latter: it is true exactly when this candidate
/// supersedes another candidate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SupersessionLineage {
    /// Digest of the candidate that supersedes this one, if any.
    pub superseded_by: Option<[u8; 32]>,
    /// True when this candidate supersedes another candidate.
    pub superseded: bool,
    /// Digest of the candidate this one supersedes, if any.
    pub supersedes: Option<[u8; 32]>,
    /// True when a later `forgets` candidate erased this one.
    pub forgotten: bool,
    /// True when this candidate is itself a zero-payload forgetting tombstone.
    pub tombstoned: bool,
}

impl EpisodeStore {
    /// Report the candidate that supersedes one memory candidate.
    ///
    /// `Ok(None)` means no supersession edge names that digest in this guild:
    /// the digest was never admitted, was never superseded, or belongs to
    /// another guild. Forgetting a candidate does not erase its supersession
    /// edge, so a forgotten candidate still reports its superseder.
    ///
    /// When two corrections supersede the same target, the most recent one is
    /// reported: the forward direction of a supersession edge is a projection,
    /// while the ledger keeps both edges.
    pub fn superseded_by(
        &self,
        guild_ref: &str,
        candidate_digest: &[u8; 32],
    ) -> Result<Option<[u8; 32]>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        Ok(self
            .state
            .memories
            .get(guild_ref)
            .and_then(|memories| memories.superseded_by.get(candidate_digest).copied()))
    }

    /// Report the supersession lineage of one memory candidate.
    ///
    /// A digest that was never admitted in this guild yields an empty lineage:
    /// no supersession, not forgotten, not tombstoned. A digest admitted in
    /// another guild is likewise invisible here.
    ///
    /// The lineage never hides a record. A forgotten candidate still reports
    /// the candidate that superseded it, and a tombstone still reports the
    /// candidate it erased.
    pub fn supersession_lineage(
        &self,
        guild_ref: &str,
        candidate_digest: &[u8; 32],
    ) -> Result<SupersessionLineage, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        let Some(memories) = self.state.memories.get(guild_ref) else {
            return Ok(SupersessionLineage::default());
        };
        let supersedes = memories.supersedes.get(candidate_digest).copied();
        Ok(SupersessionLineage {
            superseded_by: memories.superseded_by.get(candidate_digest).copied(),
            superseded: supersedes.is_some(),
            supersedes,
            forgotten: memories.forgotten.contains(candidate_digest),
            tombstoned: memories.tombstones.contains(candidate_digest),
        })
    }
}
