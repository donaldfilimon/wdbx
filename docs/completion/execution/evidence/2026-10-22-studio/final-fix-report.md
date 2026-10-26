# Task 8 final review fix report — 2026-10-22

Status: DONE_WITH_CONCERNS. Both Minor findings in `studio-final-review.md` are addressed in one bounded fix wave. Source is stable for the controller's one scoped rereview. No staging or commits. Full WDBX completion and parent Task 8 remain incomplete; all 163 requirements stay open/C0/unknown with 118 candidates, 29 local gaps and 16 external gaps.

## Scope and resulting behavior

Read `task-8-final-fix-brief.md`, the full tranche final review, prior parity/evidence briefs and reports, Studio AGENTS.md and the TDD skill/reference. The preserved pre-fix source identity is `snapshot-studio-tranche-final/hashes.json`; this run's pre-fix hash observation is retained separately in `final-fix-before-hashes.json`.

Exactly six existing source/observation/narrative paths changed against that reviewed snapshot:

- `specimen-studio/native/specimen-wasm/src/identity_probe.rs`: separate `InsertOutcome::Inserted` and `InsertOutcome::Duplicate` match arms serialize a `kind` from the actual observed enum variant. No expected fixture-step label is used to infer the variant. `Full` remains its existing string.
- `specimen-studio/native/specimen-wasm/tests/identity_probe.rs`: independent literals require spill `{kind: Inserted, node: 2, entry: 1021}` and repeat `{kind: Duplicate, node: 1, entry: 1001}`, retaining every previous exact ref/order/state assertion.
- `specimen-studio/tests/identity-parity.mjs`: independent JS literals require those same observed success variants in addition to byte equality and all existing fixture properties.
- `docs/completion/execution/studio-research.md`: corrects the default-WASM description to separately compiled and exports inspected; only the feature artifact is instantiated. Describes current 4,205-byte success and kind-only negative evidence, resolves the prior Minor paragraph, and explicitly keeps earlier 4,168-byte evidence/default gates historical.
- `docs/completion/execution/studio-research-v1.json`: refreshes current source identities, adds three explicit mismatches against pre-fix historical pins, adds final-fix execution/log identities and resolves the old trace limitation. The old command outcomes remain intact and are explicitly qualified as historical prior-snapshot observations. Current S03's existing bounded mechanism description now includes observed success-kind discrimination; no new requirement mapping or scientific claim is introduced.
- `docs/completion/execution/register-v1.json`: refreshes existing reference hashes to the new source/observation bytes, attaches the fresh fix logs to already-mapped identity-probe rows, and resolves the stale success-kind Minor text while retaining LLVM and broader blockers. Routing, path mappings, disposition counts, all open/C0/unknown fields and historical audit objects remain unchanged.

The candidate path catalog and execution checker's independently pinned catalog literal need no changes: all implementation/test/inspection path mappings are unchanged. The copied observation-closure helper is byte-identical to the historical helper; no structural or no-promotion guard was relaxed.

No kernel identity behavior/source/test/fixture, Cargo feature/manifest/lockfile, raw export/buffer ownership, parity gate script, frozen contract, old raw log/report/review/receipt or unrelated dirty file changed. No subagents, installs, network, services, global toolchain changes, live data, holdouts or credentials were used.

## TDD and affected verification

The new assertion would detect missing or wrong success variant even when concrete refs and all state/order observations remain correct. Both independent consumers were changed before the serializer.

All commands below ran from `/Users/donald/sources/repos/wdbx`. Rust commands used:

```sh
export RUSTUP_TOOLCHAIN=1.99.1
export PATH=/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:/usr/bin:$PATH
```

1. `cargo test --locked --manifest-path specimen-studio/Cargo.toml -p specimen-wasm --features identity-probe --test identity_probe` before serializer fix: **exit 101**, 0 passed/1 failed. The real assertion sees `{node: 2, entry: 1021}` while expecting `{kind: Inserted, node: 2, entry: 1021}`. Raw: `final-fix-native-red.log`.
2. `bun specimen-studio/tests/identity-parity.mjs <prior>/native.json <prior>/identity.wasm <prior>/default.wasm /tmp/task-8-final-fix-js-red-wasm.json`: **exit 1** at the independent capacity expectation against preserved old output. Both prior native/WASM bytes still agree; missing kinds fail direct expectations. Raw: `final-fix-js-red.log`. `<prior>` is `/var/folders/c_/thtfn_7d1p9378rxsp4pwz6c0000gn/T/task-8-identity-parity.smflwU`.
3. Minimal serializer fix, followed by `rustfmt --edition 2024` naming only the owned probe source/test.
4. The same focused Cargo test: **exit 0**, 1 passed/0 failed. Raw: `final-fix-native-green.log`.
5. `cargo clippy --locked --manifest-path specimen-studio/Cargo.toml -p specimen-wasm --all-targets --features identity-probe -- -D warnings`: **exit 0**. Raw: `final-fix-clippy.log`.
6. `bash specimen-studio/scripts/check-identity-parity.sh`: **exit 0**, actual native example execution, separate default WASM build/copy, feature WASM build/instantiation/execution, exact 4,205-byte agreement, and independent expected `Inserted`/`Duplicate` plus all prior refs/order/state properties. Raw: `final-fix-parity-green.log`.

