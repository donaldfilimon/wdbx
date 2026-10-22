//! Bounded, non-normalizing reader for the frozen restricted profile.

use super::{CanonicalValue, EpisodeCommitment, MAX_NESTING_DEPTH, PROFILE_NAME};
use core::cmp::Ordering;
use thiserror::Error;

/// Finite reader policy; these bounds do not restrict the existing encoder.
///
/// Defaults allow 1 MiB input, 65,536 total values, 16,384 entries per
/// collection, 1 MiB per string, and 4,096 episode parents.
///
/// Owned value nodes and collection storage add overhead beyond input bytes.
/// Each limit is independent; zero allows no instances of the counted resource
/// (empty strings/collections still have zero bytes/entries). Values count all
/// items, including the root, map keys, and map values. The parent cap applies
/// only to episode decoding. Nesting has the fixed profile maximum of 32.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanonicalDecodeLimits {
    /// Maximum complete input slice length in bytes.
    pub max_input_bytes: usize,
    /// Maximum number of values across the entire input.
    pub max_value_count: usize,
    /// Maximum entries per array or key/value pairs per map.
    pub max_collection_entries: usize,
    /// Maximum bytes per byte string or UTF-8 text string.
    pub max_string_bytes: usize,
    /// Maximum declared parent digests in an episode envelope.
    pub max_parent_count: usize,
}
impl Default for CanonicalDecodeLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 1_048_576,
            max_value_count: 65_536,
            max_collection_entries: 16_384,
            max_string_bytes: 1_048_576,
            max_parent_count: 4_096,
        }
    }
}

/// Content-free syntax, shape, and resource failures from restricted decoding.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum CanonicalDecodeError {
    /// Input exceeds reader policy.
    #[error("input byte limit exceeded")]
    InputLimit,
    /// Total values exceed reader policy.
    #[error("value count limit exceeded")]
    ValueLimit,
    /// A collection exceeds reader policy.
    #[error("collection entry limit exceeded")]
    CollectionLimit,
    /// A string exceeds reader policy.
    #[error("string byte limit exceeded")]
    StringLimit,
    /// Episode parents exceed reader policy.
    #[error("parent count limit exceeded")]
    ParentLimit,
    /// Input ends before a complete item.
    #[error("unexpected end of input")]
    UnexpectedEnd,
    /// Input contains bytes after the root item.
    #[error("trailing input bytes")]
    TrailingBytes,
    /// Reserved or indefinite additional information is inadmissible.
    #[error("invalid additional information")]
    InvalidAdditionalInformation,
    /// An integer or length does not use the shortest width.
    #[error("nonminimal argument encoding")]
    NonMinimalArgument,
    /// The item type is outside the restricted profile.
    #[error("unsupported canonical value type")]
    UnsupportedType,
    /// Text is not UTF-8.
    #[error("invalid UTF-8 text")]
    InvalidUtf8,
    /// A negative integer is outside the i64 domain.
    #[error("integer out of range")]
    IntegerOutOfRange,
    /// An item exceeds profile depth 32.
    #[error("canonical value exceeds nesting limit")]
    NestingLimit,
    /// A length or checked size cannot be represented.
    #[error("length out of range")]
    LengthOutOfRange,
    /// Fallible owned storage reservation failed.
    #[error("allocation reservation failed")]
    AllocationFailure,
    /// Two map keys have equal canonical encodings.
    #[error("duplicate canonical map key")]
    DuplicateMapKey,
    /// Encoded map keys are not length-first ordered.
    #[error("noncanonical map key order")]
    MapKeyOrder,
    /// Root is not the frozen five-member envelope.
    #[error("invalid episode envelope")]
    InvalidEnvelope,
    /// Profile/domain name is not recognized.
    #[error("episode profile mismatch")]
    ProfileMismatch,
    /// Schema version zero is reserved.
    #[error("schema version must be non-zero")]
    ZeroSchemaVersion,
    /// Header is not a map.
    #[error("commitment header must be a map")]
    HeaderMustBeMap,
    /// Payload is not a map.
    #[error("commitment payload must be a map")]
    PayloadMustBeMap,
    /// A parent is not a 32-byte byte string.
    #[error("invalid parent digest")]
    InvalidParentDigest,
    /// Parent digests are not nondecreasing.
    #[error("noncanonical parent order")]
    ParentOrder,
}
use CanonicalDecodeError as E;

