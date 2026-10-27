# Integrated language/text/scanner final fix report

2026-10-22 local / 2026-10-23 UTC. **Product source and current evidence frozen for the one scoped independent rereview.** The three Important findings and three small API/assertion Minors are addressed by implementation and bounded regressions. This is not review preapproval, production adoption or efficacy acceptance. No subagents, staging, commit, install, network, frozen holdout, live store or host effects were used. Unrelated dirty work is preserved.

All paths below are relative to repository root; `F` means `docs/completion/execution/evidence/2026-10-22-language/final-fix/`. This report is copied there. Old raw evidence, manifests, receipts and original legacy-baseline.json are preserved. The old one-shot observation refresh was not rerun.

## Finding dispositions

1. **Important 1, opaque conditions:** `language_v3/prepare.rs` compares the complete original antecedent through token-span-aware normalization. Only unquoted token text is lowercased; every quoted byte, escape, case and Unicode sequence and intervening whitespace remains exact. Context does not infer truth. Regressions cover exact positive, quoted case/Unicode/canonical-sequence mismatch, unquoted case, full compound antecedent versus partial attestation, escaped quotes and apostrophes, and guarded execution refusal.
2. **Important 2, control vocabulary:** `UtilityTables::new` refuses Grammar Negation/Connective assignments, conjunctions consuming builtin negators, any shared token between custom negation and conjunction phrases (including cross-prefix/suffix overlap), reserved If/Then reassignment, and conflicting builtin connectives. This intentionally conservative constructor returns InvalidTables before any Prepared value. Valid custom negators still inhibit guarded execution, quoted controls remain inert, and the existing nine-utility/rewrite suites pass. No universal language efficacy claim follows.
3. **Important 3, bounded candidate work:** routes use an O(1) borrowed `HybridTable::entries()` slice. Query preflight reserves table indexing before visiting entries or allocating the reference map, even for empty Prepared input. Structural matches stream in insertion order rather than allocating `find_all`. Borrowed checks reserve population/key comparisons and every selected original, alternative vector, nested expression/string/argument vector and origin payload before cloning. Registration borrows entries instead of copying the complete table and its resources. No identity acceptance ceiling was introduced. Regressions exercise a 257-entry empty query with one work step, a valid 12000-alternative selected target, many structural matches with shared route origins, nested payloads, and exact success/error budget boundaries.
4. **Minor 1:** independent JS pins exactly four ordered component labels and count. Separate actual comparator processes reject digest and component-label corruption; self-test also rejects a missing component. The pre-fix comparator accepted the corrupted label (RED evidence preserved).
5. **Minor 2:** `SemanticAst::token_count`, PreparedAction complexity and text-v3 now share one lexical count excluding punctuation and counting explicit connective/negation nodes. Direct agreement tests cover punctuation, quoted literals/operators and conditional/negated trees.
6. **Minor 3:** registry validation and parsing share the ASCII call-name alphabet (alphanumeric, underscore, hyphen). Declared names and aliases including `my-text`, underscore and digit cases compile and execute through registry lookup. Registry identity/subsystem validation remains separate in purpose.
7. **Minor 4:** environment qualifications are retained without repair: missing-libLLVM rust-objcopy stripping diagnostics; dependency/import/build-script/block future-compatibility notices; browser chunk/static-analysis limitations; Bun 1.4.4 canary versus CI 1.4.0; one existing ignored native test.

## Source and API boundary

Exactly 12 previously existing product files changed and four files were added, all within the brief. `F/product-final-fix.diff` contains the exact pre-fix to post-fix source/test/script patch. `F/product-freeze.json` pins 43 relevant files. The additions are kernel `tests/language_final_fix.rs`, `tests/routes_final_fix.rs`, route `work.rs`, and the feature-witness `language_probe/final_fix.rs`. The existing language-probe test and JS witness were extended.

The sole authorized legacy extension is five lines in `identity/mod.rs`: two documentation lines and an immutable borrowed `entries()` accessor. `F/identity-mod.before.txt` retains original SHA256 `2b28dc381bc67ee27cb484618188b2457fc58f53a37da09057ace5e1b67182c1`. `F/identity-accessor-extension.json` records the exact addition and proves removing only those bytes reproduces the original body. **The original 23-file baseline is never repinned: 22 unchanged files plus one explicit accessor-only extension.** Identity admission/IDs/algorithms, save schema, default dispatch, legacy language/engine/mutate/commands, locks, goldens and frozen C0 contracts remain unchanged.

The query allowance is logical bounded work, not measured timing: table indexing reserves `n * (1 + bit_length(n))`, structural traversal reserves visits and compared metadata bytes, target preflight charges vector lengths before iteration and every nested string/tree/vector before copying, and repeated origins pay separately. No unbounded alternative count is inspected or copied before its O(1) length reservation. Complete results preserve candidate/order/constraint/target-inhibition semantics; exhaustion returns BudgetExceeded without a partial result. Unrelated resource contents and population payloads are not copied. Global scanning, scheduling and elapsed qualification remain outside this API.

## TDD and product verification

Pinned cached environment throughout: `PATH=/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:/usr/bin:/bin:/opt/homebrew/bin`, `RUSTUP_TOOLCHAIN=1.99.1`, `CARGO_NET_OFFLINE=true`. No toolchain fallback or dependency installation.

