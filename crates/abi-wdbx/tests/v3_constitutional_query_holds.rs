//! The constitutional query's report of a hold and of a revocation.
//!
//! A hold is a placement that freezes a memory candidate; a revocation
//! withdraws one by authority. Both are governance decisions about a
//! candidate rather than new kinds of episode, so both live in derived
//! sidecars beside the ledger. These tests pin that the constitutional
//! query answers for both of them without a reader ever opening a sidecar:
//!
//! - a held candidate is listed with its hold state and never reads
//!   `Accepted`;
//! - a revoked candidate is listed with its revocation state and never
//!   reads `Accepted`;
//! - the lifecycle dimension distinguishes *no hold* from *held* and *not
//!   revoked* from *revoked*, and nothing else moves when the hold state
//!   moves;
//! - the explanation names the hold or the revocation with a closed reason;
//! - the default lists a held and a revoked record, and the caller's own
//!   exclusion flags drop them;
//! - a record with neither reads exactly as it did before;
//! - nothing crosses a guild boundary.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, ConstitutionalQuery, ConstitutionalResult, Disposition, EdgeReason,
    EpisodeEvent, EpisodeSource, EpisodeStore, EpisodeWrite, EvidenceDimension, EvidenceLevel,
    GuildEpisodePolicy, HoldEvidence, HoldKind, HoldKinds, HoldReleaseRequest, HoldRequest,
    MemoryCandidate, MemoryClass, MemoryEdge, MemoryEdgeKind, MemoryResults, Reason,
    RetentionClass, RevocationEvidence, RevocationRequest, StorePolicy,
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
        Self(std::env::temp_dir().join(format!("wdbx-constitutional-holds-{}", Uuid::new_v4())))
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

fn guild_policy(version: &str) -> GuildEpisodePolicy {
    GuildEpisodePolicy {
        learning_enabled: true,
        policy_version: version.into(),
        token_budget: 1_000,
        storage_budget_bytes: 1024 * 1024,
        current_consent_epoch: Some(7),
        retention_ttl_ms: None,
        retention_windows: None,
        authorized_principals: None,
    }
}

fn policy(version: &str) -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([
            (GUILD.into(), guild_policy(version)),
            (OTHER_GUILD.into(), guild_policy(version)),
        ]),
    }
}

