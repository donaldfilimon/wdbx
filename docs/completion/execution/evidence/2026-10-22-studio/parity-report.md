# Task 8.parity implementation report

Status: DONE_WITH_CONCERNS. Actual native/WASM identity mechanism execution agrees. No staging or commits. This is the bounded parity witness requested by task-8-parity-brief.md, not full Task 8 completion or a production identity integration.

## Files and scope

- `specimen-studio/native/specimen-wasm/Cargo.toml`: explicit dependency-free `identity-probe` feature and example requiring that feature.
- `specimen-studio/native/specimen-wasm/src/lib.rs`: feature-gated module and no-input raw WASM export registration; existing buffer hand-out/free discipline reused.
- `specimen-studio/native/specimen-wasm/src/identity_probe.rs`: bounded deterministic synthetic runtime fixture, using the reviewed pure-kernel identity API and the existing `tests/identity_fixture.rs` hook by path.
- `specimen-studio/native/specimen-wasm/examples/identity_probe.rs`: native execution of the same fixture, emitting the JSON bytes without an extra newline.
- `specimen-studio/native/specimen-wasm/tests/identity_probe.rs`: focused independent expected refs/counts/order and deterministic repeat assertions.
- `specimen-studio/scripts/check-identity-parity.sh`: explicit locked native/default-WASM/feature-WASM gate, isolated retained target and artifacts, identity/hash output and Bun invocation.
- `specimen-studio/tests/identity-parity.mjs`: standalone WASM execution, owned UTF-8 buffer release, default export absence, exact-byte comparison and independent field assertions.
- This report.

Read the parity brief, parent task brief, identity implementation report, Studio AGENTS.md and identity implementation API. Preserved existing dirty work. No kernel source, production dispatcher/command ABI, Tauri, package scripts, existing probes/goldens, dependencies, lockfiles, TS product source or substrate was changed. No corpus/holdout, installs, network, services, subagents, staging or commits were used.

## Runtime fixture and independent assertions

The JSON is only an experiment artifact: `schema=task8-identity-mechanism-probe-v1`. It is neither normative identity encoding nor save/wire data. The output contains runtime API observations; expected arrays are independently constructed by JS and the native focused test.

- Capacity: 20 entries fill node 1. Entry 21 returns `Full` without mutation when no host ref is supplied. A colliding spill ref returns `DuplicateRef` without mutation. A valid host ref creates node 2 and admits entry 1021. Ordered lookup returns exactly 1001..1021 and node refs exactly `[1,2]`. A fresh-ref semantic retry returns node 1/entry 1001 and creates no additional record.
- Original-byte collision: distinct same-key entries 2001/2002 on distinct concrete nodes preserve precomposed `é` bytes `[195,169]` and combining-spelling bytes `[101,204,129]` separately.
- Complete-key dimensions: base plus independently changed encoder version, modality, resolution and only the digest's last byte each return their own singleton entry ref 3001..3005. Wrong-key reuse of node 21 returns `KeyMismatch` without mutation.
- Entry lookup: exactly 4001..4064 in the 64-record array path, exactly 4001..4065 in the 65-record indexed path, the same complete order after rebuild, and reversed order after importing reversed owned entries. Final-match entry 4065 remains reachable with original `indexed 64`.
- Supporting resources: exactly 5001..5064 for array logical-link lookup, exactly 5001..5065 for indexed logical-link lookup, exactly that sequence after rebuild, and reversed sequence after a reversed resource import. All 65 share the same logical link and original, retaining their concrete refs. Structural lookup includes the unlinked resource 5066 and its original `unlinked original`. These exact ref assertions strengthen the identity review's resource-crossover count-only observation.
- Atomic rejection: nonfinite-weight insert returns `InvalidWeight` with unchanged complete snapshot; duplicate-node-ref import returns `DuplicateRef` with unchanged complete snapshot.
- Reviewed fixture hook independently checks `[1001..1065,2001,2002,2003,2001,2002]` from its actual 65-entry/rebuild and linked/unlinked resource execution.

The export takes no input, so callers cannot supply unbounded JSON or amplify fixture size. JS checks pointer, length (1..65536), memory bounds and fatal UTF-8 decoding, copies output and frees it exactly once through the existing `free`. Host imports throw if called: the successful fixture requires no wall clock, random IDs or progress service. Missing tools, missing target/build/export, traps, malformed JSON, byte mismatch and unexpected output shape all fail. Exact positive property assertions prevent two identical error envelopes from being accepted as parity.

