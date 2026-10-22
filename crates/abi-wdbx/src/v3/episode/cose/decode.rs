//! Strict, bounded reader for the private `COSE_Sign1` prototype.
//!
//! The crate's canonical reader is deliberately closed: it rejects CBOR tags
//! and therefore cannot read a `COSE_Sign1` container. Relaxing that reader
//! would widen the frozen `abbey-cbor-episode-v1` domain, so this module
//! carries its own small parser instead. The parser is strict rather than
//! lenient: it accepts exactly the shape this profile emits, and everything
//! else is an error.
//!
//! What the parser refuses:
//!
//! - a container that is not tagged 18, or does not carry exactly four
//!   elements;
//! - a protected header that is not a canonical map, exceeds the size cap,
//!   repeats a key, orders its keys non-canonically, or names a label this
//!   profile does not define;
//! - a `crit` label, because this profile implements no critical-header
//!   processing and must not appear to accept one;
//! - an algorithm other than Ed25519;
//! - a key identifier that is not a well-formed [`SignerKeyId`];
//! - a payload that is not exactly 32 bytes, or a signature that is not
//!   exactly 64 bytes;
//! - an unprotected header that is not empty;
//! - integers that do not use their shortest encoding;
//! - bytes after the container.

use super::CoseError;
use super::profile::{
    ALG_ED25519, COSE_SIGN1_TAG, KEY_ID_BYTES, LABEL_ALG, LABEL_CRIT, LABEL_KID,
    MAX_CONTAINER_BYTES, MAX_PROTECTED_BYTES, PAYLOAD_BYTES, SIGNATURE_BYTES, protected_limits,
};
use crate::v3::commitment::CanonicalValue;
use crate::v3::episode::signing::SignerKeyId;

/// A decoded `COSE_Sign1` container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoseContainer {
    protected: Vec<u8>,
    alg: i64,
    kid: Vec<u8>,
    key_id: SignerKeyId,
    payload: [u8; PAYLOAD_BYTES],
    signature: [u8; SIGNATURE_BYTES],
}

impl CoseContainer {
    /// The protected header bytes exactly as they were carried.
    ///
    /// The signature covers these bytes, so a verifier rebuilds the
    /// `Sig_structure` from them rather than from a re-encoded map.
    #[must_use]
    pub fn protected(&self) -> &[u8] {
        &self.protected
    }

    /// The algorithm named by the protected header.
    #[must_use]
    pub fn alg(&self) -> i64 {
        self.alg
    }

    /// The key-identifier bytes carried by the protected header.
    #[must_use]
    pub fn kid(&self) -> &[u8] {
        &self.kid
    }

    /// The parsed key identifier.
    #[must_use]
    pub fn key_id(&self) -> &SignerKeyId {
        &self.key_id
    }

    /// The signed payload: the episode digest.
    #[must_use]
    pub fn payload(&self) -> &[u8; PAYLOAD_BYTES] {
        &self.payload
    }

    /// The Ed25519 signature carried by the container.
    #[must_use]
    pub fn signature(&self) -> &[u8; SIGNATURE_BYTES] {
        &self.signature
    }
}

/// Decode one `COSE_Sign1` container.
///
/// This checks syntax and shape only. It says nothing about trust,
/// authorization, admission, or whether the signature verifies.
pub fn decode_container(container: &[u8]) -> Result<CoseContainer, CoseError> {
    if container.len() > MAX_CONTAINER_BYTES {
        return Err(CoseError::ContainerTooLarge);
    }
    let mut reader = Reader {
        input: container,
        cursor: 0,
    };

    let (major, tag) = reader.head()?;
    if major != 6 || tag != COSE_SIGN1_TAG {
        return Err(CoseError::UnexpectedTag);
    }
    let (major, count) = reader.head()?;
    if major != 4 {
        return Err(CoseError::UnexpectedStructure);
    }
    if count != 4 {
        return Err(CoseError::UnexpectedElementCount);
    }

    let protected = reader.capped_byte_string(MAX_PROTECTED_BYTES)?;
    let (major, count) = reader.head()?;
    if major != 5 {
        return Err(CoseError::UnexpectedStructure);
    }
    if count != 0 {
        return Err(CoseError::UnprotectedHeaderNotEmpty);
    }

    let payload = reader.exact_byte_string(PAYLOAD_BYTES, CoseError::InvalidPayloadLength)?;
    let payload: [u8; PAYLOAD_BYTES] = payload
        .try_into()
        .map_err(|_| CoseError::InvalidPayloadLength)?;
    let signature = reader.exact_byte_string(SIGNATURE_BYTES, CoseError::InvalidSignatureLength)?;
    let signature: [u8; SIGNATURE_BYTES] = signature
        .try_into()
        .map_err(|_| CoseError::InvalidSignatureLength)?;

    if reader.cursor != container.len() {
        return Err(CoseError::TrailingBytes);
    }

    let (alg, kid, key_id) = protected_header(&protected)?;
    Ok(CoseContainer {
        protected,
        alg,
        kid,
        key_id,
        payload,
        signature,
    })
}

