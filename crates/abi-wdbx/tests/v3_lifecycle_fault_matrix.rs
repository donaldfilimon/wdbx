//! The named crash-boundary matrix of the v3 episode store's lifecycle.
//!
//! Every row composes a crash state from real pre-state and post-state file
//! bytes: it runs the store up to a boundary, captures every file under the
//! store directory, and writes back the mixture a crash at that boundary
//! leaves behind. It then reopens the store twice and byte-compares the two
//! reopens, so recovery is idempotent, and checks what the store answers.
//!
//! What the rows pin:
//!
//! - `retention_stamp_synced_ledger_line_missing`: an orphan retention stamp
//!   (synced before the ledger line it describes, which never landed) leaves
//!   `retention_state` and `due_expirations` unchanged. A retry of the same
//!   write binds a second stamp for the same digest, and the effective
//!   deadline after reopen is the retry's: the index is **last-wins**. This
//!   is pinned as observed behavior, not as a fix.
//! - `retry_after_crash_between_retention_and_ledger_extends_deadline`: the
//!   same crash, read from the other side: the retry's later arrival extends
//!   the deadline past the orphan's, so the candidate is not due at the
//!   orphan's deadline.
//! - `ledger_tail_partial`: a torn final ledger line is truncated on the
//!   first reopen, and the second reopen changes no byte.
//! - `gc_reclaimed_committed_rename_blocked_then_reopen_then_retry`: the
//!   reclaimed index is committed but the ledger rewrite never lands; after
//!   a reopen the retried collection reaches exactly the ledger and the
//!   single-line reclaimed index of an uninterrupted run.
//! - `gc_temp_left_after_full_write`: a fully written temporary ledger that
//!   was never renamed is ignored by the reopen and replaced by the retry.
//! - `sidecar_tmp_left_behind`: a stale object-key or quota sidecar
//!   temporary file is ignored by the reopen and replaced by the next
//!   commit.
//! - `tombstone_never_resurrected_after_each_row`: after every row, a
//!   `supersedes` or a `forgets` naming an erased digest is refused, before
//!   and after a reopen.
//!
//! File composition is a simulation of a crash at a named boundary. It is
//! not evidence of power-loss or hardware durability.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeSource, EpisodeStore, EpisodeStoreError,
    EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MemoryCandidate, MemoryClass,
    ObjectKeyDestructionRequest, ObjectKeyRef, QuotaAdjustmentRequest, RetentionClass, StorePolicy,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

/// The retention window: long enough that no deadline passes while a test
/// runs, so a deadline is reached only by an explicit `now_ms`.
const TTL_MS: u64 = 3_600_000;
const GUILD: &str = "guild_ref";
const LEDGER_FILE: &str = "episodes.v1.jsonl";
const RETENTION_FILE: &str = "episodes.v1.retention.jsonl";
const RECLAIMED_FILE: &str = "episodes.v1.reclaimed.jsonl";
const OBJECT_KEY_FILE: &str = "episodes.v1.object_keys.jsonl";
const QUOTA_FILE: &str = "episodes.v1.quotas.jsonl";
const GC_TEMP_FILE: &str = "episodes.v1.gc.tmp";
const OBJECT_KEY_TEMP: &str = ".episodes.v1.object_keys.jsonl.tmp";
const QUOTA_TEMP: &str = ".episodes.v1.quotas.jsonl.tmp";

/// A disposable store directory that removes itself.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-fault-matrix-{}", Uuid::new_v4())))
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

type Files = BTreeMap<PathBuf, Vec<u8>>;

fn actor(principal_id: &str, kind: ActorKind) -> ActorRef {
    ActorRef {
        principal_id: principal_id.into(),
        kind,
    }
}

fn owner() -> ActorRef {
    actor("guild_owner", ActorKind::GuildOwner)
}

fn policy() -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([(
            GUILD.into(),
            GuildEpisodePolicy {
                learning_enabled: true,
                policy_version: "policy_v1".into(),
                token_budget: 1_000,
                storage_budget_bytes: 1024 * 1024,
                current_consent_epoch: Some(7),
                retention_ttl_ms: Some(TTL_MS),
                retention_windows: None,
                authorized_principals: None,
            },
        )]),
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