## RED, GREEN and exact commands

All Cargo commands used pinned `RUSTUP_TOOLCHAIN=1.99.1` and this environment prefix:

```sh
RUSTUP_TOOLCHAIN=1.99.1 PATH=/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:/usr/bin:$PATH
```

Commands below run from `/Users/donald/sources/repos/wdbx` unless marked otherwise. Raw logs are retained under `/tmp`; they contain complete compiler/runtime stdout and stderr.

Initial manifest setup first refused a missing example file; that setup failure is not the required RED. After creating the example and focused test, before registering or implementing the probe:

```sh
<environment> cargo test --locked --manifest-path specimen-studio/Cargo.toml \
  -p specimen-wasm --features identity-probe --test identity_probe
# exit 101; /tmp/task-8-parity-red.log
# error[E0433]: cannot find `identity_probe` in `specimen_wasm`
# tests/identity_probe.rs:7:31 and :40:37
```

After implementation and formatting of owned Rust files:

```sh
<environment> cargo test --locked --manifest-path specimen-studio/Cargo.toml \
  -p specimen-wasm --features identity-probe --test identity_probe
# exit 0; /tmp/task-8-parity-native-green.log
# running 1 test
# test fixture_executes_successful_identity_mechanisms ... ok
# test result: ok. 1 passed; 0 failed; 0 ignored; finished in 0.01s

<environment> cargo clippy --locked --manifest-path specimen-studio/Cargo.toml \
  -p specimen-wasm --all-targets --features identity-probe -- -D warnings
# exit 0; /tmp/task-8-parity-clippy.log
# Checking specimen-wasm v0.2.0
# Finished dev profile in 0.33s

<environment> bash specimen-studio/scripts/check-identity-parity.sh
# exit 0; /tmp/task-8-parity-runtime-green.log
# native example compiled and executed; default WASM compiled/copied; feature WASM compiled/copied/executed
# default-wasm ... identity_probe_raw=absent
# feature-wasm ... identity_probe_raw=executed
# native-output ... bytes=4168
# wasm-output ... bytes=4168
# PASS: exact runtime bytes and independent fixture properties
```

The shell gate internally runs these locked commands from Studio, with `$artifacts` set to its retained temporary directory:

```sh
cargo run --locked -p specimen-wasm --features identity-probe --example identity_probe \
  --target-dir "$artifacts/target" > "$artifacts/native.json"
cargo build --locked -p specimen-wasm --target wasm32-unknown-unknown --release \
  --no-default-features --target-dir "$artifacts/target"
# copy default.wasm before the next feature build
cargo build --locked -p specimen-wasm --target wasm32-unknown-unknown --release \
  --features identity-probe --target-dir "$artifacts/target"
bun tests/identity-parity.mjs "$artifacts/native.json" "$artifacts/identity.wasm" \
  "$artifacts/default.wasm" "$artifacts/wasm.json"
```

No normal browser WASM output is overwritten by this isolated gate. The default module retains `memory`, `alloc`, `free`, `call` and `probe_raw` and omits the new test export. Full browser/lint/native gates remain parent final-recheck work; the focused feature checks above do not stand in for those gates.

## Deliberately corrupt native-output witness

With `artifacts=/var/folders/c_/thtfn_7d1p9378rxsp4pwz6c0000gn/T/task-8-identity-parity.smflwU`, Bun parsed the successful `native.json`, changed only `value.lookup.indexed[64]` from 4065 to 999999 and wrote `native-corrupt.json`. The good native JSON and both WASM binaries were retained unchanged. Executed:

```sh
bun specimen-studio/tests/identity-parity.mjs \
  "$artifacts/native-corrupt.json" "$artifacts/identity.wasm" \
  "$artifacts/default.wasm" "$artifacts/negative-wasm.json"
# exit 1; /tmp/task-8-parity-negative.log
# AssertionError: native and WASM runtime bytes differ
# at tests/identity-parity.mjs:41:8
```

The wrapper explicitly asserted that this command's exit status was nonzero and passed. Corrupt-native SHA-256 is `0079ef94d7d1e0ef52accbc751ed602414af56893aa6fceccc5ea083d68f8b1a`; the freshly executed negative-run WASM output still has the correct output hash below. This is a failing comparator witness, not a regenerated golden.

