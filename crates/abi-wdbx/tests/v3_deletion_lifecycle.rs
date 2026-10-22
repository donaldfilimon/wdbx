//! The deletion lifecycle of the v3 episode store: redaction of derived
//! copies, object-key destruction requests, and garbage collection of
//! expired and forgotten records.
//!
//! What these tests pin:
//!
//! - a redaction request for a forgotten target is recorded, is reported, and
//!   survives close and reopen;
//! - a redaction request for a live target is refused, and the refusal does
//!   not distinguish a live target from an unknown digest;
//! - an object-key destruction request for a live target is refused, and one
//!   for a forgotten target is recorded and reported;
//! - the collection plan never proposes reclaiming a tombstone, a live
//!   record, a record under an open review, or a record with no deadline;
//! - after the collection, the store still opens and still replays, and every
//!   surviving record keeps its digest byte for byte;
//! - a forgotten and expired record is reclaimed and disappears from the
//!   ledger, while its tombstone remains;
//! - an interrupted collection leaves the original ledger intact and
//!   openable;
//! - a reclaimed record never becomes live again: a `supersedes` naming it
//!   is refused exactly as a `supersedes` naming a forgotten record is;
//! - the sidecar files carry no episode content.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, DerivedCopyKind, EdgeReason, EpisodeEvent, EpisodeSigner, EpisodeSource,
    EpisodeStore, EpisodeStoreError, EpisodeWrite, EvidenceLevel, GuildEpisodePolicy,
    MemoryCandidate, MemoryClass, MemoryEdge, MemoryEdgeKind, ObjectKeyDestructionRequest,
    ObjectKeyRef, RedactionRequest, RetentionClass, SignatureStatus, StorePolicy,
};
use ed25519_dalek::SigningKey;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// The retention window the tests configure.
const TTL_MS: u64 = 60_000;
const LEDGER_FILE: &str = "episodes.v1.jsonl";
const REDACTION_FILE: &str = "episodes.v1.redaction.jsonl";
const OBJECT_KEY_FILE: &str = "episodes.v1.object_keys.jsonl";
const RECLAIMED_FILE: &str = "episodes.v1.reclaimed.jsonl";
const GC_TEMP_FILE: &str = "episodes.v1.gc.tmp";

/// A payload commitment that is unmistakable in a byte-level assertion.
const PAYLOAD_MARKER: [u8; 32] = [0x5a; 32];

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-deletion-lifecycle-{}", Uuid::new_v4())))
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

