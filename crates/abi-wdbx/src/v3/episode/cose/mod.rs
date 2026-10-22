//! Private, crate-internal `COSE_Sign1` prototype for episode digests.
//!
//! # What this is
//!
//! A prototype of a `COSE_Sign1` container that carries a detached Ed25519
//! signature over a v3 episode digest, plus an in-memory trust list that
//! keeps "does the mathematics hold" apart from "may this key be trusted
//! now".
//!
//! # What this is not
//!
//! - **It is not a ratified wire profile.** Nothing here has been ratified by
//!   ABI, nothing has been published as a specification, and no consumer may
//!   depend on these bytes.
//! - **It does not implement admission or authorization semantics.** The
//!   trust list models the *shape* of a trust decision; it is not the
//!   decision, and it grants nothing.
//! - **It is not production key custody.** No key is stored, protected,
//!   rotated, or audited here. The prototype holds verifying keys in memory
//!   and forgets them at exit.
//! - **It is not a second-language verifier.** There is no independent
//!   implementation of this profile in another language, so there is no
//!   cross-language evidence that these bytes mean what this module says
//!   they mean.
//! - **It has not had a cryptographic review.** It is prototype code.
//!
//! # The ledger format is unchanged
//!
//! The durable episode ledger still stores the detached
//! [`EpisodeSignature`](crate::v3::episode::EpisodeSignature): a raw
//! Ed25519 signature over the bare 32-byte digest, stored beside the record
//! and outside the committed envelope. That format is untouched by this
//! module, and no committed digest changes because of it.
//!
//! The COSE container is a **separate signature kind over the same digest**.
//! RFC 9052 signs a `Sig_structure` that binds the protected header and the
//! external AAD, so the 64 signature bytes produced here are *different* from
//! the 64 bytes today's `EpisodeSignature` carries. The two are not
//! interchangeable, and the prototype never tries to reuse one as the other.
//!
//! # Shape
//!
//! ```text
//! COSE_Sign1 = [
//!   protected : bstr,   # canonical map {1: -19, 4: kid}
//!   unprotected : map,   # always empty
//!   payload : bstr,     # the 32-byte episode digest
//!   signature : bstr,   # 64-byte Ed25519 signature over the Sig_structure
//! ]
//! ```
//!
//! The `Sig_structure` is RFC 9052's:
//!
//! ```text
//! ["Signature1", protected, external_aad, payload]
//! ```
//!
//! where `external_aad` is this profile's fixed domain separator. The
//! container itself is detached: it never enters a committed envelope, so
//! adding one changes no episode digest.

pub(crate) mod decode;
pub(crate) mod encode;
pub(crate) mod profile;
pub(crate) mod trust;
pub(crate) mod verify;

pub use decode::{CoseContainer, decode_container};
pub use encode::CoseSign1;
pub use trust::{KeyStatus, TrustError, TrustList};
pub use verify::{Integrity, Verification, verify};

use thiserror::Error;

/// Content-free failures from the private `COSE_Sign1` prototype.
///
/// Every variant is a static description. No variant carries bytes, key
/// identifiers, digests, or paths: an error says what was wrong with the
/// shape of a container, never what the container held.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum CoseError {
    /// The container is larger than the profile allows.
    #[error("cose container exceeds the size limit")]
    ContainerTooLarge,
    /// The container ends before a complete item.
    #[error("cose container is truncated")]
    Truncated,
    /// The container is not tagged as `COSE_Sign1`.
    #[error("cose container is not a COSE_Sign1 tag")]
    UnexpectedTag,
    /// The container does not carry the four `COSE_Sign1` elements.
    #[error("cose container does not carry four elements")]
    UnexpectedElementCount,
    /// An element has a CBOR type this profile does not admit.
    #[error("cose container element has an unexpected type")]
    UnexpectedStructure,
    /// The unprotected header is not empty.
    #[error("cose unprotected header is not empty")]
    UnprotectedHeaderNotEmpty,
    /// The protected header exceeds the profile's size cap.
    #[error("cose protected header exceeds the size limit")]
    ProtectedHeaderTooLarge,
    /// The protected header is not a canonical map.
    #[error("cose protected header is malformed")]
    MalformedProtectedHeader,
    /// The protected header names a label this profile does not define.
    #[error("cose protected header names an unknown label")]
    UnknownProtectedLabel,
    /// The protected header asks for critical-header processing.
    #[error("cose protected header requests critical processing")]
    CriticalHeaderUnsupported,
    /// The protected header names an algorithm other than Ed25519.
    #[error("cose algorithm is not supported")]
    UnsupportedAlgorithm,
    /// The key identifier is missing or malformed.
    #[error("cose key identifier is invalid")]
    InvalidKeyId,
    /// The payload is not a 32-byte digest.
    #[error("cose payload is not a 32-byte digest")]
    InvalidPayloadLength,
    /// The signature is not 64 bytes.
    #[error("cose signature is not 64 bytes")]
    InvalidSignatureLength,
    /// The container carries bytes after the `COSE_Sign1` value.
    #[error("cose container has trailing bytes")]
    TrailingBytes,
    /// An integer does not use its shortest encoding.
    #[error("cose integer is not minimally encoded")]
    NonMinimalArgument,
    /// The container could not be encoded.
    #[error("cose container could not be encoded")]
    EncodeFailed,
}