fn forget(target: [u8; 32]) -> MemoryCandidate {
    MemoryCandidate {
        payload_commitment: [0; 32],
        payload_bytes: 0,
        forgets: Some(target),
        ..candidate(0)
    }
}

fn supersede(seed: u8, target: [u8; 32]) -> MemoryCandidate {
    MemoryCandidate {
        supersedes: Some(target),
        ..candidate(seed)
    }
}

fn memory(request_id: &str, candidate: MemoryCandidate) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: format!("op_{request_id}"),
        contract_revision: 2,
        contract_digest: [9; 32],
        guild_ref: GUILD.into(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C1,
        event: EpisodeEvent::MemoryCandidate {
            recorded_by: actor("abbey_service", ActorKind::Service),
            candidate,
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}

fn append(store: &mut EpisodeStore, mut event: EpisodeWrite) -> [u8; 32] {
    let preview = store
        .preview_commitment(&event)
        .expect("valid event previews");
    event.expected_commitment = Some(preview);
    let receipt = store.propose_write(&event).expect("valid event appends");
    assert_eq!(receipt.episode_digest, preview);
    preview
}

/// Refuse one write, checking that the preview gives the same reason.
fn rejected(store: &mut EpisodeStore, event: &EpisodeWrite) -> EpisodeStoreError {
    let error = store.propose_write(event).expect_err("rejected write");
    assert_eq!(
        store
            .preview_commitment(event)
            .map(|_| ())
            .expect_err("rejected preview")
            .to_string(),
        error.to_string(),
        "preview and append agree on the rejection"
    );
    error
}

fn open(root: &Path) -> EpisodeStore {
    EpisodeStore::open(root, policy()).expect("the store opens")
}

/// Every file under the store directory with its exact bytes, keyed by its
/// path relative to the store root so two stores compare.
fn store_bytes(root: &Path) -> Files {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("read store dir") {
            let path = entry.expect("store dir entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path.strip_prefix(root).expect("under root").to_path_buf();
                files.insert(relative, fs::read(&path).expect("read store file"));
            }
        }
    }
    files
}

/// Replace the store directory with exactly `files`.
fn write_state(root: &Path, files: &Files) {
    let _ = fs::remove_dir_all(root);
    fs::create_dir_all(root).expect("store dir");
    for (relative, bytes) in files {
        fs::write(root.join(relative), bytes).expect("write composed file");
    }
}

fn file(files: &Files, name: &str) -> Vec<u8> {
    files
        .get(Path::new(name))
        .unwrap_or_else(|| panic!("{name} is present"))
        .clone()
}

fn with(files: &Files, name: &str, bytes: Vec<u8>) -> Files {
    let mut composed = files.clone();
    composed.insert(PathBuf::from(name), bytes);
    composed
}

/// A hex SHA-256 over every path and its bytes, for the evidence log.
fn state_digest(files: &Files) -> String {
    let mut hasher = Sha256::new();
    for (path, bytes) in files {
        let name = path.to_string_lossy();
        hasher.update(u64::try_from(name.len()).expect("len").to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update(u64::try_from(bytes.len()).expect("len").to_le_bytes());
        hasher.update(bytes);
    }
    hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").expect("writing to a string succeeds");
            hex
        })
}

/// Reopen the store twice, assert the second reopen changed no byte, and
/// print the row's evidence line.
fn reopen_twice(row: &str, root: &Path, crash: &Files) -> (EpisodeStore, Files) {
    drop(open(root));
    let first = store_bytes(root);
    let store = open(root);
    let second = store_bytes(root);
    assert_eq!(first, second, "{row}: the second reopen changed a byte");
    println!(
        "row={row} before={} after={} reopen1_eq_reopen2=true",
        state_digest(crash),
        state_digest(&second)
    );
    (store, second)
}

