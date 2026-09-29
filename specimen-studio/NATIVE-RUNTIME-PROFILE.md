# Native runtime profile · 0.2

This profile records current implementation boundaries. Completion depends on the separate verification report.

## 1. Authority and interpretation

The approved Native Runtime Expansion adds a versioned implementation profile and optional generative extensions. Original design mechanisms remain authoritative requirements. An implementation statement is not acceptance evidence; platform, model, persistence and UI evidence are tracked separately in the verification report. The original 26 chapters remain intact.

## 2. Architecture and responsibility boundaries

The desktop shell is Tauri 2 with bundled React assets and Geist fonts. Cycles, reviews and maintenance run in this native process; every edit runs in the same Rust kernel compiled to WebAssembly inside the webview (the CSP allows `wasm-unsafe-eval` for that). The engine owns durable state in a dedicated application-data directory. WDBX abi-wdbx and abi-compute use pinned revision 3ac03f085196dbe6e922a127ceaa37bc2368de89. ABI, Abbey and WDBX sibling repositories remain unchanged. Cycles clone a consistent snapshot and commit through revision checks. Progress and cancellation use a separate IPC channel.

## 3. Terminology and identifiers

Text IDs use text-v2 and visual IDs use visual-v2; the complexity class is part of the encoder profile. Identical IDs mean identical quantized feature signatures. Quantization boundaries can separate similar inputs. A matching ID is candidate evidence only. Stable node and entry references identify contributors independently of encoder IDs. Legacy logical Pattern IDs remain in the presentation projection, while nativeIds maps entry and resource references to rebuilt encoder IDs. This is an explicit migration map, not silent replacement of existing references.

## 4. Invariants

Native commits validate schema, network shape, bounded collections, original-pattern uniqueness, identities, references, action syntax and limits. Stale revisions fail without replacing committed state. Cancelled cycles publish no partial durable edits. Generated text is escaped as an inert literal action when explicitly learned. Installed pretrained weights and persisted transient-network weights do not change through feedback. Mutation lineage is monotonic under ordinary edits.

## 5. Logical data models

The native envelope is wdbx.native.v2: schema, revision, specimen, network, visuals, artifacts, tombstones and nativeIds. The specimen field preserves the wdbx.studio.v1 presentation model. A visual record links an original asset digest, focus, analysis, node reference and OCR correction. A generated artifact records prompt, seed, model identity, pinned model digest, parameters, timestamp and optional asset digest. Activations, jobs, leases and process credentials are ephemeral.

## 6. HybridTable and storage organization

Native persistence uses a transactional WDBX V2Store key-value record for the coherent specimen envelope. It does not reinterpret WDBX episodes. Derived retrieval maps are rebuilt from original patterns. Repeated encoder IDs across nodes are allowed so every candidate remains reachable; bounded entry tables and unique original patterns continue to apply. WDBX compaction is attempted periodically after committed revisions. The browser retains its array/hash HybridTable and IndexedDB store.

## 7. Language Governance and input preparation

The initial native language profile splits independent clauses at semicolons and bounded conjunctions, preserves source spans, emits simple subject/predicate/object records, expands bounded thesaurus alternatives, captures numbers and scopes negation per clause. “Do not calculate 2 + 2; hello” suppresses the calculation while retaining the greeting. This is a deterministic lexical profile; unrestricted natural-language semantic understanding is not claimed.

## 8. Sigil scopes and action scripting

Native actions parse into nested Text and Call expressions. A six-scope registry checks permitted names before evaluation, with declared reads and no arbitrary persistent writes. Numeric bind sigils, arithmetic, repetition, tape automata, resource lookup, temporal lookup, recall, tone and imagination execute through bounded handlers. The new literal(JSON-string) extension preserves generated text without interpreting embedded sigils. Unknown or out-of-scope native calls fail with InvalidScope. Browser compatibility retains its separate limited interpreter.

## 9. Pattern-node activation and votes

All entries selected by exact encoder IDs, explicit binds, wildcards and automata are independently scored. Text confidence uses 100 times token intersection/union minus its complementary dissimilarity, with successful explicit structural binds evaluated separately. Visual confidence compares color histograms, spatial occupancy, signed-distance fields and hole mismatch. Evidence exposes similarity, dissimilarity, deterministic ATP modulation, temporary jitter and final confidence. Signed values clamp to [-100,100] and qualification uses confidence >= threshold. Same-ID collisions can fail this gate.

## 10. Node attachments and automatic wiring

