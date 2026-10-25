//! Exact detached full-payload P1 parser from the private design.
//! No signature is verified by this module. Limits bound inputs and selected
//! buffers, not total heap consumption or allocation failure.
use super::super::{
    Actor, Authority, CompleteEpisodeDecodeError, CompleteEpisodeInput, DecodedCompleteEpisode,
    Identity,
};
use crate::v3::commitment::{CanonicalDecodeError, CanonicalDecodeLimits};
use crate::v3::episode::SignerKeyId;

pub(super) const PROFILE_ID: &str = "wdbx-complete-cose-sign1-prototype-v1";
const CONTAINER_CAP: usize = 256;
const PROTECTED_CAP: usize = 128;
const PAYLOAD_CAP: usize = 1_048_576;
const COMBINED_CAP: usize = 1_048_832;
const STRUCTURE_CAP: usize = 1_049_088;

#[derive(Clone, Copy)]
pub(super) struct CryptoLimits {
    pub max_container_bytes: usize,
    pub max_protected_bytes: usize,
    pub max_payload_bytes: usize,
    pub max_combined_bytes: usize,
    pub max_sig_structure_bytes: usize,
    pub payload_limits: CanonicalDecodeLimits,
}
impl Default for CryptoLimits {
    fn default() -> Self {
        Self {
            max_container_bytes: CONTAINER_CAP,
            max_protected_bytes: PROTECTED_CAP,
            max_payload_bytes: PAYLOAD_CAP,
            max_combined_bytes: COMBINED_CAP,
            max_sig_structure_bytes: STRUCTURE_CAP,
            payload_limits: CanonicalDecodeLimits::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum ProfileError {
    #[error("profile resource limit")]
    ResourceLimit,
    #[error("malformed profile CBOR")]
    MalformedCbor,
    #[error("noncanonical profile encoding")]
    NonCanonicalEncoding,
    #[error("invalid profile header set")]
    InvalidHeaderSet,
    #[error("unsupported profile algorithm")]
    UnsupportedAlgorithm,
    #[error("invalid profile length")]
    InvalidLength,
    #[error("invalid profile payload")]
    InvalidPayload,
}
/// Immutable original bytes and the exact message a future backend must check.
/// Parsing conveys no integrity, trust, or admission result.
pub(super) struct ParsedP1 {
    container: Vec<u8>,
    payload: Vec<u8>,
    protected: Vec<u8>,
    signature: [u8; 64],
    key_id: SignerKeyId,
    sig_structure: Vec<u8>,
    input: CompleteEpisodeInput,
}
redacted!(ParsedP1);
impl ParsedP1 {
    pub(super) fn origin_actor(&self) -> &Actor {
        &self.input.origin.actor
    }
    pub(super) fn authority(&self) -> &Authority {
        &self.input.authority
    }
    pub(super) fn identity(&self) -> &Identity {
        &self.input.identity
    }
    pub(super) fn origin_signer(&self) -> &SignerKeyId {
        &self.input.origin.signer
    }
    pub(super) fn container(&self) -> &[u8] {
        &self.container
    }
    pub(super) fn payload(&self) -> &[u8] {
        &self.payload
    }
    pub(super) fn protected(&self) -> &[u8] {
        &self.protected
    }
    pub(super) fn signature(&self) -> &[u8; 64] {
        &self.signature
    }
    pub(super) fn key_id(&self) -> &SignerKeyId {
        &self.key_id
    }
    pub(super) fn sig_structure(&self) -> &[u8] {
        &self.sig_structure
    }
}
pub(super) fn parse_p1(
    container: &[u8],
    payload: &[u8],
    limits: CryptoLimits,
) -> Result<ParsedP1, ProfileError> {
    if container.len() > limits.max_container_bytes.min(CONTAINER_CAP)
        || payload.len() > limits.max_payload_bytes.min(PAYLOAD_CAP)
        || container
            .len()
            .checked_add(payload.len())
            .ok_or(ProfileError::ResourceLimit)?
            > limits.max_combined_bytes.min(COMBINED_CAP)
    {
        return Err(ProfileError::ResourceLimit);
    }
    let mut reader = Reader::new(container);
    reader.expect(6, 18)?;
    reader.expect(4, 4)?;
    let protected = reader.bytes(limits.max_protected_bytes.min(PROTECTED_CAP))?;
    reader.expect(5, 0)?;
    // A null detached payload is mandatory; attached/digest payloads are a
    // different profile, with no heuristic fallback here.
    if reader.byte()? != 0xf6 {
        return Err(ProfileError::MalformedCbor);
    }
    let signature: [u8; 64] = reader
        .bytes(64)?
        .try_into()
        .map_err(|_| ProfileError::InvalidLength)?;
    reader.finish()?;
    let mut header = Reader::new(protected);
    header
        .expect(5, 2)
        .map_err(|_| ProfileError::InvalidHeaderSet)?;
    header
        .expect(0, 1)
        .map_err(|_| ProfileError::InvalidHeaderSet)?;
    if header.head()? != (1, 18) {
        return Err(ProfileError::UnsupportedAlgorithm);
    }
    header
        .expect(0, 4)
        .map_err(|_| ProfileError::InvalidHeaderSet)?;
    let kid = header.bytes(72)?;
    if kid.len() != 72 {
        return Err(ProfileError::InvalidLength);
    }
    header.finish()?;
    let key_id = std::str::from_utf8(kid)
        .ok()
        .and_then(SignerKeyId::parse)
        .ok_or(ProfileError::InvalidHeaderSet)?;
    let mut decode_limits = limits.payload_limits;
    let caps = CanonicalDecodeLimits::default();
    decode_limits.max_input_bytes = decode_limits.max_input_bytes.min(caps.max_input_bytes);
    decode_limits.max_value_count = decode_limits.max_value_count.min(caps.max_value_count);
    decode_limits.max_collection_entries = decode_limits
        .max_collection_entries
        .min(caps.max_collection_entries);
    decode_limits.max_string_bytes = decode_limits.max_string_bytes.min(caps.max_string_bytes);
    decode_limits.max_parent_count = decode_limits.max_parent_count.min(caps.max_parent_count);
    let decoded = DecodedCompleteEpisode::from_canonical_bytes(payload, decode_limits)
        .map_err(payload_error)?;
    let size = 1usize
        .checked_add(11)
        .and_then(|n| n.checked_add(head_size(protected.len())))
        .and_then(|n| n.checked_add(protected.len()))
        .and_then(|n| n.checked_add(head_size(PROFILE_ID.len())))
        .and_then(|n| n.checked_add(PROFILE_ID.len()))
        .and_then(|n| n.checked_add(head_size(payload.len())))
        .and_then(|n| n.checked_add(payload.len()))
        .ok_or(ProfileError::ResourceLimit)?;
    if size > limits.max_sig_structure_bytes.min(STRUCTURE_CAP) {
        return Err(ProfileError::ResourceLimit);
    }
    let mut structure = Vec::with_capacity(size);
    structure.extend_from_slice(b"\x84\x6aSignature1");
    for bytes in [protected, PROFILE_ID.as_bytes(), payload] {
        push_bytes(&mut structure, bytes);
    }
    Ok(ParsedP1 {
        container: container.to_vec(),
        payload: payload.to_vec(),
        protected: protected.to_vec(),
        signature,
        key_id,
        sig_structure: structure,
        input: decoded.into_parts().0,
    })
}
fn payload_error(error: CompleteEpisodeDecodeError) -> ProfileError {
    match error {
        CompleteEpisodeDecodeError::Canonical(
            CanonicalDecodeError::InputLimit
            | CanonicalDecodeError::ValueLimit
            | CanonicalDecodeError::CollectionLimit
            | CanonicalDecodeError::StringLimit
            | CanonicalDecodeError::ParentLimit
            | CanonicalDecodeError::NestingLimit
            | CanonicalDecodeError::LengthOutOfRange
            | CanonicalDecodeError::AllocationFailure,
        ) => ProfileError::ResourceLimit,
        CompleteEpisodeDecodeError::Canonical(CanonicalDecodeError::NonMinimalArgument)
        | CompleteEpisodeDecodeError::NonCanonicalEncoding => ProfileError::NonCanonicalEncoding,
        _ => ProfileError::InvalidPayload,
    }
}
fn head_size(len: usize) -> usize {
    if len < 24 {
        1
    } else if len <= 255 {
        2
    } else if len <= 65535 {
        3
    } else {
        5
    }
}
fn push_bytes(output: &mut Vec<u8>, bytes: &[u8]) {
    match head_size(bytes.len()) {
        1 => output.push(0x40 | bytes.len() as u8),
        2 => output.extend_from_slice(&[0x58, bytes.len() as u8]),
        3 => {
            output.push(0x59);
            output.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        }
        _ => {
            output.push(0x5a);
            output.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        }
    }
    output.extend_from_slice(bytes);
}
struct Reader<'a> {
    input: &'a [u8],
    cursor: usize,
}
impl<'a> Reader<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self { input, cursor: 0 }
    }
    fn byte(&mut self) -> Result<u8, ProfileError> {
        let byte = *self
            .input
            .get(self.cursor)
            .ok_or(ProfileError::MalformedCbor)?;
        self.cursor += 1;
        Ok(byte)
    }
    fn head(&mut self) -> Result<(u8, u64), ProfileError> {
        let byte = self.byte()?;
        let info = byte & 31;
        let (count, min) = match info {
            0..=23 => return Ok((byte >> 5, u64::from(info))),
            24 => (1, 24),
            25 => (2, 256),
            26 => (4, 65536),
            27 => (8, 1u64 << 32),
            _ => return Err(ProfileError::MalformedCbor),
        };
        let mut value = 0u64;
        for _ in 0..count {
            value = (value << 8) | u64::from(self.byte()?);
        }
        if value < min {
            return Err(ProfileError::NonCanonicalEncoding);
        }
        Ok((byte >> 5, value))
    }
    fn expect(&mut self, major: u8, value: u64) -> Result<(), ProfileError> {
        if self.head()? != (major, value) {
            return Err(ProfileError::MalformedCbor);
        }
        Ok(())
    }
    fn bytes(&mut self, cap: usize) -> Result<&'a [u8], ProfileError> {
        let (major, len) = self.head()?;
        if major != 2 {
            return Err(ProfileError::MalformedCbor);
        }
        let len = usize::try_from(len).map_err(|_| ProfileError::ResourceLimit)?;
        if len > cap {
            return Err(ProfileError::ResourceLimit);
        }
        let end = self
            .cursor
            .checked_add(len)
            .ok_or(ProfileError::ResourceLimit)?;
        let bytes = self
            .input
            .get(self.cursor..end)
            .ok_or(ProfileError::MalformedCbor)?;
        self.cursor = end;
        Ok(bytes)
    }
    fn finish(&self) -> Result<(), ProfileError> {
        if self.cursor != self.input.len() {
            return Err(ProfileError::MalformedCbor);
        }
        Ok(())
    }
}
