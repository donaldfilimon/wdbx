//! Retention classes and TTL expiry for the v3 episode store.
//!
//! The store binds a candidate's retention deadline when the candidate is
//! admitted, reports the deadline, and emits the `forgets` writes that close
//! the expired ones. The emitter never writes: the caller routes the emitted
//! writes through the one canonical write path, `propose_write`.
//!
//! What these tests pin:
//!
//! - a guild policy without a TTL expires nothing, exactly as before;
//! - a candidate is due only once its deadline has passed;
//! - the emitted forget write is an ordinary memory write and is accepted;
//! - a forgotten candidate is a tombstone: it is never forgotten again and
//!   never superseded;
//! - the deadline survives close and reopen, because the replay rebuilds it;
//! - a `durable` candidate never expires;
//! - a candidate under an open quarantine or contradiction is not silently
//!   forgotten;
//! - the write gate refuses a candidate whose guild TTL is zero, the only
//!   way a deadline can be closed at admission;
//! - a host policy that omits the TTL field still parses.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, EdgeReason, EpisodeEvent, EpisodeSource, EpisodeStore, EpisodeStoreError,
    EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MemoryCandidate, MemoryClass, MemoryEdge,
    MemoryEdgeKind, RetentionClass, StorePolicy, TerminalStatus,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// The retention window the tests configure.
const TTL_MS: u64 = 60_000;
const LEDGER_FILE: &str = "episodes.v1.jsonl";

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-retention-expiry-{}", Uuid::new_v4())))
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

fn guild_policy(ttl_ms: Option<u64>) -> GuildEpisodePolicy {
    GuildEpisodePolicy {
        learning_enabled: true,
        policy_version: "policy_v1".into(),
        token_budget: 1_000,
        storage_budget_bytes: 1024 * 1024,
        current_consent_epoch: Some(7),
        retention_ttl_ms: ttl_ms,
        retention_windows: None,
        authorized_principals: None,
    }
}

