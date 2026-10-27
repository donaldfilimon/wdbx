#!/bin/sh
# Builds the specimen-wasm module through scripts/pinned-cargo.sh (pinned
# rustup toolchain from rust-toolchain.toml) and copies it from the target
# directory cargo actually uses (CARGO_TARGET_DIR, CARGO_BUILD_TARGET_DIR or
# build.target-dir, as reported by `cargo metadata`).
set -eu
cd "$(dirname "$0")/.."
cargo="sh scripts/pinned-cargo.sh"
$cargo build --locked -p specimen-wasm --target wasm32-unknown-unknown --release
target_dir=$($cargo metadata --locked --offline --no-deps --format-version 1 |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')
mkdir -p lib/specimen/wasm
cp -f "$target_dir/wasm32-unknown-unknown/release/specimen_wasm.wasm" lib/specimen/wasm/
