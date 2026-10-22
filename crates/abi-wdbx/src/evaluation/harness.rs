//! The run: build the ledger, drive both arms, measure, and report.
//!
//! The harness is deterministic by construction. There is no wall clock, no
//! entropy, no network, and no dependence on the order the filesystem
//! returns anything in. The only randomness is the paired bootstrap, and it
//! is pinned to the seed the frozen protocol names.

use crate::evaluation::constitution::{self, DimensionReport};
use crate::evaluation::corpus::{Corpus, CorpusError, RecordFixture};
use crate::evaluation::ledger::{self, BuiltLedger, LedgerError};
use crate::evaluation::metrics::{
    self, AblationRow, ArmOutcome, BootstrapReport, FamilyTruth, Measurement, RelativeReduction,
    Thresholds,
};
use crate::evaluation::report::{FamilyOutput, OracleFamily, ResultRecord, file_digest};
use crate::index::ExactIndex;
use crate::v3::episode::{ConstitutionalResult, Disposition, EpisodeStoreError, EvidenceDimension};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// The frozen thresholds the verdict is measured against.
pub const FROZEN_THRESHOLDS: Thresholds = Thresholds {
    relative_failure_reduction: 0.25,
    relative_stale_reduction: 0.25,
};

/// The resample count the frozen protocol prescribes.
pub const BOOTSTRAP_RESAMPLES: usize = 10_000;

/// The seed the frozen protocol pins.
pub const BOOTSTRAP_SEED: u64 = 104_729;

/// The suite this harness belongs to.
pub const SUITE: &str = "retrieval";

/// The case this harness runs.
pub const CASE_ID: &str = "retrieval-baseline-v1";

/// How the harness is configured.
#[derive(Clone, Debug)]
pub struct EvaluationConfig {
    /// The frozen corpus fixture.
    pub corpus_path: PathBuf,
    /// The dimensions the constitutional arm is not allowed to consult.
    ///
    /// Empty for the real run. Dropping every dimension reproduces the plain
    /// vector arm exactly, which is how the harness proves it can report
    /// "no benefit".
    pub dropped_dimensions: Vec<EvidenceDimension>,
    /// How many bootstrap resamples to take.
    pub resamples: usize,
    /// The bootstrap seed.
    pub bootstrap_seed: u64,
    /// The thresholds the verdict is measured against.
    pub thresholds: Thresholds,
}

impl Default for EvaluationConfig {
    fn default() -> Self {
        Self {
            corpus_path: PathBuf::new(),
            dropped_dimensions: Vec::new(),
            resamples: BOOTSTRAP_RESAMPLES,
            bootstrap_seed: BOOTSTRAP_SEED,
            thresholds: FROZEN_THRESHOLDS,
        }
    }
}

/// Why a run could not be completed.
#[derive(Debug)]
pub enum EvaluationError {
    /// The corpus fixture was invalid.
    Corpus(CorpusError),
    /// The ledger could not be built.
    Ledger(LedgerError),
    /// The store refused a read.
    Store(EpisodeStoreError),
    /// The index refused a vector.
    Index,
    /// The harness's ablation view of the constitutional rules no longer
    /// reproduces the canonical disposition. The ablation would then be
    /// measuring a restatement of rules that no longer exist.
    ConstitutionalDrift,
}

impl std::fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Corpus(error) => write!(formatter, "{error}"),
            Self::Ledger(error) => write!(formatter, "{error}"),
            Self::Store(error) => write!(formatter, "{error}"),
            Self::Index => formatter.write_str("the exact index refused a vector"),
            Self::ConstitutionalDrift => formatter.write_str(
                "the ablation view of the constitutional rules disagrees with the canonical query",
            ),
        }
    }
}

impl std::error::Error for EvaluationError {}

impl From<CorpusError> for EvaluationError {
    fn from(error: CorpusError) -> Self {
        Self::Corpus(error)
    }
}

impl From<LedgerError> for EvaluationError {
    fn from(error: LedgerError) -> Self {
        Self::Ledger(error)
    }
}

