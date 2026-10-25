//! Durable append-only ledger for complete episodes.
//!
//! # What this is
//!
//! A single-writer, append-only, durable store of *complete episodes*: the
//! richer provisional record this crate's private prototype defines, with its
//! identity, time, origin, context, decision, outcome, authority, evidence,
//! lifecycle, and provenance groups.
//!
//! # Domain separation
//!
//! A complete episode is **not** a v3 episode candidate. The two live in
//! different domains with different commitments:
//!
//! - The v3 episode ledger (`crate::v3::episode::EpisodeStore`) stores
//!   `EpisodeEvent`s under the `abbey-cbor-episode-v1` commitment profile,
//!   with its own file family (`episodes.v1.jsonl` and its sidecars).
//! - This ledger stores complete episodes under
//!   [`COMPLETE_EPISODE_DOMAIN`], with its own file family
//!   (`complete-episodes.v1.jsonl` and `complete-episodes.v1.lock`).
//!
//! The digests are therefore different values over different bytes, and this
//! ledger never writes a v3 candidate, never reads the v3 ledger, and never
//! presents a complete episode as a v3 episode. A complete episode is a
//! *richer record*, not a second canonical episode for the same content: a
//! caller that wants a v3 candidate writes a v3 candidate through the v3
//! store, and the two stores answer for different domains. A caller that
//! wants to relate the two must say so itself; this ledger does not invent
//! that relation.
//!
//! # Record layout
//!
//! One record is one complete episode:
//!
//! - the **canonical complete-episode bytes**, exactly as
//!   [`CompleteEpisode::canonical_bytes`] produces them;
//! - the **digest**, SHA-256 over those bytes and nothing else;
//! - the **detached signature**, when the store signs, beside the record and
//!   outside the digest.
//!
//! Because the digest covers the canonical bytes only, signing a record never
//! moves its digest, and an unsigned record is a complete record.
//!
//! # Chain semantics
//!
//! The v3 episode ledger chains `previous_digest` *within one operation*: a
//! proposal, its approval, and its execution are one chain. A complete
//! episode is a single self-contained record, so this ledger chains *across
//! the ledger*: record *n* names record *n-1*'s digest, and the first record
//! names none.
//!
//! - `sequence` is one-based and increases by exactly one per record.
//! - `previous_digest` is the prior record's `episode_digest`, or `null` for
//!   the first record.
//! - A gap in the sequence, a `previous_digest` that names another record, a
//!   missing link, or a duplicate digest is corruption.
//!
//! One ledger file holds one chain. The chain is a ledger-level integrity
//! mechanism, not a scope mechanism: the scope of every episode is inside
//! its own committed bytes.
//!
//! # Replay validation
//!
//! Opening a ledger re-validates every complete line, in order:
//!
//! 1. the line parses as exactly this profile's record;
//! 2. `domain` is [`COMPLETE_EPISODE_DOMAIN`];
//! 3. `sequence` is the expected position;
//! 4. `previous_digest` names the prior record's digest;
//! 5. the canonical bytes are within the size ceiling;
//! 6. the digest re-derives from the canonical bytes;
//! 7. the canonical bytes decode through
//!    [`DecodedCompleteEpisode::from_canonical_bytes`] and pass the
//!    complete episode's own validation, which is the same validation the
//!    append path runs;
//! 8. a signature that claims this store's own key verifies over the digest;
//! 9. the digest is not already in the ledger.
//!
//! A line that fails any of these is [`CompleteEpisodeStoreError::Corrupt`].
//! Nothing is repaired and nothing is dropped: a mutated byte is refused, not
//! silently accepted.
//!
//! An *incomplete* final line (one without its newline) is the one thing the
//! store discards, and only after the complete prefix has been fully
//! verified: that is the crash tail of an interrupted append, exactly as the
//! v3 episode ledger treats it.
//!
//! # Signing
//!
//! [`CompleteEpisodeStore::open_with_signer`] signs every record it appends,
//! with the same detached Ed25519 signature over the same 32-byte digest
//! that the v3 episode ledger uses, and a key id derived from the key rather
//! than chosen. Unsigned records stay legal and are reported as
//! [`SignatureStatus::Unsigned`], never as invalid. A record signed under
//! another key is that key's to judge: the store leaves it alone and reports
//! it through [`CompleteEpisodeStore::signature_status`].
//!
//! Use a key that signs nothing else. The v2 segment-signing key
//! (`crate::v2::read_signing_key_file`) and the v3 episode-signing key must
//! both stay distinct from this ledger's key, so that a signature made in one
//! domain can never be presented in another.
//!
//! [`CompleteEpisodeStore::open_authenticated`] instead requires a valid
//! signature from a supplied historical key for every record. The host must
//! select that mode on every open; the policy is not persisted in the ledger.
//! This detects signature removal and unresolved-key substitution in that mode,
//! but does not prove current key authority, prevent whole-ledger rollback,
//! or turn the compatibility opens into authenticated readers.
//!
//! # Not implemented
//!
//! This is a private, provisional, crate-internal prototype. It has:
//!
//! - **no ABI ratification**: no ratified wire contract, no published
//!   specification, and no consumer may depend on these bytes;
//! - **no quorum or fencing**: the v3 quorum shard does not cover this
//!   ledger, and there is no term, no certification, and no replicated
//!   acceptance here;
//! - **no federation**: nothing is exported to, or imported from, another
//!   organization;
//! - **no cross-language verification**: the ledger format has no second
//!   implementation in another language, so nothing here is evidence that two
//!   languages read these bytes the same way;
//! - **no deletion, retention, or hold lifecycle**: a complete episode's
//!   retention intent and holds are committed *inside* the record, but
//!   nothing in this ledger expires, reclaims, redacts, or holds a record;
//! - **no reader projection**: this ledger stores and verifies records. It
//!   does not answer queries, rank results, or project the record into a
//!   consumer's shape;
//! - **no tamper-proofing without a signature**: an unsigned ledger detects
//!   accidental mutation and inconsistent edits, but an actor that rewrites
//!   the whole file can also recompute every digest. A signed record binds
//!   its digest to a key.
//!
//! # Writer exclusion
//!
//! One writer at a time, enforced by `complete-episodes.v1.lock`, which the
//! store holds for its lifetime. Contention is retried for up to 50 ms to
//! tolerate transient inherited descriptors; persistent contention is
//! [`CompleteEpisodeStoreError::WriterBusy`], never permission to open
//! without the lock.

