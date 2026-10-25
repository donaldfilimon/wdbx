//! Independent token IDs, merge choices and work limits for the production BPE.
use abi_wdbx::model::{
    error::ErrorKind,
    ids::{Digest, TokenId},
    tokenizer::{ByteBpe, TokenizerLimits},
};
fn train(docs: &[&str], requested: u32) -> ByteBpe {
    ByteBpe::train(
        docs,
        Digest([7; 32]),
        requested,
        TokenizerLimits::default(),
        || false,
    )
    .unwrap()
}
#[test]
fn ties_overlaps_and_document_boundaries_have_literal_ids() {
    let t = train(&["aaaa", "aaaa"], 262);
    assert_eq!(
        t.contract()
            .merges
            .iter()
            .map(|m| (m.left.0, m.right.0, m.result.0))
            .collect::<Vec<_>>(),
        [(101, 101, 260), (260, 260, 261)]
    );
    assert_eq!(t.encode_utf8("aaaa", || false).unwrap(), [TokenId(261)]);
    assert_eq!(
        t.encode_utf8("aaa", || false).unwrap(),
        [TokenId(260), TokenId(101)]
    );
    let tie = train(&["abab", "cdcd"], 261);
    assert_eq!(
        (
            tie.contract().merges[0].left.0,
            tie.contract().merges[0].right.0
        ),
        (101, 102)
    );
    assert!(train(&["a", "a"], 280).contract().merges.is_empty());
}
#[test]
fn raw_and_structural_decode_are_distinct_and_unicode_is_byte_exact() {
    let t = train(&["éé", "👩‍💻👩‍💻", "e\u{301}e\u{301}"], 280);
    for s in ["", "<BOS><EOS><PAD>", "é e\u{301}\n👩‍💻\t中文"] {
        let ids = t.encode_utf8(s, || false).unwrap();
        assert!(ids.iter().all(|i| i.0 >= 4));
        assert_eq!(t.decode_bytes(&ids).unwrap(), s.as_bytes());
        let seq = t.encode_sequence(s, || false).unwrap();
        assert_eq!(t.decode_sequence_bytes(&seq).unwrap(), s.as_bytes());
    }
    for ids in [vec![TokenId(1)], vec![TokenId(4000)], vec![TokenId(2)]] {
        assert_eq!(
            t.decode_bytes(&ids).unwrap_err().kind,
            ErrorKind::InvalidTokenizer
        );
    }
    for ids in [
        vec![TokenId(1), TokenId(1), TokenId(2)],
        vec![TokenId(101), TokenId(0)],
        vec![TokenId(2), TokenId(101)],
        vec![TokenId(3)],
    ] {
        assert!(t.decode_sequence_bytes(&ids).is_err());
    }
    assert_eq!(
        t.decode_sequence_bytes(&[TokenId(1), TokenId(101), TokenId(2), TokenId(0)])
            .unwrap(),
        b"a"
    );
}
#[test]
fn cancelled_and_bounded_work_never_return_partial_success() {
    assert_eq!(
        ByteBpe::train(
            &["aaaa"],
            Digest([1; 32]),
            300,
            TokenizerLimits::default(),
            || true
        )
        .unwrap_err()
        .kind,
        ErrorKind::Cancelled
    );
    let limits = TokenizerLimits {
        trainer_visits: 2,
        ..TokenizerLimits::default()
    };
    assert_eq!(
        ByteBpe::train(&["aaaa"], Digest([1; 32]), 300, limits, || false)
            .unwrap_err()
            .kind,
        ErrorKind::CapacityExceeded
    );
    let t = ByteBpe::train(
        &["aaaa", "aaaa"],
        Digest([1; 32]),
        262,
        TokenizerLimits {
            encoder_visits: 1,
            ..TokenizerLimits::default()
        },
        || false,
    )
    .unwrap();
    assert_eq!(
        t.encode_utf8("aaaa", || false).unwrap_err().kind,
        ErrorKind::CapacityExceeded
    );
    assert_eq!(
        ByteBpe::train(
            &[],
            Digest([0; 32]),
            260,
            TokenizerLimits::default(),
            || false
        )
        .unwrap_err()
        .kind,
        ErrorKind::InvalidTokenizer
    );
}
#[test]
fn independent_python_training_vectors_pin_merges_corpus_and_exact_token_ids() {
    use sha2::{Digest as _, Sha256};
    let bytes = include_bytes!("fixtures/model-v1/bpe-trainer-vectors-v1.json");
    assert_eq!(
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "04eadb1cd0cc65d43afb764dbe6a622646fb142dd1b93bfc516a0456d7406fbd"
    );
    let rows: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    for row in rows["vectors"].as_array().unwrap() {
        let docs = row["documents"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect::<Vec<_>>();
        let t = train(&docs, row["requested"].as_u64().unwrap() as u32);
        let hex = t
            .contract()
            .training_corpus_digest
            .0
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert_eq!(hex, row["corpus"]);
        let merges = t
            .contract()
            .merges
            .iter()
            .map(|m| vec![m.left.0, m.right.0, m.result.0])
            .collect::<Vec<_>>();
        assert_eq!(serde_json::to_value(merges).unwrap(), row["merges"]);
        for sample in row["samples"].as_array().unwrap() {
            let tokens = t
                .encode_utf8(sample["text"].as_str().unwrap(), || false)
                .unwrap()
                .iter()
                .map(|id| id.0)
                .collect::<Vec<_>>();
            assert_eq!(serde_json::to_value(tokens).unwrap(), sample["tokens"]);
        }
    }
}
#[test]
fn duplicate_token_bytes_from_distinct_ranked_decompositions_remain_legal() {
    use abi_wdbx::model::contract::tokenizer::Merge;
    let mut t = train(&[], 260).contract().clone();
    t.requested_vocab_size = 264;
    t.vocabulary.extend([
        b"ab".to_vec(),
        b"bc".to_vec(),
        b"abc".to_vec(),
        b"abc".to_vec(),
    ]);
    t.merges.extend(
        [
            (101, 102, 260),
            (102, 103, 261),
            (260, 103, 262),
            (101, 261, 263),
        ]
        .map(|(l, r, o)| Merge {
            left: TokenId(l),
            right: TokenId(r),
            result: TokenId(o),
        }),
    );
    let t = ByteBpe::from_contract(t, TokenizerLimits::default()).unwrap();
    assert_eq!(t.encode_utf8("abc", || false).unwrap(), [TokenId(262)]);
    assert_eq!(t.decode_bytes(&[TokenId(263)]).unwrap(), b"abc");
    assert_eq!(t.decode_bytes(&[TokenId(259)]).unwrap(), [255]);
}
#[test]
fn exact_trainer_and_encoder_visit_boundaries_refuse_before_extra_work() {
    let docs = ["aaaa", "aaaa"];
    assert!(
        ByteBpe::train(
            &docs,
            Digest([7; 32]),
            262,
            TokenizerLimits {
                trainer_visits: 20,
                ..TokenizerLimits::default()
            },
            || false
        )
        .is_ok()
    );
    assert_eq!(
        ByteBpe::train(
            &docs,
            Digest([7; 32]),
            262,
            TokenizerLimits {
                trainer_visits: 19,
                ..TokenizerLimits::default()
            },
            || false
        )
        .unwrap_err()
        .kind,
        ErrorKind::CapacityExceeded
    );
    let exact = ByteBpe::train(
        &docs,
        Digest([7; 32]),
        262,
        TokenizerLimits {
            encoder_visits: 3,
            ..TokenizerLimits::default()
        },
        || false,
    )
    .unwrap();
    assert_eq!(exact.encode_utf8("aaaa", || false).unwrap(), [TokenId(261)]);
    let below = ByteBpe::from_contract(
        exact.contract().clone(),
        TokenizerLimits {
            encoder_visits: 2,
            ..TokenizerLimits::default()
        },
    )
    .unwrap();
    assert_eq!(
        below.encode_utf8("aaaa", || false).unwrap_err().kind,
        ErrorKind::CapacityExceeded
    );
    assert_eq!(
        ByteBpe::train(
            &docs,
            Digest([7; 32]),
            262,
            TokenizerLimits {
                training_bytes: 64,
                ..TokenizerLimits::default()
            },
            || false
        )
        .unwrap_err()
        .kind,
        ErrorKind::CapacityExceeded
    );
}
#[test]
fn legal_large_tokens_decode_beyond_raw_encode_input_limit() {
    use abi_wdbx::model::contract::tokenizer::Merge;
    let mut t = train(&[], 260).contract().clone();
    let mut previous = 101;
    for result in 260..276 {
        let mut bytes = t.vocabulary[previous as usize].clone();
        bytes.extend_from_within(..);
        t.vocabulary.push(bytes);
        t.merges.push(Merge {
            left: TokenId(previous),
            right: TokenId(previous),
            result: TokenId(result),
        });
        previous = result;
    }
    t.requested_vocab_size = 276;
    let t = ByteBpe::from_contract(t, TokenizerLimits::default()).unwrap();
    let decoded = t.decode_bytes(&[TokenId(275), TokenId(275)]).unwrap();
    assert_eq!(decoded.len(), 131072);
    assert!(decoded.iter().all(|b| *b == b'a'));
}
