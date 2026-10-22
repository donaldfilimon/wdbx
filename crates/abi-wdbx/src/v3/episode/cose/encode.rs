//! Encoding of the private `COSE_Sign1` prototype.
//!
//! The container is the RFC 9052 `COSE_Sign1` shape:
//!
//! ```text
//! COSE_Sign1 = [
//!   protected : bstr,   # canonical map {1: -19, 4: kid}
//!   unprotected : map,   # always empty in this profile
//!   payload : bstr,     # the 32-byte episode digest
//!   signature : bstr,   # 64-byte Ed25519 signature
//! ]
//! ```
//!
//! **The signature covers the `Sig_structure`, not the bare digest.** RFC
//! 9052 signs
//!
//! ```text
//! Sig_structure = ["Signature1", protected, external_aad, payload]
//! ```
//!
//! so the bytes signed here are that structure, and the resulting 64-byte
//! signature is *different* from the detached [`EpisodeSignature`] the ledger
//! stores today. That is deliberate: today's signature is a raw signature over
//! the 32-byte digest, and reusing those bytes here would claim an RFC 9052
//! conformance the bytes do not have. The two signature kinds are separate
//! artifacts over the same digest, and the ledger format is unchanged.

use super::CoseError;
use super::profile::{
    ALG_ED25519, EXTERNAL_AAD, KEY_ID_BYTES, LABEL_ALG, LABEL_KID, MAX_PROTECTED_BYTES,
    PAYLOAD_BYTES, SIGNATURE_BYTES, SIGNATURE_CONTEXT,
};
use crate::v3::commitment::CanonicalValue;
use crate::v3::episode::signing::EpisodeSigner;

/// The private `COSE_Sign1` profile.
///
/// This type names the profile's encoder; it carries no state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoseSign1;

impl CoseSign1 {
    /// Encode a detached `COSE_Sign1` container over one episode digest.
    ///
    /// The signature is made over the RFC 9052 `Sig_structure` and is
    /// therefore distinct from the detached episode signature the ledger
    /// stores. The container never enters a committed envelope.
    pub fn encode(digest: &[u8; 32], signer: &EpisodeSigner) -> Result<Vec<u8>, CoseError> {
        let protected = protected_header(signer.key_id().as_str().as_bytes())?;
        let structure = sig_structure(&protected, digest)?;
        let signature = signer.sign_message(&structure);
        frame(&protected, digest, &signature)
    }
}

/// Build the canonical protected header for one key identifier.
fn protected_header(kid: &[u8]) -> Result<Vec<u8>, CoseError> {
    if kid.len() != KEY_ID_BYTES {
        return Err(CoseError::InvalidKeyId);
    }
    let header = CanonicalValue::Map(vec![
        (
            CanonicalValue::Unsigned(LABEL_ALG),
            CanonicalValue::Negative(ALG_ED25519),
        ),
        (
            CanonicalValue::Unsigned(LABEL_KID),
            CanonicalValue::Bytes(kid.to_vec()),
        ),
    ]);
    let bytes = header
        .canonical_bytes()
        .map_err(|_| CoseError::EncodeFailed)?;
    if bytes.len() > MAX_PROTECTED_BYTES {
        return Err(CoseError::ProtectedHeaderTooLarge);
    }
    Ok(bytes)
}

/// Build the RFC 9052 `Sig_structure` that this profile signs.
///
/// The verifier rebuilds this from the container's own protected header, so a
/// container that carries different protected bytes cannot be verified with
/// the same signature.
pub(super) fn sig_structure(
    protected: &[u8],
    payload: &[u8; PAYLOAD_BYTES],
) -> Result<Vec<u8>, CoseError> {
    let structure = CanonicalValue::Array(vec![
        CanonicalValue::Text(SIGNATURE_CONTEXT.to_owned()),
        CanonicalValue::Bytes(protected.to_vec()),
        CanonicalValue::Bytes(EXTERNAL_AAD.to_vec()),
        CanonicalValue::Bytes(payload.to_vec()),
    ]);
    structure
        .canonical_bytes()
        .map_err(|_| CoseError::EncodeFailed)
}

/// Frame the four `COSE_Sign1` elements under tag 18.
///
/// The tag and the array head are written by hand: the crate's canonical
/// encoder deliberately bans CBOR tags, and this profile must not relax that
/// ban for the frozen `abbey-cbor-episode-v1` domain.
fn frame(
    protected: &[u8],
    payload: &[u8; PAYLOAD_BYTES],
    signature: &[u8; SIGNATURE_BYTES],
) -> Result<Vec<u8>, CoseError> {
    let mut output = Vec::with_capacity(protected.len() + payload.len() + signature.len() + 8);
    output.push(0xd2); // tag(18)
    output.push(0x84); // array(4)
    push_byte_string(&mut output, protected)?;
    output.push(0xa0); // empty unprotected map
    push_byte_string(&mut output, payload)?;
    push_byte_string(&mut output, signature)?;
    Ok(output)
}

/// Append one definite-length byte string.
///
/// Every length in this profile is below 256 bytes, so the one-byte length
/// form is always the canonical one.
fn push_byte_string(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), CoseError> {
    let length = u8::try_from(bytes.len()).map_err(|_| CoseError::EncodeFailed)?;
    output.push(0x58);
    output.push(length);
    output.extend_from_slice(bytes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::episode::signing::EpisodeSigner;
    use ed25519_dalek::SigningKey;

    fn signer() -> EpisodeSigner {
        EpisodeSigner::new(SigningKey::from_bytes(&[1_u8; 32]))
    }

    #[test]
    fn the_protected_header_is_the_expected_canonical_map() {
        let key_id = signer().key_id().as_str().as_bytes().to_vec();
        let header = protected_header(&key_id).expect("protected header");
        assert_eq!(header.len(), 78);
        assert_eq!(&header[..3], &[0xa2, 0x01, 0x32]);
        assert_eq!(&header[3..5], &[0x04, 0x58]);
        assert_eq!(header[5], u8::try_from(KEY_ID_BYTES).expect("kid length"));
        assert_eq!(&header[6..], key_id.as_slice());
    }

    #[test]
    fn the_sig_structure_starts_with_the_rfc_9052_context_string() {
        let structure = sig_structure(&[0xa0], &[0_u8; 32]).expect("sig structure");
        assert_eq!(&structure[..12], b"\x84\x6aSignature1");
    }
}
