# Memory-edge resolution provenance implementation plan

Approved contract: `abi/docs/superpowers/specs/2026-09-16-spec-memory-edge-episodes.md`
section 4. The ecosystem completion request authorizes closing this existing gap.

**Goal:** when a caller reads a quarantine or contradiction edge, expose the
canonical episode digest that resolved that exact edge, including after reopen.
Complete the WDBX query, ABI gateway projection and CLI readout as one slice.
This does not claim complete candidate-level edge history or retrieval scoring.

## Invariants

- Preserve every event field, commitment byte and existing golden digest.
- Preserve `MemoryEdgeState` and `memory_edge_open`; add a complementary query.
- At most one resolution closes an edge. A later edge has a different digest;
  never replace an earlier edge's resolver with a candidate's latest resolution.
- Malformed guild identifiers retain `InvalidInput`. Wrong guild, unknown digest,
  candidate/proposal/resolution inputs and open edges yield no resolver.
- Only validated canonical store records supply the resolver. Reopen rebuilds
  the same relationship; caller-provided transport fields are not authority.
- Additive protobuf field 9 is a read projection. Generate bindings through the
  repository build. Empty bytes represent no resolver; nonempty is 32 bytes.
- Scratch stores only. No private-store read, live write, service restart,
  dependency addition, commit or deployment.

## Ownership and files

One implementation worker owns the WDBX store query and regression tests, ABI
`abi-wdbx-gateway` episode lookup/service projection, `proto/gateway.proto`, ABI
CLI episode output and their existing integration tests. Root owns this plan,
ecosystem records and heavy gates. Preserve existing dirty work, including
WDBX's cross-language pipe fixes and ABI's training changes.

Likely seams (inspect current source before editing):
`wdbx/crates/abi-wdbx/src/v3/episode/store.rs`,
`wdbx/crates/abi-wdbx/tests/v3_memory_edge.rs`,
`abi/crates/abi-wdbx-gateway/src/episodes.rs`, gateway service projection,
`abi/crates/abi-wdbx-gateway/proto/gateway.proto`,
`abi/crates/abi-cli/src/wdbx/episode.rs`,
`abi/crates/abi-wdbx-gateway/tests/episodes.rs`,
`abi/crates/abi-cli/tests/episode_gateway.rs`.
Keep every Rust file below the repository limit; split sibling test modules if
needed. Do not modify generated bindings by hand.

## Steps

- [x] Wait for Bot's current frozen strict gate to finish before source mutation.
- [x] Read the exact store/replay/projection seams. Add regressions for both edge
      kinds, exact resolving digest, reopen, unknown/nonclosable/wrong guild,
      new edge after closure and refusal of duplicate resolution.
- [x] Observe meaningful RED, then add the smallest canonical query. Prefer
      deriving from validated stored events over a new persistence format.
- [x] Add gateway/protobuf field and CLI output with end-to-end scratch gateway
      coverage. Absence must be explicit and backward-compatible.
- [x] Focused tests and formatting; freeze exact owned diff/hashes.
- [x] Independent Standards and Spec reviews; fix findings and re-review.
- [x] Root runs WDBX and ABI gates after source freeze, records counts/exits and
      unchanged candidate/edge/cross-language golden commitments.
- [x] Update claims only to the implemented per-edge read provenance scope.

## Acceptance commands

From ABI using its pinned sibling-aware wrapper:

```sh
./tools/cargo.sh test --manifest-path ../wdbx/Cargo.toml -p abi-wdbx --test v3_memory_edge --test v3_memory_candidate --test v3_cross_language_episode < /dev/null
./tools/cargo.sh test -p abi-wdbx-gateway --test episodes < /dev/null
./tools/cargo.sh test -p abi-cli --test episode_gateway < /dev/null
```

Then from WDBX, with its pinned compiler/system tools first:
`bash tools/check.sh`. From ABI: `./tools/check.sh`.
A green source gate does not qualify deployed adapters, full constitutional
memory conformance, retrieval weighting or candidate-level history.

## Qualified result — 2026-10-03

Independent Standards and Spec reviews passed. Root full gates passed with source,
HEAD and index unchanged: WDBX 647 Rust executions; ABI 988 Rust plus 144 Python.
The initial ABI invocation selected Apple Python 3.9 and failed before compilation;
a process-local Python 3.14.8 shim corrected the environment, and the complete
gate then passed. No repository or global toolchain configuration changed.

Frozen receipts and logs: `.superpowers/sdd/2026-10-03-ecosystem/evidence/`
`resolution-final-gates/` and `resolution-abi-python314/`. Source-derived claims
now describe only per-edge resolution provenance. The broader ecosystem plan
remains Partial.
