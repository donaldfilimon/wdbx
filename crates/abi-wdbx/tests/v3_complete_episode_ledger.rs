//! The private complete-episode ledger, driven from outside the crate.
//!
//! This exercises a provisional prototype, not a ratified profile. The ledger
//! has no ABI ratification, no wire contract, no quorum, no federation, and
//! no cross-language verification, and the crate's own module documentation
//! says so in full. The hidden facade this test uses exists only so that the
//! crate's own integration tests can reach a deliberately private prototype;
//! consumers must not depend on it.
//!
//! The canonical bytes below are the prototype's own pinned fixtures, so this
//! test does not invent a second encoder for them.

use abi_wdbx::v3::commitment::{CanonicalValue, EpisodeCommitment};
use abi_wdbx::v3::complete_episode_prototype::{
    CompleteEpisodeReceipt, CompleteEpisodeStoreError, PrototypeEpisode, PrototypeLedger,
};
use abi_wdbx::v3::episode::{EpisodeSigner, SignatureStatus, SignerKeyId};
use ed25519_dalek::{SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::ops::Range;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// The canonical bytes of the prototype's pinned minimal fixture.
const MINIMAL_HEX: &str = include_str!("../src/v3/complete_episode/tests/provisional-minimal.hex");
/// The canonical bytes of the prototype's pinned populated fixture.
const POPULATED_HEX: &str =
    include_str!("../src/v3/complete_episode/tests/provisional-populated.hex");

/// The ledger file this prototype owns, pinned by name.
const LEDGER_FILE: &str = "complete-episodes.v1.jsonl";
/// The writer lock this prototype owns, pinned by name.
const LOCK_FILE: &str = "complete-episodes.v1.lock";
/// The v3 episode ledger's own file, for the domain-separation check.
const V3_LEDGER_FILE: &str = "episodes.v1.jsonl";
/// The marker that opens the persisted signature byte array.
///
/// The ledger's line format carries the detached signature as a JSON array of
/// 64 decimal bytes, so one signature byte is one array element. The line
/// carries no per-line checksum, so a mutated byte is not recomputed away.
const SIGNATURE_ARRAY_MARKER: &str = "\"signature\":[";

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

fn unhex(text: &str) -> Vec<u8> {
    let trimmed = text.trim();
    assert!(trimmed.len().is_multiple_of(2), "hex must be even-length");
    (0..trimmed.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&trimmed[index..index + 2], 16).expect("hex"))
        .collect()
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn minimal() -> PrototypeEpisode {
    PrototypeEpisode::from_canonical_bytes(&unhex(MINIMAL_HEX)).expect("minimal fixture")
}

fn populated() -> PrototypeEpisode {
    PrototypeEpisode::from_canonical_bytes(&unhex(POPULATED_HEX)).expect("populated fixture")
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
    let episode = populated();
    let digest = episode.digest();
    assert_eq!(digest, sha256(episode.canonical_bytes()));

    let receipt: CompleteEpisodeReceipt = {
        let mut ledger = PrototypeLedger::open(scratch.path()).expect("open");
        let receipt = ledger.append(&episode).expect("append");
        assert_eq!(receipt.sequence, 1);
        assert_eq!(receipt.previous_digest, None);
        assert_eq!(receipt.domain, "wdbx-complete-episode-v1");
        assert_eq!(receipt.signer_key_id, None);
        assert_eq!(ledger.record_count(), 1);
        receipt
    };

    let ledger = PrototypeLedger::open(scratch.path()).expect("reopen");
    assert_eq!(ledger.record_count(), 1);
    assert_eq!(ledger.receipts(), vec![receipt.clone()]);
    let stored = ledger.episode(&digest).expect("record");
    assert_eq!(stored.canonical_bytes(), episode.canonical_bytes());
    assert_eq!(stored.digest(), digest);
}

#[test]
fn a_mutated_ledger_byte_is_refused() {
    let scratch = Scratch::new();
    let first = minimal();
    let second = populated();
    {
        let mut ledger = PrototypeLedger::open(scratch.path()).expect("open");
        ledger.append(&first).expect("append");
        ledger.append(&second).expect("append");
    }
    let original = fs::read_to_string(scratch.ledger()).expect("ledger");
    let lines: Vec<String> = original.lines().map(str::to_owned).collect();
    assert_eq!(lines.len(), 2);

    // Several mutation positions. Each one must be refused, never accepted.
    let mut mutations: Vec<(&str, String)> = Vec::new();
    {
        let mutated = lines[1].replace(
            &hex_of_bytes(second.canonical_bytes()),
            &hex_of_bytes(first.canonical_bytes()),
        );
        mutations.push(("episode payload", rebuild(&lines, 1, &mutated)));
    }
    {
        let mutated = lines[1].replace(&hex_of(&second.digest()), &hex_of(&first.digest()));
        mutations.push(("episode digest", rebuild(&lines, 1, &mutated)));
    }
    {
        let mutated = lines[1].replacen("\"sequence\":2", "\"sequence\":3", 1);
        mutations.push(("sequence", rebuild(&lines, 1, &mutated)));
    }
    {
        let mutated = lines[1].replacen(&hex_of(&first.digest()), &hex_of(&second.digest()), 1);
        mutations.push(("previous digest", rebuild(&lines, 1, &mutated)));
    }
    {
        let mutated = lines[0].replacen("wdbx-complete-episode-v1", "abbey-cbor-episode-v1", 1);
        mutations.push(("domain", rebuild(&lines, 0, &mutated)));
    }
    {
        let mutated = lines[0].replacen("\"sequence\":1", "\"sequence\":1,\"extra\":1", 1);
        mutations.push(("unknown member", rebuild(&lines, 0, &mutated)));
    }
    for (label, mutated) in mutations {
        assert_ne!(mutated, original, "{label} must change the file");
        fs::write(scratch.ledger(), &mutated).expect("write");
        assert!(
            matches!(
                PrototypeLedger::open(scratch.path()),
                Err(CompleteEpisodeStoreError::Corrupt)
            ),
            "{label} must be refused"
        );
    }

    // The untouched ledger still opens, so the refusals are about the
    // mutation and not about the fixture.
    fs::write(scratch.ledger(), &original).expect("write");
    assert!(PrototypeLedger::open(scratch.path()).is_ok());
}

#[test]
fn an_incomplete_final_line_is_discarded() {
    let scratch = Scratch::new();
    let episode = minimal();
    {
        let mut ledger = PrototypeLedger::open(scratch.path()).expect("open");
        ledger.append(&episode).expect("append");
    }
    let valid = fs::read_to_string(scratch.ledger()).expect("ledger");

    // A torn append tail is the crash tail of an interrupted append: it is
    // discarded, and the verified prefix survives.
    OpenOptions::new()
        .append(true)
        .open(scratch.ledger())
        .expect("ledger")
        .write_all(b"{\"domain\":\"wdbx-complete-episode-v1\",\"sequ")
        .expect("torn tail");
    {
        let ledger = PrototypeLedger::open(scratch.path()).expect("repair");
        assert_eq!(ledger.record_count(), 1);
    }
    assert_eq!(fs::read_to_string(scratch.ledger()).expect("ledger"), valid);

    // A complete but invalid line plus a torn tail is refused, and the file
    // is left exactly as it was: a failed replay repairs nothing.
    let corrupted = format!(
        "{}{{\"partial\":",
        valid.replace("\"sequence\":1", "\"sequence\":7")
    );
    fs::write(scratch.ledger(), &corrupted).expect("write");
    assert!(matches!(
        PrototypeLedger::open(scratch.path()),
        Err(CompleteEpisodeStoreError::Corrupt)
    ));
    assert_eq!(
        fs::read_to_string(scratch.ledger()).expect("ledger"),
        corrupted
    );
}

#[test]
fn signing_states_are_kept_apart() {
    let scratch = Scratch::new();
    let episode = minimal();
    let digest = episode.digest();

    // Without a signer the record is unsigned, which is never invalid.
    {
        let mut ledger = PrototypeLedger::open(scratch.path()).expect("open");
        ledger.append(&episode).expect("append");
    }
    {
        let ledger = PrototypeLedger::open(scratch.path()).expect("reopen");
        assert_eq!(
            ledger.signature_status(&digest, resolver(&[])),
            Some(SignatureStatus::Unsigned)
        );
        assert_eq!(ledger.signer_key_id(), None);
    }

    // With a signer the record carries a detached signature that verifies.
    let signed = Scratch::new();
    let writer = signer(1);
    let writer_key = writer.verifying_key();
    let writer_key_id = writer.key_id().clone();
    {
        let mut ledger =
            PrototypeLedger::open_with_signer(signed.path(), writer).expect("open with signer");
        ledger.append(&episode).expect("append");
        assert_eq!(ledger.signer_key_id(), Some(&writer_key_id));
    }
    {
        let ledger = PrototypeLedger::open_with_signer(signed.path(), signer(1))
            .expect("reopen with signer");
        assert_eq!(
            ledger.signature_status(&digest, resolver(&[writer_key])),
            Some(SignatureStatus::Valid(writer_key_id))
        );
    }

    // A record signed under another key is that key's to judge: it is never
    // invalid here, and the opening writer's own key does not judge it.
    let foreign = Scratch::new();
    let other = signer(2);
    let other_key = other.verifying_key();
    let other_key_id = other.key_id().clone();
    {
        let mut ledger =
            PrototypeLedger::open_with_signer(foreign.path(), other).expect("open with signer");
        ledger.append(&episode).expect("append");
    }
    let ledger = PrototypeLedger::open_with_signer(foreign.path(), signer(1)).expect("reopen");
    assert_eq!(
        ledger.signature_status(&digest, resolver(&[other_key])),
        Some(SignatureStatus::Valid(other_key_id.clone()))
    );
    assert_eq!(
        ledger.signature_status(&digest, resolver(&[])),
        Some(SignatureStatus::UnknownKey(other_key_id))
    );
    assert!(!matches!(
        ledger.signature_status(&digest, resolver(&[])),
        Some(SignatureStatus::Invalid(_))
    ));
}

#[test]
fn a_tampered_own_signature_is_corruption() {
    let scratch = Scratch::new();
    let episode = minimal();
    let writer_key_id = signer(1).key_id().as_str().to_owned();
    {
        let mut ledger =
            PrototypeLedger::open_with_signer(scratch.path(), signer(1)).expect("open with signer");
        ledger.append(&episode).expect("append");
    }

    let original = fs::read_to_string(scratch.ledger()).expect("ledger");
    let lines: Vec<String> = original.lines().map(str::to_owned).collect();
    assert_eq!(lines.len(), 1, "one record, one line");

    // The unmutated ledger replays under the same signer, so the refusal
    // below is about the mutation and not about the fixture.
    {
        let ledger = PrototypeLedger::open_with_signer(scratch.path(), signer(1))
            .expect("reopen with signer");
        assert_eq!(ledger.record_count(), 1);
    }

    // The persisted line parses, and it carries the writer's own key id and
    // 64 signature bytes.
    let (key_id, bytes) = signature_of(&lines[0]);
    assert_eq!(
        key_id, writer_key_id,
        "the record claims the writer's own key"
    );
    assert_eq!(bytes.len(), 64, "a detached signature is 64 bytes");

    // One byte of the signature is flipped. The line carries no per-line
    // checksum, so nothing recomputes the mutation away.
    let (tampered, array) = tamper_signature_byte(&lines[0], 0);
    assert_ne!(tampered, lines[0]);
    assert_eq!(
        tampered.len(),
        lines[0].len(),
        "the mutation must not change the line's length"
    );
    let differing = differing_positions(&lines[0], &tampered);
    assert_eq!(differing.len(), 1, "exactly one character differs");
    assert!(
        array.contains(&differing[0]),
        "the difference is inside the signature array"
    );

    // The mutation reaches the signature: the key id is untouched, and
    // exactly one signature byte differs, by one bit.
    let (tampered_key_id, tampered_bytes) = signature_of(&tampered);
    assert_eq!(tampered_key_id, key_id, "the key id is untouched");
    let changed: Vec<usize> = bytes
        .iter()
        .zip(&tampered_bytes)
        .enumerate()
        .filter(|(_, (before, after))| before != after)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(changed, vec![0], "exactly one signature byte differs");
    assert_eq!(bytes[0] ^ tampered_bytes[0], 0x01, "one bit differs");

    fs::write(scratch.ledger(), rebuild(&lines, 0, &tampered)).expect("write");
    assert!(
        matches!(
            PrototypeLedger::open_with_signer(scratch.path(), signer(1)),
            Err(CompleteEpisodeStoreError::Corrupt)
        ),
        "a record that claims this store's own key must verify"
    );
}

#[test]
fn a_tampered_foreign_signature_is_not_this_stores_to_judge() {
    let scratch = Scratch::new();
    let episode = minimal();
    let other = signer(2);
    let other_key = other.verifying_key();
    let other_key_id = other.key_id().clone();
    {
        let mut ledger =
            PrototypeLedger::open_with_signer(scratch.path(), other).expect("open with signer");
        ledger.append(&episode).expect("append");
    }

    let original = fs::read_to_string(scratch.ledger()).expect("ledger");
    let lines: Vec<String> = original.lines().map(str::to_owned).collect();
    assert_eq!(lines.len(), 1, "one record, one line");

    // The same mutation as under the store's own key: one byte of the
    // signature is flipped, and the line has no per-line checksum to
    // recompute it away.
    let (tampered, _) = tamper_signature_byte(&lines[0], 0);
    assert_ne!(tampered, lines[0]);
    fs::write(scratch.ledger(), rebuild(&lines, 0, &tampered)).expect("write");

    // The record claims another key, so this store does not judge it: the
    // ledger replays, and the tampering is reported as the signature state
    // instead of as corruption.
    let digest = episode.digest();
    let ledger =
        PrototypeLedger::open_with_signer(scratch.path(), signer(1)).expect("reopen with signer");
    assert_eq!(ledger.record_count(), 1);
    assert_eq!(
        ledger.signature_status(&digest, resolver(&[other_key])),
        Some(SignatureStatus::Invalid(other_key_id.clone()))
    );
    assert_eq!(
        ledger.signature_status(&digest, resolver(&[])),
        Some(SignatureStatus::UnknownKey(other_key_id))
    );
}

#[test]
fn a_second_writer_is_refused() {
    let scratch = Scratch::new();
    let ledger = PrototypeLedger::open(scratch.path()).expect("open");
    assert!(matches!(
        PrototypeLedger::open(scratch.path()),
        Err(CompleteEpisodeStoreError::WriterBusy)
    ));
    assert!(scratch.path().join(LOCK_FILE).exists());
    drop(ledger);
    assert!(PrototypeLedger::open(scratch.path()).is_ok());
}

#[test]
fn the_chain_names_the_prior_record() {
    let scratch = Scratch::new();
    let first = minimal();
    let second = populated();
    {
        let mut ledger = PrototypeLedger::open(scratch.path()).expect("open");
        let one = ledger.append(&first).expect("append");
        let two = ledger.append(&second).expect("append");
        assert_eq!(one.previous_digest, None);
        assert_eq!(
            two.previous_digest,
            Some(hex_of(&first.digest())),
            "the second record chains to the first"
        );
    }
    let ledger = PrototypeLedger::open(scratch.path()).expect("reopen");
    {
        let receipts = ledger.receipts();
        assert_eq!(receipts.len(), 2);
        assert_eq!(receipts[1].previous_digest, Some(hex_of(&first.digest())));
    }
    drop(ledger);

    // A forged chain is corruption: the second record names a digest that is
    // not the record before it.
    let original = fs::read_to_string(scratch.ledger()).expect("ledger");
    let lines: Vec<String> = original.lines().map(str::to_owned).collect();
    let forged_line = lines[1].replacen(&hex_of(&first.digest()), &hex_of(&second.digest()), 1);
    assert_ne!(forged_line, lines[1]);
    fs::write(scratch.ledger(), rebuild(&lines, 1, &forged_line)).expect("write");
    assert!(matches!(
        PrototypeLedger::open(scratch.path()),
        Err(CompleteEpisodeStoreError::Corrupt)
    ));
}

#[test]
fn the_bounds_are_enforced() {
    let scratch = Scratch::new();
    let episode = minimal();

    // An over-large record is refused before it is decoded.
    let oversized = vec![0_u8; (1 << 20) + 1];
    assert!(matches!(
        PrototypeEpisode::from_canonical_bytes(&oversized),
        Err(CompleteEpisodeStoreError::RecordTooLarge)
    ));

    // An over-large ledger is refused on open.
    let mut ledger = PrototypeLedger::open(scratch.path()).expect("open");
    ledger.append(&episode).expect("append");
    let path = ledger.ledger_path();
    drop(ledger);
    let file = OpenOptions::new().write(true).open(&path).expect("ledger");
    file.set_len(64 * 1024 * 1024 + 1).expect("grow");
    drop(file);
    assert!(matches!(
        PrototypeLedger::open(scratch.path()),
        Err(CompleteEpisodeStoreError::LimitExceeded)
    ));
}

#[test]
fn the_receipt_carries_no_episode_content() {
    let scratch = Scratch::new();
    let episode = populated();
    let bytes = episode.canonical_bytes();
    let mut ledger = PrototypeLedger::open(scratch.path()).expect("open");
    let receipt = ledger.append(&episode).expect("append");

    let serialized = serde_json::to_vec(&receipt).expect("receipt JSON");
    let text = String::from_utf8(serialized.clone()).expect("receipt UTF-8");
    // A byte-level assertion: the episode's own bytes are not in the
    // receipt, in any spelling.
    assert!(
        !serialized
            .windows(bytes.len())
            .any(|window| window == bytes),
        "the receipt must not carry the episode bytes"
    );
    assert!(!text.contains(&hex_of_bytes(bytes)));
    assert!(!text.contains("synthetic-tenant"));
    assert!(!text.contains("synthetic-guild"));
    assert!(!text.contains("synthetic-principal"));
    // The receipt names the record, and only the record.
    assert!(text.contains(&receipt.episode_digest));
    assert!(text.contains(receipt.domain));
}

#[test]
fn the_ledger_is_a_different_store_from_the_v3_episode_ledger() {
    let scratch = Scratch::new();
    let episode = minimal();
    {
        let mut ledger = PrototypeLedger::open(scratch.path()).expect("open");
        ledger.append(&episode).expect("append");
    }
    // A different file family, in the same directory.
    assert!(scratch.path().join(LEDGER_FILE).exists());
    assert!(scratch.path().join(LOCK_FILE).exists());
    assert!(!scratch.path().join(V3_LEDGER_FILE).exists());
    assert!(!scratch.path().join("episodes.v1.lock").exists());

    // The v3 episode ledger can share the directory without touching the
    // complete-episode ledger's files.
    let policy = abi_wdbx::v3::episode::StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: std::collections::BTreeMap::new(),
    };
    {
        let _v3 =
            abi_wdbx::v3::episode::EpisodeStore::open(scratch.path(), policy).expect("v3 open");
    }
    assert!(scratch.path().join(V3_LEDGER_FILE).exists());
    let complete = fs::read_to_string(scratch.ledger()).expect("ledger");
    assert!(complete.contains("wdbx-complete-episode-v1"));
    assert!(!complete.contains("abbey-cbor-episode-v1"));

    // A v3 episode envelope is not a complete episode: the domains differ,
    // and the types are distinct, so no v3 candidate can be written here.
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
    .expect("v3 envelope");
    assert!(matches!(
        PrototypeEpisode::from_canonical_bytes(&v3),
        Err(CompleteEpisodeStoreError::InvalidEpisode)
    ));
}

