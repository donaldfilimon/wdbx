# Task8.orchestration report — pure broker, attachments and completed graph

Status: implemented, uncommitted, stable for independent review. Final focused
26/0, full kernel 221/0, kernel all-target Clippy with `-D warnings`, owned rustfmt
and diff/whitespace checks passed. No staging, commits, installs, network, holdout
or live-store access occurred. The scorer, scanner, producers, identity, legacy
runtime/saves/commands, Cargo manifests/locks, substrate and frozen contracts were
preserved. Other dirty work was not reverted.

## Product manifest and scope

Repository root: `/Users/donald/sources/repos/wdbx`.

Five new kernel modules under
`specimen-studio/native/specimen-kernel/src/orchestration_v3/`: `mod.rs`,
`broker.rs`, `budget.rs`, `attachments.rs`, `compose.rs`. Minimal existing-file
registration: `pub mod orchestration_v3;` in kernel `src/lib.rs`. Three new test
targets: `orchestration_v3.rs`, `orchestration_v3_attachments.rs`,
`orchestration_v3_graph.rs`; shared synthetic fixture:
`tests/orchestration_v3/support.rs`. Ten product/test files, including the whole
lib registration file, have absolute paths, SHA-256 and line counts in
`docs/completion/execution/evidence/2026-10-23-scoring/orchestration/source-freeze.json`.
The manifest records a snapshot, not source authenticity. Earlier lib module
registrations are unrelated pre-existing work.

Read the task brief, operational profile, approved scoring report/API/review,
frozen scoring/scheduler design entries, architecture sections 9–11, Studio AGENTS
and both runtime profiles. Scope is the opt-in pure local protocol, not execution
of the legacy engine, support retrieval, S16 or a production scheduler.

## Stable API and protocol

Import `specimen_kernel::orchestration_v3::*`.

- `Broker::new(Population,generation,Limits)` creates one explicitly shared broker.
  The host must own exactly one per runtime domain. `begin` returns a private
  `CycleHandle` with owner identity, creation generation and incarnation. All
  cycle controls, including admission, discovery close, sealing, cancellation,
  deadline polling, retirement and composition require that handle plus explicit
  current scope/generation. Delayed controls cannot address a reused cycle ID.
- `Comparison` borrows immutable host snapshots: full EntryRecord, prepared
  source/target, CandidateOrigin, parent lineage, dependencies, group and kind.
  These are references to host-owned payload; the broker neither copies that
  payload at admission nor claims to measure external snapshot memory. Descriptor
  metadata is an explicit host contract, not a credential or authorization proof.
  Each `Job` binds exactly one entry/origin comparison and its unique cycle job ID.
- `admit` validates the entire batch, metadata, work/byte/count limits and duplicate
  IDs before queue/index allocation. It returns explicit `Backpressure`, never
  truncates accepted work. Foreground and attachments share FIFO; maintenance gets
  one quantum per nine dispatches while both classes remain queued.
- `dispatch` mints private broker/generation/serial-bound `Lease` tokens.
  `complete` accepts an optional private scorer Contribution, validates the entire
  binding and recomputes immutable scanner evidence for agreement. Wrong scope,
  stale/duplicate tokens or mismatched contribution fields do not release permits.
  A successful no-vote completion is a host report of finished work; the kernel
  cannot independently prove that a physical worker ran or returned no vote.
- Qualified completion returns a private `CompletedIssuer` receipt. A cycle
  retains every admitted record until completion/cancellation, then scoped
  `retire_cycle` can release its metadata. Retirement requires Composed or Partial
  and refuses pending, active or quarantined work. Receipts carry incarnation so
  retirement and ID reuse cannot revive old issuers. Cleanup releases no other
  cycle's records or permits.
- `cancel` removes pending work and marks active leases revoked. They remain
  counted/quarantined until `acknowledge_fence` validates the current owner generation and that
  specific lease's revoked worker generation, then releases it once. Delayed
  acknowledgements from a previous owner generation refuse without mutation. Revoked ordinary completion cannot
  publish or release early. `restart` advances the broker generation and cancels
  surviving cycles without freeing workers. After trusted acknowledgement, an
  original handle plus the current broker generation can retire its old cycle.
  `counts` exposes pending/active/quarantined/owned metadata accounting.
