# WDBX Contracts — Status: Honest

This document mirrors the [Status: honest table](https://github.com/donaldfilimon/wdbx/blob/main/README.md#status-honest) from the repository README.

## Current (Implemented)

| Capability | Status | Notes |
|------------|--------|-------|
| Multi-parent causal audit DAG | ✅ Implemented | `versioned.rs:430` appends blocks with all observed heads as parents; `v2.rs:319`/`326` validate parent hashes and reject self-parenting |
| SHA-256 content addressing | ✅ Implemented | Commitment is `SHA256(serde_json::to_vec(AuditHashInput))` over parents in insertion order |
| Ed25519 signing | ✅ Implemented | Over transaction and segment objects in `v2/security.rs` (not over audit blocks) |
| MVCC with conflict sets | ✅ Implemented | Multi-version concurrency control |
| WAL + segment durability with CRC framing | ✅ Implemented | Write-ahead log with segment checkpoints and CRC framing |
| Cluster replication + read repair | ✅ Implemented | Raft-style replication with read repair |
| Pluggable retrieval scoring seam (`HybridScorer`) | ✅ Implemented | Four dimensions: semantic, temporal, causal, persona |
| `abbey-cbor-episode-v1` envelope encoder (C1) | ✅ Implemented | Deterministic CBOR profile, sorted parent commitments, exact positive golden vectors |
| Single-writer `EpisodeStore` (prototype) | ✅ Implemented | Reconstructs canonical envelope from typed lifecycle events; computes commitment inside WDBX; verifies on reopen; content-free receipts |

## Not Implemented (And Not Claimed)

| Capability | Status | Reference |
|------------|--------|-----------|
| Complete constitutional v3 `EpisodeBlock` schema | ❌ Not implemented | Gap analysis §6.2 — `V2AuditBlock` carries 8 fields vs ~28 specified |
| General canonical-CBOR reader | ❌ Not implemented | Gap analysis §6.4 — `serde_json` digest, no CBOR/COSE |
| COSE envelope | ❌ Not implemented | Gap analysis §6.4 — zero occurrences in crate |
| Episode signing (Ed25519 over `EpisodeBlock`) | ❌ Not implemented | Gap analysis §6.4 — segment-level only, no `signer_key_id` on blocks |
| `policy_version`, `signer_key_id`, `task_regime`, `regime_posterior` fields | ❌ Not implemented | Gap analysis §6.2 — absent from v2 record format |
| Contradiction edges | ❌ Not implemented | Gap analysis §6.5, §6.9 — zero occurrences |
| Quarantine edges | ❌ Not implemented | Gap analysis §6.5, §6.9 — zero occurrences |
| Evidence-weighted retrieval (8 dimensions) | ❌ Not implemented | Gap analysis §6.5 — only 4 dimensions, multiplicative collapse |
| Block-level retention, redaction, deletion | ❌ Not implemented | Gap analysis §6.9 — absent in full |
| Signature verification as retrieval dimension | ❌ Not implemented | Gap analysis §6.5 — verification exists at segment level only |
| Hosted service semantics | ❌ Not implemented | Gap analysis §6.7 — no `ProposeWrite`, `Verify` on gateway |

## Gap Analysis Reference

Detailed per-section gap tables are in the ABI repository:

- **§6.2 Logical block schema** — [Gap analysis table](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md#62-logical-block-schema)
- **§6.4 Serialization, content addressing, and signing** — [Two independent failures](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md#64-serialization-content-addressing-and-signing)
- **§6.5 Trust and semantic validity** — [Eight dimensions vs four](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md#65-trust-and-semantic-validity)
- **§6.6 Threat model** — [Nine threats, partial coverage](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md#66-threat-model)
- **§6.7 Service boundaries** — [Missing `ProposeWrite`/`Verify`](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md#67-service-boundaries)
- **§6.9 Retention and deletion semantics** — [Absent in full](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md#69-retention-and-deletion-semantics)

See also: [Canonical WDBX Episodes Specification](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md) for the proposed design that closes these gaps.