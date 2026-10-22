//! Verification of the private `COSE_Sign1` prototype.
//!
//! Verification answers two different questions and keeps them apart:
//!
//! - *does the mathematics hold?* — the signature verifies under the key the
//!   container names, over the `Sig_structure` this profile defines;
//! - *can the mathematics be judged at all?* — the named key is available to
//!   the verifier.
//!
//! A missing key is not a forgery, so it is reported as
//! [`Integrity::Unverifiable`] rather than [`Integrity::Invalid`]. A single
//! boolean would collapse "cannot tell" into "forged", and that collapse is
//! how missing evidence turns into negative evidence.
//!
//! The key named by the container is the *only* key tried. A verifier never
//! tries every key it holds, because a container that fails under its named
//! key must not be rescued by another key.

use super::CoseError;
use super::decode::decode_container;
use super::encode::sig_structure;
use crate::v3::episode::signing::SignerKeyId;
use ed25519_dalek::{Signature, VerifyingKey};

/// The mathematical outcome of checking one container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Integrity {
    /// The signature verifies under the named key and attests the digest.
    Valid,
    /// A signature is present but does not verify, or does not attest the
    /// digest that was checked against.
    Invalid,
    /// The container carries no signature.
    ///
    /// This profile requires an inline 64-byte signature, so its decoder
    /// never produces this state. The variant is kept so that a future
    /// detached-signature container does not have to widen the axis.
    Unsigned,
    /// The named key could not be resolved, so the signature cannot be judged.
    Unverifiable,
}

/// The result of verifying one container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verification {
    /// What the mathematics says.
    pub integrity: Integrity,
    /// The key identifier the container named, when it named one.
    pub key_id: Option<SignerKeyId>,
}

/// Verify a container against the digest it is supposed to attest.
///
/// `resolve` maps the key identifier the container names to its verifying
/// key. Returning `None` yields [`Integrity::Unverifiable`], never
/// [`Integrity::Invalid`]. A decode failure is an error, not a verdict: the
/// container was never understood, so nothing can be said about it.
pub fn verify(
    container: &[u8],
    digest: &[u8; 32],
    resolve: impl Fn(&SignerKeyId) -> Option<VerifyingKey>,
) -> Result<Verification, CoseError> {
    let parsed = decode_container(container)?;
    let key_id = Some(parsed.key_id().clone());

    // The payload is the digest. A container that carries another digest
    // does not attest this one, whatever its signature says.
    if parsed.payload() != digest {
        return Ok(Verification {
            integrity: Integrity::Invalid,
            key_id,
        });
    }

    let Some(key) = resolve(parsed.key_id()) else {
        return Ok(Verification {
            integrity: Integrity::Unverifiable,
            key_id,
        });
    };

    // A signature is never credited to a key it was not made under.
    if SignerKeyId::for_key(&key) != *parsed.key_id() {
        return Ok(Verification {
            integrity: Integrity::Invalid,
            key_id,
        });
    }

    let structure = sig_structure(parsed.protected(), parsed.payload())?;
    let signature = Signature::from_bytes(parsed.signature());
    let integrity = if key.verify_strict(&structure, &signature).is_ok() {
        Integrity::Valid
    } else {
        Integrity::Invalid
    };
    Ok(Verification { integrity, key_id })
}