use super::decode::DecodedCompleteEpisode;
use super::{COMPLETE_EPISODE_DOMAIN, CompleteEpisode, MAX_ENCODED_SIZE};
use crate::v3::commitment::CanonicalDecodeLimits;
use crate::v3::episode::{
    EpisodeSignature, EpisodeSigner, SignatureStatus, SignerKeyId, check_signature,
};
use ed25519_dalek::VerifyingKey;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

mod filesystem;
mod format;

use filesystem::{
    acquire_writer_lock, owner_file, prepare_directory, read_bounded_file,
    reject_symlink_if_present, sync_directory,
};
use format::{
    LEDGER_FILE, LOCK_FILE, MAX_LEDGER_BYTES, MAX_RECORDS, decode_digest, decode_hex, decode_lines,
    encode_hex, serialized_line,
};

/// Closed complete-episode ledger failures.
///
/// Every variant is content-free: no raw bytes, no path, and no digest is
/// ever rendered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CompleteEpisodeStoreError {
    /// A filesystem operation failed.
    #[error("complete_episode_store_io")]
    Io,
    /// Another writer holds the ledger's writer lock.
    #[error("complete_episode_store_writer_busy")]
    WriterBusy,
    /// The ledger holds a malformed, mutated, or inconsistent record.
    #[error("complete_episode_store_corrupt")]
    Corrupt,
    /// The episode failed the complete episode's own validation.
    #[error("complete_episode_store_invalid_episode")]
    InvalidEpisode,
    /// The record exceeds the per-record size ceiling.
    #[error("complete_episode_store_record_too_large")]
    RecordTooLarge,
    /// The ledger exceeds its record-count or byte ceiling.
    #[error("complete_episode_store_limit_exceeded")]
    LimitExceeded,
    /// The episode is already in the ledger.
    #[error("complete_episode_store_replay")]
    Replay,
}

