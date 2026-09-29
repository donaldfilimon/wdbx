# P0: Kernel Extraction and WASM Feasibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split the pure specimen rules out of `native/specimen-core` into a new `native/specimen-kernel` crate that builds and runs on `wasm32-unknown-unknown`, prove native and WASM produce byte-identical output on the starter specimen, and record the feasibility verdict.

**Architecture:** The kernel receives every impure service through a `Host` value: `Env` (wall clock, monotonic clock, IDs), `Search` (the raft scan, which uses threads natively), and an optional `Accelerator` (the `wgpu` layer path). `specimen-core` keeps its crate name, re-exports the moved modules so existing paths keep compiling, and supplies `NativeHost`. A tiny `specimen-wasm` cdylib exposes one raw-ABI probe that runs a full cycle; a Rust test and a Bun test both check its output against one committed golden digest.

**Tech Stack:** Rust edition 2024 on `nightly-2026-09-01` (from `rust-toolchain.toml`), target `wasm32-unknown-unknown` (already installed), Bun 1.4 test runner, existing crates only (`serde`, `serde_json`, `thiserror`, `sha2`, `regex`).

**Spec:** `docs/superpowers/specs/2026-09-28-specimen-studio-desktop-design.md` (sections 1 and 5; phase P0).

**Deferred from spec section 1:** `protocol.rs` stays in `specimen-core` for P0 because it imports `vision::{Analysis, Focus}`; it moves with the command layer in P1.

## Global Constraints

- All commands run from `specimen-studio/` (the directory holding `package.json` and the workspace `Cargo.toml`).
- Toolchain `nightly-2026-09-01`, edition 2024. `Cargo.lock` changes are committed; gates use `--locked`.
- `specimen-kernel` dependencies: `serde`, `serde_json`, `thiserror`, `sha2`, `regex` only. No `chrono`, `uuid`, `getrandom`, `wgpu`, `std::thread`, `std::time::Instant` anywhere in the kernel.
- The crate name `specimen-core` and the Tauri package `wdbx-studio-desktop` do not change.
- The CI test path `specimen-core neural::tests::cpu_gpu_parity` (`.github/workflows/specimen-studio-accelerator.yml`) must keep resolving.
- `lib/specimen/engine.ts` and the browser edition are untouched in P0.
- No behavior change on native: every existing test in `native/specimen-core/tests/conformance.rs` passes unchanged except for the added `host` argument.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. Transcendental math (`f32::exp` in `neural.rs`, `f64::exp` in `engine.rs`) may differ in the last bit between macOS libm and Rust's wasm libm. The golden-digest tests (Task 6) detect it; a mismatch is recorded in the verdict as a finding, never hidden by loosening the comparison.
2. `Sequential` search must reproduce raft semantics exactly: same indices in the same order, same error codes for chunk 0 and worker counts 0 or above 64, and `Cancelled` when the flag is set. Task 3 pins this with an equivalence test against `RaftSearch`.
3. `settings.gpu == true` with no accelerator available (FixedHost, WASM) must run on CPU, report `backend: "cpu"`, and not panic. Task 4 pins it.
4. Malformed JSON into the WASM probe must come back as `{"error":{"code":"MalformedSave",...}}`, not a trap. Task 6 pins it.
5. A `FixedEnv` run must be byte-deterministic across two calls in the same process (IDs restart per host). Task 5 pins it.

---

## File Structure

| Path | Action | Responsibility |
|---|---|---|
| `Cargo.toml` | modify | add workspace members |
| `native/specimen-kernel/Cargo.toml` | create | kernel manifest |
| `native/specimen-kernel/src/lib.rs` | create | `Error`, `Result`, `error`, `digest`, module list |
| `native/specimen-kernel/src/host.rs` | create | `Env`, `FixedEnv`, `Host`, `FixedHost` |
| `native/specimen-kernel/src/search.rs` | create | `Search` trait, `Sequential` |
| `native/specimen-kernel/src/language.rs` | move from core | unchanged logic |
| `native/specimen-kernel/src/neural.rs` | move from core | CPU path plus `Accelerator` trait |
| `native/specimen-kernel/src/engine.rs` | move from core | host-threaded engine |
| `native/specimen-core/src/lib.rs` | modify | re-exports, `SystemEnv`, `NativeHost` |
| `native/specimen-core/src/host.rs` | create | `SystemEnv`, `RaftSearch`, `NativeHost` |
| `native/specimen-core/src/neural.rs` | rewrite | re-export plus `WgpuAccelerator` and `cpu_gpu_parity` test |
| `native/specimen-wasm/Cargo.toml`, `src/lib.rs` | create | raw-ABI probe cdylib |
| `conformance/starter.json` | move from `native/specimen-core/tests/fixtures/` | shared corpus seed |
| `conformance/p0-probe.sha256` | create | golden digest |
| `tests/wasm-probe.test.js` | create | Bun parity test |
| `src-tauri/src/main.rs`, `native/specimen-core/tests/conformance.rs`, `native/specimen-core/examples/acceptance.rs`, `native/specimen-core/src/persistence.rs` | modify | pass a host / new paths |
| `package.json`, `.github/workflows/specimen-studio-desktop.yml`, `AGENTS.md`, `VALIDATION.md` | modify | gates and verdict |

