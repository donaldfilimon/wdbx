# Task 8 scoring/orchestration final independent whole-tranche review

2026-10-23 UTC. This is the one final integrated review of the current bounded
Studio scoring, pure orchestration, portable-budget, runtime-witness and evidence
tranche. It does not approve the unfinished parent completion program.

**Specification compliance: PASS for the authorized bounded tranche.**

**Code and evidence quality: PASS for the bounded tranche, with the existing
nonblocking Minor M1 retained below.**

No new Critical or Important finding was established. No integrated product fix
wave or scoped rereview is required by this review. This approval does not close
a frozen requirement, approve production adoption, supply ABI-owner agreement or
authorize a public launch.

## Boundary and method

The governing boundary is the final-review brief, including its authoritative
canonical-budget/fix1 and final-package appendices; the current operational
profile and continuation rulings; and the frozen scoring, scheduling and S07–S15 /
D06–D08/D16 obligations. Root and Studio instructions were consulted. Legacy
runtime defaults do not replace the explicitly versioned opt-in profile.

Read the exact 10,772-line, 40-file final source package once. Terminal output
cut off part of the initial source read; only the missing hunks were recovered
in bounded follow-up reads. Reviewed the compact evidence package's narrative,
requirement mapping, checker delta, closure and final receipt/attachment writers.
Read the scoring/orchestration reports and independent reviews, orchestration
fix1 rereview, canonical-budget and integration-fix1 reviews, complete-probe
review and rename-only rereview, and fresh evidence-task review. The latter is
the independent task-review authority for the large six-file inventory delta;
its full JSON/reference audit was not repeated.

Read-only byte checks establish all of the following:

- The final source snapshot hash list equals `final/product-freeze.json`, and
  all 40 listed current files match those hashes.
- Every new/context hunk line in both supplied final packages agrees with its
  current file. This checks the captured package boundary without reviewing
  unrelated dirty work.
- Final source package SHA256:
  `413eb4e45226ff0c4b6635a16e920523cfa0918ba6dcfc71adb1fdf4445a197a`.
- Compact evidence package SHA256:
  `cf8b62cdee8ac470d4974ffe2a5b268c67003f9c366c615638b296f136c4180f`.

Inspected the four current gate command/environment/result records and relevant
raw result/warning lines. Did not rerun tests, builds, green suites, checkers or
receipt writers. An initial status read established the dirty-tree context; no
repeated Git audit, staging, commit, network, installation, holdout/live-store
access, source edit or subagent delegation occurred. This report is the sole
reviewer write. Hash agreement identifies bytes, not source or execution
authenticity.

## Findings and prior-observation disposition

**Critical: none established. Important: none outstanding in this tranche.**

**M1 — Minor, nonblocking: independently pin defining numeric/election inputs
when extending or generalizing the witness.**

`specimen-studio/tests/scoring-parity.mjs:1721` pins numeric scenario names,
count and order, while `:1726` checks the emitted scan/policy/draw inputs through
independent equations. The corresponding election loop at `:1751` also checks
mathematical consistency without pinning every defining per-ID input. A future
internally consistent change from brainstorm cap5 to cap1, for example, could
retain its scenario name and pass those checks. This is the original complete-
probe review's M1, not a new observed arithmetic error.

The current frozen vectors at
`specimen-studio/native/specimen-wasm/src/scoring_probe/numeric.rs:10` and `:134`
do supply the intended distinct policy/election inputs. This final review read
those vectors and the independent equations together; the source is hash-bound
to the retained execution. Fixed-budget inputs and their 24 outcomes already
have independent literal/field pins. Therefore M1 does not invalidate the current
finite evidence and is accepted as a nonblocking follow-up. When changing these
vectors, pin the per-ID scan/policy/draw/weight/eligibility obligations or their
distinguishing predicates and add a substitution/omission control. No source
change is required for this final approval.

Other prior observations are resolved or remain qualifications:

- Orchestration I1/I2 are addressed: descriptors are unconditionally preflighted
  before selected/declined attachment matching, and restart performs one cycle
  traversal plus one clear of each global queue. Acceptance reserves future
  cleanup; cancelled/quarantined retained records keep their base reservation.
- Portable-budget integration is addressed, including attachment Context/Visit/
  reference/RouteCapture costs. The former dominated `size_of::<Span>()`
  observation is resolved at `scoring_v3/policy.rs:198`; it is not an outstanding
  finding. The current 14-module production package uses fixed logical policy
  units rather than target layout expressions.
- Probe Important I1 is addressed by `featureModule` and its references at
  `specimen-studio/tests/scoring-parity.mjs:1917`. The required current broad
  lint gate passes without suppression or assertion removal.
- The native debug-stripping/dyld warning remains an actual current environment
  qualification, detailed below. It is not silently declared historical or
  explained away as cache replay.

## Integrated compliance and strengths

Paths in the following source references are relative to
`specimen-studio/native/specimen-kernel/` unless qualified otherwise.

