# WDBX 3D source audit — T00, 2026-10-20

Scope: source qualification for the approved local T00–T06 packet. This is not a
complete repository audit, current runtime qualification or claim promotion.
CI remains paused; no commit, push, deployment or live-store change is authorized.

## Source identities and read coverage

WDBX baseline is `08a69b3f22db61093e06527b3308b10589687022`; ABI is
`ca4a725c07ff75f885eb60bd59fe9797e12e4d2c`. The task worktree becomes dirty
as concurrent implementation proceeds; ABI was clean at inventory. The
[public manifest](../../claims/wdbx-3d/source-manifest.json) hashes baseline Git
object bytes, compares working bytes and records explicit read ranges. Run
receipts must independently bind their actual dirty build inputs.

The root and source-audit workers collectively read complete current forms of
ABI North-Star, multiway, agent architecture, canonical episodes, completion
design and member erasure; WDBX claims and README; ABI constitution; the full
original embedded specimen; browser/native runtime profiles; and the actual
acceptance ledger. Full-read rows describe collective inspection, not every
worker independently reading everything. The root separately recovered original
specimen 1470–1505 after truncated output. The source-audit worker read complete
v6 (1–1636) and requirements JSON (1–882), recovering truncated middle portions.
V5 inspection is deliberately scoped to 1–624, 2511–2610, 2645–2725 and
2766–2804: shared contracts, DAG, T00–T06, original acceptance matrix and source
register. Later packets are not freshly audited. `read_scope: first-packet`
records that limit. Hash-only fixture/lockfile rows do not imply full reads. Root Cargo.lock uses
`read_scope: dependency-inventory`, with its full SHA-256 and actual local-package
read range1–45. Studio Cargo.lock1–42 was read to reconcile its git sources.
Machine metadata and build identities are recorded separately under
`docs/execution/spatial-checked-v1/evidence/metadata.txt` and
`docs/execution/spatial-checked-v1/evidence/t01-source-hashes.json`; T01 owns
those run artifacts, which this source inspection does not promote.

Supplied artifacts have immutable SHA-256 identity and `commit: null` with
explicit non-Git provenance. No git revision was invented. Public outputs omit
operator paths, author emails, credentials and private runtime records. Reading
an authority establishes its wording, not its historical execution claims.

## Actual implementation seams

| Boundary | Observed source contract | Consequence |
|---|---|---|
| WDBX | Five abi-* crates; edition 2024, floor 1.99, pinned 1.99.1 | Preserve layering and identities |
| ABI | Manifest uses sibling WDBX path dependencies | Separate task worktree does not automatically retarget ABI |
| Studio | Current workspace is embedded `specimen-studio/`, separately excluded Cargo workspace | Do not modify/recreate the retired standalone repository |
| Studio dependencies | Core/kernel Cargo manifests and lockfile pin `3ac03f085196dbe6e922a127ceaa37bc2368de89` | Preserve pin; current substrate evidence cannot qualify this distinct revision |
| Legacy store | DurableStore over checkpoint/WAL/snapshot | Compatibility/migration remains separate |
| v2 | Actual root `crates/abi-wdbx/src/v2.rs`; independent journals/heads/segments/artifacts/leases | No invented `v2/mod.rs` or replacement canonical store |
| v2 identities | `RecordId::Legacy(u64)` / `V2(Uuid)`; causal `Version<T>`; V2SpatialRecord f32 | No narrowing to octree u32; T08 owns durable identity mapping |
| v3 | Commitment and episode modules; EpisodeStore/signing/closed policy/event types | Transport JSON is not canonical authority; no v2/v3 atomicity claim |
| Compute | Object-safe Accelerator returns dense ScoredIndex; score/index ordering | Storage owns mapping; retain cycle-free compute seam |
| CPU | Safe fixed-width f32 lanes; successful operation sets executed/runtime_verified | Existing capability behavior is not independent numeric, ISA or speedup qualification |
| Spatial | f32 Point3D, u32 IDs; three metrics; lazy octree threshold64/depth16 | Add checked APIs, preserve legacy methods; qualify topology in T06 |

ABI frozen command/tool authority lives in `usage.rs::COMMANDS` (14) and
`handlers.rs::TOOLS` (12), backed by ABI `tests/golden/`. Studio preserves its
nine original slash commands, visible editors/file pickers and learning gate.
The actual ABI `proto/gateway.proto` package `abi.wdbx.gateway.v1` has ten RPCs:
PutVector, Search, PutKv, GetKv, ResolveConflict, Stats, MembershipChange,
WatchMutations, ProposeEpisodeWrite and VerifyEpisode. Episode writes carry JSON
bytes plus preview_only; canonical ownership stays in the store. VerifyEpisode
returns sanitized receipt, signature/key status and memory-edge state. No new
spatial RPC is introduced. REST retains its five loopback routes; custom MCP
HTTP+SSE is not Streamable HTTP.

