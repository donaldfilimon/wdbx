//! The answer a query returns: one result per record, and the counts.

use super::super::super::types::{EpisodeReceipt, OpenContradiction};
use super::dimensions::EvidenceDimensions;
use super::verdict::{Disposition, Explanation};
use std::collections::BTreeMap;

/// The lineage of one record, as digests and flags. It projects the edges
/// the ledger already holds and never carries a payload.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lineage {
    /// The previous episode of the same operation chain, if any.
    pub previous_digest: Option<[u8; 32]>,
    /// The candidate that supersedes this one, if any.
    pub superseded_by: Option<[u8; 32]>,
    /// The open contradictions naming this record, ordered by counterpart.
    pub open_contradictions: Vec<OpenContradiction>,
    /// The resolution edge that closed this record's quarantine or
    /// contradiction, if any.
    pub resolution_edge: Option<[u8; 32]>,
}

/// One record, projected as eight separate dimensions. There is no single
/// value here, and no field that could stand in for one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstitutionalResult {
    /// The canonical commitment of the episode.
    pub episode_digest: [u8; 32],
    /// What a caller may do with the record.
    pub disposition: Disposition,
    /// The eight dimensions, each separately inspectable.
    pub dimensions: EvidenceDimensions,
    /// The record's lineage.
    pub lineage: Lineage,
    /// Why each dimension reads as it does.
    pub explanation: Explanation,
    /// The sanitized receipt of the episode.
    pub receipt: EpisodeReceipt,
}

/// The answer to one constitutional query. The counts are per disposition;
/// there is no single value, and no field that could stand in for one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryResults {
    /// The scope the answer is for: the guild the query named.
    ///
    /// The answer carries its own scope, so a caller cannot present one
    /// guild's answer as another's.
    pub scope: String,
    /// The matching records, in ledger order.
    pub results: Vec<ConstitutionalResult>,
    /// How many results carry each disposition.
    pub disposition_counts: BTreeMap<Disposition, usize>,
}
