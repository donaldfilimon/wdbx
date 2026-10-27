//! Task-04 disposition matrix residual: the constitutional query cases the
//! earlier suites leave implicit.
//!
//! Requirement traceability: P04.01, at C0. E05/E06 are touched only where a
//! case below asserts that dimension. Nothing here advances a requirement or
//! a register row; these are local test observations of the shipped read
//! model ([`EpisodeStore::query`]) over synthetic scratch stores.
//!
//! # Pinned observation (Step 1 probe)
//!
//! Before these tests were written, a throwaway probe observed that
//! [`EpisodeStore::open`] *accepts* a policy that omits the guild a stored
//! record belongs to, and that a query for that guild then lists the record
//! with `CompatibilityEvidence::Unknown`, `Reason::BindingUnknown` and
//! `Disposition::Uncertain`. The test below pins that observed outcome; it
//! does not branch at runtime between "unknown" and "refused". The refusing
//! arm was not observed.
//!
//! # Existing coverage (consolidation, not new evidence)
//!
//! These cases are already exercised elsewhere and are not repeated here:
//!
//! * an unverifiable signature reads `Unknown`, never `Rejected`:
//!   `v3_constitutional_query.rs:585-618`;
//! * a foreign guild's record never crosses into another guild's answer:
//!   `v3_constitutional_query.rs:794` and
//!   `v3_constitutional_query_holds.rs:621-647`;
//! * the frozen baseline comparison over the synthetic corpus:
//!   `v3_retrieval_baseline.rs` and
//!   `tests/fixtures/retrieval-baseline/README.md`.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, CompatibilityEvidence, ConstitutionalQuery, ConstitutionalResult,
    Disposition, EpisodeEvent, EpisodeSource, EpisodeStore, EpisodeWrite, EvidenceDimension,
    EvidenceLevel, GuildEpisodePolicy, MemoryCandidate, MemoryClass, MemoryResults, Reason,
    RegimeEvidence, RetentionClass, StorePolicy,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const GUILD: &str = "guild_ref";
const OTHER_GUILD: &str = "other_guild";
/// A guild no policy names and no record belongs to.
const UNSERVED_GUILD: &str = "unserved_guild";
const CONTRACT_REVISION: u64 = 2;
const CONTRACT_DIGEST: [u8; 32] = [9; 32];

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-disposition-matrix-{}", Uuid::new_v4())))
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

/// A store policy binding the named guilds, all at one policy version.
fn policy_for(guilds: &[&str], version: &str) -> StorePolicy {
    StorePolicy {
        contract_revision: CONTRACT_REVISION,
        contract_digest: CONTRACT_DIGEST,
        guilds: guilds
            .iter()
            .map(|guild| ((*guild).to_owned(), guild_policy(version)))
            .collect::<BTreeMap<_, _>>(),
    }
}