fn policy(ttl_ms: Option<u64>) -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([
            ("guild_ref".into(), guild_policy(ttl_ms)),
            ("other_guild".into(), guild_policy(ttl_ms)),
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

fn operational(seed: u8) -> MemoryCandidate {
    MemoryCandidate {
        retention: RetentionClass::Operational,
        ..candidate(seed)
    }
}

fn session(seed: u8) -> MemoryCandidate {
    MemoryCandidate {
        retention: RetentionClass::Session,
        ..candidate(seed)
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

fn edge(request_id: &str, operation_id: &str, edge: MemoryEdge) -> EpisodeWrite {
    write(
        request_id,
        operation_id,
        EpisodeEvent::MemoryEdge {
            recorded_by: actor("abbey_service", ActorKind::Service),
            edge,
        },
    )
}

fn resolution(request_id: &str, operation_id: &str, edge_digest: [u8; 32]) -> EpisodeWrite {
    write(
        request_id,
        operation_id,
        EpisodeEvent::MemoryEdge {
            recorded_by: actor("guild_owner", ActorKind::GuildOwner),
            edge: MemoryEdge {
                kind: MemoryEdgeKind::Resolves,
                target: edge_digest,
                counterpart: None,
                reason: EdgeReason::ReviewedValid,
            },
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

/// Every file under the store directory with its exact bytes.
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

fn wall_clock_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

#[test]
fn a_policy_without_a_ttl_expires_nothing() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(None)).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", operational(1)));

    let state = store
        .retention_state("guild_ref", &digest)
        .expect("retention state")
        .expect("admitted candidate");
    assert_eq!(state.class, RetentionClass::Operational);
    assert_eq!(state.expires_at, None, "no TTL, no deadline");
    assert!(!state.expired);
    assert_eq!(
        store
            .due_expirations("guild_ref", u64::MAX)
            .expect("due expirations"),
        Vec::<[u8; 32]>::new(),
        "a guild without a TTL expires nothing"
    );
    assert_eq!(
        store
            .expiry_writes("guild_ref", u64::MAX, "request_exp", "operation_exp")
            .expect("expiry writes"),
        Vec::<EpisodeWrite>::new(),
        "the emitter emits nothing without a TTL"
    );

    // The ledger line carries no time field: the record format is unchanged.
    let ledger = fs::read_to_string(scratch.path().join(LEDGER_FILE)).expect("ledger");
    assert!(!ledger.contains("expires_at"));
    assert!(!ledger.contains("recorded_at"));
}

#[test]
fn an_operational_candidate_is_due_only_after_its_ttl() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(TTL_MS))).expect("open");
    let before = wall_clock_ms();
    let digest = append(&mut store, memory("request_1", "memory_1", operational(1)));
    let after = wall_clock_ms();

    let state = store
        .retention_state("guild_ref", &digest)
        .expect("retention state")
        .expect("admitted candidate");
    assert_eq!(state.class, RetentionClass::Operational);
    let expires_at = state
        .expires_at
        .expect("a TTL-bound candidate has a deadline");
    assert!(
        expires_at >= before.saturating_add(TTL_MS) && expires_at <= after.saturating_add(TTL_MS),
        "the deadline is the arrival plus the TTL: {expires_at}"
    );
    assert!(!state.expired, "the deadline has not passed yet");
    assert!(
        !store
            .memory_edge_state("guild_ref", &digest)
            .expect("edge state")
            .expect("admitted candidate")
            .forgotten,
        "the candidate is still live"
    );

    assert!(
        store
            .due_expirations("guild_ref", expires_at - 1)
            .expect("due expirations")
            .is_empty(),
        "nothing is due before the deadline"
    );
    assert_eq!(
        store
            .due_expirations("guild_ref", expires_at)
            .expect("due expirations"),
        vec![digest],
        "the candidate is due once the deadline has passed"
    );
}

#[test]
fn the_emitted_forget_write_is_accepted_and_yields_a_receipt() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(TTL_MS))).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", operational(1)));
    let expires_at = store
        .retention_state("guild_ref", &digest)
        .expect("retention state")
        .expect("admitted candidate")
        .expires_at
        .expect("deadline");

    let writes = store
        .expiry_writes("guild_ref", expires_at, "request_exp", "operation_exp")
        .expect("expiry writes");
    assert_eq!(writes.len(), 1);
    let write = &writes[0];
    assert_eq!(write.guild_ref, "guild_ref");
    assert_eq!(write.contract_revision, 2);
    assert_eq!(write.contract_digest, [9; 32]);
    assert_eq!(write.policy_version, "policy_v1");
    let EpisodeEvent::MemoryCandidate {
        recorded_by,
        candidate,
    } = &write.event
    else {
        panic!("the emitter writes a memory candidate");
    };
    assert_eq!(recorded_by.kind, ActorKind::Service);
    assert_eq!(candidate.forgets, Some(digest));
    assert_eq!(candidate.payload_bytes, 0);
    assert_eq!(candidate.payload_commitment, [0; 32]);
    assert_eq!(candidate.supersedes, None);
    assert_eq!(
        candidate.retention,
        RetentionClass::Operational,
        "the forget carries the forgotten candidate's retention class"
    );

    let receipt = store.propose_write(write).expect("the forget appends");
    assert_eq!(receipt.event_kind, "memory_candidate");
    assert_eq!(receipt.terminal_status, Some(TerminalStatus::Completed));
    assert!(receipt.redacted);

    let state = store
        .memory_edge_state("guild_ref", &digest)
        .expect("edge state")
        .expect("admitted candidate");
    assert!(state.forgotten, "the candidate is forgotten");
    let lineage = store
        .supersession_lineage("guild_ref", &digest)
        .expect("lineage");
    assert!(lineage.forgotten);
    assert!(
        store
            .due_expirations("guild_ref", u64::MAX)
            .expect("due expirations")
            .is_empty(),
        "a forgotten candidate is not due again"
    );
}

#[test]
fn a_forgotten_candidate_is_never_forgotten_or_superseded_again() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(TTL_MS))).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", operational(1)));
    let expires_at = store
        .retention_state("guild_ref", &digest)
        .expect("retention state")
        .expect("admitted candidate")
        .expires_at
        .expect("deadline");
    let writes = store
        .expiry_writes("guild_ref", expires_at, "request_exp", "operation_exp")
        .expect("expiry writes");
    store.propose_write(&writes[0]).expect("the forget appends");

    // A second forget of the same target is refused.
    assert!(matches!(
        rejected(&mut store, &memory("request_2", "memory_2", forget(digest))),
        EpisodeStoreError::InvalidTransition
    ));
    // A supersession of the same target is refused too.
    assert!(matches!(
        rejected(
            &mut store,
            &memory(
                "request_3",
                "memory_3",
                MemoryCandidate {
                    supersedes: Some(digest),
                    ..candidate(3)
                }
            )
        ),
        EpisodeStoreError::InvalidTransition
    ));
    // The expiry decision does not resurrect it either.
    assert_eq!(
        store
            .due_expirations("guild_ref", u64::MAX)
            .expect("due expirations"),
        Vec::<[u8; 32]>::new()
    );
}

