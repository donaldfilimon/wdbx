# Task 8 language numeric integration followup

2026-10-22. Ordinary number-word-only clauses now remain lexical unless the existing explicit calculation command supplies arithmetic intent. Source is stable for independent review. This follows the parent-approved narrow predicate and does not introduce grammar or change the public immutable API, budgets, frames, legacy behavior, saves, commands, or frozen profile.

## Cause and exact recognition predicate

The existing action preparation selected its first unquoted numeric literal, zero..ten word, or `-`, `+`, `(` token as a candidate tail. A tail starting at token zero was unconditionally sent to the arithmetic normalizer. Because every zero..ten word is in the normalizer's allowed vocabulary, ordinary adjacent number words were converted to numeric operands and then refused by the Pratt parser as Syntax. A single word incorrectly became a numeric capture/value.

The candidate-start selection is unchanged. Let `start` be that first candidate token. Normalization is now eligible exactly when either (a) `start == 0` and the tail contains an unquoted token whose existing kind is Number or Operator, or whose lemma is one of the existing operator words `plus`, `minus`, `times`, `divided`, `modulo`, `power`; or (b) an unquoted `calculate`, `compute`, or `evaluate` marker occurs before `start`. Thus a number-word-only tail without a command has no arithmetic intent; `two plus two` and `two + two` retain it; `calculate two` retains the existing explicit-command behavior. This is a bounded recognition rule, not a universal inference of mathematical intent.

The normalizer still checks its entire candidate tail against the existing quote-free numeric/operator vocabulary, then uses the unchanged bounded Pratt parser. Closed-vocabulary malformed operator expressions still return their existing typed errors; this change adds no syntax-error-to-text fallback. Existing mixed-prose behavior remains unchanged: a candidate tail containing non-arithmetic lexical content or quotes is not normalized. A new grammar for prose expressions, leading operator words, or number composition is outside this followup. The direct `evaluate_math` API already expresses intent and is unchanged. Explicit numeric-frame slot capture is independent and unchanged.

## Owned changes

Only these three source/test paths changed during this followup:

- `specimen-studio/native/specimen-kernel/src/language_v3/arithmetic.rs`: seven-line internal `has_explicit_syntax` predicate reusing the existing operator-word vocabulary.
- `specimen-studio/native/specimen-kernel/src/language_v3/prepare.rs`: narrow guard on the existing leading-tail branch plus a clarifying comment.
- `specimen-studio/native/specimen-kernel/tests/language_v3_numeric.rs`: six synthetic behavioral regressions using the immutable preparation API and nine supplied utility snapshots.

All three stay below 1000 lines (221, 753, 189 respectively). The two pre-change producer files are retained under `before/`; `numeric-followup.diff` compares them to final source and includes the new test. Exact final source hashes are in `source-manifest.json`. No edits were made before the coordination interruption. Other dirty work was preserved. No subagents, staging, commits, installs, network access, holdout access, or manifests/locks/frozen-contract edits were performed.

## Behavioral RED/GREEN and verification

`red.log` is a real behavioral RED before the producer fix: 3 passed, 3 failed. The exact seven number words returned Syntax; the single/adjacent test first caught `zero` becoming Number; the combined quoted/negated/ordinary fixture returned Syntax on its ordinary final clause. Existing word arithmetic, frame capture, and malformed explicit arithmetic assertions passed in that same run.

`green-numeric.log`: all 6 tests pass. They verify exact `one two three four five six seven` lexical lemmas, no decimal evidence/captures/operators/arithmetic value, and no arithmetic route; single/adjacent number words without a frame; word/symbol expressions computing 4 with ordered source/exact operand evidence and operator sequence; explicit numeric frames and quoted refusal; typed malformed closed arithmetic failures including trailing/misordered operands/operators, missing parenthesis and zero division; direct-math/explicit-command single operands; opaque Unicode quotes, scoped negation/inhibition, UTF-8 byte spans, and unchanged original source bytes.

Final checks on stable source:

- `green-focused.log`: 37 passed, 0 failed (20 preparation, 3 limits, 8 scripts, 6 new numeric).
- `kernel-suite.log`: 146 passed, 0 failed across the full current specimen-kernel suite, including the frozen text/scanner modules present in this checkout; 0 doctests.
- `clippy.log`: `cargo clippy -p specimen-kernel --locked --all-targets -- -D warnings`, exit 0.
- `fmt-check.log`: exact three-owned-path rustfmt check, exit 0.
- `diff-check-final.log`: git whitespace checks have no diagnostics, including no-index before/final comparisons and the new untracked test. The initial accounting wrapper misread git no-index's ordinary differences exit 1 as an error and asserted; raw exit/output records are retained in `diff-check.log`, then correctly interpreted without source changes.

All Rust commands used pinned 1.99.1 with the required toolchain-first PATH. Exact commands, working directory and outcomes are in `commands.txt`. Raw logs preserve the existing rust-objcopy missing-libLLVM.dylib stripping warning; tests compiled and ran, and nothing was installed or changed to conceal it. Source/log hashes and the report copy are retained beside these logs.

These are producer/kernel checks. The parent owns independent review, full Studio gates and native/WASM parity. This report makes no claim of full Studio qualification, scientific efficacy, or frozen C0 promotion.

Durable evidence: `docs/completion/execution/evidence/2026-10-22-language/language/numeric-followup/`. Working report: `.superpowers/sdd/2026-10-20-wdbx-full-completion/task-8-language-numeric-report.md`.
