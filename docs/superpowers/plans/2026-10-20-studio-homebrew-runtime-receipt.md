# Studio Homebrew runtime inspection receipt

2026-10-20. User authorized trying Homebrew for Studio runtime dependencies. Official `homebrew/core` packages were installed with automatic broad updates, installation cleanup and installed-dependent checks disabled. No third-party tap, direct-source fallback, model download, server startup, or Studio configuration/pin change was performed.

`brew install --formula llama.cpp` exited 0. Installed bottled packages include llama.cpp 0.7.0, ggml 0.27.0 and libomp 23.1.3; the package also requires openssl@4. Installation log: `/tmp/wdbx-20261020-brew-llama-install.log`.

The actual ARM64 executable is `/opt/homebrew/Cellar/llama.cpp/0.7.0/bin/llama-server` (linked from `/opt/homebrew/bin/llama-server`), SHA-256 `fd5cb5a7974c1633c9f7e487908cc270094f8506d823523b16c4b3ed723eabde`. Non-serving `--version` and `--help` each exited 0. Version output: `0.7.0 (build 11669, commit 620d2225c)`, AppleClang 21.0.0.21000334, Darwin arm64. Help lists every current Studio launch flag: `--model`, `--host`, `--port`, `--api-key`, `--ctx-size`, `--threads`, `--n-gpu-layers`, `--jinja`. Raw probes are `/tmp/wdbx-studio-brew-runtime-probes.json`.

`otool -L` shows dependent llama libraries plus Homebrew ggml and openssl@4 dynamic libraries. This installed executable is not a self-contained desktop sidecar. Flag availability and successful non-serving probes establish CLI surface and loader availability only; HTTP responses, model loading, inference, GPU execution and packaged portability were not exercised.

No official `sd-cli` provider was established: `stable-diffusion.cpp` and `stable-diffusion` formula lookup failed; official formula searches for stable-diffusion/sd.cpp/diffusion.cpp variants and broad diffusion found no matching provider. The llama package's separately named diffusion utility is not qualified as a replacement for Studio's `sd-cli` interface.

This is partial dependency availability, not pinned runtime qualification. Brew llama commit `620d2225c` differs from Studio llama pin `163a40796f0ebaae246325f8d2e15028b413fa9d`, and Brew ggml 0.27.0 differs from pinned ggml `e20c3a14aa70ee84ca58499814206dd08d8026bc`. The stable-diffusion pin `6b3edaaf32cc19e5bb2d819c788bd557eddc8eba` remains unavailable through the inspected official packages. Source pins and lockfiles remain unchanged. Studio's required bundled resources and full installed-artifact/inference gates remain open; no dummy file, symlink or gate relaxation was used.
