//! Fixtures for the rejected-write and governance-refusal invariance tests.
//!
//! The harness builds one store that already holds every kind of state a
//! refusal could disturb: live, held, forgotten, revoked, and TTL-bound
//! candidates, tombstones, a live and a released hold, a redaction request,
//! an object-key destruction request, and a quota adjustment. A
//! [`Snapshot`] captures every byte under the store directory together with
//! every content-free read the store answers, so a test can prove that a
//! refused call changed nothing at all.
//!
//! The byte walk follows the `store_bytes` pattern of
//! `support/quota_windows.rs`.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, DerivedCopyKind, EpisodeEvent, EpisodeReceipt, EpisodeSource,
    EpisodeStore, EpisodeStoreError, EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, HoldKind,
    HoldReleaseRequest, HoldRequest, HoldState, MemoryCandidate, MemoryClass,
    ObjectKeyDestructionRequest, ObjectKeyDestructionState, ObjectKeyRef, QuotaAdjustmentRecord,
    QuotaAdjustmentRequest, QuotaState, RedactionRequest, RedactionState, RetentionClass,
    RetentionState, RetentionWindows, RevocationRequest, RevocationState, StorePolicy,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// The served guild every row targets.
pub const GUILD: &str = "guild_ref";
/// A second served guild, which no row may disturb either.
pub const OTHER: &str = "other_guild";
/// A served guild whose learning is not opted in.
pub const DISABLED: &str = "disabled_guild";
/// A well-formed guild reference the store does not serve.
pub const UNSERVED: &str = "unserved_guild";
/// The operational retention window: long enough that no deadline passes
/// while a test runs.
pub const TTL_MS: u64 = 3_600_000;
/// The host policy's token budget.
pub const TOKEN_BUDGET: u64 = 1_000;
/// The host policy's storage budget.
pub const STORAGE_BUDGET: u64 = 64 * 1024;
/// The token budget the fixture's quota adjustment lowers the guild to.
pub const ADJUSTED_TOKENS: u64 = 900;
/// The storage budget the fixture's quota adjustment lowers the guild to.
pub const ADJUSTED_BYTES: u64 = 60 * 1024;

/// A disposable store directory that removes itself.
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-rejection-invariance-{}", Uuid::new_v4())))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn actor(principal_id: &str, kind: ActorKind) -> ActorRef {
    ActorRef {
        principal_id: principal_id.into(),
        kind,
    }
}

/// The guild owner: the authority every accepted governance call uses.
pub fn owner() -> ActorRef {
    actor("guild_owner", ActorKind::GuildOwner)
}

/// A guild administrator: below the owner in the governance order.
pub fn admin() -> ActorRef {
    actor("guild_admin", ActorKind::GuildAdministrator)
}

/// The service that writes memory candidates. The guild names it, so a
/// refusal of it is the class rule's, not the principal map's.
pub fn service() -> ActorRef {
    actor("abbey_service", ActorKind::Service)
}

/// A human subject the guild names. The class rule refuses it.
pub fn subject() -> ActorRef {
    actor("subject_user", ActorKind::HumanSubject)
}

/// An owner-class principal the guild's authority map does not name.
pub fn stranger() -> ActorRef {
    actor("stranger", ActorKind::GuildOwner)
}

fn guild_policy(learning_enabled: bool) -> GuildEpisodePolicy {
    GuildEpisodePolicy {
        learning_enabled,
        policy_version: "policy_v1".into(),
        token_budget: TOKEN_BUDGET,
        storage_budget_bytes: STORAGE_BUDGET,
        current_consent_epoch: Some(7),
        retention_ttl_ms: None,
        // A zero session window makes the zero-TTL row reachable; the
        // operational window is long, so a TTL-bound candidate is live.
        retention_windows: Some(RetentionWindows {
            session_ttl_ms: Some(0),
            operational_ttl_ms: Some(TTL_MS),
            durable_ttl_ms: None,
        }),
        authorized_principals: Some(vec![
            "guild_owner".into(),
            "guild_admin".into(),
            "abbey_service".into(),
            "subject_user".into(),
        ]),
    }
}

pub fn policy() -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([
            (GUILD.into(), guild_policy(true)),
            (OTHER.into(), guild_policy(true)),
            (DISABLED.into(), guild_policy(false)),
        ]),
    }
}

