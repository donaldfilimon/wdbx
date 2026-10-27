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