/// The retention stamps, in file order, as `(digest, expires_at_ms)`.
fn stamps(files: &Files) -> Vec<(Vec<u8>, u64)> {
    String::from_utf8(file(files, RETENTION_FILE))
        .expect("utf-8 index")
        .lines()
        .map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).expect("stamp parses");
            let digest = value["digest"]
                .as_array()
                .expect("digest array")
                .iter()
                .map(|byte| u8::try_from(byte.as_u64().expect("byte")).expect("u8"))
                .collect();
            (digest, value["expires_at_ms"].as_u64().expect("deadline"))
        })
        .collect()
}

fn deadline(store: &EpisodeStore, digest: &[u8; 32]) -> Option<u64> {
    store
        .retention_state(GUILD, digest)
        .expect("retention state")
        .and_then(|state| state.expires_at)
}

/// A finished row: the store directory and the digests it erased.
struct Finished {
    scratch: Scratch,
    erased: Vec<[u8; 32]>,
}

/// The pre-state every row starts from: a live durable candidate and an
/// operational candidate that was forgotten (erased), whose deadline is
/// returned.
fn base(root: &Path) -> ([u8; 32], u64) {
    let mut store = open(root);
    append(&mut store, memory("live", candidate(1)));
    let erased = append(&mut store, memory("erased", operational(2)));
    let erased_deadline = deadline(&store, &erased).expect("a TTL-bound candidate");
    append(&mut store, memory("erase", forget(erased)));
    (erased, erased_deadline)
}

/// The retention crash both retention rows share.
///
/// The retention stamp of a TTL-bound write is synced, and the ledger line
/// it describes never lands. Returns the reopened store after the retry,
/// the retried digest, the orphan's and the retry's deadlines, and the
/// erased digest.
struct RetentionCrash {
    finished: Finished,
    store: EpisodeStore,
    digest: [u8; 32],
    orphan_ms: u64,
    retry_ms: u64,
}

fn retention_crash(row: &str) -> RetentionCrash {
    let scratch = Scratch::new();
    let root = scratch.path();
    let (erased, _) = base(root);
    let before = store_bytes(root);
    let event = memory("ttl_target", operational(3));
    let (due_before, digest) = {
        let mut store = open(root);
        let due = store.due_expirations(GUILD, u64::MAX).expect("due");
        (due, append(&mut store, event.clone()))
    };
    let after = store_bytes(root);
    let crash = with(&before, RETENTION_FILE, file(&after, RETENTION_FILE));
    assert_ne!(crash, after, "{row}: the crash state lacks the ledger line");
    write_state(root, &crash);

    let (mut store, reopened) = reopen_twice(row, root, &crash);
    assert_eq!(reopened, crash, "{row}: the reopen repaired nothing");
    assert_eq!(
        store.retention_state(GUILD, &digest).expect("retention"),
        None,
        "{row}: an orphan stamp names no admitted candidate"
    );
    assert_eq!(
        store.due_expirations(GUILD, u64::MAX).expect("due"),
        due_before,
        "{row}: an orphan stamp makes nothing due"
    );
    let orphan = stamps(&crash).last().cloned().expect("the orphan stamp");
    assert_eq!(
        orphan.0,
        digest.to_vec(),
        "{row}: the orphan names the write"
    );

    // The arrival clock is a millisecond clock: wait so the retry's arrival
    // is strictly later and first-wins and last-wins disagree.
    std::thread::sleep(Duration::from_millis(5));
    assert_eq!(
        append(&mut store, event),
        digest,
        "{row}: the retry is the same write"
    );
    let in_memory = deadline(&store, &digest).expect("the retry binds a deadline");
    drop(store);
    let lines = stamps(&store_bytes(root));
    let named: Vec<u64> = lines
        .iter()
        .filter(|(stamp, _)| *stamp == digest.to_vec())
        .map(|(_, ms)| *ms)
        .collect();
    assert_eq!(named.len(), 2, "{row}: two stamp lines name the digest");
    assert_eq!(named[0], orphan.1, "{row}: the orphan line is kept");
    let retry_ms = named[1];
    assert_eq!(in_memory, retry_ms, "{row}: in memory the retry wins");

    let store = open(root);
    RetentionCrash {
        finished: Finished {
            scratch,
            erased: vec![erased],
        },
        store,
        digest,
        orphan_ms: orphan.1,
        retry_ms,
    }
}

