# WDBX Zig inventory and Rust 1.99.1 checkpoint

New direct user requirement relayed at 09:20 UTC: active project Zig behavior must move to stable Rust 1.99.1 with API/save/wire/behavior parity before retiring Zig paths. This is a checkpoint within the [existing completion program](2026-10-20-wdbx-full-completion.md), not permission to restart historical Zig implementation or rewrite unrelated languages.

Owner: WDBX chat `01a15872-c366-7cc1-97e6-274eb388ed4b`. ABI and Abbey keep ownership of their repositories under the [consumer checkpoints](2026-10-20-wdbx-consumer-checkpoints.md). No consumer-owned source or plans are changed by this inventory.

## Inventory (source inspection)

| Classification | Exact paths | Observation / disposition |
| --- | --- | --- |
| Active project Zig source | Tracked and nonignored project `*.zig`, `*.zig.zon`, `build.zig` files | None found in WDBX/Studio. Generated build output, Git metadata and node_modules were excluded. No Zig production subsystem was identified to rewrite. |
| Root compiler/build | `rust-toolchain.toml`, `Cargo.toml`, `tools/check.sh`, `.github/workflows/ci.yml` | Root is already Rust, pinned to 1.99.1, edition 2024/floor 1.99. No active Zig build command found in project gate/CI/scripts. |
| Transitional external Zig oracle | `crates/abi-wdbx/tests/zig_writer_compat.rs` | Optional `ABI_ZIG_BIN` project CLI verifies a Rust-created checkpoint/WAL. Without it the test returns; default gate is not Zig parity evidence. Preserve until replacement regression evidence exists. |
| Transitional external Zig oracle | `crates/abi-wdbx/tests/multiway_zig_compat.rs` | Optional external project CLI compares canonical multiway export. Preserve historical behavior/lineage; replace oracle dependency only after independent expected bytes are frozen. |
| Historical port provenance | Rust module comments under `crates/abi-foundation`, `abi-telemetry`, `abi-core`, `abi-wdbx`; `crates/abi-foundation/src/wyhash.rs` | Describes Rust ports and a historical reference-generation snippet. These mentions are not active Zig source/build dependencies and must not be erased as if they were runtime code. |
| Historical independent fixtures | `crates/abi-foundation/tests/wyhash_zig_refs.txt`, `crates/abi-wdbx/tests/golden/wdbx-format.md` and existing segment goldens | Retain provenance and bytes. Do not regenerate reference expectations from the Rust implementation being tested. |
| Separate Studio Rust workspace | `specimen-studio/rust-toolchain.toml`, `native/specimen-core/Cargo.toml`, `native/specimen-kernel/Cargo.toml`, workspace lockfile | Toolchain pin remains nightly-2026-09-01. Donald allowed the local folder combine: `native/specimen-core/Cargo.toml` path-depends on `../../../crates/abi-wdbx` and `../../../crates/abi-compute`, and `native/specimen-kernel/Cargo.toml` path-depends on `../../../crates/abi-compute`. Git pin `3ac03f085196dbe6e922a127ceaa37bc2368de89` is not what Studio compiles. `rustup run 1.99.1 cargo check -p specimen-kernel -p specimen-core --manifest-path /Users/donald/sources/repos/wdbx/specimen-studio/Cargo.toml` exited 0 in 15.77s. Full native/WASM/browser qualification is not done. Stable alignment remains a consumer dependency/parity checkpoint, not a Zig rewrite. |

The installed `/opt/homebrew/bin/zig` is a compiler, not the old ABI project CLI expected by `ABI_ZIG_BIN`. No legacy project binary was found at `../abi/zig-out/bin/abi` or `../abi/bin/abi`; this does not assert that no archived artifact exists elsewhere. No install or historical build was attempted.

## Evidence and milestones

- [x] **Z-0 Inventory:** no project Zig source files or active Zig gate/build invocations identified; two optional external-oracle tests and historical fixtures classified separately.
- [x] **Z-1 Root toolchain:** installed `rustc 1.99.1 (58cf8f9c3 2026-10-13)` and `cargo 1.99.1 (5f94df478 2026-08-27)` verified through the pinned toolchain directory, without installation. The baseline root gate passed 647 tests, zero failures/ignored, on the preserved working source. Optional Zig-oracle branches were not exercised and are not counted as cross-language execution evidence.
- [ ] **Z-2 Oracle retirement:** locate already-retained independently produced checkpoint/WAL/multiway byte fixtures or an authorized historical project-binary receipt. Add always-running Rust 1.99.1 regression coverage against those independent bytes. Check record counts/order, chain validity, WAL frames and exact multiway export. Only then retire active external-Zig test invocation; keep reference provenance. If no independent oracle artifact is available, record that limit rather than generating a self-comparison.

### Z-2 observed limit (2026-10-20)

Searched the WDBX tree for a checkpoint, WAL, or multiway export that was not produced by the Rust encoder under test. None is retained.

