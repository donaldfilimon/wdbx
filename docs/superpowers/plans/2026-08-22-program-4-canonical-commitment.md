# Program 4: Canonical WDBX Commitment — Implementation Plan

> **Status:** approved Program 4 design, first local implementation slice only.
> This plan is non-normative. The Abbey System Constitution and the approved
> federation reconciliation remain authoritative.

## Objective

Add a v3-only, storage-independent commitment primitive for durable Abbey
episodes. It must encode the commitment envelope under a precisely documented
RFC 8949 deterministic-CBOR profile, sort parent digests lexicographically,
and compute SHA-256 over the exact canonical bytes. Existing v2 bytes and
digests remain untouched.

## Reconciliation note: mandatory incidents

The older companion design says mandatory safety events and failures bypass the
write gate entirely. That sentence is stale. Per the later approved
reconciliation, `MandatoryIncident` bypasses **only discretionary utility
scoring**. It never bypasses scope, minimization, redaction, legal or operational
hold handling, retention validation, or deletion-key validation. This slice
does not implement a write gate, and no later implementation may treat
mandatory retention as permission to skip those controls.

## Commitment profile

Create `abi_wdbx::v3::commitment` with:

- profile identifier `abbey-cbor-episode-v1`;
- an exact five-field envelope whose integer keys are frozen:
  `0 = profile`, `1 = schema version`, `2 = header`, `3 = payload`,
  `4 = sorted parent digests`;
- the profile field as the domain separator; the digest is exactly
  `SHA256(canonical_envelope_bytes)` with no hidden prefix or library-specific
  framing;
- a deliberately small RFC 8949 deterministic-CBOR value model: unsigned and
  negative integers, byte/text strings, arrays, maps, booleans, and null;
- preferred (shortest) integer/length serialization, definite lengths only,
  UTF-8 text, and map keys ordered first by encoded-key length and then
  bytewise, as specified by RFC 8949 section 4.2.1;
- no floating-point values, tags, indefinite-length items, `undefined`, or
  duplicate encoded map keys;
- exact supplied UTF-8 bytes with no implicit Unicode normalization; schemas
  own any normalization rule, and identifiers or integers outside the admitted
  64-bit domains are text;
- explicit absent-versus-zero semantics: omission from a map is absent, while
  an encoded zero or null remains present;
- a finite nesting limit and content-free error messages;
- parent permutation invariance by sorting `[u8; 32]` digests before encoding,
  while preserving duplicates because deduplication is not authorized by the
  approved design.

## Implementation sequence

1. Add focused tests that require exact envelope bytes, RFC 8949 map ordering,
   parent permutation invariance, explicit absent-versus-zero distinction,
   duplicate-map-key rejection, and unchanged v2 hash behavior.
2. Add the additive `v3` commitment module and public exports. Do not modify
   `V2AuditBlock`, `AuditHashInput`, `VersionedStore`, or any format reader or
   writer.
3. Add positive golden fixtures containing exact canonical-CBOR bytes and
   SHA-256 digests for an empty-parent episode and a two-parent episode.
4. Run the focused v3 tests, the existing v2/versioned tests, then the complete
   WDBX strict gate.

## Acceptance

- Two logically identical inputs with different parent order produce identical
  canonical bytes and digest.
- Exact positive fixture bytes and digests are stable and reviewed in source.
- Changing absent to present-zero changes bytes and digest.
- Invalid profile values fail before hashing and errors contain no episode
  content.
- Existing v2 golden bytes and digest tests pass without modification.
- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets`, and
  `cargo test --workspace` all pass.

## Explicit non-goals and evidence boundary

This slice adds no `EpisodeBlock` domain schema, COSE or Ed25519 signature,
selective write gate, evidence scoring, retention/deletion implementation,
gateway RPC, migration, reader cutover, `DurableStore` access, network call, or
production write. Passing tests is C1 source/contract evidence only. It is not
replay, recovery, cross-language, live-provider, Discord, hosted, or production
evidence.