The gate builds into a new retained temporary directory; ordinary browser artifacts are not overwritten. The default WASM is compiled and exports inspected, with no identity export; its binary hash is identical to the prior default artifact. The feature artifact is instantiated and the probe executed. Unexpected host calls still throw and returned memory is copied/freed through the unchanged comparator ownership discipline.

## Kind-only negative witness

Starting with fresh successful native JSON, replace only the first exact `"kind":"Inserted"` with `"kind":"Duplicate"`, changing `capacity.spill.kind`. An independent JSON check restores that one field and asserts complete equality with the good output; all concrete refs, arrays, order and other values are therefore preserved. Write the corrupt JSON only in the new retained temporary directory. Execute:

```sh
bun specimen-studio/tests/identity-parity.mjs \
  "$artifacts/native-corrupt-kind.json" "$artifacts/identity.wasm" \
  "$artifacts/default.wasm" "$artifacts/negative-wasm.json"
```

**Exit 1 as expected**, `AssertionError: native and WASM runtime bytes differ`. The wrapper requires a nonzero comparator result, and confirms freshly executed negative-run WASM output still equals good native bytes. Raw `final-fix-kind-negative.log` includes the exact absolute command, single-field mutation statement, complete failure output and comparator exit. This negative result is deliberate evidence, not an unresolved positive failure.

Removing only the new spill/repeat `kind` fields from current successful decoded output reproduces the complete prior successful decoded trace. The new fielded JSON is an experiment artifact only; it does not change a normative save/wire schema or committed digest.

## Current runtime and artifact identities

Gate source HEAD: `36bc99b699de60be6c07b2838d74580084fb5d4c` plus the preserved dirty work; HEAD alone does not identify uncommitted source. The raw gate logs every relevant source hash.

- Cargo: `/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin/cargo`, `cargo 1.99.1 (5f94df478 2026-08-27)`, SHA-256 `6d576d8058f8ca643e8fce55b64ee3c614ba64c9da2e536bae843e6a2e1f3951`.
- Rustc: same toolchain bin directory, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, host `aarch64-apple-darwin`, LLVM 23.1.1, SHA-256 `1ba7bb8e273bd64125f4048d2ecce530e0a6470e565ab1b3aecf0b82b532f7b1`.
- Bun: `/opt/homebrew/bin/bun`, version 1.4.4; negative-run diagnostic identifies `1.4.4-canary.1+ea6a2c7b7 (macOS arm64)`, SHA-256 `30b3578e2f0e4901998ed11d85566f294f7b654da00883fd2357eed7387b2621`. CI specifies 1.4.0; this does not claim the CI runtime.

Retained artifacts: `/var/folders/c_/thtfn_7d1p9378rxsp4pwz6c0000gn/T/task-8-identity-parity.Xgldlk`. Binary files remain temporary, outside the repository. Their exact identities are also in `final-fix-artifact-hashes.json`.

