# Task8.language fix-1 scoped rereview

## Verdict

**Needs fixes: all five original Important findings and the arithmetic-label Minor are ADDRESSED; one NEW Important bounded-work regression is introduced by this fix diff. No new Critical finding identified.**

## Spec compliance and quality

The fixes preserve the immutable prepared-result/pure-plan architecture visible in the reviewed diff. Conditional refusal, the common rewrite safety vocabulary, resolved frame roles, exact original source slicing and recursive output sizing directly repair the reported counterexamples. The regressions cover each original case, with additional cross-boundary safety phrases and exact UTF-8 output boundaries. The existing limits test is moved without assertion changes.

The new rewrite validator does not yet preserve the intended bounded-work discipline: it adds repeated whole-clause/table scans without a checked shared counter. This prevents approval despite the original correctness findings being repaired. No scientific efficacy, ABI qualification, full Studio parity, legacy save compatibility or consumer integration is established by this scoped review. The diff adds no legacy save/identity/runtime edits; unchanged behavior is not independently revalidated here.

## Original findings

| Finding | Status | Reviewed evidence (new-working-tree lines) |
| --- | --- | --- |
| Important 1: mixed Sequence output bypass | **ADDRESSED** | `script.rs:373,385,462-474` checks every produced value and recursively sums Sequence payloads with checked addition; literals and nested mixed output cannot erase payload size. Regression `tests/language_v3_scripts.rs:322` covers refusal and exact boundaries, including UTF-8 plus bytes. |
| Important 2: duplicate Then escapes conditional | **ADDRESSED** | `prepare.rs:219` returns Syntax when Then has no pending If. `tests/language_v3.rs:750` covers the exact duplicate-Then escape and a standalone stray Then. |
| Important 3: rewrites introduce unguarded safety vocabulary | **ADDRESSED** | `tables.rs:193-236,315-320` validates complete tokenized replacement/context vocabulary, including phrases crossing changed boundaries; the shared negator predicate includes the contractions. Dictionary, morphology and lexical alternatives invoke it at `prepare.rs:356,406,695`. Separate regressions start at `tests/language_v3.rs:797,816,843,866,888`. The new work regression below is distinct from this repaired safety check. |
| Important 4: required frames ignore Grammar roles | **ADDRESSED** | `prepare.rs:539,547` uses existing Subject/Object roles before Other-token fallback. `tests/language_v3.rs:915` verifies the combined Grammar/frame example resolves and executes. |
| Important 5: replacement source disagrees with source span | **ADDRESSED** | `prepare.rs:698-702` obtains source directly from the preserved original action slice. `tests/language_v3.rs:956` asserts slice equality for tabs, repeated spaces and multibyte Unicode. |
| Minor: exact arithmetic exhaustion mislabeled InvalidBudget | **ADDRESSED** | `script.rs:405` refuses zero remaining arithmetic work as StepBudget before Math/Bind evaluation. `tests/language_v3_scripts.rs:382` reproduces the exact two-call boundary. |

Source paths above are relative to `specimen-studio/native/specimen-kernel/src/language_v3/`, except explicit `tests/` paths, which are relative to the kernel directory.

## New findings in the fix diff

### Important: shared rewrite safety validation multiplies unbudgeted whole-clause scans

**Location:** `specimen-studio/native/specimen-kernel/src/language_v3/tables.rs:206-233`, especially `:226-228`, called inside the per-token Dictionary and morphology paths at `prepare.rs:356,406`.

Every call rebuilds the complete clause lexical view and invokes the table matcher for both safety kinds at every token position. Dictionary and morphology now call it for each changed token, but the validator accepts no shared work counter and never charges these token/row comparisons. A bounded accepted input containing about 4,000 one-character words can repeatedly match one Dictionary rule; with about 4,095 custom safety rows, the new safety pass alone performs on the order of 4,000 rewrites × 4,000 positions × 4,095 safety rows, roughly **65 billion candidate comparisons**, before ordinary fanout checks. The guard rejects no unsafe phrase in this case, so those limits do not make it fail promptly. This is a static complexity counterexample, not a measured timing claim.

Compile/index the immutable safety vocabulary once and check only matches that can overlap the changed lexical range, or pass one **shared checked preparation work budget** through every validation call and charge view expansion plus safety matching before doing the work. Exhaustion should return the existing typed budget refusal and expose no Prepared result. Add a deterministic repeated-rewrite regression using a small supplied budget; it should demonstrate bounded refusal without a wall-clock assertion. A fresh per-rewrite counter would still allow the multiplicative total and would not resolve this finding.

## Deferred Minor / evidence qualification

- **Environment Minor retained:** the implementation report discloses pinned-toolchain rust-objcopy/libLLVM debug-stripping warnings. Keep that qualification in final gate reporting. This review neither repairs nor treats the warnings as hidden failures.
- The fix-1 report states 29 focused tests, 109 kernel tests, all-target Clippy with `-D warnings`, exact-owned rustfmt and diff checks passed. These are **reported results**, not rerun or independently inspected raw-log evidence in this rereview. They are kernel checks, not the controller's four Studio gates or held-out evaluation.
- Unchanged compatibility/integration surfaces and any observations outside this fix diff remain deferred; no new Important claim is made about untouched code.

## Review method

Read the supplied prior review, task brief, implementation report/final fix section and exact old-reviewed-snapshot-to-working-tree fix diff. A tool-output truncation was recovered through bounded diff chunks; hunk metadata supplied current line references. No changed source was read separately, no repository crawl or git diff was performed, no tests were rerun, no subagents were spawned, and no product source was edited. Only this requested review report was written.
