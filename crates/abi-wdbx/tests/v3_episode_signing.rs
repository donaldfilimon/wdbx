//! Per-episode writer signatures on the durable v3 episode store.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeSigner, EpisodeSource, EpisodeStore,
    EpisodeStoreError, EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, SignatureStatus,
    SignerKeyId, StorePolicy,
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-episode-sign-{}", Uuid::new_v4())))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn ledger(&self) -> PathBuf {
        self.0.join("episodes.v1.jsonl")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn signer(seed: u8) -> EpisodeSigner {
    EpisodeSigner::new(SigningKey::from_bytes(&[seed; 32]))
}

fn policy() -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([(
            "guild_ref".into(),
            GuildEpisodePolicy {
                learning_enabled: true,
                policy_version: "policy_v1".into(),
                token_budget: 1_000,
                storage_budget_bytes: 1024 * 1024,
                current_consent_epoch: Some(7),
            },
        )]),
    }
}

fn proposal(request_id: &str, operation_id: &str) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        contract_revision: 2,
        contract_digest: [9; 32],
        guild_ref: "guild_ref".into(),
        consent_epoch: Some(7),
        source_type: EpisodeSource::DiscordVoice,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C2,
        event: EpisodeEvent::Proposal {
            requested_by: ActorRef {
                principal_id: "requester_ref".into(),
                kind: ActorKind::HumanSubject,
            },
            proposed_by: ActorRef {
                principal_id: "abbey_service".into(),
                kind: ActorKind::Service,
            },
        },
        token_cost: 3,
        expected_commitment: None,
        quiet: false,
    }
}

fn append(store: &mut EpisodeStore, write: &EpisodeWrite) -> [u8; 32] {
    store
        .propose_write(write)
        .expect("valid write appends")
        .episode_digest
}

fn resolver(keys: &[VerifyingKey]) -> impl Fn(&SignerKeyId) -> Option<VerifyingKey> + '_ {
    |id| {
        keys.iter()
            .copied()
            .find(|key| &SignerKeyId::for_key(key) == id)
    }
}

#[test]
fn signing_leaves_the_digest_and_the_unsigned_ledger_bytes_unchanged() {
    let unsigned_dir = Scratch::new();
    let signed_dir = Scratch::new();
    let write = proposal("request_1", "operation_1");

    let unsigned_digest = {
        let mut store = EpisodeStore::open(unsigned_dir.path(), policy()).expect("open");
        append(&mut store, &write)
    };
    let signed_digest = {
        let mut store =
            EpisodeStore::open_with_signer(signed_dir.path(), policy(), signer(1)).expect("open");
        append(&mut store, &write)
    };
    // The signature is detached: it never enters the committed envelope.
    assert_eq!(signed_digest, unsigned_digest);

    let unsigned_line = fs::read_to_string(unsigned_dir.ledger()).expect("ledger");
    let signed_line = fs::read_to_string(signed_dir.ledger()).expect("ledger");
    assert!(!unsigned_line.contains("signature"));
    assert!(signed_line.contains("\"signature\""));
    assert!(signed_line.contains(signer(1).key_id().as_str()));
}

#[test]
fn a_signed_store_reports_valid_and_survives_reopen() {
    let scratch = Scratch::new();
    let key = signer(1).verifying_key();
    let digest = {
        let mut store =
            EpisodeStore::open_with_signer(scratch.path(), policy(), signer(1)).expect("open");
        assert_eq!(store.signer_key_id(), Some(signer(1).key_id()));
        append(&mut store, &proposal("request_1", "operation_1"))
    };

    let reopened =
        EpisodeStore::open_with_signer(scratch.path(), policy(), signer(1)).expect("replay");
    assert_eq!(
        reopened
            .signature_status("guild_ref", &digest, resolver(&[key]))
            .expect("lookup"),
        Some(SignatureStatus::Valid(signer(1).key_id().clone()))
    );
    // Without the key the answer is "cannot tell", never "forged".
    assert_eq!(
        reopened
            .signature_status("guild_ref", &digest, |_| None)
            .expect("lookup"),
        Some(SignatureStatus::UnknownKey(signer(1).key_id().clone()))
    );
    // A digest that was never appended is absent, not unsigned.
    assert_eq!(
        reopened
            .signature_status("guild_ref", &[0; 32], resolver(&[key]))
            .expect("lookup"),
        None
    );
}

#[test]
fn an_existing_unsigned_ledger_opens_with_a_signer_and_mixes_states() {
    let scratch = Scratch::new();
    let key = signer(1).verifying_key();
    let old = {
        let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
        assert_eq!(store.signer_key_id(), None);
        append(&mut store, &proposal("request_1", "operation_1"))
    };
    let mut store =
        EpisodeStore::open_with_signer(scratch.path(), policy(), signer(1)).expect("upgrade");
    let new = append(&mut store, &proposal("request_2", "operation_2"));

    assert_eq!(
        store
            .signature_status("guild_ref", &old, resolver(&[key]))
            .expect("old"),
        Some(SignatureStatus::Unsigned)
    );
    assert_eq!(
        store
            .signature_status("guild_ref", &new, resolver(&[key]))
            .expect("new"),
        Some(SignatureStatus::Valid(signer(1).key_id().clone()))
    );
}