impl CanonicalValue {
    /// Decode one complete restricted canonical value without normalizing bytes.
    ///
    /// This validates syntax only, not episode semantics, trust, or admission.
    pub fn from_canonical_bytes(input: &[u8], limits: CanonicalDecodeLimits) -> Result<Self, E> {
        Reader::decode(input, limits, false)
    }
}
impl EpisodeCommitment {
    /// Decode the exact frozen envelope, including sorted, possibly duplicate parents.
    ///
    /// Nonzero future schema numbers are syntactically accepted. Success makes no
    /// claim about schema semantics, signature validity, authorization, or admission.
    pub fn from_canonical_bytes(input: &[u8], limits: CanonicalDecodeLimits) -> Result<Self, E> {
        let CanonicalValue::Map(entries) = Reader::decode(input, limits, true)? else {
            return Err(E::InvalidEnvelope);
        };
        if entries.len() != 5 {
            return Err(E::InvalidEnvelope);
        }
        let mut members = entries.into_iter();
        let mut take = |expected| {
            let (key, value) = members.next().ok_or(E::InvalidEnvelope)?;
            if key != CanonicalValue::Unsigned(expected) {
                return Err(E::InvalidEnvelope);
            }
            Ok(value)
        };
        if take(0)? != CanonicalValue::Text(PROFILE_NAME.into()) {
            return Err(E::ProfileMismatch);
        }
        let CanonicalValue::Unsigned(schema_version) = take(1)? else {
            return Err(E::InvalidEnvelope);
        };
        if schema_version == 0 {
            return Err(E::ZeroSchemaVersion);
        }
        let header = take(2)?;
        if !matches!(header, CanonicalValue::Map(_)) {
            return Err(E::HeaderMustBeMap);
        }
        let payload = take(3)?;
        if !matches!(payload, CanonicalValue::Map(_)) {
            return Err(E::PayloadMustBeMap);
        }
        let CanonicalValue::Array(parents) = take(4)? else {
            return Err(E::InvalidEnvelope);
        };
        let mut parent_digests = Vec::new();
        parent_digests
            .try_reserve_exact(parents.len())
            .map_err(|_| E::AllocationFailure)?;
        for parent in parents {
            let CanonicalValue::Bytes(bytes) = parent else {
                return Err(E::InvalidParentDigest);
            };
            let digest: [u8; 32] = bytes.try_into().map_err(|_| E::InvalidParentDigest)?;
            if parent_digests
                .last()
                .is_some_and(|previous| previous > &digest)
            {
                return Err(E::ParentOrder);
            }
            parent_digests.push(digest);
        }
        Ok(Self::new(schema_version, header, payload, parent_digests))
    }
}