- `close_discovery` is an explicit trusted host declaration and refuses pending
  or active issuer work. `seal` additionally requires every record drained.
  Further jobs/attachments refuse after close/seal/cancel. `poll_deadline` takes
  supplied monotonic u64 instants, rejects regression, and yields typed Partial
  even for an expired Ready cycle. The kernel reads no clock.
- `attach` requires a qualified completed issuer, directed/bidirectional explicit
  concrete refs and explicit per-entry target jobs. Hard always schedules subject
  to validated bounds. Soft rational affinity [0,1] uses a separately labelled
  closed-u32-grid `AffinityTape`; endpoints 0/1 consume no draw. Trace records
  seed/stream/index/value, affinity and decision. Failed admission consumes neither
  RNG nor visit; explicit retry is supported. Accepted/declined visits are retained
  to prevent rerolls/repeated handoffs. Empty UTF8 remainder creates no job.
- Remainder is a checked UTF8 subspan of the original source and the prior delegated
  span. Context retains guards, captures, parent/source lineage and original
  prepared source (including numeric captures). Captures outside or partially
  outside the remainder remain contextual; only fully contained captures may be
  rebased into target authority. Root identity/span/bytes visits and internal depth
  (maximum four) never reset through handoffs; work enqueues rather than recurses.
  Targets use actual independent scorer evaluations; no attachment forces a vote.
- `compose` accepts only a sealed Ready cycle's completed private contributions,
  rejects missing refs/duplicates/cycles before random arbitration, invokes the
  reviewed exact scoped arbitration once, then returns an inert `CompletePlan`.
  Stable topological ordering uses admission order, not job numeric order or worker
  completion order. Nodes retain untouched Contribution proofs. Attachment groups
  preserve explicit issuers/member order, dependencies, source actions/facts,
  captures, inhibition/target flags and election/arbitration lineage. No semantic
  edge or executable host effect is invented. Partial/cancelled cycles never yield
  CompletePlan; no hidden voter reopen occurs.

## Budgets and evidence

Maximum active 1000; pending 100000; owned metadata/payload 16MiB. Caller limits may
lower these. Per-operation work ceiling is 1000000, trace 100000. Fixed conservative
metadata reservations cover queued/active/completed job records and cycle indexes;
full strings/vectors/trees are checked before copies. Scoring retains its own
reviewed bounds, while composition reserves conservative additional copy/key/output
space and supplies remaining work/bytes to arbitration. A byte/work ceiling can
refuse below nominal counts. Cleanup does not append historical events and cannot
be blocked by a full trace history; host retention and detached returned plans are
outside broker ownership. Accounting is a bounded logical model, not an allocator
or physical-memory measurement.

Raw NEW logs are under
`docs/completion/execution/evidence/2026-10-23-scoring/orchestration/`.
Every command used pinned cached offline Rust with:

```sh
PATH=/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:/usr/bin:/bin:/opt/homebrew/bin
RUSTUP_TOOLCHAIN=1.99.1
CARGO_NET_OFFLINE=true
cargo test --manifest-path specimen-studio/Cargo.toml -p specimen-kernel --locked --test orchestration_v3 --test orchestration_v3_attachments --test orchestration_v3_graph
# 37-focused-freeze.log: 26 passed, 0 failed (11 broker,12 attachment,3 graph)
cargo test --manifest-path specimen-studio/Cargo.toml -p specimen-kernel --locked
# 38-full-kernel-freeze.log: 221 passed,0 failed across23 summaries including docs
cargo clippy --manifest-path specimen-studio/Cargo.toml -p specimen-kernel --all-targets --locked -- -D warnings
# 39-clippy-freeze.log: exit0
rustfmt --check --edition 2024 specimen-studio/native/specimen-kernel/src/orchestration_v3/*.rs specimen-studio/native/specimen-kernel/tests/orchestration_v3*.rs specimen-studio/native/specimen-kernel/tests/orchestration_v3/support.rs
# 40-owned-fmt-freeze.log: exit0
```

`41-diff-check-freeze.log` records `git diff --check` on owned paths/lib registration
(exit0); git does not inspect untracked content, so `42-untracked-whitespace.log`
records a direct check of all ten frozen files too. `verification.json` records
final exit statuses/counts. The existing rust-objcopy SIGABRT/missing
`@rpath/libLLVM.dylib` debug-stripping warning remains visible; test commands exit0.
No warning suppression, installation or environment repair was attempted.

