//! Crate-internal tests for the complete-episode ledger.
//!
//! These drive the private store directly, so they can reach the parts a
//! consumer cannot: the typed layer's own validation, the exact on-disk
//! format, and the replay rules. The integration test
//! `tests/v3_complete_episode_ledger.rs` drives the same ledger through the
//! hidden facade.

use super::super::canonical::{Tree, Wire};
use super::super::ledger::{CompleteEpisodeStore, CompleteEpisodeStoreError};
use super::super::{COMPLETE_EPISODE_DOMAIN, MAX_ENCODED_SIZE};
use super::fixtures::*;
use crate::v3::commitment::{CanonicalValue, EpisodeCommitment};
use crate::v3::episode::{EpisodeSigner, SignatureStatus, SignerKeyId};
use ed25519_dalek::{SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// The ledger file this prototype owns, pinned by name.
const LEDGER_FILE: &str = "complete-episodes.v1.jsonl";
/// The writer lock this prototype owns, pinned by name.
const LOCK_FILE: &str = "complete-episodes.v1.lock";

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-complete-episode-ledger-{}", Uuid::new_v4())))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn ledger(&self) -> PathBuf {
        self.0.join(LEDGER_FILE)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        write!(text, "{byte:02x}").expect("writing to a String cannot fail");
        text
    })
}

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).expect("hex"))
        .collect()
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// One hand-written ledger line.
///
/// The test writes the format itself rather than reusing the writer's
/// serializer, so a bug in the writer cannot hide behind the test.
fn line(sequence: u64, previous: Option<&str>, digest: &str, episode: &str) -> String {
    let previous = previous.map_or_else(|| "null".to_owned(), |value| format!("\"{value}\""));
    format!(
        "{{\"domain\":\"{COMPLETE_EPISODE_DOMAIN}\",\"sequence\":{sequence},\
         \"previous_digest\":{previous},\"episode_digest\":\"{digest}\",\
         \"episode\":\"{episode}\"}}"
    )
}

fn signer(seed: u8) -> EpisodeSigner {
    EpisodeSigner::new(SigningKey::from_bytes(&[seed; 32]))
}

fn resolver(keys: &[VerifyingKey]) -> impl Fn(&SignerKeyId) -> Option<VerifyingKey> + '_ {
    |id| {
        keys.iter()
            .copied()
            .find(|key| &SignerKeyId::for_key(key) == id)
    }
}

#[test]
fn an_appended_episode_survives_a_reopen() {
    let scratch = Scratch::new();
    let episode = super::super::CompleteEpisode::try_new(populated()).unwrap();
    let digest = episode.digest().unwrap();
    let receipt = {
        let mut store = CompleteEpisodeStore::open(scratch.path()).unwrap();
        let receipt = store.append(&episode).unwrap();
        assert_eq!(receipt.sequence, 1);
        assert_eq!(receipt.previous_digest, None);
        assert_eq!(receipt.domain, COMPLETE_EPISODE_DOMAIN);
        assert_eq!(receipt.signer_key_id, None);
        assert_eq!(receipt.episode_digest, hex(&digest));
        receipt
    };

    let store = CompleteEpisodeStore::open(scratch.path()).unwrap();
    assert_eq!(store.record_count(), 1);
    assert_eq!(store.records().len(), 1);
    let stored = store.get(&digest).expect("record");
    assert_eq!(
        stored.canonical_bytes().unwrap(),
        episode.canonical_bytes().unwrap()
    );
    assert_eq!(store.find(&digest).unwrap(), receipt);
    assert_eq!(store.receipts(), vec![receipt]);
    assert_eq!(store.retrieve(10).len(), 1);
    assert_eq!(store.get(&[0; 32]), None);
    assert_eq!(store.find(&[0; 32]), None);
}

#[test]
fn a_hand_written_line_is_the_format_the_writer_writes() {
    let scratch = Scratch::new();
    let episode = super::super::CompleteEpisode::try_new(minimal()).unwrap();
    let bytes = episode.canonical_bytes().unwrap();
    let digest = sha256(&bytes);
    {
        let mut store = CompleteEpisodeStore::open(scratch.path()).unwrap();
        store.append(&episode).unwrap();
    }
    let written = fs::read_to_string(scratch.ledger()).unwrap();
    assert_eq!(
        written,
        format!("{}\n", line(1, None, &hex(&digest), &hex(&bytes)))
    );
    // The same record, written with another JSON spelling, replays alike.
    fs::write(
        scratch.ledger(),
        format!(
            "{{ \"sequence\" : 1 , \"domain\" : \"{COMPLETE_EPISODE_DOMAIN}\" , \
             \"episode_digest\" : \"{}\" , \"episode\" : \"{}\" , \
             \"previous_digest\" : null }}\n",
            hex(&digest),
            hex(&bytes)
        ),
    )
    .unwrap();
    let store = CompleteEpisodeStore::open(scratch.path()).unwrap();
    assert_eq!(store.record_count(), 1);
}

