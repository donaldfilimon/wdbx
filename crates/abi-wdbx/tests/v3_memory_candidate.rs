//! Memory-candidate episodes (amendment 2026-09-06): single-event terminal
//! operations whose receipts stay content-free, with supersede/forget edges
//! and payload accounting checked on both the write and the replay path.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeSource, EpisodeStore, EpisodeStoreError,
    EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MemoryCandidate, MemoryClass, RetentionClass,
    StorePolicy, TerminalReason, TerminalStatus,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const GOLDEN_PATH: &str = "tests/golden/episode_write_memory_candidate.json";
/// Digest of [`golden_write`] under [`golden_policy`]. abbey-bot's transcription
/// must reproduce this from the golden JSON byte for byte.
const GOLDEN_DIGEST: &str = "3c19a479a23077d95238b710876e5c03d2300dd77e9ccfedbbe6c11b0fc768bc";

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-memory-{}", Uuid::new_v4())))
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

fn guild_policy(bytes: u64) -> GuildEpisodePolicy {
    GuildEpisodePolicy {
        learning_enabled: true,
        policy_version: "policy_v1".into(),
        token_budget: 1_000,
        storage_budget_bytes: bytes,
        current_consent_epoch: Some(7),
    }
}

fn policy(bytes: u64) -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([
            ("guild_ref".into(), guild_policy(bytes)),
            ("other_guild".into(), guild_policy(bytes)),
        ]),
    }
}

fn write(request_id: &str, operation_id: &str, event: EpisodeEvent) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        contract_revision: 2,
        contract_digest: [9; 32],
        guild_ref: "guild_ref".into(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C1,
        event,
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}

fn candidate(seed: u8) -> MemoryCandidate {
    MemoryCandidate {
        class: MemoryClass::Fact,
        retention: RetentionClass::Durable,
        payload_commitment: [seed; 32],
        payload_bytes: 64,
        dimension: None,
        embedding_version: None,
        member_scoped: true,
        supersedes: None,
        forgets: None,
    }
}

fn forget(target: [u8; 32]) -> MemoryCandidate {
    MemoryCandidate {
        payload_commitment: [0; 32],
        payload_bytes: 0,
        forgets: Some(target),
        ..candidate(1)
    }
}

