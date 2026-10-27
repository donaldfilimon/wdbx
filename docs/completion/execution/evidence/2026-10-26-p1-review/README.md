# P1 review and local consolidation — 2026-10-26

Source implementation: `3f79bbe`, following ledger-test commit `1e7f9e2`.
The original tree was clean at `4c01514`; the requested P1 work had already
landed in `550451b`. No uncommitted P1 implementation was overwritten.

## Findings and fixes

- No functional defect was found in the inspected private P1 parser, strict
  verifier, original-byte bindings or explicit Python backend error boundary.
  Their bounded paired witness ran separately from default gates.
- Authenticated-ledger coverage now includes unsigned history without a torn
  tail, valid-prefix tail repair, malformed complete-tail byte preservation,
  and a second signed append after rotation followed by a two-key reopen.
  Existing stripping, tamper, unknown-key and current-writer tests remain.
- Scoring parity used a user-specific library path and fixed Apple ARM host.
  It now derives the sysroot, host and objcopy suffix from selected Rust,
  retains an existing loader search path, and limits Darwin library evidence
  to Darwin. Two regressions failed before their corresponding fix and passed
  afterward; the actual scoring build and comparison ran with the new loader.
- Fresh read-only review accepted the final diff without actionable findings.
  Stubbed host-path tests do not qualify execution on other operating systems.

## Actual validation

Exact commands, interpreter identities, exit codes, counts, source and raw-log
SHA-256 values are in [validation.json](validation.json). Logs are retained in
`.work/p1-completion/`; they include failed attempts.

| Run | Result |
| --- | --- |
| Final `bash tools/check.sh` after consolidation | Exit 0; 1,201 Rust tests/doctests; Python suites 52 + 28 + 14 |
| `cargo test --locked -p abi-wdbx` | Exit 0; 932 Rust tests/doctests |
| Authenticated-ledger integration test, final | Exit 0; 7 tests |
| Completion and execution validators | Exit 0; 163 open C0 requirements |
| Completion-validator mutation/unit suites, final | Exit 0; 64 tests |
| Scoring-loader regression suite | Exit 0; 2 tests |
| Explicit libsodium P1 gate | Exit 0; 57 Rust + 12 Python tests |
| Explicit PyCA P1 gate | 57 Rust + 12 Python pass; exit 1 at retained weak-key probe |
| Studio `bun run check` | Exit 0; 221 Bun tests; WASM and browser production build |
| Studio `bun run test:native` | Exit 0; 343 Rust tests/doctests, 1 pre-existing ignored |
| Studio `bun run check:native` | Exit 0; GTK checks and native Clippy |
| Identity parity script | Exit 0 |
| Language/scoring parity scripts | Exit 1 after successful comparison/self-tests/negative controls, at historical preservation checks |
| Source custody | Exit 0; AUDITED-SOURCE-02 only |
| Substrate release inventory | Exit 1; 39 source-identity drifts |
| HawkScan | Not run: executable missing and API key absent |

The initial validator-suite attempt failed while final source edits were still
drifting its references; `validators-final.log` supersedes it. The reviewed
[reference refresh](reference-refresh.json) updates only current execution
references, including pre-existing `v2.rs` drift. Frozen contracts, historical
snapshots, receipts and C0 claim levels were not changed.

## Consolidation and recovery

All 24 other local branch tips were already reachable from main; no synthetic
merge was needed. All 15 additional worktrees were archived and every regular
file and symlink compared with its archive. A full-ref Git bundle also verified.
All eight dirty tracked worktrees matched main byte-for-byte before their edits
were additionally preserved as stashes. Ignored content was relocated intact.

[Initial inventory](initial-inventory.json) and
[preservation receipts](preservation.json) bind the original identities, archive
hashes and stashes. Recovery archives, ignored files and bundle remain under
`.work/p1-completion/recovery/` (approximately 10.6 GB of archives, plus relocated
ignored files). Do not clean that directory. These are same-disk recovery copies.

Only `main` and the canonical registered worktree remain. Nine relative aliases
preserve existing adjacent ABI/Abbey consumer paths; they are not Git worktrees.
No reset, forced deletion, push, remote deletion or deployment was performed.

## Remaining blockers

- PyCA accepts the retained synthetic weak-key vector; its qualification probe
  intentionally remains failed. No automatic backend substitution was introduced.
- Historical language preservation differs at Studio `Cargo.lock` and native
  persistence; scoring preservation differs at Studio instructions, core
  manifest and native persistence. Their last changes predate this task.
  Those preservation gates remain failed; no historical baseline was repinned.
- Substrate release source pins drift at MOD-0032 through MOD-0069 and
  FORMAT-v2-journals. These broader release records need reviewed reconciliation.
- HawkScan prerequisites are unavailable. No DAST or security qualification is claimed.
- ABI/schema-owner acceptance, independent cryptographic review, durable governed
  admission, cross-platform/hardware execution and elapsed production/soak gates
  remain independent work. Local consolidation does not finish those obligations.
