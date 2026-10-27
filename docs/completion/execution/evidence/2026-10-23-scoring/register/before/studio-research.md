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

## Current language/text/scanner observations after integrated final fix

The additive opt-in packet implements immutable versioned preparation with all
nine utility effects, quote-aware clauses/conditions/negation and original UTF-8
spans, exact decimal captures, bounded scripts/Tape and pure EffectPlans. Text-v3
uses framed full SHA256 keys; the action-scoped scanner retains full validated
features and checked i128 rational evidence. Explicit grammar/wildcard/Tape routes
join concrete entries with every origin and guard. These APIs remain separate
from active engine/command/save behavior.

`evidence/2026-10-22-language/final-fix/requirement-mapping.json` maps every new
source/direct test to applicable frozen S/D and P08/chapter requirements;
`current-source-manifest.json` pins current bytes. The preceding observation wave
moved S05/S06 alone to partial candidate. This final fix leaves 120 candidates,
27 local gaps and 16 external gaps, all 163 open/C0/unknown. All 46 S and 18 D rows
remain verbatim, and the same 15 local observation IDs are retained without
appending duplicate mechanism observations.

The combined final review found three Important defects. The integrated fix
preserves every quoted antecedent byte while case-folding only unquoted token
text; exact full compound conditions and escaped quotes are tested through the
guarded execution boundary. Grammar control roles and overlapping/contradictory
negation, conjunction and reserved If/Then vocabularies now return InvalidTables
before producing a Prepared result. This is a conservative local constructor
policy, not a claim of unrestricted natural-language efficacy.

Candidate queries use an O(1) immutable borrowed entry slice. They reserve table
indexing before traversal/all-reference allocation, stream structural matches
without an uncharged all-match vector, and preflight every selected original,
alternative vector, nested expression/string/vector and origin payload before
copying. Tiny-budget 257-entry empty queries and 12000-alternative selected targets
refuse; complete successful results preserve order and shared origins. The budget
measures logical work, not elapsed time or the deferred global scheduler.

The three small API/assertion Minors are also addressed: the independent JS
comparator pins exactly four ordered component labels, the public complexity
getter shares text-v3's punctuation-excluding lexical count, and declared script
names/aliases (including hyphens) use the parser's accepted alphabet. Focused RED
and GREEN logs preserve the failures and repairs. The earlier independent task
reviews remain valid only for their historical scopes. All six combined findings
are addressed by implementation/tests; one final scoped independent rereview is
pending and no approval is presumed.

S05 still retrieves both 2+3 and 8+9 under one low key before full-original scans.
S06 retains the direct checked-i128 reducer tests beyond i64 capacity, signed
cancellation/fractions, overflow refusal and one clamp. Exact canonical decimal
comparison remains separate from finite IEEE f64 script computation. Missing
features still yield the documented +20/-20 examples; the scores are not language
accuracy, trust or acceptance evidence. Full-chunk scanning, ATP, jitter, voting
and production integration remain open.

Actual final feature-WASM/native execution yields identical 151812-byte traces,
SHA256 `743d07418aadbe60c704d0d934c2bffb237e62e1e39c162ad731aba7d4a8e4d9`,
with zero host calls. Independent JS checks transforms/full digests and exact
BigInt scanner fractions plus the new guard/refusal cases. The same comparator
rejects full-digest and label corruption in separate processes with exit 1; its
self-test also rejects a missing component. Default WASM was compiled and
export-inspected only. Existing identity native/feature-WASM parity remains
4205 bytes, SHA256
`60aaef7f217d27a6fe505f2c6405239e3032902e9964472e24273bca97c07580`.
New runtime artifacts and logs are under `final-fix/language-runtime/` and
`final-fix/identity-runtime/`; older 133421-byte traces remain historical.

The root ran four separate Studio gates after product freeze. The new
`final-fix/gates/summary.json` records all exit 0, with 289 relevant source files
unchanged before/after each gate: 221 browser/unit passes and production build,
lint, 228 native passes plus one existing ignored test, and all-target native
Clippy. Phase A separately records 7 focused passes and 175 kernel/feature passes,
feature Clippy, fmt and both parity scripts. These local executions retain Bun
1.4.4 canary versus CI 1.4.0, missing-libLLVM stripping diagnostics, existing
abi-wdbx import/build-script/block future-incompatibility warnings and browser
chunk/static-analysis limitations. The ignored test is not passing qualification.

Original legacy-baseline.json remains immutable: 22 of its 23 file hashes match;
the sole reviewed extension adds `HybridTable::entries()` to identity/mod.rs.
`identity-accessor-extension.json` and the new closure prove that removing only
those exact accessor bytes reconstructs the original hash. Identity acceptance,
IDs, saved schemas, locks, goldens, frozen C0 contracts and prior raw evidence
are unchanged. The old closure/manifests remain historical; use the new
`final-fix/check-final-closure.py` for current reviewed sources and mappings.

Active engine/commands/feedback integration, ID/save/import migration, durable
versioned utility ingestion, real host contact authorization/effects, research
efficacy and independent contamination controls/adjudication remain open. So do
scheduler/ATP/voting, ABI-owner receipt, durable trust/admission, OCR/model/GPU,
all targets, installed desktop, browser UI/theme/accessibility and elapsed-operation
qualification. No frozen holdouts, live store, credentials or production effects
were used. Inventory checks do not establish acceptance or scientific adequacy.