TDD/adverse evidence was retained. `01` is the absent API compile RED and `02`
first broker GREEN. `03` is a genuine ordering RED ([1,9] rather than admitted
[9,1]), `04` fixes it. `05/06` expose Unicode test fixtures that legitimately did
not qualify at threshold50; `07` explicitly configures threshold-100 only for those
mechanism issuers, preserving every guard and actual confidence. `08` is a test
borrow-lifetime compile error, corrected by `09`. `12` has two Clippy style
findings, fixed by `13`. Self-review found lifecycle metadata retention needed
explicit scoped retirement (`23` absent-API RED, `24` GREEN). Coordinator ruling
then required private handles for every cycle control: `25` retains local test
migration compilation errors, `26` is GREEN; `27` has needless-borrow test style
findings, corrected before final checks. `28/30` include stale handle/issuer/lease
and post-restart scoped retirement regressions. `36` is a behavioral RED where
a stale owner acknowledgement incorrectly released a quarantined old worker; the
final API checks current owner generation separately from revoked worker generation,
and `37` proves stale-owner refusal followed by exact old-worker release. These are representative test-first
cycles plus expansion, not a claim that every helper had a separate behavioral RED.

## Limits and handoff

No known remaining local test failure. Source/report are stable for independent
review; subsequent edits should be review fixes with new verification. Root owns
independent review and final four Studio gates; this worker did not run those.

All fixtures are synthetic mechanism data; no scientific efficacy or holdout claim.
Host ownership of one broker, snapshot/current utility coherence, semantic remainder
partition correctness, actual worker completion, actual host fencing/termination,
physical concurrency/parallelism and operational cancellation remain unqualified.
The host supplies authority and discovery completeness declarations. This protocol
checks containment/binding/order and refuses stale handles; it is not credential
validation. No native/WASM orchestration trace qualification was performed here.
Support retrieval/S16, Transient Memory, aftercare/tone, durable ATP dynamics and
full engine/save cutover remain explicit blockers. All163 requirements remain
open/C0/unknown; this bounded slice does not promote completion claims.


## Review fix1 — I1 and I2 (round1/5)

Status: corrected and stable for scoped rereview. Original report section above,
original raw logs, original source-freeze.json and verification.json were preserved.
All NEW fix evidence is under
`docs/completion/execution/evidence/2026-10-23-scoring/orchestration/fix1/`.
The new `source-freeze.json` records all eleven current product/test files, marks
which changed, and records the original manifest/raw-log hashes. Only
`orchestration_v3/attachments.rs`, `broker.rs`, `budget.rs` and the new
`tests/orchestration_v3_fix1.rs` changed in this fix. No public API, scorer,
producer, legacy, lib registration, manifest/lock or substrate change was made.

I1: attachment descriptor validation is now separate from selected-work capacity
reservation. All jobs' complete public metadata is bounded through one shared
operation Meter before any binding/equality, dependency contains, target capture
matching, duplicate-origin comparison or affinity sample. A second validation pass
checks population, binding, nonzero/duplicate IDs; existing cycle IDs are checked
after the metered visit test so valid repeat receipts retain `Repeated` behavior.
Declines receive the same validation as selected work. Each actual capture-pair
comparison charges both bounded names/values, each whole CandidateOrigin equality
charges both origins, and dependency/string comparisons have independent charges.
The selected branch alone reserves queue/job capacity; a validated decline is still
possible with a full pending queue. Every error precedes RNG/visit/accounting commit.

I2: the coordinator approved acceptance-time cleanup headroom before implementation;
root records the profile/ruling. The exact conservative logical bound is:

```text
R = 4 * retained_cycle_count + 4 * retained_job_record_count + E
R <= Limits.operation_work <= 1_000_000
E = sum of retained cycles' extra_bytes charges
```

The four cycle units cover cycle traversal/status/fixed cleanup bookkeeping; four
job units cover record/state/reference-drop/queue-reference structural visits.
Owned nested contribution/context/visit data additionally reserves its conservative
metered byte charge E, bounding nested container/drop work. These are logical
structural units, not machine-instruction or allocator-time measurements. Lookup
keys are fixed-size, and all owned variable data was bounded before acceptance.
Borrowed host snapshot payload is outside broker ownership and is not destroyed by
these cleanups. Extra-byte charges conservatively overreserve nested drop work.

