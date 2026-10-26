# P1 dependency decision proposal: bounded local experiments

2026-10-22. **Proposed only. Not approval, ABI ratification, Slice 2 acceptance,
or production cryptographic qualification.** This records the candidate and a
known differential failure before implementation review. It does not change the
frozen completion contracts or the authoritative [P1 design](../../superpowers/specs/2026-10-20-wdbx-cose-trust-prototype-design.md).

## Candidate and observed runtime

Use existing locked Rust `ed25519-dalek` 3.0.0 with `curve25519-dalek` 5.0.0;
use the already installed independent Python PyCA `cryptography` 50.0.2 with
OpenSSL 4.0.3 for experimental message signing and verification. No package was
installed, upgraded, or added to Cargo. No production keys or live stores were
read. No COSE dependency is proposed: retain the exact bounded private parser
and independently construct the closed P1 structure in Python. This is not a
general COSE implementation or a claim that library selection review is complete.

Structural-library evaluation: a general library such as `coset` would still
need the profile's bounded preflight, closed tag/header rules and exact raw
protected-byte preservation; using one does not supply those guarantees. For
this private experiment, reuse the existing dedicated bounded P1 parser and
checked structure encoder and independently implement the same narrow profile
in Python. Add no generic COSE dependency or relaxed decoder. This chooses the
smaller reviewed surface for the experiment, not a claim about a `coset` release
or a final production dependency decision.

The bundled Python is 3.12.14, at
`/Users/donald/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3`.
Its observed installed dependencies are cffi 2.1.1 and pycparser 3.11. The PyCA
wheel metadata names `cp311-abi3-macosx_11_0_arm64`; cffi names
`cp312-cp312-macosx_11_0_arm64`. This observation establishes local execution,
not all-platform support. System Homebrew Python 3.15 lacks PyCA. PyNaCl,
pysodium and sodium were absent from the bundled runtime; libsodium was absent
from the inspected Homebrew, `/usr/local/lib`, and bundled library directories.
No alternate backend has been installed or approved.

[Exact inventory](evidence/2026-10-22-continuation/crypto-dependencies/inventory.json)
pins distribution metadata, native module, Python executable, Cargo lock,
local Rust manifest and probe-library hashes. These identify observed bytes;
they do not authenticate upstream wheel provenance. PyCA is Apache-2.0 OR
BSD-3-Clause; cffi is MIT-0; pycparser and both dalek crates are BSD-3-Clause.
Both dalek crates declare Rust 1.85 minimum, below this checkout's pinned 1.99.1.
PyCA declares Python >=3.9 excluding 3.9.0/3.9.1; the installed cffi and pycparser
require >=3.10. The inventory records additional locked Rust support packages
and PyCA's declared Python dependencies, not a complete native dependency audit.
The inventory also hashes raw `cargo metadata --locked --offline` and
`cargo tree -p abi-wdbx --locked --offline --edges normal,build` artifacts for
the complete resolved default workspace and host/default-feature graph. These
do not qualify optional FHE, every target/feature combination or native backend
dependencies. Package-file hashes and registry checksums are observations, not
an authenticated upstream release provenance attestation.

## API constraints and known disagreement

Rust must use pure-message `SigningKey` signing and `VerifyingKey::verify_strict`
over the complete P1 `Sig_structure`. Do not use prehashed, streaming, batch,
ordinary lax verification, caller-selected AAD, or legacy digest signatures.
`VerifyingKey::from_bytes` uses ZIP-215 point acceptance; it alone does not
reject weak points. P1 key handling must reject `is_weak()` and key-ID mismatch;
strict verification independently rejects low-order public keys and R.

Python must independently parse the exact detached P1 container, retain the
original protected and canonical payload bytes, and reconstruct the complete
`Sig_structure`. Use `Ed25519PrivateKey.from_private_bytes(...).sign(message)`
and `Ed25519PublicKey.from_public_bytes(...).verify(signature, message)`.
Catch `InvalidSignature` as a backend refusal; missing library/interpreter,
unsupported algorithm, or unexpected backend failures must fail the invoked
witness. Python construction of a 32-byte public key is not point validation.
No hand-written Edwards arithmetic or signature checking is permitted.

