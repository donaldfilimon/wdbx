# Bounded Canonical CBOR Reader Implementation Plan

> **For agentic workers:** Use subagent-driven implementation with independent spec/quality review after each task. This is a slice of Task 2 in `2026-10-20-wdbx-full-completion.md`, not completion of the constitutional envelope.

**Goal:** Parse and validate the existing `abbey-cbor-episode-v1` values/envelopes without accepting alternative encodings or changing any existing digest.

**Architecture:** Add a bounded, dependency-free reader under `v3::commitment`. Parsing rejects noncanonical bytes rather than normalizing them. A second-language reader independently checks Rust results. No signing, store admission, transport authority or new commitment profile is introduced by this slice.

**Tech Stack:** Rust 1.99.1; standard-library Python witness.

**Spec:** Approved full-completion program Task 2; current `crates/abi-wdbx/src/v3/commitment.rs`; RFC 8949 section 4.2.3 length-first ordering. The frozen profile intentionally excludes floats, tags, indefinite lengths and undefined.

## Global Constraints

- Preserve existing encoder behavior, goldens, duplicate parents and v2 commitments byte-for-byte.
- Canonical writer/admission and signature semantics remain separate from syntax decoding.
- All errors are content-free. Reject before allocating from untrusted lengths. No new dependency, unsafe code, or Rust file above 1000 lines.
- Map keys may be any supported canonical value, including compound keys. Order is encoded length then encoded bytes. Parents are nondecreasing, not unique.
- Root depth is zero, including full envelope; maximum item depth is existing MAX_NESTING_DEPTH=32. Unsupported negative numbers below i64::MIN are rejected.
- Canonical decode means full slice consumption, minimal integer/length widths, valid UTF-8, canonical map order and exact admitted envelope shape. No implicit normalization.

## Review Focus

1. A tiny input declaring a huge allocation cannot exhaust memory before bounds checks.
2. Compound map keys and duplicate sorted parents cannot be rejected by oversimplified validation.
3. Decoder success must imply identical reencoding and digest, not merely equivalent values.
4. Nesting limits count map keys, container children and outer envelope consistently with the encoder.
5. Missing Python or malformed differential output is failure, never a skipped witness.

## Task 1: Rust bounded profile reader

**Files:** `crates/abi-wdbx/src/v3/commitment/decode.rs`; extend `commitment.rs`; add `tests/v3_canonical_decode.rs` and split resource/malformed tests if needed.

**Interfaces:** Public reexports from `v3::commitment`:

```rust
pub struct CanonicalDecodeLimits {
    pub max_input_bytes: usize,
    pub max_value_count: usize,
    pub max_collection_entries: usize,
    pub max_string_bytes: usize,
    pub max_parent_count: usize,
}
impl Default for CanonicalDecodeLimits { /* limits below */ }
// Separate content-free decoding errors; do not add variants to encoding errors.
pub enum CanonicalDecodeError { /* static syntax/shape/resource categories */ }
impl CanonicalValue {
    pub fn from_canonical_bytes(input: &[u8], limits: CanonicalDecodeLimits)
        -> Result<Self, CanonicalDecodeError>;
}
impl EpisodeCommitment {
    pub fn from_canonical_bytes(input: &[u8], limits: CanonicalDecodeLimits)
        -> Result<Self, CanonicalDecodeError>;
    pub const fn schema_version(&self) -> u64;
    pub const fn header(&self) -> &CanonicalValue;
    pub const fn payload(&self) -> &CanonicalValue;
    pub fn parent_digests(&self) -> &[[u8; 32]];
}
```

- [ ] Write failing tests for existing goldens, exact roundtrip/digests, scalar extremes, Unicode/absence distinctions, compound keys and duplicate parents.
- [ ] Add malformed tests for every truncated golden prefix, trailing data, overlong arguments/lengths, invalid UTF-8, unsupported types and noncanonical maps/parents/envelopes.
- [ ] Add exact-boundary tests for input, value, collection, string, parent and depth limits; giant declared lengths; deterministic byte mutations must not panic.
- [ ] Implement one-pass checked parsing with input-backed raw key comparisons, global value budget (keys+values included), finite collection/string caps and fallible reservation. Root envelope is exactly keys 0..4 with exact profile, nonzero u64 schema, map header/payload and sorted 32-byte parents. Nonzero future schema is syntactically accepted without semantic claims.
- [ ] Defaults: 1,048,576 input bytes; 65,536 values; 16,384 collection entries; 1,048,576 string bytes; 4,096 parents. These are configurable reader policy, not retroactive encoder restrictions. Zero limit is valid and denies the applicable resource; no zero-as-unlimited convention.
- [ ] Run targeted reader/canonical/store/candidate/edge/signing suites and formatting/clippy as appropriate; preserve existing tests unchanged. Commit exact paths and report evidence.

## Task 2: Independent Python decoding witness

**Files:** extend `tools/abbey_cbor_episode_v1.py` using a sibling reader module if needed; add `crates/abi-wdbx/tests/v3_cross_language_decode.rs`.

**Interfaces:** Add JSON-lines command `decode-corpus`. Input per line: `{"cbor_hex":"...","kind":"value"|"episode","limits":{...}}`; omit limits for defaults. Output success: `{"ok":true,"cbor_hex":"..."}`; error: `{"ok":false,"error":"<stable category>"}`. Errors are static and must not echo input. Existing commands remain unchanged. Bad JSON/hex/unknown request keys fail with a static request-error response. This command is a test witness, never canonical signing authority.

- [ ] Independently implement the same profile/resource checks in Python; do not invoke Rust to decode. Keep encode and stored episode derivation unchanged.
- [ ] Differential tests cover goldens, nested values, boundaries, reordered keys/parents, truncation, unsupported forms and independent limits. Batch requests with concurrent stdin writing/stdout draining to avoid pipe deadlock.
- [ ] Missing interpreter, nonzero process exit, wrong response count or invalid output fails the test. Follow WDBX_PYTHON convention.
- [ ] Run new suites, both standalone golden checks and full `bash tools/check.sh` on the final code state. Record source/local interoperability evidence only.
