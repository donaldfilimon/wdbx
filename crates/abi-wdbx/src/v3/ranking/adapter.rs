//! The production vector-index adapter: a vector source the ranking's vector
//! arm can read, and the end-to-end path that merges it into a constitutional
//! ranking.
//!
//! # What the adapter is
//!
//! A *vector source* answers one question — "which stored vectors are closest
//! to this query?" — in its own terms. The HNSW source answers with graph
//! slots and an approximate order; the exact source answers with contiguous
//! identifiers and the exact order. Neither knows what an episode digest is.
//!
//! The adapter is what binds the two together. It owns a vector index *and*
//! the substrate's [`VectorBindingIndex`], so a search can answer with the
//! episode digest each slot stands for. A slot the binding index does not know
//! is counted, never resolved: the adapter never invents an episode for a
//! vector it cannot place.
//!
//! # The binding index is single-writer
//!
//! The binding index is derived state beside the ledger, and it is written by
//! whole-file replacement. It therefore takes the same writer exclusion the
//! episode store takes: an owner file and a lock held for the index's
//! lifetime, and a second writer is refused rather than allowed to lose the
//! first writer's bindings. Every write also re-reads and merges the file on
//! disk, so a binding another writer added is never lost.
//!
//! # Admission is checked against the caller's answer
//!
//! A binding may name only a digest the scope's constitutional query
//! answered. That check is made against the *caller's* admission set: the
//! adapter trusts the caller to hand over the query's own answer, and it
//! cannot re-derive the answer without becoming a second constitutional read
//! model. What it does do is close the forging of the set: the admission set
//! is sealed — the adapter's own [`admitted_scope`] mints it from the
//! store's constitutional query — and it carries that answer's own scope.
//!
//! # Approximation is declared, not assumed
//!
//! [`VectorSource::is_approximate`] says whether the source's own search may
//! miss a nearest neighbour. The HNSW source answers `true`: its graph is
//! navigated, not scanned, and a nearest neighbour can be missed. The exact
//! source answers `false` and is the correctness oracle the approximate
//! source is checked against.
//!
//! # Determinism
//!
//! The HNSW graph's level sampling is driven by an explicit, pinned seed, and
//! the source exposes it. Two sources built with the same seed, given the
//! same insertions in the same order, hold the same graph and answer the same
//! search the same way. Nothing here reads a clock or an unseeded generator.
//!
//! # What the adapter drops
//!
//! * **The graph's approximation.** An HNSW search is a graph walk, not a
//!   scan: a true nearest neighbour may be absent from the answer. The exact
//!   source is the oracle; the HNSW source is the production path.
//! * **The binding index is separate from the ledger.** The mapping lives in
//!   its own derived sidecar, so the ledger stays the canonical record and
//!   the mapping can be rebuilt or lost without touching an episode.
//! * **A vector's payload.** The adapter holds vectors, never episode
//!   content: a binding names a digest and nothing else.
//! * **An unbound vector's episode.** A slot with no binding is counted in
//!   [`VectorArm::hits_without_binding`] and never resolved. It is never
//!   invented into the ranking.
//! * **An asset digest.** The vector arm binds *episode* digests and nothing
//!   else: it never holds an asset's bytes, an asset's chunks or an asset's
//!   digest. The offline outbox's asset profile has its own commitment rule,
//!   and that rule numbers the chunks it is given by their *position* in the
//!   slice it is handed, while the asset state holds its chunk digests in
//!   chunk-index order: a caller that hands the profile its chunks in any
//!   other order computes a different digest than the state holds. That
//!   constraint belongs to the offline slice's own API, and the vector arm
//!   never meets it.

use crate::hnsw::HnswIndex;
use crate::index::{ExactIndex, SearchResult};
use crate::v3::episode::{ConstitutionalQuery, EpisodeStore, EpisodeStoreError};
use crate::v3::ranking::merge::{MergedRanking, VectorHit, merge_vector_arm};
use crate::v3::ranking::rank_constitutional_query;
use crate::v3::ranking::vector::{AdmittedScope, VectorBindingError, VectorBindingIndex};

