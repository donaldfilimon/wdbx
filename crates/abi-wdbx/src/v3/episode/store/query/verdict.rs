//! The verdict: the closed disposition vocabulary, the closed reason
//! vocabulary, and the rules that derive one from the other.

use super::super::super::types::{EpisodeSource, EvidenceLevel};
use super::dimensions::{
    CompatibilityEvidence, EvidenceDimension, EvidenceDimensions, OutcomeEvidence,
    RevocationEvidence, SignatureEvidence,
};

/// What a caller may do with one record.
///
/// A disposition is a short verdict; the eight dimensions and the
/// explanation say why. Two rules come straight from the canonical contract:
/// a cryptographically invalid record is rejected, and a validly signed but
/// semantically uncertain record is retained and inspectable rather than
/// dropped.
///
/// The vocabulary is closed. A hold and a revocation each have their own
/// member, because both are facts a caller must see before it offers a
/// record: a held record is frozen by a live governance placement, and a
/// revoked record has been withdrawn by an authority. Neither is folded into
/// another member, because each of the others means something different: a
/// quarantine and a contradiction doubt the content, a rejection says the
/// record is not evidence at all, and `Uncertain` says the record is
/// retained and inspectable but its binding is not current.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Disposition {
    /// Nothing in the ledger contradicts the record.
    ///
    /// An unsigned record is accepted on its commitment: the ledger
    /// reproduced its digest, and the cryptographic dimension still reports
    /// `Unsigned`, so a caller that requires a signature can see that there
    /// is none.
    Accepted,
    /// The record is retained and inspectable, but semantically uncertain:
    /// it is superseded, it was proposed rather than observed, its binding is
    /// stale, or its recorded maturity is specification only.
    Uncertain,
    /// An open contradiction names the record.
    Contradicted,
    /// An open quarantine names the record.
    Quarantined,
    /// A live hold freezes the record.
    ///
    /// The record's content is preserved: it is not expired away, not
    /// reclaimed, not silently superseded, and its payload is not destroyed.
    /// Nothing in the ledger doubts the content itself, and the record stays
    /// listed with the hold state the lifecycle dimension reports.
    Held,
    /// An authority revoked the record.
    ///
    /// The record is withdrawn from use: it is not live, no later correction
    /// may name it, and it is never resurrected. Its record and its
    /// anti-resurrection metadata are retained, so this is not an erasure
    /// and not a rejection.
    Revoked,
    /// The record must not be used: its cryptographic evidence is invalid, or
    /// a forgetting tombstone erased its content.
    Rejected,
    /// The store holds too little evidence to judge the record.
    Unknown,
}

impl Disposition {
    /// The canonical name of the disposition.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Uncertain => "uncertain",
            Self::Contradicted => "contradicted",
            Self::Quarantined => "quarantined",
            Self::Held => "held",
            Self::Revoked => "revoked",
            Self::Rejected => "rejected",
            Self::Unknown => "unknown",
        }
    }
}

/// Why one dimension reads as it does. Every reason is a closed,
/// content-free label: no payload, path, key identifier, or number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// A detached signature verified under the resolved key.
    SignatureVerified,
    /// The signature does not verify, or the digest does not reproduce.
    SignatureFailed,
    /// The record names a key the resolver could not resolve.
    SignatureKeyUnresolved,
    /// The record carries no signature.
    SignatureAbsent,
    /// The source class and the recording role are bound in the commitment.
    SourceBound,
    /// The record was proposed; it was never an executed observation.
    ProposedSource,
    /// The ledger holds no calibration or hardware-health evidence.
    NoCalibration,
    /// A contradiction names the record.
    Contradicted,
    /// A review closed a quarantine on the record and found it valid.
    ReviewedValid,
    /// No outcome has been observed for the record.
    OutcomeUnobserved,
    /// The record matches the store's contract and policy binding.
    BindingCurrent,
    /// The record was written under another contract revision or digest.
    BindingContractStale,
    /// The record was written under another policy version.
    BindingPolicyStale,
    /// The store holds no policy binding for the record's guild.
    BindingUnknown,
    /// The ledger holds no task-regime or safety-constraint evidence.
    NoRegime,
    /// Nothing supersedes, forgets, or flags the record.
    Live,
    /// A later candidate supersedes this one.
    Superseded,
    /// A later tombstone forgot this one.
    Forgotten,
    /// This record is itself a zero-payload forgetting tombstone.
    Tombstone,
    /// An open quarantine edge names the record.
    Quarantined,
    /// A live hold freezes the record.
    Held,
    /// An authority revoked the record.
    Revoked,
    /// A live hold freezes the record and an authority revoked it.
    ///
    /// The two facts are named together because a reader of the explanation
    /// must not be left to guess whether the hold is in force.
    HeldAndRevoked,
    /// The record's evidence maturity is bound in the commitment.
    MaturityBound,
}

/// The per-dimension explanation of one result: one entry per dimension,
/// bounded by the number of dimensions, and never one number for the record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Explanation {
    /// One `(dimension, reason)` pair per dimension, in canonical order.
    pub entries: Vec<(EvidenceDimension, Reason)>,
}

