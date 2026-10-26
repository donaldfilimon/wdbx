# Independent libsodium witness dependency decision

2026-10-22. Reviewed proposal for a **bounded private local experiment** only.
No installation or execution of the downloaded wheel occurred in this review.
No ABI ratification, Slice 2 acceptance, production cryptographic qualification,
or target-platform qualification follows.

## Exact candidate and review

Select PyNaCl 1.6.2, Apache-2.0, Python >=3.8, official PyPI wheel
`pynacl-1.6.2-cp38-abi3-macosx_10_10_universal2.whl`, SHA256
`c949ea47e4206af7c8f604b8278093b674f7c79ed0d4719cc836902bf4517465`.
Downloaded into `/tmp`, checked against official release JSON, and inspected as
ZIP data without importing or executing it. The CPython abi3 universal2 wheel
is a candidate for this arm64 Python 3.12 host; actual loading remains untested.
Pin its active Python dependencies to cffi 2.1.1 and pycparser 3.11; their exact
macOS arm64 / universal Python wheel URLs and SHA256 values are in
[evidence](evidence/2026-10-22-continuation/libsodium-dependencies/requirements-local-arm64.txt).
This avoids inheriting arbitrary runtime site-packages. Existing bundled Python
3.12.14 is the interpreter; do not alter its installed packages.

PyNaCl's exact 1.6.2 changelog states its bundled libsodium is 1.0.20-stable,
2025-12-31 build, addressing CVE-2025-69277. Do not infer the fix merely from a
runtime `1.0.20` version string. Retain the exact selected wheel identity.
PyNaCl is Apache-2.0, bundled libsodium ISC, cffi MIT-0, pycparser BSD-3-Clause.
The captured metadata and tagged upstream source record this bounded review;
wheel hashing does not independently authenticate upstream build provenance.

Inspected wheel source `VerifyKey.verify(message, signature)` prefixes the
64-byte signature to the exact message, then calls `nacl.bindings.crypto_sign_open`.
The binding calls native `lib.crypto_sign_open` and raises `BadSignatureError`
on refusal. Tagged vendored libsodium source routes to pure-message Ed25519
verification, not Ed25519ph; it checks scalar canonicality, canonical public-key
encoding and small-order points in its default (non-ED25519_COMPAT) source branch
without application-written curve arithmetic. This static source inspection does
not prove the selected wheel's compile-time branch; execution must test it.
Construction of `VerifyKey` itself is not trusted key registration. Rust still
requires derived key identity checks, weak-key rejection and `verify_strict`.
No hand-written point arithmetic or silent backend substitution is allowed.

This backend is a plausible independent strict alternative to the observed PyCA
identity-point acceptance. **Parity has not been tested.** It must reject the
exact retained identity-key/signature probe and agree on the full P1 positive,
malformed-point/scalar, altered-message/AAD and legacy-transplant matrix before
any local parity claim. Unexpected divergence is a failing qualification result;
never weaken dalek or filter the disagreement. Preserve the PyCA failed result
as a separate backend characterization even if libsodium later agrees.

## Isolated installation recipe for subsequent execution

This review approves the following exact, reversible local experimental install
for subsequent execution. It has not run these commands. The venv must be newly
created without system site-packages; no source builds, dependency resolution,
global install, runtime mutation, or user configuration is used.

```sh
P1_ENV_PARENT=$(mktemp -d /tmp/wdbx-p1-libsodium.XXXXXX)
/Users/donald/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3 -m venv "$P1_ENV_PARENT/venv"
"$P1_ENV_PARENT/venv/bin/python" -m pip --isolated install --disable-pip-version-check --no-cache-dir --only-binary=:all: --no-deps --require-hashes -r docs/completion/execution/evidence/2026-10-22-continuation/libsodium-dependencies/requirements-local-arm64.txt
"$P1_ENV_PARENT/venv/bin/python" -m pip --isolated check
```

Use the resulting exact interpreter explicitly for the separately invoked
witness; missing imports, failed loading or unexpected exceptions fail rather
than skip. Capture interpreter, installed distribution metadata and native-module
hashes before the experiment, then direct positive cross-signatures and rejection
vectors. Do not run the witness against an implicitly selected interpreter.
No automatic backend fallback is allowed. The default root gate stays independent
of this experimental dependency.

## Advisory scope and evidence

The captured OSV query checked exact PyNaCl 1.6.2, cffi 2.1.1 and pycparser 3.11;
all returned empty results. This is not a complete native/transitive/OS audit or
an absence-of-vulnerabilities assertion. No install or execution is authorized
as production acceptance. Independent review of source, dependencies, vectors,
key custody and the authoritative P1 owner gate remain open.

Primary sources inspected:

- [Official PyPI 1.6.2 release metadata](https://pypi.org/pypi/PyNaCl/1.6.2/json).
- [Exact release changelog](https://github.com/pyca/pynacl/blob/1.6.2/CHANGELOG.rst).
- [Tagged pure-message verification implementation](https://github.com/pyca/pynacl/blob/1.6.2/src/libsodium/src/libsodium/crypto_sign/ed25519/ref10/open.c).
- [Maintainer signing API](https://pynacl.readthedocs.io/en/latest/signing/).
- [CVE advisory](https://github.com/advisories/GHSA-mrfv-m5wm-5w6w).

Captured PyPI metadata, wheel source/metadata, tagged native verifier source,
advisory query and hash-locked recipe are under
`evidence/2026-10-22-continuation/libsodium-dependencies/`.
