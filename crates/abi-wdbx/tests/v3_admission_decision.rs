//! Structured admission decisions for the v3 episode write gate.
//!
//! The decision names the class a write would be admitted as and the reason
//! a write was refused, and it must reach exactly the verdict the append
//! path reaches: the same validation, the same commitment check, and the
//! same quota checks, with no effect on the ledger.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, AdmissionClass, AdmissionDecision, AdmissionReason, EpisodeEvent,
    EpisodeSource, EpisodeStore, EpisodeStoreError, EpisodeWrite, EvidenceLevel,
    GuildEpisodePolicy, MemoryCandidate, MemoryClass, RetentionClass, StorePolicy, TerminalReason,
    TerminalStatus, classify, classify_write,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const LEDGER_FILE: &str = "episodes.v1.jsonl";

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-admission-{}", Uuid::new_v4())))
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
                retention_ttl_ms: None,
                retention_windows: None,
                authorized_principals: None,
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

fn candidate(seed: u8, retention: RetentionClass) -> MemoryCandidate {
    MemoryCandidate {
        class: MemoryClass::Fact,
        retention,
        payload_commitment: [seed; 32],
        payload_bytes: 64,
        dimension: None,
        embedding_version: None,
        member_scoped: true,
        supersedes: None,
        forgets: None,
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

/// Every file under the store directory with its exact bytes, so a decision
/// can be shown to leave the ledger, the lock, and any sidecar untouched.
fn store_bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("read store dir") {
            let path = entry.expect("store dir entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = fs::read(&path).expect("read store file");
                files.insert(path, bytes);
            }
        }
    }
    files
}

fn ledger_len(root: &Path) -> u64 {
    fs::metadata(root.join(LEDGER_FILE)).map_or(0, |meta| meta.len())
}

/// The refusal the decision reports, with the reason it maps to.
fn refused(store: &EpisodeStore, event: &EpisodeWrite) -> (EpisodeStoreError, AdmissionReason) {
    let error = store.decide_write(event).expect_err("write is refused");
    let reason = AdmissionReason::from(&error);
    (error, reason)
}

#[test]
fn v3_admission_decision_accepts_a_valid_write_without_touching_the_ledger() {
    let scratch = Scratch::new();
    let store = EpisodeStore::open(scratch.path(), policy(true, 1_000, 100_000)).expect("open");
    let before = store_bytes(scratch.path());

    let decision = store
        .decide_write(&proposal("request_1", "operation_1"))
        .expect("a valid write is admitted");
    assert_eq!(decision.reason(), AdmissionReason::Accepted);
    assert_eq!(decision.class(), AdmissionClass::Durable);
    assert!(decision.receipt().is_none(), "a decision appends nothing");

    assert_eq!(
        store_bytes(scratch.path()),
        before,
        "a decision must not change any store byte"
    );
    assert_eq!(ledger_len(scratch.path()), 0);
    assert_eq!(
        store.retrieve("guild_ref", 10).expect("receipts"),
        Vec::new()
    );

    // The debug form is content-free: no identifier, digest, or label.
    let debug = format!("{decision:?}");
    assert!(debug.contains("Accepted"), "{debug}");
    for forbidden in ["request_1", "operation_1", "guild_ref", "requester_ref"] {
        assert!(!debug.contains(forbidden), "{forbidden} leaked: {debug}");
    }
}

