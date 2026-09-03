# WDBX Architecture Summary

**Authoritative reference:** [`abi/docs/spec/agent-wdbx-architecture.mdx`](https://github.com/donaldfilimon/abi/blob/main/docs/spec/agent-wdbx-architecture.mdx)

## Crate Graph (Dependency Order)

```
abi-foundation  (no deps)
    ├── errors, env, time, validation, JSON, logging, credentials, atomic IO, HTTP helpers
abi-telemetry   (no deps)
    ├── bounded process-wide counters, insertion order preserved
abi-compute     (no deps)
    ├── compute selection, deterministic CPU SIMD, `Accelerator` contract
abi-core        → abi-foundation, abi-telemetry
    ├── config, task scheduler, memory accounting, plugin registry
abi-wdbx        → abi-foundation, abi-compute
    ├── The substrate: segments, CRC-framed WAL, checkpoints, MVCC, causal DAG,
    │   exact + layered-HNSW search, 3-D spatial index, cluster replication +
    │   read repair, quantization/Huffman/rANS/autoencoder codecs, FHE reference
```

All five crates live in this repository. `abi-core` is a leaf — nothing in WDBX depends on it. Consumers (`abi`, `abbey`) depend on these crates by **sibling path** (`../wdbx/crates/...`) under `~/dev/active`. This layout is load-bearing: mixing git and path dependencies would give Cargo two distinct `abi-wdbx` copies whose types would not unify.

## Storage Path

| Component | Status |
|-----------|--------|
| **Segments + WAL** | Immutable segment files with CRC framing; WAL for uncommitted writes |
| **Checkpoints** | Periodic snapshots published independently; salvage on reopen |
| **MVCC** | Multi-version concurrency with conflict sets; no global writer lock |
| **Causal DAG (v2)** | Multi-parent audit blocks (`V2AuditBlock`); SHA-256 over JSON parents in insertion order |
| **Episodes (v3, prototype)** | `abbey-cbor-episode-v1` envelope (RFC 8949 deterministic CBOR); sorted parent digests; `EpisodeStore` single-writer |
| **HNSW Search** | Deterministic layered-HNSW cosine search; exact fallback |
| **Spatial Index** | 3-D octree for spatial records |
| **Cluster** | Raft-style replication with read repair; token + allowlist auth |

## Retrieval Scoring

Current (`ScoreComponents`, `temporal.rs`):
- `semantic` × `temporal` × `causal` × `persona` (multiplicative collapse)

Proposed (8 dimensions, per spec §6.5):
- Cryptographic validity, Source identity, Calibration, Outcome, Compatibility, Regime, Lifecycle, Confidence

`HybridScorer` is a pluggable seam — extending it is additive, not a redesign.

## Golden Fixtures

- `crates/abi-wdbx/tests/golden/wdbx-format.md` — On-disk format samples (moved from ABI root during extraction)
- Positive `abbey-cbor-episode-v1` vectors: exact canonical-CBOR bytes and SHA-256 digests for empty-parent and two-parent episodes
- Episode-store replay tests: deterministic reconstruction from typed lifecycle events

CLI/MCP output goldens (`help-wdbx.txt`, etc.) remain in `abi` because they describe ABI surfaces.

## Concurrency & Safety

- **No `unsafe`** — workspace denies `unsafe_code`
- **Deterministic** — pinned nightly toolchain (`rust-toolchain.toml`); SIMD via `portable_simd` (nightly)
- **Tests** — Must use `ABI_WDBX_PATH=:memory:`, `ABI_WDBX_PERSIST=0`, or scratch paths; never `~/.abi/`
- **Gateway** — Bounded authenticated gRPC (`WdbxGateway`: `PutVector`, `Search`, `PutKv`, `GetKv`, `ResolveConflict`, `Stats`, `MembershipChange`, `WatchMutations`)

## Non-Goals (Current)

- No durable background jobs, remote workers, or multi-host execution (`abi-core::Scheduler` is one-process)
- No production multi-host deployment / sharding proven
- Compression/FHE are reference implementations unless audited artifact cited
- Metal acceleration reported only when linked kernels initialize; CUDA/Vulkan/ANE not implied

## Gate

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```