#[test]
fn a_mutated_ledger_byte_is_refused() {
    let scratch = Scratch::new();
    let first = super::super::CompleteEpisode::try_new(minimal()).unwrap();
    let second = super::super::CompleteEpisode::try_new(populated()).unwrap();
    let first_bytes = first.canonical_bytes().unwrap();
    let second_bytes = second.canonical_bytes().unwrap();
    let first_digest = sha256(&first_bytes);
    let second_digest = sha256(&second_bytes);
    let valid = two_records(&first_digest, &first_bytes, &second_digest, &second_bytes);
    fs::create_dir_all(scratch.path()).unwrap();

    let mutations: Vec<(&str, String)> = vec![
        (
            "episode payload",
            two_records(
                &first_digest,
                &first_bytes,
                &second_digest,
                &mutate(&second_bytes),
            ),
        ),
        (
            "episode digest",
            two_records(
                &first_digest,
                &first_bytes,
                &mutate_digest(&second_digest),
                &second_bytes,
            ),
        ),
        (
            "sequence",
            two_records(&first_digest, &first_bytes, &second_digest, &second_bytes).replacen(
                "\"sequence\":2",
                "\"sequence\":3",
                1,
            ),
        ),
        (
            "previous digest",
            two_records(&first_digest, &first_bytes, &second_digest, &second_bytes).replacen(
                &hex(&first_digest),
                &hex(&second_digest),
                1,
            ),
        ),
        (
            "domain",
            two_records(&first_digest, &first_bytes, &second_digest, &second_bytes).replacen(
                COMPLETE_EPISODE_DOMAIN,
                "abbey-cbor-episode-v1",
                1,
            ),
        ),
        (
            "unknown member",
            two_records(&first_digest, &first_bytes, &second_digest, &second_bytes).replacen(
                "\"sequence\":2",
                "\"sequence\":2,\"extra\":1",
                1,
            ),
        ),
        (
            "dropped member",
            two_records(&first_digest, &first_bytes, &second_digest, &second_bytes).replacen(
                "\"previous_digest\":",
                "\"skipped\":",
                1,
            ),
        ),
    ];
    for (label, mutated) in mutations {
        fs::write(scratch.ledger(), &mutated).unwrap();
        assert!(
            matches!(
                CompleteEpisodeStore::open(scratch.path()),
                Err(CompleteEpisodeStoreError::Corrupt)
            ),
            "{label} must be refused"
        );
    }

    // The untouched ledger still opens, so the refusals above are about the
    // mutation and not about the fixture.
    fs::write(scratch.ledger(), &valid).unwrap();
    assert!(CompleteEpisodeStore::open(scratch.path()).is_ok());
}

fn mutate(bytes: &[u8]) -> Vec<u8> {
    let mut mutated = bytes.to_vec();
    let last = mutated.len() - 1;
    mutated[last] ^= 0x01;
    mutated
}

fn mutate_digest(digest: &[u8; 32]) -> [u8; 32] {
    let mut mutated = *digest;
    mutated[31] ^= 0x01;
    mutated
}

/// A two-record ledger, written by hand.
fn two_records(
    first_digest: &[u8; 32],
    first_bytes: &[u8],
    second_digest: &[u8; 32],
    second_bytes: &[u8],
) -> String {
    format!(
        "{}\n{}\n",
        line(1, None, &hex(first_digest), &hex(first_bytes)),
        line(
            2,
            Some(&hex(first_digest)),
            &hex(second_digest),
            &hex(second_bytes)
        )
    )
}