fn actor(principal_id: &str, kind: ActorKind) -> ActorRef {
    ActorRef {
        principal_id: principal_id.into(),
        kind,
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
        member_scoped: false,
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

fn edge_write(request_id: &str, edge: MemoryEdge) -> EpisodeWrite {
    write(
        request_id,
        &format!("op_{request_id}"),
        EpisodeEvent::MemoryEdge {
            recorded_by: actor("abbey_service", ActorKind::Service),
            edge,
        },
    )
}

/// A contradiction in canonical (ascending) order.
fn contradiction(one: [u8; 32], other: [u8; 32]) -> MemoryEdge {
    MemoryEdge {
        kind: MemoryEdgeKind::Contradicts,
        target: one.min(other),
        counterpart: Some(one.max(other)),
        reason: EdgeReason::ConflictingObservation,
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

/// A hold request from the guild's owner.
fn hold(target: [u8; 32], kind: HoldKind) -> HoldRequest {
    HoldRequest {
        guild_ref: GUILD.into(),
        target_digest: target,
        kind,
        authority: actor("guild_owner", ActorKind::GuildOwner),
    }
}

/// A release request from the guild's owner.
fn release(target: [u8; 32], kind: HoldKind) -> HoldReleaseRequest {
    HoldReleaseRequest {
        guild_ref: GUILD.into(),
        target_digest: target,
        kind,
        authority: actor("guild_owner", ActorKind::GuildOwner),
    }
}

/// A revocation request from the guild's owner.
fn revoke(target: [u8; 32]) -> RevocationRequest {
    RevocationRequest {
        guild_ref: GUILD.into(),
        target_digest: target,
        authority: actor("guild_owner", ActorKind::GuildOwner),
    }
}

fn query_of(store: &EpisodeStore, query: &ConstitutionalQuery) -> MemoryResults {
    store.query(query).expect("the query answers")
}

fn base_query() -> ConstitutionalQuery {
    ConstitutionalQuery::new(GUILD, 64)
}

fn results_of(store: &EpisodeStore) -> MemoryResults {
    query_of(store, &base_query())
}

/// The one result for one digest, or a failure: a record the store holds is
/// never silently omitted.
fn result_for(store: &EpisodeStore, digest: [u8; 32]) -> ConstitutionalResult {
    results_of(store)
        .results
        .into_iter()
        .find(|result| result.episode_digest == digest)
        .expect("the record is listed")
}

/// The reason the explanation gives the lifecycle dimension.
fn lifecycle_reason(result: &ConstitutionalResult) -> Reason {
    result
        .explanation
        .entries
        .iter()
        .find(|(dimension, _)| *dimension == EvidenceDimension::Lifecycle)
        .map(|(_, reason)| *reason)
        .expect("the lifecycle dimension has a reason")
}

#[test]
fn v3_constitutional_query_held_candidate_is_listed_with_its_hold_state() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let held = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let free = append(&mut store, memory("request_2", "memory_2", candidate(2)));
    store
        .commit_hold(&hold(held, HoldKind::Legal))
        .expect("the hold commits");

    let results = results_of(&store);
    assert_eq!(results.results.len(), 2, "the store hides nothing");

    let held_result = result_for(&store, held);
    assert_eq!(
        held_result.disposition,
        Disposition::Held,
        "a held record never reads accepted"
    );
    assert_ne!(held_result.disposition, Disposition::Accepted);
    assert_eq!(
        held_result.dimensions.lifecycle.hold,
        HoldEvidence::Held(HoldKinds {
            legal: true,
            ..HoldKinds::default()
        }),
        "the lifecycle dimension names the hold"
    );
    assert_eq!(
        held_result.dimensions.lifecycle.revocation,
        RevocationEvidence::NotRevoked
    );
    assert_eq!(lifecycle_reason(&held_result), Reason::Held);

    // A record no hold names reads as no hold: the dimension distinguishes
    // the two states rather than reporting one measurement for both.
    let free_result = result_for(&store, free);
    assert_eq!(free_result.dimensions.lifecycle.hold, HoldEvidence::NotHeld);
    assert_eq!(free_result.disposition, Disposition::Accepted);
    assert_eq!(lifecycle_reason(&free_result), Reason::Live);

    assert_eq!(results.disposition_counts.get(&Disposition::Held), Some(&1));
    assert_eq!(
        results.disposition_counts.get(&Disposition::Accepted),
        Some(&1)
    );
}

#[test]
fn v3_constitutional_query_reports_the_kinds_of_the_live_hold() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    store
        .commit_hold(&hold(digest, HoldKind::Legal))
        .expect("the hold commits");
    store
        .commit_hold(&hold(digest, HoldKind::Investigation))
        .expect("the second hold commits");

    let result = result_for(&store, digest);
    assert_eq!(
        result.dimensions.lifecycle.hold,
        HoldEvidence::Held(HoldKinds {
            legal: true,
            investigation: true,
            user_request: false,
        })
    );
    assert_eq!(result.disposition, Disposition::Held);
}

#[test]
fn v3_constitutional_query_a_released_hold_reads_as_no_hold() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    store
        .commit_hold(&hold(digest, HoldKind::Legal))
        .expect("the hold commits");
    assert_eq!(result_for(&store, digest).disposition, Disposition::Held);

    store
        .release_hold(&release(digest, HoldKind::Legal))
        .expect("the hold is released");

    let result = result_for(&store, digest);
    assert_eq!(
        result.dimensions.lifecycle.hold,
        HoldEvidence::NotHeld,
        "a released hold freezes nothing"
    );
    assert_eq!(result.disposition, Disposition::Accepted);
    assert_eq!(lifecycle_reason(&result), Reason::Live);
}

#[test]
fn v3_constitutional_query_revoked_candidate_is_listed_with_its_revocation_state() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let revoked = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let free = append(&mut store, memory("request_2", "memory_2", candidate(2)));
    store
        .commit_revocation(&revoke(revoked))
        .expect("the revocation commits");

    let results = results_of(&store);
    assert_eq!(results.results.len(), 2, "the store hides nothing");

    let revoked_result = result_for(&store, revoked);
    assert_eq!(
        revoked_result.disposition,
        Disposition::Revoked,
        "a revoked record never reads accepted"
    );
    assert_ne!(revoked_result.disposition, Disposition::Accepted);
    assert_eq!(
        revoked_result.dimensions.lifecycle.revocation,
        RevocationEvidence::Revoked
    );
    assert_eq!(
        revoked_result.dimensions.lifecycle.hold,
        HoldEvidence::NotHeld
    );
    assert_eq!(lifecycle_reason(&revoked_result), Reason::Revoked);

    let free_result = result_for(&store, free);
    assert_eq!(
        free_result.dimensions.lifecycle.revocation,
        RevocationEvidence::NotRevoked,
        "the dimension distinguishes not revoked from revoked"
    );
    assert_eq!(free_result.disposition, Disposition::Accepted);

    assert_eq!(
        results.disposition_counts.get(&Disposition::Revoked),
        Some(&1)
    );
}