fn retention_stamp_synced_ledger_line_missing() -> Finished {
    let row = "retention_stamp_synced_ledger_line_missing";
    let crash = retention_crash(row);
    assert_eq!(
        deadline(&crash.store, &crash.digest),
        Some(crash.retry_ms),
        "{row}: after reopen the last stamp wins"
    );
    crash.finished
}

fn retry_after_crash_between_retention_and_ledger_extends_deadline() -> Finished {
    let row = "retry_after_crash_between_retention_and_ledger_extends_deadline";
    let crash = retention_crash(row);
    assert!(
        crash.retry_ms > crash.orphan_ms,
        "{row}: the retry's later arrival extends the deadline"
    );
    let due = crash
        .store
        .due_expirations(GUILD, crash.orphan_ms)
        .expect("due");
    assert!(
        !due.contains(&crash.digest),
        "{row}: the candidate is not due at the orphan's deadline"
    );
    let due = crash
        .store
        .due_expirations(GUILD, crash.retry_ms)
        .expect("due");
    assert!(
        due.contains(&crash.digest),
        "{row}: the candidate is due at the retry's deadline"
    );
    crash.finished
}

fn ledger_tail_partial() -> Finished {
    let row = "ledger_tail_partial";
    let scratch = Scratch::new();
    let root = scratch.path();
    let (erased, _) = base(root);
    let before = store_bytes(root);
    let mut torn = file(&before, LEDGER_FILE);
    torn.extend_from_slice(b"{\"partial\":");
    let crash = with(&before, LEDGER_FILE, torn);
    write_state(root, &crash);

    let (store, reopened) = reopen_twice(row, root, &crash);
    assert_eq!(
        reopened, before,
        "{row}: the first reopen truncates exactly the torn tail"
    );
    assert_eq!(store.retrieve(GUILD, 100).expect("receipts").len(), 3);
    Finished {
        scratch,
        erased: vec![erased],
    }
}

/// The pre-crash snapshot of a collection, and an uninterrupted run of the
/// collection from that snapshot.
struct Collection {
    snapshot: Files,
    uninterrupted: Files,
    erased: [u8; 32],
    now_ms: u64,
}

fn collection() -> Collection {
    let source = Scratch::new();
    let (erased, now_ms) = base(source.path());
    let snapshot = store_bytes(source.path());

    let clean = Scratch::new();
    write_state(clean.path(), &snapshot);
    let mut store = open(clean.path());
    let plan = store.gc_plan(GUILD, now_ms).expect("plan");
    assert_eq!(plan.reclaimable, vec![erased]);
    let report = store.apply_gc_plan(&plan).expect("the collection applies");
    assert_eq!(report.reclaimed, vec![erased]);
    drop(store);
    Collection {
        snapshot,
        uninterrupted: store_bytes(clean.path()),
        erased,
        now_ms,
    }
}

/// Retry the collection and compare it with the uninterrupted run.
fn retry_collection(row: &str, root: &Path, run: &Collection, mut store: EpisodeStore) {
    assert!(
        store
            .find_receipt(GUILD, &run.erased)
            .expect("receipt")
            .is_some(),
        "{row}: the interrupted collection reclaimed nothing yet"
    );
    let plan = store.gc_plan(GUILD, run.now_ms).expect("re-plan");
    assert_eq!(plan.reclaimable, vec![run.erased], "{row}: re-plan");
    let report = store.apply_gc_plan(&plan).expect("the retry applies");
    assert_eq!(
        report.reclaimed,
        vec![run.erased],
        "{row}: the retry reclaims"
    );
    drop(store);
    let finished = store_bytes(root);
    assert_eq!(
        file(&finished, LEDGER_FILE),
        file(&run.uninterrupted, LEDGER_FILE),
        "{row}: the retried ledger equals the uninterrupted run's"
    );
    let reclaimed = file(&finished, RECLAIMED_FILE);
    assert_eq!(
        reclaimed,
        file(&run.uninterrupted, RECLAIMED_FILE),
        "{row}: the reclaimed index equals the uninterrupted run's"
    );
    assert_eq!(
        reclaimed.split(|byte| *byte == b'\n').count() - 1,
        1,
        "{row}: the reclaimed index holds a single line"
    );
    assert!(
        !finished.contains_key(Path::new(GC_TEMP_FILE)),
        "{row}: no temporary ledger is left"
    );
}

