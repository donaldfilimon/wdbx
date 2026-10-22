# Studio local ARM64 runtime provenance receipt

2026-10-20. Bounded local source-build qualification under the approved completion program. No commits or pushes by this lane; Studio's intentional path dependencies and lockfiles were preserved.

The original archive builds inherited the enclosing WDBX Git revision in their version metadata. The reviewed helper correction disables both upstream Git discovery mechanisms, clears stale dirty metadata, supplies the actual pinned archive identities and verifies each executable before replacing its destination. Independent specification/quality review passed; five focused Python tests passed independently and in the controller run, including a real CMake fixture. The simulated Windows branch is test evidence only. The fixture currently depends on macOS/Homebrew and requires adaptation before cross-platform CI use.

Both real rebuild commands exited zero from `specimen-studio/`, using `/opt/homebrew/bin/python3` and PATH `/usr/bin:/bin:/opt/homebrew/bin:/usr/sbin:/sbin`:

- `python3 scripts/build-runtimes.py --only diffusion`
- `python3 scripts/build-runtimes.py --only llama`

The CPU/static flags remain unchanged. Diffusion uses its separately pinned ggml; llama uses its own bundled ggml. Upstream deprecated literal-operator warnings remain visible in the build log. No model was downloaded or executed, and no listener was started.

| Artifact | Reported source revision | SHA-256 |
| --- | --- | --- |
| sd-cli | 6b3edaaf32cc19e5bb2d819c788bd557eddc8eba | 2029b8957791b5f944c6604757d8966ceed85fde1c2642a1e5c7314622f5b787 |
| llama-server | 163a40796f0ebaae246325f8d2e15028b413fa9d | d52b8fb0c2e0dbfbe47236d4f89ca8cba10dac103614dc99d05e676b1f0e2e9f |

Each executable's `--version` and `--help` exited zero under a 30-second timeout. `otool -L` exited zero and showed system libraries/frameworks without Homebrew dependencies. Archive build counts remain unknown (reported as zero), and diffusion's release version remains explicitly unknown; neither is invented from the surrounding repository.

[Qualification inventory](../../completion/runtime-evidence/2026-10-20-local-arm64/qualification.json) records probe outputs, binary/archive/input hashes, source file manifests, selected CMake metadata, tools and build-log hashes. Binaries remain ignored local build artifacts. Source manifests describe the actual local build inputs; they are not independent upstream signatures or cryptographic review.

Studio prerequisite checks also passed: `bun install --frozen-lockfile` (934 packages), `bun run lint`, and `bunx tsc --noEmit`. The initial lint attempt failed because oxlint had not yet been installed; the retry passed. The available Bun reports 1.4.4-canary.1, rather than CI's exact 1.4.0. JavaScript and Cargo lockfiles remain unchanged.

This receipt does not establish Studio's full check/native gates, native UI, model inference, acceleration, installed artifacts, signing/notarization, other platforms, federation, hosted recovery or sustained operation. Root Rust qualification is separately held for a coherent handoff from the concurrent episode-store writer. The earlier binaries with parent Git identities remain historical, unqualified artifacts; this receipt identifies their replacements exactly.