/// Read the protected header and return its algorithm, key id, and identifier.
fn protected_header(protected: &[u8]) -> Result<(i64, Vec<u8>, SignerKeyId), CoseError> {
    let header = CanonicalValue::from_canonical_bytes(protected, protected_limits())
        .map_err(|_| CoseError::MalformedProtectedHeader)?;
    let CanonicalValue::Map(entries) = header else {
        return Err(CoseError::MalformedProtectedHeader);
    };

    let mut alg = None;
    let mut kid = None;
    for (key, value) in entries {
        let CanonicalValue::Unsigned(label) = key else {
            return Err(CoseError::MalformedProtectedHeader);
        };
        match label {
            LABEL_ALG => {
                let CanonicalValue::Negative(algorithm) = value else {
                    return Err(CoseError::UnsupportedAlgorithm);
                };
                alg = Some(algorithm);
            }
            LABEL_KID => {
                let CanonicalValue::Bytes(bytes) = value else {
                    return Err(CoseError::InvalidKeyId);
                };
                kid = Some(bytes);
            }
            LABEL_CRIT => return Err(CoseError::CriticalHeaderUnsupported),
            _ => return Err(CoseError::UnknownProtectedLabel),
        }
    }

    let alg = alg.ok_or(CoseError::MalformedProtectedHeader)?;
    if alg != ALG_ED25519 {
        return Err(CoseError::UnsupportedAlgorithm);
    }
    let kid = kid.ok_or(CoseError::MalformedProtectedHeader)?;
    if kid.len() != KEY_ID_BYTES {
        return Err(CoseError::InvalidKeyId);
    }
    let text = core::str::from_utf8(&kid).map_err(|_| CoseError::InvalidKeyId)?;
    let key_id = SignerKeyId::parse(text).ok_or(CoseError::InvalidKeyId)?;
    Ok((alg, kid, key_id))
}

/// A bounded reader over one container.
struct Reader<'a> {
    input: &'a [u8],
    cursor: usize,
}

impl<'a> Reader<'a> {
    /// Read one CBOR head and return its major type and argument.
    ///
    /// Only definite-length items with the shortest argument encoding are
    /// admitted; indefinite lengths and reserved additional information are
    /// errors.
    fn head(&mut self) -> Result<(u8, u64), CoseError> {
        let initial = *self.input.get(self.cursor).ok_or(CoseError::Truncated)?;
        self.cursor += 1;
        let major = initial >> 5;
        let info = initial & 0x1f;
        let argument = match info {
            0..=23 => u64::from(info),
            24 => self.short_argument(1, 24)?,
            25 => self.short_argument(2, 256)?,
            26 => self.short_argument(4, 65_536)?,
            27 => self.short_argument(8, 4_294_967_296)?,
            _ => return Err(CoseError::UnexpectedStructure),
        };
        Ok((major, argument))
    }

    /// Read a fixed-width argument and reject a non-shortest encoding.
    fn short_argument(&mut self, width: usize, minimum: u64) -> Result<u64, CoseError> {
        let bytes = self.take(width)?;
        let mut argument = 0_u64;
        for byte in bytes {
            argument = (argument << 8) | u64::from(*byte);
        }
        if argument < minimum {
            return Err(CoseError::NonMinimalArgument);
        }
        Ok(argument)
    }

    /// Read one definite-length byte string of at most `cap` bytes.
    fn capped_byte_string(&mut self, cap: usize) -> Result<Vec<u8>, CoseError> {
        let (major, length) = self.head()?;
        if major != 2 {
            return Err(CoseError::UnexpectedStructure);
        }
        let length = usize::try_from(length).map_err(|_| CoseError::Truncated)?;
        if length > cap {
            return Err(CoseError::ProtectedHeaderTooLarge);
        }
        Ok(self.take(length)?.to_vec())
    }

    /// Read one definite-length byte string of exactly `expected` bytes.
    ///
    /// A length mismatch is reported as `mismatch`, which names the element
    /// that was wrong without describing its bytes.
    fn exact_byte_string(
        &mut self,
        expected: usize,
        mismatch: CoseError,
    ) -> Result<Vec<u8>, CoseError> {
        let (major, length) = self.head()?;
        if major != 2 {
            return Err(CoseError::UnexpectedStructure);
        }
        let length = usize::try_from(length).map_err(|_| CoseError::Truncated)?;
        if length != expected {
            return Err(mismatch);
        }
        Ok(self.take(length)?.to_vec())
    }

    /// Borrow the next `length` bytes.
    fn take(&mut self, length: usize) -> Result<&'a [u8], CoseError> {
        let end = self
            .cursor
            .checked_add(length)
            .ok_or(CoseError::Truncated)?;
        let bytes = self
            .input
            .get(self.cursor..end)
            .ok_or(CoseError::Truncated)?;
        self.cursor = end;
        Ok(bytes)
    }
}
