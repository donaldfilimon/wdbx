//! Holds and candidate revocation for the v3 episode store.
//!
//! A hold freezes a memory candidate: while it is live, the candidate is not
//! expired away, not reclaimed, not silently superseded, and its payload is
//! not destroyed. A revocation withdraws a candidate by authority: it is not
//! live any more, it can never be resurrected, and its record and its
//! anti-resurrection metadata are retained.
//!
//! What these tests pin:
//!
//! - a hold is placed, reported, and survives close and reopen;
//! - a hold names a candidate of the guild and refuses an unknown digest;
//! - only the placing authority, or one above it in the governance order,
//!   may release a hold;
//! - while a hold is live the candidate is proposed neither for expiry nor
//!   for reclamation, and its payload may not be destroyed;
//! - a revoked candidate is reported as not live, and a `supersedes` naming
//!   it is refused;
//! - a `Service` actor cannot revoke; a guild owner, administrator, manager,
//!   or organization owner can;
//! - the revocation state survives close and reopen;
//! - a revoked candidate is never resurrected by a later forget or
//!   supersede;
//! - the sidecar files carry no episode content.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, DerivedCopyKind, EpisodeEvent, EpisodeSource, EpisodeStore,
    EpisodeStoreError, EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, HoldKind,
    HoldReleaseRequest, HoldRequest, HoldStatus, MemoryCandidate, MemoryClass,
    ObjectKeyDestructionRequest, ObjectKeyRef, RedactionRequest, RetentionClass, RevocationRequest,
    StorePolicy,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// The retention window the tests configure.
const TTL_MS: u64 = 60_000;
const LEDGER_FILE: &str = "episodes.v1.jsonl";
const HOLDS_FILE: &str = "episodes.v1.holds.jsonl";
const REVOCATION_FILE: &str = "episodes.v1.revocations.jsonl";

/// A payload commitment that is unmistakable in a byte-level assertion.
const PAYLOAD_MARKER: [u8; 32] = [0x5a; 32];

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-holds-revocation-{}", Uuid::new_v4())))
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

/// A hold request from the guild's owner.
fn hold(target: [u8; 32], kind: HoldKind) -> HoldRequest {
    HoldRequest {
        guild_ref: "guild_ref".into(),
        target_digest: target,
        kind,
        authority: actor("guild_owner", ActorKind::GuildOwner),
    }
}

/// A release request from one authority.
fn release(
    target: [u8; 32],
    kind: HoldKind,
    principal_id: &str,
    authority: ActorKind,
) -> HoldReleaseRequest {
    HoldReleaseRequest {
        guild_ref: "guild_ref".into(),
        target_digest: target,
        kind,
        authority: actor(principal_id, authority),
    }
}

