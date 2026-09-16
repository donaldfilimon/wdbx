//! Per-episode Ed25519 signatures over the canonical v3 digest.
//!
//! This is the signing half that `crate::v3::commitment` deliberately leaves
//! out: that module computes `d = SHA-256(canonical envelope)` and stops. Here
//! the canonical writer signs exactly those 32 bytes, as the canonical-episode
//! specification's commitment function states:
//!
//! ```text
//! sig = Ed25519_sign(signing_key, d)
//! ```
//!
//! Nothing in this module enters the committed envelope. A signature is
//! detached metadata stored beside the record, so every existing v3 digest is
//! unchanged and an unsigned record stays a valid record.
//!
//! **Who signs.** Only the episode store, when it admits a write. Adapters may
//! carry and echo a signature but never compute one, for the same reason they
//! never compute a digest. The signer identity is therefore the canonical
//! writer's key; the episode's *source* stays in the committed header, and the
//! signature binds "this writer admitted an envelope naming that source".
//!
//! **Key separation.** Use a key that signs nothing else. v2 segment
//! authentication signs raw 32-byte values with its own key; if one key signed
//! both domains, a v3 episode signature could be presented as a v2 one. Keeping
//! the keys distinct lets this module sign `d` exactly as specified, without a
//! prefix the specification does not define.
//!
//! **Out of scope here, and not implied by the word "signing":** `COSE_Sign1`
//! wrapping, cross-language verification, key rotation or revocation, and
//! source-side (adapter) signatures.

use ed25519_dalek::{Signature, Signer as _, SigningKey, VerifyingKey};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};
use std::fmt;
use std::path::Path;

/// Prefix of every derived signer key identifier.
pub const SIGNER_KEY_ID_PREFIX: &str = "ed25519:";

/// Identifier of an episode-signing key: `ed25519:` plus the lowercase hex of
/// SHA-256 over the 32 verifying-key bytes.
///
/// The identifier is derived from the key rather than chosen by an operator,
/// so a record cannot claim a key id that its signature was not made under
/// without verification noticing the mismatch.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignerKeyId(String);

impl SignerKeyId {
    /// Derive the identifier for a verifying key.
    #[must_use]
    pub fn for_key(key: &VerifyingKey) -> Self {
        let fingerprint = Sha256::digest(key.as_bytes());
        let mut id = String::with_capacity(SIGNER_KEY_ID_PREFIX.len() + 64);
        id.push_str(SIGNER_KEY_ID_PREFIX);
        for byte in fingerprint {
            id.push(hex_digit(byte >> 4));
            id.push(hex_digit(byte & 0x0f));
        }
        Self(id)
    }

    /// Parse a stored identifier, accepting only the exact derived shape.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let hex = value.strip_prefix(SIGNER_KEY_ID_PREFIX)?;
        (hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
            .then(|| Self(value.to_owned()))
    }

    /// The identifier text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SignerKeyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for SignerKeyId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SignerKeyId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).ok_or_else(|| serde::de::Error::custom("invalid signer key id"))
    }
}

/// A detached Ed25519 signature over one episode digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeSignature {
    /// Identifier of the key the signature claims to be made under.
    pub signer_key_id: SignerKeyId,
    /// The 64-byte Ed25519 signature over the 32-byte episode digest.
    #[serde(with = "signature_bytes")]
    pub signature: [u8; 64],
}

/// Outcome of checking an episode's signature.
///
/// The four states are kept apart on purpose. An unsigned episode is not an
/// invalid one, and a signature under a key the verifier does not hold is
/// unverifiable rather than forged; collapsing either into `Invalid` would
/// turn missing evidence into negative evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignatureStatus {
    /// The record carries no signature.
    Unsigned,
    /// The signature verifies under the named key.
    Valid(SignerKeyId),
    /// The signature is present but does not verify under the named key.
    Invalid(SignerKeyId),
    /// The record names a key the verifier could not resolve.
    UnknownKey(SignerKeyId),
}

/// An episode-signing key held by the canonical writer.
pub struct EpisodeSigner {
    key: SigningKey,
    key_id: SignerKeyId,
}