#[test]
fn v3_constitutional_query_the_revocation_takes_precedence_over_a_contradiction() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let revoked = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let counterpart = append(&mut store, memory("request_2", "memory_2", candidate(2)));
    append(
        &mut store,
        edge_write("request_3", contradiction(revoked, counterpart)),
    );
    store
        .commit_revocation(&revoke(revoked))
        .expect("the revocation commits");

    // The revoked candidate reads as revoked: the authority's withdrawal is
    // the more severe of the two findings.
    let revoked_result = result_for(&store, revoked);
    assert_eq!(revoked_result.disposition, Disposition::Revoked);
    assert_eq!(
        revoked_result.dimensions.outcome,
        abi_wdbx::v3::episode::OutcomeEvidence::Contradicted,
        "the contradiction stays visible in the outcome dimension"
    );
    assert_eq!(
        revoked_result.dimensions.lifecycle.revocation,
        RevocationEvidence::Revoked
    );

    // The counterpart is contradicted, and a contradiction outranks a hold:
    // a hold doubts nothing about the content.
    let counterpart_result = result_for(&store, counterpart);
    assert_eq!(counterpart_result.disposition, Disposition::Contradicted);

    // A hold on a contradicted record does not outrank the contradiction.
    store
        .commit_hold(&hold(counterpart, HoldKind::Legal))
        .expect("the hold commits");
    let held_counterpart = result_for(&store, counterpart);
    assert_eq!(held_counterpart.disposition, Disposition::Contradicted);
    assert_eq!(
        held_counterpart.dimensions.lifecycle.hold,
        HoldEvidence::Held(HoldKinds {
            legal: true,
            ..HoldKinds::default()
        }),
        "the hold is still visible in the lifecycle dimension"
    );
}

#[test]
fn v3_constitutional_query_a_hold_and_a_revocation_are_named_together() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    store
        .commit_hold(&hold(digest, HoldKind::Investigation))
        .expect("the hold commits");
    store
        .commit_revocation(&revoke(digest))
        .expect("the revocation commits");

    let result = result_for(&store, digest);
    assert_eq!(result.disposition, Disposition::Revoked);
    assert_eq!(
        lifecycle_reason(&result),
        Reason::HeldAndRevoked,
        "the explanation names both facts"
    );
    assert_eq!(
        result.dimensions.lifecycle.hold,
        HoldEvidence::Held(HoldKinds {
            investigation: true,
            ..HoldKinds::default()
        })
    );
    assert_eq!(
        result.dimensions.lifecycle.revocation,
        RevocationEvidence::Revoked
    );
}

