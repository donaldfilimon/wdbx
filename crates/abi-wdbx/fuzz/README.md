# abi-wdbx fuzz targets

Standalone workspace. The wdbx root globs `crates/*`, which does not match
`crates/abi-wdbx/fuzz`, so `cargo fmt/clippy/test --workspace` at the root never
reaches these targets — verified green with this directory present. Fuzz builds
need nightly plus a sanitizer runtime and must not gate ordinary development.

## Run

    cd crates/abi-wdbx
    RUST_BACKTRACE=1 cargo fuzz run ans_roundtrip -- \
      -max_total_time=240 -rss_limit_mb=2048 -timeout=10 -max_len=65536 \
      -dict=fuzz/ans.dict

## Why round-trip and not decode-only

`ans_decode` takes an `AnsEncoded` struct, so a decode harness would synthesize
`mode`/`data`/`original_len` directly. Nothing in either workspace rehydrates an
`AnsEncoded` from stored bytes — the only consumer is
`abi/crates/abi-cli/src/wdbx/secure.rs`, which encodes and immediately decodes.
Feeding a hand-built struct would therefore "find" failures on an invariant
production cannot reach. (`ans_decode` also reads only `data`; `original_len` is
never used to size an allocation, so there is no length-field OOM here.)

Every assertion in the harness restates one the crate's own unit tests already
make, so a failure is a real defect, not a harness artefact.

## Gotcha: `cargo fuzz init` picks the wrong crate name

With cargo-fuzz 0.13.2, running `cargo fuzz init` inside a workspace member
generates `name = "<first workspace member>-fuzz"` and a matching `[dependencies]`
key — here `abi-compute` — while pointing `path = ".."` at the containing crate.
The manifest will not build until the dependency is renamed to the crate that
`..` actually resolves to. Check the generated `Cargo.toml` before building.

## Findings

`unsafe_code = "deny"` across this workspace, so ASan can only fire inside
dependencies. The actionable classes are panics, hangs and OOM, which is why the
release profile here sets `debug-assertions` and `overflow-checks` — without
them a wrapping subtraction in `normalize()` would go unreported.

2026-09-06, `ans_roundtrip`: 835,940 executions in 241s (~3,470 exec/s),
coverage 216 -> 222 edges, features 420 -> 818, corpus 9 -> 76 entries,
**0 crashes and 0 artifacts**. Coverage plateaued early: the encoder-reachable
surface is small. This is a clean short campaign, not proof the codec is
bug-free.

## Candidate next targets

- `v2.rs` journal object reader (the loop over length-prefixed objects). Needs
  the post-decrypt parser exposed as `#[doc(hidden)] pub`, since driving it
  through the file path means disk I/O and an AEAD tag the fuzzer cannot forge.
- `entropy.rs::entropy_round_trip` — same shape as this target, already public.
- `PqArtifact::from_json` -> `validate()` -> `decode()`. Examined by hand and the
  indexing invariants are fully covered: `validate()` proves
  `codebooks.len() == subspaces` and each `codebook.len() == centroids`, and
  `validate_codes` proves every code is in range. Low expected yield.
