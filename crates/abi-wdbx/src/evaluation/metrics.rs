//! The frozen metrics: failure rate, stale reuse rate, ablation and the
//! paired bootstrap.
//!
//! Every rate here is defined over the frozen corpus and the frozen
//! candidate sets. Nothing is learned, fitted or tuned. The definitions are:
//!
//! * **`repeated_failure_rate`** — the share of query families whose selected
//!   record is one the corpus ground truth marks as contradicted,
//!   quarantined, forgotten or written under a stale policy **and** which
//!   the arm presents as usable. The plain vector arm has no disposition,
//!   so it presents everything as usable; the constitutional arm presents a
//!   record as usable only when its disposition is `applicable`. A record
//!   the constitutional arm hands back *with* a disposition is not a
//!   repeated failure: the caller can see it.
//! * **`stale_reuse_rate`** — the same, restricted to records written under
//!   an older policy version.
//! * **`strict_failure_rate`** — the share of families whose selected record
//!   is a failure record at all, whatever disposition it carries. This is
//!   the stricter reading and it is reported next to the headline rate so
//!   that a disposition-marked answer cannot be mistaken for a clean one.
//! * **`unsigned_reuse_rate`** — the share of families whose selected record
//!   nobody signed.
//! * **`applicability_accuracy`** — the share of families whose selected
//!   record is applicable.
//!
//! An abstention counts as a failure in `repeated_failure_rate` and
//! `strict_failure_rate`: the frozen protocol says an abstention is a
//! failure, never an improvement.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// One query family and the ground truth the harness scores against.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FamilyTruth {
    /// The family identifier.
    pub family_id: String,
    /// Records the corpus marks as contradicted, quarantined, forgotten or
    /// written under a stale policy.
    pub failure_ids: BTreeSet<String>,
    /// Records the corpus marks as written under a stale policy.
    pub stale_ids: BTreeSet<String>,
    /// Records the corpus marks as applicable.
    pub applicable_ids: BTreeSet<String>,
    /// Records the corpus marks as unsigned.
    pub unsigned_ids: BTreeSet<String>,
}

/// What one arm selected for one family.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArmOutcome {
    /// The family identifier.
    pub family_id: String,
    /// The selected record, or `None` when the arm abstained.
    pub top1: Option<String>,
    /// The canonical label of the disposition the arm attached, if any.
    pub disposition: Option<String>,
}

impl ArmOutcome {
    /// Whether the arm presented its selection as usable.
    ///
    /// An arm that reports no disposition presents everything as usable:
    /// that is the whole point of the plain vector baseline. An arm that
    /// reports a disposition presents a record as usable only when the
    /// canonical disposition is `accepted`.
    #[must_use]
    pub fn presents_as_usable(&self) -> bool {
        match self.disposition.as_deref() {
            None => true,
            Some(disposition) => disposition == "accepted",
        }
    }
}

/// The measured rates of one arm.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArmMetrics {
    /// How many families were measured.
    pub families: usize,
    /// Families whose selection was a failure record presented as usable.
    pub repeated_failures: usize,
    /// Families whose selection was a stale record presented as usable.
    pub stale_reuses: usize,
    /// Families whose selection was a failure record at all.
    pub strict_failures: usize,
    /// Families whose selection was unsigned.
    pub unsigned_reuses: usize,
    /// Families whose selection was applicable.
    pub applicable_top1: usize,
    /// The share of families that returned a failure record as if usable.
    pub repeated_failure_rate: f64,
    /// The share of families that reused a stale record as if usable.
    pub stale_reuse_rate: f64,
    /// The share of families that returned a failure record at all.
    pub strict_failure_rate: f64,
    /// The share of families that returned an unsigned record.
    pub unsigned_reuse_rate: f64,
    /// The share of families that returned an applicable record.
    pub applicability_accuracy: f64,
}

/// Round a rate to six decimals so a pinned comparison is stable.
fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

