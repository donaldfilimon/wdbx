# Frozen local baseline comparison harness — retrieval falsification receipt

2026-10-21. The frozen local baseline comparison harness for evidence-aware
retrieval is implemented and green on the canonical checkout. It is the
falsification harness requirement `P04.03` asks for: a plain vector retrieval
arm and an evidence-aware arm over the *same* frozen corpus, the *same*
frozen embeddings, the *same* storage and the *same* candidate count,
measured on repeated failures and stale-policy reuse, with the negative
results retained. The verdict is reported whether or not the evidence-aware
arm wins, and the harness can report "no benefit" — it does so when every
dimension is suppressed.

The harness lives in new files only: `crates/abi-wdbx/src/evaluation/`
(`mod.rs`, `corpus.rs`, `ledger.rs`, `constitution.rs`, `metrics.rs`,
`report.rs`, `harness.rs`), the integration test
`crates/abi-wdbx/tests/v3_retrieval_baseline.rs`, and the frozen fixtures in
`crates/abi-wdbx/tests/fixtures/retrieval-baseline/` (`corpus-v1.json`,
`expected-metrics-v1.json`, `README.md`, `generate_corpus.py`). The only
pre-existing file touched is `crates/abi-wdbx/src/lib.rs`, by the single
line `pub mod evaluation;`. Nothing was committed.

## The two arms

The **baseline arm** is plain cosine retrieval over
`crate::index::ExactIndex`, the crate's deterministic exact-search backend.
It sees vectors and nothing else: no disposition, no policy binding, no
edge state. It cannot know that a record was written under an older policy,
contradicted, quarantined, forgotten or never signed, so it returns them
*as if they were usable*.

The **constitutional arm** is the canonical constitutional read model,
`crate::v3::episode::EpisodeStore::query` (`crates/abi-wdbx/src/v3/episode/
store/query/`). The harness asks that query once per run, indexes the
answers by episode digest, and reads the disposition, the eight dimensions,
the lineage and the explanation out of them. It owns no second
constitutional implementation. The eight dimensions are never collapsed into
one number: the ranking uses an explicit, named disposition tier and breaks
ties by the same cosine the baseline used, and every selected record carries
all eight dimensions separately with a per-dimension reason.

One place restates the canonical rules:
`evaluation::constitution::disposition_with` is the *ablation view*, needed
because the canonical query has no way to suppress one dimension. The
harness therefore checks, on every record of every family, that the
ablation view with nothing suppressed reproduces the canonical disposition
exactly; a change in the canonical rules fails the run
(`EvaluationError::ConstitutionalDrift`) instead of silently changing a
number.

## The frozen corpus

`crates/abi-wdbx/tests/fixtures/retrieval-baseline/corpus-v1.json`,
SHA-256 `0a8567711d920fb24ac07cc6b9c79a8d7de8e2ff7bc1dffe17662b82891b8f9d`.

Twelve query families, 160 records, synthetic tenant `synthetic-guild`
(and `synthetic-guild-alt` for the tenant-filter test). Every embedding is
a literal list of numbers in the fixture; the harness never re-embeds,
never learns and never re-normalises. The corpus is adversarial by
construction: each family is a cluster of near-duplicate records about one
fact, the query vector is the *old* phrasing of that fact, and the records
that keep the old phrasing are the ones a vector index ranks first. That
is the repeated-failure and stale-reuse failure mode under test, and it is
what makes the baseline fail (the non-vacuity requirement).

Hand-authored mix, one defect per record:

| Role | Count | Canonical disposition |
| --- | --- | --- |
| `healthy_current` | 2–3 | `accepted` |
| `stale_policy` | 2–4 | `uncertain` (stale policy binding) |
| `contradicted` | 2 | `contradicted` |
| `quarantined` | 1–2 | `quarantined` |
| `forgotten` | 1 | `rejected` (a tombstone erased it) |
| `unsigned` | 1 | `accepted` (the query accepts an unsigned record on its commitment) |
| `incomplete_lineage` | 1 | `uncertain` (its superseder is quarantined, so the chain does not resolve) |
| `wrong_regime` | 1 | `uncertain` (committed maturity is specification-only) |

The ledger is real: the harness appends the corpus to an `EpisodeStore` in
three sessions (the stale-policy records, written under the older policy and
signed under it as well; the current-policy records that nobody signed; then
everything else, signed by the canonical writer, together with the memory edges
and the forgetting tombstones) and reads every fact back out of that ledger.
The digests, the edges, the lineage and the signature states are the store's
own.

Candidate count: **12 before the shared filter**, identical for both arms.
The shared filter (tenant, scope, cryptographic rejection) is applied
identically to both and removes the other tenant's record, so both arms
rank the same 11 survivors. The index's binary32 ordering was checked
against a binary64 cosine reference for every family: they agree
(`binary64_ordering_agrees: true`).

