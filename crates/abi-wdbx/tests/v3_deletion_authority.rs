//! Attributable destruction orders in the v3 episode store.
//!
//! A redaction request and an object-key destruction request are governance
//! acts: they order the destruction of a forgotten candidate's derived
//! copies and of the object key that holds its payload. Both carry the
//! `actor` that issues them, and the store refuses the order unless the
//! guild's authority map names that principal and the class is one that may
//! destroy.
//!
//! What these tests pin:
//!
//! - a destruction order by a permitted authority is accepted, reported, and
//!   survives close and reopen, and the sidecar records the principal;
//! - a `Service` may not order a destruction, even when the map names it;
//! - a `HumanSubject` may not order a destruction;
//! - a principal the guild's map does not name is refused, and so is a
//!   principal named for a different guild;
//! - every refusal is content-free: it names neither the principal, nor the
//!   guild, nor the key, nor the target;
//! - a truncated or mis-checksummed sidecar is corruption;
//! - a live target is still refused and a forgotten target still allowed.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, DerivedCopyKind, EpisodeEvent, EpisodeSource, EpisodeStore,
    EpisodeStoreError, EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MemoryCandidate,
    MemoryClass, ObjectKeyDestructionRequest, ObjectKeyRef, RedactionRequest, RetentionClass,
    StorePolicy,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// The guild whose policy names its own principal.
const GUILD_A: &str = "guild_a";
/// The guild whose policy names a different principal.
const GUILD_B: &str = "guild_b";
/// The principal the first guild names.
const OWNER_A: &str = "owner_a";
/// The principal the second guild names.
const OWNER_B: &str = "owner_b";
/// The service that records the memory candidates.
const SERVICE: &str = "abbey_service";
/// The human subject that filed the request.
const SUBJECT: &str = "request_subject";
/// The one refusal every authority failure answers with.
const REFUSAL: &str = "episode_transition_invalid";
/// The redaction sidecar beside the ledger.
const REDACTION_FILE: &str = "episodes.v1.redaction.jsonl";
/// The object-key sidecar beside the ledger.
const OBJECT_KEY_FILE: &str = "episodes.v1.object_keys.jsonl";
/// The object key the destruction orders name.
const KEY_ID: &str = "object_1";

/// A disposable store directory that removes itself.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-deletion-authority-{}", Uuid::new_v4())))
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