/// A revocation request from one authority.
fn revoke(target: [u8; 32], principal_id: &str, authority: ActorKind) -> RevocationRequest {
    RevocationRequest {
        guild_ref: "guild_ref".into(),
        target_digest: target,
        authority: actor(principal_id, authority),
    }
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
fn a_hold_is_placed_reported_and_survives_close_and_reopen() {
    let scratch = Scratch::new();
    let store_policy = policy(None);
    let digest;
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
        assert!(
            store
                .hold_state("guild_ref", &digest)
                .expect("hold state")
                .is_none(),
            "no hold names the candidate yet"
        );

        let request = hold(digest, HoldKind::Legal);
        let decision = store.decide_hold(&request).expect("the decision accepts");
        assert!(!decision.already_live);
        assert_eq!(decision.kind, HoldKind::Legal);
        assert_eq!(decision.authority.principal_id, "guild_owner");
        assert!(
            store
                .hold_state("guild_ref", &digest)
                .expect("hold state")
                .is_none(),
            "the decision records nothing"
        );

        let receipt = store.commit_hold(&request).expect("the hold commits");
        assert!(!receipt.already_live);
        assert_eq!(receipt.kind, HoldKind::Legal);

        let state = store
            .hold_state("guild_ref", &digest)
            .expect("hold state")
            .expect("a recorded hold");
        assert!(state.frozen, "a live hold freezes the candidate");
        assert_eq!(state.holds.len(), 1);
        assert_eq!(state.holds[0].kind, HoldKind::Legal);
        assert_eq!(state.holds[0].status, HoldStatus::Live);
        assert_eq!(state.holds[0].target_digest, digest);
        assert_eq!(state.holds[0].authority.principal_id, "guild_owner");
        assert_eq!(state.holds[0].released_by, None);

        // The request is idempotent: the same authority placing the same
        // hold again records nothing new.
        let again = store.commit_hold(&request).expect("the hold repeats");
        assert!(again.already_live);
        assert_eq!(
            store
                .hold_state("guild_ref", &digest)
                .expect("hold state")
                .expect("a recorded hold")
                .holds
                .len(),
            1
        );
    }

    // The hold survives close and reopen: the sidecar is replayed.
    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    let state = reopened
        .hold_state("guild_ref", &digest)
        .expect("hold state")
        .expect("a recorded hold");
    assert!(state.frozen);
    assert_eq!(state.holds.len(), 1);
    assert_eq!(state.holds[0].status, HoldStatus::Live);
    assert_eq!(state.holds[0].authority.principal_id, "guild_owner");
}