fn candidate(_seed: u8) -> MemoryCandidate {
    MemoryCandidate {
        class: MemoryClass::Fact,
        retention: RetentionClass::Durable,
        payload_commitment: PAYLOAD_MARKER,
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

/// The episode digests a guild's ledger holds, in ledger order.
fn digests(store: &EpisodeStore, guild_ref: &str) -> Vec<[u8; 32]> {
    store
        .retrieve(guild_ref, 1_000)
        .expect("receipts")
        .into_iter()
        .map(|receipt| receipt.episode_digest)
        .collect()
}

/// The ledger's lines, parsed, in ledger order.
fn ledger_lines(root: &Path) -> Vec<serde_json::Map<String, serde_json::Value>> {
    let text = fs::read_to_string(root.join(LEDGER_FILE)).expect("ledger");
    text.lines()
        .map(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .expect("the line parses")
                .as_object()
                .expect("the line is an object")
                .clone()
        })
        .collect()
}

/// Forget one candidate with an explicit write, the way a service would.
fn forget_direct(
    store: &mut EpisodeStore,
    request_id: &str,
    operation_id: &str,
    target: [u8; 32],
) -> [u8; 32] {
    append(store, memory(request_id, operation_id, forget(target)))
}

/// Forget one candidate the way the expiry engine does.
fn forget_candidate(store: &mut EpisodeStore, target: [u8; 32], now_ms: u64) -> [u8; 32] {
    let writes = store
        .expiry_writes("guild_ref", now_ms, "request_exp", "operation_exp")
        .expect("expiry writes");
    let write = writes
        .iter()
        .find(|write| {
            matches!(
                &write.event,
                EpisodeEvent::MemoryCandidate { candidate, .. } if candidate.forgets == Some(target)
            )
        })
        .expect("the emitter emits the forget");
    append(store, write.clone())
}

/// The deadline the store bound to one candidate.
fn deadline(store: &EpisodeStore, digest: &[u8; 32]) -> u64 {
    store
        .retention_state("guild_ref", digest)
        .expect("retention state")
        .expect("admitted candidate")
        .expires_at
        .expect("a TTL-bound candidate has a deadline")
}

#[test]
fn a_redaction_request_for_a_forgotten_target_is_recorded_and_survives_reopen() {
    let scratch = Scratch::new();
    let store_policy = policy(Some(TTL_MS));
    let digest;
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        digest = append(&mut store, memory("request_1", "memory_1", operational(1)));
        let expires_at = deadline(&store, &digest);
        forget_candidate(&mut store, digest, expires_at);

        // The target is forgotten, so a redaction request is admissible.
        let request = RedactionRequest {
            guild_ref: "guild_ref".into(),
            target_digest: digest,
            kinds: vec![DerivedCopyKind::Embedding, DerivedCopyKind::IndexEntry],
            actor: actor("guild_owner", ActorKind::GuildOwner),
        };
        let decision = store
            .decide_redaction(&request)
            .expect("the decision accepts");
        assert_eq!(decision.requested.len(), 2, "no request is recorded yet");
        assert_eq!(decision.already_requested.len(), 0);
        assert!(
            store
                .redaction_state("guild_ref", &digest)
                .expect("redaction state")
                .is_none(),
            "the decision records nothing"
        );

        let receipt = store
            .commit_redaction(&request)
            .expect("the request commits");
        assert_eq!(receipt.requested.len(), 2);
        assert_eq!(receipt.already_requested.len(), 0);

        let state = store
            .redaction_state("guild_ref", &digest)
            .expect("redaction state")
            .expect("a recorded request");
        assert_eq!(state.target_digest, digest);
        assert_eq!(
            state.kinds,
            vec![DerivedCopyKind::Embedding, DerivedCopyKind::IndexEntry]
        );

        // The request is idempotent per kind: a second request records
        // nothing new.
        let again = store
            .commit_redaction(&request)
            .expect("the request repeats");
        assert_eq!(again.requested.len(), 0);
        assert_eq!(again.already_requested.len(), 2);
    }

    // The request survives close and reopen: the sidecar is replayed.
    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    let state = reopened
        .redaction_state("guild_ref", &digest)
        .expect("redaction state")
        .expect("a recorded request");
    assert_eq!(
        state.kinds,
        vec![DerivedCopyKind::Embedding, DerivedCopyKind::IndexEntry]
    );
}

