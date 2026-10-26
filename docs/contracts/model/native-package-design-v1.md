# N01/N02: native WDBX package and tokenizer contracts

Draft, 2026-10-25. Preparation under the approved `docs/plans/2026-10-25-products-modernization-approved.md`, M2/M4; not implementation or qualification evidence. This refines `.work/drafts/wdbx-native-design.md` §§2–5 and N01/N02. It intentionally moves that draft's ABI manifest work out of N01 into N05. Read the approved master and company consolidation entrypoint together: legacy products stay independent retained products, and this work does not qualify LYLX replacement. No source inventories, approval documents, source code or Git state are changed by this preparation.

All proposed paths below are relative to the owning repository, except explicitly company-relative references. Implementation starts from the integration-approved dependency cohort in its own task checkout, not by modifying `cohorts/000-baseline`. The baseline guide's five crates remain: compute/foundation/telemetry have no workspace dependencies; core uses foundation/telemetry; WDBX uses compute/foundation. N01/N02 need no new crate or ABI dependency, and no numerical backend edits.

## 1. Scope, identities, and validation levels

The distinct named identities are `wdbx-model-package-v1`, `wdbx-spatial-neural-v1`, `wdbx-byte-bpe-v1`, `cpu-scalar-f32params-f64accum-v1`, and (only when selected) `wdbx-online-learning-v1`. A new package is never a legacy segment, `.wdbxspecimen`, codec autoencoder, ABI bigram, Safetensors or GGUF artifact. Filename suffixes are presentation only. No caller may infer architecture, permission or trained quality from `.wdbx`.

Public construction boundaries:

1. `read_bounded_snapshot(Read, limits) -> PackageBytes`: owns one bounded byte snapshot. Read the 96-byte header, validate declared lengths with checked arithmetic, then read exactly that total plus an EOF probe. A short read is truncation; any extra byte is trailing data. The remaining snapshot is allocated once only after the cap check. No borrowed caller-mutable buffer survives this operation. A file descriptor, no-follow open, metadata check, or stable pathname alone does **not** make file bytes immutable against in-place writes.
2. `validate_framing(PackageBytes, limits) -> FramedPackage`: verifies outer framing, supported container version, directory, required-kind/version compatibility, and every component hash on those same owned bytes. It retains unknown optional components as bounded byte ranges. Its summary says `framing_validated`, never `model_validated`. It grants neither inference nor resume, and must not expose an execution constructor.
3. `validate_model(FramedPackage, limits) -> ValidatedModelState`: decodes the eight recognized components, validates every enum/value/canonical order, exact tensor membership/axes, tokenizer and cross-component identity/shape constraints. Only this type may construct a later N03 execution model. Its fields/constructors are private; deserialization cannot manufacture it. N02 may return it without supplying execution code.
4. `check_resume(ValidatedModelState, ResumeContext) -> ResumableState`: additionally requires the resumable purpose and exact caller-supplied dataset/split/preparation/objective identities and local authorized policy context. A checkpoint's claimed audience/license/policy is not an authorization grant. Actual execution/policy integration is N04–N06.

Semantic validity establishes a coherent supported model state, not a successful forward pass, useful weights, authorized activation or trained-model qualification. A zero/untrained fixture may be semantically valid and must be labeled `Diagnostic` provenance. Tests explicitly distinguish these outcomes. N05 must still perform ABI license/artifact/audience admission and real normal model selection after N03/N04 work; do not add a dummy provider, fake v1 manifest, fake URL or external tokenizer to satisfy current ABI manifest constraints.

## 2. Canonical binary grammar and outer framing

The schemas below are concrete structs, not a general map/tag language. Each field is written once in the listed order, with no implicit padding or native-layout serialization. Integers are unsigned little-endian fixed width. `F32/F64` are little-endian IEEE-754 binary32/binary64; every stored float must be finite, including optimizer moments. Signed zero is accepted and its exact bits preserved. No float normalization. No varints, alignment padding, JSON, CBOR, bincode, compression, executable expressions, URIs for operators, shaders or embedded programs.

Notation: `D` is 32 raw digest bytes; `I` is 16 opaque identity bytes (not a string UUID); `S` is `u32 byte_length` followed by strict UTF-8 bytes; `B` is `u32 byte_length` followed by arbitrary bytes; `A<T>` is `u32 count` followed by exactly count T values. `Opt<T>` is `u8 tag` (0 absent, 1 present) then T only for 1. Other option tags fail. A fixed `[T;n]` has no count. `S` is at most 4096 bytes and not silently Unicode-normalized. Descriptive strings are display-only, never paths to open. Counts must satisfy context-specific caps before reservation. An enum-backed `u8`/`u16` has only the discriminants named for that field; strings with fixed identity values must match exact ASCII bytes. A component must end exactly after its final field. There are no unrecognized trailing fields in schema 1.0. Unspecified enum discriminants fail. Maps below are ordered arrays with explicit keys and strict ordering, so duplicate keys fail. No native `usize`, `isize`, `bool` or Rust enum layout appears on disk.

The outer 96-byte header is exactly:

| Byte range | Type/name/value |
|---|---|
| 0..8 | `[u8;8] magic = ASCII WDBXMOD1` |
| 8..10 | `u16 major = 1` |
| 10..12 | `u16 minor = 0` |
| 12..16 | `u32 header_bytes = 96` |
| 16..24 | `u64 total_bytes` |
| 24..28 | `u32 component_count`, 8..32 |
| 28..32 | `u32 flags = 0` |
| 32..64 | `D directory_sha256` |
| 64..96 | `[u8;32] reserved = 0` |

Directory begins at 96, comprises exactly `component_count * 80` bytes, and is SHA-256 hashed as stored. Each entry is exactly:

| Entry byte range | Type/name |
|---|---|
| 0..4 | `u32 kind` |
| 4..6, 6..8 | `u16 schema_major`, `u16 schema_minor` |
| 8..12 | `u32 flags` (bit 0 required; all other bits zero) |
| 12..16 | `u32 reserved = 0` |
| 16..24, 24..32 | `u64 offset`, `u64 length` |
| 32..64 | `D component_sha256` |
| 64..80 | `[u8;16] reserved = 0` |

Kinds are strictly increasing and nonzero. Kinds 1..8 appear exactly once, flags exactly 1, schema exactly (1,0). They cannot be made optional. Unknown kinds (>=9) with flags 0 are annotations only; preserve their kind, version and bytes even if the version is unknown. Unknown required kind is `UnsupportedRequiredComponent`. Known kind with unsupported major **or minor** is `UnsupportedComponentVersion`; the reader does not guess that higher minor versions are safe. Container version other than (1,0) fails. Future readers must deliberately register supported versions and migration; changing a field meaning or enum interpretation requires a new component version, changing framing requires a new container version. Existing 1.0 fixtures remain frozen.

