//! The frozen corpus fixture: schema, loader and ground truth.
//!
//! The fixture is a JSON document under `tests/fixtures/retrieval-baseline/`.
//! It is the freeze: the harness never regenerates it, and the expected
//! metrics next to it are pinned against exactly these bytes.
//!
//! The fixture carries three different kinds of information and they must not
//! be confused:
//!
//! * **Corpus inputs** — the embedding vectors, the query vectors, the
//!   tenant, the policy versions and the candidate count. Both arms receive
//!   exactly these.
//! * **Ledger shape** — which record supersedes which, which record forgets
//!   which, and which edges exist. The harness replays these into a real
//!   [`crate::v3::episode::EpisodeStore`] so the constitutional arm reads
//!   state the store computed, not state the fixture asserts.
//! * **Ground truth** — the hand-authored expected disposition of every
//!   record. The constitutional arm never reads it; it is used only to
//!   score the result afterwards.

use crate::v3::episode::EvidenceLevel;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// Schema identifier of the frozen corpus fixture.
pub const CORPUS_SCHEMA: &str = "wdbx.evaluation.corpus.v1";

/// A frozen corpus that could not be loaded or is internally inconsistent.
#[derive(Debug)]
pub enum CorpusError {
    /// The fixture file could not be read.
    Io,
    /// The fixture is not valid JSON, or not the expected schema.
    Schema,
    /// The fixture is internally inconsistent.
    Inconsistent(&'static str),
}

impl fmt::Display for CorpusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io => formatter.write_str("corpus fixture could not be read"),
            Self::Schema => formatter.write_str("corpus fixture schema is not the frozen one"),
            Self::Inconsistent(reason) => {
                write!(formatter, "corpus fixture is inconsistent: {reason}")
            }
        }
    }
}

impl std::error::Error for CorpusError {}

/// The frozen corpus as it appears on disk.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CorpusFixture {
    /// Schema identifier; must equal [`CORPUS_SCHEMA`].
    pub schema: String,
    /// Stable corpus identifier.
    pub corpus_id: String,
    /// The tenant the query runs in.
    pub tenant: String,
    /// A second tenant, used only to prove the shared filter works.
    pub other_tenant: String,
    /// Width of every embedding in the fixture.
    pub embedding_dimension: usize,
    /// Candidates handed to each ranker before the shared filter.
    pub candidate_count: usize,
    /// The ABI contract binding every episode.
    pub contract: ContractFixture,
    /// The policy versions the corpus spans.
    pub policy: PolicyFixture,
    /// The retrieval query parameters shared by every family.
    pub query: QueryFixture,
    /// Seed of the deterministic Ed25519 episode-signing key.
    pub signer_seed: u8,
    /// The query families.
    pub families: Vec<FamilyFixture>,
}

/// The ABI contract revision and corpus digest the episodes bind to.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContractFixture {
    /// Expected contract revision.
    pub revision: u64,
    /// Expected contract digest, lowercase hex.
    pub digest_hex: String,
}

/// The current and the stale policy version.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PolicyFixture {
    /// The policy version the query expects.
    pub current: String,
    /// The older policy version some records were written under.
    pub stale: String,
}

/// Query parameters shared by every family.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct QueryFixture {
    /// The policy version the query demands.
    pub policy_version: String,
    /// The lowest evidence maturity the query accepts.
    pub min_evidence_level: String,
    /// The scope the query is bound to.
    pub scope: String,
}

/// One query family: a query vector and the records around it.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FamilyFixture {
    /// Stable family identifier.
    pub family_id: String,
    /// The hand-authored template the family follows.
    pub template: String,
    /// The hand-authored level order the family follows.
    pub level_order: String,
    /// The query vector.
    pub query_vector: Vec<f64>,
    /// The records of this tenant in this family.
    pub records: Vec<RecordFixture>,
    /// The memory edges of this family.
    pub edges: Vec<EdgeFixture>,
    /// The forgetting tombstones of this family.
    pub tombstones: Vec<TombstoneFixture>,
    /// A record of the other tenant, placed closest to the query.
    pub cross_tenant_record: RecordFixture,
}