/// The tombstone that forgets one candidate.
fn forget(guild_ref: &str, request_id: &str, operation_id: &str, target: [u8; 32]) -> EpisodeWrite {
    write(
        guild_ref,
        request_id,
        operation_id,
        EpisodeEvent::MemoryCandidate {
            recorded_by: actor(SERVICE, ActorKind::Service),
            candidate: MemoryCandidate {
                payload_commitment: [0; 32],
                payload_bytes: 0,
                forgets: Some(target),
                ..candidate(1)
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
    preview
}

/// One candidate of a guild, already forgotten.
fn forgotten_target(store: &mut EpisodeStore, guild_ref: &str, seed: u8) -> [u8; 32] {
    let target = append(
        store,
        memory(
            guild_ref,
            &format!("request_{seed}"),
            &format!("memory_{seed}"),
            seed,
        ),
    );
    append(
        store,
        forget(
            guild_ref,
            &format!("forget_{seed}"),
            &format!("tombstone_{seed}"),
            target,
        ),
    );
    target
}

fn redaction(guild_ref: &str, target: [u8; 32], actor: &ActorRef) -> RedactionRequest {
    RedactionRequest {
        guild_ref: guild_ref.into(),
        target_digest: target,
        kinds: vec![DerivedCopyKind::Embedding],
        actor: actor.clone(),
    }
}

fn destruction(guild_ref: &str, target: [u8; 32], actor: &ActorRef) -> ObjectKeyDestructionRequest {
    ObjectKeyDestructionRequest {
        guild_ref: guild_ref.into(),
        target_digest: target,
        key: ObjectKeyRef {
            tenant: "tenant_a".into(),
            key_id: KEY_ID.into(),
        },
        actor: actor.clone(),
    }
}

/// Assert that a refusal is the one content-free refusal.
fn assert_refusal(error: &EpisodeStoreError) {
    assert_eq!(error.to_string(), REFUSAL, "the refusal is content-free");
    assert!(matches!(error, EpisodeStoreError::InvalidTransition));
}

/// Assert that a refusal names neither the principal, nor the guild, nor the
/// key, nor the target.
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
    assert!(
        !rendered.contains(KEY_ID),
        "the refusal names the key: {rendered}"
    );
}

/// The hexadecimal form of one digest.
fn hex(digest: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut text = String::with_capacity(64);
    for byte in digest {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// A destruction order by a permitted authority is accepted, reported, and
/// survives close and reopen — and the sidecar records the principal, so the
/// order is attributable.
#[test]
fn a_permitted_authority_orders_a_destruction_that_survives_reopen() {
    let scratch = Scratch::new();
    let store_policy = policy(Some(vec![OWNER_A]), None);
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    let target;
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        target = forgotten_target(&mut store, GUILD_A, 1);

        // The decision accepts, and records nothing.
        let decision = store
            .decide_redaction(&redaction(GUILD_A, target, &owner))
            .expect("the decision accepts");
        assert_eq!(decision.requested, vec![DerivedCopyKind::Embedding]);
        assert!(
            store
                .redaction_state(GUILD_A, &target)
                .expect("redaction state")
                .is_none(),
            "the decision records nothing"
        );

        // The commit records the request, and the state reports it.
        let receipt = store
            .commit_redaction(&redaction(GUILD_A, target, &owner))
            .expect("the request commits");
        assert_eq!(receipt.requested, vec![DerivedCopyKind::Embedding]);
        let state = store
            .redaction_state(GUILD_A, &target)
            .expect("redaction state")
            .expect("a recorded request");
        assert_eq!(state.target_digest, target);
        assert_eq!(state.kinds, vec![DerivedCopyKind::Embedding]);

        // The object-key order is accepted as well.
        store
            .commit_object_key_destruction(&destruction(GUILD_A, target, &owner))
            .expect("the object-key request commits");
        let state = store
            .object_key_destruction_state(
                GUILD_A,
                &ObjectKeyRef {
                    tenant: "tenant_a".into(),
                    key_id: KEY_ID.into(),
                },
            )
            .expect("object key state")
            .expect("a recorded request");
        assert_eq!(state.target_digest, target);
    }

    // The sidecar records the principal that issued the order.
    let redaction_bytes =
        fs::read(scratch.path().join(REDACTION_FILE)).expect("the redaction index");
    let redaction_text = String::from_utf8(redaction_bytes).expect("the sidecar is text");
    assert!(
        redaction_text.contains(&format!("\"principal_id\":\"{OWNER_A}\"")),
        "the sidecar records the principal: {redaction_text}"
    );
    assert!(
        redaction_text.contains("\"authority_kind\":\"guild_owner\""),
        "the sidecar records the class: {redaction_text}"
    );

    // The request survives close and reopen: the sidecar is replayed.
    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    let state = reopened
        .redaction_state(GUILD_A, &target)
        .expect("redaction state")
        .expect("a recorded request");
    assert_eq!(state.kinds, vec![DerivedCopyKind::Embedding]);
    let state = reopened
        .object_key_destruction_state(
            GUILD_A,
            &ObjectKeyRef {
                tenant: "tenant_a".into(),
                key_id: KEY_ID.into(),
            },
        )
        .expect("object key state")
        .expect("a recorded request");
    assert_eq!(state.target_digest, target);
}

/// A service may not order a destruction, even when the map names it: the
/// service writes the candidates, and a writer that could destroy the
/// evidence it just wrote would defeat the erasure record.
#[test]
fn a_service_may_not_order_a_destruction() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![SERVICE]), None)).expect("open");
    let service = actor(SERVICE, ActorKind::Service);
    let target = forgotten_target(&mut store, GUILD_A, 1);

    // The map admits the principal, so the class rule is what refuses.
    assert!(store.actor_may_act(GUILD_A, &service));
    let error = store
        .decide_redaction(&redaction(GUILD_A, target, &service))
        .expect_err("a service may not order a redaction");
    assert_refusal(&error);
    assert_leaks_nothing(&error, SERVICE, GUILD_A);
    let error = store
        .commit_redaction(&redaction(GUILD_A, target, &service))
        .expect_err("a service may not order a redaction");
    assert_refusal(&error);
    let error = store
        .decide_object_key_destruction(&destruction(GUILD_A, target, &service))
        .expect_err("a service may not order an object-key destruction");
    assert_refusal(&error);
    let error = store
        .commit_object_key_destruction(&destruction(GUILD_A, target, &service))
        .expect_err("a service may not order an object-key destruction");
    assert_refusal(&error);

    // Nothing is recorded.
    assert!(
        store
            .redaction_state(GUILD_A, &target)
            .expect("redaction state")
            .is_none(),
        "a refused order is recorded nowhere"
    );
    assert!(
        store
            .object_key_destruction_state(
                GUILD_A,
                &ObjectKeyRef {
                    tenant: "tenant_a".into(),
                    key_id: KEY_ID.into(),
                },
            )
            .expect("object key state")
            .is_none(),
        "a refused order is recorded nowhere"
    );
}