/// Content-free receipt for one appended complete episode.
///
/// The receipt names the record's position, its digest, the digest it chains
/// to, the domain it is committed under, and the key its detached signature
/// was made under. It never carries episode content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CompleteEpisodeReceipt {
    /// One-based position of the record in the ledger.
    pub sequence: u64,
    /// SHA-256 over the canonical complete-episode bytes, hex encoded.
    pub episode_digest: String,
    /// The prior record's digest, hex encoded, when there is one.
    pub previous_digest: Option<String>,
    /// The domain separator the record is committed under.
    pub domain: &'static str,
    /// The key the detached signature was made under, when there is one.
    pub signer_key_id: Option<SignerKeyId>,
}

/// One replayed record.
#[derive(Clone, Debug)]
struct LedgerEntry {
    sequence: u64,
    digest: [u8; 32],
    previous_digest: Option<[u8; 32]>,
    episode: CompleteEpisode,
    signature: Option<EpisodeSignature>,
}

/// Durable, single-writer, append-only complete-episode ledger.
///
/// Appends synchronize ledger data. Unix initialization also synchronizes
/// directory entries; other platforms retain file-only synchronization and
/// provide no directory-entry crash-durability guarantee.
pub(crate) struct CompleteEpisodeStore {
    /// Held for the store's lifetime: the writer lock.
    _writer_lock: File,
    /// The append-only ledger file.
    ledger: File,
    /// The path of the ledger file.
    ledger_path: PathBuf,
    /// The replayed records, in ledger order.
    entries: Vec<LedgerEntry>,
    /// The canonical writer's signing key, when the store signs.
    signer: Option<EpisodeSigner>,
    /// Set when a write failed: the ledger is no longer trusted for appends.
    poisoned: bool,
}

impl std::fmt::Debug for CompleteEpisodeStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CompleteEpisodeStore")
            .field("records", &self.entries.len())
            .field(
                "signer_key_id",
                &self.signer.as_ref().map(EpisodeSigner::key_id),
            )
            .finish_non_exhaustive()
    }
}

impl CompleteEpisodeStore {
    /// Open and replay-verify a ledger, discarding only an incomplete tail.
    ///
    /// Writer-lock contention is retried for up to 50 ms to tolerate transient
    /// inherited descriptors; persistent contention returns
    /// [`CompleteEpisodeStoreError::WriterBusy`].
    ///
    /// A store opened this way appends unsigned records. Existing signed
    /// records remain readable; use [`Self::signature_status`] to check them.
    pub(crate) fn open(directory: impl AsRef<Path>) -> Result<Self, CompleteEpisodeStoreError> {
        Self::open_inner(directory.as_ref(), None, None)
    }

    /// Open a ledger whose canonical writer signs every record it appends.
    ///
    /// Replay additionally verifies every existing record that claims this
    /// signer's key id; one that fails is corruption, exactly as a mutated
    /// digest is. Records signed under other keys, and unsigned records, are
    /// accepted and left for [`Self::signature_status`] to report.
    pub(crate) fn open_with_signer(
        directory: impl AsRef<Path>,
        signer: EpisodeSigner,
    ) -> Result<Self, CompleteEpisodeStoreError> {
        Self::open_inner(directory.as_ref(), Some(signer), None)
    }

    /// Require every historical signature to verify against the supplied keys.
    ///
    /// The host supplies this policy on every open. Keys are historical
    /// verification keys, not proof of current authority or key custody.
    /// The current signer must be included. This refuses unsigned history
    /// and unresolved keys before crash-tail repair and changes no record bytes.
    pub(crate) fn open_authenticated(
        directory: impl AsRef<Path>,
        signer: EpisodeSigner,
        trusted_keys: &[VerifyingKey],
    ) -> Result<Self, CompleteEpisodeStoreError> {
        if !trusted_keys.contains(&signer.verifying_key()) {
            return Err(CompleteEpisodeStoreError::Corrupt);
        }
        Self::open_inner(directory.as_ref(), Some(signer), Some(trusted_keys))
    }

