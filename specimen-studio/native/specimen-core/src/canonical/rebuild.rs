//! Rebuilding the studio's document from the canonical ledger.
//!
//! # What a rebuild is
//!
//! The studio's snapshot is a *projection* of the canonical ledger. A rebuild
//! walks the ledger, resolves every episode's payload commitment back to the
//! studio's content, and reconstructs the document the studio would hold if
//! the ledger were the only record.
//!
//! # What a rebuild refuses to do
//!
//! It never produces a partial document. When a content blob is missing, or
//! does not hash to the commitment the episode committed to, the rebuild
//! reports the gap and returns no document at all: a document missing a
//! state is not a projection of the ledger, it is a different thing.
//!
//! # What the ledger cannot supply
//!
//! The ledger commits to a payload; it never stores one, and it records no
//! timestamp. The content bytes live in the studio's own store, and the
//! instant an erasure happened is a studio-side fact. Both are declared in
//! the loss manifest.

use super::journal::Digest;
use serde_json::Value;

/// Why a rebuild could not reproduce the document.
///
/// Every variant is a *declared* gap: the rebuild names it, and the caller
/// sees it. None of them is ever repaired by dropping the episode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RebuildGap {
    /// The ledger holds an episode the studio's journal does not know.
    ///
    /// The studio wrote the episode and did not record it, or another writer
    /// wrote it. The rebuild cannot resolve its content.
    UnindexedEpisode {
        /// The episode digest the ledger holds.
        episode: Digest,
    },
    /// The studio's journal holds an episode the ledger does not.
    ///
    /// The studio recorded an episode that was never appended, or the ledger
    /// was replaced. The rebuild cannot place it.
    MissingEpisode {
        /// The episode digest the studio recorded.
        episode: Digest,
    },
    /// The content an episode committed to is gone from the studio's store.
    MissingContent {
        /// The episode that committed to the content.
        episode: Digest,
        /// The content commitment the episode committed to.
        content: Digest,
    },
    /// The content does not hash to the commitment it is filed under.
    ContentMismatch {
        /// The episode that committed to the content.
        episode: Digest,
        /// The content commitment the episode committed to.
        content: Digest,
        /// The digest the stored bytes actually have.
        found: Digest,
    },
    /// The ledger holds no live state at all.
    ///
    /// Every episode the ledger holds was erased, so there is no document to
    /// rebuild. The rebuild says so instead of returning an empty document
    /// that would look like a fresh workspace.
    NoLiveState {
        /// How many episodes the ledger holds.
        episodes: usize,
    },
    /// The rebuilt erasure list is not the studio's own.
    ///
    /// The studio's rows and the ledger's forget candidates are two records
    /// of one erasure, and the rebuild compares them. They differ when the
    /// studio recorded an erasure the ledger never saw, or when the ledger
    /// holds an erasure the studio does not know.
    TombstoneMismatch {
        /// How many rows the rebuild produced.
        rebuilt: usize,
        /// How many rows the studio's snapshot holds.
        snapshot: usize,
    },
}

/// What a rebuild found.
#[derive(Clone, Debug, PartialEq)]
pub struct RebuildReport {
    /// The rebuilt document, when the rebuild found no gap.
    pub document: Option<Value>,
    /// The rebuilt tombstone rows, in ledger order.
    pub tombstones: Vec<Value>,
    /// How many episodes the rebuild walked.
    pub episodes: usize,
    /// The gaps the rebuild found.
    pub gaps: Vec<RebuildGap>,
}

impl RebuildReport {
    /// Whether the rebuild reproduced the document without a gap.
    pub fn is_complete(&self) -> bool {
        self.gaps.is_empty()
    }
}

/// Whether the studio's erasure list and the ledger's forget candidates agree.
///
/// The two are separate records of the same erasure: the studio's tombstone
/// rows name the node that disappeared, and the ledger's forget candidate
/// names the episode digest that was erased. The ledger holds no node and no
/// instant, so the studio's journal carries the pairing, and this report
/// checks it against the ledger's own answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ForgetAgreement {
    /// The ledger's forget candidates, in ledger order.
    pub ledger_tombstones: Vec<Digest>,
    /// The studio's recorded erasure episodes, in ledger order.
    pub studio_forgets: Vec<Digest>,
    /// How many tombstone rows the studio's snapshot holds.
    pub studio_tombstones: usize,
    /// How many tombstone rows the studio's journal claims.
    pub journal_tombstones: usize,
    /// Forget candidates the ledger holds and the studio does not know.
    pub unknown: Vec<Digest>,
    /// Erasure episodes the studio recorded and the ledger does not hold.
    pub missing: Vec<Digest>,
    /// Erasure targets the ledger does not report as forgotten.
    pub unerased: Vec<Digest>,
}

impl ForgetAgreement {
    /// Whether the two records agree.
    pub fn agrees(&self) -> bool {
        self.unknown.is_empty()
            && self.missing.is_empty()
            && self.unerased.is_empty()
            && self.ledger_tombstones == self.studio_forgets
            && self.studio_tombstones == self.journal_tombstones
    }
}