First payload offset is exactly `96 + 80*count`; subsequent offset is previous offset + previous length; final end is `total_bytes`. Lengths are positive. No gaps, overlap, alias, wrap, trailing file bytes or alignment bytes. Each component digest is plain `SHA256(component_bytes)`. Integrity verification precedes semantic decoding; tests that target semantics must recompute hashes rather than accidentally testing only checksum rejection.

Domain-separated identities (literal ASCII plus one NUL, no length prefix on the fixed domain):

* `PackageDigest = SHA256("wdbx-model-package-v1\0" || whole_file)`; ordinary `ArtifactSha256 = SHA256(whole_file)` also returned for ABI verification.
* `StateRoot = SHA256("wdbx-model-state-v1\0" || concat_{kind=1..7}(u32(kind) || u16(1) || u16(0) || component_sha256))`.
* `GenerationId = SHA256("wdbx-model-generation-v1\0" || I(model_id) || D(partition_id) || Opt<D>(parent_generation) || D(StateRoot) || D(component_8_sha256))`.

A package contains neither its own digest nor its own generation ID. Component 8 may contain parent identities, never its own root; no digest cycle. Computational state identity includes purpose, objective, moments and RNG/order, not only weights. Component 8 binds provenance/audience/policy/lineage into generation identity. Unknown optional annotations affect package/artifact digests, never StateRoot or GenerationId, and cannot change model semantics. Exporting a training checkpoint as inference-only removes resume state using the explicit absent schemas below and changes StateRoot/GenerationId; this is a derived artifact with its source generation in provenance, not the same resumable state. Exact re-export preserves all component bytes/optional annotations and is byte-identical.

N06's publication receipt/journal chooses a generation; this content identity is not a head pointer, publication sequence or authority proof. Re-selecting the same eligible generation on rollback creates a new publication event while preserving this identity and durable monotonic allocation history.

## 3. Component 1: architecture and compatibility (schema 1.0)

Field order:

```
S architecture = "wdbx-spatial-neural-v1"
S numerical_profile = "cpu-scalar-f32params-f64accum-v1"
u8 purpose                 # 0 InferenceOnly, 1 ResumableTraining
u8 learning_profile        # 0 OfflineExplicit, 1 OnlineV1
u16 operator_set = 1       # compiled spatial-recurrent-ops-v1, not instructions
u64 required_features = 3  # bit0 causal text, bit1 graph ranking; no other bits
u32 node_count
u32 edge_count
u32 hidden_width
u32 relation_count
u32 vocab_count
u32 message_steps
u32 max_sequence_tokens
u32 max_generated_tokens
u32 max_batch_examples
u32 node_capacity
u32 edge_capacity
u32 max_incoming_degree
u32 max_trainable_scalars
u64 max_session_working_bytes
u64 max_loaded_and_trainer_bytes
u64 max_training_scalar_ops
F64 spatial_scale
F64 coordinate_radius
```

The string and enum combination is the entire operator/compatibility declaration. Set 1 fixes gather/matvec/add/tanh/squared-distance/segment-softmax/scatter-sum/masked-cross-entropy/AdamW semantics; no serialized graph of arbitrary operator calls is allowed. No backend/device placement is claimed. `OnlineV1` denotes `wdbx-online-learning-v1` and still requires an external enabled policy. `OfflineExplicit` permits explicit training through a trusted caller; it does not alter the historical Specimen fixed/manual profiles.

Bounds: `2<=N<=node_capacity<=128`; `1<=M<=edge_capacity<=1024`; `1<=d<=64`; `1<=R<=8`; `260<=V<=4096`; `1<=K<=4`; `2<=max_sequence_tokens<=128` (includes BOS); `1<=max_generated_tokens<=128`; `1<=batch<=4`; `1<=max_incoming_degree<=16`. `max_trainable_scalars` is 1..1,000,000; actual scalar count across catalog 1..16 must not exceed it. A text session must also preflight that admitted input plus requested generated tokens fits max_sequence_tokens, and check the remaining context before every token; max_generated_tokens is an additional ceiling, not permission to overrun context. Resource declarations are positive and no greater than 256 MiB/session, 512 MiB/model+trainer, 500,000,000 scalar multiply/add operations/batch. They are ceilings, not trusted estimates. Effective limits are minima of these declarations, application policy and actual backend limits. N03/N04 compute the real working estimate before execution; N02 validates package-loading allocations only.

`spatial_scale` is finite strictly positive (diagnostic default 1.0). `coordinate_radius` is exactly 4.0 for this profile; no hidden configurable clamp. Coordinates belong to learned unitless `[-4,4]^3`. The default diagnostic dimensions remain N=16,d=16,K=2,M=48,V<=512,sequence<=64,generation<=32,batch=1; these are fixture choices, not mandatory dimensions. Nothing requires every combination of individual maxima to fit total resource limits.

## 4. Component 2: topology and stable-ID axes (schema 1.0)

Field order:

```
u16 coordinate_frame = 1       # LearnedUnitlessCartesian3
u64 node_id_high_water
u64 edge_id_high_water
A<Relation> relations
A<Node> nodes
A<Edge> edges

Relation := u32 relation_id, S label
Node     := u64 node_id, u8 role, Opt<D> source_ref
Edge     := u64 edge_id, u64 source_node_id, u64 destination_node_id,
            u32 relation_id
```

Relations are exactly R entries, IDs 0..R-1 in that order; nonempty labels, at most 64 UTF-8 bytes and unique by exact bytes. Relation IDs are immutable semantic axes for this architecture version; v1 topology operations cannot rename/reorder/add relations. Labels do not dispatch code. Nodes are exactly N entries strictly increasing by nonzero node ID; roles 0 Input, 1 Hidden, 2 Memory, 3 Readout. Exactly one Input and one Readout, necessarily distinct nodes. No other roles. A nonempty graph-ranking batch requires candidate Memory nodes, but a package with none may load and later return `NoEligibleCandidates`; it must not silently score Input/Readout nodes. `source_ref` is permitted only for Memory and is a reference into component 8's declared data roots; optional synthetic/learned memory nodes can omit it. Presence is not proof of access eligibility.

Edges are exactly M entries strictly increasing by nonzero edge ID. All endpoints and relation IDs exist. Duplicate `(source,destination,relation)` triples fail; self-edges and cycles are allowed. Incoming degree must obey component 1. Execution creates a separate permutation sorted `(destination_id,relation_id,source_id,edge_id)` for reductions; serialized edge rows and the gate tensor remain in EdgeId order. Never conflate reduction order with the persistent axis.