/// The disposition the dimensions imply.
///
/// The precedence is fixed, and every clause names the dimension it reads:
///
/// 1. an invalid signature, or a forgotten record, is
///    [`Disposition::Rejected`];
/// 2. a revoked record is [`Disposition::Revoked`];
/// 3. an open quarantine is [`Disposition::Quarantined`];
/// 4. an open contradiction is [`Disposition::Contradicted`];
/// 5. a live hold is [`Disposition::Held`];
/// 6. a superseded record, a stale binding, a proposed record, or a record
///    bound at specification-only maturity is [`Disposition::Uncertain`];
/// 7. an unverifiable signature is [`Disposition::Unknown`], because missing
///    evidence is not a negative finding;
/// 8. anything else is [`Disposition::Accepted`].
///
/// The order is a judgement, and two of its steps are worth stating:
///
/// * **A revocation takes precedence over a quarantine and a
///   contradiction.** A quarantine and a contradiction are open, reviewable
///   disputes about the content; a revocation is a terminal withdrawal by an
///   authority. A record that is both revoked and contradicted is read as
///   `Revoked`, and the contradiction stays visible in the outcome
///   dimension.
/// * **A hold comes after a quarantine and a contradiction, and before a
///   superseded record.** A hold doubts nothing about the content: it
///   freezes the record's destruction and its supersession. A dispute about
///   the content is worse news than a freeze, and a freeze is worse news
///   than a stale correction, because the freeze is a live constraint on the
///   record while the correction is a fact about its history.
///
/// A cryptographically invalid record is rejected and a semantically
/// uncertain one is retained and inspectable: neither is ever dropped, and
/// neither is a held or a revoked record, which is exactly why the hold and
/// the revocation have members of their own.
pub(super) fn disposition_of(dimensions: &EvidenceDimensions) -> Disposition {
    if dimensions.cryptographic.signature == SignatureEvidence::Invalid {
        return Disposition::Rejected;
    }
    if dimensions.lifecycle.forgotten {
        return Disposition::Rejected;
    }
    if dimensions.lifecycle.revocation == RevocationEvidence::Revoked {
        return Disposition::Revoked;
    }
    if dimensions.lifecycle.open_quarantine.is_some() {
        return Disposition::Quarantined;
    }
    if dimensions.outcome == OutcomeEvidence::Contradicted {
        return Disposition::Contradicted;
    }
    if dimensions.lifecycle.hold.is_held() {
        return Disposition::Held;
    }
    if dimensions.lifecycle.superseded_by.is_some() {
        return Disposition::Uncertain;
    }
    if dimensions.compatibility != CompatibilityEvidence::Compatible {
        return Disposition::Uncertain;
    }
    if dimensions.source.source_type == EpisodeSource::Proposal {
        return Disposition::Uncertain;
    }
    if dimensions.confidence.maturity == EvidenceLevel::C0 {
        return Disposition::Uncertain;
    }
    if dimensions.cryptographic.signature == SignatureEvidence::Unverifiable {
        return Disposition::Unknown;
    }
    Disposition::Accepted
}

/// One explanation entry per dimension, in canonical order.
///
/// The lifecycle entry names the most severe fact the dimension reports, and
/// it names both facts when a record is both held and revoked, so a reader
/// of the explanation alone never has to guess whether a freeze is in force.
pub(super) fn explanations(dimensions: &EvidenceDimensions) -> Vec<(EvidenceDimension, Reason)> {
    let cryptographic = match dimensions.cryptographic.signature {
        SignatureEvidence::Valid => Reason::SignatureVerified,
        SignatureEvidence::Invalid => Reason::SignatureFailed,
        SignatureEvidence::Unverifiable => Reason::SignatureKeyUnresolved,
        SignatureEvidence::Unsigned => Reason::SignatureAbsent,
    };
    let source = match dimensions.source.source_type {
        EpisodeSource::Proposal => Reason::ProposedSource,
        _ => Reason::SourceBound,
    };
    let outcome = match dimensions.outcome {
        OutcomeEvidence::Validated => Reason::ReviewedValid,
        OutcomeEvidence::Contradicted => Reason::Contradicted,
        OutcomeEvidence::Unobserved => Reason::OutcomeUnobserved,
    };
    let compatibility = match dimensions.compatibility {
        CompatibilityEvidence::Compatible => Reason::BindingCurrent,
        CompatibilityEvidence::ContractStale => Reason::BindingContractStale,
        CompatibilityEvidence::PolicyStale => Reason::BindingPolicyStale,
        CompatibilityEvidence::Unknown => Reason::BindingUnknown,
    };
    let held = dimensions.lifecycle.hold.is_held();
    let revoked = dimensions.lifecycle.revocation == RevocationEvidence::Revoked;
    let lifecycle = if dimensions.lifecycle.forgotten {
        Reason::Forgotten
    } else if revoked && held {
        Reason::HeldAndRevoked
    } else if revoked {
        Reason::Revoked
    } else if dimensions.lifecycle.open_quarantine.is_some() {
        Reason::Quarantined
    } else if held {
        Reason::Held
    } else if dimensions.lifecycle.superseded_by.is_some() {
        Reason::Superseded
    } else if dimensions.lifecycle.tombstoned {
        Reason::Tombstone
    } else {
        Reason::Live
    };
    vec![
        (EvidenceDimension::Cryptographic, cryptographic),
        (EvidenceDimension::Source, source),
        (EvidenceDimension::Calibration, Reason::NoCalibration),
        (EvidenceDimension::Outcome, outcome),
        (EvidenceDimension::Compatibility, compatibility),
        (EvidenceDimension::Regime, Reason::NoRegime),
        (EvidenceDimension::Lifecycle, lifecycle),
        (EvidenceDimension::Confidence, Reason::MaturityBound),
    ]
}