fn gc_reclaimed_committed_rename_blocked_then_reopen_then_retry() -> Finished {
    let row = "gc_reclaimed_committed_rename_blocked_then_reopen_then_retry";
    let run = collection();
    let scratch = Scratch::new();
    let root = scratch.path();
    write_state(root, &run.snapshot);
    {
        let mut store = open(root);
        // The replacement fails: the temporary ledger cannot be created.
        fs::create_dir(root.join(GC_TEMP_FILE)).expect("block the replacement");
        let plan = store.gc_plan(GUILD, run.now_ms).expect("plan");
        assert!(matches!(
            store.apply_gc_plan(&plan).expect_err("the rewrite fails"),
            EpisodeStoreError::Io
        ));
    }
    fs::remove_dir(root.join(GC_TEMP_FILE)).expect("unblock the replacement");
    let crash = store_bytes(root);
    assert_eq!(
        file(&crash, LEDGER_FILE),
        file(&run.snapshot, LEDGER_FILE),
        "{row}: the ledger is the pre-crash ledger"
    );
    assert_eq!(
        file(&crash, RECLAIMED_FILE),
        file(&run.uninterrupted, RECLAIMED_FILE),
        "{row}: the reclaimed index was committed before the rewrite"
    );

    let (store, reopened) = reopen_twice(row, root, &crash);
    assert_eq!(reopened, crash, "{row}: the reopen repaired nothing");
    retry_collection(row, root, &run, store);
    Finished {
        scratch,
        erased: vec![run.erased],
    }
}

fn gc_temp_left_after_full_write() -> Finished {
    let row = "gc_temp_left_after_full_write";
    let run = collection();
    let scratch = Scratch::new();
    let root = scratch.path();
    let crash = with(
        &with(
            &run.snapshot,
            RECLAIMED_FILE,
            file(&run.uninterrupted, RECLAIMED_FILE),
        ),
        GC_TEMP_FILE,
        file(&run.uninterrupted, LEDGER_FILE),
    );
    write_state(root, &crash);

    let (store, reopened) = reopen_twice(row, root, &crash);
    assert_eq!(reopened, crash, "{row}: the reopen ignores the temporary");
    retry_collection(row, root, &run, store);
    Finished {
        scratch,
        erased: vec![run.erased],
    }
}

fn sidecar_tmp_left_behind() -> Finished {
    let row = "sidecar_tmp_left_behind";
    let scratch = Scratch::new();
    let root = scratch.path();
    let (erased, _) = base(root);
    let key = ObjectKeyRef {
        tenant: "tenant_a".into(),
        key_id: "key_a".into(),
    };
    let destruction = ObjectKeyDestructionRequest {
        guild_ref: GUILD.into(),
        target_digest: erased,
        key: key.clone(),
        actor: owner(),
    };
    let adjustment = QuotaAdjustmentRequest {
        guild_ref: GUILD.into(),
        token_budget: 900,
        storage_budget_bytes: 512 * 1024,
        authority: owner(),
    };
    let before = store_bytes(root);
    let quota_before = open(root).quota_state(GUILD).expect("quota state");
    {
        let mut store = open(root);
        store
            .commit_object_key_destruction(&destruction)
            .expect("the owner binds a key to a forgotten target");
        store
            .commit_quota_adjustment(&adjustment)
            .expect("the owner lowers the budget");
    }
    let after = store_bytes(root);
    // The crash lands after each temporary was fully written and before it
    // was renamed over its sidecar.
    let crash = with(
        &with(&before, OBJECT_KEY_TEMP, file(&after, OBJECT_KEY_FILE)),
        QUOTA_TEMP,
        file(&after, QUOTA_FILE),
    );
    write_state(root, &crash);

    let (mut store, reopened) = reopen_twice(row, root, &crash);
    assert_eq!(reopened, crash, "{row}: the reopen ignores the temporaries");
    assert_eq!(
        store
            .object_key_destruction_state(GUILD, &key)
            .expect("object key state"),
        None,
        "{row}: the uncommitted key binding is not answered"
    );
    assert_eq!(
        store.quota_state(GUILD).expect("quota state"),
        quota_before,
        "{row}: the uncommitted adjustment is not in force"
    );

    store
        .commit_object_key_destruction(&destruction)
        .expect("the retried binding commits");
    store
        .commit_quota_adjustment(&adjustment)
        .expect("the retried adjustment commits");
    drop(store);
    let finished = store_bytes(root);
    assert!(!finished.contains_key(Path::new(OBJECT_KEY_TEMP)));
    assert!(!finished.contains_key(Path::new(QUOTA_TEMP)));
    assert_eq!(
        file(&finished, OBJECT_KEY_FILE),
        file(&after, OBJECT_KEY_FILE),
        "{row}: the retried binding equals the uninterrupted one"
    );
    let quota = open(root)
        .quota_state(GUILD)
        .expect("quota")
        .expect("served");
    assert_eq!(
        (
            quota.token_budget,
            quota.storage_budget_bytes,
            quota.adjusted_by
        ),
        (900, 512 * 1024, Some(owner())),
        "{row}: the retried adjustment is in force"
    );
    Finished {
        scratch,
        erased: vec![erased],
    }
}

