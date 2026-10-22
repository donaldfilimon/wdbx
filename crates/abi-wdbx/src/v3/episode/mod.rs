//! Append-only, content-free canonical episode storage.

mod admission;
pub(crate) mod cose;
mod signing;
mod store;
mod types;

pub use admission::{
    AdmissionClass, AdmissionDecision, AdmissionReason, classify, classify_event, classify_write,
};
pub use signing::{
    EpisodeSignature, EpisodeSigner, SIGNER_KEY_ID_PREFIX, SignatureStatus, SignerKeyId,
    check_signature, verify_digest,
};
pub use store::query::{
    AgeEvidence, CalibrationEvidence, CompatibilityEvidence, ConstitutionalQuery,
    ConstitutionalResult, CryptographicEvidence, Disposition, EvidenceDimension,
    EvidenceDimensions, Explanation, HoldEvidence, HoldKinds, KeyResolver, LifecycleEvidence,
    Lineage, MemberScope, MemoryResults, OutcomeEvidence, QueryConstraints, Reason, RegimeEvidence,
    ReputationEvidence, RevocationEvidence, SignatureEvidence, SignerKeyStatus, SourceEvidence,
    UncertaintyEvidence,
};
pub use store::{
    DerivedCopyKind, EpisodeStore, EpisodeStoreError, GcPlan, GcReport, Hold, HoldDecision,
    HoldKind, HoldReceipt, HoldReleaseReceipt, HoldReleaseRequest, HoldRequest, HoldState,
    HoldStatus, ObjectKeyDestructionDecision, ObjectKeyDestructionReceipt,
    ObjectKeyDestructionRequest, ObjectKeyDestructionState, ObjectKeyRef, QuotaAdjustmentDecision,
    QuotaAdjustmentReceipt, QuotaAdjustmentRecord, QuotaAdjustmentRequest, QuotaResetReceipt,
    QuotaState, RedactionDecision, RedactionReceipt, RedactionRequest, RedactionState,
    RetentionState, RevocationDecision, RevocationReceipt, RevocationRequest, RevocationState,
    SupersessionLineage,
};
pub use types::{
    ActorKind, ActorRef, AttributionResult, AuthorizationState, EdgeReason, EpisodeEvent,
    EpisodeReceipt, EpisodeSource, EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MediaOutcome,
    MemoryCandidate, MemoryClass, MemoryEdge, MemoryEdgeKind, MemoryEdgeState, OpenContradiction,
    RetentionClass, RetentionWindows, StorePolicy, TerminalReason, TerminalStatus, VoiceEvidence,
    VoiceTransition,
};

/// Hidden facade over the private COSE_Sign1 prototype.
///
/// The prototype itself is crate-internal. This re-export exists only so that
/// this crate's own integration tests can exercise it; it is hidden from the
/// documentation, it carries no stability promise, and it is not a ratified
/// profile. Consumers must not depend on it.
#[doc(hidden)]
pub mod cose_prototype {
    pub use super::cose::{
        CoseContainer, CoseError, CoseSign1, Integrity, KeyStatus, TrustError, TrustList,
        Verification, decode_container, verify,
    };
}