pub fn write(request_id: &str, operation_id: &str, event: EpisodeEvent) -> EpisodeWrite {
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

/// A durable candidate: it binds no retention deadline and reads no clock.
pub fn candidate(seed: u8) -> MemoryCandidate {
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

/// A TTL-bound candidate: admitting it would add a retention line.
pub fn operational(seed: u8) -> MemoryCandidate {
    MemoryCandidate {
        retention: RetentionClass::Operational,
        ..candidate(seed)
    }
}

/// A session candidate, which the zero session window refuses.
pub fn session(seed: u8) -> MemoryCandidate {
    MemoryCandidate {
        retention: RetentionClass::Session,
        ..candidate(seed)
    }
}

pub fn supersede(seed: u8, target: [u8; 32]) -> MemoryCandidate {
    MemoryCandidate {
        supersedes: Some(target),
        ..candidate(seed)
    }
}

pub fn forget(target: [u8; 32]) -> MemoryCandidate {
    MemoryCandidate {
        payload_commitment: [0; 32],
        payload_bytes: 0,
        forgets: Some(target),
        ..candidate(0)
    }
}

pub fn memory(request_id: &str, candidate: MemoryCandidate) -> EpisodeWrite {
    write(
        request_id,
        &format!("op_{request_id}"),
        EpisodeEvent::MemoryCandidate {
            recorded_by: service(),
            candidate,
        },
    )
}

/// Append one write the gate accepts, predicting its commitment first.
pub fn append(store: &mut EpisodeStore, mut event: EpisodeWrite) -> [u8; 32] {
    let preview = store
        .preview_commitment(&event)
        .expect("valid event previews");
    event.expected_commitment = Some(preview);
    let receipt = store.propose_write(&event).expect("valid event appends");
    assert_eq!(receipt.episode_digest, preview);
    preview
}

/// Every file under the store directory with its exact bytes.
pub fn store_bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).expect("read store dir") {
            let path = entry.expect("store dir entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = fs::read(&path).expect("read store file");
                let relative = path.strip_prefix(root).expect("under root").to_path_buf();
                files.insert(relative, bytes);
            }
        }
    }
    files
}

/// The object key the fixture's destruction request binds.
pub fn key(key_id: &str) -> ObjectKeyRef {
    ObjectKeyRef {
        tenant: "tenant_a".into(),
        key_id: key_id.into(),
    }
}

pub fn hold(target: [u8; 32], kind: HoldKind, authority: ActorRef) -> HoldRequest {
    HoldRequest {
        guild_ref: GUILD.into(),
        target_digest: target,
        kind,
        authority,
    }
}

pub fn release(target: [u8; 32], kind: HoldKind, authority: ActorRef) -> HoldReleaseRequest {
    HoldReleaseRequest {
        guild_ref: GUILD.into(),
        target_digest: target,
        kind,
        authority,
    }
}

pub fn revocation(target: [u8; 32], authority: ActorRef) -> RevocationRequest {
    RevocationRequest {
        guild_ref: GUILD.into(),
        target_digest: target,
        authority,
    }
}

pub fn redaction(target: [u8; 32], actor: ActorRef) -> RedactionRequest {
    RedactionRequest {
        guild_ref: GUILD.into(),
        target_digest: target,
        kinds: vec![DerivedCopyKind::Embedding],
        actor,
    }
}

pub fn destruction(target: [u8; 32], key_id: &str, actor: ActorRef) -> ObjectKeyDestructionRequest {
    ObjectKeyDestructionRequest {
        guild_ref: GUILD.into(),
        target_digest: target,
        key: key(key_id),
        actor,
    }
}

pub fn adjustment(tokens: u64, bytes: u64, authority: ActorRef) -> QuotaAdjustmentRequest {
    QuotaAdjustmentRequest {
        guild_ref: GUILD.into(),
        token_budget: tokens,
        storage_budget_bytes: bytes,
        authority,
    }
}

/// A store that already holds every kind of state a refusal could disturb.
pub struct Fixture {
    pub scratch: Scratch,
    pub store: EpisodeStore,
    /// A live durable candidate under a live legal hold placed by the owner.
    pub held: [u8; 32],
    /// A durable candidate forgotten by `tombstone`, with a redaction
    /// request and the object key `key_b` bound to it.
    pub forgotten: [u8; 32],
    /// The tombstone that forgot `forgotten`.
    pub tombstone: [u8; 32],
    /// A live TTL-bound candidate whose user-request hold was released.
    pub ttl_live: [u8; 32],
    /// A candidate the owner revoked.
    pub revoked: [u8; 32],
    /// A second forgotten candidate, with no object key bound.
    pub second_forgotten: [u8; 32],
    /// The tombstone that forgot `second_forgotten`.
    pub second_tombstone: [u8; 32],
}