/// A human subject may not order a destruction: the subject of a request is
/// not an authority in this store.
#[test]
fn a_human_subject_may_not_order_a_destruction() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![SUBJECT]), None)).expect("open");
    let subject = actor(SUBJECT, ActorKind::HumanSubject);
    let target = forgotten_target(&mut store, GUILD_A, 1);

    // The map admits the principal, so the class rule is what refuses.
    assert!(store.actor_may_act(GUILD_A, &subject));
    let error = store
        .decide_redaction(&redaction(GUILD_A, target, &subject))
        .expect_err("a human subject may not order a redaction");
    assert_refusal(&error);
    assert_leaks_nothing(&error, SUBJECT, GUILD_A);
    let error = store
        .commit_redaction(&redaction(GUILD_A, target, &subject))
        .expect_err("a human subject may not order a redaction");
    assert_refusal(&error);
    let error = store
        .commit_object_key_destruction(&destruction(GUILD_A, target, &subject))
        .expect_err("a human subject may not order an object-key destruction");
    assert_refusal(&error);
    assert!(
        store
            .redaction_state(GUILD_A, &target)
            .expect("redaction state")
            .is_none(),
        "a refused order is recorded nowhere"
    );
}

/// A principal the guild's map does not name may not order a destruction.
#[test]
fn a_principal_the_map_does_not_name_is_refused() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![OWNER_A]), None)).expect("open");
    let stranger = actor(OWNER_B, ActorKind::GuildOwner);
    let target = forgotten_target(&mut store, GUILD_A, 1);

    assert!(!store.actor_may_act(GUILD_A, &stranger));
    let error = store
        .decide_redaction(&redaction(GUILD_A, target, &stranger))
        .expect_err("the stranger may not order a redaction");
    assert_refusal(&error);
    assert_leaks_nothing(&error, OWNER_B, GUILD_A);
    let error = store
        .commit_redaction(&redaction(GUILD_A, target, &stranger))
        .expect_err("the stranger may not order a redaction");
    assert_refusal(&error);
    let error = store
        .commit_object_key_destruction(&destruction(GUILD_A, target, &stranger))
        .expect_err("the stranger may not order an object-key destruction");
    assert_refusal(&error);
    assert!(
        store
            .redaction_state(GUILD_A, &target)
            .expect("redaction state")
            .is_none(),
        "a refused order is recorded nowhere"
    );
}

/// A principal named for one guild may not order a destruction in another,
/// even where that other guild states no map of its own.
#[test]
fn a_principal_named_for_another_guild_is_refused() {
    let scratch = Scratch::new();
    let store_policy = policy(Some(vec![OWNER_A]), Some(vec![OWNER_B]));
    let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
    let owner_a = actor(OWNER_A, ActorKind::GuildOwner);
    let owner_b = actor(OWNER_B, ActorKind::GuildOwner);
    let target_a = forgotten_target(&mut store, GUILD_A, 1);
    let target_b = forgotten_target(&mut store, GUILD_B, 2);

    // The owner of the second guild may not order a destruction in the first.
    assert!(!store.actor_may_act(GUILD_A, &owner_b));
    let error = store
        .commit_redaction(&redaction(GUILD_A, target_a, &owner_b))
        .expect_err("the owner of one guild may not destroy in another");
    assert_refusal(&error);
    assert_leaks_nothing(&error, OWNER_B, GUILD_A);

    // And the owner of the first guild may not order one in the second.
    assert!(!store.actor_may_act(GUILD_B, &owner_a));
    let error = store
        .commit_object_key_destruction(&destruction(GUILD_B, target_b, &owner_a))
        .expect_err("the owner of one guild may not destroy in another");
    assert_refusal(&error);
    assert_leaks_nothing(&error, OWNER_A, GUILD_B);

    // Each guild's own principal may order a destruction there.
    store
        .commit_redaction(&redaction(GUILD_A, target_a, &owner_a))
        .expect("the named principal orders a redaction");
    store
        .commit_object_key_destruction(&destruction(GUILD_B, target_b, &owner_b))
        .expect("the named principal orders an object-key destruction");
}

