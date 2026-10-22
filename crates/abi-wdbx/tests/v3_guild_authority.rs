//! Guild-scoped authority for the v3 episode store.
//!
//! The host policy may name the principals that may act as authorities for a
//! guild (`GuildEpisodePolicy::authorized_principals`). The map is
//! guild-scoped: a principal named for one guild is refused for every guild
//! that does not name it, whatever class it carries.
//!
//! What these tests pin:
//!
//! - the compatibility case: a policy that names no principal anywhere keeps
//!   today's behaviour exactly — the class rule alone decides;
//! - a principal named for a guild may act for that guild, and a different
//!   principal of the same class may not;
//! - a principal named for one guild is refused for another guild, even
//!   where that other guild states no map of its own;
//! - naming a principal never widens what its *class* may do: a service
//!   named for a guild still may not place a hold, revoke a candidate, or
//!   raise a budget;
//! - an empty map is the same state as an absent map: it names nobody and
//!   constrains nobody of its own;
//! - every refusal answers the same content-free error, and that error names
//!   neither the principal nor the guild;
//! - the hold, release, revocation, quota-adjustment and quota-clear paths
//!   all honour the map;
//! - a store whose policy names principals for one guild constrains that
//!   guild and leaves the others to the class rule;
//! - the map moves no committed digest: it is host policy, not episode
//!   material.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeSource, EpisodeStore, EpisodeStoreError,
    EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, HoldKind, HoldReleaseRequest, HoldRequest,
    MemoryCandidate, MemoryClass, QuotaAdjustmentRequest, RetentionClass, RevocationRequest,
    StorePolicy,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// The guild whose policy names principals.
const GUILD_A: &str = "guild_a";
/// The guild whose policy names no principal.
const GUILD_B: &str = "guild_b";

/// The principal the map names.
const OWNER_A: &str = "owner_a";
/// A different principal of the same class, named by no map.
const OWNER_B: &str = "owner_b";
/// The service that records the memory candidates.
const SERVICE: &str = "abbey_service";

/// The one refusal every authority failure answers with.
///
/// The refusal is content-free: it names neither the principal nor the
/// guild, so it is the same answer for "not authorized for this guild" and
/// "no such principal".
const REFUSAL: &str = "episode_transition_invalid";

/// A disposable store directory that removes itself.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-guild-authority-{}", Uuid::new_v4())))
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

/// The guild policy under test.
///
/// `authorized` is the guild's own authority map: `None` is an absent map
/// and `Some(vec![])` is an empty one.
fn guild_policy(authorized: Option<Vec<&str>>) -> GuildEpisodePolicy {
    GuildEpisodePolicy {
        learning_enabled: true,
        policy_version: "policy_v1".into(),
        token_budget: 1_000,
        storage_budget_bytes: 1024 * 1024,
        current_consent_epoch: Some(7),
        retention_ttl_ms: None,
        retention_windows: None,
        authorized_principals: authorized
            .map(|principals| principals.into_iter().map(str::to_string).collect()),
    }
}

/// The store policy under test: two guilds, each with its own map.
fn policy(guild_a: Option<Vec<&str>>, guild_b: Option<Vec<&str>>) -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([
            (GUILD_A.into(), guild_policy(guild_a)),
            (GUILD_B.into(), guild_policy(guild_b)),
        ]),
    }
}

fn write(
    guild_ref: &str,
    request_id: &str,
    operation_id: &str,
    event: EpisodeEvent,
) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        contract_revision: 2,
        contract_digest: [9; 32],
        guild_ref: guild_ref.into(),
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

