//! Normative rank-order left-to-right merge application.
use super::{ByteBpe, Result, cancel, charge, require, vector};
use crate::model::{error::ErrorKind as E, ids::TokenId, limits::MAX_ENCODE_INPUT_BYTES};
impl ByteBpe {
    /// Encode exact UTF-8 bytes; textual spellings never become structural tokens.
    pub fn encode_utf8(
        &self,
        text: &str,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Vec<TokenId>> {
        require(
            text.len() as u64 <= MAX_ENCODE_INPUT_BYTES,
            E::CapacityExceeded,
            "encode-input",
        )?;
        cancel(&mut cancelled)?;
        let mut tokens = vector(text.len() + 2)?;
        tokens.extend(text.bytes().map(|b| TokenId(u32::from(b) + 4)));
        let mut visits = 0;
        for m in &self.contract.merges {
            cancel(&mut cancelled)?;
            let (mut read, mut write) = (0, 0);
            while read < tokens.len() {
                charge(&mut visits, 1, self.limits.encoder_visits)?;
                if read + 1 < tokens.len() && tokens[read] == m.left && tokens[read + 1] == m.right
                {
                    tokens[write] = m.result;
                    read += 2;
                } else {
                    tokens[write] = tokens[read];
                    read += 1;
                }
                write += 1;
            }
            tokens.truncate(write);
        }
        cancel(&mut cancelled)?;
        Ok(tokens)
    }
    /// A complete typed training sequence, BOS + content + EOS; no truncation.
    pub fn encode_sequence(
        &self,
        text: &str,
        cancelled: impl FnMut() -> bool,
    ) -> Result<Vec<TokenId>> {
        let mut tokens = self.encode_utf8(text, cancelled)?;
        tokens.insert(0, TokenId(1));
        tokens.push(TokenId(2));
        Ok(tokens)
    }
}