Attachment traversal carries the actual lexical unmatched remainder, deduplicates visited jobs and limits traversal to four hops. Hard links and affinity-sampled soft links remain distinct. Repeated co-activation records bounded correlation evidence and can create soft links after three observations. The first profile limits each cycle to eight unique contributors for correlation updates and caps automatic attachments at 4,096. Semantic attachment correlation beyond this observable lexical evidence remains unqualified.

## 11. Cycle lifecycle and orchestration

The native trace records preparation, retrieval, raft coverage, deep scan, votes and composition. Qualifying votes are collected before orchestration. Redundancy is resolved within an originating input group using confidence and strength, while complementary actions and attachment groups survive. Each output segment retains source-vote and contributor references. Image cycles enter the same orchestration path with independently computed visual scores. Action errors become visible rejected-action output instead of partially committed state.

## 12. Pattern-ID-guided supporting retrieval

Supporting lookup preserves explicit legacy resource-to-node handles and compares native IDs computed from original resource keys against input/action keys. Relevance filtering then checks context. The native profile builds one resource index per consistent cycle snapshot, then performs indexed lookups and relevance filtering. Persistent incremental index maintenance remains an optimization, not a requirement for complete retrieval. Missing resources produce explicit bounded fallback output rather than invented knowledge.

## 13. Transient Memory and imagination

The fixed native network is sparse feed-forward CSR with 128 inputs, 64 hidden units and 32 composition outputs. Initial layers use eight connections per output row, persisted weights and biases, and ephemeral activation vectors. Inputs encode retrieved text and learned visual features, context, familiarity and ATP. A seeded biased coin selects a layer activation: familiar input favors sigmoid at probability 0.75, unfamiliar input at 0.25; the alternative is ReLU. CPU execution is the reference. Optional wgpu computes matrix products, reports its backend, and falls back explicitly on failure. Shape, finite-value and composition checks reject invalid output. Brainstorming perturbs composition sampling by at most 0.05 per output and leaves fixed weights intact. Saved results retain composition parameters and execution metadata; raw activation vectors remain ephemeral.

## 14. Action Tone Predictor and stochastic behavior

ATP valence/intensity state is separate from confidence jitter and fixed-network activations. The initial profile exponentially decays state on a 180-second time scale, applies bounded chargebook contributions, reduces intensity on release and prevents additional charging during a configurable 30-second cooldown. Confidence modulation is deterministic and bounded; jitter is newly sampled for eligible comparisons and does not accumulate. Meaning-preserving language re-inflection beyond the implemented tone handlers requires further qualification.

## 15. Conversation memory and temporal coherence

Conversation history retains inputs, segments, votes, trace, feedback and pins. Temporal lookup reads actual stored timestamps and pinned/history records. Rolling limits preserve pinned records separately. A native save restores committed history and starts with no active jobs; it does not replay unfinished cycles or model inference. Node and history views are virtualized and searchable. Resource and artifact views currently retain bounded display windows, which are separate from their stored record limits.

## 16. Visual perception and ephemeral fuzzy art

Images enter through file input, paste or drag-and-drop. Originals are stored by SHA-256; a normalized movable focus defines the crop while the UI blurs peripheral regions. The initial encoder downsamples to 64 by 64, extracts boundary points, holes, local convex/concave indicators, a 64-bin RGB histogram and 16 spatial occupancy features. A distinct DistanceField type stores actual signed distances; aligned composition parameter arrays remain another type. OCR uses ocrs 0.12.2 and RTen 0.24 with pinned detection/recognition artifacts. Text lines and regions retain provenance and may be corrected before explicit Learn. The simple shape descriptors and observed OCR accuracy are documented as profile limits, not general vision accuracy claims.

## 17. Non-pattern nodes

Type A reviews recent unmatched cycles with an explicit matching context, within a default 90-second residual window. A correction links correctionOf to its originating cycle and the original is marked reviewed, allowing at most one correction. Type B inspects bounded recent unmatched history and creates a learning proposal with pros, cons, evidence count and observer identity. Neither path silently adds inferred patterns. The present Type B profile is repetition-based contextual evidence; unrestricted internal-state inference remains outside its demonstrated behavior.

## 18. Contributor provenance and feedback

Votes expose node and entry references, actions, scores, source input/group, bindings and supporting-resource references. Segments expose contributing nodes, originating votes and transformations. Feedback remains an independent strength update for each eligible specimen contributor and never trains pretrained models. Visual learning preserves original asset and OCR correction records. Generated artifacts retain model provenance until explicitly learned. Explicit learning links the artifact to its new node and retains model or corrected-OCR provenance. Direct model-panel requests have no specimen contributors; that empty lineage must not be filled with invented contributors.

