#!/usr/bin/env bash
# Explicit opt-in; every build stays in a retained temporary target directory.
set -euo pipefail
preservation_mode=current
case "$#:$*" in
  0:) ;;
  1:--historical-preservation) preservation_mode=historical ;;
  *) echo 'usage: check-language-parity.sh [--historical-preservation]' >&2; exit 2 ;;
esac
cd "$(dirname "$0")/.."
export RUSTUP_TOOLCHAIN=1.99.1
export CARGO_NET_OFFLINE=true
for tool in cargo rustc bun shasum mktemp cp python3 rg; do
  command -v "$tool" >/dev/null || { echo "missing required tool: $tool" >&2; exit 1; }
done
[[ "$(rustc --version)" == 'rustc 1.99.1 '* ]] || {
  echo 'caller PATH must select pinned Rust 1.99.1' >&2; exit 1;
}
artifacts=$(mktemp -d "${TMPDIR:-/tmp}/task-8-language-parity.XXXXXX")
trap 'echo "retained-artifacts=$artifacts"' EXIT
echo "source-head=$(git rev-parse HEAD)"
echo "source-root=$(pwd)"
echo "PATH=$PATH"
echo "RUSTUP_TOOLCHAIN=$RUSTUP_TOOLCHAIN CARGO_NET_OFFLINE=$CARGO_NET_OFFLINE"
echo "cargo=$(command -v cargo)"
cargo --version
echo "rustc=$(command -v rustc)"
rustc -vV
echo "bun=$(command -v bun)"
bun --version
shasum -a 256 "$(command -v cargo)" "$(command -v rustc)" "$(command -v bun)"
rg --files native/specimen-kernel/src/identity native/specimen-kernel/src/language_v3 \
  native/specimen-kernel/src/text_v3 native/specimen-kernel/src/scanner_v3 \
  native/specimen-wasm/src native/specimen-wasm/examples native/specimen-wasm/tests \
  | sort | while IFS= read -r path; do shasum -a 256 "$path"; done > "$artifacts/source.sha256"
shasum -a 256 Cargo.toml Cargo.lock native/specimen-wasm/Cargo.toml \
  scripts/check-language-parity.sh tests/language-parity.mjs >> "$artifacts/source.sha256"
shasum -a 256 scripts/check_preservation.py scripts/tests/test_check_preservation.py \
  ../docs/completion/execution/evidence/2026-10-26-studio-preservation-transition.json \
  ../docs/completion/execution/evidence/2026-10-22-language/legacy-baseline.json \
  ../docs/completion/execution/evidence/2026-10-23-scoring/preservation-baseline.json \
  >> "$artifacts/source.sha256"
cat "$artifacts/source.sha256"
set -x
cargo run --locked -p specimen-wasm --features language-probe --example language_probe \
  --target-dir "$artifacts/target" > "$artifacts/native.json"
shasum -a 256 "$artifacts/target/debug/examples/language_probe"
cargo build --locked -p specimen-wasm --target wasm32-unknown-unknown --release \
  --no-default-features --target-dir "$artifacts/target"
cp "$artifacts/target/wasm32-unknown-unknown/release/specimen_wasm.wasm" "$artifacts/default.wasm"
cargo build --locked -p specimen-wasm --target wasm32-unknown-unknown --release \
  --no-default-features --features language-probe --target-dir "$artifacts/target"
cp "$artifacts/target/wasm32-unknown-unknown/release/specimen_wasm.wasm" "$artifacts/language.wasm"
bun tests/language-parity.mjs "$artifacts/native.json" "$artifacts/language.wasm" \
  "$artifacts/default.wasm" "$artifacts/wasm.json" "$artifacts/corrupt-digest.json"
bun tests/language-parity.mjs --self-test "$artifacts/native.json"
negative_status=0
bun tests/language-parity.mjs --check-json "$artifacts/corrupt-digest.json" > "$artifacts/negative.log" 2>&1 || negative_status=$?
echo "negative-comparator-exit=$negative_status"
if [[ "$negative_status" -eq 0 ]]; then
  echo 'FAIL: comparator accepted corrupted full32 digest' >&2
  exit 1
fi
rg -q 'full32 digest' "$artifacts/negative.log"
cat "$artifacts/negative.log"
echo 'PASS: load-bearing corrupted digest refused by same comparator with nonzero exit'
label_status=0
bun tests/language-parity.mjs --check-json "$artifacts/corrupt-digest.json.label.json" > "$artifacts/negative-label.log" 2>&1 || label_status=$?
echo "negative-label-comparator-exit=$label_status"
if [[ "$label_status" -eq 0 ]]; then
  echo 'FAIL: comparator accepted corrupted scanner component name' >&2
  exit 1
fi
rg -q 'ordered scanner component names/count' "$artifacts/negative-label.log"
cat "$artifacts/negative-label.log"
echo 'PASS: load-bearing corrupted label refused by same comparator with nonzero exit'

shasum -a 256 "$artifacts/native.json" "$artifacts/default.wasm" "$artifacts/language.wasm" \
  "$artifacts/wasm.json" "$artifacts/corrupt-digest.json" "$artifacts/negative.log" \
  "$artifacts/corrupt-digest.json.label.json" "$artifacts/negative-label.log" > "$artifacts/artifacts.sha256"
cat "$artifacts/artifacts.sha256"
python3 scripts/check_preservation.py --profile language --mode "$preservation_mode"
echo 'Evidence limit: synthetic pure API execution only; default WASM compiled/export-inspected, not instantiated; no production cutover, efficacy, effect hosting, migration, scheduling or target qualification.'
