# Task 3: named crash-boundary matrix (2026-10-27)

Tranche 2, Task 3 (`docs/plans/2026-10-27-wdbx-completion-tranche-2.md`).
This task adds tests only. No `src/` file changed: `store.rs`, `expiry.rs` and
every other source file are byte-identical to the task's base commit.
Requirements P03.04, P03.02, C18 and C19 stay `proposed`/C0, and every
execution register row stays `open`/`C0`/`unknown`. `register-v1.json` is not
edited.

New test: `crates/abi-wdbx/tests/v3_lifecycle_fault_matrix.rs`.

## Method

Each row runs the store up to a named boundary. It captures every file under
the store directory as exact bytes and writes back the mixture of pre-state and
post-state files that a crash at that boundary leaves. It then reopens the store
twice. The second reopen must leave every byte unchanged. The row then checks
what the store answers and, where there is one, the retry.

Each row prints one evidence line: a SHA-256 over every path and its bytes for
the composed crash state (`before`) and for the state after the second reopen
(`after`). Those states contain wall-clock retention deadlines, so the digests
change from run to run. The values below come from one captured run
(`fault-matrix-nocapture.log`, single-threaded). Only the equalities they show
are meant to be stable.

## Row table

| Row | Boundary | Simulated fault | before | after | reopen-1 = reopen-2 |
| --- | --- | --- | --- | --- | --- |
| `retention_stamp_synced_ledger_line_missing` | `RetentionIndex::commit` synced, ledger append not reached | pre-state files plus the post-state `episodes.v1.retention.jsonl` | `667eecf1…` | `667eecf1…` (reopen repairs nothing) | yes |
| `retry_after_crash_between_retention_and_ledger_extends_deadline` | same | same, followed by a retry of the same write ≥5 ms later | `77926efa…` | `77926efa…` | yes |
| `ledger_tail_partial` | ledger line torn mid-write | pre-state ledger plus `{"partial":` | `24e22eb3…` | `9ac77e4e…` (equals pre-state: tail truncated) | yes |
| `gc_reclaimed_committed_rename_blocked_then_reopen_then_retry` | reclaimed index committed, `episodes.v1.gc.tmp` cannot be created | real failed `apply_gc_plan` (blocking directory, as at `v3_deletion_lifecycle.rs:655`), store dropped, block removed | `7e877cf9…` | `7e877cf9…` | yes |
| `gc_temp_left_after_full_write` | temporary ledger fully written and synced, rename not reached | pre-crash snapshot plus the uninterrupted run's reclaimed index, with the uninterrupted run's ledger placed as `episodes.v1.gc.tmp` | `89022fcb…` | `89022fcb…` (temporary ignored) | yes |
| `sidecar_tmp_left_behind` | sidecar temporary fully written, rename not reached | pre-state files plus `.episodes.v1.object_keys.jsonl.tmp` and `.episodes.v1.quotas.jsonl.tmp` holding the post-state sidecars | `9e5bb601…` | `9e5bb601…` (temporaries ignored) | yes |
| `tombstone_never_resurrected_after_each_row` | after every row above | `supersedes` and `forgets` of each erased digest, before and after one more reopen | n/a | n/a | n/a |

What each row also asserts:

- Retention rows:
  - The orphan stamp leaves `retention_state(digest)` at `None` and
    `due_expirations(u64::MAX)` equal to the pre-crash list.
  - The retry produces the same digest and appends a second stamp line for it,
    so two lines name the digest. The orphan line stays in place.
  - The effective deadline equals the retry's stamp, both in memory and after
    reopen.
  - The extension row also asserts `retry > orphan` and that the candidate is
    not due at the orphan's deadline but is due at the retry's deadline.
- `ledger_tail_partial` builds on `v3_episode_store.rs:438,471`. It adds a
  second reopen and requires byte equality with the pre-state.
- Garbage-collection rows: both the interrupted run and the uninterrupted run
  start from one copied pre-crash snapshot and plan at the same `now_ms`.
  - After the reopen, the erased record is still in the ledger, and a re-plan
    still lists it.
  - The retry returns `Ok` and reclaims the record.
  - The resulting ledger bytes and the single-line reclaimed index equal the
    uninterrupted run's. No `episodes.v1.gc.tmp` is left.
- `sidecar_tmp_left_behind`:
  - The uncommitted key binding is not answered, and the uncommitted quota
    adjustment is not in force (the `quota_state` from before the crash is
    unchanged).
  - The retried commits succeed and remove both temporaries.
  - The object-key sidecar equals the uninterrupted bytes. The quota state is
    compared by its fields, because the quota line carries a wall-clock
    `adjusted_at_ms`.

## Discriminating mutations (temporary, reverted)

Every row passed on the unmodified code, so each family was proved
discriminating by a temporary source mutation. Each mutation was reverted from
a saved copy, and `git diff --quiet -- crates/abi-wdbx/src` was confirmed
afterwards.

| Mutation | Site | Rows that failed |
| --- | --- | --- |
| retention replay made first-wins (`entry().or_insert`) | `store/expiry.rs:286` | both retention rows and the tombstone sweep |
| `commit_reclamations` no longer skips an already-indexed digest | `store/gc.rs:696` | both GC rows (the reclaimed index gains a second line) and the tombstone sweep |
| ledger tail repair skipped | `store.rs:394` | `ledger_tail_partial` and the tombstone sweep |
| object-key sidecar temporary renamed `.tmp2` | `store/deletion/index.rs:368` | `sidecar_tmp_left_behind` and the tombstone sweep |

The tombstone sweep itself was not mutated. It reuses the refusal that
`v3_deletion_lifecycle.rs` (`a_reclaimed_record_never_becomes_live_again`)
already pins, and it fails whenever any row it runs fails.

## Open design question: last-wins and "bound once"

The `bind_retention` doc comment says the deadline "is bound once, here, at
admission" (`store.rs:715-720`), and the module doc of `expiry.rs` says the same.
The observed behavior after the crash at this boundary is different:

- A retry of the same write binds a second stamp line for the same digest.
- Both the in-memory index (`expiry.rs:238`) and the replay (`expiry.rs:286`)
  keep the last line. The effective deadline is therefore the retry's, which
  is later than the orphan's.

The matrix pins this **last-wins** behavior as observed. It does not fix it.
Whether the orphan's earlier deadline should win (first-wins), or whether the
retry should refuse to re-stamp, is an open design question. It is recorded
here and not resolved silently. Changing it would edit `expiry.rs`/`store.rs`
and needs its own reviewed task.

## Limits

- File composition simulates a crash at a named boundary. It does not prove
  power-loss, filesystem-ordering or hardware durability. No `fsync` behavior
  is exercised beyond what the store already does on the happy path.
- No row depends on the task-02 rejection-invariance envelope
  (`tests/support/rejection_invariance.rs`). The matrix has its own local
  helpers and its own store policy.
- The rows run on the current Mac only (APFS). No other platform or filesystem
  is qualified.
- Requirements stay `proposed`/C0. These tests are implementation evidence,
  not advancement.

## Logs

- `step2-tests.log`: `cargo test --locked -p abi-wdbx --test v3_lifecycle_fault_matrix --test v3_retention_expiry --test v3_retention_windows --test v3_deletion_lifecycle < /dev/null`.
  Results: 7 + 10 + 4 + 11 passed, 0 failed.
- `fault-matrix-nocapture.log`: the same matrix with `--nocapture --test-threads=1`,
  holding the per-row evidence lines.
- `check.log`: `bash tools/check.sh < /dev/null`, ending `check: all green`.
