# Spec Compliance

- ✅ Spec compliant for Task8.language-parity. The exact frozen review patch adds only the separately gated fixture/example/test, feature registration, and explicit parity script/comparator. The default export remains excluded by cfg (`specimen-studio/native/specimen-wasm/src/lib.rs:131`); the fixture calls real utility, script, HybridTable and route APIs (`src/language_probe.rs:10,266,320,409,432`, under that same crate).
- ✅ The native tests assert mechanisms rather than only trace equality (`specimen-studio/native/specimen-wasm/tests/language_probe.rs:4,32`). Independent JS reconstructs transform/LP/SHA256 bytes and BigInt fractions, checks utilities/source spans/guards, conditional body polarity, shared-key originals and distinct joined route origins (`specimen-studio/tests/language-parity.mjs:104,106,195,264,322,420,459,501`).
- ✅ Actual feature WASM instantiation uses throwing host imports and releases the returned buffer in finally (`specimen-studio/tests/language-parity.mjs:541,551,576`). The same comparator rejects a digest mutation, with a separate nonzero-exit process required by the script (`specimen-studio/tests/language-parity.mjs:511,585`; `specimen-studio/scripts/check-language-parity.sh:46`).
- ✅ Retained execution evidence agrees with the bounded claims: compiled/export-inspected default, executed feature, zero host calls, identical 133421-byte native/WASM traces, negative exit 1, and 23 preserved baseline hashes (`docs/completion/execution/evidence/2026-10-22-language/parity/runtime-final.log:154-197`). Native suite records 13 library plus 2 new integration passes (`native-suite.log:60,74` in that evidence directory); feature all-target Clippy completes (`clippy.log:2`; exact invocation and exit in `checks.json:20-40`).
- ⚠️ Cannot verify from this diff: producer correctness beyond this synthetic witness, preservation of every file outside the 23-file baseline, the parent's final four Studio gates, or research efficacy/real effect hosting/authentication/migration/global scheduling/target qualification. These are correctly left outside the report's parity claim. The supplied diff is the review boundary; prior unrelated work was excluded by the controller.

# Strengths

- Real shared fixture execution and independent semantic checking give complementary evidence; native/WASM byte agreement alone is not used as the semantic oracle (`specimen-studio/tests/language-parity.mjs:104-109,195-262,578-585`).
- Concrete entries are retrieved from an actual shared-low-key HybridTable and then scanned using their full originals; explicit grammar/wildcard/Tape routes retain separate origins and blocked execution remains visible (`specimen-studio/native/specimen-wasm/src/language_probe.rs:320-430`; `specimen-studio/tests/language-parity.mjs:423-505`).
- Evidence distinguishes compilation from execution and retains failed intermediate runs and the successful corruption refusal (`task-8-language-parity-report.md`, Executed versus compiled and RED/GREEN sections; `docs/completion/execution/evidence/2026-10-22-language/parity/runtime-final.log:154-195`).

# Issues

## Critical

- None.

## Important

- None.

## Minor

- `specimen-studio/tests/language-parity.mjs:239`: scanner component weights, fractions, reasons and spans are checked by position, but each emitted `name` is ignored. A label regression could leave the independent comparator green while making the trace misleading. Pin the four expected names alongside their expected weights. The existing load-bearing digest corruption test remains valid.
- `docs/completion/execution/evidence/2026-10-22-language/parity/runtime-final.log:90-105` and `native-suite.log:2-41`: successful validation contains rust-objcopy SIGABRT/missing-libLLVM stripping warnings. The report discloses them accurately; this is host toolchain noise, not evidence of a fixture failure. Track environment repair separately and retain the warnings when describing this run.

# Assessment

**Task quality: Approved.** No blocking defect found in the scoped implementation. The witness covers the requested pure mechanisms and makes appropriately bounded execution claims.

**Review checks:** Read the supplied frozen patch and task/profile/dependency reports; inspected retained manifest, command, RED/GREEN, native-suite, Clippy and final parity evidence. The first diff output cut the middle hunk, so only that missing range was retrieved. No changed source file was separately opened, no codebase crawl or git diff was run, and no native/WASM/Clippy validation was rerun. No product source edits were made; only this requested review report was written.