---

### Task 1: Kernel crate with error type and host environment

**Files:**
- Create: `native/specimen-kernel/Cargo.toml`, `native/specimen-kernel/src/lib.rs`, `native/specimen-kernel/src/host.rs`
- Modify: `Cargo.toml` (workspace members), `native/specimen-core/Cargo.toml`, `native/specimen-core/src/lib.rs`

**Interfaces:**
- Produces: `specimen_kernel::{Error, Result, error, digest}`; `specimen_kernel::host::{Env, FixedEnv}`. `Env: Sync` with `now_rfc3339(&self) -> String`, `now_millis(&self) -> i64`, `uid(&self) -> String`, `monotonic_ms(&self) -> f64`. `FixedEnv::new()` returns millis `1767225600000`, ISO `2026-01-01T00:00:00.000Z`, monotonic `0.0`, IDs `00000000-0000-4000-8000-000000000001`, `...0002`, and so on.
- `specimen_core::{Error, Result, error, digest}` become re-exports; `specimen_core::{now, uid}` stay as they are.

- [ ] **Step 1: Write the failing test** in `native/specimen-kernel/src/host.rs`

```rust
use std::sync::atomic::{AtomicU64, Ordering};

/// Every impure value the kernel needs comes from here.
pub trait Env: Sync {
    fn now_rfc3339(&self) -> String;
    fn now_millis(&self) -> i64;
    fn uid(&self) -> String;
    /// Milliseconds on a monotonic clock; only differences are meaningful.
    fn monotonic_ms(&self) -> f64;
}

/// Deterministic environment for tests and conformance.
#[derive(Debug, Default)]
pub struct FixedEnv {
    next: AtomicU64,
}

impl FixedEnv {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Env for FixedEnv {
    fn now_rfc3339(&self) -> String {
        todo!()
    }
    fn now_millis(&self) -> i64 {
        todo!()
    }
    fn uid(&self) -> String {
        todo!()
    }
    fn monotonic_ms(&self) -> f64 {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_env_is_deterministic() {
        let a = FixedEnv::new();
        let b = FixedEnv::new();
        assert_eq!(a.now_millis(), 1_767_225_600_000);
        assert_eq!(a.now_rfc3339(), "2026-01-01T00:00:00.000Z");
        assert_eq!(a.monotonic_ms(), 0.0);
        assert_eq!(a.uid(), "00000000-0000-4000-8000-000000000001");
        assert_eq!(a.uid(), "00000000-0000-4000-8000-000000000002");
        assert_eq!(b.uid(), "00000000-0000-4000-8000-000000000001");
    }
}
```

`native/specimen-kernel/src/lib.rs` (the four helpers are moved verbatim from `native/specimen-core/src/lib.rs` lines 9-26 and 40-43):

```rust
pub mod host;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct Error {
    pub code: String,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn error(code: &str, message: impl ToString) -> Error {
    Error {
        code: code.into(),
        message: message.to_string(),
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        error("Storage", e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        error("MalformedSave", e)
    }
}
pub fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
```

`native/specimen-kernel/Cargo.toml`:

```toml
[package]
name = "specimen-kernel"
version = "0.2.0"
edition = "2024"
[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
sha2.workspace = true
regex = "1"
```

Workspace `Cargo.toml`: `members = ["native/specimen-kernel", "native/specimen-core", "src-tauri"]`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p specimen-kernel host::tests::fixed_env_is_deterministic`
Expected: FAIL, panic `not yet implemented`.

- [ ] **Step 3: Implement**

```rust
impl Env for FixedEnv {
    fn now_rfc3339(&self) -> String {
        "2026-01-01T00:00:00.000Z".into()
    }
    fn now_millis(&self) -> i64 {
        1_767_225_600_000
    }
    fn uid(&self) -> String {
        let n = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        format!("00000000-0000-4000-8000-{n:012x}")
    }
    fn monotonic_ms(&self) -> f64 {
        0.0
    }
}
```

In `native/specimen-core/Cargo.toml` add `specimen-kernel = { path = "../specimen-kernel" }`. In `native/specimen-core/src/lib.rs` delete the `Error`, `Result`, `error`, the two `From` impls and `digest`, and add `pub use specimen_kernel::{Error, Result, digest, error};`. Keep `now()` and `uid()`.

- [ ] **Step 4: Run tests**

Run: `cargo test -p specimen-kernel && cargo test -p specimen-core --locked` (drop `--locked` for this first run only if Cargo reports the lock needs updating, then rerun with it)
Expected: PASS for both.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock native/specimen-kernel native/specimen-core/Cargo.toml native/specimen-core/src/lib.rs
git commit -m "feat(specimen-studio): add specimen-kernel crate with Env seam"
```

### Task 2: Move `language` into the kernel and prove the wasm32 build

