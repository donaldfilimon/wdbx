# wdbx: local main integration readiness

## Goal and authority

User-approved documentation task, 2026-10-25. Prepare owned work for local
`main` only after ownership release, preservation and the repository gates.
Status: **pending owner confirmation and fresh checks**, not merge-ready.
Existing product plans and qualification contracts remain authoritative.

## Observed Git state

Inventory recorded on 2026-10-25; re-read Git and writer state before acting.
Canonical branch: `main`; HEAD: `36bc99b699de60be6c07b2838d74580084fb5d4c`.
Remote comparisons below use existing local tracking refs; no fetch was run.

| Local branch | Pinned tip | Commits only on main / only on branch | Cached upstream state |
| --- | --- | --- | --- |
| `main` | `36bc99b699de` | 0 / 0 | origin/main  |
| `task/modernization-abbey-gtk-20261025` | `81ae29a7c472` | 0 / 4 | none  |
| `task/modernization-baseline-20261025` | `6372def9afbc` | 0 / 1 | none  |
| `task/modernization-consumer-dependencies-20261025` | `81ae29a7c472` | 0 / 4 | none  |
| `task/modernization-dependency-cohort-20261025` | `6372def9afbc` | 0 / 1 | none  |
| `task/modernization-native-model-contracts-20261025` | `cb129cf95910` | 0 / 7 | none  |
| `task/modernization-source-custody-20261025` | `755ba2e3211c` | 0 / 3 | none  |
| `task/modernization-studio-gtk-20261025` | `70102403788b` | 0 / 11 | none  |
| `task/modernization-studio-zip9-20261025` | `0a1f8a8dce71` | 0 / 8 | none  |
| `task/modernization-updater-zip9-20261025` | `81ae29a7c472` | 0 / 4 | none  |
| `task/modernization-worker-lock-20261025` | `755ba2e3211c` | 0 / 3 | none  |

| Existing worktree (relative to DEV_HOME) | Branch | Pinned HEAD | Tracked dirty / untracked files |
| --- | --- | --- | --- |
| `projects/company/legacy/wdbx` | `main` | `36bc99b699de` | 12 / 1077 |
| `worktrees/local/the-donald-company/products-modernization-20261025/cohorts/000-baseline/wdbx` | `task/modernization-baseline-20261025` | `6372def9afbc` | 2 / 0 |
| `worktrees/local/the-donald-company/products-modernization-20261025/tasks/002-source-custody/wdbx` | `task/modernization-source-custody-20261025` | `755ba2e3211c` | 2 / 0 |
| `worktrees/local/the-donald-company/products-modernization-20261025/tasks/003-dependency-cohort/wdbx` | `task/modernization-dependency-cohort-20261025` | `6372def9afbc` | 2 / 0 |
| `worktrees/local/the-donald-company/products-modernization-20261025/tasks/004-worker-lock/wdbx` | `task/modernization-worker-lock-20261025` | `755ba2e3211c` | 2 / 0 |
| `worktrees/local/the-donald-company/products-modernization-20261025/tasks/005-consumer-dependencies/wdbx` | `task/modernization-consumer-dependencies-20261025` | `81ae29a7c472` | 2 / 0 |
| `worktrees/local/the-donald-company/products-modernization-20261025/tasks/007-updater-zip9/wdbx` | `task/modernization-updater-zip9-20261025` | `81ae29a7c472` | 2 / 0 |
| `worktrees/local/the-donald-company/products-modernization-20261025/tasks/008-studio-zip9/wdbx` | `task/modernization-studio-zip9-20261025` | `0a1f8a8dce71` | 2 / 0 |
| `worktrees/local/the-donald-company/products-modernization-20261025/tasks/009-abbey-gtk/wdbx` | `task/modernization-abbey-gtk-20261025` | `81ae29a7c472` | 0 / 0 |
| `worktrees/local/the-donald-company/products-modernization-20261025/tasks/010-studio-gtk/wdbx` | `task/modernization-studio-gtk-20261025` | `70102403788b` | 2 / 1 |
| `worktrees/local/the-donald-company/products-modernization-20261025/tasks/011-native-model-contracts/wdbx` | `task/modernization-native-model-contracts-20261025` | `cb129cf95910` | 0 / 0 |

