#!/bin/sh
# Runs cargo from the rustup toolchain pinned in ./rust-toolchain.toml.
# Homebrew's cargo/rustc ignore rust-toolchain.toml and ship no wasm32 std,
# and `rustup run` does not put the toolchain's own rustc first on PATH, so
# the pinned toolchain's bin directory is prepended explicitly. Inherited
# RUSTC/RUSTC_WRAPPER would override the compiler cargo invokes, so they are
# removed. No install or network fallback: a missing toolchain fails here.
set -eu
studio=$(cd "$(dirname "$0")/.." && pwd)
toolchain=$(sed -n 's/^channel *= *"\([^"]*\)".*/\1/p' "$studio/rust-toolchain.toml")
[ -n "$toolchain" ] || { echo "no channel in $studio/rust-toolchain.toml" >&2; exit 1; }
for var in RUSTC RUSTC_WRAPPER; do
  if eval "[ -n \"\${$var:-}\" ]"; then
    echo "pinned-cargo: ignoring inherited $var" >&2
    unset "$var"
  fi
done
cargo_bin=$(rustup which --toolchain "$toolchain" cargo)
PATH="$(dirname "$cargo_bin"):$PATH"
RUSTUP_TOOLCHAIN=$toolchain
export PATH RUSTUP_TOOLCHAIN
case "$(rustc --version)" in
  "rustc $toolchain "*) ;;
  *) echo "pinned Rust $toolchain not selected: $(rustc --version)" >&2; exit 1 ;;
esac
exec cargo "$@"
