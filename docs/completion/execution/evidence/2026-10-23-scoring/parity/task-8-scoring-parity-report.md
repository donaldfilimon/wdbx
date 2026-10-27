# Task8.scoring-parity report

Status: opt-in finite synthetic witness implemented and frozen for root's next
canonical-budget correction and independent review. The selected semantic traces
pass native/WASM parity and independent arithmetic/transition assertions. This is
not general native/WASM tight-budget admission parity. Root identified the
architecture-dependent logical accounting issue described below and owns that
separate kernel correction. No kernel source was edited by this worker.

## Files and API execution

Owned product files: five new modules in
`specimen-studio/native/specimen-wasm/src/scoring_probe/` (`mod.rs`, `project.rs`,
`numeric.rs`, `entries.rs`, `orchestration.rs`), new native example
`examples/scoring_probe.rs`, feature integration test `tests/scoring_probe.rs`,
`scripts/check-scoring-parity.sh`, and `tests/scoring-parity.mjs`. The existing
WASM manifest and lib receive only the new `scoring-probe` feature/example/module
and gated raw export registrations; `registration.diff` compares them with the
pre-parity snapshot rather than dirty HEAD. No default dispatch changes.

`source-freeze.json` names and hashes all eleven owned/registration files.
`20-source.sha256` additionally captures the actual scorer/orchestration and
identity/language/text/scanner dependencies used by the retained execution.
The source freeze is a local integrity snapshot, not source authentication.

The producer calls actual reviewed APIs: `numeric_policy`, `elect`,
`seeded_draws`, immutable language `prepare`/`text_v3::encode`,
`evaluate_entries`/`evaluate_entry`, `arbitrate`, `Broker::begin/admit/dispatch/
complete/close_discovery/seal/cancel/restart/acknowledge_fence/retire_cycle/compose`
and `Broker::attach`. Private validated contributions and private creation handles
come from those APIs. Output fields are immutable getter projections; no kernel
serde implementation, public contribution constructor or handle seam was added.

All inputs are newly authored bounded synthetic fixtures. Numeric mechanism
results are explicitly separate from actual immutable prepared-entry scanner
results. Every u128 ID, u64 seed and Rational i128 numerator/denominator is a
canonical decimal string. Original f64 alternative/arbitration weights retain
raw sixteen-digit bit patterns, including negative zero; draws remain u32 numbers.

## Scenarios and independent assertions

- Eighteen named numeric scenarios independently reconstruct exact checked-profile
  confidence, negative-only ATP penalty, clamp, inclusive threshold, strength as
  an independent gate, default50/maximum100, cap1/brainstorm5/fractional cap,
  threshold neighbors, displaced scanner endpoint, protected endpoints,
  crystallization, subthreshold strength100 jitter, disabled/zero-cap paths,
  independent sign/magnitude and clamp. BigInt rational arithmetic validates every
  score field, refusal, consumed cursor and labelled seed/stream/index/purpose.
  Sampled zero magnitude still consumes its sign draw. Six SplitMix-derived u32
  draws are independently reproduced using masked 64-bit BigInt arithmetic.
- Six alternative election scenarios cover zero-total abstention/no draw,
  singleton/no draw, excluded slots, first/last closed-grid endpoints and one
  safe interior integer-weight interval. JS verifies finite nonnegative original
  f64 bits and reconstructs proportional selection. It does not claim exhaustive
  independent validation of every compensated f64 boundary.
- One actual twelve-entry `evaluate_entries` batch scans immutable prepared inputs.
  Full four-component evidence is retained with names, versions, exact fractions,
  reasons and spans. JS checks complementary components and exact S/D/difference/
  confidence totals. Twelve actual contributions precede arbitration; eleven
  survive. Two real prepared utility-fanout siblings with equal root, parent,
  inert selected Expr and typed captures form one redundant group. Distinct
  fanout/rule refs and capture spans remain provenance and do not affect grouping.
  JS independently reconstructs keys from full typed Expr/capture/inhibition/root/
  dependency inputs, computes arbitration weights and winner, and checks stable
  surviving metadata. Complementary same-node entries, different parent, Input
  versus Target facts, typed decimal versus byte captures, attached/intermediate
  contributions and referenced dependencies remain.
- Five actual entry refusal/bypass scenarios retain real low confidence with and
  without wildcard bypass, show unresolved guards survive bypass, refuse a blocked
  automaton and return typed BindingMismatch for a stale explicit entry binding.
  Error context is explicitly probe-owned; the discriminant is from the kernel.
- The shared-cap broker scenario admits ten foreground and two maintenance jobs.
  Every one is actually scored before completion; these are not twelve synthetic
  `complete(None)` reports. Active capacity2 is shared, per-queue FIFO and 8:1
  service are independently replayed. Early seal/compose refuse. Twelve jobs drain
  before close/seal/compose. Duplicate leases refuse with unchanged counters.
- Cancellation/restart uses actual private handles and leases: pending work is
  removed, one active permit remains quarantined, late actual scored results
  cannot publish, dispatch remains blocked, stale owner and wrong worker
  acknowledgements refuse, current-owner/old-worker acknowledgement releases once,
  duplicate and post-fence late results refuse, retirement plus reused cycle ID
  rejects the old creation handle, and wrong population refuses. The ordered
  twenty-command replay checks all active/pending/quarantined counters and errors.
- A qualified issuer schedules a hard attachment into the same broker. Two target
  entries are actually evaluated: a below-threshold target produces no vote and
  a qualified target survives. Close discovery is blocked until both drain.
  Composition preserves issuer-before-dependent order, both concrete nodes,
  attachment group, remainder UTF8 span, inherited guards, parent and captures.
  JS sums actual evidence even on the refused target; hard scheduling never
  substitutes a confidence or forces admission.