/// One memory-candidate write in one guild.
fn memory(guild_ref: &str, request_id: &str, operation_id: &str, seed: u8) -> EpisodeWrite {
    write(
        guild_ref,
        request_id,
        operation_id,
        EpisodeEvent::MemoryCandidate {
            recorded_by: actor(SERVICE, ActorKind::Service),
            candidate: candidate(seed),
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
    preview
}

fn hold_request(guild_ref: &str, target: [u8; 32], authority: &ActorRef) -> HoldRequest {
    HoldRequest {
        guild_ref: guild_ref.into(),
        target_digest: target,
        kind: HoldKind::Investigation,
        authority: authority.clone(),
    }
}

fn release_request(guild_ref: &str, target: [u8; 32], authority: &ActorRef) -> HoldReleaseRequest {
    HoldReleaseRequest {
        guild_ref: guild_ref.into(),
        target_digest: target,
        kind: HoldKind::Investigation,
        authority: authority.clone(),
    }
}

fn revocation_request(
    guild_ref: &str,
    target: [u8; 32],
    authority: &ActorRef,
) -> RevocationRequest {
    RevocationRequest {
        guild_ref: guild_ref.into(),
        target_digest: target,
        authority: authority.clone(),
    }
}

fn adjustment(
    guild_ref: &str,
    token_budget: u64,
    storage_budget_bytes: u64,
    authority: &ActorRef,
) -> QuotaAdjustmentRequest {
    QuotaAdjustmentRequest {
        guild_ref: guild_ref.into(),
        token_budget,
        storage_budget_bytes,
        authority: authority.clone(),
    }
}

/// Assert that a refusal is the one content-free refusal.
fn assert_refusal(error: &EpisodeStoreError) {
    assert_eq!(error.to_string(), REFUSAL, "the refusal is content-free");
    assert!(matches!(error, EpisodeStoreError::InvalidTransition));
}

/// Assert that a refusal names neither the principal nor the guild.
fn assert_leaks_nothing(error: &EpisodeStoreError, principal_id: &str, guild_ref: &str) {
    let rendered = format!("{error:?} {error}");
    assert!(
        !rendered.contains(principal_id),
        "the refusal names the principal: {rendered}"
    );
    assert!(
        !rendered.contains(guild_ref),
        "the refusal names the guild: {rendered}"
    );
}

/// The compatibility case: a policy that names no principal anywhere keeps
/// today's behaviour exactly. The class rule alone decides, and the map
/// changes nothing.
#[test]
fn absent_map_keeps_the_legacy_class_rule() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(None, None)).expect("store opens");
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    let target = append(&mut store, memory(GUILD_A, "req_1", "op_1", 1));

    store
        .decide_hold(&hold_request(GUILD_A, target, &owner))
        .expect("the class rule admits a guild owner");
    store
        .commit_hold(&hold_request(GUILD_A, target, &owner))
        .expect("the hold commits");
    store
        .release_hold(&release_request(GUILD_A, target, &owner))
        .expect("the placing authority releases its own hold");
    store
        .commit_revocation(&revocation_request(GUILD_A, target, &owner))
        .expect("the revocation commits");
    store
        .commit_quota_adjustment(&adjustment(GUILD_A, 500, 512 * 1024, &owner))
        .expect("the adjustment commits");
    store
        .clear_quota_adjustment(GUILD_A, &owner)
        .expect("the adjustment clears");
    assert!(
        store.actor_may_act(GUILD_A, &owner),
        "an unscoped principal keeps the class rule"
    );
}

/// A principal the guild names may act for that guild, on every path.
#[test]
fn a_named_principal_may_act_for_its_own_guild() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![OWNER_A]), None)).expect("store opens");
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    let hold_target = append(&mut store, memory(GUILD_A, "req_1", "op_1", 1));
    let revoke_target = append(&mut store, memory(GUILD_A, "req_2", "op_2", 2));

    assert!(store.actor_may_act(GUILD_A, &owner));
    store
        .commit_hold(&hold_request(GUILD_A, hold_target, &owner))
        .expect("the named principal places a hold");
    store
        .release_hold(&release_request(GUILD_A, hold_target, &owner))
        .expect("the named principal releases its own hold");
    store
        .commit_revocation(&revocation_request(GUILD_A, revoke_target, &owner))
        .expect("the named principal revokes a candidate");
    store
        .commit_quota_adjustment(&adjustment(GUILD_A, 500, 512 * 1024, &owner))
        .expect("the named principal adjusts the budget");
    store
        .clear_quota_adjustment(GUILD_A, &owner)
        .expect("the named principal clears the adjustment");
}

/// A different principal of the same class is refused for the guild that
/// names somebody else.
#[test]
fn a_different_principal_with_the_same_class_is_refused() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![OWNER_A]), None)).expect("store opens");
    let stranger = actor(OWNER_B, ActorKind::GuildOwner);
    let target = append(&mut store, memory(GUILD_A, "req_1", "op_1", 1));

    assert!(!store.actor_may_act(GUILD_A, &stranger));
    let error = store
        .commit_hold(&hold_request(GUILD_A, target, &stranger))
        .expect_err("the stranger may not hold");
    assert_refusal(&error);
    assert_leaks_nothing(&error, OWNER_B, GUILD_A);
}