**Files:**
- Move: `native/specimen-core/src/language.rs` to `native/specimen-kernel/src/language.rs`
- Modify: both `lib.rs` files

**Interfaces:**
- Produces: `specimen_kernel::language` (same public items as before); `specimen_core::language` is `pub use specimen_kernel::language;`.

- [ ] **Step 1: Move the file**

```bash
git mv native/specimen-core/src/language.rs native/specimen-kernel/src/language.rs
```

In the kernel `lib.rs` add `pub mod language;`. In the core `lib.rs` replace `pub mod language;` with `pub use specimen_kernel::language;`. `language.rs` uses only `crate::{Result, error, digest}` and `regex`, which the kernel provides.

- [ ] **Step 2: Run the wasm32 check**

Run: `cargo build -p specimen-kernel --target wasm32-unknown-unknown`
Expected: builds. If `regex` fails to build for wasm32, stop and record the failure as the P0 verdict (Task 7).

- [ ] **Step 3: Run tests**

Run: `cargo test -p specimen-kernel && cargo test -p specimen-core --locked`
Expected: PASS; the `language` unit tests now run under `specimen-kernel`.

- [ ] **Step 4: Commit**

```bash
git add -A native/specimen-kernel native/specimen-core/src
git commit -m "refactor(specimen-studio): move language into specimen-kernel"
```

### Task 3: `Search` seam with a sequential kernel implementation

**Files:**
- Create: `native/specimen-kernel/src/search.rs`, `native/specimen-core/src/host.rs`
- Modify: both `lib.rs` files

**Interfaces:**
- Produces: `specimen_kernel::search::{Search, Sequential}`:

```rust
pub trait Search: Sync {
    fn find_all(
        &self,
        len: usize,
        predicate: &(dyn Fn(usize) -> bool + Sync),
        chunk: usize,
        workers: usize,
        cancel: &AtomicBool,
        progress: &dyn Fn(usize, usize),
    ) -> Result<Vec<usize>>;
}
```

- Produces: `specimen_core::host::RaftSearch` implementing `Search` via `scheduler::raft_find_all_progress`.

- [ ] **Step 1: Write the failing equivalence test** in `native/specimen-core/src/host.rs`

```rust
use crate::{Result, scheduler};
use specimen_kernel::search::Search;
use std::sync::atomic::AtomicBool;

/// The threaded raft scan behind the kernel's `Search` seam.
pub struct RaftSearch;

impl Search for RaftSearch {
    fn find_all(
        &self,
        len: usize,
        predicate: &(dyn Fn(usize) -> bool + Sync),
        chunk: usize,
        workers: usize,
        cancel: &AtomicBool,
        progress: &dyn Fn(usize, usize),
    ) -> Result<Vec<usize>> {
        let items: Vec<usize> = (0..len).collect();
        scheduler::raft_find_all_progress(&items, |i| predicate(*i), chunk, workers, cancel, progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use specimen_kernel::search::Sequential;
    use std::sync::Mutex;

    fn run(s: &dyn Search, len: usize, chunk: usize, workers: usize, cancel: bool)
        -> (Result<Vec<usize>>, Vec<(usize, usize)>) {
        let seen = Mutex::new(Vec::new());
        let flag = AtomicBool::new(cancel);
        let out = s.find_all(len, &|i| i % 3 == 0 || i % 7 == 2, chunk, workers, &flag,
            &|a, b| seen.lock().unwrap().push((a, b)));
        (out, seen.into_inner().unwrap())
    }

    #[test]
    fn sequential_matches_raft() {
        for (len, chunk, workers) in [(0, 1, 1), (1, 1, 1), (10_000, 256, 8), (9_999, 5000, 3), (17, 4, 64)] {
            let (a, pa) = run(&RaftSearch, len, chunk, workers, false);
            let (b, pb) = run(&Sequential, len, chunk, workers, false);
            assert_eq!(a.unwrap(), b.unwrap(), "len={len} chunk={chunk}");
            assert_eq!(pa, pb, "progress len={len} chunk={chunk}");
        }
    }

    #[test]
    fn sequential_matches_raft_errors() {
        for (chunk, workers, cancel) in [(0, 1, false), (8, 0, false), (8, 65, false), (8, 4, true)] {
            let (a, _) = run(&RaftSearch, 100, chunk, workers, cancel);
            let (b, _) = run(&Sequential, 100, chunk, workers, cancel);
            assert_eq!(a.unwrap_err().code, b.unwrap_err().code,
                "chunk={chunk} workers={workers} cancel={cancel}");
        }
    }
}
```

Kernel `search.rs` stub:

```rust
use crate::Result;
use std::sync::atomic::AtomicBool;
pub trait Search: Sync { /* signature above */ }
/// Single-threaded search with the raft's chunking, validation and progress.
pub struct Sequential;
impl Search for Sequential {
    fn find_all(&self, _len: usize, _predicate: &(dyn Fn(usize) -> bool + Sync), _chunk: usize,
        _workers: usize, _cancel: &AtomicBool, _progress: &dyn Fn(usize, usize)) -> Result<Vec<usize>> {
        todo!()
    }
}
```

