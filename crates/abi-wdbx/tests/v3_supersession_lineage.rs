//! Supersession lineage reads for admitted memory candidates: the
//! `supersedes` edge set, its replay parity after reopen, and its guild
//! isolation. Content-free throughout: only digests and flags cross the
//! boundary, never a payload.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeSource, EpisodeStore, EpisodeStoreError,
    EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MemoryCandidate, MemoryClass, RetentionClass,
    StorePolicy, SupersessionLineage,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const GUILD: &str = "guild_ref";
const OTHER_GUILD: &str = "other_guild";

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-supersession-{}", Uuid::new_v4())))
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
        retention_ttl_ms: None,
        retention_windows: None,
        authorized_principals: None,
    }
}

fn policy(bytes: u64) -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([
            (GUILD.into(), guild_policy(bytes)),
            (OTHER_GUILD.into(), guild_policy(bytes)),
        ]),
    }
}

fn write(request_id: &str, operation_id: &str, event: EpisodeEvent) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        contract_revision: 2,
        contract_digest: [9; 32],
        guild_ref: GUILD.into(),
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

fn superseding(seed: u8, target: [u8; 32]) -> MemoryCandidate {
    MemoryCandidate {
        supersedes: Some(target),
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
fn a_superseding_candidate_names_its_target_and_its_target_names_it() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1024 * 1024)).expect("open");
    let first = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let second = append(
        &mut store,
        memory("request_2", "memory_2", superseding(2, first)),
    );

    assert_eq!(
        store.superseded_by(GUILD, &first).expect("lookup"),
        Some(second),
        "the superseded candidate names its replacement"
    );
    assert_eq!(
        store.superseded_by(GUILD, &second).expect("lookup"),
        None,
        "the replacement is not itself superseded"
    );

    let target = store.supersession_lineage(GUILD, &first).expect("lineage");
    assert_eq!(target.superseded_by, Some(second));
    assert_eq!(target.supersedes, None);
    assert!(!target.superseded, "the target supersedes nothing");
    assert!(!target.forgotten);
    assert!(!target.tombstoned);

    let replacement = store.supersession_lineage(GUILD, &second).expect("lineage");
    assert_eq!(replacement.supersedes, Some(first));
    assert_eq!(replacement.superseded_by, None);
    assert!(
        replacement.superseded,
        "the replacement supersedes the target it corrects"
    );
    assert!(!replacement.forgotten);
    assert!(!replacement.tombstoned);
}

#[test]
fn the_lineage_survives_close_and_reopen() {
    let scratch = Scratch::new();
    let store_policy = policy(1024 * 1024);
    let (first, second, before_target, before_replacement);
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        first = append(&mut store, memory("request_1", "memory_1", candidate(1)));
        second = append(
            &mut store,
            memory("request_2", "memory_2", superseding(2, first)),
        );
        before_target = store.supersession_lineage(GUILD, &first).expect("lineage");
        before_replacement = store.supersession_lineage(GUILD, &second).expect("lineage");
        assert_eq!(
            store.superseded_by(GUILD, &first).expect("lookup"),
            Some(second)
        );
    }

    // The ledger is the only persisted state: no index file holds the lineage,
    // so a reopened store can only answer by replaying the ledger.
    let mut entries: Vec<String> = fs::read_dir(scratch.path())
        .expect("store directory")
        .map(|entry| {
            entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec![
            "episodes.v1.jsonl".to_string(),
            "episodes.v1.lock".to_string()
        ]
    );

    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    assert_eq!(
        reopened.superseded_by(GUILD, &first).expect("lookup"),
        Some(second),
        "replay rebuilds the supersession edge for {}",
        hex(&first)
    );
    assert_eq!(
        reopened
            .supersession_lineage(GUILD, &first)
            .expect("lineage"),
        before_target
    );
    assert_eq!(
        reopened
            .supersession_lineage(GUILD, &second)
            .expect("lineage"),
        before_replacement
    );
}

#[test]
fn an_unknown_digest_has_no_lineage() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1024 * 1024)).expect("open");
    let known = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let unknown = [0xab; 32];
    assert_ne!(unknown, known);

    assert_eq!(
        store.superseded_by(GUILD, &unknown).expect("lookup"),
        None,
        "an unknown digest has no superseder"
    );
    assert_eq!(
        store
            .supersession_lineage(GUILD, &unknown)
            .expect("lineage"),
        SupersessionLineage::default()
    );

    // A bounded guild reference is still required.
    assert!(matches!(
        store.superseded_by("", &known),
        Err(EpisodeStoreError::InvalidInput)
    ));
    assert!(matches!(
        store.supersession_lineage("Bad Guild", &known),
        Err(EpisodeStoreError::InvalidInput)
    ));
}

