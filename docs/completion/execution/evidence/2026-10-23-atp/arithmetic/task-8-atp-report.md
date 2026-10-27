# Task 8.atp arithmetic implementation report

2026-10-23. Private mechanism implementation, ready for independent review; no acceptance promotion. Branch `main`, HEAD `36bc99b699de60be6c07b2838d74580084fb5d4c`. This report is byte-identical at the requested W/task-8-atp-report.md and durable arithmetic/task-8-atp-report.md locations.

## Scope and authority

Implemented only three new private source files and one new private-inclusion integration test. No library registration, existing source, Cargo manifest, lock, frozen inventory, legacy ATP engine, scorer, save format, ID, command ABI or golden was edited. The private module is included with `#[path]` by the integration test. Its two source fragments use `include!` within that private module so every exposed type/function/method can remain `pub(super)` without public re-exports or an unchecked external constructor. Fields remain private; valid input constructors and immutable getters are the only input access. There is no serde/Deserialize surface or constructor bypass. Output state uses the same validated constructor.

Binding sources read: task-8-atp-brief.md, root and Studio AGENTS, Studio runtime profiles, designs.json atp/D10, shared-types-v1.json AtpState/task-08, and next-studio-research-preflight.md. The preflight was planning context, not a passing result. The TDD skill was applied. No subagents, network, installs, live stores, efficacy corpus/holdout, staging or commits were used.

## Implemented behavior and local numeric ruling

`State` and `Stimulus` are local immutable Copy values. Explicit typed errors distinguish each nonfinite and range violation in state valence/arousal/cooldown and stimulus valence/intensity. State ranges are v[-1,1], a[0,1], cooldown_seconds[0,30]; stimulus ranges are valence[-1,1], intensity[0,1]. Signed zero is normalized to positive zero at admission and on returned state/elapsed. Failed constructors and failed/successful transitions cannot mutate caller inputs.

`advance` returns a typed immutable Transition carrying state, applied_elapsed, released, and cooldown_at_entry. Nonfinite elapsed and negative elapsed have separate typed errors; -0 is admitted as 0 and any finite elapsed above3600 is capped to3600. There is no clock, seed, entropy, thread, I/O, allocation, host/model/GPU call or collection traversal in a scalar transition; work is constant.

The selected private controller ruling is `logicalf32-v1`: every named arithmetic operation and constant is f32; decay is `libm::expf((-dt)/180.0_f32)`, then separately rounded `d*v` and `d*a`. Valence charge is `(0.2_f32*valence)*intensity`; arousal uses separate `abase=a0+(0.2_f32*intensity)` then `a1=clamp(abase+(0.1_f32*urgency_as_0_or_1),0,1)`. Clamps precede inclusive `a1>=0.85_f32` release. Release uses `0.8_f32*v1`, `0.5_f32*a1`, cooldown30. A positive cooldown at entry excludes charging for the entire step, including a crossing to zero. No mul_add/FMA, f64 intermediate or decimal quantization is used. Urgency is supplied declaration only; it neither detects an active clause nor grants permission.

This is not owner-ratified shared numeric semantics or a serializable envelope. Named Studio kernel/native and ABI interface approvals remain pending. Native/WASM execution parity is a later packet, not established by native unit tests.

## Tests and meaningful TDD

The new 14-test target uses S24/D10 names, independently rounded literal decay bits, hand-derived exact charge/release values and complete transition state/elapsed/flag comparisons where appropriate. It catches omitted validation per field, signedzero admission/output, invalid elapsed, success/refusal immutability, dt0 charging, zero-intensity valence exclusion, urgency0.1, decay at180 seconds, capped3600 equality through f32::MAX, inclusive0.85 and adjacent f32 neighbors, clamping before release, cooldown30/10=>20, zero-duration cooldown, cooldown1/elapsed2 crossing followed by a separate charge, repeated buildup/release/idle, repeated identical scalar calls and absence of hidden jitter accumulation.

Admission TDD RED: three tests failed by assertions, not compile/setup errors, against deliberately permissive constructor scaffolding (accepted NaN and preserved negative zero). After validation/normalization implementation all three passed. Transition TDD RED: eleven transition tests failed by assertions against a no-op step while the three admission tests stayed green. After scalar implementation all14 passed. RED source snapshots are retained in `tdd-red-admission-source/` and `tdd-red-transition-source/`; each command receipt records before/after source hashes. These are historical failing snapshots, not the final implementation.

