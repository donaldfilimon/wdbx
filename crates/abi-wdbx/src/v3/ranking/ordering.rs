//! The ranking itself: the constitutional answer, ordered, with the eight
//! dimensions separately inspectable.
//!
//! ## What the ranking is
//!
//! A [`DimensionRanking`] is the constitutional query's answer, in an order,
//! with each result carrying one [`DimensionSignal`] per dimension. It never
//! combines the eight into one value: there is no score, no trust number and
//! no weight anywhere in this module's public surface.
//!
//! ## The orders
//!
//! * **The default order is the constitutional query's own order.** Nothing is
//!   reordered until a caller asks, and no dimension is privileged. The
//!   canonical read model answers in ledger order, so the default order is
//!   reproducible for the same ledger.
//! * **`order_by_dimension` orders by one chosen dimension.** The dimension's
//!   ordering is the documented one in
//!   [`signals`](super::signals): a labelled sequence of states, never a
//!   number. Ties keep the order the ranking already had, so sorting twice by
//!   two dimensions is the lexicographic composition of the two.
//! * **`explicit_policy_order` is the opt-in composite.** It is an arbitrary
//!   policy, named as one, and it is never the default. It does not collapse
//!   the dimensions either: it applies a *lexicographic sequence of named
//!   per-dimension orderings*, so every dimension's contribution stays
//!   separately inspectable on every result.
//!
//! ## What it is not
//!
//! The ranking is not a trust measure and does not claim to be one. It is a
//! presentation of evidence the canonical query already computed, under an
//! ordering the caller chooses and can read.

use crate::v3::episode::{ConstitutionalResult, Disposition, EvidenceDimension, MemoryResults};
use crate::v3::ranking::signals::{DimensionSignal, DimensionSignals, rank_of};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// One ranked record.
///
/// The record's canonical evidence travels with it unchanged, and the eight
/// per-dimension signals travel beside it. Nothing here is a verdict and
/// nothing here is a number standing in for the eight dimensions.
#[derive(Clone, Debug, PartialEq)]
pub struct RankedResult {
    /// The constitutional result this entry ranks.
    pub result: ConstitutionalResult,
    /// The record's position in the vector arm's own order: 1 for its first
    /// hit.
    ///
    /// This is the *vector arm's* ordinal position, preserved verbatim. It is
    /// not a trust measure and it is not a score: the vector arm's ordering is
    /// similarity, and similarity never implies applicability. The field is
    /// `None` when the record is not in the vector arm at all.
    pub vector_rank: Option<usize>,
    /// The vector arm's own similarity for the record.
    ///
    /// This is the only floating-point value the ranking exposes. It is the
    /// number the vector arm itself reported, carried through unchanged so a
    /// caller can see the arm's own reasoning beside the dimensions.
    pub vector_similarity: Option<f32>,
    /// The eight per-dimension signals.
    pub dimension_signals: DimensionSignals,
}

/// The direction of a per-dimension ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    /// The dimension's strongest value comes first.
    BestFirst,
    /// The dimension's weakest value comes first.
    WorstFirst,
}

/// An explicitly named ordering policy.
///
/// Every variant is an **arbitrary policy choice**, stated as one. The
/// constitution forbids collapsing the eight dimensions into a single number;
/// it does not say a caller may not choose which dimension to look at first.
/// This type is the place where that choice is made visible instead of being
/// hidden inside a formula.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplicitPolicy {
    /// Cryptographic evidence first, then outcome, compatibility, lifecycle,
    /// source and confidence.
    EvidenceFirst,
    /// Lifecycle state first, then cryptographic evidence, compatibility,
    /// outcome, source and confidence.
    LifecycleFirst,
    /// The caller names the criteria, in order.
    Criteria(Vec<(EvidenceDimension, SortDirection)>),
}

impl ExplicitPolicy {
    /// The criteria this policy applies, in order.
    #[must_use]
    pub fn criteria(&self) -> Vec<(EvidenceDimension, SortDirection)> {
        match self {
            Self::EvidenceFirst => vec![
                (EvidenceDimension::Cryptographic, SortDirection::BestFirst),
                (EvidenceDimension::Outcome, SortDirection::BestFirst),
                (EvidenceDimension::Compatibility, SortDirection::BestFirst),
                (EvidenceDimension::Lifecycle, SortDirection::BestFirst),
                (EvidenceDimension::Source, SortDirection::BestFirst),
                (EvidenceDimension::Confidence, SortDirection::BestFirst),
            ],
            Self::LifecycleFirst => vec![
                (EvidenceDimension::Lifecycle, SortDirection::BestFirst),
                (EvidenceDimension::Cryptographic, SortDirection::BestFirst),
                (EvidenceDimension::Compatibility, SortDirection::BestFirst),
                (EvidenceDimension::Outcome, SortDirection::BestFirst),
                (EvidenceDimension::Source, SortDirection::BestFirst),
                (EvidenceDimension::Confidence, SortDirection::BestFirst),
            ],
            Self::Criteria(criteria) => criteria.clone(),
        }
    }
}