#[test]
fn v3_admission_decision_classifies_the_requested_retention_and_defaults_to_durable() {
    let scratch = Scratch::new();
    let store = EpisodeStore::open(scratch.path(), policy(true, 1_000, 100_000)).expect("open");

    for (retention, expected) in [
        (RetentionClass::Session, AdmissionClass::Session),
        (RetentionClass::Operational, AdmissionClass::Operational),
        (RetentionClass::Durable, AdmissionClass::Durable),
    ] {
        let candidate = candidate(1, retention);
        assert_eq!(classify(&candidate), expected);
        // The same request identifier every time: a decision consumes none.
        let decision = store
            .decide_write(&memory("request_x", "operation_x", candidate))
            .expect("a candidate is admitted");
        assert_eq!(decision.class(), expected);
        assert_eq!(decision.reason(), AdmissionReason::Accepted);
    }

    // A plain episode write carries no retention request and is durable.
    let plain = proposal("request_plain", "operation_plain");
    let decision = store.decide_write(&plain).expect("a proposal is admitted");
    assert_eq!(decision.class(), AdmissionClass::Durable);
    assert_eq!(decision.class(), classify_write(&plain));
    assert_eq!(
        AdmissionDecision::accepted(AdmissionClass::Durable).reason(),
        AdmissionReason::Accepted
    );
}

#[test]
fn v3_admission_decision_maps_every_refusal_to_its_own_reason() {
    let scratch = Scratch::new();
    let store = EpisodeStore::open(scratch.path(), policy(true, 1_000, 100_000)).expect("open");

    // QUIET suppresses the durable write entirely.
    let mut quiet = proposal("request_1", "operation_1");
    quiet.quiet = true;
    let (error, reason) = refused(&store, &quiet);
    assert!(matches!(error, EpisodeStoreError::Quiet));
    assert_eq!(reason, AdmissionReason::Quiet);

    // A stale contract revision is a stale binding, not invalid input.
    let mut stale = proposal("request_2", "operation_2");
    stale.contract_revision = 1;
    let (error, reason) = refused(&store, &stale);
    assert!(matches!(error, EpisodeStoreError::StaleBinding));
    assert_eq!(reason, AdmissionReason::StaleBinding);

    // A guild the policy does not name is a stale binding too.
    let mut unknown = proposal("request_3", "operation_3");
    unknown.guild_ref = "unknown_guild".into();
    let (error, reason) = refused(&store, &unknown);
    assert!(matches!(error, EpisodeStoreError::StaleBinding));
    assert_eq!(reason, AdmissionReason::StaleBinding);

    // An unbounded identifier is invalid input.
    let mut invalid = proposal("request_4", "operation_4");
    invalid.request_id = "Bad Request".into();
    let (error, reason) = refused(&store, &invalid);
    assert!(matches!(error, EpisodeStoreError::InvalidInput));
    assert_eq!(reason, AdmissionReason::InvalidInput);

    // An approval for an operation that was never proposed.
    let approval = write(
        "request_5",
        "operation_5",
        EpisodeEvent::Approval {
            approved_by: actor("admin_ref", ActorKind::GuildAdministrator),
        },
    );
    let (error, reason) = refused(&store, &approval);
    assert!(matches!(error, EpisodeStoreError::InvalidTransition));
    assert_eq!(reason, AdmissionReason::InvalidTransition);

    // A caller prediction that does not match the canonical commitment.
    let mut mismatch = proposal("request_6", "operation_6");
    mismatch.expected_commitment = Some([4; 32]);
    let (error, reason) = refused(&store, &mismatch);
    assert!(matches!(error, EpisodeStoreError::CommitmentMismatch));
    assert_eq!(reason, AdmissionReason::CommitmentMismatch);

    // A malformed candidate is invalid input before any graph rule applies.
    let malformed = memory(
        "request_7",
        "operation_7",
        MemoryCandidate {
            payload_bytes: 0,
            ..candidate(1, RetentionClass::Durable)
        },
    );
    let (error, reason) = refused(&store, &malformed);
    assert!(matches!(error, EpisodeStoreError::InvalidInput));
    assert_eq!(reason, AdmissionReason::InvalidInput);

    // A replayed request identifier.
    let replayed = Scratch::new();
    let mut replaying =
        EpisodeStore::open(replayed.path(), policy(true, 1_000, 100_000)).expect("open");
    append(&mut replaying, proposal("request_8", "operation_8"));
    let (error, reason) = refused(&replaying, &proposal("request_8", "operation_9"));
    assert!(matches!(error, EpisodeStoreError::Replay));
    assert_eq!(reason, AdmissionReason::Replay);

    // Guild learning is not opted in.
    let other = Scratch::new();
    let disabled = EpisodeStore::open(other.path(), policy(false, 1_000, 100_000)).expect("open");
    let (error, reason) = refused(&disabled, &proposal("request_9", "operation_9"));
    assert!(matches!(error, EpisodeStoreError::LearningDisabled));
    assert_eq!(reason, AdmissionReason::LearningDisabled);

    // The guild token budget is exhausted.
    let tight = Scratch::new();
    let bounded = EpisodeStore::open(tight.path(), policy(true, 1, 100_000)).expect("open");
    let mut over = proposal("request_10", "operation_10");
    over.token_cost = 2;
    let (error, reason) = refused(&bounded, &over);
    assert!(matches!(error, EpisodeStoreError::TokenBudget));
    assert_eq!(reason, AdmissionReason::TokenBudget);

    // The guild storage budget is exhausted.
    let bytes = Scratch::new();
    let byte_limited = EpisodeStore::open(bytes.path(), policy(true, 1_000, 1)).expect("open");
    let (error, reason) = refused(&byte_limited, &proposal("request_11", "operation_11"));
    assert!(matches!(error, EpisodeStoreError::StorageBudget));
    assert_eq!(reason, AdmissionReason::StorageBudget);

    // Every reason carries its own stable label.
    assert_eq!(AdmissionReason::Accepted.label(), "accepted");
    assert_eq!(AdmissionReason::TokenBudget.label(), "token_budget");
    assert_eq!(AdmissionReason::StorageBudget.label(), "storage_budget");
    assert_eq!(AdmissionReason::StaleBinding.label(), "stale_binding");
}

