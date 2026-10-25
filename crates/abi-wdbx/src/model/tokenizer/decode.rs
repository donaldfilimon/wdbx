//! Exact raw bytes; UTF-8 presentation is a separate layer.
use super::{ByteBpe, Result, require, vector};
use crate::model::{error::ErrorKind as E, ids::TokenId};
impl ByteBpe {
    /// Decode nonstructural IDs with a bounded output; no lossy replacement.
    pub fn decode_bytes(&self, tokens: &[TokenId]) -> Result<Vec<u8>> {
        require(
            tokens.len() as u64 <= self.limits.decoder_tokens,
            E::CapacityExceeded,
            "decode-token-count",
        )?;
        let mut total = 0usize;
        for id in tokens {
            let i = id.0 as usize;
            require(
                i >= 4 && i < self.contract.vocabulary.len(),
                E::InvalidTokenizer,
                "decode-id",
            )?;
            total = total
                .checked_add(self.contract.vocabulary[i].len())
                .ok_or_else(|| super::err(E::CapacityExceeded, "decode-size"))?;
            require(
                total as u64 <= self.limits.decoder_bytes,
                E::CapacityExceeded,
                "decode-size",
            )?;
        }
        let mut out = vector(total)?;
        for id in tokens {
            out.extend_from_slice(&self.contract.vocabulary[id.0 as usize]);
        }
        Ok(out)
    }
    /// Strip one leading BOS / terminating EOS; PAD is valid only after EOS.
    pub fn decode_sequence_bytes(&self, tokens: &[TokenId]) -> Result<Vec<u8>> {
        require(
            tokens.len() as u64 <= self.limits.decoder_tokens + 2,
            E::CapacityExceeded,
            "decode-sequence-count",
        )?;
        let start = usize::from(tokens.first() == Some(&TokenId(1)));
        let mut end = tokens.len();
        let mut eos = false;
        for (i, id) in tokens.iter().enumerate().skip(start) {
            match id.0 {
                2 => {
                    require(!eos, E::InvalidTokenizer, "internal-eos")?;
                    eos = true;
                    end = i;
                }
                0 => require(eos, E::InvalidTokenizer, "internal-pad")?,
                1 | 3 => return Err(super::err(E::InvalidTokenizer, "structural-id")),
                _ => require(!eos, E::InvalidTokenizer, "content-after-eos")?,
            }
        }
        self.decode_bytes(&tokens[start..end])
    }
}
