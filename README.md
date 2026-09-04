# WDBX Specimen Studio

A working local specimen studio with the complete 26-chapter WDBX architecture specification built into its reference reader. The white and teal interface includes a selectable topology, node inspector, cycle trace, conversation, supporting memory, learning proposals, and an activity journal.

## Use the studio

1. Run **What is 2 + 2?** to inspect an actual arithmetic vote and response.
2. Open **Node Library → Add pattern**. Enter an original pattern and its response, then prompt that pattern in Studio.
3. Turn on **Contributors** to see which node supplied each output segment. **Right/Wrong** feedback performs an independent coin flip per contributor and changes strength when selected.
4. Add supporting records in **Memory** and optionally link them to a Pattern ID. Pin conversation records to retain them beyond rolling history.
5. **Save specimen** downloads the complete workspace as JSON. **Load** validates a file and previews its contents before replacement. Device storage also saves automatically through IndexedDB. A failed recovery pauses autosave to preserve existing data.
6. Read or search **Specification**, and download the full Markdown document from the reader.

`/prompt`, `/right`, `/wrong`, `/wrng`, `/contributorfractal`, `/saveSpecimen`, `/loadSpecimen`, `/addPattern`, and `/learn` are recognized by the studio composer. Add/load/learn commands open the corresponding visible editor or file chooser. `/learn` accepts a pending proposal ID or exact proposal text, or defaults to the first pending proposal.

## Local development

```sh
bun install
bun dev
bun test
bunx tsc --noEmit
bun build
```

The project uses React, TypeScript, Vinext/Vite, and the Sites Worker packaging workflow. It has no model API key requirement. Specimen content is processed in the browser and stored on the current device and origin. Use a JSON download to move work between the local preview and the hosted studio.

## Architecture and runtime boundary

The full specification is in [the downloadable Markdown](public/WDBX-Specimen-Architecture-Specification.md). It is preserved as the design authority. [RUNTIME-PROFILE.md](RUNTIME-PROFILE.md) documents the concrete browser implementation, limits, and extensions that remain outside this studio.

## Native desktop (0.2.0 qualification build)

The desktop adapter uses Tauri 2 and the Rust engine in `native/specimen-core`. WDBX storage and compute are pinned at `e55634cbb581c1de02f946a29c34db8cf5203704`. Existing ABI/Abbey/WDBX sibling checkouts are not modified.

```sh
bun install --frozen-lockfile
python3 scripts/build-runtimes.py
bun run desktop
cargo test -p specimen-core
bun run desktop:package
```

Linux builds additionally require WebKitGTK 4.1, AppIndicator, librsvg and OpenSSL development packages. Windows builds require the Visual Studio C++ toolchain and WebView2; macOS builds require Xcode command-line tools. The pinned nightly toolchain is selected by `rust-toolchain.toml`. Inference binaries build for the current machine. Models are installed separately through Vision & models; model downloads, licenses, hashes and resource estimates are visible there.

Native saves use a dedicated app-data directory and transactional WDBX storage. Export `.wdbxspecimen` for portable records and assets; pretrained models remain referenced by digest. Existing browser JSON imports retain an original recovery copy. The browser edition keeps IndexedDB and its bounded JavaScript profile.

Native UI tests use instrumented builds only: build assets with `VITE_NATIVE_E2E=1`, then `bunx tauri build --debug --features e2e --config src-tauri/tauri.e2e.conf.json --bundles app` on macOS and run `bunx wdio run wdio.conf.mjs`. Production builds omit the test feature and bridge. The desktop qualification workflow covers macOS ARM/Intel, Windows x64 and Linux x64. A configured job is not evidence of a passing platform; consult the verification report.

After `desktop:package` on macOS, run `python3 scripts/seal-macos-package.py` to seal the application bundle ad hoc and recreate the DMG. The script stages outside FileProvider-managed folders so injected Finder metadata cannot invalidate the resource seal. Developer ID signing and notarization remain separate release operations requiring the appropriate identity.