High-water marks are nonzero, >= maximum corresponding live ID and <=u64::MAX-1. Deletion/rollback cannot lower the durable lineage high-water mark. A standalone package can validate only against its own live IDs; N06 compares/imports it against lineage history and may require an allocation floor greater than its snapshot value. New IDs are floor+1 with checked arithmetic and never reused; reaching the maximum refuses growth. This cross-generation check must not be claimed as proved by N02.

Coordinates occur **only** in tensor 2, so they cannot disagree with a second copy in topology. No physical or display position is stored in these eight core components. A later typed annotation may carry those, but neither may become learned coordinates or affect execution implicitly.

## 5. Component 3: exact tensor catalog (schema 1.0)

Field order: `A<Tensor>` (exactly 16 entries, increasing tensor_id). Each Tensor:

```
u16 tensor_id
u8 dtype                    # 1 F32, 2 F64
u8 rank                     # 1..3 in this catalog
[Axis;rank] axes
u64 data_byte_length
[u8;data_byte_length] data   # little-endian row-major; last axis varies fastest

Axis := u8 kind, u32 length, [u64;length] stable_ids_if_kind_is_1_or_2
```

Axis kinds: 1 Node, 2 Edge, 3 Relation, 4 Token, 5 HiddenOut, 6 HiddenIn, 7 XYZ. For Node/Edge, stable ID arrays must equal the full ordered node/edge lists in component 2; no omission, duplication or dense-slot reinterpretation. Other kinds have **no** ID payload: Relation implicit 0..R-1, Token 0..V-1, hidden 0..d-1, XYZ exactly x,y,z indices 0,1,2. Descriptor ordering is semantic, not inferred from equal dimensions. For a hidden vector use HiddenOut. Matrices multiply column vectors: first index output, last index input.

| TensorId | Name | Dtype | Exact axes/shape | AdamW decay |
|---|---|---|---|---|
| 1 | b, trainable base states | F32 | Node(N), HiddenOut(d) | no |
| 2 | p, learned coordinates | F64 | Node(N), XYZ(3) | no |
| 3 | E, token embedding | F32 | Token(V), HiddenOut(d) | no |
| 4 | U, input projection | F32 | HiddenOut(d), HiddenIn(d) | yes |
| 5 | R, recurrent projection | F32 | HiddenOut(d), HiddenIn(d) | yes |
| 6 | W, relation messages | F32 | Relation(R), HiddenOut(d), HiddenIn(d) | yes |
| 7 | S, self projection | F32 | HiddenOut(d), HiddenIn(d) | yes |
| 8 | c, message bias | F32 | HiddenOut(d) | no |
| 9 | a, edge gate logits | F32 | Edge(M) | no |
| 10 | O, text head | F32 | Token(V), HiddenIn(d) | yes |
| 11 | o, text bias | F32 | Token(V) | no |
| 12 | J, graph query projection | F32 | HiddenOut(d), HiddenIn(d) | yes |
| 13 | Q, graph node projection | F32 | HiddenOut(d), HiddenIn(d) | yes |
| 14 | B, graph query-to-score projection | F32 | HiddenOut(d), HiddenIn(d) | yes |
| 15 | d_q, graph score bias | F32 | HiddenOut(d) | no |
| 16 | v, graph score vector | F32 | HiddenOut(d) | no |

All 16 tensors are mandatory, including graph tensors in a text-only diagnostic batch. All are trainable in this profile; no arbitrary frozen mask or optimizer subset. Checked products must equal data_byte_length/dtype_size exactly, fit memory counters and total scalar cap. Coordinate elements must be within [-4,4]. All other values finite; no arbitrary numerical magnitude cap is smuggled in here. Later forward execution rejects nonfinite intermediate results under its independent work/numerical limits. Zero-length axes and unknown/extra tensor IDs fail.

This fixes later execution ambiguities without implementing the network now. Text initializes `h[-1]=b`; at position t, `z0_i=tanh(R*h[t-1,i]+b_i+1[input] U*E[token])`. K shared-weight synchronous message layers use the source draft equations, no learned layer axis. `h[t]=zK`; `logits=O*h_readout+o`. Graph query has no structural special tokens; require at least one byte token after encoding, compute `q=J*mean(E[query_tokens])`, reset state, and initialize `z0_i=tanh(R*b_i+b_i+1[input] U*q)`. Run K identical message layers; score Memory node i with `v^T*tanh(Q*zK_i+B*q+d_q)`. Query token length obeys the same sequence cap; empty graph query returns `EmptyGraphQuery`. Every per-node tensor/moment row follows stable NodeId and every edge gate/moment follows stable EdgeId after topology edits. Tokenizer changes require explicit E/O/o and optimizer migration, not online topology mutation.

## 6. Component 4: tokenizer/encoder (schema 1.0)

Field order:

```
S identity = "wdbx-byte-bpe-v1"
u8 normalization = 0           # None; exact input bytes
u8 special_policy = 1          # structural IDs only through typed API
u16 reserved = 0
u32 pad_id = 0
u32 bos_id = 1
u32 eos_id = 2
u32 unk_id = 3
D training_corpus_digest
D training_split_digest
u32 minimum_pair_frequency = 2
u32 requested_vocab_size
A<B> vocabulary                # ID is array index
A<Merge> merges                 # rank is array index
Merge := u32 left_id, u32 right_id, u32 result_id
```

`260<=actual V<=requested_vocab_size<=4096`, merges count exactly V-260 <=3836. Vocabulary 0..3 have empty byte strings; bytes for IDs 4..259 are respectively single bytes 0..255. For rank r, result_id=260+r, both parents in [4,result_id), pair unique across prior ranks, and result bytes exactly concatenated parent bytes. No special ID in a merge. Non-special byte strings must be nonempty; **duplicate byte strings at different IDs are permitted** when valid different merge decompositions produce them. IDs/ordered merge ranks define encoding; a decoder must never deduplicate vocabulary rows. Individual token bytes <=65536 and total encoded tokenizer component <=1 MiB; check concatenation length before allocation. This 1 MiB cap can stop training before requested vocabulary size; return CapacityExceeded, not a silently incomplete successful trainer result. Fewer merges due to no pair reaching frequency 2 is a successful complete trainer result with actual V recorded.