- `crates/abi-wdbx/tests/zig_writer_compat.rs` builds a temporary store and, only when `ABI_ZIG_BIN` is set, asks that external CLI to verify it. The expected stderr lines are not stored bytes.
- `crates/abi-wdbx/tests/multiway_zig_compat.rs` asks the same external CLI to write JSON and then compares that process output with `export_multiway_json`. No frozen Zig export is in the tree.
- `crates/abi-wdbx/tests/golden/` holds segment JSONL and `abbey-cbor-episode-v1` fixtures. Those are not the checkpoint/WAL pair or the multiway export these two tests construct.
- `crates/abi-foundation/tests/wyhash_zig_refs.txt` remains a historical hash reference. It is not a checkpoint, WAL, or multiway fixture.
- No legacy project binary was present at `../abi/zig-out/bin/abi` or `../abi/bin/abi`. The installed Zig compiler is not that CLI.

The opt-in `ABI_ZIG_BIN` tests stay. They are not default-gate parity evidence. No self-comparison fixture was generated, and the process spawn was not removed. A later retirement needs an independently produced byte fixture or a historical project-binary receipt.
- [ ] **Z-3 Studio stable alignment:** Donald allowed the local folder combine. `specimen-studio/native/specimen-core/Cargo.toml` path-depends on `../../../crates/abi-wdbx` and `../../../crates/abi-compute`; `specimen-studio/native/specimen-kernel/Cargo.toml` path-depends on `../../../crates/abi-compute`. Git pin `3ac03f085196dbe6e922a127ceaa37bc2368de89` is not what Studio compiles. `rustup run 1.99.1 cargo check -p specimen-kernel -p specimen-core --manifest-path /Users/donald/sources/repos/wdbx/specimen-studio/Cargo.toml` exited 0 in 15.77s. Still open: `specimen-studio/rust-toolchain.toml` remains nightly-2026-09-01, and full native/WASM/browser qualification is not done. Qualification must cover archive/save compatibility, IDs, numerical/native-WASM parity and unchanged kernels/providers. No dependency install or publication is authorized by this checkpoint.
- [ ] **Z-4 Consumer handoff:** WDBX supplies exact qualified source and fixture/corpus identities; ABI/Abbey owners qualify their named revisions under their reservations. Do not silently update consumers or claim a moving sibling HEAD is qualified.

## Acceptance and remaining limits

Root source/build language already satisfies the migration direction; there is no new Rust replacement to claim from this inventory. The optional oracle transition and Studio stable qualification remain open. The bounded CBOR reader continues under the same Rust 1.99.1 lane, preserving all frozen commitments and original signed bytes.

No CI, new commits/pushes, installs, destructive operations, deployments, live stores or public launch are introduced. This plan and the execution ledger persist the inventory and dependency order; subsequent source/test changes require their own reviewed working-tree snapshot and actual local evidence.

## Selected next migration acceptance: Z-3 Studio stable alignment

Select this one migration checkpoint at the next safe boundary; do not open a second implementation writer. Conformance repair continues first. Its research findings do not semantically gate a compiler/dependency transition, but its current writer reservation gates scheduling; the already-prepared reader remains in the approved queue.

Donald allowed the local folder combine. Studio does not compile Git pin `3ac03f085196dbe6e922a127ceaa37bc2368de89`. The two Cargo.toml path deps are `specimen-studio/native/specimen-core/Cargo.toml` (`abi-wdbx` at `../../../crates/abi-wdbx` and `abi-compute` at `../../../crates/abi-compute`) and `specimen-studio/native/specimen-kernel/Cargo.toml` (`abi-compute` at `../../../crates/abi-compute`). `rustup run 1.99.1 cargo check -p specimen-kernel -p specimen-core --manifest-path /Users/donald/sources/repos/wdbx/specimen-studio/Cargo.toml` exited 0 in 15.77s. That check does not close this checkpoint.

Still open: `specimen-studio/rust-toolchain.toml` remains nightly-2026-09-01 and was not changed. Full native host, WASM, and browser qualification are not done. Qualification must cover archive/save compatibility, IDs, numerical/native-WASM parity, and unchanged kernels/providers. No dependency install or publication is authorized by this checkpoint. No reference bytes will be regenerated from the Rust implementation under test.

Candidate immutable WDBX revision `2969d71a2c3a81a060dcfc6cf53b4617f13c8b94` remains a candidate in this repository and the local `origin/main` tracking ref. It is not the source Studio compiles, and it is not a claim of fresh Studio or paired-consumer qualification. The root baseline source gate above is prior working-source evidence.

Acceptance still open:

1. Change the Studio toolchain pin from nightly-2026-09-01 to installed stable 1.99.1 only with the remaining qualification. The path-dep check above is not that pin change.
2. Run the native tests and warnings-denied native Clippy.
3. Build `specimen-wasm` for `wasm32-unknown-unknown` with the same stable toolchain; run the existing native/WASM golden and archive/revision/recovery conformance checks. Existing ignored hardware tests remain explicitly unqualified.
4. Run Studio's separate browser/build/lint gates when its permitted locked Bun/Node dependencies are available; preserve original numerical/save/golden bytes, independently review the exact working-tree patch, and record all artifact/content hashes and skips. A native pass alone cannot close Studio qualification.
