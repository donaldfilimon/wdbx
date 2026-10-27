# Studio research continuation — 2026-10-22

This continues approved package 8 while package 2 durable integration awaits its
named ABI-owner receipt. The binding scope remains the frozen identity design
and Studio architecture, not a new public ABI or a replacement saved schema.
All completion requirements remain open/C0. No live specimens, credentials,
model weights or frozen efficacy holdouts are used as test inputs.

## Intent and decomposition

Implement dependency-first mechanisms in the pure Rust kernel; capability-based
execution remains in host adapters. The first packet provides typed prepared-key
identity and HybridTable behavior. It deliberately leaves the active text-v2
encoder, command behavior and saved specimens intact until the text-v3 parser,
projection and migration contracts are executable. Supplied scope is an internal
host contract; the table is not a credential-authentication service.

The following packet compares actual native and WASM results through an explicit
synthetic test probe. Compilation, available adapters and fallback are distinct
from execution and interoperability. The final packet attaches source/test/hash
observations to existing requirements without promoting the frozen inventory.

## Read-only baseline audit

`evidence/2026-10-22-studio/source-audit.json` covers all 46 acceptance cases and
18 decisions. Every test reference was inspected in source; no test pass or
whole-case satisfaction is inferred. `source-audit-hashes.json` identifies the
inspected bytes. The identity implementation being developed separately was
excluded from that baseline audit.

Identified gaps include full class-specific text-v3 transforms and bind grammar,
typed identities and overflow handling, calibrated scoring/order, complementary
vote arbitration, full language governance, research efficacy, durable migration,
and measured scheduler/hardware behavior. Existing broad fixtures remain useful
candidates but do not substitute for direct assertions of every frozen row.

## Evidence boundaries

New synthetic mechanism tests are separate from frozen efficacy experiments.
No preregistered efficacy result or contamination-free holdout claim is made.
Independent contamination controls/adjudication and actual GPU/OCR/model,
installed desktop, other platform and elapsed-operation receipts remain required.
Browser UI/theme acceptance is distinct from WASM parity and is not established
by a kernel test. Canonical/offline integration follows its existing prerequisites.

The existing canonical-checkout and uncommitted-snapshot policy is retained to
preserve the prior tranche's dirty work; implementations are serialized with
exact ownership and independently reviewed working-tree diff packages. This
packet changes no default branch, normative ABI schema or deployment state.

## Reviewed identity foundation

`specimen-kernel/src/identity/` now provides host-supplied 128-bit concrete refs,
population-scoped complete keys, exact prepared-tree/original/ordered-alternative
equality and independent logical resource linkage. Semantic repeats return the
existing concrete ref even when a supplied key projection changes. Forced digest
collisions retain distinct originals. A full 20-entry node can atomically spill to
a fresh host-supplied sibling; unavailable/colliding allocation leaves it intact.

Array lookup through 64 records and multi-value hash lookup above 64 return every
match in insertion order. Rebuild validates all refs, populations, node/key
consistency, nested trees, finite weights and bounds before replacing anything.
Resource refs are concrete and globally distinct; resource link values can repeat
or be absent without losing structural retrieval. The bounds include 2,000
pattern characters and 10,000 supporting resources. New operations are a pure
owned mechanism, not a durable canonical writer or authenticated admission path.

Twelve focused tests and one deterministic fixture passed; the kernel suite
passed 80 tests. Independent task review accepted spec compliance and code quality
with no Critical/Important finding. Bounded assertions are labelled S01/S02/S03/
S04/S17/S40; those labels do not close complete runtime acceptance rows. In
particular independent qualification/scanning, feedback integration, full runtime
insertion/migration and raft/yielding behavior remain separate.

## Actual native/WASM witness

The explicitly enabled `identity-probe` feature executes actual typed table
operations on native and wasm32 builds. The final-fix run produces identical
4,205-byte JSON results in both runtimes: SHA-256
`60aaef7f217d27a6fe505f2c6405239e3032902e9964472e24273bca97c07580`.
Independent native and JavaScript assertions require the observed spill outcome
`Inserted` and repeat outcome `Duplicate`, alongside concrete counts, refs and
exact ordering, including 64/65 resource-link crossover, rebuilt and reversed
sequences. The serializer derives each `kind` from the actual `InsertOutcome`
variant. The default WASM artifact is separately compiled and its exports
inspected; it has no test-probe export. Only the feature artifact is instantiated.
A negative witness changes only the spill kind, preserves every ref and other
property, and receives comparator exit 1. No production dispatch command, saved
schema, golden, dependency or lockfile changed.

