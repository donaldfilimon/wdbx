//! Append-only, content-free canonical episode storage.

mod store;
mod types;

pub use store::{EpisodeStore, EpisodeStoreError};
pub use types::{
    ActorKind, ActorRef, AttributionResult, AuthorizationState, EpisodeEvent, EpisodeReceipt,
    EpisodeSource, EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MediaOutcome, StorePolicy,
    TerminalReason, TerminalStatus, VoiceEvidence, VoiceTransition,
};
