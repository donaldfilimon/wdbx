//! Deterministic canonical-CBOR commitments for v3 episode envelopes.
//!
//! The `abbey-cbor-episode-v1` profile is a deliberately restricted form of
//! RFC 8949 deterministic encoding:
//!
//! - integers and lengths use their shortest encoding;
//! - byte strings, UTF-8 text strings, arrays, and maps always have definite
//!   lengths;
//! - map keys are ordered first by their encoded length and then by their
//!   encoded bytes, per RFC 8949 section 4.2.1;
//! - duplicate encoded map keys are rejected;
//! - floating-point values, tags, indefinite-length items, and `undefined` are
//!   outside this profile;
//! - text is encoded as its supplied UTF-8 bytes without Unicode normalization;
//!   schemas must define any required normalization before constructing values;
//! - identifiers and integers outside the supported signed/unsigned 64-bit
//!   domains must be schema-defined text strings;
//! - omission of a map member means absent; an encoded zero or null is present.
//!
//! The committed envelope is a five-entry map with frozen integer keys:
//!
//! ```text
//! 0: "abbey-cbor-episode-v1"  # profile and domain separator
//! 1: schema_version           # non-zero unsigned integer
//! 2: header                   # canonical map
//! 3: payload                  # canonical map
//! 4: parent_digests           # lexicographically sorted byte strings
//! ```
//!
//! The digest is exactly SHA-256 of those canonical envelope bytes. There is
//! no hidden prefix, native struct layout, wire encoder output, signature, or
//! store operation. Existing v2 commitments are a separate frozen domain.

use sha2::{Digest, Sha256};
use thiserror::Error;

/// Name of the deterministic CBOR profile and its envelope domain separator.
pub const PROFILE_NAME: &str = "abbey-cbor-episode-v1";

/// Maximum supported nesting depth for caller-supplied header and payload maps.
pub const MAX_NESTING_DEPTH: usize = 32;

/// A value admitted by the `abbey-cbor-episode-v1` deterministic profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonicalValue {
    /// An unsigned integer in the full CBOR `u64` range.
    Unsigned(u64),
    /// A negative integer. A zero or positive value is rejected during encoding.
    Negative(i64),
    /// A definite-length byte string.
    Bytes(Vec<u8>),
    /// A definite-length UTF-8 text string.
    Text(String),
    /// A definite-length array.
    Array(Vec<Self>),
    /// A definite-length map. Duplicate canonical key encodings are rejected.
    Map(Vec<(Self, Self)>),
    /// A Boolean value.
    Bool(bool),
    /// The CBOR null value. Null is present and is distinct from an omitted map member.
    Null,
}

impl CanonicalValue {
    /// Encode this value under the restricted RFC 8949 deterministic profile.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CanonicalCborError> {
        let mut output = Vec::new();
        encode_value(self, 0, &mut output)?;
        Ok(output)
    }
}

/// Content-free failures from canonical-CBOR validation and encoding.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum CanonicalCborError {
    /// Schema version zero is reserved and cannot identify a committed episode.
    #[error("schema version must be non-zero")]
    ZeroSchemaVersion,
    /// The commitment header must be represented as a canonical map.
    #[error("commitment header must be a map")]
    HeaderMustBeMap,
    /// The commitment payload must be represented as a canonical map.
    #[error("commitment payload must be a map")]
    PayloadMustBeMap,
    /// The `Negative` variant contained zero or a positive value.
    #[error("negative integer variant must contain a negative value")]
    NonNegativeValue,
    /// Two map keys have identical deterministic CBOR encodings.
    #[error("duplicate canonical map key")]
    DuplicateMapKey,
    /// The configured finite nesting boundary was exceeded.
    #[error("canonical value exceeds the nesting limit")]
    NestingLimit,
    /// A collection length cannot be represented by the deterministic profile.
    #[error("canonical value length is out of range")]
    LengthOutOfRange,
}

/// Pure v3 episode commitment input.
///
/// This type owns only an encoding input. It does not represent the complete
/// constitutional `EpisodeBlock`, perform signing, or open a WDBX store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpisodeCommitment {
    schema_version: u64,
    header: CanonicalValue,
    payload: CanonicalValue,
    parent_digests: Vec<[u8; 32]>,
}

impl EpisodeCommitment {
    /// Construct a pure commitment input.
    ///
    /// Validation occurs when canonical bytes or the digest are requested so
    /// callers receive one content-free error surface for both operations.
    #[must_use]
    pub const fn new(
        schema_version: u64,
        header: CanonicalValue,
        payload: CanonicalValue,
        parent_digests: Vec<[u8; 32]>,
    ) -> Self {
        Self {
            schema_version,
            header,
            payload,
            parent_digests,
        }
    }

    /// Encode the exact domain-separated deterministic-CBOR envelope.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CanonicalCborError> {
        if self.schema_version == 0 {
            return Err(CanonicalCborError::ZeroSchemaVersion);
        }
        if !matches!(self.header, CanonicalValue::Map(_)) {
            return Err(CanonicalCborError::HeaderMustBeMap);
        }
        if !matches!(self.payload, CanonicalValue::Map(_)) {
            return Err(CanonicalCborError::PayloadMustBeMap);
        }

        let mut parents = self.parent_digests.clone();
        parents.sort_unstable();
        let parent_values = parents
            .into_iter()
            .map(|digest| CanonicalValue::Bytes(digest.to_vec()))
            .collect();
        let envelope = CanonicalValue::Map(vec![
            (
                CanonicalValue::Unsigned(0),
                CanonicalValue::Text(PROFILE_NAME.into()),
            ),
            (
                CanonicalValue::Unsigned(1),
                CanonicalValue::Unsigned(self.schema_version),
            ),
            (CanonicalValue::Unsigned(2), self.header.clone()),
            (CanonicalValue::Unsigned(3), self.payload.clone()),
            (
                CanonicalValue::Unsigned(4),
                CanonicalValue::Array(parent_values),
            ),
        ]);

