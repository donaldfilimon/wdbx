//! Constitutional query: the evidence-aware read model of the v3 episode
//! store.
//!
//! The canonical contract asks for trust to be eight separate dimensions and
//! never one number. These tests pin that promise from the outside: every
//! dimension is reported on its own, mutating one dimension in isolation
//! moves the verdict without touching the others, missing evidence is never
//! read as a negative finding, and nothing the store holds is hidden.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, AgeEvidence, CalibrationEvidence, ConstitutionalQuery,
    ConstitutionalResult, Disposition, EdgeReason, EpisodeEvent, EpisodeSigner, EpisodeSource,
    EpisodeStore, EpisodeStoreError, EpisodeWrite, EvidenceDimension, EvidenceLevel,
    GuildEpisodePolicy, KeyResolver, MemberScope, MemoryCandidate, MemoryClass, MemoryEdge,
    MemoryEdgeKind, MemoryResults, OutcomeEvidence, RegimeEvidence, ReputationEvidence,
    RetentionClass, SignatureEvidence, SignerKeyStatus, StorePolicy, UncertaintyEvidence,
};
use ed25519_dalek::SigningKey;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const GUILD: &str = "guild_ref";
const OTHER_GUILD: &str = "other_guild";
/// The store's own receipt bound (`MAX_RECEIPTS` in the episode store).
const STORE_LIMIT: usize = 2_048;