/// The dimension-separated ranking of one constitutional answer.
#[derive(Clone, Debug, PartialEq)]
pub struct DimensionRanking {
    /// The ranked entries, in the ranking's order.
    entries: Vec<RankedResult>,
    /// How many entries carry each disposition.
    disposition_counts: BTreeMap<Disposition, usize>,
}

impl DimensionRanking {
    /// Rank one constitutional answer.
    ///
    /// The order is the constitutional query's own order: this constructor
    /// reorders nothing, because choosing an order is the caller's decision.
    #[must_use]
    pub fn from_results(results: MemoryResults) -> Self {
        let entries = results
            .results
            .into_iter()
            .map(|result| {
                let dimension_signals = DimensionSignals::of(&result);
                RankedResult {
                    result,
                    vector_rank: None,
                    vector_similarity: None,
                    dimension_signals,
                }
            })
            .collect();
        Self {
            entries,
            disposition_counts: results.disposition_counts,
        }
    }

    /// Build a ranking from entries the caller already assembled.
    pub(super) fn from_entries(
        entries: Vec<RankedResult>,
        disposition_counts: BTreeMap<Disposition, usize>,
    ) -> Self {
        Self {
            entries,
            disposition_counts,
        }
    }

    /// The ranked entries, in the ranking's order.
    #[must_use]
    pub fn entries(&self) -> &[RankedResult] {
        &self.entries
    }

    /// How many entries the ranking holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the ranking holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many entries carry each disposition.
    #[must_use]
    pub fn disposition_counts(&self) -> &BTreeMap<Disposition, usize> {
        &self.disposition_counts
    }

    /// The entry for one episode digest, if the ranking holds it.
    #[must_use]
    pub fn result(&self, digest: [u8; 32]) -> Option<&RankedResult> {
        self.entries
            .iter()
            .find(|entry| entry.result.episode_digest == digest)
    }

    /// The ranked entries, consumed.
    #[must_use]
    pub fn into_entries(self) -> Vec<RankedResult> {
        self.entries
    }

    /// Order the ranking by one chosen dimension.
    ///
    /// The order is the dimension's documented ordering: a sequence of
    /// labelled states, never a number. Records that tie on that dimension
    /// keep the order the ranking already had, so the result is reproducible
    /// for the same corpus and composes with a second ordering.
    #[must_use]
    pub fn order_by_dimension(
        &self,
        dimension: EvidenceDimension,
        direction: SortDirection,
    ) -> Self {
        self.order_by_dimensions(&[(dimension, direction)])
    }

    /// Order the ranking by an explicit list of dimensions.
    ///
    /// The criteria are applied lexicographically: the first criterion that
    /// separates two records decides their order. This is still eight
    /// separate orderings applied one after another, not eight dimensions
    /// collapsed into one number.
    #[must_use]
    pub fn order_by_dimensions(&self, criteria: &[(EvidenceDimension, SortDirection)]) -> Self {
        let mut entries = self.entries.clone();
        entries.sort_by(|left, right| compare(left, right, criteria));
        Self {
            entries,
            disposition_counts: self.disposition_counts.clone(),
        }
    }

    /// Order the ranking by an explicitly named policy.
    ///
    /// This is the opt-in composite order. It is **an arbitrary policy
    /// choice**, named as one: it is not a trust measure, it does not claim to
    /// be one, and it is never applied unless the caller asks for it by name.
    /// It collapses nothing either — the policy is a lexicographic sequence of
    /// named per-dimension orderings, and every dimension stays separately
    /// inspectable on every entry.
    #[must_use]
    pub fn explicit_policy_order(&self, policy: &ExplicitPolicy) -> Self {
        self.order_by_dimensions(&policy.criteria())
    }
}

/// Compare two entries under a list of per-dimension criteria.
fn compare(
    left: &RankedResult,
    right: &RankedResult,
    criteria: &[(EvidenceDimension, SortDirection)],
) -> Ordering {
    for (dimension, direction) in criteria {
        let ordering = compare_dimension(
            *dimension,
            left.dimension_signals.get(*dimension),
            right.dimension_signals.get(*dimension),
            *direction,
        );
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    Ordering::Equal
}

/// Compare two signals under one dimension's ordering.
///
/// A signal whose label the dimension does not know sorts after every known
/// signal, in either direction: an unclassifiable signal is never presented
/// as a strong one.
fn compare_dimension(
    dimension: EvidenceDimension,
    left: DimensionSignal,
    right: DimensionSignal,
    direction: SortDirection,
) -> Ordering {
    match (rank_of(dimension, &left), rank_of(dimension, &right)) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(left_rank), Some(right_rank)) => match direction {
            SortDirection::BestFirst => left_rank.cmp(&right_rank),
            SortDirection::WorstFirst => right_rank.cmp(&left_rank),
        },
    }
}
