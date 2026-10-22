//! Closed failure vocabulary for the quorum-fenced episode shard.
//!
//! Every variant is a static, content-free label. No path, digest, ledger
//! byte, request identifier, or episode content ever reaches an error
//! string: the shard reports *what* failed, never *what* it failed to.

use thiserror::Error;

/// Closed quorum-shard failures.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum ShardError {
    /// A shard file or voter operation failed.
    #[error("quorum_shard_io")]
    Io,
    /// A durable shard file is malformed, truncated, or fails its checksum.
    #[error("quorum_shard_corrupt")]
    Corrupt,
    /// An identifier, membership, or argument is outside the bounded profile.
    #[error("quorum_shard_invalid_input")]
    InvalidInput,
    /// The write names a scope the shard does not own.
    #[error("quorum_shard_scope_mismatch")]
    ScopeMismatch,
    /// The term carried by a writer or an acknowledgement is not current.
    #[error("quorum_shard_stale_term")]
    StaleTerm,
    /// Fewer than a quorum of voters accepted the write.
    #[error("quorum_shard_no_quorum")]
    NoQuorum,
    /// A member holds a different history than the shard acknowledged.
    #[error("quorum_shard_diverged")]
    Diverged,
    /// A voter refused the write.
    #[error("quorum_shard_voter_rejected")]
    VoterRejected,
    /// Two voters acknowledged different digests for one write.
    #[error("quorum_shard_conflict")]
    Conflict,
    /// Another writer holds a voter's ledger lock.
    #[error("quorum_shard_writer_busy")]
    WriterBusy,
    /// A guild budget or the shard's own bounds are exhausted.
    #[error("quorum_shard_budget_exhausted")]
    BudgetExhausted,
}
