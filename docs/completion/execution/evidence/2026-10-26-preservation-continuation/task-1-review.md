# Task 1 independent review

Reviewed package: `ad92e81..a2e2f26`; final implementation `a2e2f26887ccdcf6cce493f3a507ed532193fe9d`.

## Verdicts

- **Specification compliance: PASS for implementation; completion conditional on the pending portable archive test passing.** The approved fourth scoring transition is included (five physical transitioned files total).
- **Code quality and safety: PASS.** No actionable defects found in the reviewed change. No tests were rerun during this review.

## Review evidence

The six-file commit scope matches the owned checker/tests, two parity scripts, additive manifest and successor document. The original baseline/catalog/source snapshot/golden files and substrate implementation are absent from the commit diff. Current working-tree status shows only the controller-owned untracked continuation plan.

The checker pins both complete original baseline byte streams, preserving membership and original hashes, and separately pins the entire transition manifest. It validates every transition profile, exact changed-path sets, duplicate paths and JSON keys, prior digests, hash shapes, provenance and rationale. Every baseline row remains enforced; changed or missing sources fail with nonzero exit. Historical mode retains the original hashes. Only language normalizes the original exact accessor bytes, requiring precisely one occurrence; scoring hashes those bytes without normalization. Unselected profiles are schema/pin-validated without substituting their transition hashes into the selected profile.

The synthetic tests exercise current acceptance, historical refusal and historical exact acceptance, every transitioned file's mutation/removal, untouched-source mutation, malformed/duplicate/extra/missing entries, altered prior/current digests, baseline/manifest digest tampering, and accessor deletion/duplication/modification. Their fixture pins are deliberately replaced only in isolated temporary roots; production pins remain literal constants.

Both shell changes retain native/WASM comparators and load-bearing negative controls unchanged, reject all alternative arguments except the single historical flag, and write artifact hashes before the final preservation check. Source receipts include checker, tests, transition and both original baselines. The additive successor document separates reviewed current identity from unchanged historical identity and explicitly documents stricter ZIP9 malformed-archive refusal. No C0/owner/crypto/platform qualification boundary is promoted.

## Cannot verify / completion gates

- The portable archive run remains pending: the reviewed report marks it pending and the inspected `.work/preservation-task1/archive-tests.log` ends during compilation, with no final test result. This is a completion gate, not an implementation finding. Controller must obtain exit 0 and the test summary before marking Task 1 complete.
- The report's Python/GTK/syntax/checker-mode results were reviewed as reported evidence, not independently rerun, per review instructions.
- Full parity execution is controller-owned after review; no native/WASM runtime success is inferred from the script diff.
- Existing baseline-covered dependency/persistence/instruction changes are the approved input identities; this review verifies binding and scope, not a fresh independent audit of those earlier commits or platform/production behavior.

## Focused remaining checks

1. Finish the already running isolated `specimen-core --test portable_archive` command and retain its exit status/log; update the task report from pending with the actual result.
2. Execute both default current parity scripts under the pinned toolchain; retain comparator outputs, negative-control failures and artifact receipts. Historical refusal remains expected for the reviewed successor tree.
3. Preserve the unchanged original evidence assets and controller-owned untracked plan through final integration/status review.