    fn open_inner(
        directory: &Path,
        signer: Option<EpisodeSigner>,
        trusted_keys: Option<&[VerifyingKey]>,
    ) -> Result<Self, CompleteEpisodeStoreError> {
        let directory = prepare_directory(directory)?;
        let ledger_path = directory.join(LEDGER_FILE);
        let lock_path = directory.join(LOCK_FILE);
        reject_symlink_if_present(&lock_path)?;
        reject_symlink_if_present(&ledger_path)?;

        let writer_lock = owner_file(&lock_path)?;
        acquire_writer_lock(&writer_lock)?;
        let raw = read_bounded_file(&ledger_path, MAX_LEDGER_BYTES)?;
        let (records, valid_bytes) = decode_lines(&raw)?;
        let entries = replay(records, signer.as_ref(), trusted_keys)?;
        // A failed replay must not repair the file. Only a fully verified
        // prefix permits discarding an incomplete final line.
        if valid_bytes != raw.len() {
            let repair = OpenOptions::new()
                .write(true)
                .open(&ledger_path)
                .map_err(|_| CompleteEpisodeStoreError::Io)?;
            repair
                .set_len(u64::try_from(valid_bytes).map_err(|_| CompleteEpisodeStoreError::Io)?)
                .map_err(|_| CompleteEpisodeStoreError::Io)?;
            repair
                .sync_data()
                .map_err(|_| CompleteEpisodeStoreError::Io)?;
        }
        let ledger = owner_file(&ledger_path)?;
        sync_directory(&directory)?;
        Ok(Self {
            _writer_lock: writer_lock,
            ledger,
            ledger_path,
            entries,
            signer,
            poisoned: false,
        })
    }

    /// Identifier of the key this store signs new records with, if any.
    pub(crate) fn signer_key_id(&self) -> Option<&SignerKeyId> {
        self.signer.as_ref().map(EpisodeSigner::key_id)
    }

    /// The path of the ledger file.
    pub(crate) fn ledger_path(&self) -> &Path {
        &self.ledger_path
    }

    /// The number of records the ledger holds.
    pub(crate) fn record_count(&self) -> usize {
        self.entries.len()
    }

    /// Every record, in ledger order.
    pub(crate) fn records(&self) -> Vec<&CompleteEpisode> {
        self.entries.iter().map(|entry| &entry.episode).collect()
    }

    /// The receipt of every record, in ledger order.
    pub(crate) fn receipts(&self) -> Vec<CompleteEpisodeReceipt> {
        self.entries.iter().map(receipt).collect()
    }

    /// At most `limit` receipts, in ledger order.
    ///
    /// The window is a read convenience only. Verification uses
    /// [`Self::find`], which scans the whole ledger.
    pub(crate) fn retrieve(&self, limit: usize) -> Vec<CompleteEpisodeReceipt> {
        self.entries.iter().take(limit).map(receipt).collect()
    }

    /// The record with this digest, if the ledger holds one.
    pub(crate) fn get(&self, digest: &[u8; 32]) -> Option<&CompleteEpisode> {
        self.entries
            .iter()
            .find(|entry| &entry.digest == digest)
            .map(|entry| &entry.episode)
    }

    /// The receipt of the record with this digest, if the ledger holds one.
    pub(crate) fn find(&self, digest: &[u8; 32]) -> Option<CompleteEpisodeReceipt> {
        self.entries
            .iter()
            .find(|entry| &entry.digest == digest)
            .map(receipt)
    }