Add `pub mod search;` to the kernel `lib.rs` and `pub mod host;` to the core `lib.rs`.

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p specimen-core host::tests`
Expected: FAIL, panic `not yet implemented`.

- [ ] **Step 3: Implement `Sequential`**

It mirrors `RaftCursor::new` and `RaftCursor::advance` in `native/specimen-core/src/scheduler.rs` (lines ~200-275) without threads or the comparison permit:

```rust
use crate::{Result, error};
use std::sync::atomic::{AtomicBool, Ordering};

impl Search for Sequential {
    fn find_all(&self, len: usize, predicate: &(dyn Fn(usize) -> bool + Sync), chunk: usize,
        workers: usize, cancel: &AtomicBool, progress: &dyn Fn(usize, usize)) -> Result<Vec<usize>> {
        if chunk == 0 {
            return Err(error("BudgetExceeded", "Invalid raft chunk size"));
        }
        let chunk = chunk.min(4096);
        let mut next = 0;
        let mut found = Vec::new();
        loop {
            if workers == 0 || workers > 64 {
                return Err(error("BudgetExceeded", "Invalid raft worker count"));
            }
            if cancel.load(Ordering::Relaxed) {
                return Err(error("Cancelled", "Raft cancelled"));
            }
            if next == len {
                progress(next, len);
                return Ok(found);
            }
            let end = (next + chunk).min(len);
            found.extend((next..end).filter(|&i| predicate(i)));
            next = end;
            if cancel.load(Ordering::Relaxed) {
                return Err(error("Cancelled", "Raft cancelled"));
            }
            progress(next, len);
            if next == len {
                return Ok(found);
            }
        }
    }
}
```

Before relying on this, read `raft_find_all_progress` and `RaftCursor::advance` once more and confirm the order of checks (worker validation, cancellation, the `next == total` early return, and when `progress` fires). If the test disagrees, the raft is the reference: change `Sequential`, never the raft.

- [ ] **Step 4: Run tests**

Run: `cargo test -p specimen-core host::tests && cargo test -p specimen-kernel`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add native/specimen-kernel/src native/specimen-core/src
git commit -m "feat(specimen-studio): add Search seam with sequential kernel scan"
```

### Task 4: Move `neural` into the kernel behind an `Accelerator` trait

**Files:**
- Move: `native/specimen-core/src/neural.rs` to `native/specimen-kernel/src/neural.rs`
- Create: `native/specimen-core/src/neural.rs` (new, small)

**Interfaces:**
- Produces: `specimen_kernel::neural::{Layer, Network, Synthesis, Rng, encode, Accelerator}` with

```rust
pub trait Accelerator: Sync {
    /// Short backend name reported in `Synthesis::backend`, e.g. "wgpu".
    fn name(&self) -> &'static str;
    fn layer(&self, layer: &Layer, input: &[f32]) -> Result<Vec<f32>>;
}
impl Network {
    pub fn run(&self, input: &[f32], familiar: bool, seed: u64,
        accel: Option<&dyn Accelerator>) -> Result<Synthesis>;
}
```

- Produces: `specimen_core::neural::WgpuAccelerator` (unit struct implementing `Accelerator` with the former `gpu_layer` body); `specimen_core::neural` re-exports every kernel item so `specimen_core::neural::Network` still resolves.

- [ ] **Step 1: Move and write the failing test**

```bash
git mv native/specimen-core/src/neural.rs native/specimen-kernel/src/neural.rs
```