impl fmt::Debug for EpisodeSigner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EpisodeSigner")
            .field("key_id", &self.key_id)
            .finish_non_exhaustive()
    }
}

impl EpisodeSigner {
    /// Wrap a signing key. Callers are responsible for keeping it distinct
    /// from any key used for v2 segment authentication.
    #[must_use]
    pub fn new(key: SigningKey) -> Self {
        let key_id = SignerKeyId::for_key(&key.verifying_key());
        Self { key, key_id }
    }

    /// Load a raw 32-byte Ed25519 secret key from an owner-only file, through
    /// the same bounded, permission-checked reader v2 uses for its keys.
    pub fn from_key_file(path: impl AsRef<Path>) -> Result<Self, crate::v2::SecurityError> {
        crate::v2::read_signing_key_file(path.as_ref()).map(Self::new)
    }

    /// Identifier of this signer's key.
    #[must_use]
    pub fn key_id(&self) -> &SignerKeyId {
        &self.key_id
    }

    /// The verifying key matching this signer.
    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }

    /// Sign exactly the 32-byte episode digest.
    #[must_use]
    pub fn sign_digest(&self, digest: &[u8; 32]) -> EpisodeSignature {
        EpisodeSignature {
            signer_key_id: self.key_id.clone(),
            signature: self.key.sign(digest).to_bytes(),
        }
    }
}

/// Verify a signature over a digest under a given verifying key.
///
/// Uses Ed25519 strict verification, which rejects non-canonical and
/// small-order encodings that plain verification accepts. A signature whose
/// recorded key id does not match `key` is `Invalid`, not re-checked under a
/// different key.
#[must_use]
pub fn verify_digest(
    key: &VerifyingKey,
    digest: &[u8; 32],
    signature: &EpisodeSignature,
) -> SignatureStatus {
    let claimed = signature.signer_key_id.clone();
    if SignerKeyId::for_key(key) != claimed {
        return SignatureStatus::Invalid(claimed);
    }
    let parsed = Signature::from_bytes(&signature.signature);
    if key.verify_strict(digest, &parsed).is_ok() {
        SignatureStatus::Valid(claimed)
    } else {
        SignatureStatus::Invalid(claimed)
    }
}

/// Check an optional signature, resolving its key through `resolve`.
#[must_use]
pub fn check_signature(
    digest: &[u8; 32],
    signature: Option<&EpisodeSignature>,
    resolve: impl Fn(&SignerKeyId) -> Option<VerifyingKey>,
) -> SignatureStatus {
    let Some(signature) = signature else {
        return SignatureStatus::Unsigned;
    };
    match resolve(&signature.signer_key_id) {
        Some(key) => verify_digest(&key, digest, signature),
        None => SignatureStatus::UnknownKey(signature.signer_key_id.clone()),
    }
}

fn hex_digit(nibble: u8) -> char {
    char::from(if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + nibble - 10
    })
}