The pinned expectation is
`crates/abi-wdbx/tests/fixtures/retrieval-baseline/expected-metrics-v1.json`,
SHA-256 `3e9e483d8e064c517590dde03af5a6e576f8b394d6458b86eb0b64567fb8b14b`.
Regenerate it deliberately, with both opt-ins, with
`WDBX_WRITE_EXPECTED=1 WDBX_REGENERATE_EXPECTED=1 cargo test -p abi-wdbx --test v3_retrieval_baseline`;
the rewrite prints a `WARNING:` line on stderr, `WDBX_WRITE_EXPECTED=1` alone
is refused, and the repository gate unsets both variables so the gate always
asserts the pin instead of rewriting it.

## Commands

```sh
PATH="$HOME/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:$PATH" \
  cargo test -p abi-wdbx --test v3_retrieval_baseline
PATH="$HOME/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:$PATH" \
  cargo clippy --workspace --all-targets
PATH="$HOME/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:$PATH" \
  cargo fmt --all --check
bash tools/check_rust_sizes.sh
```

Environment: rustc 1.99.1 (58cf8f9c3 2026-08-27 pin, `rust-toolchain.toml`),
host `aarch64-apple-darwin`, macOS 27.2, working tree at commit
`e54a75e54577ddf8835704be3d45ca19b37410aa` plus this slice's uncommitted
files and the concurrent constitutional-query slice's uncommitted files.

## Measured numbers

Twelve families; every rate is `count / 12`.

| Metric | Plain vector baseline | Constitutional arm |
| --- | --- | --- |
| `repeated_failure_rate` | **0.666667** (8/12) | **0.000000** (0/12) |
| `stale_reuse_rate` | **0.666667** (8/12) | **0.000000** (0/12) |
| `strict_failure_rate` | 0.666667 (8/12) | 0.000000 (0/12) |
| `unsigned_reuse_rate` | 0.083333 (1/12) | **0.583333 (7/12)** |
| `applicability_accuracy` | 0.250000 (3/12) | 0.416667 (5/12) |

Relative reductions (baseline − candidate) / baseline:
`repeated_failure` **1.000000**, `stale_reuse` **1.000000**,
`strict_failure` **1.000000**. The frozen thresholds are 0.25 on both
headline rates, so both are met.

Paired family-cluster bootstrap, 10 000 resamples, seed 104 729, over the
headline metrics: failure gain mean 0.666667, lower two-sided 95 % bound
**0.416667**; stale gain mean 0.666667, lower bound **0.416667**. Both
lower bounds are strictly positive at this corpus size.

Definitions, stated so the numbers cannot be read more generously than they
are:

* `repeated_failure_rate` — the share of families whose selected record is
  contradicted, quarantined, forgotten or stale-policy **and** which the
  arm presents as usable. The baseline has no disposition, so it presents
  everything as usable; the constitutional arm presents a record as usable
  only when its canonical disposition is `accepted`.
* `stale_reuse_rate` — the same, restricted to stale-policy records.
* `strict_failure_rate` — the share of families whose selected record is a
  failure record *at all*, whatever disposition it carries. It is reported
  next to the headline rate so that a disposition-marked answer cannot be
  mistaken for a clean one.
* `unsigned_reuse_rate` — the share of families whose selected record
  nobody signed.
* `applicability_accuracy` — the share of families whose selected record is
  one of the corpus's `healthy_current` records.
* An abstention counts as a failure in `repeated_failure_rate` and
  `strict_failure_rate`: the frozen protocol says an abstention is a
  failure, never an improvement.

## Per-dimension ablation

Each dimension is dropped in turn and the whole constitutional arm is
re-measured. `changed` is the number of families whose selected record
moved.

| Dropped dimension | `repeated_failure_rate` | `stale_reuse_rate` | `strict_failure_rate` | `unsigned_reuse_rate` | changed | carries the benefit |
| --- | --- | --- | --- | --- | --- | --- |
| cryptographic | 0.000000 | 0.000000 | 0.000000 | 0.583333 | 0 | no |
| source | 0.000000 | 0.000000 | 0.000000 | 0.583333 | 0 | no |
| calibration | 0.000000 | 0.000000 | 0.000000 | 0.583333 | 0 | no |
| outcome | **0.500000** | 0.000000 | 0.500000 | 0.083333 | 6 | **yes** |
| compatibility | **0.666667** | **0.666667** | 0.666667 | 0.083333 | 8 | **yes** |
| regime | 0.000000 | 0.000000 | 0.000000 | 0.583333 | 0 | no |
| lifecycle | **0.500000** | 0.000000 | 0.500000 | 0.083333 | 6 | **yes** |
| confidence | 0.000000 | 0.000000 | 0.000000 | 0.583333 | 0 | no |

Reading the table:

