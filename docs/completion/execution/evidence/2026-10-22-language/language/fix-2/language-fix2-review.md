# Task8.language fix-2 scoped rereview

## Verdict

**Approved for this scoped fix: the remaining Important finding is ADDRESSED. No new Critical or Important finding identified in the supplied fix diff.**

## Spec compliance and quality

The change follows the parent's explicit API ruling: `Budget.work_steps` accepts 1..=1,000,000 with default/ceiling 1,000,000, and exhaustion uses the existing typed StepBudget refusal. The profile and implementation agree. The diff changes the pure v3 configuration and private accounting; it contains no legacy, save, or wire-schema change.

One `PreparationWork` is constructed after budget validation in `prepare.rs:28-44`. Its mutable reference reaches original-action Dictionary/morphology validation and both explicit-sense and sampled replacement paths (`prepare.rs:79,103-106,137,350-360,400-410,685-702`). It is not recreated per clause, token, or variant. Checked subtraction in `tables.rs:365-378` refuses exhausted work, and propagation returns an error without a Prepared result.

The accounting closes the reported multiplication. Before replacement validation/tokenization, `tables.rs:224-234` reserves replacement bytes. Before expanding or cloning each view token, `tables.rs:237-251` reserves source/lemma bytes, then charges expanded leaves before appending them. Before each position's two safety-kind matcher calls, `tables.rs:267-270` charges the cached worst-case cost. The immutable constructor computes that cost once, with checked aggregation, from two full row traversals plus twice the safety phrases' word-byte/overlap allowance (`tables.rs:177-199`). Charging a matcher pass therefore does not itself rescan the table. Larger vocabularies consume more of the same invocation-wide allowance even when no unsafe match is found. This is a conservative logical work bound, not a timing claim or a new bound for all unchanged preparation phases.

## Finding disposition

| Finding | Status | Evidence |
| --- | --- | --- |
| Important: shared rewrite safety validation multiplies unbudgeted whole-clause scans | **ADDRESSED** | Invocation-wide checked counter, precharged lexical/view work and cached matcher cost described above. `tests/language_v3_limits.rs:142-184` demonstrates one rewrite succeeds at work_steps=64 while repeated clauses return StepBudget; resetting the counter per rewrite would fail this regression. `tests/language_v3_limits.rs:186-206` checks zero and above-ceiling budgets return InvalidBudget. |

The previous five Important fixes and arithmetic-label Minor are outside the reopened finding. Their ADDRESSED disposition is retained from fix-1; the supplied fix-2 diff does not reverse them. No additional Critical/Important issue was found in the changed accounting or its call-site wiring.

## Deferred Minor / evidence qualification

- **Environment Minor retained:** pinned-toolchain rust-objcopy/libLLVM debug-stripping warnings remain disclosed in the implementation report. This review neither repairs nor removes that qualification.
- The fix-2 report states 31 focused tests, 111 complete kernel tests, all-target kernel Clippy with `-D warnings`, exact-owned rustfmt and diff checks passed. These are reported results, not tests rerun or independently inspected raw logs in this rereview. They do not establish the four full Studio gates, parity, held-out efficacy, hardware qualification, or consumer integration.
- Unchanged source and any observations outside this supplied fix diff remain deferred. No new finding is asserted against untouched code.

Source/test paths above are relative to `specimen-studio/native/specimen-kernel/src/language_v3/` and `specimen-studio/native/specimen-kernel/`, respectively.

## Review method

Reviewed the supplied fix-1 review, task brief, implementation report (including final fix-2 section), and exact fix-2 diff for both spec compliance and code quality. Read the diff once; the initial combined output truncated only report content, recovered with a bounded report-tail read. Used supplied hunk coordinates for current line references. No changed source was opened separately, no repository crawl or git diff was performed, no tests were rerun, no subagents were spawned, and no product source was edited. Only this requested report was written.