/// The principal named for one guild is refused for another guild, even
/// where that other guild states no map of its own: naming a principal
/// scopes it to the guilds that name it.
#[test]
fn a_named_principal_is_refused_for_another_guild() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![OWNER_A]), None)).expect("store opens");
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    let target = append(&mut store, memory(GUILD_B, "req_1", "op_1", 1));

    assert!(!store.actor_may_act(GUILD_B, &owner));
    let error = store
        .commit_hold(&hold_request(GUILD_B, target, &owner))
        .expect_err("the owner of one guild may not hold in another");
    assert_refusal(&error);
    assert_leaks_nothing(&error, OWNER_A, GUILD_B);
    let error = store
        .commit_revocation(&revocation_request(GUILD_B, target, &owner))
        .expect_err("the owner of one guild may not revoke in another");
    assert_refusal(&error);
    let error = store
        .commit_quota_adjustment(&adjustment(GUILD_B, 500, 512 * 1024, &owner))
        .expect_err("the owner of one guild may not adjust another budget");
    assert_refusal(&error);
    let error = store
        .clear_quota_adjustment(GUILD_B, &owner)
        .expect_err("the owner of one guild may not clear another budget");
    assert_refusal(&error);
}

/// Naming a principal never widens what its class may do. A service named
/// for a guild may still only *lower* a budget, and never holds, revokes, or
/// clears.
#[test]
fn a_listed_service_is_still_refused() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![SERVICE]), None)).expect("store opens");
    let service = actor(SERVICE, ActorKind::Service);
    let target = append(&mut store, memory(GUILD_A, "req_1", "op_1", 1));

    // The map admits the principal, so the class rule is what refuses.
    assert!(store.actor_may_act(GUILD_A, &service));
    let error = store
        .commit_hold(&hold_request(GUILD_A, target, &service))
        .expect_err("a service may not place a hold");
    assert_refusal(&error);
    let error = store
        .commit_revocation(&revocation_request(GUILD_A, target, &service))
        .expect_err("a service may not revoke");
    assert_refusal(&error);
    let error = store
        .clear_quota_adjustment(GUILD_A, &service)
        .expect_err("a service may not clear an adjustment");
    assert_refusal(&error);

    // A service may lower a budget, and the map lets it through to that
    // class rule. The refusal above is therefore the class rule, not the
    // map.
    store
        .commit_quota_adjustment(&adjustment(GUILD_A, 500, 512 * 1024, &service))
        .expect("a service may lower a budget");
    // ... and it may not raise one back.
    let error = store
        .commit_quota_adjustment(&adjustment(GUILD_A, 900, 900 * 1024, &service))
        .expect_err("a service may not raise a budget");
    assert_refusal(&error);
}

/// An empty map is the same state as an absent map: the guild names nobody
/// and constrains nobody of its own.
#[test]
fn an_empty_map_constrains_nobody() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(vec![]), Some(vec![])))
        .expect("store opens");
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    let target = append(&mut store, memory(GUILD_A, "req_1", "op_1", 1));

    assert!(store.actor_may_act(GUILD_A, &owner));
    store
        .commit_hold(&hold_request(GUILD_A, target, &owner))
        .expect("an empty map constrains nobody of its own");
    store
        .commit_revocation(&revocation_request(GUILD_A, target, &owner))
        .expect("an empty map constrains nobody of its own");
    store
        .commit_quota_adjustment(&adjustment(GUILD_A, 500, 512 * 1024, &owner))
        .expect("an empty map constrains nobody of its own");
}

/// Every refusal answers the same content-free error, whatever the reason.
#[test]
fn the_refusal_leaks_nothing() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![OWNER_A]), None)).expect("store opens");
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    let stranger = actor(OWNER_B, ActorKind::GuildOwner);
    let target = append(&mut store, memory(GUILD_A, "req_1", "op_1", 1));

    // The stranger is refused in the guild that names somebody else, and the
    // named owner is refused in the guild that names nobody. The two
    // refusals are the same answer.
    let stranger_error = store
        .commit_hold(&hold_request(GUILD_A, target, &stranger))
        .expect_err("the stranger is refused");
    let scoped_error = store
        .commit_hold(&hold_request(GUILD_B, target, &owner))
        .expect_err("the scoped principal is refused");
    assert_eq!(stranger_error.to_string(), scoped_error.to_string());
    assert_eq!(stranger_error.to_string(), REFUSAL);
    assert_leaks_nothing(&stranger_error, OWNER_B, GUILD_A);
    assert_leaks_nothing(&scoped_error, OWNER_A, GUILD_B);
}

