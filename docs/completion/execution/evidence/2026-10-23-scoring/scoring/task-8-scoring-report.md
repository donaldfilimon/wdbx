# Task8.scoring report — bounded opt-in local scoring

Status: implemented, uncommitted, ready for independent review. No staging, commits,
installs, network, holdouts, producer/scanner/identity changes, legacy engine/save
changes, Cargo manifest/lock changes, or substrate edits were performed by this
worker. Existing unrelated dirty work was preserved. Source/API freeze applies to
the exact product files listed in `source-freeze.json` below; subsequent changes
must be review fixes and receive new verification.

## Binding scope and product manifest

Read the task brief, scoring-orchestration-v3 profile, frozen designs.json scoring,
architecture sections 9/11, Studio AGENTS and both runtime profiles. The implemented
slice is confidence/randomness/entry alternatives/private contributions/scoped
redundancy. Queue, attachment scheduling and composition remain the next task.

Repository root: `/Users/donald/sources/repos/wdbx`.

New product modules under `specimen-studio/native/specimen-kernel/src/scoring_v3/`:
`mod.rs`, `math.rs`, `draws.rs`, `policy.rs`, `budget.rs`, `election.rs`,
`contribution.rs`, `arbitration.rs`. Existing `src/lib.rs` received only the minimal
`pub mod scoring_v3;` registration; its earlier module registrations are unrelated
pre-existing work. New integration targets: `tests/scoring_v3_policy.rs`,
`tests/scoring_v3_election.rs`, `tests/scoring_v3_contributions.rs`.

Absolute paths, SHA-256 and line counts for all 12 product/test files are recorded
in `docs/completion/execution/evidence/2026-10-23-scoring/scoring/source-freeze.json`.
The manifest is snapshot evidence, not a claim of source authenticity.

## Stable API for the next pure orchestration consumer

Import `specimen_kernel::scoring_v3::*`.

- `Atp`, `Policy`, `ScoringBudget`, `DrawTape`, `DrawIdentity`, `DrawRecord`,
  `DrawPurpose`, `JitterReason`, `Refusal`, `ScoreTrace`, `ScoringError`.
- `numeric_policy(scan: Rational, Policy, &mut DrawTape) -> Result<ScoreTrace>`;
  `numeric_balance(similarity, dissimilarity, Policy, &mut DrawTape)` explicitly
  provide numeric mechanism tests, not scanner/admission authenticity.
- `validate_evidence(&Evidence)` validates version, component names/weights,
  exact complementary fractions, reason/fraction consistency, aggregate totals,
  bounded spans, span ordering and constraint consistency. It cannot construct a
  contribution or authenticate caller data.
- `DrawTape::new(seed, stream, &[u32], limit)`, `consumed()`, `identity()`;
  `seeded_draws(explicit_state, count)` is a named bounded deterministic
  SplitMix64-derived u32 helper without entropy, time or I/O.
- `elect(weights, eligible, &mut DrawTape) -> Result<ElectionTrace>` is a bounded
  numeric election helper. `ElectionTrace` retains original slot weights,
  refused eligibility slots, random identity, actual draw and winner index.
- `EntryRequest` supplies contribution ID, immutable prepared source/target,
  `EntryRecord`, `CandidateOrigin`, explicit `TargetBinding {node, entry, origin}`,
  host parent lineage, typed inhibition facts, dependencies, attachment group,
  `ContributionKind::{Action,IntermediateFact}` and policy.
- `evaluate_entry(request, tape, budget) -> Result<EntryOutcome>` and
  `evaluate_entries(&[EntryRequest], tape, budget) -> Result<Vec<EntryOutcome>>`.
  Both scan the actual immutable prepared inputs; they never reprepare text or
  resample utility fanout. EntryOutcome carries policy/gate refusals, optional
  alternative election, and an optional private validated `Contribution`.
- `Contribution` has no public field constructor or Deserialize implementation.
  Immutable getters are `id`, `record`, `origin`, `source_original`,
  `target_origin`, `parent_lineage`, `inhibitions`, `dependencies`,
  `attachment_group`, `kind`, `slot`, `score`, `election`, `evidence`, `strength`,
  `action`. Node/entry/population are available through `record`; scanner version
  and both source/target guards through `evidence`; source IDs, fanout/rule refs,
  captures/spans and route metadata through `origin`. Random identity survives
  no-draw exceptions. A mutable public ScoreTrace cannot forge a Contribution.