/// The constitutional query module's own source, for the structural claim that
/// no public result type carries an opaque scalar.
///
/// The read model lives in a sibling directory of submodules, so the whole
/// module is read: one file would hide a forbidden token in another.
const QUERY_SOURCE: &str = concat!(
    include_str!("../src/v3/episode/store/query/mod.rs"),
    include_str!("../src/v3/episode/store/query/constraints.rs"),
    include_str!("../src/v3/episode/store/query/dimensions.rs"),
    include_str!("../src/v3/episode/store/query/project.rs"),
    include_str!("../src/v3/episode/store/query/result.rs"),
    include_str!("../src/v3/episode/store/query/verdict.rs"),
);

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-constitutional-{}", Uuid::new_v4())))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn ledger(&self) -> PathBuf {
        self.0.join("episodes.v1.jsonl")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn signer(seed: u8) -> EpisodeSigner {
    EpisodeSigner::new(SigningKey::from_bytes(&[seed; 32]))
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

fn candidate(seed: u8, member_scoped: bool) -> MemoryCandidate {
    MemoryCandidate {
        class: MemoryClass::Fact,
        retention: RetentionClass::Durable,
        payload_commitment: [seed; 32],
        payload_bytes: 64,
        dimension: None,
        embedding_version: None,
        member_scoped,
        supersedes: None,
        forgets: None,
    }
}

fn forget(target: [u8; 32]) -> MemoryCandidate {
    MemoryCandidate {
        payload_commitment: [0; 32],
        payload_bytes: 0,
        forgets: Some(target),
        ..candidate(1, false)
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

fn quarantine(target: [u8; 32]) -> MemoryEdge {
    MemoryEdge {
        kind: MemoryEdgeKind::Quarantines,
        target,
        counterpart: None,
        reason: EdgeReason::SourceUntrusted,
    }
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

fn result_for_with(
    store: &EpisodeStore,
    resolver: KeyResolver,
    digest: [u8; 32],
) -> ConstitutionalResult {
    query_of(store, &base_query().with_key_resolver(resolver))
        .results
        .into_iter()
        .find(|result| result.episode_digest == digest)
        .expect("the record is listed")
}

/// A stable rendering of one dimension, for comparing two results.
fn dimension_text(result: &ConstitutionalResult, dimension: EvidenceDimension) -> String {
    let dimensions = &result.dimensions;
    match dimension {
        EvidenceDimension::Cryptographic => format!("{:?}", dimensions.cryptographic),
        EvidenceDimension::Source => format!("{:?}", dimensions.source),
        EvidenceDimension::Calibration => format!("{:?}", dimensions.calibration),
        EvidenceDimension::Outcome => format!("{:?}", dimensions.outcome),
        EvidenceDimension::Compatibility => format!("{:?}", dimensions.compatibility),
        EvidenceDimension::Regime => format!("{:?}", dimensions.regime),
        EvidenceDimension::Lifecycle => format!("{:?}", dimensions.lifecycle),
        EvidenceDimension::Confidence => format!("{:?}", dimensions.confidence),
    }
}

/// Flip one byte of the stored signature, leaving the digest and the
/// envelope untouched, so only the signature check can catch it.
fn tamper_signature(scratch: &Scratch) {
    let original = fs::read_to_string(scratch.ledger()).expect("ledger");
    let start =
        original.find("\"signature\":[").expect("signature array") + "\"signature\":[".len();
    let end = start + original[start..].find([',', ']']).expect("first byte");
    let first: u8 = original[start..end].parse().expect("byte");
    let tampered = format!("{}{}{}", &original[..start], first ^ 1, &original[end..]);
    fs::write(scratch.ledger(), &tampered).expect("synthetic tamper");
}

#[test]
fn v3_constitutional_query_reports_eight_dimensions_separately() {
    let scratch = Scratch::new();
    let digest = {
        let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
        append(
            &mut store,
            memory("request_1", "memory_1", candidate(1, false)),
        )
    };
    let store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("replay");
    let results = results_of(&store);

    assert_eq!(results.results.len(), 1);
    let result = &results.results[0];
    assert_eq!(result.episode_digest, digest);
    assert_eq!(result.disposition, Disposition::Accepted);
    assert_eq!(result.receipt.episode_digest, digest);
    assert!(result.receipt.redacted);

    // Every dimension is present, and every dimension has exactly one
    // reason: the explanation is per dimension, never one combined entry.
    assert_eq!(result.explanation.entries.len(), 8);
    let dimensions: Vec<EvidenceDimension> = result
        .explanation
        .entries
        .iter()
        .map(|(dimension, _)| *dimension)
        .collect();
    assert_eq!(dimensions, EvidenceDimension::all().to_vec());
    assert_eq!(
        result.dimensions.cryptographic.signature,
        SignatureEvidence::Unsigned
    );
    assert_eq!(
        result.dimensions.cryptographic.key_status,
        SignerKeyStatus::Absent
    );

    // The dimensions the ledger holds no evidence for say so, rather than
    // reporting a zero or a default that could pass for a measurement.
    assert_eq!(result.dimensions.calibration, CalibrationEvidence::Unknown);
    assert_eq!(result.dimensions.regime, RegimeEvidence::Unknown);
    assert_eq!(
        result.dimensions.source.reputation,
        ReputationEvidence::Unknown
    );
    assert_eq!(result.dimensions.lifecycle.age, AgeEvidence::Unknown);
    assert_eq!(
        result.dimensions.confidence.uncertainty,
        UncertaintyEvidence::Unknown
    );

    // The counts are per disposition, and nothing else.
    assert_eq!(
        results.disposition_counts.get(&Disposition::Accepted),
        Some(&1)
    );
    assert_eq!(results.disposition_counts.len(), 1);
}

#[test]
fn v3_constitutional_query_mutating_one_dimension_in_isolation() {
    for dimension in EvidenceDimension::all() {
        let (baseline, mutated) = pair(dimension);
        for other in EvidenceDimension::all() {
            if other == dimension {
                continue;
            }
            assert_eq!(
                dimension_text(&baseline, other),
                dimension_text(&mutated, other),
                "{dimension:?} must not move {other:?}"
            );
        }
        match dimension {
            // The ledger holds no calibration record and no task-regime
            // field, so these two dimensions read Unknown for every record
            // and no write can move them. That is the constitutional promise:
            // a dimension with no evidence says so instead of inventing one.
            EvidenceDimension::Calibration | EvidenceDimension::Regime => {
                assert_eq!(dimension_text(&baseline, dimension), "Unknown");
                assert_eq!(dimension_text(&mutated, dimension), "Unknown");
                assert_eq!(
                    baseline.disposition, mutated.disposition,
                    "{dimension:?} holds no evidence, so it cannot move the verdict"
                );
            }
            _ => {
                assert_ne!(
                    dimension_text(&baseline, dimension),
                    dimension_text(&mutated, dimension),
                    "{dimension:?} did not move"
                );
                assert_ne!(
                    baseline.disposition, mutated.disposition,
                    "{dimension:?} did not move the disposition"
                );
            }
        }
    }
}

/// The baseline and the mutated result for one dimension.
fn pair(dimension: EvidenceDimension) -> (ConstitutionalResult, ConstitutionalResult) {
    match dimension {
        EvidenceDimension::Cryptographic => cryptographic_pair(),
        EvidenceDimension::Source => source_pair(),
        EvidenceDimension::Calibration | EvidenceDimension::Regime => unmoved_pair(),
        EvidenceDimension::Outcome => outcome_pair(),
        EvidenceDimension::Compatibility => compatibility_pair(),
        EvidenceDimension::Lifecycle => lifecycle_pair(),
        EvidenceDimension::Confidence => confidence_pair(),
    }
}

/// The signature stops verifying: the record is rejected, not dropped.
fn cryptographic_pair() -> (ConstitutionalResult, ConstitutionalResult) {
    let scratch = Scratch::new();
    let key_id = signer(1).key_id().clone();
    let key = signer(1).verifying_key();
    let resolver = KeyResolver::new(move |requested: &_| (requested == &key_id).then_some(key));
    let digest = {
        let mut store =
            EpisodeStore::open_with_signer(scratch.path(), policy("policy_v1"), signer(1))
                .expect("open");
        append(
            &mut store,
            memory("request_1", "memory_1", candidate(1, false)),
        )
    };
    let baseline = {
        let store = EpisodeStore::open_with_signer(scratch.path(), policy("policy_v1"), signer(1))
            .expect("replay");
        result_for_with(&store, resolver.clone(), digest)
    };
    tamper_signature(&scratch);
    // The writer that owns the key refuses the ledger; a reader that holds
    // no key opens it and reports the forgery.
    assert!(matches!(
        EpisodeStore::open_with_signer(scratch.path(), policy("policy_v1"), signer(1)),
        Err(EpisodeStoreError::Corrupt)
    ));
    let store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("unsigned replay");
    let mutated = result_for_with(&store, resolver, digest);
    (baseline, mutated)
}

/// The record was proposed rather than observed.
fn source_pair() -> (ConstitutionalResult, ConstitutionalResult) {
    let observed = Scratch::new();
    let proposed = Scratch::new();
    let mut proposed_write = memory("request_1", "memory_1", candidate(1, false));
    proposed_write.source_type = EpisodeSource::Proposal;
    let observed_digest = {
        let mut store = EpisodeStore::open(observed.path(), policy("policy_v1")).expect("open");
        append(
            &mut store,
            memory("request_1", "memory_1", candidate(1, false)),
        )
    };
    let proposed_digest = {
        let mut store = EpisodeStore::open(proposed.path(), policy("policy_v1")).expect("open");
        append(&mut store, proposed_write)
    };
    let observed_store = EpisodeStore::open(observed.path(), policy("policy_v1")).expect("replay");
    let proposed_store = EpisodeStore::open(proposed.path(), policy("policy_v1")).expect("replay");
    (
        result_for(&observed_store, observed_digest),
        result_for(&proposed_store, proposed_digest),
    )
}

/// Nothing the ledger holds can move the calibration or the regime
/// dimension, so the mutation changes only what no dimension reads.
fn unmoved_pair() -> (ConstitutionalResult, ConstitutionalResult) {
    let first = Scratch::new();
    let second = Scratch::new();
    let first_digest = {
        let mut store = EpisodeStore::open(first.path(), policy("policy_v1")).expect("open");
        append(
            &mut store,
            memory("request_1", "memory_1", candidate(1, false)),
        )
    };
    let second_digest = {
        let mut store = EpisodeStore::open(second.path(), policy("policy_v1")).expect("open");
        append(
            &mut store,
            memory("request_1", "memory_1", candidate(2, false)),
        )
    };
    let first_store = EpisodeStore::open(first.path(), policy("policy_v1")).expect("replay");
    let second_store = EpisodeStore::open(second.path(), policy("policy_v1")).expect("replay");
    (
        result_for(&first_store, first_digest),
        result_for(&second_store, second_digest),
    )
}

/// A second candidate contradicts the first.
fn outcome_pair() -> (ConstitutionalResult, ConstitutionalResult) {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let first = append(
        &mut store,
        memory("request_1", "memory_1", candidate(1, false)),
    );
    let second = append(
        &mut store,
        memory("request_2", "memory_2", candidate(2, false)),
    );
    let baseline = result_for(&store, first);
    append(
        &mut store,
        edge_write("request_3", contradiction(first, second)),
    );
    let mutated = result_for(&store, first);
    (baseline, mutated)
}

/// The store is opened under another policy version: the record's binding is
/// stale.
fn compatibility_pair() -> (ConstitutionalResult, ConstitutionalResult) {
    let scratch = Scratch::new();
    let digest = {
        let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
        append(
            &mut store,
            memory("request_1", "memory_1", candidate(1, false)),
        )
    };
    let baseline = {
        let store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("replay");
        result_for(&store, digest)
    };
    let mutated = {
        let store = EpisodeStore::open(scratch.path(), policy("policy_v2")).expect("replay");
        result_for(&store, digest)
    };
    (baseline, mutated)
}

/// A later candidate supersedes the first.
fn lifecycle_pair() -> (ConstitutionalResult, ConstitutionalResult) {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let first = append(
        &mut store,
        memory("request_1", "memory_1", candidate(1, false)),
    );
    let baseline = result_for(&store, first);
    append(
        &mut store,
        memory(
            "request_2",
            "memory_2",
            MemoryCandidate {
                supersedes: Some(first),
                ..candidate(2, false)
            },
        ),
    );
    let mutated = result_for(&store, first);
    (baseline, mutated)
}

/// The record is bound at a different evidence maturity.
fn confidence_pair() -> (ConstitutionalResult, ConstitutionalResult) {
    let specified = Scratch::new();
    let exercised = Scratch::new();
    let mut specified_write = memory("request_1", "memory_1", candidate(1, false));
    specified_write.evidence_level = EvidenceLevel::C0;
    let mut exercised_write = memory("request_1", "memory_1", candidate(1, false));
    exercised_write.evidence_level = EvidenceLevel::C1;
    let specified_digest = {
        let mut store = EpisodeStore::open(specified.path(), policy("policy_v1")).expect("open");
        append(&mut store, specified_write)
    };
    let exercised_digest = {
        let mut store = EpisodeStore::open(exercised.path(), policy("policy_v1")).expect("open");
        append(&mut store, exercised_write)
    };
    let specified_store =
        EpisodeStore::open(specified.path(), policy("policy_v1")).expect("replay");
    let exercised_store =
        EpisodeStore::open(exercised.path(), policy("policy_v1")).expect("replay");
    (
        result_for(&specified_store, specified_digest),
        result_for(&exercised_store, exercised_digest),
    )
}

#[test]
fn v3_constitutional_query_invalid_signature_is_rejected() {
    // A signature that does not verify: rejected, and still listed.
    let scratch = Scratch::new();
    let key_id = signer(1).key_id().clone();
    let key = signer(1).verifying_key();
    let resolver = KeyResolver::new(move |requested: &_| (requested == &key_id).then_some(key));
    let digest = {
        let mut store =
            EpisodeStore::open_with_signer(scratch.path(), policy("policy_v1"), signer(1))
                .expect("open");
        append(
            &mut store,
            memory("request_1", "memory_1", candidate(1, false)),
        )
    };
    tamper_signature(&scratch);
    let store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("unsigned replay");
    let results = query_of(&store, &base_query().with_key_resolver(resolver));
    assert_eq!(
        results.results.len(),
        1,
        "a rejected record is still listed"
    );
    let result = &results.results[0];
    assert_eq!(result.episode_digest, digest);
    assert_eq!(result.disposition, Disposition::Rejected);
    assert_eq!(
        result.dimensions.cryptographic.signature,
        SignatureEvidence::Invalid
    );
    assert_eq!(
        result.dimensions.cryptographic.key_status,
        SignerKeyStatus::Resolved
    );
    assert_eq!(
        results.disposition_counts.get(&Disposition::Rejected),
        Some(&1)
    );

    // A signature under a key the verifier cannot resolve is unverifiable,
    // never invalid: missing evidence is not negative evidence.
    let other = Scratch::new();
    let digest = {
        let mut store =
            EpisodeStore::open_with_signer(other.path(), policy("policy_v1"), signer(1))
                .expect("open");
        append(
            &mut store,
            memory("request_1", "memory_1", candidate(1, false)),
        )
    };
    let store = EpisodeStore::open_with_signer(other.path(), policy("policy_v1"), signer(2))
        .expect("replay");
    let results = results_of(&store);
    let result = results
        .results
        .iter()
        .find(|result| result.episode_digest == digest)
        .expect("the record is listed");
    assert_eq!(
        result.dimensions.cryptographic.signature,
        SignatureEvidence::Unverifiable
    );
    assert_eq!(
        result.dimensions.cryptographic.key_status,
        SignerKeyStatus::Unresolved
    );
    assert_ne!(
        result.disposition,
        Disposition::Rejected,
        "an unresolvable key is not a forgery"
    );
    assert_eq!(result.disposition, Disposition::Unknown);
}

#[test]
fn v3_constitutional_query_quarantined_candidate_is_returned() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let digest = append(
        &mut store,
        memory("request_1", "memory_1", candidate(1, false)),
    );
    let edge = append(&mut store, edge_write("request_2", quarantine(digest)));

    let results = results_of(&store);
    assert_eq!(results.results.len(), 1, "the edge episode is not a memory");
    let result = &results.results[0];
    assert_eq!(result.disposition, Disposition::Quarantined);
    assert_eq!(
        result.dimensions.lifecycle.open_quarantine,
        Some(edge),
        "the lifecycle dimension names the open quarantine"
    );
    assert_eq!(
        results.disposition_counts.get(&Disposition::Quarantined),
        Some(&1)
    );

    // The caller may exclude it, but the store never hides it on its own.
    let mut excluding = base_query();
    excluding.constraints.include_quarantined = false;
    assert_eq!(query_of(&store, &excluding).results, Vec::new());
}

#[test]
fn v3_constitutional_query_contradicted_candidate_keeps_its_counterpart_visible() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let first = append(
        &mut store,
        memory("request_1", "memory_1", candidate(1, false)),
    );
    let second = append(
        &mut store,
        memory("request_2", "memory_2", candidate(2, false)),
    );
    append(
        &mut store,
        edge_write("request_3", contradiction(first, second)),
    );

    let results = results_of(&store);
    assert_eq!(results.results.len(), 2, "both candidates stay visible");
    for result in &results.results {
        assert_eq!(result.disposition, Disposition::Contradicted);
        assert_eq!(result.dimensions.outcome, OutcomeEvidence::Contradicted);
        assert_eq!(result.lineage.open_contradictions.len(), 1);
    }
    let first_result = results
        .results
        .iter()
        .find(|result| result.episode_digest == first)
        .expect("the first candidate is listed");
    let second_result = results
        .results
        .iter()
        .find(|result| result.episode_digest == second)
        .expect("the second candidate is listed");
    assert_eq!(
        first_result.lineage.open_contradictions[0].counterpart,
        second
    );
    assert_eq!(
        second_result.lineage.open_contradictions[0].counterpart,
        first
    );
    assert_eq!(
        results.disposition_counts.get(&Disposition::Contradicted),
        Some(&2)
    );
}

#[test]
fn v3_constitutional_query_forgotten_candidate_is_still_listed() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let digest = append(
        &mut store,
        memory("request_1", "memory_1", candidate(1, false)),
    );
    let tombstone = append(&mut store, memory("request_2", "memory_2", forget(digest)));

    // By default a tombstoned memory is not surfaced: the content is gone.
    let results = results_of(&store);
    assert_eq!(results.results.len(), 1);
    assert_eq!(results.results[0].episode_digest, tombstone);
    assert!(results.results[0].dimensions.lifecycle.tombstoned);

    // The caller that asks for the erased memory still sees it, with the
    // lifecycle dimension reporting the erasure and the disposition saying
    // that it must not be used.
    let mut including = base_query();
    including.constraints.include_forgotten = true;
    let results = query_of(&store, &including);
    assert_eq!(results.results.len(), 2);
    let forgotten = results
        .results
        .iter()
        .find(|result| result.episode_digest == digest)
        .expect("the forgotten candidate is listed");
    assert!(forgotten.dimensions.lifecycle.forgotten);
    assert_eq!(forgotten.disposition, Disposition::Rejected);
    assert_eq!(
        results.disposition_counts.get(&Disposition::Rejected),
        Some(&1)
    );
}

#[test]
fn v3_constitutional_query_evidence_floor_and_edge_filters() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let mut specified = memory("request_1", "memory_1", candidate(1, false));
    specified.evidence_level = EvidenceLevel::C0;
    let specified_digest = append(&mut store, specified);
    let exercised_digest = append(
        &mut store,
        memory("request_2", "memory_2", candidate(2, false)),
    );
    let second = append(
        &mut store,
        memory("request_3", "memory_3", candidate(3, false)),
    );
    append(
        &mut store,
        edge_write("request_4", contradiction(exercised_digest, second)),
    );

    // The evidence floor excludes the records below it.
    let mut floored = base_query();
    floored.constraints.min_evidence_level = Some(EvidenceLevel::C1);
    let digests: Vec<[u8; 32]> = query_of(&store, &floored)
        .results
        .iter()
        .map(|result| result.episode_digest)
        .collect();
    assert!(!digests.contains(&specified_digest));
    assert!(digests.contains(&exercised_digest));

    // The contradiction filter is the caller's, not the store's. Both sides
    // of an open contradiction are contradicted, so both are excluded.
    let mut excluding = base_query();
    excluding.constraints.include_contradicted = false;
    let digests: Vec<[u8; 32]> = query_of(&store, &excluding)
        .results
        .iter()
        .map(|result| result.episode_digest)
        .collect();
    assert!(!digests.contains(&exercised_digest));
    assert!(!digests.contains(&second));
    assert!(digests.contains(&specified_digest));
}

