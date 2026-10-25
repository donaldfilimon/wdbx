# Studio GTK verification — 2026-10-25

GTK source: `7010240`; separate builder repair: `5ce4535`.
Base: `43060e2d37f2fb0987c78ecab5d31d03bac2cf34`.
Host: macOS arm64, Rust 1.99.1, GLib/GObject 2.90.1, Bun 1.4.4.
The GTK and expanded-builder source cohorts are recorded independently in private
receipts; gates ran with isolated writable targets and stdin at EOF. Functional
checks under concurrent load establish no performance qualification.

| Check | Actual outcome |
| --- | --- |
| Five original archives, licenses, metadata and exact local transformations | Passed; fresh upstream bytes matched independent cached pins |
| Consumer/vendor locked resolution, replacement features and source controls | Passed; 12 contract/negative-control tests |
| Both macro libraries and four intended invalid expansions | Passed; pinned compiler, one intended error per fixture, no panic/unresolved crate |
| GLib enum/value/clone and optimized string iterator | 2 dev + 2 release tests passed; actual GLib artifact opt-level 3 in both profiles |
| Upstream GLib variant iterator | 11 dev + 11 release tests passed |
| Studio durable roundtrip regression | One exact test passed |
| `bun run check` | Passed, 221 tests and production browser build; actual fresh WASM |
| `bun run test:native` | Passed, 337 tests, one explicitly ignored GPU parity test |
| `bun run check:native` on expanded source | Passed, including provenance gate and workspace Clippy with warnings denied |
| Runtime builder / extraction | 21 tests passed on Python 3.9 and 3.15; 4 desktop caller tests passed |
| Actual source extraction | 6,070 original regular-file hashes matched across all three pinned archives; one internal ggml documentation link safely materialized |
| Final native asset staging | Sidecars match qualified input hashes; fresh nested WASM copy matches artifact and passes four golden/error probes |
| Native GTK widget fixture, native Linux gates and Windows build/install | Unexecuted; GTK 3 development libraries are absent on this Mac |

The existing Linux-only Studio GLib conformance test was preserved and was not
counted as executed on macOS. The real GTK fixture remains ready for native Linux
with GTK 3 libraries and a declared display. No generated TypeScript, kernel,
portable-save code, root/fuzz lock or WDBX crate changed. ZIP 9.0.5, Tokio 1.54.1,
ctutils 0.4.4 and the distinct GLib 0.21 graph remain unchanged.

The first llama prerequisite build fetched mutable Hugging Face `latest` UI
assets. That attempt and all 140 asset hashes remain private and unqualified.
The separate tracked builder repair disables both upstream UI routes, refuses
stale source/build UI before download/build, and safely extracts source on Python
3.9. Only this task's generated UI/binary was quarantined. The final API server
retains `LLAMA_BUILD_SERVER=ON`; both UI flags are OFF and its generated asset
array has length zero. No model or service was started. Sidecars establish host
build/version identity, not inference, installed-package or `native.wdbx`
activation qualification. Source archives, SDK identity, build flags, versions,
licenses and artifact identities are retained privately.

Strict `cargo audit --deny warnings --json` exits **1** for both locks, with no
ignores. Database: `21d909e4c8bd4e14c7aea58bf980c881a0053ab1`, 1,352 advisories,
updated 2026-10-24. Studio retains paste 1.0.15 (2024-0436), five unic 0.9.0
maintenance findings (2025-0075/0080/0081/0098/0100), and lru 0.12.5 unsoundness
(2026-0002/0253; combined fixed floor 0.18.2). The vendor development graph retains
basic-toml 0.1.10 (2026-0374, no patched version), from upstream trybuild2 1.3.0
(`glib` requires `1`, `glib-macros` requires `1.1`, trybuild2 requires basic-toml
`0.1`). These remain separately owned; this is not a whole-graph security pass.

Raw logs preserve GTK macro dead-field/resolver and GLib lifetime warnings,
ABI unused-import and block future-compatibility warnings, frontend chunk-size
warnings, and actual nested rust-objcopy SIGABRT/libLLVM loader failures (44 in
native tests, one in final Clippy). Functional exit zero does not prove stripping
worked. Three `git diff --check` findings are exact original archive whitespace,
including both supplied LICENSE endings; those bytes were deliberately preserved.

| Frozen input/artifact | SHA-256 |
| --- | --- |
| Studio Cargo.lock | `668a48b9eaf9c5bcdc7b2cce4b73ebf9e1d2a618f3053f103a03fbc3ffe2545a` |
| Vendor Cargo.lock | `26b5b1fa4bb7703beefae215a69eb46de5193e60a824dda8b21cf4711d34a1f9` |
| Source manifest | `f0da7ea3319364955f713f3b405702755667b6762343205ecee3a6837d2b47b4` |
| Runtime builder | `cf32a9520e407e229788226152b70b3e432f5827d881dcadd1abbbb248abaaa0` |
| Headless llama-server | `d11574310c922bd7b03ad62a90958524b9d320313e05a17e2eaa768d77e3e248` |
| sd-cli | `b1190020f5c30f2b3a67f8a5f3c05c5072339c6f5b15013cae6c4ab10f2a086d` |
| Browser-gate WASM | `837a188e6058deeb04eb4d6ec1dfc2bd57586bdc6ed7ed5b85a6795635efc599` |
| Final nested native WASM | `631441389f64a24e9f202351cdeae6dd315ef233a611c44801fa8d01d659d0e9` |

Revert the task source commits to restore the previous graph/builder. No live
store migration, SDK installation, model download, deployment or retirement was
performed. Private command/environment/source/lock/artifact receipts are under
`.work/gtk/receipts/`; no raw machine-specific environment is duplicated here.

## Review fix: bounded TAR metadata

The table and artifact hashes above remain evidence for the initial GTK/builder
cohorts. Review I1 found that the initial extractor limited ordinary members only
after TAR had parsed hidden PAX/GNU records. The follow-up builder SHA is
`a7340c8531b85b8aaceb3b0607f13aee918b0290a81bab93b6fe0c686c1f4e82`.
It streams gzip TAR into a private bounded spool before extended-header parsing,
counts physical headers/payloads/padding and trailing decoded bytes, limits both
individual/cumulative metadata and header recursion, rejects sparse extensions,
and charges metadata alongside materialized file aliases. No unbounded pre-read
or metadata exemption is used; cancellation closes the spool and removes staging.

The retained reviewer input is now refused before publication on Python 3.9.6
and 3.15.1. Covering extraction/provenance suites pass 31 tests on each; actual
three-archive replay again matches all 6,070 file hashes and the one safe alias
on both interpreters. Cached headless builder/provenance replay passes with both
UI flags OFF and server ON; both sidecar hashes above are unchanged. CMake
warnings remain visible. This scoped Python fix did not rerun or extend the
earlier full Rust/browser qualification. Platform, advisory, stripping and
service/model/installed-product limits above remain unchanged.
