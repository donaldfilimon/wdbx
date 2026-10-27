# Task 2 review

Reviewed package: `a2e2f26..c2178a4`; static review of the supplied diff, task brief/report, and bounded surrounding checker code. No source edits, subagents, or test reruns.

Spec verdict: CHANGES REQUIRED.
Quality verdict: CHANGES REQUIRED (one P2 issue).

## Finding

- **P2 — Keep validation failures on stderr in JSON mode.** `tools/check_substrate_release.py:220-225` prints the report and returns before the existing `FAIL:` stderr loop. Thus `--drift-report` with invalid contracts or source drift emits no normal validation errors on stderr, contrary to the review requirement that JSON use stdout while normal validation errors remain on stderr and exit status stays 1. `tools/tests/test_check_substrate_release.py:338` explicitly asserts empty stderr and entrenches the incompatible behavior. Emit JSON on stdout while also retaining the normal validation error loop on stderr, and update the CLI regression assertion. Preserve absence of the success prose in report mode.

## Satisfied by scoped static inspection

- Catalog pin is checked before parsing membership and collecting live diagnostics; unpinned or parse/shape-failing catalogs return `complete=false` and refuse.
- Existing bounded regular-file reader and symlink/path protections are reused. Missing, directory, oversized, symlink-refused and OS read failures yield null actual hashes and explicit read errors.
- Live identities group declaration/source IDs by path and cache one live read per unique catalog path; conflicting expected hashes refuse before live reads.
- Snapshot-only and report CLI flags are mutually exclusive; report mode retains nonzero validation failure status.
- Added tests use isolated synthetic checkout fixtures and scoped temporary catalog pin mocking, with live drift, missing reads, grouped 38+1 declarations, cache and refusal controls.
- Supplied diff changes only checker, its test suite and release README; pin constants, frozen inputs and source migration are not changed in that diff. Documentation retains independent migration, C0 and evidence boundaries.

## Cannot verify in this review

- Reported 52-test pass, RED failure, checker invocations and timings were not rerun or independently reproduced, per review scope.
- Full-tree byte identity, unrelated checkout changes, source snapshot/golden preservation outside the supplied diff, and actual commit ownership are not established by this scoped static review.
- Rust/Studio, optional crypto, platform/hardware, live-store or production qualification are outside this task; no such qualification follows from this review. Reported HawkScan unavailability was not independently checked.