#[test]
fn v3_constitutional_query_no_cross_guild_recall() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    let scoped = append(
        &mut store,
        memory("request_1", "memory_1", candidate(1, true)),
    );
    let shared = append(
        &mut store,
        memory("request_2", "memory_2", candidate(2, false)),
    );
    let mut foreign_write = memory("request_3", "memory_3", candidate(3, false));
    foreign_write.guild_ref = OTHER_GUILD.into();
    let foreign = append(&mut store, foreign_write);

    let results = results_of(&store);
    let digests: Vec<[u8; 32]> = results
        .results
        .iter()
        .map(|result| result.episode_digest)
        .collect();
    assert!(digests.contains(&scoped));
    assert!(digests.contains(&shared));
    assert!(
        !digests.contains(&foreign),
        "another guild's digest never crosses over"
    );
    for result in &results.results {
        assert_eq!(result.receipt.guild_ref, GUILD);
    }

    // A member scope partitions the guild's own records.
    let mut member = base_query();
    member.constraints.member_scope = Some(MemberScope::Member);
    let digests: Vec<[u8; 32]> = query_of(&store, &member)
        .results
        .iter()
        .map(|result| result.episode_digest)
        .collect();
    assert_eq!(digests, vec![scoped]);

    let mut guild = base_query();
    guild.constraints.member_scope = Some(MemberScope::Guild);
    let digests: Vec<[u8; 32]> = query_of(&store, &guild)
        .results
        .iter()
        .map(|result| result.episode_digest)
        .collect();
    assert_eq!(digests, vec![shared]);
}

