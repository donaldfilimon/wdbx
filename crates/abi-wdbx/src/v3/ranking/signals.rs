//! The eight per-dimension signals: what each dimension says, separately.
//!
//! The canonical contract asks for trust to be eight dimensions and never one
//! number. This module is the ranking's view of that: for every result it
//! derives one [`DimensionSignal`] per dimension, and every signal stays
//! inspectable on its own.
//!
//! ## Missing is not zero
//!
//! The four signal states exist so that a gap in the evidence never reads as
//! a measurement:
//!
//! * [`DimensionSignal::Present`] — the dimension was measured and the label
//!   names the value.
//! * [`DimensionSignal::Absent`] — the dimension was measured and reads zero:
//!   the ledger looked and found nothing (no signature was carried, no
//!   outcome was observed). This is a finding.
//! * [`DimensionSignal::Unknown`] — the dimension was never measured: the
//!   ledger holds no such evidence at all. This is a gap, never a zero and
//!   never a default that could pass for a measurement.
//! * [`DimensionSignal::NotApplicable`] — the dimension does not apply to this
//!   record.
//!
//! ## What the signal drops
//!
//! A signal is a *lossy projection* of the canonical evidence, and it says so.
//! It keeps the fact that moves an ordering and drops the rest; the
//! authoritative per-dimension evidence stays on the result itself
//! ([`crate::v3::episode::ConstitutionalResult::dimensions`]). Concretely, the
//! source signal drops the recording role and the reputation, the lifecycle
//! signal drops the retention class, the age, and the kinds of a live hold,
//! and the confidence signal drops the recorded uncertainty. None of those is
//! lost: the caller reads them from the canonical dimensions.
//!
//! ## The lifecycle signal and the disposition
//!
//! The lifecycle signal names the most severe lifecycle fact the canonical
//! query reports, in the canonical query's own precedence: a forgotten
//! record, a revoked record, an open quarantine, a live hold, a superseded
//! record, a tombstone, and otherwise a live record. The signal reads the
//! canonical
//! [`LifecycleEvidence`](crate::v3::episode::LifecycleEvidence) rather than
//! re-deriving the facts, so sorting by the lifecycle dimension never
//! contradicts the disposition the canonical query reported.
//!
//! A record that is both held and revoked reads `revoked`, because the
//! canonical precedence puts a revocation above a hold. The hold stays
//! readable on the canonical dimension; the signal names the one fact that
//! decides the ordering.

use crate::v3::episode::{
    CompatibilityEvidence, ConstitutionalResult, EpisodeSource, EvidenceDimension,
    EvidenceDimensions, OutcomeEvidence, RevocationEvidence, SignatureEvidence,
};

/// The marker of a signal whose measurement reads zero.
const ABSENT: &str = "\u{1}absent";

/// The marker of a signal the ledger never measured.
const UNKNOWN: &str = "\u{1}unknown";

/// The marker of a signal that does not apply to the record.
const NOT_APPLICABLE: &str = "\u{1}not-applicable";

/// What one evidence dimension says about one record.
///
/// The signal is a labelled state, never a number: there is nothing here that
/// could be multiplied into a score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DimensionSignal {
    /// The dimension was measured; the label names the value.
    Present(&'static str),
    /// The dimension was measured and reads zero: nothing was found.
    Absent,
    /// The dimension was never measured: the ledger holds no such evidence.
    Unknown,
    /// The dimension does not apply to this record.
    NotApplicable,
}

impl DimensionSignal {
    /// The stable text of the signal.
    ///
    /// For a measured signal this is the value's name (`"valid"`,
    /// `"live"`, `"C3"`, ...); otherwise it is the state's own name.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Present(label) => label,
            Self::Absent => "absent",
            Self::Unknown => "unknown",
            Self::NotApplicable => "not-applicable",
        }
    }

    /// The kind of the signal, without its value.
    #[must_use]
    pub const fn kind(self) -> &'static str {
        match self {
            Self::Present(_) => "present",
            Self::Absent => "absent",
            Self::Unknown => "unknown",
            Self::NotApplicable => "not-applicable",
        }
    }

    /// Whether the dimension was measured at all.
    ///
    /// A measured signal has a value, even when that value is zero.
    #[must_use]
    pub const fn is_measured(self) -> bool {
        matches!(self, Self::Present(_) | Self::Absent)
    }
}

/// The eight dimensions, each with its own signal.
///
/// No accessor turns the eight into one number, and no field of this type
/// could stand in for one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DimensionSignals {
    /// The cryptographic dimension's signal.
    pub cryptographic: DimensionSignal,
    /// The source dimension's signal.
    pub source: DimensionSignal,
    /// The calibration dimension's signal.
    pub calibration: DimensionSignal,
    /// The outcome dimension's signal.
    pub outcome: DimensionSignal,
    /// The compatibility dimension's signal.
    pub compatibility: DimensionSignal,
    /// The regime dimension's signal.
    pub regime: DimensionSignal,
    /// The lifecycle dimension's signal.
    pub lifecycle: DimensionSignal,
    /// The confidence dimension's signal.
    pub confidence: DimensionSignal,
}