impl From<EpisodeStoreError> for EvaluationError {
    fn from(error: EpisodeStoreError) -> Self {
        Self::Store(error)
    }
}

/// The state the harness needs to answer one query.
struct QueryContext<'a> {
    corpus: &'a Corpus,
    ledger: &'a BuiltLedger,
    /// The canonical constitutional reading of every record of the tenant.
    readings: BTreeMap<[u8; 32], ConstitutionalResult>,
}

/// The frozen embeddings are stored in binary64 and indexed in binary32.
///
/// The exact index is the binary32 backend the crate already ships; the
/// harness checks its ordering against a binary64 cosine reference, so the
/// narrowing is a property the run measures rather than one it assumes.
#[allow(clippy::cast_possible_truncation)]
fn to_f32(vector: &[f64]) -> Vec<f32> {
    vector.iter().map(|value| *value as f32).collect()
}

/// The binary64 cosine the frozen protocol names.
fn cosine_f64(left: &[f64], right: &[f64]) -> f64 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0_f64;
    let mut norm_left = 0.0_f64;
    let mut norm_right = 0.0_f64;
    for (&left_value, &right_value) in left.iter().zip(right) {
        dot += left_value * right_value;
        norm_left += left_value * left_value;
        norm_right += right_value * right_value;
    }
    let denominator = norm_left.sqrt() * norm_right.sqrt();
    if denominator == 0.0 {
        0.0
    } else {
        dot / denominator
    }
}

/// A candidate that survived the shared filter.
#[derive(Clone, Debug)]
struct Survivor {
    record_id: String,
    cosine: f32,
}

/// The result of one arm on one family.
#[derive(Clone, Debug)]
struct FamilySelection {
    outcome: ArmOutcome,
    /// The canonical result of the selection, and the disposition the arm
    /// actually applied to it.
    detail: Option<(ConstitutionalResult, Disposition)>,
}

/// Apply the shared filter both arms share.
///
/// The filter is tenant, scope and cryptographic rejection only. Both arms
/// receive exactly the same surviving identifiers and vectors: the two arms
/// differ in what they do with them, never in what they are given.
///
/// Cryptographically *invalid* records are dropped. Unsigned records are not
/// invalid, and requirement P04.03 asks for the unsigned case to be
/// exercised, so they are kept and left to the disposition.
fn shared_filter(
    context: &QueryContext<'_>,
    record: &RecordFixture,
) -> Result<bool, EpisodeStoreError> {
    let Some(digest) = context.ledger.digests.get(&record.record_id) else {
        return Ok(false);
    };
    let Some(receipt) = context
        .ledger
        .store
        .find_receipt(context.corpus.tenant(), digest)?
    else {
        // The record belongs to another tenant, or was never appended.
        return Ok(false);
    };
    if receipt.guild_ref != context.corpus.tenant() {
        return Ok(false);
    }
    let scope_ok = match context.corpus.query().scope.as_str() {
        "member" => record.member_scoped,
        "guild" => !record.member_scoped,
        _ => true,
    };
    if !scope_ok {
        return Ok(false);
    }
    let Some(result) = context.readings.get(&receipt.episode_digest) else {
        return Ok(false);
    };
    Ok(result.dimensions.cryptographic.signature != crate::v3::episode::SignatureEvidence::Invalid)
}

/// Rank the survivors the way the constitutional arm does: by disposition
/// tier first, then by the same cosine the baseline used, then by record
/// identifier.
///
/// The tier is an explicit, named state, not a product of the eight
/// dimensions. A rejected record is never returned.
fn rank_constitutional(
    evaluations: &[(String, f32, ConstitutionalResult, Disposition)],
) -> Option<&(String, f32, ConstitutionalResult, Disposition)> {
    let mut ranked: Vec<&(String, f32, ConstitutionalResult, Disposition)> = evaluations
        .iter()
        .filter(|(_, _, _, disposition)| *disposition != Disposition::Rejected)
        .collect();
    ranked.sort_by(|left, right| {
        constitution::tier(left.3)
            .cmp(&constitution::tier(right.3))
            .then_with(|| right.1.total_cmp(&left.1))
            .then_with(|| left.0.cmp(&right.0))
    });
    ranked.first().copied()
}