struct Reader<'a> {
    input: &'a [u8],
    cursor: usize,
    remaining_values: usize,
    limits: CanonicalDecodeLimits,
    episode: bool,
}
impl<'a> Reader<'a> {
    fn decode(
        input: &'a [u8],
        limits: CanonicalDecodeLimits,
        episode: bool,
    ) -> Result<CanonicalValue, E> {
        if input.len() > limits.max_input_bytes {
            return Err(E::InputLimit);
        }
        let mut reader = Self {
            input,
            cursor: 0,
            remaining_values: limits.max_value_count,
            limits,
            episode,
        };
        let value = reader.value(0, false)?;
        if reader.cursor != input.len() {
            return Err(E::TrailingBytes);
        }
        Ok(value)
    }
    fn bytes(&mut self, length: usize) -> Result<&'a [u8], E> {
        let end = self.cursor.checked_add(length).ok_or(E::LengthOutOfRange)?;
        let bytes = self.input.get(self.cursor..end).ok_or(E::UnexpectedEnd)?;
        self.cursor = end;
        Ok(bytes)
    }
    fn argument(&mut self, info: u8) -> Result<u64, E> {
        let (length, minimum) = match info {
            0..=23 => return Ok(u64::from(info)),
            24 => (1, 24),
            25 => (2, 256),
            26 => (4, 65536),
            27 => (8, 0x1_0000_0000),
            _ => return Err(E::InvalidAdditionalInformation),
        };
        let mut argument = 0_u64;
        for byte in self.bytes(length)? {
            argument = (argument << 8) | u64::from(*byte);
        }
        if argument < minimum {
            return Err(E::NonMinimalArgument);
        }
        Ok(argument)
    }
    fn collection(&self, length: usize, multiplier: usize) -> Result<(), E> {
        if length > self.limits.max_collection_entries {
            return Err(E::CollectionLimit);
        }
        let children = length.checked_mul(multiplier).ok_or(E::LengthOutOfRange)?;
        if children > self.remaining_values {
            return Err(E::ValueLimit);
        }
        // Every child needs at least one byte. Reject impossible declarations
        // before reserving collection storage, even under permissive policy.
        if children > self.input.len() - self.cursor {
            return Err(E::UnexpectedEnd);
        }
        Ok(())
    }
    fn value(&mut self, depth: usize, parents: bool) -> Result<CanonicalValue, E> {
        if depth > MAX_NESTING_DEPTH {
            return Err(E::NestingLimit);
        }
        self.remaining_values = self.remaining_values.checked_sub(1).ok_or(E::ValueLimit)?;
        let initial = self.bytes(1)?[0];
        let major = initial >> 5;
        let info = initial & 31;
        if major == 7 {
            return match initial {
                0xf4 => Ok(CanonicalValue::Bool(false)),
                0xf5 => Ok(CanonicalValue::Bool(true)),
                0xf6 => Ok(CanonicalValue::Null),
                _ => Err(E::UnsupportedType),
            };
        }
        if major == 6 {
            return Err(E::UnsupportedType);
        }
        let argument = self.argument(info)?;
        match major {
            0 => Ok(CanonicalValue::Unsigned(argument)),
            1 => {
                let integer = i64::try_from(argument).map_err(|_| E::IntegerOutOfRange)?;
                Ok(CanonicalValue::Negative(-1 - integer))
            }
            2 | 3 => {
                let length = usize::try_from(argument).map_err(|_| E::LengthOutOfRange)?;
                if length > self.limits.max_string_bytes {
                    return Err(E::StringLimit);
                }
                let bytes = self.bytes(length)?;
                if major == 2 {
                    let mut owned = Vec::new();
                    owned
                        .try_reserve_exact(length)
                        .map_err(|_| E::AllocationFailure)?;
                    owned.extend_from_slice(bytes);
                    Ok(CanonicalValue::Bytes(owned))
                } else {
                    let text = core::str::from_utf8(bytes).map_err(|_| E::InvalidUtf8)?;
                    let mut owned = String::new();
                    owned
                        .try_reserve_exact(length)
                        .map_err(|_| E::AllocationFailure)?;
                    owned.push_str(text);
                    Ok(CanonicalValue::Text(owned))
                }
            }
            4 => {
                // Apply the episode-only declared count before reserving or
                // parsing children; generic values never receive this context.
                if parents
                    && argument > u64::try_from(self.limits.max_parent_count).unwrap_or(u64::MAX)
                {
                    return Err(E::ParentLimit);
                }
                let length = usize::try_from(argument).map_err(|_| E::LengthOutOfRange)?;
                self.collection(length, 1)?;
                let mut values = Vec::new();
                values
                    .try_reserve_exact(length)
                    .map_err(|_| E::AllocationFailure)?;
                for _ in 0..length {
                    values.push(self.value(depth + 1, false)?);
                }
                Ok(CanonicalValue::Array(values))
            }
            5 => {
                let length = usize::try_from(argument).map_err(|_| E::LengthOutOfRange)?;
                self.collection(length, 2)?;
                let mut entries = Vec::new();
                entries
                    .try_reserve_exact(length)
                    .map_err(|_| E::AllocationFailure)?;
                let mut previous: Option<&[u8]> = None;
                for _ in 0..length {
                    let start = self.cursor;
                    let key = self.value(depth + 1, false)?;
                    // Keep key encodings input-backed: compound keys require
                    // no reencoding or ancestor-sized temporary buffers.
                    let raw = &self.input[start..self.cursor];
                    if let Some(prior) = previous {
                        match prior.len().cmp(&raw.len()).then_with(|| prior.cmp(raw)) {
                            Ordering::Equal => return Err(E::DuplicateMapKey),
                            Ordering::Greater => return Err(E::MapKeyOrder),
                            Ordering::Less => {}
                        }
                    }
                    previous = Some(raw);
                    let parent_array =
                        self.episode && depth == 0 && key == CanonicalValue::Unsigned(4);
                    let value = self.value(depth + 1, parent_array)?;
                    entries.push((key, value));
                }
                Ok(CanonicalValue::Map(entries))
            }
            _ => Err(E::UnsupportedType),
        }
    }
}
