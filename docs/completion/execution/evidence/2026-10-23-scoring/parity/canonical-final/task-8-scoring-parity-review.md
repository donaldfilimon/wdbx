# Task 8 scoring-parity independent complete-probe review

2026-10-23 UTC. Reviewed the frozen final probe against both task briefs, the
current scoring/orchestration profile, the reviewed scoring/orchestration APIs,
and the canonical-budget/fix1 continuation. The canonical-final appendix and
durable packet govern the final result; earlier architecture-sensitive and
pre-fix1 runs remain historical.

## Verdicts

- **Specification compliance: PASS for the bounded finite witness.** Actual
  native/WASM public-API calls, independent arithmetic/transition checks, the
  same fixed 24 decision rows, and matching logical diagnostic arrays satisfy the
  scoped probe contract. This does not close the wider Task 8 or any requirement.
- **Code quality/integration: CHANGES REQUIRED.** One Important finding blocks
  the required Studio broad lint gate. No Critical finding or additional
  Important behavioral defect was established. One Minor comparator-hardening
  observation is nonblocking for the current frozen evidence.

## Findings

### I1 — Important: the comparator fails the required broad lint gate

`specimen-studio/tests/scoring-parity.mjs:1917` binds the feature WASM module to
the identifier `module`. The project's actual `bun run lint` run refuses this as
`next(no-assign-module-variable)`. This is a current integration failure, even
though Bun successfully executes the comparator and Rust Clippy passes.

Evidence: `docs/completion/execution/evidence/2026-10-23-scoring/gates/lint.log:2`
records that exact file/line/rule; `gates/lint.json` records exit 1 and unchanged
source across all 324 gate inputs. Rename the local binding and its references
to an allowed name, preserving the compile/instantiate/export/summary behavior.
Retain the failing receipt, then refresh source hashes and affected verification
on the revised source before reporting the four Studio gates green. This review
did not edit the source or reproduce the already-observed lint failure.

### M1 — Minor: named numeric scenarios do not pin their defining inputs

`specimen-studio/tests/scoring-parity.mjs:1721` fixes the numeric scenario IDs,
count and order, but the loop at `:1726` passes each emitted scan/policy/draw vector
directly into `checkScore`. It does not independently assert the per-ID input
obligations. For example, by control-flow inspection, changing the `brainstorm`
input cap from 5 to 1 together with its internally consistent output would be
accepted under the same scenario name. Similarly, the name alone does not prove
that `crystallized` still supplies strength 100 or that an endpoint case still
supplies an exact endpoint. The election scenario loop at `:1751` has the same
general pattern for its named zero/singleton/endpoint cases.

The current frozen producer really does supply the intended distinct vectors
(`native/specimen-wasm/src/scoring_probe/numeric.rs:10`, `:71`, `:134`), and the
comparator independently checks their arithmetic and draw consumption. Therefore
this is a future coverage-loss risk, not an observed arithmetic mismatch or
missing current scenario. Pin defining per-ID inputs or explicit distinguishing
predicates and add an omission/renaming control if this hardening is taken up.
No mutation experiment or suite rerun was performed for this observation.

Critical: none established. Other Important findings: none established.

## Reviewed boundary and integrity checks

The supplied `task-8-scoring-parity-review.diff` is the ownership boundary. Its
SHA-256 is `3c3d33fc3204b7555b2002bd06aac51a286781bfb1177051803741602d70b54f`.
I read the exact diff once; where terminal output omitted a middle portion, I
read the corresponding frozen source rather than expanding the review to dirty
HEAD. A direct reconstruction confirms all eleven new-file diff bodies equal
their current files. The two minimal registrations only add the opt-in feature,
example, module and gated raw export.

Independent read-only hash checks found:

- All 13 current owned/registration files match canonical-final/source-freeze.json.
- All 25 current kernel/API/test inputs match the independently reviewed
  canonical-budget/fix1/source-freeze.json.
