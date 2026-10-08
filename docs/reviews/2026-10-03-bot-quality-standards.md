# Abbey Bot Task 5 — Standards review

**Verdict: Pass after scoped fix1 re-review.** Both original findings are resolved; no new defect found in the four-path fix. Reviewed the 13-path frozen `owned-review.diff` against baseline `e742375ff9b3d20b97df09c7fad10c1225ae957c`, then `fix1/fix-review.diff` and current source. Fix1 frozen manifest: `f6a56b1660e9a026780e222f2062bc5f0acdeab41eb765c2a98024b10cea94a0`.

Standards: bot `AGENTS.md`/`CLAUDE.md`, module contracts, `docs/spec/brain.md`, and `docs/spec/adaptivelearning.md`. No tests rerun and no bot files changed; this verdict does not establish terminal strict-gate or live acceptance.

## Findings

### 1. Unbounded expansion behind bounded input

- **Severity:** Important (P1)
- **File:line:** Original `abbey-bot/src/brain/quality_evaluation.rs:160`; downstream `src/grounding.rs:613`.
- **Description:** A 20,000-segment dotted source could expand approximately 40 KB into approximately 400 MB of prefix strings despite the input byte cap.
- **Suggestion:** Bound inputs before grounding and check corpus dimensions first.
- **Status:** Resolved in fix1. Current `src/brain/quality_evaluation.rs:190` bounds source/citation counts, metadata, source/claim bytes, and tokens. Both direct-case admission (`:159`) and corpus validation (`:222`, `:257`) precede grounding. `src/learning_quality_cli.rs:51` checks five-class dimensions before evaluation. Boundary regressions cover refusal rather than exhausting resources.

### 2. Regular-file validation races opening

- **Severity:** Important (P2)
- **File:line:** Original `abbey-bot/src/learning_quality_cli.rs:95`.
- **Description:** Separate pathname metadata/open calls allowed FIFO substitution to block opening indefinitely.
- **Suggestion:** Open nonblocking on Unix and inspect the same handle.
- **Status:** Resolved in fix1. Current `src/learning_quality_cli.rs:105` uses existing rustix `RDONLY|NONBLOCK|CLOEXEC`; `:123` opens once and `:124` validates that handle before reading. Non-Unix also validates the opened handle. Regression coverage includes substitution, production flags, regular-file reads, and actual CLI FIFO admission.

## Reviewed judgments

Early dispatch in `main.rs:276` and the separate evaluator justify this slice's 870-line main. Atomic nonfinite rejection in `dqn.rs:275`, strict types, cited-current-source filtering, seeded behavior, and content-free output remain acceptable. Test-only unwraps are assertions. Inspected fix1 `focused-final.log`: 19 passed, zero failed/ignored; no independent rerun. Synthetic-label/provider/live limits remain explicit.