The raw probes found a material difference:

- Public key is `01` followed by 31 zero bytes (identity point).
- Signature is `01` followed by 63 zero bytes (identity R and zero S).
- Message is hex `70726f6265` (`probe`).
- PyCA/OpenSSL **accepts**; dalek reports a weak key and **refuses** strict
  verification. Ordinary synthetic positive, tampering, message substitution,
  noncanonical scalar, invalid R and truncation cases behave as the probe logs
  record.

The [Python probe and raw output](evidence/2026-10-22-continuation/crypto-dependencies/probe.log)
and [Rust strict output](evidence/2026-10-22-continuation/crypto-dependencies/rust-probe.log)
retain the exact evidence. The scripts sit beside those logs. Rust was compiled
against the locally built compatible pinned-toolchain dalek rlib named in the
inventory; a more recent Homebrew-built rlib was incompatible with the pinned
compiler and was not used. Neither probe is a P1 integration test.

This disagreement leaves strict cross-language qualification **failed/open**.
Do not weaken Rust, silently filter backend disagreement, add custom point
arithmetic, or report ordinary positives as full parity. A separate successful
characterization test may pin the disagreement; its success is not conformance
success. Evaluating a maintained strict alternative such as PyNaCl/libsodium
requires a separately reviewed exact version/backend and differential vectors.

## Proposed witness boundary and next gate

Keep the existing standard-library-only default gate independent of PyCA.
Use an explicit test-only `p1-crypto-witness` Cargo feature and dedicated
`tools/check_p1_crypto.sh`, selecting `${WDBX_CRYPTO_PYTHON:-python3}`.
An explicitly invoked witness must hard-fail on a missing dependency or
interpreter and report its exact dependency/backend identity. Do not silently
choose a fallback Python or crypto backend. The bundled runtime can be selected
explicitly for the local run. Default root-gate success is not Slice 2 evidence.

The private verifier must retain expected domain/version/contract, tenant/scope,
origin signer, local trust namespace and immutable snapshot identity bindings;
a signature result alone grants no authority. Retired/revoked mathematical
resolution and current admission remain distinct later slices. No public
interface, trust persistence, admission, wire freeze, or crypto qualification
is authorized by this proposal.

Before calling Slice 2 accepted, independently review the exact dependency and
source decision, preserve positive/malformed vectors for kid/key/origin,
algorithm -8/-19, AAD/payload substitution, weak points, noncanonical R/S,
Ed25519ph confusion and legacy transplants, and resolve the observed strict
backend disagreement under the authoritative design. Named ABI and independent
crypto reviews remain external gates.

## Advisory scope and primary references

The live [OSV request and response](evidence/2026-10-22-continuation/crypto-dependencies/osv-query.json)
records its UTC timestamp and exact five-version scope. All five returned empty
results. This is not a full transitive, native binary, operating-system, supply
chain or cryptographic audit, and does not establish absence of vulnerabilities.
RustSec's historical double-public-key signing issue is patched in >=2; the
candidate is 3.0.0 and does not use hazmat decoupled signing inputs.

Primary sources inspected on 2026-10-22:

- [PyCA 50.0.2 Ed25519 API](https://cryptography.io/en/50.0.2/hazmat/primitives/asymmetric/ed25519/).
- [PyCA 50.0.2 release notes](https://cryptography.io/en/50.0.2/changelog/): official wheels updated to OpenSSL 4.0.3.
- [PyCA security guidance](https://cryptography.io/en/50.0.2/security/) and [maintainer advisories](https://github.com/pyca/cryptography/security/advisories).
- [Dalek 3.0.0 verifying-key API](https://docs.rs/ed25519-dalek/3.0.0/ed25519_dalek/struct.VerifyingKey.html), checked against locally cached source.
- [RUSTSEC-2022-0093](https://rustsec.org/advisories/RUSTSEC-2022-0093.html).
- [OpenSSL vulnerability inventory](https://openssl-library.org/news/vulnerabilities/); not a substitute for backend qualification.
- [Libsodium maintained signing API](https://doc.libsodium.org/public-key_cryptography/public-key_signatures), alternative candidate only, not tested or selected.
