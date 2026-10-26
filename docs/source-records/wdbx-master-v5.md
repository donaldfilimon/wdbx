# WDBX 3D Neural Specimen — Master Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: use `superpowers:subagent-driven-development` when independent workers and reviewers are actually available, or `superpowers:executing-plans` for inline execution. Execute approved task packets, not the entire roadmap in one unattended run. Checkboxes record implementation work, not document-authoring progress. No commit, push, deployment, production-data modification, or destructive action is authorized by this document.

**Goal:** Evolve the existing WDBX substrate into a correctly scoped, snapshot-consistent 3D/vector/graph database, with separately qualified CPU/GPU/NPU/cross-node execution and full preservation of the original Neural Specimen requirements.

**Architecture:** Retain the `abi-*` crates, `VersionedStore`, historical v1/v2/v3 formats, current octree, HNSW, causal conflict semantics, and the separate specimen runtime. Add validated query contracts and rebuildable index generations around those boundaries. Authorization, canonical persistence, derived search, hardware execution, and specimen behavior have distinct authorities and evidence requirements.

**Tech Stack:** Stable Rust **1.99.1**, edition **2024**; existing WDBX/ABI dependencies and gateway; existing browser/native specimen clients. Metal, CUDA, Vulkan, Core ML and other inference providers are optional adapters. No production dependency on unstable portable SIMD; no automatic addition of an external database, model service, renderer, or microservice.

**Spec:** The user-supplied WDBX 3D Original-Spec-Preserving Master Build Directive; the seven named ABI/WDBX source documents; the original 26-chapter Specimen Architecture and Behavioral Specification, including R01–R23 and I01–I24; and the inherited integrated architecture catalogue retained in Appendix A of this document.

**Document revision:** 5.0 — source-reconciled execution edition.  
**Prepared:** 2026-10-20.  
**Supersedes for planning:** revision 4.0 of `WDBX-3D-Neural-Specimen-MASSIVE-Master-Plan.md`.  
**Original artifact SHA-256:** `3cf8200fb9cb346a9b89aa5b329507f594411b37f4aad5510334ca4b52db340b`.  
**Original artifact:** 125,344 bytes; 1,016 lines. The original file is retained unchanged.  
**Implementation status:** Proposed. This revision records source inspection and document checks, not a new WDBX build, test run, benchmark, deployment, security audit, or approval of the erasure draft.

## Global Constraints

- Preserve existing public crate identities, legacy numeric and UUID record identities, command/tool names, gateway contracts, and sibling dependency relationships unless an explicitly reviewed compatibility change requires otherwise.
- Preserve original signed bytes, digests, v2 parent ordering, v3 canonical commitments, detached signature semantics, v1 compatibility, migration recovery copies, and explicit causal conflict resolution.
- Preserve HNSW, the existing lazy octree, temporal/causal/persona ranking, multiway event multiplicity and resume behavior, compression, authenticated-object options, and research/demo boundaries.
- Preserve every original specimen chapter, R01–R23, I01–I24, default **20 entries per node**, default **1,000 simultaneous high-resolution comparisons per specimen**, confidence range **[-100,100]**, and default **60–120 seconds of eligible idle time** between idle events.
- Physical geometry, semantic vectors, and visual projections are separate types and namespaces. A projected location is not physical position or evidence of causation.
- Use **Rust 1.99.1** and edition **2024** for the qualified production profile. Retain `unsafe_code = "deny"`; isolated unsafe or FFI exceptions require an approved ADR and dedicated review.
- Authorization must protect content and derived observables. Existing bearer authentication or persona affinity does not, by itself, establish row-level tenant authorization.
- An exhaustive scan is not automatically mathematically exact arithmetic. State numeric semantics, approximation, snapshot coverage, and authorization coverage independently.
- No unbounded allocation, queue, traversal, retry, device submission, speculative task, or autonomous mutation. Enforce budgets before allocation/submission, not only after work completes.
- Cancellation before commit can prevent mutation; cancellation after commit cannot truthfully undo it. Never report a committed operation as an uncommitted cancellation.
- Preserve the member-erasure draft's separate approval and constitutional-amendment requirements. Planning, ordinary spatial work, and test fixtures do not authorize key destruction, production migration, or erasure.
- Never use an operator's live store, credentials, member facts, backups, or application-data directory as a fixture. Do not change backup settings or delete recovery copies.
- Code presence, compilation, execution, numerical qualification, performance qualification, device placement, and production approval are different facts. Preserve Current/Partial/Proposed and C0–C7 without automatic promotion.
- All proposed file paths, interfaces, defaults, and tests below are design decisions, not claims that those files or APIs already exist.

## Review Focus

| Failure class | Required behavior | Owning tasks |
|---|---|---|
| Finite but extreme coordinates; exact-boundary ties; legacy IDs wider than a dense slot | No overflow, false-negative pruning, silent narrowing, or identity loss | T03–T08 |
| Old snapshots combined with revoked policy or mismatched index generations | Bind the complete read view; fail closed or reconcile a proven complete delta | T11–T16 |
| Commit succeeds while cancellation, disconnect, or retry races it | One authoritative terminal result; no duplicate durable effect or false rollback claim | T15, T25, T39, T49 |
| A disconnected worker keeps running after its lease expires | Uncertain credits remain reserved; a strict global scan cap trades availability for safety | T33, T44, T52 |
| An accelerator, ANN planner, or shard merge omits a winning candidate | Exact mode proves coverage; otherwise return an explicit approximation/completeness contract | T18–T20, T30–T45 |

---

## Contents

