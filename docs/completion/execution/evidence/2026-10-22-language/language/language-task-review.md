### Spec Compliance

- ❌ Issues found. The preparation/script surface implements the requested architecture, but conditional containment, substitution safety, required-frame resolution, provenance fidelity and aggregate output limits have Important defects below.
- ⚠️ Cannot verify from this task diff: unchanged legacy IDs/save fixtures, full Studio native/WASM parity, consumer integration and actual reported suite execution. The diff only registers the opt-in module in `specimen-studio/native/specimen-kernel/src/lib.rs:9`; the controller owns final Studio gates. No held-out efficacy or ABI ratification is implied by this review.

### Strengths

- Validated prepared results have private fields and immutable views, without Deserialize (`specimen-studio/native/specimen-kernel/src/language_v3/model.rs:256`, `:349`); exact decimal operands remain separate from f64 computation (`specimen-studio/native/specimen-kernel/src/language_v3/arithmetic.rs:4`, `:61`, `:146`).
- Registry content, identity and revision are bound to both compiled scripts and capability ledgers, and a whole-tree contact pass precedes evaluation (`specimen-studio/native/specimen-kernel/src/language_v3/script.rs:286`, `:316`, `:353`). Nested parent narrowing is exercised by an actual registered LookUp (`specimen-studio/native/specimen-kernel/tests/language_v3_scripts.rs:143`).
- Tape support executes the real bounded automaton rather than a placeholder; wrapping cells, byte output and typed failures are implemented (`specimen-studio/native/specimen-kernel/src/language_v3/script/tape.rs:38`, `:46`, `:55`).

### Issues

#### Critical (Must Fix)

- None identified.

#### Important (Should Fix)

1. **Sequence results bypass the aggregate output ceiling.** `specimen-studio/native/specimen-kernel/src/language_v3/script.rs:386`, `:458`, `:318`: literal nodes are never charged, Sequence is charged zero, and the final size check examines only Text. With the test registry, `Parent("hello",Tape("",""))` and `ScriptBudget { output_bytes: 1, ..Default::default() }` returns a successful Sequence containing five output text bytes. A nontext child therefore disables the literal-output protection added for root literals. Charge every emitted value recursively/consistently, including literal children of mixed sequences, and add a mixed-sequence boundary regression.

2. **A second `then` escapes the original conditional group.** `specimen-studio/native/specimen-kernel/src/language_v3/prepare.rs:215`, `:219`, `:76`: a Then without a pending If becomes Separate; preparation then clears the condition. `if rain then hello then calculate 2+2`, with no supplied condition, leaves the last action executable with ConditionalDisposition::None. The profile defines semicolons as independent-group boundaries and requires the IfThen to bind its entire group (`docs/completion/execution/language-v3-profile.md:36`). Reject stray/duplicate Then as Syntax, or otherwise preserve the enclosing condition until a real semicolon; test this exact escape.

3. **Lexical rewrites can introduce executable safety vocabulary without any guard.** `specimen-studio/native/specimen-kernel/src/language_v3/tables.rs:225`, `specimen-studio/native/specimen-kernel/src/language_v3/prepare.rs:342`, `:694`: safe_lexical omits supported negators `can't`, `dont` and `don’t`, and the custom safety check compares one replacement token against the whole registered phrase. An Equivalent `hello -> can't`, or `hello -> halt now` with a registered two-word Negation `halt now`, is accepted and yields an executable, uninhibited variant. Dictionary/morphology rewrites also run after safety recognition without the replacement safety check, so a Dictionary `hello -> halt` can introduce a configured `halt` inhibitor into an executable original. Validate every lexical rewrite against the same complete tokenized built-in/custom safety vocabulary, including multiword phrases, before returning a validated result. Add separate regressions for built-in contraction, custom multiword and dictionary/morphology introduction.

4. **Required frames ignore already resolved grammar slots.** `specimen-studio/native/specimen-kernel/src/language_v3/prepare.rs:540`, `:543`: subject/object searches only accept Role::Other after Grammar has already assigned roles. Configure Grammar `alice -> Subject`, `gift -> Object` and Frame `give { subject:true, object:true, numeric_object:false }`; preparing `alice give gift` reports both SPO slots resolved yet marks unresolved_frame=true and refuses execution. The two requested utilities conflict in ordinary combined use. Honor existing validated Subject/Object assignments before inferring missing Other slots, and derive the missing-frame guard from the final resolved slots.

5. **Multiword replacement source bytes no longer match their preserved source span.** `specimen-studio/native/specimen-kernel/src/language_v3/prepare.rs:683`, `:706`: replace reconstructs token.source by joining token strings with one space. For input `red\tfox` and a two-word equivalent `red fox -> vulpine`, the replacement span is the original `0..7`, but source() is `red fox` rather than the actual `red\tfox`; repeated spaces produce the same inconsistency. This violates the source-byte/provenance API promised to downstream text/scanner work. Slice the preserved original input using source_span, and assert replacement.source() equals that slice for tabs, repeated whitespace and Unicode.

#### Minor (Nice to Have)

- **Exact aggregate arithmetic exhaustion returns the wrong typed error.** `specimen-studio/native/specimen-kernel/src/language_v3/script.rs:409`, `specimen-studio/native/specimen-kernel/src/language_v3/arithmetic.rs:136`: `Parent(Math("1"),Math("1"))` with arithmetic_steps=1 spends the first step, then passes zero to evaluate_counted, which returns InvalidBudget. The caller supplied a valid budget; this is runtime StepBudget exhaustion. Check remaining steps before another math invocation, and test exact exhaustion rather than only the existing partially consumed case.
- **Reported test output is not pristine.** `.superpowers/sdd/2026-10-20-wdbx-full-completion/task-8-language-report.md:62` (Tests and RED/GREEN evidence) discloses rust-objcopy warnings about missing pinned libLLVM.dylib. This is explicitly reported as an existing toolchain problem, not hidden by the task; retain the warning qualification in final gate reporting. No environment repair is requested here.

### Assessment

**Task quality:** Needs fixes.

**Reasoning:** The immutable opt-in architecture and pure capability planning are well separated and broadly tested, but the counterexamples above violate preparation constraints and output budgets that downstream consumers are expected to trust. Existing tests mostly cover each utility/guard independently and miss these compositions.

**Review checks:** Read the frozen task diff once in sequential chunks, then extracted line references from its hunk metadata; no changed source files were separately read, no broad repository crawl or git diff was performed, and no product files were edited. No reported tests were rerun. Findings are direct control/data-flow counterexamples for focused RED/GREEN validation by the implementer.
