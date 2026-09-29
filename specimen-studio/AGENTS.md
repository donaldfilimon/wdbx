# Repository Guidelines

Canonical project guidance for both browser and native editions.

This project lives in `specimen-studio/` inside the `donaldfilimon/wdbx` repository
(folded in with full history on 2026-09-28 from the standalone
`donaldfilimon/wdbx-specimen-studio`, now retired). Run every command below from
`specimen-studio/`. It stays its own Cargo workspace: the wdbx root `Cargo.toml`
excludes it, and wdbx's `tools/check.sh` does not build or test it.

## Project Structure & Module Organization

- `app/` contains the React studio, routes, and styles; `components/` contains shared controls and the native lab; `hooks/` contains React hooks.
- `lib/specimen/` holds browser engine logic, contracts, persistence, and the native bridge. `lib/webmcp.ts` exposes optional browser tools.
- `native/specimen-kernel/` holds the pure specimen rules (`engine`, `language`, the CPU path of `neural`, `search`). It performs no I/O and builds for `wasm32-unknown-unknown`: wall clock, IDs, monotonic time (`host::Env`), the raft scan (`search::Search`) and GPU layers (`neural::Accelerator`) arrive through `host::Host`. `FixedHost` is the deterministic host for tests. Never add `chrono`, `uuid`, `std::thread`, `std::time::Instant` or `wgpu` to it. It enables `serde_json/preserve_order` itself because the native build gets it from `abi-wdbx`; without it the browser build would serialize JSON objects in a different key order than desktop.
- `native/specimen-core/` is the native host: WDBX persistence, models, vision, the threaded raft scheduler, `WgpuAccelerator`, `host::NativeHost`, and the conformance tests. It re-exports the kernel modules, so `specimen_core::engine` and friends still resolve. `src-tauri/` contains desktop integration and icons; `desktop/` provides its frontend entry point.
- `native/specimen-wasm/` is the P0 feasibility probe: one raw-ABI export running a full cycle on `FixedHost`. `conformance/` holds the shared corpus (`starter.json`) and `p0-probe.sha256`, the golden digest that the native test and `tests/wasm-probe.test.js` must both reproduce; `p0-probe-imagine.sha256` covers the transcendental paths and is a known WASM divergence (`test.failing`, see `VALIDATION.md`). Regenerate goldens only from native with `UPDATE_GOLDEN=1 cargo test -p specimen-wasm`, never from WASM. The program design is `docs/superpowers/specs/2026-09-28-specimen-studio-desktop-design.md`.
- `tests/` contains browser unit and interface tests. `public/` holds static assets and specification Markdown; `scripts/` manages specification generation, runtimes, and packaging.

## Build, Test, and Development Commands

Use Bun, Node.js 22.13+, and `nightly-2026-09-01` from `rust-toolchain.toml`.
CI uses Bun 1.4.0; dependency resolutions live in `bun.lock` and `Cargo.lock`.

- `bun install --frozen-lockfile`: install locked JavaScript dependencies.
- `bun dev`: start the Vinext/Vite browser preview.
- `bun run build`: create the production Worker build; `bun start` serves it locally.
- `bun test`: run Bun unit and UI contract tests.
- `bunx tsc --noEmit`: check TypeScript types.
- `bun run lint:studio`: lint studio/runtime code; `bun run lint` checks the broader project.
- `bun run desktop`: launch Tauri; first build inference binaries with `python3 scripts/build-runtimes.py`.
- `bun run desktop:package`: package the desktop application.
- `bun run test:native`: `cargo test --locked` over `specimen-kernel`, `specimen-core` and `specimen-wasm`; `bun run check:native` runs workspace Clippy with warnings denied.
- `bun run build:wasm`: release-build the kernel probe for `wasm32-unknown-unknown` (target installed by `rustup`); `bun test` needs it for `tests/wasm-probe.test.js`, so `bun run check` runs it first.
- Single tests: `bun test tests/engine.test.js` runs one file, `bun test -t '<name pattern>'` filters by name; `cargo test -p specimen-core --locked --test conformance -- <test_name>` runs one Rust conformance test (`native/specimen-core/tests/conformance.rs`); `cargo test -p specimen-kernel <filter>` runs kernel unit tests.

`bun run check` is the browser gate: `scripts/check-instructions.sh` (CLAUDE.md must
stay a pointer to this file), `bunx tsc --noEmit`, `bun run lint:studio`,
`bun run build:wasm`, `bun test`, then `bun run build`. `bun run check:all` chains it with
`bun run check:native` (workspace Clippy). Neither runs the native tests: native CI
uses `bun run test:native`, and none of these
cover desktop UI qualification.

This project has two halves and therefore two gates. Running only one half and
calling it green is the standing mistake here: report both `bun run check` and
`bun run test:native`.

It has been found running (`bun dev` + `vite` + `workerd`). Check for live processes
before touching `node_modules`, `.wrangler`, or `.next`.

## Runtime boundaries

