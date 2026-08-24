//! Durable canonical episode write-gate and recovery tests.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, AttributionResult, AuthorizationState, EpisodeEvent, EpisodeSource,
    EpisodeStore, EpisodeStoreError, EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MediaOutcome,
    StorePolicy, TerminalReason, TerminalStatus, VoiceEvidence, VoiceTransition,
};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use uuid::Uuid;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-episode-{}", Uuid::new_v4())))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn actor(principal_id: &str, kind: ActorKind) -> ActorRef {
    ActorRef {
        principal_id: principal_id.into(),
        kind,
    }
}

fn policy(enabled: bool, tokens: u64, bytes: u64) -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([(
            "guild_ref".into(),
            GuildEpisodePolicy {
                learning_enabled: enabled,
                policy_version: "policy_v1".into(),
                token_budget: tokens,
                storage_budget_bytes: bytes,
                current_consent_epoch: Some(7),
            },
        )]),
    }
}

fn write(request_id: &str, operation_id: &str, event: EpisodeEvent) -> EpisodeWrite {
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
        event,
        token_cost: 3,
        expected_commitment: None,
        quiet: false,
    }
}

fn proposal(request_id: &str, operation_id: &str) -> EpisodeWrite {
    write(
        request_id,
        operation_id,
        EpisodeEvent::Proposal {
            requested_by: actor("requester_ref", ActorKind::HumanSubject),
            proposed_by: actor("abbey_service", ActorKind::Service),
        },
    )
}

fn append(store: &mut EpisodeStore, mut event: EpisodeWrite) -> [u8; 32] {
    let preview = store
        .preview_commitment(&event)
        .expect("valid event previews");
    event.expected_commitment = Some(preview);
    let receipt = store.propose_write(&event).expect("valid event appends");
    assert_eq!(receipt.episode_digest, preview);
    assert!(receipt.redacted);
    preview
}

fn voice_evidence(participant_count: u16) -> VoiceEvidence {
    VoiceEvidence {
        consent_epoch: 7,
        participant_count,
        authorization_state: AuthorizationState::Authorized,
        attribution: AttributionResult::Attributed,
        stt: MediaOutcome::Succeeded,
        tts: MediaOutcome::Succeeded,
        playback: MediaOutcome::Succeeded,
        barge_in_count: 1,
        transitions: vec![
            VoiceTransition::Opened,
            VoiceTransition::Attested,
            VoiceTransition::Paused,
            VoiceTransition::Resumed,
        ],
        terminal_reason: TerminalReason::Completed,
    }
}

#[test]
fn lifecycle_is_linked_sanitized_and_stable_across_reopen() {
    let scratch = Scratch::new();
    let store_policy = policy(true, 100, 1024 * 1024);
    let expected_receipts;
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        let first = append(&mut store, proposal("request_1", "operation_1"));
        let second = append(
            &mut store,
            write(
                "request_2",
                "operation_1",
                EpisodeEvent::Approval {
                    approved_by: actor("admin_ref", ActorKind::GuildAdministrator),
                },
            ),
        );
        let third = append(
            &mut store,
            write(
                "request_3",
                "operation_1",
                EpisodeEvent::Execution {
                    executed_by: actor("bot_service", ActorKind::Service),
                    voice: Some(voice_evidence(2_048)),
                },
            ),
        );
        let fourth = append(
            &mut store,
            write(
                "request_4",
                "operation_1",
                EpisodeEvent::Compensation {
                    compensated_by: actor("bot_service", ActorKind::Service),
                    exact_restore_observed: true,
                },
            ),
        );
        let fifth = append(
            &mut store,
            write(
                "request_5",
                "operation_1",
                EpisodeEvent::Terminal {
                    status: TerminalStatus::Compensated,
                    reason: TerminalReason::Completed,
                },
            ),
        );
        assert_ne!(first, second);
        assert_ne!(second, third);
        assert_ne!(third, fourth);
        assert_ne!(fourth, fifth);
        expected_receipts = store.retrieve("guild_ref", 10).expect("receipts");
        assert_eq!(expected_receipts.len(), 5);
        assert_eq!(expected_receipts[1].previous_digest, Some(first));
        assert_eq!(expected_receipts[4].previous_digest, Some(fourth));
        assert_eq!(store.guild_usage("guild_ref").expect("usage").0, 15);
        let serialized = serde_json::to_string(&expected_receipts).expect("receipt JSON");
        for forbidden in [
            "requester_ref",
            "admin_ref",
            "transcript",
            "audio",
            "response_text",
            "participant_identity",
        ] {
            assert!(!serialized.contains(forbidden));
        }
    }

    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    assert_eq!(
        reopened.retrieve("guild_ref", 10).expect("receipts"),
        expected_receipts
    );
}

