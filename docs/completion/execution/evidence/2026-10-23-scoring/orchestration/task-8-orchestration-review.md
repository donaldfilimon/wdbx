# Task8.orchestration independent review

Spec compliance verdict: **CHANGES REQUIRED** for the bounded orchestration slice.

Code quality verdict: **CHANGES REQUIRED**. Two Important findings concern the
declared operation-work boundary. No Critical finding was established. The
generation/incarnation fencing, retirement isolation and completed-plan barrier
are substantially sound in this captured tranche.

## Scope and method

Reviewed the brief, implementation report, operational profile and captured task
diff. The coordinator corrected the original package to
`task-8-orchestration-complete-review.diff` because its glob had omitted
`tests/orchestration_v3/support.rs`; the additional fixture hunk was reviewed
without repeating the already reviewed product diff. Terminal truncation required
reading the missing middle hunks separately. The complete ten-file package is the
review boundary, excluding unrelated pre-existing lib registrations and dirty work.

Read Studio AGENTS and the approved scoring review. Followed only the concrete
unchanged-dependency questions listed below. All ten product/test files match the
SHA-256 and line counts in the reported source freeze. This establishes snapshot
agreement, not independent source authenticity.

No tests, builds, formatters, git commands, network, installs, holdouts or live
stores were used. No product code was changed, staged or committed. This report
is the only review mutation. Findings below follow from the source; suggested
regressions were not executed by this reviewer.

Unless otherwise stated, source/test paths below are relative to
`specimen-studio/native/specimen-kernel/`.

## Important findings

### I1. Preflight attachment job metadata before traversing it, including declines

Evidence: `src/orchestration_v3/attachments.rs:276-315,349-351`.

The initial meter charges the outer jobs slice and the issuer's captures, but
does not charge each supplied job's metadata. The next loop searches
`c.dependencies` and traverses every `c.origin.captures` element, performing a
nested search through inherited captures and variable-length value comparisons.
Only afterward does `self.preflight` validate/charge the supplied comparisons,
and that call is skipped entirely when soft affinity declines. Charging one unit
for pairwise job-origin equality also does not cover its variable-length origin
comparison.

A concrete input is one otherwise matching target job whose public
`CandidateOrigin.captures` contains an arbitrarily large repetition of a valid
whole inherited capture. Every capture passes the containment predicate, so
`attach` traverses the entire caller-sized vector before it can reject the size.
With soft affinity zero, it never applies the comparison preflight at all and can
return `AffinityDeclined`, recording the visit, after this unbounded traversal.
A very large dependency slice with the issuer at its end supplies another
unmetered traversal. These are public borrowed descriptors, so the private
scorer Contribution bounds do not prevent these inputs.

This violates the brief's preflight-before-work requirement and the profile's
per-operation ceiling. It also means invalid descriptor identities/populations
can be consumed as a declined visit without the validation used for selected
jobs. This is a local validation issue; it does not depend on qualifying host
credentials or semantic partition correctness.

Required correction: validate and charge all supplied comparison metadata before
dependency/capture/equality traversal or sampling, using a shared operation meter.
Charge nested matching/equality work conservatively or use bounded indexes. Keep
descriptor validation separate from selected-work capacity reservation if declines
should remain possible when the queue is full. A failure must preserve the tape,
visit set and broker accounting. Add adverse selected/declined attachment cases
with oversized captures/dependencies and a lower caller work limit; preserve the
existing explicit retry behavior.

### I2. Restart performs quadratic global queue cleanup without an operation bound

Evidence: `src/orchestration_v3/broker.rs:477-507,534-548`, especially the two
global `retain` calls at lines 500-501 inside the per-cycle restart loop.

`restart` collects every surviving cycle and calls `cancel_inner` once per cycle.
Each call scans both global queues, including unrelated cycles' pending records.
For N open cycles with one foreground job each, restart performs
N+(N-1)+...+1 queue predicate visits. N=2000 already means 2,001,000 predicate
visits, before cycle/job work, while those inputs are comfortably within the
100000 pending and 16MiB logical metadata limits. No meter or other ceiling
limits that operation. Larger allowed populations amplify this into hundreds of
millions of visits in the pure kernel.

The global count caps therefore do not establish the advertised 1,000,000
operation-work bound. Cancellation must also remain available without depending
on a full history buffer, so simply returning Backpressure midway through this
mutating loop would not be an adequate correction.

Required correction: make restart's queue removal a single global pass (or clear
the queues once after all pending records are transitioned), and make cleanup's
work guarantee explicit for lowered caller limits. If a bounded incremental
protocol is necessary, it must retain fencing/capacity invariants throughout;
do not release revoked workers to satisfy the budget. Add a many-small-cycles
regression covering pending removal, generation advance, quarantine retention and
unrelated retained/composed metadata. Establish the work bound structurally or
with deterministic operation accounting, without claiming wall-clock evidence.

## Critical and Minor

Critical: none established.

Minor: none separately reported. The two Important findings are approval blockers;
this review does not inflate them into an authorization or physical concurrency
failure.

## Strengths and contract evidence

- Shared limits and admission: `src/orchestration_v3/broker.rs:115-151,236-310`
  checks maximum limits, pending/cycle headroom, job identity, descriptor bounds
  and duplicate batch IDs before insertion. Accepted records remain in the cycle;
  dispatch removes only queue references. `broker.rs:312-369` applies shared active
  capacity and FIFO dispatch with eight foreground quanta followed by maintenance
  while both exist. Foreground includes attachment insertions.