#[test]
fn a_hold_names_a_candidate_and_refuses_an_unknown_digest() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(None)).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));

    // A live target is accepted.
    let request = hold(digest, HoldKind::Investigation);
    store
        .decide_hold(&request)
        .expect("a live target is accepted");
    store.commit_hold(&request).expect("the hold commits");
    assert!(
        store
            .hold_state("guild_ref", &digest)
            .expect("hold state")
            .expect("a recorded hold")
            .frozen
    );

    // An unknown digest is refused, and the refusal does not distinguish an
    // unknown digest from a candidate of another guild.
    let unknown = hold([7; 32], HoldKind::Legal);
    assert!(matches!(
        store
            .decide_hold(&unknown)
            .expect_err("an unknown target is refused"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(matches!(
        store
            .commit_hold(&unknown)
            .expect_err("an unknown target is refused"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(
        store
            .hold_state("guild_ref", &[7; 32])
            .expect("hold state")
            .is_none(),
        "a refused hold is recorded nowhere"
    );

    // A guild the store does not serve is refused as well.
    let other_guild = HoldRequest {
        guild_ref: "unknown_guild".into(),
        ..hold(digest, HoldKind::Legal)
    };
    assert!(matches!(
        store
            .decide_hold(&other_guild)
            .expect_err("an unserved guild is refused"),
        EpisodeStoreError::StaleBinding
    ));

    // A service may not place a hold: the writer of memory candidates may
    // not freeze them.
    let service = HoldRequest {
        authority: actor("abbey_service", ActorKind::Service),
        ..hold(digest, HoldKind::Legal)
    };
    assert!(matches!(
        store
            .decide_hold(&service)
            .expect_err("a service may not place a hold"),
        EpisodeStoreError::InvalidTransition
    ));
}

#[test]
fn a_hold_is_released_only_by_the_placing_authority_or_one_above_it() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(None)).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let request = hold(digest, HoldKind::Legal);
    store.commit_hold(&request).expect("the hold commits");

    // Another authority below the placing one may not release the hold.
    assert!(matches!(
        store
            .release_hold(&release(
                digest,
                HoldKind::Legal,
                "guild_manager",
                ActorKind::GuildManager
            ))
            .expect_err("another authority may not release the hold"),
        EpisodeStoreError::InvalidTransition
    ));
    // A service may not either.
    assert!(matches!(
        store
            .release_hold(&release(
                digest,
                HoldKind::Legal,
                "abbey_service",
                ActorKind::Service
            ))
            .expect_err("a service may not release the hold"),
        EpisodeStoreError::InvalidTransition
    ));
    // The hold is still live.
    assert!(
        store
            .hold_state("guild_ref", &digest)
            .expect("hold state")
            .expect("a recorded hold")
            .frozen
    );

    // The placing authority releases it.
    let receipt = store
        .release_hold(&release(
            digest,
            HoldKind::Legal,
            "guild_owner",
            ActorKind::GuildOwner,
        ))
        .expect("the placing authority releases the hold");
    assert_eq!(receipt.released.principal_id, "guild_owner");
    assert_eq!(receipt.released_by.principal_id, "guild_owner");
    let state = store
        .hold_state("guild_ref", &digest)
        .expect("hold state")
        .expect("a recorded hold");
    assert!(!state.frozen, "a released hold freezes nothing");
    assert_eq!(state.holds[0].status, HoldStatus::Released);
    assert_eq!(
        state.holds[0]
            .released_by
            .as_ref()
            .map(|actor| actor.principal_id.clone()),
        Some("guild_owner".to_string())
    );

    // Releasing again is refused: there is no live hold to release, and the
    // refusal does not distinguish "never placed" from "already released".
    assert!(matches!(
        store
            .release_hold(&release(
                digest,
                HoldKind::Legal,
                "guild_owner",
                ActorKind::GuildOwner
            ))
            .expect_err("a released hold is not released twice"),
        EpisodeStoreError::InvalidTransition
    ));

    // The candidate is free again: a correction may supersede it.
    append(
        &mut store,
        memory(
            "request_2",
            "memory_2",
            MemoryCandidate {
                supersedes: Some(digest),
                ..candidate(2)
            },
        ),
    );
}

#[test]
fn an_authority_above_the_placing_one_may_release_a_hold() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(None)).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let request = HoldRequest {
        guild_ref: "guild_ref".into(),
        target_digest: digest,
        kind: HoldKind::UserRequest,
        authority: actor("guild_manager", ActorKind::GuildManager),
    };
    store.commit_hold(&request).expect("the hold commits");

    // The organization owner is above the guild manager in the store's
    // governance order, and may release the hold.
    let receipt = store
        .release_hold(&release(
            digest,
            HoldKind::UserRequest,
            "organization_owner",
            ActorKind::OrganizationOwner,
        ))
        .expect("the organization owner releases the hold");
    assert_eq!(receipt.released.principal_id, "guild_manager");
    assert_eq!(receipt.released_by.principal_id, "organization_owner");
    assert!(
        !store
            .hold_state("guild_ref", &digest)
            .expect("hold state")
            .expect("a recorded hold")
            .frozen
    );
}

#[test]
fn a_held_candidate_is_not_expired_away_and_not_silently_superseded() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(TTL_MS))).expect("open");

    // Two TTL-bound candidates: one is held, the other is not.
    let held = append(&mut store, memory("request_1", "memory_1", operational(1)));
    let free = append(&mut store, memory("request_2", "memory_2", operational(2)));
    let held_deadline = deadline(&store, &held);

    // Without a hold, the candidate is due once its own deadline has passed.
    assert!(
        store
            .due_expirations("guild_ref", held_deadline)
            .expect("due expirations")
            .contains(&held),
        "the candidate is due once its deadline has passed"
    );
    assert_eq!(
        store
            .due_expirations("guild_ref", u64::MAX)
            .expect("due expirations"),
        vec![free.min(held), free.max(held)],
        "both candidates are due without a hold"
    );

    // The hold freezes the held candidate.
    store
        .commit_hold(&hold(held, HoldKind::Legal))
        .expect("the hold commits");
    let due = store
        .due_expirations("guild_ref", u64::MAX)
        .expect("due expirations");
    assert_eq!(due, vec![free], "a held candidate is not due");
    let writes = store
        .expiry_writes("guild_ref", u64::MAX, "request_exp", "operation_exp")
        .expect("expiry writes");
    assert_eq!(
        writes.len(),
        1,
        "the emitter emits nothing for a held candidate"
    );
    assert!(
        !writes.iter().any(|write| {
            matches!(
                &write.event,
                EpisodeEvent::MemoryCandidate { candidate, .. } if candidate.forgets == Some(held)
            )
        }),
        "the held candidate is not among the emitted forgets"
    );

    // A held candidate is not silently superseded.
    assert!(matches!(
        rejected(
            &mut store,
            &memory(
                "request_3",
                "memory_3",
                MemoryCandidate {
                    supersedes: Some(held),
                    ..candidate(3)
                }
            )
        ),
        EpisodeStoreError::InvalidTransition
    ));
}