- `arbitrate(&[Contribution], tape, budget) -> Result<Arbitration>` returns stable
  surviving contributions and per-actual-redundant-group `ArbitrationTrace`
  candidates/original arbitration weights/draw/winner. Missing dependency refs,
  duplicate contribution IDs, mixed populations, conflicting repeated concrete
  entry snapshots, and inconsistent node key bindings refuse atomically.
  Dependency cycles intentionally belong to the later composer.

## Exact policy and local choices

Confidence remains checked i128 Rational through Cscan, negative-only ATP penalty,
baseline clamp, jitter, inclusive threshold and endpoint decisions. Strength is
an independent >0 gate; it never alters confidence. Default cap 1, brainstorm 5;
explicit rational cap [0,5] and ATP fractions obey the <=1,000,000 denominator
bounds. Arithmetic overflow is a typed error with no result/cursor commit.

Magnitude uses the closed u32/u32::MAX grid and the next independent draw's low
bit supplies sign. Every used draw is labeled/indexed with seed and stream. Zero
cap/disabled/endpoint/max-strength-qualified exceptions consume no jitter draws;
a sampled zero magnitude still consumes its independent sign. No durable ATP or
baseline drift is introduced. Numeric confidence is not treated as probability.

Sampling validates all weights including inhibited slots. Zero total abstains;
one positive eligible slot consumes no draw. Multiple positives use finite
maximum-normalized compensated proportional sums. Explicit grid endpoints choose
the first/last positive slot, including extreme subnormal weights; internal
intervals are half-open. f64 is confined to declared sampling weights and the
specified post-confidence arbitration conversion.

Redundancy key is chunk root (input/clause/action), explicit parent lineage,
canonical selected inert Expr tags/original name/literal bytes/ordered arguments,
sorted typed Input/Target inhibition facts, ordered named typed capture values,
and dependency set. Capture spans remain provenance only. Concrete node identity,
alternative election weight and individual fanout/rule refs are excluded from
canonical equality. Fanout siblings group; attached groups, IntermediateFact
values, and any referenced dependency contribution are exempt. Different actions,
parents, captures, inhibition facts, trees or dependency sets survive. Every entry
may contribute; no global or one-entry-per-node election exists. A zero-total
actual redundant group emits no action, while nonredundant bypass contributions
retain their real confidence, including negative values.

A declared wildcard bypass removes only BelowThreshold; it never substitutes 100,
removes strength/guard/alternative inhibition, or turns a blocked automaton into a
successful route. Structural/automaton paths cannot claim wildcard bypass.
Source origin/span/token-spans/guards must match the immutable PreparedAction;
target node/entry/origin/original and current structural key must match the explicit
record binding. No implicit target re-preparation occurs.

All complete collection metadata and bounded trees/strings/vectors are charged
before keys/output copies; slice lengths reject huge inputs early. Ceilings are
100,000 contributions, 1,000,000 logical work, 16 MiB owned payload and 100,000
draws; caller budgets may lower entry/batch/arbitration limits. Tree depth is 16
and structural nodes 4096, counting argument containers (including empty ones)
so a single Expr cannot hide a huge argument vector. Leaf name/text/byte payloads
are bounded at 8000 bytes. Conservative reservations can refuse a collection below
a nominal ceiling; no silent truncation occurs. Draw cursor commits only on total
success; batch failures roll it back and return no partial vector.

## Verification and raw evidence

All commands ran from repository root with:

```
PATH=/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:/usr/bin:/bin:/opt/homebrew/bin
RUSTUP_TOOLCHAIN=1.99.1
CARGO_NET_OFFLINE=true
```

Final commands/results (each exited 0):

