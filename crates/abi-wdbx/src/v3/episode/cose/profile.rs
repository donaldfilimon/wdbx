//! Fixed parameters of the private `COSE_Sign1` prototype.
//!
//! Every value here is part of the prototype's frozen shape. Changing one
//! changes the bytes this prototype produces, so each is a constant rather
//! than a runtime setting: the profile is a prototype, and a prototype with
//! knobs is two profiles.
//!
//! The profile is deliberately narrow:
//!
//! - one algorithm: Ed25519 (`-19`), never negotiated;
//! - one protected header: `{1: -19, 4: kid}`, nothing else;
//! - one key identifier: the 72 ASCII bytes of an existing
//!   [`SignerKeyId`](crate::v3::episode::SignerKeyId);
//! - one payload: the 32-byte episode digest;
//! - one external AAD: a fixed domain separator, so a signature made under
//!   this profile cannot be replayed into another context that signs the same
//!   bytes.

use crate::v3::commitment::CanonicalDecodeLimits;

/// CBOR tag number that identifies a `COSE_Sign1` object.
pub const COSE_SIGN1_TAG: u64 = 18;

/// COSE algorithm identifier for Ed25519 (RFC 9052 section 3.1).
pub const ALG_ED25519: i64 = -19;

/// Protected-header label for the algorithm identifier.
pub const LABEL_ALG: u64 = 1;

/// Protected-header label for the key identifier.
pub const LABEL_KID: u64 = 4;

/// Protected-header label for critical headers.
///
/// This profile defines no critical-header processing, so a container that
/// carries the label is rejected rather than interpreted.
pub const LABEL_CRIT: u64 = 2;

/// Context string of the RFC 9052 `Sig_structure`.
pub const SIGNATURE_CONTEXT: &str = "Signature1";

/// External additional authenticated data bound into every signature.
///
/// This is a domain separator, not a secret. It exists so that the same
/// digest signed under this profile is a different byte string from the same
/// digest signed by any other caller of the same key.
pub const EXTERNAL_AAD: &[u8] = b"wdbx-cose-sign1-p1";

/// Length of the key-identifier byte string.
///
/// `SignerKeyId` is `ed25519:` (8 ASCII bytes) followed by 64 lowercase hex
/// digits, so the identifier is always exactly 72 bytes.
pub const KEY_ID_BYTES: usize = 72;

/// Length of an Ed25519 signature.
pub const SIGNATURE_BYTES: usize = 64;

/// Length of the signed payload: the 32-byte episode digest.
pub const PAYLOAD_BYTES: usize = 32;

/// Largest accepted encoded container, in bytes.
pub const MAX_CONTAINER_BYTES: usize = 512;

/// Largest accepted encoded protected header, in bytes.
pub const MAX_PROTECTED_BYTES: usize = 128;

/// Reader policy for the protected header.
///
/// The protected header is a two-member map whose only byte string is the
/// key identifier, so these bounds are far below the profile's own caps.
#[must_use]
pub const fn protected_limits() -> CanonicalDecodeLimits {
    CanonicalDecodeLimits {
        max_input_bytes: MAX_PROTECTED_BYTES,
        max_value_count: 16,
        max_collection_entries: 8,
        max_string_bytes: MAX_PROTECTED_BYTES,
        max_parent_count: 0,
    }
}
