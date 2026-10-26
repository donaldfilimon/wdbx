# Task8.scoring independent review

Spec compliance verdict: **PASS for the bounded scoring slice**.

Code quality verdict: **PASS**. No actionable Critical, Important or Minor defect
was established in the reviewed tranche. This is not completion of the wider
scoring/orchestration design or a promotion from C0.

## Scope and method

Reviewed the task brief, worker report and the supplied exact
`task-8-scoring-review.diff` against the operational profile and frozen scoring
entry in `docs/completion/designs.json:38-51`. The supplied diff is the review
boundary; unrelated dirty work and the rest of dirty HEAD were not reviewed.
Inspected all eight scoring modules, three new test targets and the minimal
`lib.rs` registration. All 12 current product/test files matched the SHA-256 and
line counts in the supplied `source-freeze.json` when checked during this review.
This establishes agreement with that snapshot, not independent authenticity of
the captured baseline or evidence.

Read Studio AGENTS and the runtime profiles, and checked the relevant architecture
9/11 requirements. The operational profile is the scoped specification; the older
runtime profile's legacy defaults do not replace the frozen v3 scoring defaults.
The profile's clarification at lines 71-73 was disclosed by the coordinator during
review: additional typed inhibition facts are bounded host provenance/equality
declarations. This review does not infer an executable inhibition interpreter or
capability authorization from those facts.

No tests, builds, formatters or git commands were rerun. No source fix, staging,
commit, install, network access, holdout access or live-store access was performed.
The only review mutation is this report.

## Spec evidence

- Exact policy and defaults: `scoring_v3/policy.rs:24-40,76-88,114-176` retains
  Rational values through negative-only ATP dampening, baseline clamping,
  baseline-qualified exceptions, independent jitter, final clamping and inclusive
  gating. Strength is a separate refusal and is absent from confidence arithmetic.
  The defaults are threshold 50/cap 1 and brainstorm cap 5; max strength is 100.
  `math.rs:13-46` uses checked rational multiplication/comparison. Overflow returns
  an error before the public caller commits the tape position.
- Numeric/scanner distinction: `policy.rs:90-113` supplies explicitly numeric
  helpers; `contribution.rs:128-137` obtains contribution confidence from an actual
  scan of the immutable supplied PreparedAction pair. Externally supplied evidence
  cannot construct a Contribution. `policy.rs:180-255` checks scanner version,
  component names and weights, complementary fractions/reasons, span direction,
  aggregate totals and constraint consistency.
- Draw discipline: `draws.rs:33-86` bounds identity/draw use and labels actual
  positions and purposes; `policy.rs:131-156` consumes two independent draws only
  on the eligible path, including a sign draw for zero sampled magnitude. The
  no-draw trace retains identity. `draws.rs:89-104` has explicit bounded state and
  no entropy, clock or I/O.
- Alternative elections: `election.rs:38-104` validates every weight, including
  ineligible slots, excludes inhibited slots, abstains at zero total, avoids a
  singleton draw, preserves stable slot order and closed-grid endpoints, and uses
  finite maximum-normalized compensated sums. Original weights/refusals/draw and
  winner are retained. This floating-point use is confined to declared weights.
- Private contribution and bindings: `contribution.rs:44-111` exposes immutable
  getters, with no public field constructor or Deserialize path. Its provenance
  preserves the concrete record, source origin/text, target origin, captures,
  guards, lineage, slot, evidence, score and election. Preflight at lines 231-333
  validates policy, bounds, finite alternative weights, duplicate metadata,
  source origin/span/token spans/guards and explicit target node/entry/origin/
  original/key bindings. It does not reprepare either input or resample fanout.
- Narrow bypass and guards: `contribution.rs:138-185,213-229,308-311` removes only
  BelowThreshold for a declared wildcard bypass, retains actual confidence and
  all other gates, refuses blocked automata, and prevents inhibited alternatives
  from producing a contribution. The selected legacy Expr is an inert projection.