## 19. Growth, idle mutation, and PHAGY

Native idle mutation selects stronger donors, compares original patterns and literal vote text independently, keeps stable alternative slots, preserves inhibition and permanent remix markers, and rejects reciprocal borrowing through lineage records. Eligible literal text uses second-order Markov transitions; structured programs are excluded. Changes commit transactionally only after validation. PHAGY uses an exclusive cancellable lease, cleans retained history/activity and orphaned attachment references, rebuilds derived IDs on commit, and removes expired scratch or unreferenced asset files after a configurable default 24-hour retention. Asset reclamation conservatively preserves recovery copies and all currently referenced assets. Each deletion is idempotent and cancellation leaves committed specimen references intact.

## 20. Index Rafts, chunking, and scheduling

Native Index Rafts partition candidate work into disjoint chunks, join results in stable order and continue over all chunks. The 1,000 comparison ceiling governs active work, never total records searched. Scheduler jobs and worker counts bound actual concurrency. Cancellation is checked between partitions and comparisons. UI foreground work cancels its owned idle operation before proceeding; a full engine-level fairness queue and durable resume cursor remain qualification items. Browser rafts remain cooperative tasks on one JavaScript execution lane.

## 21. Compartmental contact ledger

The registry exposes name, allowed scopes, declared reads and writes. Reserved scope grants no calls. Privileged model processes receive only bounded prompts, fixed parameters and owned artifact paths through the native provider manager. The local text server binds authenticated loopback; image jobs run as supervised child processes. Model generation is currently a dedicated typed UI operation rather than a general sigil callable from every scope. Production builds omit the WebdriverIO test bridge and its permissions.

## 22. Commands

The visible studio continues to support /prompt, /right, /wrong, /wrng, /contributorfractal, /saveSpecimen, /loadSpecimen, /addPattern and /learn. Existing documented action aliases remain in the specification. Native image analysis, image-cycle execution, recognized-text execution, model install/import/remove/unload, generation and cancellation are exposed through the desktop workspace. Text and image artifacts require explicit Learn. Commands that open editors or file pickers retain those reviewable UI steps.

## 23. Configuration and limits

Native defaults retain 20 entries/table, confidence threshold 62, jitter amplitude 1.5, initial/max strength 5/10, fan-out 12 and history 100,000. Idle delays are freshly sampled from 60–120 seconds. Autonomous proposals, mutation and maintenance remain user-controlled. Native bounds include 8,000 prompt/action bytes, 2,000 pattern bytes, 12,000 resource-value bytes, 16 sigil nesting levels, 32 KiB action output, 1,024 tape cells and 10,000 tape instructions, eight active jobs, 64 workers/job and 4,096 records/chunk. Images are limited to 32 MiB and 8,192 pixels per axis; native manifests to 64 MiB and portable archives to 512 MiB expanded. CPU inference memory estimates are admission requirements, not measured universal minima.

## 24. Persistence and restoration

Native import stages validation before activation. Browser v1 originals, identities, strengths, inhibition, history, pins and mutation records are preserved. Derived IDs rebuild from original patterns with an explicit reference map. The source import and prior snapshot are retained as recovery copies. Portable .wdbxspecimen ZIP archives contain a versioned manifest and content-addressed required visual assets; model files are referenced by identity/digest. Archive traversal, oversized entries, unsupported schema and digest mismatch are rejected. Durable edits use expected revisions. Deleted-node tombstones preserve lifetime mutation constraints. Crash recovery uses committed WDBX state and never resumes old jobs automatically.

## 25. Implementation decisions and acceptance criteria

Acceptance is recorded by layer: engine unit/conformance tests; browser desktop/mobile regression; native UI on each OS/architecture; real OCR; real model inference; accelerator tolerance; installers; signing/notarization. The macOS test bridge is feature-gated and absent from release builds. A built runtime or verified model download does not prove generation; a packaged installer does not prove native UI behavior on another OS. Camera input, hosted inference and online training remain excluded. The source includes executable tests and a platform qualification workflow, while the verification report identifies outstanding evidence.

## 26. Source coverage map

This revision preserves every original chapter and its source-coverage map. The Native implementation profile paragraphs are additions derived from the approved expansion plan and current source, explicitly separating bounded initial mechanisms, proposed requirements and unresolved acceptance. Optional Qwen/SDXL generation is a new extension, not a retroactive reinterpretation of the original specimen notes.
