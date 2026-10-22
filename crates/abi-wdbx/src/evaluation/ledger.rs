//! Builds the canonical episode ledger the harness measures over.
//!
//! The harness does not simulate the ledger. It appends the frozen corpus to
//! a real [`EpisodeStore`], so every fact the constitutional arm reports is
//! read back out of a store that computed it: digests, edge state, lineage
//! and signature status all come from the ledger, never from the fixture.
//!
//! The append is split into three sessions because the store binds a policy
//! version per session and because only a store opened with a signer signs:
//!
//! 1. the stale-policy records, written under the older policy and signed
//!    under it as well, so that a stale record is stale and nothing else;
//! 2. the current-policy records that nobody signed;
//! 3. everything else, signed by the canonical writer, then the memory edges,
//!    then the forgetting tombstones.
//!
//! The ledger keeps the older records: a stale-policy episode stays in the
//! store with its own `policy_version` on its receipt, which is exactly the
//! stale reuse the retrieval program has to catch.

use crate::evaluation::corpus::{Corpus, CorpusError, RecordFixture};
use crate::v3::episode::{
    ActorKind, ActorRef, EdgeReason, EpisodeEvent, EpisodeSigner, EpisodeSource, EpisodeStore,
    EpisodeStoreError, EpisodeWrite, EvidenceLevel, GuildEpisodePolicy, MemoryCandidate,
    MemoryClass, MemoryEdge, MemoryEdgeKind, RetentionClass, StorePolicy,
};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// A scratch directory that removes itself when it is dropped.
#[derive(Debug)]
pub struct Scratch {
    path: PathBuf,
}

impl Scratch {
    /// Create a fresh, uniquely named scratch directory.
    ///
    /// The name mixes the process identifier with a monotonic counter so two
    /// runs never share a ledger. Nothing about the measurement depends on
    /// the name.
    #[must_use]
    pub fn new(prefix: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        let _ = std::fs::create_dir_all(&path);
        Self { path }
    }

    /// The directory path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// The ledger the harness measures over, and the identity maps over it.
#[derive(Debug)]
pub struct BuiltLedger {
    /// The store holding the canonical ledger.
    pub store: EpisodeStore,
    /// The directory the ledger lives in.
    pub scratch: Scratch,
    /// Episode digest of every corpus record.
    pub digests: BTreeMap<String, [u8; 32]>,
    /// The signer that signed the signed records.
    pub signer: EpisodeSigner,
}

/// Why the ledger could not be built.
#[derive(Debug)]
pub enum LedgerError {
    /// The corpus fixture was invalid.
    Corpus(CorpusError),
    /// The store refused an append the fixture described.
    Store(EpisodeStoreError),
    /// The fixture asked for something the ledger cannot hold.
    Fixture(&'static str),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Corpus(error) => write!(formatter, "{error}"),
            Self::Store(error) => write!(formatter, "{error}"),
            Self::Fixture(reason) => {
                write!(formatter, "corpus fixture cannot be replayed: {reason}")
            }
        }
    }
}

impl std::error::Error for LedgerError {}

impl From<CorpusError> for LedgerError {
    fn from(error: CorpusError) -> Self {
        Self::Corpus(error)
    }
}

impl From<EpisodeStoreError> for LedgerError {
    fn from(error: EpisodeStoreError) -> Self {
        Self::Store(error)
    }
}

fn service() -> ActorRef {
    ActorRef {
        principal_id: "synthetic_abbey_service".into(),
        kind: ActorKind::Service,
    }
}

fn evidence_level(label: &str) -> Result<EvidenceLevel, LedgerError> {
    Ok(match label {
        "C0" => EvidenceLevel::C0,
        "C1" => EvidenceLevel::C1,
        "C2" => EvidenceLevel::C2,
        "C3" => EvidenceLevel::C3,
        "C4" => EvidenceLevel::C4,
        "C5" => EvidenceLevel::C5,
        "C6" => EvidenceLevel::C6,
        "C7" => EvidenceLevel::C7,
        _ => return Err(LedgerError::Fixture("evidence level is not a closed class")),
    })
}