fn policy() -> StorePolicy {
    policy_for(&[GUILD, OTHER_GUILD], "policy_v1")
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

fn memory(guild: &str, request_id: &str, candidate: MemoryCandidate) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: format!("op_{request_id}"),
        contract_revision: CONTRACT_REVISION,
        contract_digest: CONTRACT_DIGEST,
        guild_ref: guild.into(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C1,
        event: EpisodeEvent::MemoryCandidate {
            recorded_by: ActorRef {
                principal_id: "abbey_service".into(),
                kind: ActorKind::Service,
            },
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

fn query(store: &EpisodeStore, guild: &str) -> MemoryResults {
    store
        .query(&ConstitutionalQuery::new(guild, 64))
        .expect("the query answers")
}

fn only_result(results: &MemoryResults, digest: [u8; 32]) -> &ConstitutionalResult {
    assert_eq!(results.results.len(), 1, "exactly one record is listed");
    let result = &results.results[0];
    assert_eq!(result.episode_digest, digest);
    result
}

fn reason(result: &ConstitutionalResult, dimension: EvidenceDimension) -> Reason {
    result
        .explanation
        .entries
        .iter()
        .find(|(entry, _)| *entry == dimension)
        .map(|(_, reason)| *reason)
        .expect("every dimension has an explanation entry")
}

/// One record written under the default binding, then the scratch store is
/// closed so the caller can reopen it under another policy.
fn one_record_store() -> (Scratch, [u8; 32]) {
    let scratch = Scratch::new();
    let digest = {
        let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
        append(&mut store, memory(GUILD, "request_1", candidate(1)))
    };
    (scratch, digest)
}

/// Assert the stale-binding verdict for one record under one reopen policy.
fn assert_binding(
    scratch: &Scratch,
    reopen: StorePolicy,
    digest: [u8; 32],
    compatibility: CompatibilityEvidence,
    expected: Reason,
) {
    let store = EpisodeStore::open(scratch.path(), reopen).expect("reopen is accepted");
    let results = query(&store, GUILD);
    let result = only_result(&results, digest);
    assert_eq!(result.dimensions.compatibility, compatibility);
    assert_eq!(reason(result, EvidenceDimension::Compatibility), expected);
    assert_eq!(result.disposition, Disposition::Uncertain);
    assert_eq!(
        results.disposition_counts,
        BTreeMap::from([(Disposition::Uncertain, 1)])
    );
}

/// The store is reopened under another contract revision, and separately
/// under another contract digest. Only that one policy field changes, and
/// nothing is appended after the reopen (a write under the stale binding
/// would be refused by the write gate anyway). The record is retained and
/// listed as `Uncertain`, with the compatibility dimension naming the stale
/// contract binding.
#[test]
fn stale_contract_is_uncertain_with_binding_contract_stale() {
    let (scratch, digest) = one_record_store();

    // The control: the unchanged policy reads the binding as current.
    {
        let store = EpisodeStore::open(scratch.path(), policy()).expect("replay");
        let results = query(&store, GUILD);
        let result = only_result(&results, digest);
        assert_eq!(
            result.dimensions.compatibility,
            CompatibilityEvidence::Compatible
        );
        assert_eq!(
            reason(result, EvidenceDimension::Compatibility),
            Reason::BindingCurrent
        );
        assert_eq!(result.disposition, Disposition::Accepted);
    }

    let mut revision = policy();
    revision.contract_revision = CONTRACT_REVISION + 1;
    assert_binding(
        &scratch,
        revision,
        digest,
        CompatibilityEvidence::ContractStale,
        Reason::BindingContractStale,
    );

    let mut contract_digest = policy();
    contract_digest.contract_digest = [8; 32];
    assert_binding(
        &scratch,
        contract_digest,
        digest,
        CompatibilityEvidence::ContractStale,
        Reason::BindingContractStale,
    );
}

/// The store is reopened under another policy version for the record's
/// guild, with the contract binding unchanged: the reason is the stale
/// policy, not the stale contract.
#[test]
fn stale_policy_reason_is_binding_policy_stale() {
    let (scratch, digest) = one_record_store();
    assert_binding(
        &scratch,
        policy_for(&[GUILD, OTHER_GUILD], "policy_v2"),
        digest,
        CompatibilityEvidence::PolicyStale,
        Reason::BindingPolicyStale,
    );
}

/// A record whose guild the reopen policy no longer names.
///
/// Pinned observed behaviour (see the module doc): the open is accepted, and
/// the record is listed for a query of its own guild with an unknown
/// binding. Missing binding evidence is reported as `Unknown`, not as a
/// stale binding and not as a rejection, and the record stays `Uncertain`.
#[test]
fn unbound_guild_binding_reads_unknown_or_is_refused() {
    let scratch = Scratch::new();
    let (unbound, bound) = {
        let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
        let unbound = append(&mut store, memory(OTHER_GUILD, "request_1", candidate(1)));
        let bound = append(&mut store, memory(GUILD, "request_2", candidate(2)));
        (unbound, bound)
    };

    let store = EpisodeStore::open(scratch.path(), policy_for(&[GUILD], "policy_v1"))
        .expect("an open whose policy omits a stored record's guild is accepted");

    let results = query(&store, OTHER_GUILD);
    assert_eq!(results.scope, OTHER_GUILD);
    let result = only_result(&results, unbound);
    assert_eq!(
        result.dimensions.compatibility,
        CompatibilityEvidence::Unknown
    );
    assert_eq!(
        reason(result, EvidenceDimension::Compatibility),
        Reason::BindingUnknown
    );
    assert_eq!(result.disposition, Disposition::Uncertain);

    // The guild the policy still names keeps a current binding.
    let results = query(&store, GUILD);
    let result = only_result(&results, bound);
    assert_eq!(
        result.dimensions.compatibility,
        CompatibilityEvidence::Compatible
    );
    assert_eq!(result.disposition, Disposition::Accepted);
}

/// A supersedes A, B is then forgotten by a tombstone.
///
/// The superseded record A stays visible and `Uncertain`, and its lineage
/// still names B even though B's content is erased: the reference is
/// dangling by design, because the supersession is a fact about A's history
/// and the forgetting does not rewrite it. B itself is hidden by default and
/// listed as `Rejected` only when the caller asks for forgotten records.
#[test]
fn incomplete_lineage_superseder_forgotten_stays_visible() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
    let superseded = append(&mut store, memory(GUILD, "request_1", candidate(1)));
    let superseder = append(
        &mut store,
        memory(
            GUILD,
            "request_2",
            MemoryCandidate {
                supersedes: Some(superseded),
                ..candidate(2)
            },
        ),
    );
    let tombstone = append(
        &mut store,
        memory(
            GUILD,
            "request_3",
            MemoryCandidate {
                payload_commitment: [0; 32],
                payload_bytes: 0,
                forgets: Some(superseder),
                ..candidate(3)
            },
        ),
    );

    let results = query(&store, GUILD);
    let digests: Vec<[u8; 32]> = results
        .results
        .iter()
        .map(|result| result.episode_digest)
        .collect();
    assert_eq!(digests, vec![superseded, tombstone]);

    let kept = &results.results[0];
    assert_eq!(kept.lineage.superseded_by, Some(superseder));
    assert_eq!(kept.dimensions.lifecycle.superseded_by, Some(superseder));
    assert!(!kept.dimensions.lifecycle.forgotten);
    assert_eq!(
        reason(kept, EvidenceDimension::Lifecycle),
        Reason::Superseded
    );
    assert_eq!(kept.disposition, Disposition::Uncertain);

    let mut including = ConstitutionalQuery::new(GUILD, 64);
    including.constraints.include_forgotten = true;
    let results = store.query(&including).expect("the query answers");
    let erased = results
        .results
        .iter()
        .find(|result| result.episode_digest == superseder)
        .expect("the forgotten superseder is listed on request");
    assert!(erased.dimensions.lifecycle.forgotten);
    assert_eq!(erased.disposition, Disposition::Rejected);
    assert_eq!(
        reason(erased, EvidenceDimension::Lifecycle),
        Reason::Forgotten
    );
}

/// The "wrong regime" case, by proxy.
///
/// The ledger holds no task-regime field, so no record can be in a measured
/// wrong regime. The proxy used here is the C0 maturity binding: a record
/// written at specification-only maturity is `Uncertain`, and its regime
/// dimension still reads `Unknown` with `Reason::NoRegime`. This pins that
/// the proxy moves the verdict through the confidence dimension, never by
/// inventing a regime measurement.
#[test]
fn wrong_regime_proxy_reason_is_no_regime() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
    let mut specified = memory(GUILD, "request_1", candidate(1));
    specified.evidence_level = EvidenceLevel::C0;
    let proxy = append(&mut store, specified);
    let exercised = append(&mut store, memory(GUILD, "request_2", candidate(2)));

    let results = query(&store, GUILD);
    for result in &results.results {
        assert_eq!(result.dimensions.regime, RegimeEvidence::Unknown);
        assert_eq!(reason(result, EvidenceDimension::Regime), Reason::NoRegime);
    }
    let by_digest = |digest| {
        results
            .results
            .iter()
            .find(|result| result.episode_digest == digest)
            .expect("the record is listed")
    };
    let proxy_result = by_digest(proxy);
    assert_eq!(
        proxy_result.dimensions.confidence.maturity,
        EvidenceLevel::C0
    );
    assert_eq!(proxy_result.disposition, Disposition::Uncertain);
    assert_eq!(
        reason(proxy_result, EvidenceDimension::Lifecycle),
        Reason::Live
    );
    assert_eq!(by_digest(exercised).disposition, Disposition::Accepted);
}

/// The answer carries the scope it was asked for, and a guild that no
/// policy names and no record belongs to is answered, not refused: an empty
/// answer whose scope still names the queried guild.
#[test]
fn answer_scope_equals_queried_guild_and_unserved_guild_is_pinned() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy()).expect("open");
    let own = append(&mut store, memory(GUILD, "request_1", candidate(1)));
    let foreign = append(&mut store, memory(OTHER_GUILD, "request_2", candidate(2)));

    let results = query(&store, GUILD);
    assert_eq!(results.scope, GUILD);
    only_result(&results, own);

    let results = query(&store, OTHER_GUILD);
    assert_eq!(results.scope, OTHER_GUILD);
    only_result(&results, foreign);

    let unserved = query(&store, UNSERVED_GUILD);
    assert_eq!(unserved.scope, UNSERVED_GUILD);
    assert!(unserved.results.is_empty());
    assert!(unserved.disposition_counts.is_empty());
}
