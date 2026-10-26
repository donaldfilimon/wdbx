# SDD ledger — plan: docs/plans/2026-10-26-wdbx-preservation-continuation.md

Baseline: ad92e81. Canonical main is clean except this new task-owned plan. Prior consolidation remains complete. No new worktree needed under explicit user authorization to continue on main.

## Preflight

| Tasks | Shared interface or internal consistency | Finding |
| --- | --- | --- |
| 1, 2 | Both preserve historical identities; distinct files and CLIs | Independent implementation, sequential writers |
| 1, 3 | Task 1 changes scripts whose current execution references Task 3 refreshes | Refresh after source stabilizes |
| 2, 3 | Task 2 diagnostic output remains nonqualifying, Task 3 records failed live gate | No contradictory pass claim |
| 1 | Reviewed successor exact membership vs retained historical mode | Consistent; independent review before final gate claims |
| 2 | Diagnostics vs default failure behavior | Consistent; no source-version migration accepted implicitly |
| 3 | Local passing checks vs external failures | Both recorded independently |

Ruling: Use subagent-driven implementation sequentially and parallel read-only investigation — user invoked both execution methods; prevents shared-checkout conflicts — costs extra review time.
Ruling: Retain the frozen release live failure while adding diagnostics and tests — its catalog lacks newly added model declarations and cannot be silently repinned — costs a remaining release migration blocker.

Task 1: started at ad92e81.

Task 1: Ruling: Include the language parity script as a fifth explicitly transitioned physical file — original scoring preservation includes that script, so required checker wiring cannot preserve its old hash — costs one additional reviewed transition and mutation regression; original baseline stays unchanged.

Task 1: complete (commits ad92e81..a2e2f26, spec/quality review clean; preservation 11, GTK Python 12, GTK diagnostics 1, archive 12 pass). Reviewer archival-test condition resolved by final exit0 log.
Task 2: started at a2e2f26.

Task 2: fix round 1/5 (1 addressed, 0 open — JSON mode stderr diagnostics; commits c2178a4..97af6e5, RED/GREEN 52 tests).
Task 2: complete (commits a2e2f26..97af6e5, spec/quality review clean after scoped fix review).
Task 3: integration validation started at 97af6e5.

Final review: clean, no actionable findings or deferred minors. Parent completion gate resolved: final completion suite 64/64 pass; release 52/52 and preservation 11/11 pass on Python 3.9; root and Studio browser gates exit0; both full current parity gates exit0.
Final: Ruling: Broader production, optional crypto, hardware and platform qualification remain unproved by this checker tranche — retain their independent acceptance gates — costs separate external validation work.
Final: Ruling: The separate source/version migration remains unaccepted — new model declarations require reviewed inventory evolution and default live refusal is correct — costs the retained release blocker.
Task 3: complete (final source 97af6e5; reference refresh independently reviewed; final command exits and hashes in validation.json).