RED logs: `F/red.log` records all four new language/API tests failing behaviorally; `F/red-routes.log` records all three route tests failing behaviorally. Commands were `cargo test --manifest-path specimen-studio/Cargo.toml --locked -p specimen-kernel --test language_final_fix --test routes_final_fix`, then the route target alone because Cargo stops after the first failing target. `F/red-corrupt-label.json` and `F/red-comparator.log` preserve the pre-fix comparator accepting a corrupted name. Original independent review probe source/output are also copied under F.

`F/run-phase-a.py` and `F/phase-a-commands.json` record exact argv, cwd, pinned environment, start times, exit codes and log hashes. All six final commands exit 0, run from `specimen-studio/`:

```sh
cargo test --locked -p specimen-kernel --test language_final_fix --test routes_final_fix
cargo test --locked -p specimen-kernel -p specimen-wasm --features language-probe,identity-probe
cargo clippy --locked -p specimen-kernel -p specimen-wasm --all-targets --features language-probe,identity-probe -- -D warnings
cargo fmt --check -p specimen-kernel -p specimen-wasm
bash scripts/check-language-parity.sh
bash scripts/check-identity-parity.sh
```

Results: **7 focused passes; 175 full kernel/feature passes; feature Clippy and fmt pass.** Existing identity tests run in the full command. Separate runtime scripts build and actually execute both feature witnesses. Build directories remain retained in each runtime `retention.json`; copied raw artifacts remain under F.

The expanded language native/actual feature-WASM traces match at **151812 bytes**, SHA256 `743d07418aadbe60c704d0d934c2bffb237e62e1e39c162ad731aba7d4a8e4d9`, with zero host calls. Feature language WASM SHA256 is `28fa421f31cc3f087bc81d4e66774913fe7e949781c39dbbe9023bfb243f0ddd`. The unchanged identity traces match at **4205 bytes**, original SHA256 `60aaef7f217d27a6fe505f2c6405239e3032902e9964472e24273bca97c07580`; feature identity WASM SHA256 is `f8a0c5a272e9bbcbde8bb8b7c6e4f384652b588cce05228e3bcadf3ba87d925a`. Default WASM SHA256 is `c3f56ce5fa884c05f619c508b066341592f3826df0007b3e7cf0cd21e9b978e3`; it is only compiled/export-inspected, not instantiated. Full digest and label negative processes each exit 1 under the same semantic comparator.

After absolute product freeze, **root** ran the four separate Studio gates, all exit 0: 221 browser/unit passes and production build, lint, 228 native passes plus one existing ignored test, and native all-target Clippy. `F/gates/summary.json` and individual gate JSON/logs record actual commands/environment/times/hashes and the same 289 relevant source files before/after each gate. This fixer did not rerun those gates. No product source or product reruns followed the evidence-only refresh.

## Current evidence and inventory validation

`F/refresh-current.py` updates current pointers and mapped source/test lists without rerunning the append-only historical script. Current register/catalog/Studio observations/narrative/README and the exact catalog pin were refreshed; current mapping and source manifest are new files under F. The checker policy is unchanged except catalog SHA256 `2b9c55c5aa4ef46acf21ece5f30f8a7f35e2de2da9b9bdd691d058fed7acdc81`. Existing execution observations explicitly remain historical; new frozen-source executions are separately recorded. Old evidence bodies are never rewritten or reassigned.

Counts remain **163 open/C0/unknown; 120 candidate / 27 local gap / 16 external gap; 46 S + 18 D verbatim frozen rows; the same 15 local observation IDs.** All new source/test paths map to applicable requirements. The three Important findings and three small Minors are recorded as locally addressed with scoped rereview pending.

After the evidence stabilized, the following checks each passed under current Python 3.15.0 and system/Xcode Python 3.9.6 with `PYTHONDONTWRITEBYTECODE=1`:

```sh
python3 tools/check_completion_execution.py
python3 -m unittest discover -s tools/tests -p test_check_completion_execution.py
python3 docs/completion/execution/evidence/2026-10-22-language/final-fix/check-final-closure.py
python3 tools/check_completion.py
python3 -m unittest discover -s tools/tests -p test_check_completion.py
```

Each interpreter passed **27 execution mutations and 37 separate frozen-contract tests**. Exact executable identities/argv/cwd/times/exits/raw-log hashes are in `F/current-commands.json` and `F/system-commands.json`; `F/run-validation.py` is the recorder. `git diff --check` exits 0. No unchanged 3-D/FHE suite was rerun.

The new closure checks current references and mappings, old raw bytes and frozen contracts, the exact 12 reviewed source modifications/four additions, original baseline with only the explicit accessor extension, both actual trace hashes and all four fresh root gate logs/source continuity. The previous closure remains a historical checker of its snapshot and is not represented as current.

## Remaining limits and handoff

Exact decimal captures remain separate from f64 arithmetic; +20/-20 missing-feature consequences remain unchanged. Pure plans do not authenticate or perform real host effects. Active engine/commands/feedback, ID/save/import migration, durable utilities, full-chunk scanner, ATP/jitter/voting/global scheduler, scientific efficacy/contamination controls/adjudication, ABI-owner receipt and durable trust/admission remain open. OCR/model/GPU, all targets, installed desktop, browser UI/theme/accessibility and elapsed qualification remain separate gaps. No completion claim is promoted.

Both product and final evidence are now frozen. Root owns the exact combined source/evidence patch and one final scoped independent rereview. No second fix wave is presumed.