pub fn fixture() -> Fixture {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
    let held = append(&mut store, memory("held", candidate(1)));
    let forgotten = append(&mut store, memory("forgotten", candidate(2)));
    let tombstone = append(&mut store, memory("tombstone", forget(forgotten)));
    let ttl_live = append(&mut store, memory("ttl_live", operational(3)));
    let revoked = append(&mut store, memory("revoked", candidate(4)));
    let second_forgotten = append(&mut store, memory("second", candidate(5)));
    let second_tombstone = append(&mut store, memory("second_tomb", forget(second_forgotten)));
    store
        .commit_hold(&hold(held, HoldKind::Legal, owner()))
        .expect("the owner holds a live candidate");
    store
        .commit_hold(&hold(ttl_live, HoldKind::UserRequest, owner()))
        .expect("the owner holds a TTL-bound candidate");
    store
        .release_hold(&release(ttl_live, HoldKind::UserRequest, owner()))
        .expect("the owner releases its own hold");
    store
        .commit_revocation(&revocation(revoked, owner()))
        .expect("the owner revokes a live candidate");
    store
        .commit_redaction(&redaction(forgotten, owner()))
        .expect("the owner requests a redaction of a forgotten target");
    store
        .commit_object_key_destruction(&destruction(forgotten, "key_b", owner()))
        .expect("the owner binds a key to a forgotten target");
    store
        .commit_quota_adjustment(&adjustment(ADJUSTED_TOKENS, ADJUSTED_BYTES, owner()))
        .expect("the owner lowers the budget");
    Fixture {
        scratch,
        store,
        held,
        forgotten,
        tombstone,
        ttl_live,
        revoked,
        second_forgotten,
        second_tombstone,
    }
}

/// Every byte under the store directory and every content-free read the
/// store answers about the fixture's state.
#[derive(Debug, PartialEq)]
pub struct Snapshot {
    bytes: BTreeMap<PathBuf, Vec<u8>>,
    receipts: Vec<Vec<EpisodeReceipt>>,
    usage: Vec<Option<(u64, u64)>>,
    quota: Vec<Option<QuotaState>>,
    quota_history: Vec<QuotaAdjustmentRecord>,
    holds: Vec<Option<HoldState>>,
    revocations: Vec<Option<RevocationState>>,
    redactions: Vec<Option<RedactionState>>,
    retention: Vec<Option<RetentionState>>,
    object_keys: Vec<Option<ObjectKeyDestructionState>>,
}

impl Fixture {
    fn digests(&self) -> [[u8; 32]; 7] {
        [
            self.held,
            self.forgotten,
            self.tombstone,
            self.ttl_live,
            self.revoked,
            self.second_forgotten,
            self.second_tombstone,
        ]
    }

    pub fn snapshot(&self) -> Snapshot {
        let store = &self.store;
        let guilds = [GUILD, OTHER, DISABLED];
        let digests = self.digests();
        Snapshot {
            bytes: store_bytes(self.scratch.path()),
            receipts: guilds
                .iter()
                .map(|guild| store.retrieve(guild, 2_048).expect("retrieve"))
                .collect(),
            usage: guilds
                .iter()
                .map(|guild| store.guild_usage(guild))
                .collect(),
            quota: guilds
                .iter()
                .map(|guild| store.quota_state(guild).expect("quota state"))
                .collect(),
            quota_history: store.quota_history(GUILD).expect("quota history"),
            holds: digests
                .iter()
                .map(|digest| store.hold_state(GUILD, digest).expect("hold state"))
                .collect(),
            revocations: digests
                .iter()
                .map(|digest| store.revocation_state(GUILD, digest).expect("revocation"))
                .collect(),
            redactions: digests
                .iter()
                .map(|digest| store.redaction_state(GUILD, digest).expect("redaction"))
                .collect(),
            retention: digests
                .iter()
                .map(|digest| store.retention_state(GUILD, digest).expect("retention"))
                .collect(),
            object_keys: ["key_b", "key_e"]
                .iter()
                .map(|key_id| {
                    store
                        .object_key_destruction_state(GUILD, &key(key_id))
                        .expect("object key state")
                })
                .collect(),
        }
    }

    /// Assert one write is refused by both the decision and the append with
    /// the same reason, and that the refusal changed nothing.
    pub fn assert_write_refused(&mut self, row: &str, event: &EpisodeWrite, expected: &str) {
        let before = self.snapshot();
        let decided = self
            .store
            .decide_write(event)
            .err()
            .unwrap_or_else(|| panic!("{row}: the decision refuses the write"));
        let refused = self
            .store
            .propose_write(event)
            .err()
            .unwrap_or_else(|| panic!("{row}: the append refuses the write"));
        assert_eq!(refused.to_string(), expected, "{row}: append reason");
        assert_eq!(decided.to_string(), expected, "{row}: decision reason");
        assert_eq!(
            self.snapshot(),
            before,
            "{row}: a refused write changed state"
        );
    }

    /// Assert one governance call is refused with `expected` and that the
    /// refusal changed nothing.
    pub fn assert_refused<T>(
        &mut self,
        row: &str,
        expected: &str,
        call: impl FnOnce(&mut EpisodeStore) -> Result<T, EpisodeStoreError>,
    ) {
        let before = self.snapshot();
        let Err(error) = call(&mut self.store) else {
            panic!("{row}: the call is refused");
        };
        assert_eq!(error.to_string(), expected, "{row}: refusal reason");
        assert_eq!(self.snapshot(), before, "{row}: a refusal changed state");
    }
}