`begin` reserves4, job acceptance reserves4/job, contribution completion reserves
its new metered owned-extra charge, and attachment acceptance reserves its owned
context/visit extra plus4 per selected job. All additions are checked before any
state/lease/RNG/visit mutation. Insufficient headroom is explicit `Backpressure`;
a valid contribution refusal leaves the comparison active and unpublished, and the
caller can retain/resubmit its proof after retiring other metadata. It is never
silently converted to a no-vote completion. Nominal caps remain upper bounds; in
particular100000 plain pending jobs need400000 cleanup units plus cycle units and
still pass the original exact-boundary fixture under the default1m limit.

Restart now makes one pass through retained cycles, transitions each applicable
job once, then clears each global queue once. It allocates no per-cycle ID vector
and never calls global queue retain inside that loop. No fallible work reservation
follows mutation; the accepted-inventory invariant already guarantees R within the
caller's limit. Single-cycle cancel makes its one local transition pass and one
pass over global queues. Revoked active leases remain counted/quarantined.

Cancellation/restart release E only after owned contributions and contexts have
been dropped and the visit vector replaced/dropped. They keep all4/cycle and4/job
reservations for retained cancelled or quarantined records. Fence acknowledgement
keeps that job reservation because the record remains. Scoped retirement releases
only its removed cycle/job/extra reservations. Composed metadata remains retained
and reserved until explicit retirement; restart neither drops it nor credits it.
Cleanup has no historical event append and cannot fail midway on a full trace.

Eight new tests cover oversized capture/dependency vectors for hard, soft0 and
interior selected/declined draws, invalid identity/population on declines,
queue-full declines, shared nested matching under a lowered20000 work limit,
no-RNG/no-visit failure and retry, exact lowered reservation boundaries,2046 empty
retained cycles after one active record at8192 units, atomic completion/attachment
headroom refusal, and a2000 one-job-cycle restart under24000 units. The latter
asserts generation advance, one composed result retained,1997 pending removed,
three active workers quarantined until individual acknowledgements, and isolated
retirement/accounting restoration. Assertions use deterministic state/count/bound
arithmetic; none claim wall-clock or physical concurrency evidence. Existing26
orchestration tests, including exact100000 pending and1501 complete drain, still pass.

Final commands from repository root used the same pinned1.99.1/offline environment
as above, with `--locked` on every Cargo command:

```sh
cargo test --manifest-path specimen-studio/Cargo.toml -p specimen-kernel --locked --test orchestration_v3_fix1 --test orchestration_v3 --test orchestration_v3_attachments --test orchestration_v3_graph
# fix1/06-focused-final.log:34 passed,0 failed (11+12+8+3)
cargo test --manifest-path specimen-studio/Cargo.toml -p specimen-kernel --locked
# fix1/07-full-kernel-final.log:229 passed,0 failed across24 summaries
cargo clippy --manifest-path specimen-studio/Cargo.toml -p specimen-kernel --all-targets --locked -- -D warnings
# fix1/08-clippy-final.log:exit0
rustfmt --check --edition 2024 specimen-studio/native/specimen-kernel/src/orchestration_v3/*.rs specimen-studio/native/specimen-kernel/tests/orchestration_v3*.rs specimen-studio/native/specimen-kernel/tests/orchestration_v3/support.rs
# fix1/09-owned-fmt-final.log:exit0
```

`fix1/10-diff-check-final.log` records `git diff --check` on the owned source/new
test paths; `fix1/11-untracked-whitespace-final.log` additionally checks all eleven
frozen files directly. `fix1/verification.json` records final exits/counts. The
existing rust-objcopy SIGABRT/missing libLLVM warning remains; test exits are0.

Adverse evidence is retained: `fix1/01-review-red.log` reproduces declined oversized
descriptor acceptance and missing lowered cleanup headroom. `02-first-green.log`
exposes changed repeat-error precedence; validation/identity phases were separated
before `03-review-green.log`. `04-adverse-expansion.log` exposed a nested-capture
fixture whose8192-unit combined cleanup budget correctly refused its proposed good
retry; the final test uses64 inherited captures and20000 units, independently
forcing the expensive last-capture match to refuse while the first-match retry
succeeds (`05-adverse-green.log`). No failed raw log was overwritten or omitted.

No additional feature or profile edit was made by this worker. Independent scoped
rereview and root's final Studio gates remain pending; no dependent probe was run.
All original host execution/fencing/authorization/semantic partition and broader
completion limits remain in force, and all163 requirements remain open/C0/unknown.