#[test]
fn v3_admission_decision_and_the_append_always_agree() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(true, 1_000, 100_000)).expect("open");

    let mut quiet = proposal("request_4", "operation_4");
    quiet.quiet = true;
    let mut stale = proposal("request_5", "operation_5");
    stale.contract_revision = 1;
    let mut unknown = proposal("request_6", "operation_6");
    unknown.guild_ref = "unknown_guild".into();
    let mut invalid = proposal("request_7", "operation_7");
    invalid.request_id = "Bad Request".into();
    let mut mismatch = proposal("request_9", "operation_9");
    mismatch.expected_commitment = Some([4; 32]);
    let approval = write(
        "request_8",
        "operation_8",
        EpisodeEvent::Approval {
            approved_by: actor("admin_ref", ActorKind::GuildAdministrator),
        },
    );
    let malformed = memory(
        "request_10",
        "operation_10",
        MemoryCandidate {
            payload_bytes: 0,
            ..candidate(2, RetentionClass::Durable)
        },
    );

    let cases = vec![
        ("valid proposal", proposal("request_1", "operation_1")),
        ("replayed request", proposal("request_1", "operation_2")),
        ("replayed operation", proposal("request_2", "operation_1")),
        ("quiet write", quiet),
        ("stale contract", stale),
        ("unknown guild", unknown),
        ("invalid identifier", invalid),
        ("approval without proposal", approval),
        ("commitment mismatch", mismatch),
        ("malformed candidate", malformed),
        (
            "valid candidate",
            memory(
                "request_11",
                "operation_11",
                candidate(3, RetentionClass::Operational),
            ),
        ),
        (
            "second valid proposal",
            proposal("request_12", "operation_12"),
        ),
    ];

    for (name, event) in cases {
        let decision = store.decide_write(&event);
        let appended = store.propose_write(&event);
        match (decision, appended) {
            (Ok(decision), Ok(receipt)) => {
                assert_eq!(
                    decision.reason(),
                    AdmissionReason::Accepted,
                    "{name}: an accepted decision"
                );
                assert!(decision.receipt().is_none(), "{name}: no receipt yet");
                assert!(receipt.redacted, "{name}: a content-free receipt");
            }
            (Err(decision_error), Err(append_error)) => {
                assert_eq!(
                    decision_error.to_string(),
                    append_error.to_string(),
                    "{name}: both paths refuse alike"
                );
                assert_eq!(
                    AdmissionReason::from(&decision_error),
                    AdmissionReason::from(&append_error),
                    "{name}: both paths name the same reason"
                );
            }
            (Ok(_), Err(_)) => panic!("{name}: the decision admitted what the append refused"),
            (Err(_), Ok(_)) => panic!("{name}: the append admitted what the decision refused"),
        }
    }
}

