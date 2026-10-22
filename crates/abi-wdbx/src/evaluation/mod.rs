//! Frozen local baseline comparison harness for evidence-aware retrieval.
//!
//! This module is the C1 falsification harness requirement P04.03 asks for:
//! compare a plain vector retrieval arm against an evidence-aware arm over
//! a **frozen** corpus, with frozen embeddings, frozen storage, and a
//! frozen candidate count, and report the result whether or not the
//! evidence-aware arm wins.
//!
//! ## What the two arms are
//!
//! * **Baseline arm** — plain cosine retrieval over the exact vector index
//!   ([`crate::index::ExactIndex`], the deterministic correctness backend).
//!   It sees vectors and nothing else. It cannot know that a record was
//!   written under an older policy, contradicted, quarantined, forgotten or
//!   never signed, so it returns them *as if they were usable*.
//! * **Constitutional arm** — the same vectors, the same storage, the same
//!   candidate count, over the same episode ledger. It asks the canonical
//!   constitutional read model
//!   ([`crate::v3::episode::EpisodeStore::query`]) and attaches the
//!   disposition, the eight evidence dimensions, the lineage and the
//!   explanation the canonical query returns. The harness owns no second
//!   constitutional implementation.
//!
//! ## What it deliberately does not do
//!
//! * It does **not** collapse the eight dimensions into one score. That
//!   opaque collapse is what the constitution forbids, and
//!   [`crate::temporal::ScoreComponents::combined`] is the existing example
//!   of it. Each dimension is reported on its own, with its own reason.
//! * It does **not** read the corpus ground truth while ranking. The
//!   constitutional reading comes from the canonical query; the ground
//!   truth is used afterwards, to score.
//! * It does **not** tune anything. The corpus, the expected metrics and the
//!   verdict are frozen fixtures, and the harness reports "no benefit"
//!   plainly when that is the honest answer.
//!
//! ## Evidence boundary
//!
//! This is a deterministic local comparison over a synthetic corpus. It is
//! **not** a research result, **not** a production claim, and **not**
//! evidence that evidence-weighted retrieval beats a vector database in
//! production. See the receipt under `docs/superpowers/plans/`.
//!
//! ## Layout
//!
//! * [`corpus`] — the frozen fixture schema and its loader.
//! * [`ledger`] — builds the canonical episode ledger from the fixture.
//! * [`constitution`] — the constitutional reading: the canonical query's
//!   answer, the disposition tiers, and the verified ablation view of the
//!   canonical rules.
//! * [`metrics`] — the two arms, the frozen rates, the per-dimension
//!   ablation and the paired bootstrap.
//! * [`report`] — the machine-readable record, shaped after the repository's
//!   experiment-record family.
//! * [`harness`] — the run that ties them together.

pub mod constitution;
pub mod corpus;
pub mod harness;
pub mod ledger;
pub mod metrics;
pub mod report;

pub use constitution::{DimensionReport, disposition_with, explanation, read, tier};
pub use corpus::{
    ContractFixture, Corpus, CorpusError, CorpusFixture, EdgeFixture, ExpectedDisposition,
    FamilyFixture, PolicyFixture, QueryFixture, RecordFixture, TombstoneFixture,
};
pub use harness::{EvaluationConfig, EvaluationError, run};
pub use ledger::{BuiltLedger, LedgerError};
pub use metrics::{
    AblationRow, ArmMetrics, ArmOutcome, ArmPair, BootstrapReport, Criteria, FamilyTruth,
    Measurement, RelativeReduction, Thresholds, Verdict,
};
pub use report::{FamilyOutput, OracleFamily, ResultRecord};