/// One memory record of the corpus.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecordFixture {
    /// Stable record identifier.
    pub record_id: String,
    /// The hand-authored role of the record.
    pub role: String,
    /// The frozen embedding.
    pub vector: Vec<f64>,
    /// The tenant the record belongs to.
    pub tenant: String,
    /// The policy version the record was written under.
    pub policy_version: String,
    /// The committed evidence maturity.
    pub evidence_level: String,
    /// Whether the canonical writer signed the record.
    pub signed: bool,
    /// Whether the record is guild-plus-user isolated.
    pub member_scoped: bool,
    /// The record this one supersedes, if any.
    pub supersedes: Option<String>,
    /// The hand-authored expected disposition.
    pub expected_disposition: String,
}

/// One memory edge of the corpus.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EdgeFixture {
    /// `contradicts` or `quarantines`.
    pub kind: String,
    /// The record the edge names.
    pub target: String,
    /// The other side of a contradiction.
    pub counterpart: Option<String>,
    /// The content-free reason.
    pub reason: String,
}

/// One forgetting tombstone of the corpus.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TombstoneFixture {
    /// Stable identifier of the tombstone.
    pub record_id: String,
    /// The record the tombstone erases.
    pub forgets: String,
    /// The policy version the tombstone was written under.
    pub policy_version: String,
    /// The committed evidence maturity.
    pub evidence_level: String,
    /// Whether the canonical writer signed the tombstone.
    pub signed: bool,
}

/// The hand-authored expected disposition of a record.
///
/// The labels are the canonical query's own dispositions, so the corpus
/// states what the constitutional query is expected to answer for each
/// record. This is the ground truth the harness scores against; the
/// constitutional arm never reads it.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedDisposition {
    /// The query accepts the record.
    Accepted,
    /// The query retains the record and reports it as uncertain.
    Uncertain,
    /// An open contradiction names the record.
    Contradicted,
    /// An open quarantine names the record.
    Quarantined,
    /// The record must not be used.
    Rejected,
    /// The query holds too little evidence to judge the record.
    Unknown,
    /// The record is not a candidate at all.
    FilteredOut,
}

impl ExpectedDisposition {
    /// Stable wire label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Uncertain => "uncertain",
            Self::Contradicted => "contradicted",
            Self::Quarantined => "quarantined",
            Self::Rejected => "rejected",
            Self::Unknown => "unknown",
            Self::FilteredOut => "filtered_out",
        }
    }

    /// Parse a stored label.
    #[must_use]
    pub fn parse(label: &str) -> Option<Self> {
        match label {
            "accepted" => Some(Self::Accepted),
            "uncertain" => Some(Self::Uncertain),
            "contradicted" => Some(Self::Contradicted),
            "quarantined" => Some(Self::Quarantined),
            "rejected" => Some(Self::Rejected),
            "unknown" => Some(Self::Unknown),
            "filtered_out" => Some(Self::FilteredOut),
            _ => None,
        }
    }
}

/// Parse a committed evidence maturity label.
///
/// The label set is closed. The corpus loader refuses a fixture that uses
/// anything else.
#[must_use]
pub fn parse_evidence_level(label: &str) -> Option<EvidenceLevel> {
    match label {
        "C0" => Some(EvidenceLevel::C0),
        "C1" => Some(EvidenceLevel::C1),
        "C2" => Some(EvidenceLevel::C2),
        "C3" => Some(EvidenceLevel::C3),
        "C4" => Some(EvidenceLevel::C4),
        "C5" => Some(EvidenceLevel::C5),
        "C6" => Some(EvidenceLevel::C6),
        "C7" => Some(EvidenceLevel::C7),
        _ => None,
    }
}

/// One record of the corpus, indexed by its identifier.
#[derive(Clone, Debug)]
pub struct CorpusRecord {
    /// The record.
    pub record: RecordFixture,
    /// The family the record belongs to.
    pub family_id: String,
}

/// A validated, frozen corpus.
#[derive(Clone, Debug)]
pub struct Corpus {
    fixture: CorpusFixture,
    by_id: BTreeMap<String, CorpusRecord>,
}

