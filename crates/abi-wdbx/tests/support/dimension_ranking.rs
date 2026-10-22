//! Shared fixtures for the dimension-separated ranking tests.
//!
//! The ranking tests need a real episode ledger with a spread of evidence:
//! signed and unsigned records, a superseded record, a contradiction, a
//! quarantine, a proposal, a forgetting tombstone, a specification-only
//! record and a stale-policy record. The fixtures live here so the test file
//! stays about the ranking.

use abi_wdbx::index::ExactIndex;
use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, ConstitutionalQuery, EdgeReason, EpisodeEvent, EpisodeSigner,
    EpisodeSource, EpisodeStore, EpisodeWrite, EvidenceDimension, EvidenceLevel,
    GuildEpisodePolicy, MemoryCandidate, MemoryClass, MemoryEdge, MemoryEdgeKind, RetentionClass,
    StorePolicy,
};
use abi_wdbx::v3::ranking::{
    AdmittedScope, DimensionRanking, DimensionSignal, VectorHit, rank_constitutional_query,
};
use ed25519_dalek::SigningKey;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// The guild the fixtures write to.
pub const GUILD: &str = "guild_ref";
/// The policy version the current records are written under.
pub const CURRENT_POLICY: &str = "policy_v2";
/// The policy version one record is written under, to make it stale.
pub const STALE_POLICY: &str = "policy_v1";

/// A scratch directory that removes itself when it is dropped.
pub struct Scratch(PathBuf);

impl Scratch {
    /// Create a fresh, uniquely named scratch directory.
    #[must_use]
    pub fn new() -> Self {
        Self(std::env::temp_dir().join(format!("wdbx-dimension-ranking-{}", Uuid::new_v4())))
    }

    /// The directory path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The episode signer the fixtures use.
#[must_use]
pub fn signer(seed: u8) -> EpisodeSigner {
    EpisodeSigner::new(SigningKey::from_bytes(&[seed; 32]))
}

/// The guild's policy under one policy version.
#[must_use]
pub fn guild_policy(version: &str) -> GuildEpisodePolicy {
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

/// The store's policy under one policy version.
#[must_use]
pub fn policy(version: &str) -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([(GUILD.into(), guild_policy(version))]),
    }
}

/// The store's policy over several guilds, each under one policy version.
///
/// A store that serves several guilds is what the scope-partitioned tests
/// need: an admission set is minted by the store's own constitutional query,
/// so a test that wants one for a guild of its own needs a store that serves
/// that guild.
#[must_use]
#[allow(dead_code)]
pub fn policy_for(guilds: &[&str], version: &str) -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: guilds
            .iter()
            .map(|guild| ((*guild).to_string(), guild_policy(version)))
            .collect(),
    }
}

/// The recording identity every fixture record carries.
#[must_use]
pub fn actor(principal_id: &str, kind: ActorKind) -> ActorRef {
    ActorRef {
        principal_id: principal_id.into(),
        kind,
    }
}