#[test]
fn a_tampered_own_signature_fails_replay_but_a_foreign_one_is_left_to_the_verifier() {
    let scratch = Scratch::new();
    let digest = {
        let mut store =
            EpisodeStore::open_with_signer(scratch.path(), policy(), signer(1)).expect("open");
        append(&mut store, &proposal("request_1", "operation_1"))
    };

    // Flip one signature byte in the stored JSON array. The digest and the
    // envelope are untouched, so only the signature check can catch this.
    let original = fs::read_to_string(scratch.ledger()).expect("ledger");
    let start =
        original.find("\"signature\":[").expect("signature array") + "\"signature\":[".len();
    let end = start + original[start..].find([',', ']']).expect("first byte");
    let first: u8 = original[start..end].parse().expect("byte");
    let tampered = format!("{}{}{}", &original[..start], first ^ 1, &original[end..]);
    fs::write(scratch.ledger(), &tampered).expect("synthetic tamper");

    // The writer that owns this key refuses the ledger, like a mutated digest.
    assert!(matches!(
        EpisodeStore::open_with_signer(scratch.path(), policy(), signer(1)),
        Err(EpisodeStoreError::Corrupt)
    ));

    // A reader holding a different key cannot judge it at replay, so it opens,
    // and the verifier reports the forgery once given the right key.
    let key = signer(1).verifying_key();
    {
        let foreign = EpisodeStore::open_with_signer(scratch.path(), policy(), signer(2))
            .expect("foreign-key replay does not judge other keys");
        assert_eq!(
            foreign
                .signature_status("guild_ref", &digest, resolver(&[key]))
                .expect("lookup"),
            Some(SignatureStatus::Invalid(signer(1).key_id().clone()))
        );
    } // releases the single-writer lock before the next open
    let plain = EpisodeStore::open(scratch.path(), policy()).expect("unsigned open");
    assert_eq!(
        plain
            .signature_status("guild_ref", &digest, resolver(&[key]))
            .expect("lookup"),
        Some(SignatureStatus::Invalid(signer(1).key_id().clone()))
    );
}

#[test]
fn a_relabelled_key_id_does_not_let_another_key_vouch_for_the_record() {
    let scratch = Scratch::new();
    let digest = {
        let mut store =
            EpisodeStore::open_with_signer(scratch.path(), policy(), signer(1)).expect("open");
        append(&mut store, &proposal("request_1", "operation_1"))
    };
    let original = fs::read_to_string(scratch.ledger()).expect("ledger");
    let relabelled = original.replace(signer(1).key_id().as_str(), signer(2).key_id().as_str());
    assert_ne!(relabelled, original);
    fs::write(scratch.ledger(), relabelled).expect("synthetic relabel");

    // Key 2's writer now sees a record claiming its own key, and rejects it.
    assert!(matches!(
        EpisodeStore::open_with_signer(scratch.path(), policy(), signer(2)),
        Err(EpisodeStoreError::Corrupt)
    ));
    let plain = EpisodeStore::open(scratch.path(), policy()).expect("unsigned open");
    let key2 = signer(2).verifying_key();
    assert_eq!(
        plain
            .signature_status("guild_ref", &digest, resolver(&[key2]))
            .expect("lookup"),
        Some(SignatureStatus::Invalid(signer(2).key_id().clone()))
    );
}

#[test]
fn a_malformed_key_id_on_disk_is_corruption_for_every_reader() {
    let scratch = Scratch::new();
    {
        let mut store =
            EpisodeStore::open_with_signer(scratch.path(), policy(), signer(1)).expect("open");
        append(&mut store, &proposal("request_1", "operation_1"));
    }
    let original = fs::read_to_string(scratch.ledger()).expect("ledger");
    let broken = original.replace(signer(1).key_id().as_str(), "ed25519:not-hex");
    fs::write(scratch.ledger(), broken).expect("synthetic damage");
    assert!(matches!(
        EpisodeStore::open(scratch.path(), policy()),
        Err(EpisodeStoreError::Corrupt)
    ));
}

#[test]
fn a_signing_key_file_must_be_owner_only_and_32_bytes() {
    let scratch = Scratch::new();
    fs::create_dir_all(scratch.path()).expect("dir");
    let path = scratch.path().join("episode-signing.key");
    fs::write(&path, [1_u8; 32]).expect("key");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("mode");
        assert!(EpisodeSigner::from_key_file(&path).is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("mode");
    }
    let loaded = EpisodeSigner::from_key_file(&path).expect("owner-only key loads");
    assert_eq!(loaded.key_id(), signer(1).key_id());

    fs::write(&path, [1_u8; 31]).expect("short key");
    assert!(EpisodeSigner::from_key_file(&path).is_err());
}
