//! Additive v3 WDBX primitives.
//!
//! V3 does not reinterpret or rewrite v2 data. Its first slice is the pure,
//! storage-independent canonical episode commitment profile.

pub mod commitment;
pub mod episode;
pub mod federation;
pub mod offline;
pub mod ranking;
pub mod replication;

pub(crate) mod complete_episode;

/// Hidden facade over the private complete-episode prototype.
///
/// The prototype itself is crate-internal. This re-export exists only so that
/// this crate's own integration tests can exercise it; it is hidden from the
/// documentation, it carries no stability promise, and it is not a ratified
/// profile. The complete-episode ledger it reaches is provisional, has no
/// ABI ratification, no quorum, no federation, and no cross-language
/// verification. Consumers must not depend on it.
#[doc(hidden)]
pub mod complete_episode_prototype {
    pub use super::complete_episode::facade::{
        CompleteEpisodeReceipt, CompleteEpisodeStoreError, PrototypeEpisode, PrototypeLedger,
    };
}
