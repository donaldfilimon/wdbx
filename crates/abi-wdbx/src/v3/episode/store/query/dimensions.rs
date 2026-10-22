//! The eight evidence dimensions and the state each one reports.
//!
//! Every dimension is a separate type with its own closed vocabulary. None
//! of them carries a number that could stand in for the dimension, and none
//! of them collapses the dimension into a value that could pass for a
//! measurement: a dimension the ledger holds no evidence for says
//! `Unknown`, and a dimension the ledger *does* hold reports a measured
//! state.

use super::super::super::types::{ActorKind, EpisodeSource, EvidenceLevel, RetentionClass};
use super::super::holds::{Hold, HoldKind, HoldStatus};

/// The eight evidence dimensions a constitutional query reports.
///
/// The names are the canonical ones. They are separate types, never one
/// number: the caller inspects each dimension on its own and decides what to
/// do with the record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EvidenceDimension {
    /// Signature evidence and the status of the key it names.
    Cryptographic,
    /// Identity, role, and reputation of the source.
    Source,
    /// Sensor and model calibration, and hardware health.
    Calibration,
    /// The observed outcome of the record.
    Outcome,
    /// Model, policy, and schema compatibility.
    Compatibility,
    /// Task regime and safety-constraint compatibility.
    Regime,
    /// Age, supersession, retention class, quarantine, hold, and revocation.
    Lifecycle,
    /// Recorded uncertainty or interval.
    Confidence,
}

impl EvidenceDimension {
    /// The canonical name of the dimension.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cryptographic => "cryptographic",
            Self::Source => "source",
            Self::Calibration => "calibration",
            Self::Outcome => "outcome",
            Self::Compatibility => "compatibility",
            Self::Regime => "regime",
            Self::Lifecycle => "lifecycle",
            Self::Confidence => "confidence",
        }
    }

    /// Every dimension, in canonical order.
    #[must_use]
    pub const fn all() -> [Self; 8] {
        [
            Self::Cryptographic,
            Self::Source,
            Self::Calibration,
            Self::Outcome,
            Self::Compatibility,
            Self::Regime,
            Self::Lifecycle,
            Self::Confidence,
        ]
    }
}

/// The state of the detached signature over an episode digest.
///
/// The four states are kept apart on purpose. `Unsigned` and `Unverifiable`
/// are missing evidence; only `Invalid` is a negative finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureEvidence {
    /// The signature verifies under the resolved key.
    Valid,
    /// The signature does not verify, or the record's digest does not
    /// reproduce from the record's own fields.
    Invalid,
    /// The record names a key the caller's resolver could not resolve.
    Unverifiable,
    /// The record carries no signature.
    Unsigned,
}

/// The status of the key a signature names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignerKeyStatus {
    /// The resolver returned the key the signature names.
    Resolved,
    /// The resolver does not hold the key the signature names.
    Unresolved,
    /// The record names no key, because it carries no signature.
    Absent,
}

/// The cryptographic dimension: the detached signature and the key it names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CryptographicEvidence {
    /// The state of the detached signature over the episode digest.
    pub signature: SignatureEvidence,
    /// The status of the key the signature names.
    pub key_status: SignerKeyStatus,
}

/// The reputation the ledger holds for a source identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReputationEvidence {
    /// The ledger records no reputation signal: missing evidence, not a zero.
    Unknown,
}

/// The source dimension: what kind of source recorded the episode, in which
/// role, and what the ledger knows about its reputation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceEvidence {
    /// The closed source class bound into the commitment.
    pub source_type: EpisodeSource,
    /// The role class of the identity that recorded the episode.
    pub actor_kind: ActorKind,
    /// The reputation of that identity.
    pub reputation: ReputationEvidence,
}

/// The calibration dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationEvidence {
    /// The ledger holds no calibration record and no hardware-health signal.
    Unknown,
}

/// The outcome dimension: what the ledger observed about the record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutcomeEvidence {
    /// A review closed a quarantine on the record and found it valid.
    Validated,
    /// A contradiction names the record.
    Contradicted,
    /// No outcome has been observed for the record.
    Unobserved,
}

/// The compatibility dimension: whether the record still binds to the
/// contract and policy this store was opened with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompatibilityEvidence {
    /// The contract revision, the contract digest, and the guild's current
    /// policy version all match the store's binding.
    Compatible,
    /// The record was written under another contract revision or digest.
    ContractStale,
    /// The record was written under another policy version.
    PolicyStale,
    /// The store holds no policy binding for the record's guild.
    Unknown,
}

/// The regime dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegimeEvidence {
    /// The ledger holds no task-regime field and no safety-constraint binding.
    Unknown,
}

/// The age the ledger holds for a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgeEvidence {
    /// The ledger records no timestamp, so it holds no age.
    Unknown,
}