Tokenizer training input is an ordered array of UTF-8 documents from the already admitted training split, no more than 4096 documents, 64 KiB/document and 16 MiB total bytes. Deduplication and train/eval splitting precede this task; trainer must not split or inspect evaluation documents on its own. `training_corpus_digest=SHA256("wdbx-bpe-corpus-v1\0" || u32(document_count) || concat B(document_utf8_bytes))`. `training_split_digest` is the externally supplied training-split identity and must be nonzero. An empty corpus has a valid domain-separated digest and yields base vocabulary only; a training caller can forbid it separately. Never fabricate a corpus digest when only an existing vocabulary is available: import requires its supplied provenance and is unqualified until authenticated externally.

Deterministic trainer: initialize every document to byte IDs, count adjacent pair occurrences (including overlapping occurrences), consider only pairs meeting frequency 2, choose maximum count then lexicographically lowest `(left_id,right_id)`, append that exact concatenated token and merge, replace its occurrences left-to-right without overlap within each document, repeat. No cross-document or structural-special merges. Pair frequencies use checked u64; IDs remain u32. Production inference encode starts at byte IDs and applies each stored merge once in rank order with the same left-to-right nonoverlap replacement; this is the normative algorithm. Later optimized encoders must produce identical IDs. Structural specials are inserted only by typed `encode_sequence` helpers, never recognized as textual spellings in user input.

`decode_bytes` accepts only non-special IDs, concatenates their exact byte strings, and fails on an out-of-range or structural ID. `decode_sequence_bytes` separately strips a single leading BOS and a single terminating EOS if present, permits PAD only as a suffix after EOS, rejects UNK/internal BOS/internal EOS/other PAD, then returns content bytes. Generated raw tokens/bytes remain available; incremental UTF-8 display is a separate presentation profile, not part of tokenizer identity. `decode_bytes(encode_utf8(s))==s.as_bytes()` for all admitted UTF-8 strings. Literal `<BOS>`, `<EOS>` and similar strings are encoded as bytes. Empty text is allowed: typed training sequence `[BOS,EOS]` has one prediction.

Bound work independently of byte/shape checks. Initial simple trainer maximum pair-scan visits plus replacement visits is 100,000,000; count each visited adjacent pair and each visited token once per replacement pass, refuse before exceeding the cap. Encoder maximum is 8,000,000 token visits; cap raw input 64 KiB and charge each token tested during each merge pass. Keep a live allocation budget 128 MiB for tokenizer training, within the broader 256 MiB package/session envelope. Fail early using a conservative preflight estimate; runtime counters enforce actual loops. Policy may lower these caps. Cancellation between documents and merge passes returns no vocabulary publication. No checkpoint/resume of a partial tokenizer-training pass in this version.

## 7. Component 5: objectives/data signatures (schema 1.0)

Field order:

```
u16 objective_profile = 1       # next-token+graph-ce+coordinate-l2-v1
u16 text_mask_profile = 1       # disallow PAD/BOS/UNK; allow EOS and byte/merge IDs
u16 text_reduction = 1          # mean over valid target tokens across batch
u16 graph_reduction = 1         # mean over graph examples, each CE over candidates
F64 text_weight = 1.0
F64 graph_weight
F64 coordinate_l2_weight
u16 sequence_policy = 1         # BOS, content, EOS; no packing/truncation
u16 graph_candidate_policy = 1  # explicit unique authorized Memory IDs, one target
u32 max_graph_candidates
D dataset_manifest_digest
D split_manifest_digest
D preparation_digest
```

graph_weight and coordinate_l2_weight must be finite >=0 (defaults 1 and 1e-4); at least one data objective with positive weight and a nonzero valid-example denominator appears in every training batch. Text weight fixed at 1.0. Candidate maximum in 1..node_capacity; selected candidates strictly increasing NodeId, no duplicates, target exactly one member. Dataset/split/preparation digests are nonzero and identical to component 7 when present. Their referenced manifests/records are external admitted data objects, not paths embedded in the package. The digest is an ordinary SHA-256 over exact immutable manifest bytes supplied by the data pipeline; N04 must define/verify the data pipeline's record format and verify its references before resume. N02 never fetches them or treats a digest as permission.

No per-example labels/masks/candidates in the model package. These are batch inputs: teacher forcing predicts next token from prefix; PAD suffix has no objective term, EOS is a valid final target; zero valid text targets fails for a present text objective. A missing objective contributes 0 rather than a divide-by-zero denominator. Graph loss is the mean of candidate-set CE per graph example, not an accidental token-weighted sum. Combined loss is `present_text_mean + graph_weight*present_graph_mean + coordinate_l2_weight*(sum_i ||p_i||^2/N)`. CE never admits PAD/BOS/UNK targets. Dataset preprocessing must preserve causal isolation; no label/target/peer-derived feature may enter earlier text computation.

## 8. Component 6: optimizer (schema 1.0)

Field order starts with `u8 state`: 0 Absent (component ends here), 1 AdamW. InferenceOnly requires Absent; ResumableTraining requires AdamW. No other variants. AdamW continues:

```
u16 optimizer_profile = 1       # adamw-globalnorm-matrixdecay-v1
u16 scheduler_profile = 1       # constant-lr-v1, no hidden scheduler counter
u64 successful_step
F64 learning_rate
F64 beta1
F64 beta2
F64 epsilon
F64 weight_decay
F64 gradient_clip_norm
A<Moments> moments              # exactly 16, increasing tensor_id
Moments := u16 tensor_id, u64 scalar_count,
           [F64;scalar_count] first_moment,
           [F64;scalar_count] second_moment
```

Require successful_step <u64::MAX, `0<lr<=1`, `0<=beta1,beta2<1`, `0<epsilon<=1`, `0<=weight_decay<=1`, `0<clip<=1000`. Diagnostic defaults 1e-3,.9,.999,1e-8,1e-4,1.0. Per-tensor sums follow the tensor table and row-major order for the numerical reference; moments inherit that ordering, not an incidental hash-map iteration order. Stronger local policy may narrow these ranges. Every second moment >=0; first moments finite. At step 0 every moment is numerically zero (signed zero preserved). Each count equals corresponding tensor scalar count, flattened in its exact persistent row-major axis order. No extra/missing/frozen members. Moment dtype is always F64 even for F32 tensors. The shape is inherited from the catalog; omitting redundant dimensions prevents contradictory moment axes. Moment arrays are never copied by dense position from a different topology: N04/N07 must first join the old and new tensor axes by their stable IDs, then write the new canonical order.

N04 uses this fixed order: global gradient norm across tensor IDs/row order, clip, Adam moments, bias correction at next successful_step, decoupled decay on only the table's yes tensors, update, coordinate projection into [-4,4], then cast F32 parameters once. Nonfinite candidate fails atomically without visible step, cursor or RNG advancement. Checkpoint only after a full successful step; no partial gradient/tape is serialized. The constant scheduler is sufficient for exact resume in this initial profile; another scheduler requires an explicit new optimizer schema/profile and fixtures.