#[test]
fn v3_constitutional_query_mutating_the_hold_state_moves_only_the_lifecycle_dimension() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let other = append(&mut store, memory("request_2", "memory_2", candidate(2)));
    let baseline = result_for(&store, digest);
    let other_baseline = result_for(&store, other);

    store
        .commit_hold(&hold(digest, HoldKind::Legal))
        .expect("the hold commits");
    let mutated = result_for(&store, digest);

    // Every other dimension stands still.
    assert_eq!(
        baseline.dimensions.cryptographic,
        mutated.dimensions.cryptographic
    );
    assert_eq!(baseline.dimensions.source, mutated.dimensions.source);
    assert_eq!(
        baseline.dimensions.calibration,
        mutated.dimensions.calibration
    );
    assert_eq!(baseline.dimensions.outcome, mutated.dimensions.outcome);
    assert_eq!(
        baseline.dimensions.compatibility,
        mutated.dimensions.compatibility
    );
    assert_eq!(baseline.dimensions.regime, mutated.dimensions.regime);
    assert_eq!(
        baseline.dimensions.confidence,
        mutated.dimensions.confidence
    );
    assert_ne!(
        baseline.dimensions.lifecycle, mutated.dimensions.lifecycle,
        "the lifecycle dimension is the one that moves"
    );
    assert_eq!(
        baseline.disposition,
        Disposition::Accepted,
        "the baseline record is untouched"
    );
    assert_ne!(baseline.disposition, mutated.disposition);

    // The record that was not touched reads exactly as before.
    let other_mutated = result_for(&store, other);
    assert_eq!(other_mutated, other_baseline);
}

#[test]
fn v3_constitutional_query_mutating_the_revocation_state_moves_only_the_lifecycle_dimension() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let baseline = result_for(&store, digest);

    store
        .commit_revocation(&revoke(digest))
        .expect("the revocation commits");
    let mutated = result_for(&store, digest);

    assert_eq!(
        baseline.dimensions.cryptographic,
        mutated.dimensions.cryptographic
    );
    assert_eq!(baseline.dimensions.source, mutated.dimensions.source);
    assert_eq!(
        baseline.dimensions.calibration,
        mutated.dimensions.calibration
    );
    assert_eq!(baseline.dimensions.outcome, mutated.dimensions.outcome);
    assert_eq!(
        baseline.dimensions.compatibility,
        mutated.dimensions.compatibility
    );
    assert_eq!(baseline.dimensions.regime, mutated.dimensions.regime);
    assert_eq!(
        baseline.dimensions.confidence,
        mutated.dimensions.confidence
    );
    assert_ne!(baseline.dimensions.lifecycle, mutated.dimensions.lifecycle);
    assert_ne!(baseline.disposition, mutated.disposition);
}

#[test]
fn v3_constitutional_query_the_constraint_flags_exclude_held_and_revoked_records() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let held = append(&mut store, memory("request_1", "memory_1", candidate(1)));
    let revoked = append(&mut store, memory("request_2", "memory_2", candidate(2)));
    let free = append(&mut store, memory("request_3", "memory_3", candidate(3)));
    store
        .commit_hold(&hold(held, HoldKind::Legal))
        .expect("the hold commits");
    store
        .commit_revocation(&revoke(revoked))
        .expect("the revocation commits");

    // The default lists them: a hold and a revocation are evidence the
    // caller must not silently ignore.
    let digests: Vec<[u8; 32]> = results_of(&store)
        .results
        .iter()
        .map(|result| result.episode_digest)
        .collect();
    assert!(digests.contains(&held));
    assert!(digests.contains(&revoked));
    assert!(digests.contains(&free));

    // The caller may exclude them.
    let mut excluding_held = base_query();
    excluding_held.constraints.include_held = false;
    let digests: Vec<[u8; 32]> = query_of(&store, &excluding_held)
        .results
        .iter()
        .map(|result| result.episode_digest)
        .collect();
    assert!(
        !digests.contains(&held),
        "the caller excluded the held record"
    );
    assert!(digests.contains(&revoked));
    assert!(digests.contains(&free));

    let mut excluding_revoked = base_query();
    excluding_revoked.constraints.include_revoked = false;
    let digests: Vec<[u8; 32]> = query_of(&store, &excluding_revoked)
        .results
        .iter()
        .map(|result| result.episode_digest)
        .collect();
    assert!(!digests.contains(&revoked));
    assert!(digests.contains(&held));
    assert!(digests.contains(&free));

    let mut excluding_both = base_query();
    excluding_both.constraints.include_held = false;
    excluding_both.constraints.include_revoked = false;
    let digests: Vec<[u8; 32]> = query_of(&store, &excluding_both)
        .results
        .iter()
        .map(|result| result.episode_digest)
        .collect();
    assert_eq!(digests, vec![free]);
}

