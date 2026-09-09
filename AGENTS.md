# Repository Guidelines

Canonical project guidance for both browser and native editions.

## Project Structure & Module Organization

- `app/` contains the React studio, routes, and styles; `components/` contains shared controls and the native lab; `hooks/` contains React hooks.
- `lib/specimen/` holds browser engine logic, contracts, persistence, and the native bridge. `lib/webmcp.ts` exposes optional browser tools.
- `native/specimen-core/` contains the Rust engine and conformance tests; `src-tauri/` contains desktop integration and icons; `desktop/` provides its frontend entry point.
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
- `bun run test:native`: test `specimen-core`; `bun run check:native` runs workspace Clippy with warnings denied.

There is no aggregate `check` script. Browser code verification combines `bun test`,
`bunx tsc --noEmit`, `bun run lint:studio`, and `bun run build`. Native CI uses
`cargo test -p specimen-core --locked`; this excludes desktop UI qualification.

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
  `native/specimen-core/Cargo.toml`, not sibling path dependencies. Do not change
  ABI/Abbey/WDBX checkouts to implement this studio's runtime behavior.
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

`.github/workflows/desktop.yml` separates desktop qualification from opt-in text
inference; `image-model.yml` is manually dispatched image inference qualification.
Native UI requires both `VITE_NATIVE_E2E=1` assets and Rust `e2e` instrumentation.
Keep the test bridge out of production. Configured jobs are not passing receipts.

## Remotes: there are two, and the default one is dead

`git remote -v` is the check here, not `git branch -vv`.

- `origin` points at `git.chatgpt-team.site`, the generated Codex-app host. It is
  **unusable from this machine**: push, fetch, and `ls-remote` all fail with
  `could not read Username ... Device not configured`, because only GitHub has a
  credential helper configured. It is also the tracking upstream, so `git branch -vv`
  reports an ahead-count against a server nothing can reach, and `@{u}` answers off the
  stale cached ref. A `0 ahead` reading there is not evidence anything was pushed.
- `github` points at `donaldfilimon/wdbx-specimen-studio` and **is reachable**. This is
  the real backup: `git ls-remote github` exits 0 and `main` matches. Push here.

The consequence of reading only the upstream line is concluding this repository is
unbacked-up and needs rescue bundling, which was believed for two days. Its bundle is
still worth keeping for a different reason: it carries `refs/codex/turn-diffs/*` refs that
GitHub does not have, and `git bundle list-heads` lists those first, so grep `refs/heads/`
rather than taking the first line.

## Commit & Pull Request Guidelines

History uses short imperative subjects, such as `Fix managed preview compatibility`. Keep commits focused. PRs should explain behavior and rationale, link relevant issues, report actual checks and platform limitations, and include screenshots for visual changes. Preserve unrelated working-tree edits.

## Architecture & Specification

Read `RUNTIME-PROFILE.md` and `NATIVE-RUNTIME-PROFILE.md` before changing runtime behavior. Edit `public/WDBX-Specimen-Architecture-Specification.md` as the specification authority, then regenerate `lib/specification.json` with `python3 scripts/sync-specification.py` (requires `markdown-it-py`).
