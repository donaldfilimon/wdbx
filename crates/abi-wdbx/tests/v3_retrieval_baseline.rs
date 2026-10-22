//! The frozen local baseline comparison for evidence-aware retrieval.
//!
//! This is the falsification harness requirement P04.03 asks for: a plain
//! vector retrieval arm and an evidence-aware arm over the *same* frozen
//! corpus, the *same* frozen embeddings, the *same* storage and the *same*
//! candidate count, measured on repeated failures and stale-policy reuse,
//! with the negative results retained.
//!
//! The constitutional arm is the canonical
//! [`abi_wdbx::v3::episode::EpisodeStore::query`]: the harness asks the
//! shipped constitutional read model and reads the disposition, the eight
//! dimensions, the lineage and the explanation out of its answer. It owns no
//! second constitutional implementation.
//!
//! The evidence boundary matters more than the numbers. This is a
//! deterministic local C1 comparison over a synthetic frozen corpus. It is
//! not a research result, not a production claim, and not evidence that
//! evidence-weighted retrieval beats a vector database in production. The
//! receipt under `docs/superpowers/plans/` states the boundary in full.

use abi_wdbx::evaluation::{
    Corpus, EvaluationConfig, ExpectedDisposition, Measurement, constitution, ledger,
};
use abi_wdbx::v3::episode::{Disposition, EvidenceDimension};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The tests live in a module so that the repository's focused-test
/// command selects them:
///
/// ```sh
/// cargo test -p abi-wdbx v3_retrieval_baseline
/// ```
mod v3_retrieval_baseline {
    use super::*;

    /// The frozen corpus fixture.
    const CORPUS: &str = "tests/fixtures/retrieval-baseline/corpus-v1.json";
    /// The pinned expected metrics.
    const EXPECTED: &str = "tests/fixtures/retrieval-baseline/expected-metrics-v1.json";

    /// The pinned expectation document.
    #[derive(Debug, Deserialize)]
    struct ExpectedMetrics {
        /// Schema identifier of the expectation document.
        schema: String,
        /// The corpus identifier the expectation belongs to.
        corpus_id: String,
        /// The digest of the corpus the expectation was measured on.
        corpus_sha256: String,
        /// The verdict the expectation records.
        outcome: String,
        /// The whole deterministic measurement block.
        measurement: Measurement,
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(name)
    }

    fn config() -> EvaluationConfig {
        EvaluationConfig {
            corpus_path: fixture(CORPUS),
            ..EvaluationConfig::default()
        }
    }

    /// Whether an environment variable is explicitly set to `1`.
    ///
    /// The regeneration opt-ins are deliberately narrow: a variable that is
    /// merely *present* is not an opt-in, so a stray export cannot repin a
    /// metric.
    fn opted_in(name: &str) -> bool {
        std::env::var(name).ok().as_deref() == Some("1")
    }

    /// Print a notice on the process's own stderr.
    ///
    /// The test harness captures the printing macros, so a notice that has
    /// to be seen even when the test passes is written to the real stderr.
    fn notice(message: &str) {
        use std::io::Write;
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(stderr, "{message}");
        let _ = stderr.flush();
    }

    /// The measurement must not move between two runs of the same binary.
    #[test]
    fn the_harness_is_deterministic() {
        let first = abi_wdbx::evaluation::run(&config()).expect("first run");
        let second = abi_wdbx::evaluation::run(&config()).expect("second run");
        let first = serde_json::to_string(&first).expect("first JSON");
        let second = serde_json::to_string(&second).expect("second JSON");
        assert_eq!(first, second, "two runs must be byte-identical");
    }

    /// The comparison is meaningless if the baseline never fails, so the corpus
    /// has to exercise the failure mode for real.
    #[test]
    fn the_plain_vector_baseline_really_fails() {
        let report = abi_wdbx::evaluation::run(&config()).expect("run");
        let baseline = &report.measurement.arms.baseline;
        assert!(
            baseline.repeated_failure_rate > 0.0,
            "the baseline must return contradicted, quarantined, forgotten or stale records as if usable; it returned none"
        );
        assert!(
            baseline.stale_reuse_rate > 0.0,
            "the baseline must reuse stale-policy records; it reused none"
        );
        assert!(baseline.families > 1, "one family cannot show a rate");
        assert!(
            report.measurement.binary64_ordering_agrees,
            "the index's ordering must agree with the binary64 cosine the frozen protocol names"
        );
        assert_eq!(
            report.state_hash_before, report.state_hash_after,
            "the measurement must not mutate the canonical ledger"
        );
    }