#[test]
fn a_redaction_request_for_a_live_target_is_refused() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(TTL_MS))).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", operational(1)));

    let request = RedactionRequest {
        guild_ref: "guild_ref".into(),
        target_digest: digest,
        kinds: vec![DerivedCopyKind::Export],
        actor: actor("guild_owner", ActorKind::GuildOwner),
    };
    assert!(matches!(
        store
            .decide_redaction(&request)
            .expect_err("a live target is refused"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(matches!(
        store
            .commit_redaction(&request)
            .expect_err("a live target is refused"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(
        store
            .redaction_state("guild_ref", &digest)
            .expect("redaction state")
            .is_none(),
        "a refused request is recorded nowhere"
    );

    // An unknown digest is refused the same way: the store is not an
    // existence oracle for the digests it does not hold.
    let unknown = RedactionRequest {
        guild_ref: "guild_ref".into(),
        target_digest: [7; 32],
        kinds: vec![DerivedCopyKind::Export],
        actor: actor("guild_owner", ActorKind::GuildOwner),
    };
    assert!(matches!(
        store
            .decide_redaction(&unknown)
            .expect_err("an unknown target is refused"),
        EpisodeStoreError::InvalidTransition
    ));
}

#[test]
fn an_object_key_destruction_is_refused_for_a_live_target_and_allowed_for_a_forgotten_one() {
    let scratch = Scratch::new();
    let store_policy = policy(Some(TTL_MS));
    let digest;
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        digest = append(&mut store, memory("request_1", "memory_1", operational(1)));
        let key = ObjectKeyRef {
            tenant: "tenant_a".into(),
            key_id: "object_1".into(),
        };
        let request = ObjectKeyDestructionRequest {
            guild_ref: "guild_ref".into(),
            target_digest: digest,
            key: key.clone(),
            actor: actor("guild_owner", ActorKind::GuildOwner),
        };

        // A live record's payload must not be destroyed under the canonical
        // ledger.
        assert!(matches!(
            store
                .decide_object_key_destruction(&request)
                .expect_err("a live target is refused"),
            EpisodeStoreError::InvalidTransition
        ));
        assert!(matches!(
            store
                .commit_object_key_destruction(&request)
                .expect_err("a live target is refused"),
            EpisodeStoreError::InvalidTransition
        ));
        assert!(
            store
                .object_key_destruction_state("guild_ref", &key)
                .expect("object key state")
                .is_none(),
            "a refused request is recorded nowhere"
        );

        // Once the target is forgotten, the request is admissible.
        let expires_at = deadline(&store, &digest);
        forget_candidate(&mut store, digest, expires_at);
        let receipt = store
            .commit_object_key_destruction(&request)
            .expect("the request commits");
        assert!(!receipt.already_requested);
        assert_eq!(receipt.key, key);

        let state = store
            .object_key_destruction_state("guild_ref", &key)
            .expect("object key state")
            .expect("a recorded request");
        assert_eq!(state.target_digest, digest);
        assert_eq!(state.guild_ref, "guild_ref");

        // The request is idempotent for the same target.
        let again = store
            .commit_object_key_destruction(&request)
            .expect("the request repeats");
        assert!(again.already_requested);
    }

    // The request survives close and reopen.
    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    let key = ObjectKeyRef {
        tenant: "tenant_a".into(),
        key_id: "object_1".into(),
    };
    let state = reopened
        .object_key_destruction_state("guild_ref", &key)
        .expect("object key state")
        .expect("a recorded request");
    assert_eq!(state.target_digest, digest);
}

#[test]
fn the_plan_never_reclaims_a_tombstone_a_live_record_a_reviewed_record_or_an_unknown_deadline() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(TTL_MS))).expect("open");

    // A live record is never reclaimed, whatever the clock says.
    let live = append(&mut store, memory("request_1", "memory_1", operational(1)));

    // A forgotten and expired record is reclaimable.
    let expired = append(&mut store, memory("request_2", "memory_2", operational(2)));
    let expired_deadline = deadline(&store, &expired);
    let tombstone = forget_candidate(&mut store, expired, expired_deadline);

    // A forgotten and expired record under an open review is not reclaimable.
    let reviewed = append(&mut store, memory("request_3", "memory_3", operational(3)));
    let reviewed_deadline = deadline(&store, &reviewed);
    append(
        &mut store,
        edge(
            "request_4",
            "edge_4",
            MemoryEdge {
                kind: MemoryEdgeKind::Quarantines,
                target: reviewed,
                counterpart: None,
                reason: EdgeReason::SourceUntrusted,
            },
        ),
    );
    // The expiry engine never expires a record under an open review, so the
    // tombstone is written explicitly.
    let _ = reviewed_deadline;
    forget_direct(&mut store, "request_5", "memory_5", reviewed);

    let plan = store.gc_plan("guild_ref", u64::MAX).expect("plan");
    assert_eq!(plan.reclaimable, vec![expired]);
    assert!(
        !plan.reclaimable.contains(&live),
        "a live record is never reclaimed"
    );
    assert!(
        !plan.reclaimable.contains(&reviewed),
        "a record under an open review is never reclaimed"
    );
    assert!(
        !plan.reclaimable.contains(&tombstone),
        "a tombstone is never reclaimed"
    );
    assert!(
        plan.retained_tombstones.contains(&tombstone),
        "a tombstone is retained forever"
    );
    assert_eq!(plan.reclaimable_count(), 1);
    assert_eq!(plan.retained_tombstone_count(), 2);
}

#[test]
fn a_record_with_an_unknown_deadline_is_never_reclaimed() {
    let scratch = Scratch::new();
    // No TTL: the store binds no deadline, so no record of this guild ever
    // expires on a guessed time.
    let mut store = EpisodeStore::open(scratch.path(), policy(None)).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    forget_direct(&mut store, "request_2", "memory_2", digest);

    let plan = store.gc_plan("guild_ref", u64::MAX).expect("plan");
    assert!(
        plan.reclaimable.is_empty(),
        "a record with no deadline is kept"
    );
    assert_eq!(plan.retained_tombstone_count(), 1);
}

#[test]
fn the_surviving_records_keep_their_digests_after_the_collection() {
    let scratch = Scratch::new();
    let store_policy = policy(Some(TTL_MS));
    let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");

    // Two live candidates, one of which is forgotten and expired.
    let kept = append(&mut store, memory("request_1", "memory_1", operational(1)));
    let reclaimed = append(&mut store, memory("request_2", "memory_2", operational(2)));
    let later = append(&mut store, memory("request_3", "memory_3", operational(3)));
    let expires_at = deadline(&store, &reclaimed);
    forget_candidate(&mut store, reclaimed, expires_at);

    let before = digests(&store, "guild_ref");
    assert_eq!(before.len(), 4, "three candidates and one tombstone");
    let before_lines = ledger_lines(scratch.path());
    let plan = store.gc_plan("guild_ref", expires_at).expect("plan");
    assert_eq!(plan.reclaimable, vec![reclaimed]);
    let report = store.apply_gc_plan(&plan).expect("the collection applies");
    assert_eq!(report.reclaimed, vec![reclaimed]);
    assert_eq!(report.skipped.len(), 0);
    assert_eq!(report.records_before, 4);
    assert_eq!(report.records_after, 3);
    assert!(
        report.bytes_after < report.bytes_before,
        "the collection returns the reclaimed bytes"
    );

    // Every surviving record keeps its digest, and the order is preserved.
    let after = digests(&store, "guild_ref");
    assert_eq!(after.len(), 3);
    assert_eq!(after, vec![before[0], before[2], before[3]]);

    // The surviving records' bytes are unchanged but for their position: the
    // canonical digest covers the header, the payload, and the parents, and
    // never the ledger sequence.
    let after_lines = ledger_lines(scratch.path());
    assert_eq!(before_lines.len(), 4);
    assert_eq!(after_lines.len(), 3);
    for (before_index, after_index) in [(0_usize, 0_usize), (2, 1), (3, 2)] {
        let mut before = before_lines[before_index].clone();
        let mut after = after_lines[after_index].clone();
        before.remove("sequence");
        after.remove("sequence");
        assert_eq!(
            before, after,
            "the surviving record is unchanged but for its position"
        );
    }

    // The store still opens and still replays.
    drop(store);
    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    assert_eq!(digests(&reopened, "guild_ref"), after);
    assert_eq!(
        reopened
            .gc_plan("guild_ref", u64::MAX)
            .expect("plan")
            .reclaimable
            .len(),
        0,
        "the reclaimed record is gone for good"
    );
    let _ = (kept, later);
}

#[test]
fn a_forgotten_and_expired_record_disappears_while_its_tombstone_remains() {
    let scratch = Scratch::new();
    let store_policy = policy(Some(TTL_MS));
    let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", operational(1)));
    let expires_at = deadline(&store, &digest);
    let tombstone = forget_candidate(&mut store, digest, expires_at);

    let plan = store.gc_plan("guild_ref", expires_at).expect("plan");
    assert_eq!(plan.reclaimable, vec![digest]);
    assert_eq!(plan.retained_tombstones, vec![tombstone]);
    store.apply_gc_plan(&plan).expect("the collection applies");

    // The payload-bearing record is gone; the tombstone remains.
    let remaining = digests(&store, "guild_ref");
    assert_eq!(remaining, vec![tombstone]);
    assert!(
        !remaining.contains(&digest),
        "the reclaimed record left the ledger"
    );
    assert!(
        store
            .find_receipt("guild_ref", &digest)
            .expect("receipt")
            .is_none(),
        "the reclaimed record has no receipt any more"
    );
    // The tombstone still says the candidate was forgotten.
    let lineage = store
        .supersession_lineage("guild_ref", &digest)
        .expect("lineage");
    assert!(lineage.forgotten, "a reclaimed candidate stays forgotten");

    drop(store);
    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    assert_eq!(digests(&reopened, "guild_ref"), vec![tombstone]);
}

#[test]
fn an_interrupted_collection_leaves_the_original_ledger_intact() {
    let scratch = Scratch::new();
    let store_policy = policy(Some(TTL_MS));
    let digest;
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        digest = append(&mut store, memory("request_1", "memory_1", operational(1)));
        let expires_at = deadline(&store, &digest);
        forget_candidate(&mut store, digest, expires_at);
        let before = fs::read(scratch.path().join(LEDGER_FILE)).expect("ledger");

        // The replacement fails: the temporary ledger cannot be created.
        fs::create_dir(scratch.path().join(GC_TEMP_FILE)).expect("block the replacement");
        let plan = store.gc_plan("guild_ref", expires_at).expect("plan");
        assert!(matches!(
            store
                .apply_gc_plan(&plan)
                .expect_err("the replacement fails"),
            EpisodeStoreError::Io
        ));

        // The original ledger is intact.
        let after = fs::read(scratch.path().join(LEDGER_FILE)).expect("ledger");
        assert_eq!(before, after, "the original ledger is untouched");
        assert_eq!(digests(&store, "guild_ref").len(), 2);
    }

    // And it still opens.
    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    assert_eq!(digests(&reopened, "guild_ref").len(), 2);
    assert!(
        reopened
            .find_receipt("guild_ref", &digest)
            .expect("receipt")
            .is_some(),
        "the interrupted collection reclaimed nothing"
    );
}

