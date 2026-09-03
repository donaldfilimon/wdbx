# WDBX Claim Ledger

**Claim ledger is [`README.md:44`](https://github.com/donaldfilimon/wdbx/blob/main/README.md#L44).**

**Detailed gap analysis at [`abi/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md`](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md).**

---

## Evidence Levels (Constitution C0–C7)

| Level | Description | WDBX Coverage |
|-------|-------------|---------------|
| C0 | Source conforms under test | ✅ `cargo test --workspace` passes |
| C1 | Source evidence + local deterministic replay | ✅ Golden fixtures, episode-store replay tests |
| C2 | Cross-language canonicalization | ❌ Not implemented (CBOR/COSE) |
| C3 | Live provider / Discord evidence | ❌ Not claimed |
| C4 | Hosted service / federation evidence | ❌ Not claimed |
| C5 | Production deployment evidence | ❌ Not claimed |
| C6 | Operator witnessed exact outcome | ❌ No `Verify` RPC |
| C7 | Reconstructible experiment manifest | ❌ Partial (`abi-telemetry` exists but incomplete) |

## Current Claim Boundary

> This codebase implements most of the **structural** half of the substrate and little of the **evidence** half.

Closing the evidence half is Program 4: `canonical-wdbx-episodes-claims`.