1. **Confidence and random decisions remain distinct and exact.**
   `src/scoring_v3/policy.rs:118` applies the negative-only supplied ATP penalty,
   baseline clamp and baseline-qualified jitter exceptions before independent
   magnitude/sign draws, final clamp and inclusive gating. Strength is a separate
   refusal and redundancy weight, not confidence scaling. `math.rs:13` uses
   checked rational operations; public operations commit tape position only
   after success. Endpoint/zero-cap/max-strength traces retain stream identity.
   `election.rs:38` validates original finite weights, excludes ineligible slots,
   abstains at zero total, avoids singleton draws and handles the closed grid
   with stable finite proportional selection. The declared f64 domain does not
   leak into threshold or ATP arithmetic.

2. **Contribution proof and exact redundancy survive broker integration.**
   `src/scoring_v3/contribution.rs:44` keeps construction private, with immutable
   getters and no Deserialize bypass. `:119` scans actual immutable prepared
   inputs after bounded preflight. Bypass removes only the numeric threshold
   refusal; actual confidence, strength, guards, blocked automata and alternative
   inhibition remain. `arbitration.rs:45` includes chunk root, parent lineage,
   complete typed selected tree, typed captures, inhibition set and dependency
   set. `:134` exempts attachments, intermediate facts and referenced parents;
   `:167` elects only within actual redundant groups. Thus composition cannot
   lose a dependency parent to unrelated or global election.

3. **Shared capacity, cancellation and reuse form one coherent lifecycle.**
   `src/orchestration_v3/broker.rs:237` atomically checks pending/cycle/owned/
   cleanup headroom before inserting all supplied jobs. `:344` dispatches one
   comparison per lease under shared active capacity and FIFO8:1 service.
   Private owner/serial/generation leases refuse duplicate, stale and foreign
   completion. `:518` cancellation revokes active work into counted quarantine;
   `:584` restart advances generation without releasing it; `:611` requires
   both current-owner generation and the exact revoked worker receipt to
   acknowledge a fence once. Incarnation-bearing handles and issuer receipts
   prevent old controls from affecting a reused cycle ID. Retirement at `:639`
   requires drained/fenced Partial or Composed state and releases only its own
   recorded accounting. These are model invariants, not proof of host fencing.

4. **Attachments preserve the barrier and inherited authority boundaries.**
   `src/orchestration_v3/attachments.rs:175` requires a qualified completed
   issuer and current incarnation before scheduling. UTF-8 subspan checks,
   inherited constraints, whole-capture rebasing, root visits and depth4 bound
   remain in force. Every descriptor is charged before dependency searches and
   nested capture comparisons, including declined affinity. At `:354`, checked
   fixed Context320/Visit80/reference8/RouteCapture96 and UTF-8 additions precede
   both reservations, insertion, visit publication and RNG advancement. Soft
   endpoints consume no draw; admission refusal remains retryable. Hard links
   schedule evaluation without guaranteeing a target contribution.

5. **Composition consumes a completed collection and remains inert.**
   `src/orchestration_v3/compose.rs:28` validates the full explicit dependency
   graph and stable admission order before arbitration draws. `:77` requires a
   sealed Ready cycle, preflights copies and invokes scoped arbitration once.
   Surviving contributions retain complete origins/captures/actions/facts and
   attachment groups. No semantic edge, script execution, host effect, support
   retrieval or durable ATP update is invented. Partial/deadline/restarted work
   cannot become a complete plan.

6. **Portable logical policy is exercised at decisions, not inferred from
   projection equality.** `src/logical_charge_v1.rs:18` defines a private,
   concrete-type fixed table, with scalar and collection Contribution2048.
   Both budget mechanisms use those units while preserving their separate
   item-work and byte-work policies. The two public boundary test files pin
   literal minus-one/exact limits, UTF-8 bytes, refusal atomicity, retained
   lease/visit/RNG state and cleanup release. In particular, attachment extra521
   and selected/declined cleanup2934/2930 include the inherited RouteCapture
   charge rather than the producer Capture charge. The model bounds logical
   policy inventory; it does not measure heap allocations.

7. **The runtime witness checks the combined contract independently.**
   `specimen-studio/native/specimen-wasm/src/scoring_probe/` calls actual public
   preparation/scoring/broker APIs and projects immutable proofs. The JS witness
   uses decimal strings/BigInt for exact confidence, seeds and IDs; original
   f64 weight bits and eligibility are retained. It independently reconstructs
   equations, typed groups, queue/fence transitions and the fixed UTF-8/C/J/E
   budget ledger. All 24 same-limit outcomes and 68 logical diagnostics are
   checked. The feature module actually executes with throwing import wrappers
   and zero host calls; default WASM is only compiled/export-inspected. Three
   corrupt fixtures reach the same normal comparator in separate nonzero
   processes. This goes beyond native/WASM equality alone.