#[test]
fn a_reclaimed_record_never_becomes_live_again() {
    let scratch = Scratch::new();
    let store_policy = policy(Some(TTL_MS));
    let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", operational(1)));
    let expires_at = deadline(&store, &digest);
    forget_candidate(&mut store, digest, expires_at);
    let plan = store.gc_plan("guild_ref", expires_at).expect("plan");
    store.apply_gc_plan(&plan).expect("the collection applies");

    // A supersession naming the reclaimed record is refused exactly as a
    // supersession naming a forgotten record is.
    assert!(matches!(
        rejected(
            &mut store,
            &memory(
                "request_2",
                "memory_2",
                MemoryCandidate {
                    supersedes: Some(digest),
                    ..candidate(3)
                }
            )
        ),
        EpisodeStoreError::InvalidTransition
    ));
    // A second forget of the reclaimed record is refused too.
    assert!(matches!(
        rejected(&mut store, &memory("request_3", "memory_3", forget(digest))),
        EpisodeStoreError::InvalidTransition
    ));

    // And the refusal survives the reopen.
    drop(store);
    let mut reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    assert!(matches!(
        rejected(
            &mut reopened,
            &memory(
                "request_4",
                "memory_4",
                MemoryCandidate {
                    supersedes: Some(digest),
                    ..candidate(3)
                }
            )
        ),
        EpisodeStoreError::InvalidTransition
    ));
}