/// Run the constitutional arm over one family.
///
/// `dropped` suppresses dimensions for the ablation. With nothing dropped,
/// the canonical disposition is used unchanged; the harness's ablation view
/// must reproduce it exactly, and the run fails if it does not.
fn constitutional_family(
    context: &QueryContext<'_>,
    survivors: &[Survivor],
    dropped: &[EvidenceDimension],
) -> FamilySelection {
    let mut evaluations: Vec<(String, f32, ConstitutionalResult, Disposition)> = Vec::new();
    for survivor in survivors {
        let Some(digest) = context.ledger.digests.get(&survivor.record_id) else {
            continue;
        };
        let Some(result) = context.readings.get(digest) else {
            continue;
        };
        let disposition = if dropped.is_empty() {
            result.disposition
        } else {
            constitution::disposition_with(&result.dimensions, dropped)
        };
        evaluations.push((
            survivor.record_id.clone(),
            survivor.cosine,
            result.clone(),
            disposition,
        ));
    }
    let selected = rank_constitutional(&evaluations);
    let outcome = match selected {
        Some((record_id, _, _, disposition)) => ArmOutcome {
            family_id: String::new(),
            top1: Some(record_id.clone()),
            disposition: Some(disposition.label().to_string()),
        },
        None => ArmOutcome {
            family_id: String::new(),
            top1: None,
            disposition: None,
        },
    };
    FamilySelection {
        outcome,
        detail: selected.map(|(_, _, result, disposition)| (result.clone(), *disposition)),
    }
}

/// The ground truth of one family, read from the corpus fixture.
fn family_truth(corpus: &Corpus, family_id: &str) -> FamilyTruth {
    let family = corpus
        .families()
        .iter()
        .find(|family| family.family_id == family_id)
        .expect("the family is in the corpus");
    let mut failure_ids = BTreeSet::new();
    let mut stale_ids = BTreeSet::new();
    let mut applicable_ids = BTreeSet::new();
    let mut unsigned_ids = BTreeSet::new();
    for record in &family.records {
        let role = record.role.as_str();
        if matches!(
            role,
            "contradicted" | "quarantined" | "forgotten" | "stale_policy"
        ) {
            failure_ids.insert(record.record_id.clone());
        }
        if role == "stale_policy" {
            stale_ids.insert(record.record_id.clone());
        }
        if role == "unsigned" {
            unsigned_ids.insert(record.record_id.clone());
        }
        if record.role == "healthy_current" {
            applicable_ids.insert(record.record_id.clone());
        }
    }
    FamilyTruth {
        family_id: family_id.to_string(),
        failure_ids,
        stale_ids,
        applicable_ids,
        unsigned_ids,
    }
}