mod signature_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(
        bytes: &[u8; 64],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(bytes.iter())
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; 64], D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("episode signature must be 64 bytes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signer(seed: u8) -> EpisodeSigner {
        EpisodeSigner::new(SigningKey::from_bytes(&[seed; 32]))
    }

    #[test]
    fn a_signature_verifies_over_exactly_its_digest() {
        let signer = signer(1);
        let digest = [7_u8; 32];
        let signature = signer.sign_digest(&digest);
        assert_eq!(
            verify_digest(&signer.verifying_key(), &digest, &signature),
            SignatureStatus::Valid(signer.key_id().clone())
        );

        let mut tampered = digest;
        tampered[31] ^= 1;
        assert_eq!(
            verify_digest(&signer.verifying_key(), &tampered, &signature),
            SignatureStatus::Invalid(signer.key_id().clone())
        );
    }

    #[test]
    fn a_mutated_signature_byte_is_invalid() {
        let signer = signer(1);
        let digest = [7_u8; 32];
        let mut signature = signer.sign_digest(&digest);
        signature.signature[0] ^= 1;
        assert!(matches!(
            verify_digest(&signer.verifying_key(), &digest, &signature),
            SignatureStatus::Invalid(_)
        ));
    }

    #[test]
    fn another_key_cannot_verify_even_with_a_forged_key_id() {
        let real = signer(1);
        let other = signer(2);
        let digest = [7_u8; 32];
        let signature = real.sign_digest(&digest);
        // Wrong key, honest id: the id no longer matches the key.
        assert!(matches!(
            verify_digest(&other.verifying_key(), &digest, &signature),
            SignatureStatus::Invalid(_)
        ));
        // Wrong key, id relabelled to match it: the signature itself fails.
        let relabelled = EpisodeSignature {
            signer_key_id: other.key_id().clone(),
            ..signature
        };
        assert_eq!(
            verify_digest(&other.verifying_key(), &digest, &relabelled),
            SignatureStatus::Invalid(other.key_id().clone())
        );
    }

    #[test]
    fn a_valid_signature_is_never_credited_to_a_key_id_it_was_not_made_under() {
        // Key B genuinely signs, but the record claims key A. Handing B's key
        // to the verifier (a confused or hostile resolver would) must not
        // yield `Valid(A)`: that would attribute B's signature to A.
        let a = signer(1);
        let b = signer(2);
        let digest = [7_u8; 32];
        let misattributed = EpisodeSignature {
            signer_key_id: a.key_id().clone(),
            ..b.sign_digest(&digest)
        };
        assert_eq!(
            verify_digest(&b.verifying_key(), &digest, &misattributed),
            SignatureStatus::Invalid(a.key_id().clone())
        );
        let b_key = b.verifying_key();
        assert_eq!(
            check_signature(&digest, Some(&misattributed), |_| Some(b_key)),
            SignatureStatus::Invalid(a.key_id().clone())
        );
    }

    #[test]
    fn unsigned_and_unknown_key_are_not_invalid() {
        let signer = signer(1);
        let digest = [7_u8; 32];
        assert_eq!(
            check_signature(&digest, None, |_| None),
            SignatureStatus::Unsigned
        );
        let signature = signer.sign_digest(&digest);
        assert_eq!(
            check_signature(&digest, Some(&signature), |_| None),
            SignatureStatus::UnknownKey(signer.key_id().clone())
        );
        let key = signer.verifying_key();
        assert_eq!(
            check_signature(&digest, Some(&signature), |id| (id == signer.key_id())
                .then_some(key)),
            SignatureStatus::Valid(signer.key_id().clone())
        );
    }

    #[test]
    fn key_ids_are_derived_and_parsed_strictly() {
        let id = signer(1).key_id().clone();
        assert!(id.as_str().starts_with(SIGNER_KEY_ID_PREFIX));
        assert_eq!(id.as_str().len(), SIGNER_KEY_ID_PREFIX.len() + 64);
        assert_eq!(SignerKeyId::parse(id.as_str()), Some(id.clone()));
        assert_ne!(signer(2).key_id(), &id);
        for bad in [
            "",
            "ed25519:",
            "ed25519:ABC",
            "rsa:0000000000000000000000000000000000000000000000000000000000000000",
            "ed25519:000000000000000000000000000000000000000000000000000000000000000G",
            "ed25519:00000000000000000000000000000000000000000000000000000000000000000",
        ] {
            assert_eq!(SignerKeyId::parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_signature_round_trips_through_json_and_rejects_a_short_one() {
        let signature = signer(1).sign_digest(&[7_u8; 32]);
        let json = serde_json::to_string(&signature).expect("serialize");
        let back: EpisodeSignature = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, signature);

        let truncated = serde_json::json!({
            "signer_key_id": signature.signer_key_id.as_str(),
            "signature": [1, 2, 3],
        });
        assert!(serde_json::from_value::<EpisodeSignature>(truncated).is_err());
        let extra = serde_json::json!({
            "signer_key_id": signature.signer_key_id.as_str(),
            "signature": signature.signature.to_vec(),
            "note": "x",
        });
        assert!(serde_json::from_value::<EpisodeSignature>(extra).is_err());
    }

    #[test]
    fn debug_never_prints_the_secret_key() {
        let rendered = format!("{:?}", signer(1));
        assert!(rendered.contains("key_id"));
        assert!(!rendered.contains("SigningKey"));
    }
}