#[test]
fn v3_constitutional_query_no_opaque_scalar() {
    // The structural claim, checked at the source: the constitutional query
    // declares no floating-point value and no combined number.
    for forbidden in ["f32", "f64", "score", "combined", "rank", "weight"] {
        assert!(
            !QUERY_SOURCE.contains(forbidden),
            "{forbidden} appears in the constitutional query"
        );
    }
    assert!(!QUERY_SOURCE.contains("fn combined"));
    assert!(!QUERY_SOURCE.contains("fn score"));

    // The result types are finite and hold no numeric verdict.
    assert!(std::mem::size_of::<ConstitutionalResult>() > 0);
    assert!(std::mem::size_of::<ConstitutionalResult>() < 4_096);
    assert!(std::mem::size_of::<ConstitutionalResult>().is_multiple_of(8));

    // The explanation is per dimension: one entry per dimension, each with
    // its own closed reason.
    let scratch = Scratch::new();
    let digest = {
        let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
        append(
            &mut store,
            memory("request_1", "memory_1", candidate(1, false)),
        )
    };
    let store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("replay");
    let result = result_for(&store, digest);
    assert_eq!(
        result.explanation.entries.len(),
        EvidenceDimension::all().len()
    );
    let mut seen = Vec::new();
    for (dimension, reason) in &result.explanation.entries {
        assert!(!seen.contains(dimension), "one reason per dimension");
        seen.push(*dimension);
        assert!(
            format!("{reason:?}").chars().any(char::is_uppercase),
            "a reason is a named label"
        );
    }
    assert_eq!(seen, EvidenceDimension::all().to_vec());
}

