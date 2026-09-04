# Browser runtime profile

This document describes executable behavior in this studio. The full WDBX specification remains a broader architecture, with its original mechanisms, open choices, and explicit contradiction resolutions intact. This application is a bounded, inspectable implementation profile, not certification of every proposed mechanism.

| Area | Working behavior |
| --- | --- |
| Pattern storage | Unique node references, repeating Pattern IDs, original entries, weighted alternatives, inhibition terms, strength, jitter, and tone metadata. Additional entry arrays can be edited in the advanced node editor. |
| Pattern retrieval | Three deterministic complexity classes. Named structural signatures cover arithmetic, repeat, recall, time, tone, negation, greeting, and imagination; other shapes use a stable FNV-derived signature. Equality retrieval uses an array/hash HybridTable. Bind templates and wildcard candidates supplement the ID route. |
| Confidence | Signed token overlap for nonbinding comparisons; successful structural bind matches score 100. Values clamp to −100…100; endpoints and qualifying crystallized nodes do not jitter. |
| Voting | Every applicable entry is considered. Noninhibited alternatives are sampled by weight. Equivalent normalized action groups use confidence × strength sampling. Complementary actions keep separate output segments. |
| Sigils | Bounded arithmetic parser, repeat up to 100, local time, linked dictionary/pinned recall, ATP tone read, fixed visual synthesis, current-input substitution, linked memory substitution, and an eight-primitive tape interpreter. Unrecognized sigils remain literal text. No arbitrary JavaScript execution. |
| Negation | Conservative cycle-wide negation suppression for “don't,” “do not,” and “never.” The architecture’s full clause-level semantic scopes are not implemented. |
| Attachments | Hard or probabilistic directional/bidirectional links pass lexical unmatched remainder, with visited-job deduplication and a four-hop bound. This is a lexical approximation of semantic unmatched-region delegation. |
| Orchestration | Preparation, retrieval, deep scan, vote, enrichment, and composition appear in the trace. Supporting information is filtered by candidate Pattern IDs or linked node IDs and contextual relevance. The browser profile does not implement the full grammatical/semantic governance engine. |
| Provenance and feedback | Segment contributor references, vote references, transformation history, and color labels persist. Feedback is deduplicated by contributing node across overlapping selections. Nodes at zero are removed; links detach only when the final node with an ID is removed. |
| Type A | A matching supervisor can prepare a proposal from a recent unmatched cycle within its 90-second residual window. Review follows a prompt when a Type A node is present and is also available in Activity. Learning remains an explicit reviewed action. |
| Type B | An idle observer prepares a proposal after at least two repeated unmatched prompts in its optional context scope. It runs during idle maintenance or manual context review. This uses repeated exact prompts rather than an open-ended semantic induction model. |
| Index Rafts | Context history review uses disjoint cooperative subtasks, joins all partitions in a chunk, preserves original ordering, and visits every record. Max rafts, activation threshold, and chunk size affect this real traversal. One JavaScript execution lane; these are not OS threads. |
| Transient synthesis | Deterministic prompt features and seeded sampling choose fixed ReLU/sigmoid activations to create aligned x/y/color/brightness arrays. No online weight training or claimed learned image generation. The resulting display is explicitly called a visual study. |
| ATP | Decaying valence/intensity and chargebook matching update between cycles; tone sigils read current ATP. The full two-stage modulation and language re-inflection design is outside this browser profile. |
| Mutation | One bounded weak/strong scan; similar literal action pairs can remix sentence transitions and inherit a weight. Remixed slots and original inhibited pairings retain lineage. Procedural programs are excluded from remixing. |
| PHAGY | One bounded cleanup job trims old activity, rolling history, and invalid attachments; pins and mutation history survive. General specimen-wide biological digestion and learned pruning remain architecture extensions. |
| Persistence | IndexedDB stores complete committed specimens. JSON import/export contains nodes, resources, attachments, history, feedback, ATP, settings, proposals, and mutation lineage. Import validates nested records, limits, identities, and aligned visual arrays before replacement. There is no server-side specimen database or cross-device synchronization. |
| Commands | Prompt, feedback, contributor display, save, load, add-pattern, and reviewed learning flows are implemented. Their graphical equivalents remain available. The full specification’s command grammar is retained in the reader. |
| WebMCP | Feature-detected page tools read the specimen, run a prompt, and navigate to a reference chapter. They share the visible state and validate their inputs; unsupported browsers retain all UI workflows. |

## Bounds and interpretation

- The active scan ceiling controls cooperative work batches. The one-lane executor never exceeds that concurrency ceiling; it does not discard later queued entries.
- Arithmetic: 2,000 input characters and 500 parser steps. Tape programs: 1,024 byte cells and 10,000 instructions. Repeat: 1–100.
- Prompt length: 8,000 characters. Pattern length: 2,000. Action length: 8,000. Memory value length: 12,000. Imported file size: 20 MB.
- Default node entry capacity: 20. Defaults and editable bounds appear in Settings and are validated on import. Maximum stored supporting resources: 10,000; activity retains at most 500 records, or 200 before a new PHAGY event.
- The topology shows up to 16 nodes. Lists show the first 100 filtered records; use search to narrow a larger collection. Complete records remain in the saved specimen.
- Browser autosave is specific to origin and device. Switching between preview and the private hosted URL starts separate device storage. Download and load a specimen to transfer it.
- GPU/NPU acceleration, adaptive OS threading, general learned vision, full symbolic language governance, full Type A/B autonomous induction, unrestricted procedural grammars, and general PHAGY are extension work described by the full architecture.