/// A memory candidate with a distinct payload commitment.
#[must_use]
pub fn candidate(seed: u8) -> MemoryCandidate {
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

/// One memory-candidate write.
#[must_use]
pub fn write(
    request_id: &str,
    operation_id: &str,
    policy_version: &str,
    level: EvidenceLevel,
    source: EpisodeSource,
    candidate: MemoryCandidate,
) -> EpisodeWrite {
    write_for(
        GUILD,
        request_id,
        operation_id,
        policy_version,
        level,
        source,
        candidate,
    )
}

/// One memory-candidate write for an explicit guild.
///
/// The contract revision and digest are the store policy's own, so the write
/// is admitted by any store opened with [`policy_for`].
#[must_use]
#[allow(dead_code, clippy::too_many_arguments)]
pub fn write_for(
    guild_ref: &str,
    request_id: &str,
    operation_id: &str,
    policy_version: &str,
    level: EvidenceLevel,
    source: EpisodeSource,
    candidate: MemoryCandidate,
) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        contract_revision: 2,
        contract_digest: [9; 32],
        guild_ref: guild_ref.into(),
        consent_epoch: None,
        source_type: source,
        policy_version: policy_version.into(),
        evidence_level: level,
        event: EpisodeEvent::MemoryCandidate {
            recorded_by: actor("abbey_service", ActorKind::Service),
            candidate,
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}

/// One memory-edge write.
#[must_use]
pub fn edge_write(request_id: &str, edge: MemoryEdge) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: format!("op_{request_id}"),
        contract_revision: 2,
        contract_digest: [9; 32],
        guild_ref: GUILD.into(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: CURRENT_POLICY.into(),
        evidence_level: EvidenceLevel::C3,
        event: EpisodeEvent::MemoryEdge {
            recorded_by: actor("abbey_service", ActorKind::Service),
            edge,
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}

/// A quarantine edge.
#[must_use]
pub fn quarantine(target: [u8; 32]) -> MemoryEdge {
    MemoryEdge {
        kind: MemoryEdgeKind::Quarantines,
        target,
        counterpart: None,
        reason: EdgeReason::SourceUntrusted,
    }
}

/// A contradiction edge, in the canonical ascending order.
#[must_use]
pub fn contradiction(one: [u8; 32], other: [u8; 32]) -> MemoryEdge {
    MemoryEdge {
        kind: MemoryEdgeKind::Contradicts,
        target: one.min(other),
        counterpart: Some(one.max(other)),
        reason: EdgeReason::ConflictingObservation,
    }
}

/// Append one write and return its commitment.
pub fn append(store: &mut EpisodeStore, mut event: EpisodeWrite) -> [u8; 32] {
    let preview = store
        .preview_commitment(&event)
        .expect("valid event previews");
    event.expected_commitment = Some(preview);
    let receipt = store.propose_write(&event).expect("valid event appends");
    assert_eq!(receipt.episode_digest, preview);
    preview
}

/// A store that serves several guilds, with one candidate written per seed.
///
/// The digests are the ones the store really committed. An admission set is
/// minted by the store's own constitutional query, so a test that wants one
/// for a guild of its own needs a real ledger behind it: a hand-built answer
/// cannot become a set.
#[allow(dead_code)]
pub fn guilds(
    scratch: &Scratch,
    plan: &[(&str, &[u8])],
) -> (EpisodeStore, BTreeMap<String, Vec<[u8; 32]>>) {
    let names: Vec<&str> = plan.iter().map(|(guild, _)| *guild).collect();
    let mut store = EpisodeStore::open(scratch.path(), policy_for(&names, CURRENT_POLICY))
        .expect("the store opens");
    let mut digests: BTreeMap<String, Vec<[u8; 32]>> = BTreeMap::new();
    for (guild, seeds) in plan {
        for seed in *seeds {
            let digest = append(
                &mut store,
                write_for(
                    guild,
                    &format!("request_{seed}"),
                    &format!("memory_{seed}"),
                    CURRENT_POLICY,
                    EvidenceLevel::C1,
                    EpisodeSource::DiscordGuild,
                    candidate(*seed),
                ),
            );
            digests
                .entry((*guild).to_string())
                .or_default()
                .push(digest);
        }
    }
    (store, digests)
}

/// The admission set the store's own constitutional query mints for a scope.
///
/// The set is sealed: this is the one public way to have one, and it always
/// runs the query over the store's own ledger.
#[allow(dead_code)]
pub fn admitted_for(store: &EpisodeStore, scope: &str) -> AdmittedScope {
    store
        .admitted_scope(&ConstitutionalQuery::new(scope, 64))
        .expect("the canonical query answers")
}

/// A store with a spread of evidence: signed and unsigned records, a
/// superseded record, a contradiction, a quarantine, a proposal, a forgetting
/// tombstone, a specification-only record and a stale-policy record.
pub struct Fixture {
    /// The scratch directory the ledger lives in.
    pub scratch: Scratch,
    /// The commitment of every record, by label.
    pub digests: BTreeMap<&'static str, [u8; 32]>,
}

impl Fixture {
    /// The store the ranking reads from. It holds the signing key, so the
    /// canonical query can resolve the key the signed records name.
    pub fn store(&self) -> EpisodeStore {
        EpisodeStore::open_with_signer(self.scratch.path(), policy(CURRENT_POLICY), signer(1))
            .expect("the query store opens")
    }

    /// The commitment of one labelled record.
    #[must_use]
    pub fn digest(&self, label: &str) -> [u8; 32] {
        *self.digests.get(label).expect("the record was appended")
    }
}

/// Build the fixture's ledger.
#[must_use]
pub fn build_fixture() -> Fixture {
    let scratch = Scratch::new();
    let mut digests: BTreeMap<&'static str, [u8; 32]> = BTreeMap::new();
    append_stale_record(&scratch, &mut digests);
    append_unsigned_record(&scratch, &mut digests);
    append_signed_records(&scratch, &mut digests);
    Fixture { scratch, digests }
}

/// One record written under the older policy: it is stale and nothing else, so
/// the compatibility dimension can be read on its own.
fn append_stale_record(scratch: &Scratch, digests: &mut BTreeMap<&'static str, [u8; 32]>) {
    let mut store = EpisodeStore::open(scratch.path(), policy(STALE_POLICY)).expect("open");
    let digest = append(
        &mut store,
        write(
            "req_stale",
            "op_stale",
            STALE_POLICY,
            EvidenceLevel::C1,
            EpisodeSource::DiscordGuild,
            candidate(1),
        ),
    );
    digests.insert("stale", digest);
}

/// One current-policy record nobody signed.
fn append_unsigned_record(scratch: &Scratch, digests: &mut BTreeMap<&'static str, [u8; 32]>) {
    let mut store = EpisodeStore::open(scratch.path(), policy(CURRENT_POLICY)).expect("open");
    let digest = append(
        &mut store,
        write(
            "req_unsigned",
            "op_unsigned",
            CURRENT_POLICY,
            EvidenceLevel::C1,
            EpisodeSource::DiscordGuild,
            candidate(2),
        ),
    );
    digests.insert("unsigned", digest);
}

/// Everything else, signed by the canonical writer.
fn append_signed_records(scratch: &Scratch, digests: &mut BTreeMap<&'static str, [u8; 32]>) {
    let mut store =
        EpisodeStore::open_with_signer(scratch.path(), policy(CURRENT_POLICY), signer(1))
            .expect("open");
    let signed = append(
        &mut store,
        write(
            "req_signed",
            "op_signed",
            CURRENT_POLICY,
            EvidenceLevel::C1,
            EpisodeSource::DiscordGuild,
            candidate(3),
        ),
    );
    digests.insert("signed", signed);

    let superseded = append(
        &mut store,
        write(
            "req_superseded",
            "op_superseded",
            CURRENT_POLICY,
            EvidenceLevel::C1,
            EpisodeSource::DiscordGuild,
            candidate(4),
        ),
    );
    digests.insert("superseded", superseded);
    let successor = append(
        &mut store,
        write(
            "req_successor",
            "op_successor",
            CURRENT_POLICY,
            EvidenceLevel::C1,
            EpisodeSource::DiscordGuild,
            MemoryCandidate {
                supersedes: Some(superseded),
                ..candidate(5)
            },
        ),
    );
    digests.insert("successor", successor);

    append_edge_records(&mut store, digests);
}

/// The records that carry an edge, a proposal, a tombstone or a
/// specification-only maturity.
fn append_edge_records(store: &mut EpisodeStore, digests: &mut BTreeMap<&'static str, [u8; 32]>) {
    let contradicted = append(
        store,
        write(
            "req_contradicted",
            "op_contradicted",
            CURRENT_POLICY,
            EvidenceLevel::C1,
            EpisodeSource::DiscordGuild,
            candidate(6),
        ),
    );
    digests.insert("contradicted", contradicted);
    let counterpart = append(
        store,
        write(
            "req_counterpart",
            "op_counterpart",
            CURRENT_POLICY,
            EvidenceLevel::C1,
            EpisodeSource::DiscordGuild,
            candidate(7),
        ),
    );
    digests.insert("counterpart", counterpart);
    append(
        store,
        edge_write(
            "req_contradiction",
            contradiction(contradicted, counterpart),
        ),
    );

    let quarantined = append(
        store,
        write(
            "req_quarantined",
            "op_quarantined",
            CURRENT_POLICY,
            EvidenceLevel::C1,
            EpisodeSource::DiscordGuild,
            candidate(8),
        ),
    );
    digests.insert("quarantined", quarantined);
    append(store, edge_write("req_quarantine", quarantine(quarantined)));

    let proposal = append(
        store,
        write(
            "req_proposal",
            "op_proposal",
            CURRENT_POLICY,
            EvidenceLevel::C1,
            EpisodeSource::Proposal,
            candidate(9),
        ),
    );
    digests.insert("proposal", proposal);

    let erased = append(
        store,
        write(
            "req_erased",
            "op_erased",
            CURRENT_POLICY,
            EvidenceLevel::C1,
            EpisodeSource::DiscordGuild,
            candidate(10),
        ),
    );
    digests.insert("erased", erased);
    let tombstone = append(
        store,
        write(
            "req_tombstone",
            "op_tombstone",
            CURRENT_POLICY,
            EvidenceLevel::C1,
            EpisodeSource::DiscordGuild,
            MemoryCandidate {
                payload_commitment: [0; 32],
                payload_bytes: 0,
                forgets: Some(erased),
                ..candidate(11)
            },
        ),
    );
    digests.insert("tombstone", tombstone);

    let specified = append(
        store,
        write(
            "req_specified",
            "op_specified",
            CURRENT_POLICY,
            EvidenceLevel::C0,
            EpisodeSource::DiscordGuild,
            candidate(12),
        ),
    );
    digests.insert("specified", specified);
}

/// The ranking of the fixture's store.
#[must_use]
pub fn rank_fixture(fixture: &Fixture) -> DimensionRanking {
    let store = fixture.store();
    rank_constitutional_query(
        &store,
        &abi_wdbx::v3::episode::ConstitutionalQuery::new(GUILD, 64),
    )
    .expect("the constitutional query answers")
}

/// The digests of a ranking, in order.
#[must_use]
pub fn digests_of(ranking: &DimensionRanking) -> Vec<[u8; 32]> {
    ranking
        .entries()
        .iter()
        .map(|entry| entry.result.episode_digest)
        .collect()
}

/// The signal labels of one dimension, in the ranking's order.
#[must_use]
pub fn labels_of(ranking: &DimensionRanking, dimension: EvidenceDimension) -> Vec<&'static str> {
    ranking
        .entries()
        .iter()
        .map(|entry| entry.dimension_signals.get(dimension).label())
        .collect()
}

/// The label of one record's signal on one dimension.
#[must_use]
pub fn label_of(
    ranking: &DimensionRanking,
    digest: [u8; 32],
    dimension: EvidenceDimension,
) -> &'static str {
    ranking
        .result(digest)
        .expect("the record is ranked")
        .dimension_signals
        .get(dimension)
        .label()
}