impl Corpus {
    /// Load and validate the frozen corpus from disk.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError`] when the file cannot be read, is not the
    /// frozen schema, or is internally inconsistent.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, CorpusError> {
        let bytes = std::fs::read(path.as_ref()).map_err(|_| CorpusError::Io)?;
        let fixture: CorpusFixture =
            serde_json::from_slice(&bytes).map_err(|_| CorpusError::Schema)?;
        Self::from_fixture(fixture)
    }

    /// Validate a parsed fixture.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError`] when the fixture is not the frozen schema or
    /// is internally inconsistent.
    pub fn from_fixture(fixture: CorpusFixture) -> Result<Self, CorpusError> {
        if fixture.schema != CORPUS_SCHEMA {
            return Err(CorpusError::Schema);
        }
        if fixture.embedding_dimension == 0
            || fixture.candidate_count == 0
            || fixture.families.is_empty()
            || fixture.tenant == fixture.other_tenant
        {
            return Err(CorpusError::Inconsistent("header"));
        }
        if fixture.contract.revision == 0 || fixture.contract.digest_hex.len() != 64 {
            return Err(CorpusError::Inconsistent("contract"));
        }

        let mut by_id: BTreeMap<String, CorpusRecord> = BTreeMap::new();
        for family in &fixture.families {
            if family.query_vector.len() != fixture.embedding_dimension {
                return Err(CorpusError::Inconsistent("query vector width"));
            }
            let mut members = family.records.clone();
            members.push(family.cross_tenant_record.clone());
            for record in &members {
                if record.vector.len() != fixture.embedding_dimension {
                    return Err(CorpusError::Inconsistent("embedding width"));
                }
                if parse_evidence_level(&record.evidence_level).is_none() {
                    return Err(CorpusError::Inconsistent("evidence maturity"));
                }
                if ExpectedDisposition::parse(&record.expected_disposition).is_none() {
                    return Err(CorpusError::Inconsistent("expected disposition"));
                }
                if by_id
                    .insert(
                        record.record_id.clone(),
                        CorpusRecord {
                            record: record.clone(),
                            family_id: family.family_id.clone(),
                        },
                    )
                    .is_some()
                {
                    return Err(CorpusError::Inconsistent("duplicate record id"));
                }
            }
            for record in &family.records {
                if let Some(target) = &record.supersedes
                    && !family
                        .records
                        .iter()
                        .any(|other| &other.record_id == target)
                {
                    return Err(CorpusError::Inconsistent("supersedes target"));
                }
            }
            for edge in &family.edges {
                if !family
                    .records
                    .iter()
                    .any(|other| other.record_id == edge.target)
                {
                    return Err(CorpusError::Inconsistent("edge target"));
                }
                if let Some(counterpart) = &edge.counterpart
                    && !family
                        .records
                        .iter()
                        .any(|other| &other.record_id == counterpart)
                {
                    return Err(CorpusError::Inconsistent("edge counterpart"));
                }
            }
            for tombstone in &family.tombstones {
                if !family
                    .records
                    .iter()
                    .any(|other| other.record_id == tombstone.forgets)
                {
                    return Err(CorpusError::Inconsistent("tombstone target"));
                }
            }
        }
        Ok(Self { fixture, by_id })
    }

    /// The fixture as parsed.
    #[must_use]
    pub fn fixture(&self) -> &CorpusFixture {
        &self.fixture
    }

    /// Stable corpus identifier.
    #[must_use]
    pub fn corpus_id(&self) -> &str {
        &self.fixture.corpus_id
    }

    /// The tenant the query runs in.
    #[must_use]
    pub fn tenant(&self) -> &str {
        &self.fixture.tenant
    }

    /// Candidates handed to each ranker before the shared filter.
    #[must_use]
    pub fn candidate_count(&self) -> usize {
        self.fixture.candidate_count
    }

    /// Width of every embedding.
    #[must_use]
    pub fn embedding_dimension(&self) -> usize {
        self.fixture.embedding_dimension
    }

    /// The query families.
    #[must_use]
    pub fn families(&self) -> &[FamilyFixture] {
        &self.fixture.families
    }

    /// The query parameters shared by every family.
    #[must_use]
    pub fn query(&self) -> &QueryFixture {
        &self.fixture.query
    }

    /// The policy versions the corpus spans.
    #[must_use]
    pub fn policy(&self) -> &PolicyFixture {
        &self.fixture.policy
    }

    /// The ABI contract the episodes bind to.
    #[must_use]
    pub fn contract(&self) -> &ContractFixture {
        &self.fixture.contract
    }

    /// Seed of the deterministic episode-signing key.
    #[must_use]
    pub fn signer_seed(&self) -> u8 {
        self.fixture.signer_seed
    }

    /// Look a record up by identifier.
    #[must_use]
    pub fn record(&self, record_id: &str) -> Option<&CorpusRecord> {
        self.by_id.get(record_id)
    }

    /// Every record of the corpus, in identifier order.
    pub fn records(&self) -> impl Iterator<Item = &CorpusRecord> {
        self.by_id.values()
    }
}
