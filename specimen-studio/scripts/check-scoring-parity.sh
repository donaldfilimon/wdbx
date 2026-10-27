#!/usr/bin/env bash
# Explicit opt-in, cached dependencies only; all build artifacts are retained.
set -euo pipefail
preservation_mode=current
case "$#:$*" in
  0:) ;;
  1:--historical-preservation) preservation_mode=historical ;;
  *) echo 'usage: check-scoring-parity.sh [--historical-preservation]' >&2; exit 2 ;;
esac
cd "$(dirname "$0")/.."
export RUSTUP_TOOLCHAIN=1.99.1
export CARGO_NET_OFFLINE=true
toolchain_lib="$(rustc --print sysroot)/lib"
export DYLD_LIBRARY_PATH="$toolchain_lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
toolchain_host="$(rustc -vV | sed -n 's/^host: //p')"
objcopy="$toolchain_lib/rustlib/$toolchain_host/bin/rust-objcopy"
case "$toolchain_host" in *windows*) objcopy="$objcopy.exe" ;; esac
for tool in cargo rustc bun shasum mktemp cp python3 rg; do
  command -v "$tool" >/dev/null || { echo "missing required tool: $tool" >&2; exit 1; }
done
[[ "$(rustc --version)" == 'rustc 1.99.1 '* ]] || {
  echo 'caller PATH must select pinned Rust 1.99.1' >&2; exit 1;
}
artifacts=$(mktemp -d "${TMPDIR:-/tmp}/task-8-scoring-parity.XXXXXX")
trap 'echo "retained-artifacts=$artifacts"' EXIT
echo "source-head=$(git rev-parse HEAD)"
echo "source-root=$(pwd)"
echo "PATH=$PATH"
echo "RUSTUP_TOOLCHAIN=$RUSTUP_TOOLCHAIN CARGO_NET_OFFLINE=$CARGO_NET_OFFLINE"
echo "DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH"
printf '%s\n' "PATH=$PATH" "RUSTUP_TOOLCHAIN=$RUSTUP_TOOLCHAIN" "CARGO_NET_OFFLINE=$CARGO_NET_OFFLINE" "DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH" > "$artifacts/environment.txt"
"$objcopy" --version > "$artifacts/objcopy-load.log"
cat "$artifacts/objcopy-load.log"
if [[ "$toolchain_host" == *apple-darwin ]]; then
  shasum -a 256 "$toolchain_lib/libLLVM.dylib" > "$artifacts/loader-library.sha256"
else
  echo "Darwin loader evidence not applicable: $toolchain_host" > "$artifacts/loader-library.sha256"
fi
echo "cargo=$(command -v cargo)"
cargo --version
echo "rustc=$(command -v rustc)"
rustc -vV
echo "bun=$(command -v bun)"
bun --version
shasum -a 256 "$(command -v cargo)" "$(command -v rustc)" "$(command -v bun)" "$(command -v python3)" "$objcopy" > "$artifacts/tools.sha256"
cat "$artifacts/tools.sha256"
rg --files native/specimen-kernel/src/identity native/specimen-kernel/src/language_v3 \
  native/specimen-kernel/src/text_v3 native/specimen-kernel/src/scanner_v3 \
  native/specimen-kernel/src/scoring_v3 native/specimen-kernel/src/orchestration_v3 \
  native/specimen-wasm/src native/specimen-wasm/examples native/specimen-wasm/tests \
  | sort | while IFS= read -r path; do shasum -a 256 "$path"; done > "$artifacts/source.sha256"
shasum -a 256 Cargo.toml Cargo.lock native/specimen-wasm/Cargo.toml native/specimen-kernel/src/logical_charge_v1.rs native/specimen-kernel/src/lib.rs \
  scripts/check-scoring-parity.sh tests/scoring-parity.mjs >> "$artifacts/source.sha256"
shasum -a 256 scripts/check_preservation.py scripts/tests/test_check_preservation.py \
  ../docs/completion/execution/evidence/2026-10-26-studio-preservation-transition.json \
  ../docs/completion/execution/evidence/2026-10-22-language/legacy-baseline.json \
  ../docs/completion/execution/evidence/2026-10-23-scoring/preservation-baseline.json \
  >> "$artifacts/source.sha256"
cat "$artifacts/source.sha256"
set -x
cargo run --offline --locked -p specimen-wasm --features scoring-probe --example scoring_probe \
  --target-dir "$artifacts/target" > "$artifacts/native.json"
shasum -a 256 "$artifacts/target/debug/examples/scoring_probe"
cargo build --offline --locked -p specimen-wasm --target wasm32-unknown-unknown --release \
  --no-default-features --target-dir "$artifacts/target"
cp "$artifacts/target/wasm32-unknown-unknown/release/specimen_wasm.wasm" "$artifacts/default.wasm"
cargo build --offline --locked -p specimen-wasm --target wasm32-unknown-unknown --release \
  --no-default-features --features scoring-probe --target-dir "$artifacts/target"
cp "$artifacts/target/wasm32-unknown-unknown/release/specimen_wasm.wasm" "$artifacts/scoring.wasm"
bun tests/scoring-parity.mjs "$artifacts/native.json" "$artifacts/scoring.wasm" \
  "$artifacts/default.wasm" "$artifacts/wasm.json" "$artifacts/corrupt-arithmetic.json" \
  "$artifacts/corrupt-quarantine.json" "$artifacts/corrupt-boundary.json"
bun tests/scoring-parity.mjs --self-test "$artifacts/native.json"
for kind in arithmetic quarantine boundary; do
  status=0
  bun tests/scoring-parity.mjs --check-json "$artifacts/corrupt-$kind.json" \
    > "$artifacts/negative-$kind.log" 2>&1 || status=$?
  echo "negative-$kind-comparator-exit=$status"
  [[ "$status" -ne 0 ]] || { echo "FAIL: normal comparator accepted $kind corruption" >&2; exit 1; }
  if [[ "$kind" == arithmetic ]]; then
    rg -q 'exact confidence/gate effective' "$artifacts/negative-$kind.log"
  elif [[ "$kind" == quarantine ]]; then
    rg -q 'broker independent transition cancel/quarantined' "$artifacts/negative-$kind.log"
  else
    rg -q 'fixed boundary actual refusal/outcome' "$artifacts/negative-$kind.log"
  fi
  cat "$artifacts/negative-$kind.log"
done
shasum -a 256 "$artifacts/native.json" "$artifacts/wasm.json" \
  "$artifacts/native.json.semantic.json" "$artifacts/wasm.json.semantic.json" \
  "$artifacts/default.wasm" "$artifacts/scoring.wasm" \
  "$artifacts/target/debug/examples/scoring_probe" \
  "$artifacts/corrupt-arithmetic.json" "$artifacts/corrupt-quarantine.json" "$artifacts/corrupt-boundary.json" \
  "$artifacts/negative-arithmetic.log" "$artifacts/negative-quarantine.log" "$artifacts/negative-boundary.log" \
  "$artifacts/environment.txt" "$artifacts/objcopy-load.log" "$artifacts/loader-library.sha256" \
  > "$artifacts/artifacts.sha256"
cat "$artifacts/artifacts.sha256"
python3 scripts/check_preservation.py --profile scoring --mode "$preservation_mode"
echo 'Evidence limit: finite synthetic pure mechanisms; default WASM compile/export inspection only; feature WASM executed with throwing imports and zero actual host calls. No physical concurrency, host fencing, efficacy, installed execution, ABI acceptance or production cutover claim; every requirement remains open/C0/unknown.'