impl DimensionSignals {
    /// Derive the eight signals from one constitutional result.
    ///
    /// The derivation reads only the canonical evidence; it never invents a
    /// measurement and never consults anything outside the result.
    #[must_use]
    pub fn of(result: &ConstitutionalResult) -> Self {
        let dimensions = &result.dimensions;
        Self {
            cryptographic: cryptographic_signal(dimensions),
            source: source_signal(dimensions),
            // The ledger holds no calibration record and no hardware-health
            // signal. That is a gap, and it reads as one.
            calibration: DimensionSignal::Unknown,
            outcome: outcome_signal(dimensions),
            compatibility: compatibility_signal(dimensions),
            // The ledger holds no task-regime field and no safety-constraint
            // binding. That is a gap too.
            regime: DimensionSignal::Unknown,
            lifecycle: lifecycle_signal(dimensions),
            confidence: confidence_signal(dimensions),
        }
    }

    /// One dimension's signal.
    #[must_use]
    pub fn get(&self, dimension: EvidenceDimension) -> DimensionSignal {
        match dimension {
            EvidenceDimension::Cryptographic => self.cryptographic,
            EvidenceDimension::Source => self.source,
            EvidenceDimension::Calibration => self.calibration,
            EvidenceDimension::Outcome => self.outcome,
            EvidenceDimension::Compatibility => self.compatibility,
            EvidenceDimension::Regime => self.regime,
            EvidenceDimension::Lifecycle => self.lifecycle,
            EvidenceDimension::Confidence => self.confidence,
        }
    }
}

/// The cryptographic dimension's signal.
///
/// The four signing states stay apart. A record that carries no signature is
/// a *measured absence* (the ledger looked at the envelope and there was no
/// signature); a record signed under a key the verifier cannot resolve is a
/// *gap* (the value cannot be established). A forged signature is neither: it
/// is a measured, negative finding.
fn cryptographic_signal(dimensions: &EvidenceDimensions) -> DimensionSignal {
    match dimensions.cryptographic.signature {
        SignatureEvidence::Valid => DimensionSignal::Present("valid"),
        SignatureEvidence::Invalid => DimensionSignal::Present("invalid"),
        SignatureEvidence::Unverifiable => DimensionSignal::Unknown,
        SignatureEvidence::Unsigned => DimensionSignal::Absent,
    }
}

/// The source dimension's signal.
///
/// The source class is bound in the commitment, so the dimension is measured.
/// The recording role and the reputation are dropped from the signal: the
/// ledger holds no reputation at all, and the role does not move an ordering.
/// Both stay readable on the canonical result.
fn source_signal(dimensions: &EvidenceDimensions) -> DimensionSignal {
    match dimensions.source.source_type {
        EpisodeSource::Proposal => DimensionSignal::Present("proposal"),
        EpisodeSource::DiscordGuild => DimensionSignal::Present("discord_guild"),
        EpisodeSource::DiscordVoice => DimensionSignal::Present("discord_voice"),
        EpisodeSource::LocalRuntime => DimensionSignal::Present("local_runtime"),
    }
}

/// The outcome dimension's signal.
///
/// "Unobserved" is the ledger's own word for it: no outcome was observed. That
/// is a measured zero of observations, not a missing measurement, so it reads
/// as absent rather than unknown.
fn outcome_signal(dimensions: &EvidenceDimensions) -> DimensionSignal {
    match dimensions.outcome {
        OutcomeEvidence::Validated => DimensionSignal::Present("validated"),
        OutcomeEvidence::Contradicted => DimensionSignal::Present("contradicted"),
        OutcomeEvidence::Unobserved => DimensionSignal::Absent,
    }
}

/// The compatibility dimension's signal.
fn compatibility_signal(dimensions: &EvidenceDimensions) -> DimensionSignal {
    match dimensions.compatibility {
        CompatibilityEvidence::Compatible => DimensionSignal::Present("compatible"),
        CompatibilityEvidence::ContractStale => DimensionSignal::Present("contract-stale"),
        CompatibilityEvidence::PolicyStale => DimensionSignal::Present("policy-stale"),
        CompatibilityEvidence::Unknown => DimensionSignal::Unknown,
    }
}

/// The lifecycle dimension's signal.
///
/// The flags that move an ordering are kept; the retention class, the age,
/// and the kinds of a live hold are dropped from the signal and stay
/// readable on the canonical result.
///
/// The signal names the most severe lifecycle fact the canonical query
/// reports, in the canonical query's own precedence, so the ranking never
/// contradicts the disposition the query reported: a forgotten record, a
/// revoked record, an open quarantine, a live hold, a superseded record, a
/// tombstone, and otherwise a live record. The facts are read from the
/// canonical lifecycle evidence, never re-derived here.
fn lifecycle_signal(dimensions: &EvidenceDimensions) -> DimensionSignal {
    let lifecycle = &dimensions.lifecycle;
    if lifecycle.forgotten {
        DimensionSignal::Present("forgotten")
    } else if lifecycle.revocation == RevocationEvidence::Revoked {
        DimensionSignal::Present("revoked")
    } else if lifecycle.open_quarantine.is_some() {
        DimensionSignal::Present("quarantined")
    } else if lifecycle.hold.is_held() {
        DimensionSignal::Present("held")
    } else if lifecycle.superseded_by.is_some() {
        DimensionSignal::Present("superseded")
    } else if lifecycle.tombstoned {
        DimensionSignal::Present("tombstone")
    } else {
        DimensionSignal::Present("live")
    }
}