#[test]
fn v3_admission_decision_refuses_a_budget_exhausted_write_before_any_append() {
    let scratch = Scratch::new();
    let store = EpisodeStore::open(scratch.path(), policy(true, 1, 100_000)).expect("open");
    let mut over = proposal("request_1", "operation_1");
    over.token_cost = 2;

    let error = store
        .decide_write(&over)
        .expect_err("the token budget is spent");
    assert!(matches!(error, EpisodeStoreError::TokenBudget));
    assert_eq!(AdmissionReason::from(&error), AdmissionReason::TokenBudget);

    // The preview path no longer promises a commitment the store would
    // refuse to append.
    assert!(matches!(
        store.preview_commitment(&over),
        Err(EpisodeStoreError::TokenBudget)
    ));
    assert_eq!(ledger_len(scratch.path()), 0);
    assert_eq!(
        store.retrieve("guild_ref", 10).expect("receipts"),
        Vec::new()
    );

    // The same holds for the storage budget.
    let bytes = Scratch::new();
    let byte_limited = EpisodeStore::open(bytes.path(), policy(true, 1_000, 1)).expect("open");
    let error = byte_limited
        .decide_write(&proposal("request_2", "operation_2"))
        .expect_err("the storage budget is spent");
    assert!(matches!(error, EpisodeStoreError::StorageBudget));
    assert_eq!(
        AdmissionReason::from(&error),
        AdmissionReason::StorageBudget
    );
    assert!(matches!(
        byte_limited.preview_commitment(&proposal("request_2", "operation_2")),
        Err(EpisodeStoreError::StorageBudget)
    ));
    assert_eq!(ledger_len(bytes.path()), 0);
}

