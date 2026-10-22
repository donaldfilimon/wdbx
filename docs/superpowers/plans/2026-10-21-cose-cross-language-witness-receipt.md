# Private COSE cross-language *encoding* witness receipt

2026-10-21. A second-language **encoding** witness now exists for the private
crate-internal `COSE_Sign1` prototype in `crates/abi-wdbx/src/v3/episode/cose/`.
It proves that the container bytes and the RFC 9052 `Sig_structure` bytes mean
the same thing in Rust and in Python. It does **not** prove, and does not
claim, cross-language signature verification: the Python side does not verify
signatures and cannot, because this environment has no Python cryptography
library (`cryptography` and `pynacl` are both absent) and this repository's
design guidance forbids hand-rolling cryptography in Python. Nothing under
`src/v3/episode/cose/` was modified, no dependency was added, and the
committed ledger format and every committed digest are unchanged.

## What was added

| Artifact | Role |
| --- | --- |
| `tools/wdbx_cose_sign1_p1.py` | standard-library-only Python witness: bounded container reader, bounded protected-header reader, RFC 9052 `Sig_structure` rebuild, `verify-goldens` and a strict JSON-lines protocol |
| `crates/abi-wdbx/tests/v3_cross_language_cose.rs` | the Rust↔Python differential driver (10 tests) |
| `crates/abi-wdbx/tests/golden/cose-sign1-p1/` | the pinned vectors: `container.hex`, `container.sha256`, `protected.hex`, `payload.hex`, `kid.hex`, `alg.txt`, `sig_structure.hex`, `sig_structure.sha256`, `README.md` |
| `tools/tests/test_wdbx_cose_sign1_p1.py` | the witness's own unittest suite (28 tests) |
| `tools/check.sh` | one extra gate step running that suite |

The prototype's own golden container (`container.hex`, `container.sha256`) was
already present and is unchanged: the same synthetic seed (`0x01` repeated) and
the same synthetic digest (`0x5a` repeated) produce the same 183-byte
container.

## The pinned vectors

The container is `d2 84 58 4e <78 bytes> a0 58 20 <32 bytes> 58 40 <64 bytes>`:
tag 18, an array of four elements, a 78-byte protected header
`a2 01 32 04 58 48 <72-byte kid>`, an empty unprotected map, the 32-byte
episode digest as the payload, and the 64-byte Ed25519 signature over the
`Sig_structure`.

| Golden file | SHA-256 of the file | What it pins |
| --- | --- | --- |
| `container.hex` | `605ed0e0962a33979f7ce5f22a15f77d82a0d90d23e4e14fff54e5aac9b371da` | the container bytes; their content digest is `ef0a47cd04fbcebb4343a9952c84d05cfce6c4336c4d2fb72fa96e0e87392884` |
| `container.sha256` | `943f736cf719c64856de0c6df4bc6bc49592030cce46b865fff4f6eb6188cf95` | that content digest |
| `protected.hex` | `72b06fa1479b92c4bdcc9073fb99a9484fd1f63e085fb85792087728ae908741` | the 78 protected-header bytes |
| `payload.hex` | `da1be6f38a667d45b3096240f14e835aa4264cb6728641b92362b7d0a0fb292f` | the 32 payload bytes (`0x5a` repeated) |
| `kid.hex` | `797ddbe6e440d5aa6fc66f534eeec777ade63acf902ae69dd9721f7312d84e0a` | the 72 key-identifier bytes (ASCII `ed25519:34750f98bd59fcfc946da45aaabe933be154a4b5094e1c4abf42866505f3c97e`) |
| `alg.txt` | `8adce69f5bee54553e3aaeb68824c94fb1787a5a2472d7ad6802c716a4ecb8a5` | the algorithm identifier `-19` |
| `sig_structure.hex` | `811a5e70fbf6f4718d276a278ffc3060bbc11c8e22d1673978fde17fecfbdf2c` | the 145 `Sig_structure` bytes; their content digest is `d5731e9465e2d3a4d4fd78db5284976bc848b1fb57eae615f7ad093bc97a240e` |
| `sig_structure.sha256` | `1fc8052c9e66e39cbb731c6d94eeb3d06c60f14a558035ba38b17ef90f74822f` | that content digest |

The `Sig_structure` is RFC 9052's `["Signature1", protected, external_aad,
payload]` in the crate's canonical encoding: `84 6a "Signature1" 58 4e
<protected> 52 "wdbx-cose-sign1-p1" 58 20 <payload>`. The external AAD is 18
bytes, so its canonical byte-string head is the one-byte form `52`, not
`58 12`.

## Commands and their output

```console
$ PATH="$HOME/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:$PATH" \
    cargo test -p abi-wdbx --test v3_cross_language_cose
running 10 tests
test v3_cross_language_cose::a_missing_witness_interpreter_is_a_hard_failure_not_a_skip ... ok
test v3_cross_language_cose::a_content_mutation_keeps_the_shape_and_breaks_only_the_rust_side_check ... ok
test v3_cross_language_cose::a_mutated_container_is_refused_by_the_python_parser_too ... ok
test v3_cross_language_cose::both_sides_refuse_the_same_malformed_containers ... ok
test v3_cross_language_cose::python_and_rust_agree_on_every_encoding_fact ... ok
test v3_cross_language_cose::rust_side_verification_succeeds ... ok
test v3_cross_language_cose::the_container_bytes_are_pinned_by_the_golden ... ok
test v3_cross_language_cose::the_signed_bytes_are_the_agreed_sig_structure ... ok
test v3_cross_language_cose::the_witness_runner_fails_loudly_without_an_interpreter ... ok
test v3_cross_language_cose::the_witness_verifies_the_golden_vectors_standalone ... ok
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