/// Run the harness.
///
/// # Errors
///
/// Returns [`EvaluationError`] when the corpus, the ledger or a store read
/// fails, or when the harness's ablation view of the constitutional rules
/// drifts away from the canonical disposition.
pub fn run(config: &EvaluationConfig) -> Result<ResultRecord, EvaluationError> {
    let corpus = Corpus::load(&config.corpus_path)?;
    let corpus_digest = file_digest(&config.corpus_path);
    let ledger = ledger::build(&corpus)?;
    let ledger_path = ledger.scratch.path().join("episodes.v1.jsonl");
    let state_hash = file_digest(&ledger_path);

    let readings = constitution::read(&ledger.store, corpus.tenant(), &ledger.signer)?;
    let context = QueryContext {
        corpus: &corpus,
        ledger: &ledger,
        readings,
    };

    // The index holds every record of the corpus, every tenant. The shared
    // filter, not the index, is what keeps one tenant out of another's
    // answers.
    let mut index = ExactIndex::new();
    let mut index_ids: Vec<String> = Vec::new();
    for record in corpus.records() {
        let values = to_f32(&record.record.vector);
        index
            .put_vector(&values)
            .map_err(|_| EvaluationError::Index)?;
        index_ids.push(record.record.record_id.clone());
    }

    let mut baseline_outcomes: Vec<ArmOutcome> = Vec::new();
    let mut constitutional_outcomes: Vec<ArmOutcome> = Vec::new();
    let mut baseline_output: Vec<FamilyOutput> = Vec::new();
    let mut candidate_output: Vec<FamilyOutput> = Vec::new();
    let mut oracle: Vec<OracleFamily> = Vec::new();
    let mut truths: Vec<FamilyTruth> = Vec::new();
    let mut binary64_agrees = true;

    for family in corpus.families() {
        let query_vector = to_f32(&family.query_vector);
        let hits = index
            .search(&query_vector, corpus.candidate_count())
            .map_err(|_| EvaluationError::Index)?;
        let pre_filter: Vec<(String, f32)> = hits
            .iter()
            .map(|hit| {
                (
                    index_ids[usize::try_from(hit.id - 1).unwrap_or(0)].clone(),
                    hit.score,
                )
            })
            .collect();

        // The binary64 reference the frozen protocol names must agree with
        // the index's ordering, or the two are not interchangeable.
        let mut reference: Vec<(String, f64)> = corpus
            .records()
            .map(|record| {
                (
                    record.record.record_id.clone(),
                    cosine_f64(&family.query_vector, &record.record.vector),
                )
            })
            .collect();
        reference.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        let reference_ids: Vec<&String> = reference
            .iter()
            .take(corpus.candidate_count())
            .map(|(record_id, _)| record_id)
            .collect();
        let index_ids: Vec<&String> = pre_filter.iter().map(|(id, _)| id).collect();
        if reference_ids != index_ids {
            binary64_agrees = false;
        }

        let mut survivors: Vec<Survivor> = Vec::new();
        for (record_id, cosine) in &pre_filter {
            let Some(record) = corpus.record(record_id) else {
                continue;
            };
            if shared_filter(&context, &record.record)? {
                survivors.push(Survivor {
                    record_id: record_id.clone(),
                    cosine: *cosine,
                });
            }
        }

        // The plain vector arm: cosine only, no disposition.
        let baseline_top1 = survivors.first().map(|survivor| survivor.record_id.clone());
        let baseline_outcome = ArmOutcome {
            family_id: family.family_id.clone(),
            top1: baseline_top1.clone(),
            disposition: None,
        };

        let selection = constitutional_family(&context, &survivors, &config.dropped_dimensions);
        let mut constitutional_outcome = selection.outcome.clone();
        constitutional_outcome
            .family_id
            .clone_from(&family.family_id);

        // The ablation view must reproduce the canonical disposition on
        // every record, or the ablation would measure a restatement of
        // rules that no longer exist.
        for survivor in &survivors {
            let Some(digest) = ledger.digests.get(&survivor.record_id) else {
                continue;
            };
            let Some(result) = context.readings.get(digest) else {
                continue;
            };
            if constitution::disposition_with(&result.dimensions, &[]) != result.disposition {
                return Err(EvaluationError::ConstitutionalDrift);
            }
        }

        let truth = family_truth(&corpus, &family.family_id);
        let baseline_is_failure = baseline_top1
            .as_ref()
            .is_some_and(|top1| truth.failure_ids.contains(top1));
        let baseline_is_stale = baseline_top1
            .as_ref()
            .is_some_and(|top1| truth.stale_ids.contains(top1));
        let constitutional_top1 = constitutional_outcome.top1.clone();
        let constitutional_is_failure = constitutional_top1
            .as_ref()
            .is_some_and(|top1| truth.failure_ids.contains(top1));
        let constitutional_is_stale = constitutional_top1
            .as_ref()
            .is_some_and(|top1| truth.stale_ids.contains(top1));

        baseline_output.push(FamilyOutput {
            family_id: family.family_id.clone(),
            template: family.template.clone(),
            candidates_before_filter: pre_filter
                .iter()
                .map(|(record_id, _)| record_id.clone())
                .collect(),
            candidates: survivors.iter().map(|s| s.record_id.clone()).collect(),
            top1: baseline_top1,
            disposition: None,
            dimensions: Vec::new(),
            lineage: None,
            explanation: Vec::new(),
            is_failure: baseline_is_failure,
            is_stale: baseline_is_stale,
        });
        candidate_output.push(FamilyOutput {
            family_id: family.family_id.clone(),
            template: family.template.clone(),
            candidates_before_filter: pre_filter
                .iter()
                .map(|(record_id, _)| record_id.clone())
                .collect(),
            candidates: survivors.iter().map(|s| s.record_id.clone()).collect(),
            top1: constitutional_top1,
            disposition: selection
                .detail
                .as_ref()
                .map(|(_, disposition)| disposition.label().to_string()),
            dimensions: selection
                .detail
                .as_ref()
                .map(|(result, _)| constitution::dimension_reports(result))
                .unwrap_or_default(),
            lineage: selection
                .detail
                .as_ref()
                .map(|(result, _)| crate::evaluation::report::lineage_of(&result.lineage)),
            explanation: selection
                .detail
                .as_ref()
                .map(|(result, _)| constitution::explanation(result))
                .unwrap_or_default(),
            is_failure: constitutional_is_failure,
            is_stale: constitutional_is_stale,
        });
        oracle.push(OracleFamily {
            family_id: family.family_id.clone(),
            failure_ids: truth.failure_ids.iter().cloned().collect(),
            stale_ids: truth.stale_ids.iter().cloned().collect(),
            applicable_ids: truth.applicable_ids.iter().cloned().collect(),
            unsigned_ids: truth.unsigned_ids.iter().cloned().collect(),
        });
        baseline_outcomes.push(baseline_outcome);
        constitutional_outcomes.push(constitutional_outcome);
        truths.push(truth);
    }

    let baseline_metrics = metrics::score_arm(&baseline_outcomes, &truths);
    let constitutional_metrics = metrics::score_arm(&constitutional_outcomes, &truths);

    // The per-dimension ablation: drop one dimension at a time and measure
    // again. The arm that drops a dimension stops being able to see that
    // signal, which is exactly what the ablation is meant to show.
    let mut ablation: Vec<AblationRow> = Vec::new();
    for dimension in EvidenceDimension::all() {
        let dropped = vec![dimension];
        let mut outcomes: Vec<ArmOutcome> = Vec::new();
        for family in corpus.families() {
            let query_vector = to_f32(&family.query_vector);
            let hits = index
                .search(&query_vector, corpus.candidate_count())
                .map_err(|_| EvaluationError::Index)?;
            let mut survivors: Vec<Survivor> = Vec::new();
            for hit in &hits {
                let record_id = index_ids[usize::try_from(hit.id - 1).unwrap_or(0)].clone();
                let Some(record) = corpus.record(&record_id) else {
                    continue;
                };
                if shared_filter(&context, &record.record)? {
                    survivors.push(Survivor {
                        record_id,
                        cosine: hit.score,
                    });
                }
            }
            let mut outcome = constitutional_family(&context, &survivors, &dropped).outcome;
            outcome.family_id.clone_from(&family.family_id);
            outcomes.push(outcome);
        }
        let metrics = metrics::score_arm(&outcomes, &truths);
        let top1_changed = outcomes
            .iter()
            .zip(constitutional_outcomes.iter())
            .filter(|(left, right)| left.top1 != right.top1)
            .count();
        ablation.push(AblationRow {
            dropped_dimension: dimension.label().to_string(),
            repeated_failure_rate: metrics.repeated_failure_rate,
            stale_reuse_rate: metrics.stale_reuse_rate,
            strict_failure_rate: metrics.strict_failure_rate,
            unsigned_reuse_rate: metrics.unsigned_reuse_rate,
            top1_changed,
            carries_benefit: metrics.repeated_failure_rate
                > constitutional_metrics.repeated_failure_rate
                || metrics.stale_reuse_rate > constitutional_metrics.stale_reuse_rate,
        });
    }

    let failure_gains =
        metrics::paired_failure_gains(&baseline_outcomes, &constitutional_outcomes, &truths);
    let stale_gains =
        metrics::paired_stale_gains(&baseline_outcomes, &constitutional_outcomes, &truths);
    let (failure_mean, failure_lower) =
        metrics::paired_bootstrap(&failure_gains, config.resamples, config.bootstrap_seed);
    let (stale_mean, stale_lower) =
        metrics::paired_bootstrap(&stale_gains, config.resamples, config.bootstrap_seed);
    let strictly_positive = failure_lower > 0.0 && stale_lower > 0.0;
    let bootstrap = BootstrapReport {
        resamples: config.resamples,
        seed: config.bootstrap_seed,
        failure_gain_mean: failure_mean,
        failure_gain_lower_95: failure_lower,
        stale_gain_mean: stale_mean,
        stale_gain_lower_95: stale_lower,
        strictly_positive,
        note: "the frozen protocol asks for 10000 family-cluster paired resamples with seed 104729 over the headline metrics; a lower bound of zero would mean the corpus is too small to separate the arms at the 95% level, which is a limitation of this local corpus and not a pass".into(),
    };

    let failure_reduction = metrics::relative_reduction(
        baseline_metrics.repeated_failure_rate,
        constitutional_metrics.repeated_failure_rate,
    );
    let stale_reduction = metrics::relative_reduction(
        baseline_metrics.stale_reuse_rate,
        constitutional_metrics.stale_reuse_rate,
    );
    let strict_reduction = metrics::relative_reduction(
        baseline_metrics.strict_failure_rate,
        constitutional_metrics.strict_failure_rate,
    );
    let verdict = metrics::verdict(
        &baseline_metrics,
        &constitutional_metrics,
        failure_reduction,
        stale_reduction,
        strictly_positive,
        config.thresholds.clone(),
    );

    let measurement = Measurement {
        corpus_id: corpus.corpus_id().to_string(),
        corpus_sha256: corpus_digest.clone(),
        families: corpus.families().len(),
        candidate_count_before_filter: corpus.candidate_count(),
        binary64_ordering_agrees: binary64_agrees,
        arms: metrics::ArmPair {
            baseline: baseline_metrics.clone(),
            constitutional: constitutional_metrics.clone(),
        },
        relative_reduction: RelativeReduction {
            repeated_failure: failure_reduction,
            stale_reuse: stale_reduction,
            strict_failure: strict_reduction,
        },
        ablation,
        paired_bootstrap: bootstrap,
        verdict: verdict.clone(),
    };

    Ok(ResultRecord {
        schema: crate::evaluation::report::REPORT_SCHEMA.to_string(),
        protocol_digest: corpus_digest,
        implementation_commit: None,
        dirty_paths: Vec::new(),
        artifact_hash: state_hash.clone(),
        platform: None,
        backend_executed: "abi-wdbx::evaluation (ExactIndex + EpisodeStore::query)".into(),
        suite: SUITE.into(),
        case_id: CASE_ID.into(),
        seed: config.bootstrap_seed,
        baseline_output,
        candidate_output,
        oracle_result: oracle,
        independent_rating: None,
        latency_ms: None,
        peak_memory_bytes: None,
        state_hash_before: state_hash.clone(),
        state_hash_after: state_hash,
        stderr: String::new(),
        outcome: verdict.outcome.clone(),
        measurement,
    })
}

/// The dimension reports the harness renders for the constitutional arm.
///
/// This is the report's view of the canonical eight dimensions.
pub type Reports = Vec<DimensionReport>;