| Retained artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/debug/examples/identity_probe` | 1516176 | `398653223dadbd049b758aee5ef4f32e8263fe7f8cf69cce2050ebb5062e099a` |
| `default.wasm` | 2556788 | `58d010f48c79f6ad109d975d186a817c9569488c0cd33cd1d563639607da47ca` |
| `identity.wasm` | 2664509 | `3b6475da09548aaa3e9531fac36f2d896d9866452dfde9de26994e84569c560b` |
| `native.json` | 4205 | `60aaef7f217d27a6fe505f2c6405239e3032902e9964472e24273bca97c07580` |
| `wasm.json` | 4205 | `60aaef7f217d27a6fe505f2c6405239e3032902e9964472e24273bca97c07580` |
| `native-corrupt-kind.json` | 4206 | `f33556413753ae1bc45c6fd6a86165ab97acccc776b47d2e89d47cf46a74d75a` |
| `negative-wasm.json` | 4205 | `60aaef7f217d27a6fe505f2c6405239e3032902e9964472e24273bca97c07580` |

Current modified source/observation identities (`final-fix-current-hashes.json`):

| Path | SHA-256 |
| --- | --- |
| `docs/completion/execution/register-v1.json` | `535a89bf360bdc3a0fbbaa581455a426d0a2b49a367073dde0a37bfecc1179ec` |
| `docs/completion/execution/studio-research-v1.json` | `27e55f78913a395012cce1c7a021042565803dfc4a2dbe46d7ad2e15c06d278d` |
| `docs/completion/execution/studio-research.md` | `5de233db49b692a1bb8919f79111b9a9b73779570f7e2a8eea729288dd314aeb` |
| `specimen-studio/native/specimen-wasm/src/identity_probe.rs` | `1219bef564855db408cd83fad1f95071e01afa501ef9715e0fbe17a615807c35` |
| `specimen-studio/native/specimen-wasm/tests/identity_probe.rs` | `86316c8343f8c5d4f72d04d43ce67478bd0847b48aff93dd327320aabb3b5ab3` |
| `specimen-studio/tests/identity-parity.mjs` | `938b098eab09f16b33151503603f9ee96a9e31984494b77cda4c0da00d5e7d48` |

## Inventory checks, preservation and limits

Both `/opt/homebrew/bin/python3` (Python 3.15.0) and `/usr/bin/python3` (Python 3.9.6) ran each of the following with `PYTHONDONTWRITEBYTECODE=1`; all commands **exit 0**:

```sh
<python> tools/check_completion.py
<python> tools/check_completion_execution.py
<python> -m unittest discover -s tools/tests -p test_check_completion_execution.py
<python> docs/completion/execution/evidence/2026-10-22-studio/final-fix/check-observation-closure.py
```

Each mutation suite ran **27 tests, OK**. Each closure checks ordered 46 S + 18 D IDs, verbatim frozen rows, 64 explicit blocker/C0/no-promotion records, **608 hash-matching safe file references**, design bundles, six existing bounded observations and unchanged counts/claim fields. It reports observation-artifact hash `27e55f78913a395012cce1c7a021042565803dfc4a2dbe46d7ad2e15c06d278d`. These are inventory checks, not product/efficacy/hardware qualification.

`final-fix-preservation.log` records independent standard-library comparisons: all **44 historical durable evidence files** match their pre-fix hashes; exactly the six allowed reviewed snapshot paths changed; kernel/fixture/manifests/locks/catalog/checker stayed unchanged; all 163 routing/claim fields, reference path mappings and 118/29/16 dispositions stayed unchanged; historical audit objects are identical. The old 4,168-byte witness `eb84ea042097bba7786b5c6cf23ae808b8a95f02887c668999934d44760d55d1`, old negative outputs and all old reports/reviews remain historical bytes, not tests of the amended trace. `git diff --check` exits 0 and cached path listing is empty. No stage or commit action was performed.

The full default Studio browser/lint/native/Clippy gates were run by the controller on the prior snapshot, as retained in their existing raw logs. This wave does not rerun them or relabel them as current amended opt-in-source validation. The requested focused feature test/Clippy and actual isolated parity gate were rerun and passed. Default WASM bytes/export absence remain unchanged.

Native links retain nonfatal `rust-objcopy` SIGABRT/missing `@rpath/libLLVM.dylib` diagnostics; actual native tests/example execute and return 0. The prior native Clippy `block 0.1.6` future-incompatibility notice remains disclosed. Nothing installed or altered the host toolchain to hide these observations; this is not pristine-link/toolchain qualification.

Broader scanner/text-v3 encoder, independent research/holdout/adjudication, production command/feedback/import/save migration, full overflow/host allocation, scheduler/raft/yielding, model/OCR/GPU, installed desktop/UI/theme, other platform/engine and elapsed-operation obligations remain open. The pending named ABI-owner receipt and current independent PyCA verifier failure remain in the existing crypto rows. No whole requirement, scientific claim or crypto profile is promoted by this synthetic witness or inventory closure. This report is implementation evidence awaiting the controller's one scoped rereview; it does not represent a new review verdict.

## Exact fresh durable evidence filenames

All new raw logs are under `docs/completion/execution/evidence/2026-10-22-studio/final-fix/`:

- `final-fix-clippy.log`
- `final-fix-current-completion-checker.log`
- `final-fix-current-execution-checker.log`
- `final-fix-current-execution-mutations.log`
- `final-fix-current-observation-closure.log`
- `final-fix-diff-check.log`
- `final-fix-js-red.log`
- `final-fix-kind-negative.log`
- `final-fix-native-green.log`
- `final-fix-native-red.log`
- `final-fix-parity-green.log`
- `final-fix-preservation.log`
- `final-fix-system-completion-checker.log`
- `final-fix-system-execution-checker.log`
- `final-fix-system-execution-mutations.log`
- `final-fix-system-observation-closure.log`

Additional fresh files in that directory:

- `check-observation-closure.py`
- `final-fix-artifact-hashes.json`
- `final-fix-before-hashes.json`
- `final-fix-current-hashes.json`
- `final-fix-evidence-hashes.json`
- `final-fix-results.json`

`final-fix-evidence-hashes.json` records hashes/byte counts for the new helper/results/hash snapshots/raw logs, excluding itself and the report to avoid circular identities. Temporary pre-fix source copies used for comparisons are outside the repository at `/tmp/task-8-final-fix-before`; their byte identities are pinned in `final-fix-before-hashes.json`. No historical raw output was overwritten.

The identical full report is saved at `.superpowers/sdd/2026-10-20-wdbx-full-completion/task-8-final-fix-report.md` and durably copied to `docs/completion/execution/evidence/2026-10-22-studio/final-fix-report.md`.
