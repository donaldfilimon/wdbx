//! Deterministic byte BPE, independent of files, providers and authority services.
mod decode;
mod encode;
mod train;
use super::package::{Result, err, require};
use crate::model::{
    contract::{TOKENIZER_ID, tokenizer::Tokenizer},
    error::{ErrorKind as E, ModelError},
    limits::{
        MAX_DECODE_OUTPUT_BYTES, MAX_DECODE_TOKEN_IDS, MAX_ENCODER_VISITS, MAX_TOKENIZER_BYTES,
        MAX_TOKENIZER_TRAINING_BYTES, MAX_TRAINER_VISITS,
    },
};
/// Local tokenizer ceilings. Requests are clamped to the hard native profile.
#[derive(Clone, Copy, Debug)]
pub struct TokenizerLimits {
    /// Pair-scan plus replacement visits during training.
    pub trainer_visits: u64,
    /// Tokens tested by encoding merge passes.
    pub encoder_visits: u64,
    /// Conservative peak training allocations.
    pub training_bytes: u64,
    /// Raw decode token IDs; structural sequence wrappers are accounted separately.
    pub decoder_tokens: u64,
    /// Decoded byte output, independently capped from raw UTF-8 encode input.
    pub decoder_bytes: u64,
}
impl Default for TokenizerLimits {
    fn default() -> Self {
        Self {
            trainer_visits: MAX_TRAINER_VISITS,
            encoder_visits: MAX_ENCODER_VISITS,
            training_bytes: MAX_TOKENIZER_TRAINING_BYTES,
            decoder_tokens: MAX_DECODE_TOKEN_IDS,
            decoder_bytes: MAX_DECODE_OUTPUT_BYTES,
        }
    }
}
impl TokenizerLimits {
    fn effective(self) -> Self {
        Self {
            trainer_visits: self.trainer_visits.min(MAX_TRAINER_VISITS),
            encoder_visits: self.encoder_visits.min(MAX_ENCODER_VISITS),
            training_bytes: self.training_bytes.min(MAX_TOKENIZER_TRAINING_BYTES),
            decoder_tokens: self.decoder_tokens.min(MAX_DECODE_TOKEN_IDS),
            decoder_bytes: self.decoder_bytes.min(MAX_DECODE_OUTPUT_BYTES),
        }
    }
}
/// Validated vocabulary; no mutable contract or partial training output is exposed.
#[derive(Debug)]
pub struct ByteBpe {
    contract: Tokenizer,
    limits: TokenizerLimits,
}
impl ByteBpe {
    /// Import an explicitly supplied vocabulary/provenance, validating all rows.
    pub fn from_contract(contract: Tokenizer, limits: TokenizerLimits) -> Result<Self> {
        validate_contract(&contract)?;
        Ok(Self {
            contract,
            limits: limits.effective(),
        })
    }
    /// Exact metadata for checkpoint serialization; does not authenticate its sources.
    #[must_use]
    pub fn contract(&self) -> &Tokenizer {
        &self.contract
    }
}
pub(crate) fn validate_contract(t: &Tokenizer) -> Result<()> {
    require(
        t.identity == TOKENIZER_ID
            && t.reserved == 0
            && [t.pad_id.0, t.bos_id.0, t.eos_id.0, t.unk_id.0] == [0, 1, 2, 3]
            && t.minimum_pair_frequency == 2,
        E::InvalidTokenizer,
        "tokenizer-profile",
    )?;
    require(
        t.training_corpus_digest.0 != [0; 32] && t.training_split_digest.0 != [0; 32],
        E::InvalidProvenance,
        "tokenizer-source-digest",
    )?;
    let v = t.vocabulary.len();
    require(
        (260..=4096).contains(&v)
            && v <= t.requested_vocab_size as usize
            && t.requested_vocab_size <= 4096
            && t.merges.len() == v - 260,
        E::InvalidTokenizer,
        "vocab-count",
    )?;
    require(
        t.vocabulary[..4].iter().all(Vec::is_empty)
            && t.vocabulary[4..260]
                .iter()
                .enumerate()
                .all(|(i, b)| b.as_slice() == [u8::try_from(i).expect("256 base bytes")]),
        E::InvalidTokenizer,
        "base-vocabulary",
    )?;
    let mut size = 4 + TOKENIZER_ID.len() + 4 + 16 + 64 + 8 + 4 + 4;
    for bytes in &t.vocabulary {
        require(
            bytes.len() <= 65536,
            E::InvalidTokenizer,
            "token-byte-length",
        )?;
        size += 4 + bytes.len();
    }
    size += t.merges.len() * 12;
    require(
        size as u64 <= MAX_TOKENIZER_BYTES,
        E::CapacityExceeded,
        "tokenizer-component",
    )?;
    for (i, m) in t.merges.iter().enumerate() {
        let out = 260 + i;
        let left = m.left.0 as usize;
        let right = m.right.0 as usize;
        require(
            m.result.0 as usize == out
                && (4..out).contains(&left)
                && (4..out).contains(&right)
                && t.merges[..i]
                    .iter()
                    .all(|p| (p.left, p.right) != (m.left, m.right)),
            E::InvalidTokenizer,
            "merge-order",
        )?;
        let a = &t.vocabulary[left];
        let b = &t.vocabulary[right];
        let bytes = &t.vocabulary[out];
        require(
            a.len() + b.len() <= 65536
                && bytes.len() == a.len() + b.len()
                && bytes.starts_with(a)
                && bytes[a.len()..] == b[..],
            E::InvalidTokenizer,
            "merge-bytes",
        )?;
    }
    Ok(())
}
fn vector<T>(n: usize) -> Result<Vec<T>> {
    let mut out = Vec::new();
    out.try_reserve_exact(n)
        .map_err(|_| err(E::CapacityExceeded, "tokenizer-allocation"))?;
    Ok(out)
}
fn charge(visits: &mut u64, n: u64, max: u64) -> Result<()> {
    *visits = visits
        .checked_add(n)
        .ok_or_else(|| err(E::CapacityExceeded, "tokenizer-work"))?;
    require(*visits <= max, E::CapacityExceeded, "tokenizer-work")
}
fn cancel(check: &mut impl FnMut() -> bool) -> Result<()> {
    if check() {
        Err(ModelError::new(
            E::Cancelled,
            "tokenizer-cancelled",
            None,
            None,
        ))
    } else {
        Ok(())
    }
}