- All 80 preservation-baseline hashes match current files.
- All 57 source receipt entries, five tool binaries, the LLVM library and all
  16 retained artifact receipt entries exist and match their recorded hashes.
- The workspace report and canonical-final durable report are byte-identical.

These checks establish agreement with retained snapshots, not source
authentication or independent authentication of historical command execution.
No git commands, builds, tests, formatters, green-suite reruns, source changes,
staging, commits, installation, network, holdouts or live-store access occurred.
This review document is the sole review write. No subagents were used.

## Specification evidence

Paths below are relative to `specimen-studio/` unless otherwise stated.

1. **Actual public mechanisms and immutable projections.**
   `native/specimen-wasm/src/scoring_probe/numeric.rs:132` and `:142` call
   `numeric_policy` and `elect`; `:145` calls the seeded helper.
   `entries.rs:37`, `:258`, `:267`, `:290` call actual preparation, batch entry
   evaluation, arbitration and refusal evaluation. `entries.rs:174` supplies
   actual immutable prepared source/target pairs for broker scoring. Contributions
   are obtained from public evaluation and projected through getters in
   `project.rs:41`; none is constructed or deserialized by the probe. The broker
   scenarios use actual private handles, leases and issuer receipts from public
   calls, with no kernel seam or test-only production helper.
2. **Exact numbers and independent arithmetic.** `project.rs:7`, `:13`, `:28`,
   `:31`, `:41` retain rational components, seeds and large IDs as decimal strings;
   election weights retain original f64 bits. `tests/scoring-parity.mjs:11`
   checks canonical decimal syntax/ranges and reduced i128 fractions. Its BigInt
   calculation reconstructs negative-only ATP modulation, baseline/final clamps,
   inclusive thresholds, strength as a separate gate, independent magnitude/sign,
   no-draw exceptions and actual draw labels/cursors. `:1763` independently derives
   all six masked SplitMix64 outputs. Weight validation includes excluded slots
   and negative zero; authored interior integer-weight intervals use BigInt.
   The stated limitation about general compensated-f64 boundaries is appropriate.
3. **Scanned evidence, redundancy and refusals.** The comparator fixes four
   scanner component names/weights, validates complementary fractions and sums
   S/D/difference/confidence, and compares immutable contribution metadata to full
   input projections. `entries.rs:209` creates actual utility-fanout siblings;
   `:224` through `:250` distinguish parent lineage, typed inhibition domains,
   typed captures, attached/intermediate entries, dependency retention and nested
   selected actions. Independent grouping uses the full typed tree/root/lineage/
   capture/inhibition/dependency inputs, with stable survivors and original
   provenance retained. The twelve-entry/eleven-survivor and five-refusal counts
   are asserted. Low-confidence bypass retains the actual score; guard/blocked
   and stale-binding refusal paths are actual API results.
4. **Queues, completion barriers and quarantine.**
   `native/specimen-wasm/src/scoring_probe/orchestration.rs:26` calls the same
   broker for ten foreground/two maintenance jobs at active cap 2; every job is
   actually evaluated before completion. JS independently replays the complete
   required command list, FIFO and 8:1 ordering, shared capacity, duplicate lease
   refusal, early compose/seal refusal and final drain. The scenario at `:123`
   preserves cancelled active capacity across restart, rejects late results,
   distinguishes stale owner/current owner/old worker acknowledgements, accepts
   exactly one explicit fence acknowledgement, and rejects a stale creation
   handle after cycle-ID reuse. These are model transitions, not physical worker
   termination or host-authentication evidence.
5. **Actual attachments and composition.** `orchestration.rs:269` obtains a
   qualified issuer receipt, calls `attach`, actually evaluates both target
   entries, refuses the below-threshold vote and blocks discovery close until
   drain. Composition retains both surviving concrete nodes, dependency order,
   group, parent, remainder span and context projections. The accounting-only
   target completions in `budget_probe/attachment.rs:175` are explicitly labelled
   host no-vote, so they are not substituted for the original actual scans.
