# Memory-edge resolution provenance review

Date: 2026-10-03. Verdict: **scoped PASS** for the frozen additive per-edge resolution read path. Standards: **0 actionable findings**. Specification: **0 actionable findings**. Separate benchmark wording appendix: **PASS**.

This is source review plus inspection of the supplied focused-test evidence. Full WDBX and ABI gates remain the coordinating agent's acceptance work. No source fixes, private-store operations, commits, or deployment were performed by the reviewers.

## Scope and fixed baseline

- WDBX root: `/Users/donaldfilimon/dev/active/wdbx`; HEAD `e45410356ceb4e96f0dc26a83b02e1c18cf94860`.
- ABI root: `/Users/donaldfilimon/dev/active/abi`; HEAD `80dfe079ebe7413a6815c2d564be1f0aaa901267`.
- Frozen uncommitted task diff: `wdbx-owned.patch`, `abi-owned.patch`, and `source-receipt.json` under [retained evidence](../../.superpowers/sdd/2026-10-03-ecosystem/memory-edge-resolution/source-receipt.json); original evidence under `/tmp/memory-edge-resolution-20261003/`.
- Review rehashed all nine owned source files and all eight supplied test/lint logs: every hash matched the original receipt. Both repository HEADs matched. Other dirty files are outside this review.
- Contract: [approved memory-edge spec, section 4](../../../abi/docs/superpowers/specs/2026-09-16-spec-memory-edge-episodes.md) and [bounded implementation plan](../superpowers/plans/2026-10-03-memory-edge-resolution-provenance.md). Candidate-level history and retrieval scoring are expressly outside this slice.
- Standards sources: home/workspace guidance, both repository `AGENTS.md` files, ABI `tasks/lessons.md`, and the code-review skill's smell baseline. Standards and specification were reviewed separately; the specification pass was delegated to an independent child reviewer.

Paths below use `wdbx/` and `abi/` for the two roots above.

## Standards

**PASS: no actionable documented-standard violation or design-smell finding.**

The production change is additive and keeps canonical semantics in WDBX. The query returns a copied fixed-size digest from validated records without new persistence fields, adapter authority, dependencies, or synchronization. Gateway failures retain the established typed error mapping. Existing `MemoryEdgeState` and `memory_edge_open` APIs, committed event types, canonical encoders, and golden fixtures are unchanged by the owned diff. The gateway test split follows the repository's file-size layout rule.

Requested panic, clone, and lock audit:

| Severity | File:line | Description | Suggestion | Status |
| --- | --- | --- | --- | --- |
| Informational | `wdbx/crates/abi-wdbx/tests/v3_memory_edge.rs:188`; `abi/crates/abi-wdbx-gateway/tests/episodes/memory_edges.rs:24`; `abi/crates/abi-cli/tests/episode_gateway/memory_edges.rs:10` | New `unwrap()` calls are in scratch test setup/assertions and deliberately fail the test when the expected successful operation fails. No new production `unwrap()` is introduced. | No change required. | Reviewed; accepted test use. |
| Informational | `wdbx/crates/abi-wdbx/tests/v3_memory_edge.rs:215`; `abi/crates/abi-cli/src/wdbx/episode.rs:400` | The edge clone preserves a fixture for a later cycle. The existing status clone supplies an owned output field. The new resolver projection formats borrowed bytes; no unnecessary new production clone was found. | No change required. | Reviewed; accepted ownership. |
| Informational | `abi/crates/abi-wdbx-gateway/src/episodes.rs:219`; `abi/crates/abi-wdbx-gateway/src/executor.rs:140` | Resolver, receipt, signature, and open/closed state are read inside the existing blocking executor job under one state guard. No await occurs while that guard is held, and the patch adds no lock. | No change required. | Reviewed; consistent projection. |

The resolver scan is linear in the in-memory record count, consistent with adjacent lookup APIs and the existing bounded ledger. No performance qualification is inferred from this review.

## Spec

**Independent specification axis: scoped PASS.** No actionable missing requirements, incorrect behavior, or scope creep in the frozen patches.

| Severity | File:line | Description | Suggestion | Status |
| --- | --- | --- | --- | --- |
| None | `wdbx/crates/abi-wdbx/src/v3/episode/store.rs:516`; `abi/crates/abi-wdbx-gateway/src/episodes.rs:219`; `abi/crates/abi-wdbx-gateway/proto/gateway.proto:149`; `abi/crates/abi-cli/src/wdbx/episode.rs:399` | Spec §4 asks for “the closing `resolves` digest, if any” and “matching additive fields” (`abi/docs/superpowers/specs/2026-09-16-spec-memory-edge-episodes.md:165`). The query selects the exact target and guild from canonical records. Gateway field 9 carries empty or 32-byte values, and CLI text/JSON expose digest or `none`. | Retain the per-edge qualification boundary. | Satisfied. |
| None | `wdbx/crates/abi-wdbx/src/v3/episode/store/validate.rs:363`; `wdbx/crates/abi-wdbx/src/v3/episode/store.rs:684`; `wdbx/crates/abi-wdbx/src/v3/episode/store/validate.rs:183` | Spec §3.3 says “Resolving closes that edge” and an already-closed edge is `InvalidTransition` (spec `:104`). Admission requires a same-guild open edge; closure removes it. Replay recomputes commitments and reruns transition validation before exposing records, preserving canonical authority. | No change required. | Satisfied. |
| None | `wdbx/crates/abi-wdbx/tests/v3_memory_edge.rs:182`; `abi/crates/abi-wdbx-gateway/tests/episodes/memory_edges.rs:207`; `abi/crates/abi-wdbx-gateway/tests/episodes/memory_edges.rs:6` | Inspected regressions cover both edge kinds, malformed/wrong guild, unknown/nonedge/resolution inputs, open/closed state, two cycles, duplicate closure, and reopen. Gateway tests cover exact resolver projection, reopen, and protobuf compatibility in both directions. | No change required. | Covered by regression source and supplied logs. |