Independent task review accepted spec compliance and code quality. The resource
crossover observation from identity review was resolved by the stronger witness.
The final-fix wave addresses both Minor findings from the full tranche review:
explicit success-variant serialization/assertions and precise default-artifact
wording. Native and JS expectations first failed against the old serialization;
the amended fixture and actual native/WASM gate then passed. Fresh raw logs are
under `evidence/2026-10-22-studio/final-fix/`. The earlier 4,168-byte result
`eb84ea042097bba7786b5c6cf23ae808b8a95f02887c668999934d44760d55d1`,
old ref-corruption negative and prior reports/reviews remain historical snapshots;
they do not validate the amended trace. The probe is synthetic mechanism
evidence, not text-v3 generation, semantic accuracy, save migration or full
engine/native/WASM parity certification.

## Product checks and environment

On the prior snapshot, pinned Rust 1.99.1 Studio checks passed separately:
`bun run check` (221 browser/unit tests and production build), `bun run lint`, `bun run test:native`
(151 Rust tests), and `bun run check:native` (workspace/all-target Clippy with
warnings denied). The final-fix wave reran the affected opt-in native test,
feature all-target Clippy with warnings denied, and actual isolated parity gate; all passed. Full
default gates were not rerun for this opt-in trace/doc fix. Their prior outcomes,
pre-feature results and all RED/negative outcomes remain historical alongside
fresh final-fix logs.

The installed Bun reports 1.4.4 canary, while CI specifies 1.4.0; exact local runtime
identity is recorded, without claiming that CI runtime was used. Test linking
emits a nonfatal rust-objcopy SIGABRT diagnostic because its LLVM library cannot
be found. Test binaries execute and return 0; this is not pristine-link/toolchain
qualification. Workspace Clippy also reports an existing future-incompatibility
notice for dependency block 0.1.6. No global toolchain/library installation or
substitution was made to hide either diagnostic.

No UI flow changed. Browser theme/accessibility e2e, installed desktop and other
platform receipts are not inferred from these gates. No OCR/model/GPU inference,
private staging, credentials, public ingress, commits or deployment were performed.

## Requirement observations and remaining work

`studio-research-v1.json` records all 46 acceptance cases and 18 decisions once,
with verbatim frozen rows, source/test identities, bounded local observations and
explicit blockers. The identity packet recorded 118 candidates, 29 local gaps
and 16 external gaps; only S01/S02/S03/S04/S17 moved from local gap to partial
candidate because reviewed source/tests now exist. Every requirement stays
open/C0 with unknown whole-requirement test results. Both completion checkers,
27 mutation tests per Python interpreter and separate observation-closure checks
passed. These checks protect the inventory; they do not judge scientific adequacy.

At the identity packet snapshot, the next local dependency was language/scanning: executable versioned utility
tables, source spans and clause/bind ASTs, followed by class-specific text-v3
transforms and explicit grammar/wildcard routes. Then scoring/orchestration,
synthesis/ATP/vision, supervision/mutation/graves and scheduling/tuning follow
the frozen order. The new identity mechanism still needs production command,
feedback, save/import and projection migration integration; those consumers must
preserve legacy IDs/refs and require their own direct tests and parity receipts.
Those are remaining local implementation tasks, not all blocked by ABI approval.

Package2 durable trust/admission and later canonical consumer integration still
require the named ABI-owner receipt and reviewed durability/privacy/recovery
ordering. Independent research controls/adjudication, model/OCR/GPU runtime,
installed platforms and elapsed operation remain distinct qualification gaps.
No failed or missing result has been converted into completion.

`evidence/2026-10-22-studio/prior-source-reuse.json` confirms 26 prior root/P1
source identities are unchanged. Their earlier raw gates remain historical;
this Studio packet does not rerun or relabel them as new substrate conformance.
Final automated review scope and a current-byte manifest accompany this packet.

## Current reviewed language/text/scanner observations

