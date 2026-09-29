# Browser runtime profile

This document describes executable behavior in this studio. The full WDBX specification remains a broader architecture, with its original mechanisms, open choices, and explicit contradiction resolutions intact. This application is a bounded, inspectable implementation profile, not certification of every proposed mechanism.

Since B2 (2026-09-29) the browser runs the same Rust kernel as desktop,
compiled to WebAssembly (`native/specimen-kernel` via `native/specimen-wasm`,
loaded by `lib/specimen/kernel.ts`). Retrieval, voting, sigils, negation,
attachments, orchestration, feedback, Type A/B review, mutation and PHAGY are
therefore the native behavior described in `NATIVE-RUNTIME-PROFILE.md`; this
page lists only what differs in the browser.

| Area | Browser behavior |
| --- | --- |
| Engine | The kernel WASM (about 1.7 MB) loads once at startup. Edits (seed, add node/entry, remove node, memories, feedback, pins, settings checks) are synchronous calls on the main thread. Cycles and reviews run in a Web Worker so the page stays responsive; Cancel terminates the worker. Trace steps stream as the kernel emits them. |
| Index Rafts | The scan is sequential (`search::Sequential`) with the same chunking, validation, progress checkpoints and cancellation checks as the native threaded scan; results and ordering are identical. |
| Neural synthesis | The browser uses the kernel's default network; desktop persists and edits its own. CPU only; no GPU accelerator. |
| Persistence | IndexedDB stores complete committed specimens. JSON import/export contains nodes, resources, attachments, history, feedback, ATP, settings, proposals, and mutation lineage. Import runs structural checks (`contracts.ts`) and then the kernel's native validation before replacement. Saves from the retired TypeScript engine migrate on load: Pattern IDs are recomputed with the native identifier and memory links are remapped. There is no server-side specimen database or cross-device synchronization. |
| Vision | Image analysis, OCR and model inference are desktop-only. |
| WebMCP | Feature-detected page tools read the specimen, run a prompt, and navigate to a reference chapter. They share the visible state and validate their inputs; unsupported browsers retain all UI workflows. |

Semantic changes from the retired TypeScript engine (native semantics won):
Pattern IDs use `text-v2:<resolution>:<digest>`; feedback coin flips use the
kernel's 64-bit xorshift; Type A review records a correction instead of a
proposal; settings bounds are the native ones; the starter arithmetic node
binds one binary operation (`7 * 6`), so chained expressions are unmatched;
`then`/`and`/`but` start new clauses, so a negation inhibits only its own
clause; trace phases are Prepare, Retrieve, Index Rafts, Deep scan, Vote,
Compose.

## Bounds and interpretation

- Arithmetic: 2,000 input characters and 500 parser steps. Tape programs: 1,024 byte cells and 10,000 instructions. Repeat: 1–100.
- Prompt length: 8,000 characters. Pattern length: 2,000. Action length: 8,000. Memory value length: 12,000. Imported file size: 20 MB.
- Default node entry capacity: 20. Defaults and editable bounds appear in Settings and are validated on import. Maximum stored supporting resources: 10,000; activity retains at most 500 records, or 200 before a new PHAGY event.
- The shell shows one view, or two side by side at 1024 px and wider (`?split=`); the command palette (⌘K), icon-rail sidebar (⌘B) and activity dock (backtick) are keyboard-reachable. The topology shows up to 16 nodes. Node and history lists use virtual scrolling; supporting-resource views show the first 100 filtered records and remain searchable. Complete records remain in the saved specimen.
- Browser autosave is specific to origin and device. Switching between preview and the private hosted URL starts separate device storage. Download and load a specimen to transfer it.
- GPU/NPU acceleration and adaptive OS threading are desktop concerns; general learned vision, full symbolic language governance, full Type A/B autonomous induction, unrestricted procedural grammars, and general PHAGY are extension work described by the full architecture.