#[test]
fn the_collection_keeps_the_surviving_signatures_verifiable() {
    let scratch = Scratch::new();
    let store_policy = policy(Some(TTL_MS));
    let signer = EpisodeSigner::new(SigningKey::from_bytes(&[7; 32]));
    let mut store =
        EpisodeStore::open_with_signer(scratch.path(), store_policy.clone(), signer).expect("open");
    let kept = append(&mut store, memory("request_1", "memory_1", operational(1)));
    let reclaimed = append(&mut store, memory("request_2", "memory_2", operational(2)));
    let expires_at = deadline(&store, &reclaimed);
    forget_candidate(&mut store, reclaimed, expires_at);
    let plan = store.gc_plan("guild_ref", expires_at).expect("plan");
    store.apply_gc_plan(&plan).expect("the collection applies");
    drop(store);

    // The rewritten ledger keeps the detached signatures, and the reopened
    // store verifies them.
    let key = SigningKey::from_bytes(&[7; 32]);
    let verifying = key.verifying_key();
    let key_id = EpisodeSigner::new(key).key_id().clone();
    let reopened = EpisodeStore::open_with_signer(
        scratch.path(),
        store_policy,
        EpisodeSigner::new(SigningKey::from_bytes(&[7; 32])),
    )
    .expect("open");
    let status = reopened
        .signature_status("guild_ref", &kept, |id| {
            (id == &key_id).then_some(verifying)
        })
        .expect("signature status");
    assert!(
        matches!(status, Some(SignatureStatus::Valid(_))),
        "the surviving record keeps its signature"
    );
}

