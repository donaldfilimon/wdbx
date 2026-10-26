//! Tokenizer schema 1.0; imported provenance is never synthesized.
use super::{Cursor, Result};
use crate::model::{
    contract::tokenizer::{Merge, Tokenizer},
    ids::TokenId,
};
pub(super) fn parse(r: &mut Cursor<'_, '_>) -> Result<Tokenizer> {
    r.tokenizer_cap()?;
    let identity = r.string(4096)?;
    let normalization = r.e8()?;
    let special_policy = r.e8()?;
    let reserved = r.u16()?;
    let pad_id = TokenId(r.u32()?);
    let bos_id = TokenId(r.u32()?);
    let eos_id = TokenId(r.u32()?);
    let unk_id = TokenId(r.u32()?);
    let training_corpus_digest = r.digest()?;
    let training_split_digest = r.digest()?;
    let minimum_pair_frequency = r.u32()?;
    let requested_vocab_size = r.u32()?;
    let n = r.count(4096)?;
    let mut vocabulary = r.vec(n)?;
    for _ in 0..n {
        vocabulary.push(r.blob(65536, true)?);
    }
    let n = r.count(3836)?;
    let mut merges = r.vec(n)?;
    for _ in 0..n {
        merges.push(Merge {
            left: TokenId(r.u32()?),
            right: TokenId(r.u32()?),
            result: TokenId(r.u32()?),
        });
    }
    r.end()?;
    let tokenizer = Tokenizer {
        identity,
        normalization,
        special_policy,
        reserved,
        pad_id,
        bos_id,
        eos_id,
        unk_id,
        training_corpus_digest,
        training_split_digest,
        minimum_pair_frequency,
        requested_vocab_size,
        vocabulary,
        merges,
    };
    crate::model::tokenizer::validate_contract(&tokenizer)?;
    Ok(tokenizer)
}
