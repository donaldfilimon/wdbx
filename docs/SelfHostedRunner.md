# Self-hosted macOS runner

The `gate (instructions, sizes, fmt, clippy, test)` job (`gate`) in `.github/workflows/ci.yml` runs on a macOS arm64 runner registered to this repository. GitHub-hosted jobs can't start while the account's Actions billing is locked, but self-hosted jobs still run.

## Registration

| Field | Value |
|-------|-------|
| Labels | `self-hosted`, `macOS`, `ARM64`, `wdbx` |
| Register at | [Settings → Actions → Runners → New self-hosted runner](https://github.com/donaldfilimon/wdbx/settings/actions/runners/new?arch=arm64) (macOS, ARM64) |

A runner is registered to one repository. If the same Mac already runs a runner for another repository (for example `abi` or `gama`), install a second runner in its own directory (for example `~/actions-runner-wdbx`), pass `--labels wdbx` to `./config.sh` (or add the custom label afterwards on the runner's settings page), then run `./svc.sh install && ./svc.sh start`.

Until a runner with these labels is online, same-repo `gate` jobs wait in the queue.

## Host requirements

- Xcode Command Line Tools (`xcode-select --install`). They supply the `cc` linker Rust needs on macOS, `git`, and `/usr/bin/python3`, which `cargo test --workspace` calls for the cross-language golden checks (`tools/abbey_cbor_episode_v1.py` runs on the CLT's Python 3.9). Set `WDBX_PYTHON` in the runner's `.env` file to use another interpreter.
- Network access to `sh.rustup.rs`, `static.rust-lang.org` and crates.io. `dtolnay/rust-toolchain` installs rustup into `~/.cargo` when the host lacks it, installs `nightly-2026-09-01` with rustfmt and clippy, and makes it the runner account's default toolchain (`rustup default`). `rust-toolchain.toml` adds `rust-src`.
- Nothing needs `sudo`. The job puts `$CARGO_HOME/bin` first on `PATH` and fails if `cargo` still resolves elsewhere, because a Homebrew `cargo` ignores `rust-toolchain.toml`.
- The shell steps use `/bin/bash`; `tools/check*.sh` work with macOS's bash 3.2.

## Security

This repository is public, so the self-hosted job runs only for `push` to `main`, `workflow_dispatch`, and pull requests from branches in this repository (`github.event.pull_request.head.repo.full_name == github.repository`). It also requires `github.repository == 'donaldfilimon/wdbx'`, so forks that copy the workflow never target this runner. Fork pull requests use the GitHub-hosted `gate-hosted` job, an unchanged copy of the original job on `macos-latest`. No workflow uses `pull_request_target`, `issue_comment` or `workflow_run`.

Checkouts use `persist-credentials: false`, and the workflow token stays `contents: read`. Where you can, use a dedicated macOS user for the runner rather than your daily account, since `rustup default` changes that account's toolchain. Keep no production secrets on the host.

## Not covered

Every job in this repository's CI now has a self-hosted path for trusted events. `gate-hosted` (fork PRs only) still needs a GitHub-hosted runner, so fork PRs stay blocked until the billing lock is cleared.