- The dedicated cleanup boundary uses operation_work8, two retained cycles, no
  jobs and E=0. Independent R=4*C+4*J+E equals8; a third cycle refuses atomically,
  restart retains the reservation, retirement permits retry. Observed before/after
  owned bytes are equal. General transcripts assert only structural cleanup lower
  bounds; they do not independently reproduce architecture-dependent nested byte
  charges or assert allocator/instruction equivalence.

Strict envelope/field/scenario/order/count checks prevent omission-based passes.
The same normal semantic validator is run on native and feature-WASM outputs.
Eight omitted/unknown/invalid fraction/cursor/scenario cases plus malformed JSON
are refused in the comparator self-test. Two separate mutated raw envelopes are
then passed through the same normal `--check-json` mode in separate processes:
exact effective confidence corruption and a quarantined-capacity transition
corruption both exit1. Their raw inputs, assertion output and SHA-256 are retained.

Kernel-only boundaries remain separate evidence from reviewed scorer/orchestration
tests: overflow/extreme f64/metadata limits, larger queue boundaries, soft-affinity
endpoints/depth and other hostile inputs are not newly claimed by this trace.
The earlier1501 broker fixture completes host no-vote reports; it is not1501
actual scans or a measured physical comparison workload.

## Architecture-sensitive diagnostic ruling

The first full parity attempt failed because actual `Counts.owned_bytes` includes
`size_of` charges that differ between native and wasm32. Its original log and
native/WASM traces are preserved as `07-parity.log` and
`first-architecture-sensitive-*`. No bytes were normalized or fabricated.

Root approved an explicit envelope with two views. `semantic` excludes only actual
architecture-sensitive byte counters while retaining queue semantics, inputs,
results and a target-observed owned-within-limit predicate. `diagnostics` retains
68 actual path-labelled owned-byte observations, configured limit and actual target
architecture. The comparator validates all diagnostic paths/counts/labels and
checks each actual value is within the configured limit. The two raw envelopes
are deliberately unequal; only their serialized semantic projections are compared
byte-for-byte. The default module is not instantiated.

Root's subsequent read-only preflight established that these `size_of` charges
also influence tight-budget admission/work/cleanup decisions. Therefore this
packet does not establish general portable admission parity. A separate narrow
canonical logical-budget fix is pending with root; final source verification and
parity must be refreshed after that correction. The witness stays frozen for that
serialized follow-up. No claim that the current diagnostic projection repairs the
kernel issue is made.

## Verification, artifacts and preservation

Final commands and exact status/log mappings are in `verification.json`:
feature focused integration test (1 passed), feature package all-target Clippy
with `-D warnings`, owned Rust fmt, actual parity script including negative
controls/self-test and owned diff whitespace check all exit0. The final script is
`20-parity-frozen.log`; earlier attempts remain unmodified. Root will run the fresh
four Studio gates after final kernel/source freeze; this worker did not run them.

Pinned cached-only environment:

```
PATH=/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:/usr/bin:/bin:/opt/homebrew/bin
RUSTUP_TOOLCHAIN=1.99.1
CARGO_NET_OFFLINE=true
```

Every Cargo invocation uses `--locked`; build targets/artifacts are isolated and
retained. `20-tools.sha256` records cargo/rustc/Bun/Python binary hashes; the script
log records PATH, compiler verbose version, host, executable paths and commands.
No installs/network/lock updates/golden regeneration occurred. Existing
rust-objcopy SIGABRT/missing `@rpath/libLLVM.dylib` debug-stripping warnings remain
visible; successful command exits are not warning-free claims.

Source80 preservation is checked against
`docs/completion/execution/evidence/2026-10-23-scoring/preservation-baseline.json`:
all80 current hashes match, with individual results in `preservation-check.json`.
The allowed WASM lib/manifest registrations are reviewed separately. Legacy IDs,
saves, engine/default dispatch, earlier probe files/fixtures/exports, goldens,
locks, substrate and frozen contracts were not edited by this worker.

Durable evidence lives at
`docs/completion/execution/evidence/2026-10-23-scoring/parity/`. The report's durable
copy is there too. Synthetic raw envelopes, semantic projections, negative inputs/
logs, source/tool/artifact hashes and verification receipts are copied into it.
Compiled binaries stay at the exact retained directory in `verification.json`;
no large target tree is added to the repository. `20-artifacts.sha256` names native
binary/default WASM/feature WASM/raw/semantic/negative hashes. The exact semantic
length/SHA and differing raw lengths/SHA are in `verification.json.summary`.

Intermediate failures are retained: an initial private Rational::whole call was
corrected to public Rational::new; a comparator initially confused dispatch order
with admitted ordinal composition order; a missing local variable and a
hand-entered scope-byte-count typo were corrected. These are development receipts,
not kernel behavioral defects. The architecture-sensitive full-trace failure is
separately disclosed above. No evidence history was rewritten to imply earlier
success.

## Limits

Finite authored synthetic mechanism execution only. Pure broker counts and a
trusted-host fence acknowledgement model do not measure physical concurrency,
worker termination, host authentication, semantic remainder completeness, entropy,
efficacy or installed execution. ATP is a supplied immutable snapshot, not full
release/decay/reward dynamics. No action tree is executed, no host effect imported,
no live store accessed and no ABI acceptance or production/default cutover claimed.
Every163 requirement remains open/C0/unknown.