#[test]
fn v3_constitutional_query_a_record_with_neither_reads_exactly_as_before() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", candidate(1)));

    let result = result_for(&store, digest);
    assert_eq!(result.disposition, Disposition::Accepted);
    assert_eq!(result.dimensions.lifecycle.hold, HoldEvidence::NotHeld);
    assert_eq!(
        result.dimensions.lifecycle.revocation,
        RevocationEvidence::NotRevoked
    );
    assert_eq!(lifecycle_reason(&result), Reason::Live);
    assert_eq!(result.explanation.entries.len(), 8);
    assert_eq!(
        result
            .explanation
            .entries
            .iter()
            .map(|(dimension, _)| *dimension)
            .collect::<Vec<_>>(),
        EvidenceDimension::all().to_vec()
    );
}

#[test]
fn v3_constitutional_query_a_hold_and_a_revocation_do_not_cross_guilds() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let mut foreign_write = memory("request_1", "memory_1", candidate(1));
    foreign_write.guild_ref = OTHER_GUILD.into();
    let foreign = append(&mut store, foreign_write);
    let own = append(&mut store, memory("request_2", "memory_2", candidate(2)));

    // A hold in the guild names only the guild's own candidate.
    store
        .commit_hold(&hold(own, HoldKind::Legal))
        .expect("the hold commits");
    store
        .commit_revocation(&RevocationRequest {
            guild_ref: GUILD.into(),
            target_digest: own,
            authority: actor("guild_owner", ActorKind::GuildOwner),
        })
        .expect("the revocation commits");

    let results = results_of(&store);
    assert_eq!(
        results.results.len(),
        1,
        "the other guild is not in the answer"
    );
    assert_eq!(results.results[0].episode_digest, own);
    assert_eq!(results.results[0].disposition, Disposition::Revoked);

    // The other guild's own query never sees the guild's decisions.
    let foreign_results = query_of(&store, &ConstitutionalQuery::new(OTHER_GUILD, 64));
    assert_eq!(foreign_results.results.len(), 1);
    let foreign_result = &foreign_results.results[0];
    assert_eq!(foreign_result.episode_digest, foreign);
    assert_eq!(foreign_result.disposition, Disposition::Accepted);
    assert_eq!(
        foreign_result.dimensions.lifecycle.hold,
        HoldEvidence::NotHeld
    );
    assert_eq!(
        foreign_result.dimensions.lifecycle.revocation,
        RevocationEvidence::NotRevoked
    );
}

#[test]
fn v3_constitutional_query_the_hold_and_the_revocation_survive_close_and_reopen() {
    let scratch = Scratch::new();
    let store_policy = policy("policy_v1");
    let (held, revoked);
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        held = append(&mut store, memory("request_1", "memory_1", candidate(1)));
        revoked = append(&mut store, memory("request_2", "memory_2", candidate(2)));
        store
            .commit_hold(&hold(held, HoldKind::Legal))
            .expect("the hold commits");
        store
            .commit_revocation(&revoke(revoked))
            .expect("the revocation commits");
    }

    // The sidecars are read at open, so the reopened store answers exactly
    // like the store that made the decisions.
    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    let results = results_of(&reopened);
    assert_eq!(results.results.len(), 2);
    assert_eq!(result_for(&reopened, held).disposition, Disposition::Held);
    assert_eq!(
        result_for(&reopened, revoked).disposition,
        Disposition::Revoked
    );
    assert_eq!(results.disposition_counts.get(&Disposition::Held), Some(&1));
    assert_eq!(
        results.disposition_counts.get(&Disposition::Revoked),
        Some(&1)
    );
}
