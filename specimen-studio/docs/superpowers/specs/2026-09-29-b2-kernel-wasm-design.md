# B2: Kernel parity and the WASM browser engine

Date: 2026-09-29. Program: `~/.claude/plans/continue-with-all-superpowers-brainstorm-serene-parnas.md`
(B2). Decisions by Donald: the browser moves onto the Rust kernel now, and
**Rust/native semantics win** where the engines disagree.

## Problem

The browser runs `lib/specimen/engine.ts` (1,640 lines). Desktop runs the Rust
kernel only for cycle, review and maintenance; every other change (seed, add
node/entry, remove node, save resource, feedback, pin, log, settings checks)
is TypeScript on both editions. The engines disagree on pattern IDs
(`low:arithmetic` vs `text-v2:low:<sha>`), RNG (32-bit vs 64-bit xorshift),
Type A review (proposal vs recorded correction) and settings bounds.

## Design

1. **Kernel mutations** (`native/specimen-kernel/src/mutate.rs`, split into a
   directory if it passes 1,000 lines): `seed`, `add_node`, `add_entry`,
   `remove_node`, `save_resource`, `feedback`, `toggle_pin`, `log`,
   `validate_settings`, `migrate_ids`. Behavior and user-facing messages follow
   `engine.ts`; pattern IDs come from `language::identify` (native `text-v2`);
   feedback coin flips use `neural::Rng` seeded from `settings.seed`, written
   back; settings bounds are the native ones in `engine::validate`, plus the
   integer/boolean type checks TS had. Timestamps and IDs come from `host::Env`.
2. **One command ABI** (`native/specimen-wasm`): `alloc`, `free`, and
   `call(ptr, len) -> packed(ptr, len)` taking `{"op": …, …}` JSON and returning
   `{"ok": value}` or `{"error": {"code", "message"}}`. Ops: every mutation
   above, `validate`, `cycle`, `review`, `maintain`. The host is a `WasmHost`
   whose `Env` reads the clock and random IDs through imported JS functions
   (`env.now_ms`, `env.random_u32`); `Search` is `Sequential`; no accelerator.
   `cycle` streams trace steps through an imported `env.progress(ptr, len)`.
   No wasm-bindgen toolchain: the ABI is small and auditable.
3. **Browser facade** `lib/specimen/kernel.ts`: `initKernel(bytes|url)` once,
   then synchronous functions with today's names and shapes (`seedSpecimen`,
   `addNode`, `addEntry`, `removeNode`, `saveResource`, `feedback`,
   `togglePin`, `log`, `validateSettings`, `validateSpecimen`, `maintenance`,
   `migrateIds`), plus async `runCycle(state, input, onTrace, signal)` and
   `observeContext(state, signal)` that run in a module Web Worker
   (`kernel-worker.ts`, same WASM). Both keep a cooperative checkpoint (a
   `setTimeout(0)` yield and an abort check before dispatch; the caller's
   existing stale-snapshot check after), which the accessibility suite's
   Processing / Cancelled / Failed sequence relies on. Cancellation terminates
   and respawns the worker. Desktop keeps `runNative`/`nativeReview` for
   cycle/review/maintenance (network, GPU, threads) and uses the facade for
   every mutation, so both editions share one mutation path.
4. **Validation at the boundary**: `validateSpecimen` keeps the structural
   checks in `contracts.ts` and the friendly schema message, then calls the
   kernel `validate` (native bounds, action parsing).
5. **Migration**: on load, `migrateIds` recomputes node and resource pattern
   IDs natively and remaps `resourceId` links; desktop already does this.
6. **Loading**: `build:wasm` builds and copies `specimen_wasm.wasm` to
   `lib/specimen/wasm/` (git-ignored); Vite imports it with `?url`. The Tauri
   CSP gains `'wasm-unsafe-eval'` (required to compile WebAssembly);
   `beforeDevCommand`/`beforeBuildCommand` build the WASM first.
7. **Retirement**: `engine.ts` and the TS `validateSettings` are deleted once
   the facade tests, conformance and both e2e suites pass. `tests/engine.test.js`
   becomes `tests/kernel.test.js` against the facade; tests of deleted TS
   internals (tape, HybridTable, fanouts, raft helpers) are dropped in favor of
   the existing Rust unit tests; spec-level tests that differ under native
   semantics are rewritten to native behavior and listed in `VALIDATION.md`.

## Testing

Rust unit tests per mutation (messages, bounds, IDs, RNG determinism with
`FixedHost`); `conformance/` gains a mutation sequence golden reproduced by
native and WASM; `tests/kernel.test.js`; `wasm-probe` becomes an ABI test;
both e2e suites in both themes; desktop build and Tauri smoke.