- `vite.config.ts` combines Vinext, Sites, and Cloudflare; `next.config.ts` is not
  the build entry point. `bun start` needs generated `dist/server/wrangler.json`.
  Preserve project-local Wrangler paths and Seatbelt polling setup in Vite.
- Tauri uses `vite.desktop.config.ts`: `desktop/` entry, localhost:1420,
  `desktop-dist/` output, and separate before-dev/build commands in
  `src-tauri/tauri.conf.json`. Do not package the Worker output as desktop assets.
- `lib/specimen/storage.ts` selects origin-local IndexedDB or the native bridge.
  `lib/specimen/native.ts` serializes durable edits with expected revisions;
  `native/specimen-core/src/persistence.rs` owns the WDBX store and recovery copies.
- Native `abi-wdbx` and `abi-compute` are Git-revision dependencies in
  `native/specimen-core/Cargo.toml` and (`abi-compute` only) `native/specimen-kernel/Cargo.toml` (pinned to wdbx rev `3ac03f0`, which predates the
  fold), not path dependencies on the sibling `../crates/`, even though those crates
  now sit in the same repository. Picking up WDBX changes means bumping that rev;
  switching to path dependencies is a deliberate decision, not a cleanup. Do not
  change the wdbx crates, ABI or Abbey to implement this studio's runtime behavior.
- Keep the root Cargo GLib patch and its attribution/safety rationale in
  `native/vendor/README.md`; the vendored crate is excluded from the workspace.
- `scripts/build-runtimes.py` downloads pinned sources and compiles local CPU
  inference binaries into `src-tauri/binaries/`. Model downloads are separate;
  neither a successful binary build nor a model download establishes inference.

## Coding Style & Naming Conventions

Use strict TypeScript, two-space indentation, single quotes, and semicolons. Oxfmt configures an 80-column width; format changed files with `bun run format <paths>`. Use PascalCase component/type names, camelCase functions, and kebab-case component filenames. Avoid explicit `any`. Format Rust with `cargo fmt --all`; use snake_case functions/modules.

## Testing Guidelines

Use descriptive behavior names in `tests/*.test.js` (`bun:test`) and Rust `#[test]` functions. Cover malformed imports, persistence recovery, cancellation, and browser/native boundaries when affected. No numeric coverage threshold is configured.

With the preview running and Chrome installed, run `bun run test:browser`; `STUDIO_URL` overrides `http://localhost:3000`. Native interface tests use WebdriverIO/Mocha and require the instrumented build described in `README.md`.

CI lives at the wdbx repository root as `.github/workflows/specimen-studio-*.yml`
(GitHub ignores workflows in subdirectories). Run steps default to
`working-directory: specimen-studio`; action inputs (cache, upload-artifact,
`hashFiles`) carry the `specimen-studio/` prefix. The workflow-path provenance
checks in `scripts/qualification-summary.py` and `scripts/repackage-macos.py` name
those files, so renaming a workflow means updating them and their tests together.
`specimen-studio-desktop.yml` separates desktop qualification from opt-in text
inference; `specimen-studio-image-model.yml` is manually dispatched image inference
qualification. Native UI requires both `VITE_NATIVE_E2E=1` assets and Rust `e2e`
instrumentation. Keep the test bridge out of production. Configured jobs are not
passing receipts. The Apple silicon jobs (browser, `desktop (macos-14)`,
`package (macos-14)`, accelerator, OCR, consolidation) run on the wdbx repository's
self-hosted macOS arm64 runner (label `wdbx`); see `docs/SelfHostedRunner.md` for
host setup, the trust gate, and the GitHub-hosted jobs removed on 2026-09-28. The
text-model and image-model inference jobs also run on that runner now, and the
Windows workflows are gone, so the collector's desktop matrix is `macos-14` only
and it no longer knows the `windows-signing` kind.

## Repository and history

The code is versioned by the wdbx repository; `git remote -v` there is the check.
The retired standalone repository had two remotes: `github`
(`donaldfilimon/wdbx-specimen-studio`, archived on GitHub after the fold) and a dead `origin` on
`git.chatgpt-team.site`, the generated Codex-app host, which was never reachable from
this machine. Its `refs/codex/turn-diffs/*` refs were deliberately not imported into
wdbx; they survive only in
`~/at-risk-bundles/2026-09-28-consolidation/wdbx-specimen-studio.bundle`
(`git bundle list-heads` lists them first, so grep `refs/heads/`).

## Commit & Pull Request Guidelines

History uses short imperative subjects, such as `Fix managed preview compatibility`. Keep commits focused. PRs should explain behavior and rationale, link relevant issues, report actual checks and platform limitations, and include screenshots for visual changes. Preserve unrelated working-tree edits.

## Architecture & Specification

Read `RUNTIME-PROFILE.md` and `NATIVE-RUNTIME-PROFILE.md` before changing runtime behavior. Edit `public/WDBX-Specimen-Architecture-Specification.md` as the specification authority, then regenerate `lib/specification.json` with `python3 scripts/sync-specification.py` (requires `markdown-it-py`).