fn edge_reason(label: &str) -> Result<EdgeReason, LedgerError> {
    Ok(match label {
        "source_untrusted" => EdgeReason::SourceUntrusted,
        "signature_invalid" => EdgeReason::SignatureInvalid,
        "policy_violation" => EdgeReason::PolicyViolation,
        "operator_report" => EdgeReason::OperatorReport,
        "conflicting_observation" => EdgeReason::ConflictingObservation,
        "superseded_evidence" => EdgeReason::SupersededEvidence,
        "reviewed_valid" => EdgeReason::ReviewedValid,
        "reviewed_invalid" => EdgeReason::ReviewedInvalid,
        _ => return Err(LedgerError::Fixture("edge reason is not a closed class")),
    })
}

/// SHA-256 over the exact little-endian bytes of the frozen embedding.
fn payload_commitment(vector: &[f64]) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(vector.len() * 8);
    for value in vector {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    Sha256::digest(&bytes).into()
}

fn store_policy(
    corpus: &Corpus,
    tenant_policy: &str,
    revision: u64,
    digest: [u8; 32],
) -> StorePolicy {
    let guild = |policy_version: &str| GuildEpisodePolicy {
        learning_enabled: true,
        policy_version: policy_version.into(),
        token_budget: 1_000_000,
        storage_budget_bytes: 8 * 1024 * 1024,
        current_consent_epoch: None,
        retention_ttl_ms: None,
        retention_windows: None,
        authorized_principals: None,
    };
    StorePolicy {
        contract_revision: revision,
        contract_digest: digest,
        guilds: BTreeMap::from([
            (corpus.tenant().to_string(), guild(tenant_policy)),
            (
                corpus.fixture().other_tenant.clone(),
                guild(&corpus.policy().current),
            ),
        ]),
    }
}