    /// The constitutional side must mark exactly what the corpus says it should.
    ///
    /// The stale, contradicted, quarantined and forgotten cases are the ones the
    /// requirement names. The unsigned case is deliberately *not* asserted here:
    /// the canonical query accepts an unsigned record on its commitment and
    /// reports the missing signature in its cryptographic dimension, so the
    /// corpus records that expectation and the harness measures the difference
    /// rather than pretending it is not there.
    #[test]
    fn the_constitutional_dispositions_match_the_hand_authored_ground_truth() {
        let corpus = Corpus::load(fixture(CORPUS)).expect("corpus");
        let built = ledger::build(&corpus).expect("ledger");
        let readings = constitution::read(&built.store, corpus.tenant(), &built.signer)
            .expect("the constitutional query answers");

        let mut checked = 0_usize;
        for family in corpus.families() {
            for record in &family.records {
                let digest = built
                    .digests
                    .get(&record.record_id)
                    .expect("the record was appended");
                let Some(result) = readings.get(digest) else {
                    panic!("{} is not in the constitutional answer", record.record_id);
                };
                let expected = ExpectedDisposition::parse(&record.expected_disposition)
                    .expect("expected label");
                let expected = match expected {
                    ExpectedDisposition::Accepted => Disposition::Accepted,
                    ExpectedDisposition::Uncertain => Disposition::Uncertain,
                    ExpectedDisposition::Contradicted => Disposition::Contradicted,
                    ExpectedDisposition::Quarantined => Disposition::Quarantined,
                    ExpectedDisposition::Rejected => Disposition::Rejected,
                    ExpectedDisposition::Unknown => Disposition::Unknown,
                    ExpectedDisposition::FilteredOut => {
                        panic!("a tenant record cannot be filtered out")
                    }
                };
                assert_eq!(
                    result.disposition.label(),
                    expected.label(),
                    "{} was classified as {} but the corpus says {}",
                    record.record_id,
                    result.disposition.label(),
                    expected.label()
                );
                assert_eq!(
                    result.explanation.entries.len(),
                    EvidenceDimension::all().len(),
                    "{} must carry one explanation per dimension",
                    record.record_id
                );
                checked += 1;
            }
        }
        assert!(checked >= 100, "the corpus must be large enough to matter");
    }

    /// The pinned expectation must hold exactly, metric for metric.
    #[test]
    fn the_measurement_matches_the_pinned_expectation() {
        let report = abi_wdbx::evaluation::run(&config()).expect("run");

        // The pinned expectation is *asserted*, never rewritten by accident.
        // Rewriting it takes two explicit opt-ins, and it says so loudly:
        // a stray environment variable must not be able to repin a metric.
        if opted_in("WDBX_WRITE_EXPECTED") {
            assert!(
                opted_in("WDBX_REGENERATE_EXPECTED"),
                "WDBX_WRITE_EXPECTED=1 needs WDBX_REGENERATE_EXPECTED=1 as well: \
                 the pinned expectation is only rewritten on purpose"
            );
            notice(&format!(
                "WARNING: rewriting the pinned retrieval-baseline expectation at {}",
                fixture(EXPECTED).display()
            ));
            let document = serde_json::json!({
                "schema": "wdbx.evaluation.expected.v1",
                "corpus_id": report.measurement.corpus_id,
                "corpus_sha256": report.measurement.corpus_sha256,
                "outcome": report.outcome,
                "measurement": report.measurement,
            });
            let mut bytes = serde_json::to_vec_pretty(&document).expect("pretty JSON");
            bytes.push(b'\n');
            std::fs::write(fixture(EXPECTED), bytes).expect("write the pinned expectation");
        } else if std::env::var_os("WDBX_WRITE_EXPECTED").is_some() {
            notice(
                "WARNING: WDBX_WRITE_EXPECTED is set but is not \"1\": the pinned \
                 expectation is NOT rewritten",
            );
        }

        let expected_bytes =
            std::fs::read(fixture(EXPECTED)).expect("the pinned expectation exists");
        let expected: ExpectedMetrics =
            serde_json::from_slice(&expected_bytes).expect("the pinned expectation parses");

        assert_eq!(expected.schema, "wdbx.evaluation.expected.v1");
        assert_eq!(report.measurement.corpus_id, expected.corpus_id);
        assert_eq!(report.measurement.corpus_sha256, expected.corpus_sha256);
        assert_eq!(report.outcome, expected.outcome);
        assert_eq!(
            report.measurement, expected.measurement,
            "the measured numbers drifted from the pinned expectation"
        );
    }

