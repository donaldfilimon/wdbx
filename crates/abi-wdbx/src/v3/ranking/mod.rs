//! Dimension-separated ranking over the constitutional query.
//!
//! The canonical episode contract asks for trust to be eight dimensions and
//! never one score, and it names
//! [`crate::temporal::ScoreComponents::combined`] as the opaque collapse the
//! constitution forbids. This module is the other side of that promise: a
//! ranking that keeps the eight dimensions *separately inspectable* instead of
//! multiplying them into one number.
//!
//! ## The path
//!
//! ```text
//! store -> constitutional query -> dimension-separated ranking
//! ```
//!
//! [`rank_constitutional_query`] is that path in one call: it asks the
//! canonical read model
//! ([`crate::v3::episode::EpisodeStore::query`]) and ranks the answer. The
//! ranking owns no second constitutional implementation; every fact it reports
//! is a fact the canonical query computed.
//!
//! ## What it exposes
//!
//! * [`DimensionSignals`] — one [`DimensionSignal`] per dimension, with
//!   *missing* evidence (`Unknown`), a *measured zero* (`Absent`) and a
//!   *not applicable* dimension (`NotApplicable`) kept apart.
//! * [`DimensionRanking::order_by_dimension`] — an ordering by one chosen
//!   dimension, over labelled states rather than numbers.
//! * [`DimensionRanking::explicit_policy_order`] — the opt-in composite: an
//!   arbitrary policy, named as one, never a trust measure.
//! * [`merge_vector_arm`] — the vector-integration seam: a plain vector arm's
//!   own order and similarity, merged in beside the dimensions, with no
//!   scalar invented.
//! * [`VectorBindingIndex`] — the substrate-side binding between a vector
//!   index's identifiers and the episode digests the constitutional query
//!   answers. It is derived state beside the ledger, it is partitioned by
//!   scope, it takes a writer lock for its lifetime, and it binds only
//!   digests a scope's query admitted.
//! * [`EpisodeStore::admitted_scope`] — the one public way to have an
//!   [`AdmittedScope`]: the store answers the constitutional query itself and
//!   mints the set from that answer, so the set is sealed and cannot be
//!   forged from a hand-built answer.
//! * [`VectorSource`] and its two implementations — the production
//!   vector-index adapter: an approximate HNSW source with a pinned seed and
//!   the exact source that is its correctness oracle.
//! * [`rank_with_vector_source`] — the end-to-end path: store, constitutional
//!   query, vector source, merged ranking.
//!
//! ## What it does not do
//!
//! It does not produce a trust number, it does not hide a dimension, and it
//! does not replace the canonical query's verdict. The dispositions, the
//! lineage, the explanation and the eight canonical dimensions travel with
//! every result unchanged.

pub mod adapter;
pub mod merge;
pub mod ordering;
pub mod signals;
pub mod vector;

pub use adapter::{
    ExactVectorSource, HnswVectorSource, VectorArm, VectorError, VectorRankingError, VectorSource,
    admitted_scope, rank_with_vector_source,
};
pub use merge::{MergedRanking, VectorHit, merge_vector_arm};
pub use ordering::{DimensionRanking, ExplicitPolicy, RankedResult, SortDirection};
pub use signals::{DimensionSignal, DimensionSignals};
pub use vector::{
    AdmittedScope, BindingReceipt, VECTOR_BINDING_FILE, VECTOR_BINDING_LOCK, VectorBindingError,
    VectorBindingIndex,
};

use crate::v3::episode::{ConstitutionalQuery, EpisodeStore, EpisodeStoreError};

/// Rank one guild's constitutional answer, dimension-separated.
///
/// This is the end-to-end entry point: the store answers the constitutional
/// query, and the answer is ranked with its eight dimensions kept apart.
///
/// # Errors
///
/// Returns [`EpisodeStoreError`] when the store refuses the query.
pub fn rank_constitutional_query(
    store: &EpisodeStore,
    query: &ConstitutionalQuery,
) -> Result<DimensionRanking, EpisodeStoreError> {
    let results = store.query(query)?;
    Ok(DimensionRanking::from_results(results))
}

/// Rank one guild's constitutional answer and merge a vector arm into it.
///
/// The vector arm's own order and similarity travel beside the per-dimension
/// signals; nothing is combined.
///
/// # Errors
///
/// Returns [`EpisodeStoreError`] when the store refuses the query.
pub fn rank_with_vector_arm(
    store: &EpisodeStore,
    query: &ConstitutionalQuery,
    arm: &[VectorHit],
) -> Result<MergedRanking, EpisodeStoreError> {
    let ranking = rank_constitutional_query(store, query)?;
    Ok(merge_vector_arm(&ranking, arm))
}