#[test]
fn an_incomplete_tail_is_discarded_and_a_failed_replay_is_left_alone() {
    let scratch = Scratch::new();
    let episode = super::super::CompleteEpisode::try_new(minimal()).unwrap();
    {
        let mut store = CompleteEpisodeStore::open(scratch.path()).unwrap();
        store.append(&episode).unwrap();
    }
    let valid = fs::read_to_string(scratch.ledger()).unwrap();

    // A torn append tail is discarded: the verified prefix survives and the
    // file is repaired to exactly that prefix.
    OpenOptions::new()
        .append(true)
        .open(scratch.ledger())
        .unwrap()
        .write_all(b"{\"domain\":\"wdbx-complete-episode-v1\",\"sequ")
        .unwrap();
    {
        let store = CompleteEpisodeStore::open(scratch.path()).unwrap();
        assert_eq!(store.record_count(), 1);
    }
    assert_eq!(fs::read_to_string(scratch.ledger()).unwrap(), valid);

    // A complete but invalid line plus a torn tail is refused, and the file
    // is not repaired: a failed replay must not discard anything.
    let corrupted = format!(
        "{}{{\"partial\":",
        valid.replace("\"sequence\":1", "\"sequence\":3")
    );
    fs::write(scratch.ledger(), &corrupted).unwrap();
    assert!(matches!(
        CompleteEpisodeStore::open(scratch.path()),
        Err(CompleteEpisodeStoreError::Corrupt)
    ));
    assert_eq!(
        fs::read_to_string(scratch.ledger()).unwrap(),
        corrupted,
        "a failed replay must not truncate even an incomplete final line"
    );
}

#[test]
fn signing_states_are_kept_apart() {
    let scratch = Scratch::new();
    let episode = super::super::CompleteEpisode::try_new(minimal()).unwrap();
    let digest = episode.digest().unwrap();

    // No signer: the record is unsigned, which is never invalid.
    {
        let mut store = CompleteEpisodeStore::open(scratch.path()).unwrap();
        store.append(&episode).unwrap();
    }
    {
        let store = CompleteEpisodeStore::open(scratch.path()).unwrap();
        assert_eq!(
            store.signature_status(&digest, resolver(&[])),
            Some(SignatureStatus::Unsigned)
        );
        assert_eq!(store.signer_key_id(), None);
    }

    // With a signer: the record carries a detached signature that verifies.
    let signed = Scratch::new();
    let writer = signer(1);
    let writer_key_id = writer.key_id().clone();
    let writer_key = writer.verifying_key();
    {
        let mut store = CompleteEpisodeStore::open_with_signer(signed.path(), writer).unwrap();
        store.append(&episode).unwrap();
        assert_eq!(store.signer_key_id(), Some(&writer_key_id));
    }
    {
        let store = CompleteEpisodeStore::open_with_signer(signed.path(), signer(1)).unwrap();
        let keys = [writer_key];
        assert_eq!(
            store.signature_status(&digest, resolver(&keys)),
            Some(SignatureStatus::Valid(writer_key_id))
        );
    }

    // A record signed under another key is that key's to judge: it is never
    // invalid here, and the opening writer's own key does not judge it.
    let foreign = Scratch::new();
    let other = signer(2);
    let other_key_id = other.key_id().clone();
    let other_key = other.verifying_key();
    {
        let mut store = CompleteEpisodeStore::open_with_signer(foreign.path(), other).unwrap();
        store.append(&episode).unwrap();
    }
    let store = CompleteEpisodeStore::open_with_signer(foreign.path(), signer(1)).unwrap();
    let other_keys = [other_key];
    assert_eq!(
        store.signature_status(&digest, resolver(&other_keys)),
        Some(SignatureStatus::Valid(other_key_id.clone()))
    );
    assert_eq!(
        store.signature_status(&digest, resolver(&[])),
        Some(SignatureStatus::UnknownKey(other_key_id))
    );
}

#[test]
fn a_second_writer_is_refused() {
    let scratch = Scratch::new();
    let store = CompleteEpisodeStore::open(scratch.path()).unwrap();
    assert!(matches!(
        CompleteEpisodeStore::open(scratch.path()),
        Err(CompleteEpisodeStoreError::WriterBusy)
    ));
    assert!(scratch.path().join(LOCK_FILE).exists());
    drop(store);
    assert!(CompleteEpisodeStore::open(scratch.path()).is_ok());
}