    /// Report the signature state of one appended record.
    ///
    /// `Ok(None)` means the digest is not in this ledger. `resolve` maps a
    /// signer key id to its verifying key; returning `None` yields
    /// [`SignatureStatus::UnknownKey`], never `Invalid`.
    pub(crate) fn signature_status(
        &self,
        digest: &[u8; 32],
        resolve: impl Fn(&SignerKeyId) -> Option<ed25519_dalek::VerifyingKey>,
    ) -> Option<SignatureStatus> {
        self.entries
            .iter()
            .find(|entry| &entry.digest == digest)
            .map(|entry| check_signature(&entry.digest, entry.signature.as_ref(), &resolve))
    }

    /// Validate, hash, durably append, and return a content-free receipt.
    ///
    /// The append path runs the same validation the replay path runs, so a
    /// write this accepts is a write a reopen accepts.
    pub(crate) fn append(
        &mut self,
        episode: &CompleteEpisode,
    ) -> Result<CompleteEpisodeReceipt, CompleteEpisodeStoreError> {
        if self.poisoned {
            return Err(CompleteEpisodeStoreError::Io);
        }
        let bytes = episode
            .canonical_bytes()
            .map_err(|_| CompleteEpisodeStoreError::InvalidEpisode)?;
        if bytes.len() > MAX_ENCODED_SIZE {
            return Err(CompleteEpisodeStoreError::RecordTooLarge);
        }
        // Re-derive the digest from the bytes and re-run the complete
        // episode's own validation, so the append path and the replay path
        // reach the same verdict about the same bytes.
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        if digest
            != episode
                .digest()
                .map_err(|_| CompleteEpisodeStoreError::InvalidEpisode)?
        {
            return Err(CompleteEpisodeStoreError::InvalidEpisode);
        }
        DecodedCompleteEpisode::from_canonical_bytes(&bytes, CanonicalDecodeLimits::default())
            .map_err(|_| CompleteEpisodeStoreError::InvalidEpisode)?;
        if self.entries.iter().any(|entry| entry.digest == digest) {
            return Err(CompleteEpisodeStoreError::Replay);
        }
        if self.entries.len() >= MAX_RECORDS {
            return Err(CompleteEpisodeStoreError::LimitExceeded);
        }
        let sequence = u64::try_from(self.entries.len())
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(CompleteEpisodeStoreError::LimitExceeded)?;
        let previous_digest = self.entries.last().map(|entry| entry.digest);
        // The signature is detached: it is not covered by the digest, so
        // signing a record never moves its commitment.
        let signature = self
            .signer
            .as_ref()
            .map(|signer| signer.sign_digest(&digest));
        let record = format::LedgerRecord {
            domain: COMPLETE_EPISODE_DOMAIN.to_owned(),
            sequence,
            previous_digest: previous_digest.map(|digest| encode_hex(&digest)),
            episode_digest: encode_hex(&digest),
            episode: encode_hex(&bytes),
            signature: signature.clone(),
        };
        let line = serialized_line(&record)?;
        let current = self
            .ledger
            .metadata()
            .map_err(|_| CompleteEpisodeStoreError::Io)?
            .len();
        let projected = current
            .checked_add(u64::try_from(line.len()).map_err(|_| CompleteEpisodeStoreError::Io)?)
            .ok_or(CompleteEpisodeStoreError::LimitExceeded)?;
        if projected > MAX_LEDGER_BYTES {
            return Err(CompleteEpisodeStoreError::LimitExceeded);
        }
        if self.ledger.write_all(&line).is_err() || self.ledger.sync_data().is_err() {
            self.poisoned = true;
            return Err(CompleteEpisodeStoreError::Io);
        }
        let entry = LedgerEntry {
            sequence,
            digest,
            previous_digest,
            episode: episode.clone(),
            signature,
        };
        let receipt = receipt(&entry);
        self.entries.push(entry);
        Ok(receipt)
    }
}

/// The content-free receipt of one record.
fn receipt(entry: &LedgerEntry) -> CompleteEpisodeReceipt {
    CompleteEpisodeReceipt {
        sequence: entry.sequence,
        episode_digest: encode_hex(&entry.digest),
        previous_digest: entry.previous_digest.map(|digest| encode_hex(&digest)),
        domain: COMPLETE_EPISODE_DOMAIN,
        signer_key_id: entry
            .signature
            .as_ref()
            .map(|signature| signature.signer_key_id.clone()),
    }
}