## Runtime, source and binary identity

Source HEAD during gate: `36bc99b699de60be6c07b2838d74580084fb5d4c`, with the preexisting dirty work and new owned files. The runtime raw log prints hashes for the Studio Cargo manifest/lockfile, every parity implementation/test/script file, identity module sources and reviewed fixture hook. These identify inspected source bytes without pretending the dirty tree is described by HEAD alone.

Toolchain/runtime:

- Cargo `/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin/cargo`: `cargo 1.99.1 (5f94df478 2026-08-27)`; SHA-256 `6d576d8058f8ca643e8fce55b64ee3c614ba64c9da2e536bae843e6a2e1f3951`.
- Rustc same toolchain bin directory: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`; commit `58cf8f9c35b355199727627d07ae34a4e2846d67`, host `aarch64-apple-darwin`, LLVM 23.1.1; SHA-256 `1ba7bb8e273bd64125f4048d2ecce530e0a6470e565ab1b3aecf0b82b532f7b1`.
- Bun `/opt/homebrew/bin/bun`: `--version` prints `1.4.4`; failure diagnostic identifies `1.4.4-canary.1+ea6a2c7b7 (macOS arm64)`; SHA-256 `30b3578e2f0e4901998ed11d85566f294f7b654da00883fd2357eed7387b2621`. This is the installed runtime, not CI's Bun 1.4.0.

Retained artifact directory: `/var/folders/c_/thtfn_7d1p9378rxsp4pwz6c0000gn/T/task-8-identity-parity.smflwU` (the raw log spells an equivalent double slash after `T/`). No binary is committed.

| Artifact relative to retained directory | SHA-256 |
| --- | --- |
| `target/debug/examples/identity_probe` | `8ae0a102914fd689559a3a7ee1d999df9bedc9a318c7cff4528a8e798e1bf2ef` |
| `default.wasm` | `58d010f48c79f6ad109d975d186a817c9569488c0cd33cd1d563639607da47ca` |
| `identity.wasm` | `76a9d1595ea206c03791df599891d2aac1d01b6cae9beaa72a5022ce50f7cf58` |
| `native.json` (4168 bytes) | `eb84ea042097bba7786b5c6cf23ae808b8a95f02887c668999934d44760d55d1` |
| `wasm.json` (4168 bytes) | `eb84ea042097bba7786b5c6cf23ae808b8a95f02887c668999934d44760d55d1` |
| `negative-wasm.json` | `eb84ea042097bba7786b5c6cf23ae808b8a95f02887c668999934d44760d55d1` |

## Self-review and limitations

- Confirmed production module/export registration is behind `identity-probe` and the default feature set does not enable it. Example is unavailable without explicit feature. Existing probes, goldens, dispatcher and package scripts were untouched.
- Confirmed the fixture calls the reviewed API for every reported mechanism. Only expected values live in comparator/tests; probe result arrays come from actual lookups/snapshots. Unexpected refusal panics instead of returning a shared error JSON. The direct assertions require successful admitted records as well as expected refusal behavior.
- Confirmed 64/65 entry and logical-link resource transitions, exact ordering after both ordinary and reordered rebuilds, final entry reachability, complete-key separation, retained original bytes and transactional failures. Native focused test repeats the fixture to check deterministic output.
- No new dependency or unsafe buffer pattern. The no-input export uses existing `hand_out`; JS checks bounds, copies and releases once. Native JSON equality includes key ordering and exact UTF-8 bytes.
- `git diff --check` passed. Largest new Rust file is 228 lines. Only owned new Rust files were named to rustfmt; the included reviewed fixture was already formatted and its recorded identity hash remains unchanged.
- Native test/example links emit the preexisting nonfatal `rust-objcopy` SIGABRT diagnostic because `@rpath/libLLVM.dylib` is missing. Tests/example really execute and succeed; feature Clippy with warnings denied succeeds. No toolchain/library/profile changes hide the diagnostic. This is an environment concern and is not clean-link evidence.
- Evidence is one installed native arm64 runtime and one Bun WebAssembly engine on this host. It does not qualify all WASM engines, platforms, production host adapters, persistence/import schemas, source encoder efficacy, migration, model/hardware execution, complete frozen cases or full Task 8.
- Temporary artifacts are retained for review and may later be cleaned by the host. Durable parent execution receipts should preserve the text raw logs/report; no generated binary belongs in the repository.