#[test]
fn v3_admission_decision_the_outcome_never_fails_and_agrees_with_the_decision() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(true, 1_000, 100_000)).expect("open");

    let mut quiet = proposal("request_4", "operation_4");
    quiet.quiet = true;
    let mut stale = proposal("request_5", "operation_5");
    stale.contract_revision = 1;
    let mut unknown = proposal("request_6", "operation_6");
    unknown.guild_ref = "unknown_guild".into();
    let mut invalid = proposal("request_7", "operation_7");
    invalid.request_id = "Bad Request".into();
    let mut mismatch = proposal("request_9", "operation_9");
    mismatch.expected_commitment = Some([4; 32]);
    let approval = write(
        "request_8",
        "operation_8",
        EpisodeEvent::Approval {
            approved_by: actor("admin_ref", ActorKind::GuildAdministrator),
        },
    );
    let malformed = memory(
        "request_10",
        "operation_10",
        MemoryCandidate {
            payload_bytes: 0,
            ..candidate(2, RetentionClass::Durable)
        },
    );

    let cases = vec![
        ("valid proposal", proposal("request_1", "operation_1")),
        (
            "valid candidate",
            memory(
                "request_11",
                "operation_11",
                candidate(3, RetentionClass::Operational),
            ),
        ),
        ("quiet write", quiet),
        ("stale contract", stale),
        ("unknown guild", unknown),
        ("invalid identifier", invalid),
        ("approval without proposal", approval),
        ("commitment mismatch", mismatch),
        ("malformed candidate", malformed),
    ];

    for (name, event) in cases {
        let outcome = store.decide_write_outcome(&event);
        // The outcome never fails: it is one value, whatever the verdict.
        assert_eq!(
            outcome.class(),
            classify_write(&event),
            "{name}: the outcome names the class the write would be admitted as"
        );
        assert!(outcome.receipt().is_none(), "{name}: nothing was appended");
        match store.decide_write(&event) {
            Ok(decision) => {
                assert_eq!(
                    outcome.reason(),
                    AdmissionReason::Accepted,
                    "{name}: the outcome agrees with the decision"
                );
                assert_eq!(outcome.reason(), decision.reason(), "{name}");
            }
            Err(error) => {
                assert_eq!(
                    outcome.reason(),
                    AdmissionReason::from(&error),
                    "{name}: the outcome names the reason the refusal maps to"
                );
                assert_ne!(outcome.reason(), AdmissionReason::Accepted, "{name}");
            }
        }
        // The debug form stays content-free: no identifier, digest, or label
        // from the write.
        let debug = format!("{outcome:?}");
        for forbidden in ["request_", "operation_", "guild_ref", "admin_ref"] {
            assert!(!debug.contains(forbidden), "{forbidden} leaked: {debug}");
        }
    }

    // A replayed request and an exhausted budget are refusals too.
    append(&mut store, proposal("request_20", "operation_20"));
    let replayed = store.decide_write_outcome(&proposal("request_20", "operation_21"));
    assert_eq!(replayed.reason(), AdmissionReason::Replay);

    let tight = Scratch::new();
    let bounded = EpisodeStore::open(tight.path(), policy(true, 1, 100_000)).expect("open");
    let mut over = proposal("request_22", "operation_22");
    over.token_cost = 2;
    assert_eq!(
        bounded.decide_write_outcome(&over).reason(),
        AdmissionReason::TokenBudget
    );
    let bytes = Scratch::new();
    let byte_limited = EpisodeStore::open(bytes.path(), policy(true, 1_000, 1)).expect("open");
    assert_eq!(
        byte_limited
            .decide_write_outcome(&proposal("request_23", "operation_23"))
            .reason(),
        AdmissionReason::StorageBudget
    );

    // The decision path still reports the store error itself, unchanged.
    assert!(matches!(
        store.decide_write(&proposal("request_20", "operation_24")),
        Err(EpisodeStoreError::Replay)
    ));
}

#[test]
fn v3_admission_decision_does_not_consume_the_request_it_refused() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(true, 1_000, 100_000)).expect("open");
    let mut stale = proposal("request_1", "operation_1");
    stale.contract_revision = 1;
    let error = store.decide_write(&stale).expect_err("stale binding");
    assert!(matches!(error, EpisodeStoreError::StaleBinding));

    // The refused request claimed neither its request nor its operation, so
    // the same write still appends afterwards.
    let digest = append(&mut store, proposal("request_1", "operation_1"));
    let receipts = store.retrieve("guild_ref", 10).expect("receipts");
    assert_eq!(receipts.len(), 1);
    assert_eq!(
        store.find_receipt("guild_ref", &digest).expect("lookup"),
        Some(receipts[0].clone())
    );
}

#[test]
fn v3_admission_decision_accepts_a_terminal_chain_alike_the_append_path() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(true, 1_000, 100_000)).expect("open");
    let chain = vec![
        proposal("request_1", "operation_1"),
        write(
            "request_2",
            "operation_1",
            EpisodeEvent::Approval {
                approved_by: actor("admin_ref", ActorKind::GuildAdministrator),
            },
        ),
        write(
            "request_3",
            "operation_1",
            EpisodeEvent::Execution {
                executed_by: actor("bot_service", ActorKind::Service),
                voice: None,
            },
        ),
        write(
            "request_4",
            "operation_1",
            EpisodeEvent::Terminal {
                status: TerminalStatus::Completed,
                reason: TerminalReason::Completed,
            },
        ),
    ];
    for event in chain {
        let decision = store
            .decide_write(&event)
            .expect("every lifecycle event is admitted");
        assert_eq!(decision.reason(), AdmissionReason::Accepted);
        assert_eq!(decision.class(), AdmissionClass::Durable);
        append(&mut store, event);
    }
    assert_eq!(store.retrieve("guild_ref", 10).expect("receipts").len(), 4);
}