#[test]
fn the_sidecar_files_carry_no_episode_content() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(TTL_MS))).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", operational(1)));
    let expires_at = deadline(&store, &digest);
    forget_candidate(&mut store, digest, expires_at);
    store
        .commit_redaction(&RedactionRequest {
            guild_ref: "guild_ref".into(),
            target_digest: digest,
            kinds: vec![DerivedCopyKind::Embedding, DerivedCopyKind::Cache],
            actor: actor("guild_owner", ActorKind::GuildOwner),
        })
        .expect("the redaction request commits");
    store
        .commit_object_key_destruction(&ObjectKeyDestructionRequest {
            guild_ref: "guild_ref".into(),
            target_digest: digest,
            key: ObjectKeyRef {
                tenant: "tenant_a".into(),
                key_id: "object_1".into(),
            },
            actor: actor("guild_owner", ActorKind::GuildOwner),
        })
        .expect("the object-key request commits");
    let plan = store.gc_plan("guild_ref", expires_at).expect("plan");
    store.apply_gc_plan(&plan).expect("the collection applies");
    drop(store);

    let files = store_bytes(scratch.path());
    let redaction = files
        .get(&scratch.path().join(REDACTION_FILE))
        .expect("the redaction index exists");
    let object_keys = files
        .get(&scratch.path().join(OBJECT_KEY_FILE))
        .expect("the object-key index exists");
    let reclaimed = files
        .get(&scratch.path().join(RECLAIMED_FILE))
        .expect("the reclaimed index exists");

    // The payload commitment never appears in a sidecar.
    let marker = format!("{PAYLOAD_MARKER:?}");
    for bytes in [redaction, object_keys, reclaimed] {
        assert!(
            !String::from_utf8_lossy(bytes).contains(&marker),
            "a sidecar carries no payload commitment"
        );
    }

    // Every sidecar line carries only digests, kinds, and identifiers. The
    // two destruction sidecars open with a header line that names their
    // format and the request lines they hold.
    for name in [REDACTION_FILE, OBJECT_KEY_FILE, RECLAIMED_FILE] {
        let (keys, headed) = sidecar_fields(name);
        let bytes = files
            .get(&scratch.path().join(name))
            .expect("the sidecar exists");
        let text = String::from_utf8(bytes.clone()).expect("the sidecar is text");
        assert!(!text.is_empty(), "{name} holds its request");
        let mut lines = text.lines();
        if headed {
            let front: serde_json::Value =
                serde_json::from_str(lines.next().expect("the sidecar opens with a header"))
                    .expect("the header parses");
            let object = front.as_object().expect("the header is an object");
            let mut found: Vec<&str> = object.keys().map(String::as_str).collect();
            found.sort_unstable();
            assert_eq!(
                found,
                vec!["checksum", "format", "records"],
                "{name} opens with its header"
            );
        }
        for line in lines {
            let value: serde_json::Value = serde_json::from_str(line).expect("the line parses");
            let object = value.as_object().expect("the line is an object");
            let mut found: Vec<&str> = object.keys().map(String::as_str).collect();
            found.sort_unstable();
            assert_eq!(found, keys, "{name} holds only its own fields");
        }
    }
}

/// The fields one sidecar line may hold, and whether the sidecar opens with
/// a header line.
fn sidecar_fields(name: &str) -> (Vec<&'static str>, bool) {
    match name {
        REDACTION_FILE => (
            vec![
                "authority_kind",
                "checksum",
                "digest",
                "guild_ref",
                "kind",
                "principal_id",
            ],
            true,
        ),
        OBJECT_KEY_FILE => (
            vec![
                "authority_kind",
                "checksum",
                "digest",
                "guild_ref",
                "key_id",
                "principal_id",
                "tenant",
            ],
            true,
        ),
        _ => (
            vec![
                "checksum",
                "digest",
                "guild_ref",
                "member_scoped",
                "retention",
            ],
            false,
        ),
    }
}
