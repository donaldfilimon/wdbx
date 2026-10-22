//! The closed failures of the offline outbox.
//!
//! Every variant is a static label. No variant carries a path, a digest, an
//! operation identifier, a scope name, or any proposal content, so an error
//! can be logged or mapped to a transport status without leaking the work it
//! describes.

/// The closed failures of the offline outbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum OutboxError {
    /// The outbox file could not be read, written, or replaced.
    #[error("offline_outbox_io")]
    Io,
    /// Another writer owns the outbox.
    #[error("offline_outbox_writer_busy")]
    WriterBusy,
    /// A complete line does not parse, or its checksum does not match.
    #[error("offline_outbox_corrupt")]
    Corrupt,
    /// An identifier, collection, counter, or payload is outside its bound.
    #[error("offline_outbox_input_invalid")]
    InvalidInput,
    /// The outbox already holds the greatest number of operations it may.
    ///
    /// This is distinct from [`OutboxError::InvalidInput`]: the proposal is
    /// well formed, and the *queue* is full. A caller can tell a full queue
    /// from a malformed proposal, and it can make room by removing
    /// terminal operations with an authorized caller.
    #[error("offline_outbox_capacity_exhausted")]
    Capacity,
    /// The operation identifier is already in the outbox.
    #[error("offline_outbox_duplicate_operation")]
    DuplicateOperation,
    /// The operation depends on an operation the outbox does not hold.
    #[error("offline_outbox_unknown_dependency")]
    UnknownDependency,
    /// The operation is not in a state that permits the requested transition.
    #[error("offline_outbox_state_invalid")]
    InvalidState,
    /// The operation may not leave the outbox: another operation depends on
    /// it.
    ///
    /// A dependent operation names the operation it depends on. Removing
    /// that operation would leave the dependent with a dependency the outbox
    /// does not hold, and every later drain would defer the dependent for
    /// ever, because no drain could ever satisfy a dependency that is not
    /// there. The removal is refused instead, and the caller resolves the
    /// dependents first.
    ///
    /// The refusal names neither the operation nor the dependent: it is a
    /// static label, exactly like every other variant here.
    #[error("offline_outbox_dependency_present")]
    DependencyPresent,
    /// The authority may not perform this action.
    #[error("offline_outbox_authority_denied")]
    AuthorityDenied,
    /// The asset chunk contradicts a chunk already transferred.
    #[error("offline_outbox_chunk_mismatch")]
    ChunkMismatch,
    /// The transferred chunks do not produce the declared asset digest.
    #[error("offline_outbox_asset_digest_mismatch")]
    AssetDigestMismatch,
}
