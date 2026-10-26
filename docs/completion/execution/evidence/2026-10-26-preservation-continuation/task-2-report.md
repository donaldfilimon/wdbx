# Task 2 report

Status: implemented on local main, commit `c2178a405582136e06a34ed9ec97cfed7c8151d9`.
Owned changes: `tools/check_substrate_release.py`, `tools/tests/test_check_substrate_release.py`, `docs/completion/substrate-release/README.md` only. Parent register/evidence/plan changes preserved. No frozen catalog, snapshot, baseline or source migration; no installs, push, branch or worktree.

## Behavior

`--drift-report` emits version-1 live stdout JSON with mode/valid/complete/errors/drifted_paths, retaining refusal status. Mutually exclusive with snapshot-only. Each path has expected/actual SHA-256, declaration/source IDs and read_error. Missing, symlink, oversized or OS read refusals use null actual hashes. Unique live reads are cached; conflicting expected hashes refuse before reads. Catalog pin mismatch refuses before trusting membership. Default drift diagnostics preserve their row-specific stderr errors.

## TDD and checks

- RED: `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/tests -p test_check_substrate_release.py` after initial live tests and before implementation: 48 tests, 1 failure and 10 errors (missing report API/flag and repeated live reads); log `/tmp/wdbx-task2-red.log`.
- GREEN final same command: 52 tests, OK (39 existing + 13 live controls), 4.608 seconds; log `/tmp/wdbx-task2-green.log`.
- Live fixture copies frozen directory, materializes live source snapshots, writes deterministic declaration bytes across 31 unique paths and changes only temporary declaration hashes. Only LiveRelease mocks temporary CATALOG_PIN; original pin tests stay unchanged.
- Controls: exact identities, added declaration bytes, two-path/38+1 grouping, changed/missing live sources, missing/directory/oversized/symlink/PermissionError reads, caching, pinned malformed/unpinned catalog refusal, conflicting hashes, default and JSON CLI refusal, JSON success with no prose, mutually exclusive flags, snapshot portability.
- `python3 tools/check_substrate_release.py`: exit 1 with 39 source identity drift errors; stdout empty. `/tmp/wdbx-task2-default.err`.
- `python3 tools/check_substrate_release.py --drift-report`: exit 1, valid=false, complete=true, 39 errors, exactly two drift paths: lib.rs with 38 module IDs; v2.rs with one format ID. `/tmp/wdbx-task2-report.json`.
- `python3 tools/check_substrate_release.py --snapshot-only`: exit 0; frozen C0 contract passes.
- `git diff --check`: exit 0.

## Limits and concerns

Current model declaration/read_v2_root growth still requires independently reviewed source/version and declaration-inventory migration. No release acceptance, source refresh or claim promotion is approved by this change. Tests use standard-library Python and Python 3.9-compatible syntax; executed with host python3, no separate Python 3.9 runtime qualification.

Post-commit HawkScan requested by hook: unavailable. No hawk/hawkscan binary or tool discovered, HAWK_API_KEY is unset. No installation attempted. Root Rust/Studio gates belong to controller integration; this task did not run them.

## Review fix round 1: preserve stderr in JSON mode

Reviewer identified that report mode returned before the normal FAIL stderr loop.
Changed the CLI regression first to require FAIL stderr in JSON refusal mode.
RED full suite: 52 tests, exactly one failure (`test_cli_json_and_default_refuse`),
15.398s; `/tmp/wdbx-task2-fix1-red.log` retained. Minimal fix emits the existing
FAIL loop before JSON output/return, preserving parseable stdout and nonzero status.
GREEN same full command: 52 tests OK, 12.347s;
`/tmp/wdbx-task2-fix1-green.log`. `git diff --check` passed.
Commit `97af6e53fffb1a460fbfd52a93fbda007af58d4f` owns only checker and regression.
No other behavior or source migration changed. HawkScan remains unavailable as
recorded above; no install attempted.