## 9. Component 7: RNG and data order (schema 1.0)

Field order begins `u8 state`: 0 Absent and end (InferenceOnly), 1 StepBoundary (ResumableTraining), no other variant. StepBoundary continues:

```
u16 rng_profile = 1             # xoshiro256starstar-v1
u16 seed_profile = 1            # splitmix64-v1
u64 root_seed
A<Stream> streams               # exactly 5, ID order 1..5
u16 sampler_profile = 1         # fisher-yates-unbiased-u64-v1
u16 batch_policy = 1            # ordered contiguous; short final batch, no drop
u16 pending_state = 0           # no in-flight gradients/tape/candidate
D dataset_manifest_digest
D split_manifest_digest
D preparation_digest
u64 successful_step
u64 epoch
u32 example_count
u32 next_example_cursor
u32 batch_examples
A<u32> permutation
u64 accepted_topology_proposals
u64 next_topology_proposal_step

Stream := u8 stream_id, [u64;4] state_words, u64 consumed_u64
```

Stream IDs: 1 ParameterInitialization, 2 DataShuffle, 3 GraphNegativeSampling, 4 TextSamplingTemplate, 5 TopologyProposal. Each 256-bit state is not all zero; consumed_u64 <u64::MAX. TextSamplingTemplate is copied into a request-local sampler, not mutated by inference or evidence-free export; a continuing sampled generation session requires its own future session protocol. Persisting this seed template does not claim that model checkpoints resume in-flight inference sessions. Current explicit graph-candidate batches need not consume stream 3 but preserve it for the declared training sampler. No runtime RNG is substituted silently.

All u64 PRNG arithmetic alone wraps modulo 2^64; state/cursor/step/proposal counters elsewhere use checked arithmetic and fail on exhaustion. For xoshiro256** words `(s0,s1,s2,s3)`, output `rotl(s1*5,7)*9`; then `t=s1<<17; s2^=s0; s3^=s1; s1^=s2; s0^=s3; s2^=t; s3=rotl(s3,45)`; increment consumed count once per output. Initial streams use SplitMix with starting state `root_seed + 0x9e3779b97f4a7c15*stream_id` modulo 2^64. Each SplitMix draw increments that state by 0x9e3779b97f4a7c15, then `z=(z^(z>>30))*0xbf58476d1ce4e5b9; z=(z^(z>>27))*0x94d049bb133111eb; return z^(z>>31)`; four outputs are the initial words. Seed initialization does not count as xoshiro output consumption. At consumed_u64=0 the state must equal the seed-derived initial state for that stream. For later counters the loader validates the stored nonzero state and bounds; it does not replay arbitrarily many draws to establish its history. The provenance/equivalence harness, not the counter alone, establishes actual deterministic replay. Literal initial/output vectors must be pinned independently in N01.

Shuffle builds `[0..example_count)` and iterates i from count-1 down to 1. For bound b=i+1, let threshold=`(0-b) mod 2^64 mod b`; draw r until r>=threshold, choose j=r mod b, swap i/j. This rejection rule removes modulo bias; every rejected draw still advances the stream and consumed count. Record actual order and post-shuffle stream, not seed alone. No shuffle when count=1. `1<=example_count<=65536`, permutation length=count and a bijection of 0..count-1. cursor in 0..count-1, batch_examples in 1..component1.max_batch_examples; later N04 uses min(batch_examples,count-cursor) for the short final batch. Boundary normalization is immediate: after the final successful batch, increment epoch, generate the next epoch's permutation, set cursor=0 before committing the step. Thus cursor==count is never serialized and no ambiguous pre/post-shuffle checkpoint exists. Step count must equal component 6; dataset triplet must equal component 5.

`accepted_topology_proposals` and `next_topology_proposal_step` are checked counters, with zero next step meaning proposals disabled. If nonzero it must be >successful_step at a committed boundary. OfflineExplicit requires both zero in v1. OnlineV1 may carry counters, but N06/N07 own policy permission, schedules and migration tests. No topology proposal payload or arbitrary executable state is admitted here. Counters alone are not permission to adapt topology.

## 10. Component 8: provenance and compatibility references (schema 1.0)

Field order:

```
I model_id
D partition_id
u8 origin                       # 0 Diagnostic, 1 Imported, 2 LocallyTrained
Opt<D> parent_generation
Opt<D> parent_state_root
Opt<D> derivation_generation     # e.g. source resumable generation of inference export
Opt<I> operation_id
D audience_digest
Opt<D> policy_digest
u64 policy_epoch
A<SourceCohort> source_cohorts
A<DataRoot> data_roots
A<D> license_digests
Opt<D> validation_report_digest
u32 restrictions                # bit0 diagnostic-only, bit1 no-external-export

SourceCohort := S name, S revision, D source_tree_digest
DataRoot     := D content_digest, D consent_or_eligibility_digest,
                u8 kind         # 0 training-corpus, 1 dataset-manifest,
                                # 2 split-manifest, 3 source-memory,
                                # 4 preparation-manifest
```

Model I, partition D, audience D and all referenced digest/operation values must be nonzero. Parent generation and parent state are both absent or both present; when a parent is present operation_id is required. No timestamp is needed for the canonical identity; optional export annotations may carry display times without pretending they are witnessed time. `source_cohorts` <=32, names strictly increasing UTF-8 bytes, nonempty name/revision strings. `data_roots` <=1024, sorted `(kind,content_digest)`, unique keys, kinds only as listed. license_digests <=64, strictly increasing bytes, nonempty for Imported and LocallyTrained; an empty Diagnostic license list means unknown/unqualified, not public-domain permission. Consent/eligibility digests are references to evidence, not embedded grants. Signed metadata/verifier trust remains external.

Required root coverage: kind0 covers component4.training_corpus_digest; kind2 covers component4.training_split_digest and component5.split_manifest_digest (one entry if equal); kind1 covers component5.dataset_manifest_digest; kind4 covers component5.preparation_digest; every topology source_ref is kind3. A root may not appear with conflicting eligibility for the same `(kind,digest)` key. Extra declared roots are permitted within bounds; undisclosed source training must not be inferred safe from a sparse list. N04/N06 verify actual dependency completeness against the data/lineage service. `restrictions` only bits0..1, Diagnostic requires bit0. Restrictions can reduce permissions but never expand them. Inference/exports must obey external authority even if flags=0.

