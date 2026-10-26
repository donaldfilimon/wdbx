# Typed ledger history validation

This completion continuation addresses the two review findings before returning
to the existing full substrate release plan. It does not narrow or close that
release objective.

Own `src/v2/types.rs`, CLI `typed.rs`, `tests/typed_cli.rs` (under
`crates/abi-wdbx/`) and this packet's docs. Preserve the pre-existing local frame
slice and all unrelated work. Work remains on canonical local `main`; no other
active WDBX writer was observed. No commits, pushes or live stores are involved.

1. Reproduce inspect's acceptance of a forged superseded envelope with otherwise
   valid V2 integrity, and of valid typed data mixed with temporal nodes/edges.
2. Add borrowed access to retained KV versions without changing serialization or
   maximal-version semantics; authenticate every retained value before projection.
3. Reject both temporal kinds in typed-only roots. Refused inspect/commit must
   leave the synthetic store's transaction count unchanged.
4. Run the real CLI regression suite and `bash tools/check.sh` with Rust 1.99.1,
   stdin at EOF and isolated existing packet build output. Preserve red/green logs.
5. Record exact outcomes and remaining release/acceptance obligations. DAST is
   unavailable unless the missing HawkScan CLI/environment is supplied; do not
   infer security qualification from local tests.

The full release plan and its owner/interface decisions remain authoritative.