/// The refusal names nothing: not the principal, not the guild, not the key,
/// and not the target. It is the same answer for every kind of refusal.
#[test]
fn the_refusal_leaks_nothing() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![OWNER_A]), None)).expect("open");
    let stranger = actor(OWNER_B, ActorKind::GuildOwner);
    let target = forgotten_target(&mut store, GUILD_A, 1);

    // The refusal for an unauthorized actor.
    let unauthorized = store
        .commit_redaction(&redaction(GUILD_A, target, &stranger))
        .expect_err("the stranger is refused");
    assert_refusal(&unauthorized);
    let rendered = format!("{unauthorized:?} {unauthorized}");
    assert!(
        !rendered.contains(&hex(&target)),
        "the refusal names the target: {rendered}"
    );

    // The refusal for a live target, by an authorized actor, is the same
    // answer: the store is not an oracle about what it holds.
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    let live = append(&mut store, memory(GUILD_A, "live_1", "live_memory_1", 3));
    let refused = store
        .commit_redaction(&redaction(GUILD_A, live, &owner))
        .expect_err("a live target is refused");
    assert_refusal(&refused);
    assert_eq!(unauthorized.to_string(), refused.to_string());
}

/// A truncated or mis-checksummed sidecar is corruption: the store refuses
/// to open rather than believe a shorter or altered history.
#[test]
fn a_truncated_or_mis_checksummed_sidecar_is_corrupt() {
    let scratch = Scratch::new();
    let store_policy = policy(Some(vec![OWNER_A]), None);
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        let target = forgotten_target(&mut store, GUILD_A, 1);
        store
            .commit_redaction(&redaction(GUILD_A, target, &owner))
            .expect("the request commits");
        store
            .commit_object_key_destruction(&destruction(GUILD_A, target, &owner))
            .expect("the object-key request commits");
    }

    // A sidecar whose tail is cut off is corruption.
    let path = scratch.path().join(REDACTION_FILE);
    let bytes = fs::read(&path).expect("the redaction index");
    fs::write(&path, &bytes[..bytes.len() - 12]).expect("the sidecar is truncated");
    assert!(matches!(
        EpisodeStore::open(scratch.path(), store_policy.clone()).expect_err("a truncated sidecar"),
        EpisodeStoreError::Corrupt
    ));

    // A sidecar truncated at a line boundary is corruption too: the header
    // still names the request lines the file no longer holds.
    let header_end = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .expect("the sidecar opens with a header")
        + 1;
    fs::write(&path, &bytes[..header_end]).expect("the sidecar is truncated");
    assert!(matches!(
        EpisodeStore::open(scratch.path(), store_policy.clone()).expect_err("a truncated sidecar"),
        EpisodeStoreError::Corrupt
    ));

    // A sidecar whose request line was altered is corruption: the line's own
    // checksum no longer matches.
    let path = scratch.path().join(REDACTION_FILE);
    let text = String::from_utf8(bytes.clone()).expect("the sidecar is text");
    let altered = text.replace(OWNER_A, OWNER_B);
    assert_ne!(altered, text, "the principal was altered");
    fs::write(&path, altered).expect("the sidecar is altered");
    assert!(matches!(
        EpisodeStore::open(scratch.path(), store_policy.clone()).expect_err("an altered sidecar"),
        EpisodeStoreError::Corrupt
    ));

    // The same holds for the object-key sidecar.
    let path = scratch.path().join(OBJECT_KEY_FILE);
    let bytes = fs::read(&path).expect("the object-key index");
    fs::write(&path, &bytes[..bytes.len() - 8]).expect("the sidecar is truncated");
    assert!(matches!(
        EpisodeStore::open(scratch.path(), store_policy.clone()).expect_err("a truncated sidecar"),
        EpisodeStoreError::Corrupt
    ));
    let text = String::from_utf8(bytes.clone()).expect("the sidecar is text");
    let altered = text.replace(KEY_ID, "object_2");
    assert_ne!(altered, text, "the key was altered");
    fs::write(&path, altered).expect("the sidecar is altered");
    assert!(matches!(
        EpisodeStore::open(scratch.path(), store_policy.clone()).expect_err("an altered sidecar"),
        EpisodeStoreError::Corrupt
    ));
}

