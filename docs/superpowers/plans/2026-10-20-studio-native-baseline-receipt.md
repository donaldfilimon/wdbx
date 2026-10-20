# Studio locked native baseline receipt

2026-10-20. Root HEAD5182bc3; Studio source unchanged. This receipt records qualification, not stable migration, research efficacy, browser/WASM artifact or hardware evidence.

## Dependency availability

Normal installed Cargo1.99.1 `cargo fetch --locked --manifest-path specimen-studio/Cargo.toml` exited0. Source https://github.com/donaldfilimon/wdbx at locked3ac03f085196dbe6e922a127ceaa37bc2368de89 and other locked dependencies retrieved. Log `/tmp/wdbx-20261020-studio-fetch.log`. No approval denial, network failure, credential change, cache substitution, system software or toolchain installation. Lock/manifests unchanged. Initial direct-Cargo invocation with +toolchain syntax failed locally, corrected by using installed toolchain-bin PATH without that rustup-only argument.

## Native test result

Command:

```sh
PATH=/Users/donald/.rustup/toolchains/nightly-2026-09-01-aarch64-apple-darwin/bin:/usr/bin:/bin CARGO_BUILD_JOBS=2 cargo test --offline --locked --manifest-path specimen-studio/Cargo.toml -p specimen-kernel -p specimen-core -p specimen-wasm
```

Exit0; **90passed,0failed,1ignored**,13summaries. Log `/tmp/wdbx-20261020-studio-native-baseline.log`.

Ignored `neural::tests::cpu_gpu_parity`: requires qualified GPU. Multiple installed-nightly rust-objcopy debug-stripping SIGABRT warnings occurred; test executables nevertheless ran. No hardware parity claim or pristine compiler-output claim. Existing tests include synthetic conformance, persistence recovery, mutation, network edits, raft planning and native WASM-probe reference paths. This is not execution of compiled browser WASM.

## Remaining qualification

Stable1.99.1/pin transition, successful native Clippy, browser check/lint, real WASM parity, installed production packages, real model/hardware qualification and all other platform receipts remain open. No Rust/Studio tracked files changed in this qualification.

## Source hashes

- `specimen-studio/rust-toolchain.toml`: `d338641f9a9ef86e52206e08411cbee71c4d117b17f89bec416c30087b3ac5e7`
- `specimen-studio/Cargo.lock`: `4de8c5cef824339500f05696f2473bd648b840f1a9f0495d26f209341426b8dd`
- `specimen-studio/native/specimen-core/Cargo.toml`: `db1452415da037dbe0a0d423bf8e5a4fffca7f0a9b86921b389e0ec367d7663c`
- `specimen-studio/native/specimen-kernel/Cargo.toml`: `bcabc6c49c3b3a75a0ebc6dfaec388d3c80a30f3c64cf342a0f6195bbb33c53b`

## Full native Clippy attempt

Installed nightly, offline locked: `cargo clippy --manifest-path specimen-studio/Cargo.toml --workspace --all-targets -- -D warnings` exited101. Desktop custom build failed: `glob pattern binaries/* path not found or didn't match any files.` Missing `src-tauri/binaries/` sidecars remain an exact build prerequisite. Nightly stripping warning identifies `Library not loaded: @rpath/libLLVM.dylib`; no system/toolchain repair attempted. Log `/tmp/wdbx-20261020-studio-native-clippy.log`. No successful full native Clippy claim; no sidecar/model downloads or hidden fallback.

Sidecar prerequisite inspection: `scripts/build-runtimes.py` builds `llama-server` from ggml-org/llama.cpp@163a40796f0ebaae246325f8d2e15028b413fa9d, `sd-cli` from leejet/stable-diffusion.cpp@6b3edaaf32cc19e5bb2d819c788bd557eddc8eba, and its ggml source ggml-org/ggml@e20c3a14aa70ee84ca58499814206dd08d8026bc. Expected work/runtimes source CMakeLists and output directory are absent. Commands `python3 scripts/build-runtimes.py --only llama` and `--only diffusion` (from Studio) retrieve pinned codeload archives when missing, compile local CPU executables with CMake, copy executables/licenses to src-tauri/binaries; no model weights. No existing local source-only build can currently satisfy these prerequisites. These commands were inspected, not executed. No sidecar download/build or dummy resource/gate weakening.