/// Score one arm against the frozen ground truth.
///
/// The family counts are small and exact; the conversion to a rate is the
/// only place a `usize` becomes an `f64`.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn score_arm(outcomes: &[ArmOutcome], truths: &[FamilyTruth]) -> ArmMetrics {
    let families = truths.len();
    let mut repeated_failures = 0_usize;
    let mut stale_reuses = 0_usize;
    let mut strict_failures = 0_usize;
    let mut unsigned_reuses = 0_usize;
    let mut applicable_top1 = 0_usize;

    for (outcome, truth) in outcomes.iter().zip(truths.iter()) {
        let Some(top1) = outcome.top1.as_ref() else {
            // An abstention is a failure, never an improvement.
            repeated_failures += 1;
            strict_failures += 1;
            continue;
        };
        let is_failure = truth.failure_ids.contains(top1);
        let is_stale = truth.stale_ids.contains(top1);
        if is_failure {
            strict_failures += 1;
        }
        if is_failure && outcome.presents_as_usable() {
            repeated_failures += 1;
        }
        if is_stale && outcome.presents_as_usable() {
            stale_reuses += 1;
        }
        if truth.unsigned_ids.contains(top1) {
            unsigned_reuses += 1;
        }
        if truth.applicable_ids.contains(top1) {
            applicable_top1 += 1;
        }
    }

    let denominator = families.max(1) as f64;
    ArmMetrics {
        families,
        repeated_failures,
        stale_reuses,
        strict_failures,
        unsigned_reuses,
        applicable_top1,
        repeated_failure_rate: round6(repeated_failures as f64 / denominator),
        stale_reuse_rate: round6(stale_reuses as f64 / denominator),
        strict_failure_rate: round6(strict_failures as f64 / denominator),
        unsigned_reuse_rate: round6(unsigned_reuses as f64 / denominator),
        applicability_accuracy: round6(applicable_top1 as f64 / denominator),
    }
}

/// The relative reduction of one rate, candidate against baseline.
#[must_use]
pub fn relative_reduction(baseline: f64, candidate: f64) -> Option<f64> {
    if baseline <= 0.0 {
        return None;
    }
    Some(round6((baseline - candidate) / baseline))
}

/// One row of the per-dimension ablation.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AblationRow {
    /// The dimension that was dropped.
    pub dropped_dimension: String,
    /// The failure rate with that dimension dropped.
    pub repeated_failure_rate: f64,
    /// The stale reuse rate with that dimension dropped.
    pub stale_reuse_rate: f64,
    /// The strict failure rate with that dimension dropped.
    pub strict_failure_rate: f64,
    /// The unsigned reuse rate with that dimension dropped.
    pub unsigned_reuse_rate: f64,
    /// How many families selected a different record.
    pub top1_changed: usize,
    /// Whether dropping this dimension made either headline rate worse.
    pub carries_benefit: bool,
}

/// The paired family-cluster bootstrap the frozen protocol prescribes.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BootstrapReport {
    /// How many resamples were taken.
    pub resamples: usize,
    /// The pinned seed.
    pub seed: u64,
    /// The mean paired gain of the failure metric.
    pub failure_gain_mean: f64,
    /// The lower two-sided 95% bound of the failure gain.
    pub failure_gain_lower_95: f64,
    /// The mean paired gain of the stale metric.
    pub stale_gain_mean: f64,
    /// The lower two-sided 95% bound of the stale gain.
    pub stale_gain_lower_95: f64,
    /// Whether both lower bounds are strictly positive.
    pub strictly_positive: bool,
    /// Why the criterion did or did not hold.
    pub note: String,
}

/// `SplitMix64`: a small, fully deterministic generator.
///
/// The bootstrap is a resampling device, not an experiment: the same seed
/// must give the same numbers on every machine, forever.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }
}

/// The paired gain per family for the repeated-failure metric.
///
/// A family contributes `1` when the baseline presented a failure record as
/// usable and the constitutional arm did not, `-1` when the reverse happened
/// and `0` otherwise. This is the same definition as
/// [`ArmMetrics::repeated_failure_rate`], so the bootstrap measures the
/// headline metric and not a different one.
#[must_use]
pub fn paired_failure_gains(
    baseline: &[ArmOutcome],
    candidate: &[ArmOutcome],
    truths: &[FamilyTruth],
) -> Vec<f64> {
    let mut gains = Vec::with_capacity(truths.len());
    for ((base, cand), truth) in baseline.iter().zip(candidate.iter()).zip(truths.iter()) {
        let base_failure = base.presents_as_usable()
            && base
                .top1
                .as_ref()
                .is_some_and(|top1| truth.failure_ids.contains(top1));
        let cand_failure = cand.presents_as_usable()
            && cand
                .top1
                .as_ref()
                .is_some_and(|top1| truth.failure_ids.contains(top1));
        gains.push(f64::from(u8::from(base_failure) - u8::from(cand_failure)));
    }
    gains
}

