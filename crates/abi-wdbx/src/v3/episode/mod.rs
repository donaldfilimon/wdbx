//! Append-only, content-free canonical episode storage.

mod signing;
mod store;
mod types;

pub use signing::{
    EpisodeSignature, EpisodeSigner, SIGNER_KEY_ID_PREFIX, SignatureStatus, SignerKeyId,
    check_signature, verify_digest,
};
pub use store::{EpisodeStore, EpisodeStoreError};
pub use types::{
    ActorKind, ActorRef, AttributionResult, AuthorizationState, EpisodeEvent, EpisodeReceipt,
    EpisodeSource, EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MediaOutcome, MemoryCandidate,
    MemoryClass, RetentionClass, StorePolicy, TerminalReason, TerminalStatus, VoiceEvidence,
    VoiceTransition,
};