| Receipt | Exact command (from specimen-studio/) | Exit |
| --- | --- | --- |
| tdd-red-admission | `cargo test --locked --offline -p specimen-kernel --test atp_v3` | 101 |
| tdd-green-admission | `cargo test --locked --offline -p specimen-kernel --test atp_v3` | 0 |
| tdd-red-transition | `cargo test --locked --offline -p specimen-kernel --test atp_v3` | 101 |
| tdd-green-transition | `cargo test --locked --offline -p specimen-kernel --test atp_v3` | 0 |
| kernel-tests | `cargo test --locked --offline -p specimen-kernel` | 0 |
| kernel-clippy | `cargo clippy --locked --offline -p specimen-kernel --all-targets -- -D warnings` | 0 |
| kernel-format | `cargo fmt -p specimen-kernel -- --check` | 0 |
| private-format | `rustfmt --edition 2024 --check native/specimen-kernel/src/atp_v3/advance.rs native/specimen-kernel/src/atp_v3/mod.rs native/specimen-kernel/src/atp_v3/model.rs native/specimen-kernel/tests/atp_v3.rs` | 0 |

Kernel-wide result: **254 passed, 0 failed, 0 ignored**, across27 result summaries including doc-tests. The full raw output, not only a tail, is retained in `kernel-tests.log`. Kernel Clippy used `--all-targets -- -D warnings`. No warnings were emitted in the final kernel tests or Clippy. `kernel-format` checks the existing kernel module graph; `private-format` explicitly checks all new private include fragments and the integration test, which are not registered in lib.rs. Formatting application and its source hashes are separately retained in format-apply.json/log. All final gate before/after source hashes match.

Every Cargo command used `--locked --offline` where supported, stdin `/dev/null` for final checks, and the pinned toolchain environment below. Toolchain identity is recorded in toolchain.log/json: rustc1.99.1 (58cf8f9c3,2026-10-13), aarch64-apple-darwin, LLVM23.1.1.

```text
RUSTUP_TOOLCHAIN=1.99.1
PATH=/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:/usr/bin:/bin:/usr/sbin:/sbin
DYLD_LIBRARY_PATH=/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/lib
CARGO_NET_OFFLINE=true
```

## Preservation and source freeze

`preservation-before.json` and `preservation-after.json` contain all **118** expected and actual hashes from the supplied preservation-baseline.json: **zero mismatches before and after**. `git-tracked-before.diff` and `git-tracked-after.diff` are byte-identical, confirming the existing tracked edits were preserved. Both git status snapshots are retained. Baseline comparisons include prior mechanisms, legacy IDs/saves/goldens and locks; no expected values were regenerated. summary.json records counts and exact observed branch/HEAD.

Final source hashes and line counts:

| File | Lines | SHA-256 |
| --- | --- | --- |
| `specimen-studio/native/specimen-kernel/src/atp_v3/advance.rs` | 74 | `e95a1eccd90f88a791ebf3ced38b1532cf79e2afe62494b40f279d3743c3f4cd` |
| `specimen-studio/native/specimen-kernel/src/atp_v3/mod.rs` | 4 | `4d49058592d17757d83eba510f54a6753cad05b8eb4511e649914afdf900763b` |
| `specimen-studio/native/specimen-kernel/src/atp_v3/model.rs` | 105 | `dd49a2639fe6d1a3484e405adce4ffbd5fe175b795cd74f2f035a4c7251a23c8` |
| `specimen-studio/native/specimen-kernel/tests/atp_v3.rs` | 339 | `a875d3819af1ce122999f80cdef63fdfb991ae23ae2e7b0acc128f5292e4d395` |

`source-freeze/` contains exact copies of these four files. `source-freeze.json` pins their bytes, this report, and all packet receipts except itself. The live source and copies match the final kernel gate hashes. The report and source freeze conclude implementation ownership; further source changes require an explicit review fix. No independent review is claimed here: the controller owns that review after freeze.

## Remaining gates and limits

This packet establishes native scalar arithmetic and refusal mechanisms only. It does not close full S24 cycle/view integration, S25, CH14 or P08.05. It does not implement charge aggregation/active-clause eligibility, ephemeral tonal view, tone equivalence/retries, reward/autonomy, native/WASM comparison, host monotonic timing/cancellation, persisted ATP/save migration, scorer rational conversion, hardware or efficacy qualification. Pending named-owner interface approval and ABI approval are unchanged. The root gate and four Studio gates were intentionally not rerun here because the controller owns those later gates. No frozen claim was promoted.
