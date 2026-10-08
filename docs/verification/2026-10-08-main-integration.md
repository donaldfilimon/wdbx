# Main integration verification — 2026-10-08

Current local evidence for the retained WDBX documentation and Specimen Studio
changes. The WDBX substrate source remains at the gate-qualified 7ddeb3d revision:
647 Rust test executions, zero failed or ignored, exact-revision CI passed.
No substrate Rust source changed during this documentation/Studio integration.

From `specimen-studio/`, with pinned Rust, system Swift and Python 3.14.8:

- `bun run check`: exit 0; TypeScript, studio lint, WASM build, 195 Bun tests
  (zero failures), production Worker build passed.
- `bun run lint`: exit 0.
- `bun run test:native`: exit 0; 90 Rust tests passed, one existing ignored.
- `bun run check:native`: exit 0; warnings-denied workspace Clippy passed.
- `bun run test:browser` against a fresh local preview: exit 0 in Chrome,
  Firefox and WebKit, each in default dark and explicit light themes. All
  interaction, accessibility and UI-component suites passed. Accessibility
  covered 390, 768 and 1440 pixels, including the split-studio-nodes state.

The initial browser gate selected Apple's Python 3.9 and failed in typed Python
helpers; changing interpreter selection to installed Python 3.14.8 resolved it
without source changes. No private store was used. The owned preview was stopped.

Current ecosystem prerequisites: ABI main 5c275dd and Abbey main e4724c8 passed
local gates and exact-revision CI. AbbeyBot main c8c76ef passed strict local and
exact-revision CI; its transactional installation reports ready. Installed ABI
CLI/gateway/shims and bot match release artifact bytes. These facts do not prove
provider answer quality or human voice acceptance.

Historical 2026-10-03 reviews and ledgers retain their original scope. Their
private/local artifact paths are historical references, not hosted deliverables.
Packaging/signing/notarization, native UI, inference, hardware and broader
ecosystem roadmap/launch acceptance remain Partial or open where stated.