/// The lifecycle signal the canonical lifecycle evidence implies.
///
/// It mirrors the ranking's own derivation: the most severe lifecycle fact
/// the canonical query reports, in the canonical query's precedence.
#[must_use]
pub fn expected_lifecycle_signal(
    result: &abi_wdbx::v3::episode::ConstitutionalResult,
) -> DimensionSignal {
    let lifecycle = &result.dimensions.lifecycle;
    if lifecycle.forgotten {
        DimensionSignal::Present("forgotten")
    } else if lifecycle.revocation == abi_wdbx::v3::episode::RevocationEvidence::Revoked {
        DimensionSignal::Present("revoked")
    } else if lifecycle.open_quarantine.is_some() {
        DimensionSignal::Present("quarantined")
    } else if lifecycle.hold.is_held() {
        DimensionSignal::Present("held")
    } else if lifecycle.superseded_by.is_some() {
        DimensionSignal::Present("superseded")
    } else if lifecycle.tombstoned {
        DimensionSignal::Present("tombstone")
    } else {
        DimensionSignal::Present("live")
    }
}

/// The vector arm over the fixture: one vector per record, in a fixed order.
///
/// Two of the ledger's records hold no vector at all ("stale" and
/// "tombstone"), so the constitutional arm contributes records the vector arm
/// does not hold. The forgotten record holds a vector but the constitutional
/// query does not answer it, so the vector arm contributes a hit the
/// constitutional arm does not hold.
#[must_use]
pub fn vector_arm(fixture: &Fixture) -> (Vec<VectorHit>, Vec<[u8; 32]>) {
    let vectors: Vec<(&'static str, [f32; 4])> = vec![
        ("signed", [1.0, 0.0, 0.0, 0.0]),
        ("unsigned", [0.9, 0.1, 0.0, 0.0]),
        ("superseded", [0.8, 0.2, 0.0, 0.0]),
        ("successor", [0.1, 0.9, 0.0, 0.0]),
        ("contradicted", [0.0, 0.0, 1.0, 0.0]),
        ("counterpart", [0.0, 0.0, 0.9, 0.1]),
        ("quarantined", [0.0, 0.0, 0.0, 1.0]),
        ("proposal", [0.0, 0.0, 0.0, 0.9]),
        ("erased", [0.5, 0.5, 0.5, 0.5]),
        ("specified", [0.7, 0.2, 0.1, 0.0]),
    ];
    let mut index = ExactIndex::new();
    let mut digests = Vec::new();
    for (label, vector) in &vectors {
        index.put_vector(vector).expect("the vector fits the index");
        digests.push(fixture.digest(label));
    }
    let hits = index
        .search(&[1.0, 0.0, 0.0, 0.0], vectors.len())
        .expect("search");
    let arm: Vec<VectorHit> = hits
        .iter()
        .map(|hit| VectorHit {
            episode_digest: digests[usize::try_from(hit.id - 1).expect("ids start at one")],
            similarity: hit.score,
        })
        .collect();
    (arm, digests)
}

/// The public fields of a source file, by name and type.
#[must_use]
pub fn public_fields(source: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("pub ") else {
            continue;
        };
        if rest.starts_with("fn ")
            || rest.starts_with("const ")
            || rest.starts_with("enum ")
            || rest.starts_with("struct ")
            || rest.starts_with("mod ")
            || rest.starts_with("use ")
            || rest.starts_with("type ")
            || rest.starts_with("static ")
            || rest.starts_with("trait ")
            || rest.starts_with("impl ")
        {
            continue;
        }
        let Some((name, ty)) = rest.split_once(':') else {
            continue;
        };
        let name = name.trim().to_string();
        let ty = ty
            .trim()
            .trim_end_matches(',')
            .trim_end_matches('{')
            .trim()
            .to_string();
        fields.push((name, ty));
    }
    fields
}