/// The closed failures of the vector adapter.
///
/// Every variant is a static label: no variant carries a path, a raw digest,
/// a vector identifier or any episode content.
#[derive(Debug, thiserror::Error)]
pub enum VectorError {
    /// The vector index refused the operation.
    #[error("vector_index_failed")]
    Index,
    /// The binding index refused the operation.
    #[error("vector_binding_failed")]
    Binding(#[from] VectorBindingError),
}

/// The closed failures of the end-to-end vector ranking.
#[derive(Debug, thiserror::Error)]
pub enum VectorRankingError {
    /// The episode store refused the constitutional query.
    #[error("vector_ranking_store_failed")]
    Store(#[from] EpisodeStoreError),
    /// The vector source refused the search.
    #[error("vector_ranking_source_failed")]
    Source(#[from] VectorError),
    /// The binding index refused the request.
    #[error("vector_ranking_binding_failed")]
    Binding(#[from] VectorBindingError),
}

/// The vector arm's own answer to one query.
///
/// The arm holds the hits it resolved, in its own order, and the count of the
/// hits it did not. The resolved hits reuse the ranking's own
/// [`VectorHit`]: an episode digest and the arm's own similarity. A hit with
/// no binding has no digest to carry, so it is counted beside the arm instead
/// of being given an invented one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VectorArm {
    /// The hits the arm resolved to an episode, in the arm's own order.
    pub hits: Vec<VectorHit>,
    /// How many of the arm's hits no binding resolved.
    ///
    /// These are counted, never ranked: the binding index is the substrate's
    /// own record of which slot stands for which episode, and a slot it does
    /// not know stands for nothing.
    pub hits_without_binding: usize,
}

/// A vector index the ranking's vector arm can read.
pub trait VectorSource {
    /// Search the source and answer its own hits, in its own order.
    ///
    /// # Errors
    ///
    /// Returns [`VectorError::Index`] when the index refuses the query, and
    /// [`VectorError::Binding`] when the binding index cannot be read.
    fn search(&self, query: &[f32], limit: usize) -> Result<VectorArm, VectorError>;

    /// Whether the source's own search is approximate.
    ///
    /// An approximate source may miss a nearest neighbour: its answer is a
    /// graph walk, not a scan. The exact source is never approximate, and is
    /// the correctness oracle the approximate source is checked against.
    fn is_approximate(&self) -> bool;
}

/// The HNSW-backed vector source.
///
/// The source owns the graph and the binding index that resolves a graph slot
/// to an episode digest. Its identifiers are dense and contiguous, exactly as
/// the v2 vector index's dense slots are, so the binding index's identifier
/// space never leaks an internal graph position as a public identity.
///
/// The source's own search is approximate. [`ExactVectorSource`] is the
/// deterministic oracle it is checked against.
#[derive(Debug)]
pub struct HnswVectorSource {
    index: HnswIndex,
    bindings: VectorBindingIndex,
    scope: String,
    seed: u64,
    next_id: u64,
}

impl HnswVectorSource {
    /// Create a source over a fresh graph with an explicit, pinned seed.
    ///
    /// The seed drives the graph's level sampling. It is never derived from
    /// the clock or an unseeded generator, and the source exposes it, so two
    /// runs with the same seed hold the same graph.
    ///
    /// # Errors
    ///
    /// Returns [`VectorError::Index`] when the graph refuses the dimensions.
    pub fn new(
        dimensions: usize,
        seed: u64,
        bindings: VectorBindingIndex,
        scope: impl Into<String>,
    ) -> Result<Self, VectorError> {
        let index = HnswIndex::with_seed(dimensions, seed).map_err(|_| VectorError::Index)?;
        Ok(Self {
            index,
            bindings,
            scope: scope.into(),
            seed,
            next_id: 1,
        })
    }

    /// The pinned seed of the graph's level sampling.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The scope the source answers in.
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// The binding index the source resolves through.
    #[must_use]
    pub fn bindings(&self) -> &VectorBindingIndex {
        &self.bindings
    }

    /// Consume the source and answer its binding index.
    #[must_use]
    pub fn into_bindings(self) -> VectorBindingIndex {
        self.bindings
    }

    /// How many vectors the graph holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Whether the graph holds no vector.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// The identifier the next bound insert receives.
    #[must_use]
    pub fn next_vector_id(&self) -> u64 {
        self.next_id
    }

    /// Insert one vector and bind it to one episode digest.
    ///
    /// The source assigns the identifier. The digest must be admitted in the
    /// source's scope, and the admission set must belong to that scope: a
    /// binding never crosses a scope.
    ///
    /// # Errors
    ///
    /// Returns [`VectorError::Index`] when the graph refuses the vector, and
    /// [`VectorError::Binding`] when the binding index refuses the binding.
    pub fn insert_bound(
        &mut self,
        digest: [u8; 32],
        vector: &[f32],
        admitted: &AdmittedScope,
    ) -> Result<u64, VectorError> {
        let vector_id = self.next_id;
        self.bindings
            .decide_bind(&self.scope, vector_id, digest, admitted)?;
        self.index
            .insert(vector_id, vector)
            .map_err(|_| VectorError::Index)?;
        match self.bindings.bind(&self.scope, vector_id, digest, admitted) {
            Ok(receipt) => {
                self.index.commit_last_insert(vector_id);
                self.next_id = self.next_id.saturating_add(1);
                Ok(receipt.vector_id)
            }
            Err(error) => {
                // The binding did not survive, so the graph must not keep a
                // slot that stands for nothing.
                self.index.rollback_last_insert(vector_id);
                Err(VectorError::Binding(error))
            }
        }
    }

    /// Insert one vector without binding it to an episode.
    ///
    /// A vector may be indexed before its episode is admitted. Such a hit is
    /// counted by the search and never resolved: the source never invents an
    /// episode for it.
    ///
    /// # Errors
    ///
    /// Returns [`VectorError::Index`] when the graph refuses the vector.
    pub fn insert_unbound(&mut self, vector: &[f32]) -> Result<u64, VectorError> {
        let vector_id = self.next_id;
        self.index
            .insert(vector_id, vector)
            .map_err(|_| VectorError::Index)?;
        self.index.commit_last_insert(vector_id);
        self.next_id = self.next_id.saturating_add(1);
        Ok(vector_id)
    }
}

impl VectorSource for HnswVectorSource {
    fn search(&self, query: &[f32], limit: usize) -> Result<VectorArm, VectorError> {
        let hits = self
            .index
            .search(query, limit)
            .map_err(|_| VectorError::Index)?;
        Ok(resolve_arm(&self.bindings, &self.scope, &hits))
    }

    fn is_approximate(&self) -> bool {
        true
    }
}

/// The exact, deterministic vector source.
///
/// The exact source scans every stored vector and orders by cosine
/// similarity, with ties broken by identifier. It is the correctness oracle
/// the approximate HNSW source is checked against, and it is never
/// approximate.
#[derive(Debug)]
pub struct ExactVectorSource {
    index: ExactIndex,
    bindings: VectorBindingIndex,
    scope: String,
}

impl ExactVectorSource {
    /// Create a source over an empty exact index.
    #[must_use]
    pub fn new(bindings: VectorBindingIndex, scope: impl Into<String>) -> Self {
        Self {
            index: ExactIndex::new(),
            bindings,
            scope: scope.into(),
        }
    }

    /// The scope the source answers in.
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// The binding index the source resolves through.
    #[must_use]
    pub fn bindings(&self) -> &VectorBindingIndex {
        &self.bindings
    }

    /// Consume the source and answer its binding index.
    #[must_use]
    pub fn into_bindings(self) -> VectorBindingIndex {
        self.bindings
    }

    /// How many vectors the index holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Whether the index holds no vector.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Insert one vector and bind it to one episode digest.
    ///
    /// # Errors
    ///
    /// Returns [`VectorError::Index`] when the index refuses the vector, and
    /// [`VectorError::Binding`] when the binding index refuses the binding.
    pub fn insert_bound(
        &mut self,
        digest: [u8; 32],
        vector: &[f32],
        admitted: &AdmittedScope,
    ) -> Result<u64, VectorError> {
        let vector_id = self.index.next_id();
        self.bindings
            .decide_bind(&self.scope, vector_id, digest, admitted)?;
        let assigned = self
            .index
            .put_vector(vector)
            .map_err(|_| VectorError::Index)?;
        match self.bindings.bind(&self.scope, assigned, digest, admitted) {
            Ok(receipt) => Ok(receipt.vector_id),
            Err(error) => Err(VectorError::Binding(error)),
        }
    }

    /// Insert one vector without binding it to an episode.
    ///
    /// # Errors
    ///
    /// Returns [`VectorError::Index`] when the index refuses the vector.
    pub fn insert_unbound(&mut self, vector: &[f32]) -> Result<u64, VectorError> {
        self.index
            .put_vector(vector)
            .map_err(|_| VectorError::Index)
    }
}

impl VectorSource for ExactVectorSource {
    fn search(&self, query: &[f32], limit: usize) -> Result<VectorArm, VectorError> {
        let hits = self
            .index
            .search(query, limit)
            .map_err(|_| VectorError::Index)?;
        Ok(resolve_arm(&self.bindings, &self.scope, &hits))
    }

    fn is_approximate(&self) -> bool {
        false
    }
}

/// The digests one scope's constitutional query admits.
///
/// The admission set is the canonical read model's own answer for the scope: a
/// binding may name only a digest the query answered. The set is minted by
/// the store's own query — [`EpisodeStore::admitted_scope`] — so it carries
/// the answer's own scope and cannot be presented as another scope's, and it
/// cannot be built by hand: the set is sealed.
///
/// The check is still a check against the *caller's* answer: the adapter
/// trusts the caller to hand over the query's own answer and does not
/// independently prove admission.
///
/// # Errors
///
/// Returns [`VectorRankingError::Store`] when the store refuses the query.
pub fn admitted_scope(
    store: &EpisodeStore,
    query: &ConstitutionalQuery,
) -> Result<AdmittedScope, VectorRankingError> {
    Ok(store.admitted_scope(query)?)
}

/// Rank the constitutional answer and merge a vector source's own arm into it.
///
/// This is the end-to-end path of the vector arm: the store answers the
/// constitutional query, the source answers its own search, and the two merge
/// without a number being invented. The merged ranking carries the vector
/// arm's own position and similarity beside the eight dimensions, and it
/// counts the hits no binding resolved.
///
/// # Errors
///
/// Returns [`VectorRankingError::Store`] when the store refuses the query, and
/// [`VectorRankingError::Source`] when the source refuses the search.
pub fn rank_with_vector_source(
    store: &EpisodeStore,
    query: &ConstitutionalQuery,
    source: &dyn VectorSource,
    vector: &[f32],
    limit: usize,
) -> Result<MergedRanking, VectorRankingError> {
    let ranking = rank_constitutional_query(store, query)?;
    let arm = source.search(vector, limit)?;
    let mut merged = merge_vector_arm(&ranking, &arm.hits);
    merged.vector_hits_without_binding = arm.hits_without_binding;
    Ok(merged)
}

/// Resolve a search's hits through the binding index.
///
/// A hit whose identifier names no episode in the scope is counted, never
/// resolved. A hit whose identifier is bound in *another* scope is likewise
/// counted here: the scope's answer never crosses a scope boundary.
fn resolve_arm(bindings: &VectorBindingIndex, scope: &str, hits: &[SearchResult<'_>]) -> VectorArm {
    let mut arm = VectorArm::default();
    for hit in hits {
        match bindings.digest_of(scope, hit.id) {
            Some(digest) => arm.hits.push(VectorHit {
                episode_digest: digest,
                similarity: hit.score,
            }),
            None => arm.hits_without_binding += 1,
        }
    }
    arm
}