/// The paired gain per family for the stale-policy reuse metric.
#[must_use]
pub fn paired_stale_gains(
    baseline: &[ArmOutcome],
    candidate: &[ArmOutcome],
    truths: &[FamilyTruth],
) -> Vec<f64> {
    let mut gains = Vec::with_capacity(truths.len());
    for ((base, cand), truth) in baseline.iter().zip(candidate.iter()).zip(truths.iter()) {
        let base_stale = base.presents_as_usable()
            && base
                .top1
                .as_ref()
                .is_some_and(|top1| truth.stale_ids.contains(top1));
        let cand_stale = cand.presents_as_usable()
            && cand
                .top1
                .as_ref()
                .is_some_and(|top1| truth.stale_ids.contains(top1));
        gains.push(f64::from(u8::from(base_stale) - u8::from(cand_stale)));
    }
    gains
}

/// Run the paired family-cluster bootstrap the frozen protocol prescribes.
///
/// `gains` holds the per-family paired gain of one metric. The lower bound is
/// the 2.5th percentile of the resampled means, taken by nearest rank.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
#[must_use]
pub fn paired_bootstrap(gains: &[f64], resamples: usize, seed: u64) -> (f64, f64) {
    if gains.is_empty() || resamples == 0 {
        return (0.0, 0.0);
    }
    let mut rng = SplitMix64::new(seed);
    let mut means = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let mut total = 0.0;
        for _ in 0..gains.len() {
            let index = (rng.next_u64() % gains.len() as u64) as usize;
            total += gains[index];
        }
        means.push(total / gains.len() as f64);
    }
    means.sort_by(f64::total_cmp);
    let lower_index = (resamples * 25 / 1000).min(resamples - 1);
    let lower = means[lower_index];
    let mean = gains.iter().sum::<f64>() / gains.len() as f64;
    (round6(mean), round6(lower))
}

/// The frozen thresholds the verdict is measured against.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Thresholds {
    /// The relative reduction the failure rate must reach.
    pub relative_failure_reduction: f64,
    /// The relative reduction the stale reuse rate must reach.
    pub relative_stale_reduction: f64,
}

/// The criteria the verdict records, one by one.
///
/// The criteria are a checklist, so they are booleans.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Criteria {
    /// Whether the baseline failed at all; a zero denominator is
    /// inconclusive, never a pass.
    pub baseline_failures_nonzero: bool,
    /// Whether the failure reduction reached the frozen threshold.
    pub relative_failure_reduction_met: bool,
    /// Whether the stale reuse reduction reached the frozen threshold.
    pub relative_stale_reduction_met: bool,
    /// Whether the paired bootstrap's lower bound was strictly positive.
    pub paired_bootstrap_lower_gain_strictly_positive: bool,
    /// Whether the constitutional arm did not hand back *more* unsigned
    /// records than the baseline did.
    pub unsigned_reuse_not_worse: bool,
    /// The grade this run can claim. It is never the preregistered
    /// experiment: that needs an independent held-out corpus and an
    /// independent runner.
    pub evidence_grade: String,
}

/// The verdict of one run.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Verdict {
    /// `benefit`, `no_benefit` or `inconclusive`.
    pub outcome: String,
    /// The same verdict in one sentence.
    pub statement: String,
    /// The thresholds the verdict was measured against.
    pub thresholds: Thresholds,
    /// The criteria, one by one.
    pub criteria: Criteria,
    /// The negative results this run also produced. A verdict without them
    /// would be a summary that hides what the run cost.
    pub caveats: Vec<String>,
}

