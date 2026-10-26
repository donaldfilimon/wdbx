# Task8.orchestration fix1 scoped rereview

Spec compliance verdict: **PASS for the I1/I2 fix**, against the coordinator's
approved acceptance-time cleanup reservation and updated operational profile.

Code quality verdict: **PASS for the scoped fix**. I1 and I2 are **ADDRESSED**.
No new Critical or Important defect was established in the fix diff. This closes
the two blockers from the initial review; it does not widen that review's scope
or qualify host execution.

## Scope and method

Reviewed `task-8-orchestration-fix1-review.diff` once, recovering only the omitted
middle section after terminal truncation. Reviewed the standalone durable fix1
report and the updated cleanup contract in
`docs/completion/execution/scoring-orchestration-v3-profile.md:99-109`.
The boundary is the changes to `attachments.rs`, `broker.rs`, `budget.rs` and new
`tests/orchestration_v3_fix1.rs`, plus their effect on the previously established
I1/I2 invariants. No broad task or unchanged-dependency audit was repeated.

All eleven current product/test files match fix1's recorded SHA-256 and line
counts. Parsed supplied raw final test summaries and inspected the verification,
Clippy and whitespace receipts. No tests, builds, formatters or git commands were
rerun; no product edits, staging, commits, installs, network, holdout or live-store
access occurred. This report is the only mutation.

Paths below are relative to `specimen-studio/native/specimen-kernel/` unless
otherwise stated.

## I1 — ADDRESSED

`src/orchestration_v3/attachments.rs:243-244` now passes all supplied descriptors
through the shared operation meter before visit matching, dependency searches,
capture matching, duplicate-origin equality or sampling. The implementation in
`broker.rs:255-287` first bounds every comparison's variable metadata, then checks
binding, population and nonzero/duplicate batch IDs. The extracted
`budget.rs:148-179` helpers charge the variable-length binding and CandidateOrigin
equality data rather than treating those comparisons as constant work.

The attachment loop charges dependency and string comparisons, and each attempted
capture pair charges both bounded captures before equality
(`attachments.rs:285-326`). Duplicate-origin comparisons likewise charge both
candidates. Existing job identity checks follow the metered visit test, retaining
the original `Repeated` precedence, and precede sampling
(`attachments.rs:328-340`).

Descriptor validation is unconditional for hard, soft-zero, selected interior and
declined interior paths. Queue/job capacity reservation is separate and remains
selected-only (`attachments.rs:359-378`), so a valid decline is still possible
with a full pending queue. All fallible validation/reservation occurs before
insertion, visit/accounting mutation and tape advancement. `sample` still reads
without committing the tape, so later headroom refusal preserves retry behavior.

The new tests cover oversized caller capture/dependency vectors on all four
paths, atomic counts/RNG and a valid retry (`tests/orchestration_v3_fix1.rs:20-85`),
lowered-budget nested matching with a cheap retry (`:109-172`), and identity/scope
validation on declines while the queue is full (`:175-245`). These directly cover
the input paths identified in I1.

## I2 — ADDRESSED

The fix enforces the approved logical inventory reservation:

```text
R = 4 * retained cycles + 4 * retained job records + sum(extra_bytes)
R <= Limits.operation_work
```

`budget.rs:5-7` names the structural constants. `broker.rs:226-235` checks added
reservation with checked arithmetic. Begin reserves before inserting a cycle
(`:179-197`); job reservation precedes insertion and charges each retained job
(`:236-253,342`). Completion reserves its contribution's extra charge before
changing state or removing the active lease (`:425-457`). Attachment acceptance
checks the combined context/visit extra and selected-job reserve before committing
either (`attachments.rs:368-407`). The combined checks prevent independently
valid additions from exceeding the shared inventory ceiling.

Restart now traverses retained cycles once, calls the local record-transition
helper only for non-Partial/non-Composed cycles, then clears each global queue
once (`broker.rs:584-608`). It allocates no cycle-ID buffer and performs no
per-cycle global retain. No fallible reservation follows mutation. Single-cycle
cancellation still makes one local pass and one scan of each global queue; the
shared retained-inventory reservation bounds that structural cleanup as well.

`cancel_records` drops contribution/context references and replaces the visit
vector before returning the released extra charge (`broker.rs:535-560`). Both
cancel and restart release only that extra amount. They retain the cycle/job base
reservation for cancelled and quarantined records; fence acknowledgement does not
credit retained records. Composed cycles and their extra remain charged across
restart. Retirement subtracts the removed cycle's exact base/job/extra reserve
after the existing drained/fenced checks (`broker.rs:639-671`). The accounting
changes do not introduce a path to release active worker capacity early.

Completion headroom refusal returns Backpressure before publishing or consuming
the active lease; it does not change a real contribution to a no-vote result.
The caller must retain a proof for retry, as demonstrated by the regression that
submits a clone before the expected refusal (`tests/orchestration_v3_fix1.rs:345-385`).
Attachment headroom refusal similarly preserves RNG/visit state and retries after
metadata retirement (`:388-450`). Lowered-limit boundaries and retained cancelled
cycle reservations are covered at `:88-106,248-275`.

The 2000-small-cycle regression (`:278-342`) verifies pending removal, generation
advance, composed metadata retention, three quarantined workers, individual fence
acknowledgements and scoped retirement restoring the remaining accounting. The
linear-work conclusion comes from the changed control flow and reservation
invariant; elapsed test time is not used as concurrency or work-bound evidence.

## New fix-only findings

Critical: none established.

Important: none established.

Minor: none separately reported.

## Supplied verification and qualifications

Fix1 `06-focused-final.log` contains 34 passed/0 failed across four summaries;
`07-full-kernel-final.log` contains 229 passed/0 failed across 24 summaries.
`08-clippy-final.log` records successful completion. The format/diff transcripts
are empty, so their exit-zero status remains supplied by `verification.json`.
The direct whitespace receipt records eleven checked product/test files.
The report retains the existing rust-objcopy/libLLVM warning and original evidence;
this rereview does not independently authenticate historical command execution.

The accepted reservation is a conservative logical structural/drop model, not an
allocator, instruction-count or wall-clock measurement. It intentionally permits
Backpressure below nominal pending/byte limits, including on completion when
additional owned proof retention lacks cleanup headroom. Host snapshot ownership,
returned-plan ownership and actual host termination remain separate assumptions.

All prior qualifications remain: no actual worker execution/no-vote proof, host
fencing/authentication, one-broker deployment, physical parallelism, semantic
partition/discovery completeness, independent native/WASM parity, scientific
efficacy or full Studio gates are established by this rereview. Support retrieval,
Transient Memory, aftercare/tone, durable ATP and full engine/save cutover remain
outside this slice. All 163 requirements remain open/C0/unknown.
