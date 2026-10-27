#!/usr/bin/env bash
# Explicit opt-in; retained temp artifacts never overwrite browser build output.
set -euo pipefail
cd "$(dirname "$0")/.."
export RUSTUP_TOOLCHAIN=1.99.1
for tool in cargo rustc bun shasum mktemp cp; do
  command -v "$tool" >/dev/null || { echo "missing required tool: $tool" >&2; exit 1; }
done
[[ "$(rustc --version)" == 'rustc 1.99.1 '* ]] || {
  echo 'caller PATH must select pinned Rust 1.99.1' >&2; exit 1;
}
artifacts=$(mktemp -d "${TMPDIR:-/tmp}/task-8-identity-parity.XXXXXX")
trap 'echo "retained-artifacts=$artifacts"' EXIT
echo "source-head=$(git rev-parse HEAD)"
echo "source-root=$(pwd)"
echo "cargo=$(command -v cargo)"
cargo --version
echo "rustc=$(command -v rustc)"
rustc -vV
echo "bun=$(command -v bun)"
bun --version
shasum -a 256 "$(command -v cargo)" "$(command -v rustc)" "$(command -v bun)"
shasum -a 256 Cargo.toml Cargo.lock \
  native/specimen-wasm/Cargo.toml native/specimen-wasm/src/lib.rs \
  native/specimen-wasm/src/identity_probe.rs native/specimen-wasm/examples/identity_probe.rs \
  native/specimen-wasm/tests/identity_probe.rs \
  native/specimen-kernel/src/identity/*.rs native/specimen-kernel/tests/identity_fixture.rs \
  scripts/check-identity-parity.sh tests/identity-parity.mjs
cargo run --locked -p specimen-wasm --features identity-probe --example identity_probe \
  --target-dir "$artifacts/target" > "$artifacts/native.json"
shasum -a 256 "$artifacts/target/debug/examples/identity_probe"
cargo build --locked -p specimen-wasm --target wasm32-unknown-unknown --release \
  --no-default-features --target-dir "$artifacts/target"
cp "$artifacts/target/wasm32-unknown-unknown/release/specimen_wasm.wasm" "$artifacts/default.wasm"
cargo build --locked -p specimen-wasm --target wasm32-unknown-unknown --release \
  --features identity-probe --target-dir "$artifacts/target"
cp "$artifacts/target/wasm32-unknown-unknown/release/specimen_wasm.wasm" "$artifacts/identity.wasm"
bun tests/identity-parity.mjs "$artifacts/native.json" "$artifacts/identity.wasm" \
  "$artifacts/default.wasm" "$artifacts/wasm.json"
echo 'Evidence limit: synthetic pure-kernel identity mechanisms only; no encoder, save migration, model/hardware or full Task 8 qualification.'