fn hex_of(digest: &[u8; 32]) -> String {
    hex_of_bytes(digest)
}

fn hex_of_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        write!(text, "{byte:02x}").expect("writing to a String cannot fail");
        text
    })
}

/// Rebuild a ledger file with one line replaced.
fn rebuild(lines: &[String], index: usize, replacement: &str) -> String {
    let mut rebuilt = lines.to_vec();
    replacement.clone_into(&mut rebuilt[index]);
    let mut text = String::new();
    for line in &rebuilt {
        text.push_str(line);
        text.push('\n');
    }
    text
}

/// The persisted signature of one ledger line: the key it names and its bytes.
///
/// The line is parsed as the store parses it, so a mutation that does not
/// survive the parse is caught here rather than being mistaken for a
/// signature that simply fails to verify.
fn signature_of(line: &str) -> (String, Vec<u8>) {
    let value: serde_json::Value = serde_json::from_str(line).expect("the line is JSON");
    let signature = value
        .get("signature")
        .expect("the line carries a signature")
        .as_object()
        .expect("the signature is an object");
    let key_id = signature
        .get("signer_key_id")
        .and_then(serde_json::Value::as_str)
        .expect("the signature names a key");
    let bytes = signature
        .get("signature")
        .and_then(serde_json::Value::as_array)
        .expect("the signature carries bytes")
        .iter()
        .map(|byte| {
            byte.as_u64()
                .and_then(|value| u8::try_from(value).ok())
                .expect("a signature byte")
        })
        .collect();
    (key_id.to_owned(), bytes)
}