This pass does not qualify complete candidate history, retrieval weighting, deployed adapters, or full constitutional memory conformance.

## Validation evidence

These are inspected worker artifacts, not reviewer reruns. Recorded process exits come from the receipt; log contents confirm the results below. The review independently verified each log's SHA-256 against the receipt.

| Evidence | Observed result | Status |
| --- | --- | --- |
| `/tmp/memory-edge-wdbx-red-20261003.log`, gateway red log, CLI red log | Runtime failures at the newly added resolver assertions before their respective implementation steps; receipt exit 101 for each. | Meaningful RED evidence inspected. |
| `/tmp/memory-edge-wdbx-green-20261003.log` | Cross-language episode: 2 passed; candidate: 6 passed; edge: 9 passed. Zero failed/ignored. Receipt exit 0. | Focused WDBX evidence; existing commitment goldens remain exercised. |
| `/tmp/memory-edge-abi-final-20261003.log` | Gateway integration: 12 passed; CLI integration: 2 passed. Zero failed/ignored. Receipt exit 0. | Final focused ABI evidence after test refactor. |
| `/tmp/memory-edge-clippy-final-20261003.log` | Finished successfully; receipt exit 0 for focused gateway/CLI integration Clippy with `-D warnings`. | Focused lint evidence. |
| Receipt checks | Owned Rust formatting exit 0; largest owned Rust file 914 lines; generated bindings originate from the unchanged gateway `build.rs`. | Inspected receipt and source. |
| Reviewer `git diff --check` on tracked owned WDBX/ABI paths plus benchmark | Exit 0 in each repository. | Independently executed. |
| Full repository gates | Not run by these reviewers. | Pending coordinating agent acceptance. |

CLI resolver coverage uses a separate CLI process against a real loopback scratch gateway and checks both output formats (`abi/crates/abi-cli/tests/episode_gateway/memory_edges.rs:58`). Contradiction and replay semantics are independently covered below that CLI presentation layer.

## Separate appendix: benchmark wording

**PASS; zero actionable findings.** Scope is only the output-string change at `abi/crates/abi-cli/src/wdbx/benchmark.rs:77`, reviewed against HEAD separately from the nine resolution files. Reviewed file SHA-256: `26b3fc5cfc457762b6ff196691018988de382f7afe84807a865aeac08c569115`.

| Severity | File:line | Description | Suggestion | Status |
| --- | --- | --- | --- | --- |
| None | `abi/crates/abi-cli/src/wdbx/benchmark.rs:25`; `:32`; `:57`; `:77` | The benchmark constructs an in-memory four-dimensional HNSW index, inserts `[i % 97, i % 31, 0, 0]`, repeatedly searches `[1, 0, 0, 0]`, and discards results without measuring recall. The amended wording accurately identifies this workload and removes the unsupported per-operation acceleration-dispatch claim. Timing logic is unchanged. | No change required. | Source-verified. |
| None | `wdbx/crates/abi-wdbx/src/hnsw.rs:318`; `:373`; `:587`; `:696`; `wdbx/crates/abi-wdbx/src/index.rs:234` | Insertion/search use HNSW traversal and its distance helpers; those call the scalar cosine implementation. No durable-store operation or accelerator dispatch appears in this measured path. | Keep any future accelerator or recall claims tied to separate measured evidence. | Source-verified; no benchmark run claimed. |

Standards: **0 actionable findings; scoped PASS**. Spec: **0 actionable findings; scoped PASS**. Benchmark appendix: **0 actionable findings; PASS**. Broad-gate and deployment acceptance are not part of this verdict.

## Coordinator acceptance after review

The subsequent root-owned full gates both passed on unchanged source, HEAD and
index: WDBX 647 Rust executions; ABI 988 Rust executions and 144 Python tests.
The review verdict above remains scoped to its frozen diff. Full receipts are
retained in `.superpowers/sdd/2026-10-03-ecosystem/evidence/` under
`resolution-final-gates/` and `resolution-abi-python314/`. The initial ABI Python
selection failure is retained; its full rerun used a process-local Python 3.14.8
shim. CUDA was unavailable. No deployment is implied.