fn candidate_write(
    corpus: &Corpus,
    record: &RecordFixture,
    revision: u64,
    digest: [u8; 32],
    supersedes: Option<[u8; 32]>,
) -> Result<EpisodeWrite, LedgerError> {
    let candidate = MemoryCandidate {
        class: MemoryClass::Embedding,
        retention: RetentionClass::Durable,
        payload_commitment: payload_commitment(&record.vector),
        payload_bytes: u64::try_from(record.vector.len() * 8).unwrap_or(0),
        dimension: u16::try_from(corpus.embedding_dimension()).ok(),
        embedding_version: Some("synthetic-embed-v1".into()),
        member_scoped: record.member_scoped,
        supersedes,
        forgets: None,
    };
    Ok(EpisodeWrite {
        request_id: format!("req-{}", record.record_id),
        operation_id: format!("op-{}", record.record_id),
        contract_revision: revision,
        contract_digest: digest,
        guild_ref: record.tenant.clone(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: record.policy_version.clone(),
        evidence_level: evidence_level(&record.evidence_level)?,
        event: EpisodeEvent::MemoryCandidate {
            recorded_by: service(),
            candidate,
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    })
}

fn tombstone_write(
    corpus: &Corpus,
    record_id: &str,
    policy_version: &str,
    level: &str,
    revision: u64,
    digest: [u8; 32],
    forgets: [u8; 32],
) -> Result<EpisodeWrite, LedgerError> {
    let candidate = MemoryCandidate {
        class: MemoryClass::Fact,
        retention: RetentionClass::Durable,
        payload_commitment: [0; 32],
        payload_bytes: 0,
        dimension: None,
        embedding_version: None,
        member_scoped: true,
        supersedes: None,
        forgets: Some(forgets),
    };
    Ok(EpisodeWrite {
        request_id: format!("req-{record_id}"),
        operation_id: format!("op-{record_id}"),
        contract_revision: revision,
        contract_digest: digest,
        guild_ref: corpus.tenant().to_string(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: policy_version.into(),
        evidence_level: evidence_level(level)?,
        event: EpisodeEvent::MemoryCandidate {
            recorded_by: service(),
            candidate,
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    })
}

fn edge_write(
    corpus: &Corpus,
    record_id: &str,
    policy_version: &str,
    level: &str,
    revision: u64,
    digest: [u8; 32],
    edge: MemoryEdge,
) -> Result<EpisodeWrite, LedgerError> {
    Ok(EpisodeWrite {
        request_id: format!("req-{record_id}"),
        operation_id: format!("op-{record_id}"),
        contract_revision: revision,
        contract_digest: digest,
        guild_ref: corpus.tenant().to_string(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: policy_version.into(),
        evidence_level: evidence_level(level)?,
        event: EpisodeEvent::MemoryEdge {
            recorded_by: service(),
            edge,
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    })
}

fn append(
    store: &mut EpisodeStore,
    mut write: EpisodeWrite,
) -> Result<[u8; 32], EpisodeStoreError> {
    let preview = store.preview_commitment(&write)?;
    write.expected_commitment = Some(preview);
    let receipt = store.propose_write(&write)?;
    Ok(receipt.episode_digest)
}

/// Build the canonical ledger described by the frozen corpus.
///
/// # Errors
///
/// Returns [`LedgerError`] when the fixture cannot be replayed into a store.
pub fn build(corpus: &Corpus) -> Result<BuiltLedger, LedgerError> {
    let fixture = corpus.fixture();
    let revision = fixture.contract.revision;
    let contract_digest = hex_digest(&fixture.contract.digest_hex)?;
    let scratch = Scratch::new("wdbx-eval-retrieval");
    // Three signers over the same seed: the two stores each take one, and
    // the harness keeps one to resolve the key id when it asks a store about
    // a signature. The key id is derived from the key, so all three agree.
    let signer = EpisodeSigner::new(SigningKey::from_bytes(&[corpus.signer_seed(); 32]));
    let signer_stale = EpisodeSigner::new(SigningKey::from_bytes(&[corpus.signer_seed(); 32]));
    let verifier = EpisodeSigner::new(SigningKey::from_bytes(&[corpus.signer_seed(); 32]));

    let mut digests: BTreeMap<String, [u8; 32]> = BTreeMap::new();

    // Session one: the records written under the older policy. They are
    // signed: a stale-policy record is stale and nothing else, so that the
    // ablation can isolate the compatibility dimension.
    {
        let policy = store_policy(corpus, &fixture.policy.stale, revision, contract_digest);
        let mut store = EpisodeStore::open_with_signer(scratch.path(), policy, signer_stale)?;
        for family in &fixture.families {
            for record in family
                .records
                .iter()
                .filter(|record| record.policy_version == fixture.policy.stale)
            {
                let write = candidate_write(corpus, record, revision, contract_digest, None)?;
                let digest = append(&mut store, write)?;
                digests.insert(record.record_id.clone(), digest);
            }
        }
    }

    // Session two: the current-policy records nobody signed.
    {
        let policy = store_policy(corpus, &fixture.policy.current, revision, contract_digest);
        let mut store = EpisodeStore::open(scratch.path(), policy)?;
        for family in &fixture.families {
            for record in family
                .records
                .iter()
                .filter(|record| !record.signed && record.policy_version == fixture.policy.current)
            {
                let write = candidate_write(corpus, record, revision, contract_digest, None)?;
                let digest = append(&mut store, write)?;
                digests.insert(record.record_id.clone(), digest);
            }
            let cross = &family.cross_tenant_record;
            if !cross.signed {
                let write = candidate_write(corpus, cross, revision, contract_digest, None)?;
                let digest = append(&mut store, write)?;
                digests.insert(cross.record_id.clone(), digest);
            }
        }
    }

    // Session three: the signed records, then the edges, then the tombstones.
    {
        let policy = store_policy(corpus, &fixture.policy.current, revision, contract_digest);
        let mut store = EpisodeStore::open_with_signer(scratch.path(), policy, signer)?;
        for family in &fixture.families {
            // A superseding candidate may only name an already admitted
            // candidate, so the plain candidates are appended first.
            for record in family.records.iter().filter(|record| {
                record.signed
                    && record.supersedes.is_none()
                    && record.policy_version == fixture.policy.current
            }) {
                let write = candidate_write(corpus, record, revision, contract_digest, None)?;
                let digest = append(&mut store, write)?;
                digests.insert(record.record_id.clone(), digest);
            }
            for record in family.records.iter().filter(|record| {
                record.signed
                    && record.supersedes.is_some()
                    && record.policy_version == fixture.policy.current
            }) {
                let supersedes = match &record.supersedes {
                    Some(target) => Some(
                        *digests
                            .get(target)
                            .ok_or(LedgerError::Fixture("supersedes target is not admitted"))?,
                    ),
                    None => None,
                };
                let write = candidate_write(corpus, record, revision, contract_digest, supersedes)?;
                let digest = append(&mut store, write)?;
                digests.insert(record.record_id.clone(), digest);
            }
            let cross = &family.cross_tenant_record;
            if cross.signed {
                let write = candidate_write(corpus, cross, revision, contract_digest, None)?;
                let digest = append(&mut store, write)?;
                digests.insert(cross.record_id.clone(), digest);
            }
        }
        for family in &fixture.families {
            for (index, edge) in family.edges.iter().enumerate() {
                let target = *digests
                    .get(&edge.target)
                    .ok_or(LedgerError::Fixture("edge target is not admitted"))?;
                let edge_kind_label = edge.kind.clone();
                let memory_edge = match edge.kind.as_str() {
                    "contradicts" => {
                        let counterpart = *digests
                            .get(edge.counterpart.as_deref().unwrap_or_default())
                            .ok_or(LedgerError::Fixture("edge counterpart is not admitted"))?;
                        // One disagreement has one encoding: the pair is
                        // stored in ascending digest order.
                        let (low, high) = if target < counterpart {
                            (target, counterpart)
                        } else {
                            (counterpart, target)
                        };
                        MemoryEdge {
                            kind: MemoryEdgeKind::Contradicts,
                            target: low,
                            counterpart: Some(high),
                            reason: edge_reason(&edge.reason)?,
                        }
                    }
                    "quarantines" => MemoryEdge {
                        kind: MemoryEdgeKind::Quarantines,
                        target,
                        counterpart: None,
                        reason: edge_reason(&edge.reason)?,
                    },
                    _ => return Err(LedgerError::Fixture("edge kind is not a closed class")),
                };
                let write = edge_write(
                    corpus,
                    &format!("{}-edge-{edge_kind_label}-{index}", family.family_id),
                    &fixture.policy.current,
                    "C3",
                    revision,
                    contract_digest,
                    memory_edge,
                )?;
                append(&mut store, write)?;
            }
        }
        for family in &fixture.families {
            for tombstone in &family.tombstones {
                let forgets = *digests
                    .get(&tombstone.forgets)
                    .ok_or(LedgerError::Fixture("tombstone target is not admitted"))?;
                let write = tombstone_write(
                    corpus,
                    &tombstone.record_id,
                    &tombstone.policy_version,
                    &tombstone.evidence_level,
                    revision,
                    contract_digest,
                    forgets,
                )?;
                append(&mut store, write)?;
            }
        }
    }

    let store = EpisodeStore::open(
        scratch.path(),
        store_policy(corpus, &fixture.policy.current, revision, contract_digest),
    )?;
    Ok(BuiltLedger {
        store,
        scratch,
        digests,
        signer: verifier,
    })
}

fn hex_digest(value: &str) -> Result<[u8; 32], LedgerError> {
    if value.len() != 64 {
        return Err(LedgerError::Fixture("contract digest is not 32 bytes"));
    }
    let mut out = [0_u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        let hex = &value[index * 2..index * 2 + 2];
        *byte = u8::from_str_radix(hex, 16)
            .map_err(|_| LedgerError::Fixture("contract digest is not hex"))?;
    }
    Ok(out)
}