#[test]
fn the_retention_state_survives_close_and_reopen() {
    let scratch = Scratch::new();
    let store_policy = policy(Some(TTL_MS));
    let digest;
    let expires_at;
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        digest = append(&mut store, memory("request_1", "memory_1", operational(1)));
        expires_at = store
            .retention_state("guild_ref", &digest)
            .expect("retention state")
            .expect("admitted candidate")
            .expires_at
            .expect("deadline");
    }

    // The replay rebuilds the deadline from the retention index, so the
    // reopened store answers exactly like the store that wrote the record.
    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    let state = reopened
        .retention_state("guild_ref", &digest)
        .expect("retention state")
        .expect("admitted candidate");
    assert_eq!(state.class, RetentionClass::Operational);
    assert_eq!(
        state.expires_at,
        Some(expires_at),
        "the deadline is unchanged"
    );
    assert!(!state.expired);
    assert_eq!(
        reopened
            .due_expirations("guild_ref", expires_at)
            .expect("due expirations"),
        vec![digest],
        "the reopened store reaches the same decision"
    );
    assert_eq!(
        reopened
            .expiry_writes("guild_ref", expires_at, "request_exp", "operation_exp")
            .expect("expiry writes")
            .len(),
        1,
        "the reopened store emits the same forget"
    );
}

#[test]
fn a_durable_candidate_never_expires() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(TTL_MS))).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));

    let state = store
        .retention_state("guild_ref", &digest)
        .expect("retention state")
        .expect("admitted candidate");
    assert_eq!(state.class, RetentionClass::Durable);
    assert_eq!(state.expires_at, None, "a durable candidate never expires");
    assert!(!state.expired);
    assert_eq!(
        store
            .due_expirations("guild_ref", u64::MAX)
            .expect("due expirations"),
        Vec::<[u8; 32]>::new(),
        "no now_ms expires a durable candidate"
    );
    assert_eq!(
        store
            .expiry_writes("guild_ref", u64::MAX, "request_exp", "operation_exp")
            .expect("expiry writes"),
        Vec::<EpisodeWrite>::new(),
        "the emitter emits nothing for a durable candidate"
    );
}

#[test]
fn a_candidate_under_review_is_not_silently_forgotten() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(TTL_MS))).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", operational(1)));

    // A service quarantines the candidate: it stays visible and stays live.
    let quarantine = append(
        &mut store,
        edge(
            "request_2",
            "edge_2",
            MemoryEdge {
                kind: MemoryEdgeKind::Quarantines,
                target: digest,
                counterpart: None,
                reason: EdgeReason::SourceUntrusted,
            },
        ),
    );
    assert!(
        store
            .due_expirations("guild_ref", u64::MAX)
            .expect("due expirations")
            .is_empty(),
        "a candidate under review is not due"
    );
    assert!(
        store
            .expiry_writes("guild_ref", u64::MAX, "request_exp", "operation_exp")
            .expect("expiry writes")
            .is_empty(),
        "the emitter emits nothing for a candidate under review"
    );
    assert!(
        !store
            .memory_edge_state("guild_ref", &digest)
            .expect("edge state")
            .expect("admitted candidate")
            .forgotten,
        "the candidate is still live"
    );

    // The review resolves the edge; the deadline was never cleared.
    append(&mut store, resolution("request_3", "edge_3", quarantine));
    assert_eq!(
        store
            .due_expirations("guild_ref", u64::MAX)
            .expect("due expirations"),
        vec![digest],
        "the candidate is due once the review closed"
    );

    // A contradiction is the same kind of hold.
    let other = append(&mut store, memory("request_4", "memory_4", operational(2)));
    let (low, high) = if digest < other {
        (digest, other)
    } else {
        (other, digest)
    };
    append(
        &mut store,
        edge(
            "request_5",
            "edge_5",
            MemoryEdge {
                kind: MemoryEdgeKind::Contradicts,
                target: low,
                counterpart: Some(high),
                reason: EdgeReason::ConflictingObservation,
            },
        ),
    );
    assert!(
        store
            .due_expirations("guild_ref", u64::MAX)
            .expect("due expirations")
            .is_empty(),
        "a contradicted candidate is not due either"
    );
}