#[test]
fn v3_constitutional_query_limit_is_bounded() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
    append(
        &mut store,
        memory("request_1", "memory_1", candidate(1, false)),
    );
    append(
        &mut store,
        memory("request_2", "memory_2", candidate(2, false)),
    );
    append(
        &mut store,
        memory("request_3", "memory_3", candidate(3, false)),
    );

    let mut bounded = base_query();
    bounded.limit = 0;
    assert!(matches!(
        store.query(&bounded),
        Err(EpisodeStoreError::InvalidInput)
    ));
    let mut bounded = base_query();
    bounded.limit = STORE_LIMIT + 1;
    assert!(matches!(
        store.query(&bounded),
        Err(EpisodeStoreError::InvalidInput)
    ));
    let mut at_bound = base_query();
    at_bound.limit = STORE_LIMIT;
    assert!(store.query(&at_bound).is_ok());

    let mut limited = base_query();
    limited.limit = 2;
    let results = query_of(&store, &limited);
    assert_eq!(results.results.len(), 2);
    assert_eq!(
        results.disposition_counts.get(&Disposition::Accepted),
        Some(&2),
        "the counts follow the returned records"
    );

    // A guild reference is still bounded.
    let mut unbounded = base_query();
    unbounded.guild_ref = "Bad Guild".into();
    assert!(matches!(
        store.query(&unbounded),
        Err(EpisodeStoreError::InvalidInput)
    ));
}

#[test]
fn v3_constitutional_query_replay_parity() {
    let scratch = Scratch::new();
    let before = {
        let mut store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("open");
        let first = append(
            &mut store,
            memory("request_1", "memory_1", candidate(1, true)),
        );
        let second = append(
            &mut store,
            memory("request_2", "memory_2", candidate(2, true)),
        );
        append(&mut store, edge_write("request_3", quarantine(first)));
        append(
            &mut store,
            edge_write("request_4", contradiction(first, second)),
        );
        append(&mut store, memory("request_5", "memory_3", forget(second)));
        results_of(&store)
    };
    let store = EpisodeStore::open(scratch.path(), policy("policy_v1")).expect("replay");
    let after = results_of(&store);
    assert_eq!(after, before, "the ledger is the only state that answers");
}
