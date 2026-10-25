//! Strict authentication of the private complete-episode ledger.
use abi_wdbx::v3::complete_episode_prototype::{
    CompleteEpisodeStoreError, PrototypeEpisode, PrototypeLedger,
};
use abi_wdbx::v3::episode::EpisodeSigner;
use ed25519_dalek::SigningKey;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

struct Scratch(PathBuf);
impl Scratch {
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn tempdir() -> std::io::Result<Scratch> {
    let path = std::env::temp_dir().join(format!("wdbx-authenticated-{}", Uuid::new_v4()));
    fs::create_dir(&path)?;
    Ok(Scratch(path))
}

fn signer(seed: u8) -> EpisodeSigner {
    EpisodeSigner::new(SigningKey::from_bytes(&[seed; 32]))
}

fn episode() -> PrototypeEpisode {
    let hex = include_str!("../src/v3/complete_episode/tests/provisional-minimal.hex").trim();
    let bytes: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
        .collect();
    PrototypeEpisode::from_canonical_bytes(&bytes).unwrap()
}

#[test]
fn required_signatures_reject_stripping_before_tail_repair() {
    let directory = tempdir().unwrap();
    let key = signer(1).verifying_key();
    let path;
    {
        let mut ledger = PrototypeLedger::open_with_signer(directory.path(), signer(1)).unwrap();
        ledger.append(&episode()).unwrap();
        path = ledger.ledger_path();
    }
    let original = fs::read(&path).unwrap();
    let mut row: serde_json::Value = serde_json::from_slice(&original).unwrap();
    row.as_object_mut().unwrap().remove("signature");
    let stripped = format!("{}\n{{\"torn\":", serde_json::to_string(&row).unwrap());
    fs::write(&path, &stripped).unwrap();
    assert!(matches!(
        PrototypeLedger::open_authenticated(directory.path(), signer(1), &[key]),
        Err(CompleteEpisodeStoreError::Corrupt)
    ));
    assert_eq!(fs::read(&path).unwrap(), stripped.as_bytes());
}

#[test]
fn authentication_checks_historical_keys_after_rotation() {
    let directory = tempdir().unwrap();
    let old_key = signer(1).verifying_key();
    let new_key = signer(2).verifying_key();
    {
        let mut ledger = PrototypeLedger::open_with_signer(directory.path(), signer(1)).unwrap();
        ledger.append(&episode()).unwrap();
    }
    // A current writer alone cannot authenticate the previous writer's history.
    assert!(matches!(
        PrototypeLedger::open_authenticated(directory.path(), signer(2), &[new_key]),
        Err(CompleteEpisodeStoreError::Corrupt)
    ));
    let ledger =
        PrototypeLedger::open_authenticated(directory.path(), signer(2), &[old_key, new_key])
            .unwrap();
    assert_eq!(ledger.record_count(), 1);
}

#[test]
fn untrusted_current_writer_is_refused_before_creating_a_store() {
    let directory = tempdir().unwrap();
    let missing = directory.path().join("missing");
    assert!(
        PrototypeLedger::open_authenticated(&missing, signer(1), &[signer(2).verifying_key()],)
            .is_err()
    );
    assert!(!missing.exists());
}

#[test]
fn valid_authenticated_append_survives_reopen_without_changing_record_bytes() {
    let directory = tempdir().unwrap();
    let key = signer(1).verifying_key();
    let path;
    {
        let mut ledger =
            PrototypeLedger::open_authenticated(directory.path(), signer(1), &[key]).unwrap();
        ledger.append(&episode()).unwrap();
        path = ledger.ledger_path();
    }
    let bytes = fs::read(&path).unwrap();
    let ledger = PrototypeLedger::open_authenticated(directory.path(), signer(1), &[key]).unwrap();
    assert_eq!(ledger.record_count(), 1);
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn trusted_signature_corruption_and_key_substitution_are_refused() {
    let directory = tempdir().unwrap();
    let key = signer(1).verifying_key();
    let path;
    {
        let mut ledger = PrototypeLedger::open_with_signer(directory.path(), signer(1)).unwrap();
        ledger.append(&episode()).unwrap();
        path = ledger.ledger_path();
    }
    let original = fs::read(&path).unwrap();
    let row: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let mut corrupted = row.clone();
    let byte = corrupted["signature"]["signature"][0].as_u64().unwrap();
    corrupted["signature"]["signature"][0] = ((byte + 1) % 256).into();
    let mut substituted = row;
    substituted["signature"]["signer_key_id"] = signer(2).key_id().as_str().into();
    for mutation in [corrupted, substituted] {
        let bytes = format!("{}\n", serde_json::to_string(&mutation).unwrap());
        fs::write(&path, &bytes).unwrap();
        assert!(matches!(
            PrototypeLedger::open_authenticated(directory.path(), signer(1), &[key]),
            Err(CompleteEpisodeStoreError::Corrupt)
        ));
        assert_eq!(fs::read(&path).unwrap(), bytes.as_bytes());
    }
}
