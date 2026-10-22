//! The machine-readable result record.
//!
//! The shape follows the repository's existing experiment-record family
//! (`docs/completion/experiments.json`, its `result_schema`): a record
//! carries the protocol and artifact digests, the backend that ran, the
//! suite and case, the seed, both arms' outputs, the oracle's answer, the
//! state hashes before and after, and one `outcome` word.
//!
//! The environment-dependent fields are present and explicitly `null` when
//! the harness did not measure them. The harness measures no wall clock and
//! no memory, so it reports no latency and no peak memory rather than
//! inventing them; it does not read the repository, so it reports no commit
//! and no dirty paths. A caller that wants them fills them in.
//!
//! The `measurement` block is the whole deterministic part of the run: it is
//! what the pinned expected-metrics fixture compares, byte for byte.

use crate::evaluation::constitution::DimensionReport;
use crate::evaluation::metrics::Measurement;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Schema identifier of the harness result document.
pub const REPORT_SCHEMA: &str = "wdbx.evaluation.retrieval-baseline.v1";

/// The supersession lineage of one record, without any content.
///
/// This is the canonical lineage the constitutional query projects, rendered
/// as strings so that the record stays machine-readable.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Lineage {
    /// The digest of the record that supersedes this one.
    pub superseded_by: Option<String>,
    /// How many open contradictions name this record.
    pub open_contradictions: usize,
    /// The resolution edge that closed this record's quarantine or
    /// contradiction.
    pub resolution_edge: Option<String>,
}

/// Project the canonical lineage into the report's form.
#[must_use]
pub fn lineage_of(lineage: &crate::v3::episode::Lineage) -> Lineage {
    Lineage {
        superseded_by: lineage.superseded_by.map(|digest| hex(&digest)),
        open_contradictions: lineage.open_contradictions.len(),
        resolution_edge: lineage.resolution_edge.map(|digest| hex(&digest)),
    }
}

/// One family's result for one arm.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FamilyOutput {
    /// The family identifier.
    pub family_id: String,
    /// The hand-authored template the family follows.
    pub template: String,
    /// The candidates the shared retrieval stage handed over, before the
    /// shared filter.
    pub candidates_before_filter: Vec<String>,
    /// The surviving candidates, in descending cosine order.
    pub candidates: Vec<String>,
    /// The record the arm selected.
    pub top1: Option<String>,
    /// The disposition the arm attached, if any. The plain vector arm
    /// attaches none: that is why it cannot know.
    pub disposition: Option<String>,
    /// The eight dimensions of the selection, reported separately.
    pub dimensions: Vec<DimensionReport>,
    /// The supersession lineage of the selection.
    pub lineage: Option<Lineage>,
    /// The reasons the disposition holds.
    pub explanation: Vec<String>,
    /// Whether the selection is a failure record under the ground truth.
    pub is_failure: bool,
    /// Whether the selection is a stale record under the ground truth.
    pub is_stale: bool,
}

/// The ground truth for one family.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OracleFamily {
    /// The family identifier.
    pub family_id: String,
    /// Records the corpus marks as contradicted, quarantined, forgotten or
    /// written under a stale policy.
    pub failure_ids: Vec<String>,
    /// Records the corpus marks as written under a stale policy.
    pub stale_ids: Vec<String>,
    /// Records the corpus marks as applicable.
    pub applicable_ids: Vec<String>,
    /// Records the corpus marks as unsigned.
    pub unsigned_ids: Vec<String>,
}

/// The result of one harness run.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResultRecord {
    /// Schema identifier of this document.
    pub schema: String,
    /// The digest of the frozen protocol the run follows.
    pub protocol_digest: String,
    /// The commit the run was made from. `null` when the harness did not
    /// read the repository.
    pub implementation_commit: Option<String>,
    /// The dirty paths of that commit. Empty when the harness did not read
    /// the repository.
    pub dirty_paths: Vec<String>,
    /// The digest of the frozen corpus artifact.
    pub artifact_hash: String,
    /// The platform the run happened on. `null` when the harness did not
    /// measure it.
    pub platform: Option<String>,
    /// The backend that produced the result.
    pub backend_executed: String,
    /// The suite this record belongs to.
    pub suite: String,
    /// The case this record belongs to.
    pub case_id: String,
    /// The pinned seed of the run.
    pub seed: u64,
    /// The plain vector arm's output.
    pub baseline_output: Vec<FamilyOutput>,
    /// The evidence-aware arm's output.
    pub candidate_output: Vec<FamilyOutput>,
    /// The oracle's answer.
    pub oracle_result: Vec<OracleFamily>,
    /// The independent rating. `null`: this harness is not an independently
    /// rated experiment.
    pub independent_rating: Option<String>,
    /// The measured latency. `null`: the harness measures no wall clock.
    pub latency_ms: Option<u64>,
    /// The measured peak memory. `null`: the harness measures no memory.
    pub peak_memory_bytes: Option<u64>,
    /// The digest of the canonical state before the measurement.
    pub state_hash_before: String,
    /// The digest of the canonical state after the measurement.
    pub state_hash_after: String,
    /// Anything the run wrote to standard error.
    pub stderr: String,
    /// One word: `benefit`, `no_benefit` or `inconclusive`.
    pub outcome: String,
    /// The whole deterministic measurement block.
    pub measurement: Measurement,
}

impl ResultRecord {
    /// Render the record as JSON.
    ///
    /// # Errors
    ///
    /// Returns a [`serde_json::Error`] when the record cannot be rendered.
    pub fn to_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

/// The SHA-256 of a file's bytes, as lowercase hex.
#[must_use]
pub fn file_digest(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) => hex(&Sha256::digest(&bytes)),
        Err(_) => String::new(),
    }
}

/// The SHA-256 of some bytes, as lowercase hex.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
