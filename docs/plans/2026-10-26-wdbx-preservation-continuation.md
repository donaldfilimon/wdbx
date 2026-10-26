# WDBX Preservation Continuation Implementation Plan

**Goal:** Restore useful strict Studio parity gates across reviewed modernization while preserving historical evidence, and make release source drift independently testable and actionable.
**Architecture:** Explicit hash-bound successor transitions for five Studio files; immutable original baselines remain available through historical mode. Substrate release continues refusing unreviewed source evolution, with isolated live tests and grouped diagnostics.
**Tech stack:** Python standard library (3.9 compatible), Bash, pinned Rust 1.99.1, Bun.
**Spec:** Existing P1 completion plan and the historical preservation baselines; this document pins continuation behavior.

## Global Constraints

- Work on canonical main with one implementation writer; preserve unrelated work and recovery archives.
- Original baseline/catalog/source snapshots and independent goldens remain byte-identical.
- All C0 status and owner/crypto/platform/hardware qualification boundaries remain unchanged.
- No push, remote deletion, dependency installation, live-store access, or automatic backend substitution.
- Tests use synthetic temporary fixtures. Unexpected source drift fails; no warning-only bypass.

## Review Focus

- An unlisted source change must fail current preservation.
- Transition-manifest tampering or membership changes must fail.
- Historical language accessor exception remains exact and historical drift remains visible.
- Native/WASM comparisons and negative controls remain unchanged.
- Release live checks must keep failing for actual source/inventory drift.

### Task 1: Reviewed Studio preservation successor

**Ownership:** Studio scripts/tests, one additive dated transition manifest, and Studio verification documentation only. No substrate source, original baseline or execution-register edits.

- Write failing isolated Python tests for a shared preservation checker: valid reviewed current transition; historical refusal on changed source; every transitioned-file mutation; untouched-file mutation; malformed/duplicate/extra/missing transition entries; baseline digest tampering; exact language accessor handling.
- Implement the standard-library checker under specimen-studio/scripts with explicit language/scoring profiles and current/historical modes. Pin the reviewed transition manifest hash in checker code. Original baseline hash/membership is enforced. Each profile's changed path membership is exact: language Cargo.lock/persistence.rs; scoring AGENTS.md/core Cargo.toml/persistence.rs/check-language-parity.sh.
- Add one dated transition manifest containing baseline digests, prior/current file hashes, provenance commits and concise rationale. All unchanged original rows remain enforced. Language's exact accessor normalization stays unchanged.
- Wire both parity scripts to default current preservation and accept only the explicit --historical-preservation alternative (reject other arguments). Preserve all comparators and negative controls. Retain hashes/artifacts even if the final preservation check fails.
- Document that reviewed current identity is distinct from historical unchanged-byte claims; include ZIP9 stricter malformed-archive behavior.
- Run focused Python tests, shell syntax checks, GTK checks and archive tests. Commit only owned files and write the task report, including RED/GREEN evidence and exact commands.

### Task 2: Live release-source regression coverage and diagnostics

**Ownership:** tools/check_substrate_release.py, tools/tests/test_check_substrate_release.py, and release README only.

- Add isolated live-mode tests using a synthetic trusted checkout fixture: exact identities pass; changed/missing source refuses; snapshot-only portability remains; added declaration/source drift refuses; immutable pins and promotion failures stay enforced.
- Add optional --drift-report JSON diagnostic output listing each drifted path once, expected/actual SHA-256 and affected declaration IDs. Report generation must retain exit 1 for live drift and must never modify contracts or create an accepted successor implicitly.
- Test grouped reporting for 38 module rows sharing lib.rs, the v2 format row, malformed/missing source behavior and CLI nonzero status. Keep default errors and snapshot-only semantics compatible.
- Document that the two changed source files also introduce model declaration growth, requiring independently reviewed source/version migration rather than a two-hash blanket refresh.
- Run the full release checker unittest suite, default checker (expected refusal), and snapshot-only (expected pass). Commit only owned files and report RED/GREEN evidence.

### Task 3: Integration validation and report

- Fresh task review after each implementation; fresh whole-range review after both.
- Refresh only affected current execution-register references with an additive refresh receipt.
- Run both Studio parity scripts in current mode and retain historical-mode failure evidence; run affected Python suites and execution validators.
- Run root gate and Studio checks appropriate to changed scripts, record exact source identities/commands/log hashes, and preserve all failures.
- Commit the continuation report and ledger outcome on main. Report unresolved release migration, PyCA disagreement, unavailable HawkScan, and external acceptance without promotion.

## Execution outcome

Tasks 1–3 completed. Both full current parity gates pass; historical mode retains
real refusal. Release diagnostics group the unchanged 39 live failures into two
paths without accepting migration. Final root, Studio browser, 64 completion,
52 release and 11 preservation checks passed; GTK/archive checks passed.
Fresh whole-range review found no remaining actionable issues or deferred minors.
The source-version, crypto/owner, security-tooling and external qualification
gates remain open. See the execution report for exact commands and hashes:
[continuation receipt](../completion/execution/evidence/2026-10-26-preservation-continuation/README.md).
