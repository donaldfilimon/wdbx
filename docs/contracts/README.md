# WDBX Contracts — Status: Honest

This document mirrors the [Status: honest table](https://github.com/donaldfilimon/wdbx/blob/main/README.md#status-honest) from the repository README.

## Current (Implemented)

| Capability | Status | Notes |
|------------|--------|-------|
| Multi-parent causal audit DAG | ✅ Implemented | `versioned.rs:430` appends blocks with all observed heads as parents; `v2.rs:319`/`326` validate parent hashes and reject self-parenting |
| SHA-256 content addressing | ✅ Implemented | The frozen v2 commitment uses `serde_json` bytes with parents in insertion order; the separate v3 episode profile hashes deterministic CBOR with sorted parent digests |
| Ed25519 transaction and segment signing | ✅ Implemented | `v2/security.rs`; this does not change the frozen v2 audit-block format |
| MVCC with conflict sets | ✅ Implemented | Multi-version concurrency control |
| WAL + segment durability with CRC framing | ✅ Implemented | Write-ahead log with segment checkpoints and CRC framing |
| Cluster replication + read repair | ✅ Implemented | Raft-style replication with read repair |
| Pluggable retrieval scoring seam (`HybridScorer`) | ✅ Implemented | Four dimensions: semantic, temporal, causal, persona |
| `abbey-cbor-episode-v1` envelope encoder (C1) | ✅ Implemented | Deterministic CBOR profile, sorted parent commitments, exact positive golden vectors |
| Single-writer `EpisodeStore` (prototype) | ✅ Implemented | Reconstructs canonical envelope from typed lifecycle events; computes commitment inside WDBX; verifies on reopen; content-free receipts |
| Detached v3 episode signing (C0/C1) | ✅ Implemented | `src/v3/episode/signing.rs` signs the canonical digest; `tests/v3_episode_signing.rs` pins unsigned-byte compatibility, reopen, key identity and tamper rejection. Foreign keys remain explicitly unresolved unless supplied to the verifier |
| v3 policy/consent binding | ✅ Implemented | Admission rejects policy-version and consent-epoch drift before append; `tests/v3_episode_store.rs` verifies receipts, usage and ledger bytes remain unchanged |
| v3 contradiction, quarantine and resolution edges (C0/C1) | ✅ Implemented | `EpisodeEvent::MemoryEdge`; `tests/v3_memory_edge.rs` pins authority, scope, reopen and forgetting behavior. Edges do not yet affect retrieval ranking |
| Per-edge resolution provenance | ✅ Implemented | `EpisodeStore::memory_edge_resolution` in `src/v3/episode/store.rs` returns the canonical resolving digest for one exact edge and guild, including after reopen. ABI exposes it through additive gateway field 9 and CLI text/JSON. This does not implement candidate-level history or retrieval weighting; [review and qualification](../reviews/2026-10-03-memory-edge-resolution.md) |
| Cross-language encoding and episode digest witness (partial C2) | ✅ Implemented | `tools/abbey_cbor_episode_v1.py` and the two `tests/v3_cross_language_*.rs` suites reproduce canonical bytes and receipt digests; no Python signature verifier or admission-rule implementation |

Source paths beginning `src/` or `tests/` are relative to
`crates/abi-wdbx/`. These are source/local-test capabilities, not evidence of a
deployed or hosted service.

## Not Implemented (And Not Claimed)

| Capability | Status | Reference |
|------------|--------|-----------|
| Complete constitutional v3 `EpisodeBlock` schema | ❌ Not implemented | Gap analysis §6.2 — `V2AuditBlock` carries 8 fields vs ~28 specified |
| General canonical-CBOR reader | ❌ Not implemented | The v3 encoder is deliberately a restricted commitment encoder, not a general CBOR decoder |
| COSE envelope, key rotation/revocation and cross-language signature verification | ❌ Not implemented | Detached v3 signatures do not implement these capabilities |
| Complete constitutional metadata across storage generations | ❌ Not implemented | v3 binds `policy_version` and detached signatures carry a derived `signer_key_id`; v2 remains unchanged, and the complete constitutional schema is still absent |
| Evidence-weighted retrieval (8 dimensions) | ❌ Not implemented | Gap analysis §6.5 — only 4 dimensions, multiplicative collapse |
| Full block-level retention, redaction and deletion lifecycle | ❌ Not implemented | v3 memory candidates have supersession/forgetting tombstones (`tests/v3_memory_candidate.rs`); these do not establish the full constitutional lifecycle or physical erasure |
| Signature verification as retrieval dimension | ❌ Not implemented | v3 digest verification exists, but retrieval does not incorporate it as an evidence dimension |
| Full constitutional hosted service/federation semantics | ❌ Not implemented | ABI has policy-gated `ProposeEpisodeWrite` and `VerifyEpisode` RPCs; their existence is not full service conformance or production federation evidence |

## Gap Analysis Reference

The original 2026-08-22 per-section gap tables are in the ABI repository.
They describe the historical baseline; use the current rows above and the
repository README for capabilities subsequently implemented:

- **§6.2 Logical block schema** — [Gap analysis table](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md#62-logical-block-schema)
- **§6.4 Serialization, content addressing, and signing** — [Two independent failures](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md#64-serialization-content-addressing-and-signing)
- **§6.5 Trust and semantic validity** — [Eight dimensions vs four](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md#65-trust-and-semantic-validity)
- **§6.6 Threat model** — [Nine threats, partial coverage](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md#66-threat-model)
- **§6.7 Service boundaries** — [Historical service gap](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md#67-service-boundaries)
- **§6.9 Retention and deletion semantics** — [Constitutional lifecycle requirements](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md#69-retention-and-deletion-semantics)

See also: [Canonical WDBX Episodes Specification](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md) for the proposed design that closes these gaps.
