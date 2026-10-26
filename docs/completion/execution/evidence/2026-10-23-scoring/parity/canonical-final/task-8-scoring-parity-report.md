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

## Canonical-final continuation — 2026-10-23 UTC

The pending canonical-budget/attachment integration statements above are historical.
The independently reviewed canonical-budget correction and its integration fix1
are now used by an actual fresh native/WASM run. The bounded final witness passes;
root's independent full-probe review and four fresh Studio gates remain next.
No kernel source was edited by this probe worker.

The final probe adds `scoring_probe/budget_probe.rs` and its nested
`budget_probe/attachment.rs`, retaining every original numeric/election/entry/
broker/attachment scenario. The envelope/semantic schema is version2 with a named
logical-charge-v1 profile. The complete final owned/registration source freeze is
thirteen files in `parity/canonical-final/source-freeze.json`; it does not overwrite
the eleven-file intermediate or twelve-file pre-integration freezes.

The final new decision evidence comprises twelve minus-one/exact pairs,24 rows,
using identical literal limits on native and WASM:

| Decision | Refuse at | Admit at |
| --- | ---: | ---: |
| Single scoring payload | 2970 | 2971 |
| Single scoring item-work | 2157 | 2158 |
| One-request batch payload | 3354 | 3355 |
| One-contribution arbitration payload | 2791 | 2792 |
| One-job broker descriptor work | 465 | 466 |
| Broker completion owned | 2985 | 2986 |
| Broker completion cleanup/work | 2303 | 2304 |
| Attachment operation work,60 declared dependency IDs | 2936 | 2937 |
| Selected attachment owned | 4023 | 4024 |
| Declined attachment owned | 3863 | 3864 |
| Selected attachment cleanup | 2933 | 2934 |
| Declined attachment cleanup | 2929 | 2930 |

These invoke actual public scorer/broker/attachment APIs. No expected result or
limit is adapted from a target counter. The exact new fixture uses source=target
hello, input ID input, inert Text action é, wildcard/score stream test and parent
parent. The attachment issuer adds a declared capture named cap with Text value é;
the target carries no new capture. IDs, bindings and private issuer/cycle/lease
receipts come from the real calls. The separate work fixture's60 dependency IDs
are inert descriptor metadata, not a claimed executable graph.

Independent JS defines the reviewed fixed named table without importing the
production budget helper. It validates input/profile/field/row shape and uses
actual field UTF-8 lengths, including é as2 bytes, to derive entry payload2971,
item-work2158, arbitration nested248, record127, comparison178, binding28,
descriptor466, retained completion2296, broker274 and pre-completion690. Attachment
arithmetic independently derives inherited capture101, Context320+Visit80,
reference8, retained extra521, issuer contribution2397 and root-owned3087.
Work2937 is descriptor466+dependencies960+headers400+capture101+parent6+stream6+
binding38+dependencies960. With the second empty cycle retained, C=2,J=2,E=2918
requires2934 for selected cleanup; C=2,J=1,E=2918 requires2930 for a declined visit.
These bounded integer charge sums are exact; confidence/ATP/jitter arithmetic
continues to use BigInt rationals.

Refused calls retain exact errors, zero RNG consumption and unchanged lifecycle/
counts. Explicit retry uses the same private issuer/target/job/remainder/tape
following cheaper metadata or retirement of the declared expendable cycle. A
successful visit is then rejected as Repeated without another draw. The work
fixture preserves its unrelated active lease until normal completion, and rejects
a duplicate completion. Exact cleanup saturation rejects another cycle atomically.
Queued target cleanup is explicitly a host no-vote completion in these accounting
fixtures; it is not reported as actual target scoring. The original hard attachment
scenario still actually scores both target entries and retains below-threshold
no-vote semantics. Restart/retirement restores broker274 without a host-effect claim.

