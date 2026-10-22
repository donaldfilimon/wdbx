# `cose-sign1-p1` container fixture

This fixture pins the exact bytes of one detached `COSE_Sign1` container built
by the crate's private COSE prototype (`src/v3/episode/cose/`), and the
encoding facts a second language must reproduce from those bytes.

The container is synthetic: it signs the 32-byte digest `0x5a` repeated, with
the Ed25519 key derived from the fixed 32-byte seed `0x01` repeated. Ed25519
is deterministic, so the container bytes are reproducible.

The layout is the profile's frozen shape:

```text
d2                                   # tag(18): COSE_Sign1
84                                   # array(4)
58 4e <78 bytes>                     # protected: bstr {1: -19, 4: kid}
a0                                   # unprotected: empty map
58 20 <32 bytes>                     # payload: the episode digest
58 40 <64 bytes>                     # signature: Ed25519 over the Sig_structure
```

The `kid` is the 72 ASCII bytes of the existing `SignerKeyId` for that key
(`ed25519:` followed by the lowercase hex of SHA-256 over the verifying key).
The signature covers the RFC 9052 `Sig_structure`
`["Signature1", protected, external_aad, payload]`, with the prototype's fixed
external AAD `wdbx-cose-sign1-p1`; those 64 signature bytes are therefore
*different* from the detached `EpisodeSignature` the episode ledger stores,
which signs the bare 32-byte digest.

## Files

| File | Contents |
| --- | --- |
| `container.hex` | the container bytes, lowercase hex |
| `container.sha256` | SHA-256 over those container bytes |
| `protected.hex` | the protected-header bytes carried by the container |
| `payload.hex` | the 32-byte episode digest carried by the container |
| `kid.hex` | the 72 key-identifier bytes, as lowercase hex of their ASCII form |
| `alg.txt` | the algorithm identifier, `-19` (Ed25519) |
| `sig_structure.hex` | the RFC 9052 `Sig_structure` bytes the signature covers |
| `sig_structure.sha256` | SHA-256 over those `Sig_structure` bytes |

The `.hex` files hold bytes as lowercase hex and the `.sha256` files hold the
lowercase SHA-256 digest of the bytes they name. `alg.txt` holds the single
decimal integer `-19`. They contain no user, provider, Discord, credential, or
production data.

## What the cross-language witness proves, and what it does not

`tools/wdbx_cose_sign1_p1.py` is a standard-library-only Python witness over
these bytes. It parses the container with a strict bounded reader, rebuilds the
`Sig_structure` from the container's own protected bytes, and reports the
protected header, key identifier, algorithm, payload and `Sig_structure` so the
Rust side can compare bytes. `crates/abi-wdbx/tests/v3_cross_language_cose.rs`
requires those bytes to be identical on both sides and requires both sides to
refuse the same malformed containers with the same error names.

That is **encoding agreement**: two languages read the same bytes and rebuild
the same signed structure. It is **not** cross-language signature
verification. The Python side does not verify signatures and cannot — this
environment has no Python cryptography library, and the repository's design
guidance forbids hand-rolling cryptography in Python. The Ed25519 checks in
the Rust test are Rust-side checks over Rust-produced bytes.

This fixture pins a *private prototype*, not a ratified profile. Do not treat
it as a wire specification.
