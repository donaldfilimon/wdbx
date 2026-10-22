//! The federation canonical encoding.
//!
//! The federation slice needs one thing the v3 episode ledger deliberately
//! does not have: a *portable* encoding that two independent organizations
//! can agree on byte-for-byte without sharing a canonical episode writer.
//!
//! This module defines that encoding. It is a length-prefixed, domain-tagged
//! sequence of fields, hashed with SHA-256:
//!
//! ```text
//! digest = SHA-256(domain || field1 || field2 || ...)
//! ```
//!
//! Every variable-length field is prefixed with its length as a big-endian
//! `u64`, so no two distinct field sequences can encode to the same bytes.
//! Every domain is a distinct literal, so a record digest can never be
//! confused with a manifest digest, a bundle digest, or a v3 episode digest.
//!
//! # This is not the v3 commitment domain
//!
//! The v3 episode commitment (`abbey-cbor-episode-v1`) is frozen and is
//! mirrored byte-for-byte by a second-language witness. This encoding is a
//! *separate* domain with its own literals. It never produces a value that
//! could be mistaken for an episode digest, and it never feeds the episode
//! ledger.
//!
//! # Evidence
//!
//! This encoding has no second-language witness. Two independent
//! implementations agreeing on these bytes is *not* established here; only
//! this crate's own implementation exists.

use sha2::{Digest as _, Sha256};

/// Domain literal of a federation record digest.
pub(crate) const RECORD_DOMAIN: &[u8] = b"wdbx-federation-record-v1";

/// Domain literal of a bundle manifest digest.
pub(crate) const MANIFEST_DOMAIN: &[u8] = b"wdbx-federation-manifest-v1";

/// Domain literal of a bundle digest.
pub(crate) const BUNDLE_DOMAIN: &[u8] = b"wdbx-federation-bundle-v1";

/// SHA-256 over the concatenation of the parts.
pub(crate) fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

/// Append a length-prefixed byte string.
pub(crate) fn put_bytes(output: &mut Vec<u8>, bytes: &[u8]) {
    let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);
}

/// Append a length-prefixed UTF-8 string.
pub(crate) fn put_text(output: &mut Vec<u8>, text: &str) {
    put_bytes(output, text.as_bytes());
}

/// Append a big-endian unsigned integer.
pub(crate) fn put_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_be_bytes());
}

/// Append a boolean as one byte.
pub(crate) fn put_bool(output: &mut Vec<u8>, value: bool) {
    output.push(u8::from(value));
}

/// Append a fixed-size digest.
pub(crate) fn put_digest(output: &mut Vec<u8>, digest: &[u8; 32]) {
    output.extend_from_slice(digest);
}

/// Append a counted list of digests.
pub(crate) fn put_digest_list(output: &mut Vec<u8>, digests: &[[u8; 32]]) {
    put_u64(output, u64::try_from(digests.len()).unwrap_or(u64::MAX));
    for digest in digests {
        put_digest(output, digest);
    }
}

/// A lowercase hexadecimal digit.
pub(crate) fn hex_digit(nibble: u8) -> char {
    char::from(if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + nibble - 10
    })
}