/// Flip the low bit of one byte of a line's detached signature.
///
/// The line format carries the detached signature as a JSON array of 64
/// decimal bytes, so one signature byte is one array element. Flipping the
/// low bit of a decimal byte never changes its digit count, so the tampered
/// line differs from the original in exactly one character, and that
/// character is inside the signature array.
///
/// Returns the tampered line and the byte range the array occupies.
fn tamper_signature_byte(line: &str, index: usize) -> (String, Range<usize>) {
    assert_eq!(
        line.matches(SIGNATURE_ARRAY_MARKER).count(),
        1,
        "the line carries exactly one signature array"
    );
    let array_start = line
        .find(SIGNATURE_ARRAY_MARKER)
        .expect("the line carries a signature array")
        + SIGNATURE_ARRAY_MARKER.len();
    let array_end = array_start
        + line[array_start..]
            .find(']')
            .expect("the signature array is closed");
    let elements: Vec<&str> = line[array_start..array_end].split(',').collect();
    assert_eq!(elements.len(), 64, "a detached signature is 64 bytes");
    let byte: u8 = elements[index].parse().expect("a decimal signature byte");
    let tampered_byte = byte ^ 0x01;
    assert_ne!(tampered_byte, byte);
    assert_eq!(
        tampered_byte.to_string().len(),
        byte.to_string().len(),
        "the mutation must not change the line's length"
    );

    let mut tampered = String::with_capacity(line.len());
    tampered.push_str(&line[..array_start]);
    for (position, element) in elements.iter().enumerate() {
        if position == index {
            tampered.push_str(&tampered_byte.to_string());
        } else {
            tampered.push_str(element);
        }
        if position + 1 < elements.len() {
            tampered.push(',');
        }
    }
    tampered.push_str(&line[array_end..]);
    (tampered, array_start..array_end)
}

/// The byte positions at which two lines differ.
fn differing_positions(left: &str, right: &str) -> Vec<usize> {
    left.bytes()
        .zip(right.bytes())
        .enumerate()
        .filter(|(_, (left, right))| left != right)
        .map(|(position, _)| position)
        .collect()
}
