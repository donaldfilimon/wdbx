//! The studio's read model over the canonical ledger.
//!
//! The projection is the substrate's own answer, kept whole. Every episode
//! carries the disposition the substrate derived and the eight evidence
//! dimensions the substrate measured, each separately inspectable. The
//! projection adds no number of its own, and it never folds the dimensions
//! into one: that collapse is the failure this read model exists to avoid.

use abi_wdbx::v3::episode::{ConstitutionalResult, Disposition};
use std::collections::BTreeMap;

/// The greatest number of episodes one projection carries.
///
/// The substrate bounds a query by its own receipt bound; this is the
/// studio's own, smaller bound, and the projection says when it reached it.
pub const PROJECTION_LIMIT: usize = 1_024;

/// The studio's projection of the canonical ledger.
///
/// The projection is a read model and not a ranking: it holds the
/// substrate's disposition and the substrate's eight dimensions, and no
/// value that could stand in for them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalProjection {
    /// The scope the answer is for.
    pub scope: String,
    /// The episodes, in ledger order.
    pub episodes: Vec<ConstitutionalResult>,
    /// How many episodes carry each disposition.
    pub disposition_counts: BTreeMap<Disposition, usize>,
    /// True when the projection reached its own bound, so the ledger may hold
    /// more episodes than the projection lists.
    pub truncated: bool,
}
