# Repository Guidelines

## Project Structure & Module Organization

- `app/` contains the React studio, routes, and styles; `components/` contains shared controls and the native lab; `hooks/` contains React hooks.
- `lib/specimen/` holds browser engine logic, contracts, persistence, and the native bridge. `lib/webmcp.ts` exposes optional browser tools.
- `native/specimen-core/` contains the Rust engine and conformance tests; `src-tauri/` contains desktop integration and icons; `desktop/` provides its frontend entry point.
- `tests/` contains browser unit and interface tests. `public/` holds static assets and specification Markdown; `scripts/` manages specification generation, runtimes, and packaging.

## Build, Test, and Development Commands

Use Bun, Node.js 22.13+, and the Rust toolchain pinned in `rust-toolchain.toml`.

- `bun install --frozen-lockfile`: install locked JavaScript dependencies.
- `bun dev`: start the Vinext/Vite browser preview.
- `bun run build`: create the production Worker build; `bun start` serves it locally.
- `bun test`: run Bun unit and UI contract tests.
- `bunx tsc --noEmit`: check TypeScript types.
- `bun run lint:studio`: lint studio/runtime code; `bun run lint` checks the broader project.
- `bun run desktop`: launch Tauri; first build inference binaries with `python3 scripts/build-runtimes.py`.
- `bun run desktop:package`: package the desktop application.
- `bun run test:native`: test `specimen-core`; `bun run check:native` runs workspace Clippy with warnings denied.

## Coding Style & Naming Conventions

Use strict TypeScript, two-space indentation, single quotes, and semicolons. Oxfmt configures an 80-column width; format changed files with `bun run format <paths>`. Use PascalCase component/type names, camelCase functions, and kebab-case component filenames. Avoid explicit `any`. Format Rust with `cargo fmt --all`; use snake_case functions/modules.

## Testing Guidelines

Use descriptive behavior names in `tests/*.test.js` (`bun:test`) and Rust `#[test]` functions. Cover malformed imports, persistence recovery, cancellation, and browser/native boundaries when affected. No numeric coverage threshold is configured.

With the preview running and Chrome installed, run `bun run test:browser`; `STUDIO_URL` overrides `http://localhost:3000`. Native interface tests use WebdriverIO/Mocha and require the instrumented build described in `README.md`.

## Commit & Pull Request Guidelines

History uses short imperative subjects, such as `Fix managed preview compatibility`. Keep commits focused. PRs should explain behavior and rationale, link relevant issues, report actual checks and platform limitations, and include screenshots for visual changes. Preserve unrelated working-tree edits.

## Architecture & Specification

Read `RUNTIME-PROFILE.md` and `NATIVE-RUNTIME-PROFILE.md` before changing runtime behavior. Edit `public/WDBX-Specimen-Architecture-Specification.md` as the specification authority, then regenerate `lib/specification.json` with `python3 scripts/sync-specification.py` (requires `markdown-it-py`).
