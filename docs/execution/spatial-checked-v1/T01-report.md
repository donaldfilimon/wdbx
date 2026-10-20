# T01 stable baseline qualification

The pinned default workspace gate passed on Rust 1.99.1, target aarch64-apple-darwin. Its test stage reported **647 passed, 0 failed, 0 ignored**, across 24 summaries (19 nonzero, 5 zero). `cargo check --workspace --all-targets --locked` separately passed and supplies compile evidence only.

This is baseline qualification of HEAD `08a69b3f22db61093e06527b3308b10589687022` plus the formatting-only ordered-fold correction in `cpu.rs`, before checked-spatial implementation. The initial root gate failed formatting; `evidence/workspace-gate.txt` preserves that failure. This later pass does not qualify the eventual implementation diff. `evidence/t01-source-hashes.json` pins the tracked source inputs; none changed between that snapshot and completion. The identity log also preserves the exact cpu.rs diff and compiler version.

The executed command was `bash tools/check.sh`, with `RUSTUP_TOOLCHAIN=1.99.1` and the pinned toolchain bin directory followed by `/usr/bin:/bin:/usr/sbin:/sbin` prepended to PATH. The script does not override toolchain selection. It ran instruction and size checks, formatting, workspace/all-target clippy and workspace tests. Clippy produced one permitted pedantic warning (`v3_memory_edge`, 123/100 lines); it did not fail. Reported clippy build time was 6.66s and test build time 12.28s. Peak memory and CPU utilization were not measured.

Cargo.lock SHA-256 remained `4900d9f3e71f6915b0a3fb88a74a357772151b82975cb2dd0be07099262f9f54`. The canonical gate does not pass `--locked`; the before/after digest establishes preservation. No dependency or toolchain changes were made.

The actual optional feature matrix is `abi-wdbx/full-fhe` and `abi-wdbx/experimental-dghv-bootstrap`, with default features empty. abi-compute declares no feature switches. Only workspace defaults were qualified; both optional FHE tuples remain explicitly unverified and outside this spatial packet. No all-features union was forced.

ABI remains pinned to nightly-2026-09-01 and consumes canonical sibling WDBX paths, not this worktree. Studio remains a separate excluded workspace on nightly-2026-09-01 with WDBX git revision `3ac03f085196dbe6e922a127ceaa37bc2368de89`. Both were inspected read-only and remain untested. No Abbey sibling checkout was present. No Studio native/model workloads ran.

The earlier execution pause and current local-test authorization are separate states: these results come from the currently authorized local run. CI remains paused. No live stores were used. No SIMD ISA, speedup, performance, consumer integration or production-system claim follows from this gate.

Named acceptance cases: `compiler_is_requested_release` passes by pinned executable and rustc -Vv; `check_is_not_test_evidence` passes through separate compile-only and executed-test records; `feature_matrix_is_supported` passes through manifest inspection and explicit unverified optional tuples. Machine-readable details and raw log paths are in `docs/claims/wdbx-3d/stable-qualification.json`.

The `real_store::reads_the_real_store_when_opted_in` harness case returns early without opt-in; its reported success is not live-store evidence. A post-run environment presence check found {"ABI_WDBX_REAL_STORE_TEST": false, "ABI_ZIG_BIN": false}. This was not a captured pre-run environment, and no claim of explicit pre-run variable removal is made for this baseline. Subsequent final gates must explicitly remove both variables in the child environment.
