### Spec Compliance

- ❌ Issues found: one Important scoring defect in affected-scope negation handling (`specimen-studio/native/specimen-kernel/src/scanner_v3/mod.rs:136-142,182`). Conditional antecedent polarity is incorrectly used to decide conflict for body-local SPO.
- ✅ The owned diff otherwise implements the selected text-v3 framing/classes, immutable full-feature scanning, checked rational reduction, explicit typed routes, complete-or-error query results, concrete-reference joins and retained origin/constraint records. Evidence: `src/text_v3/mod.rs:53-151,292-327`; `src/scanner_v3/mod.rs:157-245`; `src/scanner_v3/rational.rs:93-101`; `src/scanner_v3/routes/mod.rs:51-156,277-411` (source paths below `specimen-studio/native/specimen-kernel/`). Only the two module registrations change `src/lib.rs:15,17`.
- ⚠️ Cannot verify from this diff: the broader frozen contract bundles, unchanged producer correctness, actual legacy byte preservation, native/WASM parity, production script/import registration, runtime/save integration, efficacy or global scheduling. The report explicitly leaves integration/efficacy/global budgets open; the known ordinary-number-word producer Syntax gap is disclosed and belongs to the separately serialized producer fix, not an undisclosed defect in this owned diff. Controller must rerun integration validation after that producer changes.

### Strengths

- `specimen-studio/native/specimen-kernel/src/scanner_v3/mod.rs:157-245`: scanner inputs are immutable prepared actions, with no key/class route into scoring; the four independently inspectable components preserve exact fractions and constraints.
- `specimen-studio/native/specimen-kernel/src/scanner_v3/rational.rs:25-57,93-101`: checked signed arithmetic retains unclamped S/D and subtracts before a single clamp. Direct large-value, cancellation and fractional tests exist at `tests/scanner_v3.rs:41-83`.
- `specimen-studio/native/specimen-kernel/src/scanner_v3/routes/mod.rs:51-156,277-411`: complete batch validation precedes publication; queries reject stale refs/registry and return only complete results. Candidate joins preserve distinct origins rather than collapsing captures, exercised by `tests/scanner_v3_routes.rs:344-377,596-704`.
- `specimen-studio/native/specimen-kernel/tests/text_v3.rs:44-72`: exact 6/7/16/17 boundaries and full SHA256 preimage framing are directly checked. Numeric coarsening versus full-original evidence is exercised by `tests/scanner_v3.rs:101-118` and `tests/scanner_v3_routes.rs:115-146`.

### Issues

#### Critical (Must Fix)

- None identified in the owned diff.

#### Important (Should Fix)

- **[P2] Use affected body scope for the SPO negation conflict.** `specimen-studio/native/specimen-kernel/src/scanner_v3/mod.rs:136-142,175-182` reduces the entire action tree to “any negation exists,” traversing both children of `IfThen`, while the compared SPO slots describe only its body. With resolved `alice/sees/bird` roles, compare the body actions of `if not rain then alice sees bird` and `if not rain then not alice sees bird`: both trees contain a negation, so the current implementation awards 20S/0D for identical SPO despite opposite body polarity. Conversely, `if rain then alice sees bird` versus `if not rain then alice sees bird` spuriously receives 0S/20D for identical positive body roles. Each error changes the final S−D by 40. The profile explicitly keeps antecedent lexical evidence separate and invents no antecedent roles/truth; antecedent negation must therefore not determine the body-role conflict. Inspect explicit negation affecting the body subtree (and preserve uncertain scope as uncertainty), then add regressions for both counterexamples. Existing simple negation and positive-condition tests at `tests/scanner_v3.rs:140-229` do not cover this intersection.

#### Minor (Nice to Have)

- `docs/completion/execution/evidence/2026-10-22-language/text-scan/green-focused.log:2-25` and `kernel-suite.log:2-105`: the recorded tests pass but contain repeated rust-objcopy SIGABRT stripping warnings. The report accurately discloses these as the existing missing-libLLVM toolchain issue; they are environmental noise rather than evidence of a scanner failure. Track/repair the pinned stripping environment separately so future evidence is clean.
- `specimen-studio/native/specimen-kernel/src/text_v3/mod.rs:313`: the comment says punctuation is retained by “high/medium ASTs,” but `render_ast` filters punctuation in medium and the selected profile/tests intentionally ignore it there. Say “high AST” to keep the comment consistent with behavior.

### Assessment

**Task quality:** Needs fixes.

**Reasoning:** The implementation is decomposed, bounded at the advertised local interfaces, and substantially tested, but the conditional negation/SPO intersection produces incorrect score evidence and blocks approval. Fix that scope calculation and add focused RED/GREEN regressions before approving this task.

**Checks performed:** Read the supplied diff once in sequential ranges; inspected the selected profile, brief and report. For the named concrete-reference deduplication risk, checked unchanged `identity/mod.rs:11-14,31-79,113-125`: concrete refs are globally unique and `find_all` is population-fenced, so keying the join map by EntryRef is sound. For the named negation-scope risk, checked unchanged `language_v3/prepare.rs:95-98,493-510` and `language_v3/model.rs:259-305`: producer wraps retained antecedent and body in `IfThen`, while body tokens/SPO/inhibition remain separate. No broader crawl, holdout access, product edits, git commands or test reruns. Inspected existing raw logs: `green-focused.log:38,58,72` records 7+14+8 passing focused tests; `kernel-suite.log:140-323` records 140 passing kernel tests; `clippy.log:3` records completion without compiler warnings. Behavioral RED logs include `red-punctuation.log:53`, `red-route-work.log:49` and `red-antecedent.log:44`. These recorded runs do not cover the newly identified conditional negation case.