Compatibility inventory includes original WDBX format/manifest/segments, v3 CBOR
vectors, memory-candidate/edge JSON, Abbey projection and ABI help/MCP goldens.
Relevant existing executable targets include v3_canonical_commitment,
v3_episode_store, v3_episode_signing, v3_cross_language_commitment,
v3_cross_language_episode, v3_memory_candidate, v3_memory_edge, abbey_contracts,
abbey_bot_projection_conformance, zig_writer_compat, multiway_zig_compat and ABI
gateway episodes/gateway. Existence/hash identity is not execution. Optional
historical Zig comparison requires its separately supplied binary.

## Acceptance ledger and missing evidence

The actual ledger is **`specimen-studio/VALIDATION.md` (1–1018)**, headed
“Release-candidate acceptance ledger.” It calls itself pre-freeze acceptance
authority and assigns frozen-SHA final results to an external qualification
artifact. Its later section links `docs/verification/2026-10-08-main-integration.md`
(33 lines). Both were fully read and inventoried as **reported historical**
evidence. Their counts, UI/model/device/signing reports do not qualify the new
stable migration or current dirty tree.

The external private `wdbx-source-qualification.md` final receipt was not supplied
or opened; no raw historical artifacts were replayed. This limits reproducing
historical claims and does not block unrelated approved checked-point work.
The supplied package also lacks `WDBX-3D-Neural-Specimen-MASSIVE-Master-Plan.md`.
V5 reports 125,344 bytes/1,016 lines and SHA-256
`3cf8200fb9cb346a9b89aa5b329507f594411b37f4aad5510334ca4b52db340b`.
Those are inherited identifiers, not independently verified bytes or read ranges.
Full original-artifact provenance remains open; supplied v6/v5 first-packet
sources are available and pinned.

## Discrepancy register

| ID | Conflict | Treatment / owner / closure evidence |
|---|---|---|
| D01 | Constitution599–606 versus claims.md table/completion design | Constitution defines C0 specification, C1 tests, C2 replay, C3 offline evaluation, C4 shadow, C5 canary, C6 witnessed outcome, C7 sustained reliability. Claims prose maps C0 tests, C1 replay, C2 cross-language, C3 provider, C4 hosted, C5 production, C7 experiment manifest. Preserve original docs; report concrete evidence without level promotion. Constitution owner must reconcile before promotion; numeric implementation unaffected |
| D02 | Agent architecture says nightly; North-Star names std::simd; manifests/CPU are stable1.99.1 safe lanes | Source governs build state; T01 qualifies without repeating migration or inferring emitted ISA |
| D03 | North-Star “all eight” gateway methods versus ten in proto | Actual schema controls; T02 preserves consumer fixtures |
| D04 | Native runtime pinned3ac03f versus substrate08a69b3 | Preserve dependency; separate T01 consumer evidence required |
| D05 | V5 discovers retired Studio repository | Closed by embedded workspace/AGENTS/manifests at baseline |
| D06 | Erasure rev3 has empty approval; plans paraphrase hourly behavior | Original exact wording and separate approval govern; no custody/erasure implementation; security owner/T54 |
| D07 | Original massive artifact and private final receipts absent | Preserve reported identities/limitations; no fabricated read/execution; provenance custodian remains responsible |

No original source was changed to hide these conflicts. Original specimen equality
codec, same-origin redundancy predicate, grave retention/resurrection and
unqualified synthesis/attention choices remain gates for dependent full
conformance work, not permission to invent replacements.

## Scoped ADR record

No separate established ADR directory/procedure was found in repository docs.
These decisions implement explicit user first-packet direction; no constitutional
or later-catalogue approval is implied.

- **ADR02/T03:** preserve f32 bytes/APIs; add private validated f64 point/radius
  types with explicit widening. Reject silent precision migration. Validate
  numeric domain/boundaries and compatibility. Rollback removes additive exports.
- **ADR03/T06:** reuse existing octree only after independent complete reference
  parity. Defer replacement tree/store. Retain complete fallback where bounds
  cannot qualify. Test boundaries, degenerate data,63/64/65,invalidation and
  seeded cases. Rollback disables optimized checked path.
- **ADR18/T01:** preserve `bash tools/check.sh`, source-size gate, current warning
  policy and separate Studio gates. No new WDBX -D warnings or blanket
  all-features gate. Optional FHE tuples remain separate. CI stays paused.

## Verification boundary

Root owns validator red/green logs, named counts and final run receipt. Manifest
admission checks typed inventory/read coverage, not runtime correctness or
historical execution. T01 owns pinned local gates and consumer limitations; T02
owns compatibility execution. This documentation worker ran only manifest
admission and `git diff --check`; both exited0 at this checkpoint. No broad
Cargo, Studio, hardware or production gate was run by this worker. Stop for
review after T00–T06; this audit does not qualify later profiles.