/// Replay every record of a ledger, in order.
///
/// Every rule here is the same rule the append path applies, so a ledger that
/// replays is a ledger this build would have written.
fn replay(
    records: Vec<format::LedgerRecord>,
    signer: Option<&EpisodeSigner>,
    trusted_keys: Option<&[VerifyingKey]>,
) -> Result<Vec<LedgerEntry>, CompleteEpisodeStoreError> {
    let mut entries: Vec<LedgerEntry> = Vec::with_capacity(records.len());
    let mut digests: BTreeSet<[u8; 32]> = BTreeSet::new();
    let mut previous: Option<[u8; 32]> = None;
    for (index, record) in records.into_iter().enumerate() {
        let format::LedgerRecord {
            domain,
            sequence,
            previous_digest,
            episode_digest,
            episode,
            signature,
        } = record;
        if domain != COMPLETE_EPISODE_DOMAIN {
            return Err(CompleteEpisodeStoreError::Corrupt);
        }
        let expected = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(CompleteEpisodeStoreError::Corrupt)?;
        if sequence != expected {
            return Err(CompleteEpisodeStoreError::Corrupt);
        }
        match (previous_digest, previous) {
            (None, None) => {}
            (Some(text), Some(prior)) => {
                if decode_digest(&text).ok_or(CompleteEpisodeStoreError::Corrupt)? != prior {
                    return Err(CompleteEpisodeStoreError::Corrupt);
                }
            }
            _ => return Err(CompleteEpisodeStoreError::Corrupt),
        }
        let bytes = decode_hex(&episode).ok_or(CompleteEpisodeStoreError::Corrupt)?;
        if bytes.len() > MAX_ENCODED_SIZE {
            return Err(CompleteEpisodeStoreError::RecordTooLarge);
        }
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        if episode_digest != encode_hex(&digest) {
            return Err(CompleteEpisodeStoreError::Corrupt);
        }
        // The complete episode's own validation runs here, exactly as it ran
        // when the record was written: a record whose typed layer is invalid
        // is refused, never accepted.
        let decoded =
            DecodedCompleteEpisode::from_canonical_bytes(&bytes, CanonicalDecodeLimits::default())
                .map_err(|_| CompleteEpisodeStoreError::Corrupt)?;
        if let Some(keys) = trusted_keys {
            let status = check_signature(&digest, signature.as_ref(), |id| {
                keys.iter()
                    .find(|key| &SignerKeyId::for_key(key) == id)
                    .copied()
            });
            if !matches!(status, SignatureStatus::Valid(_)) {
                return Err(CompleteEpisodeStoreError::Corrupt);
            }
        } else if let Some(signer) = signer {
            validate_own_signature(&digest, signature.as_ref(), signer)?;
        }
        if !digests.insert(digest) {
            return Err(CompleteEpisodeStoreError::Corrupt);
        }
        entries.push(LedgerEntry {
            sequence,
            digest,
            previous_digest: previous,
            episode: decoded.episode().clone(),
            signature,
        });
        previous = Some(digest);
    }
    Ok(entries)
}

/// Replay check for records claiming the opening writer's own key.
///
/// A record signed under another key is that key's to judge, and is left
/// alone.
fn validate_own_signature(
    digest: &[u8; 32],
    signature: Option<&EpisodeSignature>,
    signer: &EpisodeSigner,
) -> Result<(), CompleteEpisodeStoreError> {
    let Some(signature) = signature else {
        return Ok(());
    };
    if &signature.signer_key_id != signer.key_id() {
        return Ok(());
    }
    let key = signer.verifying_key();
    match check_signature(digest, Some(signature), |_| Some(key)) {
        SignatureStatus::Valid(_) => Ok(()),
        _ => Err(CompleteEpisodeStoreError::Corrupt),
    }
}