#[test]
fn quiet_opt_out_stale_bindings_and_budgets_fail_before_append() {
    let scratch = Scratch::new();
    let mut disabled =
        EpisodeStore::open(scratch.path(), policy(false, 100, 10_000)).expect("open");
    assert!(matches!(
        disabled.propose_write(&proposal("request_1", "operation_1")),
        Err(EpisodeStoreError::LearningDisabled)
    ));
    drop(disabled);

    let mut store = EpisodeStore::open(scratch.path(), policy(true, 3, 10_000)).expect("open");
    let mut quiet = proposal("request_1", "operation_1");
    quiet.quiet = true;
    assert!(matches!(
        store.propose_write(&quiet),
        Err(EpisodeStoreError::Quiet)
    ));
    let mut stale = proposal("request_1", "operation_1");
    stale.contract_revision = 1;
    assert!(matches!(
        store.propose_write(&stale),
        Err(EpisodeStoreError::StaleBinding)
    ));
    append(&mut store, proposal("request_1", "operation_1"));
    assert!(matches!(
        store.propose_write(&write(
            "request_2",
            "operation_1",
            EpisodeEvent::Approval {
                approved_by: actor("admin_ref", ActorKind::GuildAdministrator),
            },
        )),
        Err(EpisodeStoreError::TokenBudget)
    ));

    let other = Scratch::new();
    let mut byte_limited = EpisodeStore::open(other.path(), policy(true, 100, 1)).expect("open");
    assert!(matches!(
        byte_limited.propose_write(&proposal("request_1", "operation_1")),
        Err(EpisodeStoreError::StorageBudget)
    ));
}

#[test]
fn replay_identity_order_and_commitment_mutation_are_rejected() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(true, 100, 100_000)).expect("open");
    append(&mut store, proposal("request_1", "operation_1"));

    assert!(matches!(
        store.propose_write(&proposal("request_2", "operation_1")),
        Err(EpisodeStoreError::Replay)
    ));
    let self_approval = write(
        "request_2",
        "operation_1",
        EpisodeEvent::Approval {
            approved_by: actor("requester_ref", ActorKind::HumanSubject),
        },
    );
    assert!(matches!(
        store.propose_write(&self_approval),
        Err(EpisodeStoreError::InvalidTransition)
    ));
    let execution = write(
        "request_2",
        "operation_1",
        EpisodeEvent::Execution {
            executed_by: actor("bot_service", ActorKind::Service),
            voice: Some(voice_evidence(2)),
        },
    );
    assert!(matches!(
        store.propose_write(&execution),
        Err(EpisodeStoreError::InvalidTransition)
    ));
    let mut mismatch = write(
        "request_2",
        "operation_1",
        EpisodeEvent::Approval {
            approved_by: actor("admin_ref", ActorKind::GuildAdministrator),
        },
    );
    mismatch.expected_commitment = Some([4; 32]);
    assert!(matches!(
        store.propose_write(&mismatch),
        Err(EpisodeStoreError::CommitmentMismatch)
    ));
}

#[test]
fn partial_tail_recovers_but_a_mutated_complete_commitment_fails_closed() {
    let scratch = Scratch::new();
    {
        let mut store =
            EpisodeStore::open(scratch.path(), policy(true, 100, 100_000)).expect("open");
        append(&mut store, proposal("request_1", "operation_1"));
    }
    let ledger_path = scratch.path().join("episodes.v1.jsonl");
    OpenOptions::new()
        .append(true)
        .open(&ledger_path)
        .expect("ledger")
        .write_all(b"{\"partial\":")
        .expect("partial tail");
    {
        let reopened = EpisodeStore::open(scratch.path(), policy(true, 100, 100_000))
            .expect("partial tail is truncated");
        assert_eq!(
            reopened.retrieve("guild_ref", 10).expect("receipt").len(),
            1
        );
    }

    let original = fs::read_to_string(&ledger_path).expect("ledger text");
    let mutated = original.replace("requester_ref", "attacker_ref");
    fs::write(&ledger_path, mutated).expect("synthetic tamper");
    assert!(matches!(
        EpisodeStore::open(scratch.path(), policy(true, 100, 100_000)),
        Err(EpisodeStoreError::Corrupt)
    ));
}

#[test]
fn raw_content_and_voice_collection_overflow_cannot_enter_the_typed_gate() {
    let raw = serde_json::json!({
        "request_id": "request_1",
        "operation_id": "operation_1",
        "contract_revision": 2,
        "contract_digest": vec![9; 32],
        "guild_ref": "guild_ref",
        "consent_epoch": 7,
        "source_type": "discord_voice",
        "policy_version": "policy_v1",
        "evidence_level": "C2",
        "event": {
            "kind": "execution",
            "executed_by": {"principal_id": "bot_service", "kind": "service"},
            "voice": null,
            "transcript": "forbidden"
        },
        "token_cost": 1,
        "expected_commitment": null,
        "quiet": false
    });
    assert!(serde_json::from_value::<EpisodeWrite>(raw).is_err());

    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(true, 100, 100_000)).expect("open");
    append(&mut store, proposal("request_1", "operation_1"));
    append(
        &mut store,
        write(
            "request_2",
            "operation_1",
            EpisodeEvent::Approval {
                approved_by: actor("admin_ref", ActorKind::GuildAdministrator),
            },
        ),
    );
    let overflow = write(
        "request_3",
        "operation_1",
        EpisodeEvent::Execution {
            executed_by: actor("bot_service", ActorKind::Service),
            voice: Some(voice_evidence(2_049)),
        },
    );
    assert!(matches!(
        store.propose_write(&overflow),
        Err(EpisodeStoreError::InvalidInput)
    ));
}