- Fencing: `broker.rs:371-422,477-507,551-579` validates private owner/serial/
  generation leases, refuses revoked ordinary completion, retains revoked active
  capacity, and requires both current owner generation and revoked worker
  generation for a once-only acknowledgement. Restart advances generation without
  releasing surviving workers. The old-owner acknowledgement regression is
  present in `tests/orchestration_v3.rs:221-293`.
- Incarnations and retirement: `broker.rs:27-62,171-224,580-610` uses a private
  creation handle with owner/incarnation/creation-generation checks for cycle
  controls. Retirement requires Partial or Composed and no active, pending or
  revoked records, releasing only that cycle's accounting. The stale controls,
  stale lease and reused IDs regression is at
  `tests/orchestration_v3.rs:424-486`; stale issuer rejection is at
  `tests/orchestration_v3_attachments.rs:626-673`.
- Completion barrier: `broker.rs:423-451` requires explicit discovery closure and
  all records Done before Ready. `broker.rs:511-530` uses supplied monotonic
  instants and converts an expired Open or Ready cycle into Partial. Admission,
  attachment and composition reject closed or partial lifecycle states.
- Scorer proof binding: `src/orchestration_v3/budget.rs:192-212` compares the
  concrete record, candidate origin, source bytes, target origin, parent lineage,
  dependencies, group, kind and scanner version, and recomputes scanner evidence.
  It accepts only qualified private Contributions. A hard attachment schedules
  evaluation without forcing a contribution; the below-threshold target fixture
  demonstrates this at `tests/orchestration_v3_attachments.rs:24-68`.
- Attachment semantics apart from I1: `attachments.rs:177-263,285-309,353-391`
  checks issuer owner/generation/incarnation, explicit direction/target, UTF8
  containment within the previous delegated span, depth four, root-based visits
  and retained contextual captures. Whole captures alone may be rebased into
  target authority. Context preserves original source, guards and lineage.
  Exact rational affinity has no-draw endpoints and explicit trace identity
  (`attachments.rs:54-97`); failed selected admission precedes tape/visit mutation.
- Inert graph: `src/orchestration_v3/compose.rs:30-75,78-189` validates explicit
  references, duplicates and cycles before arbitration, starts from admission
  order, preserves immutable contributions and attachment context, and changes
  status only after successful composition. Referenced dependencies and attached
  contributions are exempt from redundancy arbitration in the reviewed scorer.
  No source/semantic edge or host effect is invented.
- Evidence includes the 1501-work drain, exact 100000 pending boundary, shared
  foreground/maintenance FIFO, cancellation isolation, partial states, Unicode
  remainder/capture handling, repeat/depth limits, retry without reroll, multiple
  target entries, complementary facts/dependencies and once-only arbitration.
  These mechanism fixtures are materially useful, although they miss I1/I2.
- The exact product diff adds only five orchestration modules and the lib export;
  it does not alter legacy IDs, saves, producer/scanner/scorer APIs, manifests or
  locks. The report candidly keeps active-engine integration and broader completion
  requirements open.

## Focused unchanged-dependency checks

- `src/scoring_v3/contribution.rs:44-111,119-160`: checked whether a caller could
  forge a Contribution or whether complete's binding trusts an externally supplied
  score. Fields are private to scoring, getters are immutable, and construction
  performs preflight plus actual scanner evaluation. This does not bound the public
  attachment Comparison metadata implicated in I1.
- `src/scoring_v3/arbitration.rs:80-208`: checked whether composition could consume
  a draw on failure or remove a dependency parent. Arbitration validates references,
  exempts referenced parents/attachments/facts, uses a local cursor, and commits the
  tape only on successful output.
- `src/scanner_v3/routes/model.rs:118-146`: checked the shape and mutability of the
  attachment guards and captures. Constraints are fixed-size; CandidateOrigin's
  capture/token vectors are public, confirming I1's caller-sized-input path.
- `src/scanner_v3/mod.rs:167-220`: checked what complete's evidence recomputation
  actually does; it scans the immutable PreparedAction pair rather than accepting
  caller-asserted evidence. This review does not requalify scanner semantics.

No broad dependency audit or producer review was performed.

## Verification and cannot verify

Parsed the supplied raw final logs: `37-focused-freeze.log` has 26 passed/0 failed
across three summaries; `38-full-kernel-freeze.log` has 221 passed/0 failed across
23 summaries. `39-clippy-freeze.log` records successful completion. Empty owned-fmt
and diff-check transcripts cannot independently prove their exit statuses; their
exit-zero status is reported in `verification.json`. The direct untracked
whitespace transcript records ten checked files. The existing rust-objcopy
SIGABRT/missing `@rpath/libLLVM.dylib` warning remains disclosed; it does not change
the parsed test totals. No verification was rerun.

Cannot verify here: actual worker execution/no-vote completion, trusted host fence
execution or authentication, one-broker ownership in a live runtime, physical
parallelism/fairness/cancellation, allocator memory consumption, external snapshot
memory, semantic completeness of host attachment partition/discovery, independent
native/WASM traces, full Studio gates, scientific efficacy or every historical
RED/GREEN transcript's authenticity. The source/report correctly leave these
unqualified. A public host declaration is not an authentication credential.

Support retrieval/S16, Transient Memory, aftercare/tone, durable ATP dynamics and
full engine/save cutover remain blockers. No requirement is promoted: all 163 stay
open/C0/unknown. Fix I1/I2 and provide a narrowly captured review-fix diff with new
verification before approving this slice.
