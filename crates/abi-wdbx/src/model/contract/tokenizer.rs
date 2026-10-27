//! Component 4: byte BPE metadata. Actual tokenizer implementation is N02.
use crate::model::ids::{Digest, TokenId};

/// Byte identity profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Normalization {
    /// Exact UTF-8 input bytes, with no normalization.
    None = 0,
}
impl TryFrom<u8> for Normalization {
    type Error = super::super::error::ModelError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::InvalidTokenizer,
                "Normalization",
                None,
                None,
            )),
        }
    }
}

/// Structural IDs cannot be recognized from text spellings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SpecialPolicy {
    /// Only typed sequence helpers insert structural tokens.
    TypedStructural = 1,
}
impl TryFrom<u8> for SpecialPolicy {
    type Error = super::super::error::ModelError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::TypedStructural),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::InvalidTokenizer,
                "SpecialPolicy",
                None,
                None,
            )),
        }
    }
}

/// Rank is the array index; duplicate output bytes are legal.
#[derive(Clone, Debug, PartialEq)]
pub struct Merge {
    /// 4 <= left < result.
    pub left: TokenId,
    /// 4 <= right < result; pair unique among prior ranks.
    pub right: TokenId,
    /// Exactly 260 + rank; bytes equal concatenated parents.
    pub result: TokenId,
}

/// Unvalidated component 4 data in wire order.
#[derive(Clone, Debug, PartialEq)]
pub struct Tokenizer {
    /// Exactly `TOKENIZER_ID`.
    pub identity: String,
    /// Exact input bytes.
    pub normalization: Normalization,
    /// Structural tokens only through typed API.
    pub special_policy: SpecialPolicy,
    /// Must be zero.
    pub reserved: u16,
    /// Exactly 0.
    pub pad_id: TokenId,
    /// Exactly 1.
    pub bos_id: TokenId,
    /// Exactly 2.
    pub eos_id: TokenId,
    /// Exactly 3.
    pub unk_id: TokenId,
    /// Domain-separated ordered training documents; never fabricated from vocabulary.
    pub training_corpus_digest: Digest,
    /// Nonzero externally supplied admitted training-split identity.
    pub training_split_digest: Digest,
    /// Exactly 2.
    pub minimum_pair_frequency: u32,
    /// Actual V..=4096.
    pub requested_vocab_size: u32,
    /// ID is index; first four empty, 256 byte rows, then ranked merge rows.
    pub vocabulary: Vec<Vec<u8>>,
    /// Exactly V-260 ordered merges. No deduplication of equal byte strings.
    pub merges: Vec<Merge>,
}
