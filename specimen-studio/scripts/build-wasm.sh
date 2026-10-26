#!/bin/sh
# Builds the specimen-wasm module with the pinned rustup toolchain.
# Homebrew's cargo/rustc ignore rust-toolchain.toml and ship no wasm32 std,
# and `rustup run` does not put the toolchain's own rustc first on PATH, so
# the pinned toolchain's bin directory is prepended explicitly. No install or
# network fallback: a missing toolchain or target fails here.
set -eu
cd "$(dirname "$0")/.."
toolchain=1.99.1
cargo_bin=$(rustup which --toolchain "$toolchain" cargo)
PATH="$(dirname "$cargo_bin"):$PATH"
export PATH
case "$(rustc --version)" in
  "rustc $toolchain "*) ;;
  *) echo "pinned Rust $toolchain not selected: $(rustc --version)" >&2; exit 1 ;;
esac
cargo build --locked -p specimen-wasm --target wasm32-unknown-unknown --release
mkdir -p lib/specimen/wasm
cp -f "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/specimen_wasm.wasm" lib/specimen/wasm/