OnlineV1 requires policy_digest present and policy_epoch>0. OfflineExplicit requires policy_digest absent, policy_epoch=0 in this format; explicit offline training still requires caller authority. The validation report digest is optional because this component cannot recursively embed an evaluation of its own final bytes; external signed admission/evaluation records may bind the completed package/generation. A present report refers to an earlier independent report whose declared subject must be checked by N05/N06, not accepted because a hash-shaped field exists.

## 11. Aggregate resource and hostile-input rules

Default artifact 64 MiB; directory 32 records; tokenizer component 1 MiB; strings 4 KiB; schemas flat with bounded arrays/options (no recursive nesting). Decoded metadata aggregate <=2 MiB: all component bytes **except** tensor numeric payloads, optimizer moment numeric payloads and tokenizer vocabulary byte payloads; tokenizer descriptors/counts remain metadata. Unknown optional components count fully as metadata. This definition prevents optimizer arrays being accidentally limited as metadata while retaining a separate 1 MiB total tokenizer cap. Metadata accounting uses checked byte charges while parsing, not declarations trusted as estimates.

N02 preflight loading upper bound = owned snapshot + typed numerical buffers + copied variable byte/string/axis/permutation buffers + parser tables + fixed 16 MiB overhead reserve; ceiling 256 MiB. Return CapacityExceeded if unable to prove the bound before reservation. Prefer validated views into the immutable snapshot for opaque/tokenizer payloads so they need not be copied; count retained snapshots once and forbid hidden whole-file clones. Use fallible exact reservations for all attacker-sized vectors; a budget failure is a typed error, not process abort. The later loaded model+trainer 512 MiB ceiling includes whichever snapshot/views remain alive and all training copies; N03/N04 must recompute it. A writer streams hashes/bytes with a 64 KiB work buffer or owns a single bounded output snapshot under the same budget, never multiple unchecked package-sized Vecs.

Reject overflow in offset+length, header+directory, count*entry, dimensions*dtype, metadata sums and usize conversions, before slicing/reserving. Bound streaming loops by admitted byte totals; EOF/read interruptions are not permission to retry an unbounded mutable stream. Cancellation terminates before publishing an object. Error families: Io, Truncated, TrailingBytes, BadMagic, UnsupportedContainerVersion, UnsupportedRequiredComponent, UnsupportedComponentVersion, NonCanonical, IntegrityMismatch, MissingComponent, CapacityExceeded, InvalidTopology, InvalidTensor, InvalidTokenizer, InvalidObjective, InvalidOptimizer, InvalidRngOrder, InvalidProvenance, NotResumable, ResumeMismatch, Cancelled. Expose field/component and bounded offset context, never entire payloads or private source data. Choose deterministic first-error order: outer/header bounds; directory canonicality/version; component hashes in kind order; component schemas in kind order; cross-component relationships; caller resume context.

`import_staged` writes a fresh task-owned private object from the validated snapshot; it never reopens the input path and assumes it still matches. Use create-new temporary file in the destination object directory, write, sync file, promote without clobbering another object to `<PackageDigest.hex>.wdbx`, sync directory using the platform-specific filesystem wrapper. If destination exists, verify the existing object's exact digest and size before treating it as the same object; a mismatch is corruption, never overwrite. The object store API must state platform durability guarantees and typed unsupported cases. N02 can implement bounded staging and export, but no current-head mutation or generation-store journal. N04/N06 own crash-durable checkpoint/publication; a successful rename alone is not their acceptance. Preserve task-owned incomplete objects for explicit cleanup/retention; do not delete unknown existing objects or age-based leftovers.

## 12. Requirement mapping (exact coverage.json identities)

These are mappings, not status promotions. Company source is `docs/plans/modernization/coverage.json`; inherited `source_document` values retain their original repository ownership. `M2` and `M4` below are only readable shorthand for approved master sections, never new requirement IDs.

| Exact coverage identity | N01/N02 responsibility / retained obligation |
|---|---|
| company `wdbx-model-package-v1` | Direct: complete versioned package grammar, identities, bounded validation, fixtures and export/import objects. Publication remains N04/N06. |
| company `wdbx-spatial-neural-v1` | Prerequisite only: catalog/axes/operator vocabulary support actual recurrent text+graph execution; N03/N04 prove execution/training. |
| company `wdbx-online-learning-v1` | Prerequisite only: explicit profile, complete state/lineage references; N06/N07 own authorized automatic updates, publication, isolation and recovery. |
| abi `C23` | Original Rust causal decoder training remains binding; this spatial architecture does not discharge it. |
| abi `C24`, `C25`, `C26`, `C27` | Versioned tokenizer/checkpoint/optimizer, deterministic resume, licensed provenance/split isolation and broad frozen evaluation retained. N01/N02 supply contracts/fixtures, not passing runtime/quality evidence. |
| wdbx `PLAN-0073`, `V6-1403` | Preserve legacy record/hash/checkpoint/WAL/locking and historical reader behavior; native package dispatch separate. |
| wdbx `V6-1215` | Explicit unsupported format/reader-floor failure; no silent reinterpretation. |
| wdbx `V6-0459` | Artifact/source/tokenizer/input-output identity and permissions retained; N05 implements ABI v2 admission and license acceptance, never automatic fetching. |
| wdbx `V6-0589`, `V6-0628` | Consent lineage, held-out evidence and scope remain authority checks, not provenance assertions accepted from a file. |
| wdbx `P08.04`, `H-P08.04`, `V6-0986` | Historical fixed non-learning synthesis remains its own profile; native online learning amendment is explicit, no global relaxation. |
| wdbx `V6-1067` | Original specimen save/load preserved; native package excludes credentials, handles, threads, temporary activations and executable continuations. |
| wdbx `V6-1080` | New capabilities negotiated separately from episode/storage and transport identities; control/TS/ABI wiring remains later M2/N05. |

Additional exact company keys (prefixes are spelled out so an implementer can join without guessing):

