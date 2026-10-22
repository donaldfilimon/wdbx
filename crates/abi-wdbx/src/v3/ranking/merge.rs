//! The vector-integration seam: merging a plain vector arm into a
//! dimension-separated ranking without collapsing anything.
//!
//! ## Why this exists
//!
//! The frozen baseline harness compares a plain vector arm against the
//! constitutional query, and the plain arm has one thing the constitutional
//! arm does not: a similarity. A caller who wants both has to be able to see
//! the vector arm's own reasoning *beside* the eight dimensions, not folded
//! into them.
//!
//! ## What the merge does
//!
//! It merges the two arms' orders and keeps the vector arm's own position and
//! similarity beside every result. It does not add, average, weight or
//! otherwise combine anything: no number is invented, and the eight
//! dimensions are exactly what the constitutional query reported.
//!
//! The merge is a documented stable merge:
//!
//! * at each step the arm whose next unemitted record sits earlier *within
//!   that arm* goes first, and a tie goes to the constitutional arm. Neither
//!   arm can push the other to the end, and the two arms are blended in
//!   proportion to their own lengths;
//! * each arm's own order is preserved for the records that arm alone
//!   contributes: a merge only ever takes an arm's next record, so the
//!   constitutional arm's exclusive records keep their ledger order and the
//!   vector arm's exclusive records keep their similarity order;
//! * a record both arms hold appears once, at the turn that reaches it
//!   first. The two arms can disagree about such a record — that is the whole
//!   point of having two arms — and no single list can honour both orders at
//!   once;
//! * the vector arm's own order is never lost: every record the arm holds
//!   keeps the arm's own position in [`RankedResult::vector_rank`], so
//!   sorting the merged ranking by that position reproduces the vector arm's
//!   order exactly;
//! * a vector hit the constitutional query did not answer is *not* added to
//!   the ranking: the canonical query is the authority on which records
//!   exist. It is counted instead, in
//!   [`MergedRanking::vector_hits_without_constitutional_result`].

use crate::v3::ranking::ordering::{DimensionRanking, RankedResult};
use std::collections::{BTreeMap, BTreeSet};

/// One hit of a vector arm, in the vector arm's own order.
///
/// The slice of hits *is* the arm's order: the first hit is the arm's best
/// match. A caller maps its own vector identifiers to episode digests before
/// handing the hits over; this module never guesses that mapping. The
/// adapter in [`crate::v3::ranking::adapter`] makes that mapping through the
/// substrate's own binding index, so a hit that no binding resolves is
/// counted rather than named.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorHit {
    /// The episode the hit refers to.
    pub episode_digest: [u8; 32],
    /// The vector arm's own similarity for the hit.
    pub similarity: f32,
}

/// The merge of a constitutional answer and a vector arm's own order.
#[derive(Clone, Debug, PartialEq)]
pub struct MergedRanking {
    /// The merged order, with the vector arm's position and similarity beside
    /// every entry's per-dimension signals.
    pub ranking: DimensionRanking,
    /// How many vector hits the constitutional query did not answer.
    ///
    /// These are not added to the ranking: the canonical query is the
    /// authority on which records exist, and a vector arm's extra hit is not
    /// evidence of anything on its own.
    pub vector_hits_without_constitutional_result: usize,
    /// How many of the vector arm's hits no binding resolved to an episode.
    ///
    /// A hit whose vector identifier names no episode is counted, never
    /// invented into the ranking: the binding index is the substrate's own
    /// record of which slot stands for which episode, and a slot it does not
    /// know stands for nothing.
    ///
    /// A merge over an already-resolved arm holds none: the arm handed over
    /// carries only hits it could name an episode for.
    pub vector_hits_without_binding: usize,
}

/// Merge a vector arm into a constitutional ranking.
///
/// The merge preserves each arm's own order, exposes the vector arm's own
/// position and similarity beside the per-dimension signals, and invents no
/// number.
#[must_use]
pub fn merge_vector_arm(ranking: &DimensionRanking, arm: &[VectorHit]) -> MergedRanking {
    let constitutional: Vec<[u8; 32]> = ranking
        .entries()
        .iter()
        .map(|entry| entry.result.episode_digest)
        .collect();
    let known: BTreeSet<[u8; 32]> = constitutional.iter().copied().collect();

    // The vector arm's own positions. The first occurrence of a digest wins;
    // a hit the constitutional query did not answer is counted, not ranked.
    let mut positions: BTreeMap<[u8; 32], (usize, f32)> = BTreeMap::new();
    let mut unmatched = 0_usize;
    for (index, hit) in arm.iter().enumerate() {
        if !known.contains(&hit.episode_digest) {
            unmatched += 1;
            continue;
        }
        positions
            .entry(hit.episode_digest)
            .or_insert((index + 1, hit.similarity));
    }

    // The merged order: a stable merge of the two arms. At each step the arm
    // whose next unemitted record sits earlier within that arm goes first, and
    // a tie goes to the constitutional arm.
    let mut order: Vec<[u8; 32]> = Vec::with_capacity(constitutional.len());
    let mut seen: BTreeSet<[u8; 32]> = BTreeSet::new();
    let mut constitutional_index = 0_usize;
    let mut vector_index = 0_usize;
    loop {
        while constitutional_index < constitutional.len()
            && seen.contains(&constitutional[constitutional_index])
        {
            constitutional_index += 1;
        }
        while vector_index < arm.len() && seen.contains(&arm[vector_index].episode_digest) {
            vector_index += 1;
        }
        let constitutional_next = constitutional.get(constitutional_index);
        let vector_next = arm.get(vector_index);
        match (constitutional_next, vector_next) {
            (None, None) => break,
            (Some(digest), None) => {
                seen.insert(*digest);
                order.push(*digest);
                constitutional_index += 1;
            }
            (None, Some(hit)) => {
                if known.contains(&hit.episode_digest) {
                    seen.insert(hit.episode_digest);
                    order.push(hit.episode_digest);
                }
                vector_index += 1;
            }
            (Some(digest), Some(hit)) => {
                if constitutional_index <= vector_index {
                    seen.insert(*digest);
                    order.push(*digest);
                    constitutional_index += 1;
                } else if known.contains(&hit.episode_digest) {
                    seen.insert(hit.episode_digest);
                    order.push(hit.episode_digest);
                    vector_index += 1;
                } else {
                    vector_index += 1;
                }
            }
        }
    }

    let entries: Vec<RankedResult> = order
        .iter()
        .filter_map(|digest| {
            ranking.result(*digest).map(|entry| {
                let mut entry = entry.clone();
                if let Some((rank, similarity)) = positions.get(digest) {
                    entry.vector_rank = Some(*rank);
                    entry.vector_similarity = Some(*similarity);
                }
                entry
            })
        })
        .collect();

    MergedRanking {
        ranking: DimensionRanking::from_entries(entries, ranking.disposition_counts().clone()),
        vector_hits_without_constitutional_result: unmatched,
        vector_hits_without_binding: 0,
    }
}
