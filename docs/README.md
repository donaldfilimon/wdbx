# WDBX Documentation

Substrate documentation is federated from ABI.

## Specification Documents

The authoritative design and gap-analysis documents live in the ABI repository:

- **Canonical WDBX Episodes Specification**: [`abi/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md`](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md) — Program 4 constitutional specification for episode blocks, commitment, trust dimensions, write gate, and deletion semantics.

- **WDBX Conformance Gap Analysis**: [`abi/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md`](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md) — Measured distance between current implementation and the CSAPS/WDBX specification (sections 6.1–6.9, R1–R12).

## Plans and Specs Index

For the full index of active plans and design drafts, see:
[`abi/docs/superpowers/README.md`](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/README.md)

## Local Documentation

- [`contracts/README.md`](contracts/README.md) — Honest status table mirroring the README claim ledger.
- [`architecture.md`](architecture.md) — Crate graph, storage path, and concurrency model summary.
- [`claims.md`](claims.md) — Claim ledger pointer.
- [`superpowers/README.md`](superpowers/README.md) — Local mirror of the ABI superpowers index header.

## Gate

All documentation changes must pass the workspace gate:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```