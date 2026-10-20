# WDBX Specimen Studio

A working local specimen studio with the complete 26-chapter WDBX architecture specification built into its reference reader. The interface is dark graphite and teal by default, with the original white and teal as a light theme, and includes a selectable topology, node inspector, cycle trace, conversation, supporting memory, learning proposals, and an activity journal.

## Use the studio

1. Run **What is 2 + 2?** to inspect an actual arithmetic vote and response.
2. Open **Node Library → Add pattern**. Enter an original pattern and its response, then prompt that pattern in Studio.
3. Turn on **Contributors** to see which node supplied each output segment. **Right/Wrong** feedback performs an independent coin flip per contributor and changes strength when selected.
4. Add supporting records in **Memory** and optionally link them to a Pattern ID. Pin conversation records to retain them beyond rolling history.
5. **Save specimen** exports browser JSON or a native `.wdbxspecimen` archive, depending on the runtime. **Load** validates before replacement. Browser data saves through IndexedDB; desktop data saves transactionally through WDBX. Native import retains recovery copies.
6. Read or search **Specification**, and download the full Markdown document from the reader.

If a prompt has no matching pattern, choose **Teach this prompt** beside the
result. Its text is filled into the pattern editor; add a name and response,
then run the same prompt again. Opening or cancelling that editor leaves any
unfinished next prompt intact. On phones and tablets, the prompt stays above
the diagnostic panels after a run. The Run button enables when you enter text;
press Enter or choose a starter prompt to begin.

The shell is keyboard-first. **⌘K** (Ctrl-K) opens the command palette: go to any view, open a second view beside the current one, save, load, reset, or switch theme. **⌘\\** opens or closes that side pane (it appears at 1024 px and wider, and stays in the URL as `?split=`). **⌘B** collapses the sidebar to an icon rail. The backtick key toggles the activity dock. The sun/moon button at the bottom of the sidebar switches between the dark and light themes; the choice is remembered on this device.

`/prompt`, `/right`, `/wrong`, `/wrng`, `/contributorfractal`, `/saveSpecimen`, `/loadSpecimen`, `/addPattern`, `/learn`, `/dismiss`, `/reconsider` are recognized by the studio composer. Add/load/learn commands open the corresponding visible editor or file chooser. `/learn` accepts a pending proposal ID or exact proposal text, or defaults to the first pending proposal. Use `/dismiss <id>` or button to remove from suggestions; `/reconsider <id>` to re-activate a dismissed proposal for learning. Activity view shows all proposals with status and controls for full self-learning cycle.

## Local development

```sh
bun install
bun dev          # Node: vinext dev with the Cloudflare plugin (fails under Bun)
bun test
bunx tsc --noEmit
bun run build
```

The project uses React, TypeScript, Vinext/Vite, and the Sites Worker packaging workflow. It has no model API key requirement. In the browser edition, specimen content is processed in the browser and stored on the current device and origin. Use a JSON download to move work between the local preview and the hosted studio.

## Architecture and runtime boundary

The full specification is in [the downloadable Markdown](public/WDBX-Specimen-Architecture-Specification.md). It is preserved as the design authority. [RUNTIME-PROFILE.md](RUNTIME-PROFILE.md) documents the concrete browser implementation, limits, and extensions that remain outside this studio.

## Native desktop (0.2.0 qualification build)

The desktop adapter uses Tauri 2 and the Rust engine in `native/specimen-core`.
WDBX storage and compute use this checkout's `crates/` through path dependencies;
both workspaces use stable Rust 1.99.1. ABI/Abbey sibling checkouts are not modified.

```sh
bun install --frozen-lockfile
python3 scripts/build-runtimes.py
bun run desktop
bun run test:native
bun run desktop:package
```

From `specimen-studio/`, `cargo run --locked` also builds and launches the
complete desktop app: the WASM kernel, embedded Vite frontend, and Rust host.
Missing inference executables are built automatically (requires CMake and a C++
compiler); model downloads remain separate. Install Bun dependencies and the
`wasm32-unknown-unknown` Rust target first. Nested WASM builds use
`target/desktop-wasm` so they cannot deadlock on Cargo's native build lock.
Use `bun run desktop` for the live-reloading Vite development server instead.

Linux builds additionally require WebKitGTK 4.1, AppIndicator, librsvg and OpenSSL development packages. Windows builds require the Visual Studio C++ toolchain and WebView2; macOS builds require Xcode command-line tools. Stable Rust 1.99.1 is selected by `rust-toolchain.toml`, matching the parent workspace. Inference binaries build for the current machine. Models are installed separately through Vision & models; model downloads, licenses, hashes and resource estimates are visible there.

Native saves use a dedicated app-data directory and transactional WDBX storage. Export `.wdbxspecimen` for portable records and assets; pretrained models remain referenced by digest. Existing browser JSON imports retain an original recovery copy. The browser edition keeps IndexedDB and its bounded JavaScript profile.

Native UI tests use instrumented builds only: build assets with `VITE_NATIVE_E2E=1`, then `bunx tauri build --debug --features e2e --config src-tauri/tauri.e2e.conf.json --bundles app` on macOS and run `bunx wdio run wdio.conf.mjs`. Production builds omit the test feature and bridge. The current desktop qualification workflow targets the self-hosted macOS ARM64 runner; Intel macOS, Windows and Linux require separate runtime evidence. A configured job is not evidence of a passing platform; consult the [acceptance ledger](VALIDATION.md).

For local native UI tests, set `WDBX_STUDIO_TEST_DATA` to a newly created scratch
directory (for example, `mktemp -d "$PWD/work/native-ui-XXXXXX"`) before running
WebdriverIO. Run that test runner under Node, not `bunx --bun`; its matcher
dependencies do not currently initialize under Bun. Never use your normal
desktop app-data directory for acceptance tests. `bun run test:browser` uses
disposable browser contexts and includes the mobile and teaching-flow
regressions; repeat with `STUDIO_THEME=light` for the light theme.

After `desktop:package` on macOS, run `python3 scripts/seal-macos-package.py` to seal the application bundle ad hoc and recreate the DMG. The script stages outside FileProvider-managed folders so injected Finder metadata cannot invalidate the resource seal. Developer ID signing and notarization remain separate release operations requiring the appropriate identity.

The GLib 0.18 dependency includes the documented upstream `VariantStrIter` safety backport in `native/vendor/README.md`. Linux conformance tests exercise the optimized iterator. React/React DOM/server components are pinned to 19.2.8 and Vite to 8.0.16 for their published security fixes.

After editing the authoritative Markdown, run `python3 scripts/sync-specification.py` (requires `markdown-it-py`) to regenerate all chapter text, prose and figure segments, and reading times together. It fails unless the Markdown still has 29 fenced figures.