#[test]
fn a_forgotten_candidate_still_reports_its_superseder() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1024 * 1024)).expect("open");
    let first = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let second = append(
        &mut store,
        memory("request_2", "memory_2", superseding(2, first)),
    );
    let tombstone = append(&mut store, memory("request_3", "memory_3", forget(first)));

    // Forgetting and superseding are independent edges: one never erases the
    // other.
    assert_eq!(
        store.superseded_by(GUILD, &first).expect("lookup"),
        Some(second),
        "a forgotten candidate still reports its superseder"
    );
    let forgotten = store.supersession_lineage(GUILD, &first).expect("lineage");
    assert!(forgotten.forgotten);
    assert_eq!(forgotten.superseded_by, Some(second));
    assert_eq!(forgotten.supersedes, None);
    assert!(!forgotten.tombstoned);

    // The tombstone is a candidate of its own: it forgets, it supersedes
    // nothing, and nothing forgets it.
    let erased = store
        .supersession_lineage(GUILD, &tombstone)
        .expect("lineage");
    assert!(erased.tombstoned);
    assert!(!erased.forgotten);
    assert_eq!(erased.superseded_by, None);
    assert_eq!(erased.supersedes, None);
}

#[test]
fn chained_corrections_keep_every_edge() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1024 * 1024)).expect("open");
    let first = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let second = append(
        &mut store,
        memory("request_2", "memory_2", superseding(2, first)),
    );
    let third = append(
        &mut store,
        memory("request_3", "memory_3", superseding(3, second)),
    );

    assert_eq!(
        store.superseded_by(GUILD, &first).expect("lookup"),
        Some(second),
        "the first correction still names its target"
    );
    assert_eq!(
        store.superseded_by(GUILD, &second).expect("lookup"),
        Some(third),
        "the second correction names the third"
    );
    assert_eq!(store.superseded_by(GUILD, &third).expect("lookup"), None);

    let middle = store.supersession_lineage(GUILD, &second).expect("lineage");
    assert_eq!(middle.supersedes, Some(first));
    assert_eq!(middle.superseded_by, Some(third));
    assert!(middle.superseded);
    assert!(!middle.forgotten);
}

#[test]
fn the_most_recent_correction_of_a_target_is_reported() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1024 * 1024)).expect("open");
    let first = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let second = append(
        &mut store,
        memory("request_2", "memory_2", superseding(2, first)),
    );
    let third = append(
        &mut store,
        memory("request_3", "memory_3", superseding(3, first)),
    );

    // Two corrections of one memory are two edges, not a rewrite. The forward
    // direction is a projection and reports the latest correction; the reverse
    // direction keeps both.
    assert_eq!(
        store.superseded_by(GUILD, &first).expect("lookup"),
        Some(third)
    );
    assert_eq!(
        store
            .supersession_lineage(GUILD, &second)
            .expect("lineage")
            .supersedes,
        Some(first)
    );
    assert_eq!(
        store
            .supersession_lineage(GUILD, &third)
            .expect("lineage")
            .supersedes,
        Some(first)
    );
}

#[test]
fn a_digest_is_invisible_in_another_guild() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1024 * 1024)).expect("open");
    let first = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let second = append(
        &mut store,
        memory("request_2", "memory_2", superseding(2, first)),
    );

    assert_eq!(
        store.superseded_by(OTHER_GUILD, &first).expect("lookup"),
        None,
        "another guild does not see this guild's lineage"
    );
    assert_eq!(
        store
            .supersession_lineage(OTHER_GUILD, &first)
            .expect("lineage"),
        SupersessionLineage::default()
    );

    // The other guild keeps its own lineage, and a candidate of one guild is
    // not a supersession target in the other.
    let mut foreign = memory("request_3", "memory_3", candidate(3));
    foreign.guild_ref = OTHER_GUILD.into();
    let foreign_digest = append(&mut store, foreign);
    let mut foreign_correction = memory("request_4", "memory_4", superseding(4, foreign_digest));
    foreign_correction.guild_ref = OTHER_GUILD.into();
    let foreign_second = append(&mut store, foreign_correction);

    assert_eq!(
        store
            .superseded_by(OTHER_GUILD, &foreign_digest)
            .expect("lookup"),
        Some(foreign_second)
    );
    assert_eq!(
        store.superseded_by(GUILD, &foreign_digest).expect("lookup"),
        None
    );

    let cross = memory("request_5", "memory_5", superseding(5, foreign_digest));
    assert!(matches!(
        rejected(&mut store, &cross),
        EpisodeStoreError::InvalidTransition
    ));
    assert_eq!(
        store.superseded_by(GUILD, &first).expect("lookup"),
        Some(second),
        "the refused write left the lineage untouched"
    );
}
