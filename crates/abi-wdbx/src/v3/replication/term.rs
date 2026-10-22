//! Fencing epochs and voter identities.
//!
//! The term is the shard's fence. It only ever increases, it is durable, and
//! every acknowledgement carries the term it was accepted under. A writer
//! that presents an older term is refused; so is an acknowledgement that
//! carries one.

use crate::v3::replication::error::ShardError;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;

/// Maximum length of a bounded identifier.
const MAX_IDENTIFIER: usize = 64;

/// A fencing epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShardTerm(u64);

impl ShardTerm {
    /// The epoch a fresh shard starts at.
    pub(crate) const FIRST: ShardTerm = ShardTerm(1);

    /// The numeric epoch.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// The next epoch, saturating at the maximum.
    pub(crate) const fn next(self) -> ShardTerm {
        ShardTerm(self.0.saturating_add(1))
    }

    /// Rebuild an epoch from durable state.
    pub(crate) fn from_u64(value: u64) -> Result<Self, ShardError> {
        if value == 0 {
            return Err(ShardError::Corrupt);
        }
        Ok(ShardTerm(value))
    }
}

impl fmt::Display for ShardTerm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "term_{}", self.0)
    }
}

impl Serialize for ShardTerm {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.0)
    }
}

/// A bounded voter identity inside one shard.
///
/// The identity is a label, not a credential: it says which member of the
/// shard acknowledged a write, and nothing else.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VoterId(String);

impl VoterId {
    /// Build a bounded voter identity.
    pub fn new(value: &str) -> Result<Self, ShardError> {
        if !bounded_identifier(value, MAX_IDENTIFIER) {
            return Err(ShardError::InvalidInput);
        }
        Ok(Self(value.to_string()))
    }

    /// The identity as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VoterId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Whether a value is a bounded, opaque identifier.
///
/// The alphabet is the store's: lowercase letters, digits, `_`, `-`, and
/// `.`. A scope or voter identity therefore never carries a path separator,
/// a space, or an uppercase letter.
pub(super) fn bounded_identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
        })
}