/// Decide the verdict of a run.
///
/// The verdict is about the two frozen rates. When the constitutional side
/// does not beat the baseline on them the verdict is `no_benefit`, plainly.
/// When the baseline never failed, the comparison has no denominator and the
/// verdict is `inconclusive`, never a pass.
#[must_use]
pub fn verdict(
    baseline: &ArmMetrics,
    candidate: &ArmMetrics,
    failure_reduction: Option<f64>,
    stale_reduction: Option<f64>,
    bootstrap_strictly_positive: bool,
    thresholds: Thresholds,
) -> Verdict {
    let baseline_failures_nonzero =
        baseline.repeated_failure_rate > 0.0 && baseline.stale_reuse_rate > 0.0;
    let failure_met =
        failure_reduction.is_some_and(|value| value >= thresholds.relative_failure_reduction);
    let stale_met =
        stale_reduction.is_some_and(|value| value >= thresholds.relative_stale_reduction);
    let unsigned_not_worse = candidate.unsigned_reuse_rate <= baseline.unsigned_reuse_rate;

    let (outcome, statement) = if !baseline_failures_nonzero {
        (
            "inconclusive",
            "the plain vector baseline never failed on this corpus, so the comparison has no denominator and no benefit can be claimed",
        )
    } else if failure_met && stale_met {
        (
            "benefit",
            "on this frozen corpus the constitutional arm reduced both the repeated-failure rate and the stale-policy reuse rate against the plain vector baseline by at least the frozen threshold",
        )
    } else {
        (
            "no_benefit",
            "on this frozen corpus the constitutional arm did not reduce both the repeated-failure rate and the stale-policy reuse rate against the plain vector baseline by at least the frozen threshold",
        )
    };

    let mut caveats = Vec::new();
    if !unsigned_not_worse {
        caveats.push(format!(
            "the constitutional arm handed back an unattributed record in {} of {} families, against {} of {} for the plain vector baseline: the constitutional query accepts an unsigned record on its commitment, and the cryptographic dimension reports the missing signature without withholding the record",
            candidate.unsigned_reuses,
            candidate.families,
            baseline.unsigned_reuses,
            baseline.families
        ));
    }
    if candidate.applicability_accuracy <= baseline.applicability_accuracy {
        caveats.push(format!(
            "the constitutional arm returned an applicable record in {} of {} families, against {} of {} for the plain vector baseline",
            candidate.applicable_top1,
            candidate.families,
            baseline.applicable_top1,
            baseline.families
        ));
    }
    if !bootstrap_strictly_positive {
        caveats.push(
            "the paired bootstrap's lower bound was not strictly positive, so the arms are not separated at the 95% level on this corpus size"
                .into(),
        );
    }

    Verdict {
        outcome: outcome.into(),
        statement: statement.into(),
        thresholds,
        criteria: Criteria {
            baseline_failures_nonzero,
            relative_failure_reduction_met: failure_met,
            relative_stale_reduction_met: stale_met,
            paired_bootstrap_lower_gain_strictly_positive: bootstrap_strictly_positive,
            unsigned_reuse_not_worse: unsigned_not_worse,
            evidence_grade: "C1_deterministic_local_comparison_synthetic_corpus".into(),
        },
        caveats,
    }
}

/// The relative reductions of a run.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RelativeReduction {
    /// The reduction of the headline failure rate.
    pub repeated_failure: Option<f64>,
    /// The reduction of the headline stale reuse rate.
    pub stale_reuse: Option<f64>,
    /// The reduction of the strict failure rate.
    pub strict_failure: Option<f64>,
}

/// The two arms of a run.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArmPair {
    /// The plain vector arm.
    pub baseline: ArmMetrics,
    /// The evidence-aware arm.
    pub constitutional: ArmMetrics,
}

/// Everything the harness measured, in one deterministic block.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    /// The corpus identifier.
    pub corpus_id: String,
    /// The SHA-256 of the frozen corpus fixture.
    pub corpus_sha256: String,
    /// How many families were measured.
    pub families: usize,
    /// How many candidates each ranker received before the shared filter.
    pub candidate_count_before_filter: usize,
    /// Whether the index's ordering agreed with the binary64 cosine the
    /// frozen protocol names, for every family.
    pub binary64_ordering_agrees: bool,
    /// The two arms.
    pub arms: ArmPair,
    /// The relative reductions.
    pub relative_reduction: RelativeReduction,
    /// The per-dimension ablation.
    pub ablation: Vec<AblationRow>,
    /// The paired bootstrap.
    pub paired_bootstrap: BootstrapReport,
    /// The verdict.
    pub verdict: Verdict,
}