fn memory(request_id: &str, operation_id: &str, candidate: MemoryCandidate) -> EpisodeWrite {
    write(
        request_id,
        operation_id,
        EpisodeEvent::MemoryCandidate {
            recorded_by: actor("abbey_service", ActorKind::Service),
            candidate,
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

fn rejected(store: &mut EpisodeStore, event: &EpisodeWrite) -> EpisodeStoreError {
    let error = store.propose_write(event).expect_err("rejected write");
    assert_eq!(
        store
            .preview_commitment(event)
            .map(|_| ())
            .unwrap_err()
            .to_string(),
        error.to_string(),
        "preview and append agree on the rejection"
    );
    error
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

#[test]
fn a_candidate_is_one_completed_operation_with_a_content_free_receipt() {
    let scratch = Scratch::new();
    let store_policy = policy(1024 * 1024);
    let receipts;
    let usage;
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
        receipts = store.retrieve("guild_ref", 10).expect("receipts");
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].episode_digest, digest);
        assert_eq!(receipts[0].event_kind, "memory_candidate");
        assert_eq!(receipts[0].terminal_status, Some(TerminalStatus::Completed));
        assert_eq!(receipts[0].previous_digest, None);
        assert_eq!(
            store.find_receipt("guild_ref", &digest).expect("lookup"),
            Some(receipts[0].clone())
        );
        let serialized = serde_json::to_string(&receipts).expect("receipt JSON");
        for forbidden in ["payload", "abbey_service", "commitment", "embedding"] {
            assert!(!serialized.contains(forbidden), "{forbidden} leaked");
        }
        usage = store.guild_usage("guild_ref").expect("usage");
        assert_eq!(usage.0, 1);
        assert!(usage.1 > 64, "payload bytes are charged on top of the line");

        // Terminal in the same append: nothing may follow on that operation.
        let follow = write(
            "request_2",
            "memory_1",
            EpisodeEvent::Terminal {
                status: TerminalStatus::Completed,
                reason: TerminalReason::Completed,
            },
        );
        assert!(matches!(
            rejected(&mut store, &follow),
            EpisodeStoreError::InvalidTransition
        ));
        let approve = write(
            "request_3",
            "memory_1",
            EpisodeEvent::Approval {
                approved_by: actor("admin_ref", ActorKind::GuildAdministrator),
            },
        );
        assert!(matches!(
            rejected(&mut store, &approve),
            EpisodeStoreError::InvalidTransition
        ));
        // A second candidate on the same operation identifier is a replay.
        assert!(matches!(
            rejected(&mut store, &memory("request_4", "memory_1", candidate(2))),
            EpisodeStoreError::Replay
        ));
        // A human cannot record a candidate; a service must.
        let human = write(
            "request_5",
            "memory_2",
            EpisodeEvent::MemoryCandidate {
                recorded_by: actor("admin_ref", ActorKind::GuildAdministrator),
                candidate: candidate(3),
            },
        );
        assert!(matches!(
            rejected(&mut store, &human),
            EpisodeStoreError::InvalidTransition
        ));
    }

    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    assert_eq!(
        reopened.retrieve("guild_ref", 10).expect("receipts"),
        receipts
    );
    assert_eq!(reopened.guild_usage("guild_ref"), Some(usage));
}

#[test]
fn shape_rules_hold_before_any_append() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1024 * 1024)).expect("open");

    let mut voice = memory("request_v", "memory_v", candidate(1));
    voice.source_type = EpisodeSource::DiscordVoice;
    voice.consent_epoch = Some(7);
    assert!(matches!(
        rejected(&mut store, &voice),
        EpisodeStoreError::InvalidInput
    ));

    let bad_shapes = [
        MemoryCandidate {
            class: MemoryClass::Embedding,
            dimension: Some(0),
            ..candidate(1)
        },
        MemoryCandidate {
            class: MemoryClass::Embedding,
            dimension: None,
            ..candidate(1)
        },
        MemoryCandidate {
            dimension: Some(3),
            ..candidate(1)
        },
        MemoryCandidate {
            embedding_version: Some("Bad Version!".into()),
            ..candidate(1)
        },
        MemoryCandidate {
            payload_bytes: 0,
            ..candidate(1)
        },
        MemoryCandidate {
            payload_commitment: [0; 32],
            ..candidate(1)
        },
        MemoryCandidate {
            payload_bytes: 12,
            ..forget([5; 32])
        },
        MemoryCandidate {
            payload_commitment: [5; 32],
            ..forget([5; 32])
        },
        MemoryCandidate {
            supersedes: Some([5; 32]),
            ..forget([5; 32])
        },
    ];
    for (index, shape) in bad_shapes.into_iter().enumerate() {
        let event = memory(
            &format!("request_{index}"),
            &format!("memory_{index}"),
            shape,
        );
        assert!(
            matches!(
                rejected(&mut store, &event),
                EpisodeStoreError::InvalidInput
            ),
            "shape {index} must be invalid input"
        );
    }
    assert_eq!(store.retrieve("guild_ref", 10).expect("receipts"), vec![]);

    let embedding = MemoryCandidate {
        class: MemoryClass::Embedding,
        retention: RetentionClass::Operational,
        dimension: Some(384),
        embedding_version: Some("wyhash-embedding-v1".into()),
        ..candidate(2)
    };
    append(&mut store, memory("request_ok", "memory_ok", embedding));

    // The payload itself cannot ride along: the candidate is a closed shape.
    let mut raw =
        serde_json::to_value(memory("request_x", "memory_x", candidate(3))).expect("write to JSON");
    raw["event"]["candidate"]["payload"] = serde_json::Value::String("secret text".into());
    assert!(serde_json::from_value::<EpisodeWrite>(raw).is_err());
}

#[test]
fn supersede_and_forget_edges_name_admitted_digests_only() {
    let scratch = Scratch::new();
    let store_policy = policy(1024 * 1024);
    let first;
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        first = append(&mut store, memory("request_1", "memory_1", candidate(1)));

        let unknown = MemoryCandidate {
            supersedes: Some([0xaa; 32]),
            ..candidate(2)
        };
        assert!(matches!(
            rejected(&mut store, &memory("request_2", "memory_2", unknown)),
            EpisodeStoreError::InvalidTransition
        ));
        assert!(matches!(
            rejected(
                &mut store,
                &memory("request_3", "memory_3", forget([0xaa; 32]))
            ),
            EpisodeStoreError::InvalidTransition
        ));

        // A digest admitted in another guild is not an edge target here.
        let mut foreign = memory("request_4", "memory_4", candidate(4));
        foreign.guild_ref = "other_guild".into();
        let foreign_digest = append(&mut store, foreign);
        let cross = MemoryCandidate {
            supersedes: Some(foreign_digest),
            ..candidate(5)
        };
        assert!(matches!(
            rejected(&mut store, &memory("request_5", "memory_5", cross)),
            EpisodeStoreError::InvalidTransition
        ));

        // Two corrections of one memory are two edges, not a rewrite.
        let second = append(
            &mut store,
            memory(
                "request_6",
                "memory_6",
                MemoryCandidate {
                    supersedes: Some(first),
                    ..candidate(6)
                },
            ),
        );
        append(
            &mut store,
            memory(
                "request_7",
                "memory_7",
                MemoryCandidate {
                    supersedes: Some(first),
                    ..candidate(7)
                },
            ),
        );
        assert_ne!(first, second);

        // Forgetting is a tombstone: no further edge may name the digest.
        let receipt = store
            .propose_write(&memory("request_8", "memory_8", forget(first)))
            .expect("forget appends");
        assert_eq!(receipt.event_kind, "memory_candidate");
        assert_eq!(receipt.terminal_status, Some(TerminalStatus::Completed));
        assert!(matches!(
            rejected(&mut store, &memory("request_9", "memory_9", forget(first))),
            EpisodeStoreError::InvalidTransition
        ));
        let after_forget = MemoryCandidate {
            supersedes: Some(first),
            ..candidate(9)
        };
        assert!(matches!(
            rejected(&mut store, &memory("request_10", "memory_10", after_forget)),
            EpisodeStoreError::InvalidTransition
        ));
        // The superseding record itself is still live and may be forgotten.
        append(
            &mut store,
            memory("request_11", "memory_11", forget(second)),
        );
    }

    // Replay rebuilds the edge sets, so the tombstone survives reopen.
    let mut reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    assert!(matches!(
        rejected(
            &mut reopened,
            &memory("request_12", "memory_12", forget(first))
        ),
        EpisodeStoreError::InvalidTransition
    ));
    assert_eq!(
        reopened.retrieve("guild_ref", 20).expect("receipts").len(),
        5
    );
}

