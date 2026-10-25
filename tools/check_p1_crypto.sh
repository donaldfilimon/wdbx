#!/usr/bin/env bash
# Explicit private P1 witness gate; default tools/check.sh does not run this.
# Caller selects pinned Rust on PATH and an interpreter. No fallback/install.
set -euo pipefail
cd "$(dirname "$0")/.."
crypto_python="${WDBX_CRYPTO_PYTHON:-python3}"
crypto_backend="${WDBX_CRYPTO_BACKEND:-pyca}"
case "$crypto_backend" in
  pyca|libsodium) ;;
  *) printf 'Unsupported explicit P1 crypto backend\n' >&2; exit 2 ;;
esac
export WDBX_CRYPTO_PYTHON="$crypto_python"
export WDBX_CRYPTO_BACKEND="$crypto_backend"
"$crypto_python" -m unittest discover -s tools/tests -p test_wdbx_complete_p1_crypto.py
cargo test -p abi-wdbx --features p1-crypto-witness complete_episode::crypto
if [[ "$crypto_backend" == pyca ]]; then
  # Retained known disagreement deliberately fails this invoked checker.
  "$crypto_python" tools/wdbx_complete_p1_crypto.py backend-probe --qualify
else
  # A single backend probe never qualifies anything. Only report this bounded
  # paired suite after both explicit Python and Rust matrices have passed.
  "$crypto_python" - <<'PY'
import json
import sys
sys.path.insert(0, 'tools')
import wdbx_complete_p1_crypto as witness
probe = witness.backend_probe()
print(json.dumps(probe, sort_keys=True))
if probe['selected_backend'] != 'libsodium' or probe['raw_weak_key_accepted']:
    raise SystemExit(1)
print('Unratified bounded P1 differential matrix passed; production qualification remains open.')
PY
fi
