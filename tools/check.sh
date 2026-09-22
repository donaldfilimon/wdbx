#!/usr/bin/env bash
# WDBX repository gate: the same steps CI runs (.github/workflows/ci.yml).
#
# Invoke with the pinned rustup toolchain on PATH (rust-toolchain.toml), not a
# Homebrew cargo/rustc pair. There is no tools/cargo.sh in this repository.
#
# clippy deliberately runs WITHOUT `-D warnings`: the workspace denies
# unsafe_code and clippy `all` through [workspace.lints], but keeps
# `missing_docs` and clippy `pedantic` as warnings. Adding `-D warnings` here
# would turn those into failures; that is a separate, deliberate decision.
#
# `cargo test --workspace` shells out to python3 (or $WDBX_PYTHON) for the
# cross-language episode goldens; a missing interpreter fails, never skips.
set -euo pipefail

cd "$(dirname "$0")/.."

step() {
    printf '\n==> %s\n' "$1"
}

step "agent instructions (CLAUDE.md is the AGENTS.md pointer form)"
bash ./tools/check_instructions.sh

step "Rust source size limits"
bash ./tools/check_rust_sizes.sh

step "format (check only)"
cargo fmt --all --check

step "clippy (workspace lints; no -D warnings)"
cargo clippy --workspace --all-targets

step "tests"
cargo test --workspace < /dev/null

printf '\ncheck: all green\n'