```
cargo test --manifest-path specimen-studio/Cargo.toml -p specimen-kernel --locked \
  --test scoring_v3_policy --test scoring_v3_election --test scoring_v3_contributions
# 26-focused-freeze.log: 38 passed (11 policy, 5 election, 22 contribution)
cargo test --manifest-path specimen-studio/Cargo.toml -p specimen-kernel --locked
# 27-full-kernel-freeze.log: 195 passed, 0 failed across 20 result summaries
cargo clippy --manifest-path specimen-studio/Cargo.toml -p specimen-kernel \
  --all-targets --locked -- -D warnings
# 28-clippy-freeze.log: passed
rustfmt --check --edition 2024 \
  specimen-studio/native/specimen-kernel/src/scoring_v3/*.rs \
  specimen-studio/native/specimen-kernel/tests/scoring_v3*.rs
# 29-owned-fmt-freeze.log: passed
git diff --check -- <owned paths and the minimal lib.rs registration>
# 30-diff-check-freeze.log: passed
```

Every raw log is NEW under
`docs/completion/execution/evidence/2026-10-23-scoring/scoring/`.
The existing rust-objcopy SIGABRT / missing `@rpath/libLLVM.dylib` debug-stripping
warning remains visible in test logs; test commands still exit 0. No warning
suppression, install or environment repair was attempted. Full Studio/browser,
WASM scoring traces and host integration gates were not run by this worker.

Representative behavioral RED/GREEN witnesses retained:

- 01 -> 02: ATP fixture initially remained 55, then correctly produced 50 and
  qualified inclusively.
- 04 -> 05: absent private contribution/binding refusal, then actual scanner entry
  contribution and stale binding rejection.
- 07 -> 08: redundant inputs initially all survived [1,2,3], then correctly [2,3].
- 11 -> 13/14: oversized externally supplied span vectors initially accepted,
  then refused; final focused run includes the regression.
- 12 -> 13/14: zero-grid draw with a subnormal first weight initially selected the
  wrong slot, then correctly selected the first positive.
- 19 -> 20: huge empty-argument vector initially survived, then refused before copy.
- 25 -> 26: conflicting concrete EntryRef snapshots initially survived, then refused.

Adverse/intermediate runs are retained rather than omitted. 03 had a test temporary
borrow compilation error, fixed with an owned draw array. 06 accidentally selected
zero tests with a shortened exact filter; 07 reran the complete exact name and
provided the real behavioral RED. 10 exposed incorrect hand-entered seeded witness
numbers; an independent standard-library Python integer computation confirmed the
correct SplitMix64 vector before correction. 13 also exposed a fanout test whose
one-word confidence was negative (a legitimate zero-total election); its synthetic
fixture was changed to an arithmetic-bearing positive-confidence example. 16 had
two local Clippy style findings, fixed before 18/22/28 passed. 17 is a missing-field
compile RED for no-draw random identity, not a behavioral RED. The implementation
used representative test-first behavioral cycles plus test expansion; the logs are
not a claim that every initial helper had an independent behavioral RED before
its first implementation.

## Self-review and limits

All fixtures are newly authored synthetic mechanism data. They provide exact
policy/grouping/budget evidence only, not scientific efficacy, heldout behavior,
production authorization, host concurrency, independent native/WASM scoring parity,
or runtime admission integration. All163 requirements remain open/C0/unknown.

The legacy Expr action is an inert bounded projection. Existing alternative
inhibition is a boolean, and additional Input/Target inhibition facts are explicit
host declarations used for provenance/equality; this module does not interpret a
richer executable inhibition language or compile/execute v3 scripts. Action call
names are structurally compared, not authorized as capabilities.

PreparedAction authenticity/current utility/context coherence, parent branch IDs,
route/bypass authorization, capture meaning, dependency semantics and attachment
group correctness remain host responsibilities. Captures/inhibition facts are
bounded typed declarations, not authenticated proof. Actual immutable scanner
guards and explicit record/origin bindings are enforced locally, but the public
route/record types cannot certify host authorization. The target projection
requires its supplied prepared action original bytes to equal record.original;
compound-target decomposition needs explicit host policy, not an implicit parser
or fanout resample here. No live store, default runtime dispatch, producer semantics,
legacy IDs, saves or frozen contract file was changed.

No known remaining local test failure. Independent review, pure orchestration,
composition, opt-in native/WASM scoring trace comparison and final full Studio
verification remain outside this slice's completed evidence.