In the kernel `neural.rs`, replace the old `tests` module with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_and_validated() {
        let n = Network::default();
        let input = encode("red circle", &[], 0.0, 0.2);
        assert_eq!(
            n.run(&input, false, 9, None).unwrap().values,
            n.run(&input, false, 9, None).unwrap().values
        );
        let mut broken = n.clone();
        broken.layers[0].columns[0] = 200;
        assert!(broken.validate().is_err());
    }
    struct Failing;
    impl Accelerator for Failing {
        fn name(&self) -> &'static str { "failing" }
        fn layer(&self, _: &Layer, _: &[f32]) -> Result<Vec<f32>> {
            Err(crate::error("Accelerator", "unavailable"))
        }
    }
    #[test]
    fn missing_or_failing_accelerator_falls_back_to_cpu() {
        let n = Network::default();
        let input = encode("red circle", &[], 0.0, 0.2);
        let cpu = n.run(&input, false, 9, None).unwrap();
        assert_eq!(cpu.backend, "cpu");
        let fell = n.run(&input, false, 9, Some(&Failing)).unwrap();
        assert_eq!(fell.backend, "cpu");
        assert!(fell.fallback.is_some());
        assert_eq!(fell.values, cpu.values);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p specimen-kernel neural`
Expected: FAIL to compile (`run` still takes `bool`; `gpu_layer` references `wgpu`).

- [ ] **Step 3: Implement**

In the kernel `neural.rs`:
- Delete `fn gpu_layer` (lines ~168-285 of the old file) and cut its body into the core file below.
- Add the `Accelerator` trait above.
- In `run`, change the `gpu: bool` parameter to `accel: Option<&dyn Accelerator>`. Where the old code did `let mut used_gpu = gpu;` and called `gpu_layer(l, &values)`, use `let mut used = accel;` and call `a.layer(l, &values)` when `used` is `Some(a)`. On error, set `used = None` and record the fallback exactly as the old code did. Report `backend` as `used.map_or("cpu", |a| a.name())`.

New `native/specimen-core/src/neural.rs`:

```rust
pub use specimen_kernel::neural::*;
use crate::{Result, error};

/// GPU layer evaluation through wgpu; the kernel falls back to CPU on error.
pub struct WgpuAccelerator;

impl Accelerator for WgpuAccelerator {
    fn name(&self) -> &'static str {
        "wgpu"
    }
    fn layer(&self, l: &Layer, input: &[f32]) -> Result<Vec<f32>> {
        // Body of the former `fn gpu_layer(l: &Layer, input: &[f32])`, verbatim.
        // It already begins with `pollster::block_on(async { ... })`.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires qualified GPU"]
    fn cpu_gpu_parity() {
        let n = Network::default();
        let input = encode("red circle", &[], 0.0, 0.2);
        let cpu = n.run(&input, false, 9, None).unwrap();
        let gpu = n.run(&input, false, 9, Some(&WgpuAccelerator)).unwrap();
        assert_eq!(gpu.backend, "wgpu");
        let mut max_delta = 0.0_f32;
        for (index, (a, b)) in cpu.values.iter().zip(&gpu.values).enumerate() {
            let delta = (a - b).abs();
            println!("output[{index}] delta={delta:.8}");
            assert!(delta < 1e-4, "output[{index}] delta {delta} exceeded 1e-4");
            max_delta = max_delta.max(delta);
        }
        println!("backend={} max_delta={max_delta:.8} threshold=0.0001", gpu.backend);
    }
}
```

(The comment inside `layer` is an instruction to paste the moved body; the pasted code replaces it. `error` is imported because that body uses it.) Add `pub mod neural;` to the kernel `lib.rs`. `engine.rs` still calls `network.run(..., bool)` and will not compile; fix it in this step with the smallest bridge: at the call site (old `engine.rs` line ~444-448) pass `if ctx.state["settings"]["gpu"] == true { Some(&crate::neural::WgpuAccelerator as &dyn crate::neural::Accelerator) } else { None }`. Task 5 replaces this with the host.

- [ ] **Step 4: Run tests**

Run: `cargo test -p specimen-kernel neural && cargo test -p specimen-core --locked && cargo build -p specimen-kernel --target wasm32-unknown-unknown && cargo test -p specimen-core neural::tests::cpu_gpu_parity -- --list --ignored`
Expected: PASS; the last command lists `neural::tests::cpu_gpu_parity`.

- [ ] **Step 5: Commit**

```bash
git add -A native/specimen-kernel/src native/specimen-core/src
git commit -m "refactor(specimen-studio): move neural into kernel behind Accelerator"
```

### Task 5: Move `engine` into the kernel and thread `Host`

**Files:**
- Move: `native/specimen-core/src/engine.rs` to `native/specimen-kernel/src/engine.rs`
- Modify: `native/specimen-kernel/src/host.rs`, `native/specimen-core/src/host.rs`, `native/specimen-core/src/lib.rs`, `native/specimen-core/src/persistence.rs`, `src-tauri/src/main.rs`, `native/specimen-core/tests/conformance.rs`, `native/specimen-core/examples/acceptance.rs`
- Move: `native/specimen-core/tests/fixtures/starter.json` to `conformance/starter.json`

**Interfaces:**
- Produces in `specimen_kernel::host`:

```rust
pub struct Host<'a> {
    pub env: &'a dyn Env,
    pub search: &'a dyn Search,
    pub accel: Option<&'a dyn Accelerator>,
}
/// Owned deterministic host: FixedEnv, Sequential search, no accelerator.
#[derive(Default)]
pub struct FixedHost { pub env: FixedEnv }
impl FixedHost { pub fn host(&self) -> Host<'_>; }
```

- Produces in `specimen_core::host`: `SystemEnv` (wall clock via `chrono`, IDs via `uuid` v4, monotonic via an `Instant` origin) and

```rust
pub struct NativeHost { pub env: SystemEnv }
impl NativeHost { pub fn new() -> Self; pub fn host(&self) -> Host<'_>; } // RaftSearch + WgpuAccelerator
```

- Changes signatures to: `engine::cycle(host: &Host, source, input, network, cancel, progress)`, `engine::cycle_with_visual(host: &Host, ...)`, `engine::review(host: &Host, source, network, cancel)`, `engine::maintain(host: &Host, source, mode, cancel)`. `validate`, `rows`, `text`, `number`, `trim` are unchanged.

- [ ] **Step 1: Write the failing determinism test** at the end of the kernel `engine.rs` (after the move below)

```rust
#[cfg(test)]
mod host_tests {
    use super::*;
    use crate::host::FixedHost;
    use std::sync::atomic::AtomicBool;
    fn starter() -> Value {
        serde_json::from_str(include_str!("../../../conformance/starter.json")).unwrap()
    }
    fn run_once() -> String {
        let fixed = FixedHost::default();
        let (state, cycle) = cycle(&fixed.host(), &starter(), "What is 2 + 2?",
            &Network::default(), &AtomicBool::new(false), &|_| {}).unwrap();
        serde_json::to_string(&(state, cycle)).unwrap()
    }
    #[test]
    fn fixed_host_cycle_is_byte_deterministic() {
        assert_eq!(run_once(), run_once());
    }
    #[test]
    fn gpu_setting_without_accelerator_runs_on_cpu() {
        let mut s = starter();
        s["settings"]["gpu"] = serde_json::json!(true);
        let fixed = FixedHost::default();
        assert!(cycle(&fixed.host(), &s, "What is 2 + 2?", &Network::default(),
            &AtomicBool::new(false), &|_| {}).is_ok());
    }
}
```

- [ ] **Step 2: Move files and run to verify failure**

```bash
mkdir -p conformance
git mv native/specimen-core/tests/fixtures/starter.json conformance/starter.json
git mv native/specimen-core/src/engine.rs native/specimen-kernel/src/engine.rs
```

Add `pub mod engine;` to the kernel `lib.rs`; in the core `lib.rs` replace `pub mod engine;` with `pub use specimen_kernel::engine;`. In `conformance.rs` change `include_str!("fixtures/starter.json")` to `include_str!("../../../conformance/starter.json")`.

Run: `cargo test -p specimen-kernel host_tests`
Expected: FAIL to compile (`crate::uid`, `crate::now`, `chrono`, `Instant`, `crate::scheduler` are unresolved in the kernel).

- [ ] **Step 3: Implement**

Add `Host` and `FixedHost` to the kernel `host.rs`:

```rust
use crate::{neural::Accelerator, search::{Search, Sequential}};
pub struct Host<'a> {
    pub env: &'a dyn Env,
    pub search: &'a dyn Search,
    pub accel: Option<&'a dyn Accelerator>,
}
#[derive(Default)]
pub struct FixedHost {
    pub env: FixedEnv,
}
impl FixedHost {
    pub fn host(&self) -> Host<'_> {
        Host { env: &self.env, search: &Sequential, accel: None }
    }
}
```

Edit the kernel `engine.rs`, letting the compiler list each remaining site:
1. Imports: drop `time::Instant`; import `crate::host::Host`.
2. `struct Context<'a>` (line ~290): add `host: &'a Host<'a>,` and set it where the context is built.
3. `crate::now()` becomes `host.env.now_rfc3339()` (or `ctx.host.env...` inside `eval`); `crate::uid()` becomes `host.env.uid()`.
4. `fn append_event(s, kind, title, detail)` gains a first parameter `env: &dyn Env`; update its callers.
5. `let start = Instant::now();` becomes `let start = host.env.monotonic_ms();`, and `start.elapsed().as_secs_f64()*1000.` becomes `host.env.monotonic_ms() - start`.
6. `chrono::Utc::now().timestamp_millis()` (lines ~920 and ~1000) becomes `host.env.now_millis()`.
7. The `network.run` call replaces the Task 4 bridge with `if ctx.state["settings"]["gpu"] == true { ctx.host.accel } else { None }`.
8. `crate::scheduler::raft_find_all_progress(&jobs, |job| PRED, chunk, workers, cancel, &PROGRESS)` becomes `host.search.find_all(jobs.len(), &|i| { let job = &jobs[i]; PRED }, chunk, workers, cancel, &PROGRESS)`, with `PRED` and `PROGRESS` unchanged.
9. Add `host: &Host` as the first parameter of `cycle`, `cycle_with_visual`, `review` and `maintain`, and pass it through.

