# WDBX

The provenance-aware episodic substrate beneath ABI, and the foundation layer it
is built on.

Extracted from [`donaldfilimon/abi`](https://github.com/donaldfilimon/abi) on
2026-08-22 with history preserved. Every commit that ever touched these five
crates is present; nothing was squashed or replayed.

This repository is public so ABI, Abbey, and external forks can build the exact
pinned substrate without a cross-repository credential. That visibility covers
source code only: WDBX stores, episodes, evidence payloads, operator state,
credentials, and consumer runtime data stay private to their owners. Nothing in
this repository provides a hosted database or production authority.

## What this is

Under the Abbey System Constitution, ABI is the canonical cognitive and
governance runtime and **WDBX is its provenance-aware memory and evidence
substrate**. The distinction the substrate exists to enforce is:

> memory != database lookup

A vector database retrieves similar content. An episodic substrate must also
preserve context, causal dependencies, outcomes, versions, constraints, and
evidence, so that later it can answer: what happened, what was predicted, what
was done, what followed, and why this record is trusted.

## Crates

| Crate | Role |
| --- | --- |
| `abi-foundation` | Shared primitives: errors, env, time, validation, JSON, logging, credentials, atomic IO, HTTP helpers. No dependency on any other crate here. |
| `abi-telemetry` | Bounded, process-wide counters with insertion order preserved. No dependencies. |
| `abi-compute` | Compute selection, deterministic CPU SIMD primitives, and the `Accelerator` contract that consumers implement. No dependencies. |
| `abi-core` | Config, task scheduler, memory accounting, plugin registry. Depends on `abi-foundation` and `abi-telemetry`. |
| `abi-wdbx` | The substrate itself, 25,171 lines: on-disk segment format, CRC-framed WAL recovery, checkpoint publication and salvage, MVCC with conflict sets, multi-parent causal audit DAG, exact and layered-HNSW search, 3-D spatial index, cluster replication with read repair, reference quantization/Huffman/rANS/autoencoder codecs, and optional FHE reference paths. |

Crate names keep the `abi-` prefix deliberately. Renaming them would churn every
consumer for no behavioral gain, and the constitution's point is that ABI owns
this layer, not that the layer must be re-branded.

## Status: honest

Measured against the CSAPS/WDBX specification (see
`abi/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md`), this
codebase implements most of the **structural** half of the substrate and little
of the **evidence** half.

**Current.** Multi-parent causal audit DAG. SHA-256 content addressing. Ed25519
signing over transaction and segment objects. MVCC. WAL plus segments with CRC
framing. Cluster replication and read repair. A pluggable retrieval scoring seam.

**Not implemented, and not claimed.** Canonical CBOR or COSE encoding; the commit
digest is taken over `serde_json` bytes with parents in insertion order rather
than sorted, so it satisfies neither the encoding nor the ordering half of the
specification's commitment function. No `schema_version`, `policy_version`,
`signer_key_id`, `task_regime`, or `regime_posterior`. No contradiction or
quarantine edges. No evidence-weighted retrieval: ranking is semantic, temporal,
causal, and persona affinity, combined multiplicatively into one score, which is
the opaque collapse the constitution's invariant I3 forbids. No selective write
gate. No block-level retention, redaction, or deletion semantics.

Closing those gaps is Program 3, the Canonical WDBX Episodic Contract.

## Gate

```
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

All three green as of extraction: 558 tests passed, 0 failed, 0 clippy
diagnostics. The workspace denies `unsafe_code` and all of clippy, and warns on
`missing_docs` and clippy pedantic.

## Consumers

`abi` and `abbey` consume these crates by relative path as siblings under
`~/dev/active`. That layout is load-bearing: a git-dependency for one consumer
and a path dependency for another would give Cargo two distinct copies of
`abi-wdbx`, and types would stop unifying where they cross.
