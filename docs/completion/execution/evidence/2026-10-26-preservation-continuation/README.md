# WDBX preservation continuation — 2026-10-26

Implementation commits: `a2e2f26` (strict Studio preservation successor),
`c2178a4` (live release diagnostics/tests), `97af6e5` (stderr regression fix).
Baseline: `ad92e81`. All work remains on local main; no push or deployment.

## Result

Both complete language and scoring parity scripts now exit **0**, including
actual native/WASM comparisons, existing semantic checks and corruption negative
controls. Current preservation permits only independently reviewed transitions
for five physical files. Their prior/current hashes, exact membership and
provenance are pinned in the additive
[transition](../2026-10-26-studio-preservation-transition.json).
All other original rows remain enforced. The original baselines were not edited.

`--historical-preservation` remains available on each script; the shared checker's
historical mode was executed and still refuses actual historical drift (language
two paths; scoring four, including the newly wired language script). Historical
refusal is not converted into a passing unchanged-byte claim. The full historical
scripts were not rebuilt a second time: current full execution and direct
historical-checker refusal are separate receipts.

The release checker now has isolated live-mode coverage and optional
`--drift-report` stdout JSON. Default and JSON live checks still exit **1**.
All 39 declaration diagnostics group into two actual changed files:

- `crates/abi-wdbx/src/lib.rs`: 38 module declarations share its full-file hash;
  later changes added the model module and read-root reexport.
- `crates/abi-wdbx/src/v2.rs`: the journal format row shares the file identity
  changed by the typed CLI read-root helper.

The [JSON report](release-drift.json) does not accept either source change.
New model declarations require an independently reviewed inventory/version
migration; replacing two hashes alone would miss declaration growth.
`--snapshot-only` continues to validate the frozen contract separately.

## Verification

Exact commands, exit codes, source and raw-log SHA-256 values, tool versions and
five unchanged frozen-artifact hashes are in [validation.json](validation.json).

| Executed check | Outcome |
| --- | --- |
| `bash tools/check.sh` | Exit 0; 1,201 Rust tests/doctests, Python witnesses 52 + 28 + 14 |
| Studio `bun run check` | Exit 0; 221 Bun tests, type/lint/WASM/production build |
| Full language and scoring parity scripts | Both exit 0; original comparators and negative controls retained |
| Preservation tests, Python 3.9 and default | 11 pass on each interpreter |
| Release checker tests | 52 pass; 39 prior + 13 live controls; final Python 3.9 run passes |
| Completion-validator tests | 64 pass |
| Completion and current execution validators | Both exit 0; 163 requirements remain open/C0 |
| GTK provenance/source/archive/locked graph checks | Pass; GTK Python 12 pass and diagnostics 1 pass |
| Portable archive runtime suite | 12 pass |
| Frozen release snapshot | Exit 0 |
| Live release and grouped diagnostics | Exit 1; 39 stderr rows, two grouped paths |
| Historical preservation checkers | Both exit 1 against their actual old identities |

The complete native Studio suite and P1 crypto backends were not rerun in this
checker-only tranche; prior execution remains in the P1 review receipt. No Rust
runtime implementation or crypto backend changed here.

Both task reviews and the fresh whole-range review passed. Review found and
fixed one CLI gap: JSON mode initially omitted stderr diagnostics. Its regression
failed before the fix; all 52 tests then passed, and scoped re-review passed.
The final reviewer independently verified all 38 current reference replacements
against their receipt, live hashes and implementation-commit hashes.

Raw logs and both native/WASM artifact sets, including executable probes, are
retained under `.work/preservation-continuation/`.
[Artifact identities](parity-artifacts.json), task reports/reviews and the
[execution ledger](progress.md) preserve failures, decisions and review scope.
Original consolidation recovery archives and stashes remain untouched.

## Remaining gates

- Live release source/declaration-inventory version migration remains blocked.
- PyCA's retained weak-key disagreement, independent cryptographic review and
  named-owner ABI/durable-admission acceptance remain open.
- HawkScan executable and API key are unavailable; no DAST scan or clearance.
- Cross-platform/hardware execution and elapsed operational qualification remain
  separate. Passing current preservation is no release or production claim.

Only main and the canonical registered worktree remain; no new topic branches
or worktrees were created.