#[test]
fn the_chain_names_the_prior_record() {
    let scratch = Scratch::new();
    let first = super::super::CompleteEpisode::try_new(minimal()).unwrap();
    let second = super::super::CompleteEpisode::try_new(populated()).unwrap();
    let first_digest = first.digest().unwrap();
    let second_digest = second.digest().unwrap();
    {
        let mut store = CompleteEpisodeStore::open(scratch.path()).unwrap();
        let one = store.append(&first).unwrap();
        let two = store.append(&second).unwrap();
        assert_eq!(one.previous_digest, None);
        assert_eq!(two.previous_digest, Some(hex(&first_digest)));
        assert_eq!(two.episode_digest, hex(&second_digest));
    }
    {
        let store = CompleteEpisodeStore::open(scratch.path()).unwrap();
        let receipts = store.receipts();
        assert_eq!(receipts.len(), 2);
        assert_eq!(receipts[1].previous_digest, Some(hex(&first_digest)));
    }

    // A forged chain, a gap, and a fork are corruption.
    let first_bytes = first.canonical_bytes().unwrap();
    let second_bytes = second.canonical_bytes().unwrap();
    let forged_link = format!(
        "{}\n{}\n",
        line(1, None, &hex(&first_digest), &hex(&first_bytes)),
        line(
            2,
            Some(&hex(&second_digest)),
            &hex(&second_digest),
            &hex(&second_bytes)
        )
    );
    fs::write(scratch.ledger(), &forged_link).unwrap();
    assert!(matches!(
        CompleteEpisodeStore::open(scratch.path()),
        Err(CompleteEpisodeStoreError::Corrupt)
    ));

    // A fork: the same record twice, so the second copy names a digest that
    // is not the record before it.
    let duplicated = format!(
        "{}\n{}\n{}\n",
        line(1, None, &hex(&first_digest), &hex(&first_bytes)),
        line(
            2,
            Some(&hex(&first_digest)),
            &hex(&second_digest),
            &hex(&second_bytes)
        ),
        line(
            3,
            Some(&hex(&first_digest)),
            &hex(&second_digest),
            &hex(&second_bytes)
        )
    );
    fs::write(scratch.ledger(), &duplicated).unwrap();
    assert!(matches!(
        CompleteEpisodeStore::open(scratch.path()),
        Err(CompleteEpisodeStoreError::Corrupt)
    ));
}

#[test]
fn a_duplicate_episode_is_refused() {
    let scratch = Scratch::new();
    let episode = super::super::CompleteEpisode::try_new(minimal()).unwrap();
    let mut store = CompleteEpisodeStore::open(scratch.path()).unwrap();
    store.append(&episode).unwrap();
    assert!(matches!(
        store.append(&episode),
        Err(CompleteEpisodeStoreError::Replay)
    ));
    assert_eq!(store.record_count(), 1);
}

#[test]
fn the_ledger_and_record_ceilings_are_enforced() {
    let scratch = Scratch::new();
    let episode = super::super::CompleteEpisode::try_new(minimal()).unwrap();
    let mut store = CompleteEpisodeStore::open(scratch.path()).unwrap();
    store.append(&episode).unwrap();

    // A ledger that would grow past its byte ceiling is refused before the
    // write, and the file is left as it was.
    let ledger = OpenOptions::new()
        .write(true)
        .open(scratch.ledger())
        .unwrap();
    ledger.set_len(64 * 1024 * 1024).unwrap();
    drop(ledger);
    assert!(matches!(
        store.append(&episode),
        Err(CompleteEpisodeStoreError::Replay)
    ));
    let other = super::super::CompleteEpisode::try_new(populated()).unwrap();
    assert!(matches!(
        store.append(&other),
        Err(CompleteEpisodeStoreError::LimitExceeded)
    ));
    drop(store);

    // A ledger file that is already over its ceiling is refused on open.
    let ledger = OpenOptions::new()
        .write(true)
        .open(scratch.ledger())
        .unwrap();
    ledger.set_len(64 * 1024 * 1024 + 1).unwrap();
    drop(ledger);
    assert!(matches!(
        CompleteEpisodeStore::open(scratch.path()),
        Err(CompleteEpisodeStoreError::LimitExceeded)
    ));
}

#[test]
fn the_record_count_ceiling_is_enforced() {
    let scratch = Scratch::new();
    // One line past the ceiling. The lines parse; the ceiling is what stops
    // the reader, so the payload is a stub and never reaches the replay.
    let mut ledger = String::new();
    for sequence in 1..=65_537 {
        ledger.push_str(&line(sequence, None, &hex(&[0_u8; 32]), "00"));
        ledger.push('\n');
    }
    fs::create_dir_all(scratch.path()).unwrap();
    fs::write(scratch.ledger(), &ledger).unwrap();
    assert!(matches!(
        CompleteEpisodeStore::open(scratch.path()),
        Err(CompleteEpisodeStoreError::LimitExceeded)
    ));
}

#[test]
fn an_oversized_episode_is_refused() {
    let scratch = Scratch::new();
    let store = CompleteEpisodeStore::open(scratch.path()).unwrap();
    let oversized = vec![0_u8; MAX_ENCODED_SIZE + 1];
    assert!(matches!(
        super::super::facade::PrototypeEpisode::from_canonical_bytes(&oversized),
        Err(CompleteEpisodeStoreError::RecordTooLarge)
    ));
    assert_eq!(store.record_count(), 0);
}

