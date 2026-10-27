//! Hidden facade over the private complete-episode prototype.
//!
//! # Why this module exists
//!
//! The complete-episode prototype is deliberately private: it is provisional,
//! it has no ABI ratification, and nothing outside this crate may depend on
//! it. This module is the one narrow exception, and it exists only so that
//! this crate's own integration tests can drive the private ledger from the
//! outside, exactly as `crate::v3::episode::cose_prototype` does for the
//! private COSE prototype.
//!
//! It is re-exported as `#[doc(hidden)]`, it is hidden from the
//! documentation, it carries no stability promise, and it is not a ratified
//! profile. Consumers must not depend on it.
//!
//! # What it exposes
//!
//! Opaque handles, never the prototype's own types:
//!
//! - [`PrototypeEpisode`] holds one validated complete episode. Its bytes
//!   are validated on construction, so a handle can only ever name an
//!   episode the complete episode's own validation accepts.
//! - [`PrototypeLedger`] holds one complete-episode ledger. The ledger type
//!   itself stays crate-private.
//!
//! The two content-free value types the ledger answers with,
//! [`CompleteEpisodeReceipt`] and [`CompleteEpisodeStoreError`], are
//! re-exported as they are: a receipt carries no episode content, and an
//! error is a static, content-free variant.

use super::decode::DecodedCompleteEpisode;
use super::ledger::CompleteEpisodeStore;
use super::{CompleteEpisode, MAX_ENCODED_SIZE};
use crate::v3::commitment::CanonicalDecodeLimits;
use crate::v3::episode::{EpisodeSigner, SignatureStatus, SignerKeyId};
use ed25519_dalek::VerifyingKey;
use std::path::{Path, PathBuf};

pub use super::ledger::{CompleteEpisodeReceipt, CompleteEpisodeStoreError};

/// One validated complete episode.
///
/// The handle is opaque: it names an episode and its canonical bytes, and it
/// holds no part of the prototype's typed model.
#[derive(Clone)]
pub struct PrototypeEpisode {
    episode: CompleteEpisode,
    bytes: Vec<u8>,
    digest: [u8; 32],
}

impl std::fmt::Debug for PrototypeEpisode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PrototypeEpisode")
            .field("digest", &encode_hex(&self.digest))
            .finish_non_exhaustive()
    }
}

impl PrototypeEpisode {
    /// Validate canonical complete-episode bytes and hold the episode.
    ///
    /// The bytes must decode through the prototype's own bounded decoder and
    /// pass the complete episode's own validation. Bytes beyond the encoded
    /// size ceiling are refused before they are decoded.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, CompleteEpisodeStoreError> {
        if bytes.len() > MAX_ENCODED_SIZE {
            return Err(CompleteEpisodeStoreError::RecordTooLarge);
        }
        let decoded =
            DecodedCompleteEpisode::from_canonical_bytes(bytes, CanonicalDecodeLimits::default())
                .map_err(|_| CompleteEpisodeStoreError::InvalidEpisode)?;
        let episode = decoded.episode().clone();
        let digest: [u8; 32] = sha256(bytes);
        Ok(Self {
            episode,
            bytes: bytes.to_vec(),
            digest,
        })
    }

    /// SHA-256 over the canonical complete-episode bytes.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// The canonical complete-episode bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn inner(&self) -> &CompleteEpisode {
        &self.episode
    }
}

/// One complete-episode ledger.
///
/// The handle is opaque: the ledger type stays crate-private, and this
/// handle exposes only the ledger's reads, its append, and its paths.
pub struct PrototypeLedger {
    store: CompleteEpisodeStore,
}

impl std::fmt::Debug for PrototypeLedger {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PrototypeLedger")
            .field("records", &self.store.record_count())
            .finish_non_exhaustive()
    }
}

impl PrototypeLedger {
    /// Open and replay-verify a ledger, discarding only an incomplete tail.
    pub fn open(directory: impl AsRef<Path>) -> Result<Self, CompleteEpisodeStoreError> {
        Ok(Self {
            store: CompleteEpisodeStore::open(directory)?,
        })
    }

    /// Open a ledger whose canonical writer signs every record it appends.
    pub fn open_with_signer(
        directory: impl AsRef<Path>,
        signer: EpisodeSigner,
    ) -> Result<Self, CompleteEpisodeStoreError> {
        Ok(Self {
            store: CompleteEpisodeStore::open_with_signer(directory, signer)?,
        })
    }

    /// Open with required signatures from the supplied historical trust keys.
    ///
    /// Every record must have a valid signature from one of these keys, and
    /// the new writer's key must be included. Missing, invalid, and unresolved
    /// signatures are corruption, checked before crash-tail repair. The host
    /// must supply this policy on every open; it is not persisted. These keys
    /// establish historical mathematics only, not current admission authority.
    /// Compatibility `open` and `open_with_signer` remain permissive.
    /// This provisional API has no ABI ratification or stability promise.
    pub fn open_authenticated(
        directory: impl AsRef<Path>,
        signer: EpisodeSigner,
        trusted_keys: &[VerifyingKey],
    ) -> Result<Self, CompleteEpisodeStoreError> {
        Ok(Self {
            store: CompleteEpisodeStore::open_authenticated(directory, signer, trusted_keys)?,
        })
    }

    /// Validate, hash, durably append, and return a content-free receipt.
    pub fn append(
        &mut self,
        episode: &PrototypeEpisode,
    ) -> Result<CompleteEpisodeReceipt, CompleteEpisodeStoreError> {
        self.store.append(episode.inner())
    }

    /// The receipt of every record, in ledger order.
    #[must_use]
    pub fn receipts(&self) -> Vec<CompleteEpisodeReceipt> {
        self.store.receipts()
    }

    /// At most `limit` receipts, in ledger order.
    #[must_use]
    pub fn retrieve(&self, limit: usize) -> Vec<CompleteEpisodeReceipt> {
        self.store.retrieve(limit)
    }

    /// The number of records the ledger holds.
    #[must_use]
    pub fn record_count(&self) -> usize {
        self.store.record_count()
    }

    /// The record with this digest, if the ledger holds one.
    #[must_use]
    pub fn episode(&self, digest: &[u8; 32]) -> Option<PrototypeEpisode> {
        let episode = self.store.get(digest)?;
        let bytes = episode.canonical_bytes().ok()?;
        Some(PrototypeEpisode {
            episode: episode.clone(),
            bytes,
            digest: *digest,
        })
    }

    /// Report the signature state of one appended record.
    ///
    /// `resolve` maps a signer key id to its verifying key; returning `None`
    /// yields [`SignatureStatus::UnknownKey`], never `Invalid`.
    #[must_use]
    pub fn signature_status(
        &self,
        digest: &[u8; 32],
        resolve: impl Fn(&SignerKeyId) -> Option<VerifyingKey>,
    ) -> Option<SignatureStatus> {
        self.store.signature_status(digest, resolve)
    }

    /// Identifier of the key this ledger signs new records with, if any.
    #[must_use]
    pub fn signer_key_id(&self) -> Option<&SignerKeyId> {
        self.store.signer_key_id()
    }

    /// The path of the ledger file.
    #[must_use]
    pub fn ledger_path(&self) -> PathBuf {
        self.store.ledger_path().to_path_buf()
    }
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::Digest as _;
    sha2::Sha256::digest(bytes).into()
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(nibble(byte >> 4));
        text.push(nibble(byte & 0x0f));
    }
    text
}

fn nibble(value: u8) -> char {
    char::from(if value < 10 {
        b'0' + value
    } else {
        b'a' + value - 10
    })
}