The additive opt-in packet now implements immutable versioned preparation with
all nine utility effects, quote-aware clauses/conditions/negation and original
UTF-8 spans, exact decimal captures, bounded scripts/Tape and pure EffectPlans.
Text-v3 class transforms use framed full SHA256 keys; the action-scoped scanner
uses full validated features with exact i128 rational evidence. Explicit
grammar/wildcard/Tape routes join concrete entries while retaining every origin
and guard. This updates the earlier missing-source observations; the historical
baseline audit and earlier reports are retained as their original snapshots.

`evidence/2026-10-22-language/register/requirement-mapping.json` links every new
source/direct-test file to applicable frozen S/D and P08/chapter requirements.
`current-source-manifest.json` pins current bytes, including the changed module
entrypoints and WASM feature manifest. All 46 S and 18 D rows remain verbatim.
S05/S06 alone move local gap to partial candidate, leaving 120 candidates,
27 local gaps and 16 external gaps. Nine new bounded observations supplement
the six prior identity observations. Every requirement remains open/C0/unknown.

S05's direct table test retrieves both 2+3 and 8+9 under one low key, then observes
different full-original operand evidence. S06's direct reducer test exercises
values beyond i64 capacity, exact cancellation/fractions, overflow refusal and
one final clamp. Numeric preparation/script tests preserve ordered operands and
compute 4; finite IEEE f64 computation remains distinct from exact canonical
i128 coefficient/scale capture comparison. Unknown conditions and negation block
pure execution plans, not actual host effects. All-zero weights, malformed
utilities/contacts, limits, stale registry and runaway Tape remain typed failures.

Current profile choices and costs are in `language-v3-profile.md`. In particular,
identical nonnumeric two-token text with unresolved SPO scores +20, and a single
opaque token scores -20 because absent components retain dissimilarity. These
results were directly tested and are not adjusted to imply language efficacy.
The scanner has no full-chunk API and does not supply ATP, jitter, voting,
constitutional trust, admission or the global 1000-comparison scheduler.

The final language fix-2, numeric followup, text/scanner fix-1 and parity independent
task reviews approve their scoped snapshots; preceding findings/RED logs remain
linked and preserved. Review methods differ: numeric/fix reviews did not rerun
tests or independently inspect raw evidence; the parity reviewer inspected the
retained raw witness. These reviews do not establish scientific adequacy. The
parity review's Minor about not checking emitted component names in JavaScript
remains open, alongside the stripping environment qualification. Final whole-
tranche review will follow this observation packet and is not yet approved.

Actual native/feature-WASM execution returns identical 133421-byte JSON traces,
SHA256 `0874bb27c8a593db459c47f06faa46e62cf2c917f31e7eaa78845a5df7475bae`,
with zero host calls. JavaScript independently rebuilds framed transforms/full
digests and exact BigInt scanner fractions. The same comparator refuses the
corrupted full 32-byte digest in a separate process with exit 1. Default WASM
was compiled/export-inspected only; it was not instantiated. Evidence is under
`evidence/2026-10-22-language/parity/`. This is one synthetic native/Bun witness,
not all engine/target interoperability or production integration.

The retained final four combined Studio gates each exit 0, recorded by
`evidence/2026-10-22-language/gates/summary.json`: `bun run check` executes
221 browser/unit tests and a production build; lint completes; native tests
report 221 passes and one existing ignored test; native all-target Clippy
completes. This observation task did not rerun product gates. Raw logs retain
Bun 1.4.4 canary versus CI 1.4.0, rust-objcopy/missing libLLVM, existing abi-wdbx
unused-import/build-script and block future-incompatibility diagnostics, plus
browser chunk/static-analysis limitations. Successful execution does not erase
those qualifications or turn the ignored test into passing evidence.

Active engine/commands/feedback integration and ID/save projection/import
migration remain open, including durable/versioned utility ingestion. Real host
contact authorization/effects, full research efficacy and independent contamination
controls/adjudication, scheduler/ATP/voting/budgets, ABI-owner receipt and durable
trust/admission remain open. OCR/model/GPU, all targets, installed desktop, browser
UI/theme/accessibility and elapsed operation need separate qualification.
No frozen efficacy holdouts, live store, credentials or production effects were
used. All 23 legacy-baseline identities are rechecked separately; historical
raw evidence and receipt203 bytes remain unchanged. The new observation checks
protect inventory and source identities, not acceptance or hardware efficacy.