#[test]
fn the_gate_refuses_a_candidate_whose_ttl_is_zero() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(0))).expect("open");

    // A zero TTL is the only way a deadline can be closed at admission: the
    // deadline is bound from the store's own clock at that instant, so
    // `arrival + 0` is the arrival itself. The gate refuses the candidate
    // instead of admitting one that is already expired. It never compares a
    // deadline against a caller-supplied clock.
    assert!(matches!(
        rejected(&mut store, &memory("request_1", "memory_1", session(1))),
        EpisodeStoreError::InvalidInput
    ));
    assert!(matches!(
        rejected(&mut store, &memory("request_2", "memory_2", operational(2))),
        EpisodeStoreError::InvalidInput
    ));
    // The non-mutating decision reaches the same verdict as the append.
    assert!(matches!(
        store
            .decide_write(&memory("request_2b", "memory_2b", operational(2)))
            .expect_err("the decision refuses the expired candidate"),
        EpisodeStoreError::InvalidInput
    ));
    // A durable candidate never expires, so the same policy admits it.
    let digest = append(&mut store, memory("request_3", "memory_3", candidate(3)));
    assert!(
        store
            .retention_state("guild_ref", &digest)
            .expect("retention state")
            .expect("admitted candidate")
            .expires_at
            .is_none()
    );
    // A tombstone is not a memory and is never given a deadline.
    append(&mut store, memory("request_4", "memory_4", forget(digest)));
    assert_eq!(store.retrieve("guild_ref", 10).expect("receipts").len(), 2);
}

#[test]
fn a_host_policy_without_the_ttl_field_still_parses() {
    let policy_json = r#"{
        "learning_enabled": true,
        "policy_version": "policy_v1",
        "token_budget": 1000,
        "storage_budget_bytes": 1048576,
        "current_consent_epoch": null
    }"#;
    let guild: GuildEpisodePolicy = serde_json::from_str(policy_json).expect("policy parses");
    assert_eq!(guild.retention_ttl_ms, None);
    assert!(guild.learning_enabled);

    let store_json = r#"{
        "contract_revision": 2,
        "contract_digest": [1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
        "guilds": {
            "guild_ref": {
                "learning_enabled": true,
                "policy_version": "policy_v1",
                "token_budget": 1000,
                "storage_budget_bytes": 1048576,
                "current_consent_epoch": null
            }
        }
    }"#;
    let store_policy: StorePolicy = serde_json::from_str(store_json).expect("store policy parses");
    assert_eq!(
        store_policy.guilds["guild_ref"].retention_ttl_ms, None,
        "an omitted TTL means no expiry"
    );

    // A policy that names the TTL keeps its value.
    let with_ttl = r#"{
        "learning_enabled": true,
        "policy_version": "policy_v1",
        "token_budget": 1000,
        "storage_budget_bytes": 1048576,
        "current_consent_epoch": null,
        "retention_ttl_ms": 60000
    }"#;
    let guild: GuildEpisodePolicy = serde_json::from_str(with_ttl).expect("policy parses");
    assert_eq!(guild.retention_ttl_ms, Some(60_000));
}

#[test]
fn the_store_leaves_the_ledger_untouched_until_a_write_is_accepted() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(TTL_MS))).expect("open");
    let files = store_bytes(scratch.path());
    assert!(
        files.contains_key(&scratch.path().join(LEDGER_FILE)),
        "the ledger exists"
    );
    assert_eq!(
        files.get(&scratch.path().join(LEDGER_FILE)).map(Vec::len),
        Some(0),
        "opening a store appends nothing"
    );
    append(&mut store, memory("request_1", "memory_1", operational(1)));
    let after = store_bytes(scratch.path());
    assert!(
        after
            .get(&scratch.path().join(LEDGER_FILE))
            .map_or(0, Vec::len)
            > 0,
        "the accepted write reached the ledger"
    );
}
