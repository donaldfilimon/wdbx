//! The studio's declared losses.
//!
//! A consumer of the canonical ledger may expose a lossy projection, but it
//! must declare what it drops. The studio declares here, as a typed and
//! inspectable value, exactly what its projection does not carry, and the
//! browser edition ships its own declaration beside its own code.

use crate::{Result, error};
use serde::{Deserialize, Serialize};

/// The browser edition's own declaration, kept beside the browser code.
///
/// The browser edition is a separate program from the native one, so its
/// declaration lives with it and is read here. The native gate reads the same
/// file, so the two editions cannot drift apart silently.
const BROWSER_MANIFEST: &str = include_str!("../../../../lib/specimen/canonical-loss.json");

/// The closed vocabulary of what a studio projection can drop.
///
/// The vocabulary is closed so a declaration names a known loss: a consumer
/// cannot invent a loss to explain away a defect, and a reader can compare
/// two declarations field by field.
///
/// The two editions share this vocabulary: the browser edition's manifest is
/// read into these ids, so an id it declares is an id the native edition
/// knows. The browser edition declares a *subset* of it — it drops the whole
/// ledger — and an id the browser edition does not declare is still a known
/// loss, not an unknown one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LossId {
    /// The browser edition holds no canonical ledger at all.
    BrowserEdition,
    /// The calibration and hardware-health evidence the ledger does not hold.
    CalibrationEvidence,
    /// The canonical episode ledger and every answer only it can give.
    CanonicalLedger,
    /// The resolution from a content commitment to the content bytes.
    ContentResolution,
    /// A content blob the ledger committed to and the studio no longer holds.
    MissingContent,
    /// The observed outcome of a record.
    OutcomeObservation,
    /// The digest that binds stored content to a canonical episode.
    PayloadCommitment,
    /// The content bytes a candidate commits to.
    PayloadContent,
    /// The age of a record.
    RecordAge,
    /// The uncertainty a record carries.
    RecordedUncertainty,
    /// The task-regime and safety-constraint evidence the ledger does not hold.
    RegimeEvidence,
    /// Quorum replication, federation, and cross-language verification.
    ReplicationScope,
    /// The detached episode signature and the key it names.
    SignatureEvidence,
    /// The detached signature on the episodes the studio itself wrote.
    UnsignedEpisodes,
    /// The node an erasure removed, and the instant it happened.
    DeletionFacts,
    /// The reputation the ledger holds for a recording identity.
    SourceReputation,
    /// The accepted disposition a proposed source never carries.
    ProposedSource,
}

impl LossId {
    /// The stable label of the loss.
    pub fn label(self) -> &'static str {
        match self {
            Self::BrowserEdition => "browser_edition",
            Self::CalibrationEvidence => "calibration_evidence",
            Self::CanonicalLedger => "canonical_ledger",
            Self::ContentResolution => "content_resolution",
            Self::MissingContent => "missing_content",
            Self::OutcomeObservation => "outcome_observation",
            Self::PayloadCommitment => "payload_commitment",
            Self::PayloadContent => "payload_content",
            Self::RecordAge => "record_age",
            Self::RecordedUncertainty => "recorded_uncertainty",
            Self::RegimeEvidence => "regime_evidence",
            Self::ReplicationScope => "replication_scope",
            Self::SignatureEvidence => "signature_evidence",
            Self::UnsignedEpisodes => "unsigned_episodes",
            Self::DeletionFacts => "deletion_facts",
            Self::SourceReputation => "source_reputation",
            Self::ProposedSource => "proposed_source",
        }
    }
}

/// One declared loss.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LossDeclaration {
    /// Which loss this is.
    pub id: LossId,
    /// What the projection does not carry.
    pub dropped: String,
    /// Why the projection may leave it out without inventing evidence.
    pub reason: String,
}

/// What one edition's projection drops relative to the canonical evidence.
///
/// The manifest is a value, not a note: it is typed, it is serializable, and
/// a caller can read it and show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LossManifest {
    /// The edition the manifest describes.
    pub edition: String,
    /// The declared losses, in a stable order.
    pub declarations: Vec<LossDeclaration>,
}