    /// The ablation must name every dimension, and it must be able to say that
    /// a dimension carries no benefit.
    #[test]
    fn the_ablation_lists_all_eight_dimensions() {
        let report = abi_wdbx::evaluation::run(&config()).expect("run");
        let labels: Vec<&str> = report
            .measurement
            .ablation
            .iter()
            .map(|row| row.dropped_dimension.as_str())
            .collect();
        let mut expected: Vec<&str> = EvidenceDimension::all()
            .iter()
            .map(|dimension| dimension.label())
            .collect();
        expected.sort_unstable();
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted, expected,
            "the ablation must drop each dimension once"
        );
        assert_eq!(labels.len(), 8);
    }

    /// The harness must be able to say "no benefit".
    ///
    /// Dropping every dimension turns the constitutional arm back into the plain
    /// vector arm, so the rates must match exactly and the verdict must be
    /// `no_benefit`. If the harness could only ever say "benefit", it would be
    /// a machine for manufacturing one.
    #[test]
    fn the_harness_can_report_no_benefit() {
        let mut blind = config();
        blind.dropped_dimensions = EvidenceDimension::all().to_vec();
        let report = abi_wdbx::evaluation::run(&blind).expect("run");
        assert_eq!(
            report.outcome, "no_benefit",
            "an arm that sees no dimension must report no benefit"
        );
        assert_eq!(
            report.measurement.arms.constitutional.repeated_failure_rate,
            report.measurement.arms.baseline.repeated_failure_rate
        );
        assert_eq!(
            report.measurement.arms.constitutional.stale_reuse_rate,
            report.measurement.arms.baseline.stale_reuse_rate
        );
        assert!(
            report
                .measurement
                .verdict
                .statement
                .contains("did not reduce")
        );
    }

    /// The constitutional arm must not present an uncertain record as usable.
    #[test]
    fn the_constitutional_arm_never_presents_an_uncertain_record_as_usable() {
        let report = abi_wdbx::evaluation::run(&config()).expect("run");
        for family in &report.candidate_output {
            let Some(disposition) = family.disposition.as_deref() else {
                continue;
            };
            if family.is_failure {
                assert_ne!(
                    disposition,
                    Disposition::Accepted.label(),
                    "{} returned a failure record as accepted",
                    family.family_id
                );
            }
            assert!(
                !family.explanation.is_empty(),
                "{} returned a record with a disposition but no explanation",
                family.family_id
            );
            assert_eq!(
                family.dimensions.len(),
                EvidenceDimension::all().len(),
                "{} must report all eight dimensions",
                family.family_id
            );
        }
    }

    /// Both arms must receive exactly the same candidates, and the shared tenant
    /// filter must actually remove something.
    #[test]
    fn both_arms_receive_the_same_candidates_after_the_shared_filter() {
        let report = abi_wdbx::evaluation::run(&config()).expect("run");
        for (baseline, candidate) in report
            .baseline_output
            .iter()
            .zip(report.candidate_output.iter())
        {
            assert_eq!(
                baseline.candidates, candidate.candidates,
                "{}: the arms must see the same surviving candidates",
                baseline.family_id
            );
            assert_eq!(
                baseline.candidates_before_filter.len(),
                report.measurement.candidate_count_before_filter,
                "{}: the frozen candidate count must hold",
                baseline.family_id
            );
            assert!(
                baseline.candidates.len() < baseline.candidates_before_filter.len(),
                "{}: the shared filter must remove the other tenant's record",
                baseline.family_id
            );
            for record_id in &baseline.candidates_before_filter {
                if !baseline.candidates.contains(record_id) {
                    assert!(
                        record_id.ends_with("-x"),
                        "{}: only the cross-tenant record may be filtered out",
                        baseline.family_id
                    );
                }
            }
        }
    }
}