/// Refuse a supersession and a forget of every erased digest.
fn assert_not_resurrected(row: &str, store: &mut EpisodeStore, erased: &[[u8; 32]], tag: &str) {
    for (index, digest) in erased.iter().enumerate() {
        let error = rejected(
            store,
            &memory(&format!("sup_{tag}_{index}"), supersede(40, *digest)),
        );
        assert!(
            matches!(error, EpisodeStoreError::InvalidTransition),
            "{row}: a supersession of an erased digest is refused"
        );
        let error = rejected(
            store,
            &memory(&format!("fgt_{tag}_{index}"), forget(*digest)),
        );
        assert!(
            matches!(error, EpisodeStoreError::InvalidTransition),
            "{row}: a forget of an erased digest is refused"
        );
    }
}

/// One named row of the matrix and the function that runs it.
type Row = (&'static str, fn() -> Finished);

const ROWS: [Row; 6] = [
    (
        "retention_stamp_synced_ledger_line_missing",
        retention_stamp_synced_ledger_line_missing,
    ),
    (
        "retry_after_crash_between_retention_and_ledger_extends_deadline",
        retry_after_crash_between_retention_and_ledger_extends_deadline,
    ),
    ("ledger_tail_partial", ledger_tail_partial),
    (
        "gc_reclaimed_committed_rename_blocked_then_reopen_then_retry",
        gc_reclaimed_committed_rename_blocked_then_reopen_then_retry,
    ),
    (
        "gc_temp_left_after_full_write",
        gc_temp_left_after_full_write,
    ),
    ("sidecar_tmp_left_behind", sidecar_tmp_left_behind),
];

#[test]
fn row_retention_stamp_synced_ledger_line_missing() {
    retention_stamp_synced_ledger_line_missing();
}

#[test]
fn row_retry_after_crash_between_retention_and_ledger_extends_deadline() {
    retry_after_crash_between_retention_and_ledger_extends_deadline();
}

#[test]
fn row_ledger_tail_partial() {
    ledger_tail_partial();
}

#[test]
fn row_gc_reclaimed_committed_rename_blocked_then_reopen_then_retry() {
    gc_reclaimed_committed_rename_blocked_then_reopen_then_retry();
}

#[test]
fn row_gc_temp_left_after_full_write() {
    gc_temp_left_after_full_write();
}

#[test]
fn row_sidecar_tmp_left_behind() {
    sidecar_tmp_left_behind();
}

#[test]
fn tombstone_never_resurrected_after_each_row() {
    for (row, run) in ROWS {
        let finished = run();
        let root = finished.scratch.path();
        let mut store = open(root);
        assert_not_resurrected(row, &mut store, &finished.erased, "a");
        drop(store);
        let mut reopened = open(root);
        assert_not_resurrected(row, &mut reopened, &finished.erased, "b");
    }
}
