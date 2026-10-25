//! Deterministic corpus counts, lexicographic ties and bounded merge work.
use super::{ByteBpe, Result, TokenizerLimits, cancel, charge, require, vector};
use crate::model::{
    contract::{
        BPE_CORPUS_DOMAIN, TOKENIZER_ID,
        tokenizer::{Merge, Normalization, SpecialPolicy, Tokenizer},
    },
    error::ErrorKind as E,
    ids::{Digest, TokenId},
    limits::{
        MAX_CORPUS_BYTES, MAX_DOCUMENT_BYTES, MAX_TOKEN_BYTES, MAX_TOKENIZER_BYTES,
        MAX_TRAINING_DOCUMENTS,
    },
};
use sha2::{Digest as _, Sha256};
impl ByteBpe {
    /// Train on already admitted training-only UTF-8 documents. Never chooses splits.
    pub fn train(
        documents: &[&str],
        split: Digest,
        requested: u32,
        limits: TokenizerLimits,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Self> {
        let limits = limits.effective();
        cancel(&mut cancelled)?;
        require(split.0 != [0; 32], E::InvalidTokenizer, "training-split")?;
        require(
            (260..=4096).contains(&requested),
            E::InvalidTokenizer,
            "requested-vocab",
        )?;
        let (mut docs, corpus, total) = prepare(documents, limits, &mut cancelled)?;
        let mut vocabulary = vector(requested as usize)?;
        for _ in 0..4 {
            vocabulary.push(Vec::new());
        }
        for b in 0..=255 {
            let mut bytes = vector(1)?;
            bytes.push(b);
            vocabulary.push(bytes);
        }
        let mut merges = vector((requested - 260) as usize)?;
        let mut pairs = vector(super::super::package::size(total)?)?;
        let mut visits = 0;
        let mut component_bytes = 4 + TOKENIZER_ID.len() + 4 + 16 + 64 + 8 + 4 + 4 + 260 * 4 + 256;
        while vocabulary.len() < requested as usize {
            cancel(&mut cancelled)?;
            let Some(pair) = best_pair(
                &docs,
                &mut pairs,
                &mut visits,
                limits.trainer_visits,
                &mut cancelled,
            )?
            else {
                break;
            };
            let left = u32::try_from(pair >> 32).expect("packed left token");
            let right = u32::try_from(pair & u64::from(u32::MAX)).expect("packed right token");
            let result = u32::try_from(vocabulary.len()).expect("bounded vocabulary");
            let a = &vocabulary[left as usize];
            let b = &vocabulary[right as usize];
            let len = a.len() + b.len();
            require(
                len as u64 <= MAX_TOKEN_BYTES,
                E::CapacityExceeded,
                "trained-token-size",
            )?;
            component_bytes += 4 + len + 12;
            require(
                component_bytes as u64 <= MAX_TOKENIZER_BYTES,
                E::CapacityExceeded,
                "trained-tokenizer-size",
            )?;
            let mut joined = vector(len)?;
            joined.extend_from_slice(a);
            joined.extend_from_slice(b);
            vocabulary.push(joined);
            merges.push(Merge {
                left: TokenId(left),
                right: TokenId(right),
                result: TokenId(result),
            });
            for doc in &mut docs {
                cancel(&mut cancelled)?;
                let (mut read, mut write) = (0, 0);
                while read < doc.len() {
                    let matched =
                        read + 1 < doc.len() && doc[read] == left && doc[read + 1] == right;
                    let consumed = if matched { 2 } else { 1 };
                    charge(&mut visits, consumed, limits.trainer_visits)?;
                    doc[write] = if matched { result } else { doc[read] };
                    read += usize::try_from(consumed).expect("one or two consumed tokens");
                    write += 1;
                }
                doc.truncate(write);
            }
        }
        cancel(&mut cancelled)?;
        Self::from_contract(
            Tokenizer {
                identity: TOKENIZER_ID.to_owned(),
                normalization: Normalization::None,
                special_policy: SpecialPolicy::TypedStructural,
                reserved: 0,
                pad_id: TokenId(0),
                bos_id: TokenId(1),
                eos_id: TokenId(2),
                unk_id: TokenId(3),
                training_corpus_digest: corpus,
                training_split_digest: split,
                minimum_pair_frequency: 2,
                requested_vocab_size: requested,
                vocabulary,
                merges,
            },
            limits,
        )
    }
}

fn prepare(
    documents: &[&str],
    limits: TokenizerLimits,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<(Vec<Vec<u32>>, Digest, u64)> {
    require(
        documents.len() as u64 <= MAX_TRAINING_DOCUMENTS,
        E::CapacityExceeded,
        "document-count",
    )?;
    let mut total = 0u64;
    for document in documents {
        require(
            document.len() as u64 <= MAX_DOCUMENT_BYTES,
            E::CapacityExceeded,
            "document-size",
        )?;
        total = total
            .checked_add(document.len() as u64)
            .ok_or_else(|| super::err(E::CapacityExceeded, "corpus-size"))?;
    }
    require(
        total <= MAX_CORPUS_BYTES,
        E::CapacityExceeded,
        "corpus-size",
    )?;
    // Token rows + packed adjacent pairs + vector headers + full vocabulary +
    // conservative reserve. Sorting the packed pairs allocates no scratch heap.
    let estimate = total
        .checked_mul(16)
        .and_then(|n| n.checked_add(documents.len() as u64 * 64 + 4 * MAX_TOKENIZER_BYTES));
    require(
        estimate.is_some_and(|n| n <= limits.training_bytes),
        E::CapacityExceeded,
        "training-allocation-preflight",
    )?;
    let mut hash = Sha256::new();
    hash.update(BPE_CORPUS_DOMAIN);
    hash.update(
        u32::try_from(documents.len())
            .expect("bounded documents")
            .to_le_bytes(),
    );
    let mut docs = vector(documents.len())?;
    for document in documents {
        cancel(cancelled)?;
        hash.update(
            u32::try_from(document.len())
                .expect("bounded document")
                .to_le_bytes(),
        );
        hash.update(document.as_bytes());
        let mut ids = vector(document.len())?;
        ids.extend(document.bytes().map(|b| u32::from(b) + 4));
        docs.push(ids);
    }
    let corpus = Digest(hash.finalize().into());
    Ok((docs, corpus, total))
}

fn best_pair(
    docs: &[Vec<u32>],
    pairs: &mut Vec<u64>,
    visits: &mut u64,
    max_visits: u64,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<Option<u64>> {
    pairs.clear();
    for doc in docs {
        cancel(cancelled)?;
        for pair in doc.windows(2) {
            charge(visits, 1, max_visits)?;
            pairs.push((u64::from(pair[0]) << 32) | u64::from(pair[1]));
        }
    }
    pairs.sort_unstable();
    let mut best = None;
    let mut i = 0;
    while i < pairs.len() {
        let key = pairs[i];
        let start = i;
        i += 1;
        while i < pairs.len() && pairs[i] == key {
            i += 1;
        }
        let count = u64::try_from(i - start).expect("bounded corpus pair count");
        if count >= 2 && best.is_none_or(|(_, n)| count > n) {
            best = Some((key, count));
        }
    }
    Ok(best.map(|(pair, _)| pair))
}
