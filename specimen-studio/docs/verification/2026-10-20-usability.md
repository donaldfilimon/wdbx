# Studio usability and local verification — 2026-10-20

Scope: the browser and native Studio in this checkout on macOS ARM64, with
stable Rust 1.99.1 selected ahead of Homebrew Rust and Swift compiler shims.
This report concerns a working-tree candidate, including the pre-existing
tablet-header CSS adjustment. It is not an immutable-release or CI receipt.
No ABI, Abbey, substrate, live-store, model-weight or installed-app changes
were made by this slice.

## Improvements

- On stacked phone/tablet layouts, the composer remains ahead of the topology
  and inspector after a run and after reload. Desktop result-first layout is
  unchanged.
- Run is disabled for empty/whitespace input and before workspace hydration.
  Starter prompts are also disabled before hydration; Enter still runs a
  nonempty prompt.
- An unmatched result offers **Teach this prompt**, pre-filling its original
  input in the existing pattern editor. The editor override is separate from
  the composer draft. Cancel/teach preserves the draft; reopening the ordinary
  editor does not retain a stale override. No automatic learning was added.
- Learning proposals (from repeated unmatched) are now fully manageable in
  Activity: **Dismiss** removes from suggestions; **Reconsider** re-activates
  a dismissed one. All proposals visible with status badges; auto-marks learned
  on teach. Full cycle (pending/learned/dismissed) via kernel commands + UI.
- The browser gate now includes four disposable-context usability scenarios
  (added dismiss/reconsider); the native suite covers teaching, draft
  preservation, durable reload and proposal lifecycle too.

All three usability scenarios failed against the original UI and pass against
the candidate. An intermediate test selector was corrected to target the
workflow navigation instead of assuming accessible-name whitespace.

## Executed checks

Run commands from `specimen-studio/`, with the pinned rustup toolchain's `bin`
and `/usr/bin` before Homebrew/Swift shims on PATH. For native builds, set
`DYLD_FALLBACK_LIBRARY_PATH` to the toolchain `lib` directory followed by
`$HOME/lib:/usr/local/lib:/usr/lib`.

| Check | Observed result |
| --- | --- |
| `bun run check` | Passed: instructions, TypeScript, studio lint, WASM, 202 Bun tests, Worker production build |
| `bun run lint` | Passed, broader project lint |
| `bun run test:native < /dev/null` | 107 passed, 0 failed, 1 normally ignored GPU test |
| `bun run check:native` | Passed, warnings-denied workspace Clippy |
| `cargo test --locked -p wdbx-studio-desktop < /dev/null` | 1 passed: cancellation preserves revision and releases the job |
| `cargo test --locked -p specimen-core neural::tests::cpu_gpu_parity -- --ignored --nocapture < /dev/null` | Passed separately: backend `wgpu`, max delta 0.00000012 against 0.0001 tolerance |
| `bun run test:browser` | Passed in installed Chrome, default dark + light themes, fresh production preview (incl. new proposal mgmt) |
| `STUDIO_THEME=light bun run test:browser` | Passed in Chrome, light theme, same production preview |
| Native instrumented UI through Node/WebdriverIO | 7 scenarios passed in native WebKit, isolated new app-data directory |
| `bunx --bun tauri build --bundles app` | Production macOS application built after instrumented testing |
| `python3 scripts/check-production-bridge.py` | Production frontend and normal dependency graph exclude test bridge; not a binary audit |
| `git diff --check` | Passed |

The browser suites cover arithmetic, clause-scoped negation, topology/trace,
node inspection, contributor feedback/deduplication, pinning, teaching and
custom responses, notes, settings, IndexedDB reload, JSON download/restore and
malformed-import rejection, chapters/diagrams, command palette, split panes,
engine/store/network views and the browser Lab boundary. Accessibility passes
at 390, 768 and 1440 pixels in both themes, with zero serious/critical axe
violations in the audited states, unnamed controls or orphaned inputs.

Native UI scenarios exercise Rust cycles and feedback, stale-write refusal,
malformed import preservation, portable archive export/import/reload, image
analysis and explicit visual learning, missing-model errors, model tabs,
revision-checked network edits, unmatched teaching, proposal dismiss/reconsider
and a virtualized 100,000-record history search. These use scratch data, not
the user's store.

## Environment findings and limits

- Forcing WebdriverIO through Bun failed before application launch with
  `TypeError: The superclass is not a constructor` in its matcher dependency.
  Running `node node_modules/@wdio/cli/bin/wdio.js run wdio.conf.mjs` passed.
- The native image fixture needed Pillow, absent from the default Python. It
  was installed only in an external temporary virtual environment to run
  `scripts/ocr-fixture.py`; project dependencies were not changed.
- Native builds retain the existing sibling unused-import and upstream `block`
  future-compatibility warnings. Gate success does not erase those warnings.