/// The existing behaviour is unchanged: a live target is still refused, and
/// a forgotten target is still allowed.
#[test]
fn a_live_target_is_refused_and_a_forgotten_one_is_allowed() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![OWNER_A]), None)).expect("open");
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    let live = append(&mut store, memory(GUILD_A, "live_1", "live_memory_1", 1));

    // A live record's payload is not destroyed under the canonical ledger.
    assert!(matches!(
        store
            .decide_redaction(&redaction(GUILD_A, live, &owner))
            .expect_err("a live target is refused"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(matches!(
        store
            .commit_redaction(&redaction(GUILD_A, live, &owner))
            .expect_err("a live target is refused"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(matches!(
        store
            .commit_object_key_destruction(&destruction(GUILD_A, live, &owner))
            .expect_err("a live target is refused"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(
        store
            .redaction_state(GUILD_A, &live)
            .expect("redaction state")
            .is_none(),
        "a refused order is recorded nowhere"
    );

    // Once the target is forgotten, the order is admissible.
    append(&mut store, forget(GUILD_A, "forget_1", "tombstone_1", live));
    store
        .commit_redaction(&redaction(GUILD_A, live, &owner))
        .expect("the forgotten target's redaction commits");
    store
        .commit_object_key_destruction(&destruction(GUILD_A, live, &owner))
        .expect("the forgotten target's object-key request commits");
    assert!(
        store
            .redaction_state(GUILD_A, &live)
            .expect("redaction state")
            .is_some(),
        "the order was recorded"
    );
}

/// A guild the store does not serve is refused as stale, on every one of the
/// four destruction methods.
///
/// The governance slices answer one question in one voice: the holds slice
/// refuses an unserved guild as `StaleBinding`, the quota slice does the
/// same, and the deletion lifecycle now does too.
#[test]
fn an_unserved_guild_is_refused_as_stale() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![OWNER_A]), None)).expect("open");
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    let target = forgotten_target(&mut store, GUILD_A, 1);
    let unserved = "guild_unserved";

    // The store does not serve that guild at all.
    assert!(!store.actor_may_act(unserved, &owner));

    assert!(matches!(
        store
            .decide_redaction(&redaction(unserved, target, &owner))
            .expect_err("an unserved guild is refused"),
        EpisodeStoreError::StaleBinding
    ));
    assert!(matches!(
        store
            .commit_redaction(&redaction(unserved, target, &owner))
            .expect_err("an unserved guild is refused"),
        EpisodeStoreError::StaleBinding
    ));
    assert!(matches!(
        store
            .decide_object_key_destruction(&destruction(unserved, target, &owner))
            .expect_err("an unserved guild is refused"),
        EpisodeStoreError::StaleBinding
    ));
    assert!(matches!(
        store
            .commit_object_key_destruction(&destruction(unserved, target, &owner))
            .expect_err("an unserved guild is refused"),
        EpisodeStoreError::StaleBinding
    ));

    // And nothing is recorded.
    assert!(
        store
            .redaction_state(unserved, &target)
            .expect("redaction state")
            .is_none(),
        "a refused order is recorded nowhere"
    );
    assert!(
        store
            .object_key_destruction_state(
                unserved,
                &ObjectKeyRef {
                    tenant: "tenant_a".into(),
                    key_id: KEY_ID.into(),
                },
            )
            .expect("object key state")
            .is_none(),
        "a refused order is recorded nowhere"
    );
}

/// The unserved guild and the unauthorized principal are two different
/// answers, and the difference is the guild alone.
///
/// The guild the store does not serve is refused as stale whoever asks; a
/// guild it does serve still answers the closed, content-free refusal for an
/// unauthorized principal. The two cases are therefore distinguishable only
/// by the guild, never by the principal, and neither answer leaks anything
/// about the target.
#[test]
fn the_guild_and_not_the_principal_decides_the_refusal() {
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy(Some(vec![OWNER_A]), None)).expect("open");
    let owner = actor(OWNER_A, ActorKind::GuildOwner);
    let stranger = actor(OWNER_B, ActorKind::GuildOwner);
    let target = forgotten_target(&mut store, GUILD_A, 1);
    let unserved = "guild_unserved";

    // The unserved guild answers the same way whoever asks: the guild's own
    // authority and a stranger alike.
    for principal in [&owner, &stranger] {
        let error = store
            .decide_redaction(&redaction(unserved, target, principal))
            .expect_err("an unserved guild is refused");
        assert!(matches!(error, EpisodeStoreError::StaleBinding));
        assert_leaks_nothing(&error, &principal.principal_id, unserved);
    }

    // The served guild answers the authority refusal, and only that.
    let error = store
        .decide_redaction(&redaction(GUILD_A, target, &stranger))
        .expect_err("the stranger is refused");
    assert_refusal(&error);
    assert_leaks_nothing(&error, OWNER_B, GUILD_A);

    // The two answers are different, and neither names the target.
    let stale = store
        .decide_redaction(&redaction(unserved, target, &stranger))
        .expect_err("an unserved guild is refused");
    assert_ne!(stale.to_string(), error.to_string());
    assert_leaks_nothing(&stale, OWNER_B, unserved);
    assert!(
        !format!("{stale:?} {stale}").contains(&hex(&target)),
        "the refusal names the target: {stale:?}"
    );
}
