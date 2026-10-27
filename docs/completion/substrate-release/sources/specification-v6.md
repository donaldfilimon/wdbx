# WDBX — Full Database and Neural Specimen Specification

**Document ID:** WDBX-SPEC-006  
**Revision:** 6.0 — integrated specification draft  
**Date:** 2026-10-20  
**Target:** stable Rust 1.99.1, edition 2024  
**Planning lineage:** WDBX 3D Neural Specimen Master Implementation Plan v5.0  
**Status:** Written specification for review; not implementation, qualification, deployment, or approval of a restricted change.

> WDBX is the durable memory and evidence substrate of the ABI/Abbey ecosystem. It combines version-preserving storage, high-dimensional vector search, physical 3D geometry, typed graphs, temporal records, and provenance-aware episodes. An explicitly separate Neural Specimen runtime uses that substrate for pattern retrieval, independent voting, bounded synthesis, contributor feedback, and approved persistent change. CPU, GPU, NPU, and cross-node execution are qualified execution options—not new sources of truth.

This is a systems specification, not another list of implementation tasks. It defines required observable behavior, data boundaries, algorithms, failure semantics, profiles, and acceptance obligations. The v5 plan remains the implementation companion. Existing project documents retain authority over their original semantics and signed formats; this draft consolidates and extends them without silently ratifying new constitutional rules.

<a id="contents"></a>
## Document navigation