/// Every governance path honours the map.
#[test]
fn every_governance_path_honours_the_map() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![OWNER_A]), None)).expect("store opens");
    let stranger = actor(OWNER_B, ActorKind::GuildOwner);
    let target = append(&mut store, memory(GUILD_A, "req_1", "op_1", 1));

    // The decision and the commit paths answer alike.
    let decision = store
        .decide_hold(&hold_request(GUILD_A, target, &stranger))
        .expect_err("the decision refuses");
    let commit = store
        .commit_hold(&hold_request(GUILD_A, target, &stranger))
        .expect_err("the commit refuses");
    assert_eq!(decision.to_string(), commit.to_string());

    let decision = store
        .decide_revocation(&revocation_request(GUILD_A, target, &stranger))
        .expect_err("the decision refuses");
    let commit = store
        .commit_revocation(&revocation_request(GUILD_A, target, &stranger))
        .expect_err("the commit refuses");
    assert_eq!(decision.to_string(), commit.to_string());

    let decision = store
        .decide_quota_adjustment(&adjustment(GUILD_A, 500, 512 * 1024, &stranger))
        .expect_err("the decision refuses");
    let commit = store
        .commit_quota_adjustment(&adjustment(GUILD_A, 500, 512 * 1024, &stranger))
        .expect_err("the commit refuses");
    assert_eq!(decision.to_string(), commit.to_string());

    // The release path: a hold the named owner placed is live, and the
    // stranger may not release it.
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    store
        .commit_hold(&hold_request(GUILD_A, target, &owner))
        .expect("the named owner places a hold");
    let error = store
        .release_hold(&release_request(GUILD_A, target, &stranger))
        .expect_err("the stranger may not release the hold");
    assert_refusal(&error);
    assert_leaks_nothing(&error, OWNER_B, GUILD_A);
}

/// A store whose policy names principals for one guild constrains that guild
/// and leaves the other to the class rule.
#[test]
fn one_guild_constrained_leaves_the_others_alone() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![OWNER_A]), None)).expect("store opens");
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    let stranger = actor(OWNER_B, ActorKind::GuildOwner);
    let target_a = append(&mut store, memory(GUILD_A, "req_1", "op_1", 1));
    let target_b = append(&mut store, memory(GUILD_B, "req_2", "op_2", 2));

    // Guild A is constrained to the principal it names.
    assert!(store.actor_may_act(GUILD_A, &owner));
    assert!(!store.actor_may_act(GUILD_A, &stranger));
    let error = store
        .commit_hold(&hold_request(GUILD_A, target_a, &stranger))
        .expect_err("the stranger may not act for the constrained guild");
    assert_refusal(&error);

    // Guild B states no map, so the class rule alone decides there — for
    // every principal the store does not know to be scoped elsewhere.
    assert!(store.actor_may_act(GUILD_B, &stranger));
    store
        .commit_hold(&hold_request(GUILD_B, target_b, &stranger))
        .expect("the unconstrained guild keeps the class rule");
    // ... and the principal named for guild A is scoped to guild A.
    assert!(!store.actor_may_act(GUILD_B, &owner));
}

/// The map is host policy, not episode material: it moves no committed
/// digest, and it is read from the policy at the moment of the call.
#[test]
fn the_map_moves_no_committed_digest() {
    let plain = Scratch::new();
    let mapped = Scratch::new();
    let mut plain_store =
        EpisodeStore::open(plain.path(), policy(None, None)).expect("store opens");
    let mut mapped_store =
        EpisodeStore::open(mapped.path(), policy(Some(vec![OWNER_A]), None)).expect("store opens");

    let plain_digest = append(&mut plain_store, memory(GUILD_A, "req_1", "op_1", 7));
    let mapped_digest = append(&mut mapped_store, memory(GUILD_A, "req_1", "op_1", 7));
    assert_eq!(
        plain_digest, mapped_digest,
        "the authority map is not bound into the canonical commitment"
    );
}

#[test]
fn the_map_is_read_from_the_policy_at_the_call() {
    let scratch = Scratch::new();
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    let target = {
        let mut store =
            EpisodeStore::open(scratch.path(), policy(None, None)).expect("store opens");
        let target = append(&mut store, memory(GUILD_A, "req_1", "op_1", 1));
        store
            .commit_hold(&hold_request(GUILD_A, target, &owner))
            .expect("the hold is placed under a policy that names nobody");
        target
    };
    // The map is host policy, so reopening under a map that names another
    // principal changes what the store accepts — the hold is still live, and
    // the principal that placed it is no longer authorized.
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![OWNER_B]), None)).expect("store opens");
    let error = store
        .release_hold(&release_request(GUILD_A, target, &owner))
        .expect_err("the map that no longer names the principal refuses");
    assert_refusal(&error);
    assert_leaks_nothing(&error, OWNER_A, GUILD_A);
}