1. [What changed and why](#changes)
2. [Evidence baseline and authority](#baseline)
3. [Scope, profiles, and requirement coverage](#scope)
4. [Architecture decisions](#adrs)
5. [Normative interface and behavior contracts](#contracts)
6. [Milestones and dependency graph](#milestones)
7. [Task-by-task implementation packets](#tasks)
8. [Verification, fault injection, and benchmarks](#verification)
9. [Original specimen traceability](#traceability)
10. [Execution handoff and stop rules](#handoff)
11. [Source register](#sources)
12. [Appendix A — retained integrated architecture catalogue](#architecture-catalogue)

<a id="changes"></a>
## 1. What changed and why

This revision improves the existing plan rather than replacing the project. It removes conflicting execution instructions, narrows the first shippable milestone, replaces generic chapter checklists with named behavioral tests, and separates demonstrated repository facts from new architectural proposals. The broad original architecture remains in Appendix A, with the conflicting passages reconciled to the contracts below.

| Change ID | Problem in revision 4.0 | Revision 5.0 resolution | Tasks |
|---|---|---|---|
| CH01 | Repeatedly treated stable Rust adoption as future work | Current source already adopts 1.99.1; qualify that change instead of repeating it | T00–T02, T31–T32 |
| CH02 | One spatial interface used `f32`; another required `f64` | Keep historical `f32` bytes and APIs; new geometry is validated `f64`; widening is explicit and narrowing is never implicit | T03, T07–T10 |
| CH03 | New-index language could duplicate the existing octree | Start with additive checked access to the existing index; reuse its topology only after independent oracle qualification | T04–T06 |
| CH04 | Three incompatible accelerator status ladders | Preserve existing five capability facts and attach a separate per-operation qualification record | T30–T38 |
| CH05 | CPU backend names could be mistaken for emitted SIMD evidence | Lane width, ISA eligibility, generated instructions, executed backend, and measured speedup are separate fields | T01, T31–T34 |
| CH06 | Cancellation was described as universally preventing durable writes | Define a commit/cancel linearization point, `Committed`, `CancelledBeforeCommit`, and `OutcomeUnknown` reconciliation | T15, T25, T39, T49 |
| CH07 | Scalar snapshot/generation language blurred v2, v3, policy, and shard state | Use a read-view bundle; do not invent a global transaction or global snapshot across independent authorities | T11–T16, T42 |
| CH08 | “Leases with expiry” did not explain still-running workers | Retain uncertain reservations until cessation is established; fencing results alone does not stop computation | T33, T44 |
| CH09 | Exact reranking could be read as proving exact retrieval | Add independent candidate-coverage and numeric-accuracy fields; reranking cannot recover omitted neighbors | T18–T20, T42 |
| CH10 | PCA seed/sign rules overpromised portable bitwise reproducibility | Separate frozen-profile replay from cross-platform equivalence; handle repeated eigenvalues and cohort privacy | T26–T27 |
| CH11 | DirectML appeared among generic NPU options | Classify each provider and actual device separately; DirectML is a GPU provider, not ANE/NPU proof | T37–T38 |
| CH12 | Old-reader rollback assumed every new record was readable | Introduce minimum-reader/writer gates and a rollback compatibility matrix; do not start an incompatible writer | T10–T14, T53, T57 |
| CH13 | Blanket `--all-features` and warning promotion could misstate the repository gate | Use locked, supported feature combinations and retain the existing lint policy; detect zero-test success | T01–T02, T55 |
| CH14 | The 26-row coverage table repeated one generic instruction | Give each chapter and each invariant/resolution a specific test obligation and owner | T46–T53; section 9 |
| CH15 | Privacy language implied all timing leaks or old plaintext borrows could be revoked | State trusted-boundary limitations, revoke new materialization, invalidate derivatives, and report residual exposure | T16, T28–T29, T54–T55 |
| CH16 | Geometry, accelerator, cluster, and all specimen work could become one oversized release | Define independently releasable profiles and a dependency DAG, with optional features unable to block the small core slice | All milestones |
| CH17 | Multiway persistence could be inferred to be one atomic transaction | Preserve old behavior and add a versioned completion marker for new publication; no cross-store atomicity claim | T25 |
| CH18 | “Final” could imply all features were verified | Treat finality as a reviewed artifact revision; conformance remains an evidence-backed, profile-specific claim | T00, T55–T57 |

### 1.1 The revised first deliverable

The first implementation packet is **a checked, bounded spatial query path around the existing point index**, plus an independent oracle and compatibility evidence. It changes no canonical storage format, no signed record, no gateway, no cluster protocol, no model runtime, and no production data. Frames, `RecordId`-qualified snapshots, durable generations, and new shape storage follow as separate packets.

Stable-toolchain qualification is a prerequisite because the shared compute migration is recent; it is not a reason to rewrite `abi-compute` again. The recent source makes independent numeric qualification particularly valuable: its CPU implementation uses safe fixed-width lane accumulation, while the public backend labels remain compatible with older labels. [SRC01–SRC04]

<a id="baseline"></a>
## 2. Evidence baseline and authority

### 2.1 Source observations made while preparing this revision

| Item | Evidence captured | What it establishes | What it does not establish |
|---|---|---|---|
| Input planning artifact | SHA-256 above; all 1,016 lines read | Exact revision being improved | Implementation or acceptance |
| WDBX branch | `08a69b3f22db61093e06527b3308b10589687022`, commit dated 2026-10-19 | Current branch revision at retrieval; commit says stable Rust 1.99.1 adopted | A test run performed during this revision |
| Commit validation note | Reports `cargo check --workspace --all-targets --locked`; records tests/CI paused | Historical compile-only claim by that commit | Runtime parity, performance, or permission to restart CI now |
| `abi-compute/src/cpu.rs` | Blob `15fc17f5f291ec281c32ea62f0e06fdeb13f38e0`, complete file read | Safe lane accumulation, public CPU methods, current tests present in source | Independently validated numerics, emitted ISA, or passing tests |
| `abi-compute/src/backend.rs` | Blob `2336c6dda9a605e1de288e00291970c2e1344e83`, lines 1–300 read | Existing five capability facts and dispatch checks | A new qualification enum already exists |
| `abi-wdbx/src/spatial.rs` | Blob `958314753c0608b2356131437c48bd24984767fd`, lines 205–430 re-read | Existing insertion, octree traversal, radius/kNN, and distance-only tie logic | A reproduced runtime defect or a passing proposed fix |
| Rust release | Official 1.99.1 announcement, 2026-10-15 | The requested release exists | This workspace is qualified on every target |
| Portable SIMD | Official standard-library docs identify it as nightly-only | Stable production must not depend on `std::simd` | Compiler auto-vectorization is impossible |

The current source's `CpuBackend::capability()` marks its instance runtime-verified after successful execution. Preserve that existing public behavior until a reviewed compatibility change says otherwise, but **do not use that flag alone as independent numerical qualification for the new scheduler**. The qualification record described in section 5 carries the test corpus, operation, model, device, build, tolerance, and evidence artifact. [SRC02–SRC03]

This is a targeted source reconciliation, not a fresh full-repository audit. The original North-Star, episode, erasure, gateway, multiway, and specimen requirements remain inherited requirements. T00 must pin and re-read their complete current forms before implementation. Do not relabel an earlier source inspection as a new runtime verification.

### 2.2 How to resolve competing sources

The user's latest explicit instruction and the ratified project constitution determine authorized intended behavior. Existing signed formats and frozen public contracts determine what must remain interoperable. Source and tests determine what is currently implemented; they do not turn a documented bug into the intended design or authorize a privacy regression. Record disagreement in the discrepancy register, identify the controlling requirement, and make a reviewed compatibility decision.

The user requests plan improvement here, not implementation. The historical test pause is recorded as evidence context. The newer user directive requires TDD and verification for subsequent authorized implementation. Do not infer from either statement that tests or CI ran while this Markdown was revised, and do not create an unnecessary new pause on writing the plan.

### 2.3 Required audit outputs

T00 produces a source manifest containing repository, branch, exact commit, clean/dirty state, relative path, blob or content digest, read range, owner, role, and observed-versus-reported evidence. It includes the original specimen specification, all seven directive documents, the original command contracts, the actual gateway schema, the claims ledger, toolchain/lockfiles, and the current tests. Source manifests must omit private email addresses, credentials, operator paths, and runtime data from public exports.

Every discrepancy has: stable ID; conflicting passages or source spans; observed behavior; approved intended behavior; compatibility risk; assigned task; approval needed; and closure evidence. A changing `main` URL is a discovery pointer, not an immutable implementation input.

<a id="scope"></a>
## 3. Scope, conformance profiles, and coverage

### 3.1 What ships independently

| Profile | Required scope | Excluded from its acceptance gate | Candidate milestone |
|---|---|---|---|
| `spatial-checked-v1` | Validated legacy point access, numeric contract, complete CPU oracle, current octree qualification, compatibility | New persistent shapes, network services, accelerators, full specimen implementation | M1 |
| `spatial-durable-v1` | Additive geometry/frames, stable identity/version hits, pinned reads, verified index publication, policy binding | GPU/NPU and multi-host operation | M2 |
| `hybrid-evidence-v1` | Authorized vector/spatial/lexical/graph/temporal retrieval; explicit evidence and approximation | Learned rankers or generative-model quality claims | M3 |
| `explorer-v1` | Authorized read-only projection/scene API, text alternative, bounded UI | New mutation authority, native hardware execution claims | M4 |
| `accelerated-<backend>-v1` | One named operation/model/device profile qualified against its CPU/reference contract | Other vendors, models, dtypes, shapes, and residency not measured | M5 |
| `cluster-causal-v1` | Named causal/durability guarantees, mTLS, scoped fan-out, repair/rebalance, actual host tests | Global serializability and Byzantine fault tolerance | M6 |
| `specimen-compatible-<runtime>-v1` | All original requirements satisfied or explicitly disclosed as gaps for that runtime | Browser/native equivalence inferred without testing | M7 |
| `production-<profile>-v1` | Selected profile plus security, capacity, operational, recovery, rollback, and operator review | Any unqualified optional feature | M8 |

These profiles replace the earlier overloaded P0–P4 rollout labels. They do **not** replace the constitution's C0–C7 evidence ladder. A feature can be source-present, operationally disabled, and unqualified at the same time. Production scope is a named subset; the phrase “full original conformance” is reserved for a complete accepted traceability matrix, not the first spatial release.

### 3.2 Non-goals for the first spatial packet

No global schema redesign, crate renaming, embedded-database replacement, new WAL format, native model download, online training, full geodesy, mesh/ray engine, vector compression replacement, public Internet listener, cross-host consensus change, or member-fact erasure. Preserve those broader requirements in the roadmap; excluding them from M1 is not removing them from WDBX.

### 3.3 Master-directive requirement map

| Requirement | Concrete work | Acceptance evidence |
|---|---|---|
| A — preserve original functionality | T00–T02, T08–T15, T25, T46–T57 | Original API/format/behavior fixtures; source-digest inventory; rollback matrix |
| B — native 3D | T03–T14 | Geometry oracle; frames; identity/conflict coverage; published-generation recovery |
| C — vector intelligence | T17–T20, T31–T38, T56 | Embedding-space validation; multi-vector aggregation; filtered recall; hardware comparison |
| D — projection | T26–T29 | Cohort/seed/profile manifest; quality/reproducibility tests; bounded client rendering |
| E — cognitive graph | T21–T22, T25, T48–T52 | Typed edge validation, bounded paths, multiway multiplicity and attachment semantics |
| F — evidence and memory | T22–T24, T49–T54 | Eight dimensions; unknown/invalid distinctions; original learning/lifecycle gates |
| G — security/erasure | T15–T16, T28–T29, T39–T45, T54–T55 | Scope tests, policy rechecks, worker trust, explicit erasure approval boundary |
| H — Rust | T01–T02, T30–T38, T55 | 1.99.1 locked build matrix; no weakened lints; isolated reviewed native boundaries |
| I — additive API/explorer | T10, T28–T29, T53 | Versioned schemas, bounded responses/cursors, read-only defaults, compatibility goldens |
| J — verification | Every task; T55–T57 | Red/green evidence, nonzero test discovery, fault corpus, benchmarks, limitations |
| K — execution order | Milestone DAG and task dependencies | First slice remains independent; no accelerator/cluster prerequisite for M1 |
| CPU/GPU/NPU/TPU | T30–T39 | Per-operation evidence; device-class/provider distinction; policy-valid fallback |
| Cross-node | T39–T45 | Exact replication identity, causal-cut evidence, strict credit accounting, real-host faults |
| Original 26 chapters / R01–R23 / I01–I24 | T46–T53 plus section 9 | Individual named regression obligations and runtime-specific evidence |

<a id="adrs"></a>
## 4. Architecture decisions for review

Each decision below is **proposed for approval**, not silently ratified by this document. Approval of one enables only its dependent task packet. Each ADR records compatibility, alternatives, evidence, rollback, and decision owner. No global approval of all 18 is needed before reviewing the small spatial slice.

| ADR | Proposed decision | Alternative considered and reason not selected now | Gate |
|---|---|---|---|
| ADR01 | Retain canonical v1/v2/v3 stores and add namespaced attachments plus derived indexes | New universal v4 database would create unnecessary migration and authority risk | Before T10 |
| ADR02 | Keep legacy `f32` storage/API; validated new geometry uses `f64` and explicit widening | Silently changing the old record precision invalidates compatibility expectations | Before T03 |
| ADR03 | Keep the current octree; qualify it before selecting R-tree/BVH/grid by workload | Replacing it before measuring repeats existing functionality without evidence | Before T06 |
| ADR04 | Stable identities and causal version identities form hit keys; dense slots are generation-local | Casting UUIDs or large legacy IDs to `u32` loses identity | Before T08 |
| ADR05 | New physical queries require an explicit frame; legacy-unbound points remain unitless until bound | Guessing meters or a world frame manufactures location semantics | Before T09 |
| ADR06 | Read views bind v2 frontier, relevant v3 head, policy epoch, frame registry, and index generations | One `revision: u64` cannot represent independent authorities | Before T11 |
| ADR07 | Immutable index files have content identity; activation is a separately synchronized mutable reference | A checksum-only rename without source coverage can activate an incomplete generation | Before T12 |
| ADR08 | Exact/approximate coverage, numeric accuracy, and partial completion are independent response fields | One `exact: true` flag hides ANN omissions and shard failures | Before T18 |
| ADR09 | Preserve the old ranker; introduce versioned score/evidence policies with explicit unknown values | An opaque replacement breaks consumers and conflates evidence with similarity | Before T23–T24 |
| ADR10 | Keep existing capability facts; add operation/model/device/build-specific qualification | Another renamed enum would break compatibility and still conflate execution with quality | Before T30 |
| ADR11 | Policy-first dispatch and an acyclic, deadline-bounded fallback plan | A universal CPU→GPU→remote order ignores the requested backend, memory, and privacy | Before T33 |
| ADR12 | Trusted-node causal data plane; reviewed consensus only for control metadata where required | Globally linearizable user data or Byzantine tolerance are larger, unproven changes | Before T40 |
| ADR13 | Quorum ACKs describe durable copies; snapshot consistency requires causal dependency closure | Quorum alone does not supply linearizability or a coherent cross-shard cut | Before T41–T42 |
| ADR14 | Hard global specimen scan cap uses non-reassignable uncertain reservations | Lease expiry does not terminate a running CPU/GPU/NPU operation | Before T44 |
| ADR15 | Frozen execution-profile replay and tolerance-based portable projection equivalence are distinct | Seed plus eigenvector sign does not resolve all PCA degeneracy or platform variation | Before T26 |
| ADR16 | Specimen adapters preserve original semantics and per-runtime profiles | Moving all specimen logic into the database core would blur control and data ownership | Before T46 |
| ADR17 | Erasure is an approval-gated, separately specified custody/lifecycle program | Spatial work must not silently implement an unapproved constitutional amendment | Before T54 implementation |
| ADR18 | Profile-specific feature-matrix CI and measured rollback compatibility | Blanket `--all-features`, warning escalation, or old-writer fallback can create false gates | Before T01/T55/T57 |

Decision owners are role-based: storage maintainer (ADR01/04/06/07), geometry maintainer (ADR02/03/05), retrieval maintainer (ADR08/09/15), compute maintainer (ADR10/11), distributed-systems maintainer (ADR12–14), specimen custodian (ADR16), security/constitution owner (ADR17), and release maintainer (ADR18). Donald retains authority over scope and the explicitly reserved constitutional/production approvals. No individual assignee or approval is fabricated.

<a id="contracts"></a>
## 5. Normative interface and behavior contracts

### 5.1 Names, ownership, and source compatibility

Repository roots are symbolic anchors established by T00: `WDBX`, `ABI`, and `STUDIO`. `STUDIO` may resolve to the separately managed specimen workspace if its location has changed; inspect its actual root rather than editing the archived repository by assumption. Within the task packets, paths beginning `crates/abi-wdbx/` or `crates/abi-compute/` are relative to WDBX unless another root is stated.

Existing `RecordId`, version identities, `V2Snapshot`, `VersionedStore`, `Accelerator`, and `CapabilityEvidence` remain the source-defined types. All other capitalized types below are proposed contract types owned by the indicated task. Do not duplicate an existing type merely to satisfy a spelling in the plan. T00 records the exact re-export and module file before integration; in particular, do not assume the v2 root is `src/v2/mod.rs` rather than an existing `src/v2.rs`.

New checked query code initially lives beside `src/spatial.rs` in `src/spatial3d/`; the old public module remains. Storage and compute adapters remain cycle-free. Device libraries must not become mandatory dependencies of the canonical store. A design-level type definition is not a runnable library API until its owning task and contract tests are accepted.

### 5.2 Geometry and numeric accuracy

**Legacy:** `Point3D`, `V2SpatialRecord` and old point payloads retain their existing `f32` representations. Existing methods preserve compatibility behavior. New `try_*` entry points reject invalid input without silently changing the behavior of old methods. M1 uses the existing `u32` point-index identity only inside that legacy index; T08 supplies full `RecordId` and version mapping.

**New geometry:** `Point3`, `Aabb3`, and `Sphere3` have private, validated `f64` components. In the initial `cartesian-f64-v1` profile, each coordinate and radius has magnitude at most `1e150`; radii are nonnegative. This is a proposed representability/resource boundary, not a physical-world claim. Every finite `f32` coordinate widens into the profile without narrowing or rewriting its original bytes. Values outside the profile fail with `CoordinateOutOfRange`; no clamping occurs. Geographic/geodetic queries and non-rigid transforms are not interpreted as Euclidean meters.

**Boundary semantics:** boxes and spheres are closed sets; zero-radius spheres match coincident points. AABB minima may equal maxima, but may not exceed them. `-0.0` and `+0.0` are geometrically equivalent; any canonical normalization applies only to new records, never old signed bytes. Non-finite inputs are rejected before indexing and before a query receives an allocation reservation.

**Three distinct oracles:**

1. A complete scan checks all eligible observations at the pinned snapshot. This establishes candidate coverage, not mathematical arithmetic exactness.
2. A deterministic float reference specifies widening, reduction order, FMA policy, norm implementation, and output ordering for the selected numeric profile.
3. A bounded dyadic reference treats finite binary floats as integer significands times powers of two and compares squared distances exactly for the test corpus and boundary-sensitive predicates. Use a separately reviewed implementation, not the same production function called twice.

New exact geometric mode must agree with the dyadic reference for supported point/AABB/sphere membership and squared-distance ordering. A conservative numeric interval can avoid the expensive fallback when its sign is decisive; otherwise use the bounded exact comparator. If the fast implementation has not proved a conservative bound, use the complete reference path instead of an unsafe prune. Do not add an arbitrary epsilon that changes the query's meaning. Distance values shown to users are rounded `f64` measurements and are not, by themselves, authoritative ordering keys.

M1 may ship the deterministic-float complete-scan profile first, with `numeric_accuracy = defined_float_profile`; it must not label that result `exact_real_predicate`. T07 closes the stronger shape/predicate profile. Cosine is retained as a legacy point metric but is not a Euclidean geometric distance; keep its complete-scan route until a sound pruning rule is established.

**Identity ordering:** new durable hits sort by the authoritative metric key, then `RecordId`, version identity, and attachment identity. Legacy duplicate `u32` IDs remain allowed where the old API allowed them; a stable insertion ordinal disambiguates those observations within that one index. Do not confuse repeated public specimen Pattern IDs with duplicate durable record identities.

### 5.3 Spatial identity, attachments, and conflicts

`SpatialObservationKey` is `(record_id, version_id, attachment_id)`. `DenseSlot` is an internal checked integer for one immutable index generation, with a bijection to observation keys. A slot is never serialized as a public record ID. Overflow or slot exhaustion fails before publication.

A new `SpatialAttachment` binds the source record identity, exact source version or digest, geometry profile, frame/units, valid-time interval, creator/authorization context, attachment schema, and its own content identity. It is a new durable object, not a change to the source episode. A link to a v2 value must name the value's version, not only its logical ID.

`ConflictMode` has explicit `PreferredView`, `AllMaximalVersions`, and `RejectConflicted` modes. Default new query behavior is `AllMaximalVersions`; a client selecting `PreferredView` receives a conflict-present indicator where authorized. Filtering preferred and concurrent values does not resolve them. Count fields distinguish observations from logical records. A limit counts returned observations unless a separately versioned record-level aggregation policy is selected.

New geometry persistence uses a reviewed namespace through the existing canonical boundary. If a new mutation/envelope version is necessary, a minimum-reader/writer gate is mandatory. The plan does not assume old binaries can parse new variants merely because those variants are additive in Rust.

### 5.4 Read view and index coverage

`ReadView` contains: database identity; v2 causal frontier including immutable transaction identities; any v3 episode head consulted; policy/consent epoch; frame-registry digest; requested conflict mode; index generation IDs; lifetime/lease metadata; and an opaque server-side authorized scope. A scalar generation counter is insufficient. A frontier with equal writer sequence numbers but conflicting object digests is an integrity problem, not equality.

A single-store query can pin an immutable snapshot. A combined v2/v3 query must explicitly bind both roots and declare whether the relationship is an atomic publication established by a protocol or an observed pair of roots. Do not retrofit a cross-store transaction claim onto independent existing ledgers. The new publication-marker protocol can make a completed derivative bundle visible atomically without pretending its underlying writes were one transaction.

An index has one of three coverage states relative to a view: `Complete`, `ReconciledWithDelta`, or `Unusable`. A delta must include inserts, updates, deletions/tombstones, conflict changes, policy changes, and frame changes affecting the predicate. A newer record cannot be added twice and an old coordinate cannot remain visible after replacement. Without a verified complete delta, exact mode scans the snapshot or fails under its budget; it never calls a stale index exact.

A durable index manifest binds source roots, algorithm/format/numeric profiles, build parameters, observation-key mapping, frame registry, scope/cohort identity, chunk sizes and digests, and counts. Its content identity excludes nonsemantic wall-clock build metrics. Activation stores the winning manifest reference separately. A file checksum does not replace authenticated source binding; secure profiles reject or rebuild untrusted derived content before it can influence a response.

### 5.5 Query semantics and result completeness

The new query API returns a structured `QueryOutcome`, not only `Vec<Hit>`:

```text
QueryOutcome
  read_view_id
  hits[]
  candidate_coverage: exhaustive | bounded_approximate
  numeric_accuracy: exact_real_predicate | defined_float_profile | bounded_model_error
  completion: complete | partial
  partial_reason?: deadline | work_budget | result_cap | unavailable_partition
  observed_generations[]
  consistency: local_snapshot | causal_cut | best_effort
  ranking_policy_id
  continuation?: opaque authenticated cursor
  explanation_summary
```

`complete + bounded_approximate` means the requested ANN plan completed, not that its candidates are globally exhaustive. `partial` is never inferred merely from fewer than `k` results. It means work or coverage is incomplete, and the reason is explicit. Unauthorized shard/record identities are not disclosed through failure metadata. An exact query that cannot complete within budget fails with a typed error unless the caller explicitly permits a partial response.

Record-level multi-vector aggregation is versioned: `MaxSimilarity`, `MeanSimilarity`, and a separately defined late-interaction policy. Document chunk multiplicity must not silently inflate one record's score. Embedding identity includes model digest, tokenizer/preprocessor, dimension, metric, normalization, dtype, and quantization lineage; equal dimensions do not make two spaces interchangeable.

Exact local top-k results can be merged only when every shard uses the same globally defined scoring function and stable total order, with correct record ownership and replica deduplication. Locally normalized scores and local reciprocal-rank-fusion ranks are not globally interchangeable. Hybrid/global RRF either gathers enough complete component ranks under a specified algorithm or declares approximation. Exact reranking does not recover neighbors missed by ANN or a truncated shard response.

### 5.6 Authorization and privacy boundary

`AuthorizedReadView` is constructed by a trusted policy verifier; it is not deserialized from a client's claimed tenant or a `keep: bool` field. New scoped APIs require identity, operation, purpose where applicable, resource policy, and an epoch/revocation decision. Reuse verified ABI mechanisms where they actually exist; build missing scoped authorization explicitly rather than assuming bearer auth supplies it.

The strict isolation profile partitions candidate indexes and projection cohorts at a reviewed authorization boundary. It never traverses forbidden rows merely to remove them later. A broader trusted-domain profile can permit filtered internal traversal, but it must document leakage assumptions and may not advertise timing noninterference. Authorization checks protect counts, distances, edges, projection centroids, failures, caches, logs, exports and cursor state as well as payloads.

At materialization and streaming boundaries, recheck the policy epoch. Abort or restart under a new view after revocation; do not silently continue a pinned old policy. A raw immutable `Arc` or borrowed plaintext slice already handed to trusted code cannot be retroactively revoked by changing an epoch. Keep such borrows within the trusted lifetime boundary, bound leases, and disclose the residual exposure. Data already exported to a user or third party is outside immediate recall.

### 5.7 Commit, cancellation, retry, and recovery

A mutation has an operation ID and a linearized terminal decision. Pre-commit cancellation produces `CancelledBeforeCommit` and no canonical mutation for that operation. If commit wins, return `Committed { receipt }` even if the caller disconnects. If the response is lost, return or retain `OutcomeUnknown { operation_id }` until status/replay resolves it; never repeat the operation under a new ID. Reusing an operation ID with different input is a conflict, not idempotent replay.

A logically cancelled specimen cycle has no partial canonical edits. Implement this by staging changes and fencing the commit decision. Once a commit wins, the cycle is a committed cycle with a cancelled/lost response, not a cancelled cycle. Durable job admission and audit receipts are separately named operations; specify whether they remain even when the compute phase is cancelled.

Cancelling an async wrapper does not prove its blocking thread or GPU/NPU kernel stopped. Started blocking tasks require cooperative cancellation or process isolation, and queued/running device work retains its reservations until completion or verified teardown. This follows the operational distinction documented by Tokio; the plan's resource rules apply regardless of runtime choice. [EXT03]

### 5.8 Hardware capability and qualification

Retain `CapabilityEvidence` facts: **compiled, available, initialized, executed, runtime_verified**. They are source-defined compatibility facts. Do not replace them with the three inconsistent ladders in revision 4.0. Add a `QualificationRecord`, keyed by backend implementation, device, driver/runtime, operation, dtype/shape range, model/kernel digest, build fingerprint, numerical profile, and dataset/corpus digest.

The record independently reports reference comparison, error bounds, rank/threshold behavior, execution placement evidence, transfer/setup-inclusive performance, security review, supported failure behavior, timestamp, expiration/invalidation conditions, and approved use. Driver, model, compiler, or kernel changes invalidate the applicable scope of qualification. A backend may be available but not qualified for the requested operation.

CPU labels such as `cpu-avx2` remain for compatibility. The current safe lane implementation does not itself prove an AVX2 instruction was emitted. Distinguish `requested_backend`, `effective_backend`, `implementation_id`, `lane_width`, `eligible_isa`, `observed_isa_evidence`, `device_execution_evidence`, and `qualification_id`. CPU remains mandatory for core reference database operations, not a guarantee that every optional large model can fit on every host.

GPU adapters cover selected batch/geometry/model operations. NPU adapters cover compatible neural graphs. Core ML's CPU-and-Neural-Engine policy permits both; it does not prove that every operation ran on ANE. Windows ML provider identity and actual hardware determine device class; DirectML is a GPU path, not generic NPU support. Other vendor NPU and optional TPU integrations must name their model/provider/runtime and privacy contract. [EXT04–EXT05]

### 5.9 Dispatch, fallback, and remote trust

`BackendPolicy` is one of `RequireExactBackend`, `PreferLocal`, or `AllowAuthorizedRemote`, with an explicit permitted backend set. Construct a bounded acyclic candidate list after filtering for authorization, data residency, qualification, operation/model compatibility, memory, and deadline. Select by measured end-to-end cost; do not hardcode a contradictory universal fallback order.

A failed accelerator can fall back to CPU only if the CPU path fits the remaining resource budget and implements the same output contract. A different model, changed embedding space, reduced precision beyond the requested profile, or remote upload requires a separately permitted choice. `RequireExactBackend` returns a typed failure instead of silently succeeding on CPU. Only pure/idempotent work can be retried; canonical commit ownership never migrates to a device worker.

A remote result signature establishes which key signed the result bytes, not that computation was correct or that the worker erased plaintext. In the initial trusted-worker profile, numerical qualification and transport identity are explicit trust assumptions. Strict exact search still requires complete candidate coverage; spot checks cannot prove a remote worker did not omit a winning record. Confidential inputs never go to an untrusted worker merely because the channel is encrypted.

### 5.10 Cross-node semantics and hard budgets

Separate the causal data plane from a reviewed control plane for membership, placement, and any elected ownership. Immutable object identity and per-writer causality remain; no last-writer-wins replacement. A replicated-durable ACK certifies the configured number and fault-domain placement of durable copies, not global linearizability. Cross-shard snapshot mode requires a dependency-closed causal cut; otherwise label the result best-effort.

For the original specimen's strict **1,000 simultaneous high-resolution comparison** cap, the accounting invariant is:

```text
active_local + outstanding_remote_credits + uncertain_remote_credits <= configured_cap
```

Remote credits cover both queued and executing logical comparisons until accounted for. A lease expiring or a result being fenced does not stop the old worker. Do not reissue uncertain credits until the trusted protocol establishes cessation, a verified process/host teardown occurs, or an explicitly approved weaker profile is selected. A network partition can reduce availability to zero under the strict cap; that is preferable to claiming the cap while oversubscribing. GPU microbatches reserve one credit per actual high-resolution comparison, not one per kernel launch. These guarantees assume cooperating trusted workers, not a malicious host that ignores limits.

### 5.11 Projection, determinism, and specimen stochasticity

Projection data binds the authorized input cohort as well as the embedding snapshot. Fitting a global layout on forbidden records and then hiding those points can still move permitted coordinates; strict isolation therefore fits only the allowed cohort or uses a separately approved public model. Privacy changes invalidate cohort-derived centroids, transforms, and caches.

`ReplayProfile` freezes input order, algorithm/version, numeric policy, threading/reduction choices, tie handling, RNG algorithm, and model/kernel digests. Seed alone does not guarantee byte equality. PCA repeated eigenvalues require an explicit deterministic basis rule within a frozen reference implementation; portable qualification may compare subspaces, reconstruction, and aligned layouts within declared tolerances instead of demanding identical coordinates. Persisted generated coordinates retain their own original bytes and digest.

Specimen stochasticity uses recorded, purpose-separated draw identities. Independent feedback coin flips are keyed per contributor, not per retry or worker-arrival order. Jitter never mutates the baseline simply through repeated reads. The ordering/seed migration itself is versioned and must not alter the original behavioral distribution without approval. Time-bounded multiway/idle runs record actual termination and scheduling context; a fixed seed alone cannot make a wall-clock-limited run identical under different load.

### 5.12 Erasure, old readers, and publication limits

The original member-erasure proposal is still a separately gated design. This plan preserves its encrypted canonical fact custody, per-scope member keys, per-fact keys, holds, signed receipts, summary clearing, migration/shadow-read direction, and explicit approval boundary. It does not invent a fresh approval or rewrite its cryptographic choices.

Key destruction, logical invisibility, derivative invalidation, deletion from retained replicas, and physical media sanitization are different outcomes. Receipts report which happened and which remain pending. A stale backup capable of restoring a wrapped key can defeat a casual erasure claim; test key-epoch restoration and resurrection prevention. A content-free tombstone is not evidence that every uncontrolled copy disappeared.

Old-reader compatibility is tested per format and mutation namespace. Disabling an index is a safe rollback only when canonical data remains readable by the target binary. If new authoritative data is not understood by the old writer, retain read-only recovery or the compatible current writer; never reopen with a binary that might silently discard unknown records. Publication markers can make completed bundles visible without rehashing original signed records or pretending all underlying writes were atomic.

<a id="milestones"></a>
## 6. Milestones and dependency graph

| Milestone | Task packet | Independently useful result | Promotion boundary |
|---|---|---|---|
| M0 — source and qualification baseline | T00–T02 | Pinned evidence, supported build/test matrix, preserved compatibility corpus | No new product capability claim |
| M1 — checked point spatial path | T03–T06 | Validated legacy point queries, independent oracle, qualified octree path | `spatial-checked-v1` only |
| M2 — typed durable spatial data | T07–T16 | Frames/shapes, stable versions, pinned views, recovered generations, scope protection | `spatial-durable-v1` |
| M3 — hybrid/evidence | T17–T25 | Authorized vector/lexical/spatial/graph/time queries and explained evidence | Exactness/approximation stated per query |
| M4 — projections and explorer | T26–T29 | Reproducible-profile projection service and bounded read-only explorer | Runtime-specific client qualification |
| M5 — compute qualification | T30–T39 | CPU baseline, one or more independently qualified adapters and remote job contract | One operation/model/device profile at a time |
| M6 — cross-node causal service | T40–T45 | Authenticated replication, causal queries, rebalance, strict credits, real-host evidence | No global serializability or Byzantine claim |
| M7 — original specimen conformance | T46–T53 | Preserved original behavior and complete per-runtime traceability | Full claim only after all required rows pass |
| M8 — privacy/release operations | T54–T57 | Gated custody work, security evidence, benchmark manifest, rollback/restore | Explicit named production profile and approval |

Dependency spine: **M0 → M1 → M2 → M3 → M4**. Compute qualification can proceed after M0 and the relevant numeric/policy contracts; it never blocks M1. Cross-node work requires the exact local contracts, authorization, and remote-job boundaries, not every GPU vendor. Specimen conformance can proceed in parallel against the approved storage contract; distributed specimen execution additionally requires strict credit accounting. Erasure design review can proceed independently, but destructive implementation cannot.

Each task lists its exact predecessor IDs. The task graph must be acyclic. Approval or completion of one task does not automatically satisfy its dependencies' evidence gates. When hardware or a source repository is unavailable, produce a bounded blocker and leave the corresponding task unverified; continue unrelated approved planning work.

<a id="tasks"></a>
## 7. Task-by-task implementation packets

All steps below are future implementation work. **None are checked off by this planning revision.** Paths and signatures marked “proposed” are intentional new interfaces. Test names and assertions define the required behavior; they are not a report of tests already run. Each packet finishes with a reviewable diff and evidence, not an automatic commit.

**Command conventions:** commands run from the root named in each task. First use the test target's `-- --list` output to verify that the named cases exist; after execution, verify the expected nonzero count. A zero-test exit status is not a passing task. Use `--locked`; do not silently update dependencies. M0 establishes the supported feature matrix and the exact repository gate. Long-running stress, fuzz, external-provider, and device jobs require their named execution scope and resource budget.

<a id="t00"></a>
### T00 — Pin the source, authority, and evidence manifest

**State:** Proposed; not executed. **Repository root:** WDBX / ABI / STUDIO. **Dependencies:** None; read-only/source-inventory starting point.

**Files:**
- Create: `docs/superpowers/specs/2026-10-20-wdbx-3d-source-audit.md`
- Create: `docs/claims/wdbx-3d/source-manifest.json`
- Create: `tools/validate_wdbx_3d_evidence.py`
- Test: `tools/tests/test_wdbx_3d_evidence.py`

**Interfaces — consumes:** Original directive, original artifact digest, seven named source documents, full original specimen source, current repository metadata, and latest user instructions.

**Interfaces — produces:** `SourceManifest` with repository/ref/blob/range/authority/evidence-kind entries; `validate_source_manifest(document: dict) -> list[str]` returns an empty error list only for a complete well-typed inventory.

- [ ] **Step 1 — Define acceptance/negative fixtures.** Pin these named cases and exact assertions:
  - `rejects_mutable_only_source`: A source row with only a main-branch URL and no immutable identity is not implementation-ready.
  - `distinguishes_reported_and_executed`: A commit-message claim is recorded as reported, with execution evidence absent.
  - `rejects_missing_original_authority`: Missing any named directive document or original specimen authority blocks its dependent scope.

- [ ] **Step 2 — Establish the baseline or blocked gate.** Discover the actual cases and run only under the required execution authorization. Record acceptance failures or unverified status; do not manufacture a failing test when this packet is qualification/design rather than a behavior change.

```text
python3 -m unittest discover -s tools/tests -p test_wdbx_3d_evidence.py
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Read complete controlling files and preserve dirty work. Record the actual v2 module root and current STUDIO workspace location. Extract names of compatibility fixtures and supported commands without running production workloads. Pin all source links; do not copy credentials, private runtime records, or Git author emails into the public manifest.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** An engineer can identify what was read, what is inherited, what is reported, and which requirements remain unresolved. No fake complete-repository audit. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Document-only outputs can be revised; original files and repositories remain untouched until an approved implementation packet.

<a id="t01"></a>
### T01 — Qualify the already-landed stable toolchain

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T00.

**Files:**
- Read: `rust-toolchain.toml`, `Cargo.toml`, `Cargo.lock`, `tools/check.sh`
- Read: `crates/abi-compute/src/cpu.rs` and actual consumer toolchain files
- Create: `docs/claims/wdbx-3d/stable-qualification.json`
- Test: supported workspace/consumer targets discovered by T00

**Interfaces — consumes:** SourceManifest; source revision 08a69b3f... or an explicitly superseding pinned revision; existing lint and feature policies.

**Interfaces — produces:** `StableQualification` records `rustc -Vv`, Cargo version, target, feature tuple, lockfile digest, command, exit status, nonzero test counts, and untested consumers.

- [ ] **Step 1 — Define acceptance/negative fixtures.** Pin these named cases and exact assertions:
  - `compiler_is_requested_release`: The actual compiler used by every qualified command reports 1.99.1.
  - `check_is_not_test_evidence`: Compile success cannot populate a test or performance field.
  - `feature_matrix_is_supported`: Mutually incompatible or device-only features are not forced into one blanket all-features job.

- [ ] **Step 2 — Establish the baseline or blocked gate.** Discover the actual cases and run only under the required execution authorization. Record acceptance failures or unverified status; do not manufacture a failing test when this packet is qualification/design rather than a behavior change.

```text
cargo +1.99.1 check --workspace --all-targets --locked; cargo +1.99.1 fmt --all --check; cargo +1.99.1 clippy --workspace --all-targets --locked; cargo +1.99.1 test --workspace --locked
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Begin with the current safe lane implementation, not a repeat migration. Under authorized execution, run the existing gate with RUSTUP_TOOLCHAIN=1.99.1 and verify the script does not override it. Keep default warning policy. Add separate supported feature tuples only after inspecting manifests. Record the earlier test pause and the current execution authorization separately.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** All required profile commands pass on the pinned checkout; a missing runner or paused job is unverified, not a pass. No SIMD ISA or speedup claim. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** No dependency/toolchain change unless a measured blocker requires a new reviewed task. Do not unpin another consumer to make this gate pass.

<a id="t02"></a>
### T02 — Freeze compatibility behavior before new spatial contracts

**State:** Proposed; not executed. **Repository root:** WDBX / ABI. **Dependencies:** T00, T01.

**Files:**
- Create: `crates/abi-wdbx/tests/wdbx_3d_compatibility.rs`
- Create: `crates/abi-wdbx/tests/fixtures/wdbx_3d/compatibility-manifest.json`
- Read: existing migration, v3, multiway, CLI/MCP/gateway fixture paths pinned by T00

**Interfaces — consumes:** Existing golden bytes, public methods, actual RecordId/version types, native/browser format inventory.

**Interfaces — produces:** Compatibility corpus with original input/output digests and `CompatibilityReport { unchanged, changed, blocked, reason }`.

- [ ] **Step 1 — Define acceptance/negative fixtures.** Pin these named cases and exact assertions:
  - `signed_bytes_remain_identical`: Read/attachment-index operations leave v2/v3 signed original bytes and digests unchanged.
  - `legacy_reader_is_nonmutating`: Opening legacy fixtures read-only changes neither file set nor bytes.
  - `legacy_api_order_is_characterized`: Record existing equal-distance and duplicate-ID behavior before adding a new ordering contract.
  - `zero_tests_do_not_qualify`: A filter matching no tests produces a failed evidence check even if Cargo exits zero.

- [ ] **Step 2 — Establish the baseline or blocked gate.** Discover the actual cases and run only under the required execution authorization. Record acceptance failures or unverified status; do not manufacture a failing test when this packet is qualification/design rather than a behavior change.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_compatibility
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use copies of repository fixtures, never operator stores. Capture actual frozen surfaces rather than maintaining a second hand-written list. The characterization corpus records legacy behavior; it does not require new checked methods to copy known numeric weaknesses.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Compatibility fixture inventory is immutable and reviewable; future tasks can demonstrate exactly which old behaviors they preserve. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Delete only generated scratch fixtures if needed; do not regenerate old goldens to hide a behavior change.

<a id="t03"></a>
### T03 — Define finite geometry and legacy widening

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T02.

**Files:**
- Create: `crates/abi-wdbx/src/spatial3d/mod.rs`
- Create: `crates/abi-wdbx/src/spatial3d/numeric.rs`
- Modify: `crates/abi-wdbx/src/lib.rs` with additive export only
- Test: `crates/abi-wdbx/tests/wdbx_3d_numeric.rs`

**Interfaces — consumes:** ADR02; existing Point3D f32 type; section 5.2 numeric contract.

**Interfaces — produces:** `Point3::try_new(x: f64, y: f64, z: f64) -> Result<Point3, SpatialError>`; `Point3::from_legacy(p: Point3D) -> Result<Point3, SpatialError>`; `Radius::try_new(r: f64) -> Result<Radius, SpatialError>`. SpatialError defines NonFinite, CoordinateOutOfRange, NegativeRadius, InvalidBounds, BudgetExceeded, and UnsupportedMetric.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `rejects_nonfinite_and_out_of_domain`: Every NaN/infinity coordinate is rejected; abs(x)>1e150 is rejected; no clamp.
  - `widens_all_finite_f32`: Finite f32 endpoints/subnormals widen without narrowing; original f32 bits remain unchanged.
  - `normalizes_only_new_geometric_zero`: Signed zero is equal geometrically; serialization of old records remains unchanged.
  - `radius_domain_is_closed`: -1 is rejected; -0.0, 0.0 and exactly 1e150 are accepted as nonnegative bounded radii.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_numeric
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Keep validated fields private and expose read-only accessors. Use standard stable arithmetic. Do not attach frames or persist geometry in this task. Keep original Point3D and legacy methods untouched.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** All constructors enforce one documented numeric domain; no new unsafe code and no incompatible public type change. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Remove the additive module/export before any consumer is enabled; legacy storage is unaffected.

<a id="t04"></a>
### T04 — Build independent complete and boundary oracles

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T03.

**Files:**
- Create: `crates/abi-wdbx/tests/support/spatial_oracle.rs`
- Create: `crates/abi-wdbx/tests/support/dyadic_reference.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_oracle.rs`

**Interfaces — consumes:** Validated numeric domain and old point fixtures; no production octree traversal or distance helper reused in the oracle.

**Interfaces — produces:** `reference_radius(points: &[OraclePoint], center: Point3, radius: Radius) -> Vec<OracleHit>`; `reference_knn(points: &[OraclePoint], center: Point3, k: usize) -> Vec<OracleHit>`; exact dyadic comparison helpers for the bounded test corpus. OraclePoint includes a stable test identity and insertion ordinal.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `boundary_neighbors_are_distinct`: Values on the sphere boundary, immediately below it, and immediately above it classify according to the specified profile.
  - `extreme_coordinates_have_finite_reported_distance`: Opposite finite f32 extrema do not overflow after widening.
  - `oracle_does_not_call_production_kernel`: A deliberately defective production distance helper does not alter reference answers.
  - `identity_breaks_true_metric_ties`: Equal metric keys sort by identity and then insertion ordinal, not input scheduler order.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_oracle
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Implement complete enumeration independently. Build exact comparisons from float bits using bounded integer arithmetic with reviewed dependencies already available where possible. Cross-check deterministic float and exact-dyadic semantics and mark expected disagreements rather than silently treating them as identical definitions.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Known-answer, degenerate, adversarial, and seeded cases have an independent reference; exact-real and defined-float results are distinguished. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Test-only helpers do not affect product execution or persistent data.

<a id="t05"></a>
### T05 — Add checked access to the existing point index

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T03, T04.

**Files:**
- Modify: `crates/abi-wdbx/src/spatial.rs` only for additive checked methods/internal access
- Create: `crates/abi-wdbx/src/spatial3d/legacy_checked.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_checked_points.rs`

**Interfaces — consumes:** Existing SpatialIndex3D records and payload borrowing; Point3, Radius, SpatialError, independent oracle.

**Interfaces — produces:** `SpatialIndex3D::try_insert(id: u32, point: Point3D, payload: &[u8]) -> Result<(), SpatialError>`; `try_radius_search_checked(&self, center: Point3D, radius: f32, limits: QueryLimits) -> Result<Vec<CheckedSpatialHit<'_>>, SpatialError>`; `try_nearest_neighbors_checked(&self, center: Point3D, k: usize, limits: QueryLimits) -> Result<Vec<CheckedSpatialHit<'_>>, SpatialError>`. CheckedSpatialHit has id, insertion_ordinal, point, distance_f64, and borrowed payload. QueryLimits has max_work, max_results, and max_output_bytes.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `invalid_insert_is_atomic`: Rejected insert leaves count, records and existing query output unchanged.
  - `zero_k_returns_no_hits`: k=0 yields an empty complete answer without traversing points.
  - `equal_distance_uses_checked_total_order`: Opposite equidistant points inserted in reverse ID order return ascending ID in the new API.
  - `legacy_duplicates_are_retained`: Repeated legacy IDs are returned as distinct observations, with ordinals.
  - `invalid_legacy_population_is_not_silently_skipped`: A checked query encountering a record inserted unsafely through an old method reports invalid data instead of claiming completeness.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_checked_points
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use the complete checked CPU path first. Widen before subtraction/squaring and enforce budgets before allocating an output buffer. Preserve old methods, including characterized behavior. Do not cast future UUID identities into u32 or copy payloads unnecessarily.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** New safe behavior is independently tested and old fixtures still pass; no canonical changes, gateway, or accelerator dependency. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Consumers switch back to old methods only under their old documented semantics; no stored bytes need reversal.

<a id="t06"></a>
### T06 — Qualify and harden the existing octree fast path

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T05.

**Files:**
- Modify: `crates/abi-wdbx/src/spatial.rs` internal octree code with compatibility tests
- Create: `crates/abi-wdbx/src/spatial3d/octree_checked.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_octree_parity.rs`

**Interfaces — consumes:** Existing octree topology/thresholds; checked complete path; independent oracles; ADR03.

**Interfaces — produces:** `checked_octree_query(index: &mut SpatialIndex3D, query: &CheckedPointQuery) -> Result<CheckedPointOutcome<'_>, SpatialError>`; CheckedPointQuery names Radius/Knn/metric/limits; outcome reports numeric profile, path used, and coverage.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `threshold_63_64_65_has_same_semantics`: Queries at the lazy-build boundary agree with the complete oracle.
  - `degenerate_points_do_not_explode_tree`: Coincident, collinear and planar inputs stay within node/allocation limits.
  - `conservative_bounds_do_not_drop_boundary_hits`: A point whose distance is on the requested boundary survives pruning.
  - `insert_after_build_invalidates_correctly`: A new point appears on the next checked query and stale nodes do not survive.
  - `unsupported_cosine_pruning_falls_back`: Cosine remains complete-scan without an unproved Euclidean bound.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_octree_parity
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Reuse current partitioning and topology rather than replacing the store. Use overflow-safe bound calculations and a conservative prune; retain complete fallback when a bound cannot be certified. Share one checked ordering rule across insertion/top-k/traversal. Add an early leaf stop when subdivision cannot separate the population.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** M1 exit: existing fixture suite plus at least 1,000 seeded bounded cases agree under the declared numeric profile; no speedup is required or presumed. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable the new indexed path and retain the checked complete scan. Keep prior index behavior available for existing callers.

<a id="t07"></a>
### T07 — Specify closed AABB and sphere predicates

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T03, T04, T06.

**Files:**
- Create: `crates/abi-wdbx/src/spatial3d/geometry.rs`
- Create: `crates/abi-wdbx/src/spatial3d/predicates.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_geometry.rs`

**Interfaces — consumes:** Point3, Radius, SpatialError and exact-dyadic reference.

**Interfaces — produces:** `Aabb3::try_new(min: Point3, max: Point3) -> Result<Aabb3, SpatialError>`; `Sphere3::new(center: Point3, radius: Radius) -> Sphere3`; `classify_relation(left: &Geometry3, right: &Geometry3, relation: SpatialRelation) -> Result<bool, SpatialError>`. Geometry3 initially contains Point/Aabb/Sphere; relation contains Contains/Intersects.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `closed_box_includes_faces_edges_vertices`: All closed-boundary points are included; reversed bounds are rejected.
  - `zero_volume_shapes_are_valid`: Point-sized boxes and zero spheres have defined membership.
  - `sphere_tangent_is_intersection`: Tangent spheres intersect; immediately separated dyadic fixtures do not.
  - `unsupported_shape_is_explicit`: Mesh/ray/geodetic input does not silently degrade to a point.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_geometry
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use interval-certified fast predicates with exact fallback, or the exact comparator until the fast bound is proved. Preserve the catalogue of future geometry primitives without claiming their kernels are implemented in this task.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Supported relations match the exact finite-input reference; every unsupported relation is a typed error. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** No persistent encoding has changed; remove new callers or use the point-only profile.

<a id="t08"></a>
### T08 — Preserve RecordId and every maximal causal version

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T02, T07.

**Files:**
- Create: `crates/abi-wdbx/src/spatial3d/identity.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_identity_conflicts.rs`
- Read: actual `src/v2/types.rs` version definitions

**Interfaces — consumes:** Source-defined RecordId and version identity; Geometry3.

**Interfaces — produces:** `SpatialObservationKey { record_id, version_id, attachment_id }`; `DenseMapping::build(keys: &[SpatialObservationKey]) -> Result<DenseMapping, SpatialError>`; `ConflictMode::{PreferredView,AllMaximalVersions,RejectConflicted}`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `large_legacy_and_uuid_ids_roundtrip`: u64::MAX and two UUID IDs retain exact identity; no truncating cast.
  - `mapping_is_bijective_per_generation`: Every dense slot resolves to exactly one observation and vice versa.
  - `concurrent_locations_remain_visible`: AllMaximalVersions returns both concurrent positions; PreferredView reports conflicts; RejectConflicted fails.
  - `counting_unit_is_explicit`: Observation count differs from logical-record count in conflict fixtures.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_identity_conflicts
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use existing version ordering and causal semantics. Dense mappings are rebuildable generation-local artifacts. Do not equate deterministic preferred presentation with conflict resolution or choose LWW timestamps.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** No source identity or maximal version disappears; mappings and sort behavior are deterministic. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Discard mappings and rebuild; canonical identity/version objects are never rewritten.

<a id="t09"></a>
### T09 — Add an explicit frame registry and legacy-unbound view

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T07, T08.

**Files:**
- Create: `crates/abi-wdbx/src/spatial3d/frames.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_frames.rs`

**Interfaces — consumes:** Geometry3 and observation identities; ADR05.

**Interfaces — produces:** `FrameDefinition { id, version, units, handedness, axis_labels, parent, transform_digest }`; `FrameRegistry::resolve(id: &CoordinateFrameId) -> Result<&FrameDefinition, SpatialError>`; `LegacyFrameBinding { source_set_digest, frame_id, approval_ref }`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `unbound_legacy_is_not_meters`: Legacy records with no frame remain unitless; physical-frame queries reject them without an approved binding.
  - `mixed_frame_comparison_is_rejected`: Numerically equal points in unrelated frames cannot be directly compared.
  - `transform_version_changes_invalidate_index`: Registry digest changes make dependent generations unusable.
  - `cyclic_or_singular_frame_is_rejected`: Parent cycles and invalid rigid transforms cannot be registered.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_frames
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Initial physical transforms are explicitly registered rigid transforms with documented validation tolerances and uncertainty. No automatic geodetic conversion or guessed world origin. Frame metadata can be attached without mutating legacy values.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Every physical query has a compatible frame/units contract; legacy inspection still works. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Remove only the additive binding/view; original coordinate bytes retain their historical meaning.

<a id="t10"></a>
### T10 — Persist additive spatial attachments with reader gates

**State:** Proposed; not executed. **Repository root:** WDBX / ABI. **Dependencies:** T08, T09.

**Files:**
- Create: `crates/abi-wdbx/src/spatial3d/attachment.rs`
- Create: `crates/abi-wdbx/src/spatial3d/attachment_codec.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_attachments.rs`
- Modify: only the canonical admission seam selected in ADR01

**Interfaces — consumes:** SpatialObservationKey, Geometry3, FrameRegistry, existing VersionedStore transaction boundary, source digests.

**Interfaces — produces:** `SpatialAttachmentV1` fields defined in section 5.3; `encode_attachment_v1(value: &SpatialAttachmentV1) -> Result<Vec<u8>, SpatialError>`; `propose_spatial_attachment(store: &mut VersionedStore, request: AttachmentWrite) -> Result<AttachmentReceipt, SpatialError>`. AttachmentWrite includes expected source version, operation ID, authorization proof reference, and minimum reader/writer profile.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `attachment_does_not_rehash_source`: Original signed episode bytes and digest remain exactly unchanged.
  - `wrong_source_version_fails_without_partial_write`: An attachment cannot bind silently to the current preferred value after its named version changed.
  - `operation_replay_matches_original_receipt`: Same ID/same bytes is idempotent; same ID/different bytes fails.
  - `old_reader_policy_is_checked`: An incompatible reader/writer cannot be selected as rollback target.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_attachments
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use the reviewed additive namespace/codec and canonical store boundary. New floats have a defined byte encoding and zero policy; legacy codecs do not change. Validate maximum encoded length before allocation/signing. Do not assume transport JSON is the canonical commitment.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Round-trip golden vectors, signature/source preservation, replay, and minimum-reader checks pass. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable new writes; retain a compatible reader. Never erase new authoritative attachments as an automatic rollback.

<a id="t11"></a>
### T11 — Construct a snapshot-bound spatial read adapter

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T08, T10.

**Files:**
- Create: `crates/abi-wdbx/src/spatial3d/read_view.rs`
- Create: `crates/abi-wdbx/src/spatial3d/snapshot_query.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_read_view.rs`

**Interfaces — consumes:** Existing immutable V2Snapshot, source-defined causal heads/version identities, attachments, FrameRegistry, ConflictMode.

**Interfaces — produces:** `ReadView` from section 5.4; `SpatialReadView::bind(snapshot: Arc<V2Snapshot>, attachments: AttachmentView, binding: ReadBinding) -> Result<SpatialReadView, SpatialError>`; `search_snapshot(view: &SpatialReadView, query: &SpatialQuery) -> Result<QueryOutcome, SpatialError>`. SpatialQuery contains supported predicate, frame, conflict mode, limits, numeric profile, and completion policy.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `old_arc_keeps_old_coordinates`: A retained old view returns its original positions after later commits.
  - `v2_v3_pair_is_not_claimed_atomic`: An uncoordinated pair of heads is labeled observed_pair, not atomic_snapshot.
  - `frontier_object_mismatch_fails`: Equal writer/sequence with conflicting immutable object identity is rejected.
  - `all_versions_obey_one_view`: No mixture of old geometry and new vector/version metadata is returned.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_read_view
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Implement complete snapshot enumeration first. Bind every independent authority consulted. Use existing snapshot ownership/lifetimes; no unsafe lifetime extension, mutable global view, or truncating dense-ID mapping.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Snapshot correctness and conflict preservation hold without a persistent index or network service. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Drop the new view and query through existing readers; no source mutation is required.

<a id="t12"></a>
### T12 — Define content-bound derived index generations

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T11.

**Files:**
- Create: `crates/abi-wdbx/src/spatial3d/generation.rs`
- Create: `crates/abi-wdbx/src/spatial3d/manifest.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_manifest.rs`

**Interfaces — consumes:** SpatialReadView, DenseMapping, FrameRegistry, chosen index algorithm and numeric profile.

**Interfaces — produces:** `GenerationManifestV1` with source roots, algorithm/profile, cohort, mapping/chunk digests and counts; `build_generation(view: &SpatialReadView, policy: &IndexBuildPolicy) -> Result<StagedGeneration, SpatialError>`; `verify_generation(view: &SpatialReadView, staged: &StagedGeneration) -> Result<VerifiedGeneration, SpatialError>`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `same_semantic_inputs_have_same_identity`: Wall-clock build duration does not alter the semantic manifest digest.
  - `wrong_snapshot_or_frame_is_rejected`: Correctly checksummed files bound to another view are not accepted.
  - `missing_or_extra_observation_is_rejected`: Mapping counts/digests cover the complete authorized source set.
  - `unknown_critical_format_fails_closed`: Unsupported critical version flags cannot be ignored.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_manifest
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Keep generated files rebuildable and bind them to canonical source identities. Derive source-set digests in stable observation order. Authentication follows the selected threat profile; checksum presence alone is not source authenticity.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** A verified generation certifies the exact view/profile it covers and cannot be silently reused elsewhere. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Discard unactivated staged generations; source snapshots remain untouched.

<a id="t13"></a>
### T13 — Publish generations durably and recover after interruption

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T12.

**Files:**
- Create: `crates/abi-wdbx/src/spatial3d/publication.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_publication_recovery.rs`
- Reuse: actual atomic I/O and checkpoint helpers found by T00

**Interfaces — consumes:** VerifiedGeneration, source roots, current activation reference, filesystem durability contract.

**Interfaces — produces:** `publish_generation(verified: VerifiedGeneration, expected: Option<GenerationId>, destination: &Path) -> Result<PublicationReceipt, SpatialError>`; `recover_active_generation(path: &Path, view: &SpatialReadView) -> Result<RecoveryDecision, SpatialError>`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `kill_before_activation_preserves_old_generation`: Failure during chunk write/flush/verification leaves old active reference usable.
  - `activation_race_is_compare_and_publish`: A slower builder cannot replace a newer incompatible activation blindly.
  - `corrupt_active_generation_falls_back_safely`: Recovery uses a verified old generation or complete scan, not partial files.
  - `directory_sync_limits_are_reported`: Platform-specific durability limitations are visible in the receipt/profile.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_publication_recovery
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Stage files, synchronize them as required, verify, publish immutable content, then atomically advance a fenced activation reference. Use platform-specific tested helpers; do not promise POSIX rename semantics on every filesystem. Leave canonical commits independent of index availability.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Every injected interruption yields either the old verified view or the complete new one; never a half-published index. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Repoint only to a verified compatible generation after review; do not delete the last known-good or leased generation.

<a id="t14"></a>
### T14 — Prove stale-index delta coverage and safe fallback

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T11, T13.

**Files:**
- Create: `crates/abi-wdbx/src/spatial3d/coverage.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_delta_coverage.rs`

**Interfaces — consumes:** ReadView, GenerationManifestV1, canonical mutation/version information.

**Interfaces — produces:** `assess_coverage(index: &GenerationManifestV1, view: &ReadView) -> CoverageDecision`; `CoverageDecision::{Complete,Reconcile(DeltaPlan),Unusable(reason)}`; DeltaPlan includes inserts/replacements/tombstones/conflict/policy/frame changes.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `old_position_is_removed_when_new_position_arrives`: Delta reconciliation cannot return both the replaced old coordinate and the new value as one version.
  - `delete_and_conflict_changes_are_not_just_inserts`: Removed or newly concurrent values have exact snapshot semantics.
  - `incomplete_delta_forces_scan_or_error`: Missing a mutation interval never produces an exhaustive claim.
  - `newer_index_cannot_answer_older_view_by_accident`: Future records do not leak into an old pinned snapshot.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_delta_coverage
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Prefer rebuild/complete scan until complete delta proof is available. Reconcile by observation identity and causal version, not ID-only set union. Separate index freshness from query completeness in outcomes.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Indexed/reconciled results match the complete view oracle for mutation traces, including deletions and conflicts. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable delta optimization; exact snapshot scans remain the fallback.

<a id="t15"></a>
### T15 — Fence cancellation, pin lifetimes, and idempotent status

**State:** Proposed; not executed. **Repository root:** WDBX / ABI. **Dependencies:** T10, T11, T13.

**Files:**
- Create: `crates/abi-wdbx/src/spatial3d/operation.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_commit_cancel.rs`
- Modify: approved admission/status seam only

**Interfaces — consumes:** Operation IDs, existing canonical commit boundary, read-view leases, publication receipts.

**Interfaces — produces:** `OperationOutcome::{Committed(receipt),CancelledBeforeCommit,OutcomeUnknown(operation_id)}`; `request_cancel(id: OperationId) -> CancelDecision`; `operation_status(id: OperationId) -> Result<OperationOutcome, OperationError>`. CancelDecision states whether commit already won.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `cancel_before_commit_has_no_mutation`: Pre-commit cancellation leaves canonical bytes unchanged for that operation.
  - `commit_before_cancel_returns_receipt`: Post-commit cancellation cannot return an uncommitted outcome.
  - `lost_response_reuses_operation_identity`: Retry after response loss recovers the original receipt; changed payload is rejected.
  - `lease_drop_does_not_free_running_device_work`: Snapshot/device reservations are released only at their actual lifetime boundary.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_commit_cancel
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Implement one commit/cancel decision point and distinguish database commit from response delivery. Retain status evidence needed for retries under bounded retention. Audit events/job admissions have their own operation identity and documented retention.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Race and fault tests show at-most-one canonical effect per operation ID and honest unknown-state reconciliation. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable new asynchronous mutation APIs; never pretend a committed record can be cancelled away.

<a id="t16"></a>
### T16 — Create trusted scoped read authorization

**State:** Proposed; not executed. **Repository root:** WDBX / ABI. **Dependencies:** T00, T11, T15.

**Files:**
- Create: `crates/abi-wdbx/src/spatial3d/authorization.rs`
- Create: `crates/abi-wdbx/tests/wdbx_3d_authorization.rs`
- Create: `ABI/crates/abi-wdbx-gateway/src/spatial_v1/authorization.rs` after gateway inspection

**Interfaces — consumes:** Actual ABI identity/policy interfaces, ReadView, operation, resource scope, consent/policy epochs.

**Interfaces — produces:** `AuthorizedReadView` is non-client-constructible; `authorize_read(context: &VerifiedPrincipal, binding: ReadBinding) -> Result<AuthorizedReadView, AuthorizationError>`; `recheck_materialization(view: &AuthorizedReadView) -> Result<(), AuthorizationError>`. VerifiedPrincipal is created only by the gateway/policy trust seam.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `client_cannot_assert_tenant_permission`: A forged tenant field or boolean filter does not authorize reads.
  - `revocation_between_query_and_stream_is_enforced`: New materialization stops after policy epoch revocation.
  - `restricted_metadata_is_not_exposed`: No forbidden distances/counts/edge IDs appear in result or error shapes.
  - `strict_projection_cohort_excludes_forbidden_influence`: Restricted data cannot alter coordinates/centroids in the strict isolation profile.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_authorization
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Inspect existing capabilities and explicitly fill missing scoped authorization. Partition strict-mode indexes/cohorts, authenticate cursors, and constrain trusted raw borrows. Document shared-hardware timing limits rather than asserting universal noninterference.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Negative tests cover every disclosure boundary; bearer authentication is not mistaken for data-scope authorization. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable scoped new endpoints; retain existing documented access boundaries without silently widening them.

<a id="t17"></a>
### T17 — Define embedding spaces and bounded multi-vector aggregation

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T02, T16.

**Files:**
- Create: `crates/abi-wdbx/src/query3d/embedding_space.rs`
- Create: `crates/abi-wdbx/src/query3d/multivector.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_embedding_spaces.rs`

**Interfaces — consumes:** Existing vector records and dimension limit; AuthorizedReadView; model/preprocessing lineage.

**Interfaces — produces:** `EmbeddingSpaceV1 { model_digest, preprocessing_digest, dimensions, dtype, metric, normalization, quantizer }`; `validate_query_vector(space: &EmbeddingSpaceV1, values: &[f32]) -> Result<(), QueryError>`; `aggregate_record_hits(policy: AggregationPolicy, hits: &[ComponentHit]) -> Result<RecordScore, QueryError>`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `equal_dimension_different_model_is_rejected`: Two spaces with equal width but different model/preprocessor identity are not interchangeable.
  - `nonfinite_and_dimension_limits_fail_before_allocation`: NaN/Inf and widths above the source limit are rejected.
  - `maxsim_does_not_reward_duplicate_chunks`: Duplicating a component does not change a record score under MaxSimilarity.
  - `zero_vector_policy_is_metric_specific`: Preserve existing cosine compatibility while new space profiles state their own zero rules.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_embedding_spaces
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Preserve current 4096-dimension v2 behavior unless T00 finds a changed limit; raising it is a separate versioned decision. Name parent/component IDs and caps. Do not introduce arbitrary mixed-model averaging or claim re-embedding has occurred.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Every vector comparison and parent aggregation names a compatible space and stable policy. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable multi-vector/space-aware writes and retain original vectors and embedding lineage.

<a id="t18"></a>
### T18 — Implement the authorized exact hybrid reference

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T07, T11, T16, T17.

**Files:**
- Create: `crates/abi-wdbx/src/query3d/exact.rs`
- Create: `crates/abi-wdbx/src/query3d/outcome.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_hybrid_exact.rs`

**Interfaces — consumes:** AuthorizedReadView, SpatialQuery, EmbeddingSpaceV1, record aggregation, QueryOutcome contract.

**Interfaces — produces:** `execute_exact(view: &AuthorizedReadView, query: &HybridQueryV1) -> Result<QueryOutcome, QueryError>`; HybridQueryV1 includes typed hard predicates, ranking policy, k, numeric profile, work/output budget, and partial-result permission.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `all_eligible_records_are_examined`: A high-score record outside an arbitrary 8*k shortlist is still found in exact mode.
  - `filter_is_not_a_soft_score`: A spatial/authorization hard filter cannot be rescued by a high semantic score.
  - `missing_counterpart_is_explicit`: Spatial-only/vector-only records follow a declared missing-field policy.
  - `budget_exhaustion_is_not_empty_success`: Exact mode errors or explicitly reports permitted partial completion, not an exhaustive empty result.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_hybrid_exact
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use complete candidate enumeration and fixed globally meaningful score transforms. Preserve old hybrid_spatial_search behavior as compatibility; do not reuse its candidate-pool-relative normalization as the new exact contract.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Known-answer full scans yield deterministic results and clear evidence/coverage fields. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Keep the exact reference available even when later optimizations are disabled.

<a id="t19"></a>
### T19 — Add filtered ANN without hiding recall loss

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T18.

**Files:**
- Create: `crates/abi-wdbx/src/query3d/planner.rs`
- Create: `crates/abi-wdbx/src/query3d/filtered_ann.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_filtered_ann.rs`

**Interfaces — consumes:** Exact hybrid oracle, existing HNSW, AuthorizedReadView, space and numeric policies.

**Interfaces — produces:** `plan_query(view: &AuthorizedReadView, query: &HybridQueryV1) -> Result<QueryPlan, QueryError>`; `execute_plan(plan: &QueryPlan) -> Result<QueryOutcome, QueryError>`. QueryPlan records candidate routes, selectivity evidence, budgets, index bindings, and coverage claim.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `recall_is_measured_on_authorized_subset`: Oracle and ANN use the same scope, filters, aggregation and embedding model.
  - `reranking_does_not_upgrade_coverage`: Exact rescoring of an ANN candidate set remains bounded_approximate.
  - `high_selectivity_can_choose_exact_scan`: A small permitted subset is not forced into an inefficient broad ANN traversal.
  - `candidate_cap_is_exposed`: Exhausted candidates are reported according to completion semantics.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_filtered_ann
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Add measured planner decisions and a reference fallback. Do not claim exact top-k without a candidate-completeness proof. Keep strict isolation indexes separated from broader trusted traversal modes.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Recall/cost tradeoffs are explicit; a plan cannot falsely label a filtered shortlist exhaustive. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Force the exact reference path; leave HNSW and old query methods intact.

<a id="t20"></a>
### T20 — Add lexical search and globally comparable fusion

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T18, T19.

**Files:**
- Create: `crates/abi-wdbx/src/query3d/lexical.rs`
- Create: `crates/abi-wdbx/src/query3d/fusion.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_fusion.rs`

**Interfaces — consumes:** AuthorizedReadView, snapshot-bound tokenization/statistics, QueryPlan, existing vector/spatial components.

**Interfaces — produces:** `LexicalProfileV1 { tokenizer_digest, language, normalization, statistics_scope }`; `fuse_rankings(policy: &FusionPolicy, lists: &[ComponentRanking]) -> Result<FusedRanking, QueryError>`. FusionPolicy names fixed-score or RRF algorithm/parameters and coverage requirements.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `corpus_statistics_bind_to_view`: BM25 statistics cannot silently mix tenants or generations.
  - `local_rank_is_not_global_rank`: Merging local RRF scores is rejected as exact without a defined global algorithm.
  - `component_absence_has_defined_semantics`: Missing lexical or vector components do not become invented zero-confidence evidence.
  - `tie_order_is_stable`: Equal fused scores use public observation identity, not arrival order.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_fusion
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Create rebuildable postings and a versioned tokenizer. Scope corpus statistics. Use a proposed RRF constant of 60 only when that policy is selected and evaluated; record it as a design parameter, not a relevance guarantee. Keep fixed common score semantics for the simplest exact distributed merge.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Fusion policy is interpretable and comparable across its declared scope; no hidden global-rank assumption. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable lexical/fusion additions and retain the original or new exact vector/spatial policy.

<a id="t21"></a>
### T21 — Create a typed, bounded cognitive graph query

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T08, T11, T16.

**Files:**
- Create: `crates/abi-wdbx/src/query3d/graph.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_graph.rs`

**Interfaces — consumes:** Existing temporal/causal edges, v3 memory edges, AuthorizedReadView, observation identities.

**Interfaces — produces:** `GraphQueryV1 { roots, allowed_types, max_hops, max_visited, max_edges, deadline, completion_policy }`; `traverse_graph(view: &AuthorizedReadView, query: &GraphQueryV1) -> Result<GraphOutcome, QueryError>`. Edge types distinguish asserted/derived causation, association, provenance, contradiction, attachment and spatial containment.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `association_does_not_become_causation`: Vector or geometric proximity cannot produce a causes edge without a declared evidence-producing operation.
  - `cycle_and_high_degree_are_bounded`: Visits/edges/hops obey limits and partial traversal is explicit.
  - `edge_provenance_is_returned`: Each path step retains its actual edge identity/type and source record.
  - `unauthorized_intermediate_is_not_bridged`: Restricted nodes/edges cannot become invisible shortcuts between visible endpoints.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_graph
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Keep the audit DAG distinct from general cyclic association and specimen attachment graphs. Use iterative bounded traversal with deterministic neighbor ordering. Do not flatten multiway event multiplicity.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Paths are bounded, authorized and explainable without changing source graph semantics. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Discard derived adjacency and use original graph readers; no causal record mutation.

<a id="t22"></a>
### T22 — Separate valid time, transaction time, and staleness

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T11, T21.

**Files:**
- Create: `crates/abi-wdbx/src/query3d/temporal.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_temporal.rs`

**Interfaces — consumes:** Source timestamps/causal versions, ReadView, original temporal ranking.

**Interfaces — produces:** `TemporalFilterV1 { valid_interval, observed_before, as_of_frontier }`; `TemporalAssessment { valid_at_query, age, supersession, uncertainty }`; `assess_temporal(record: &TemporalRecordView, filter: &TemporalFilterV1) -> Result<TemporalAssessment, QueryError>`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `late_arrival_preserves_valid_time`: An event learned today may describe yesterday without changing transaction time.
  - `clock_skew_does_not_resolve_causality`: Wall-clock order does not discard concurrent causal values.
  - `unknown_time_is_not_zero_age`: Missing time metadata is unknown, not automatically freshest.
  - `staleness_policy_is_versioned`: A changed half-life or supersession rule changes policy identity, not old records.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_temporal
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Add temporal read-time assessment first; do not turn staleness into deletion. Preserve original multiplicative recency mode and make new evidence/lifecycle effects explicit.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Historical views and ranking distinguish event time, arrival, causality and uncertainty. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable new temporal policy while retaining original metadata and ranking.

<a id="t23"></a>
### T23 — Expose the eight evidence dimensions without inventing trust

**State:** Proposed; not executed. **Repository root:** WDBX / ABI. **Dependencies:** T02, T16, T21, T22.

**Files:**
- Create: `crates/abi-wdbx/src/query3d/evidence.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_evidence.rs`
- Read: actual v3 signing/memory-edge and gateway verification interfaces

**Interfaces — consumes:** Existing canonical episodes, detached signatures, source/edge metadata, AuthorizedReadView.

**Interfaces — produces:** `EvidenceViewV1 { cryptographic, source, calibration, outcome, compatibility, regime, lifecycle, confidence }`; `explain_evidence(view: &AuthorizedReadView, target: RecordEvidenceRef) -> Result<EvidenceViewV1, QueryError>`. Each dimension has Known/Unknown/NotApplicable and supporting references; cryptographic validity additionally distinguishes Invalid and Unverifiable.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `missing_signature_is_not_valid`: Unsigned/unverifiable data is distinct from verified and from cryptographically invalid.
  - `signed_contradiction_remains_inspectable`: Valid signatures do not erase contradiction/quarantine status.
  - `unknown_calibration_is_not_perfect`: Absent calibration/regime/outcome evidence is not given an invented score.
  - `encoding_witness_is_not_signature_verifier`: C2 encoding evidence cannot populate cross-language signature qualification.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_evidence
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Implement a read-only evidence projection over what is actually present. Keep gaps visible. Invalid cryptographic objects are rejected by the relevant secure profile; ordinary uncertainty is not fabricated as integrity failure.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Eight independently inspectable dimensions exist with source references, without claiming evidence-weighted ranking is already done. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable the new explanation endpoint; keep original episode/signature bytes and old ranker unchanged.

<a id="t24"></a>
### T24 — Evaluate a versioned evidence-aware ranking policy

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T18, T20, T23.

**Files:**
- Create: `crates/abi-wdbx/src/query3d/evidence_ranker.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_evidence_ranking.rs`
- Create: `docs/benchmarks/wdbx-3d/evidence-ranking-protocol.md`

**Interfaces — consumes:** Exact authorized baseline, component ranks, EvidenceViewV1, held-out query relevance/applicability labels.

**Interfaces — produces:** `EvidenceRankingPolicyV1` with explicit eligibility and ranking dimensions; `rank_with_evidence(candidates: &[CandidateWithEvidence], policy: &EvidenceRankingPolicyV1) -> Result<Vec<RankedCandidate>, QueryError>`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `integrity_is_not_a_continuous_multiplier`: Invalid signatures cannot be rescued by semantic similarity.
  - `uncertainty_is_not_silently_dropped`: Unknown evidence is handled by explicit policy and remains visible.
  - `compatibility_ranker_remains_available`: Old policy ID produces its characterized behavior.
  - `held_out_evaluation_prevents_self_confirmation`: Evaluation queries/labels are distinct from policy tuning data.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_evidence_ranking
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Begin with explicit eligibility plus separated relevance/evidence ordering, not an opaque learned score. Any later learned ranker requires a separate model/label/license/consent ADR. Compare equivalent embeddings, storage, filters, candidates and budgets.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Policy behavior and applicability/quality effects are measured; no automatic constitutional evidence promotion. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Revert the selected ranking policy ID, retaining evidence views and original records.

<a id="t25"></a>
### T25 — Publish multiway bundles without claiming false atomicity

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T02, T10, T15, T21.

**Files:**
- Read: `crates/abi-wdbx/src/multiway.rs` and actual persistence helpers
- Create: `crates/abi-wdbx/src/query3d/multiway_publication.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_multiway_publication.rs`

**Interfaces — consumes:** Original canonical states/transitions/events, resume cursor, export identity, existing separate durable writes, operation fence.

**Interfaces — produces:** `MultiwayPublicationV1 { operation_id, export_digest, required_object_digests, source_frontier, completion }`; `publish_multiway_bundle(bundle: &MultiwayBundle, op: OperationId) -> Result<PublicationReceipt, OperationError>`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `duplicate_rules_preserve_event_multiplicity`: Two A->B rules produce one target state, one transition and two distinct events.
  - `interrupted_bundle_is_not_visible_as_complete`: Failure after some underlying writes does not publish the completion marker.
  - `resume_matches_requested_deterministic_domain`: Resume preserves canonical IDs/export under the same effective deterministic limits.
  - `wall_clock_stop_is_not_universal_replay_claim`: A time-limited partial run records its actual termination rather than pretending seed alone fixes it.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_multiway_publication
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Reuse content-addressed writes and verify covering objects before an idempotent completion marker. Retain legacy partial-persistence error semantics and the frozen compatibility zig_version field. Do not overwrite old exports/digests or make physics/unbounded-simulation claims.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Old multiway behavior remains; new complete publication is honest despite non-atomic underlying writes. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable new bundle publication; retain all already-written canonical objects and receipts.

<a id="t26"></a>
### T26 — Implement a cohort-bound PCA projection reference

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T17, T23.

**Files:**
- Create: `crates/abi-wdbx/src/projection3d/pca.rs`
- Create: `crates/abi-wdbx/src/projection3d/profile.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_pca.rs`

**Interfaces — consumes:** AuthorizedReadView, homogeneous EmbeddingSpaceV1, ReplayProfile, source cohort and numeric budgets.

**Interfaces — produces:** `ProjectionRequestV1 { read_view, cohort_digest, algorithm, replay_profile, seed, limits }`; `project_pca(view: &AuthorizedReadView, request: &ProjectionRequestV1) -> Result<ProjectionGeneration, ProjectionError>`; ProjectionGeneration includes ordered coordinates, source/model/profile digests, variance and quality metrics.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `forbidden_points_cannot_shift_strict_layout`: Changing forbidden inputs leaves a strict-scope layout unchanged.
  - `degenerate_eigenspace_has_declared_behavior`: Repeated eigenvalues use the frozen profile basis rule or explicit equivalent-subspace qualification.
  - `empty_and_rank_deficient_inputs_are_defined`: Zero/one/two records and zero variance do not yield NaN or invented variance.
  - `replay_scope_is_not_cross_platform_bitwise`: Exact byte replay is claimed only for its qualified frozen execution profile.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_pca
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Implement bounded deterministic CPU reference first. Fix input ordering and reduction/basis policies. Bind the authorized cohort and transform parameters. Use portable equivalence metrics where bitwise identity is not justified.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Finite coordinates and reproducibility/quality claims match their stated scope; projection never becomes canonical semantic evidence. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Discard the derived generation; source embeddings and episodes remain authoritative.

<a id="t27"></a>
### T27 — Qualify optional nonlinear projection adapters

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T26.

**Files:**
- Create: `crates/abi-wdbx/src/projection3d/nonlinear.rs` only after dependency review
- Test: `crates/abi-wdbx/tests/wdbx_3d_projection_adapter.rs`
- Create: `docs/claims/wdbx-3d/projection-adapters.json`

**Interfaces — consumes:** ProjectionRequestV1 and generation contract, reviewed algorithm dependency/license, supported transform behavior.

**Interfaces — produces:** `ProjectionAdapter::project(request: &ProjectionRequestV1, input: &ProjectionInput) -> Result<ProjectionGeneration, ProjectionError>`; adapter metadata records algorithm/version, seed, neighbor parameters, transform support, determinism class and resource bound.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `unknown_algorithm_does_not_fallback_silently`: A UMAP request cannot return PCA without an explicitly accepted fallback.
  - `new_points_do_not_imply_supported_transform`: Nonparametric adapters disclose rebuild/transform limitations.
  - `seed_and_dependency_are_in_identity`: Parameter/dependency changes invalidate generation reuse.
  - `quality_is_not_visual_appeal`: Trustworthiness/neighborhood/stability metrics accompany representative visual assessment.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_projection_adapter
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Treat each optional adapter as its own qualified feature. Keep it outside default production dependencies unless approved. Benchmark against PCA under the same cohort and budgets.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Only approved, measured adapters are exposed; no claim that every nonlinear layout is reproducible across hardware. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable the adapter and retain its stored output provenance; PCA remains available.

<a id="t28"></a>
### T28 — Expose a versioned read-only spatial/projection service

**State:** Proposed; not executed. **Repository root:** ABI. **Dependencies:** T16, T18, T23, T26.

**Files:**
- Create: `crates/abi-wdbx-gateway/proto/spatial_v1.proto`
- Create: `crates/abi-wdbx-gateway/src/spatial_v1/mod.rs`
- Create: `crates/abi-wdbx-gateway/src/spatial_v1/cursor.rs`
- Test: `crates/abi-wdbx-gateway/tests/wdbx_3d_gateway.rs`

**Interfaces — consumes:** Actual gateway transport/auth contracts; AuthorizedReadView; QueryOutcome; EvidenceViewV1; ProjectionGeneration.

**Interfaces — produces:** Proposed `wdbx.spatial.v1.SpatialQueryService` methods QuerySpatial, QueryHybrid, GetProjection, ExplainEvidence; authenticated opaque cursor binds query/view/policy/generation/sort boundary. Proposed HTTP bridge uses POST `/api/spatial/v1/query` for query bodies, not vector-bearing GET URLs.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `old_rpc_surface_is_unchanged`: Existing method names/shapes and authentication defaults retain their goldens.
  - `cursor_cannot_change_scope_or_query`: Tampering or reuse under another principal/query fails.
  - `expired_view_is_not_silently_rebased`: Expired cursor returns a typed expiry, not a new snapshot.
  - `stream_rechecks_policy_and_bounds`: Revoked or slow consumers receive bounded safe termination; no forbidden deltas.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx-gateway --test wdbx_3d_gateway
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use additive transport schemas whose version is independent of episode format v3. Do not invent public paths on the existing legacy listener. Start read-only; administrative build jobs and mutation APIs require separate capability checks and packet approval.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Bounded authenticated read API and stable pagination are proven without enabling public non-loopback exposure. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable the new service/route while retaining old gateway contracts.

<a id="t29"></a>
### T29 — Build a bounded and accessible 3D explorer client

**State:** Proposed; not executed. **Repository root:** STUDIO. **Dependencies:** T28.

**Files:**
- Create: `src/wdbx3d/explorer/` client module with focused scene, inspector and transport files
- Create: `tests/wdbx3d/explorer.test.ts`
- Create: `tests/wdbx3d/explorer-accessibility.test.ts`
- Modify: only existing navigation/runtime adapter seams identified by T00

**Interfaces — consumes:** Versioned read service, explicit query/projection outcomes, current browser/native runtime profile.

**Interfaces — produces:** `ExplorerSession` binds one read view/projection generation; `loadScenePage(cursor?: string): Promise<ScenePage>`; `selectRecord(key: SpatialObservationKey): Promise<EvidenceView>`; ScenePage contains only permitted geometry/labels and bounded LOD metadata.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `scene_buffers_contain_only_permitted_records`: No hidden geometry, hover labels, centroids or exported metadata include forbidden records.
  - `generation_swap_is_coherent`: Old edges are not attached to new coordinates during a view switch.
  - `keyboard_and_text_view_cover_3d_actions`: Search/select/inspect works without drag gestures or color-only cues.
  - `resource_cap_degrades_explicitly`: LOD/pagination bound memory; unsupported rendering has a text/2D alternative.
  - `read_only_client_cannot_mutate_by_selection`: Picking a node grants no mutation capability.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
bun test tests/wdbx3d/explorer.test.ts tests/wdbx3d/explorer-accessibility.test.ts
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Integrate into the existing runtime rather than replacing it. Keep explicit physical/projection layers and evidence labels. Instrument performance locally without using fake accelerator status as decoration.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Runtime-specific UI evidence, accessibility and resource behavior pass; browser success is not native success. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable the additive explorer route; retain original Studio functionality and data.

<a id="t30"></a>
### T30 — Add independent operation-level qualification records

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T00, T01.

**Files:**
- Create: `crates/abi-compute/src/qualification.rs`
- Modify: `crates/abi-compute/src/lib.rs` with additive exports
- Test: `crates/abi-compute/tests/wdbx_3d_qualification.rs`

**Interfaces — consumes:** Existing Backend, CapabilityEvidence/CapabilityState and Accelerator; no enum renaming.

**Interfaces — produces:** `QualificationKey { implementation, operation, dtype_shape_range, model_or_kernel, build, device, driver_runtime, numeric_profile }`; `QualificationRecord` fields from section 5.8; `qualifies(record: &QualificationRecord, request: &ComputeRequest) -> QualificationDecision`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `execution_flag_is_not_independent_oracle_evidence`: An executed/runtime_verified legacy backend with no corpus result cannot satisfy a strict qualification request.
  - `driver_model_build_change_invalidates_scope`: Changing any bound dimension prevents stale evidence reuse.
  - `missing_hardware_is_unverified`: Skip/not-run status cannot be promoted to qualified.
  - `capability_compatibility_is_preserved`: Original backend labels and five facts remain unchanged.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-compute --test wdbx_3d_qualification
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Qualification is metadata separate from compatibility capability facts and from production approval. Include validity windows, negative evidence and revocation without pretending a single global state ladder represents every operation.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** A request can be accepted only by matching actual evidence; no new universal accelerated flag. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable new scheduler use of qualification records; do not relabel old capability facts.

<a id="t31"></a>
### T31 — Implement an independent CPU numeric qualification corpus

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T01, T04, T17, T30.

**Files:**
- Create: `crates/abi-compute/tests/support/reference_numeric.rs`
- Test: `crates/abi-compute/tests/wdbx_3d_cpu_reference.rs`
- Read: current `src/cpu.rs` lane reduction and public zero/dimension semantics

**Interfaces — consumes:** Current CPU lane implementation, embedding/numeric profiles, qualification schema.

**Interfaces — produces:** `ReferenceNumericReport` for dot/norm/cosine/top-k across operation/dimension/value domains; independent f64/compensated and selected exact-dyadic references where relevant.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `cancellation_sensitive_vectors_exercise_reduction_order`: Large cancelling terms and ragged tails distinguish lane orders from a naive scalar label.
  - `backend_name_does_not_prove_isa`: A CPU label alone cannot populate observed instruction evidence.
  - `nonfinite_and_overflow_are_exposed`: Inputs outside the qualified domain fail a checked boundary rather than produce trusted NaN scores.
  - `threshold_crossing_is_authoritative`: Near-tie/near-gate results invoke the reference decision policy; tolerance cannot silently flip a specimen gate.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-compute --test wdbx_3d_cpu_reference
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Preserve current public behavior while defining a checked qualification boundary. Compare old and new source revisions under the same actual environment when available. Separate bitwise compatibility, bounded numeric error, rank agreement and speed.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Each supported numeric profile has reference evidence and visible limitations; no manufactured performance conclusion. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Qualification artifacts can be invalidated without changing canonical records or public compute labels.

<a id="t32"></a>
### T32 — Optimize stable CPU kernels only after qualification

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T31.

**Files:**
- Modify: `crates/abi-compute/src/cpu.rs` only after profiling identifies a bottleneck
- Create: `crates/abi-compute/benches/wdbx_3d_cpu.rs`
- Test: `crates/abi-compute/tests/wdbx_3d_cpu_optimized.rs`

**Interfaces — consumes:** Independent reference corpus, current safe lane implementation, source lint policy, supported targets.

**Interfaces — produces:** Existing public CPU methods remain; proposed `KernelImplementationInfo` reports lane width, safe implementation digest, dispatch assumptions and optional emitted-ISA evidence.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `optimized_path_preserves_requested_numeric_profile`: Candidate optimization satisfies reference/rank/gate requirements on qualified shapes.
  - `unsupported_isa_never_executes`: CPU detection/build dispatch cannot execute instructions on an ineligible host.
  - `scalar_reference_remains_available`: Optimized failure or unqualified target can use a fitting reference profile.
  - `unsafe_policy_is_not_weakened`: Intrinsics/FFI require an isolated approved exception, not a broad allow lint.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-compute --test wdbx_3d_cpu_optimized
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Prefer safe auto-vectorizable loops and measured layout changes. Inspect generated code only as ISA evidence; benchmark the whole operation. Do not optimize solely to make the compatible backend label sound true.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Any enabled optimization has correctness and transfer-free CPU performance evidence on its declared target. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Revert kernel selection to the prior qualified safe implementation; no store migration.

<a id="t33"></a>
### T33 — Build bounded dispatch and policy-valid fallback

**State:** Proposed; not executed. **Repository root:** WDBX / ABI. **Dependencies:** T15, T16, T30, T31.

**Files:**
- Create: `crates/abi-compute/src/dispatch_policy.rs`
- Create: `crates/abi-compute/tests/wdbx_3d_dispatch.rs`
- Modify: actual ABI scheduler seam without creating a crate cycle

**Interfaces — consumes:** QualificationRecord, requested numeric/model contract, BackendPolicy, hierarchical resource budgets, cancellation fence.

**Interfaces — produces:** `plan_dispatch(request: &ComputeRequest, context: &ComputeAuthorization, budget: &ComputeBudget) -> Result<DispatchPlan, DispatchError>`; `DispatchPlan` has an acyclic bounded attempt list; `ComputeOutcome` reports requested/effective backend, qualification, fallback reasons, resource accounting and output contract.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `required_backend_never_silently_falls_back`: RequireExactBackend fails explicitly when unavailable.
  - `fallback_cannot_change_model_or_privacy`: No hidden remote upload, embedding-space change or weakened precision.
  - `running_work_holds_reservations`: Timeout of the wrapper does not release live kernel/blocking-work memory/credits.
  - `retry_list_is_bounded_and_acyclic`: Repeated failures cannot create a CPU-GPU-remote loop.
  - `fair_admission_prevents_tenant_starvation`: Background work yields under bounded priority/fairness policy.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-compute --test wdbx_3d_dispatch
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Qualify then cost eligible backends; account for setup, transfers, queue, verification and remaining deadline. Pure jobs may retry with the same operation identity. Separate core CPU reference availability from optional models too large for local RAM.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Fallback is truthful, bounded, policy-compliant and resource-accounted even when cancellation cannot preempt execution. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Force the known qualified local backend and disable optional dispatch paths.

<a id="t34"></a>
### T34 — Qualify Metal operations on real supported hardware

**State:** Proposed; not executed. **Repository root:** ABI. **Dependencies:** T30, T31, T33.

**Files:**
- Read/modify: existing `crates/abi-gpu/` Metal adapter and helper paths pinned by T00
- Create: `crates/abi-gpu/tests/wdbx_3d_metal.rs`
- Create: `docs/claims/wdbx-3d/metal-matrix.json`

**Interfaces — consumes:** Existing Accelerator, dispatch/qualification contracts, CPU oracle and actual Metal runtime.

**Interfaces — produces:** Existing adapter gains only approved operation implementations and QualificationRecords; output shape/buffer/ownership contract is unchanged or versioned additively.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `real_device_matches_reference`: Dot/cosine/norm/batch or selected new operations match their qualified domain.
  - `late_completion_cannot_reuse_freed_buffer`: Cancellation leaves buffers alive until command completion or verified teardown.
  - `oom_reset_and_bad_output_fail_safely`: Device errors or invalid outputs do not reach canonical commits.
  - `timings_include_setup_and_transfer`: Performance evidence includes end-to-end cold/warm costs against optimized CPU.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-gpu --test wdbx_3d_metal
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use supported macOS/Metal devices and actual driver/runtime evidence. Apple Silicon/Intel Mac are separate rows. Do not infer speedup from execution, or NPU placement from GPU activity.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Only per-device/per-operation qualified Metal work may be advertised or selected under strict policy. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable that adapter/profile; retain CPU and old compatible hardware interfaces.

<a id="t35"></a>
### T35 — Qualify a CUDA adapter as a separate optional profile

**State:** Proposed; not executed. **Repository root:** ABI. **Dependencies:** T30, T31, T33.

**Files:**
- Create/extend: reviewed CUDA adapter module under `crates/abi-gpu/`
- Create: `crates/abi-gpu/tests/wdbx_3d_cuda.rs`
- Create: `docs/claims/wdbx-3d/cuda-matrix.json`

**Interfaces — consumes:** Approved CUDA dependency/toolchain/license scope, actual NVIDIA device, shared qualification/dispatch interfaces.

**Interfaces — produces:** CUDA operation adapter implements the existing Accelerator seam plus QualificationRecord metadata; exact supported kernels, dimensions and dtypes are enumerated.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `cuda_device_and_kernel_are_identified`: Actual runtime, device, binary digest and operation appear in evidence.
  - `unsupported_shape_is_rejected`: A compiled adapter does not accept an unqualified dtype/shape.
  - `candidate_coverage_is_not_inferred_from_rerank`: GPU candidate reduction must meet the requested coverage profile.
  - `device_loss_and_exhaustion_retain_safe_ownership`: Failed contexts cannot return stale buffers or double-release budgets.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-gpu --test wdbx_3d_cuda
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Start with one dense batch operation after CPU parity. No CUDA dependency is added to default core builds. Require actual-device evidence rather than compile-only stubs.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Named CUDA profile passes correctness and failure tests; absent device is unverified. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable the optional profile without altering canonical data.

<a id="t36"></a>
### T36 — Qualify a Vulkan compute adapter independently

**State:** Proposed; not executed. **Repository root:** ABI. **Dependencies:** T30, T31, T33.

**Files:**
- Create/extend: reviewed Vulkan compute adapter module under `crates/abi-gpu/`
- Create: `crates/abi-gpu/tests/wdbx_3d_vulkan.rs`
- Create: `docs/claims/wdbx-3d/vulkan-matrix.json`

**Interfaces — consumes:** Approved Vulkan loader/shader dependencies, explicit device features/limits, shared compute contracts.

**Interfaces — produces:** Vulkan adapter exposes supported operation/feature/precision combinations and shader/runtime/device-bound qualification.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `device_limits_bound_submission`: Workgroup, buffer and descriptor limits are validated before allocation/dispatch.
  - `barriers_and_visibility_match_reference`: Host/device synchronization cannot expose stale results.
  - `precision_is_not_inferred_from_vendor`: Actual dtype/feature support and numeric behavior are tested per device.
  - `timeout_retains_live_resources`: Wrapper cancellation cannot free in-flight resources.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-gpu --test wdbx_3d_vulkan
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Qualify one bounded compute operation, including shader digest and synchronization policy. Do not treat a Vulkan-capable renderer as a qualified database compute adapter.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Explicit real-device matrix passes; no automatic parity with Metal or CUDA. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable the adapter and keep the qualified local reference.

<a id="t37"></a>
### T37 — Qualify Core ML inference without overstating ANE placement

**State:** Proposed; not executed. **Repository root:** ABI. **Dependencies:** T17, T30, T31, T33.

**Files:**
- Read/extend: existing Core ML helper and adapter paths discovered by T00
- Create: `crates/abi-gpu/tests/wdbx_3d_coreml.rs`
- Create: `docs/claims/wdbx-3d/coreml-model-matrix.json`

**Interfaces — consumes:** Model manifest, input/preprocessing identity, exact tensor constraints, compute-unit request, actual Apple runtime.

**Interfaces — produces:** `NeuralInferenceRequest` binds model/dtype/shape/preprocessing/output-space; `NeuralInferenceEvidence` records requested units, actual available placement evidence, output checks, graph partitions when observable, cold/warm timing and fallback.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `requested_ane_is_not_residency_proof`: cpuAndNeuralEngine alone cannot populate verified ANE placement.
  - `model_outputs_match_reference_contract`: Selected embeddings/inference outputs satisfy declared tolerance and downstream space semantics.
  - `partial_graph_fallback_is_disclosed`: CPU/GPU/NPU partitions are not all labeled NPU from a model-level request.
  - `unsupported_model_has_no_hidden_download`: Compilation/shape failure cannot silently fetch another model or remote service.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-gpu --test wdbx_3d_coreml
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use a bounded known model and actual supported profiling where available. Declare unknown placement when instrumentation cannot establish it. Preserve non-learning Transient Memory and separate approved training from inference.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Output correctness and placement evidence are separately accurate; no arbitrary-model support claim. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable the model/backend pair or use a qualified same-contract CPU path within budget.

<a id="t38"></a>
### T38 — Define vendor neural-provider and optional TPU conformance

**State:** Proposed; not executed. **Repository root:** ABI. **Dependencies:** T17, T30, T33, T37.

**Files:**
- Create: `crates/abi-gpu/src/neural_provider_contract.rs`
- Create: `crates/abi-gpu/tests/wdbx_3d_neural_provider.rs`
- Create: `docs/claims/wdbx-3d/neural-provider-matrix.json`

**Interfaces — consumes:** NeuralInferenceRequest/Evidence, provider model/license/runtime constraints, strict privacy policy.

**Interfaces — produces:** `NeuralProviderDescriptor { provider, actual_device_class, runtime, model_profile, shapes, dtypes, locality, download_policy }`; `validate_provider_binding(request, descriptor) -> Result<ProviderBinding, ProviderError>`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `directml_is_not_generic_npu`: DirectML descriptors classify their GPU execution correctly.
  - `vendor_npu_is_model_specific`: QNN/OpenVINO/other providers cannot inherit ANE qualification.
  - `remote_tpu_is_explicitly_remote`: A TPU enum label does not grant a service connection, model availability, or data permission.
  - `provider_download_requires_authorization`: Dynamic provider/model acquisition obeys explicit download/license policy.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-gpu --test wdbx_3d_neural_provider
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** This packet builds the conformance registry and negative adapter tests, not every vendor runtime at once. Each selected provider needs its own reviewed implementation packet and real-device/service qualification using the same contract. Preserve optional support in the roadmap without marking it Current.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Provider/device classification is correct and missing providers fail explicitly; no fictional universal NPU/TPU implementation. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable a descriptor/binding; core CPU and existing local integrations remain intact.

<a id="t39"></a>
### T39 — Secure remote compute jobs and reconcile outcomes

**State:** Proposed; not executed. **Repository root:** WDBX / ABI. **Dependencies:** T15, T16, T30, T33.

**Files:**
- Create: `crates/abi-wdbx/src/cluster3d/compute_job.rs`
- Create: `crates/abi-wdbx/tests/wdbx_3d_remote_jobs.rs`
- Extend: actual authenticated transport seam only after inspection

**Interfaces — consumes:** ComputeRequest, qualification and policy context, operation IDs, read-view/input/model digests, bounded transport.

**Interfaces — produces:** `RemoteJobEnvelopeV1` fields from section 5.9; `submit_job(envelope: RemoteJobEnvelopeV1) -> Result<JobReceipt, RemoteJobError>`; `reconcile_job(id: OperationId) -> Result<RemoteJobOutcome, RemoteJobError>`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `capability_binds_destination_and_input`: Replay to another node/scope/model/input is rejected.
  - `signature_is_not_computation_proof`: A signed wrong result fails strict validation or remains untrusted, not automatically accepted.
  - `disconnect_does_not_create_new_operation`: A retry/status lookup uses the original operation ID.
  - `remote_worker_cannot_commit_memory`: Compute credentials cannot append canonical user records.
  - `deadline_and_egress_are_enforced`: Expired jobs and unauthorized destination transfers are denied before disclosure.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_remote_jobs
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use allowlisted kernels/models and authenticated secure transport; avoid arbitrary remote code execution. Initial profile trusts qualified workers for its stated numeric/coverage assumptions. Define complete verification or explicit approximation for untrusted/spot-checked results.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Input privacy, idempotency, outcome reconciliation and worker/commit separation are tested. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable remote dispatch; retain accepted canonical records and reconcile already admitted jobs.

<a id="t40"></a>
### T40 — Specify and implement the scoped cluster control plane

**State:** Proposed; not executed. **Repository root:** WDBX / ABI. **Dependencies:** T00, T16, T39.

**Files:**
- Create: `crates/abi-wdbx/src/cluster3d/control.rs`
- Create: `crates/abi-wdbx/tests/wdbx_3d_cluster_control.rs`
- Read: current membership, rendezvous placement, RPC, and gateway lineage code

**Interfaces — consumes:** Existing signed membership behavior; trusted-node model; ADR12/13; transport and identity policies.

**Interfaces — produces:** `ClusterConfiguration { epoch, members, roles, failure_domains, placement_policy, durability_policy }`; `ControlDecision` binds prior epoch and authorized change. A selected reviewed consensus implementation or authoritative control sequencer must be named before enabling cross-host writes.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `stale_control_epoch_is_fenced`: Old owners cannot activate placements/configuration after a committed epoch change.
  - `membership_lineage_is_not_consensus_by_itself`: A signed local record does not automatically satisfy cross-host agreement.
  - `node_identity_is_not_network_address`: IP/DNS changes do not mint authority.
  - `certificate_rotation_preserves_authentication`: Overlap/expiry/revocation failure paths do not downgrade to plaintext.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_cluster_control
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Keep causal user data independent from ordered control metadata. Select the smallest reviewed control mechanism satisfying the profile; do not write an ad hoc production Raft implementation merely to fulfill the name. Existing local demos remain separate evidence.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Named control-plane safety assumptions and fault tests exist; no Byzantine or global data-linearizability claim. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Freeze control changes and use the last verified configuration under its documented availability rules.

<a id="t41"></a>
### T41 — Qualify exact replication and durable acknowledgment classes

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T13, T15, T40.

**Files:**
- Create: `crates/abi-wdbx/src/cluster3d/replication.rs`
- Create: `crates/abi-wdbx/tests/wdbx_3d_replication.rs`
- Reuse: current exact committed-transaction export/import and repair code

**Interfaces — consumes:** Exact existing object bytes, causal parents, ClusterConfiguration, operation IDs, filesystem durability evidence.

**Interfaces — produces:** `DurabilityClass::{Volatile,LocalDurable,ReplicatedDurable(policy_id)}`; `ReplicationReceipt { object_identity, configuration_epoch, durable_replica_evidence, class }`; `replicate_committed_object(object, policy) -> Result<ReplicationReceipt, ReplicationError>`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `ack_means_configured_durable_copies`: ReplicatedDurable requires the declared count and fault-domain placement of flushed verified objects.
  - `quorum_does_not_erase_causal_conflicts`: Concurrent writer versions remain visible after replication/repair.
  - `missing_parent_is_bounded_pending`: Objects with unresolved parents are not served as verified complete state.
  - `same_identity_different_bytes_is_integrity_error`: Equivocating writer/sequence content is not accepted as a duplicate.
  - `lost_ack_is_reconciled_idempotently`: Receiver restart/retry does not create an extra committed effect.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_replication
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Preserve exact bytes and original signatures; do not reserialize replicated objects. A volatile receipt makes no crash-durability promise. Quorum ACKs describe copies, not serializability.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Fault-injected ACK/recovery behavior matches each declared durability class. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable new quorum modes or fail closed; never silently reinterpret an existing acknowledged class.

<a id="t42"></a>
### T42 — Bind causal cuts and merge distributed search honestly

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T14, T18, T20, T40, T41.

**Files:**
- Create: `crates/abi-wdbx/src/cluster3d/read_cut.rs`
- Create: `crates/abi-wdbx/src/cluster3d/query_merge.rs`
- Test: `crates/abi-wdbx/tests/wdbx_3d_distributed_query.rs`

**Interfaces — consumes:** ReadView, per-shard frontiers, canonical dependency evidence, query/ranking policies, placement epoch.

**Interfaces — produces:** `DistributedReadCut { configuration_epoch, scoped_shard_views, dependency_closure, consistency }`; `merge_shard_results(cut: &DistributedReadCut, query: &HybridQueryV1, results: &[ShardQueryOutcome]) -> Result<QueryOutcome, DistributedQueryError>`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `dependent_record_without_parent_breaks_causal_cut`: A cross-shard dependency missing from the cut forces repair/wait/error, not snapshot success.
  - `replicas_do_not_duplicate_records`: Replica copies deduplicate by observation identity/version.
  - `common_score_topk_merge_matches_global_oracle`: Simple fixed-score exact results match complete global enumeration.
  - `local_rrf_is_not_claimed_global_exact`: Local ranks/normalizations are not merged as an exact global score.
  - `missing_shard_is_visible_without_scope_leak`: Partial results state permitted incompleteness without revealing forbidden topology.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_distributed_query
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Provide local_snapshot, dependency-closed causal_cut, and explicit best_effort modes. Do not invent a global scalar revision. Cursor binding includes placement epoch and source views; missing completeness is an error unless explicitly permitted.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Consistency and exactness claims match the protocol; no silent partial or mixed-generation results. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Fail back to a complete authorized local view only when it satisfies the requested contract; otherwise return unavailable.

<a id="t43"></a>
### T43 — Make rebalance and anti-entropy restartable

**State:** Proposed; not executed. **Repository root:** WDBX. **Dependencies:** T13, T40, T41, T42.

**Files:**
- Create: `crates/abi-wdbx/src/cluster3d/rebalance.rs`
- Create: `crates/abi-wdbx/tests/wdbx_3d_rebalance.rs`

**Interfaces — consumes:** Placement epochs, immutable object digests, causal cut, current replication/repair mechanisms.

**Interfaces — produces:** `RebalancePlan { source_epoch, target_epoch, object_coverage, destination_policy }`; `RebalanceState::{Copying,Verifying,CatchingUp,Ready,Activated,RetainedSource}`; progress uses object identity and validated chunk offsets.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `resume_verifies_chunks_and_final_object`: A filename/offset alone cannot authorize a resumed object.
  - `destination_policy_precedes_copy`: A faster unauthorized region/node never receives bytes.
  - `cutover_waits_for_tail_and_coverage`: Activation cannot lose writes committed during bulk copy.
  - `source_retention_is_not_automatic_gc`: Source bytes remain until verified coverage/leases/holds and authorized retention permit deletion.
  - `crash_during_cutover_has_single_ownership_epoch`: Routing never treats both stale and new owners as unfenced authorities.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_rebalance
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Reuse rendezvous placement as a baseline where suitable, with scope/failure-domain constraints. Measure hot-cell/shard skew before adding spatial partitioning. Keep copying, activation and garbage collection separate operations.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Interrupted rebalance resumes without data loss, unauthorized transfer or source disappearance. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Revert routing only to a verified compatible placement; retain both copies until safe resolution.

<a id="t44"></a>
### T44 — Enforce the strict specimen-wide comparison cap across nodes

**State:** Proposed; not executed. **Repository root:** WDBX / STUDIO. **Dependencies:** T33, T39, T40.

**Files:**
- Create: `crates/abi-wdbx/src/cluster3d/scan_credits.rs`
- Create: `crates/abi-wdbx/tests/wdbx_3d_scan_credits.rs`
- Create: `docs/claims/wdbx-3d/scan-credit-model.md`

**Interfaces — consumes:** Original I14/I23 cap, trusted worker identities, control epochs, dispatch and resource accounting.

**Interfaces — produces:** `ScanCreditGrant { specimen_id, coordinator_epoch, grant_id, credits, worker }`; `CreditState::{Granted,Running,Returned,Uncertain,CessationVerified}`; `reserve_comparisons(count: u32) -> Result<ScanCreditGrant, BudgetError>`; `record_cessation(grant, evidence) -> Result<(), BudgetError>`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `expiry_does_not_reissue_running_credits`: Disconnected worker continuing 600 comparisons leaves only 400 of 1,000 available, even after lease expiry.
  - `coordinator_restart_preserves_uncertain_reservations`: Crash recovery cannot grant the same credits twice.
  - `gpu_batch_reserves_per_comparison`: A 1,001-comparison high-resolution batch cannot use one kernel credit to bypass the cap.
  - `fenced_result_does_not_prove_compute_stopped`: Rejecting late results does not release the physical-work reservation.
  - `strict_mode_can_sacrifice_availability`: A partition with all credits uncertain blocks new work honestly.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_scan_credits
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use conservative durable accounting or an explicitly pinned single coordinator whose failover waits for cessation. Never claim a strict physical cap against malicious workers ignoring the protocol. Model small states exhaustively and test real worker processes after the protocol is accepted.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** The global accounting invariant holds under pause/partition/restart/late-completion traces. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable remote high-resolution scans and wait for existing grants to reach verified terminal state.

<a id="t45"></a>
### T45 — Run a real-host cluster qualification campaign

**State:** Proposed; not executed. **Repository root:** WDBX / ABI. **Dependencies:** T39, T40, T41, T42, T43, T44.

**Files:**
- Create: `tests/cluster3d/qualification/` harness and fixture configs
- Create: `docs/ops/wdbx-3d-cluster-qualification.md`
- Create: `docs/claims/wdbx-3d/cluster-host-matrix.json`

**Interfaces — consumes:** Approved testbed, three or more distinct hosts with documented physical/virtual fault-domain placement, bounded synthetic datasets, cluster contracts.

**Interfaces — produces:** `ClusterQualificationReport` with host identities, failure domains, network/storage layout, operation histories, injected faults, observed outcomes and protocol checks.

- [ ] **Step 1 — Define acceptance/negative fixtures.** Pin these named cases and exact assertions:
  - `majority_minority_partitions_match_contract`: Quorum-required writes fail without required replicas; optional causal-local writes are explicitly labeled.
  - `disk_full_slow_peer_and_restart_are_bounded`: Admission and recovery do not hang or silently lose acknowledged data.
  - `certificate_and_policy_revocation_propagate`: No plaintext downgrade or stale authorized materialization.
  - `shared_physical_host_is_disclosed`: Separate VM namespaces do not masquerade as independent physical failure domains.
  - `numeric_and_query_failures_preserve_claim_boundaries`: Wrong remote outputs or missing candidates are not accepted as exact.

- [ ] **Step 2 — Establish the baseline or blocked gate.** Discover the actual cases and run only under the required execution authorization. Record acceptance failures or unverified status; do not manufacture a failing test when this packet is qualification/design rather than a behavior change.

```text
Use the exact approved harness command recorded in the testbed manifest; do not substitute a local mock for a real-host row.
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Run bounded loss/latency/reordering/partition/restart/rebalance tests and preserve operation histories. This packet requires specific external execution authorization; writing its runbook does not create a testbed. Use actual networked machines for cross-host claims; retain local-process evidence separately.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Every advertised cluster guarantee has real-host evidence plus clearly stated shared failure domains and limitations. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Stop the isolated testbed and preserve evidence; no production cluster is touched.

<a id="t46"></a>
### T46 — Bind HybridTable and specimen identity without changing semantics

**State:** Proposed; not executed. **Repository root:** STUDIO. **Dependencies:** T00, T08, T10, T11, T16.

**Files:**
- Create: `native/specimen-core/src/wdbx3d/storage_adapter.rs`
- Create: `native/specimen-core/tests/wdbx_3d_specimen_storage.rs`
- Create: `tests/wdbx3d/specimen-identity.test.ts`

**Interfaces — consumes:** Original chapters 1–6, R01–R05/R22, I01–I06; existing browser/native storage profiles; WDBX read/write adapter.

**Interfaces — produces:** `SpecimenStoreAdapter::read_snapshot(specimen_id) -> Result<SpecimenSnapshot, SpecimenError>`; `commit_specimen(expected_revision, change_set) -> Result<SpecimenReceipt, SpecimenError>`; HybridTable lookup remains `find_all(PatternKey)` with unique internal node/entry/slot refs.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `duplicate_logical_ids_return_all_nodes`: Twenty is per-node entry capacity, not number of nodes/results sharing a Pattern ID.
  - `original_patterns_are_preserved`: Coarse keys never replace original text/visual payloads.
  - `structural_duplicate_checks_full_content`: Hash collisions do not incorrectly reject distinct structures.
  - `encoder_versions_do_not_compare_as_bare_strings`: Modality/resolution/version namespace remains part of equality.
  - `stale_revision_does_not_replace_committed_specimen`: CAS failure leaves original native state and recovery copy intact.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p specimen-core --test wdbx_3d_specimen_storage; bun test tests/wdbx3d/specimen-identity.test.ts
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Implement an adapter at the actual native storage seam, not a second canonical member-fact store. Preserve browser IndexedDB and native archive boundaries. Record any original open equality decision without silently redefining it.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Original identity/storage rules have per-runtime tests; no complete specimen conformance claim yet. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable the additive adapter; use original runtime storage and preserve recovery copies.

<a id="t47"></a>
### T47 — Preserve Language Governance and sandbox Sigils

**State:** Proposed; not executed. **Repository root:** STUDIO. **Dependencies:** T46.

**Files:**
- Create: `native/specimen-core/tests/wdbx_3d_language_sigils.rs`
- Create: `tests/wdbx3d/language-sigils.test.ts`
- Modify: discovered preprocessing/Sigil modules only after original behavior is characterized

**Interfaces — consumes:** Original chapters 7–8, resolution R15/R17/R21, normalized triples, scoped bindings, original commands.

**Interfaces — produces:** `LanguagePreparation` contains original input, bounded variants/triples, encoder profile and origin mapping; `SigilExecutionContext` contains declared scope/capabilities/fuel/memory/depth, not ambient shell authority.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `normalization_retains_origin_and_bounds_fanout`: Variants map back to original spans; fan-out cannot escape hard budgets.
  - `reserved_sigils_cannot_be_shadowed`: Matching a user-defined name does not grant reserved authority.
  - `wildcard_binding_uses_declared_candidate_route`: Bind rules do not bypass inhibition or high-resolution/global limits silently.
  - `generated_literal_is_inert`: Learned generated text is escaped as data and cannot become a shell/network action.
  - `fuel_and_recursion_are_enforced`: Automata/scripts terminate with explicit resource outcome.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p specimen-core --test wdbx_3d_language_sigils; bun test tests/wdbx3d/language-sigils.test.ts
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Preserve the original scoped operation set. Add missing safety enforcement at the interpreter/admission boundary, not by globally deleting user-defined Sigils. Macro reuse is not a constant-time complexity claim.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Language and script behavior retain intended semantics while capabilities and resources remain bounded. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Revert only new interpreter changes under fixture evidence; do not erase persisted user definitions.

<a id="t48"></a>
### T48 — Preserve independent votes and attachment scheduling

**State:** Proposed; not executed. **Repository root:** STUDIO. **Dependencies:** T31, T46, T47.

**Files:**
- Create: `native/specimen-core/tests/wdbx_3d_votes_attachments.rs`
- Create: `tests/wdbx3d/votes-attachments.test.ts`
- Modify: discovered vote/attachment modules in focused files

**Interfaces — consumes:** Original chapters 9–10, high-resolution comparison contract, node/entry/slot refs and scan budgets.

**Interfaces — produces:** `QualifiedVote` binds node/entry/slot, original triple/fan-out, confidence baseline/effective values and attachment group; `AttachmentActivation` guarantees target scheduling on unmatched remainder, not a vote.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `all_independently_qualified_entries_may_vote`: One node can contribute multiple eligible entries; no central winner before orchestration.
  - `deep_scan_is_always_high_resolution`: Low-resolution lookup never substitutes for secondary high-resolution verification.
  - `hard_attachment_can_produce_no_vote`: Target scheduling does not bypass its gate.
  - `same_triple_arbitration_preserves_complementary_votes`: Only redundant alternatives in the declared group compete; attachment group rules remain.
  - `confidence_and_selection_weight_are_distinct`: Confidence remains [-100,100]; selection weights are finite/nonnegative under a declared transform.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p specimen-core --test wdbx_3d_votes_attachments; bun test tests/wdbx3d/votes-attachments.test.ts
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Preserve source confidence/gate rules. Independent numeric reference decisions resolve near-threshold ambiguities; hardware batching does not silently alter contribution semantics.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Original voting/attachment invariants pass per runtime without collapsing the system into a one-winner ranker. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable new batched/adapter path and use characterized local voting.

<a id="t49"></a>
### T49 — Make cycle phases, support retrieval, and provenance coherent

**State:** Proposed; not executed. **Repository root:** STUDIO. **Dependencies:** T15, T23, T46, T48.

**Files:**
- Create: `native/specimen-core/tests/wdbx_3d_cycles_provenance.rs`
- Create: `tests/wdbx3d/cycles-provenance.test.ts`
- Modify: actual cycle/orchestrator/supporting-retrieval seams

**Interfaces — consumes:** Original chapters 11–12 and 18, immutable vote set, resource IDs, read view, commit/cancel fence.

**Interfaces — produces:** `CycleTrace` records phase transitions, consulted supporting resources, votes, transformations and output-span contributors; `CycleOutcome::{Committed,CancelledBeforeCommit,Failed,OutcomeUnknown}` uses the canonical operation identity.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `orchestration_begins_after_vote_barrier`: Post-vote composition cannot silently rewrite submitted node reasoning.
  - `support_lookup_uses_two_channels`: Generated pattern_key and optional compatible resource_id are distinct; all candidates receive contextual filtering.
  - `side_resource_filter_is_not_node_gate`: Supporting retrieval does not acquire a second invented node-confidence scan.
  - `provenance_is_built_during_composition`: Each span has actual contributions/transforms, not retrospective guesses.
  - `cancel_commit_race_has_one_truthful_outcome`: Staged edits are absent if cancellation wins; committed cycles return their receipt if commit wins.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p specimen-core --test wdbx_3d_cycles_provenance; bun test tests/wdbx3d/cycles-provenance.test.ts
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Bind one coherent input snapshot and explicit authority bundle. Preserve anti-staleness semantics, original support channels and first-class provenance. Record generated output as output/proposal, not automatically trusted memory.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Cycle lifecycle and output provenance remain consistent across cancellation, support selection and replay. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable the new cycle adapter; preserve committed state and trace receipts.

<a id="t50"></a>
### T50 — Preserve non-learning Transient Memory, ATP, history, and vision

**State:** Proposed; not executed. **Repository root:** STUDIO. **Dependencies:** T49.
**Conditional dependencies:** T37 for a selected Core ML/NPU profile; T26 only for an added projected-geometry visualization.

**Files:**
- Create: `native/specimen-core/tests/wdbx_3d_transient_atp_vision.rs`
- Create: `tests/wdbx3d/transient-atp-vision.test.ts`
- Modify: actual transient/ATP/history/visual modules only where gaps are measured

**Interfaces — consumes:** Original chapters 13–16, R08/R09/R12/R16/R18, pretrained model references and runtime profile.

**Interfaces — produces:** `TransientRun` owns bounded ephemeral activations; `AtpTransition` separates baseline from temporary modulation; `VisualObservation` preserves original visual payload and any distinct actual geometry/SDF profile.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `inference_does_not_train_or_auto_learn`: Using Transient Memory changes neither persistent weights nor learned knowledge.
  - `jitter_does_not_accumulate_on_read`: Repeated reads preserve baseline; endpoint and crystallization exceptions hold.
  - `history_default_is_editorial_and_bounded`: 100,000 remains the inherited editorial profile value, not an unlimited allocation promise.
  - `fuzzy_art_is_ephemeral_unless_saved`: Rendering does not silently become durable memory.
  - `nonlinear_sdf_label_is_not_math_proof`: Original pixel-array representation is not mislabeled as a true signed-distance field.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p specimen-core --test wdbx_3d_transient_atp_vision; bun test tests/wdbx3d/transient-atp-vision.test.ts
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** This is a preservation packet with separate subtests for each organ; fix only measured gaps. Core specimen behavior must not depend on optional Core ML being present: T37 is needed only to qualify a selected NPU path, and the local original profile remains usable without it.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Original ephemeral/durable boundaries and stochastic exceptions pass for the enabled runtime; hardware qualifications remain separate. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable optional model/device extensions; keep original organs, history, definitions and saved weights unchanged.

<a id="t51"></a>
### T51 — Preserve feedback, approved growth, mutation, and PHAGY

**State:** Proposed; not executed. **Repository root:** STUDIO. **Dependencies:** T46, T48, T49.

**Files:**
- Create: `native/specimen-core/tests/wdbx_3d_feedback_growth.rs`
- Create: `tests/wdbx3d/feedback-growth.test.ts`
- Modify: discovered Type A/B, feedback, mutation, maintenance modules

**Interfaces — consumes:** Original chapters 17–19, contributor trace, node/slot refs, learning proposals, original idle policy.

**Interfaces — produces:** `FeedbackOperation` binds cycle/contributor subset/choice/draw identities; `LearningProposal` has pending/approved/rejected/stale; `MutationLineage` is monotonic by stable slot; `MaintenanceJob` is bounded and idempotent.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `feedback_draws_are_independent_per_contributor`: Retries reuse recorded decisions; one contributor is not resampled repeatedly to bias strength.
  - `type_b_inference_needs_identified_approval`: Permitted internal edits do not imply permission to commit new inferred knowledge; /learn proposal binding is checked.
  - `slot_remixes_at_most_once`: Renumber/import/pool shortening cannot erase a prior remix marker.
  - `idle_schedule_uses_eligible_60_to_120_seconds`: Foreground load and hard caps dominate maintenance; no unexplained three-second default.
  - `zero_strength_preserves_inhibition_lineage`: Removing an active node does not erase growth inhibition/history; open grave policy is not silently invented.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p specimen-core --test wdbx_3d_feedback_growth; bun test tests/wdbx3d/feedback-growth.test.ts
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Keep Type A residual correction and Type B internal-edit scopes distinct. Use versioned deterministic draw identities for replay while preserving intended distributions. PHAGY automata receive unique jobs, deadlines and explicit cleanup results, not arbitrary deletion authority.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Approval, feedback independence and lifetime mutation restrictions survive edits/retries/restart. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable new maintenance/learning routes; never roll back by erasing mutation or approval history.

<a id="t52"></a>
### T52 — Preserve Index Rafts, contact scopes, commands and hard limits

**State:** Proposed; not executed. **Repository root:** STUDIO. **Dependencies:** T46, T47, T51.
**Conditional dependencies:** T44 for distributed high-resolution scans.

**Files:**
- Create: `native/specimen-core/tests/wdbx_3d_rafts_contacts_commands.rs`
- Create: `tests/wdbx3d/rafts-contacts-commands.test.ts`
- Modify: actual scheduler/ledger/composer seams discovered by T00

**Interfaces — consumes:** Original chapters 20–23, all-match semantics, command aliases, contact scopes and strict comparison credits.

**Interfaces — produces:** `TraversalCursor { snapshot, disjoint_range, next_index, mode }`; `TraversalMode::{FindFirst,FindAll}`; `ContactReadCapability`; unchanged original command routing and explicit new namespaces only.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `raft_ranges_are_disjoint_complete_and_start_zero`: All required positions are covered exactly once; continuation always yields.
  - `find_all_does_not_stop_on_first_duplicate`: Duplicate Pattern/resource IDs return every compatible match.
  - `contact_name_equality_is_not_authorization`: Same names in two compartments do not permit a join.
  - `all_original_commands_and_aliases_remain`: /prompt /right /wrong /wrng /contributorfractal /saveSpecimen /loadSpecimen /addPattern /learn retain behavior.
  - `local_and_remote_scans_share_one_cap`: The enabled distributed profile uses T44; local-only mode does not need a running cluster.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p specimen-core --test wdbx_3d_rafts_contacts_commands; bun test tests/wdbx3d/rafts-contacts-commands.test.ts
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Keep traversal tasks, physical chunks, threads and consensus Raft as different concepts. Do not claim scheduling removes scan work. User hard limits constrain attachments, brainstorm, scripts and maintenance. T44 is a conditional dependency for distributed scans, not a requirement for local-only conformance.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Original command/contact/traversal semantics and hard bounds pass independently per runtime/profile. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable remote scans/additive commands; retain original local traversal and compartmental data.

<a id="t53"></a>
### T53 — Preserve whole-specimen save, import, and rollback

**State:** Proposed; not executed. **Repository root:** STUDIO / WDBX. **Dependencies:** T10, T15, T46, T49, T50, T51, T52.

**Files:**
- Create: `native/specimen-core/tests/wdbx_3d_specimen_restore.rs`
- Create: `tests/wdbx3d/specimen-restore.test.ts`
- Create: `docs/ops/wdbx-3d-specimen-compatibility.md`
- Modify: actual archive/import/storage-adapter modules only after characterization

**Interfaces — consumes:** Original chapters 24–26, browser JSON/native archive formats, entire durable-organ inventory, read/write compatibility policy.

**Interfaces — produces:** `SpecimenExportManifest` binds all durable organs/assets/model references without keys/handles/live continuations; `validate_import(bytes, limits, reader_profile) -> Result<VerifiedImport, ImportError>`; `activate_import(verified, expected_revision) -> Result<SpecimenReceipt, ImportError>`.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `whole_specimen_roundtrips`: Nodes, resources, Sigils, attachments, history, ATP, network definition, inhibition, lineage, proposals and contacts survive.
  - `ephemeral_and_secret_state_is_excluded`: No live activations, GPU handles, credentials or raw keys enter default exports.
  - `archive_traversal_bombs_and_duplicate_refs_fail`: Malformed archive lengths/paths/identities fail before replacement.
  - `old_writer_cannot_destroy_new_fields`: Rollback blocks an incompatible writer rather than silently dropping records.
  - `failed_import_keeps_original_recovery_copy`: Validation/crash failure leaves the original active and recovery copy intact.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p specimen-core --test wdbx_3d_specimen_restore; bun test tests/wdbx3d/specimen-restore.test.ts
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use coherent staged validation and fenced activation. Preserve platform-specific native/browser semantics and model-by-digest references. A missing model produces explicit unavailable capability, not a hidden download.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** All original durable state is covered and restoration/rollback behavior is demonstrated for the selected runtime. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Restore an already verified compatible generation/archive; never remove recovery copies automatically.

<a id="t54"></a>
### T54 — Reconcile privacy and erasure as an approval-gated program

**State:** Proposed; not executed. **Repository root:** WDBX / ABI / STUDIO. **Dependencies:** T00, T10, T15, T16, T53.
**Conditional dependencies:** T41 for distributed erasure implementation.

**Files:**
- Create: `docs/superpowers/specs/2026-10-20-wdbx-3d-privacy-derivatives.md`
- Create: `docs/claims/wdbx-3d/derivative-inventory.json`
- Proposed tests after approval: `crates/abi-wdbx/tests/wdbx_3d_erasure_model.rs`

**Interfaces — consumes:** Full original erasure/completion design and approval status; source/derivative/key/replica/export inventory.

**Interfaces — produces:** `ErasureScopeInventory` names facts, embeddings, postings, geometry, projection models, summaries, caches, replicas and known exports; `ErasureReceiptModel` distinguishes blocked, logically_fenced, keys_destroyed, derivative_cleanup_pending, outside_control and complete_within_declared_scope.

- [ ] **Step 1 — Define acceptance/negative fixtures.** Pin these named cases and exact assertions:
  - `approval_is_not_inferred_from_spatial_work`: No key destruction/cutover/hold bypass is allowed by a spatial feature flag.
  - `held_scope_is_reported`: Active signed holds block the modeled operation under the governing draft rules.
  - `old_backup_cannot_resurrect_erased_key_epoch`: An approved future implementation must test restored wrapped-key/registry state.
  - `receipt_does_not_claim_uncontrolled_deletion`: Third-party copies, already released borrows and unknown hardware remnants are not falsely certified absent.
  - `derivative_training_cohort_is_inventory_item`: Projection/codebook/model derivatives are not omitted merely because raw records were removed.

- [ ] **Step 2 — Establish the baseline or blocked gate.** Discover the actual cases and run only under the required execution authorization. Record acceptance failures or unverified status; do not manufacture a failing test when this packet is qualification/design rather than a behavior change.

```text
After the separate design approval only: cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_erasure_model
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Planning/design reconciliation is allowed now; product erasure implementation remains blocked on the original explicit approval gate. Preserve per-fact/per-member custody and original crypto choices. Cross-host cleanup acknowledgments are evidence with limits, not mathematical proof of remote deletion. Cluster-related dependencies apply to distributed erasure, not to completing this document review.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** No destructive implementation is merged before the controlling approval. Privacy claims explicitly state scope, assumptions, pending replicas, holds and residual exposure. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** No destructive action occurs in this task until separately approved; once authorized erasure commits, rollback must not resurrect erased keys.

<a id="t55"></a>
### T55 — Build security, fuzz, supply-chain and evidence gates

**State:** Proposed; not executed. **Repository root:** WDBX / ABI / STUDIO. **Dependencies:** T02, T05.
**Conditional dependencies:** T15–T16 for durable authorized service gates; T28 for gateway/explorer gates; T30–T39 for hardware and remote-job gates; T40–T45 for cluster gates; T46–T54 for specimen/privacy gates.

**Files:**
- Create: `fuzz/fuzz_targets/wdbx_3d_query.rs`, `wdbx_3d_manifest.rs`, `wdbx_3d_remote_frame.rs`
- Create: `crates/abi-wdbx/tests/wdbx_3d_security_regressions.rs`
- Extend: T00 evidence validator and supported CI profile matrix

**Interfaces — consumes:** All enabled profile contracts, original corpus, selected dependencies/features, prior fault seeds and source manifest.

**Interfaces — produces:** `ProfileGateReport` includes exact command/build/test count, fuzz duration/seed/resource cap, findings, advisories/licenses, missing hardware, and provenance; no skip can be serialized as pass.

- [ ] **Step 1 — Write the failing behavior tests.** Pin these named cases and exact assertions:
  - `hostile_lengths_fail_before_allocation`: Oversized nesting, lengths, compressed ratios and archive members are rejected within budget.
  - `tampered_index_cannot_poison_trusted_results`: Recomputed checksum without correct source/auth binding is rejected/rebuilt under secure profile.
  - `logs_and_errors_do_not_emit_secrets`: Canary private payloads/IDs/embeddings do not appear in default logs or denied responses.
  - `feature_tuple_must_be_supported`: No blanket feature combination or warning waiver substitutes for the actual gate.
  - `test_count_and_failure_artifacts_are_required`: Zero tests, truncated logs or unknown exit status cannot qualify a task.

- [ ] **Step 2 — Discover and run the red target.** Verify the named tests are listed, then run the command below. A new behavior must fail for the expected reason; a toolchain/fixture/environment failure is a separate blocker, not the desired red evidence.

```text
cargo +1.99.1 test --locked -p abi-wdbx --test wdbx_3d_security_regressions
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Run bounded parser/geometry/transport fuzzing and property tests in isolated stores. Keep optional nightly-only fuzz instrumentation outside the stable production graph. Re-run regression seeds. Perform dependency/license review without claiming an external audit from tool success.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** All enabled release profiles have explicit gate evidence; unresolved critical issues block only the affected advertised scope or the entire release if shared. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable affected features and preserve reproducible failure artifacts; never weaken the gate to pass.

<a id="t56"></a>
### T56 — Measure benchmarks and select thresholds before promotion

**State:** Proposed; not executed. **Repository root:** WDBX / ABI. **Dependencies:** T06, T55.
**Conditional dependencies:** T18–T20 for hybrid/ANN/fusion benchmarks; T30–T39 for hardware and remote compute benchmarks; T40–T45 for real-host benchmarks.

**Files:**
- Create: `benches/wdbx_3d/` harness and bounded dataset manifests
- Create: `docs/benchmarks/wdbx-3d/protocol.md`
- Create: `docs/benchmarks/wdbx-3d/results/` raw samples and reports

**Interfaces — consumes:** Pinned baseline and candidate revisions, exact oracles, device/host profiles, workload definitions and candidate SLO policy.

**Interfaces — produces:** `BenchmarkManifest` and `BenchmarkReport` defined in section 8; measurements include p50/p95/p99, throughput, memory, disk, rebuild/recovery, authorized recall and setup/transfer/verification-inclusive hardware cost.

- [ ] **Step 1 — Define acceptance/negative fixtures.** Pin these named cases and exact assertions:
  - `baseline_and_candidate_share_workload`: Same data/model/filter/cohort/candidate budget and hardware conditions are recorded.
  - `quality_tradeoff_is_visible`: Recall loss cannot be hidden behind faster latency.
  - `tails_are_not_averaged_across_hosts`: Aggregate latency uses samples/appropriate histograms, not mean of p99 values.
  - `production_thresholds_are_not_posthoc`: Thresholds are frozen before evaluating the candidate promotion set.
  - `missing_device_has_no_speedup_number`: Unexecuted GPU/NPU/remote rows remain blank/unverified, not estimated as measurements.

- [ ] **Step 2 — Establish the baseline or blocked gate.** Discover the actual cases and run only under the required execution authorization. Record acceptance failures or unverified status; do not manufacture a failing test when this packet is qualification/design rather than a behavior change.

```text
Run the exact harness command and budgets recorded in BenchmarkManifest after the harness is implemented; keep the full argv and raw outputs.
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Use tiny/medium/stress profiles with memory/time guards. Candidate defaults are proposed, not universal SLOs: strict spatial predicate mismatch count zero; permitted ANN Recall@10 target at least 0.95 on the named corpus and degradation no worse than 0.01 against the accepted baseline; investigate reproducible latency regressions over 10%. A reviewed workload-specific policy may tighten/change these before the candidate run, never after seeing results.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Any improvement claim includes quality/cost/error bars and reproducible evidence; hardware-specific absolute SLOs have approved baselines. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable an optimization that misses its quality/cost gate; retain its evidence and the accepted baseline.

<a id="t57"></a>
### T57 — Prepare profile-specific release, recovery, and operator sign-off

**State:** Proposed; not executed. **Repository root:** WDBX / ABI / STUDIO. **Dependencies:** T00, T01, T02, T06, T55, T56.
**Conditional dependencies:** T07–T16 for the durable spatial profile; T17–T24 for the hybrid/evidence profile; T26/T28/T29 for the explorer profile; T34–T38 for each advertised hardware adapter; T40–T45 for the cluster profile; T46–T53 for full specimen conformance; T54 approved implementation for any erasure claim.

**Files:**
- Create: `docs/ops/wdbx-3d-release.md`
- Create: `docs/ops/wdbx-3d-restore-rollback.md`
- Create: `docs/claims/wdbx-3d/release-manifest.json`
- Create: profile-specific compatibility/incident/credential-rotation runbooks

**Interfaces — consumes:** Only enabled and accepted profile evidence; original source/fixture inventory; approved durability/SLO policy; compatible reader/writer matrix.

**Interfaces — produces:** `ReleaseManifest` binds binaries, source/dependency/build digests, enabled profiles, feature flags, claims, evidence, known gaps, rollback floor, and operator approvals.

- [ ] **Step 1 — Define acceptance/negative fixtures.** Pin these named cases and exact assertions:
  - `disabled_optional_features_do_not_claim_conformance`: Unavailable vendor/cluster profiles are absent from advertised capabilities.
  - `restore_demonstrates_declared_rpo_rto`: An isolated drill checks acknowledged data and measured recovery under the selected durability class.
  - `rollback_reader_and_writer_are_compatible`: New canonical namespaces cannot be dropped by an old writer.
  - `production_requires_explicit_approval`: Packaging a plan/artifact does not authorize deploying it or changing credentials.
  - `original_feature_coverage_has_no_silent_waivers`: Original-conformance claims require every controlling row to pass or a visibly approved amendment.

- [ ] **Step 2 — Establish the baseline or blocked gate.** Discover the actual cases and run only under the required execution authorization. Record acceptance failures or unverified status; do not manufacture a failing test when this packet is qualification/design rather than a behavior change.

```text
Execute only the selected profile gate and isolated restore/rollback commands in the approved release runbook; no automatic push or deployment command.
```

- [ ] **Step 3 — Implement only this packet, or complete its scoped qualification/design.** Progress through synthetic reference, shadow reads, opt-in queries, optional adapters, read-only explorer, and separately authorized cluster/production pilots. Preserve current data and recovery generations. Gate local spatial releases independently; full specimen and distributed profiles additionally require their task packets. Record witnesses rather than inventing sign-off.

- [ ] **Step 4 — Verify the accepted behavior.** Re-run the named target with nonzero case counts; run affected compatibility fixtures and the supported profile gate. Preserve exact commands, raw red/green output, source/build identities, unexecuted cases, and resource usage.

- [ ] **Step 5 — Review and stop at the packet boundary.** Release is a named, evidence-backed profile with honest limitations and explicit authorization; “final” is not shorthand for production proof. Provide the diff and evidence for review. Do not commit, push, deploy, or modify production data without separate authorization.

**Rollback/disable path:** Disable new routes/adapters/index activation when safe; retain compatible readers/writers and canonical data. Erasure rollback must never resurrect destroyed keys.

<a id="verification"></a>
## 8. Verification, fault injection, and benchmark protocol

### 8.1 Evidence states and a real definition of done

Every test/qualification row is one of `not_run`, `blocked`, `failed`, `passed`, or `not_applicable_with_reason`. A missing device is `blocked` or `not_run`, never `passed`. A source inspection is not an execution. A commit message is reported historical evidence until the underlying execution artifact is available. C0–C7 and Current/Partial/Proposed remain separate from these test-result values and the named release profiles.

A packet is ready for implementation review only when its dependencies, scoped design decisions, fixtures, expected result, budget, and rollback are explicit. It is complete only when the selected changes and required tests are actually executed, results are inspectable, compatibility remains intact, and review is recorded. Document validation below checks this plan's structure; it cannot prove the implementation or make an unrun test pass.

For a newly implemented behavior, preserve a red result that fails for the intended behavioral reason and a green result on the changed revision. For characterization, a previously implemented capability, or a qualification-only packet, an initial green result is legitimate. Do not intentionally break working code to manufacture TDD evidence. Dependency-installation failures, missing targets, zero selected tests, unavailable credentials, or unsupported devices are environment/coverage failures, not useful red tests.

### 8.2 Safe execution envelope

Establish WDBX, ABI, and STUDIO roots from real repository inspection. Record source revisions, dirty work, linked-worktree state, dependency graph, lockfiles and gate scripts. Preserve untracked/dirty files. Do not create another repository, switch branches, reset work, rewrite history, fetch private data, or alter signing/backup settings merely to satisfy a plan step.

All test stores must be uniquely named scratch directories owned by the test. Use filesystem roots and explicit environment settings that cannot accidentally select the live `~/.abi/` store or the native app's real data directory. Do not rely on the current working directory alone. Regression fixtures must contain synthetic data or already-authorized repository examples. Tests that spawn child processes pass the scratch root explicitly and remove only their own owned scratch paths under the agreed cleanup policy.

The Oct 19 source records a historical test/CI pause. The user's later master directive requires TDD and evidence for future behavioral implementation. This planning revision runs neither product tests nor CI. At execution time record the latest actual authorization and any remaining pause; do not silently override a still-active constraint. Read-only inspection and document validation do not constitute permission to deploy or modify production.

### 8.3 Supported build matrix, not a blanket feature union

The pinned toolchain is Rust 1.99.1. First record `rustc +1.99.1 -Vv` and `cargo +1.99.1 -V`. Under authorized implementation, the core qualification command set is:

```bash
cargo +1.99.1 check --workspace --all-targets --locked
cargo +1.99.1 fmt --all --check
cargo +1.99.1 clippy --workspace --all-targets --locked
cargo +1.99.1 test --workspace --locked
RUSTUP_TOOLCHAIN=1.99.1 bash tools/check.sh
```

These commands are planned, not claimed to have run. T00 verifies the gate script's actual contents and whether it overrides the selected toolchain or invokes other environments. Capture every command's exit status separately; a shell sequence's final success must not hide an earlier failure. Preserve the project's warning/lint policy rather than adding a new all-warnings-error policy without review. `--locked` prevents incidental dependency changes; an unavailable locked dependency is a blocker, not permission to update it silently.

Add separate feature tuples only after reading the actual manifests. CPU/default, optional research crypto, each GPU provider, each neural provider, and each native shell are distinct jobs. Do not combine mutually exclusive target-specific SDKs in one universal feature set. Tool-only fuzzing/sanitizer experiments may use a separate reviewed toolchain, but must not become a nightly production dependency.

| Environment | Required evidence for an advertised profile | Insufficient evidence |
|---|---|---|
| macOS ARM64 | Stable core tests; native-client tests; per-operation Metal and selected Core ML tests when advertised | A supported compute-unit preference or a detected ANE |
| macOS x86_64 | Stable core and native profile; Metal tests only on supported real device | ARM64 results or an assumed NPU |
| Linux x86_64 | Stable core; selected native/cluster profile; each selected CUDA/Vulkan/vendor runtime | A successful cross-compile |
| Linux aarch64 | Actual selected-target execution and supported provider evidence | x86_64 results with a different target name |
| Windows x86_64 | Stable core/native profile; selected Windows ML provider and actual device classification | Treating DirectML as generic NPU support |
| Browser | Browser executor, IndexedDB/export behavior, accessibility and rendering fallback | Native Rust or Tauri results |
| Separate-host cluster | Multiple actual hosts/VMs, declared physical failure-domain mapping, network/disk/process faults | Several processes or containers on one host |

Hardware configuration is evidence metadata, not an obligation to buy or provision machines during plan authoring. An unavailable profile stays unadvertised. Core spatial qualification does not wait for every vendor.

### 8.4 Deterministic fixtures and independent oracles

The small spatial fixture uses record IDs `10`, `20`, `30`, `40`, `50`, `60` at `(0,0,0)`, `(1,0,0)`, `(-1,0,0)`, `(0,1,0)`, `(0,0,1)`, `(2,0,0)`. For a unit-radius Euclidean query about the origin, the new checked ordering is `10,20,30,40,50`; a zero radius returns only `10`; an AABB `[-1,-1,-1]` through `[1,1,1]` contains those same five points. Insert them in multiple permutations. Distances tied geometrically must not change order with octant traversal. These new checked expectations do not silently redefine legacy methods.

Add a duplicate legacy ID with a distinct insertion ordinal to characterize the old raw index without losing an observation. Add `RecordId::Legacy(u64::MAX)` and a UUID in T08 to prove that durable identity never narrows to a slot. Add two maximal concurrent spatial versions of the same logical record: all-maximal mode returns both observations, preferred mode labels its presentation choice, and reject-conflicted mode fails explicitly.

For randomized tests, record generator/version/seed and the actual minimized failing case. Use 1,000 bounded seeds for the initial property budget; this is a test budget, not a proof over all floating-point values. Include empty, collinear, planar, coincident, clustered, highly skewed, octant-boundary, threshold-63/64/65, extreme finite `f32`, new-profile maximum magnitude, subnormal, signed-zero, negative-radius, invalid-bounds, NaN and infinity cases. Mutate one point and rebuild; query both the old pinned view and the new view.

An independent dyadic test comparator checks supported closed predicates and squared-distance ordering without reusing production pruning/reduction code. Exact mode must preserve boundary results rather than adding a convenient epsilon. Optimized broad-phase intervals are conservative or disabled. A new algorithm that cannot prove conservative pruning can still produce a valid explicitly approximate profile; it cannot masquerade as exact.

For vector operations, keep two references: the existing compatibility reduction profile and an independent high-precision/f64 reference suited to the bounded input domain. Capture zero-vector behavior, near-cancellation, long ragged tails, mixed magnitude, overflow/underflow, equal scores, near-threshold gates, and near-tied top-k. Per-model tolerances and ranking requirements are frozen before qualification. A universal `1e-5` tolerance is not an acceptable substitute for an operation contract.

### 8.5 Crash and race matrix

The injection point is an explicit test hook or controlled process fault in an isolated fixture; the assertion is about observable state, not merely absence of a panic.

| Fault ID | Injection | Required observable outcome | Owner |
|---|---|---|---|
| F01 | Torn canonical frame before its commit boundary | No acknowledged partial transaction; recovery follows the existing format contract | T02, T15 |
| F02 | A committed write's response is lost | Status by operation ID resolves the original receipt; retry does not create another effect | T15 |
| F03 | Index file written but manifest not durable | Old generation remains active or exact fallback is used | T13 |
| F04 | Manifest durable but activation pointer not advanced | Unpublished generation may be verified/reused; not silently treated as active | T13 |
| F05 | Activation changes while an old view is pinned | Old view retains compatible data/index leases; new view sees the new binding | T11–T15 |
| F06 | File digest valid, source/cohort/head binding wrong | Reject/rebuild; a valid checksum alone does not establish correct index contents | T12–T14 |
| F07 | Stale index missed a delete or geometry update | Complete tail reconciliation removes obsolete hits; otherwise full scan/fail | T14 |
| F08 | Policy revoked during a query or subscription | Deny later materialization at the defined boundary; never promise retroactive removal of returned plaintext | T16, T28 |
| F09 | Cancellation races canonical commit | Exactly one terminal semantic outcome: committed or cancelled-before-commit; ambiguous transport returns reconcilable unknown | T15, T49 |
| F10 | Blocking task/GPU kernel survives wrapper cancellation | Buffers and credits remain reserved until actual completion/cessation | T33, T44 |
| F11 | Accelerator returns finite but wrong shape/value | Reject under its output contract, quarantine qualification and follow allowed fallback | T30–T38 |
| F12 | CPU fallback cannot fit memory/deadline | Typed failure, not another model or an unauthorized remote upload | T33 |
| F13 | Worker sends another snapshot's valid signed result | Reject the job/read-view mismatch | T39 |
| F14 | Replica acknowledges before data durability | Test detects the forbidden ACK; profile cannot qualify | T41 |
| F15 | Quorum exists but cross-shard dependencies are missing | No coherent-snapshot claim; establish a closed cut or reject/label best-effort | T42 |
| F16 | Stale leader/control owner submits a configuration change | New control epoch fences it; causal data semantics remain separately defined | T40 |
| F17 | Rebalance interrupted before or after ownership cutover | Resume by content/epoch; source retained until coverage is established | T43 |
| F18 | Disconnected worker continues 300 credited comparisons | Those 300 credits remain unavailable for reassignment under the strict cap | T44 |
| F19 | Retry duplicates votes/feedback at another worker | One logical vote/feedback effect per stable operation identity | T49, T51 |
| F20 | Projection cohort changes with hidden data | Strict isolated output unaffected by forbidden cohort; revoke/invalidate old fit as required | T26–T29 |
| F21 | Multiway state writes succeed but export publication fails | No complete-bundle receipt; retry or inspect partial attempt under the defined marker protocol | T25 |
| F22 | Archive traversal, decompression bomb, missing model | Reject unsafe import before replacement; missing model is explicit, not silently downloaded | T53 |
| F23 | Old writer opens new authoritative attachments | Block incompatible writing; retain a safe reader/compatible writer | T10, T53, T57 |
| F24 | Approved future erasure followed by backup restoration | Key/tombstone epochs prevent modeled resurrection; uncontrolled copies remain outside the claim | T54 |

Disk-full and allocation-failure variants apply to every write/build/import path. Clock skew, process suspension, delayed messages, duplicate responses and cancellation are tested independently; a normal graceful shutdown is not a crash test. For real power-loss durability, the filesystem/storage stack and actual test method are part of the claim; a process kill alone does not prove physical power-loss behavior.

### 8.6 Benchmark workloads and promotion policy

Use small smoke, representative, and bounded stress datasets. Proposed starting points are 1k, 100k and 1m spatial observations; 10k and 100k vectors at dimensions 128/768/1536; and larger sets only after a memory estimate and explicit resource budget. No test must allocate a 10m-vector corpus merely because the roadmap lists that scale. Dataset counts and vector dimensions are independent limits: preserve source-defined vector-dimension bounds until a separate extension is approved.

Spatial distributions include uniform, clustered, coincident, anisotropic, corridor/plane, moving-point and hot-cell workloads. Vector/graph workloads include sparse and dense permission scopes, filter selectivity of 100%, 10%, 1%, and 0.1%, high and low update rates, stale-index repair, high-degree graph vertices, contradictions, quarantines, and mixed embedding spaces that must be rejected rather than compared.

Measure correctness/quality separately from speed: closed-predicate mismatch count, candidate recall, authorized Recall@1/10/100, NDCG on a labeled corpus, vector numeric error, explicit partial rate, conflict exposure, recovery integrity, projection neighborhood preservation, and scope leakage. Measure p50/p95/p99 latency, throughput, peak resident/device/staging memory, persisted bytes, write amplification, build/rebuild/recovery duration, per-query network bytes, queue delay, model cold-start and full host-device-host cost.

Freeze promotion thresholds before running the candidate set. Initial proposed defaults: zero exact-geometry mismatches; no signed-byte or causal-conflict loss; no unauthorized result materialization; ANN Recall@10 at least 0.95 on the named authorized corpus and no more than 0.01 absolute degradation from the accepted baseline; investigate reproducible end-to-end latency regressions above 10%. These are reviewable starting policies, not universal service guarantees or measured achievements. A workload-specific policy may revise them before candidate evaluation. Spatial/crypto/authorization correctness cannot be traded for a speed claim.

Compare to both the unchanged accepted implementation and an optimized CPU baseline at equivalent recall/precision. Include transfers, batch formation, verification and fallback overhead. Do not compare a warmed GPU with a cold scalar-only CPU and omit that asymmetry. Do not average p99 values across machines; aggregate raw samples or a documented compatible histogram method. Randomize baseline/candidate order or use paired runs, record thermal/power/background-load conditions, warmup and repetition counts, and keep raw samples. Report an uncertainty interval supported by the sampling method; do not imply a particular confidence bound from five anecdotal timings.

### 8.7 Machine-readable evidence formats

The implementation packet must instantiate these field contracts with actual values. Nulls mean absent evidence and must never be rendered as success. The following JSON is a **schema-shaped example**, not a completed run:

```json
{
  "schema": "wdbx.plan-task-evidence.v1",
  "task_id": "T06",
  "status": "not_run",
  "source": {"repository": "donaldfilimon/wdbx", "commit": null, "dirty_diff_sha256": null},
  "build": {"rustc_verbose": null, "target": null, "lockfile_sha256": null, "features": []},
  "execution_authorization": null,
  "commands": [],
  "cases_expected": [],
  "cases_executed": 0,
  "red_artifacts": [],
  "green_artifacts": [],
  "compatibility_artifacts": [],
  "qualification_keys": [],
  "review": {"status": "not_reviewed", "reviewer": null, "artifact": null},
  "claim_changes": [],
  "rollback": {"documented": true, "executed": false}
}
```

`BenchmarkManifest` additionally requires baseline/candidate commits, binary/lockfile hashes, complete argv/environment allowlist, dataset digest/generator/seed/license, model/encoder/quantizer digests, dimensions/counts/distributions, principal/cohort/filter definition, read-view and shard map, hardware/OS/runtime/driver, numeric/candidate profiles, memory/time limits, warmup/sample protocol, frozen thresholds and raw-artifact digests. Never put credentials or sensitive payloads in the environment capture.

`QualificationRecord` requires the exact operation/model/backend/device/driver/build/dtype/shape key, runtime-execution artifact, numerical corpus/results, placement evidence or `unknown`, benchmark evidence or `not_run`, expiry/invalidation rule, and reviewer. Existing `CapabilityEvidence` is copied as separate source facts. Changing any key field invalidates reliance on the old qualification until the declared requalification rule passes.

### 8.8 Acceptance gate by release profile

| Profile | Mandatory core evidence | Conditional evidence | Explicitly not implied |
|---|---|---|---|
| spatial-checked-v1 | T00–T06; selected T55/T56/T57 checks | Supported-target execution | Durable frames, new APIs, NPU, cluster, full specimen conformance |
| spatial-durable-v1 | T00–T16; recovery/authorization/compatibility | Selected storage-platform durability proof | Global v2/v3 atomic transactions |
| hybrid-evidence-v1 | Durable profile plus T17–T24 | Lexical/model-specific quality reports | ANN is exact or evidence dimensions prove truth |
| explorer-v1 | Authorized query/projection API and T26/T28/T29 | T27 only for enabled nonlinear projection | Browser/native parity without separate tests |
| accelerated-backend-v1 | T30–T33 and the selected adapter | Real device/provider/model tests | Other vendors, speedup, or NPU placement |
| cluster-causal-v1 | T39–T45 with actual host/fault manifest | Any enabled device profile | Global serializability or Byzantine tolerance |
| specimen-compatible-runtime-v1 | All original requirements mapped and tested in T46–T53 | T44 only for distributed scans; adapter tests only when used | Erasure approval or parity on another runtime |
| production-profile-v1 | Relevant profile evidence plus T55–T57 and explicit approval | T54 only when an erasure/custody claim is made | General production readiness beyond advertised scope |

A profile with critical shared integrity or authorization defects cannot be released by renaming it “experimental.” Research features may remain available only with clear separation and limitations that do not undermine the production profile's boundary.

<a id="traceability"></a>
## 9. Original Neural Specimen traceability

This section replaces the previous generic “map to tests” checklist with concrete, named acceptance cases. It preserves the original 26 chapters and all R01–R23/I01–I24 without claiming that the integrated runtime already satisfies them. The original complete source remains controlling; T00 must pin/read it before implementation. These rows are **additional Step 1 acceptance cases for their owning task**, not a second optional checklist. Test discovery and evidence must name the case, fixture, runtime, profile and executed outcome.

**State of every row in this document: specified; not executed.** A browser result is not a native result; a local result is not a distributed result. Original editorial resolutions and open decisions remain visible. No new exact-equality codec, redundancy predicate, grave policy or attention design is approved simply by naming its test.

### 9.1 All 26 chapters

| Chapter | Preserved authority | Task | Required named case and assertion |
|---|---|---|---|
| SPEC-01 | Authority and interpretation | T00 | `chapter01_authority_and_later_clarifications` — Source lock includes the complete original, later explicit decisions, R01–R23, and a conflict register; implementation notes cannot silently amend the design. |
| SPEC-02 | Architecture and responsibility boundaries | T46 | `chapter02_address_retrieval_reasoning_separate` — Addressing retrieves candidates; nodes supply votes; the orchestrator composes; storage or a worker cannot authorize external actions. |
| SPEC-03 | Terminology and identifiers | T46 | `chapter03_duplicate_public_ids_stable_refs` — Two compatible nodes can share a logical ID while unique internal references preserve each node and contributor. |
| SPEC-04 | Invariants | T00 | `chapter04_every_invariant_has_named_case` — All I01–I24 rows resolve to executable cases for the selected runtime, with no generic coverage checkbox substituting for execution. |
| SPEC-05 | Logical data models | T46 | `chapter05_entry_and_alternative_slot_are_distinct` — A pattern entry reference differs from a weighted alternative slot reference; donor lineage and output provenance resolve the correct object. |
| SPEC-06 | HybridTable and storage organization | T46 | `chapter06_hybridtable_physical_layout_preserves_logic` — Changing array/hash/index representation preserves scoped equality, all-match behavior, original content, and stable internal references. |
| SPEC-07 | Language Governance and input preparation | T47 | `chapter07_original_and_normalized_input_separate` — Retain original bytes separately from normalized variants, triples and context constraints; fan-out is bounded and encoder-versioned. |
| SPEC-08 | Sigil scopes and action scripting | T47 | `chapter08_reserved_sigil_cannot_be_shadowed` — User text cannot shadow a reserved capability or obtain shell/network/filesystem access by interpolation; fuel and scope are enforced. |
| SPEC-09 | Pattern-node activation and votes | T48 | `chapter09_every_qualified_entry_can_vote` — Every independently qualified entry may vote; coarse key equality supplies no confidence and cannot bypass high-resolution gating. |
| SPEC-10 | Node attachments and automatic wiring | T48 | `chapter10_hard_attachment_schedules_unmatched_remainder` — The target receives the correct unmatched remainder, consumes scan credits, and may still decline to vote. |
| SPEC-11 | Cycle lifecycle and orchestration | T49 | `chapter11_vote_barrier_and_commit_outcome` — Orchestration consumes a completed bounded vote set; cancellation uses the explicit before/after-commit outcome contract. |
| SPEC-12 | Pattern-ID-guided supporting retrieval | T49 | `chapter12_two_support_channels_no_node_gate` — Generated pattern_key and optional resource_id independently retrieve candidates; contextual support filtering does not run a node confidence gate. |
| SPEC-13 | Transient Memory and imagination | T50 | `chapter13_synthesis_is_not_persistent_learning` — Synthesis/imagination may allocate bounded scratch but never trains weights, writes learned facts, or self-approves knowledge. |
| SPEC-14 | Action Tone Predictor and stochastic behavior | T50 | `chapter14_atp_baseline_and_jitter_are_separate` — Eligible runtime modulation respects endpoint/crystallization exceptions; repeated reads do not accumulate jitter into baseline. |
| SPEC-15 | Conversation memory and temporal coherence | T50 | `chapter15_rolling_and_pinned_history_preserved` — Rolling retention does not silently remove pinned records; summaries keep scope, parent provenance and invalidation rules. |
| SPEC-16 | Visual perception and ephemeral fuzzy art | T50 | `chapter16_visual_arrays_are_not_implicit_physical_geometry` — Eye lifecycle, OCR/text references and original visual arrays survive; fuzzy art stays ephemeral unless saved and nonlinear SDF remains its qualified original label. |
| SPEC-17 | Non-pattern nodes | T51 | `chapter17_type_a_and_type_b_write_authorities` — Type A residual contributions remain distinct from Type B permitted internal edits; new Type B knowledge needs an identified /learn approval. |
| SPEC-18 | Contributor provenance and feedback | T49 | `chapter18_provenance_exists_at_composition` — Every composed span carries actual source votes/resources/transformations; contributor identity is not guessed after generation. |
| SPEC-19 | Growth, idle mutation, and PHAGY | T51 | `chapter19_idle_mutation_and_phagy_are_bounded` — Eligible idle intervals, lifetime remix markers, donor lineage, growth inhibition and PHAGY timeouts survive save/restore and retries. |
| SPEC-20 | Index Rafts, chunking, and scheduling | T52 | `chapter20_traversal_is_complete_and_yielding` — Required ranges are disjoint and complete from zero; find_all continues past the first match; physical chunks are not threads or consensus Raft. |
| SPEC-21 | Compartmental contact ledger | T52 | `chapter21_contacts_do_not_join_by_name` — An identical display name in another compartment supplies no permission to read, join, or export that contact. |
| SPEC-22 | Commands | T52 | `chapter22_original_aliases_route_unchanged` — /prompt, /right, /wrong, /wrng, /contributorfractal, /saveSpecimen, /loadSpecimen, /addPattern, /learn keep original routing and visible approval/editor behavior. |
| SPEC-23 | Configuration and limits | T52 | `chapter23_hard_limits_dominate_adaptive_work` — User ceilings constrain ordinary scans, attachments, brainstorming, urgent work, automata and maintenance; rejected work is explicit. |
| SPEC-24 | Persistence and restoration | T53 | `chapter24_entire_durable_specimen_roundtrips` — All durable organs and coherent modulation definitions round-trip; live activations, handles, threads and secret keys do not enter default exports. |
| SPEC-25 | Implementation decisions and acceptance criteria | T53 | `chapter25_open_design_is_not_conformance` — An unresolved original equality/grave/attention behavior remains a named gated decision, not a fabricated pass or silently selected replacement. |
| SPEC-26 | Source coverage map | T00 | `chapter26_coverage_is_per_runtime_and_evidence` — Coverage names original section, case, fixture, runtime/profile, source, run artifact and outcome; no browser/native/device evidence is inferred. |

### 9.2 Normalization and contradiction register R01–R23

| Original rule | Preserved resolution | Task | Required named case and assertion |
|---|---|---|---|
| R01 | Duplicate IDs are legal | T46 | `r01_duplicate_node_ids_return_all` — Public Pattern/node IDs may repeat; internal references distinguish records. |
| R02 | Logical node versus physical table | T46 | `r02_bucket_contains_multiple_node_tables` — HybridTable may physically co-locate multiple logical nodes in one Pattern-ID bucket. |
| R03 | Twenty is per-node capacity | T46 | `r03_twenty_is_not_population_or_result_cap` — Two 20-entry nodes sharing an ID are legal within global budgets, and lookup returns both. |
| R04 | Exact duplicates versus paraphrases | T46 | `r04_content_equality_after_hash_selection` — A hash collision does not prove a duplicate; original exact-repeat rules reject duplicates while compatible paraphrases remain legal under the approved equality codec. |
| R05 | Two resource identifiers | T49 | `r05_generated_key_and_optional_resource_link` — Resource pattern_key is mandatory and generated; resource_id is optional, non-unique, and links a compatible unprefixed node ID. |
| R06 | Independent voting versus arbitration | T48 | `r06_only_redundant_same_triple_votes_arbitrate` — Approved same-origin redundancy groups may arbitrate; complementary contributions survive. An unspecified redundancy predicate is a gate, not a free choice. |
| R07 | Hard attachments do not force votes | T48 | `r07_hard_attachment_target_may_decline` — Guaranteed scheduling of an unmatched remainder does not override the target gate. |
| R08 | Transient Memory is non-learning | T50 | `r08_transient_inference_leaves_weights_unchanged` — No online weight training or learned knowledge follows from temporary sparse/MLP synthesis; optional attention needs its own design. |
| R09 | Jitter exclusions | T50 | `r09_ids_permissions_and_facts_never_jitter` — Only eligible temporary numeric values vary; stable IDs, structure, permissions, caps and signed facts remain unchanged. |
| R10 | Selection weight is not confidence | T48 | `r10_sampling_weights_are_finite_nonnegative` — Confidence remains in [-100,100]; confidence × strength is converted by an explicit versioned sampling rule, never used as a negative probability. |
| R11 | Idle interval interpretation | T51 | `r11_idle_default_stays_between_60_and_120` — Fresh default intervals lie in [60,120] seconds of eligible idle time; a 3-second profile is separate. Distribution and pause/reset semantics are explicitly recorded. |
| R12 | History count editorial interpretation | T50 | `r12_history_100000_is_profile_default` — 100,000 records remains the inherited editorial default with bounded allocation; an amendment does not silently reinterpret stored history. |
| R13 | First-hit versus all-match traversal | T52 | `r13_id_lookup_uses_find_all` — Compatible duplicate-ID lookup continues to all matching nodes/resources; find_first remains a different operation. |
| R14 | Rafts are not free scans | T52 | `r14_raft_work_is_accounted` — Every traversed range and continuation consumes measured work/budget; yielding does not claim reduced total asymptotic scan work. |
| R15 | Macros have execution cost | T47 | `r15_macro_cost_consumes_fuel` — Reusable Sigil procedures consume fuel/time/memory according to their actual operations. |
| R16 | Nonlinear SDF label qualification | T50 | `r16_pixel_payload_does_not_claim_signed_distance` — Original aligned visual arrays retain fields/label without pretending they encode a true geometric signed-distance function. |
| R17 | Type B internal edits versus knowledge | T51 | `r17_internal_edit_does_not_approve_new_knowledge` — Permitted operating-state edits cannot promote a new inferred pattern without /learn <proposal_id>. |
| R18 | Durable save versus live execution | T53 | `r18_save_excludes_live_execution_handles` — Save durable organs, definitions and coherent modulation state; do not serialize threads, GPU handles, scratch activations or executable continuations by default. |
| R19 | Pool shortening and remix lifetime | T51 | `r19_pool_truncation_preserves_remix_markers` — Align only min(recipient,donor) slots, never grow the pool, and reject truncation that erases a remixed slot without an approved tombstone rule. |
| R20 | Zero strength and grave uncertainty | T51 | `r20_zero_strength_preserves_inhibition_history` — Remove active node/index entries at zero strength while preserving inhibition/mutation history; grave retention/resurrection is blocked until its rule is approved. |
| R21 | Wildcard candidate route | T47 | `r21_wildcard_route_does_not_bypass_limits` — Bind-rule candidates can use a separate route when literal IDs cannot cover them; limits, inhibition and permissions still apply. |
| R22 | Compatible identity namespaces | T46 | `r22_bare_string_equality_is_insufficient` — Equality requires modality, resolution class and encoder version as well as the Pattern-ID string. |
| R23 | Permit counts comparisons, not nodes | T52 | `r23_multientry_node_reserves_per_comparison` — A node with 20 simultaneously compared entries uses 20 permits; ordinary, attachment and remote work share the specimen cap. |

### 9.3 Original invariants I01–I24

| Original invariant | Preservation obligation | Task | Required named case and assertion |
|---|---|---|---|
| I01 | Durable organs use HybridTable | T46 | `i01_durable_organs_have_explicit_partitions` — Every durable specimen organ uses the declared HybridTable-managed boundary. |
| I02 | Duplicate IDs return all matches | T46 | `i02_all_matching_public_ids_are_observable` — Compatible duplicate node/resource IDs are legal and are not dropped by retrieval. |
| I03 | Original pattern representations retained | T46 | `i03_original_pattern_survives_reindex` — A new encoder or index does not replace the original text or visual representation. |
| I04 | Entry key matches containing node | T46 | `i04_entry_key_and_capacity_validated` — Each entry regenerates the containing node key; default per-node capacity is 20. |
| I05 | Content rather than hash-only duplicates | T46 | `i05_hash_collision_is_not_structural_duplicate` — Compare actual approved canonical structure after hash selection; preserve permitted paraphrases. |
| I06 | ID equality is only a candidate signal | T48 | `i06_matching_id_does_not_create_confidence` — A key match by itself neither computes confidence nor qualifies applicability. |
| I07 | High-resolution verification and gate | T48 | `i07_low_resolution_lookup_still_deep_scans` — Ordinary votes require high-resolution verification and the configured gate; bind overrides are explicit. |
| I08 | Bounded valid confidence | T48 | `i08_confidence_clamps_without_invalid_arithmetic` — Confidence stays within [-100,100] and rejects non-finite/overflow behavior. |
| I09 | Temporary nonaccumulating jitter | T50 | `i09_endpoint_and_nonaccumulation_rules` — Exact ±100 endpoints do not jitter; repeated reads preserve baseline. |
| I10 | Crystallization exceptions | T50 | `i10_strength_loss_removes_jitter_protection` — At maximum strength baseline-qualified confidences do not jitter; lower confidences remain eligible; protection disappears below maximum. |
| I11 | Multiple independent entry votes | T48 | `i11_one_node_may_contribute_multiple_votes` — All independently qualified entries may contribute rather than selecting one node-wide winner. |
| I12 | Composition belongs to orchestrator | T49 | `i12_node_stops_after_vote_submission` — Node-local reasoning ends at immutable vote submission; composition occurs beyond the phase barrier. |
| I13 | Attachment-group arbitration boundary | T48 | `i13_attachment_groups_excluded_from_ordinary_arbitration` — Attachment groups are not discarded by ordinary same-triple duplicate arbitration; targets see the correct remainder. |
| I14 | One specimen-wide scan ceiling | T52 | `i14_global_1000_comparisons_never_silently_drop` — Default 1,000 simultaneous comparisons includes attachments; queued candidates wait or explicitly terminate. Distributed mode additionally proves T44. |
| I15 | Support retrieval has no node-style deep gate | T49 | `i15_support_context_filter_is_not_node_confidence` — Support lookup uses equality candidates then contextual relevance, not an extra node confidence scan. |
| I16 | Transient synthesis is not learned knowledge | T50 | `i16_transient_output_has_no_automatic_durable_authority` — Temporary synthesis does not train online or create an authoritative learned memory. |
| I17 | Provenance constructed during composition | T49 | `i17_span_provenance_is_constructed_not_guessed` — Source vote/resource/transform links exist as spans are composed. |
| I18 | Feedback independent per contributor | T51 | `i18_feedback_draws_are_independent_and_subset_scoped` — Each affected contributing node receives its own coin flip; color/identity targeting only selects the subset; retries do not draw again. |
| I19 | Remix lifetime and lineage monotonicity | T51 | `i19_slot_remixes_once_across_save_and_restore` — Each recipient slot remixes at most once; donor relations and original-pairing inhibition survive normal edits and restoration. |
| I20 | Type B learning approval | T51 | `i20_learn_requires_identified_proposal_approval` — New inferred knowledge enters persistent learned structures only through the original identified learning gate. |
| I21 | Names do not grant authority | T47 | `i21_scope_authority_independent_of_textual_name` — Sigil/contact permissions are enforced independently of equal textual names; contact cases are also exercised in T52. |
| I22 | Complete disjoint yielding rafts | T52 | `i22_ranges_start_zero_and_yield` — Ranges cover required traversal without overlap, the first begins at zero, and every continuation yields. |
| I23 | Hard ceilings dominate adaptation | T52 | `i23_adaptation_cannot_raise_user_hard_cap` — Adaptive scheduling, urgency, brainstorming, automata and maintenance cannot exceed the configured hard resource ceilings. |
| I24 | Whole persistent specimen save | T53 | `i24_complete_persistent_specimen_roundtrip` — Save/restore covers every durable organ and defined modulation state, not only node tables. |

### 9.4 Coverage bookkeeping without false completeness

For each row, the implementation evidence stores `requirement_id`, original section/ref/blob, applicable runtime, task ID, exact case/fixture, expected assertion, actual source/build identity, run artifact, result, review and any approved amendment. A gap remains a gap until the right runtime executes the right behavior. An original open rule is `blocked_on_named_decision`; it is not “not applicable” merely because no implementation exists.

Additional full-system preservation obligations remain independently tracked: v1 read-only access and migration; v2 causal journals/conflicts, UUID/legacy identity, snapshot views and exact replication; v3 commitments/signatures/consent-policy admission and memory edges; HNSW/exact search; temporal/causal/persona ranking; octree/legacy spatial metrics; multiway events/resume/publication; PQ/autoencoder/int8/Huffman/rANS codec profiles; authenticated-object/key-provider boundaries; optional DGHV/additive HE/TFHE research demonstrations; CLI/MCP/REST/gRPC/WebSocket contracts; and native/browser specimen formats. T00/T02 pin their actual source/fixtures and route gaps to the owning workstream. Retaining a name or compile feature is not behavior preservation evidence.

<a id="handoff"></a>
## 10. Execution handoff and stop rules

### 10.1 First implementation packet

The first packet is **M0 plus M1: T00–T06**, not the entire 58-task roadmap. Its product deliverable is a validated, deterministic checked query path around the existing point index, with an independent oracle and a qualified existing-octree fast path. It changes no signed record format, adds no production gateway, requires no GPU/NPU/cluster, and does not migrate live data.

T00–T02 establish the actual current source and compatibility evidence. T03–T04 fix the numeric input/profile contract and independent reference. T05 adds the checked access surface without silently changing old methods. T06 may use the old octree topology only after its pruning, invalidation and result ordering agree with the declared profile. If an optimization remains unqualified, the complete reference path is a valid bounded fallback; it is not permission to claim an optimized index.

The packet handoff must include the exact source manifest, proposed diff, case list, observed red/green results where applicable, preserved compatibility results, source/lockfile/toolchain metadata, review, known limitations and rollback. None of those implementation results are supplied by this planning revision.

### 10.2 Worker and reviewer boundaries

Use independent implementer/reviewer contexts when the execution environment actually offers them. Otherwise execute inline and record that no independent review occurred until a real reviewer is available. A role name in a prompt is not another reviewer. Do not change machine-level agent settings, install plugins, or create external projects to simulate unavailable capabilities without user authorization.

Parallel tasks are limited to genuinely independent files/interfaces after their shared contract is accepted. One owner controls each canonical format, ABI boundary, spatial numeric profile, policy schema and source manifest. A concurrency conflict is resolved through reviewed changes, not by silently overwriting another worker's edits. Worktree creation, branch choices and commits require the existing user constraints to be respected.

A reviewer checks behavior against the original authority, the new contract, adverse inputs and the evidence—not only code style. Missing hardware, unresolved original decisions, unknown source revisions, environmental test failures and absent raw artifacts stay visible. Review cannot promote a compile-only result into numerical or production qualification.

### 10.3 Hard stops

Stop the affected implementation/release packet on signed-byte or digest mutation; legacy/UUID identity loss; hidden causal conflicts; a non-conservative exact-search prune; stale-index activation without complete coverage; unauthorized materialization; a silently changed model/backend contract; a false cancellation outcome; reissued zombie-worker credits under the strict cap; a silent partial cross-shard result; old-writer data loss; unapproved learning or erasure; or falsified test/benchmark/qualification status.

Stop a shared-core release when the defect crosses its advertised integrity/security boundary. A missing optional vendor does not stop unrelated local work, but that vendor cannot be advertised. An unresolved original specimen rule prevents full conformance for its runtime; it does not permit discarding the rule.

### 10.4 Copy-paste implementation instruction

> Continue the existing WDBX project from its verified current source, using this revision-5 plan and the complete original specifications. First execute only the approved T00–T06 packet. Treat Rust 1.99.1 adoption as already present at the recorded commit, with compile-only historical evidence; qualify the current checkout and consumers rather than repeating the migration. Preserve the original crate/API names, signed bytes, v1/v2/v3 compatibility, causal conflicts, HNSW, existing octree, multiway and all specimen semantics. Use separate coverage and numeric oracles; define new checked behavior without silently changing legacy methods. Record exact source/lockfile/toolchain identities, nonzero test counts, raw red/green results, review and limitations. Do not call an unavailable device tested or a successful calculation independently qualified. Do not proceed automatically to hardware, cross-node, gateway, erasure or production work. Deliver a reviewable diff and evidence at the packet boundary. No commit, push, deployment, credential change, destructive action or live-store modification is authorized without the required explicit approval.

### 10.5 Review of the plan itself

The document-authoring checks verify that tasks are uniquely numbered; unconditional dependencies refer to existing tasks and form an acyclic graph; all 26 chapter rows, R01–R23 and I01–I24 exist; the task/traceability references resolve; links/fences/JSON examples are well formed; original artifact bytes remain unchanged; and implementation checkboxes remain unchecked. These checks do **not** compile the proposed interfaces, run WDBX tests, validate cryptography, prove distributed semantics or certify completeness of the original source interpretation. Those remain implementation and review obligations.

<a id="sources"></a>
## 11. Source and evidence register

Source identity has two jobs: preserve the user's original authority and distinguish fresh code observations from inherited claims. Mutable branch URLs below are discovery links only; T00 must pin their actual revisions/blobs before dependent implementation. No source statement is promoted because it was copied into this plan.

### 11.1 Sources inspected for this revision

**[SRC01] Current WDBX source commit.** `08a69b3f22db61093e06527b3308b10589687022`, dated 2026-10-19T20:11:00Z. The commit message records adoption of stable Rust 1.99.1, safe fixed-width lane accumulation, a successful locked workspace/all-target check, and no runtime/performance qualification because tests/CI were paused. This is reported historical execution evidence, not an execution performed during this revision.

https://github.com/donaldfilimon/wdbx/commit/08a69b3f22db61093e06527b3308b10589687022

**[SRC02] CPU implementation, fully read at SRC01.** `crates/abi-compute/src/cpu.rs`; blob `15fc17f5f291ec281c32ea62f0e06fdeb13f38e0`. Safe fixed-width f32 lanes and ordered reduction; existing public capability reports successful execution as runtime-verified. Independent operation-level qualification and emitted instruction evidence are not established merely by those code paths.

https://github.com/donaldfilimon/wdbx/blob/08a69b3f22db61093e06527b3308b10589687022/crates/abi-compute/src/cpu.rs

**[SRC03] Capability implementation, lines 1–300 read at SRC01.** `crates/abi-compute/src/backend.rs`; blob `2336c6dda9a605e1de288e00291970c2e1344e83`. Existing identities and the five facts `compiled`, `available`, `initialized`, `executed`, `runtime_verified` are compatibility contracts. This revision does not replace them with a new ladder.

https://github.com/donaldfilimon/wdbx/blob/08a69b3f22db61093e06527b3308b10589687022/crates/abi-compute/src/backend.rs

**[SRC04] Spatial implementation, lines 205–430 re-read at SRC01.** `crates/abi-wdbx/src/spatial.rs`; blob `958314753c0608b2356131437c48bd24984767fd`. The inspected code includes existing octree traversal, distance-only sorting, raw u32 insertion and query methods. The inherited full-source review supplies additional context; this revision does not claim to have rerun its tests or read the entire repository.

https://github.com/donaldfilimon/wdbx/blob/08a69b3f22db61093e06527b3308b10589687022/crates/abi-wdbx/src/spatial.rs

**[SRC05] Original massive master plan.** The full 1,016-line conversation artifact was read before revision and retained unchanged. File: `WDBX-3D-Neural-Specimen-MASSIVE-Master-Plan.md`; bytes: 125,344; SHA-256: `3cf8200fb9cb346a9b89aa5b329507f594411b37f4aad5510334ca4b52db340b`. Its Volume II was retained and reconciled as Appendix A. Its duplicated plan/playbook material was consolidated into the task packets and verification protocol; it remains recoverable in the original artifact.

### 11.2 Controlling/inherited project authorities to pin in T00

The following authorities were specified by the user and/or reviewed in the preceding work. Their content was not all freshly re-fetched in this revision. Original semantics are preserved as planning requirements; current implementation status must be reconciled at an immutable source revision before code changes or conformance claims.

| Source | Authority and use | Discovery path |
|---|---|---|
| SRC06 | North-star capability inventory and claims boundaries | `ABI/docs/spec/wdbx-north-star.mdx` |
| SRC07 | Multiway state/event distinction, deterministic bounded simulation and persistence limitations | `ABI/docs/spec/wdbx-multiway.mdx` |
| SRC08 | ABI control-plane/WDBX memory-plane ownership and consumer contracts | `ABI/docs/spec/agent-wdbx-architecture.mdx` |
| SRC09 | Canonical episodes, evidence dimensions and constitutional mapping | `ABI/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md` |
| SRC10 | Evidence-half priorities and ordered completion work | `ABI/docs/superpowers/specs/2026-09-29-wdbx-completion-design.md` |
| SRC11 | Draft encrypted-member custody/erasure, exact crypto choices, holds and separate approval | `ABI/docs/superpowers/specs/2026-09-29-spec-member-erasure.md` |
| SRC12 | Existing structural/evidence claim boundary; not proof of the current test run | `WDBX/README.md`, `WDBX/docs/claims.md` |
| SRC13 | Full original 26 chapters, R01–R23, I01–I24 and native-profile additions | `STUDIO/public/WDBX-Specimen-Architecture-Specification.md` |
| SRC14 | Runtime/platform qualification and browser/native distinctions | `STUDIO/RUNTIME-PROFILE.md`, `STUDIO/VALIDATION.md`, actual workspace manifests |

ABI discovery root: https://github.com/donaldfilimon/abi

WDBX discovery root: https://github.com/donaldfilimon/wdbx

Original specimen discovery document: https://github.com/donaldfilimon/wdbx-specimen-studio/blob/main/public/WDBX-Specimen-Architecture-Specification.md

The specimen repository's archive/current-workspace status must be resolved from the connected source before editing. A historical discovery link is not permission to modify an archived repository or recreate an application elsewhere.

### 11.3 Official technical references checked during revision

**[EXT01] Rust 1.99.1 release, October 15, 2026.** Confirms the requested toolchain is a released stable version; it does not certify WDBX compatibility.

https://blog.rust-lang.org/2026/10/15/Rust-1.99.1/

**[EXT02] Rust standard-library `Simd` documentation.** The checked 1.99.1 documentation identifies the portable-SIMD API as experimental/nightly-only. Keep this separate from stable source code that the compiler may auto-vectorize.

https://doc.rust-lang.org/std/simd/struct.Simd.html

**[EXT03] Tokio `spawn_blocking` documentation.** Started blocking tasks cannot generally be aborted by aborting their async handle. This supports explicit cooperative cancellation, lifetime accounting and the rejection of a promise that cancellation always stops work immediately.

https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html

**[EXT04] Apple Core ML compute-unit policy.** A CPU-and-Neural-Engine policy permits those compute units. Request configuration is not a per-operation placement trace or proof of ANE residency.

https://developer.apple.com/documentation/coreml/mlcomputeunits/cpuandneuralengine

**[EXT05] Microsoft Windows ML execution-provider documentation.** Provider identity and actual hardware must be recorded; DirectML is a GPU execution provider, not generic NPU evidence. Other provider/model/device support must be verified for the selected implementation profile.

https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/supported-execution-providers

https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/distributing-your-app

The geometric precision bounds, proposed task APIs, error states, scope rules, exactness profiles, credit protocol, benchmark thresholds and release decomposition are **proposed engineering decisions in this plan**, not statements that these references implement or endorse WDBX.

---

<a id="architecture-catalogue"></a>
## Appendix A — Retained integrated architecture catalogue

**Origin:** Volume II of revision 4.0, incorporating the earlier integrated database specification. **Reconciled for revision 5.0:** conflicting precision, migration, capability, cancellation, consistency, lease, projection, API, CI, and rollout passages have been edited rather than left as competing instructions.

This appendix preserves the broad feature catalogue and original design intent; sections 1–10 of the main plan define the single implementation order, task interfaces, proof obligations and profile gates. The original 26-chapter specimen authority remains separately controlling for its behavior. This appendix is not a second master plan, a claim of implementation, or permission to skip task review.

**Scope:** local-first storage, spatial/vector/graph/evidence retrieval, original specimen behavior, stable CPU, optional GPU/NPU/TPU and remote execution, interoperability, operations and research horizons. Features not in the immediate spatial profile remain visible and gated rather than silently removed. Newly proposed fields, algorithms, adapters, standards and dependencies require the named task/ADR.

Historical feature-status statements inherited from the source documents remain reported at their source revision, not freshly executed proof. The narrow current-source inspection and limitations are in section 2. No repository, product runtime or deployment was changed by this document revision.

### A.0. Authority, claims, and profiles

**MUST** means required for a named conformance profile; **SHOULD** requires documented exception; **MAY** is optional. **Current** means source-and-test-backed at an identified revision; **Partial** means bounded proof; **Proposed** means not verified as implemented; **Research** must never be sold as production protection. A feature flag or detected device does not upgrade evidence. ABI's ratified constitution and privacy/authorization rules prevail; the original specimen governs specimen semantics; existing versioned on-disk formats and signed commitments govern compatibility; this document governs additive new capabilities.

The named, independently qualified profiles are defined once in main sections 3 and 8.8. They separate checked spatial, durable spatial, hybrid/evidence, explorer, each hardware adapter, causal cluster, each specimen runtime, and a scoped production release. The former P0–P4 ladder is retired because it incorrectly made unrelated features look like prerequisites for one another.

Claim levels C0–C7 are **separate** from release profiles. Reviewed source documents support v2 causal journals/MVCC/WAL, HNSW and exact search, temporal-causal-persona retrieval, a local 3D spatial index, v3 canonical CBOR episodes with detached Ed25519 signatures, bounded multiway rewriting, and single-host multi-process demonstrations. They do **not** establish audited production FHE, actual NPU residency, production multi-host deployment, measured acceleration, or complete C3–C7 evidence. The browser/native Specimen Studio is also not a production ABI deployment.

### A.1. Mission and architectural ownership

The proposed integrated WDBX Neural Specimen architecture is a **local-first, multimodal, spatial, vector, graph, episodic and provenance-aware database** connected to a separately bounded cognitive specimen execution engine. WDBX owns canonical evidence, storage, indexing, retrieval and versioned compute artifacts. The specimen engine owns pattern activation, independent votes, transient orchestration, feedback, and approved growth. ABI owns identity, authorization, policy, model/tool orchestration and consequential action approval. Abbey/Aviva/ABI are authorized clients/scopes; Studio is a client, not a hidden second authority.

The system must answer: *what happened; what was predicted; what action was taken; what followed; which records were consulted; why should this record be trusted; which hardware performed the computation; and which scope authorized it?* It must never infer that embeddings prove truth, projection proximity proves meaning, or signed data proves correctness.

```text
User / Studio / CLI / MCP / REST / gRPC / SDK
                      |
       ABI identity + policy + approval arbiter
                      |
   WDBX control plane: schema | query planner | budgets
     authorization | provenance | leases | scheduling
                      |
  +-------------------+------------------+-------------------+
  | CANONICAL STORE   | DERIVED INDEXES  | SPECIMEN RUNTIME  |
  | v1 reader, v2 DAG | exact/HNSW, text  | HybridTable, nodes |
  | WAL, v3 episodes  | spatial 3D, graph | Sigils, votes, ATP |
  | signed objects    | temporal, 3D view | PHAGY, Transient   |
  +-------------------+------------------+-------------------+
                      |
       CPU scalar/SIMD | GPU | NPU | optional TPU
                      |
        authenticated cross-node data/compute fabric
```

### A.2. Preservation matrix: all 26 original chapters

This is a traceability index, **not a replacement for the original chapter text**. Every chapter remains normative.

| Original | Preserve exactly in behavior | New integration |
|---|---|---|
| 1 Authority | R01–R23 contradiction resolutions | ADR/claims registry |
| 2 Boundaries | Address ≠ retrieval ≠ reasoning; organ ownership | Separate data/control/compute planes |
| 3 IDs | Duplicate logical IDs, stable internal refs, low/medium/high keys | Tenant-qualified IDs and embedding spaces |
| 4 Invariants | I01–I24 and all exceptions | Machine-checkable distributed invariants |
| 5 Models | Specimen, pattern/non-pattern nodes, entries, votes, resources | Typed, versioned, append-linked records |
| 6 HybridTable | Organ tables, array/hash choices, exact equality | Transactional secondary indexes |
| 7 Language | Normalization, variants, triples, constraints | Versioned preprocessors |
| 8 Sigils | Reserved/user scopes, functors, automata, scripting | Sandboxed capability DSL |
| 9 Votes | Coarse lookup, high-res scan, independent gated votes | CPU/GPU batch candidates, CPU oracle gates |
| 10 Attachments | Soft/hard, unmatched remainder, grouping | Versioned graph edges and bounded frontier |
| 11 Cycles | Independent voting, ephemeral orchestrator, anti-staleness | Snapshot-pinned replay trace |
| 12 Support | Pattern key and optional resource ID, two retrieval channels | Hybrid candidate routing |
| 13 Transient | Non-learning sparse synthesis and imagination | Ephemeral accelerator scratch |
| 14 ATP | Tone, cooldown, jitter, brainstorm, stochastic effects | Captured seed and bounded evolution |
| 15 Conversation | Rolling/pinned history, temporal coherence | Retention-aware episodes |
| 16 Vision | Eye lifecycle, visual arrays, nonlinear SDF label, fuzzy art | Image embeddings, spatial assets |
| 17 Non-pattern | Type A residual, Type B internal edits and learning proposals | Write-scope separation |
| 18 Provenance | Output-span contributors and targeted feedback | Signed trace references |
| 19 Growth | Idle mutation, one-time remix, PHAGY, grave policy | Append-only lineage and budgets |
| 20 Rafts | Disjoint complete ranges, yielding, global 1,000 scan cap | Distributed scheduler with exact permits |
| 21 Contacts | Compartmental ledger and independent permissions | Encrypted scope isolation |
| 22 Commands | `/prompt`, `/right`, `/wrong`, `/wrng`, `/contributorfractal`, `/saveSpecimen`, `/loadSpecimen`, `/addPattern`, `/learn` | Additive command versions |
| 23 Limits | User hard ceilings dominate adaptive budgets | Hierarchical admission controller |
| 24 Persistence | Whole durable specimen and coherent restore | Atomic manifest and portable export |
| 25 Acceptance | Original behavioral cases and open decisions | Hardware/network/privacy gates |
| 26 Coverage | Original source coverage map | Requirements-to-tests evidence ledger |

Key original rules: a Pattern ID indexes a *collection* of nodes, not a unique node; default 20 entries is per node, not per Pattern ID; exact repeats are rejected but paraphrases permitted; a hard attachment guarantees scheduling, not a vote; the secondary scan always uses high resolution; confidence stays in [-100,100]; independent node votes precede orchestration; Transient Memory cannot learn; Type B inferred knowledge needs `/learn <proposal_id>`; a mutation slot remixes at most once; Index Rafts are not free work; and the 1,000 high-resolution scan ceiling counts actual concurrent comparisons, including attachments.

### A.3. Integrated invariants

**N01 Canonicality:** only committed canonical records are authoritative; every index and accelerator cache is rebuildable. **N02 Identity:** logical IDs, record references, content digests and revisions remain distinct. **N03 Causality:** original signed bytes and parent hashes never mutate; legacy digests are not rewritten. **N04 Authorization:** policy applies before candidate exposure, graph traversal, spatial projection, export or remote execution. **N05 Evidence:** integrity ≠ truth; similarity ≠ applicability; proximity ≠ causation. **N06 Compute:** stable CPU scalar is the correctness oracle; acceleration cannot silently change gates. **N07 Durability:** acknowledged writes meet the configured fsync/quorum boundary. **N08 Snapshots:** each cycle/query binds the read-view bundle defined in main section 5; a lease preserves lifetime but does not manufacture cross-store atomicity. **N09 Resources:** all queues, scans, attachments, buffers and RPCs are bounded. **N10 Learning:** no persistent new inference without the authorized learning gate. **N11 Erasure:** an approved deletion profile inventories derivatives, indexes, replicas, caches and known exports, respects holds, and explicitly states scope and unresolved copies. **N12 Determinism:** canonical identity does not depend on thread or device scheduling. **N13 Remote trust:** bind remote work to identity, scope, snapshot, digest and policy epoch. **N14 Spatial types:** physical frames and semantic projections cannot be mixed. **N15 Evidence honesty:** capability detection is not execution proof.

### A.4. Identifiers, schema and transaction boundaries

Use distinct `TenantId`, `GuildRef`, `MemberRef`, `PersonaScope`, `SpecimenId`, `RecordId`, `RevisionId`, `EpisodeDigest`, `CausalHead`, `OperationId`, `RequestId`, `CycleId`, `NodeRef`, `EntryRef`, `SlotRef`, `PatternKey`, `ResourceId`, `AttachmentRef`, `CoordinateFrameId`, `EmbeddingSpaceId`, `IndexGeneration`, `ShardId`, `ReplicaId`, `WorkerId`, `CapabilityId` and `PolicyDigest`. UUIDs identify entities; hashes identify immutable bytes; generation numbers identify only their declared local publication order; multiwriter/index content identity and control epochs remain separate. All externally supplied IDs are validated for scope, length and canonical encoding. Cross-scope identifiers cannot be compared or joined without explicit authorization.

The canonical schema registry declares a stable type ID, schema version, canonical serialization profile, compatibility window, max serialized size, required policy version, retention class, optional encryption envelope, migration function and golden vectors. New fields on old signed records are *attachments*, not in-place changes. A schema reader must reject unsupported critical fields and preserve unknown noncritical fields when round-tripping is authorized. Missing and zero-valued fields remain distinguishable.

Each transaction carries operation ID, writer identity, causal parents, read snapshot, write set, preconditions, idempotency key, schema digest, policy digest, size budget and expected durability class. Admission checks authorization and budget before reserving a commit. Validation and canonicalization precede hash/signature creation. Within a named authority, a fenced publication advances a manifest from one verified generation to another. The existing v2 store and v3 episode ledger are not one global transaction; bundles across them need an explicit complete-publication marker or other reviewed protocol. Conflicting causal heads remain visible until an explicit authorized resolution causally dominates them.

### A.5. Durable storage: v1, v2, v3 and future v4

#### A.5.1 Preservation of historical formats

- **v1:** read-only opening does not mutate files; first authorized write triggers a verified sibling-generation migration with byte-exact backup. Preserve v1 WAL/JSONL/segment recovery as migration oracle.
- **v2:** retain UUID public identities, independent per-writer append-only causal journals, committed transaction objects, immutable snapshots, conflict sets, SHA-linked audit blocks, signed/encrypted object options, snapshot-backed zero-copy views and explicit conflict resolution. Preserve the exact existing v2 commitment algorithm and parent ordering for existing objects.
- **v3:** preserve deterministic `abbey-cbor-episode-v1` encoding, sorted canonical parents, detached Ed25519 signatures over episode digests, lifecycle event records, content-free receipts, consent/policy drift gates, contradiction/quarantine/resolution memory edges and replay verification. A detached signature is not COSE; do not claim cross-language signature verification until demonstrated.
- **v4 proposal:** if new specimen/spatial/accelerator metadata requires a new envelope, define a **new** namespace and profile, retaining v1/v2/v3 readers and original hashes. Never silently reinterpret old bytes.

#### A.5.2 Physical layout (proposed)

```text
store/
  CURRENT                    # atomic manifest pointer
  MANIFEST.<generation>      # checksummed snapshot + index generations
  journals/<writer-id>/...   # append-only committed transaction objects
  segments/<generation>/...  # immutable sorted record and vector blocks
  episodes/v3/...            # canonical episode ledger + signatures
  indexes/vector/...         # rebuildable HNSW/PQ
  indexes/lexical/...        # rebuildable postings/BM25
  indexes/spatial/...        # existing octree first; optional qualified alternatives
  indexes/graph/...          # rebuildable adjacency + edge metadata
  indexes/projection/...     # rebuildable semantic 3D layouts
  specimens/<id>/...         # versioned HybridTable/organ manifests
  keys/metadata/...          # key IDs only; secrets in protected provider
  snapshots/leases/...       # bounded reader leases
  receipts/...               # signed operation and migration receipts
  quarantine/...             # integrity failures isolated, never served
```

Do not introduce a second independent authoritative database merely because a convenient search library uses SQLite, RocksDB or a GPU-native format. An ADR may select a transactional metadata engine, but the resulting authority and replay rules must be singular and documented.

#### A.5.3 WAL, checkpoints and recovery

Write-ahead log frames include magic/version, bounded length, operation ID, sequence, payload digest and CRC for corruption detection; cryptographic authentication is a separate property. Recovery validates lengths before allocation, rejects torn trailing frames, replays complete commits idempotently, preserves conflict heads and verifies referenced segments. Each ACK class names its canonical durable object and required file/directory synchronization on the tested platform. A local durable canonical commit must not wait for unrelated derived-index publication. Directory-entry durability and atomic-replace behavior must be tested rather than inferred from a rename call. Checkpoint publication uses temporary file + flush + atomic rename + directory synchronization where supported. Never delete the last known good generation until an operator-approved retention gate and lease scan pass.

Recovery matrix: power loss before WAL flush; during object write; after object write before manifest; during index generation; during rekey; after replication ACK; after conflict resolution; during compaction; during member erasure; while GPU/remote tasks are running. Every case must have a deterministic expected visible state, repair action and evidence artifact.

#### A.5.4 MVCC and concurrent writers

Read-view tokens bind v2 source coverage, the relevant v3 evidence boundary, frame-registry revision, authenticated policy context, applicable shard/control epochs, and resource leases; they are not a single scalar revision. Readers pin immutable snapshots; writers append independently; conflict detection distinguishes causal concurrency from stale conditional writes. Explicit resolution records include all resolved head IDs and never silently discard concurrent values. Secondary index publication is tied to a snapshot generation; an exact current query may use a stale index only with a proven complete delta including additions, updates, deletes, conflicts, frame changes and relevant policy invalidations; otherwise scan or fail. An explicitly bounded-stale query uses its declared older read view, not a silently incomplete current view. Garbage collection respects leases, holds, unreplicated heads, signed receipts and backup generations.

### A.6. High-dimensional vector database

#### A.6.1 Embedding space contract

An `EmbeddingSpace` defines model name, immutable model digest, preprocessing version, modality, dimension, scalar type, normalization, metric (`cosine`, `dot`, `L2`), quantizer generation, tenant scope and data license. Vectors of different spaces are never directly compared. All inputs reject NaN/infinity, impossible dimensions, overflowed lengths and unauthorized model IDs. Zero vectors are handled by explicit metric rules. Embedding provenance points to the input content digest and inference model manifest, not merely a human-readable label.

#### A.6.2 Index families

- **Exact:** linear scan with stable tie-breaking by `(distance, RecordId)`, scalar CPU oracle and optional stable SIMD; correctness reference for every accelerator and ANN implementation.
- **HNSW:** preserve existing implementation and stable public record IDs; tune `M`, `efConstruction`, `efSearch` by dataset; record index build seed and parameters; ensure tombstones, filters and snapshot generations work together.
- **IVF/PQ/OPQ (optional):** store codebook/model digests, residual norms and reconstruction metrics; rerank a configurable candidate set using original vectors when available. Existing PQ/autoencoder/Huffman/rANS artifacts remain reference points; no compression-ratio claim without dataset evidence.
- **Multi-vector:** represent document chunks, visual patches, audio spans and other components with parent record IDs; support max-sim/weighted late interaction with explicit aggregation semantics and strict caps.
- **Filtered ANN:** apply ACL and selective metadata filters before candidate exposure; if post-filtering is unavoidable, guarantee no forbidden IDs, scores or count metadata escape. Recall is measured on the authorized subset, not on the full corpus.

#### A.6.3 Query contract

`VectorQuery { space_id, vector|embedding_request, k, filter, snapshot, consistency, candidate_budget, rerank_budget, deadline }`. Output includes `record_ref`, `revision`, `metric`, raw metric value, normalized score if defined, index generation, candidate method, approximation status, source episode and separate evidence dimensions. A score of `0.83` without metric/model/space is not portable. Query planners compare selectivity, vector dimension, index health, available memory, device transfer costs and deadline before selecting exact/HNSW/accelerated paths.

#### A.6.4 Evaluation

Use public and private permission-safe corpora of at least tiny, medium and stress sizes; stratify by dimension, modality, tenant count, filter selectivity and update rate. Record Recall@1/10/100, NDCG@k, exact-oracle agreement, index-build time, ingest rate, resident memory, on-disk bytes, cold-start, tail latency and index freshness. Approximate search quality must never be conflated with embedding-model relevance.

### A.7. Native three-dimensional spatial and projection database

#### A.7.1 Three independent spaces

1. **Physical/geometric 3D:** validated new `f64` geometry in an explicit frame and units under `cartesian-f64-v1`; historical `f32` bytes and APIs are retained. Legacy-unbound data stays unitless until explicitly bound.
2. **Semantic embedding space:** high-dimensional vectors; no physical distance interpretation.
3. **Visualization projection 3D:** derived coordinates generated by a versioned PCA/UMAP/other projection; distances are approximate layout aids, not canonical semantics.

A new framed physical query requires a valid `frame_id`, unit and applicable transform revision; legacy unbound inspection is a separate compatibility mode and cannot invent meters or world coordinates. Geographic coordinates require appropriate geodetic CRS and distance computation; do not treat longitude/latitude as Euclidean meters. Rotations use a documented handedness and quaternion normalization; transforms must reject singular or incompatible frames.

#### A.7.2 Spatial primitives and predicates

`Point3`, `Aabb3`, `Sphere3`, `Ray3`, `Segment3`, `Polyline3`, `Triangle3`, `MeshRef`, `VoxelGridRef`, `OrientedBox3`, `Transform3`, `TemporalPose3`, and `RegionRef`. Each geometry has precision, frame, source, validity interval, access scope and bounded payload size. Implement point-in-box, sphere overlap, AABB overlap, nearest-neighbor, ray intersection, containment and optional temporal swept-volume predicates. Define boundary inclusion and floating-point tolerances. Use the complete-scan, defined-float and bounded-dyadic distinctions in main section 5.2. Exact predicates require conservative pruning plus exact boundary handling; final checks alone cannot repair a candidate omitted by approximate traversal. Rays, meshes, oriented boxes and swept volumes are later separately specified profiles, not implied by the first point/AABB/sphere implementation.

#### A.7.3 Spatial indexes

Compare R-tree/R*-tree, bounding-volume hierarchy, uniform/hashed grid, octree and Morton-order partitions against realistic update/query workloads. Start with the independent complete reference and qualify the existing octree; compare another index only after workload evidence. Separate 3D index generation from canonical records, commit it atomically against snapshot head, and rebuild after crash. Geometry mutation invalidates only affected spatial projections and graph edges. Cross-node partitions may use coarse spatial cells for routing, but hot-cell skew and boundary overlap must be measured; cells do not override tenant or causal ownership.

#### A.7.4 3D semantic explorer

The explorer is a read-only-by-default visualization of authorized records: clustered points, explicit typed edges, temporal playback, source inspection, selected-node evidence, uncertainty shading, provenance trails, layer toggles, spatial selection, query-by-region and graph-neighborhood expansion. Every displayed node carries a stable record ref and projection generation. Picking a 3D node never confers permission to mutate it. A projection can be discarded/recomputed without touching signed episodes. Render using WebGPU/Metal/Vulkan or a browser fallback; decimate and stream level-of-detail chunks to bound client memory. Never include forbidden records in geometry buffers or WebSocket deltas.

#### A.7.5 Projection quality

For PCA/UMAP/t-SNE-like projection, capture the authorized fit cohort, model digest, algorithm/version, seed, input order/snapshot, numerical/reduction/thread profile, dimensionality, hyperparameters and transform policy. Frozen-profile replay and cross-platform tolerance/subspace equivalence are separate guarantees; repeated eigenvalues require explicit basis rules. Evaluate trustworthiness, neighborhood preservation, stability across updates, runtime and memory. Explicitly label nonparametric methods that cannot place new points without rebuilding. Avoid interpreting projection clusters as evidence of causal relationships. Strictly isolated layouts must not be trained on forbidden records merely because those points are hidden from the final scene; invalidate cohort-dependent transforms and caches on relevant privacy changes.

### A.8. Hybrid retrieval, graph, temporal and evidence

A query plan has stages: (1) authenticate and authorize, (2) bind snapshot and policy epoch, (3) choose selective lexical/spatial/temporal/graph/vector candidate routes, (4) union/intersect with bounded deduplication, (5) exact predicate validation, (6) evidence-aware reranking, (7) final authorization recheck and redaction, (8) stable pagination and explanation. Cost model includes index selectivity, device transfer, network fan-out, vector dimension, graph branching, cache locality and deadline. The planner may fall back to exact CPU search on uncertainty.

**Lexical:** tokenization and language version, postings, BM25/phrase filters, multilingual normalization, privacy-aware snippets. **Temporal:** valid time versus transaction time, recency half-life and historical snapshots. **Graph:** typed edges `causes`, `supports`, `contradicts`, `supersedes`, `references`, `attached_to`, `spatially_contains`, `derived_from`, `participated_in`, `approved_by`; distinguish inferred versus asserted edges. **Fusion:** versioned reciprocal-rank fusion or learned ranking only with held-out evaluation and explicit globally comparable inputs; shard-local ranks are not automatically global ranks; never multiply everything into an opaque single number without exposing components.

The original current ranker `semantic × temporal × causal × persona` remains available as a compatibility mode. The proposed evidence-aware mode returns separately: **cryptographic validity**, **source identity/reputation**, **calibration**, **outcome validation**, **schema/model/policy compatibility**, **task-regime compatibility**, **lifecycle/retention/quarantine**, and **confidence/uncertainty**. Invalid signatures fail closed; valid but contradicted or uncertain records remain inspectable where authorized with explicit disposition. No ranker may silently promote uncertainty into truth. `retrieved_parent_blocks` identifies actually consulted evidence, distinct from causal parents.

### A.9. Neural specimen: HybridTable, Pattern IDs and Language Governance

Each organ stores its durable state through HybridTable-managed typed partitions. HybridTable selects arrays, hashes and indexes based on measured access patterns, but physical representation must not change logical equality or iteration guarantees. Pattern keys are `(modality, resolution_class, encoder_version, pattern_id)`. Low/medium/high classification happens before key generation; matching keys only select candidates. The secondary deep scan **always** uses high resolution, even when coarse lookup was low-resolution. All compatible records sharing a logical ID are returned by `find_all`; `find_first` is a separate API. Exact structural duplicate detection compares canonical content after hash-bucket selection; hash equality alone is not proof.

Language Governance performs versioned normalization, tokenization, lexical/semantic variants, triple extraction, context constraints, language detection and bounded fan-out. It records original input bytes separately from normalized forms. Version changes require reindexing and migration maps, not reinterpretation of old Pattern IDs. Preprocessors must be deterministic or record model version/seed and nondeterminism. Injection-looking text is data, never privileged instructions.

### A.10. Sigils, scripts, pattern votes and attachments

Sigil scopes (`reserved`, `user`, specimen, tenant, conversation and task where applicable) are separately authorized. Bindings, captures, functors and automata run in a capability-restricted interpreter with fuel, recursion, time, memory and side-effect limits. No arbitrary shell/network/file operations by string interpolation. Literal output is escaped as data; reserved sigils cannot be shadowed by user names. A successful textual match is not permission to execute a tool.

Pattern nodes hold a default maximum of **20 entries per node**; unlimited duplicate public IDs are not implied, because overall tenant budgets still apply. Every eligible entry may independently vote after high-resolution comparison, confidence gate and inhibition check. Confidence is bounded to `[-100,100]`; selection weight is a separate finite nonnegative value. Bounded temporary jitter respects exact endpoints and crystallization exceptions. Same-triple spin-bottle arbitration removes redundant alternatives only within its declared group; complementary votes remain. Attachment crystallization schedules target evaluation on unmatched remainder but never bypasses its gate. Soft attachments use bounded affinity, TTL and loop detection. An attachment-triggered deep scan consumes the same global comparison permits.

Every runtime vote records `cycle_ref`, `node_ref`, `entry_ref`, `slot_ref`, selected alternative, inhibition, tone, confidence before/after jitter, strength snapshot, activating keys, attached group, origin triple, source snapshot and provenance ref. The orchestrator consumes a completed immutable vote set; it does not rewrite a node's vote after submission. Its composition trace lists ordered steps, contributors, suppressed alternatives, supporting resources, transformations and output spans.

### A.11. Neural specimen: cycles, Transient Memory, ATP, conversation and vision

#### A.11.1 Cycle state machine

`Received → Normalized → CandidateBound → ScanScheduled → VotesCollected → Orchestrating → Synthesizing(optional) → ProposedActions → AuthorizedOutput → FeedbackEligible → Closed`. Abort paths are `Cancelled`, `DeadlineExceeded`, `PolicyDenied`, `SnapshotExpired`, `ResourceExhausted`, `InvalidEvidence` and `DeviceFailed`. A cycle pins one snapshot and one policy epoch. Cancellation accepted before commit publishes no partial durable edits; after the commit fence it returns a committed receipt or a reconcilable unknown transport outcome, never a fictitious rollback. Streaming partial text may be emitted only with a provenance status and must never be mistaken for an authorized completed action. Replay captures deterministic inputs, selected seeds, model hashes, candidate ordering, vote ordering and scheduler decisions where they affect observable behavior.

#### A.11.2 Transient Memory

Transient Memory is a temporary sparse matrix/MLP or other explicitly selected synthesis utility. It is **non-learning** in the original behavioral contract: runtime activation does not update persistent weights, create permanent knowledge, or auto-approve inferred content. Scratch tensors and activations live in bounded CPU/GPU/NPU arenas, are cleared or discarded within documented API/threat-model limits. A still-executing device or blocking task retains its buffers and reservation after logical cancellation until actual completion or verified cessation; no universal hardware-zeroization promise is made. Optional pretrained text/image model extensions must declare model digest, license, runtime, quantization, device requirements, privacy policy and fallbacks. A generated output is an observation/proposal, not a trusted memory until admitted by the appropriate gate.

#### A.11.3 ATP and stochasticity

Action Tone Predictor retains original bounded tone/arousal/cooldown dynamics, self-directed behavior and brainstorm modes. Runtime jitter is temporary, does not accumulate into durable baseline by repeated reads, and never alters IDs, permissions, hard caps or signed facts. Record RNG algorithm, seed, purpose-separated draw identities and relevant termination context for replayable stochastic behavior. Retries do not draw again; wall-clock-limited runs can terminate differently under different loads even with the same seed. Brainstorm and self-directed work consume lower-priority budgets and may be disabled by the user. A cooldown is a state transition, not an excuse to bypass authorization.

#### A.11.4 Conversation and temporal coherence

Maintain rolling conversation history, pinned records and scoped summaries with explicit retention and consent. The original 100,000-history-record editorial default remains an original-profile value subject to configuration and storage budgets. Conversation history must preserve source identity, chronological order, edits/deletions, original content digest, scope and relevance selection. Summaries are derived, carry parent refs, and are invalidated or redacted when a source is erased or access changes. Retrieval should distinguish current context from historically valid context and not leak other personas/guilds.

#### A.11.5 Vision and multimodality

Retain eye lifecycle, visual pattern representation, OCR-derived text, ephemeral fuzzy art and original terminology “nonlinear SDF” without asserting that pixel-like arrays mathematically form a true signed distance field. Define any actual SDF representation separately with sampling, sign, units, resolution and transform semantics. Multimodal records may reference image patches, frames, audio spans, 3D meshes, voxels, depth maps and model outputs by digest; large artifacts are external immutable blobs, not embedded in signed episode headers. Vision compute may run on CPU/GPU/NPU but every model output records its model digest and privacy disposition. Fuzzy art stays ephemeral unless explicitly saved.

### A.12. Neural specimen: Type A/B, feedback, mutation, PHAGY and contacts

Type A non-pattern nodes contribute residual corrective votes after the main vote phase; Type B nodes may modify explicitly permitted internal operating state during bounded idle work but cannot commit newly inferred learned knowledge without `/learn <proposal_id>`. Node types are registered with versioned capabilities, schema and safe default-deny permissions. Unknown types are inert and round-trippable, not executed.

Contributor provenance is constructed **during** composition at output-span granularity, including supporting resources, transformations and selected vote alternatives. Whole-answer `/right` and `/wrong` (and alias `/wrng`) and targeted feedback retain the original independent coin-flip rule per affected contributor. Color/contributor-targeted feedback selects a subset without rewriting other contributors' history. Feedback changes eligible strengths under clamped arithmetic, not signed past episodes. It must not change pretrained weights or automatically create new patterns.

Growth policy is approval-gated where required; exact duplicate patterns are rejected while paraphrases remain legal. The idle scheduler defaults to fresh eligible idle intervals of **60–120 seconds**, yields to user work, and obeys hard CPU/memory/power ceilings. Vote mutation tag preserves donor lineage and original pairing inhibition. A recipient alternative slot may be remixed **at most once for its lifetime**; pool shortening cannot erase a remixed slot without a separately approved tombstone policy. PHAGY is a bounded cleanup mechanism with sparse ephemeral automata, unique job IDs, timeouts, retries, idempotency and inspectable outcomes. At zero strength, remove active node indexes while preserving growth inhibition and mutation history. Grave retention/resurrection remains explicitly open until approved. The compartmental contact ledger enforces scope independently of matching names and prevents cross-tenant inference through search or graph edges.

### A.13. Index Rafts, chunks and scheduling

An Index Raft is a resumable traversal task over a **disjoint, complete** index range; the first begins at zero. `find_all` must not stop at the first matching ID. Every continuation yields and preserves a cursor. A physical chunk is storage layout; a raft is scheduled work; a thread is an execution resource. Do not equate them. The original global active high-resolution comparison ceiling is **1,000 by default**, across ordinary, attachment and remote tasks; each comparison acquires a permit. Candidate queues can wait, time out or explicitly fail but cannot silently disappear. Backpressure limits queue depth and per-cycle memory; work stealing preserves deterministic final ordering. Index Rafts are a latency-management mechanism, not an asymptotic free-scan guarantee.

### A.14. Stable Rust 1.99.1 implementation profile

The requested production toolchain remains stable Rust 1.99.1, edition 2024. Current WDBX commit `08a69b3f22db61093e06527b3308b10589687022` already adopts it for shared compute using safe fixed-width lane accumulation. The commit reports a successful locked check and explicitly no runtime/performance qualification; this revision does not rerun it. Qualify the landed implementation and its consumers instead of performing a repeat migration. `std::simd` is still experimental in the checked official 1.99.1 documentation [EXT02]; any remaining consumer nightly dependency requires its own measured compatibility fix. Preserve `unsafe_code = "deny"`; optimized intrinsics/FFI require an approved isolated boundary. CPU compatibility, independent numeric reference, ISA evidence, and performance remain separate.

Suggested workspace additions **without renaming existing `abi-*` crates**: `abi-wdbx-spatial`, `abi-wdbx-projection`, `abi-wdbx-query`, `abi-wdbx-specimen`, `abi-wdbx-cluster`, `abi-wdbx-accelerators`, `abi-wdbx-interop`, `abi-wdbx-evidence` only as seams justify. Preserve `abi-foundation`, `abi-telemetry`, `abi-compute`, `abi-core`, `abi-wdbx` and sibling path dependencies until migration is explicitly proven. Avoid circular dependencies. Prefer small crates for unsafe FFI and device code, not premature fragmentation of core domain types.

Async I/O can use Tokio behind traits and bounded task groups; CPU-intensive work belongs on bounded blocking/compute pools rather than async reactor threads. Use `Arc` immutable snapshots, cancellation tokens, bounded task groups/channels, RAII lifetime leases, and typed errors. A token is cooperative: started blocking work and submitted kernels may keep running; retain reservations and reconcile commit outcomes accordingly [EXT03]. Avoid global mutable singleton schedulers, unbounded `spawn_blocking`, long-held locks across `.await`, blocking I/O in async handlers, and unsound lifetime extension of zero-copy views. Cross-platform path handling, filesystem sync and credential storage require target-specific tests.

### A.15. CPU execution architecture

#### A.15.1 Baseline and stable SIMD

The CPU reference workstream specifies and qualifies the necessary operations; this is not a claim that the current `CpuScalar` label supplies an independent mathematical oracle. Candidate reference kernels include: dot, L2, cosine, normalization, top-k, exact vector search, HNSW traversal, BM25 scoring, graph BFS, spatial predicates, matrix operations, PQ decode, sparse activations, and deterministic reductions. Stable optimized implementations may use compiler auto-vectorization or target-specific x86_64 SSE/AVX2/AVX-512 and ARM NEON intrinsics **only when compiled and runtime-detected safely**. Dispatch uses the existing five capability facts plus the independent qualification key; safe lane width, eligible ISA, observed generated instructions and actual execution are distinct. Cache only a valid implementation/qualification tuple. Never execute instructions merely because a build target claims them. Runtime detection and scalar fallback must be tested on heterogeneous nodes.

#### A.15.2 Numerical contracts

Use f64 accumulation for reference where appropriate, define float ordering including NaNs (rejected), canonical zero handling, and tie-breaking. Vectorized/GPU fused multiply-add and reduction order can change last bits; declare operation-specific absolute/relative tolerances and rank stability requirements. Any computation that affects signed commitments, exact Pattern IDs, authorization gates, threshold crossings or conflict resolution must use deterministic canonical logic, not nondeterministic floating-point reductions. Batch results may be reordered internally but final top-k ordering is stable.

#### A.15.3 CPU scheduler and locality

Work pools separate latency-critical query tasks, bounded high-resolution scans, ingestion/index builds, specimen idle maintenance, recovery and background compaction. NUMA-aware placement is optional and measured. Per-tenant CPU shares, cancellation polling, memory quotas, priority aging and fair admission prevent a hot query or mutation storm from starving other users. Optimize cache line layout and contiguous vector blocks only after profiling. No claim of “zero copy” across device boundaries; snapshot-backed CPU views and GPU upload buffers have different lifetime contracts.

### A.16. GPU execution architecture

#### A.16.1 Adapter abstraction

GPU is an **optional accelerator**, never the sole authority. Define a backend-neutral capability contract with: device UUID/vendor/driver, memory limits, supported scalar types, kernel inventory, shader binary digest, max batch/shape, queue depth, synchronization model, supported features, health state and measured cost model. Backend implementations may include Apple Metal, Vulkan compute and NVIDIA CUDA via audited FFI/vendor runtime. A successful device probe is **detected**, not **runtime-verified**. The existing Metal paths have bounded CPU-oracle local numerical tests; they do not establish speedups. CUDA/Vulkan adapters must not be labeled runtime-verified without real execution.

#### A.16.2 Candidate GPU workloads

Dense dot/cosine/L2 batches, brute-force top-k candidate generation, PQ decode, k-means/codebook training in a separately authorized offline job, bulk vector normalization, graph frontier expansion for high-degree workloads, spatial BVH broad-phase, large 3D projection transforms, image preprocessing, sparse-dense matrix operations and optional model inference. Irregular small HNSW graph walks may remain CPU-optimal due to divergence and transfer costs. GPU may generate candidates; final exact predicates, ACL and evidence gates remain authoritative in trusted CPU code unless a separately verified secure GPU boundary is established.

#### A.16.3 GPU memory and scheduling

Use bounded device arenas, explicit host/device buffer ownership, pinned staging budgets, command-buffer fences, cancellation at kernel/batch boundaries, deadline-aware queues and tenant isolation. Track upload/download bytes, PCIe/unified-memory costs, occupancy, kernel launch overhead, warm/cold timings, driver resets and out-of-memory errors. Do not assume unified memory removes synchronization or copy costs. On failure, follow the precomputed allowed fallback plan only when remaining budget and the same output contract permit it; otherwise fail explicitly. Keep uncertain work reserved until it actually stops. Never double-commit mutations. GPU buffers containing sensitive data must be cleared according to threat model and device API limitations; do not claim perfect zeroization without hardware evidence.

#### A.16.4 GPU correctness gates

Per kernel: known-answer vectors; random and adversarial numeric cases; boundary dimensions; zero vectors; huge/small magnitudes; NaN rejection; determinism or bounded tolerance; stable top-k; CPU oracle parity; concurrent queues; cancellation; device removal; out-of-memory; driver version recording. Publish speedups only against optimized CPU baseline including transfers, not scalar CPU with excluded transfer time.

### A.17. NPU and neural-engine execution architecture

#### A.17.1 NPU role

NPUs accelerate **supported neural model graphs**, not arbitrary database indexing or filesystem transactions. Candidate workloads: embedding generation, text/image/audio encoders, sparse/dense learned rerankers, optional Transient Memory inference, visual classification, and bounded local generative model inference. HNSW pointer-chasing, WAL replay, MVCC, signing, authorization and general graph traversal stay CPU unless a specific hardware/runtime supports and proves an alternative.

#### A.17.2 Apple Neural Engine / Core ML

On Apple platforms, a Core ML adapter declares model format, input/output tensor shapes, precision, compute-unit preference, compiled-model hash, OS/runtime version and device eligibility. A `.cpuAndNeuralEngine` preference **does not prove ANE residency**. A correct output under that preference proves only successful Core ML execution. Claim ANE placement only with supported device-specific profiling or authoritative instrumentation, and label uncertainty when the runtime does not expose placement. Metal GPU and Core ML NPU are separate capability states and cannot be conflated.

#### A.17.3 Other NPU runtimes

Other vendor-provider paths are optional and selected individually by T38, with actual device/model/runtime identity, license, opset, shape/dtype/quantization support, memory and cold-compile behavior. Windows ML is a provider framework; DirectML is a GPU provider, not a generic NPU backend [EXT05]. Qualcomm QNN, an Intel NPU provider or another vendor SDK requires its own current documentation and real-device qualification before implementation claims. CPU fallback is permitted only when the model/output contract, resource budget and policy allow it. One model's success is not arbitrary-model support.

#### A.17.4 Neural model lifecycle

A `ModelManifest` includes digest, source/license, model family, architecture, tokenizer/preprocessor digest, output embedding-space ID, input constraints, tensor dtypes/shapes, quantization parameters, supported backends, calibration corpus digest, numerical tolerances, storage size, memory estimate and optional privacy classification. Model downloads are explicit and hash-verified; never silently fetch remote weights. Different embedding model versions define different vector spaces and require re-embedding or a validated cross-space mapping. Online model weight updates are separate authorized training jobs, never implicit specimen feedback.

### A.18. Optional TPU and remote specialized compute

TPU support is **proposed/optional** and generally mediated by a remote, authenticated inference service or supported runtime. Advertise exact model/op support, compiler and service versions, batch shapes, quotas, locality, residency and pricing. No production TPU claim is implied by a generic backend enum. Avoid transferring sensitive embeddings or member plaintext to a remote accelerator unless scope policy explicitly allows the destination and the transport/privacy controls are verified. Device-aware scheduling may choose `cpu`, `gpu`, `npu`, `tpu`, `remote`, or `auto`; `auto` is an evidence-based cost decision, not an unconditional fastest-device claim.

### A.19. Heterogeneous compute scheduler and capability state machine

Preserve existing `CapabilityEvidence` facts: `compiled`, `available`, `initialized`, `executed`, and `runtime_verified`. Add the `QualificationRecord` from main section 5.8 keyed by operation/model/device/driver/build/dtype/shape/numerical profile; do not introduce a competing status ladder. Existing compatibility booleans do not prove independent oracle agreement, emitted ISA, NPU placement or speedup [SRC02][SRC03].

Estimate end-to-end cost as queue + setup/compile + transfers + kernel + verification + remote round-trip + bounded expected-failure cost. Filter permitted candidates by policy, model/operation compatibility, qualification, memory and remaining deadline before choosing by cost. The fallback list is bounded and acyclic; a required backend never silently changes. No fixed CPU-to-GPU-to-remote ordering overrides a user's privacy or output contract. Retries use stable identities and cannot take over canonical commit authority.

The same workload may be partitioned: CPU performs access-control and query planning; GPU generates vector candidates; NPU embeds the input; CPU validates candidates and ranks evidence; cross-node workers search authorized shards; the coordinator merges results deterministically. Each stage carries a signed/authorized context envelope and typed output contract.

### A.20. Cross-node fabric: topology, identity and trust

#### A.20.1 Node classes

- **Storage node:** owns canonical journal/segment replicas, verifies objects and publishes local indexes.
- **Query node:** coordinates authorized fan-out and merges snapshot-consistent results; may be colocated.
- **Compute node:** executes bounded CPU/GPU/NPU/TPU jobs against authorized, digest-bound inputs; cannot independently commit canonical data.
- **Witness/control node:** manages signed membership/configuration and witnesses quorum decisions where the selected consistency model requires it.
- **Edge node:** local-first cache and authorized offline replica with explicit sync/retention rules.

A physical host may play multiple roles. Roles are capabilities, not names inferred from DNS or IP address. Every node has a cryptographic identity, rotating transport certificate, explicit tenant/scope permissions, hardware inventory and admission quota. Membership changes are signed, epoch-numbered, replay-protected and audited. The existing gateway-local membership lineage and single-host multi-process demonstration are useful foundations but **do not prove production cross-host consensus**.

#### A.20.2 Transport

Use TLS 1.3 with mutual authentication for non-loopback cluster traffic, or an equivalently reviewed authenticated secure channel. Verify certificate chain, peer identity, scope, expiry, revocation policy and ALPN/protocol version. Rotate certs with overlap windows and negative tests. All RPC frames have hard byte, nesting, streaming, deadline and decompression-ratio limits. Use typed gRPC/protobuf or an equally versioned binary protocol; transport JSON is never a canonical episode commitment. Support request IDs, trace IDs, idempotency keys, bounded retries, exponential backoff with jitter, flow control and cancellation propagation. Network partition must not silently downgrade to plaintext or unauthenticated mode.

#### A.20.3 Membership and placement

Maintain signed membership epochs and node capabilities (CPU ISA, GPU/NPU runtime, memory, storage, locality, security zone, failure domain). Rendezvous hashing is an acceptable placement baseline, with configurable replication factor and anti-affinity across hosts/racks/zones. Record shard-map epoch in every request. Placement changes require staged copy, hash verification, tail catch-up, cutover and retained rollback. Do not relocate data into an unauthorized region or hardware zone merely to exploit a faster accelerator. Capacity-aware placement must avoid hot shards and noisy-neighbor starvation.

### A.21. Distributed consistency and causal semantics

#### A.21.1 Choose consistency by operation

| Operation | Minimum required semantics |
|---|---|
| Immutable object put | Content-addressed idempotence and verified digest |
| Causal episode append | Per-writer ordered append, verified parent links, explicit concurrent heads |
| Conflict resolution | Authorized write causally dominates explicitly named conflicts |
| Linearizable configuration/membership | Consensus or equivalent single authoritative sequencer; not a token-based guess |
| Snapshot query | Declared snapshot/frontier and bounded staleness; no mixed-generation silent result |
| Distributed search | Per-shard snapshot token or explicitly declared best-effort freshness |
| Member erasure | Scope-wide fencing/tombstone epoch and cryptographic key invalidation |
| Background index build | Eventually convergent, never authoritative until manifest publication |

The existing WDBX causal store supports conflict preservation; **do not replace it with last-writer-wins**. A Raft-like control plane may order membership and shard leases without imposing one global total order on all user episodes. Consensus membership, leader fencing, election terms, commit indices and snapshot installation must be specified and formally/empirically tested before P3 claims. Distinguish Raft **consensus** from specimen **Index Rafts**, which are unrelated bounded traversal jobs.

#### A.21.2 Replication protocol

A replica receives an exact committed object with canonical bytes, digest, parent refs, writer signature, schema and policy epoch. It validates the envelope and scope under the selected receipt class. A durable-object receipt acknowledges the exact stored bytes; serving or claiming a causal snapshot additionally requires verified dependencies/parents. Do not collapse object durability and causal readiness into one boolean. Replicas may temporarily lack parents; quarantine or queue bounded dependencies and request missing objects, but never serve an unverifiable object as valid. Read repair preserves all concurrent heads. Anti-entropy exchanges compact authenticated manifests/Merkle summaries, then verifies transferred bytes. Resume uses offset, digest and generation, not only file names. Rebalance never deletes source data until target coverage and quorum/retention gates are proven.

#### A.21.3 Failure and split-brain

Test loss of leader, minority/majority partitions, clock skew, duplicate delivery, reordered RPCs, stale certificates, process kill during transfer, full disk, slow replica, partial object, hash mismatch, poisoned index, inconsistent schema, device loss and simultaneous rekey/erasure. Quorum writes fail closed when quorum is unavailable; any optional offline/causal write mode must be clearly surfaced and later reconciled without hiding conflicts. A stale leader must be fenced by term/epoch. Recovery logs never expose sensitive payloads.

### A.22. Distributed query and cross-node neural execution

#### A.22.1 Query routing

Coordinator authenticates the principal, resolves permitted shard set, binds snapshot/frontier and dispatches only to authorized replicas. Each shard applies row-level and graph/spatial filters **before** candidate disclosure. Shards return bounded candidates with globally agreed metric/model/space and ranking-policy identities, read-view coverage, index generation, score components, digest and freshness. Exact global top-k requires a valid completeness bound or full eligible enumeration; local ANN candidates, local RRF ranks or pool-relative distance scaling are not that proof. Coordinator performs stable merge, preserves requested causal version semantics, validates exact predicates where required, and rechecks policy. Exact reranking of returned candidates never upgrades incomplete candidate coverage to exact retrieval. Partial results explicitly name missing shards and consistency degradation; they are not presented as exhaustive. Stable pagination uses a signed cursor containing query digest, shard-map epoch, snapshot and sort boundary.

#### A.22.2 Remote compute jobs

A job envelope includes `job_id`, `tenant`, `principal`, `policy_epoch`, `input_digest`, `snapshot`, `kernel/model_digest`, `allowed_backend`, `precision`, `resource_limits`, `deadline`, `output_contract`, `result_destination` and an authorization proof/capability with short expiry. A worker verifies scope and code/model hashes, refuses unknown kernels, runs in a sandbox with bounded CPU/GPU/NPU/memory/network, and returns a signed result digest plus hardware/runtime evidence. The coordinator validates the declared output and trusted-worker assumptions before use. Sampling may support a probabilistic quality profile but cannot prove exhaustive candidate coverage or correctness against a malicious worker; strict profiles retain those limits. Remote compute never receives long-lived data keys. Sensitive payloads are transmitted only if explicit data residency and consent policy allows; otherwise dispatch code to data or run locally.

#### A.22.3 Placement-aware scheduling

Prefer data-local compute, then same-host accelerators, then same-zone peers, then authorized remote nodes when expected end-to-end latency improves. Account for transfer bytes, accelerator queue, thermal state, batch size, model cold-load, network RTT and privacy. Partition jobs into idempotent microbatches with deterministic merge order. GPU and NPU tasks use per-node quotas; remote cancellation requests release reservations only after completion or verified cessation; unknown/zombie work remains charged. A remote failure must not cause an unbounded retry storm.

#### A.22.4 Multi-node specimen execution

One cycle coordinator owns the cycle snapshot and vote barrier. Pattern candidate partitions can be evaluated on remote workers only if each worker receives the same immutable input, compatible high-resolution encoder, seed partition, gates and policy epoch. The **global 1,000 active high-resolution comparison permits** must be enforced across workers using the strict non-reassignable uncertain-credit protocol in main section 5.10; lease expiry alone never reissues credits while a worker may still execute; local per-node caps alone do not satisfy the original invariant. Workers return votes with stable IDs and provenance. The coordinator deduplicates idempotent retries and sorts votes deterministically before the ephemeral orchestrator. No worker commits learned patterns merely because it produced a vote.

### A.23. Security, encryption, key custody and privacy

#### A.23.1 Threat model

Adversaries include unauthenticated network clients, compromised tenant accounts, malicious documents/prompt injection, rogue compute workers, stolen storage snapshots, compromised replica credentials, corrupted indexes, malicious plugins, accidental operator misuse and supply-chain compromise. Explicitly document exclusions (e.g. fully compromised trusted host kernel) and how they affect guarantees. No cryptographic algorithm compensates for an authorized principal exfiltrating plaintext.

#### A.23.2 Identity and authorization

Use deny-by-default RBAC/ABAC capabilities scoped by tenant, guild, member, persona, specimen, record class, operation, node role, region and time. Policy decisions bind to a versioned policy digest/consent epoch. Authorization must guard metadata, similarity scores, counts, ANN graph adjacency, spatial projections, contact records, logs, caches, snapshots, exports, remote jobs and WebSocket subscriptions. It is enforced at defined materialization/stream boundaries; already returned plaintext or immutable borrowed memory cannot be retroactively revoked. Timing side-channel resistance is a separately scoped measured property, not implied by final filtering. For sensitive indexes, isolate per-scope structures or implement measured non-leaking filtered traversal; merely post-filtering final results can expose data through timing or side channels.

#### A.23.3 Cryptographic objects

Preserve existing opt-in v2 XChaCha20-Poly1305 and Ed25519 authenticated-object paths with verification before parsing. Preserve detached v3 Ed25519 episode signatures; do not call them COSE. Add COSE only after a versioned envelope, canonical cross-language test corpus, key rotation/revocation and external verifier are implemented. Use established vetted cryptographic libraries and OS key providers. SHA-256/CRC serve different purposes: CRC detects accidental corruption, hashes bind bytes, signatures authenticate a signing key, encryption protects confidentiality; none independently proves semantic truth.

#### A.23.4 Member-fact erasure compatibility

The 2026-09-29 erasure spec is **draft revision 3, not approved and not implemented**. Preserve its design intent and approval gate: canonical encrypted member fact text in WDBX; per `(guild_ref, member)` member key; per-fact record key; gateway-held wrapped keys with macOS Keychain master; this-device-only after-first-unlock setting verified on target macOS; signed tombstones; signed holds; one receipt listing removed/retained/why; clearing scope summaries; a seven-day zero-mismatch shadow-read migration before automatic cutover; operator-only rollback; automatic master rewrap/rotation under the original draft's exact timing and approval rules, without selecting a new interpretation of its hourly wording; and retained old bytes only as policy explicitly permits. Do not implement or claim these until the draft's approval requirements are satisfied.

Erasure semantics must account for vector embeddings, index postings, spatial/projection positions, graph edges, replicas, backup generations, model caches, remote worker scratch, logs, export archives and derived summaries. A tombstone alone is not physical erasure; cryptographic erasure relies on actual key destruction and threat assumptions. Legal holds may block erasure and must be reported. Do not promise deletion from third-party providers or backups outside control. Cross-node erasure requires fencing epochs and signed acknowledgments, with partial failures explicitly reported.

#### A.23.5 Research crypto and compression

Existing DGHV/additive HE/TFHE demo paths remain reference/research until independent cryptographic audit and threat-specific approval. No “production FHE” or “zero knowledge” claim based on demo execution. Existing PQ/autoencoder/Huffman/rANS compression is retained; lossy codecs require explicit opt-in, persisted source digest, reconstruction error, search recall impact, codec version and deterministic fallback. Lossless canonical evidence must not be silently compressed with lossy codecs.

### A.24. API, wire contracts and compatibility

Preserve frozen CLI commands and MCP tool names from ABI's public contract. Preserve existing loopback REST `/insert`, `/query`, `/verify`, `/health`, `/stats`, v2 gRPC gateway methods (`PutVector`, `Search`, `PutKv`, `GetKv`, `ResolveConflict`, `Stats`, `MembershipChange`, `WatchMutations`) and existing episode RPCs. Existing MCP HTTP+SSE compatibility is not automatically modern Streamable HTTP. New APIs must be additive, versioned and capability-negotiated; do not silently change request/response shape or auth defaults.

The additive API namespace is defined by T28, not by a second list of illustrative `/v3` routes. The proposed spatial service is `wdbx.spatial.v1.SpatialQueryService`; an optional HTTP mapping uses `POST /api/spatial/v1/query`. Projection/graph/evidence operations belong to that task's reviewed versioned contract. Specimen cycle/feedback, remote dispatch and cluster administration remain owned by their separate service/task and approval boundaries; do not create all endpoints merely because the catalogue names a feature. Every result separates read-view identity, authorization, candidate coverage, numeric accuracy and completion. Durable service work uses explicit operation status, not a promise by this assistant to run later.

Use explicit error taxonomy: `Unauthorized`, `Forbidden`, `InvalidArgument`, `InvalidFrame`, `UnknownEmbeddingSpace`, `SnapshotExpired`, `StalePolicyEpoch`, `Conflict`, `IntegrityFailure`, `ResourceExhausted`, `DeadlineExceeded`, `Unavailable`, `PartialResult`, `UnsupportedBackend`, `ModelMismatch`, `UnverifiedAccelerator`. Include idempotency keys and server-assigned correlation IDs. No error message reveals private record existence to unauthorized callers.

### A.25. Specimen commands and UI behavior

Retain original Studio commands and aliases: `/prompt`, `/right`, `/wrong`, `/wrng`, `/contributorfractal`, `/saveSpecimen`, `/loadSpecimen`, `/addPattern`, `/learn`. Their visible editors, pending-proposal resolution, contributor selection and browser/native save semantics remain unchanged unless a separately approved versioned UI amendment states otherwise. Add commands only under new namespaces such as `/spatial`, `/hardware`, `/cluster`, `/projection`, `/evidence`, `/snapshot`, `/verify` with help text and authorization checks. Do not overload `/learn` to perform autonomous training or remote deployment.

The Studio should expose: node library, HybridTable inspector, cycle timeline, vote trace, contributor spans, memory/resources, learning proposals, attachment graph, ATP/PHAGY state, 3D knowledge scene, physical-space scene, temporal playback, CPU/GPU/NPU activity, remote-node topology, shard/replica health, query explanation, audit/erasure status and resource-budget gauges. All status badges must distinguish configured/detected/tested/deployed; no decorative “accelerated” status without runtime evidence. Accessibility: keyboard navigation, text equivalents for 3D scenes, reduced motion, color-independent contributor identification, screen-reader summaries, pagination and privacy-safe export.

### A.26. Persistence, portable specimens and restore

Preserve original browser JSON and native `.wdbxspecimen` import/export behavior, including validation before replacement, recovery copies, IndexedDB/browser boundaries and native transactional WDBX storage. The original native profile uses Tauri 2, React/TypeScript and pinned WDBX dependencies; do not overwrite it while migrating Rust toolchains. New exports should include a versioned manifest, content digests, specimen organs, pattern encoders, nodes, resources, Sigils, attachments, history, ATP state, transient-network definition (not live activations), inhibition, mutation lineage, learning proposals, contact ledger, spatial attachments, model references, index-generation hints and provenance receipts. Secret keys, live GPU handles, active threads, raw credentials and in-flight executable continuations are excluded by default.

Import validates manifest signature/digest, schema, budgets, path traversal, archive bombs, duplicate refs, Pattern-key invariants, mutation slot lifetimes, scopes, model references and all attachments before atomic activation. Rebuild indexes from canonical state. An import cannot silently replace an existing specimen or widen permissions. Export must support selective redaction and make nonportable dependencies explicit. Snapshot/restore drills include old v1/v2/v3 data, browser JSON, native archives, missing model files, unsupported GPU/NPU, interrupted imports and revoked signing keys.

### A.27. Observability, tracing, accounting and resource budgets

Telemetry separates **control-plane** (auth decisions, leases, membership, migrations), **data-plane** (writes, reads, conflicts, WAL/recovery, index freshness), **compute-plane** (CPU instructions/features, GPU kernel time/transfers, NPU inference latency/placement evidence, remote queue/network), and **specimen-plane** (candidate counts, high-resolution permit utilization, votes, attachments, ATP state changes, PHAGY jobs, learning approvals). Metrics are aggregated and privacy-safe by default; no raw prompts, fact text, embeddings, member IDs, private graph edges or contact names in traces. Sensitive debugging requires scoped opt-in, expiry and audit.

Minimum counters/histograms: write commit P50/P95/P99; WAL fsync; checkpoint duration; recovery time; snapshot lease age; conflict-set size; HNSW recall; filtered ANN recall; BM25 latency; spatial exact-predicate agreement; graph expansion counts; 3D projection quality; CPU/GPU/NPU utilization; accelerator transfer bytes; per-device queue wait; backend fallback reason; remote RPC latency; quorum availability; replica lag; repair bytes; member-erasure propagation; cycle wall time; active deep scans (must stay ≤ configured cap); canceled jobs; memory high-water; disk amplification; cache eviction; rejected permissions; provenance completeness.

Every benchmark artifact must record Git revision, compiler version, Cargo lockfile digest, feature flags, OS/kernel, CPU ISA, GPU/NPU model and driver, model weights digest, dataset generator/seed, warmup, iteration count, confidence intervals, workload mix and environmental conditions. A benchmark result without reproducible inputs is an observation, not a product claim.

### A.28. Performance goals and service objectives (targets, not claims)

Targets must be **chosen after baseline measurements** and are not guaranteed by this document. Establish workload-specific service objectives for: single-record ingest latency; 10k/1M/10M-vector query P95/P99; Recall@10 under high filter selectivity; 3D nearest-neighbor correctness; cross-host fan-out P99; recovery RTO; maximum permitted data loss RPO under each ACK class; CPU/GPU/NPU energy per query; model cold-load; cycle completion latency; erasure fencing propagation; and query completeness under degraded replicas. Publish **hardware-specific** targets rather than one universal “100× faster” promise. Every optimization must report end-to-end speedup including data transfer, warm/cold effects and recall/correctness tradeoffs.

### A.29. Test pyramid and proof obligations

#### A.29.1 Unit and property tests

Schema canonicalization; stable ID encoding; Pattern-key equality; duplicate-ID `find_all`; exact structural duplicate rejection; confidence clamp/jitter/crystallization; spin-bottle independence; hard attachment scheduling but gated vote; Type B `/learn` approval; one-time remix slot; Index Raft disjoint complete ranges; 1,000 global permit cap; spatial predicates and CRS conversions; exact vector metrics; HNSW ordering; stable tie-breaking; ACL filtering; numerical tolerance and NaN rejection; bounded Sigil interpreter; encryption envelope validation; replay/digest invariants.

#### A.29.2 Integration and failure tests

v1 read-only open; v1→v2 first-write migration and byte-exact backup; v2 concurrent causal heads; v3 canonical episode and detached signature verification; restart and WAL torn-write recovery; snapshot pinning during compaction; index rebuild; conflict-preserving read repair; member erasure draft tests only after approval; export/import round trip; cross-language canonical encoding; key rotation; driver reset; remote worker cancellation; stale policy epoch; network partition; quorum fail-closed; rebalance resume; split-brain fencing; certificate expiry; unauthorized graph/spatial search; prompt injection treated as data.

#### A.29.3 Hardware and cross-host tests

On **real hardware**, compare scalar CPU oracle to stable architecture-specific SIMD, Metal, Vulkan, CUDA and supported NPU inference per operation. Capture model/device/runtime IDs and numerical tolerances. Validate output and, separately, performance and actual execution placement where measurable. Test no-device and incompatible-model fallbacks. For `cluster-causal-v1`, use at least three distinct actual physical/virtual hosts and record whether their physical failure domains are shared; inject latency, loss, reorder, partition, disk pressure and process kill. Single-host 3–9 process tests remain local multi-process evidence, not separate-host or independent-failure-domain proof.

#### A.29.4 Security tests

Cross-tenant and cross-guild read denial; unauthorized ANN score/count leakage; projection geometry leakage; Sigil sandbox escape; malicious archive; oversized RPC/decompression; signature tampering; replay; key revocation; secret logging; stale replica resurrection after tombstone; lost erasure ACK; unauthorized remote compute; node impersonation; mTLS certificate rotation; permission downgrade/upgrade; revoked model artifact. Use fuzzing, dependency audits and third-party review where appropriate. A green unit suite is not an external security audit.

#### A.29.5 Evidence matrix

| Gate | Evidence artifact | Required outcome |
|---|---|---|
| Rust stable | `rustc --version`, `cargo +1.99.1 check/test/clippy` | No nightly-only dependencies/features |
| Original specimen | 26-chapter conformance trace, I01–I24 and R01–R23 tests | No behavioral regressions |
| Data integrity | Crash matrix, replay manifests, signatures | No acknowledged partial commit |
| Vector | Exact oracle + Recall@k reports | Declared quality thresholds met |
| Spatial | Geometric oracle + snapshot tests | Exact final predicates |
| GPU | Real device logs + CPU parity | Only verified kernels enabled |
| NPU | Model outputs + device/runtime evidence | No unsupported placement claims |
| Cross-host | Real multi-host chaos and quorum traces | Defined consistency and fencing |
| Privacy | Authorized erasure and derivative audit | No stale visible data; explicit exceptions |
| Restore | Full specimen export/import and rollback drill | Verified coherent state |
| Release | Reproducible build, SBOM, provenance manifest | Signed reviewed artifact |

### A.30. Stable Rust build and CI contract

`rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.99.1"
profile = "minimal"
components = ["rustfmt", "clippy"]
```

Workspace manifests use `edition = "2024"` and an MSRV policy consistent with 1.99.1. Review the existing `abi-wdbx` and `abi-compute` crate graphs for `#![feature(portable_simd)]`, `std::simd`, nightly compiler flags and target-specific APIs. The current source has already adopted stable shared compute; qualify its compatibility and independent numerical behavior before changing it. Any remaining consumer migration is separately scoped. Do not claim a stable build until it passes on the actual target checkout. Keep existing `./tools/check.sh` and source-size/lint discipline; do not weaken CI to force a green result.

Use main section 8.3 as the single command/feature-matrix contract. Preserve `--locked`, the existing warning policy and gate script, and nonzero test-count evidence. Add separate device/provider/research jobs for supported feature tuples rather than an indiscriminate all-features union. No product tests were run by this plan revision. Missing hardware is unverified, not a pass; optional tooling does not become a production nightly dependency.

### A.31. Migration and rollout program

The single milestone/task graph is in main sections 6–7. Start with source/qualification and additive checked point access to the existing octree. Durable attachments, hybrid/evidence queries, projections, each accelerator, cross-node service, original specimen conformance, and approved privacy work remain separate task packets. The previous second M0–M10 sequence is retired so no contradictory order survives.

Each milestone may contain several independently reviewed changes; not every milestone is a tiny implementation. Rollback disables an additive path only when the target reader/writer remains compatible with all canonical data. Never assume that an older binary safely preserves new authoritative namespaces. No commits, pushes, deployments, credential changes, erasure, backup deletion, or production mutation are authorized here.

### A.32. Open architectural decisions requiring ADRs

Main section 4 is the single ADR register. It retains the original decision areas: safe CPU optimization, octree alternatives, frames/geodesy, embedding/model spaces, evidence ranking, HybridTable adapter ownership, scoped consensus, remote privacy, device/provider qualification, client profiles, key custody, original equality/grave semantics, and benchmark/rollback policy. A still-open original design rule remains a named blocker for its feature. Do not postpone the small checked spatial profile behind unrelated optional decisions.

### A.33. Master Superpowers implementation directive

Use the execution handoff in main section 10. The task is evolutionary preservation with source-backed qualification, bounded TDD, independent review where available and actual evidence. Start from the already-landed Rust 1.99.1 commit, not an assumed rewrite. Preserve original source semantics, identities, signed bytes, conflicts, codecs, multiway and all specimen organs; qualify each new spatial/hardware/distributed profile before claiming it. No implicit approval or deployment follows from this catalogue.

### A.34. Source and reference register

- Original 26-chapter specification: https://github.com/donaldfilimon/wdbx-specimen-studio/blob/main/public/WDBX-Specimen-Architecture-Specification.md
- Studio runtime/acceptance: https://github.com/donaldfilimon/wdbx-specimen-studio/blob/main/README.md and `VALIDATION.md`
- WDBX implementation: https://github.com/donaldfilimon/wdbx
- WDBX claim ledger: https://github.com/donaldfilimon/wdbx/blob/main/docs/claims.md
- ABI North-Star: https://github.com/donaldfilimon/abi/blob/main/docs/spec/wdbx-north-star.mdx
- ABI agent architecture: https://github.com/donaldfilimon/abi/blob/main/docs/spec/agent-wdbx-architecture.mdx
- Canonical episodes: https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md
- Multiway simulator: https://github.com/donaldfilimon/abi/blob/main/docs/spec/wdbx-multiway.mdx
- WDBX completion design: https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-09-29-wdbx-completion-design.md
- Member erasure draft: https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-09-29-spec-member-erasure.md
- Rust 1.99.1 official release: https://blog.rust-lang.org/2026/10/15/Rust-1.99.1/
- `std::simd` instability: https://doc.rust-lang.org/std/simd/struct.Simd.html

### A.35. Final acceptance and sign-off

A WDBX Neural Specimen implementation is **not** “final” because this document exists. A claim of the full integrated system requires all applicable items below; the narrower named profiles have their separate gates in section 8.8. Full conformance requires: (a) the complete original 26 chapters remain behaviorally satisfied; (b) stable Rust 1.99.1 builds without nightly-only production features; (c) signed historical data and causal conflicts survive migration/recovery; (d) independent coverage and numerical/predicate oracles exist for the claimed exactness profile; (e) GPU/NPU paths have real-device evidence and honest fallbacks; (f) cross-host paths have actual network/failure evidence; (g) privacy/erasure claims match approved, implemented behavior; (h) authorization holds across derived indexes, visualizations and remote workers; (i) whole specimens export/restore coherently; and (j) reproducible acceptance artifacts and rollback are reviewed.

**Document status remains PROPOSED.** The original specification and existing source remain authoritative for already-defined behavior. No repository, deployment, runtime, device or production data was modified by writing this document.

---

**End of revision 5.0. Planning artifact only; all implementation tasks remain unexecuted in this revision.**