#[test]
fn a_held_candidate_is_neither_reclaimed_nor_destroyed() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(Some(TTL_MS))).expect("open");

    // Two TTL-bound candidates: one is held, the other is not.
    let held = append(&mut store, memory("request_1", "memory_1", operational(1)));
    let free = append(&mut store, memory("request_2", "memory_2", operational(2)));
    store
        .commit_hold(&hold(held, HoldKind::Legal))
        .expect("the hold commits");

    // The tombstone is an explicit act and is still accepted; the hold
    // freezes the destruction of the content, not the erasure request.
    append(&mut store, memory("request_3", "memory_3", forget(held)));
    append(&mut store, memory("request_4", "memory_4", forget(free)));

    // The collector keeps the held record and reclaims the other one.
    let plan = store.gc_plan("guild_ref", u64::MAX).expect("plan");
    assert_eq!(
        plan.reclaimable,
        vec![free],
        "a held record is never reclaimed"
    );
    assert!(!plan.reclaimable.contains(&held), "the held record is kept");

    // The deletion lifecycle refuses to destroy a held candidate's payload
    // and derived copies.
    assert!(matches!(
        store
            .decide_redaction(&RedactionRequest {
                guild_ref: "guild_ref".into(),
                target_digest: held,
                kinds: vec![DerivedCopyKind::Embedding],
                actor: actor("guild_owner", ActorKind::GuildOwner),
            })
            .expect_err("a held target is refused"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(matches!(
        store
            .commit_redaction(&RedactionRequest {
                guild_ref: "guild_ref".into(),
                target_digest: held,
                kinds: vec![DerivedCopyKind::Embedding],
                actor: actor("guild_owner", ActorKind::GuildOwner),
            })
            .expect_err("a held target is refused"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(matches!(
        store
            .commit_object_key_destruction(&ObjectKeyDestructionRequest {
                guild_ref: "guild_ref".into(),
                target_digest: held,
                key: ObjectKeyRef {
                    tenant: "tenant_a".into(),
                    key_id: "object_1".into(),
                },
                actor: actor("guild_owner", ActorKind::GuildOwner),
            })
            .expect_err("a held target is refused"),
        EpisodeStoreError::InvalidTransition
    ));

    // The unheld candidate's copies are destroyed as usual.
    store
        .commit_redaction(&RedactionRequest {
            guild_ref: "guild_ref".into(),
            target_digest: free,
            kinds: vec![DerivedCopyKind::Embedding],
            actor: actor("guild_owner", ActorKind::GuildOwner),
        })
        .expect("the unheld target's redaction commits");
    store
        .commit_object_key_destruction(&ObjectKeyDestructionRequest {
            guild_ref: "guild_ref".into(),
            target_digest: free,
            key: ObjectKeyRef {
                tenant: "tenant_a".into(),
                key_id: "object_1".into(),
            },
            actor: actor("guild_owner", ActorKind::GuildOwner),
        })
        .expect("the unheld target's object-key request commits");
}

#[test]
fn a_revoked_candidate_is_not_live_and_cannot_be_superseded() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(None)).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));

    let request = revoke(digest, "guild_owner", ActorKind::GuildOwner);
    let decision = store
        .decide_revocation(&request)
        .expect("the decision accepts");
    assert!(!decision.already_revoked);
    assert!(
        store
            .revocation_state("guild_ref", &digest)
            .expect("revocation state")
            .is_none(),
        "the decision records nothing"
    );

    let receipt = store
        .commit_revocation(&request)
        .expect("the revocation commits");
    assert!(!receipt.already_revoked);
    assert_eq!(receipt.authority.principal_id, "guild_owner");

    let state = store
        .revocation_state("guild_ref", &digest)
        .expect("revocation state")
        .expect("a revoked candidate");
    assert_eq!(state.target_digest, digest);
    assert_eq!(state.authority.principal_id, "guild_owner");

    // The candidate is not live: the store's own liveness logic refuses a
    // correction that names it.
    assert!(matches!(
        rejected(
            &mut store,
            &memory(
                "request_2",
                "memory_2",
                MemoryCandidate {
                    supersedes: Some(digest),
                    ..candidate(2)
                }
            )
        ),
        EpisodeStoreError::InvalidTransition
    ));

    // The revocation is idempotent.
    let again = store
        .commit_revocation(&request)
        .expect("the revocation repeats");
    assert!(again.already_revoked);
}

#[test]
fn a_service_cannot_revoke_but_a_governance_authority_can() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(None)).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));

    // A service may not revoke: the writer of memory candidates may not
    // withdraw them.
    let service = revoke(digest, "abbey_service", ActorKind::Service);
    assert!(matches!(
        store
            .decide_revocation(&service)
            .expect_err("a service may not revoke"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(matches!(
        store
            .commit_revocation(&service)
            .expect_err("a service may not revoke"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(
        store
            .revocation_state("guild_ref", &digest)
            .expect("revocation state")
            .is_none(),
        "a refused revocation is recorded nowhere"
    );

    // A human subject is not an authority either.
    let subject = revoke(digest, "member", ActorKind::HumanSubject);
    assert!(matches!(
        store
            .decide_revocation(&subject)
            .expect_err("a human subject may not revoke"),
        EpisodeStoreError::InvalidTransition
    ));

    // The guild's governance authorities may.
    for (principal_id, kind) in [
        ("guild_owner", ActorKind::GuildOwner),
        ("guild_administrator", ActorKind::GuildAdministrator),
        ("guild_manager", ActorKind::GuildManager),
        ("organization_owner", ActorKind::OrganizationOwner),
    ] {
        let revoked = append(
            &mut store,
            memory(
                &format!("request_{principal_id}"),
                &format!("memory_{principal_id}"),
                candidate(7),
            ),
        );
        let request = revoke(revoked, principal_id, kind);
        store
            .decide_revocation(&request)
            .expect("a governance authority may revoke");
        store
            .commit_revocation(&request)
            .expect("the revocation commits");
        assert_eq!(
            store
                .revocation_state("guild_ref", &revoked)
                .expect("revocation state")
                .expect("a revoked candidate")
                .authority
                .principal_id,
            principal_id
        );
    }
}

#[test]
fn the_revocation_state_survives_close_and_reopen() {
    let scratch = Scratch::new();
    let store_policy = policy(None);
    let digest;
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
        store
            .commit_revocation(&revoke(digest, "guild_owner", ActorKind::GuildOwner))
            .expect("the revocation commits");
    }

    // The replay rebuilds the liveness state, and the revocation index is
    // applied to it, so the reopened store answers exactly like the store
    // that revoked the candidate.
    let mut reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    let state = reopened
        .revocation_state("guild_ref", &digest)
        .expect("revocation state")
        .expect("a revoked candidate");
    assert_eq!(state.authority.principal_id, "guild_owner");
    assert!(matches!(
        rejected(
            &mut reopened,
            &memory(
                "request_2",
                "memory_2",
                MemoryCandidate {
                    supersedes: Some(digest),
                    ..candidate(2)
                }
            )
        ),
        EpisodeStoreError::InvalidTransition
    ));
}

#[test]
fn a_revoked_candidate_is_never_resurrected() {
    let scratch = Scratch::new();
    let store_policy = policy(None);
    let digest;
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
        store
            .commit_revocation(&revoke(digest, "guild_owner", ActorKind::GuildOwner))
            .expect("the revocation commits");

        // A later forget cannot resurrect it: the candidate is not live, so
        // the store refuses the erasure request as well.
        assert!(matches!(
            rejected(&mut store, &memory("request_2", "memory_2", forget(digest))),
            EpisodeStoreError::InvalidTransition
        ));
        // A later supersede cannot resurrect it either.
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

        // The anti-resurrection metadata is retained: the record is still in
        // the ledger and still reports the revocation.
        assert!(
            store
                .find_receipt("guild_ref", &digest)
                .expect("receipt")
                .is_some(),
            "a revoked candidate keeps its record"
        );
        assert!(
            store
                .revocation_state("guild_ref", &digest)
                .expect("revocation state")
                .is_some(),
            "a revoked candidate keeps its revocation"
        );
    }

    // And the refusal survives the reopen.
    let mut reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    assert!(
        reopened
            .find_receipt("guild_ref", &digest)
            .expect("receipt")
            .is_some(),
        "a revoked candidate keeps its record across a reopen"
    );
    assert!(matches!(
        rejected(
            &mut reopened,
            &memory("request_4", "memory_4", forget(digest))
        ),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(matches!(
        rejected(
            &mut reopened,
            &memory(
                "request_5",
                "memory_5",
                MemoryCandidate {
                    supersedes: Some(digest),
                    ..candidate(5)
                }
            )
        ),
        EpisodeStoreError::InvalidTransition
    ));
}

#[test]
fn the_sidecar_files_carry_no_episode_content() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(None)).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    store
        .commit_hold(&hold(digest, HoldKind::Legal))
        .expect("the hold commits");
    store
        .commit_revocation(&revoke(digest, "guild_owner", ActorKind::GuildOwner))
        .expect("the revocation commits");
    drop(store);

    let files = store_bytes(scratch.path());
    let holds = files
        .get(&scratch.path().join(HOLDS_FILE))
        .expect("the hold index exists");
    let revocations = files
        .get(&scratch.path().join(REVOCATION_FILE))
        .expect("the revocation index exists");
    let ledger = files
        .get(&scratch.path().join(LEDGER_FILE))
        .expect("the ledger exists");

    // The payload commitment never appears in a sidecar.
    let marker = format!("{PAYLOAD_MARKER:?}");
    for bytes in [holds, revocations] {
        assert!(
            !String::from_utf8_lossy(bytes).contains(&marker),
            "a sidecar carries no payload commitment"
        );
    }
    // The ledger does carry it, in its own JSON form: the sidecar carries
    // strictly less than the canonical record.
    let json_marker = format!(
        "[{}]",
        std::iter::repeat_n("90", 32).collect::<Vec<_>>().join(",")
    );
    assert!(String::from_utf8_lossy(ledger).contains(&json_marker));

    // Every sidecar line carries only digests, labels, and identifiers.
    let expected: BTreeMap<&str, Vec<&str>> = BTreeMap::from([
        (
            HOLDS_FILE,
            vec![
                "authority_kind",
                "checksum",
                "digest",
                "guild_ref",
                "kind",
                "principal_id",
                "status",
            ],
        ),
        (
            REVOCATION_FILE,
            vec![
                "authority_kind",
                "checksum",
                "digest",
                "guild_ref",
                "principal_id",
            ],
        ),
    ]);
    for (name, keys) in expected {
        let bytes = files
            .get(&scratch.path().join(name))
            .expect("the sidecar exists");
        let text = String::from_utf8(bytes.clone()).expect("the sidecar is text");
        assert!(!text.is_empty(), "{name} holds its decision");
        for line in text.lines() {
            let value: serde_json::Value = serde_json::from_str(line).expect("the line parses");
            let object = value.as_object().expect("the line is an object");
            let mut found: Vec<&str> = object.keys().map(String::as_str).collect();
            found.sort_unstable();
            assert_eq!(found, keys, "{name} holds only its own fields");
        }
    }
}