/// The kinds of live hold that freeze one candidate.
///
/// The vocabulary is the store's own closed one: a legal hold, an
/// investigation hold, and a user-request hold. Each kind is a named state
/// that is either in force or not. Nothing here counts anything, and the
/// three flags are read one by one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HoldKinds {
    /// A legal or regulatory hold is in force.
    pub legal: bool,
    /// An investigation hold is in force.
    pub investigation: bool,
    /// A user-requested hold is in force.
    pub user_request: bool,
}

impl HoldKinds {
    /// Whether no kind of hold is in force.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.legal && !self.investigation && !self.user_request
    }

    /// Record one kind of hold as in force.
    pub(super) fn add(&mut self, kind: HoldKind) {
        match kind {
            HoldKind::Legal => self.legal = true,
            HoldKind::Investigation => self.investigation = true,
            HoldKind::UserRequest => self.user_request = true,
        }
    }
}

/// The hold state of one memory candidate.
///
/// A hold is a placement that freezes a candidate: while a live hold names
/// it, the candidate is not expired away, not reclaimed, not silently
/// superseded, and its payload is not destroyed.
///
/// `NotHeld` is a measured state and not missing evidence: the store knows
/// which candidates it holds, and it says so. A hold that was released is
/// not in force and reads `NotHeld` too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoldEvidence {
    /// No live hold names the candidate.
    NotHeld,
    /// A live hold names the candidate and freezes it.
    Held(HoldKinds),
}

impl HoldEvidence {
    /// Whether a live hold freezes the candidate.
    #[must_use]
    pub const fn is_held(self) -> bool {
        matches!(self, Self::Held(_))
    }
}

/// The revocation state of one memory candidate.
///
/// A revocation withdraws a candidate by authority: it is not live, no later
/// correction may name it, and it is never resurrected. Its record and its
/// anti-resurrection metadata are retained, because a revocation is a
/// withdrawal from use and not an erasure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevocationEvidence {
    /// The candidate was never revoked in this guild.
    NotRevoked,
    /// An authority revoked the candidate.
    Revoked,
}

/// The lifecycle dimension: how the record stands in the guild's memory
/// graph right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifecycleEvidence {
    /// The retention class the candidate requested.
    pub retention: RetentionClass,
    /// The candidate that supersedes this one, if any.
    pub superseded_by: Option<[u8; 32]>,
    /// True when this candidate supersedes another candidate.
    pub supersedes: bool,
    /// True when a later tombstone forgot this candidate.
    pub forgotten: bool,
    /// True when this candidate is itself a zero-payload forgetting tombstone.
    pub tombstoned: bool,
    /// The open quarantine edge naming this candidate, if any.
    pub open_quarantine: Option<[u8; 32]>,
    /// The live hold that freezes this candidate, if any.
    ///
    /// The dimension distinguishes *no hold* from *held*; it never reports a
    /// hold that was released, because a released hold freezes nothing.
    pub hold: HoldEvidence,
    /// The revocation that withdrew this candidate, if any.
    ///
    /// The dimension distinguishes *not revoked* from *revoked*.
    pub revocation: RevocationEvidence,
    /// The age of this candidate.
    pub age: AgeEvidence,
}

/// The uncertainty the ledger holds for a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UncertaintyEvidence {
    /// The ledger records no uncertainty value and no interval: missing
    /// evidence, not a zero.
    Unknown,
}

/// The confidence dimension: the recorded evidence maturity of the record,
/// and the uncertainty the ledger holds for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfidenceEvidence {
    /// The evidence maturity bound into the commitment.
    pub maturity: EvidenceLevel,
    /// The uncertainty value or interval recorded for the record.
    pub uncertainty: UncertaintyEvidence,
}

/// The eight dimensions, each separately inspectable. No accessor turns them
/// into one number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceDimensions {
    /// Signature evidence and key status.
    pub cryptographic: CryptographicEvidence,
    /// Source identity, role, and reputation.
    pub source: SourceEvidence,
    /// Calibration and hardware health.
    pub calibration: CalibrationEvidence,
    /// The observed outcome.
    pub outcome: OutcomeEvidence,
    /// Model, policy, and schema compatibility.
    pub compatibility: CompatibilityEvidence,
    /// Task regime and safety-constraint compatibility.
    pub regime: RegimeEvidence,
    /// Age, supersession, retention class, quarantine, hold, and revocation.
    pub lifecycle: LifecycleEvidence,
    /// Recorded uncertainty.
    pub confidence: ConfidenceEvidence,
}

/// The hold kinds that are in force, given the holds the store records.
///
/// Only a live hold freezes a candidate, so a released hold contributes
/// nothing here.
pub(super) fn live_hold_kinds(holds: &[Hold]) -> HoldKinds {
    let mut kinds = HoldKinds::default();
    for hold in holds {
        if hold.status == HoldStatus::Live {
            kinds.add(hold.kind);
        }
    }
    kinds
}
