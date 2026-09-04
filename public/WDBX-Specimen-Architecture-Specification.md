# WDBX Specimen Architecture and Behavioral Specification

**Document version:** 2.0 · **Prepared:** 2026-09-04
**Design source:** Donald Filimon's complete user-authored notes in [Hand off specification](https://chatgpt.com/c/6a9a81c9-d454-83ea-94d9-66838ebe9549), including the later updates and clarifications.  
**Status:** Consolidated design specification. This document describes intended behavior; it does not certify an existing implementation or measured performance.

> A specimen stores learned patterns and supporting resources persistently, retrieves a bounded working set through Pattern IDs, lets nodes independently contribute gated votes, and composes those contributions through an ephemeral orchestrator. Transient Memory supplies temporary synthesis. Feedback, approved learning, attachment formation, and bounded idle-time mutation govern persistent change.

## Contents

1. [Authority and interpretation](#1-authority-and-interpretation)
2. [Architecture and responsibility boundaries](#2-architecture-and-responsibility-boundaries)
3. [Terminology and identifiers](#3-terminology-and-identifiers)
4. [Invariants](#4-invariants)
5. [Logical data models](#5-logical-data-models)
6. [HybridTable and storage organization](#6-hybridtable-and-storage-organization)
7. [Language Governance and input preparation](#7-language-governance-and-input-preparation)
8. [Sigil scopes and action scripting](#8-sigil-scopes-and-action-scripting)
9. [Pattern-node activation and votes](#9-pattern-node-activation-and-votes)
10. [Node attachments and automatic wiring](#10-node-attachments-and-automatic-wiring)
11. [Cycle lifecycle and orchestration](#11-cycle-lifecycle-and-orchestration)
12. [Pattern-ID-guided supporting retrieval](#12-pattern-id-guided-supporting-retrieval)
13. [Transient Memory and imagination](#13-transient-memory-and-imagination)
14. [Action Tone Predictor and stochastic behavior](#14-action-tone-predictor-and-stochastic-behavior)
15. [Conversation memory and temporal coherence](#15-conversation-memory-and-temporal-coherence)
16. [Visual perception and ephemeral fuzzy art](#16-visual-perception-and-ephemeral-fuzzy-art)
17. [Non-pattern nodes](#17-non-pattern-nodes)
18. [Contributor provenance and feedback](#18-contributor-provenance-and-feedback)
19. [Growth, idle mutation, and PHAGY](#19-growth-idle-mutation-and-phagy)
20. [Index Rafts, chunking, and scheduling](#20-index-rafts-chunking-and-scheduling)
21. [Compartmental contact ledger](#21-compartmental-contact-ledger)
22. [Commands](#22-commands)
23. [Configuration and limits](#23-configuration-and-limits)
24. [Persistence and restoration](#24-persistence-and-restoration)
25. [Implementation decisions and acceptance criteria](#25-implementation-decisions-and-acceptance-criteria)
26. [Source coverage map](#26-source-coverage-map)

## 1. Authority and interpretation

The complete source was read from the referenced conversation, rather than inferred from its truncated preview. The substantive design appears in one long user message with successive updates. Later explicit clarifications govern conflicts with earlier passages. Prior assistant-like commentary embedded in that message is treated as design material only where consistent with those clarifications.

This document uses four labels:

- **Required:** A mechanism or constraint stated in the notes and retained here. “Must” expresses these requirements or the consolidated rules below.
- **Resolution:** An explicit editorial reconciliation needed to make conflicting passages work together. These choices are visible and reviewable; they are not presented as quotations or previously approved implementation decisions.
- **Recommended:** An implementation measure that makes the design concrete without claiming it came from the source.
- **Open:** A detail the notes do not settle. An implementation must choose and document it before claiming conformance for the affected feature.

“Lazy-conservative” means work is event-driven or infrequent, has bounded resource consumption, prefers reuse of relevant information, yields to higher-priority work, and decays toward a baseline. It is a scheduling and modulation policy, not a complexity proof.

### 1.1 Contradiction and normalization register

| ID | Tension in the notes | Consolidated interpretation |
|---|---|---|
| R01 | Early table handles must be unique; later node IDs explicitly may repeat. | Later rules prevail. A Pattern ID indexes a collection of logical nodes. Internal record references distinguish individual nodes without making their public node IDs unique. |
| R02 | A node is a table, yet several nodes have the same table/Pattern ID. | A node is a logical pattern-entry table. HybridTable owns its physical storage. One Pattern-ID bucket may contain several such node tables. |
| R03 | Every table holds 20 patterns; an ID search may return “all 20” matching nodes. | Twenty is the default **entry capacity per node**, not a maximum number of nodes sharing an ID or a retrieval-result cap. Return every matching node. |
| R04 | Duplicate IDs become legal; exact repeats remain forbidden. | ID equality is legal. Exact repeated original patterns in the same compatible Pattern-ID population and exact duplicate node structures remain disallowed. Meaning-equivalent paraphrases remain legal. The exact structural equality codec is open. |
| R05 | Side entries all have lifetime Pattern IDs; later resource IDs are optional and must match unprefixed node IDs. | Separate the mandatory generated `pattern_key` used for structural indexing from an optional `resource_id` used for node-linked retrieval. The latter must match a node's logical ID. This two-field interpretation preserves both mechanisms; see §12. |
| R06 | Nodes compete through same-triple spin-bottle selection; later all independently qualifying nodes may contribute. | Voting remains independent. Spin-bottle arbitration removes redundant alternatives for the same originating triple, including fan-outs, while complementary contributions survive. The semantic redundancy predicate is open. |
| R07 | Crystallized attachments “always fire,” but the attached node may not vote. | Hard attachment guarantees scheduling of the target on the unmatched remainder. The target still evaluates its own gate. It does not guarantee an unconditional vote. |
| R08 | “Transformer” and possible MLP weight updates appear before a firm no-learning clarification. | Transient Memory is a non-learning sparse matrix/MLP synthesis utility. No online weight training or persistent knowledge updates follow merely from using it. An attention mechanism is optional and unresolved. |
| R09 | Almost all values jitter, but IDs are stable and crystallized/endpoint confidences have exceptions. | Only explicitly eligible runtime numeric values receive bounded temporary jitter. IDs, structure, permissions, hard limits, and durable facts do not drift. Endpoint and crystallization exceptions prevail. |
| R10 | Confidence is bounded to ±100, while selection uses confidence × strength. | The product is a separate selection weight, not a confidence score. Sampling must convert it to nonnegative finite weights. |
| R11 | Idle events occur every 1–2 minutes, but an example says 3 seconds. | Adopt a fresh random interval in **[60,120] seconds of eligible idle time**. A three-second event requires a separately configured interval; it is not part of that default. Distribution and pause/reset policy are implementation choices. |
| R12 | History capacity is written `100,00`. | Interpret as **100,000 message/answer records**, explicitly an editorial default. Record granularity and whether the intended count was 10,000 require confirmation if compatibility matters. |
| R13 | Index Rafts stop on a match, while duplicate-ID queries must return all matches. | Provide distinct `find_first` and `find_all` traversal modes. Node/resource ID lookup uses `find_all`; first-hit stopping is inappropriate there. |
| R14 | Index Rafts are described as nearly free and eliminating loop overhead. | Preserve them as resumable, partitioned, yielding traversal for latency management. Dispatch, scheduling, traversal, and synchronization have costs; recursion does not remove total scan work. Benchmark selection, rather than assume a speedup. |
| R15 | Sigil macros are said to avoid Big-O overhead. | Macros support reusable structural/procedural behavior; their execution cost depends on the operation. No asymptotic or constant-cost guarantee is implied. |
| R16 | “Nonlinear SDF” is described as aligned pixel-like arrays. | Preserve that term and exact payload fields as the specimen's visual representation. The mathematical distance-field encoding is not yet defined; a true signed-distance field is not established by those arrays alone. |
| R17 | Type B nodes can self-edit, but newly inferred knowledge requires `/learn id`. | Type B may edit permitted internal operating state. Promotion of newly inferred knowledge into persistent learned structures requires the user's identified learning approval. |
| R18 | Save “all systems” conflicts with ephemeral work and provenance. | Save all durable specimen contents, system definitions, and coherent ongoing modulation state. Do not serialize live threads, GPU handles, temporary activations, or executable in-flight continuations by default. In-flight checkpoint/resume is a separate feature. |
| R19 | A weak vote pool is shortened to donor length; each slot may remix only once. | Align only `min(recipient_count, donor_count)` alternatives and never grow the recipient pool. Preserve permanent remix markers by stable slot reference. Reject a shortening operation that would erase an already remixed slot unless an explicit tombstone/retention policy exists. |
| R20 | Weak nodes are removed at zero strength, but grave handling is unfinished. | Remove the active node and its active indexes at zero. Preserve separate growth-inhibition and mutation-history records. Grave retention and resurrection rules remain open. |
| R21 | Pattern-bind macros may match broad procedural/wildcard inputs that have unrelated literal IDs. | Compute ordinary Pattern IDs from the appropriate normalized binding representation. Maintain an explicit bind-rule candidate route for wildcards/automata that cannot be covered by one literal key. This route retrieves candidates; it does not silently bypass limits or inhibition. |
| R22 | “Every system's ID is shared” could equate low-, medium-, and high-resolution strings. | The universal lookup API is shared. Equality still requires compatible modality, resolution class, and encoder version. Identical bare strings are insufficient. |
| R23 | The scan ceiling is described as both 1,000 pattern scans and 1,000 nodes. | Count actual simultaneous high-resolution comparisons across the specimen. A multi-entry node acquires permits per comparison or scans entries sequentially; it does not multiply the ceiling by its entry count. |

### Native implementation profile · 0.2

The approved Native Runtime Expansion adds a versioned implementation profile and optional generative extensions. Original design mechanisms remain authoritative requirements. An implementation statement is not acceptance evidence; platform, model, persistence and UI evidence are tracked separately in the verification report. The original 26 chapters remain intact.

## 2. Architecture and responsibility boundaries

The architecture separates **address**, **retrieval**, and **reasoning**:

| Layer | Question answered | Mechanisms |
|---|---|---|
| Address | Where might the thing be? | Pattern ID, node ID, context ID, optional resource ID, internal record reference |
| Retrieval | How is it found with acceptable latency? | HybridTable, equality indexes, array/hash selection, chunks, Index Rafts |
| Reasoning | What is applicable, and what should happen? | Deep scans, similarity/dissimilarity, pros/cons, contextual filtering, vote composition |

These distinctions also define the major organs:

| Organ | Owns | Principal output |
|---|---|---|
| Language Governance | Linguistic interpretation and reusable lexical rules | Normalized variants, triples, contextual constraints |
| Sigil systems | Scoped macros, captures, procedural scripts, reserved operations | Bindings and permitted executable structures |
| Pattern-node population | Learned pattern → vote relationships and entry-level gates | Zero or more qualified votes per node |
| Non-pattern node population | Contextual pros/cons evaluation | Residual corrective votes or permitted internal edits and learning proposals |
| Orchestrator | Post-vote structural reasoning and step ordering | Composed response/action plan and final output |
| Transient Memory | Temporary computation, transformation, and synthesis | Ephemeral intermediate or final synthesized material |
| ATP | Persistent tonal dynamics and bounded runtime modulation | Confidence adjustments, tonal tags, arousal and cooldown state |
| Visual systems | Perception, visual pattern representation, rendering | Visual nodes/resources, OCR text, ephemeral fuzzy art |
| Growth/maintenance systems | Correlation, wiring, mutation, inhibition, cleanup | Explicit persistent updates and maintenance results |
| HybridTable | Storage and retrieval infrastructure for every organ | Addressable records with clear lifecycle boundaries |

```text
User input / permitted internal stimulus
                  |
      Language preprocessing and bounded fan-out
                  |
       Relational triples and scoped bindings
                  |
      Pattern-ID classification and retrieval
                  |
       High-resolution deep scans and gates
                  |
          Temporary qualified vote list
       =========== phase boundary ===========
                  |
      Ephemeral JIT orchestrator
       |       |          |          |
  Side data   ATP    Scoped scripts   History
       \       |          |          /
         Contextual composition
                  |
      Transient Memory when needed
                  |
     Response / actions + contributor provenance
                  |
     Feedback and residual Type A supervision

Idle: Type B correlation, attachment statistics,
      mutation-tag / PHAGY maintenance, approved growth
```

No central winner is elected during the ordinary voter phase. Nodes decide what they have evidence to contribute. Orchestration decides what can be constructed from those contributions.

### Native implementation profile · 0.2

The desktop shell is Tauri 2 with bundled React assets and Geist fonts. A runtime adapter selects Rust desktop operations or the existing browser executor. The engine owns durable state in a dedicated application-data directory. WDBX abi-wdbx and abi-compute use pinned revision e55634cbb581c1de02f946a29c34db8cf5203704. ABI, Abbey and WDBX sibling repositories remain unchanged. Cycles clone a consistent snapshot and commit through revision checks. Progress and cancellation use a separate IPC channel.

## 3. Terminology and identifiers

| Term | Meaning |
|---|---|
| **Specimen** | A complete configured instance, including all persistent organs, nodes, scripts, history, and maintenance state. “Grug” in the notes is a presentation name, not a separate architecture. |
| **Node** | A logical record/container with strength, a node type, configuration, and type-specific content. A pattern node is also called a node table. |
| **Pattern entry** | An original pattern paired with a vote definition inside a pattern node. |
| **Pattern ID** | A deterministic structural grouping string produced by the selected feature-extraction transformation. It is not an authentication credential and does not prove meaning. |
| **Pattern key** | The complete address: `(modality, resolution_class, encoder_version, pattern_id)`. Version qualification is a recommended compatibility measure. |
| **Node ID** | The non-unique logical identifier exposed for node addressing. For pattern nodes, it corresponds to the node's Pattern key/Pattern ID in a declared namespace. |
| **Context ID** | A non-pattern node's “context magnet.” Its matching/selection behavior must be defined by the node type. It is not automatically a text Pattern ID. |
| **Resource ID** | An optional, non-unique link from a side resource to an existing compatible, unprefixed node ID. |
| **Record reference** | A recommended stable internal identifier for one stored object. Necessary to target a particular node/entry despite duplicate logical IDs. Not a change to public ID semantics. |
| **Vote** | A node's qualified reasoning/action contribution. A stored vote may contain several weighted action alternatives and inhibitions. |
| **Vote alternative** | One `action[inhibition]^weight` item in a vote's weighted pool. Distinct from a node's pattern-entry slot. |
| **Cycle** | One input/stimulus evaluation and its associated vote collection and orchestration. Related residual supervision may continue afterward. |
| **Spin bottle** | Weighted stochastic selection among eligible alternatives in a specifically defined conflict/redundancy group. |
| **Crystallized node** | A node currently at maximum strength; qualified confidences lose jitter. This state can be lost. |
| **Crystallized attachment** | A hard attachment that always schedules the linked evaluation when triggered. Separate from node-strength crystallization. |
| **Index Raft** | A resumable, bounded traversal task assigned a disjoint index range. |
| **PHAGY** | The named idle cleanup mechanism using sparse, ephemeral automata with unique jobs and timeouts. No expansion of the name is specified. |

### 3.1 Pattern-ID classes

Classify input complexity **before** generating the Pattern ID:

```text
low complexity    -> low-resolution feature transformation
medium complexity -> medium-resolution feature transformation
high complexity   -> high-resolution feature transformation
```

The class-specific transformations should group sufficiently similar structures into the same key while retaining greater structural variability at higher resolutions. This is an intended property to validate, not a guarantee that all semantically similar patterns collide.

Every stored pattern must regenerate its containing node's Pattern key under that node's declared modality, resolution, and encoder. Inputs and entries compared by equality must use compatible transformations. A high-resolution ID is not directly equal to a low-resolution ID.

The **secondary deep scan always uses high resolution**, regardless of the coarse retrieval class. Resolution of retrieval and quality of verification are separate settings.

### Native implementation profile · 0.2

Text IDs use text-v2 and visual IDs use visual-v2; the complexity class is part of the encoder profile. Identical IDs mean identical quantized feature signatures. Quantization boundaries can separate similar inputs. A matching ID is candidate evidence only. Stable node and entry references identify contributors independently of encoder IDs. Legacy logical Pattern IDs remain in the presentation projection, while nativeIds maps entry and resource references to rebuilt encoder IDs. This is an explicit migration map, not silent replacement of existing references.

## 4. Invariants

| ID | Invariant |
|---|---|
| I01 | All durable specimen organs use HybridTable-managed storage with explicit subsystem boundaries. |
| I02 | Public node IDs and optional resource IDs may repeat; ID retrieval returns all matching records. |
| I03 | Pattern-node entries preserve their original text or original visual representation. Pattern IDs supplement rather than replace that content. |
| I04 | Each entry's Pattern key equals its containing node's key. Default capacity is 20 entries per pattern node. |
| I05 | Exact structural repeats are rejected; semantic paraphrases are permitted. Hash equality alone is not a duplicate-content proof. |
| I06 | Pattern-ID equality is retrieval only. It computes no confidence and proves no semantic applicability. |
| I07 | Ordinary pattern-node votes require high-resolution verification and the configured gate. Explicit bind-rule overrides have declared semantics. |
| I08 | Confidence stays within `[-100,100]`; arithmetic must not wrap, overflow silently, or propagate invalid numbers. |
| I09 | Jitter is bounded and temporary. It never accumulates into the baseline simply through repeated reads. Exact ±100 confidence endpoints do not jitter. |
| I10 | At maximum node strength, baseline-qualified confidences do not jitter. Lower confidences retain normal jitter eligibility. Dropping below maximum removes this protection. |
| I11 | A node can contribute every independently qualified entry vote. One node is not restricted to one vote per cycle. |
| I12 | Node reasoning ends at vote submission. Higher-order composition belongs to orchestration. |
| I13 | Attachment groups are excluded from ordinary same-triple duplicate arbitration; attached targets evaluate unmatched remainders. |
| I14 | All deep pattern comparisons share a global active-scan cap, **1,000 by default**, including attachments. Queued candidates are not silently discarded. |
| I15 | Side-system inference retrieval uses equality candidates followed by contextual relevance filtering, without a node-style secondary confidence scan. |
| I16 | Transient Memory performs no online learning and owns no learned knowledge merely by synthesizing an output. |
| I17 | Provenance is created and propagated during composition, not guessed afterward. |
| I18 | Feedback coin flips are independent per affected contributing node. Color-targeted feedback selects a contributor subset. |
| I19 | Each recipient mutation slot is remixed at most once in its lifetime; donor relationships and inhibited original pairings survive ordinary operation. |
| I20 | Newly inferred Type B knowledge is stored only after `/learn <proposal_id>` approval. |
| I21 | Sigil scope and contact-ledger permissions are enforced independently of textual name equality. |
| I22 | Raft ranges are disjoint and complete for the required traversal; the first starts at index 0. Every continuation yields. |
| I23 | User hard resource ceilings constrain adaptive scheduling, urgency, brainstorming, automata, and maintenance. |
| I24 | Saving and restoring covers the entire persistent specimen, not just node tables. |

### Native implementation profile · 0.2

Native commits validate schema, network shape, bounded collections, original-pattern uniqueness, identities, references, action syntax and limits. Stale revisions fail without replacing committed state. Cancelled cycles publish no partial durable edits. Generated text is escaped as an inert literal action when explicitly learned. Installed pretrained weights and persisted transient-network weights do not change through feedback. Mutation lineage is monotonic under ordinary edits.

## 5. Logical data models

The following schemas define concepts and relationships. Field spelling, wire encoding, and numeric representation are recommendations rather than a claim of an implemented file format.

### 5.1 Specimen and node records

```text
Specimen
  schema_version
  specimen_ref
  configuration
  pattern_encoders[]
  nodes: HybridTable<Node>
  side_systems: Map<SubsystemName, HybridTable<Resource>>
  sigil_registries: Map<Scope, HybridTable<SigilDefinition>>
  attachments: HybridTable<Attachment>
  conversation_history
  pinned_history
  atp_state
  transient_memory_definition
  inhibitory_rules
  mutation_history
  correlation_statistics
  learning_proposals
  maintenance_configuration
  contact_ledger

PatternNode
  record_ref                  # unique internal reference
  node_id                     # non-unique public logical ID
  type: text_pattern | image_pattern | registered_custom_pattern
  pattern_key
  strength
  jitter_enabled
  entries: PatternEntry[]     # default maximum: 20
  node_configuration

PatternEntry
  entry_ref
  original_pattern
  bind_representation         # derived, when macros are used
  vote: VoteDefinition
  binding_configuration

VoteDefinition
  tone_tag                    # e.g. happyVote
  alternatives: VoteAlternative[]

VoteAlternative
  slot_ref                    # stable despite renumbering
  action
  inhibition
  weight
  remix_state                 # never-remixed | remixed
  donor_relationships[]

NonPatternNode
  record_ref
  node_id                     # if exposed by this node type
  context_id                  # context magnet
  type: A | B
  strength
  pros_cons_rules
  permitted_reads
  permitted_writes
  timing_and_cooldown
  type_specific_configuration
```

The source's illustrative XML remains valid as a conceptual pattern-node shape:

```xml
<node tableId="pattern-id" strength="x" jitter="true">
  <entry>
    <pattern>original plaintext pattern</pattern>
    <vote>happyVote="do this[do not do that]^weight"</vote>
  </entry>
  <!-- Up to the configured entry capacity; default 20. -->
</node>
```

Non-pattern configuration preserves the separate context magnet:

```xml
<node contextID="context magnet" strength="x" type="A" />
<node contextID="context magnet" strength="x" type="B" />
```

XML is illustrative configuration notation. The required specimen save format is JSON.

### 5.2 Resources, bindings, attachments, and runtime votes

```text
Resource
  record_ref
  subsystem
  original_representation
  pattern_key                 # generated at insertion, stable for this record
  resource_id?                # optional node-linked logical ID; duplicates allowed
  payload
  entry_version

SigilDefinition
  record_ref
  original_representation     # original semantic definition/description
  pattern_key                 # same insertion-time indexing discipline as Resource
  resource_id?                # optional compatible node-linked ID
  name
  scope
  origin: reserved | user_defined
  kind: token | functor | automaton
  parameters
  expansion_or_program
  permitted_calls
  execution_limits

Attachment
  attachment_ref
  endpoint_refs[]             # concrete nodes, not ambiguous duplicate IDs
  direction: directed | bidirectional
  affinity
  hard_attach                 # attachment crystallization
  trigger_configuration

RuntimeVote
  vote_ref
  cycle_ref
  node_ref
  entry_ref?
  selected_action
  inhibition
  tone_tag
  activating_pattern_keys[]
  vote_pattern_keys[]
  source_chunk_ref
  fanout_origin_ref
  captured_original_values
  baseline_confidence
  effective_confidence
  strength_snapshot
  attachment_group_ref?
  provenance_ref
```

### 5.3 Provenance and maintenance records

```text
OutputSegment
  segment_ref
  text_or_artifact_span
  contributors[]              # concrete node and entry references
  transformations[]           # composition, substitutions, sigils, synthesis
  source_votes[]
  supporting_resources[]      # recommended extension to distinguish support
  provenance_id

LearningProposal
  proposal_id
  proposed_structure
  evidence_references[]
  contextual_explanation
  status: pending | approved | rejected | stale

MutationRecord
  recipient_node_ref
  recipient_entry_ref
  recipient_slot_ref
  donor_node_ref
  donor_entry_ref
  donor_slot_ref
  original_pairing_ref
  weight_choice: recipient | donor
  mutation_result
  timestamp

GrowthInhibitionRule
  original_pattern_vote_pair
  derived_pattern_keys[]
  fanout_constraints
  reason
  mutation_ref
```

Distinct `entry_ref` and `slot_ref` fields resolve an overloaded word in the notes: the former identifies a pattern/vote pairing; the latter identifies one weighted alternative inside that vote. The Markov pool-length examples apply to alternative slots.

### Native implementation profile · 0.2

The native envelope is wdbx.native.v2: schema, revision, specimen, network, visuals, artifacts, tombstones and nativeIds. The specimen field preserves the wdbx.studio.v1 presentation model. A visual record links an original asset digest, focus, analysis, node reference and OCR correction. A generated artifact records prompt, seed, model identity, pinned model digest, parameters, timestamp and optional asset digest. Activations, jobs, leases and process credentials are ephemeral.

## 6. HybridTable and storage organization

HybridTable is the storage abstraction for nodes **and every side organ**. It uses an array for small collections and a hash-backed representation for larger ones. The transition threshold is configurable and may be selected by the efficiency sandbox.

It supplies clear subsystem boundaries, insertion, exact-content validation, equality lookup, enumeration, update, and drop-table-like removal. A logical drop must identify the intended record/container; dropping one node must not remove all other nodes that share its public ID.

**Recommended removal consistency:** remove or deactivate attachment endpoints to the deleted concrete node. A node-linked resource ID stays valid while any compatible same-ID node remains. If the last matching node is removed, preserve the side resource and its structural Pattern key while detaching the now-unresolvable optional link; retain former linkage as historical metadata if needed. Mutation/provenance history may refer to removed records through historical references. Such references are not live attachment targets and do not imply a resurrection policy.

### 6.1 Indexes

Recommended logical indexes:

```text
nodes_by_pattern_key: PatternKey -> [NodeRecordRef]
nodes_by_public_id:   LogicalID  -> [NodeRecordRef]
resources_by_key:     (Subsystem, PatternKey) -> [ResourceRecordRef]
resources_by_link:    (Subsystem, ResourceID) -> [ResourceRecordRef]
records_by_ref:       RecordRef -> concrete record
```

A physical hash table may maintain one unique bucket key while the bucket's value holds many distinct records. That implementation detail does not make node IDs unique.

Default node entry capacity does not bound the number of records returned by an ID. If a new unique pattern belongs to an already-full node, **recommended behavior** is to create another logical node with the same key, subject to population limits. Never overwrite an existing entry implicitly.

### 6.2 Content equality and stable indexing

Use content comparison after a structural fingerprint match to reject exact repeats. Do not collapse paraphrases because they share an intended meaning. Store original strings without destroying their distinct surface forms; store normalized/binding forms separately.

Pattern-ID generation occurs once per inserted side entry. A change that alters the indexed original representation should create a replacement version with a freshly generated key and atomically update indexes. This preserves the source's lifetime-stability rule without leaving stale addresses after edits. Vote-only mutation does not change an unchanged activator's node Pattern key, but any cached vote-text keys must be invalidated.

The full-save file loads into this storage layer. It is not only a dump of one global node hash table.

### Native implementation profile · 0.2

Native persistence uses a transactional WDBX V2Store key-value record for the coherent specimen envelope. It does not reinterpret WDBX episodes. Derived retrieval maps are rebuilt from original patterns. Repeated encoder IDs across nodes are allowed so every candidate remains reachable; bounded entry tables and unique original patterns continue to apply. WDBX compaction is attempted periodically after committed revisions. The browser retains its array/hash HybridTable and IndexedDB store.

## 7. Language Governance and input preparation

Language Governance supplies reusable language tools to nodes and orchestration. Its entries follow the same plaintext-plus-Pattern-ID storage discipline as other linguistic/semantic resources.

### 7.1 Required utilities

| Utility | Stored information | Use |
|---|---|---|
| Dictionary | Word/phrase → definition; slang, concepts, technical terms | Definition and concept expansion, including procedural action support |
| Thesaurus | Synonyms and intensity weights in `[0,1]` | Input fan-out and output anti-staleness |
| Anti-Thesaurus | Context-sensitive sense distinctions | Disambiguate terms such as “race” using a context phrase |
| Chargebook | Phrase/word valence in `[-1,1]`, intensity in `[0,1]` | ATP, tonal coherence, contextual emotional interpretation |
| Verb Semantics | Subcategorization frames and required slots | Determine subject, object, recipient, and other verb roles |
| Conjunctive Rules | Rules for “and,” “or,” “but,” and “if–then” | Combine and constrain matches |
| Negation Handler | Negation scope and affected action/charge | Inhibit “don't X” and apply charge changes where appropriate |
| Basic Morphology | Lightweight stemming/lemmatization | Relate “running” and “runs” to “run” |
| Grammatical Role Mapper | Surface order → relational roles | Build triples using verb frames |

Named source dependencies are `HashTableHelper`, `PinealEntropy`, and `PatternScannerHelper` for tokenization. Their exact implementations are not prescribed.

### 7.2 Input pipeline

```text
raw input
 -> language preprocessing
 -> bounded stochastic thesaurus fan-out
 -> relational triple construction
 -> relational-triple sigil resolution
 -> pattern-bind normalization/capture
 -> complexity classification and Pattern-ID retrieval
 -> deep pattern scans
 -> qualified votes
 -> orchestration
```

Fan-out applies to **all user inputs**, including ordinary language and procedural requests. It randomly samples a bounded number of useful structural variants and stops at a configured ceiling. It must not enumerate unlimited combinations.

Each variant retains its raw-input reference, source span, and fan-out origin. Each relational triple chunk is independently classified and retrieved. Logical sequencing is one chunk at a time; the deep-comparison work within the admitted set may use the bounded scheduler.

A relational triple contains subject, predicate, and object slots. Any slot can use a reserved or user-defined scoped sigil acting as a token or functor. Verb frames and role mapping determine meaningful placement rather than relying solely on word order.

Negation and conjunction constraints survive fan-out. “Don't solve 2+2” must not become an affirmative calculation merely because its arithmetic fragment matches. Chargebook interpretation complements explicit negation handling; a charge lookup alone is not a complete negation parser.

The dictionary and reusable action scripts are intended to reduce the number of specialized nodes needed. That is a design goal, not proof that a fixed small population can define arbitrary concepts.

### Native implementation profile · 0.2

The initial native language profile splits independent clauses at semicolons and bounded conjunctions, preserves source spans, emits simple subject/predicate/object records, expands bounded thesaurus alternatives, captures numbers and scopes negation per clause. “Do not calculate 2 + 2; hello” suppresses the calculation while retaining the greeting. This is a deterministic lexical profile; unrestricted natural-language semantic understanding is not claimed.

## 8. Sigil scopes and action scripting

Sigils are end-user-configurable macros. A definition may act as a token, a parameterized functor, or an explicitly permitted user automaton. It may contain other sigils if the scope's call rules allow that composition.

### 8.1 Namespaces

| Scope | Responsibility | Examples |
|---|---|---|
| Relational triple | Subject/predicate/object construction | User-defined role tokens and functors |
| Pattern bind | Structural activation and captures | `&n`, `&op`, `&equals`, wildcard forms, `&calc(...)`, `&automata(...)` |
| Node/action | Reusable procedural actions | Say a captured phrase a captured number of times |
| Vote | Vote-space organization and signaling | Vote-specific macros and grouping constructs |
| Orchestration | Composition, retrieval, temporal filling, imagination | `&imagine(...)`, `&LookUp(...)`, `&time&` |
| Reserved system functionality | Built-in organ-specific facilities | Built-ins exposed only in declared scopes |

“Reserved” identifies origin and ownership; it is not one universal execution namespace. Each system retains its own registry and script surface. Shared names do not grant cross-scope execution. A built-in used in two scopes must explicitly expose both bindings.

The user can configure the action scripting layer. Transient Memory, ATP, and other organs can reuse permitted scripts through their own reserved interfaces instead of adding an unscoped scripting subsystem. Intertwined activity is allowed; ownership and contact boundaries remain explicit.

### 8.2 Reserved and named forms

The spelling below normalizes inconsistent trailing ampersands while retaining aliases for source compatibility. Final grammar and escaping rules are open.

| Canonical form | Scope and behavior | Source forms/notes |
|---|---|---|
| `&current_input` | Read original current input in pattern bind and post-bind orchestration; usable as a nested argument | Read-only current-cycle value |
| `&n` | Pattern-bind capture of any number | Number grammar and numeric domain are configurable/open |
| `&op` | Pattern-bind capture of a mathematical operation | Surface symbols and word aliases map to configured operations |
| `&equals` | Pattern-bind equality marker | Includes `=`, “equals,” and other configured notations |
| `&pie*`, `&*pie*` | Wildcard bind forms for prefix/contains-like matching | A matching wildcard can activate a node even when other ordinary pattern details mismatch |
| `&calc(row \|\| row \|\| &combineSlots('operation'))` | Multi-row procedural pattern-bind functor; rows accept nested sigils | Example row: `number, operation, number`; `&combineSlots('multiply')` combines row outcomes |
| `&automata(parameters)` | Instantiate/run a user-defined automaton for pattern binding | Accept `&automata&(parameters)` as a source alias if configured |
| `&imagine(scenario)` | Orchestration request for synthesis of retrieved resources | Source also uses `&imagine&`; arguments may contain sigils |
| `&LookUp(subsystem, id)` | Scoped retrieval for pattern-bind tools and orchestration | Source also uses `&LookUp&(...)`, `LookUp(...)`, and `Lookup(...)` |
| `&time&` | Reserved orchestration signal for history/time coherence | Resolve from available records; do not fabricate missing history |

`LookUp(nodes, PATTERN_ID)` addresses nodes. `LookUp(chargebook, PATTERN_ID)`, `LookUp(languageGovernance, PATTERN_ID)`, and analogous subsystem calls address side resources. The subsystem argument must identify a registered permitted storage scope.

### 8.3 Primitive operation set and automata

Pattern-bind scripting exposes the eight Turing-complete tape-machine primitives identified in the notes by the Brainfuck model: move pointer right, move pointer left, increment cell, decrement cell, output cell, input cell, loop start, and loop end. Conventional symbols are `> < + - . , [ ]`; the final sigil aliases, cell width, wrapping rules, input source, and output representation must be specified by the implementation.

Turing completeness describes the abstract operation set with unbounded resources. A deployed specimen must still enforce finite time, instruction, memory, and nesting budgets.

A user automaton is not the built-in orchestrator, PHAGY, or attachment-statistics automaton. `&automata(...)` creates a custom bind computation, potentially including linear algebra or configured nonlinear operations; its result participates in the node's normal vote path. Surface a user-facing warning when selecting automaton execution where a simpler token/functor would suffice, including its expected resource implications.

### 8.4 Procedural examples

`2+2`, `2 plus 2`, and `two plus two` can fan out/normalize to `&n &op &n`. Preserve captured operands, operation, and original input. A reusable node activates against that structure; its action computes **4** during the declared execution phase.

“Say hello three times” can bind phrase and repetition count into one reusable action. Numeric transformation must not destroy the original values needed at vote/orchestration time.

Binding validates and captures by default. A configured computational bind functor may execute during binding when required for the predicate. Its result and execution phase must be recorded to avoid computing twice or repeating an effect during retries.

Wildcard overrides bypass specified ordinary matching details, not negation, action inhibition, scope permissions, or resource ceilings. The exact wildcard admission route and whether it supplies or bypasses a similarity gate must be explicit per rule; see R21 and §25.

### Native implementation profile · 0.2

Native actions parse into nested Text and Call expressions. A six-scope registry checks permitted names before evaluation, with declared reads and no arbitrary persistent writes. Numeric bind sigils, arithmetic, repetition, tape automata, resource lookup, temporal lookup, recall, tone and imagination execute through bounded handlers. The new literal(JSON-string) extension preserves generated text without interpreting embedded sigils. Unknown or out-of-scope native calls fail with InvalidScope. Browser compatibility retains its separate limited interpreter.

## 9. Pattern-node activation and votes

### 9.1 Registration and candidate retrieval

For `/addPattern`, preserve the submitted pattern and associated vote, validate scope-specific bind syntax, derive the binding representation, classify complexity, and generate its Pattern key. Reject exact repeats under the chosen structural equality rule. Add it to an eligible node with room or create a same-key node under the capacity policy.

During inference, retrieve all nodes whose compatible keys equal a chunk's key. Pattern ID acts as a coarse filter. Query every eligible variant within the fan-out budget and deduplicate repeated record references caused by equivalent retrieval paths, while preserving the originating chunks and captures.

Ordinary retrieval does not compute confidence. Bind rules such as wildcard or custom automata may supply an additional explicitly indexed candidate route. They must not depend on accidentally sharing the literal text's Pattern ID.

### 9.2 High-resolution confidence

For each retrieved node, compare the chunk with every relevant original pattern entry using the modality's high-resolution scanner:

```text
S = sum(similarity points)
D = sum(dissimilarity points)
C_scan = clamp(safe_signed_difference(S, D), -100, 100)
```

Similarity and dissimilarity evidence, weighting, feature overlap, and handling of missing features must be defined by the scanner. The score is an evidence balance, not a calibrated probability.

Use a numeric representation that can safely accumulate both sums and subtract them. Validate finite inputs. Do not subtract unsigned values that can underflow, and do not independently clamp `S` and `D` before subtraction: doing so can erase their actual difference.

**Resolution: evaluation order.** Apply declared deterministic node/ATP modulation to produce a bounded unjittered baseline, then decide jitter eligibility, then apply the final gate:

```text
C_base = clamp(node_and_ATP_modulation(C_scan, context), -100, 100)
qualified_before_jitter = C_base >= VOTE_THRESHOLD

if not node.jitter_enabled:
    C_effective = C_base
else if abs(C_base) == 100:
    C_effective = C_base
else if node.strength == MAX_STRENGTH and qualified_before_jitter:
    C_effective = C_base
else:
    sign = independent_coin_flip() ? +1 : -1
    delta = sign * sample_bounded_magnitude(active_jitter_policy)
    C_effective = clamp(C_base + delta, -100, 100)

emit_vote = C_effective >= VOTE_THRESHOLD and local_gates_allow
```

The source uses both “past” and “high enough”; this specification adopts inclusive `>=`. A strict-threshold implementation must declare that difference. Maximum strength and threshold values are not supplied by the source.

The sign is selected by coin flip. The user controls magnitude; the default is a very small offset whose exact value is open. The sampled deviation is discarded at the cycle boundary. Endpoint protection is evaluated before jitter, preventing ±100 values from being displaced.

At maximum strength, an otherwise subthreshold baseline still jitters. It may cross the threshold in that cycle; that does not retroactively change its baseline or permanently crystallize the entry. Crystallization is derived from current node strength, not a one-way promotion.

Strength influences declared node gates and weighted arbitration, but it must not be silently treated as a probability or folded into the bounded score with an undocumented formula.

### 9.3 Weighted actions and inhibition

A source vote may be written:

```text
happyVote = "action A[inhibition A]^weightA,
             action B[inhibition B]^weightB,
             ..."
```

This is a pool of alternatives associated with one pattern entry. Choose an eligible alternative stochastically, with larger weight producing greater selection bias. Inhibition is executable/structural metadata and remains attached to its action through composition; it is not decorative text.

**Recommended sampler contract:** use nonnegative finite weights. A zero total needs an explicit policy, such as uniform selection among eligible alternatives or no action. Invalid/negative weights are rejected rather than producing undefined behavior. The exact distribution is open; proportional weighted sampling is a coherent default.

Each pattern entry that passes its gate can release its vote. A node containing 20 independently qualifying patterns may therefore release 20 votes. The default capacity of 20 does not mean the node elects just one entry.

### 9.4 Same-triple spin-bottle arbitration

The original rule selects among nodes using the same relational triple with a bias toward `confidence × strength`. Later notes require independent contributions and higher-order composition. Apply R06 as follows:

1. Collect qualified contributions without a global winner election.
2. Group potentially redundant alternatives by originating triple/chunk and fan-out lineage.
3. Preserve complementary actions, constraints, and intermediate relationships.
4. For an actual redundant alternative group, select with a confidence/strength bias.
5. Preserve provenance for the selected contribution and the arbitration transformation.

**Recommended safe selection weight:** `w_i = max(0, C_effective_i) × strength_i`, calculated in a wide numeric type. If every weight is zero, use the declared fallback. This preserves the requested positive bias without assigning negative probabilities. The redundancy classifier itself remains open and must not suppress useful multi-node reasoning merely because contributors saw the same triple.

Attached vote groups are exempt from this arbitration. Exact persisted structural duplicates are already prohibited, while paraphrases can still require runtime redundancy resolution.

### Native implementation profile · 0.2

All entries selected by exact encoder IDs, explicit binds, wildcards and automata are independently scored. Text confidence uses 100 times token intersection/union minus its complementary dissimilarity, with successful explicit structural binds evaluated separately. Visual confidence compares color histograms, spatial occupancy, signed-distance fields and hole mismatch. Evidence exposes similarity, dissimilarity, deterministic ATP modulation, temporary jitter and final confidence. Signed values clamp to [-100,100] and qualification uses confidence >= threshold. Same-ID collisions can fail this gate.

## 10. Node attachments and automatic wiring

Attachments represent semantic relationships that ordinary shared pattern structure does not capture. A user may select two or more node IDs and attach the corresponding nodes. Because IDs are non-unique, the configuration interface must resolve whether the user intends particular records or every matching record; the stored link uses concrete references.

### 10.1 Trigger and remainder flow

When an issuing node matches part of the current scanned pattern, retain an explicit unmatched remainder. On its attachment trigger:

- A soft attachment flips a biased coin using user-defined `ATTACH_AFFINITY`.
- A crystallized/hard attachment always schedules the target evaluation.
- The target receives the unmatched remainder rather than the original relational triple.
- The target performs its own scan and may or may not emit votes.

The exact affinity-to-probability mapping is configurable; affinity is not implicitly a confidence score. The source leaves whether an issuer must merely activate or actually vote partly ambiguous. **Recommended default:** trigger from a qualifying issuer contribution, and expose any broader activation-trigger behavior explicitly.

Attachments can be directed (“singleton” in the notes) or bidirectional. A bidirectional link enables triggering in either direction; it does not authorize infinite mutual triggering within one cycle. Multi-node attachment topology must be explicitly recorded rather than inferred from list order.

### 10.2 Grouped contributions

Related votes carry an attachment-group reference. The notes' notation is:

```xml
<attached>
  <vote node="issuer">...</vote>
  <vote node="attached-target">...</vote>
</attached>
```

Orchestration must preserve their joint relationship. Ordinary same-triple/fan-out spin-bottle logic cannot discard them as duplicate answers. Attached targets are not run through another relational-triple duplicate scan; they inspect a semantically delegated remainder.

“Together” means preserve dependency/group semantics for contributions actually produced. It does not force a below-threshold target to vote, nor require all actions to execute simultaneously.

### 10.3 Automatic wiring and bounds

A lazy-conservative background automaton collects correlation statistics during vote time and idle time, then gradually determines which nodes should be linked. Statistical evidence, sampling, promotion thresholds, and automatic hard-attachment promotion are open configuration decisions. The mechanism must expose why a link was created.

All attached scans draw from the same global deep-scan budget as ordinary scans. **Recommended controls:** track `(cycle, target, remainder)` visits, cap attachment depth, reject repeated non-progressing handoffs, and enqueue rather than recursively dispatching unbounded work. These controls preserve semantic attachments while preventing cycles and resource explosion.

### Native implementation profile · 0.2

Attachment traversal carries the actual lexical unmatched remainder, deduplicates visited jobs and limits traversal to four hops. Hard links and affinity-sampled soft links remain distinct. Repeated co-activation records bounded correlation evidence and can create soft links after three observations. The first profile limits each cycle to eight unique contributors for correlation updates and caps automatic attachments at 4,096. Semantic attachment correlation beyond this observable lexical evidence remains unqualified.

## 11. Cycle lifecycle and orchestration

### 11.1 Lifecycle

| Phase | Required activity | Retained evidence |
|---|---|---|
| Intake | Capture original input, cycle identity, available context, ATP state | Immutable input and context references |
| Preparation | Language preprocessing, bounded fan-out, triples, scoped bind captures | Chunk/variant lineage and constraints |
| Retrieval | Complexity-class Pattern-ID equality and declared bind-rule routes | Candidate record references and activation keys |
| Node reasoning | High-resolution scans, confidence, strength/gating, attachments | Baseline/effective scores and qualifying entry contributions |
| Vote collection | Collect all applicable entry votes and attachment groups | Temporary vote list with provenance |
| Phase boundary | Finish the admitted voter work before composing its complete results | Defined candidate-set completion or explicit cancellation |
| Higher-order reasoning | Step ordering, resource retrieval, context filtering, constraints, scripts | Intermediate structures and dependency graph |
| Temporary synthesis | Invoke Transient Memory only for actual transient work | Ephemeral computational results and transformation lineage |
| Presentation | Tonally constrained anti-staleness, final text/artifact/action | Output segments with first-class contributor metadata |
| Aftercare | History insertion, feedback target creation, Type A supervision | Final answer identity and residual-supervision deadline |
| Teardown | Release ephemeral orchestrator, vote lists, activations, temporary jitter | Durable updates only through their owning mechanisms |

The orchestrator is a JIT, ephemeral automaton that exists for the event. It organizes the completed temporary vote list into ordered linguistic, procedural, and action structures. It can combine narrow relationships into a chain such as concept → property → constraint → alternative action. Nodes need not store that entire chain individually.

Ordinary inference must not silently omit pending candidates to make a cycle appear complete. A cancellation or configured deadline yields an explicitly partial result. Any bounded iterative reasoning pass must have a declared lifecycle rather than recursively reopening the voter phase without limits.

### 11.2 Orchestration algorithm

```text
run_cycle(input):
    context = begin_cycle(input, ATP, available_history)
    work = prepare_variants_triples_and_bindings(context)
    temp_votes = []

    for chunk in work:
        candidates = retrieve_all_compatible_candidates(chunk)
        enqueue_deep_scans(candidates, global_scan_budget)
        include_bounded_attachment_handoffs()
        temp_votes += drain_qualified_votes_for_admitted_work()

    finish_voter_phase_or_report_partial()
    votes = arbitrate_redundant_unattached_alternatives(temp_votes)
    resources = retrieve_support(votes, context)
    relevant = filter_contextually(resources, context, votes)
    structure = compose_steps_and_constraints(votes, relevant)
    structure = resolve_permitted_sigils(structure, original_captures)

    if structure_requires_temporary_computation:
        structure = TransientMemory.synthesize(structure, relevant, ATP)

    output = apply_bounded_tonally_valid_variation(structure)
    finalize_output_and_provenance(output)
    append_history_and_start_type_A_supervision(output)
    release_ephemeral_cycle_state()
```

### 11.3 Anti-staleness

Orchestration varies eligible language in layers while preserving meaning and intended tone:

1. **Thesaurus substitution.** Identify words/phrases in the composed output, retrieve qualifying synonyms, and choose replacements stochastically. Higher intensity may suit emphasis; lower intensity may suit casual tone, subject to context.
2. **ATP/Chargebook validation.** Retrieve the proposed phrase's valence/intensity and compare it with the intended tonal tag. Reject substitutions that exceed the configured small tolerance; reroll within a finite budget or retain the original.

Retrieval uses Pattern-ID indexes for the relevant lexical spans. This is not a new global semantic scan. Substitutions must preserve negation, captured numeric values, procedural semantics, inhibition, and contributor lineage. A tonal match alone does not prove semantic equivalence.

Eligible numeric modulation values may use snapback jitter. Word substitutions and action choices are stochastic selections, not numerical jitter of text or identifiers.

### Native implementation profile · 0.2

The native trace records preparation, retrieval, raft coverage, deep scan, votes and composition. Qualifying votes are collected before orchestration. Redundancy is resolved within an originating input group using confidence and strength, while complementary actions and attachment groups survive. Each output segment retains source-vote and contributor references. Image cycles enter the same orchestration path with independently computed visual scores. Action errors become visible rejected-action output instead of partially committed state.

## 12. Pattern-ID-guided supporting retrieval

### 12.1 Insertion discipline

Every subsystem storing linguistic/semantic information retains the original representation and a generated Pattern key for each entry. Generate the key at insertion and retain it for the record's lifetime/version. This applies to Dictionary, Thesaurus, Anti-Thesaurus, Chargebook, language rules, sigil descriptions, and other registered semantic resources.

The same addressing convention is an architectural discipline, not a requirement for a new helper module. Organs should use HybridTable and the common lookup contract.

### 12.2 Two retrieval channels

The orchestrator uses both:

- **Structural retrieval:** Pattern keys generated from the submitted vote text **and** the Pattern keys/chunks that activated its node. Both sources are required. Relevant phrase-level keys may be added within the bounded query plan.
- **Node-linked retrieval:** Automatically retrieve side records whose optional resource IDs match currently voting node IDs, using the compatible unprefixed logical ID.

Resource IDs may repeat and may be absent. An absent link does not make an entry unindexed: its generated Pattern key remains available. A linked entry's original text need not independently hash to the node's activator key, because linkage and structural indexing serve different purposes under R05.

Namespace prefixes identify the subsystem, not a different linked node value. Implementations must preserve resolution/modality/version information when removing presentation prefixes. Do not equate unrelated bare strings accidentally.

### 12.3 Two stages

**Stage 1: equality retrieval.** For each registered permitted subsystem, fetch all candidate records matching a query key or node-linked resource ID. Do not calculate confidence or run a deep semantic similarity scan. Deduplicate concrete record references across query paths while retaining lookup provenance.

**Stage 2: contextual relevance filtering.** Inspect only the retrieved candidates and select the subdata applicable to the current request, conversation state, activated nodes, votes, bindings, and ATP state. This is a contextual applicability operation, not the pattern-node similarity/dissimilarity confidence algorithm.

```text
retrieve_support(votes, context):
    query_keys = unique(vote_text_keys(votes)
                        + activating_keys(votes)
                        + required_scoped_span_keys(context))
    voting_ids = unique(logical_node_ids(votes))
    candidates = []
    for subsystem in registered_permitted_side_systems:
        candidates += LookUpAll(subsystem, query_keys)
        candidates += LookUpLinkedAll(subsystem, voting_ids)
    return deduplicate_by_record_ref(candidates)
```

A common address does not itself determine meaning. If an exact key retrieves nothing, the implementation must report absence or use an explicitly configured bounded alternative-key policy. It must not conceal a global semantic search behind `LookUp`.

“Across the entire persistent specimen” for imagination or contextual access means all relevant registered stores can be addressed. It does not require loading or semantically comparing every stored record during each orchestration. Explicit idle global correlation is a separate bounded traversal workload.

### Native implementation profile · 0.2

Supporting lookup preserves explicit legacy resource-to-node handles and compares native IDs computed from original resource keys against input/action keys. Relevance filtering then checks context. The native profile builds one resource index per consistent cycle snapshot, then performs indexed lookups and relevance filtering. Persistent incremental index maintenance remains an optimization, not a requirement for complete retrieval. Missing resources produce explicit bounded fallback output rather than invented knowledge.

## 13. Transient Memory and imagination

### 13.1 Purpose and ownership

Transient Memory is an event-driven, lazy-conservative computation organ, implemented primarily with sparse matrix/MLP operations. It performs temporary assembly, transformation, procedural assistance, and synthesis using information exposed after voting/retrieval.

It receives the relevant resource pool, current request, vote/intermediate structures, and applicable ATP state. It returns a temporary result to orchestration. It neither replaces the voter population nor determines what the specimen has persistently learned.

**No learning:** using this organ does not train its weights online or mutate node knowledge, language data, sigil definitions, pattern tables, or persistent knowledge. The source's earlier language about updating MLP matrices is superseded. User-configured weights/architecture and explicit system configuration may persist; event activations do not become knowledge.

CPU is the default. The user may increase population/width and enable optional GPU/NPU acceleration, with explicit memory and work ceilings. There is no requirement to instantiate a large continuously running model.

### 13.2 Runtime activation policy

For each configured MLP layer or unit of activation selection, use a biased coin flip:

```text
familiar retrieved material -> greater probability of Sigmoid
unfamiliar combinations     -> greater probability of ReLU
urgent task                -> may select/lock the needed behavior
brainstorm mode             -> broader bounded stochastic variation
```

The source alternates between “each MLP” and “each layer”; the selection granularity must be an explicit setting. This is a runtime synthesis policy, not a learning policy. Familiarity must be derived from declared evidence rather than invented confidence.

Urgency may increase utilization or override conservative activation choices within hard limits. When no transient task exists, the organ remains minimally engaged. Eligible temporary values receive small, bounded, non-accumulating jitter.

### 13.3 Self-observer and inhibition

A self-observer presides over transient work, checks invariants and predictions, and can correct invalid temporary results. A side HybridTable holds the transient inhibitory listing. Define which entries are event-local versus explicitly durable governance rules; merely observing a bad synthesis does not authorize unbounded persistent learning.

Recommended observer outputs include violated invariant, affected structure, corrective transformation, and provenance link. This allows correction without silently replacing evidence.

### 13.4 Imagination

When a permitted `&imagine(scenario)` signal reaches orchestration:

1. Build a Pattern-ID-guided query plan from the request, activating nodes, votes, and scenario.
2. Retrieve relevant persistent ingredients from nodes, visual arrays, Chargebook, language resources, action sigils, and other permitted stores.
3. Context-filter the pool.
4. Convert heterogeneous resources into defined feature vectors/tensor parameters.
5. Use sparse matrix/MLP synthesis, activation bias, and ATP modulation to compose the requested material.
6. Return a narrative structure or visual parameters/instructions for an ephemeral fuzzy-art instance.
7. Preserve source-resource and transformation provenance and release temporary working state.

Persistent systems provide ingredients; Transient Memory performs synthesis. An imagined result is not automatically learned or promoted into the persistent store.

### 13.5 Unsettled computational design

The notes discuss full cross-attention, feed-forward concatenation, and possible hybrid sparse routing without selecting one. Do not claim a transformer attention implementation is specified. An attention module, if used, is part of transient computation and obeys the same no-learning and resource rules.

More fundamentally, the source does not specify how fixed matrix operations reliably convert retrieved concepts into coherent arbitrary language or visual instructions. The input encoding, initialization/preconfigured weights, composition algorithm, decoder, and output constraints must be supplied and validated. Sigmoid/ReLU selection alone does not define a complete synthesis engine.

### Native implementation profile · 0.2

The fixed native network is sparse feed-forward CSR with 128 inputs, 64 hidden units and 32 composition outputs. Initial layers use eight connections per output row, persisted weights and biases, and ephemeral activation vectors. Inputs encode retrieved features, context, familiarity and ATP. A seeded biased coin selects a layer activation: familiar input favors sigmoid at probability 0.75, unfamiliar input at 0.25; the alternative is ReLU. CPU execution is the reference. Optional wgpu computes matrix products, reports its backend, and falls back explicitly on failure. Shape, finite-value and composition checks reject invalid output. Brainstorming remains bounded sampling, not weight training.

### Optional local text extension

The curated text profile is Qwen3-4B GGUF Q4_K_M through a bundled CPU llama.cpp runtime. It uses non-thinking mode, 4,096-token context, at most 512 output tokens, a user-visible seed, authenticated 127.0.0.1 supervision and a 120-second unload timeout. Inputs are bounded; only one heavy generation operation runs at once. An estimated 4 GB of available memory is required at admission. This estimate includes approximately 2.5 GB weights, 0.6 GB context cache and 0.9 GB working overhead; actual qualification must measure usable execution.

Source: [Qwen3 4B publisher model card](https://huggingface.co/Qwen/Qwen3-4B-GGUF). License: Apache-2.0. Model revision: `bc640142c66e1fdd12af0bd68f40445458f3869b`. File: `Qwen3-4B-Q4_K_M.gguf` (2,497,280,256 bytes). SHA-256: `7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5`. Runtime revision: `163a40796f0ebaae246325f8d2e15028b413fa9d` from [the runtime source](https://github.com/ggml-org/llama.cpp).

## 14. Action Tone Predictor and stochastic behavior

### 14.1 ATP state and effects

The Action Tone Predictor (ATP) provides temporal tone coherence. A happy, stressed, playful, or adversarial presentation should not reset simply because a cycle ends. ATP maintains buildup, release, arousal, and cooldown state around a conservative baseline.

ATP can dampen/modulate confidence and switch tonal tags where context warrants. It uses Chargebook phrase/word valence and intensity and interacts with eye arousal. Optional user-defined rewards steer ATP toward selected goals/tasks.

The notes describe a “Lorenz-like” curve with snapback, slight per-use jitter, and buildup. They do not define a Lorenz differential equation or parameters. Preserve the intended damped temporal behavior; require a documented bounded curve before implementation.

**Recommended conceptual decomposition:**

```text
durable tonal state = bounded_update(previous_state, stimulus, elapsed_time)
temporary tonal view = clamp(durable tonal state + eligible_jitter)
```

The state evolves through intended buildup/release. The jitter disappears after use; it is not integrated back into the baseline. This distinction allows persistent mood without accidental random drift.

ATP is normally lazy-conservative. Detected urgency can temporarily alter modulation and resource priorities. Non-pattern nodes may directly edit declared ATP curve/cooldown fields within their permitted write scope.

### 14.2 Self-directed behavior

The source permits the specimen to be playful, stressed, adversarial, or to choose its own input rather than scanning the user's input. Preserve this as a configurable autonomy behavior, with an explicit internal-stimulus record and provenance indicating the divergence.

**Recommended boundary:** internal stimulus generation and tonal autonomy remain subject to action inhibition, scoped execution rules, and configured limits. Record when a user request was deferred or ignored so that feedback is attributable to the actual behavior.

### 14.3 Brainstorm mode

Brainstorming increases jitter magnitude while retaining bounds and broadens/smears biases used by coin flips and spin-bottle selection. It may encourage unfamiliar combinations and less deterministic presentation. It does not permit persistent random corruption, changing stable IDs, lifting hard resource caps, or bypassing inhibition.

The bias transform, maximum jitter, urgency override, and eligible-value inventory require configuration. A reproducible seeded mode is recommended for debugging; normal runtime may use the configured entropy source.

### Native implementation profile · 0.2

ATP valence/intensity state is separate from confidence jitter and fixed-network activations. The initial profile exponentially decays state on a 180-second time scale, applies bounded chargebook contributions and reduces intensity on release. Confidence modulation is deterministic and bounded; jitter is newly sampled for eligible comparisons and does not accumulate. Full buildup/release/cooldown semantics and meaning-preserving language re-inflection beyond the implemented tone handlers require further qualification.

## 15. Conversation memory and temporal coherence

Maintain a rolling array/ring of recent messages and answers. This document uses the explicitly labeled editorial capacity of **100,000 records** from R12. Oldest unpinned records are overwritten as capacity is reached. Users can pin selected values for persistence.

Recommended record fields are record reference, role, text/artifact reference, timestamp, cycle/output identity, and pin status. Pinning must preserve a record beyond ordinary ring overwrite. A separate pinned store is a practical resolution, with its own configured count/byte ceiling; reject additional pins clearly when exhausted rather than overwriting a pin.

Reserved coherence sigils such as `&time&` allow orchestration to retrieve relevant records and fill context for prompts such as “what now?” They do not imply a global replay or unlimited context window. Distinguish missing, overwritten, and pinned information.

Save retained history and pins with the specimen. Active contributor metadata has a shorter ephemeral lifetime unless a separate export/retention option is defined; storing answer text does not automatically preserve every live causal trace forever.

### Native implementation profile · 0.2

Conversation history retains inputs, segments, votes, trace, feedback and pins. Temporal lookup reads actual stored timestamps and pinned/history records. Rolling limits preserve pinned records separately. A native save restores committed history and starts with no active jobs; it does not replay unfinished cycles or model inference. Node and history views are virtualized and searchable. Resource and artifact views currently retain bounded display windows, which are separate from their stored record limits.

## 16. Visual perception and ephemeral fuzzy art

### 16.1 Eye lifecycle

The eye is a JIT, ephemeral, GPU-accelerated visual system with peripheral blur and adjustable scan/focus behavior. At sufficiently high arousal, the focused center is processed through the described convex/concave, center-out outline and cutout stage, then converted into the specimen's nonlinear-SDF parameter representation and a visual Pattern ID.

The precise contour extraction, arousal threshold, focus modulation, and parameterization are open algorithms. GPU acceleration is part of the intended eye design; behavior on systems without the required acceleration must be explicitly defined rather than silently claimed equivalent.

Image nodes use their own Pattern-ID conversion and deep-scan methods on all relevant steps. They otherwise participate in the same node strength, confidence bounds, jitter, vote, attachment, and feedback lifecycle as ordinary pattern nodes.

A supporting OCR-like utility converts visible text into actual text for quality of life. The text can enter the language pipeline with visual-source provenance.

### 16.2 Visual representation

Preserve the specified fields:

```text
VisualParameters
  xArray[]
  yArray[]
  colorArray[]
  brightnessArray[]
  size: { width, height }
  position                    # location of represented items relative to center
```

For every index `i`, `xArray[i]`, `yArray[i]`, `colorArray[i]`, and `brightnessArray[i]` describe the same sample/pixel. All arrays must have equal length. Width/height and total sample count have configured maxima. Coordinate origin, units, color format, brightness range, and the exact shape of `position` must be declared.

The notes call this a nonlinear SDF. The arrays as specified do not include a defined signed-distance function; implementations must document whether they encode contour samples, pixels, distance values, or a conversion into an actual field. Do not discard the original visual resource while generating its ID.

### 16.3 Fuzzy art

During orchestration, the specimen may create a GPU-accelerated ephemeral fuzzy-art instance. It accepts visual parameters and a JSON instruction map, including material synthesized by Transient Memory. The instance renders the requested imagination/output and is released afterward unless the user explicitly saves an artifact.

Rendering is distinct from learning a visual node. A produced image does not enter persistent knowledge automatically. Preserve provenance from retrieved shapes, colors, tones, action instructions, and synthesis operations into the output artifact where feasible.

### Native implementation profile · 0.2

Images enter through file input, paste or drag-and-drop. Originals are stored by SHA-256; a normalized movable focus defines the crop while the UI blurs peripheral regions. The initial encoder downsamples to 64 by 64, extracts boundary points, holes, local convex/concave indicators, a 64-bin RGB histogram and 16 spatial occupancy features. A distinct DistanceField type stores actual signed distances; aligned composition parameter arrays remain another type. OCR uses ocrs 0.12.2 and RTen 0.24 with pinned detection/recognition artifacts. Text lines and regions retain provenance and may be corrected before explicit Learn. The simple shape descriptors and observed OCR accuracy are documented as profile limits, not general vision accuracy claims.

### Optional local image extension

The curated image profile is SDXL-Turbo through bundled stable-diffusion.cpp, using the publisher fp16 checkpoint. Defaults are one 512 × 512 image, one inference step, Euler sampling, CFG 1 and a visible seed. The CPU profile requires an estimated 14 GB of available memory at admission. Child processes are supervised and cancellable. An image artifact records its model revision/digest, parameters and asset digest; it becomes learned only after inspection and Learn. This extension generates images with a fixed pretrained model and is distinct from specimen ingredient composition.

Source: [SDXL Turbo publisher model card](https://huggingface.co/stabilityai/sdxl-turbo). License: SAI Non-Commercial Community; commercial terms linked in model card. Model revision: `71153311d3dbb46851df1931d3ca6e939de83304`. File: `sd_xl_turbo_1.0_fp16.safetensors` (6,938,081,905 bytes). SHA-256: `e869ac7d6942cb327d68d5ed83a40447aadf20e0c3358d98b2cc9e270db0da26`. Runtime revision: `6b3edaaf32cc19e5bb2d819c788bd557eddc8eba` from [the runtime source](https://github.com/leejet/stable-diffusion.cpp).

## 17. Non-pattern nodes

Non-pattern nodes activate from system/contextual happenings rather than a deep pattern match. Both types can consult relevant information from all permitted side systems and stored nodes. “All” describes available scope; bounded indexed retrieval or scheduled traversal still governs access.

They use contextual correlation to accumulate pros and cons:

```text
P = sum(pro evidence)
N = sum(con evidence)
C_base = clamp(safe_signed_difference(P, N), -100, 100)
C_effective = bounded_snapback_jitter(C_base, permitted_policy)
```

Use the same numeric safety and endpoint discipline as ordinary confidence. Their configurations include context magnet, strength, type, operating rules, and permitted reads/writes. Thresholds and the meaning of evidence are node-type-specific.

### 17.1 Type A: residual voting modifiers

Type A nodes can emit votes. They remain active for approximately **1–2 minutes after an orchestration completes**, conservatively supervising the recent result with its relevant data. They perform residual checks, can identify a problem, and can cause a corrected/reissued reply through orchestration.

Their immediate operating lifetime is short, though a contribution can modulate short- or longer-lived state within policy. They cool down slowly and can be activated by contextual need. They can inspect recent pattern votes and edit explicitly permitted ATP/cooldown state.

Recommended implementation: bind supervision to an output/cycle lineage and an expiration deadline. Corrections carry new provenance and reference the prior answer. A correction should not reset its own supervision window indefinitely; bound recursive correction count and concurrent supervised outputs.

### 17.2 Type B: idle internal modifiers and proposed learning

Type B nodes do not emit ordinary votes. At random idle opportunities, they perform lazy-conservative global correlation. Use regular small-collection iteration when appropriate and Index Rafts for sufficiently large traversal workloads.

They can read gathered contextual data, calculate pros/cons, and edit authorized internal operating state. If evidence suggests new knowledge should be learned, they create an identified proposal and present its data/structure to the user. `/learn <proposal_id>` approves storage of that structure.

A learning proposal is a control notification, not a disguised ordinary vote. Do not treat Type B's internal read/write capability as permission to bypass approval for newly inferred knowledge. R17 preserves both capabilities.

### 17.3 Extensible node types

The architecture permits additional types whose confidence balances contextual pros/cons rather than pattern evidence. Each type must declare activation source, evidence aggregation, threshold, vote capability, write capability, lifecycle, resource use, and persistence. Preserve these declarations in `/saveSpecimen` and `/loadSpecimen`.

### Native implementation profile · 0.2

Type A reviews recent unmatched cycles with an explicit matching context, within a default 90-second residual window. A correction links correctionOf to its originating cycle and the original is marked reviewed, allowing at most one correction. Type B inspects bounded recent unmatched history and creates a learning proposal with pros, cons, evidence count and observer identity. Neither path silently adds inferred patterns. The present Type B profile is repetition-based contextual evidence; unrestricted internal-state inference remains outside its demonstrated behavior.

## 18. Contributor provenance and feedback

### 18.1 First-class ephemeral provenance

Create provenance at vote emission and propagate it through grouping, ordering, sigil execution, retrieval, Transient Memory synthesis, and anti-staleness. Do not try to reconstruct contributors from the final prose alone.

`OutputSegment` contains text/artifact span, contributors, transformations, source votes, and a provenance ID. A segment may have multiple contributing nodes and entries. Supporting resources should be distinguished from voting contributors so that use of a dictionary definition does not accidentally award node strength.

`/contributorfractal` returns the current answer color-coded by contribution segment. For example, green and red identify different contributor sets. Colors are local labels, not permanent node IDs, and may be reused in another answer. Recommended output also includes a textual segment label for accessibility and unambiguous targeting.

### 18.2 Whole-answer and targeted feedback

- `/right`: select all voting nodes that actually contributed to the current answer.
- `/wrong`: select the same contributor population for negative feedback.
- `/right green`: select only contributors to the current answer's green segment.
- `/wrong red`: select only contributors to its red segment.

Resolve color labels against the answer's live provenance map. A color can map to several nodes. Deduplicate by concrete node reference before applying feedback so that one node contributing multiple votes/segments does not receive accidental repeated coin flips in one feedback event.

For each selected node, flip an independent coin. On the successful `/right` branch, increment strength by one. On the successful `/wrong` branch, decrement strength; this specification uses **one point** as the symmetric editorial interpretation. Preserve the stochastic rule for targeted feedback as well as whole-answer feedback.

```text
apply_feedback(answer, selector, direction):
    contributors = unique_contributing_node_refs(answer, selector)
    for node in contributors:
        if independent_coin_flip_succeeds():
            node.strength = clamp(node.strength + direction, 0, MAX_STRENGTH)
        if node.strength == 0:
            remove_active_node_with_exact_record_scope(node)
        else:
            node.crystallized = (node.strength == MAX_STRENGTH)
```

Use `direction = +1` for right and `-1` for wrong. A fair coin is a reasonable default, but exact feedback probabilities are not stated. Nodes that did not contribute to the selected output are unaffected. The provenance graph, rather than all merely activated candidates, defines contribution.

Repeated feedback on the same answer is not specified. Recommended behavior is to identify feedback events, show prior application, and require an explicit policy for replay rather than letting retries accidentally change strength again.

At zero strength, remove that node using HybridTable's scoped removal. At maximum strength, qualified confidence stops jittering. If subsequent negative feedback lowers strength, the node loses crystallization. Attachment crystallization is a separate property and must not be toggled accidentally by this rule.

### Native implementation profile · 0.2

Votes expose node and entry references, actions, scores, source input/group, bindings and supporting-resource references. Segments expose contributing nodes, originating votes and transformations. Feedback remains an independent strength update for each eligible specimen contributor and never trains pretrained models. Visual learning preserves original asset and OCR correction records. Generated artifacts retain model provenance until explicitly learned. Explicit learning links the artifact to its new node and retains model or corrected-OCR provenance. Direct model-panel requests have no specimen contributors; that empty lineage must not be filled with invented contributors.

## 19. Growth, idle mutation, and PHAGY

### 19.1 Growth policy

Growth-capable organs may expand through their defined mechanisms. The specimen can ask questions when it lacks knowledge and use structural linguistic evidence to identify where, when, and how to grow. This remains lazy-conservative rather than indiscriminate population growth.

Keep the persistent-change paths distinguishable:

| Change | Governing mechanism |
|---|---|
| User-supplied pattern/knowledge | Explicit insertion command or configuration operation |
| Contributing-node strength | `/right` and `/wrong` with independent coin flips |
| Semantic attachment | User configuration or the bounded correlation/wiring automaton |
| Vote content mutation | Idle mutation-tag rules below |
| Newly inferred Type B knowledge | Identified proposal accepted with `/learn <id>` |
| Internal ATP/operating state | Authorized organ-specific state update |
| Temporary composition | Transient Memory; does not itself produce persistent learning |

Before creating a learned pattern/vote structure, consult inhibited-growth rules and relevant bounded fan-outs. A prohibited original pairing must not be reintroduced by routine growth.

### 19.2 Idle scheduler

While eligible for idle maintenance, draw a fresh interval in the configured range. This specification adopts `[60,120]` seconds from the repeated 1–2 minute description. At the event, flip a coin:

```text
heads -> bounded vote mutation-tag event
tails -> PHAGY cleanup event
```

Draw the next interval again after the event; it is not one random interval reused forever. The three-second example is preserved as a source inconsistency in R11, not silently folded into the default. Whether idle elapsed time pauses or restarts during foreground work is open; pause/resume is a recommended conservative behavior.

Maintenance yields to active input and respects all global resource budgets. Type B correlation and attachment statistics can share the idle scheduler without being confused with this specific tag/PHAGY coin flip.

### 19.3 Vote mutation tag

At a tag event, choose a sparse random group of nodes. Weak, low-strength recipients may borrow a few vote alternatives from stronger donors. Equal logical IDs identify potentially related populations, but ID equality is insufficient permission to mutate.

Required checks and transformation:

1. Confirm the donor is stronger under the configured weak/strong policy.
2. Require high similarity between the relevant node patterns using a confidence/deep scan.
3. Compare the candidate votes in the sampled group to determine aligned donor/recipient slots.
4. Select only a bounded number of eligible recipient slots that have never been remixed.
5. Check donor/recipient lineage for prohibited repeated relationships or pathological cycles.
6. Markov-remix the recipient's original vote text with the selected donor's text. Do not simply copy the donor wholesale.
7. Independently flip a coin for each slot swap to keep the recipient's original weight or inherit the donor's weight.
8. Record lineage, mark the recipient slot remixed, and install the inhibited original pattern/vote pairing.
9. Commit a validated mutation atomically within the relevant storage boundary.

“Steal” is borrowing for remix: the notes do not require removing the vote from the strong donor. This specification leaves the donor intact. All-to-all comparisons are confined to the sampled group; their work is not free and must be bounded.

### 19.4 Pool length and slot lifetime

For the aligned vote-alternative pools:

```text
aligned_count = min(recipient_count, donor_count)
```

- A weak pool of five alternatives paired with a strong pool of four is shortened to four before completing the aligned mutation transaction.
- A weak pool of four paired with a strong pool of five remains four. Stop at the fourth aligned alternative.
- No donor operation increases the recipient's alternative count through this mechanism.
- Actual text mutation per event remains bounded to the configured “few at a time.” Alignment/pool sizing does not require rewriting every surviving alternative at once.

The source does not specify whether alignment is positional or uses a similarity-based assignment. Preserve the vote-comparison requirement and store the selected slot mapping explicitly.

Each recipient slot can undergo Markov remix only once. A node can participate in later events only through still-unremixed slots. When all slots are marked, the node cannot mutate further through this mechanism. Stable slot references and persistent markers prevent renumbering from resetting eligibility.

Under R19, reject a proposed pool shortening that would erase an already-remixed tail slot unless an explicit policy preserves its permanent history/tombstone. This is a conservative resolution of the length and lifetime rules, not a source-specified grave policy.

### 19.5 Lineage and growth inhibition

Record a relationship such as:

```text
A.entry2.slot3 <- B.entry1.slot7
```

The one-remix rule alone does not prevent a donor relationship from being reused through other fresh slots. Maintain a relationship exclusion/cooldown policy to prevent self-reinforcing patterns such as A borrowing from B, B converging toward A, then repeated borrowing elsewhere in the same cluster.

The exact temporary-versus-permanent relationship ban and cycle-detection horizon are open. Recommended minimum: reject self-donation, direct reciprocal cycles within the active history window, and reuse of an explicitly prohibited donor/recipient relationship.

When an entry's vote is remixed, preserve its original pattern/vote pairing as an inhibited growth rule, with the changed slot identified. Future growth checks the original pairing and relevant fan-out variants. Do not delete or rewrite the original activator merely to implement vote mutation. Inhibition and lineage must survive saves, node removal, and ordinary cleanup unless an explicit user policy changes them.

The Markov model's order, tokenization, constraints, selection distribution, and rejection criteria are not provided. **Recommended validation:** resulting scripts must still parse, preserve declared inhibition and scope, have valid weights, and fit size limits. An invalid remix does not consume the permanent slot marker unless a documented attempted-mutation policy explicitly says otherwise.

### 19.6 PHAGY

PHAGY dispatches a sparse population of JIT ephemeral automata. Each receives a unique cleanup job and a timeout. Agents yield, report outcomes, and disappear at completion or timeout.

The source does not define what cleanup jobs may remove or compact. Before implementation, define a job catalog with each job's permitted read/write/drop scope, preconditions, time budget, result, and recovery behavior. Recommended job assignment uses a claim/lease so two agents do not perform the same active job.

PHAGY is not the user-facing `&automata(...)` bind sigil. Its agents are built-in maintenance workers governed by the system's own scope and resource policy.

### 19.7 Grave system

The notes explicitly leave the grave system's adaptation to mutation unfinished. This document does not invent resurrection or automatic forgetting behavior.

Required now: remove zero-strength nodes from the active population without removing same-ID peers, and preserve separately required inhibition/lineage information. Open decisions include grave record contents, retention, resurrection, whether a resurrected slot retains its remix marker, and how PHAGY treats graves. A resurrection design must not reset lifetime mutation restrictions inadvertently.

### Native implementation profile · 0.2

Native idle mutation selects stronger donors, compares original patterns and literal vote text independently, keeps stable alternative slots, preserves inhibition and permanent remix markers, and rejects reciprocal borrowing through lineage records. Eligible literal text uses second-order Markov transitions; structured programs are excluded. Changes commit transactionally only after validation. PHAGY uses an exclusive cancellable lease, cleans retained history/activity and orphaned attachment references, rebuilds derived IDs on commit, and removes expired scratch or unreferenced asset files after a configurable default 24-hour retention. Asset reclamation conservatively preserves recovery copies and all currently referenced assets. Each deletion is idempotent and cancellation leaves committed specimen references intact.

## 20. Index Rafts, chunking, and scheduling

### 20.1 Intended use

Index Rafts are a latency-management mechanism for **large traversal workloads**. HybridTable automatically selects them only beyond a configured/tuned threshold and where the sandbox finds them suitable. Small collections use arrays/simple iteration; direct indexed equality lookup should use the index.

An equality hash lookup is not normally a reason to walk every table index. Rafts may help enumerate a large candidate bucket, perform a required global idle correlation traversal, process an unindexed predicate, or scan a large physical collection. Keep that distinction visible in metrics and API naming.

The source's goal is reduced latency at acceptable compute cost. Partitioning work can lower elapsed time with sufficient execution resources, but does not erase total comparisons, memory traffic, scheduling overhead, or synchronization. Same-thread interleaving alone does not establish parallel speedup.

### 20.2 Disjoint ranges

Assign a stable traversal view with `N` entries. Choose ordered starting positions:

```text
0 = start[0] < start[1] < ... < start[R-1] < N
end[r] = start[r+1]  # except the last end is N
```

Each raft owns the half-open interval `[start[r], end[r])`. For starts at 0, 20, and 50:

```text
raft 0: indexes  0 through 19
raft 1: indexes 20 through 49
raft 2: indexes 50 through N-1
```

Raft 0 ensures the prefix is covered. A raft stops at its assigned boundary regardless of how fast another raft runs. Never stop merely because a later raft's current cursor is ahead; that can leave gaps. No empty/overlapping range should be dispatched.

Hash-table rehashing can move physical slots. **Recommended requirement:** traverse a stable snapshot/version or use a stable logical entry view. A mutation during traversal must not invalidate cursor meaning or silently skip records.

### 20.3 Continuations and yielding

The source describes `indexRaft(lookup, index + 1)` instead of a conventional loop. Implement that as a scheduled continuation/state machine, not unbounded native call-stack recursion:

```text
RaftState { query, next_index, end_index, mode, snapshot_ref }

step(raft):
    if cancelled_or_timed_out(raft) or raft.next_index >= raft.end_index:
        return finish(raft)

    entry = snapshot[raft.next_index]
    if matches_query(entry, raft.query):
        record_match(entry)
        if raft.mode == find_first:
            request_search_completion_under_declared_policy()

    raft.next_index += 1
    yield_to_scheduler(YIELD_BEHAVIOR)

    if more_work_is_required:
        schedule_continuation(raft)
    else:
        finish(raft)
```

Yield at the end of every raft step. The scheduler may tune cooperative yielding/backoff behavior. It must avoid a hot recursively rescheduled loop that monopolizes execution. No per-step OS thread creation is required.

`find_all` completes all assigned ranges and chunks. It is required for duplicate node/resource ID results. `find_first` may stop early, but the API must declare whether “first” means first found or lowest index; deterministic lowest-index results require appropriate prefix completion.

### 20.4 Threads, rafts, and physical chunks

Use distinct limits:

- `MAX_RAFTS`: maximum active traversal tasks; accept the source typo `MAX_RADTS` only as a compatibility alias.
- `MAX_THREADS`: maximum participating execution threads.
- `YIELD_BEHAVIOR`: how continuations relinquish execution.
- `MAX_TABLE_SIZE`: maximum entries in a physical traversal chunk, distinct from per-node pattern capacity.

Choose raft count, placements, and thread allocation jointly for table size, observed work, available resources, and foreground pressure. Many rafts can share a bounded worker pool.

If a table exceeds `MAX_TABLE_SIZE`, divide its traversal into chunks and traverse **one chunk at a time**. Rafts may operate concurrently inside the current chunk. Complete it before moving to the next. Preserve a cursor and accumulated results for `find_all`.

This is a working-set and scheduling policy, not permission to discard records beyond a size ceiling. The underlying logical collection can span multiple chunks.

### 20.5 Efficiency sandbox

Provide an optional test sandbox that evaluates array/loop, direct index, and raft-based approaches on representative workloads. It may choose transition thresholds, raft count and positions, worker allocation, yield policy, and chunk size within user ceilings.

Recommended measurements: latency distribution, total CPU time, memory footprint, throughput, foreground responsiveness, cancellation delay, and result equivalence. Test realistic hit rates, duplicate-ID populations, and sparse/dense traversals. Store the hardware/workload profile with tuning results and invalidate stale tuning after meaningful changes.

The sandbox can recommend settings for stronger hardware. It cannot increase hard limits without configuration authorization or claim a universal optimum from one sample.

### 20.6 Global deep-scan limit

At most **1,000 high-resolution pattern comparisons are active at once by default**. The user can edit this ceiling and optionally enable adaptive threading. This budget covers ordinary candidates, attached remainders, and maintenance deep comparisons when they coexist.

The notes sometimes describe this as “1,000 nodes at a time.” Because each node can hold multiple patterns, this specification counts the actual concurrent deep comparisons, rather than allowing 1,000 nodes × 20 simultaneous scans. A node may process entries sequentially or acquire permits per comparison.

Queue all candidates returned by the current lookup and process them in waves until the full admitted set is covered. The cap is concurrency, not a top-1,000 retrieval cutoff. Thread scaling can use a configured scans-per-worker ratio, but the scan cap and thread cap remain independent.

### Native implementation profile · 0.2

Native Index Rafts partition candidate work into disjoint chunks, join results in stable order and continue over all chunks. The 1,000 comparison ceiling governs active work, never total records searched. Scheduler jobs and worker counts bound actual concurrency. Cancellation is checked between partitions and comparisons. UI foreground work cancels its owned idle operation before proceeding; a full engine-level fairness queue and durable resume cursor remain qualification items. Browser rafts remain cooperative tasks on one JavaScript execution lane.

## 21. Compartmental contact ledger

The “ethereal compartmental code” idea becomes a user-defined, enforced contact ledger. Each function/structure/module should interact only with explicitly listed peers and resources that its operation requires.

Recommended ledger entry:

```text
component
  allowed_reads[]
  allowed_writes[]
  allowed_calls[]
  allowed_sigil_scopes[]
  owned_state[]
  lifetime
  scheduling_class
```

Use the ledger to make ownership, scope, and permitted interaction inspectable. Subsystems can collaborate through declared interfaces while remaining isolated from unrelated state. Sigil namespaces and storage subsystem boundaries reinforce these rules.

The notes propose this as an anti-collision/starvation theory; isolation alone does not prove freedom from races, deadlocks, or starvation. An implementation still needs compatible ownership/locking or message-passing rules, fair scheduling, cancellation, and bounded queues. Static enforcement, runtime enforcement, or both are open choices.

A script with a matching sigil name must not acquire access absent from its ledger. Built-in automata and user automata receive distinct allowed contact surfaces.

### Native implementation profile · 0.2

The registry exposes name, allowed scopes, declared reads and writes. Reserved scope grants no calls. Privileged model processes receive only bounded prompts, fixed parameters and owned artifact paths through the native provider manager. The local text server binds authenticated loopback; image jobs run as supervised child processes. Model generation is currently a dedicated typed UI operation rather than a general sigil callable from every scope. Production builds omit the WebdriverIO test bridge and its permissions.

## 22. Commands

The source defines command names but does not fully define argument grammars or every error response. This table preserves the named interface while labeling additions.

| Command | Required behavior | Notes |
|---|---|---|
| `/addPattern <pattern-and-vote-definition>` | Register a pattern/vote entry, classify/index it, enforce duplicates and limits | Exact argument syntax is open; paired vote data is required even though the original command shorthand omits it |
| `/prompt <input>` | Run the ordinary input pipeline | Preserve original input and captures |
| `/right` | Positive stochastic strength feedback for current-answer contributors | Independent per-node coin flip |
| `/wrong` | Negative stochastic strength feedback for current-answer contributors | Source typo `/wrng` may be supported as an alias |
| `/right <color-or-segment>` | Target positive feedback to selected contributors | Color form is sourced; explicit segment form is a recommended unambiguous extension |
| `/wrong <color-or-segment>` | Target negative feedback to selected contributors | Same stochastic rule |
| `/contributorfractal` | Return current answer with color-coded contributor segments | Uses recorded ephemeral provenance |
| `/learn <proposal_id>` | Accept and persist the identified proposed knowledge structure | Validate that the proposal still exists and is applicable |
| `/saveSpecimen <file>` | Serialize the complete persistent specimen to JSON | Covers all organs, not only nodes |
| `/loadSpecimen <file>` | Validate and restore a complete specimen from JSON | See §24 for coherent restore behavior |

Required configurable capabilities with **unspecified command spelling** include attaching nodes, direction and affinity, hard attachments, pin/unpin, dictionary/governance editing, scoped sigil registration, action scripts, brainstorming, rewards, resource tuning, custom node registration, and running the efficiency sandbox. Do not present invented slash commands for these as source-defined interfaces.

Recommended command results distinguish no match, malformed definition, duplicate original structure, capacity exhaustion, unsupported scope/version, missing provenance, stale proposal, invalid configuration, and save/load failure. A same-ID result containing multiple records is normal, not an ambiguity error unless a command requires one concrete target.

### Native implementation profile · 0.2

The visible studio continues to support /prompt, /right, /wrong, /wrng, /contributorfractal, /saveSpecimen, /loadSpecimen, /addPattern and /learn. Existing documented action aliases remain in the specification. Native image analysis, image-cycle execution, recognized-text execution, model install/import/remove/unload, generation and cancellation are exposed through the desktop workspace. Text and image artifacts require explicit Learn. Commands that open editors or file pickers retain those reviewable UI steps.

## 23. Configuration and limits

The notes provide only a few numeric defaults. Unspecified values below are **open**, not implied production defaults. Configuration names are normalized recommendations; preserve aliases where compatibility is needed.

### 23.1 Explicit or editorial defaults

| Setting | Value in this specification | Authority |
|---|---|---|
| `PATTERN_ENTRIES_PER_NODE` | 20, user adjustable | Explicit source default |
| `PATTERN_ID_CLASSES` | low, medium, high | Explicit source classes |
| `DEEP_SCAN_RESOLUTION` | high | Explicit source rule |
| `CONFIDENCE_MIN`, `CONFIDENCE_MAX` | -100, +100 | Explicit source bounds |
| `MAX_ACTIVE_PATTERN_SCANS` | 1,000, user adjustable | Explicit default hard concurrency ceiling |
| `THESAURUS_INTENSITY_RANGE` | `[0,1]` | Explicit source range |
| `CHARGEBOOK_VALENCE_RANGE` | `[-1,1]` | Explicit source range |
| `CHARGEBOOK_INTENSITY_RANGE` | `[0,1]` | Explicit source range |
| `IDLE_EVENT_INTERVAL_SECONDS` | `[60,120]`, fresh draw | Resolution of repeated 1–2 minute rule versus 3-second example |
| `TYPE_A_ACTIVE_WINDOW_SECONDS` | Within 60–120 after orchestration | Source range; fixed versus sampled duration is open |
| `HISTORY_CAPACITY_RECORDS` | 100,000 | Editorial reading of `100,00`; see R12 |
| `POSITIVE_FEEDBACK_STEP` | +1 on successful independent coin flip | Explicit source increment |
| `NEGATIVE_FEEDBACK_STEP` | -1 on successful independent coin flip | Symmetric editorial resolution |
| `MAX_REMIXES_PER_RECIPIENT_SLOT` | 1 over the slot's lifetime | Explicit source invariant |
| `TRANSIENT_DEVICE` | CPU, optional GPU/NPU | Explicit source default and options |
| `TRANSIENT_ONLINE_LEARNING` | Disabled | Explicit later clarification |

### 23.2 Required configurable behavior without a supplied value

| Area | Settings or choices | Constraints |
|---|---|---|
| Pattern IDs | complexity thresholds; feature extraction per class/modality; encoder version | Class before ID; stable insertion key; reproducible equality |
| Pattern scans | feature weights; similarity/dissimilarity evidence; `VOTE_THRESHOLD`; numeric representation | High-resolution verification; safe bounded confidence |
| Nodes | initial strength; `MAX_STRENGTH`; node/entry population and byte limits | Zero removes active node; maximum crystallizes; no silent overwrite |
| Jitter | default small magnitude; distribution; eligible-value list; per-node toggle | Sign coin flip; snapback; endpoint/crystallization exceptions |
| Stochastic selection | feedback coin probability; weighted-action distribution; zero-weight policy; spin-bottle fallback | Independent events; finite nonnegative sampling weights |
| Fan-out | maximum variants; sampling budget; duplicate equivalence | Applies to all inputs; preserves negation and lineage |
| Attachments | `ATTACH_AFFINITY`; direction; hard/soft mode; trigger timing; depth/visit budget | Remainder handoff; grouped votes; shared scan ceiling |
| Auto-wiring | sampling interval; statistic windows; evidence thresholds; promotion/removal rules | Bounded, inspectable correlation |
| ATP | baseline; buildup/release; curve parameters; cooldown; modulation bounds; urgency threshold; reward mapping | Temporal coherence; bounded state; temporary jitter |
| Anti-staleness | synonym selection bias; tonal tolerance; retry limit; semantic acceptance checks | Fall back to original; preserve action meaning and provenance |
| Transient Memory | population/width; sparse representation; weights initialization; activation-selection granularity; familiarity bias; attention choice | No online learning; bounded event-driven synthesis |
| Transient resources | CPU/GPU/NPU selection; memory limit; operation/time budget; observer policy | Urgency remains inside ceilings |
| Brainstorming | stronger jitter bounds; bias-smearing function; affected components | Stochastic variation without lifting invariants |
| History | record granularity; context retrieval budget; pin capacity/bytes; overwrite behavior | Pins survive ordinary overwrite; capacity is explicit |
| Provenance | current-answer retention window; segment/color mapping; feedback replay policy | Concrete contributor refs; no reconstruction from text |
| Vision | focus/periphery settings; arousal trigger; contour algorithm; `MAX_VISUAL_WIDTH/HEIGHT`; sample count; numeric ranges | Aligned arrays; bounded images; declared distance-field encoding |
| Fuzzy art | instruction grammar; rendering budget; GPU policy; artifact save behavior | Ephemeral by default; synthesis is not learning |
| Type A | window selection; cooldown; correction threshold; correction count; supervision concurrency | Residual phase expires; corrections remain attributable |
| Type B | idle schedule; correlation budget; pros/cons rules; write scope; proposal lifetime | New inferred knowledge requires `/learn` |
| Mutation tag | weak/strong threshold; group size; pattern/vote similarity thresholds; alternatives per event | Sparse selection; stronger donor; one remix per slot |
| Markov remix | model order; tokenization; string constraints; weight inheritance coin; alignment rule | No recipient growth; invalid output rejected |
| Mutation lineage | relationship cooldown/ban; cycle-detection window; inhibition matching/fan-out | No marker reset; original pairing remains inhibited |
| PHAGY | agent count; unique job catalog; job timeout; lease/yield policy | Ephemeral sparse agents; explicit cleanup ownership |
| Graves | contents; retention; resurrection; PHAGY treatment | Open design; must preserve required mutation history |
| HybridTable | array/hash transition; `MAX_TABLE_SIZE`; storage byte limits | Logical records survive physical transitions/chunking |
| Index Rafts | activation threshold; `MAX_RAFTS`; placement; `MAX_THREADS`; `YIELD_BEHAVIOR` | Disjoint coverage; chunk-at-a-time traversal; bounded workers |
| Scan threading | adaptive-threading toggle; scans-per-worker ratio; global thread budget | Does not change the active-comparison ceiling |
| Sandbox | workloads; metrics; tuning schedule; permitted ranges; profile invalidation | Cannot exceed user ceilings; preserve result equivalence |
| Sigils/scripts | scope grammar; reserved aliases; nesting; instruction/time/memory budgets; eight-primitive cell semantics | Explicit permitted calls; bounded automata |
| Contact ledger | component reads/writes/calls; enforcement mode; scheduling classes | Enforced concern separation |
| Persistence | schema/encoder versions; maximum file size; migration policy; restore behavior | Whole-specimen validation before replacement |

### 23.3 Budget interaction

Urgency, brainstorm mode, hardware tuning, auto-wiring, residual supervision, and PHAGY share finite resources. Recommended priority is foreground cycle work, bounded residual correction, then idle maintenance. A fair scheduler must still reserve opportunities or age queued work so low-priority maintenance does not starve forever.

Do not create independent 1,000-scan pools per subsystem. Use one specimen-wide permit budget. Avoid allocating `MAX_THREADS` separately to every raft or scan module; if separate pools are configured, define an aggregate hardware limit.

### Native implementation profile · 0.2

Native defaults retain 20 entries/table, confidence threshold 62, jitter amplitude 1.5, initial/max strength 5/10, fan-out 12 and history 100,000. Idle delays are freshly sampled from 60–120 seconds. Autonomous proposals, mutation and maintenance remain user-controlled. Native bounds include 8,000 prompt/action bytes, 2,000 pattern bytes, 12,000 resource-value bytes, 16 sigil nesting levels, 32 KiB action output, 1,024 tape cells and 10,000 tape instructions, eight active jobs, 64 workers/job and 4,096 records/chunk. Images are limited to 32 MiB and 8,192 pixels per axis; native manifests to 64 MiB and portable archives to 512 MiB expanded. CPU inference memory estimates are admission requirements, not measured universal minima.

## 24. Persistence and restoration

### 24.1 Save scope

`/saveSpecimen <file>` writes JSON containing all persistent specimen content:

- Every text, image, custom pattern, and non-pattern node, including context IDs, original representations, votes, strengths, and configuration.
- All language/governance resources, original content, Pattern keys, optional resource links, and subsystem registrations.
- Every sigil namespace, reserved/user definitions, permitted action scripts, and automaton definitions.
- Attachments, direction, affinity, hard-attachment state, and persistent correlation statistics.
- ATP's coherent baseline/buildup/release/cooldown state and user reward configuration.
- Retained conversation records and pins.
- Transient Memory's configured architecture/parameters and explicitly persistent inhibition/governance definitions.
- Growth-inhibition rules, one-remix markers, donor relationships, mutation records, and whatever grave policy has been explicitly implemented.
- Pending learning proposals and their approval status, if retained under the selected policy.
- Resource ceilings, timing settings, efficiency profiles, contact ledger, and schema/encoder compatibility metadata.

Runtime objects such as current vote buffers, per-use jitter, live automata, thread handles, GPU allocations, sparse event activations, and live output provenance are ephemeral. They need not be serialized as running computations. Persisting an answer's text is separate from checkpointing its execution graph.

ATP's intended temporal state can persist without persisting the last jitter sample. Serialize numeric visual arrays and any persistent matrix configuration as JSON-compatible values; specify precision/encoding and validate sizes.

### 24.2 Coherent snapshot and load

Recommended save procedure:

1. Take a consistent snapshot/version boundary across organs.
2. Validate referential and configuration invariants.
3. Serialize the complete envelope to a temporary destination beside the intended file.
4. Verify serialization and complete the destination replacement atomically where supported.
5. Report the actual saved location and failure status if any step fails.

Recommended load procedure:

1. Read into a staging specimen with configured file-size/resource limits.
2. Validate JSON, schema version, node types, sigil scopes, numeric ranges, array lengths, capacities, and record references.
3. Validate compatible encoder versions and regenerate/check indexes without silently changing logical IDs.
4. Preserve duplicate public IDs while rejecting forbidden exact structural duplicates.
5. Validate live resource links and attachment endpoints, mutation markers, inhibited pairings, and learning status. Distinguish legitimate historical references to removed nodes from invalid live references.
6. Derive node crystallization from strength and normalize durable state.
7. Replace the active specimen coherently only after validation succeeds.
8. Recreate event-driven services from definitions; do not resume partially executed actions implicitly.

On load failure, keep the current specimen intact. Missing/unsupported custom types or sigil definitions must be reported. Loading is replacement by default under this recommendation; merging two specimens is a separate explicit operation with its own duplicate and provenance policy.

Clock-based deadlines require a restore policy. Recommended behavior is to restore durable ATP state with recorded timestamps while expiring in-flight Type A supervision and ephemeral tasks, unless explicit checkpoint support exists.

### 24.3 Conceptual JSON envelope

The empty collections below illustrate top-level organization only; this is not a complete default configuration or a runnable specimen fixture.

```json
{
  "schema_version": "wdbx.specimen.v1",
  "configuration": {
    "pattern_entries_per_node": 20,
    "max_active_pattern_scans": 1000,
    "history_capacity_records": 100000,
    "transient_online_learning": false
  },
  "pattern_encoders": [],
  "nodes": [],
  "side_systems": {},
  "sigil_registries": {},
  "attachments": [],
  "conversation_history": [],
  "pinned_history": [],
  "atp_state": {},
  "transient_memory_definition": {},
  "inhibitory_rules": [],
  "mutation_history": [],
  "correlation_statistics": [],
  "learning_proposals": [],
  "maintenance_configuration": {},
  "contact_ledger": {}
}
```

### Native implementation profile · 0.2

Native import stages validation before activation. Browser v1 originals, identities, strengths, inhibition, history, pins and mutation records are preserved. Derived IDs rebuild from original patterns with an explicit reference map. The source import and prior snapshot are retained as recovery copies. Portable .wdbxspecimen ZIP archives contain a versioned manifest and content-addressed required visual assets; model files are referenced by identity/digest. Archive traversal, oversized entries, unsupported schema and digest mismatch are rejected. Durable edits use expected revisions. Deleted-node tombstones preserve lifetime mutation constraints. Crash recovery uses committed WDBX state and never resumes old jobs automatically.

## 25. Implementation decisions and acceptance criteria

### 25.1 Decisions required before implementation claims

| Decision | Why it matters |
|---|---|
| Complexity classification and class-specific feature transforms | Defines which related inputs share a key and what retrieval can miss |
| Structural duplicate equality and its population scope | Must allow paraphrases/duplicate IDs while reliably rejecting exact repeats |
| Non-unique logical IDs versus concrete internal references | Needed for attachments, feedback, mutation, and scoped deletion |
| Optional resource linkage versus generated Pattern key | R05 is an explicit two-field reconciliation; the wire schema must preserve it |
| Bind normalization and wildcard/automaton candidate routing | General macros cannot rely on literal text hashing by coincidence |
| Deep scanner evidence and confidence calibration | Required to make ±100 scores reproducible; confidence is not probability |
| Threshold, jitter, ATP order, and eligibility | R09/R10 and §9 give a coherent ordering that must be implemented consistently |
| Runtime redundancy classification | Prevents same-triple arbitration from suppressing complementary votes |
| Fixed non-learning synthesis design | Encoding, operations/weights, attention choice, decoder, and quality criteria are unspecified |
| ATP curve and urgency detection | “Lorenz-like” and “lazy-conservative” are not executable equations |
| Visual representation/contour extraction | The named nonlinear SDF format needs a mathematical definition |
| Type A correction and Type B write boundaries | Prevents self-triggering corrections and unapproved learned knowledge |
| Markov slot alignment, pool shortening, and history policy | Must preserve one-remix lifetime restrictions across edits/removal |
| PHAGY job catalog and grave lifecycle | Cleanup and resurrection behavior are explicitly unfinished in the source |
| History count/granularity and pin limits | `100,00` is ambiguous and pins need an explicit bounded policy |
| Idle timing and scheduler fairness | Default timing is reconciled; load and starvation behavior still need implementation |
| Contact-ledger enforcement | Theoretical isolation must become actual checked access boundaries |
| Save format and migration | Complete system restoration needs versioned definitions and index validation |

These are concentrated design decisions, not permission to omit the corresponding mechanisms. A conforming implementation should record its selected behavior alongside this specification.

### 25.2 Suggested implementation decomposition

Build the system around storage/identity contracts first, then deterministic input and scoring behavior, then bounded stochastic policies and composition. Keep independently testable interfaces for:

```text
HybridTable / indexes / record references
Pattern classifiers and encoders
Language Governance and scoped sigil registries
High-resolution scanners and confidence policy
Vote collection, attachment scheduling, provenance
Orchestration and contextual resource retrieval
Transient synthesis and ATP
Visual adapters and renderer
Non-pattern residual/idle nodes
Feedback, growth, mutation, PHAGY
Scheduler, Index Rafts, tuning sandbox
JSON save/load and compatibility validation
```

The document does not choose a programming language or claim existing WDBX repository parity. Source code, documentation conformance, tests, performance measurements, and live specimen behavior require separate evidence.

### 25.3 Behavioral acceptance cases

| Case | Expected result |
|---|---|
| Two original paraphrases share a compatible key | Both remain distinct entries; both can independently qualify |
| Two nodes share a logical ID | Lookup returns both; feedback/removal can target one concrete contributor |
| A node's 21st unique same-key entry arrives at default capacity | Apply the explicit full-node policy without overwriting existing entries |
| Identical ID strings use different resolution classes | They do not compare equal as complete Pattern keys |
| A low-resolution candidate is retrieved | Its secondary comparison still uses high-resolution scanning |
| Similarity and dissimilarity sums exceed narrow integer capacity | Safe arithmetic preserves their difference; confidence remains bounded |
| Confidence is exactly +100 or -100 | No jitter applies |
| Maximum-strength node has a qualified baseline | Its score does not jitter; below-threshold entries retain jitter eligibility |
| One node has several matching entries | Every qualified entry may emit its vote |
| Several complementary nodes use the same triple | Their contributions survive; no global election removes them |
| Fan-outs produce redundant alternative contributions | Scoped spin-bottle selection resolves the redundancy with valid weights |
| A hard attachment triggers a below-threshold target | Target is evaluated but emits no forced vote |
| A bidirectional attachment cycles without consuming remainder | Visit/depth policy stops repeated non-progressing work |
| Lookup returns more than 1,000 deep comparisons | Queue all admitted work; peak simultaneous comparisons stays within the cap |
| Foreground and maintenance scans coexist | They share the same cap, including attachments |
| A vote and activator reference different supporting keys | Both query paths are used; only contextually relevant returned subdata is consumed |
| Two resources share a linked resource ID | Both are retrieved; absence of a resource link does not remove structural indexing |
| Side-resource retrieval is traced | No global semantic scan or node-style confidence calculation appears in that path |
| `two plus two` and `2+2` | Bind compatible procedural structure and preserve operands; permitted action yields 4 |
| `don't solve 2+2` | Negation/inhibition survives preparation and blocks the prohibited action |
| User automaton exceeds its execution budget | It terminates with an explicit bounded-execution result |
| Imagination runs on retrieved material | Temporary synthesis uses source ingredients; persistent learned state is unchanged |
| Transient execution is repeated | Weight/knowledge state does not train or drift merely from invocation |
| ATP spans two related cycles | Intended buildup/release persists; last-cycle random jitter does not |
| Thesaurus replacement changes tone beyond tolerance | Retry within budget or use the original phrase |
| An output combines several nodes and later gets paraphrased | Provenance still identifies all causal voting contributors and transformations |
| `/right green` with one contributor appearing in several spans | That node receives one independent feedback trial for the event |
| A node falls from maximum strength | Node crystallization is removed; attachment mode is not accidentally changed |
| One same-ID node reaches zero | Remove only that active record; its peers remain |
| A Type A node detects a residual error | Correction is attributable to the original output and expires within bounded supervision |
| Type B finds new knowledge | Present an identified proposal; no persistent knowledge promotion until `/learn id` |
| Mutation pairs five weak alternatives with four strong alternatives | Recipient count becomes four only under valid history-preserving shortening rules |
| Mutation pairs four weak alternatives with five strong alternatives | Recipient remains four; no fifth alternative is added |
| A recipient slot already remixed | It is ineligible, including after save/load or slot renumbering |
| A prohibited donor relationship is selected through another slot | Lineage policy rejects the pathological reuse |
| Growth proposes an original inhibited pattern/vote pairing | Reject it, including configured relevant fan-out equivalents |
| Idle timer fires repeatedly | Each event draws a fresh interval and chooses tag or PHAGY |
| PHAGY jobs overlap or time out | Unique job claims prevent duplicate active work; timed-out agents release resources |
| Rafts start at 0, 20, and 50 | Exactly the assigned ranges are covered with no overlap/gaps and per-step yielding |
| Duplicate-ID matches span several chunks | `find_all` completes every chunk; a first hit does not terminate the search |
| The sandbox prefers a simple loop | HybridTable uses that choice within limits; raft dispatch is not mandatory |
| A visual record has unequal array lengths | Reject it before scanning/rendering |
| A history record is pinned | Ordinary rolling overwrite preserves it under the pin policy |
| A complete specimen is saved and loaded | All durable organs, duplicate IDs, strengths, links, scripts, history, and mutation markers survive |
| A saved specimen is malformed or incompatible | Load fails without replacing the current specimen |
| A sigil calls an undeclared subsystem | The scope/contact boundary rejects the call |

These are acceptance requirements for a future implementation, not a report that software tests have run. Performance claims require workload-specific measurements; narrative biological analogies are design inspiration rather than validation.

### Native implementation profile · 0.2

Acceptance is recorded by layer: engine unit/conformance tests; browser desktop/mobile regression; native UI on each OS/architecture; real OCR; real model inference; accelerator tolerance; installers; signing/notarization. The macOS test bridge is feature-gated and absent from release builds. A built runtime or verified model download does not prove generation; a packaged installer does not prove native UI behavior on another OS. Camera input, hosted inference and online training remain excluded. The source includes executable tests and a platform qualification workflow, while the verification report identifies outstanding evidence.

## 26. Source coverage map

This map traces the source's narrative blocks to their normalized homes. The source is the referenced conversation, not external framework documentation.

| Source block or mechanism | Specification location |
|---|---|
| Section 1: save file, pattern tables, default 20, original content, IDs, confidence, jitter | §§3–6, 9, 24 |
| Section 1 clarity: class selection, low/medium/high resolution equivalence | §§3.1, 4, 9.1 |
| Section 2: relational triples, fan-out, votes, weighted alternatives, inhibition, spin bottle | §§7–9, 11 |
| Strength, `/right`, `/wrong`, zero-strength removal, crystallization | §§9.2, 18, 23 |
| Nine language utilities and named dependencies | §7.1 |
| Sigil separation, procedural arithmetic/action examples, dictionary reuse | §§7–8 |
| Last `100,00` messages/answers, pins, temporal coherence | R12, §15 |
| Sparse transient matrices, ReLU/Sigmoid bias, urgency, self-observer/inhibition | §13 |
| Stronger brainstorm jitter and smeared stochastic biases | §14.3 |
| GPU eyes, peripheral focus/blur, arousal, nonlinear SDF arrays, OCR, fuzzy art | §16 |
| ATP, tonal inertia, curve/snapback, self-input, rewards | §14 |
| Growth and questions for unknown knowledge | §19.1 |
| Pattern-ID-guided side retrieval using votes and activators, no secondary confidence | §12 |
| HybridTable for all systems, small array/large hash, full JSON save/load | §§6, 24 |
| XML node/entry/vote clarification | §5.1 |
| Imagination ingredient pooling and later no-learning clarification | §13 |
| Distributed gated reasoning versus higher-order composition | §§2, 9, 11 |
| Two-pass anti-staleness using Thesaurus and ATP/Chargebook | §11.3 |
| Index Rafts, placement, yielding, threads, sandbox, chunk size | §20 |
| Reserved sigil list, eight primitives, custom automata distinction | §8 |
| Contributor fractal, segment-targeted feedback, first-class ephemeral metadata | §§5.3, 18 |
| Attachments, unmatched remainder, hard/soft mode, direction, grouped votes, autowiring | §10 |
| Duplicate node IDs and global 1,000 active scans | §§3–6, 20.6 |
| Weak/strong vote mutation, Markov remix, one-time slots, donor cycles, inhibition | §§19.3–19.5 |
| Idle tag/PHAGY coin flip, unique cleanup jobs and timeouts, unresolved grave rules | §§19.2, 19.6–19.7 |
| Address/retrieval/reasoning separation and ethereal contact ledger | §§2, 21 |
| Explicit subsystem lookup and optional duplicated resource IDs linked to node IDs | R05, §12 |
| Shorter recipient pool and per-slot donor/original weight coin flips | §§19.3–19.4 |
| Non-pattern A/B, context magnets, residual correction, idle correlation, `/learn` | §§5.1, 17, 22 |

The specification retains every described mechanism, exposes incompatible statements through the resolution register, and leaves unselected algorithms and numeric values visible. It is ready to serve as a reviewable architecture handoff, with implementation claims contingent on the decisions and acceptance evidence above.

### Native implementation profile · 0.2

This revision preserves every original chapter and its source-coverage map. The Native implementation profile paragraphs are additions derived from the approved expansion plan and current source, explicitly separating bounded initial mechanisms, proposed requirements and unresolved acceptance. Optional Qwen/SDXL generation is a new extension, not a retroactive reinterpretation of the original specimen notes.