/// The confidence dimension's signal.
///
/// A forgetting tombstone carries no remembered claim, so there is nothing
/// whose maturity could bear on a retrieval decision: the dimension does not
/// apply to it. Every other record carries a committed maturity, which is a
/// measurement; the recorded uncertainty is dropped from the signal and stays
/// readable on the canonical result.
fn confidence_signal(dimensions: &EvidenceDimensions) -> DimensionSignal {
    if dimensions.lifecycle.tombstoned {
        return DimensionSignal::NotApplicable;
    }
    DimensionSignal::Present(dimensions.confidence.maturity.label())
}

/// The ordering of one dimension, strongest value first.
///
/// Each entry is a group of labels that rank equally. The groups are the
/// documented ordering policy for that dimension, stated in one place so a
/// reader can see exactly what "sorting by this dimension" means.
///
/// The orderings mirror the canonical query's own disposition precedence, so
/// sorting by a dimension never contradicts the disposition the canonical
/// query reported:
///
/// * cryptographic: valid, unsigned, unverifiable, invalid. A record that
///   makes no cryptographic claim (unsigned) is established on its commitment
///   alone and the canonical query accepts it; a record whose signature names
///   a key nobody can resolve stays `Unknown`, and a forged signature is
///   rejected outright.
/// * source: an observation, then a proposal. The canonical query holds a
///   proposal to be uncertain and an observation to be accepted. The source
///   types are otherwise unordered: the ranking does not invent a hierarchy of
///   source classes.
/// * calibration, regime: nothing. The ledger holds neither, so ordering by
///   either is a no-op that says so.
/// * outcome: validated, unobserved, contradicted.
/// * compatibility: compatible, policy-stale, contract-stale, unknown. A
///   policy-version drift is the narrower break; a contract-digest mismatch
///   means the record was written against different rules entirely.
/// * lifecycle: live, tombstone, superseded, held, quarantined, revoked,
///   forgotten. The order mirrors the canonical query's disposition
///   precedence for the lifecycle facts: a hold is a live freeze and a
///   superseded record is a stale correction, so the freeze is worse news;
///   a quarantine is an open dispute about the content; a revocation is an
///   authority's withdrawal; and a forgotten record's content is gone.
/// * confidence: the committed maturity, from C7 down to C0.
fn order_groups(dimension: EvidenceDimension) -> &'static [&'static [&'static str]] {
    match dimension {
        EvidenceDimension::Cryptographic => &[
            &["valid"],
            &[ABSENT],
            &[UNKNOWN],
            &["invalid"],
            &[NOT_APPLICABLE],
        ],
        EvidenceDimension::Source => &[
            &["discord_guild", "discord_voice", "local_runtime"],
            &["proposal"],
            &[ABSENT],
            &[UNKNOWN],
            &[NOT_APPLICABLE],
        ],
        EvidenceDimension::Calibration | EvidenceDimension::Regime => {
            &[&[UNKNOWN], &[NOT_APPLICABLE]]
        }
        EvidenceDimension::Outcome => &[
            &["validated"],
            &[ABSENT],
            &["contradicted"],
            &[UNKNOWN],
            &[NOT_APPLICABLE],
        ],
        EvidenceDimension::Compatibility => &[
            &["compatible"],
            &["policy-stale"],
            &["contract-stale"],
            &[UNKNOWN],
            &[NOT_APPLICABLE],
        ],
        EvidenceDimension::Lifecycle => &[
            &["live"],
            &["tombstone"],
            &["superseded"],
            &["held"],
            &["quarantined"],
            &["revoked"],
            &["forgotten"],
            &[ABSENT],
            &[UNKNOWN],
            &[NOT_APPLICABLE],
        ],
        EvidenceDimension::Confidence => &[
            &["C7"],
            &["C6"],
            &["C5"],
            &["C4"],
            &["C3"],
            &["C2"],
            &["C1"],
            &["C0"],
            &[UNKNOWN],
            &[NOT_APPLICABLE],
        ],
    }
}

/// The position of one signal in one dimension's ordering.
///
/// `None` means the signal carries a label the dimension's ordering does not
/// know; such a signal sorts after every known signal, in either direction.
pub(super) fn rank_of(dimension: EvidenceDimension, signal: &DimensionSignal) -> Option<usize> {
    let groups = order_groups(dimension);
    let needle = match signal {
        DimensionSignal::Present(label) => *label,
        DimensionSignal::Absent => ABSENT,
        DimensionSignal::Unknown => UNKNOWN,
        DimensionSignal::NotApplicable => NOT_APPLICABLE,
    };
    groups.iter().position(|group| group.contains(&needle))
}