* **compatibility** carries the stale-policy benefit. Dropping it returns
  the failure rate to the baseline's 0.666667 and the stale reuse rate with
  it, and moves 8 of 12 selections.
* **outcome** and **lifecycle** carry the contradiction and quarantine
  benefit: dropping either returns 6 of 12 selections to a failure record.
* **cryptographic** carries no benefit on this corpus, because the
  canonical query accepts an unsigned record on its commitment: the
  dimension reports the missing signature but never withholds the record.
  That is the same fact the negative result below reports.
* **source**, **calibration**, **regime** and **confidence** carry no
  benefit at all. The ledger holds no calibration record, no task-regime
  field, no uncertainty interval and no reputation signal, so those four
  dimensions read `unknown` for every record. The harness reports that as a
  finding about the store, not as a measurement.
* The wrong-regime records are therefore caught by the *confidence*
  dimension (a committed maturity of specification-only), not by a regime
  check, because the ledger has no regime field.

## Honest verdict

**On the two metrics requirement P04.03 names, the constitutional arm beat
the plain vector baseline on this frozen corpus**: the repeated-failure rate
and the stale-policy reuse rate both fell from 0.666667 to 0.000000, a
relative reduction of 1.0 against a frozen threshold of 0.25, with the
paired bootstrap's lower 95 % bound strictly positive at 0.416667. The
verdict word in the pinned expectation is `benefit`.

**And the run also produced a negative result that the verdict carries**:
the constitutional arm handed back an *unattributed* record in 7 of 12
families, against 1 of 12 for the plain vector baseline. The canonical
query accepts an unsigned record on its commitment and reports the missing
signature in its cryptographic dimension without withholding the record, so
an arm that trusts the disposition ends up returning the old, unsigned
near-duplicate more often than the plain vector arm does. The
`applicability_accuracy` moved only from 0.250000 to 0.416667. The benefit
is real on the two named metrics and it is bought partly with records the
caller cannot attribute.

The harness can report "no benefit" and does: suppressing every dimension
turns the constitutional arm back into the plain vector arm, the rates
match exactly, and the verdict becomes `no_benefit`
(`the_harness_can_report_no_benefit`). The harness is therefore not a
machine for manufacturing a win.

The verdict's `caveats` list in the machine-readable record carries the
negative result, and `criteria.unsigned_reuse_not_worse` is `false`.

## Evidence boundary

This is a **deterministic local C1 comparison over a synthetic frozen
corpus**. It is:

* **not a research result.** Twelve families, one synthetic corpus, one
  machine, no independent runner, no held-out data, no contamination
  control, no blind raters. The preregistered protocol in
  `docs/completion/experiments.json` asks for 100 scenario families with
  three storage-order variants each, a train/validation/test split, an
  independent runner holding the test split and the oracle artifact, and
  10 000 family-cluster bootstrap resamples over *unique* families with the
  variants averaged. None of that is here.
* **not a production claim.** The corpus is adversarial by construction and
  the records are synthetic. A real corpus may show no benefit at all.
* **not evidence that evidence-weighted retrieval beats a vector database in
  production.** Nothing here measures production traffic, real latency, real
  memory, real users or real embeddings.
* **not a conformance result.** The harness reads the canonical
  constitutional query; it does not certify it. `docs/completion/README.md`
  keeps the C0 proposal/preregistration boundary, and requirement `P04.03`
  stays `proposed` with an empty evidence array: this receipt is a step
  towards it, not its closure.
* **not independent review.** No reviewer outside this session has read the
  harness, the corpus or the numbers.

The corpus is frozen, the measurement is deterministic, and the negative
results are retained. What the numbers support is exactly this: on one
synthetic frozen corpus, with one store, one deterministic index and one
canonical constitutional query, the evidence-aware arm returned no
contradicted, quarantined, forgotten or stale-policy record as usable,
where the plain vector arm returned one in two families out of three — and
it returned more unattributed records doing it.

## What is not implemented

* The harness does not rebuild a disposable index from a canonical snapshot
  (`rebuild(IndexVersion, CanonicalSnapshot) -> IndexDigest` in
  `docs/completion/plans.json`); the index is rebuilt in memory from the
  frozen embeddings on every run, which is deterministic but is not the
  planned index-rebuild interface.
* The harness measures no latency, no peak memory and no wall clock; those
  fields are `null` in the machine-readable record rather than invented.
* The harness reports no commit and no dirty paths for the same reason.
* The corpus is not independently held out and the oracle is not
  independently custodied, so the protocol's contamination controls are
  unmet.
* The three dimensions the ledger does not populate (calibration, regime,
  and the uncertainty half of confidence) carry no benefit here and cannot
  until the ledger holds the corresponding evidence.

[Source inventory](../../completion/requirements.json) binds the
requirement this slice answers (`P04.03`, plan `task-04`,
`docs/completion/sources/episodes.md` §"Falsification").