- Exact scoped redundancy: `arbitration.rs:15-75` compares typed tree tags,
  original bytes, ordered arguments, chunk root, parent lineage, sorted typed
  inhibition facts, ordered named typed captures and dependency sets. Concrete
  node identity, individual fanout/rule refs, capture spans and alternative
  election weight do not enter equality. `arbitration.rs:94-147` rejects duplicate
  IDs, mixed populations, conflicting repeated EntryRef snapshots, inconsistent
  node bindings and missing references, while exempting attached contributions,
  intermediate facts and referenced dependencies. Multiple qualified entries are
  otherwise preserved. Lines 149-208 restrict elections to actual redundant
  groups, use max(0,Ceff)*strength, abstain at zero total, preserve stable output
  order and commit the tape only after success.
- Bounds and transactionality: `budget.rs:24-77,101-218` checks collection lengths,
  strings, finite weights, recursive depth and structural nodes (including empty
  argument containers), and charges work/payload before owned copies.
  `contribution.rs:335-384` preflights a full batch and rolls back tape progress
  on any later error; arbitration also uses a private cursor until success.
  Conservative accounting may refuse below nominal maxima, as disclosed.

The new fixtures directly exercise the requested S07/S08 policy order and exact
threshold neighbors (`tests/scoring_v3_policy.rs:18-233`), S09 alternatives and
endpoints (`tests/scoring_v3_election.rs:3-66`), and S09-S11 entry independence,
complementary/grouped contributions, fanout roots, exemptions, bypass/guard
refusals, stale binding, late invalid input, oversize trees/payloads, and repeated
entry inconsistency (`tests/scoring_v3_contributions.rs:128-738`). No source change
to producers, scanner, identity, legacy engines, saves, manifests or locks appears
in this exact tranche.

All source/test paths above are relative to
`specimen-studio/native/specimen-kernel/`.

## Focused lookups outside the diff

Only concrete dependency/API questions were followed into source:

- `src/scanner_v3/mod.rs:26-44`: checked whether the evidence-validation zip could
  silently ignore extra/missing components; Evidence uses a fixed four-element
  array. Its scanner feature representation at lines 45-120 also confirms typed
  evidence and complementary component construction.
- `src/scanner_v3/rational.rs:2-57`: checked that scoring's exact arithmetic uses
  a reduced, positive-denominator, privately represented Rational and checked
  additions/subtractions, rather than caller-forgeable invalid fractions.
- `src/identity/model.rs:33-110`: checked PatternKey payload size and the exact
  structural/bitwise record equality relied upon by repeated-entry checks; the
  key digest is fixed-size and record equality retains bind/alternatives/weights.
- `src/language_v3/arithmetic.rs:2-50`: checked whether Number captures could hide
  uncharged variable-length payloads; CanonicalDecimal stores only i128/u8.
- `src/language_v3/model.rs:263-340`: checked that PreparedAction exposes immutable
  getters and has private fields, and that guard/executable semantics match the
  adapter's assumptions.

These are dependency checks, not a review or modification of the producer modules.

## Supplied verification and qualifications

Parsed raw final logs under
`docs/completion/execution/evidence/2026-10-23-scoring/scoring/`: the focused log
contains 38 passed/0 failed across three summaries; the full-kernel log contains
195 passed/0 failed across 20 summaries. The Clippy log records a completed dev
check. Owned-format and diff-check logs are empty; their exit-zero status is
worker-reported, not independently rerun or recoverable from an empty transcript.
The worker report identifies pinned Rust 1.99.1, cached offline dependencies and
all-target Clippy with `-D warnings`.

The raw test logs retain rust-objcopy SIGABRT warnings caused by missing
`@rpath/libLLVM.dylib`. Test result summaries are still green; debug stripping is
not independently qualified. No warning suppression or environment repair was
performed by this review.

Cannot verify from this scoped review: native/WASM independent scoring parity,
full Studio/browser/desktop gates, physical memory/time measurements, scientific
efficacy, host authorization, current utility/context coherence, semantic validity
of host captures/inhibition/dependencies/attachment groups, production concurrency
or cancellation, or the authenticity of every retained RED/GREEN log. The source
does not certify those properties, and this review does not promote them. Queue,
attachment scheduling, composition, richer script integration and durable ATP
updates remain outside Task8.scoring. All 163 requirements remain open/C0/unknown.