Add `SystemEnv` and `NativeHost` to the core `host.rs`:

```rust
use specimen_kernel::host::{Env, Host};
use std::time::Instant;
pub struct SystemEnv { origin: Instant }
impl Default for SystemEnv {
    fn default() -> Self { Self { origin: Instant::now() } }
}
impl Env for SystemEnv {
    fn now_rfc3339(&self) -> String { crate::now() }
    fn now_millis(&self) -> i64 { chrono::Utc::now().timestamp_millis() }
    fn uid(&self) -> String { crate::uid() }
    fn monotonic_ms(&self) -> f64 { self.origin.elapsed().as_secs_f64() * 1000.0 }
}
#[derive(Default)]
pub struct NativeHost { pub env: SystemEnv }
impl NativeHost {
    pub fn new() -> Self { Self::default() }
    pub fn host(&self) -> Host<'_> {
        Host { env: &self.env, search: &RaftSearch, accel: Some(&crate::neural::WgpuAccelerator) }
    }
}
```

Update callers to build `let native = specimen_core::host::NativeHost::new();` and pass `&native.host()` as the first argument: `src-tauri/src/main.rs` (lines ~74, 84, 86, 91, 196), every `engine::cycle|cycle_with_visual|review|maintain` call in `conformance.rs`, and `examples/acceptance.rs`. `persistence.rs` only uses pure helpers and needs no change beyond compiling.