* `abbey-abi-and-wdbx-completion-and-modernization-plan/1-scope-and-binding-decisions::d0bf62f1cc767b2d`: preserve WDBX storage and specimen formats/new explicit identities.
* `abbey-abi-and-wdbx-completion-and-modernization-plan/2-revised-architecture-and-interfaces/one-logical-store-consistent-learning-generations::43c2c9d60b47229f`: all independently versioned package components, validated identity rather than extension.
* `abbey-abi-and-wdbx-completion-and-modernization-plan/2-revised-architecture-and-interfaces/wdbx-as-a-trainable-3d-neural-system::6e0c963be11394a3`: learned, physical and display coordinates distinct.
* `abbey-abi-and-wdbx-completion-and-modernization-plan/2-revised-architecture-and-interfaces/wdbx-as-a-trainable-3d-neural-system::88f1fbac1a2acfef`: bounded capacity/operators/depth; no arbitrary executable operators.
* `abbey-abi-and-wdbx-completion-and-modernization-plan/3-ordered-implementation-work/2-establish-compatible-contracts-and-a-reproducible-baseline::244cf3484e5dc9f7`: architecture/training/generation contract portion only.
* `abbey-abi-and-wdbx-completion-and-modernization-plan/3-ordered-implementation-work/2-establish-compatible-contracts-and-a-reproducible-baseline::f30d9ab3c89d1f8a`: bounded artifact content outside 64-KiB control frames; transfer/auth integration remains later.
* `abbey-abi-and-wdbx-completion-and-modernization-plan/3-ordered-implementation-work/4-implement-the-wdbx-model-package-and-native-cpu-architecture::444ecf9198d7e556`: package validation/loading/content identity/export-import part now, checkpoint publication later.
* `abbey-abi-and-wdbx-completion-and-modernization-plan/3-ordered-implementation-work/4-implement-the-wdbx-model-package-and-native-cpu-architecture::3274d4aa0ed0cc47`: old model/storage readers remain under original identities.
* `abbey-abi-and-wdbx-completion-and-modernization-plan/3-ordered-implementation-work/4-implement-the-wdbx-model-package-and-native-cpu-architecture::9f44b9b0f3411234`: ABI runtime registration/normal selection explicitly N05, not N01/N02.
* `abbey-abi-and-wdbx-completion-and-modernization-plan/3-ordered-implementation-work/4-implement-the-wdbx-model-package-and-native-cpu-architecture::f357e052beec3278`: entire actual-network train/improve/generate/restore acceptance remains open after N02.

No inventory row is deleted, deduplicated or relabeled qualified by these tasks. Preserve standalone original Specimen, neural codec, tokenizer/training and substrate obligations; retain accelerator/TPU/distributed/platform and production model quality as distinct later evidence dimensions.

## 13. N01 implementation packet: freeze contracts and independent witnesses

**Owned WDBX files (only these, plus explicitly reviewed splits):**

* `docs/plans/2026-10-25-native-model-cpu.md` (task sequence/requirement mapping; does not replace company master).
* `docs/contracts/model/{README,package-v1,components-v1,tokenizer-v1,fixture-policy}.md`.
* `crates/abi-wdbx/src/model/{mod,ids,limits,error,contract}.rs`; `contract/` submodules if needed for <=1000-line rule. Add only `pub mod model;` and appropriate module docs in `crates/abi-wdbx/src/lib.rs`.
* `crates/abi-wdbx/tests/model_contracts.rs` and `crates/abi-wdbx/tests/fixtures/model-v1/{README.md,manifest.json,...}`. **Never root `tests/model_*.rs` or `tests/fixtures/model-v1`.** Checked-in fixture `.hex`/`.sha256`/descriptive `.json` children are inventoried in manifest.json with explicit schema/expected validation stage.
* `tools/wdbx_model_reference.py`, `tools/tests/test_wdbx_model_reference.py`; `tools/check.sh` adds one narrow fail-required invocation of that Python suite without weakening any current gate.

N01 has no ABI-owned files, registry modifications, source/lock changes, tokenizer crate dependency or actual provider. Use existing sha2/thiserror as needed. Contract types contain exact enums/constants/catalog/caps; no callable empty model, todo! forward path or stub that returns canned logits. Type names must separate FramedPackage and ValidatedModelState from the outset; production package implementations follow N02.

Sequence: (1) encode this contract in reviewed docs/types, (2) independently author standard-library Python bytes/hash/schema witness, (3) freeze literal small fixture bytes/digests plus human-checkable field tables, (4) write Rust contract tests against those constants, (5) prove independent negative controls by mutating known bytes and by deliberately breaking a witness expectation. Expected bytes are not computed by the production Rust serializer. Witness should parse/validate the whole catalog, not just compare a Rust-provided digest. Tests must fail if Python is unavailable or disagrees; no skip or auto-regeneration.

Minimum positive fixtures: an inference-only 3-node/2-edge/d=2/R=1/V=260/K=2 model; a resumable equivalent with nonzero step/moments and valid permutation/streams; nonempty BPE fixture; legal signed-zero payload; unknown optional annotation; same numerical state/different partition changes GenerationId; optional annotation changes PackageDigest but not GenerationId; row identities nonconsecutive to expose dense-slot assumptions; EdgeId order differs from execution reduction order. Fixture weights are literal diagnostics, not claimed learned artifacts. Literal PRNG vectors are contract witnesses, not a training run.

N01 negatives: missing/optional-required component; unknown required component; unsupported container or recognized component minor; reserved bits/bytes; noncanonical directory/tensor IDs; wrong axes even when lengths equal; shifted node/edge stable IDs; bad relation/role/missing Input or Readout; half-present parent identities; unsupported operator/profile; absent training state with resumable purpose; unexpected state with inference-only purpose; malformed BPE base/special/merge rank/result/cycle; same-size payload tamper; unknown optional payload masquerading as mandatory topology/tensor content. Include a frame-valid but semantically-invalid package that must **not** produce ValidatedModelState.

**Acceptance:** docs/types/witness/literal fixture hashes agree; old readers untouched; contract tasks cannot produce runtime-qualified claims. Fresh independent specification review checks all listed bytes, enums and axis relationships. Run focused contract/Python checks first, then full WDBX gate as permitted by dependency-cohort readiness. Contract acceptance does not make N02 implementation complete.

## 14. N02 implementation packet: bounded reader/writer, staging, real tokenizer

Depends on N01 reviewed bytes and the integration-approved dependency baseline. **Owned WDBX files:**

* `crates/abi-wdbx/src/model/package/{mod,header,directory,components,read,write,stage}.rs`; split components into `components/{manifest,topology,tensors,tokenizer,objective,optimizer,rng,provenance}.rs` before a large file forms.
* `crates/abi-wdbx/src/model/tokenizer/{mod,train,encode,decode}.rs`.
* Narrow necessary edits to `model/{mod,contract,limits,error}.rs` and fixture additions under `crates/abi-wdbx/tests/fixtures/model-v1/`.
* `crates/abi-wdbx/tests/{model_package,model_tokenizer}.rs`, optional dedicated `model_package_bounds.rs` if size demands, and independent Python witness/test extensions for N02 properties. Keep published N01 goldens unchanged; add versioned new vectors for intentional semantics changes.