#[test]
fn the_receipt_carries_no_episode_content() {
    let scratch = Scratch::new();
    let episode = super::super::CompleteEpisode::try_new(populated()).unwrap();
    let bytes = episode.canonical_bytes().unwrap();
    let mut store = CompleteEpisodeStore::open(scratch.path()).unwrap();
    let receipt = store.append(&episode).unwrap();
    let serialized = serde_json::to_vec(&receipt).unwrap();
    let text = String::from_utf8(serialized.clone()).unwrap();
    assert!(!text.contains(&hex(&bytes)));
    assert!(
        !serialized
            .windows(bytes.len())
            .any(|window| window == bytes)
    );
    // The episode's own identifiers are not in the receipt either.
    assert!(!text.contains("synthetic-tenant"));
    assert!(!text.contains("synthetic-guild"));
    assert!(!text.contains("synthetic-principal"));
    // The receipt names the record, and only the record.
    assert!(text.contains(&receipt.episode_digest));
    assert!(text.contains(COMPLETE_EPISODE_DOMAIN));
}

#[test]
fn the_ledger_is_a_different_store_from_the_v3_episode_ledger() {
    let scratch = Scratch::new();
    let episode = super::super::CompleteEpisode::try_new(minimal()).unwrap();
    {
        let mut store = CompleteEpisodeStore::open(scratch.path()).unwrap();
        store.append(&episode).unwrap();
    }
    // A different file family.
    assert!(scratch.path().join(LEDGER_FILE).exists());
    assert!(!scratch.path().join("episodes.v1.jsonl").exists());
    assert!(scratch.path().join(LOCK_FILE).exists());
    assert!(!scratch.path().join("episodes.v1.lock").exists());

    // A v3 episode envelope is not a complete episode: the domain decides.
    let v3 = EpisodeCommitment::new(
        1,
        CanonicalValue::Map(vec![(
            CanonicalValue::Text("guild_ref".into()),
            CanonicalValue::Text("synthetic-guild".into()),
        )]),
        CanonicalValue::Map(vec![(
            CanonicalValue::Text("event_kind".into()),
            CanonicalValue::Text("proposal".into()),
        )]),
        vec![],
    )
    .canonical_bytes()
    .unwrap();
    assert!(matches!(
        super::super::facade::PrototypeEpisode::from_canonical_bytes(&v3),
        Err(CompleteEpisodeStoreError::InvalidEpisode)
    ));
}

#[test]
fn the_typed_validation_runs_on_replay() {
    let scratch = Scratch::new();
    // A valid record of the same shape replays.
    let valid = super::super::CompleteEpisode::try_new(populated()).unwrap();
    let valid_bytes = valid.canonical_bytes().unwrap();
    fs::create_dir_all(scratch.path()).unwrap();
    fs::write(
        scratch.ledger(),
        format!(
            "{}\n",
            line(1, None, &hex(&sha256(&valid_bytes)), &hex(&valid_bytes))
        ),
    )
    .unwrap();
    assert!(CompleteEpisodeStore::open(scratch.path()).is_ok());

    // The same shape, invalid at the typed layer: an approval bound to
    // another tenant. The bytes are canonical, and the digest is right, so
    // only the complete episode's own validation can refuse it.
    let mut input = populated();
    input.authority.approvals[0].binding = binding("other-tenant");
    let invalid_bytes = input.wire::<Tree>().unwrap().canonical_bytes().unwrap();
    assert_ne!(invalid_bytes, valid_bytes);
    fs::write(
        scratch.ledger(),
        format!(
            "{}\n",
            line(1, None, &hex(&sha256(&invalid_bytes)), &hex(&invalid_bytes))
        ),
    )
    .unwrap();
    assert!(matches!(
        CompleteEpisodeStore::open(scratch.path()),
        Err(CompleteEpisodeStoreError::Corrupt)
    ));
}

#[test]
fn the_append_path_and_the_replay_path_agree() {
    let scratch = Scratch::new();
    let episode = super::super::CompleteEpisode::try_new(populated()).unwrap();
    let mut store = CompleteEpisodeStore::open(scratch.path()).unwrap();
    let receipt = store.append(&episode).unwrap();
    drop(store);
    let store = CompleteEpisodeStore::open(scratch.path()).unwrap();
    assert_eq!(store.receipts(), vec![receipt]);
    let bytes = episode.canonical_bytes().unwrap();
    assert_eq!(
        store
            .get(&sha256(&bytes))
            .unwrap()
            .canonical_bytes()
            .unwrap(),
        bytes
    );
    // The bytes on disk are the canonical bytes, hex encoded, and nothing else
    // claims them.
    let written = fs::read_to_string(scratch.ledger()).unwrap();
    assert!(written.contains(&hex(&bytes)));
    assert_eq!(unhex(&hex(&bytes)), bytes);
}
