//! The complete-episode ledger's on-disk record format.
//!
//! # Format
//!
//! One JSON object per line, LF-terminated, in the v3 episode ledger's JSONL
//! precedent. The file is append-only: a record is only ever appended, never
//! rewritten in place.
//!
//! ```text
//! {
//!   "domain": "wdbx-complete-episode-v1",
//!   "sequence": 1,
//!   "previous_digest": null,
//!   "episode_digest": "<64 lowercase hex characters>",
//!   "episode": "<lowercase hex of the canonical complete-episode bytes>",
//!   "signature": { "signer_key_id": "ed25519:<64 hex>", "signature": [64 bytes] }
//! }
//! ```
//!
//! - `domain` is the complete-episode domain separator. It is not the v3
//!   episode domain (`abbey-cbor-episode-v1`) and a line carrying any other
//!   domain is refused.
//! - `sequence` is the one-based position of the record in the ledger.
//! - `previous_digest` is the prior record's `episode_digest`, or `null` for
//!   the first record. It is always written, so a line's shape does not
//!   depend on its position.
//! - `episode_digest` is SHA-256 over the canonical complete-episode bytes
//!   only. It never covers the line, the sequence, the chain, or a signature,
//!   so signing a record does not move its digest.
//! - `episode` is the canonical complete-episode bytes, hex encoded. Hex is
//!   used because the payload is binary and the crate may not take a new
//!   dependency; it doubles the line's size, which the size ceilings below
//!   account for.
//! - `signature` is the detached writer signature, omitted when the record is
//!   unsigned. It is outside the digest, exactly as the v3 episode ledger's
//!   detached signature is.
//!
//! Every member is required except `previous_digest` (nullable, and absent
//! only when a line omits it) and `signature` (omitted when absent). An
//! unknown member is refused: the line must be exactly what this profile
//! describes.
//!
//! # Bounds
//!
//! - [`MAX_LINE_BYTES`] bounds one serialized line.
//! - [`MAX_LEDGER_BYTES`] bounds the whole file.
//! - [`MAX_RECORDS`] bounds the number of records.
//!
//! A line that is not LF-terminated is an incomplete tail: it is discarded
//! when the ledger is opened, and only after the complete prefix has been
//! fully verified. A complete line that is malformed is corruption, and the
//! file is left untouched.

use super::CompleteEpisodeStoreError;
use crate::v3::episode::EpisodeSignature;
use serde::{Deserialize, Serialize};

/// Name of the append-only ledger file beside the writer lock.
pub(super) const LEDGER_FILE: &str = "complete-episodes.v1.jsonl";

/// Name of the writer-lock file beside the ledger.
pub(super) const LOCK_FILE: &str = "complete-episodes.v1.lock";

/// Ceiling on the whole ledger file, in bytes.
pub(super) const MAX_LEDGER_BYTES: u64 = 64 * 1024 * 1024;

/// Ceiling on the number of records in one ledger.
pub(super) const MAX_RECORDS: usize = 65_536;

/// Ceiling on one serialized line: the hex payload plus JSON overhead.
pub(super) const MAX_LINE_BYTES: usize = super::super::MAX_ENCODED_SIZE * 2 + 4_096;

/// One ledger record, exactly as it is persisted.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct LedgerRecord {
    /// Domain separator the record is committed under.
    pub(super) domain: String,
    /// One-based position of the record in the ledger.
    pub(super) sequence: u64,
    /// The prior record's digest, or `None` for the first record.
    #[serde(default)]
    pub(super) previous_digest: Option<String>,
    /// SHA-256 over the canonical complete-episode bytes, hex encoded.
    pub(super) episode_digest: String,
    /// The canonical complete-episode bytes, hex encoded.
    pub(super) episode: String,
    /// The detached writer signature, when the record carries one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) signature: Option<EpisodeSignature>,
}

/// Serialize one record into its persisted line, newline included.
pub(super) fn serialized_line(record: &LedgerRecord) -> Result<Vec<u8>, CompleteEpisodeStoreError> {
    let mut line = serde_json::to_vec(record).map_err(|_| CompleteEpisodeStoreError::Corrupt)?;
    line.push(b'\n');
    if line.len() > MAX_LINE_BYTES {
        return Err(CompleteEpisodeStoreError::RecordTooLarge);
    }
    Ok(line)
}

/// Decode every complete line of the ledger.
///
/// Returns the records and the number of bytes that hold complete lines. A
/// trailing fragment without its newline is an incomplete tail: it is not an
/// error here, and the caller discards it only after the complete prefix has
/// been verified. A complete line that does not parse is corruption.
pub(super) fn decode_lines(
    raw: &[u8],
) -> Result<(Vec<LedgerRecord>, usize), CompleteEpisodeStoreError> {
    let mut records = Vec::new();
    let mut start = 0_usize;
    while let Some(relative_end) = raw[start..].iter().position(|byte| *byte == b'\n') {
        let end = start + relative_end;
        let line = &raw[start..end];
        if line.is_empty() || line.len() + 1 > MAX_LINE_BYTES {
            return Err(CompleteEpisodeStoreError::Corrupt);
        }
        let record =
            serde_json::from_slice(line).map_err(|_| CompleteEpisodeStoreError::Corrupt)?;
        records.push(record);
        if records.len() > MAX_RECORDS {
            return Err(CompleteEpisodeStoreError::LimitExceeded);
        }
        start = end + 1;
    }
    // A trailing fragment without its newline is an incomplete tail, and it
    // is only plausible as one while it could still be a single record. A
    // longer fragment is not a torn append: it is corruption.
    if raw.len() - start > MAX_LINE_BYTES {
        return Err(CompleteEpisodeStoreError::Corrupt);
    }
    Ok((records, start))
}

/// Encode bytes as lowercase hex.
pub(super) fn encode_hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(nibble(byte >> 4));
        text.push(nibble(byte & 0x0f));
    }
    text
}

/// Decode lowercase hex. Uppercase and odd lengths are refused so that one
/// spelling names one byte string.
pub(super) fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let bytes = text.as_bytes();
    let (pairs, remainder) = bytes.as_chunks::<2>();
    debug_assert!(remainder.is_empty(), "an even length has no remainder");
    let mut decoded = Vec::with_capacity(pairs.len());
    for pair in pairs {
        decoded.push((hex_value(pair[0])? << 4) | hex_value(pair[1])?);
    }
    Some(decoded)
}

/// Decode a hex digest into its 32 bytes.
pub(super) fn decode_digest(text: &str) -> Option<[u8; 32]> {
    let bytes = decode_hex(text)?;
    bytes.try_into().ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn nibble(value: u8) -> char {
    char::from(if value < 10 {
        b'0' + value
    } else {
        b'a' + value - 10
    })
}