The strict diagnostic check first caught a remaining68-byte attachment difference
in the pre-fix1 run. Its exit1 transcript and raw target/source/tool/environment
artifacts remain unchanged under `canonical-final/03-attachment-mismatch-*`, with
labelled differences in `03-attachment-mismatch.json`. The root-owned fix1 replaces
those remaining charges; the checker was not relaxed. All25 current kernel/source/
affected-test files match that fix1's independently reviewed freeze, recorded in
`reviewed-kernel-freeze-check.json`.

Final actual script `09-parity-frozen.log` exits0. It independently checks both
semantic views, all24 fixed-limit decisions and all68 actual logical accounting
diagnostic observations, then requires matching semantic bytes and matching
logical diagnostic arrays across targets. Actual architecture labels remain
separate (`aarch64` versus `wasm32`); no raw-wrapper byte identity or physical
allocator equivalence is claimed.

- Semantic projection:243,932 bytes on each target; SHA-256
  `7125a31a9932c95a0df0cf42c505cfe988c29653d9d36f10237f7fa7182be4d8`.
- Raw native envelope:250,864 bytes; SHA-256
  `dbd6d97c45ba23a50f912294560fe98d1ada86c29c927a9d24602034d52b19ab`.
- Raw WASM envelope:250,863 bytes; SHA-256
  `22a904c903ab0dc5a00f455dbd1b7412b51ea5ff4b41c5c363b9059d194c2d51`.
- Feature WASM SHA-256:
  `3f3b0b94759ad2d4a591f28cbe590e10aefb32432d9e21364b5199145affe520`.
  Executed with all three observed imports throwing; actual host calls0.
- Default WASM SHA-256:
  `d1112a5120d987ae4a376dc3169e29d9d9674f62a7657a8d4e1fc9f9d2e69395`.
  Compiled/export-inspected only, not instantiated; scoring export absent.

Three separate corrupt raw envelopes run through the same normal comparator
process: confidence, quarantined capacity and an attachment-work boundary refusal.
Each exits1 for its intended independent assertion; raw inputs, output and hashes
are copied as `09-corrupt-*` and `09-negative-*`. Eight invalid/incomplete semantic
self-tests plus malformed JSON also refuse. This is stronger than equality-only
negative evidence.

Fresh verification is exact in `canonical-final/verification.json`: all feature
package tests pass16 (13 library tests plus3 probe integration tests), feature
all-target Clippy `-D warnings`, owned Rust fmt, owned whitespace checks and the
actual parity script pass. The earlier pre-integration summary incorrectly counted
only8 feature tests; its raw04 log actually contains13+2=15 passing tests. That
under-counted historical summary remains preserved; this final count is taken
from the fresh10 log rather than carried forward.

Every Cargo invocation is pinned1.99.1, explicit offline/locked, with isolated
retained artifacts and explicit
`DYLD_LIBRARY_PATH=/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/lib`.
The script actually loads rust-objcopy and records tool/library/environment hashes;
these final logs have no loader warning. This uses the existing library and does
not install software, change persistent config or suppress diagnostics.

All80 preservation hashes still match, with per-file records in the final
`preservation-check.json`. Raw intermediate20 evidence, failed03 artifacts and
pre-integration receipts remain untouched. New final raw/semantic traces, negative
fixtures/logs, source/tool/library/environment/artifact hashes and report copy live
in `docs/completion/execution/evidence/2026-10-23-scoring/parity/canonical-final/`.
The compiled binary/target directory remains retained at
`/var/folders/c_/thtfn_7d1p9378rxsp4pwz6c0000gn/T/task-8-scoring-parity.qdu7oH`.
No target tree is added to the repository.

Final probe source is frozen for root's independent review. No broader feature,
legacy/default dispatch/save/ID/probe/golden/lock/substrate/contract edit, staging,
commit, install, network, holdout or live-store access was performed. This is finite
synthetic mechanism evidence, not general efficacy, physical memory/concurrency,
host authentication/fencing, installed execution, ABI acceptance or production
cutover. Every163 requirement remains open/C0/unknown.