- No real text/image model generation, model download/install/remove journey,
  OCR-model accuracy, native WebMCP registration (unsupported in tested Chrome),
  Firefox, standalone Playwright WebKit, Windows/Linux runtime, installer
  signing/notarization or live-store migration is qualified by this run.
  Image descriptor analysis is not OCR inference; GPU parity is not a speedup
  claim. There were no hosted CI, publication or deployment actions.

Local command logs use `studio-*.log` in the harness's approved external
temporary directory. Browser screenshots and native evidence are under ignored
`work/`; runtime stores and generated evidence are not committed. The preview
is served locally with `bun run start --ip 127.0.0.1 --port 3000`.

## Proposal lifecycle (self-learning)
- Created on explicit "Review context" (Type B): repeated unmatched inputs (>=2 in recent history) generate pending proposals with evidence/pros/cons/observer.
- Activity lists all with status badges + context text. Pending: Review & learn (prefills) + Dismiss; Dismissed: Reconsider.
- Transitions guarded in kernel: /learn (or first pending) pre-fills; teach (addNode) auto-marks matching pending as 'learned'; /dismiss (pending→dismissed); /reconsider (dismissed→pending). Events logged; persisted.
- Full cycle in browser usability e2e (disposable), native e2e (durable), kernel tests (roundtrips/undo), engine. Consistent with spec (no auto-inference).

## Cross-repo self-learn changes (2026-10-20)
- Specimen: Local explicit proposals (repetition + teach approval) as UX.
- ABI: `complete --learn` / SEA hardened (rejects live/bridge; requires unique scratch, evidence_count, adaptation; 8+ regressions + 2-process smoke). See tasks/todo.md.
- Abbey: `learn.rs` (corrections/routes/activity → memory); memory-sqlite-self-learn; TUI/fm verification.
- Coordinated 2026-10-20 receipts (rust-1991, CLI hardening, tui-fm-learning, this usability). Specimen verification refs siblings. No source mixing per ownership.

## Follow-up execution (post initial)
- Additional polish: pending count in top-bar header (specimen name area), observer display in activity proposals, cleaned proposal count logic.
- Full re-gates: `bun run check:all` (build + native clippy pass), `bun run test:native`, full `bun run test:browser` (4/4 usability incl. proposals + accessibility + core), production-bridge 11/11, desktop smoke 1/1.
- ABI/Abbey: focused learn/cli/tui tests + clippy pass.
- WDBX root key: instructions, sizes, fmt, clippy (pre-existing warnings only).
- Desktop: instrumented considerations + bridge + smoke validated.
- All browser e2e (incl. new header badge paths) and native coverage exercised. Verification updated for completeness.

## Agent control and 3D (2026-10-20)
- WebMCP stays least-privilege. `workspace_status` is read-only and lists what an agent may not do: dismiss, teach, model install, weight updates, or ABI/Abbey store writes. `run_specimen_prompt` remains one visible cycle, not training.
- Network scenes already render on demand with instanced meshes. Device pixel ratio is capped at 1.5. No new 3D library was added.
- Liquid Glass (`NSGlassEffectView`, macOS 26) was not added. The desktop uses Tauri's existing vibrancy materials on the window, and CSS glass only on the toolbar and sidebar. Reduced transparency uses the same solid fallback as reduced motion.

## Native toolbar and liquid glass (2026-10-20)
- Overlay title bar, transparent window, and macOS vibrancy (`underWindowBackground`, `hudWindow`, `sidebar`). `macOSPrivateApi` is enabled because Tauri requires it for that material. Browser preview stays opaque.
- Toolbar clusters group Commands, Activity, and specimen actions. A drag strip sits above the buttons. Reduced motion drops the blur and restores solid token backgrounds.
- Native File/View/Window menu emits `native-menu`. ⌘S saves even while typing; ⌘O loads when not typing. A pending-proposal Review control opens Activity. No automatic learning was added.

## Native window and honest model lab (2026-10-20)
- Desktop window uses an overlay title bar. `.native-app` tightens the sidebar and flattens panels. Browser selectors and labels are unchanged.
- Models and Generate state the claim boundary: a verified download is not generation; text and image inference are CPU; the GPU line is the composition net; generation is not learned until an explicit teach.
- No weight training, LoRA, hosted inference, or silent learning was added.

## Bulk proposal actions (2026-10-20 follow-up)
- Kernel commands `dismissAllProposals` and `reconsiderAllProposals` (composer `/dismissAll`, `/reconsiderAll` and aliases) change only matching statuses and refuse when nothing matches.
- Activity shows **Dismiss all pending** and **Reconsider all dismissed** only when that status is present. One undo restores the whole batch.
- Learned proposals are left unchanged. No automatic learning was added.
- Native instrumented UI: 7 passing on scratch data. The desktop proposal check seeds history through the test bridge and asserts review creates at least two pending proposals. Activity dismiss/reconsider stays covered by the browser usability suite and kernel tests.
