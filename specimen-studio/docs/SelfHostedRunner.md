# Self-hosted macOS runner

The account's GitHub Actions billing is locked, so GitHub-hosted jobs fail within seconds without starting. Self-hosted jobs still run. The Apple silicon jobs below therefore run on a macOS arm64 machine registered to this repository.

## Registration

| Field | Value |
|-------|-------|
| Labels | `self-hosted`, `macOS`, `ARM64`, `wdbx-specimen-studio` |
| Register at | [Settings → Actions → Runners → New self-hosted runner](https://github.com/donaldfilimon/wdbx-specimen-studio/settings/actions/runners/new?arch=arm64) (macOS, ARM64) |

A runner is registered to one repository. If the Mac already runs a runner for another repository (for example `abi` or `gama`), install a second runner in its own directory, such as `~/actions-runner-wdbx-specimen-studio`, and follow the registration page's download and `./config.sh` steps there. Add the custom label `wdbx-specimen-studio` when `config.sh` asks for extra labels. Then run `./svc.sh install && ./svc.sh start`.

`svc.sh` installs a LaunchAgent, so the runner runs in your logged-in session. The native UI tests open app windows, and code signing uses the user's keychain, so both need that session. Keep the account logged in, and use a dedicated macOS user if you can.

Until a runner with these labels is online, the moved jobs wait in the queue.

## Jobs on the self-hosted runner

The trigger for every one of these jobs is `push` to `main` or `workflow_dispatch`. Each job keeps its id and check name.

| Workflow | Job (check name) | Notes |
|----------|------------------|-------|
| `browser.yml` | `browser (chrome)`, `browser (firefox)`, `browser (webkit)` | Moved from `ubuntu-24.04`. The legs run one after another on the single runner. |
| `desktop.yml` | `desktop (macos-14)` | Only the Apple silicon matrix leg. `runs-on` maps `matrix.os == 'macos-14'` to the self-hosted labels. The matrix value stays `macos-14`, so the receipt's `runner.label`, the artifact name `wdbx-studio-macos-14` and the collector's expected matrix are unchanged. |
| `macos-package.yml` | `package (macos-14)` | Only the Apple silicon matrix leg, mapped in the same way. `WDBX_RUNNER` stays `macos-14`. |
| `accelerator.yml` | `cpu-gpu-parity` | Uses the Mac's Metal GPU through wgpu. |
| `ocr.yml` | `ocr` | |
| `qualification-summary.yml` | `consolidate` | Moved from `ubuntu-24.04`. |

The receipt's `runner.label` for the desktop and packaging legs is the matrix leg name (`macos-14`), not a claim that the job ran on GitHub's macOS 14 image. `runner.os` and `runner.arch` still come from the runner. The run's job record in the Actions API (`runner_name`, `labels`) shows that it ran on the self-hosted runner.

## Host requirements

- macOS 14 or later on Apple silicon, with the account logged in (see above).
- Xcode, installed in full and selected with `xcode-select`. The jobs use `clang`, `codesign`, `hdiutil`, and, for notarization, `xcrun notarytool` and `xcrun stapler`.
- Homebrew (`/opt/homebrew/bin` on the runner's `PATH`), with:
  - `python@3.12` or later, as the first `python3` on `PATH`. `/usr/bin/python3` (3.9) is too old: `scripts/build-runtimes.py` needs `tarfile` extraction filters, `scripts/download-models.py` needs `hashlib.file_digest`, and the collector needs 3.10 syntax. The desktop and packaging jobs check the version first.
  - `cmake` for `scripts/build-runtimes.py`.
  - `jq` for the receipts and the collector.
  - `gh` for `scripts/repackage-macos.py` and the collector. The token reaches `gh` only through `GH_TOKEN`.
  - `node` 22.13 or later (`engines.node`). Hosted images have Node, and `bunx` runs Node-shebang tools such as `tauri` and `wdio` under it in the desktop job. The browser job installs its own Node with `actions/setup-node`.
- `rustup` (from rustup.rs). The jobs install `nightly-2026-09-01` from `rust-toolchain.toml`.
- Google Chrome in `/Applications` for `browser (chrome)`. Playwright's `install chrome` on macOS runs `sudo rm -rf "/Applications/Google Chrome.app"` and reinstalls it, so the job checks that the host's Chrome exists instead. Firefox and WebKit are downloaded per user into `~/Library/Caches/ms-playwright`.
- Bun needs no host install. `oven-sh/setup-bun` fetches the pinned 1.4.0.
- The runner reads `PATH` when `config.sh` runs (in `.path` inside the runner directory). If you install tools later, run `./env.sh` in the runner directory and then `./svc.sh stop && ./svc.sh start`.

The runner sets `CI=true`. So `tauri build` skips the Finder AppleScript layout step when it makes the DMG, as it does on hosted runners.

### Step changes for macOS arm64

- `browser.yml`: `bunx playwright install --with-deps <browser>` is now "Provide Playwright browser". It checks the host's Chrome for the `chrome` leg and runs `bunx playwright install <browser>` for the others. `--with-deps` only installs apt packages on Linux.
- `desktop.yml` (the `macos-14` leg only): a first step creates a per-job virtual environment in `$RUNNER_TEMP`. Later steps call unversioned `python` and `python -m pip install pillow`, and Homebrew's Python refuses pip installs (PEP 668). That step also sets `MACOSX_DEPLOYMENT_TARGET=14.0`.
- `macos-package.yml` (the `macos-14` leg only): sets `MACOSX_DEPLOYMENT_TARGET=14.0`. Without it, CMake builds the helper runtimes for the host's SDK version, and the "portable" installer's helpers would not start on macOS 14. `tauri build` still sets its own target from `minimumSystemVersion` (12.0).
- `desktop.yml` and `macos-package.yml`: on self-hosted runners the runtime cache key starts with `native-runtimes-self-hosted-`. CMake build trees hold absolute paths, so a hosted cache must not be restored into a self-hosted workspace. Hosted legs keep their existing keys.

## Signing on a persistent host

`package (macos-14)` with `notarize: true` runs `scripts/sign-macos-ci.py`. That script creates a temporary keychain, makes it the user's default and adds it to the search list while signing, then restores both and deletes the keychain. If the job is killed before the cleanup runs, restore the settings by hand:

```sh
security default-keychain -d user -s ~/Library/Keychains/login.keychain-db
security list-keychains -d user -s ~/Library/Keychains/login.keychain-db
```

The signing secrets exist only in that job's environment. Keep no other release credentials on the host.

## Security model

This repository is public. Self-hosted runners must never run code from a fork.

- Every self-hosted job has `if: github.repository == 'donaldfilimon/wdbx-specimen-studio' && (<trusted event>)`. None of these workflows has a `pull_request`, `pull_request_target`, `issue_comment` or `workflow_run` trigger. They run only on `push` to `main` and `workflow_dispatch`, which need write access. So no GitHub-hosted fork-PR fallback job is needed.
- `consolidate` checks out `inputs.source_sha` and runs that commit's `scripts/qualification-summary.py`. GitHub serves fork pull-request commits by SHA from this repository. Dispatch it only with a commit that is on a branch of this repository.
- Checkouts use `persist-credentials: false`. No job pushes with the token. Workflow permissions are unchanged: `contents: read`, plus `actions: read` where artifacts from other runs are downloaded.
- `actions/checkout` cleans the workspace on every run (`git clean -ffdx`). Rustup toolchains, Playwright browsers and Homebrew packages stay on the host between runs.
- Two runners on one Mac share `~/.rustup`, `~/.cargo` and `~/Library/Caches/ms-playwright`. Parallel `rustup toolchain install` runs from different repositories can collide. Retry, or give each runner its own macOS user.

## Jobs that stay GitHub-hosted

These jobs wait for the billing lock to be cleared:

| Workflow | Job | Why |
|----------|-----|-----|
| `desktop.yml` | `desktop (macos-15-intel)` | Intel x86_64 qualification. An arm64 Mac cannot produce it. |
| `desktop.yml` | `desktop (windows-2022)` | NSIS installer and Windows runtimes. Needs Windows. |
| `desktop.yml` | `desktop (ubuntu-24.04)` | apt WebKitGTK, `.deb`/AppImage/RPM packaging, `xvfb`. Needs Linux. |
| `desktop.yml` | `text-model` | Real CPU text inference qualified on Intel `macos-15-intel`. Its receipt records the runner. Moving it to arm64 would change what is qualified, which is an owner decision. |
| `image-model.yml` | `image-model` | CPU SDXL-Turbo inference qualified on Intel `macos-15-large`. Same reason as `text-model`. |
| `macos-package.yml` | `package (macos-15-intel)` | Intel x86_64 portable installer. |
| `windows.yml` | `desktop (windows-2022)` | Windows-only NSIS build and runtime checks. |
| `windows-package.yml` | `package` | PowerShell, Authenticode `signtool`, registry checks and a silent install. Needs Windows. |

A consolidated qualification still needs every required run, including the Intel, Windows and Linux desktop legs and the text and image inference runs. Until those can run, the collector reports the missing evidence as unverified, which is correct.