Worktree locations describe custody, not runtime dependencies. Clean status does
not prove a writer has released a worktree. Existing ignored assets remain in
their original custody; this task neither inventories their contents nor removes
them. App-managed worktrees retain their application-managed lifecycle.

Canonical dirty-file state (grouped to keep this plan readable):

| State | File or subtree | Files |
| --- | --- | --- |
| tracked dirty | `AGENTS.md` | 1 |
| tracked dirty | `crates/abi-wdbx` | 6 |
| tracked dirty | `docs/contracts` | 1 |
| tracked dirty | `specimen-studio/AGENTS.md` | 1 |
| tracked dirty | `specimen-studio/native` | 3 |
| untracked | `crates/abi-wdbx` | 21 |
| untracked | `docs/agent-guide.md` | 1 |
| untracked | `docs/completion` | 962 |
| untracked | `docs/execution` | 2 |
| untracked | `docs/superpowers` | 1 |
| untracked | `specimen-studio/native` | 77 |
| untracked | `specimen-studio/scripts` | 3 |
| untracked | `specimen-studio/tests` | 3 |
| untracked | `tools/check_completion_execution.py` | 1 |
| untracked | `tools/check_p1_crypto.sh` | 1 |
| untracked | `tools/check_substrate_release.py` | 1 |
| untracked | `tools/tests` | 3 |
| untracked | `tools/wdbx_complete_p1_crypto.py` | 1 |

Counts cover every path from the captured short-status inventory. The separate
local execution inventory retains the complete path list. Re-enumerate paths
with `git status --short --untracked-files=all` before any integration.

This documentation task uses `task/agent-main-plans-20261025`, based on
`36bc99b699de60be6c07b2838d74580084fb5d4c`. Its worktree is separate from the
observed checkouts above. Existing uncommitted guidance was not copied into this
branch or claimed as this task's work; reconcile it with its owner at integration.

## Ownership and repository blockers

Coordinate the active modernization cohorts and canonical storage/Studio writers. Preserve independent historical and cross-language oracles. Studio has its own AGENTS.md and gates; substrate checks do not qualify Studio or external TPU/accelerator hardware.

Each existing worktree remains with its current writer until release is
confirmed. The inventory establishes branch custody, not writer identity or
authorization to merge. An unpublished commit is retained local work, even when
its branch is already reachable from main. Do not discard it or its branch label
as cleanup.

## Integration checklist

- [ ] Re-read AGENTS.md, this plan, active product plans, refs and all worktree
  statuses. Confirm each selected writer has released an exact commit/file set.
- [ ] Preserve all refs and dirty/untracked work before integration; preserve
  required ignored assets separately and verify recovery. Keep app worktrees.
- [ ] Classify every branch as already reachable, selected, owner-held or deferred
  with a reason. Review main-only/branch-only commits and file differences;
  do not merge all branches merely because they exist.
- [ ] Use one manual integration branch/worktree for the selected changes and
  an isolated writable build/test environment. Reuse only documented sibling
  layouts. Commit released dirty work on its owning branch after preservation.
- [ ] Integrate selected commits in dependency order, resolve conflicts against
  existing contracts and retain failed checks and provenance pins.
- [ ] Run the documented gates below against the exact candidate revision;
  record command, revision, result and unmet platform/external prerequisites.
- [ ] Recheck main HEAD, canonical dirty state and writer ownership immediately
  before local integration. Advance main only if those guards still hold.
- [ ] Record resulting commit and remaining work. Preserve recovery refs and
  worktrees until their owners release them and ignored-asset custody is verified.

## Required checks and completion evidence

Run from this repository's root, respecting its wrapper/toolchain prerequisites:

```sh
bash tools/check.sh
git diff --check
```

For this documentation task, use the repository's scoped documentation/instruction
checks and record actual results separately from the full integration gate.
No product gate is claimed by writing this plan. Readiness requires ownership
release, preserved work and passing applicable checks on the candidate; failures,
skips or unavailable prerequisites remain explicit blockers. Platform, hardware,
model and production qualification retain their existing independent criteria.
Publishing, remote merges, releases and deployment are outside this task.