        envelope.canonical_bytes()
    }

    /// Compute SHA-256 over the exact canonical envelope bytes.
    pub fn digest(&self) -> Result<[u8; 32], CanonicalCborError> {
        let bytes = self.canonical_bytes()?;
        Ok(Sha256::digest(bytes).into())
    }
}

fn encode_value(
    value: &CanonicalValue,
    depth: usize,
    output: &mut Vec<u8>,
) -> Result<(), CanonicalCborError> {
    if depth > MAX_NESTING_DEPTH {
        return Err(CanonicalCborError::NestingLimit);
    }

    match value {
        CanonicalValue::Unsigned(integer) => encode_argument(0, *integer, output),
        CanonicalValue::Negative(integer) => {
            if *integer >= 0 {
                return Err(CanonicalCborError::NonNegativeValue);
            }
            let argument = u64::try_from(-1_i128 - i128::from(*integer))
                .expect("every negative i64 maps to a u64 CBOR argument");
            encode_argument(1, argument, output);
        }
        CanonicalValue::Bytes(bytes) => {
            encode_length(2, bytes.len(), output)?;
            output.extend_from_slice(bytes);
        }
        CanonicalValue::Text(text) => {
            encode_length(3, text.len(), output)?;
            output.extend_from_slice(text.as_bytes());
        }
        CanonicalValue::Array(values) => {
            encode_length(4, values.len(), output)?;
            for item in values {
                encode_value(item, depth + 1, output)?;
            }
        }
        CanonicalValue::Map(entries) => encode_map(entries, depth, output)?,
        CanonicalValue::Bool(false) => output.push(0xf4),
        CanonicalValue::Bool(true) => output.push(0xf5),
        CanonicalValue::Null => output.push(0xf6),
    }
    Ok(())
}

fn encode_map(
    entries: &[(CanonicalValue, CanonicalValue)],
    depth: usize,
    output: &mut Vec<u8>,
) -> Result<(), CanonicalCborError> {
    let mut encoded = Vec::with_capacity(entries.len());
    for (key, value) in entries {
        let mut key_bytes = Vec::new();
        let mut value_bytes = Vec::new();
        encode_value(key, depth + 1, &mut key_bytes)?;
        encode_value(value, depth + 1, &mut value_bytes)?;
        encoded.push((key_bytes, value_bytes));
    }
    encoded.sort_by(|left, right| {
        left.0
            .len()
            .cmp(&right.0.len())
            .then_with(|| left.0.cmp(&right.0))
    });
    if encoded.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(CanonicalCborError::DuplicateMapKey);
    }

    encode_length(5, encoded.len(), output)?;
    for (key, value) in encoded {
        output.extend_from_slice(&key);
        output.extend_from_slice(&value);
    }
    Ok(())
}

fn encode_length(major: u8, length: usize, output: &mut Vec<u8>) -> Result<(), CanonicalCborError> {
    let argument = u64::try_from(length).map_err(|_| CanonicalCborError::LengthOutOfRange)?;
    encode_argument(major, argument, output);
    Ok(())
}

fn encode_argument(major: u8, argument: u64, output: &mut Vec<u8>) {
    let major_bits = major << 5;
    match argument {
        0..=23 => output.push(major_bits | u8::try_from(argument).expect("argument is bounded")),
        24..=0xff => {
            output.push(major_bits | 0x18);
            output.push(u8::try_from(argument).expect("argument is bounded"));
        }
        0x100..=0xffff => {
            output.push(major_bits | 0x19);
            output.extend_from_slice(
                &u16::try_from(argument)
                    .expect("argument is bounded")
                    .to_be_bytes(),
            );
        }
        0x1_0000..=0xffff_ffff => {
            output.push(major_bits | 0x1a);
            output.extend_from_slice(
                &u32::try_from(argument)
                    .expect("argument is bounded")
                    .to_be_bytes(),
            );
        }
        _ => {
            output.push(major_bits | 0x1b);
            output.extend_from_slice(&argument.to_be_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferred_integer_encodings_cover_boundaries() {
        let cases = [
            (CanonicalValue::Unsigned(23), vec![0x17]),
            (CanonicalValue::Unsigned(24), vec![0x18, 0x18]),
            (CanonicalValue::Unsigned(255), vec![0x18, 0xff]),
            (CanonicalValue::Unsigned(256), vec![0x19, 0x01, 0x00]),
            (CanonicalValue::Negative(-1), vec![0x20]),
            (CanonicalValue::Negative(-25), vec![0x38, 0x18]),
        ];

        for (value, expected) in cases {
            assert_eq!(
                value.canonical_bytes().expect("canonical integer"),
                expected
            );
        }
    }

    #[test]
    fn non_negative_negative_variant_is_rejected() {
        assert_eq!(
            CanonicalValue::Negative(0).canonical_bytes(),
            Err(CanonicalCborError::NonNegativeValue)
        );
    }

    #[test]
    fn deeply_nested_values_are_rejected() {
        let mut value = CanonicalValue::Null;
        for _ in 0..=MAX_NESTING_DEPTH {
            value = CanonicalValue::Array(vec![value]);
        }

        assert_eq!(
            value.canonical_bytes(),
            Err(CanonicalCborError::NestingLimit)
        );
    }
}