- [ ] **Step 4: Run tests**

Run: `cargo test -p specimen-kernel && cargo test -p specimen-core --locked && cargo test -p wdbx-studio-desktop --locked && cargo build -p specimen-kernel --target wasm32-unknown-unknown && grep -rnE 'Instant|chrono|uuid|std::thread|wgpu' native/specimen-kernel/src; echo "grep exit $?"`
Expected: all tests PASS; the wasm32 build succeeds; the grep prints nothing and `grep exit 1`.

- [ ] **Step 5: Commit**

```bash
git add -A conformance native src-tauri/src
git commit -m "refactor(specimen-studio): move engine into kernel and inject Host"
```

### Task 6: WASM probe and cross-target golden digest

**Files:**
- Create: `native/specimen-wasm/Cargo.toml`, `native/specimen-wasm/src/lib.rs`, `conformance/p0-probe.sha256`, `tests/wasm-probe.test.js`
- Modify: `Cargo.toml` (members), `package.json` (scripts)

**Interfaces:**
- Consumes: `engine::cycle`, `FixedHost`, `Network::default()`, `digest`.
- Produces: `specimen_wasm::probe(input: &str) -> String`. Input `{"specimen": <starter>, "input": "<prompt>"}`; output `{"specimen":..., "cycle":...}` or `{"error":{"code","message"}}`. Raw exports `alloc(len) -> *mut u8`, `probe_raw(ptr, len) -> u64` (high 32 bits pointer, low 32 bits length). `probe_raw` takes ownership of the input buffer; the output buffer is leaked (a probe, replaced by `wasm-bindgen` in P1).
- Golden: `conformance/p0-probe.sha256` holds the SHA-256 hex digest of `probe(<starter, "What is 2 + 2?">)`.

- [ ] **Step 1: Write the probe with failing tests**

`native/specimen-wasm/Cargo.toml`:

```toml
[package]
name = "specimen-wasm"
version = "0.2.0"
edition = "2024"
[lib]
crate-type = ["cdylib", "rlib"]
[dependencies]
specimen-kernel = { path = "../specimen-kernel" }
serde_json.workspace = true
```

`native/specimen-wasm/src/lib.rs`:

```rust
//! P0 feasibility probe: one full specimen cycle over a raw ABI.
//! Replaced by a wasm-bindgen command surface in P1.
use serde_json::{Value, json};
use specimen_kernel::{engine, host::FixedHost, neural::Network};
use std::sync::atomic::AtomicBool;

pub fn probe(input: &str) -> String {
    let run = || -> specimen_kernel::Result<Value> {
        let request: Value = serde_json::from_str(input)?;
        let fixed = FixedHost::default();
        let (specimen, cycle) = engine::cycle(&fixed.host(), &request["specimen"],
            request["input"].as_str().unwrap_or_default(), &Network::default(),
            &AtomicBool::new(false), &|_| {})?;
        Ok(json!({ "specimen": specimen, "cycle": cycle }))
    };
    match run() {
        Ok(v) => v.to_string(),
        Err(e) => json!({ "error": { "code": e.code, "message": e.message } }).to_string(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// # Safety
/// `ptr` must come from `alloc(len)` and hold `len` bytes of UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn probe_raw(ptr: *mut u8, len: usize) -> u64 {
    let bytes = unsafe { Vec::from_raw_parts(ptr, len, len) };
    let out = probe(&String::from_utf8_lossy(&bytes)).into_bytes().into_boxed_slice();
    let (p, n) = (out.as_ptr() as u64, out.len() as u64);
    std::mem::forget(out);
    (p << 32) | n
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> String {
        let starter: Value =
            serde_json::from_str(include_str!("../../../conformance/starter.json")).unwrap();
        json!({ "specimen": starter, "input": "What is 2 + 2?" }).to_string()
    }
    #[test]
    fn native_probe_matches_golden() {
        let got = specimen_kernel::digest(probe(&request()).as_bytes());
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/p0-probe.sha256");
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::write(path, format!("{got}\n")).unwrap();
        }
        assert_eq!(got, std::fs::read_to_string(path).unwrap().trim());
    }
    #[test]
    fn malformed_input_is_an_error_not_a_panic() {
        let out: Value = serde_json::from_str(&probe("{not json")).unwrap();
        assert_eq!(out["error"]["code"], "MalformedSave");
    }
}
```

`tests/wasm-probe.test.js`:

```js
import { expect, test } from 'bun:test';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const root = new URL('..', import.meta.url);
const wasmPath = new URL('target/wasm32-unknown-unknown/release/specimen_wasm.wasm', root);

async function probe(text) {
  const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
  const { memory, alloc, probe_raw } = instance.exports;
  const input = new TextEncoder().encode(text);
  const ptr = alloc(input.length);
  new Uint8Array(memory.buffer, ptr, input.length).set(input);
  const packed = probe_raw(ptr, input.length);
  const outPtr = Number(packed >> 32n);
  const outLen = Number(packed & 0xffffffffn);
  return new TextDecoder().decode(new Uint8Array(memory.buffer, outPtr, outLen));
}

test('wasm probe reproduces the native golden digest', async () => {
  const starter = JSON.parse(readFileSync(new URL('conformance/starter.json', root), 'utf8'));
  const out = await probe(JSON.stringify({ specimen: starter, input: 'What is 2 + 2?' }));
  const golden = readFileSync(new URL('conformance/p0-probe.sha256', root), 'utf8').trim();
  expect(createHash('sha256').update(out).digest('hex')).toBe(golden);
});

test('wasm probe reports malformed input as an error', async () => {
  const out = JSON.parse(await probe('{not json'));
  expect(out.error.code).toBe('MalformedSave');
});
```

Add `"native/specimen-wasm"` to workspace members. In `package.json` add `"build:wasm": "cargo build --locked -p specimen-wasm --target wasm32-unknown-unknown --release"` and change `check` to run `bun run build:wasm` before `bun test`.

Before relying on the JS side, confirm the input serialization matches: `JSON.stringify` of the parsed starter must produce the same request semantics as Rust's `json!`. Both feed `serde_json::from_str`, so key order and whitespace of the *request* do not matter; only the probe's *output* is hashed, and it is produced by Rust on both targets.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p specimen-wasm`
Expected: FAIL (`conformance/p0-probe.sha256` missing).

- [ ] **Step 3: Create the golden from the native run**

Run: `UPDATE_GOLDEN=1 cargo test -p specimen-wasm native_probe_matches_golden && cargo test -p specimen-wasm`
Expected: PASS; `conformance/p0-probe.sha256` holds one 64-hex line.

- [ ] **Step 4: Run the WASM side**

Run: `bun run build:wasm && bun test tests/wasm-probe.test.js`
Expected: both tests PASS. If the digest differs, do NOT regenerate the golden from WASM. Diff the two outputs (write the WASM output to a scratch file, `jq -S` both, `diff`), identify the first differing value, and record it in Task 7's verdict. A last-bit float difference from `exp` is the expected suspect (Review Focus 1).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock native/specimen-wasm conformance/p0-probe.sha256 tests/wasm-probe.test.js package.json
git commit -m "test(specimen-studio): cross-target WASM probe with golden digest"
```

### Task 7: Gates, documentation and the feasibility verdict

**Files:**
- Modify: `package.json`, `.github/workflows/specimen-studio-desktop.yml`, `AGENTS.md`, `VALIDATION.md`

- [ ] **Step 1: Wire the gates**

`package.json`: `"test:native": "cargo test --locked -p specimen-kernel -p specimen-core -p specimen-wasm"`. `.github/workflows/specimen-studio-desktop.yml` line ~78: change `cargo test -p specimen-core --locked` to `cargo test --locked -p specimen-kernel -p specimen-core -p specimen-wasm`, and add a step after it: `rustup target add wasm32-unknown-unknown && cargo build --locked -p specimen-wasm --target wasm32-unknown-unknown --release`.

- [ ] **Step 2: Measure**

Run: `command ls -l target/wasm32-unknown-unknown/release/specimen_wasm.wasm && gzip -9 -c target/wasm32-unknown-unknown/release/specimen_wasm.wasm | wc -c && /usr/bin/time -p cargo build --locked -p specimen-wasm --target wasm32-unknown-unknown --release`
Record raw bytes, gzip bytes and build time (after `cargo clean -p specimen-wasm -p specimen-kernel --target wasm32-unknown-unknown` for a cold number).

- [ ] **Step 3: Write the verdict and update AGENTS.md**

Append to `VALIDATION.md` a section `## P0 kernel WASM feasibility (<date>)` stating: the verdict (GO if the Bun digest test passes, otherwise NO-GO or GO-with-finding with the first differing value), the three measurements, the commands run and their results, and the named exception (vision and models stay native).

In `AGENTS.md`, under *Project Structure*, describe the three crates (`native/specimen-kernel` pure rules with `Host` seams; `native/specimen-core` native host; `native/specimen-wasm` probe) and `conformance/`. Under *Build, Test*, replace the native test commands with `bun run test:native`, mention `bun run build:wasm`, and restate that the report must name both halves (`bun run check` and `bun run test:native`).

- [ ] **Step 4: Run both full gates**

Run: `bun run check >| /private/tmp/claude-501/p0-check.log 2>&1; echo "check exit $?"; bun run test:native >| /private/tmp/claude-501/p0-native.log 2>&1; echo "native exit $?"; bun run check:native >| /private/tmp/claude-501/p0-clippy.log 2>&1; echo "clippy exit $?"`
Expected: three `exit 0` lines. Read each log's tail to confirm (never infer green from a pipe).

- [ ] **Step 5: Commit**

```bash
git add package.json .github/workflows/specimen-studio-desktop.yml AGENTS.md VALIDATION.md
git commit -m "docs(specimen-studio): P0 gates and WASM feasibility verdict"
```