Implement in regression-first order: bounded immutable snapshot and hashes; directory canonicality/version checks; individual schema parsers; all semantic cross-links; writer/re-export preserving opaque bytes; tokenizer encode/decode; deterministic trainer/work budgets; explicit staged object import/export. Tokenizer core has no file I/O, network, authority API or global mutable state. Pure decoder returns data; package never invokes script/compiler/plugin/provider code. Host staging receives an explicit destination object directory owned by the test/application and cannot select a live store by default.

Negative/independent matrix beyond N01:

* Truncate at each header/directory/field boundary and representative payload offsets; append bytes; zero/overflow counts/offsets/products; duplicate/unsorted components; gaps/overlap/alias; oversized metadata/strings/vocab/permutation; impossible declared allocations; adversarial many-small allocations and attempted extra package copy.
* Inject valid recomputed hashes around NaN/Inf coordinates/tensors/moments, negative second moments, dtype mismatch, transpose axes, one wrong gate row, missing graph tensors, forged moment membership/count, invalid sequence/objective tags, mismatched dataset/split/step, duplicate permutation, cursor==count, all-zero PRNG state, consumed counter overflow, missing lineage/source roots.
* Mutate/rewrite/replace the source path after snapshot read and during staged import; validated output must derive from the captured owned bytes only. Simulated concurrent in-place changes during reading either yield an integrity/semantic failure or a fully internally valid captured snapshot; external artifact digest checks still bind the authorized bytes. No path reread after verification. Test partial reads, interrupted I/O, EOF probe, cancellation and existing-destination corruption without changing selected state.
* Encode/decode UTF-8 multilingual, combining marks, emoji spanning token boundaries, whitespace, empty text, literal special-looking text, invalid raw decode IDs; duplicate-byte-string vocabulary with legal alternative merges; pair ties/overlapping `aaa`/`aaaa`, document boundaries, no pair>=2, maximum short-run work counter; stop before allocation/work cap. Assert exact token IDs from independent vectors, not only byte round trip.
* Check trainer exact output against independent slow Python on small corpus permutations and tie cases; order affects corpus digest but pair counts/tie rules may produce identical merges. Training corpus inputs are training-only and held-out leakage sentinel documents must be absent. Test cancellation leaves no partial successful tokenizer. Rust/Python round-trip catches swapped field order; a pinned altered expected digest must fail.
* Feed new package bytes to existing legacy format/specimen readers through existing public entry points where supported; they must refuse or retain their existing handling, never dispatch by extension. Existing frozen format and Specimen compatibility suites run unchanged. Do not reimplement those readers in this task.

Acceptance requires genuine production parser/writer/tokenizer using the frozen byte contract, frame/semantic/resume distinctions and bounded work/allocations. A writer-only round trip, fixture stub, SHA-only validator or tokenizer that ignores merges is insufficient. No topology training, inference execution, ABI activation or model publication may be claimed. A framed unknown optional component is retained exactly but cannot satisfy missing required components or change any tensor/operator semantics.

## 15. Gates, evidence and rollback

Commands are instructions for future implementers, not checks executed by this draft. Use the exact isolated WDBX task checkout; resolve DEV_HOME dynamically (default `~/dev`), pinned stable Rust 1.99.1 and `/usr/bin` before compiler shims; separate CARGO_TARGET_DIR from baseline/concurrent workers; no shared writable live store. Keep stdin at EOF. Each task uses fresh implementer, independent specification/code review, then final integration review per approved SDD; tests must use their actual named targets so a broad substring does not produce a misleading pass.

```
python3 -m unittest discover -s tools/tests -p 'test_wdbx_model_reference.py'
cargo test --locked -p abi-wdbx --test model_contracts < /dev/null
# N02 after its targets exist:
cargo test --locked -p abi-wdbx --test model_package --test model_tokenizer < /dev/null
# Include model_package_bounds explicitly if split into its own target.
bash tools/check.sh < /dev/null
PYTHONDONTWRITEBYTECODE=1 python3 tools/check_completion.py
python3 -m unittest discover -s tools/tests -p test_check_completion.py
```

Do not run cargo concurrently against the baseline worker's target. Preserve the repository's old cross-language/legacy fixtures and opt-in golden policy. ABI has no changes in N01/N02; its existing `./tools/check.sh` consumer gate runs at integration when the cohort owner schedules it. No bare cargo in ABI. ABI admission/selection and exact CLI/MCP fixture checks are N05 after actual model execution. No new TypeScript/control schema is hand-copied here. Studio/hardware/optional feature tests remain separate when their surfaces are affected; default green does not qualify those.

Receipt records input source/lock hashes, owned diff, exact command/toolchain/target, exit code/output digest, all fixture digest/schema identities and independent witness result, peak/bounded allocation cases, unresolved failures, reviewer decisions and explicit open downstream tasks. Tests expose the failing baseline of new behavior before the minimal implementation. Do not weaken limits or regenerate witnesses to bless a failure. No build/test/training execution took place for this design artifact.

Rollback N01: remove only unintegrated new model-module export/contracts if necessary; retain reviewed frozen fixture versions in history, never repurpose a published identity. No old format or registry rollback is needed because none changed. Rollback N02: disable the new native import/export entrypoint, preserve existing model/readers and all existing selected generations; retain task-owned staged objects until explicit safe cleanup. No head has been changed. Once N04/N06 publish a generation, rollback is their authenticated publication event, not deleting a package, rewinding step/RNG/history, or lowering stable-ID high-water marks.

## 16. Remaining boundaries and reviewer watchpoints

N03/N04 must implement and independently validate the actual recurrent spatial forward/backward network, scalar numeric profile, exact initialized tensor/RNG consumption, loss/optimizer order, coordinate influence, causal suffix isolation and real objective improvement/restore. N02's package acceptance cannot substitute for those gates. N04 must define the external dataset manifest/preparation record encoding it hashes; N02 has no data downloader. N05 owns a genuine ABI v2 native-package admission schema, old v1 fixture compatibility, principal-bound licenses, authority, authenticated artifact streams and actual Models/CLI/MCP selection. N06 owns durable one-logical-store generations, publication atomicity, audience isolation, revocation and anti-replay. N07 owns topology proposals with stable-ID/moment migration and allocation-floor history.

Notable deliberate choices: exact minor-version support, all eight required components including explicit absent state, 16 fixed semantic tensors, duplicate BPE token bytes allowed with unique ranked pair identities, no serialized activations, complete post-step RNG/order, no automatic permission inferred from provenance, immutable owned byte snapshot, and exclusion of opaque optional annotations from semantic identity. If future capabilities need to make annotation data computational or security-authoritative, they must introduce a recognized required component/profile and new fixtures; merely naming an optional component is insufficient.