- [1. Authority, terminology, and evidence status](#section-1)
- [2. Mission, boundaries, and non-goals](#section-2)
- [3. Conformance profiles and release scope](#section-3)
- [4. Global invariants](#section-4)
- [5. Logical and physical architecture](#section-5)
- [6. Identity and domain model](#section-6)
- [7. Schema, canonical encoding, and compatibility](#section-7)
- [8. Mutation, commit, idempotency, and cancellation](#section-8)
- [9. MVCC, causal versions, and read views](#section-9)
- [10. Durability classes, WAL, recovery, and checkpoints](#section-10)
- [11. Blobs, immutable bundles, compaction, and garbage collection](#section-11)
- [12. Coordinate frames and geometry types](#section-12)
- [13. Geometric predicates, accuracy, and ordering](#section-13)
- [14. Spatial index implementations and legacy adaptation](#section-14)
- [15. Derived index generations and publication](#section-15)
- [16. Embedding spaces and model registry](#section-16)
- [17. Exact vector retrieval, HNSW, and batch operations](#section-17)
- [18. Multi-vector records and compression](#section-18)
- [19. Lexical retrieval and hybrid score composition](#section-19)
- [20. Typed graphs and causal relationships](#section-20)
- [21. Temporal semantics and historical queries](#section-21)
- [22. Unified query contract and planner](#section-22)
- [23. Eight-dimensional evidence model](#section-23)
- [24. Evidence ranking, learning, and claim promotion](#section-24)
- [25. Multiway rewriting and experiment publication](#section-25)
- [26. Semantic 3D projection](#section-26)
- [27. Authorization, consent, and information-flow boundaries](#section-27)
- [28. Cryptographic protection and key lifecycle](#section-28)
- [29. Member-fact custody, retention, and erasure](#section-29)
- [30. Stable Rust and CPU execution](#section-30)
- [31. GPU execution: Metal, CUDA, and Vulkan](#section-31)
- [32. NPU, neural providers, and optional TPU](#section-32)
- [33. Capability evidence, qualification, and dispatch](#section-33)
- [34. Remote compute job contract](#section-34)
- [35. Cross-node membership and control plane](#section-35)
- [36. Replication, anti-entropy, and rebalance](#section-36)
- [37. Distributed snapshots and search](#section-37)
- [38. Strict distributed resource accounting](#section-38)
- [39. Neural Specimen organization and HybridTable](#section-39)
- [40. Language Governance and scoped Sigils](#section-40)
- [41. Pattern gates, independent votes, and attachments](#section-41)
- [42. Cycle lifecycle, orchestration, and supporting retrieval](#section-42)
- [43. Transient Memory, imagination, and ATP](#section-43)
- [44. Conversation, vision, and multimodal state](#section-44)
- [45. Feedback, Type A/B nodes, mutation, and PHAGY](#section-45)
- [46. Index Rafts, contacts, commands, and specimen persistence](#section-46)
- [47. APIs, transport, and errors](#section-47)
- [48. Explorer, observability, and operational limits](#section-48)
- [49. Verification and acceptance evidence](#section-49)
- [50. Benchmarks and qualification thresholds](#section-50)
- [51. Migration, rollback, and production operation](#section-51)
- [52. Required decisions and final conformance rule](#section-52)
- [Appendix A. Proposed record and service contracts](#appendix-a)
- [Appendix B. Canonical failure and race acceptance scenarios](#appendix-b)
- [Appendix C. Original WDBX feature preservation inventory](#appendix-c)
- [Appendix D. Original Neural Specimen acceptance matrix](#appendix-d)
- [Appendix E. Implementation companion packet index](#appendix-e)
- [Appendix F. Sources, lineage, and authoring evidence](#appendix-f)

<a id="section-1"></a>
## 1. Authority, terminology, and evidence status

### 1.1 Normative language

**MUST** and **MUST NOT** express requirements for a profile that adopts this specification. **SHOULD** permits an exception only with a recorded rationale and review. **MAY** identifies an optional capability. A requirement is conditional on its named profile: a checked local point index does not require every optional neural model or cluster feature.

All newly specified behavior is **Proposed** until implemented and supported by relevant evidence. “Current” requires identified source and appropriate passing evidence; “Partial” means the supporting evidence covers only a declared subset. “Research” describes a nonproduction experimental boundary and is not a stronger assurance level.

Document revision 6.0 is not an on-disk v6 format, API v6, product release, or claim that the original requirements are all implemented. Existing v1/v2/v3 format names retain their meanings.

### 1.2 Authority order

The user's explicit decisions and ratified project constitution govern intended behavior and permissions. Existing cryptographic commitments and frozen public contracts govern compatibility. Original WDBX and Specimen specifications govern their respective subsystems. Source and tests establish implementation state, but do not convert a bug into desired behavior. This document defines proposed additive integration. The v5 implementation plan supplies task routing, not permission to contradict these authorities.

When two authorities disagree, the implementation MUST preserve the disputed data, register the discrepancy, and block only the affected behavior. It MUST NOT silently choose a more convenient interpretation. A compatibility adapter may preserve historical behavior while a new version implements the corrected contract.

### 1.3 Source basis and limits

The directly supplied v5 plan [P1] is the primary consolidation source. Its specification contracts, corrected architecture catalogue, 58 task packets, and 73 original-specimen acceptance rows are retained by reference and mapped in the appendices. Its historical repository observations refer to WDBX revision `08a69b3f22db61093e06527b3308b10589687022`; they are not a fresh runtime audit performed while writing this specification.

The original specimen source was selectively re-read for attachment, orchestration, supporting retrieval, Transient Memory, and ATP details [P2]. This document does not pretend that every source file or every original paragraph was freshly re-audited. Before implementation, the source lock MUST pin the complete original 26-chapter document, all seven directive documents, actual workspace manifests, API schemas, and compatibility fixtures.

Rust's official release notice confirms 1.99.1 as a released toolchain [E1]. That establishes availability, not WDBX build qualification. The production profile excludes nightly portable SIMD, which the consulted official documentation marks experimental [E2].

### 1.4 Change control

Every amendment MUST identify affected requirement IDs, original authority, schema/wire impact, migration and rollback implications, acceptance tests, approval owner, and effective profile. Changes to member erasure, key custody, learning approval, original specimen interpretations, or production authority remain separately gated.

This writing request authorizes the specification artifact only. It does not authorize repository commits, pushes, deployments, credential changes, production-data access, destructive migration, or key destruction.

<a id="section-2"></a>
## 2. Mission, boundaries, and non-goals

### 2.1 Mission

WDBX MUST preserve enough context for a caller to distinguish what was stored, what was inferred, what was observed, what was consulted, and what was approved. A query must be able to return relevant content together with identity, version, temporal applicability, source lineage, evidence dimensions, scope, and computation provenance.

The system MUST support local operation without a remote service for core storage and reference queries. It MUST permit optional hardware and distributed execution without making canonical data dependent on a particular device, vendor runtime, or index format.

### 2.2 Ownership

| Boundary | Owns | Does not own |
|---|---|---|
| WDBX substrate | Durable records, causal versions, commitments, snapshots, retrieval contracts, index manifests | Unrestricted external tool execution or autonomous approval |
| ABI control plane | Identity, policy, consent, task orchestration, tool authorization, runtime governance | Rewriting historical signed data |
| Neural Specimen runtime | Pattern preparation, node gates, votes, composition, bounded internal state changes | Bypassing ABI authority or inventing durable truth |
| Compute adapter | A bounded operation on identified inputs under an output contract | Canonical commit authority or permission to export data |
| Studio/explorer | Authorized visualization, inspection, explicit user requests | A hidden second canonical database or implicit mutation rights |

The same process may host multiple boundaries. Process co-location does not collapse their contracts. For embedded use, a trusted host may supply policy decisions; a client-supplied tenant string is not a policy decision.

### 2.3 Non-goals and retained research

WDBX does not claim that a vector similarity establishes truth, a signature establishes semantic correctness, a projected cluster establishes causation, or a neural specimen establishes consciousness. It does not claim arbitrary neural inference on every NPU, production FHE, Byzantine-fault-tolerant replication, global serializability, or verified accelerator speedups without corresponding evidence.

Existing research codecs and homomorphic-encryption demonstrations remain preserved, callable within their existing compatibility boundaries, and explicitly research-scoped. Deferring a capability from an initial release does not remove it from the preservation inventory.

<a id="section-3"></a>
## 3. Conformance profiles and release scope

A release MUST identify exactly which profiles and versions it implements. Profiles compose through explicit dependencies; a product MUST NOT advertise all-profile conformance after qualifying one local path.

| Profile | Required behavior | Excluded claims |
|---|---|---|
| `spatial-checked-v1` | Validated point queries, legacy compatibility, complete reference scan, qualified octree path | New shape persistence, hardware execution, tenant service |
| `spatial-durable-v1` | Versioned geometry, frames, observation identities, pinned views, verified index publication, trusted scope binding | Cross-host consistency |
| `hybrid-evidence-v1` | Authorized vector/lexical/spatial/graph/time retrieval, explicit completeness and eight evidence dimensions | Automatically trusted conclusions or learned ranker quality |
| `explorer-v1` | Read-only authorized scene/projection API, bounded rendering, text equivalent | Mutation authority or actual accelerator placement |
| `accelerated-<backend>-v1` | Identified operation/model/device/build qualified against its reference contract | Other devices, shapes, models, or vendors |
| `cluster-causal-v1` | Authenticated peers, causal replication, durable ACK classes, dependency-closed reads, honest fan-out, restartable movement | Global linearizability or Byzantine tolerance |
| `specimen-compatible-<runtime>-v1` | All applicable original chapters, R01–R23 and I01–I24 accepted for that runtime | Browser/native/distributed parity inferred from one test |
| `production-<profile>-v1` | Named underlying profiles plus operational, security, capacity, restore, rollback and operator acceptance | Unqualified optional features |

The constitution's C0–C7 evidence ladder is separate. A profile and a claim level MUST NOT be substituted for each other. An available feature may remain operationally disabled or unqualified. A test skipped for lack of hardware MUST remain untested rather than passed.

<a id="section-4"></a>
## 4. Global invariants

| ID | Invariant |
|---|---|
| WDBX-INV-001 | Historical committed bytes and signatures remain unchanged. |
| WDBX-INV-002 | Entity identity, content identity, causal version, dense slot, and logical Pattern ID are different concepts. |
| WDBX-INV-003 | Concurrent maximal versions remain visible until an explicit authorized resolution. |
| WDBX-INV-004 | Derived indexes can be discarded without losing canonical data. |
| WDBX-INV-005 | Every new query binds a declared read view, policy scope and coverage contract. |
| WDBX-INV-006 | Authentication, authorization, evidence quality and ranking are independent checks. |
| WDBX-INV-007 | Candidate completeness, numeric accuracy and work completion are reported separately. |
| WDBX-INV-008 | Physical geometry, high-dimensional embeddings and visual projections never share an implicit coordinate meaning. |
| WDBX-INV-009 | Commit and cancellation have one authoritative race outcome; a committed operation is never reported as not committed. |
| WDBX-INV-010 | Resource accounting includes queued, running and uncertain remote/device work. |
| WDBX-INV-011 | Hardware execution never independently authorizes or commits persistent knowledge. |
| WDBX-INV-012 | Similarity, integrity and signed provenance never automatically become truth or applicability. |
| WDBX-INV-013 | Original specimen gates, jitter exceptions, voting independence, learning approval and mutation lineage survive integration. |
| WDBX-INV-014 | The strict specimen comparison cap applies across all participating workers. |
| WDBX-INV-015 | Secrets and private runtime content do not enter default logs, examples, public source manifests or exports. |
| WDBX-INV-016 | Old readers and writers are used only under a tested compatibility matrix. |
| WDBX-INV-017 | Missing evidence is represented as unknown, not fabricated as success or zero risk. |
| WDBX-INV-018 | No profile becomes production-qualified solely because it compiles or its plan exists. |

Acceptance MUST exercise the invariant in the actual owning runtime. A name match in source, a placeholder test, or a zero-test successful exit is insufficient.

<a id="section-5"></a>
## 5. Logical and physical architecture

WDBX retains `abi-foundation`, `abi-telemetry`, `abi-compute`, `abi-core` and `abi-wdbx` identities unless an approved compatibility change requires otherwise. Device implementations depend on shared compute contracts; the store does not depend on a specific GPU runtime. New modules are added at measured seams, not by preemptively creating a crate per feature.

```text
Client / CLI / MCP / Studio
          |
Identity, consent and policy verifier
          |
Authorized read/write context + budgets
          |
Query planner / mutation admission / specimen adapter
          |
+----------------------+-------------------------+
| Canonical authorities| Rebuildable projections |
| v1 compatibility     | Exact/HNSW/lexical      |
| v2 causal store      | Spatial/graph/temporal  |
| v3 episode ledger    | 3D layout/model caches  |
+----------------------+-------------------------+
          |
Qualified bounded compute and replication
CPU | Metal/CUDA/Vulkan | neural provider | authorized peer
```

The v2 store and v3 episode ledger may be independent authorities. A unified API MUST NOT imply an atomic transaction across them unless a protocol supplies that property. A publication marker may make a completed bundle visible as a unit while leaving its earlier object writes independently durable.

Physical storage layout is discovered from the current source before implementation. New index folders or manifests MUST be additive and namespaced; they MUST NOT replace the existing journal/checkpoint layout by assumption. Portable exports describe logical contents and content identities, not mandatory host paths.

<a id="section-6"></a>
## 6. Identity and domain model

### 6.1 Identity classes

`RecordId` retains historical numeric and UUID forms. `VersionId` retains the source-defined causal identity. `Digest` identifies immutable bytes under a named digest/canonicalization profile. `DenseSlot` identifies a generation-local array position only. `PatternKey` groups compatible specimen candidates and is deliberately not unique per node.

New logical types include `DatabaseId`, `ScopeId`, `SpecimenId`, `CycleId`, `OperationId`, `AttachmentId`, `EmbeddingSpaceId`, `FrameId`, `IndexGenerationId`, `QueryId`, `PolicyEpoch`, `ModelId`, `WorkerId` and `ShardId`. They MUST have explicit validation and serialization rules. Their names here describe contracts; they are not claims of existing Rust definitions.

New durable observation identity is:

```text
SpatialObservationKey = (RecordId, VersionId, AttachmentId)
VectorObservationKey  = (RecordId, VersionId, EmbeddingSpaceId, component_id)
GraphEdgeKey          = (edge_id, edge_version)
SpecimenEntryKey      = (SpecimenId, node_ref, entry_ref)
VoteAlternativeKey   = (SpecimenEntryKey, slot_ref)
```

Mapping a UUID or wide legacy ID into `u32` by truncation is prohibited. Dense-slot maps MUST be injective over the active observation set, round-trip each key, and fail before publication on slot exhaustion.

### 6.2 Canonical records and attachments

A canonical entity describes one independently versioned item. Large payloads MAY be content-addressed blobs referenced from the item. A derived artifact MUST name its source versions, not merely their current logical IDs.

A new spatial attachment MUST contain its schema/profile ID, source observation reference, geometry, frame registry reference, valid-time interval, creator context, scope binding, and attachment content identity. A new projection attachment additionally binds the input cohort and algorithm profile. Neither modifies the source signed episode.

Mutation metadata MUST distinguish a source assertion from a model inference, transformation, import, repair, or user approval. It MUST preserve provenance through derived records. A new causal version is not a replacement for historical bytes.

### 6.3 Absent, unknown and invalid

Absent fields, unknown values, explicit zero, false and invalid evidence have different meanings. They MUST not collapse through serialization defaults. Missing outcome evidence is `unobserved`, not success. An unknown coordinate frame is not the world origin. A missing vector is not a zero vector. A rejected signature is not an unsigned record.

<a id="section-7"></a>
## 7. Schema, canonical encoding, and compatibility

Each schema registration MUST specify a namespace, schema revision, field meaning, encoding profile, size/depth limits, critical extensions, reader/writer floor, and positive/negative golden fixtures. Canonical encoding MUST be deterministic under its exact registered profile; ordinary transport JSON is not automatically a commitment input.

Historical v2 serialization and parent ordering remain unchanged. Historical v3 `abbey-cbor-episode-v1` commitments and detached Ed25519 signature meanings remain unchanged. A new header, signing context, canonicalization rule or parent representation requires a new profile. Sorting old parent arrays and rehashing them as a “cleanup” is prohibited.

Unknown critical fields MUST cause refusal. Unknown noncritical fields MAY be preserved opaquely where the schema permits it; an old writer MUST NOT read and rewrite a new record while discarding unknown authoritative data. Format negotiation occurs before mutation admission.

New canonical numeric fields MUST specify signed-zero handling, non-finite rejection, integer width, string encoding and map ordering. Geometry's mathematical equivalence does not authorize normalizing existing signed `-0.0` bytes. A canonical content digest excludes build duration, thread arrival order and other diagnostic-only metadata unless those are explicitly part of the event being committed.

Schema fixtures MUST test cross-version reads, unsupported writes, unknown critical fields, changed field order, duplicate map keys, oversized nesting, altered signatures and legacy golden bytes. A second-language digest witness proves only the parts it actually reconstructs; it is not a signature verifier unless it independently verifies signatures.

<a id="section-8"></a>
## 8. Mutation, commit, idempotency, and cancellation

### 8.1 Admission

A mutation request MUST carry or obtain an operation identity, authenticated principal, authorized write scope, schema/profile, expected preconditions, input digest, requested durability and resource budget. Admission checks policy and size before expensive processing. Retrying the same operation with a different input digest MUST fail with `OperationConflict`.

The mutation pipeline is:

```text
Received -> Validated -> Authorized -> Reserved -> Staged
         -> CommitDecision -> DurableCommit -> ReceiptAvailable
```

`Rejected`, `CancelledBeforeCommit` and `FailedBeforeCommit` terminate before canonical mutation. If publication or its response cannot be determined, the operation remains `OutcomeUnknown` until reconciliation. A client MUST query status or retry the same idempotency key, not create a new mutation identity.

### 8.2 Commit ownership

The canonical owner alone makes the commit decision. GPU, NPU and remote compute results are inputs to validation, not commits. On a single authoritative mutation stream, the commit-versus-cancellation decision MUST be synchronized so only one terminal meaning wins.

If commit wins and the client disconnects, the durable operation remains committed. Reporting `CancelledBeforeCommit` in this case is forbidden. A logical specimen cycle that is cancelled before its commit may publish no partial learned edits; separately admitted audit/job records remain separate named operations and MUST be disclosed as such.

### 8.3 Idempotency lifetime

Deduplication state MUST survive restart for the accepted replay window. The window and retention mechanism MUST prevent a retry from silently executing twice after a crash. Expiration of an operation's deduplication record MUST be explicit to clients; uncertainty is not resolved by blindly resubmitting an expired key.

A response receipt MUST bind operation ID, input digest, canonical commit identity, durability actually achieved, policy context, and terminal status. A retry returning the receipt does not re-execute feedback random draws, mutation, model training or external actions.

<a id="section-9"></a>
## 9. MVCC, causal versions, and read views

### 9.1 Version preservation

Independent writers retain their causal journals and observed heads. Two updates are concurrent when neither causally dominates the other. Wall-clock order does not settle that conflict. Explicit resolution MUST identify the versions being resolved and causally dominate the named set. Presentation preference MUST NOT erase nonpreferred maximal versions.

New queries support `AllMaximalVersions`, `PreferredView` and `RejectConflicted`; the default is `AllMaximalVersions`. A limit counts observations unless an explicit record aggregation policy says otherwise. Authorized responses distinguish observation counts and logical entity counts.

### 9.2 Read-view composition

A `ReadView` MUST bind the database, immutable v2 transaction frontier, relevant v3 root, frame registry, conflict mode, policy/consent context, index generations and lifetime. Matching writer sequence numbers with different transaction digests is an integrity failure.

A combined v2/v3 view MUST state `observed_roots` unless an atomic-publication protocol establishes stronger semantics. A dependency-closed causal view MUST include the dependencies needed by every visible selected object. It cannot be manufactured by taking independently newest shard states.

### 9.3 Lifetime and revocation

Snapshots use immutable ownership, such as retained `Arc` values, within a trusted process boundary. Queries MUST not hold unbounded snapshot pins. Eviction or expiration returns an explicit outcome; pagination does not silently switch to current data.

Policy is rechecked at materialization and stream boundaries. Revocation MUST stop new disclosure, invalidate protected caches and terminate/rebind active responses as required. It cannot erase plaintext already borrowed by trusted code or already exported. The implementation MUST minimize those borrows, document exposure limits and never claim retroactive memory revocation from an epoch change alone.

<a id="section-10"></a>
## 10. Durability classes, WAL, recovery, and checkpoints

### 10.1 Durability vocabulary

| Class | Meaning of acknowledgment |
|---|---|
| `volatile` | Accepted into an explicitly nonpersistent fixture/cache; process failure may lose it |
| `local_durable` | Committed objects and required publication metadata have crossed the documented local durability boundary |
| `replicated_durable` | The configured number and placement of replicas have acknowledged durable storage of the exact committed identity |

The receipt MUST identify the actual class. No persistent product write may silently fall back to volatile memory. A replicated-durable ACK is not proof of globally linearizable reads or Byzantine safety. A replica count without failure-domain placement does not prove tolerance of a host or zone outage.

### 10.2 WAL and journal behavior

Existing WAL and causal transaction formats remain authoritative. New readers validate frame sizes before allocation, detect truncation and corruption, and distinguish a recoverable incomplete tail from corruption in an earlier acknowledged object. CRC detects accidental framing corruption; signatures and authenticated encryption have separate roles.

Recovery MUST replay complete commits idempotently, preserve causal conflicts and original IDs, verify referenced immutable objects, and expose unresolved integrity failures rather than fabricating a healthy store. A salvage mode is explicit, read-only by default, and reports omitted data; it is not ordinary successful recovery.

### 10.3 Checkpoints

A checkpoint represents identified canonical roots. Publication MUST use the platform's tested flush and atomic-reference mechanism, including directory durability where required and supported. It MUST never assume that an in-memory rename call alone proves power-loss durability on every filesystem.

A failed checkpoint leaves the previously valid generation usable. Cleanup MUST not delete the last usable checkpoint, live reader dependencies, unreplicated heads, protected backups or held content. Original recovery copies remain protected under existing user constraints.

### 10.4 Required recovery states

| Failure point | Required visible result |
|---|---|
| Before commit decision | No canonical effect for the operation |
| After durable object, before bundle marker | Orphan/staged objects may exist; bundle is not complete |
| After commit, before receipt delivery | Status resolves committed; no duplicate effect |
| During checkpoint/index staging | Previous verified generation remains usable |
| Corrupt derived index | Reject/rebuild index or use complete scan; canonical data stays intact |
| Corrupt canonical object | Integrity failure/quarantine; no silent repair from an untrusted derivative |
| Disk full | Typed failure with terminal commit status reconciled |
| Crash during rekey/erasure | Follow separately approved key transaction protocol; never infer success |

<a id="section-11"></a>
## 11. Blobs, immutable bundles, compaction, and garbage collection

Large model artifacts, source documents, images, traces, meshes and tensor blocks SHOULD be stored as bounded content-addressed objects with explicit scope and retention. References MUST contain a digest, content/media type, size, codec and encryption context where applicable. A content hash is not permission to retrieve an object.

A bundle manifest MUST list members, their immutable identities, schema profiles, source roots and completeness conditions. A `BundlePublished` marker is valid only after all required members are verified. Readers distinguish staged, complete, partial and corrupted bundles. Publication visibility is atomic only at the marker boundary; member writes are not retroactively one cross-store transaction.

Compaction MUST preserve logical observations, unresolved conflicts, signatures and supported historical reads. Lossy vector compression is an explicit representation choice, not a valid operation on lossless evidence fields. Rebuilding a search index does not permit dropping canonical vectors needed by the selected exact/rerank profile.

Garbage collection uses a bounded reference graph rooted in current canonical state, live snapshots, retention holds, accepted receipts, required replicas and protected recovery generations. It must first produce an inspectable reachability decision. Removing payloads, invalidating a derivative and deleting a decryption key are separate actions with separate authorization. A mark/sweep pass MUST defend against changes between mark and delete through snapshot binding or revalidation.

<a id="section-12"></a>
## 12. Coordinate frames and geometry types

### 12.1 Three spaces

Physical/virtual Cartesian geometry, semantic embedding vectors and visual layout coordinates MUST have separate namespaces and type constructors. The expression “3D vector” is not enough to select one. Legacy unbound points remain explicitly unitless until a frame binding is supplied; the system MUST NOT guess meters or a geographic location.

### 12.2 Initial geometry profile

`cartesian-f64-v1` uses finite `f64` coordinates bounded in magnitude by `1e150`. Radii are finite, nonnegative and at most `1e150`. These are proposed representability limits inherited from v5, not physical measurements. Existing `f32` records remain byte-identical; widening finite `f32` into this view is explicit and nonmutating.

Validated types have private components and checked constructors. `Point3`, `Aabb3` and `Sphere3` form the required initial shape set. AABBs permit degenerate axes; minima MUST NOT exceed maxima. Signed zeros are geometrically equal. No constructor silently clamps invalid coordinates or radii.

`Ray3`, oriented boxes, segments, triangles, meshes, voxel grids and swept volumes are optional extensions. Their availability MUST be advertised by a separately specified predicate profile; their names in the catalogue do not imply implementation.

### 12.3 Frame registry

A frame record identifies units, origin convention, handedness, axis interpretation, transform version, valid interval, scope and provenance. Initial cross-frame conversion MAY support only reviewed rigid transforms and explicit unit scales. Unsupported conversions fail rather than approximating silently. Transformed geometry is revalidated against the numeric domain.

Geographic longitude/latitude MUST NOT be evaluated as Cartesian meters. A geodetic extension requires its own coordinate-reference specification, ellipsoid/datum rules, altitude interpretation, transformation evidence and acceptance corpus. Pose uncertainty MAY be recorded but is not collapsed into a deterministic point without an explicit query policy.

<a id="section-13"></a>
## 13. Geometric predicates, accuracy, and ordering

### 13.1 Required predicates

The durable spatial profile MUST support point-in-AABB, point-in-sphere/radius, AABB intersection, sphere intersection and point nearest-neighbor search. Shapes use closed boundaries: touching counts as intersection, and zero-radius membership selects coincident points. Queries MUST state their coordinate frame and numeric profile.

For Cartesian points, the membership contract for a sphere is the comparison of squared Euclidean distance with squared radius over the represented inputs. The initial exact-real predicate profile treats each finite binary input as an exact dyadic value for the decision. It does not claim that the input is an infinitely precise measurement of the physical world.

AABB containment is componentwise comparison of represented coordinates. Sphere-sphere intersection compares center distance with the sum of radii. Shape nearest distance MUST specify distance to volume versus boundary; the initial point-neighbor profile MUST NOT ambiguously reuse one for the other.

### 13.2 Three independent assurances

A complete scan proves that every eligible observation was considered. A defined floating-point kernel specifies arithmetic and rounding behavior. An exact predicate comparator establishes the Boolean/order decision over represented inputs. These are distinct assurances.

The float reference MUST state accumulation type, reduction order, FMA policy, signed-zero rules and handling of subnormals. The exact comparator MAY use conservative intervals and a bounded dyadic fallback. If a fast interval cannot prove a comparison, it MUST fall back rather than guess. Arbitrary epsilon expansion that changes a closed-set predicate is prohibited unless the query explicitly requests a separately named tolerance geometry.

Distances displayed as `f64` are rounded measurements. They MUST NOT be the sole ordering key for a profile promising stronger exact ordering. If two exact keys differ but round to equal displayed values, the exact order remains authoritative.

### 13.3 Stable result order

New durable spatial hits sort by metric key, then stable record identity, source version and attachment identity. The legacy checked index may additionally use an insertion ordinal to distinguish duplicate local numeric IDs. Neither index traversal order nor worker arrival order determines ties.

`k = 0` returns a complete empty selection without searching. `k` larger than the eligible population returns all available observations, not a partial result. Negative radius, NaN, infinity, malformed boxes and out-of-profile coordinates fail before search. A missing embedding or shape is handled according to explicit join semantics, not filled with a fabricated zero.

<a id="section-14"></a>
## 14. Spatial index implementations and legacy adaptation

The original octree remains the first optimized candidate path. New code MUST wrap or extend it through additive checked APIs while preserving historical `Point3D`, insertion and result signatures where required. No initial migration may cast durable `RecordId` into the octree's local ID without a checked generation-local map.

The checked path MUST validate both newly inserted records and records already present through legacy insertion. Query validation alone cannot sanitize an index containing non-finite legacy points. A checked view encountering invalid stored input either rejects with an integrity/validation outcome or uses an explicitly approved exclusion policy reported to the caller; silent omission is forbidden.

Every index implementation MUST declare supported predicates, metrics, numeric profile, update behavior, memory budget and fallback. Euclidean and Manhattan point queries MAY use sound spatial bounds. Cosine on a 3D legacy point remains a separate nongeometric metric and MUST use a complete scan unless a sound pruning rule is qualified.

The index abstraction MUST preserve an immutable observation map and expose build, query, health and coverage metadata. Octree, R-tree, BVH and grid implementations are alternatives behind that contract, not competing authoritative stores. Replacement requires benchmark evidence covering clustered, uniform, planar, collinear, coincident, boundary-heavy, skewed and mutation-heavy workloads.

No indexed prune may exclude a potential winning observation under the advertised exactness profile. A reference fallback is required when a bound is unqualified. Bounded tree depth and allocation accounting MUST prevent degenerate data from causing unbounded recursion, empty-child expansion or memory growth.

<a id="section-15"></a>
## 15. Derived index generations and publication

### 15.1 Manifest contract

An index manifest MUST bind:

```text
index_kind, format_version, algorithm_id, algorithm_version
source_database, canonical_roots, causal_frontier
schema_profile, numeric_profile, frame_registry_digest
scope_or_cohort_identity, policy_binding_strategy
observation_map_digest, conflict_mode
build_parameters, deterministic_seed_when_used
chunks[{digest, length, codec}], observation_count
required_reader_profile, completeness_evidence
```

A manifest's content identity is computed from its immutable semantic fields. Runtime build duration and machine diagnostics are evidence attachments, not accidental identity inputs. A checksum detects damaged bytes; it does not establish that an attacker-supplied index faithfully represents the source. Secure profiles MUST authenticate or verify that representation before use.

### 15.2 Generation lifecycle

```text
Absent -> Building -> Sealed -> Verified -> Published -> Active
                          \-> Rejected
Active -> Retired -> EligibleForCollection
```

A build reserves resources, pins source roots, writes a private staging area, seals chunks, verifies identities and coverage, and publishes a manifest. An activation reference advances through the storage platform's tested durable synchronization mechanism. Concurrent builders cannot overwrite an unrelated winning generation. Failed or cancelled staging remains unreachable or is safely cleaned up later.

An active generation is immutable. Updates produce a delta or new generation; they do not rewrite data underneath a retained reader. Retired generations remain while legitimate pins or recovery requirements reference them.

### 15.3 Coverage against a read view

Coverage is `Complete`, `ReconciledWithDelta` or `Unusable`. Delta reconciliation MUST cover inserts, replacements, removals, tombstones, maximal-version changes, frame changes and policy/cohort changes affecting visibility. A delta containing new points but not removed old positions is incomplete.

If a complete delta cannot be established, an exact query scans the canonical view or returns a typed resource/availability error. It MUST NOT serve a stale index while claiming exact snapshot coverage. An explicitly best-effort approximate request may use stale state only with declared bounds and freshness metadata.

<a id="section-16"></a>
## 16. Embedding spaces and model registry

### 16.1 Space identity

An embedding space is identified by immutable model and preprocessing identity, modality, dimensions, dtype, normalization, metric, component semantics and quantization lineage. Equal dimensionality alone does not make embeddings comparable. A model alias is a lookup convenience, not an immutable space identity.

Vector admission MUST validate dimensionality, finite components, byte count, declared space, scope and storage budget. Existing format limits remain enforced; larger new profiles require explicit schema/reader support. Zero-vector behavior MUST be defined per metric. In a cosine compatibility profile, the historical behavior is preserved; new profiles MUST not silently reinterpret it.

### 16.2 Model manifests

A model manifest MUST record source, immutable artifact digest, license information, tokenizer/preprocessor digests, input/output signatures, runtime conversion, quantization, memory estimate, approved backends and input privacy class. Loading a model verifies bytes before execution. Downloads or provider activation require explicit permission; `auto` dispatch does not authorize fetching weights.

Model changes create new space identities or a separately validated mapping. Re-embedding preserves source lineage and old vectors required for reproducibility or rollback. New embeddings become visible only when their source/model binding is complete. A partial re-embedding run MUST not mix spaces in one unnamed index.

Training or fitting learned codecs/rankers is an explicit job with dataset consent, model lineage, resource limits and promotion criteria. Specimen feedback MUST NOT silently update pretrained neural weights.

<a id="section-17"></a>
## 17. Exact vector retrieval, HNSW, and batch operations

Core retrieval MUST retain exact search and existing HNSW behavior. An exact reference evaluates every eligible vector under a named scoring profile and stable total order. A batch operation MUST specify whether errors reject the batch atomically or return per-item outcomes; it cannot silently omit failed elements.

HNSW manifests bind observation IDs, source view, embedding space, build parameters and relevant randomness. Search parameters such as candidate budget are explicit. Tombstones, updates and conflicts must have defined visibility. A new index MUST not turn preferred-version presentation into causal conflict resolution.

Approximate search reports candidate coverage as bounded approximate even after exact reranking. Recall MUST be measured against the authorized exact reference using identical spaces and aggregation rules. An optimization that increases throughput by lowering recall is not a like-for-like speedup unless the tradeoff is disclosed.

Filtered ANN MUST respect the selected privacy profile. Strict isolation builds or selects a permitted candidate structure before traversal. A trusted-domain profile may allow internal filtered traversal under documented leakage assumptions, but forbidden IDs, scores and counts MUST not cross the boundary. Increasing an oversampling multiplier does not prove complete filtered recall.

Batch limits account for input vectors, temporary scores, result storage, candidate expansion and device transfers. Dot, cosine, norm, L2 and top-k operations MUST be independently qualified; passing one does not qualify all.

<a id="section-18"></a>
## 18. Multi-vector records and compression

A multi-vector record groups bounded components under one source record/version. Components may represent document chunks, image patches, audio spans or other declared units. Their identities and offsets remain explicit. Parent aggregation MUST prevent arbitrary chunk duplication from silently amplifying a document's score.

The initial aggregation catalogue contains `MaxSimilarity` and `MeanSimilarity`; the query names the policy and how missing/filtered components are treated. Late interaction is a separate profile with a fully specified query-component/document-component reduction. Mixing parent-level and component-level ranking without stating the final unit is prohibited.

Existing int8, PQ, autoencoder, Huffman and rANS paths remain preserved according to their source-defined boundaries. Lossless and lossy representations MUST be distinguished. A lossy vector manifest records codebook/model digest, source digest, dimensionality, reconstruction error protocol and retrieval-quality results.

If exact reranking requires original vectors, the release MUST retain them or explicitly refuse that exact mode. Decoding a lossy vector cannot recreate the original by assertion. Learned codecs require bounds on artifact size, malformed input, numeric output and decompression work. No codec may rewrite a signed episode's logical evidence fields under the label of optimization.

<a id="section-19"></a>
## 19. Lexical retrieval and hybrid score composition

Lexical indexing MUST identify tokenizer, language handling, normalization, stopword and stemming profiles where used. Source text remains separate from derived tokens. Posting lists and snippets obey scope and version visibility, including tombstones and access changes.

A BM25 profile MUST identify its corpus statistics, parameter values, document-length unit and treatment of repeated/missing terms. Distributed shards cannot silently use incomparable local corpus statistics for a claimed globally exact ranking. Statistics may be computed over an authorized cohort; their scope itself can be sensitive.

Hybrid composition MUST distinguish hard predicates from ranking preferences. A physical region restriction is a predicate unless the caller requests a soft spatial preference. Authorization is always a gate, not a low score. Cryptographic invalidity is a disposition rule, not a weight multiplied by similarity.

A fusion policy names component scores, normalization, weights, missing-value handling and tie order. Reciprocal-rank fusion over truncated local rankings is approximate unless a complete-rank protocol proves otherwise. A score normalized by the maximum distance in a local candidate pool is not automatically comparable across shards.

The historical multiplicative semantic/temporal/causal/persona ranker remains available for compatibility. A new evidence-aware mode is additive and versioned. It MUST report its components and cannot silently replace the old endpoint's output semantics.

<a id="section-20"></a>
## 20. Typed graphs and causal relationships

Graph edges MUST distinguish causal audit parents, temporal succession, asserted domain causation, inferred associations, support, contradiction, supersession, derivation, spatial containment and specimen attachment. Different meanings require different edge types and provenance. Spatial closeness or embedding similarity cannot directly create an authoritative causal edge.

An edge references source and target identities/versions, edge type, creator, scope, validity interval and evidence. A causal audit DAG must preserve its acyclicity rules. A general cognitive graph may contain cycles; traversal must bound hops, visited observations, edges, output paths and work.

A graph query MUST define whether it returns reachable nodes, edges, one path, shortest paths or bounded explanatory paths. “Nearest” on a graph MUST identify the edge cost and admissible edge set. A breadth-first hop count is not a physical or semantic distance by default.

Authorization applies to both endpoints and to relationship existence. A visible endpoint does not make every incident edge visible. Explanations MUST not reveal hidden intermediate nodes. A forbidden path may be reported as unavailable without distinguishing hidden existence from absence to an unauthorized client.

Deletion, quarantine and supersession affect graph views according to explicit lifecycle policy. Memory-edge episodes remain immutable evidence; excluding a quarantined result from ordinary retrieval does not erase its inspectable history for an authorized reviewer.

<a id="section-21"></a>
## 21. Temporal semantics and historical queries

The system MUST distinguish valid time, transaction/observation time and monotonic runtime duration. Wall-clock timestamps do not establish causal order. Historical queries bind both the requested validity interval and the source snapshot used to answer.

New valid-time intervals use a documented half-open convention `[start, end)` with an explicit open end; point events are represented separately rather than by ambiguous zero-length intervals. Historical formats retain their existing meanings. Invalid ordering, arithmetic overflow and unsupported timestamp range fail validation.

Recency decay is a ranking policy with identified reference time and parameters. Replaying a query MUST use the recorded reference time, not silently substitute the wall clock. Staleness may indicate age, source supersession, changed model/policy or changed task applicability; these causes MUST be distinguishable.

A historical snapshot is not an authorization bypass. Current access and consent restrictions control new disclosure unless a specifically authorized historical-access policy says otherwise. Retention may make an old view unavailable; the service MUST report that limit rather than reconstruct a fictional complete past.

<a id="section-22"></a>
## 22. Unified query contract and planner

### 22.1 Request semantics

A query request declares target collections/spaces, predicates, ranking policy, conflict mode, requested consistency, exactness profile, result unit, `k`/page limit, deadline, work/memory budgets and whether partial output is permitted. Identity and policy are resolved independently of client claims.

The planner performs validation, scope binding, snapshot selection, selective candidate routing, exact predicate checks, aggregation, ranking, materialization and final policy recheck. Plans MUST record a digest and relevant algorithm versions. Cost estimates influence execution order, never authorization or canonical semantics.

### 22.2 Outcome contract

```text
QueryOutcome
  query_id, read_view_id, query_digest
  hits[]
  candidate_coverage: exhaustive | bounded_approximate
  numeric_accuracy: exact_real_predicate | defined_float_profile | bounded_model_error
  completion: complete | partial
  partial_reason?: deadline | work_budget | result_cap | unavailable_partition
  consistency: local_snapshot | causal_cut | best_effort
  ranking_policy_id, observed_generations[]
  execution_evidence_refs[]
  continuation?: authenticated_opaque_cursor
  explanation_summary
```

For a top-k query, returning k results is not a `result_cap` partial condition if the top-k computation completed as requested. A radius enumeration stopped by an output safety cap is partial unless a complete continuation is provided under the requested contract. Fewer than k results can be complete when the eligible population is smaller.

An exact request unable to finish fails with a typed error unless the caller explicitly allowed partial results. `complete + bounded_approximate` means the approximate plan completed; it never means exhaustive coverage.

### 22.3 Cursor and pagination

A cursor binds query digest, read view, ranking policy, stable sort boundary, authorization context and expiry. It MUST be authenticated or stored as opaque server state. Changing a query or policy invalidates reuse. Snapshot expiry is explicit; the server cannot restart on current data while presenting it as the next historical page.

Deduplication occurs at the requested observation/record level. Replica duplicates are not distinct results. Different maximal causal versions remain distinct unless the selected aggregation contract says otherwise. Pagination tests MUST cover equal scores, concurrent versions, revoked access, expiry and replay.

<a id="section-23"></a>
## 23. Eight-dimensional evidence model

Evidence is returned as separate typed dimensions, each with provenance and an explicit unknown state.

| Dimension | Required representation |
|---|---|
| Cryptographic | Valid, invalid, unsigned or unverifiable; profile/key status and verification context |
| Source | Identified origin, asserted role and scoped reputation evidence, not assumed authority |
| Calibration | Sensor/model calibration identity, applicable range and health evidence where known |
| Outcome | Validated, contradicted or unobserved; expected and actual outcome references |
| Compatibility | Schema, model, policy and contract compatibility under the current query |
| Regime | Applicability to the current task/environment/safety constraints, or unknown |
| Lifecycle | Age, retention, quarantine, supersession, erasure restrictions and holds |
| Confidence | Recorded uncertainty or interval with method and interpretation |

Invalid cryptographic evidence MUST not be treated as valid content. In a protected profile it is rejected from ordinary retrieval and quarantined for authorized inspection. Unsigned legacy content and unverifiable signatures are distinct and follow a declared policy. A valid signature does not settle source trust or observed correctness.

`retrieved_parent_blocks` identifies what a decision actually consulted; causal parents identify history/dependency. These relationships MUST remain separate. Outcome observations link back to the prediction/action being evaluated, avoiding fabricated retrospective certainty.

An evidence-aware ranker MUST expose its policy and explain the effect of missing/contradicted evidence. It cannot generate default confidence values merely to fill a field. Regime and calibration data absent from old formats remain unknown or separately attached; historical bytes are not reinterpreted to invent them.

<a id="section-24"></a>
## 24. Evidence ranking, learning, and claim promotion

A new ranker is a versioned policy with deterministic inference rules or a separately versioned model. Qualification compares it with a plain vector baseline under equivalent embeddings, storage, candidate budget and authorized corpus. Relevance improvement, evidence calibration and safety disposition are separate measurements.

Training datasets MUST carry source/consent lineage. Evaluation uses held-out cases and records failure modes, including apparently similar but incompatible tasks, signed misinformation, stale facts, conflicting outcomes and unknown evidence. A learned ranking score cannot override an access denial or required cryptographic rejection.

Promotion requires a reviewed evidence artifact naming the profile, source revision, corpus, executed tests, measured outcomes and limitations. Claim levels C0–C7 follow the controlling constitution's definitions. This draft does not redefine that ladder or create a new C8. Historical source claims MUST be distinguished from newly executed qualification.

The claim registry MUST allow regression/demotion when source, model, driver, policy or environment changes. “Qualified once” is not permanent authority. A release manifest records the exact set of qualified combinations and disabled or unverified paths.

<a id="section-25"></a>
## 25. Multiway rewriting and experiment publication

The existing bounded multiway engine MUST preserve canonical states, distinct transitions and every valid event multiplicity. Two different rule applications reaching the same state may share the state/transition but remain separate events. Content identity cannot be replaced by discovery order.

Exact string rewriting retains literal and overlapping matches, deterministic enumeration rules, source configuration identity and bounded expansion. Depth, state count, event count, payload, time and memory budgets are explicit. Exhaustive-in-domain is claimed only when the frontier is exhausted without unexpanded work; hitting a cap produces a valid partial result with a termination reason.

Resume MUST retain frontier/cursor/configuration information necessary for the specified equivalence. A fixed seed does not guarantee identical wall-clock-limited results under different load; determinism claims identify effective limits and scheduling profile. Exported timing diagnostics do not accidentally alter canonical experiment identity.

The existing persistence path's independent mutations MUST not be relabeled as atomic. New complete-bundle publication may use a versioned marker binding state objects, event export, provenance and resume data. Partial persistence remains inspectable and retryable under the same experiment identity.

State/event graphs and token-lineage causal graphs retain their source-defined distinctions. Hypergraph and physics extensions remain explicitly separate research proposals. The engine does not claim to enumerate the complete ruliad or establish fundamental physical laws.

<a id="section-26"></a>
## 26. Semantic 3D projection

Projection records MUST bind embedding space, authorized input cohort, exact source view, algorithm/version, numeric/replay profile, input order, seed where relevant, parameters and output identity. Coordinates are derived navigation aids, not physical positions or canonical semantic distances.

The initial reference uses a bounded CPU PCA profile. It specifies centering, covariance/computation method, ordering, sign convention and a deterministic repeated-eigenspace basis rule within a frozen implementation. Seed alone is insufficient. Portable qualification may compare subspaces, reconstruction and aligned layouts within tolerance; byte replay is reserved for the frozen profile that actually guarantees it.

UMAP or other nonlinear methods require independent dependency/license, determinism, memory and quality review. The implementation MUST state whether new points can be transformed without refitting. A change in model, cohort or policy invalidates affected cached projections.

Strict isolation fits only the permitted cohort or uses an explicitly approved public projection model. Fitting on hidden records and then removing their points can still change visible coordinates; that is not strict isolation. Centroids, density maps, labels and layout-derived summaries are protected outputs.

Qualification measures neighborhood preservation, trustworthiness where applicable, explained variance/reconstruction for PCA, stability across updates, memory, build latency and output volume. A visually attractive cluster is not evidence of truth, causal structure or ranking quality.

<a id="section-27"></a>
## 27. Authorization, consent, and information-flow boundaries

### 27.1 Trusted authorization context

An `AuthorizedReadView` or write capability is created only by a trusted verifier. It binds authenticated principal, scope, operation, policy/consent epoch, purpose where required, expiry and resource restrictions. The service MUST NOT deserialize such authority directly from an untrusted request body. Bearer authentication and persona affinity alone do not establish row-level permissions.

Policy applies to canonical content, candidate traversal, score/distance disclosure, relationship existence, statistics, projection coordinates, cursors, exports, logs and remote dispatch. A request to inspect provenance does not authorize access to every parent payload. Authority for one guild, specimen or compartment is not transitive through a matching name or shared embedding.

### 27.2 Isolation profiles

The strict profile uses authorization-partitioned candidate structures and projection cohorts. It MUST not traverse forbidden records as part of ordinary query work. A trusted-domain profile MAY permit broader internal traversal with output filtering, but MUST document observable timing/cache leakage assumptions and MUST NOT claim strict noninterference.

Result explanations and error messages MUST avoid disclosing forbidden record or shard existence. Administrative diagnostic detail requires separate authority. A failed lookup may intentionally have a common absent/forbidden presentation while retaining restricted audit detail internally.

### 27.3 Revocation and consent changes

Policy changes invalidate relevant cached authorization and derivative views. Materialization and streaming recheck epochs; an invalidated stream stops or restarts explicitly. A pinned data snapshot cannot pin obsolete permission indefinitely. Consent changes affect new learning/admission independently of whether previously retained records remain readable.

Already released plaintext cannot be recalled by changing metadata. The threat model MUST distinguish trusted in-process borrowing, delivered output, authorized exports, third-party copies and live internal caches. Claims about deletion or revocation MUST name which of these boundaries they cover.

### 27.4 Prompt and script injection

Retrieved text, imported patterns, model outputs, contact names and visual/OCR text are data. They cannot become privileged scripts, policy changes or shell commands through interpolation. Explicit Sigil compilation and tool invocation require capability checks and typed argument validation. A record saying “ignore prior rules” has no additional authority because it was retrieved from WDBX.

<a id="section-28"></a>
## 28. Cryptographic protection and key lifecycle

### 28.1 Distinct assurances

CRC protects framing against accidental corruption. A digest binds bytes under its encoding profile. A signature binds those bytes to possession of a signing key. Authenticated encryption protects payload confidentiality/integrity under its assumptions. None independently proves that the payload is true or that an authorized host kept it private.

Existing v2 optional XChaCha20-Poly1305 and Ed25519 object protection remains compatible. Existing detached v3 episode signatures remain detached and are not renamed COSE. A new COSE envelope requires a separately versioned profile, independent verifier, key identity/revocation semantics and fixtures. Plaintext compatibility remains visible; the product MUST NOT claim universal encryption when a supported store may be plaintext.

### 28.2 Key providers and trust roots

Key material MUST remain in approved protected providers or explicitly protected local files according to the selected profile. Keys are not embedded in source, debug output, portable specimens or telemetry. Provider availability, initialization, actual round-trip execution and recovery behavior are independently qualified per platform.

A credential adapter compiled for Linux or Windows is not proof of live provider execution. An owner-only file is not automatically equivalent to a hardware-backed keystore. Platform-specific claims require evidence of the actual provider and access attributes used.

### 28.3 Rotation, rekey and revocation

Rotation changes future key use; rekey changes protected representations; revocation changes acceptance policy. These operations MUST be distinguishable. Historical signature verification MUST identify whether a key was valid at signing, valid now, unknown or revoked under the query's policy. A universal rule that rejects all historical signatures after routine rotation is not assumed.

Rekey uses verified staged generations or the source-defined safe protocol, preserving a recoverable predecessor until permitted cleanup. Crash tests cover each registry/envelope/publication boundary. A recovery operation MUST not restore a revoked or erased key without reconciling the authoritative key/tombstone epoch.

### 28.4 Research encryption

Additive HE, bounded DGHV, secret-key-assisted refresh and optional TFHE demonstrations remain research/reference surfaces. An executed demo is not an independent cryptographic audit, production parameter selection or permission for protective use. Query-over-encrypted-memory extensions require their own threat model, leakage definition, key distribution, correctness and performance evidence.

<a id="section-29"></a>
## 29. Member-fact custody, retention, and erasure

### 29.1 Separate approval boundary

The inherited member-erasure design remains a separately approved program, not incidental spatial work. This specification preserves its intent without authorizing implementation: canonical encrypted member fact text inside WDBX; per `(guild_ref, member)` custody; per-fact keys under member keys; gateway-held wrapped keys; approved Keychain master behavior; signed holds/tombstones/receipts; scope-summary clearing; migration parity and operator-controlled rollback.

The original draft's exact cryptographic choice, access attributes, rotation cadence, automatic cutover conditions and constitutional amendments MUST be pinned and reviewed before coding that profile. Conflicting paraphrases of “hourly” MUST not decide whether a rotation deadline or frequency limit was intended; the original approved wording controls. Writing this document does not clear its approval gate.

### 29.2 Erasure outcomes

An erasure receipt MUST distinguish requested scope, authorization, active holds, logical invisibility, key destruction, derivative invalidation, replica acknowledgments, retained material and residual exposure. A tombstone alone is not physical sanitization. Deleting a live wrapped key is not sufficient if a retained backup can restore it under a surviving master.

Spatial positions, projections, embeddings, graph edges, summaries, caches and worker scratch are potentially member-derived. Their lifecycle MUST be included in the erasure graph. Content-free receipts MUST avoid retaining low-entropy plaintext-equivalent commitments or metadata that defeats the promised privacy boundary.

### 29.3 Holds, propagation and resurrection

A hold is an authorized signed policy record with scope and reason. It blocks only the actions required by its governing policy and is reported rather than bypassed. Future expiry/TTL mechanisms MUST check holds; automatic cleanup is not an exemption.

Cross-node erasure requires an authoritative scope epoch, denial of new materialization and acknowledgments from required participants. Offline replicas are pending, not silently erased. Rejoining replicas reconcile tombstones and key epochs before serving. Restoration from backups must not resurrect forbidden keys, source payloads or derivatives.

WDBX cannot promise deletion from uncontrolled third parties or already delivered exports. Such boundaries MUST appear in the receipt/threat statement rather than being hidden in a generic “success” status.

<a id="section-30"></a>
## 30. Stable Rust and CPU execution

### 30.1 Production toolchain

The qualified production target is Rust 1.99.1, edition 2024. Toolchain and dependency lockfiles are pinned. The existing `unsafe_code = "deny"` discipline remains; audited FFI/intrinsic exceptions require narrow isolation and explicit review rather than workspace-wide relaxation. Production builds MUST not require nightly portable SIMD [E1, E2].

The v5 source record reports that stable lane accumulation was already adopted at the cited WDBX revision. The first execution step therefore qualifies that change instead of rewriting it again. Public labels such as `cpu-avx2` remain compatibility labels; they are not proof that a particular machine instruction was emitted [P1].

### 30.2 Reference kernels

The CPU reference supports core exact vector/spatial operations, deterministic ordering, graph traversal, validation, canonicalization and storage control. Numeric qualification uses independent known-answer and adversarial corpora, not the same implementation called through two adapters.

Reference contracts distinguish historical f32 behavior from stronger new f64 or exact-predicate modes. Changing an accumulation order or zero-vector convention cannot silently alter a frozen compatibility endpoint. Where optimized arithmetic affects a gate near a threshold, the implementation MUST recompute under the authoritative reference or meet the declared threshold-equivalence contract.

### 30.3 Optimization and scheduling

Safe loops and compiler auto-vectorization are preferred initial optimized paths. Architecture-specific kernels MAY be introduced under approved isolation, actual target-feature checks, disassembly/profiling evidence where ISA claims are made, and independent parity. The core remains usable on supported CPUs without an accelerator.

CPU-intensive work runs in bounded compute pools rather than monopolizing async I/O threads. Pools account for query, ingest, index build, recovery and specimen maintenance priorities. Queues are finite, fairness is explicit, and cancellation is cooperative for already-running blocking work. Tokio's documentation specifically warns that started `spawn_blocking` tasks cannot generally be aborted through their handles [E3].

NUMA placement, memory layout, batching and cache reuse are measured optimizations. They MUST not change scope, snapshot, ordering or commitment behavior. A memory fit check applies before scheduling; a fallback is not guaranteed merely because it is called “CPU.”

<a id="section-31"></a>
## 31. GPU execution: Metal, CUDA, and Vulkan

### 31.1 Optional operation adapters

GPU adapters are optional and independently qualified. Candidate operations include batched dot/L2/cosine, vector top-k, quantized decode, projection transforms, spatial broad-phase and compatible model computations. Small irregular HNSW or graph walks MAY stay on CPU. The choice is measured end-to-end, not assumed from hardware class.

Every operation binds kernel binary/source identity, device, driver/runtime, dtype, shape limits, memory requirements, precision, result layout and synchronization contract. Metal success does not qualify CUDA or Vulkan. A feature flag, linked library or detected GPU establishes neither execution nor speedup.

### 31.2 Buffer ownership

Host and device buffers have explicit ownership, scope, source generation and completion fences. A buffer cannot be reused or freed while queued/running work may access it. Unified memory does not remove lifetime or synchronization requirements. Transfers, staging buffers and allocator overhead count against budgets.

Each job is bounded by input size, batch size, command depth, runtime policy and output validation. Cancelling an application future does not prove a kernel stopped. Reservations remain until completion or verified teardown. A driver reset is a failure requiring reconciliation; it does not authorize replay of a mutation under a new identity.

### 31.3 Correctness and fallback

Outputs are checked for shape, finite values, bounds, source binding and required numerical agreement. Candidate kernels cannot omit winners in a claimed exact mode without a coverage proof. GPU broad-phase results require authoritative final predicates. Sampling checks do not prove exhaustive candidate completeness.

Fallback may use CPU only when it implements the same operation/model/output contract within remaining budgets and is permitted by policy. Device-only requirements produce an explicit failure instead. Sensitive device buffers are cleared/released according to supported mechanisms and threat assumptions; perfect physical zeroization is not asserted without evidence.

### 31.4 Performance claims

Qualification measures setup, upload, kernel, synchronization, download and verification, with cold and warm conditions. Speedup comparisons use a qualified optimized CPU baseline at matched accuracy and workload. Kernel-only throughput is labeled kernel-only and cannot be presented as end-to-end query performance.

<a id="section-32"></a>
## 32. NPU, neural providers, and optional TPU

### 32.1 Neural-provider role

NPU support is defined per supported neural graph, model conversion and runtime. Candidate workloads include embeddings, modality encoders, calibrated rerankers and bounded transient inference. Generic database transactions, WAL replay, arbitrary graph traversal and signing remain CPU responsibilities unless a specific verified extension supplies them.

An NPU provider MUST advertise supported tensor types/shapes, model/opset constraints, conversion identity, memory, compilation behavior, execution context and failure semantics. A provider name alone does not identify actual device placement.

### 32.2 Core ML and Apple Neural Engine

Core ML requests record the compute-unit policy, compiled model identity, OS/runtime and actual returned outputs. Requesting a CPU-and-Neural-Engine policy does not by itself prove per-operation ANE placement. Placement claims require supported evidence separate from correct model output [E4]. Unknown placement is reported as unknown, not inferred from hardware availability.

Metal GPU and Core ML neural execution retain distinct evidence. A CPU fallback using the same model may be acceptable under a permitted policy; replacing the model or changing the embedding space is not an equivalent fallback.

### 32.3 Other providers and TPU

Windows ML and other vendor runtimes are optional adapters with device/provider-specific support. Provider, model and runtime identity MUST be recorded; GPU execution must not be relabeled NPU merely because the API is a machine-learning API. Vendor support varies and requires the current published runtime contract plus real-device qualification [E5].

TPU support is an optional local-runtime or remote-service profile, not guaranteed by a backend enum. It MUST declare supported workloads, locality, authorization, quotas, model/compiler identity and transport assumptions. Remote specialized hardware is subject to exactly the same data-export and result-trust restrictions as other remote workers.

<a id="section-33"></a>
## 33. Capability evidence, qualification, and dispatch

### 33.1 Compatibility facts versus qualification

Retain existing capability facts `compiled`, `available`, `initialized`, `executed` and `runtime_verified`. Add a separate qualification record rather than redefining the original public fields. Successful execution alone cannot stand in for independent numerical qualification.

```text
QualificationKey
  backend_implementation, build_fingerprint
  device_identity, driver_runtime
  operation, dtype, supported_shape_domain
  model_or_kernel_digest, numeric_profile

QualificationRecord
  key, corpus_digest, reference_implementation
  executed_cases, numerical_result, threshold_rank_result
  placement_evidence, performance_evidence
  failure_tests, privacy_review
  created_at, invalidation_rules, approved_profile
```

A model, kernel, compiler, driver or relevant policy change invalidates the affected qualification scope. Qualification records are not blanket permanent endorsements of a device.

### 33.2 Policy-first selection

Dispatch first filters by authorization, residency, approved operation/model, qualification, precision, memory and deadline. It then selects using measured cost including queue wait, setup, transfer, computation, verification and network overhead. Backend preference cannot weaken a hard privacy rule.

Policies include `RequireExactBackend`, `PreferLocal` and `AllowAuthorizedRemote` plus an explicit backend allowlist. The fallback chain is finite and acyclic. Each attempted path consumes the same logical job budget; retries do not reset deadlines or memory ceilings.

### 33.3 Outcomes

An outcome identifies requested/effective backend, operation/model identity, qualification, source digest, attempt count, fallback reason and terminal state. `Unavailable`, `Unqualified`, `InsufficientMemory`, `DeadlineExceeded` and `OutputContractViolation` are distinct causes. Output violation quarantines the affected qualified path pending review and never becomes a silent best-effort success.

<a id="section-34"></a>
## 34. Remote compute job contract

A remote job carries job/operation ID, authorized principal/scope, policy epoch, input digest, read-view reference, kernel/model digest, permitted device set, numeric/output contract, budgets, deadline and short-lived authorization proof. Workers reject unknown kernels, mismatched inputs, stale authority and oversized payloads before execution.

The initial cluster assumes cooperating trusted workers. Transport identity and a signed result authenticate a source, not mathematical correctness or erasure of worker plaintext. Confidential data cannot be sent to an untrusted worker merely because TLS is used. A stronger adversarial-compute profile requires an independent protocol and is outside the initial guarantee.

Workers return result bytes/digests, source binding, actual backend evidence and completion state. The coordinator validates output contracts and retains canonical commit ownership. Remote candidate omission cannot be ruled out by checking only returned scores; strict exact search requires a trusted exhaustive route or a stronger coverage protocol.

Retries preserve job identity and return stored terminal outcomes where available. Lost responses are reconciled, not guessed. Cancellation propagates but resources remain reserved while execution may continue. Workers receive no long-lived master/data keys; any permitted plaintext access is bounded by the job and threat model.

A compute worker is not a storage replica unless separately granted that role. A storage replica is not authorized to execute arbitrary supplied code. Both roles use fixed capability inventories and bounded interpreters/runtimes.

<a id="section-35"></a>
## 35. Cross-node membership and control plane

The cluster separates a causal data plane from strongly coordinated control metadata where required. Membership, shard placement, elected ownership and fencing are versioned control objects. The data plane preserves independent writer causality and conflicts rather than replacing them with last-writer-wins.

A node identity is cryptographic and role-scoped; DNS or IP labels do not grant authority. Membership changes record epoch, predecessor, authorized signer, roles, scopes and required acknowledgments. A node removed from membership MUST not serve or accept new delegated work under stale authority.

Non-loopback cluster traffic uses reviewed mutually authenticated encrypted transport. Certificate identity, expiry, revocation policy, scope, protocol version and rotation overlap are tested. The system MUST NOT downgrade to unauthenticated/plaintext traffic after handshake failure.

A reviewed consensus implementation or explicitly single-authority control arrangement supplies the advertised control-plane guarantees. A token or signed membership file alone is not consensus. Split-brain prevention requires enforceable fencing at the resource owner; rejecting a late response at the coordinator is not equivalent to stopping a stale writer.

Placement honors capacity, locality, scope/residency and failure-domain separation. Rendezvous placement may be retained as a baseline but must account for hot shards and heterogeneous capacity through an approved extension. A faster GPU in an unauthorized region cannot attract restricted data.

<a id="section-36"></a>
## 36. Replication, anti-entropy, and rebalance

Replicas transfer exact committed object identities and bytes. Receipt validates bounds, supported schema, digest, required signature and causal dependency handling. Missing parents are queued or quarantined within strict limits; dependent content is not served as fully verified until its required closure is available.

A replicated-durable receipt states the required copy count and failure domains actually acknowledged. Quorum availability does not itself provide linearizable data semantics. Existing conflict-preserving read repair remains; repair must not discard an inconvenient concurrent version.

Anti-entropy compares authenticated manifests or equivalent summaries and transfers only verified missing objects. A matching filename or length is not an identity proof. Resumable transfer binds object digest, generation and verified offsets/chunks.

Rebalance follows copy, verify, tail catch-up, coverage check, fenced cutover and retained recovery. Source removal occurs only after target completeness and applicable retention/replication approvals. An interrupted move resumes without duplicate visible records or lost causal versions. A stale shard-map epoch is either safely redirected or rejected; it cannot silently duplicate commit authority.

Tests include slow/full disks, partial objects, duplicate delivery, reordered transactions, incompatible schema, lost acknowledgments, process restart and membership change during movement. A local multi-process demonstration remains local evidence, not a real-host qualification.

<a id="section-37"></a>
## 37. Distributed snapshots and search

A coordinator authenticates the caller, resolves permitted shards, binds the shard-map epoch and requests a named consistency contract. A `causal_cut` requires a dependency-closed set of source versions across the required shards. Where that cannot be established, the response is best-effort or unavailable—not an invented atomic snapshot.

Shards enforce scope locally and return observation keys, source frontier, index generation, metric/ranking identity, bounded hits and completeness metadata. Missing shard details are disclosed only to authorized callers. Replica duplicates are reconciled without collapsing distinct causal versions.

Exact top-k merging is valid only with globally comparable scoring, identical total order, correct ownership/aggregation and complete sufficient shard results. For record-level multi-vector scoring, a parent's components split across shards may require additional aggregation before top-k; naive local top-k is not automatically sufficient. Global lexical statistics and fusion ranks require an explicit protocol.

An ANN shard may return a completed approximate result. The coordinator cannot turn that into exhaustive search by reranking. Unknown shard coverage, unavailable partitions, timeouts and output limits appear in the outcome or cause failure under an exact-only request.

Pagination binds the distributed cut and shard map. Rebalance cannot silently change the meaning of a cursor. A query that survives a routing change must prove it still references the same logical view and ordering; otherwise it returns an explicit retry/expiry outcome.

<a id="section-38"></a>
## 38. Strict distributed resource accounting

### 38.1 Per-specimen comparison cap

The original default cap is **1,000 simultaneous high-resolution comparisons per specimen**, including ordinary, attached and remote scans. It is not 1,000 per node, thread, GPU kernel or coordinator. A user may configure the limit within governing hard ceilings.

The strict accounting invariant is:

```text
active_local + outstanding_remote_credits + uncertain_remote_credits
    <= configured_specimen_cap
```

These are disjoint accounting states. A remote allocation transitions from outstanding to uncertain; it is not counted in both. Credits cover queued and executing logical comparisons conservatively. A batch reserves one credit per actual high-resolution comparison rather than one per dispatch.

### 38.2 Partition and cancellation behavior

Lease expiry does not prove work stopped. Credits assigned to a disconnected worker remain uncertain and cannot be reissued until the trusted protocol proves cessation or verified process/host teardown. Fencing the result alone does not free execution capacity. Under partition the strict profile may stop admitting work; this availability loss is part of the safety contract.

A weaker best-effort cap MAY exist only as a separately named, explicitly approved profile. It MUST not advertise the original strict invariant. The initial guarantee assumes workers obey the protocol; a malicious host that ignores limits is outside it.

### 38.3 Hierarchical budgets

Global host budgets, tenant budgets, specimen budgets and per-query/job budgets compose by taking the most restrictive applicable ceiling. CPU shares, RAM, device RAM, network bytes, output bytes, graph visits, task count and retries are independently bounded. Recovery/control capacity is reserved so query overload cannot indefinitely prevent safe recovery or revocation.

All reservations have ownership and terminal reconciliation. An async handle drop cannot release a reservation still protecting a running blocking/device task. Admission, cancellation, retry and teardown tests MUST verify no leak and no premature reissue.

<a id="section-39"></a>
## 39. Neural Specimen organization and HybridTable

### 39.1 Persistent specimen

A specimen is a configured instance containing durable organs and explicit ephemeral execution state. Required persistent contents include pattern/non-pattern nodes, original patterns, votes and alternatives, side resources, Sigil definitions, attachments, history and pins, ATP definition/state, transient-network definition, inhibitory rules, mutation lineage, correlation statistics, learning proposals, maintenance configuration and compartmental contacts.

HybridTable owns the specimen's logical storage abstraction. Physical arrays, hash tables, chunks and secondary indexes MAY change for measured efficiency but MUST preserve equality, all-match retrieval, stable internal references and original payloads. A physical database page is not a logical node; a Pattern-ID bucket can contain multiple node tables.

### 39.2 Pattern identity

`PatternKey = (modality, resolution_class, encoder_version, pattern_id)`. Low/medium/high complexity classification precedes the corresponding key transformation. Equality only groups candidates. It neither calculates confidence nor proves semantic applicability. Secondary verification always uses the declared high-resolution mechanism.

Public node IDs may repeat. Internal `node_ref`, `entry_ref` and `slot_ref` distinguish actual structures and contributors. The default **20 entries** is per node, not per Pattern ID, population, query or vote pool. All matching nodes are returned subject to explicit bounded traversal/continuation, never an invented 20-result cap.

Exact repeated original patterns and exact duplicate node structures follow the original rules. Hash equality alone is insufficient; compare canonical structure under an approved equality codec. Paraphrases remain permitted. Until that codec's unresolved original choices are approved, the affected full-conformance claim remains gated rather than silently selecting a convenient implementation.

### 39.3 Resource identity

Each semantic side record retains original representation and a generated lifetime/version `pattern_key`. Its optional nonunique `resource_id` links a compatible unprefixed node ID. These are different channels. A missing resource link does not make the record unindexed, and a shared display string across incompatible encoders does not authorize an equality join.

Native persistence uses the approved WDBX adapter boundary; browser persistence retains its declared local profile. Browser and native schemas may have explicit migration projections, but cannot silently replace one another's authoritative semantics.

<a id="section-40"></a>
## 40. Language Governance and scoped Sigils

### 40.1 Input preparation

Language Governance retains original input bytes, normalization lineage, chunks, bounded variants, relational triples, captures and contextual constraints. Dictionaries, Thesaurus, Anti-Thesaurus, Chargebook and registered lexical rules remain separately addressable resources. Original captured values remain available for later substitution; normalized identifiers cannot overwrite their meaning.

The preparation pipeline records encoder/profile and each fan-out origin. Resource ceilings constrain variants, triples, tokens and bind candidates. Missing resources produce explicit absence or a declared bounded fallback, not an invisible whole-database semantic scan.

### 40.2 Sigil registry

Sigils retain scope, original semantic representation, kind, parameters, expansion/program, permitted calls and execution limits. Reserved and user-defined namespaces are separate. Tokens, functors and automata are compiled/validated against a bounded typed interpreter. Imported/generated text is inert until explicitly registered under the appropriate authority.

The interpreter MUST enforce instruction fuel, memory, recursion/call depth, output size, deadlines and side-effect permissions. Textual name equality never grants reserved authority. Captures are typed data; string interpolation cannot bypass validation to execute filesystem, network or shell actions.

### 40.3 Bind rules and cost

Broad procedural/wildcard bindings may require a separate candidate route when one literal Pattern key is insufficient. That route is explicit, bounded and provenance-tracked. It cannot bypass deep-scan/inhibition rules unless the binding's approved semantics specifically define the alternative gate.

Macros are reusable operations, not zero-cost or asymptotically free behavior. The runtime accounts for expansion and traversal work. No exception to user hard budgets is granted because an operation is named a Sigil or automaton.

<a id="section-41"></a>
## 41. Pattern gates, independent votes, and attachments

### 41.1 Voting

Each candidate entry performs its own high-resolution verification, confidence calculation, inhibition and configured gate. Every independently qualified entry may contribute. The voter phase MUST not elect one global winner before composition. Node reasoning ends with vote submission; higher-order structure is the orchestrator's responsibility.

Confidence remains within **[-100,100]**. A confidence-strength product is a separate selection weight and MUST be converted by an explicit finite nonnegative sampling rule. Negative probabilities, silent overflow and NaN propagation are forbidden. A new sampling rule is versioned and must preserve the original approved distribution.

Temporary jitter never accumulates into baseline merely through reads. Exact ±100 confidence endpoints do not jitter. Maximum-strength/crystallized nodes protect baseline-qualified confidences according to the original exception; lower confidences retain their eligible behavior, and loss of maximum strength removes the protection. IDs, structure, caps, permissions and durable facts never jitter.

### 41.2 Arbitration

Same-originating-triple spin-bottle arbitration removes redundant unattached alternatives, including redundant fan-outs, under an approved redundancy predicate. Complementary contributions survive. Attachment-group contributions retain their dependency relationship and cannot be discarded as ordinary duplicate answers.

The exact redundancy predicate is an original gated decision. Its absence is not permission to discard every similar-looking vote. Records of suppression MUST identify the group/policy and preserve contributor provenance for authorized inspection.

### 41.3 Attachment execution

Soft attachments schedule by the configured affinity-to-probability mapping. Hard/crystallized attachments guarantee scheduling of the target's evaluation, not a successful vote. Targets receive the unmatched remainder and evaluate their own gate. Attachment identity references concrete nodes rather than ambiguous duplicate public IDs.

Directed and bidirectional links retain explicit topology. A bidirectional link does not authorize infinite mutual activation. Visits bind cycle, target and remainder; nonprogressing repeats are rejected or bounded. All comparisons consume the same specimen-wide scan credits.

Automatic wiring records actual correlation evidence and the rule that proposed/created a link. Co-activation is not semantic causation. Promotion thresholds and hard-link formation follow the approved runtime profile; this integration does not invent new automatic-learning authority.

<a id="section-42"></a>
## 42. Cycle lifecycle, orchestration, and supporting retrieval

### 42.1 Cycle state

```text
Received -> Prepared -> CandidatesBound -> Scanning -> VotesCollected
         -> Orchestrating -> Synthesizing(optional) -> OutputPrepared
         -> CommitDecision(if durable effects) -> Presented -> Closed
```

Failure/partial outcomes include policy denial, invalid evidence, resource exhaustion, deadline, snapshot expiry, device failure and cancellation. The cycle pins the input/context, specimen snapshot, policy, encoder/model identities and stochastic profile. It records admitted and unfinished work.

Orchestration consumes the completed admitted vote set or an explicitly partial set. Pending candidates cannot be silently dropped while claiming complete inference. A bounded iterative pass is a declared pass with its own limit, not unbounded recursion into the voter phase.

### 42.2 Ephemeral composition

The event-local orchestrator orders linguistic/procedural structures, constraints, selected actions and supporting material. It may invoke scoped scripts and bounded Transient Memory. It cannot modify a node's already-submitted vote or mint an approval for persistent learning.

Output provenance is constructed as composition occurs. Each text/artifact span identifies contributing node/entry/alternative refs, source votes, support resources, captures and transformations. Where synthesized wording has mixed contribution, the provenance records that mixture rather than fabricating one exact origin after generation.

### 42.3 Supporting retrieval

Structural support queries use keys from both vote text and activating input/chunks. Node-linked support retrieves resources whose compatible optional `resource_id` matches a currently voting node's logical ID. Both channels are required.

Stage one retrieves equality/link candidates from permitted registered organs. Stage two filters them contextually. Supporting retrieval does not run the pattern-node confidence/deep-scan algorithm by default. Deduplication uses concrete record refs and retains lookup provenance. No-result behavior is explicit.

### 42.4 Presentation and effects

Anti-staleness substitutions must preserve negation, numeric captures, procedural meaning, inhibition, tone and provenance. Chargebook/ATP checks constrain eligible substitutions, with bounded rerolls and retention of the original when no safe choice is found.

Streaming partial text MUST be labeled as incomplete where completion matters. External actions remain subject to ABI approval and their own idempotent execution contract. A generated instruction is not an executed action. Durable cycle effects obey the commit/cancellation contract in section 8; post-commit disconnection is not rollback.

<a id="section-43"></a>
## 43. Transient Memory, imagination, and ATP

### 43.1 Non-learning synthesis

Transient Memory is a bounded, event-driven computation organ. It consumes exposed support, current request, votes/intermediate structures and applicable ATP state. Sparse matrix/MLP synthesis and approved optional models return temporary structures; they do not train persistent weights or approve new knowledge merely by running.

Network architecture, fixed/configured weights and explicit configuration may persist. Activations and temporary scratch do not become knowledge. CPU is the reference path; optional GPU/NPU execution inherits exactly the same no-learning contract and budgets. A large generative model is not required merely to implement the organ.

### 43.2 Activation and observer

The original runtime policy permits a recorded biased activation selection in which familiar material favors sigmoid and unfamiliar combinations favor ReLU, with urgency and brainstorm behavior bounded by hard limits. Granularity, familiarity evidence and probability values are profile-defined; this draft does not substitute an unapproved distribution for an existing one.

A self-observer checks transient invariants, predictions and output constraints. It may correct temporary structures and record the reason/provenance. Persistent inhibitory rules remain separate from event-local failures; an observed bad synthesis does not authorize unlimited durable self-modification.

### 43.3 Imagination

An authorized imagination request retrieves scoped ingredients through the Pattern-ID/support channels, context-filters them, maps them into the approved synthesis representation, and returns narrative or visual instructions. Fuzzy-art outputs remain ephemeral unless explicitly saved. Imagined content is never automatically validated evidence.

Attention, sparse routing, fixed network and optional model decoding choices remain declared implementation profiles. Sigmoid/ReLU selection alone is not a complete language generator; each selected decoder/representation requires its own acceptance evidence.

### 43.4 ATP

ATP preserves bounded temporal tone, arousal, buildup/release and cooldown around its declared baseline. Durable state evolution is separate from temporary jitter. The original “Lorenz-like” description does not authorize claiming a particular differential-equation model without defining it.

ATP may modulate eligible confidence/tone within original rules; it cannot change identity, permissions, hard ceilings or signed facts. Self-directed/brainstorm work is configurable, provenance-marked and lower priority than user work where the approved policy requires. It does not grant permission to ignore governance or execute external actions.

<a id="section-44"></a>
## 44. Conversation, vision, and multimodal state

Rolling conversation and pinned history remain distinct. The inherited editorial default is **100,000 message/answer records**, subject to configured resource and retention limits; it is not a forced preallocation. Record granularity follows the original selected profile. Pinned data is not immune to lawful policy/approved erasure, but ordinary rolling eviction MUST not silently remove it.

Conversation records preserve origin, chronology, source edits, scope and linkage to generated responses. Summaries are derived, with parent references and invalidation rules. Cross-guild or cross-compartment summaries cannot leak private context through shared names, embeddings or graph edges.

The visual system retains eye lifecycle, visual pattern arrays, OCR-derived text, arousal interaction and ephemeral fuzzy art. “Nonlinear SDF” remains the original qualified representation label; a true signed-distance field requires a separately defined mathematical encoding. Visual array coordinates do not automatically become physical geometry.

Multimodal artifacts identify source digest, media type, dimensions/duration, preprocessing, model and scope. Frames, patches, depth and audio spans retain explicit offsets/units. Decoder bounds protect against oversized media and resource exhaustion. An unavailable perception model is an explicit limitation, not a fabricated observation.

Optional text/image generation profiles identify model/runtime/artifact and user-approved download requirements. Their outputs are separate from learned node votes and carry generation provenance. Hardware configuration and model estimates are qualified per runtime, not assumed from an old Studio configuration.

<a id="section-45"></a>
## 45. Feedback, Type A/B nodes, mutation, and PHAGY

### 45.1 Feedback

Whole-answer and contributor-targeted feedback acts only on identified eligible contributors. Independent coin flips occur per affected contributing node, not per output character or retry. A recorded feedback operation binds the selected subset and draw identities so retry does not resample. Strength arithmetic is bounded.

Color is presentation, not identity or permission. Targeting by color resolves to stable contributor refs and cannot accidentally update unrelated nodes sharing that color. Feedback never rewrites past signed episodes or updates pretrained model weights implicitly.

### 45.2 Non-pattern nodes and learning

Type A nodes provide residual corrective contributions under their timing/scope rules. Type B nodes may edit explicitly permitted internal state, but new inferred learned structures require the original identified `/learn` approval. Pending proposals carry stable IDs, proposed structure, evidence and status. Stale proposals must be revalidated before acceptance.

Unknown custom node types are inert until a registered schema/capability contract is approved. They may be preserved opaquely where permitted by import/export rules, but are not executed as arbitrary code.

### 45.3 Idle mutation and lineage

Default eligible idle intervals are newly selected within **[60,120] seconds**. Eligible-time accounting, distribution and pause/reset behavior are recorded by profile. A three-second experiment is a different configuration, not the default. User activity and hard budgets constrain maintenance.

A recipient alternative slot can be remixed at most once during its lifetime. Stable slot identity survives reordering, save/load and ordinary edits. Mutation retains donor node/entry/slot refs, weight choice, original pairing and growth inhibition. Shortening a vote pool cannot erase an already remixed slot without an approved tombstone/retention rule. Only the aligned minimum donor/recipient range is remixed; mutation does not silently grow the pool.

### 45.4 PHAGY and grave handling

PHAGY uses bounded sparse ephemeral automata with unique jobs, ownership, timeout and inspectable outcomes. Cleanup is idempotent and respects user hard limits and policy. At zero strength, active nodes and indexes may be removed according to the original mechanism while inhibition and mutation history remain.

Grave retention/resurrection is a named original design gate. Absence of a selected grave policy does not permit permanent deletion or uncontrolled resurrection. Full conformance for that behavior requires an approved explicit profile and acceptance tests.

<a id="section-46"></a>
## 46. Index Rafts, contacts, commands, and specimen persistence

### 46.1 Index Rafts

An Index Raft is resumable traversal over a disjoint range, not consensus Raft, a thread, or a physical chunk. Required ranges cover from index zero through the selected view without gaps/overlap. Every continuation yields and preserves its cursor. `find_all` never stops on the first matching Pattern ID; `find_first` is a separate operation.

Scheduling may change physical work order but not final coverage, candidate identity or stochastic draw meaning. The shared high-resolution comparison cap applies to actual comparisons inside multi-entry nodes and attachment work. Queue exhaustion produces an explicit partial/failure outcome rather than disappearing candidates.

### 46.2 Compartmental contact ledger

Contacts retain compartment-specific identity, scope and permitted relationships. Matching names or public handles across compartments do not establish identity equivalence or authorization. Read, join, export and action permissions are independent. Optional cross-compartment linkage is an explicit authorized relation with provenance.

### 46.3 Commands

Preserve `/prompt`, `/right`, `/wrong`, `/wrng`, `/contributorfractal`, `/saveSpecimen`, `/loadSpecimen`, `/addPattern` and `/learn` with original runtime-specific routing. Editor/file chooser and proposal-resolution behavior remain visible and compatible. `/learn` does not become a generic training/deployment command.

New spatial, hardware, cluster or evidence commands require additive namespaces, help, permission checks and versioned semantics. Text that resembles a command in a stored pattern is not automatically executed as a privileged user command.

### 46.4 Save and restore

Save captures the entire persistent specimen and coherent configured modulation state, not only nodes. It excludes raw credentials, live GPU handles, threads, transient activations and executable in-flight continuations by default. Model weights may be referenced by verified digest and license rather than embedded.

Browser JSON and native `.wdbxspecimen` profiles retain their original behavior. Imports validate schema, references, duplicate structure, Pattern-key invariants, mutation markers, permissions, sizes, paths and artifact digests before activation. Archive traversal, symlink escape and decompression bombs are rejected. An import cannot broaden authority or silently replace a live specimen.

Restoration retains an appropriate recovery copy and rebuilds derivatives from canonical contents. Older readers/writers are admitted only under a tested matrix. Secret/key state requires its own approved recovery process; portable specimen loading cannot restore erased authority.

<a id="section-47"></a>
## 47. APIs, transport, and errors

### 47.1 Compatibility surfaces

Existing CLI/MCP tool names, loopback REST, gateway methods, episode RPCs and WebSocket behavior remain frozen according to pinned source contracts. The source manifest—not a new prose approximation—is the exact authority for their names and fields. Transport JSON/protobuf is distinct from canonical episode encoding.

New capabilities are negotiated additively. An API revision is independent of the v3 episode format and document revision. Existing custom HTTP+SSE behavior cannot be renamed as a newer MCP transport without actual protocol conformance.

### 47.2 Proposed logical service operations

The following are specification operations, not claims of existing endpoint paths:

| Operation | Request | Required result |
|---|---|---|
| `DescribeCapabilities` | Authorized profile scope | Supported schema/query/device profiles and qualification state |
| `OpenReadView` | Consistency, scope and lifetime request | Opaque view token, roots and expiry |
| `QuerySpatial` | Predicate, frame, conflict/numeric profile, limits | QueryOutcome with geometry observations |
| `QueryHybrid` | Named spaces/components, predicates, rank policy | QueryOutcome with separated components/evidence |
| `TraverseGraph` | Start set, edge set and work bounds | Authorized bounded graph outcome |
| `InspectEvidence` | Record/version and view | Eight dimensions and permitted provenance |
| `PutSpatialAttachment` | Validated geometry/source preconditions | Idempotent canonical receipt |
| `BuildProjection` | View/cohort/algorithm/budget | Job identity and verified publication status |
| `ReadScene` | Projection/view token and page/LOD | Bounded authorized scene chunks |
| `RunSpecimenCycle` | Specimen revision, input, profile and limits | Cycle outcome, actual provenance and commit status |
| `ApplyFeedback` | Cycle/contributor selection and operation ID | Idempotent bounded feedback receipt |
| `GetOperationStatus` | Operation ID with authority | Known terminal result or unresolved status |
| `DispatchCompute` | Capability-bound job | Qualified result or explicit failure/fallback |

Final transport names are assigned through versioned schemas after gateway review. Remote mutation, projection build and compute jobs are never enabled by exposing a read-only scene endpoint.

### 47.3 Error taxonomy

Required errors distinguish invalid argument/frame/space, unauthorized/forbidden, stale policy, snapshot expired, conflict, unsupported schema/backend, unqualified backend, integrity failure, resource exhaustion, deadline, unavailable partition, operation conflict and outcome unknown. Public errors avoid private existence leaks. Restricted diagnostics preserve actionable causes without exposing payloads.

All parsers enforce byte/depth/count and decompression limits before allocation. Streaming enforces per-frame and aggregate budgets. Authentication failures do not reveal more schema/data detail than permitted. A retry hint does not authorize retrying a non-idempotent operation under a new identity.

<a id="section-48"></a>
## 48. Explorer, observability, and operational limits

### 48.1 Explorer

The explorer is read-only by default and distinguishes physical and projected scenes. Nodes expose stable identity, snapshot/generation, evidence and explicit typed edges. Selecting a point does not grant mutation authority. Timeline playback indicates the selected historical view and current access constraints.

Scene delivery uses bounded pagination/LOD and policy-filtered geometry buffers. Hidden nodes cannot influence strict-cohort centroids, labels, heatmaps or exports. The client must purge invalidated cached scene data and stop old streams after policy change.

Keyboard navigation, camera reset, search, text/tabular equivalents, reduced motion, color-independent contributor identity and screen-reader descriptions are required. Large-scene handling is tested with realistic memory/device limits. A browser WebGPU renderer does not prove database GPU compute execution.

### 48.2 Observability

Telemetry distinguishes data, control, compute and specimen work. Required measures include commit latency, recovery/checkpoint time, index freshness, conflict count, query coverage/errors, graph work, queue delay, peak memory, device transfer/kernel time, remote bytes, replica lag, fallback causes, scan-credit states and cycle outcomes.

Default telemetry excludes prompts, member fact text, embeddings, contact names, raw identifiers and private graph topology. Sensitive debug mode requires scoped approval, expiry and protected retention. High-cardinality labels are bounded; private IDs are not casually placed in metrics dimensions.

Metrics are observations, not automatic claim promotion. `runtime_verified` is displayed distinctly from independent operation qualification, measured placement, benchmark result and production approval.

### 48.3 Configuration and limits

Every deployed profile MUST supply finite limits for request bytes, collection counts, vectors/components, graph visits, projection cohorts, queues, snapshot lifetime, CPU/device memory, retries and output. Defaults inherited from original specimen semantics remain 20 entries per node, 1,000 simultaneous comparisons, confidence [-100,100], and eligible idle intervals [60,120].

New operational ceilings are chosen in the deployment profile from capacity evidence, not invented as universal performance facts. Missing required limits fail startup/admission. User hard ceilings dominate adaptive urgency, brainstorming and maintenance. Lowering a ceiling drains or refuses new work safely; it cannot pretend running work disappeared.

<a id="section-49"></a>
## 49. Verification and acceptance evidence

### 49.1 Verification layers

Document validation, source compilation, unit behavior, integration, failure recovery, device qualification and production operation are separate evidence layers. Each acceptance record names requirement, profile/runtime, source/build, case/fixture, command, observed result, artifact and reviewer. No checkbox substitutes for actual execution.

For a behavior change, tests MUST fail for the intended behavioral reason before the fix and pass after it. Baseline qualification of already-existing behavior may legitimately be a verification packet rather than an artificially manufactured failure. Test discovery MUST show a nonzero expected case set.

### 49.2 Required independent oracles

Spatial testing includes complete scans, independent numeric reference and dyadic boundary cases. Vector testing compares equivalent spaces and aggregation against exhaustive authorized scoring. Graph testing validates bounded reachability/path semantics. Episode testing preserves pinned canonical goldens and signature behavior. Specimen testing checks all original acceptance rows for each claimed runtime.

The optimized implementation cannot be its own independent oracle. Mock devices test control logic, not real hardware placement or numerical qualification. A source-only cross-compile is not a provider runtime round trip.

### 49.3 Fault corpus

Required injected failures include torn journals, corrupt canonical/index objects, stale index deltas, concurrent causal changes, signature tampering, disk full, lost commit receipts, cancellation races, stale policy, expired views, archive bombs, driver OOM/reset, worker disconnect, lease expiry with ongoing work, partitions, duplicate/reordered messages, replica lag and interrupted rebalance.

Each fixture defines the expected visible canonical state, operation status, coverage, resource ownership and recovery action. No failure test may use the operator's live database, keys, private facts or backup location. Fuzz/stress/hardware runs use explicit budgets and execution authorization.

### 49.4 Build matrix

Qualify supported feature combinations per target rather than blindly enabling mutually incompatible `--all-features`. Keep existing lint/source-size discipline; do not suppress warnings or remove tests to produce a green result. Dependency changes preserve the lockfile and trigger relevant review.

Typical verification commands, only within an authorized fixture checkout, are:

```sh
rustc +1.99.1 --version --verbose
cargo +1.99.1 check --workspace --all-targets --locked
cargo +1.99.1 fmt --all --check
cargo +1.99.1 clippy --workspace --all-targets --locked
cargo +1.99.1 test --workspace --locked
```

The actual repository gate and test targets are discovered from source. These commands are instructions, not a report that they were run during specification authoring.

<a id="section-50"></a>
## 50. Benchmarks and qualification thresholds

Benchmarks MUST capture source revision, compiler/dependency identities, build flags, target, CPU/RAM, device/driver, OS, model/kernel, dataset/seed, corpus license, dimensions, filter selectivity, concurrency, warmup, samples, failures and power/thermal conditions relevant to the comparison.

Measure p50/p95/p99 latency, throughput, recall, relevance where applicable, exact predicate agreement, memory peak, disk amplification, ingest/build/recovery time, network and host/device bytes, cold-model latency and energy when supported by valid measurements. Tail statistics require a justified sample count; a tiny run cannot support confident extreme-percentile claims.

Compare equivalent quality/coverage and realistic end-to-end work. A kernel benchmark excluding transfers is labeled accordingly. Scalar-only comparison does not establish superiority over an optimized CPU. ANN speedup with lower recall is explicitly a tradeoff.

Thresholds are set before promotion using a named workload/profile and baseline. No universal latency, scale or “100× faster” guarantee is created by this specification. Initial data sets include empty/tiny correctness fixtures and medium/stress sizes appropriate to resource budgets. Exactness failures and scope leaks are release blockers regardless of speed.

<a id="section-51"></a>
## 51. Migration, rollback, and production operation

### 51.1 Incremental adoption

The first accepted slice remains checked access around the current spatial index. Typed durable geometry, authorized hybrid retrieval, projection, hardware, cross-host and full specimen conformance follow independent profile gates. Optional GPU/NPU vendors cannot block a useful CPU release, and a local release does not imply their support.

Shadow reads compare equivalent contracts on permitted fixtures before enabling new query paths. Additive index generations can be disabled without changing canonical data. New canonical namespaces require tested readers/writers and explicit minimum-version gates. Existing signed data remains unchanged.

### 51.2 Rollback

Rollback is not automatically “run the older binary.” The matrix MUST identify whether the older reader can inspect, preserve and safely write each new namespace. If it cannot, rollback retains a compatible writer or read-only recovery rather than discarding unknown data. Disabling an accelerator cannot substitute a different model without policy approval.

Migration/restore receipts state source roots, target roots, changed representations, lost capabilities if any and verification artifacts. Recovery copies and backups are not deleted or reconfigured without the specific required approval. Member-erasure migration remains its separate program.

### 51.3 Operational release

Production qualification requires reviewed install/upgrade, certificate/key handling, scope configuration, capacity, restore, incident response, shutdown/drain and rollback procedures. Operators witness required drills and sign off the named profile. Actual deployed topology/failure domains appear in evidence.

A graceful shutdown stops admission, drains or reconciles work, preserves uncertain reservations and flushes required state. Forced shutdown reports what remains unknown. Monitoring alerts distinguish data integrity, scope failure, resource pressure, replica lag and unqualified acceleration; secrets do not enter incident reports.

<a id="section-52"></a>
## 52. Required decisions and final conformance rule

This draft deliberately does not forge approval for unresolved original or deployment-specific choices. The following decisions MUST be recorded before the affected profile is called conformant:

| Decision | Owning boundary | Safe behavior until resolved |
|---|---|---|
| Original exact-structure equality codec | Specimen identity | Preserve source content; do not claim complete duplicate-rule conformance |
| Same-triple redundancy predicate and sampling semantics | Voting | Keep contributions; do not silently discard complementary votes |
| Grave retention/resurrection | Maintenance/lifecycle | Preserve lineage; block unapproved destructive behavior |
| ATP curve and unresolved synthesis/attention profile | Specimen compute | Use only an approved existing profile; no invented neural capability |
| New canonical namespace/reader floor | Storage | Read-only/disabled mutation for unsupported formats |
| Cross-frame/geodetic and advanced-shape profile | Geometry | Reject unsupported transformations/predicates |
| Control-plane protocol and host topology | Cluster | No production cross-host consistency claim |
| Model/provider/device and numeric tolerances | Compute | Use qualified compatible paths or explicit failure |
| Member-erasure constitutional and implementation approval | Privacy | No key destruction, automatic cutover or erasure implementation |
| Workload SLOs, limits and rollback matrix | Release | No production promotion |

An unresolved decision blocks its dependent profile, not unrelated specification work. Original requirements cannot be marked “not applicable” simply because they have not been implemented.

**Final conformance rule:** WDBX may claim only the profiles whose requirements and inherited obligations have corresponding accepted evidence at the deployed source/build/runtime. Every advertised exactness, durability, authorization, device, specimen and cross-node claim must be supported independently. The existence of this full specification supplies no runtime evidence and authorizes no production or destructive action.

<a id="appendix-a"></a>
## Appendix A. Proposed record and service contracts

These contracts specify new logical data. They do not replace existing v1/v2/v3 encodings or claim that these exact public Rust types or transport paths already exist. An implementation schema MUST encode the listed semantics and pass golden compatibility cases before publication. All referenced identities are scoped and validated; opaque authorization tokens are issued by the trusted service, never accepted as self-asserted privileges.

### A.1 SpatialAttachment

| Field | Required semantics |
|---|---|
| `schema_profile` | Namespaced version; unsupported critical profiles reject mutation |
| `attachment_id` | Stable immutable attachment identity |
| `source_record` | Full legacy/UUID RecordId; no dense-slot substitution |
| `source_version` | Exact source version or immutable committed digest |
| `geometry` | Tagged point/AABB/sphere value under a supported numeric profile |
| `frame_ref` | Exact frame identity and registry version/digest |
| `valid_time` | Declared validity interval or point-event representation |
| `scope_binding` | Trusted authorization scope reference; not a free-form grant |
| `provenance` | Origin, creator/event, derivation and source references |
| `content_digest` | Calculated inside the trusted canonical boundary over the registered encoding |

Attaching a new location creates a new attachment/version. It does not rewrite a signed source record. Querying a logical source ID alone cannot accidentally join an old vector with a newer incompatible spatial observation; the join policy identifies exact versions or an explicitly declared logical aggregation.

### A.2 EmbeddingSpace and VectorComponent

`EmbeddingSpace` contains immutable model digest, preprocessing/tokenizer identity, modality, dimension, dtype, normalization, metric and aggregation compatibility. A model alias resolves to this identity at request admission. `VectorComponent` contains parent record/version, component identity, space ID, original/encoded representation identity, source segment offsets where applicable, and inference provenance.

A space change is not a mutable in-place edit to all vectors. Component order and weighting are explicit. A component whose source was deleted or whose scope was revoked follows lifecycle/policy handling before materialization. The response can report a missing authorized component; it cannot fabricate an embedding.

### A.3 Episode and evidence attachment

Historical v3 episode fields remain as their source schema defines. A new evidence attachment names the exact target episode and the dimension being asserted or observed, with author, validity, method, value/unknown state and its own commitment. It MUST distinguish an asserted source reputation from an independently measured calibration or outcome observation.

Outcome evidence references a prediction/action identity and the observation used to evaluate it. Evidence updates append new records; a later conclusion does not rewrite the earlier prediction. Revocation/disposition changes remain query-time policy over preserved history unless a separately approved privacy action restricts payload availability.

### A.4 ReadView and AuthorizedReadView

`ReadView` is a data-consistency object: database identity, canonical root vector, causal dependencies, frame registry, conflict mode and generation references. `AuthorizedReadView` wraps an approved subset plus policy/consent binding and expiry within the trusted runtime.

A client token maps to this state or authenticates it under a supported token profile. Public token contents cannot reveal hidden membership or topology. Changing ranking/predicates while reusing a cursor fails digest validation. A service may avoid exposing underlying root details to ordinary users while preserving them in restricted evidence.

### A.5 QueryRequest

Required logical fields are `operation`, `profile`, `view`, `predicate`, `result_unit`, `ranking`, `limits`, `partial_policy` and `numeric_profile`. Vector requests additionally name space and query embedding/inference request. Graph requests name allowed edges and traversal semantics. Spatial requests name frame and geometry.

`limits` includes explicit maximum work, memory and output appropriate to the profile; the service clamps authorization-independent resource requests only according to a disclosed admission contract or rejects them. It does not silently reduce a requested exact search budget then report complete exact results after truncation.

### A.6 QueryHit

Every hit contains the observation identity, source version, metric/ranking context, permitted component values, approximation/numeric profile and evidence/provenance references. A rendered snippet or distance is a view of that observation, not its immutable identity. Distances may be omitted when not applicable; missing is not zero.

Component scores are comparable only within their registered policy. A scalar convenience score MAY coexist with full components but cannot replace hard evidence disposition or disclosure limits. A client cannot infer source truth from a high similarity score without examining the separate evidence contract.

### A.7 ComputeJob

| Field group | Required contents |
|---|---|
| Identity | Job ID, parent operation/cycle, input digest |
| Authority | Principal/scope reference, policy epoch, permitted destination/device set |
| Program | Operation, exact model/kernel digest, supported runtime/profile |
| Input | Shape/dtype/layout, source/read-view binding, bounded payload references |
| Output | Shape/dtype, numeric and completeness contract, destination |
| Resources | Deadline, CPU/device RAM, concurrency credits, retry/transfer budgets |
| Evidence | Required qualification record and placement/performance disclosure |

The job excludes general canonical commit authority and long-lived keys. Resubmission under the same identity with changed inputs is rejected. Device output may be usable only as approximate candidates; its contract must say so before execution, not only after a mismatch.

### A.8 SpecimenNode, Vote and OutputSegment

A pattern node contains `node_ref`, nonunique logical ID, compatible Pattern key, strength, configuration and bounded entries. An entry contains `entry_ref`, original pattern, optional bind representation and a vote definition. Weighted alternatives have stable `slot_ref`, action, inhibition, finite weight, remix state and donor lineage.

A runtime vote contains cycle/node/entry/slot refs, selected alternative, source snapshot, baseline/effective confidence, strength snapshot, origin triple/fan-out, activating keys, captures, optional attachment group and provenance. It is immutable after submission.

An output segment references source votes, contributors, supporting resources and transformations. Generated text without a attributable source span may be recorded as synthesis with the supporting set; the system MUST not invent precision in attribution. Feedback resolves segment contributors to stable nodes before independent draws.

### A.9 OperationReceipt and terminal outcome

A receipt binds operation identity, input digest, committed object identity, achieved durability, policy decision, terminal result and receipt encoding profile. The authoritative terminal result is committed, rejected, failed-before-commit or cancelled-before-commit; outcome-unknown is a reconciliation state, not proof of noncommit.

Receipt verification does not independently prove an external action succeeded unless actual outcome evidence is attached. A database commit acknowledging a request and a remote side effect are separate facts. The service MUST expose this distinction to action orchestrators.

### A.10 Synthetic spatial query example

This is an illustrative new wire projection for a proposed API—not an existing gateway request, an authorization token, a canonical signature input, or a test execution report.

```json
{
  "operation": "QuerySpatial",
  "profile": "spatial-durable-v1",
  "view": "synthetic-authorized-view-1",
  "predicate": {
    "kind": "point_within_sphere",
    "frame": "synthetic-cartesian-frame-1",
    "center": [0.0, 0.0, 0.0],
    "radius": 5.0
  },
  "result_unit": "observation",
  "conflict_mode": "AllMaximalVersions",
  "numeric_profile": "exact-dyadic-predicate-v1",
  "ranking": "euclidean_then_observation_key",
  "limits": {
    "max_results": 100,
    "max_observations_examined": 1000,
    "max_memory_bytes": 1048576,
    "deadline_ms": 1000
  },
  "partial_policy": "reject_partial"
}
```

For a synthetic authorized corpus consisting of `(0,0,0)`, `(3,4,0)` and `(6,0,0)`, the required result includes the first two observations, in that order. The point at distance exactly five is included by the closed-boundary contract. The result reports exhaustive candidate coverage only after considering the whole eligible fixture, regardless of which index supplied candidates. The example's limits are fixture choices, not product defaults or latency promises.

### A.11 Synthetic query outcome envelope

```json
{
  "query_id": "synthetic-query-1",
  "read_view_id": "synthetic-authorized-view-1",
  "hits": [
    {"observation_ref": "synthetic-observation-1", "distance": 0.0},
    {"observation_ref": "synthetic-observation-2", "distance": 5.0}
  ],
  "candidate_coverage": "exhaustive",
  "numeric_accuracy": "exact_real_predicate",
  "completion": "complete",
  "partial_reason": null,
  "consistency": "local_snapshot",
  "ranking_policy_id": "euclidean_then_observation_key",
  "observed_generations": [],
  "execution_evidence_refs": [],
  "continuation": null,
  "explanation_summary": "Synthetic expected outcome; not an executed query."
}
```

A deployed response includes the full hit and evidence fields required by its schema. Empty evidence refs in this example are not permission to fabricate production execution evidence. Exact predicate accuracy applies to membership/order, not an assertion that decimal display values are infinitely precise physical measurements.

<a id="appendix-b"></a>
## Appendix B. Canonical failure and race acceptance scenarios

Every scenario below is a required specification case for the relevant profile. None were executed while writing this document.

| Case | Trigger | Required observation |
|---|---|---|
| DB-CASE-01 | Cancel before commit decision | No canonical effect; `CancelledBeforeCommit`; reservations reconciled |
| DB-CASE-02 | Commit wins, response is lost | Status returns original committed identity; retry does not duplicate |
| DB-CASE-03 | Same operation ID, different payload | Conflict; no second mutation |
| DB-CASE-04 | Index manifest is truncated | Reject generation; old generation or complete scan remains available |
| DB-CASE-05 | Delta adds a new position but omits removal of the old one | Coverage rejected as incomplete; no exact claim |
| DB-CASE-06 | Equal rounded distances but distinct exact keys | Order follows authoritative exact key, then stable identity |
| DB-CASE-07 | Two maximal spatial versions | Both observable under default new conflict mode; no implicit resolution |
| DB-CASE-08 | Legacy point lacks units/frame | Unitless legacy view only; no inferred geographic/metric meaning |
| DB-CASE-09 | ANN misses the winning record | Approximation remains explicit after reranking |
| DB-CASE-10 | One required shard is unavailable | Exact-only request fails; permitted partial response marks missing coverage without unauthorized disclosure |
| DB-CASE-11 | Shards normalize scores differently | Reject exact global merge or use a declared approximate/global-score protocol |
| DB-CASE-12 | Policy revoked after snapshot pin | New materialization stops/rebinds; old plaintext borrows are not falsely claimed erased |
| DB-CASE-13 | Hidden records affect projection fit | Strict-cohort qualification fails even if hidden points are not rendered |
| DB-CASE-14 | GPU cancellation leaves a kernel running | Device memory and credits stay reserved until completion/verified teardown |
| DB-CASE-15 | Remote lease expires while worker is disconnected | Credits become uncertain and are not reissued |
| DB-CASE-16 | NPU request returns correct output without placement trace | Output may qualify; ANE/NPU residency remains unknown |
| DB-CASE-17 | Required GPU unavailable | Typed failure under exact-backend policy; no silent CPU substitution |
| DB-CASE-18 | Old writer does not understand a new namespace | Refuse writes or use compatible recovery; do not discard data |
| DB-CASE-19 | Incomplete multiway bundle | Partial/staged visibility only; complete marker absent |
| DB-CASE-20 | Feedback delivery repeats | Same independent draw decisions and one durable strength effect |
| DB-CASE-21 | Hard attachment target fails its gate | Target was scheduled; no forced vote |
| DB-CASE-22 | Save after a mutation-slot remix | Lifetime marker and donor/inhibition lineage survive restoration |
| DB-CASE-23 | Backup contains a potentially recoverable erased key | Erasure claim remains conditional/pending under its threat contract |
| DB-CASE-24 | New model produces same-width embeddings | Separate space identity; no automatic direct comparison |
| DB-CASE-25 | Qualification runs zero tests | Not accepted; discovery/coverage failure |
| DB-CASE-26 | Feature compiles but hardware is absent | Compiled evidence only; runtime/placement/benchmark untested |

<a id="appendix-c"></a>
## Appendix C. Original WDBX feature preservation inventory

This inventory is a conformance obligation derived from the supplied plan and original directive, not a fresh assertion that every row is currently complete in source.

| Original feature family | Required preservation |
|---|---|
| v1 compatibility | Nonmutating read, historical identities, format fixtures and original recovery behavior |
| v1 to v2 migration | Verified source equivalence and protected recovery copy; no incidental destructive migration |
| v2 causal store | Independent writers, exact transaction identity, maximal conflicts, explicit resolution |
| Snapshot-backed views | Correct lifetime and immutable source binding across later commits |
| v3 episodes | Canonical encoding, detached signatures, original admission/consent-policy rules and receipt semantics |
| Audit provenance | Historical hashes/parents, multi-parent causal DAG and integrity verification |
| HNSW and exact retrieval | Existing contracts and baseline, explicit approximation and deterministic ordering |
| Original spatial octree | Legacy types and metrics; checked additive paths instead of a duplicate replacement |
| Temporal/causal/persona ranking | Original compatibility scoring plus separately versioned improvements |
| Key/value and metadata | Existing product boundary, conflicts, durable retrieval and source semantics |
| Multiway rewriting | State deduplication, event multiplicity, boundedness, resume and canonical exports |
| Experiment persistence | Honest partial write behavior and new marker protocol only when implemented |
| Compression | Existing int8/PQ/autoencoder/Huffman/rANS profiles, lossiness labels and source identities |
| Authenticated objects | Existing optional encryption/signature formats and verified read behavior |
| Credential providers | Platform-specific status and limits, no invented cross-platform runtime proof |
| Research cryptography | Additive HE/DGHV/TFHE demos remain research without production-protection claims |
| Cluster replication | Exact committed transfer, conflict-preserving repair, bounded rebalancing and honest local evidence |
| Compute contracts | Existing backend identities and capability facts; independent qualification added |
| CLI/MCP/REST/gRPC/WebSocket | Frozen surface contracts and explicitly versioned extensions |
| ABI/Abbey integration | Control-plane/data-plane ownership and approved dependency/consumer compatibility |
| Specimen runtime | Original 26 chapters and all 47 numbered resolutions/invariants, per runtime |
| Native/browser portability | Original save/load, migration, recovery and qualification distinctions |


<a id="appendix-d"></a>
## Appendix D. Original Neural Specimen acceptance matrix

The following 73 obligations are retained from the supplied v5 plan [P1]: 26 chapter rows, 23 resolution rows, and 24 invariant rows. They preserve the original source's authority but do not reproduce its entire text or supersede its unresolved decisions. Each case remains **specified, not executed**. The complete original 26-chapter specification remains incorporated by reference [P2].

Task IDs refer to the unchanged v5 implementation companion; Appendix E identifies those packets. For any claimed runtime, acceptance must name the actual fixture, source/build, execution artifact, outcome and review. These rows are mandatory original-profile obligations, not permission to claim browser/native/distributed equivalence.

### D.1 All 26 chapters

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

### D.2 Normalization and contradiction register R01–R23

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

### D.3 Original invariants I01–I24

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


### D.4 Evidence record for each obligation

Each row's record contains `requirement_id`, original source/ref/blob, applicable runtime/profile, owning task, exact case/fixture, expected assertion, actual build identity, execution artifact, outcome and review. A named unresolved original design choice is `blocked_on_named_decision`; lack of implementation does not make it not applicable. The machine-readable registry shipped with this specification indexes these original obligations, the 18 global invariants and the 26 failure scenarios. It is an indexed subset, not a claim that every normative sentence has been automatically formalized.

<a id="appendix-e"></a>
## Appendix E. Implementation companion packet index

The v5 plan remains unchanged and supplies detailed dependencies, proposed paths, tests and review boundaries. The following index keeps its task references interpretable without duplicating the plan into this specification. No packet was executed by writing this document.

| Packet | Existing v5 plan title |
|---|---|
| T00 | Pin the source, authority, and evidence manifest |
| T01 | Qualify the already-landed stable toolchain |
| T02 | Freeze compatibility behavior before new spatial contracts |
| T03 | Define finite geometry and legacy widening |
| T04 | Build independent complete and boundary oracles |
| T05 | Add checked access to the existing point index |
| T06 | Qualify and harden the existing octree fast path |
| T07 | Specify closed AABB and sphere predicates |
| T08 | Preserve RecordId and every maximal causal version |
| T09 | Add an explicit frame registry and legacy-unbound view |
| T10 | Persist additive spatial attachments with reader gates |
| T11 | Construct a snapshot-bound spatial read adapter |
| T12 | Define content-bound derived index generations |
| T13 | Publish generations durably and recover after interruption |
| T14 | Prove stale-index delta coverage and safe fallback |
| T15 | Fence cancellation, pin lifetimes, and idempotent status |
| T16 | Create trusted scoped read authorization |
| T17 | Define embedding spaces and bounded multi-vector aggregation |
| T18 | Implement the authorized exact hybrid reference |
| T19 | Add filtered ANN without hiding recall loss |
| T20 | Add lexical search and globally comparable fusion |
| T21 | Create a typed, bounded cognitive graph query |
| T22 | Separate valid time, transaction time, and staleness |
| T23 | Expose the eight evidence dimensions without inventing trust |
| T24 | Evaluate a versioned evidence-aware ranking policy |
| T25 | Publish multiway bundles without claiming false atomicity |
| T26 | Implement a cohort-bound PCA projection reference |
| T27 | Qualify optional nonlinear projection adapters |
| T28 | Expose a versioned read-only spatial/projection service |
| T29 | Build a bounded and accessible 3D explorer client |
| T30 | Add independent operation-level qualification records |
| T31 | Implement an independent CPU numeric qualification corpus |
| T32 | Optimize stable CPU kernels only after qualification |
| T33 | Build bounded dispatch and policy-valid fallback |
| T34 | Qualify Metal operations on real supported hardware |
| T35 | Qualify a CUDA adapter as a separate optional profile |
| T36 | Qualify a Vulkan compute adapter independently |
| T37 | Qualify Core ML inference without overstating ANE placement |
| T38 | Define vendor neural-provider and optional TPU conformance |
| T39 | Secure remote compute jobs and reconcile outcomes |
| T40 | Specify and implement the scoped cluster control plane |
| T41 | Qualify exact replication and durable acknowledgment classes |
| T42 | Bind causal cuts and merge distributed search honestly |
| T43 | Make rebalance and anti-entropy restartable |
| T44 | Enforce the strict specimen-wide comparison cap across nodes |
| T45 | Run a real-host cluster qualification campaign |
| T46 | Bind HybridTable and specimen identity without changing semantics |
| T47 | Preserve Language Governance and sandbox Sigils |
| T48 | Preserve independent votes and attachment scheduling |
| T49 | Make cycle phases, support retrieval, and provenance coherent |
| T50 | Preserve non-learning Transient Memory, ATP, history, and vision |
| T51 | Preserve feedback, approved growth, mutation, and PHAGY |
| T52 | Preserve Index Rafts, contact scopes, commands and hard limits |
| T53 | Preserve whole-specimen save, import, and rollback |
| T54 | Reconcile privacy and erasure as an approval-gated program |
| T55 | Build security, fuzz, supply-chain and evidence gates |
| T56 | Measure benchmarks and select thresholds before promotion |
| T57 | Prepare profile-specific release, recovery, and operator sign-off |

<a id="appendix-f"></a>
## Appendix F. Sources, lineage, and authoring evidence

### F.1 Supplied project sources

**[P1] Supplied v5 implementation plan.** `WDBX-3D-Neural-Specimen-Master-Plan-v5.md`, 344,347 bytes, SHA-256 `f98d61f6f80503a2d00687c8cb6f38ed654cf9ad46cf55407ba5e06765354822`. Its content is the principal consolidation source for this draft. The companion is included unchanged in the ZIP package. Source-inspection statements inherited from that plan remain historical, not fresh runtime tests.

**[P2] Original Specimen Architecture and Behavioral Specification.** The original 26-chapter design, R01–R23, I01–I24 and runtime-profile additions remain controlling for specimen behavior. During this writing, a selected source range was retrieved for attachment, orchestration, support, Transient Memory and ATP semantics; the entire original was not freshly re-audited. Discovery URL: https://github.com/donaldfilimon/wdbx-specimen-studio/blob/main/public/WDBX-Specimen-Architecture-Specification.md

The original source is incorporated by reference, not bundled as an invented or reconstructed original. Before implementation, resolve the actual current Studio workspace and pin the complete original at an immutable revision.

**[P3] Controlling WDBX/ABI source inventory.** The user named the following documents; pin complete current versions before implementation or a conformance claim:

| Source | Role |
|---|---|
| `ABI/docs/spec/wdbx-north-star.mdx` | Capability inventory, architecture and claims discipline |
| `ABI/docs/spec/wdbx-multiway.mdx` | Bounded state/event semantics, resume and persistence boundary |
| `ABI/docs/spec/agent-wdbx-architecture.mdx` | ABI ownership, WDBX boundary and consumer contracts |
| `ABI/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md` | Canonical episodes and eight evidence dimensions |
| `ABI/docs/superpowers/specs/2026-09-29-wdbx-completion-design.md` | Evidence-half priorities and separately gated work |
| `ABI/docs/superpowers/specs/2026-09-29-spec-member-erasure.md` | Exact custody/erasure design and approval requirements |
| `WDBX/docs/claims.md` and `WDBX/README.md` | Existing evidence boundary and source inventory |

Repository discovery roots: https://github.com/donaldfilimon/wdbx and https://github.com/donaldfilimon/abi . Mutable discovery links are not a source lock. The v5 source baseline identifies WDBX commit `08a69b3f22db61093e06527b3308b10589687022`; this draft does not claim a new checkout or execution at that commit.

### F.2 Official technical references consulted

**[E1] Rust Release Team — Announcing Rust 1.99.1, October 15, 2026.** Confirms the requested release; no WDBX qualification implied. https://blog.rust-lang.org/2026/10/15/Rust-1.99.1/

**[E2] Rust standard-library portable SIMD documentation.** Identifies the experimental/nightly-only API, distinct from safe code the compiler may auto-vectorize. https://doc.rust-lang.org/stable/core/simd/

**[E3] Tokio task::spawn_blocking documentation.** Started blocking work is not generally stopped by aborting its handle; explicit execution/resource reconciliation is necessary. https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html

**[E4] Apple Core ML MLComputeUnits.cpuAndNeuralEngine API reference.** Referenced compute-unit policy; the retrieved page required JavaScript, so the placement distinction is also inherited from [P1]. No actual Core ML execution or placement profiling was performed. https://developer.apple.com/documentation/coreml/mlcomputeunits/cpuandneuralengine

**[E5] Microsoft Windows ML execution-provider documentation.** Consulted for provider/device separation. This draft deliberately does not freeze vendor driver/model support from a changing catalogue. https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/supported-execution-providers

### F.3 Authoring validation and scope

The companion validation report checks the generated document's numbered sections, navigation anchors, original acceptance IDs, global invariants, failure scenarios, task-index references, Markdown fences, structured JSON examples and unchanged source-plan checksum. These are document checks only.

No WDBX product code, dependency graph, runtime, signed record, key, backup, deployment, device or cluster was changed or tested during this writing. No original erasure or constitutional approval was inferred. The specification remains a review draft until its intended profile decisions are approved and its actual implementation evidence exists.

**End of WDBX-SPEC-006, revision 6.0.**