```console
$ python3 tools/wdbx_cose_sign1_p1.py verify-goldens
ok: container: sha256(golden bytes) == container.sha256
ok: container: protected bytes == protected.hex
ok: container: payload bytes == payload.hex
ok: container: kid bytes == kid.hex
ok: container: alg == alg.txt
ok: protected: the canonical header of {1: alg, 4: kid} == the carried bytes
ok: sig_structure: the rebuilt structure == sig_structure.hex
ok: sig_structure: sha256(rebuilt structure) == sig_structure.sha256
0 failure(s)
```

```console
$ python3 -m unittest discover -s tools/tests -p 'test_wdbx_cose_sign1_p1.py'
............................
Ran 28 tests in 0.051s

OK
```

```console
$ bash tools/check.sh
==> agent instructions (CLAUDE.md is the AGENTS.md pointer form)
==> Rust source size limits
==> format (check only)
==> clippy (workspace lints; no -D warnings)
==> python complete-episode decode witness tests (unittest)
Ran 52 tests in 3.081s
OK
==> python cose-sign1-p1 encoding witness tests (unittest)
Ran 28 tests in 0.051s
OK
==> tests
...
check: all green
```

`bash tools/check_rust_sizes.sh` reports "Rust source sizes: all within
limits"; `cargo fmt --all --check` is clean; `cargo clippy --workspace
--all-targets` reports no errors. The new Rust file is 738 lines and the new
Python files are 793 and 289 lines.

## What the witness actually checks

For one container, the Python side and the Rust side must agree on the
protected-header bytes, the key identifier, the algorithm, the payload bytes,
the carried signature bytes, and the rebuilt `Sig_structure` bytes, and both
sides must refuse the same malformed containers with the same error names. The
malformed corpus is 16 cases: a wrong tag, a wrong element count, a
non-array body, a non-empty unprotected map, a 31-byte payload, trailing bytes,
a truncated signature, an oversized container, an unknown protected label, a
`crit` label, a duplicate protected key, protected keys out of canonical order,
a non-minimal integer, a non-Ed25519 algorithm, a 71-byte key identifier and
an oversized protected header. The error names are the Rust `CoseError` variant
names, mirrored one for one.

The Rust-side `Sig_structure` is rebuilt in the test with the crate's *public*
`CanonicalValue` encoder, because the prototype's own `sig_structure` is
`pub(super)` and unreachable from an integration test. That is not taken on
trust: `rust_side_verification_succeeds` shows that the prototype's internal
structure is the message the Ed25519 signature covers (the crate's own
`verify` returns `Integrity::Valid`), and
`the_signed_bytes_are_the_agreed_sig_structure` signs the bytes the *Python*
witness rebuilt with the pinned key and reproduces the container's 64 signature
bytes exactly. Ed25519 signing is deterministic, so equal signature bytes mean
equal signed messages.

## The honest boundary

- **Encoding agreement across two languages; cryptographic verification
  remains Rust-side.** The Python side parses the container, rebuilds the
  `Sig_structure`, and compares bytes. It never verifies a signature and never
  claims a signature is valid: every response it writes carries
  `"scope": "encoding-agreement-only"`, and the file, the test, the golden
  directory's README and this receipt all say *encoding witness*, never
  *verification witness*.
- **No cross-language signature verification.** There is no second-language
  Ed25519 verifier, and this witness is not one. The Rust-side Ed25519 checks
  in `v3_cross_language_cose.rs` are Rust-side checks over Rust-produced
  bytes; they are not evidence about any other implementation.
- **The COSE profile is unratified and crate-internal.** The witness pins a
  private prototype, not a wire specification. Nothing here ratifies the
  profile, changes the detached `EpisodeSignature` ledger format, or changes
  any committed digest.
- **No admission, authorization, trust or key custody.** The prototype's
  `TrustList` semantics are untouched, and the witness says nothing about
  them.
- **No independent cryptographic review.** This is prototype code plus a
  second-language encoding check. It is not a security review, not a
  qualification, and not a conformance claim.
- **A missing interpreter is red, never green.** The Rust driver fails hard
  when `WDBX_PYTHON` names an interpreter that does not exist, and the gate
  runs the witness's own unittest suite as a dedicated step, so an absent
  Python reads as a failed gate.

## Not done

No file under `crates/abi-wdbx/src/v3/episode/cose/` was changed, so the
prototype's behaviour and its bytes are exactly as they were. No cargo
dependency was added. No golden was rewritten to make a mismatch disappear:
the `Sig_structure` vector was derived by hand from the container's own bytes
with an independent byte-level assembly before the witness was pointed at it,
and the one disagreement found along the way — an 18-byte external AAD whose
canonical head is `52`, not `58 12` — was resolved in the *hand-derived
vector*, not in the encoder. The commit is left to the owner.
