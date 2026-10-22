//! The outbox's asset transfer profile.
//!
//! # The profile
//!
//! An asset is transferred in chunks. The outbox holds the asset's
//! *commitment*, its length, its chunk size, and the state of the transfer —
//! never the bytes. The chunk payload is supplied by the caller, one chunk
//! at a time, and the outbox keeps only the chunk's digest.
//!
//! ```text
//! chunk_digest(i, bytes) = SHA-256(bytes)
//! asset_digest(chunks)   = SHA-256( u64le(count)
//!                                    || for each i: u64le(i) || chunk_digest(i) )
//! ```
//!
//! The chunks are numbered by their **chunk index**, not by the order the
//! caller happens to hold them in: [`asset_digest_of`] sorts them by index
//! before it hashes them, so the same chunk set always yields the same
//! digest. A caller can therefore never compute a digest that disagrees with
//! the one the outbox derives, and a correctly transferred asset is never
//! reported as corrupted.
//!
//! The asset digest is therefore computable from the chunk digests alone,
//! which is what lets the outbox verify a completed transfer *without*
//! holding the asset. A resumed transfer never re-requests a chunk it
//! already holds, and a corrupted chunk is detected by the final digest.
//!
//! # What the bytes are for
//!
//! The bytes are the caller's. The outbox hashes them, records the digest,
//! and forgets them. An asset that is never completed leaves no bytes
//! behind, and an asset that is completed leaves a commitment, not a
//! payload.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// The greatest number of chunks one asset transfer may hold.
pub const MAX_ASSET_CHUNKS: u32 = 1024;

/// The digest of one asset chunk.
#[must_use]
pub fn chunk_digest(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

/// The digest of a whole asset, from its chunk digests in index order.
///
/// The digests are numbered by their **position** in the slice, so the slice
/// must already be in chunk-index order. [`AssetState::ordered_digests`]
/// returns them in exactly that order.
#[must_use]
pub fn asset_digest(chunk_digests: &[[u8; 32]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(
        u64::try_from(chunk_digests.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for (index, digest) in chunk_digests.iter().enumerate() {
        hasher.update(u64::try_from(index).unwrap_or(u64::MAX).to_le_bytes());
        hasher.update(digest);
    }
    hasher.finalize().into()
}

/// The digest of a whole asset, from its bytes.
///
/// Each chunk carries its **chunk index**, and the chunks are sorted by that
/// index before they are hashed. The digest therefore does not depend on the
/// order the caller hands the chunks over in: the same chunk set yields the
/// same digest whatever order it arrives in, and it agrees with the digest
/// the outbox derives from the chunk digests it recorded.
///
/// That agreement is the point. A caller that numbered its chunks by the
/// order it happened to hold them could compute a different digest for a
/// correctly transferred asset, and the outbox would report a corruption
/// that never happened.
#[must_use]
pub fn asset_digest_of(chunks: &[(u32, &[u8])]) -> [u8; 32] {
    let mut indexed: Vec<(u32, [u8; 32])> = chunks
        .iter()
        .map(|(index, bytes)| (*index, chunk_digest(bytes)))
        .collect();
    indexed.sort_by_key(|(index, _)| *index);
    let digests: Vec<[u8; 32]> = indexed.iter().map(|(_, digest)| *digest).collect();
    asset_digest(&digests)
}

/// The transferred chunks of one asset operation.
///
/// The map holds the digest of each chunk the caller has supplied, keyed by
/// the chunk's index. The bytes themselves are never held.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AssetState {
    /// The digest of each transferred chunk, by chunk index.
    pub chunks: BTreeMap<u32, [u8; 32]>,
}

impl AssetState {
    /// The number of chunks already transferred.
    #[must_use]
    pub fn completed_chunks(&self) -> u32 {
        u32::try_from(self.chunks.len()).unwrap_or(u32::MAX)
    }

    /// The chunk digests, in index order.
    #[must_use]
    pub fn ordered_digests(&self) -> Vec<[u8; 32]> {
        self.chunks.values().copied().collect()
    }
}

/// The outcome of supplying one asset chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetChunkOutcome {
    /// The chunk the caller supplied.
    pub chunk_index: u32,
    /// Whether the chunk had already been transferred.
    ///
    /// A chunk that is supplied again with the same bytes is accepted and
    /// changes nothing: the transfer is idempotent.
    pub already_transferred: bool,
    /// The number of bytes the chunk carries.
    pub bytes: u64,
}

/// The number of chunks an asset of this length needs.
pub(crate) fn chunk_count(length: u64, chunk_size: u64) -> u32 {
    if length == 0 || chunk_size == 0 {
        return 0;
    }
    let count = length.div_ceil(chunk_size);
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// The number of bytes one chunk of an asset carries.
pub(crate) fn chunk_length(length: u64, chunk_size: u64, index: u32) -> u64 {
    if chunk_size == 0 {
        return 0;
    }
    let offset = u64::from(index).saturating_mul(chunk_size);
    if offset >= length {
        return 0;
    }
    length.saturating_sub(offset).min(chunk_size)
}