8. **Evidence describes the implemented clauses and the remaining gaps.**
   The integrated mapping accurately separates numeric S07/S08 fixtures,
   actual scanner entries, S10's distinct-node graph, and S14's 1501 logical
   descriptors completed with None. Small actual scanner/broker scenarios are
   separate from the large accounting fixture. The evidence-task PASS confirms
   exactly S07–S15 become partial candidates, with 129/18/16 routing, all163
   open/C0/unknown, exact46S/18D and preserved baseline observations. Its 43/40/
   80/3/815 identity checks and 74-input/28-pin reuse qualification remain scoped
   evidence-task findings; they were not duplicated as a second broad audit.
   Historical failures/reviews and current qualified source references are
   retained. The only execution-checker delta is the reviewed catalog pin.

## Supplied executions and environment qualifications

The current `parity/review-fix1/` and canonical-final receipts describe actual
native/feature-WASM execution with identical 243,932-byte semantic projections,
SHA256 `7125a31a9932c95a0df0cf42c505cfe988c29653d9d36f10237f7fa7182be4d8`,
24 fixed-boundary outcomes, 68 equal logical diagnostics and zero host calls.
Raw architecture wrappers remain distinct. The three normal-comparator negative
processes each record exit1 at the intended arithmetic, quarantine or boundary
assertion. The rename-only refresh reuses qualified Rust artifacts and re-executes
the native binary and feature WASM; it does not manufacture a fresh Rust build.

The four separate current Studio gate receipts record exact commands/cwd/time,
pinned Rust1.99.1, offline Cargo, explicit PATH and DYLD_LIBRARY_PATH, source HEAD
36bc99b699de60be6c07b2838d74580084fb5d4c and 324 source identities unchanged
during each command. Their exits are zero: browser check/build with221 passes,
broad lint, native tests with311 passes plus one ignored, and native all-target
Clippy. These are supplied executions inspected by this review, not reviewer
reruns. Default feature-disabled probe targets legitimately contribute zero
tests; their feature-enabled execution evidence is separate.

No root Rust workspace gate, optional FHE runtime or 3-D evidence revalidation is
inferred from these Studio executions.

`gates/review-fix1/native-test.log:11` and
`gates/review-fix1/native-clippy.log:12` still report rust-objcopy SIGABRT and dyld
missing `@rpath/libLLVM.dylib`, despite the supplied library environment. The
isolated objcopy-load/probe-build repair supports only that narrower execution.
Whether broad-gate diagnostics were replayed from cache is unadjudicated. The
test totals and command exits are green, but warning-free broad linking/debug
stripping is not qualified. Dependency unused-import/block future-compatibility,
browser chunk/static-analysis limitations and Bun1.4.4 canary versus CI1.4.0 also
remain attached. The ignored test is not passing qualification.

## Final receipt-writer safeguards

Reviewed, but did not execute, `final/write-review-attachment.py` and
`final/write-handoff-receipt.py` from the compact package. They require a saved
review and refuse an already-existing output receipt. The attachment writer
only emits C0 partial-mechanism review routing and asserts mapped execution
claim triples remain open/C0/unknown. It records review, source-freeze and mapping
hashes, including a scoped rereview if one exists, without promoting inventory.

The handoff writer validates relative nonsymlink file references and current
hashes, exact163/129/18/16 claim state, all80 preservation and3 frozen baseline
identities, and the final source bytes against each gate source manifest before
capturing packet file identities. It labels the result an unqualified current-
byte receipt and explicitly excludes execution authenticity and product
acceptance. These safeguards are appropriate to the authorized local handoff;
execution of these newly authored writers remains root's post-review step. File
presence/hashes alone do not authenticate review authorship or command history.

## Cannot verify and remaining program work

Cannot independently authenticate historical command execution, reconstruct every
original runtime input population, resolve the native cache-warning explanation,
or infer general compensated-f64 correctness from the authored interior integer
vectors. Current bytes and the inspected source support bounded local behavior;
they do not establish all-input/all-target qualification.

Actual one-broker runtime ownership, worker execution/no-vote authenticity,
credential/contact authorization, host termination/fencing, physical concurrency/
fairness/memory/latency, complete semantic remainder discovery/partition, and
meaning of host captures/lineage/dependencies remain unqualified. Active-engine/
commands/feedback integration, legacy ID/save/import migration, durable utility
ingestion and ATP dynamics, support retrieval/Transient Memory/tone/aftercare,
scientific efficacy/contamination controls/adjudication, ABI-owner agreement,
OCR/model/GPU, installed desktop, browser UI/theme/accessibility and elapsed
operation remain open.

The next Studio ATP preflight is planning only and was not treated as an
implementation, execution or closure. The parent full plan remains unfinished;
there is no public-launch or production-cutover approval. All163 requirements
remain open/C0/unknown. This final PASS is limited to the reviewed current local
mechanism and evidence tranche, with M1 and the stated qualifications retained.