impl LossManifest {
    /// The native edition's declared losses.
    ///
    /// Every entry names a thing the studio's projection does not carry, and
    /// says why leaving it out is honest rather than a hidden defect.
    pub fn native() -> Self {
        Self {
            edition: "native".into(),
            declarations: vec![
                LossDeclaration {
                    id: LossId::BrowserEdition,
                    dropped: "the browser edition holds no canonical episode ledger at all, so it drops canonical_ledger, payload_commitment, signature_evidence, and replication_scope.".into(),
                    reason: "The browser edition is a lossy projection of the native-owned ledger. Its own declaration is lib/specimen/canonical-loss.json, and the native manifest names exactly what that edition drops.".into(),
                },
                LossDeclaration {
                    id: LossId::PayloadContent,
                    dropped: "the specimen bytes and the proposal bytes a candidate commits to.".into(),
                    reason: "The episode ledger commits to a payload; it never stores one. The content stays in the studio's own store, which owns it. The proposed write carries the content digest and the byte count; the constitutional projection carries the episode digest and neither of them.".into(),
                },
                LossDeclaration {
                    id: LossId::PayloadCommitment,
                    dropped: "the payload commitment of a candidate and the byte count the substrate charges for it.".into(),
                    reason: "The constitutional projection carries the episode digest, which commits to the payload and to its byte count but exposes neither of them. Both live only in the proposed write, so a reader of the projection cannot name them.".into(),
                },
                LossDeclaration {
                    id: LossId::ContentResolution,
                    dropped: "the resolution from a canonical episode to the content it committed to.".into(),
                    reason: "The episode ledger commits to a payload and exposes no read API that returns a committed record's payload commitment. The studio resolves the commitment through its own journal, a derived sidecar beside its own snapshot, and the rebuild checks every content blob against the commitment it is filed under.".into(),
                },
                LossDeclaration {
                    id: LossId::MissingContent,
                    dropped: "a content blob the ledger committed to and the studio no longer holds.".into(),
                    reason: "The content lives in the studio's own store and the studio owns it. When a blob is gone the rebuild reports a declared gap and returns no document at all, never a partial one that would read as the whole state.".into(),
                },
                LossDeclaration {
                    id: LossId::DeletionFacts,
                    dropped: "which studio node an erasure removed, and the instant it happened.".into(),
                    reason: "The ledger's tombstone names an episode digest; it holds no node and no timestamp. The studio's journal carries the pairing, and the studio's agreement check holds that pairing against the ledger's own answer.".into(),
                },
                LossDeclaration {
                    id: LossId::UnsignedEpisodes,
                    dropped: "the detached signature on every episode the studio itself wrote, and the key status behind it.".into(),
                    reason: "The studio opens the canonical ledger without a signer, so the episodes it writes carry no signature and the cryptographic dimension reads Unsigned. The studio's own journal therefore attributes an episode to the substrate's admission, not to a key, and the rebuild verifies content by digest rather than by signature.".into(),
                },
                LossDeclaration {
                    id: LossId::SignatureEvidence,
                    dropped: "the detached episode signature and the status of the key it names.".into(),
                    reason: "The studio opens the episode store without a signer: episode key custody is not implemented here, so the cryptographic dimension reads Unsigned for every episode rather than pretending a signature exists.".into(),
                },
                LossDeclaration {
                    id: LossId::CalibrationEvidence,
                    dropped: "sensor and model calibration, and hardware health.".into(),
                    reason: "The episode ledger holds no calibration record, so the calibration dimension reads Unknown. The projection reports that state instead of inventing a measurement.".into(),
                },
                LossDeclaration {
                    id: LossId::RegimeEvidence,
                    dropped: "task regime and safety-constraint compatibility.".into(),
                    reason: "The episode ledger holds no task-regime field, so the regime dimension reads Unknown.".into(),
                },
                LossDeclaration {
                    id: LossId::SourceReputation,
                    dropped: "the reputation the ledger holds for a recording identity.".into(),
                    reason: "The episode ledger records no reputation signal, so the source dimension's reputation reads Unknown.".into(),
                },
                LossDeclaration {
                    id: LossId::RecordedUncertainty,
                    dropped: "the uncertainty or interval a record carries.".into(),
                    reason: "The episode ledger records no uncertainty value, so the confidence dimension's uncertainty reads Unknown.".into(),
                },
                LossDeclaration {
                    id: LossId::RecordAge,
                    dropped: "the age of a record.".into(),
                    reason: "The episode ledger records no timestamp, so the lifecycle dimension's age reads Unknown.".into(),
                },
                LossDeclaration {
                    id: LossId::OutcomeObservation,
                    dropped: "an observed outcome for a record.".into(),
                    reason: "The studio records no outcome edge, so the outcome dimension reads Unobserved until a review resolves one.".into(),
                },
                LossDeclaration {
                    id: LossId::ProposedSource,
                    dropped: "the accepted disposition a learned proposal would carry if the studio had observed it instead of proposing it.".into(),
                    reason: "The studio records a learned proposal as a proposal, and the substrate treats a proposed source as Uncertain by design. The projection reports that disposition instead of relabelling the episode, so a learned fact reads Uncertain and never Accepted.".into(),
                },
                LossDeclaration {
                    id: LossId::ReplicationScope,
                    dropped: "quorum attestations, federation grants, and cross-language verification.".into(),
                    reason: "The studio's ledger is one local writer. The substrate's quorum shard and its federation module are not wired into the studio, so nothing stands behind these episodes but the local ledger that holds them.".into(),
                },
            ],
        }
    }

    /// The browser edition's declared losses, read from the manifest the
    /// browser edition ships beside its own code.
    pub fn browser() -> Result<Self> {
        serde_json::from_str(BROWSER_MANIFEST).map_err(|e| {
            error(
                "MalformedSave",
                format!("The browser loss manifest is unreadable: {e}"),
            )
        })
    }

    /// The declaration for one loss, if the manifest names it.
    pub fn declaration(&self, id: LossId) -> Option<&LossDeclaration> {
        self.declarations.iter().find(|item| item.id == id)
    }
}