#[test]
fn payload_bytes_are_charged_against_the_storage_budget() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(100_000)).expect("open");
    let large = MemoryCandidate {
        payload_bytes: 50_000,
        ..candidate(1)
    };
    append(&mut store, memory("request_1", "memory_1", large.clone()));
    let (_, bytes) = store.guild_usage("guild_ref").expect("usage");
    assert!(
        bytes > 50_000 && bytes < 60_000,
        "line plus payload: {bytes}"
    );

    // Budgets are checked at append time only; previews do not charge them.
    assert!(matches!(
        store.propose_write(&memory("request_2", "memory_2", large)),
        Err(EpisodeStoreError::StorageBudget)
    ));
    assert_eq!(store.retrieve("guild_ref", 10).expect("receipts").len(), 1);

    let fits = MemoryCandidate {
        payload_bytes: 40_000,
        ..candidate(2)
    };
    append(&mut store, memory("request_3", "memory_3", fits));
    assert!(matches!(
        rejected(
            &mut store,
            &memory("request_4", "memory_4", forget([0xbb; 32]))
        ),
        EpisodeStoreError::InvalidTransition
    ));
}

/// Deterministic 32-byte pattern `index * mul + add` (wrapping), for fixtures.
fn pattern(mul: u8, add: u8) -> [u8; 32] {
    let mut out = [0_u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::try_from(index)
            .expect("index below 32")
            .wrapping_mul(mul)
            .wrapping_add(add);
    }
    out
}

fn golden_policy() -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: pattern(7, 1),
        guilds: BTreeMap::from([(
            "discord-123456789012345678".into(),
            GuildEpisodePolicy {
                learning_enabled: true,
                policy_version: "policy_v1".into(),
                token_budget: 1_000,
                storage_budget_bytes: 1024 * 1024,
                current_consent_epoch: None,
            },
        )]),
    }
}

fn golden_write() -> EpisodeWrite {
    EpisodeWrite {
        request_id: "req-memory-00000000deadbeef".into(),
        operation_id: "memory-embedding-0123456789abcdef".into(),
        contract_revision: 2,
        contract_digest: pattern(7, 1),
        guild_ref: "discord-123456789012345678".into(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C0,
        event: EpisodeEvent::MemoryCandidate {
            recorded_by: actor("abbey-service", ActorKind::Service),
            candidate: MemoryCandidate {
                class: MemoryClass::Embedding,
                retention: RetentionClass::Durable,
                payload_commitment: pattern(5, 2),
                payload_bytes: 1_536,
                dimension: Some(384),
                embedding_version: Some("abbey-embedding-v1".into()),
                member_scoped: true,
                supersedes: None,
                forgets: None,
            },
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}

/// The wire form and digest downstream transcriptions (abbey-bot) must match.
/// Regenerate the JSON with `WDBX_WRITE_GOLDEN=1` after a deliberate change;
/// the digest constant is then updated by hand from the failing assertion.
#[test]
fn wire_form_and_digest_are_pinned_for_transcriptions() {
    let write = golden_write();
    let json = serde_json::to_string(&write).expect("write JSON");
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN_PATH);
    if std::env::var_os("WDBX_WRITE_GOLDEN").is_some() {
        fs::write(&golden, format!("{json}\n")).expect("write golden");
    }
    let stored = fs::read_to_string(&golden).expect("golden fixture present");
    assert_eq!(
        stored.trim_end(),
        json,
        "golden JSON drifted from the canonical type"
    );
    assert_eq!(
        serde_json::from_str::<EpisodeWrite>(&stored).expect("golden parses"),
        write
    );
    assert!(stored.contains("\"kind\":\"memory_candidate\""));

    let scratch = Scratch::new();
    let store = EpisodeStore::open(scratch.path(), golden_policy()).expect("open");
    let digest = store.preview_commitment(&write).expect("golden previews");
    assert_eq!(hex(&digest), GOLDEN_DIGEST);
}
