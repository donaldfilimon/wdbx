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
| C6 | Operator witnessed exact outcome | ❌ Not claimed: `VerifyEpisode` exists on the ABI gateway, but no operator-witnessed outcome is recorded |
| C7 | Reconstructible experiment manifest | ❌ Partial (`abi-telemetry` exists but incomplete) |

## Current Claim Boundary

> This codebase implements most of the **structural** half of the substrate and little of the **evidence** half.

Closing the evidence half is Program 4: `canonical-wdbx-episodes-claims`.
**Per-episode signatures (2026-09-16): C0/C1 only.** The v3 episode store can
sign each episode digest with Ed25519 (`src/v3/episode/signing.rs`), and replay
rejects a tampered signature under the writer's own key. This is a detached
Ed25519 signature over a canonical digest, verified in Rust tests. It is **not**
COSE, is not verified by any other language, and has no key rotation or
revocation, so C2 above stays ❌.

**C6 reason corrected (2026-09-16).** The earlier reason, "No `Verify` RPC", was
stale: `VerifyEpisode` / `ProposeEpisodeWrite` live on the ABI gateway
(`abi/crates/abi-wdbx-gateway/src/service.rs`, `abi wdbx episode verify`). An RPC
existing is not an operator witnessing an outcome, so the level stays ❌.