6. **Fixed portable decisions and atomic refusal.**
   `budget_probe.rs:162` and `:197` use identical literal limits in actual
   scoring/broker calls; `budget_probe/attachment.rs:48` adds the ten actual
   attachment rows. `tests/scoring-parity.mjs:1126` defines the fixed private-table
   specification independently of production imports, derives UTF-8/collection
   charges from the pinned fixture, and checks all 24 names/order/limits/outcomes.
   It derives 2971/2158/3355/2792, descriptor466, retained completion2296,
   broker274/before690 and completion limits2986/2304. Attachment calculation
   independently derives capture101, context extra521, root contribution2397,
   root-owned3087, work2937 and the selected/declined C/J/E cleanup limits2934/
   2930. The before/after/RNG/status/retry/duplicate/retirement fields are checked;
   refusal cannot be silently changed into a vote or consume its private lease.
   These are logical units, not allocator or physical-memory measurements.
7. **Semantic and diagnostic target comparison.** `scoring_probe/mod.rs:11`
   moves actual counters into labelled diagnostics; it does not alter their
   values. `tests/scoring-parity.mjs:1809` validates expected diagnostic paths and
   exactly 68 observations, each within its actual configured limit. At `:1951`
   the entire logical observation arrays must match across targets. Semantic
   projections must also match as serialized bytes; architecture labels stay
   separate. Direct artifact checks confirm both projections are exactly 243,932
   bytes with SHA-256
   `7125a31a9932c95a0df0cf42c505cfe988c29653d9d36f10237f7fa7182be4d8`.
   Raw wrappers retain the reported distinct byte lengths/hashes.
8. **Host-import and negative controls.** `tests/scoring-parity.mjs:1909`
   compiles/export-inspects the default module without instantiation and requires
   the scoring export absent. The feature path at `:1925` installs throwing
   wrappers for every actual function import, invokes the real raw export,
   copies its output, frees it and requires zero calls. The retained run reports
   progress/now_ms/random_u32 as the three imports and host_calls 0. All three
   corrupt raw fixtures are submitted to the same normal `--check-json` validator
   in separate processes by `scripts/check-scoring-parity.sh:55`. Retained logs
   show exit 1 specifically for exact effective confidence, cancel/quarantined
   transition and attachment-boundary refusal. Eight shape/omission/cursor controls
   plus malformed JSON refusal are also retained. None of these controls relies
   only on target equality.

## Inspected execution evidence and limits

The canonical-final `09-parity-frozen.log` contains the actual matching projection,
24-row/68-observation summary, all three expected negative exits and preservation
success. Its retained native/default/feature artifacts match the recorded hashes.
`10-feature-tests.log` contains 13 library tests plus three feature integration
tests, all passing; the disabled identity/language targets contribute zero tests.
`11-feature-clippy.log` records completed all-target Clippy with denied warnings.
The empty format/diff logs depend on the exit-zero verification receipt; they
were not independently rerun. Earlier 8/15-count summaries are historical and
were not used as the final count.

The script and receipts use pinned Rust1.99.1, offline/locked Cargo and isolated
retained targets. The explicit DYLD_LIBRARY_PATH names the existing pinned
toolchain library; the objcopy-load transcript actually reports LLVM23.1.1.
Current tool/library hashes match the receipts. This review does not silently
reinterpret earlier loader warnings or failed architecture-sensitive runs.

The root separately supplied fresh Studio gate progress and the actual broad-lint
failure above. I did not rerun those gates, and this report does not claim a
four-gate pass. Any source fix makes pre-fix freezes/gate receipts historical for
that revised source and needs the coordinator's new verification.

Cannot verify here: scientific efficacy, installed/default adoption, physical
memory/concurrency/work bounds, host authentication or real fencing/termination,
semantic completeness of host remainder/dependency declarations, ABI acceptance,
production cutover, or independent authenticity of every historical receipt.
All 163 requirements remain open/C0/unknown.
