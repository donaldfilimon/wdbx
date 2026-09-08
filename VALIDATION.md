# Validation record

## Release-candidate acceptance ledger

Status: **qualification in progress; no signed release candidate is certified**.
This pre-freeze record does not declare a final source SHA. Its clean consolidated baseline is
`6d76b0c0c08353ee5537d8c006b38038337b9682`. This task owns final consolidation,
qualification, and the authorized non-force push to `github/main`; the custom
`origin` must remain unchanged. Historical successes do not qualify this candidate.

### External final receipt convention

This document records the procedure and verified pre-freeze results. After the
final source commit is frozen and pushed, exact qualifying run URLs, platform
conclusions, artifact hashes, signature results and local/manual observations
belong in the qualification-summary workflow's JSON/Markdown artifact. Its
source SHA must equal every qualifying run's `headSha`. No documentation-only
commit may invalidate that frozen SHA. Failed, blocked and unverified layers
must remain distinct from passed layers. A corrective source commit establishes
a new candidate and requires the complete final-SHA hosted matrix again.

### Fresh local baseline, September 8, 2026

At `6d76b0c`, the following commands returned exit 0 in this task:

- `bun install --frozen-lockfile`: checked 986 installs across 1100 packages,
  no dependency changes; Bun 1.4.3-canary.1 locally (hosted CI pins 1.4.0).
- `bun test`: 58 passed, zero failed, 262 assertions across five files.
- `bunx tsc --noEmit`, `bun run lint:studio`, and `bun run build`: passed.
  Build completed all five Vinext stages; route classification and Node
  `module.register()` deprecation notices remain advisory.
- `cargo fmt --all -- --check`: passed.
- `cargo test -p specimen-core --locked`: 13 unit and 18 conformance tests
  passed; one GPU test ignored by the normal suite; zero doctests.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed. Rust reports
  future-incompatibility notices for block/wgpu dependencies separately.
- `cargo test -p specimen-core neural::tests::cpu_gpu_parity -- --ignored
--exact --nocapture`: one passed; backend wgpu, 32 measured outputs, maximum
  delta 0.00000012 below 0.0001. Log: `work/qualification-20260908/gpu.log`.
- `cargo test -p wdbx-studio-desktop --locked`: one invocation cancellation /
  durable-revision regression passed.
- `python3 scripts/build-runtimes.py`: built both CPU helpers. Upstream llama
  UI asset retrieval fell back from unavailable `b38` to its `latest` archive;
  this ancillary UI asset is not revision-pinned. Third-party compilation
  warnings were retained, not patched.
- `bun run desktop:package`: first attempt failed in Tauri's DMG wrapper after
  producing the executable and app. A single `--verbose` retry passed with no
  source changes. The first wrapper failure's cause is not established.
  Both logs remain under `work/qualification-20260908/`.
- `python3 scripts/seal-macos-package.py`: strict ad-hoc bundle verification
  and DMG integrity passed. Production artifacts were copied to
  `work/qualification-20260908/production-bundle/` before E2E asset generation.
  The ARM64 DMG SHA-256 is
  `4478090b4b43887a9440c360292bcd9fb24cf7d45d43776f74e1bf1e8ad4dfa0`.
  This is a local baseline artifact, not a final-SHA release or notarization.
- `python3 scripts/check-production-bridge.py`: production frontend bytes and
  330-package normal dependency graph passed exclusion checks. The receipt
  explicitly records documentation/evidence-collector edits in the tree; this
  is build-input evidence, not an independent compiled-binary inspection.
- `python3 scripts/ocr-fixture.py work/acceptance`, verified existing pinned
  detection/recognition models, and release-mode Rust acceptance `ocr`: passed;
  recognized output `E HELLOWORLD 1` contains both required words. Result:
  `work/acceptance/results/ocr.json`.
- `GITHUB_SHA=6d76b0c0c08353ee5537d8c006b38038337b9682
STUDIO_BROWSER=<chrome|firefox|webkit> STUDIO_URL=http://127.0.0.1:4187
bun run test:browser`: all three complete runs passed. Each engine passed
  workflow/topology and focused accessibility at 390/768/1440 with zero page
  and console errors. Engine-suffixed JSON/screenshots are under `work/`;
  logs are under `work/qualification-20260908/`. Playwright is the repository
  gate; the separate Browser skill/plugin is not available in this session.
- Firefox 155.0.1 installed through its Homebrew cask; strict `codesign` and
  `spctl --assess --type execute` passed (`Notarized Developer ID`). Chrome
  remains 152.0.7977.83; Safari remains 27.0.
- Instrumented frontend (`VITE_NATIVE_E2E=1 bun run desktop:build`) and Rust
  build (`TAURI_CONFIG` from `src-tauri/tauri.e2e.conf.json`, features
  `e2e,custom-protocol`) passed. WDIO with a fresh isolated data directory
  initially passed three cases and failed waiting for the large-history row.
  A diagnostic rerun passed all four cases, nine initial virtualized rows,
  37 ms target search. Only failure diagnostics changed between those runs.
  Logs: `work/qualification-20260908/native-e2e.log` and
  `native-e2e-diagnostic.log`. Embedded-driver discovery, focus-timeout and
  teardown warnings remain harness limitations; the initial race is not proven
  fixed. Production artifacts remained preserved separately.

The literal `python3 -m pip install pillow` command was rejected by Homebrew's
externally-managed-environment policy. Existing Pillow 12.3.0 imports successfully;
no system-package override was used. Endor's package-risk advisory could not run
because installed `endorctl` lacks the `agent` command; risk is unknown.

After collector consolidation through `289f28d`, a fresh full Bun run passed
77 tests / 316 assertions across six files. TypeScript, Studio lint and the
five-stage production browser build also passed again. Rust formatting,
31 core/conformance tests and workspace Clippy passed again, captured in
`work/qualification-20260908/rust-pre-freeze.log`. These changes add
evidence collection, fixture coverage, workflow retention and failure-only
native test diagnostics; browser/native runtime inputs remain unchanged from
the baseline above. Final source-bound browser receipts and the complete hosted
matrix remain required after freezing.
The subsequent `68dc110` completeness fix adds required platform/browser/a11y
coverage, mandatory signed-output evidence and same-run failure isolation.
Its implementation gate passed 78 Bun tests / 334 assertions (focused collector:
20 tests / 72 assertions), workflow lint and Python bytecode compilation.
The final accessibility-content fix `1fdbd05` requires populated per-width audits,
zero violation counters, target and keyboard evidence, and trace outcomes. Its
gate passed 78 Bun tests / 338 assertions (collector: 20 / 76), Python compilation
and whitespace checks. The implementer's later `actionlint` attempts stalled
without diagnostics and bounded runs returned 124; their owned stale processes
were terminated. This does not replace the root's successful independent
all-workflow lint result recorded above. No workflow changed in `1fdbd05`.
The independent command was `actionlint .github/workflows/*.yml` and returned
exit 0 with no diagnostics.
The root's subsequent full-suite run reached 77 passes but the expanded browser
receipt fixture exceeded Bun's default 5000 ms test timeout at 5705 ms; cleanup
then caused a secondary missing-file error. This fixture launches the collector
eight times. Its test-only budget was made explicit at 30000 ms without changing
assertions, and the focused rerun passed 20 tests / 76 assertions. The initial
failure remains in `work/qualification-20260908/bun-reviewed-tree.log`.
The full rerun passed all 78 tests / 338 assertions in 7.75 seconds; log:
`work/qualification-20260908/bun-reviewed-tree-rerun.log`.

Partial graphical-browser observations were captured separately under
`work/qualification-20260908/`: Safari and installed Firefox responsive views
at 390, 768 and 1440, arithmetic completion, trace/dossier disclosure and
overlapping feedback disablement. Firefox's mobile drawer wrapped keyboard
focus and returned it to Open navigation on Escape; its specification link
opened the named reader and chapter navigation. The 1440 Firefox viewport
exceeds this screen's visible width, so its native screenshot alone does not
establish full-width visibility. These observations do not establish the full
manual matrix, 200% browser zoom, enlarged text, VoiceOver or packaged acceptance.
The original Increase contrast preference was off. It was briefly enabled,
then restored to off after the audit was interrupted; Reduce transparency and
Show borders remained off and the display-contrast slider remained zero.
No contrast-audit pass is claimed.
Actual Chrome additionally completed the keyboard route from the visible skip
link through the prompt, dossier, segment feedback and trace. After feedback
disabled the overlapping controls, the next Tab reached the comparison
disclosure. The trace exposed Completed and selected toggle states. A 200%
browser-zoom screenshot records stacked reflow of the prompt and result;
Chrome zoom was restored to 100%. This is a partial zoom observation, not the
complete three-width zoom/manual matrix or a screen-reader speech capture.
The Chrome keyboard journey also reached topology zoom: 130% disabled Zoom in,
the next Tab focused the named scrollable canvas with a visible focus outline,
arrow keys exercised its scrollbars, and Shift-Tab/Enter on Fit restored 100%.
Opening Edit node focused Node name; Shift-Tab wrapped to Close, Tab returned
to Node name, and Escape restored focus to Edit node without changing the node.
VoiceOver was initially off and Show caption panel was already on. VoiceOver
was briefly enabled, but the app-window screenshot omitted its overlay and
selecting the VoiceOver surface timed out. VoiceOver was restored to off and
verified; the caption preference stayed on. Spoken names/states, trace outcomes
and feedback speech remain unverified; accessibility-tree text is not a caption
or audio receipt.

| Requirement                              | Gate / platform                                                  | Current result                                             | Evidence / unresolved dependency                                                                                                                       |
| ---------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Browser engine, imports and UI contracts | Canonical `bun test`, local macOS                                | Passed baseline: 58 tests / 262 assertions                 | Final source and evidence-collector tests still require review                                                                                         |
| Browser types / lint / production build  | `bunx tsc --noEmit`, `bun run lint:studio`, `bun run build`      | Passed baseline                                            | Exact commands above; final candidate qualification pending                                                                                            |
| Browser interaction and accessibility    | `bun run test:browser`, Chrome / Firefox / WebKit                | Passed all three baseline engines                          | `work/browser-checks-*.json`, `work/accessibility-checks-*.json`; final SHA qualification pending                                                      |
| Native conformance baseline              | `cargo test -p specimen-core --locked`, Apple silicon            | Passed baseline: 13 unit + 18 conformance, one GPU ignored | Not a final candidate receipt                                                                                                                          |
| Native Clippy baseline                   | `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed baseline                                            | Dependency future-incompatibility notices remain                                                                                                       |
| Native UI                                | Instrumented WebdriverIO, four desktop targets                   | Local diagnostic rerun passed 4/4; hosted matrix pending   | Initial large-history visibility failure and harness warnings retained; instrumented tests do not qualify the signed payload                           |
| Real OCR                                 | Pinned detection/recognition models and HELLO WORLD fixture      | Passed local baseline                                      | Actual recognized output above; final hosted source-bound receipt pending                                                                              |
| Real text inference                      | Qwen3-4B, macOS Intel reference                                  | Historical success only                                    | [Run 33878636044](https://github.com/donaldfilimon/wdbx-specimen-studio/actions/runs/33878636044), source `b2c382d`; desktop job skipped               |
| Real image inference                     | SDXL-Turbo, macOS reference                                      | Historical success only                                    | [Run 33878632925](https://github.com/donaldfilimon/wdbx-specimen-studio/actions/runs/33878632925), source `b2c382d`; new source requires qualification |
| Accelerator                              | Apple silicon wgpu, tolerance 0.0001                             | Passed local parity                                        | Measured max delta 0.00000012; final source-bound receipt pending                                                                                      |
| Installed helper portability             | All four desktop targets                                         | Pending candidate matrix                                   | Must start/stop packaged helpers; startup is not inference qualification                                                                               |
| macOS signed DMGs                        | Developer ID, notarization, stapling, final payload              | Blocked                                                    | No local Developer ID Application identity and repository secret inventory empty                                                                       |
| Windows signed installer                 | Authenticode and installed payload hashes                        | Blocked                                                    | Repository signing secrets absent; historical runs do not qualify new source                                                                           |
| Linux packages                           | AppImage / Debian installed launch and helpers                   | Pending                                                    | Requires Linux runner and candidate artifacts                                                                                                          |
| Upgrade / uninstall                      | Prior package and isolated application data                      | Pending                                                    | Establish prior-package baseline before claiming preservation                                                                                          |
| Manual accessibility                     | Safari, Firefox, VoiceOver, physical touch, enlarged text        | Pending                                                    | Automated WebKit is not Safari or physical-device acceptance                                                                                           |
| Optional WebMCP                          | Real `document.modelContext` registration / invocation           | Unsupported in tested headless browsers                    | Graphical fallback remains available; native API execution unqualified                                                                                 |

Evidence must name the source SHA, successful workflow run, runner and artifact
digest. Signing or repackaging requires a new final-payload digest and installed
verification. Changed source invalidates affected candidate gates. Release
notes and execution status are in `RELEASE-CANDIDATE.md`; this ledger is the
acceptance authority before freeze; the external final receipt carries the exact
frozen-SHA results afterward. Only the explicitly authorized GitHub push and
qualification workflows are in scope, not a separate application deployment.

## Historical browser layout verification

The following record predates the release-candidate changes and is retained as
historical evidence, not as certification of the current source or CI matrix.

Validated on September 8, 2026, using the repository's local Vite preview and
headless Chrome.

- TypeScript type checking: `bunx tsc --noEmit` passed.
- Focused lint checks: `bun run lint:studio` passed for the studio, specimen
  runtime, and WebMCP adapter.
- Production build: `bun run build` passed. Vinext emitted its existing dynamic
  route-classification note and Node emitted the `module.register()` deprecation
  warning; neither stopped the build.
- Runtime tests: `bun test` passed 17 tests across 2 files, with 77 `expect()`
  calls and no failures. Coverage includes the instrument-console contracts,
  arithmetic and parser failures, exact duplicate rejection, same-ID peers,
  preserved multi-entry edits, bind templates, bounded confidence, feedback
  deduplication, raft coverage/cancellation, pin retention, complete JSON
  round-trip, malformed nested data, last-peer detachment, Type A/B proposals,
  and bounded automata/PHAGY.
- Browser E2E: `STUDIO_URL=http://127.0.0.1:4176 bun run test:browser`
  passed. It exercised arithmetic, topology/trace switching, all six trace
  phases, result outcome context, contributor provenance and native evidence,
  feedback deduplication, pinning, node creation, a taught response, combined
  settings/name save, IndexedDB reload, JSON download and restore, malformed
  import rejection, specification navigation, and the phone drawer lifecycle.
- Independent browser rerun: `STUDIO_URL=http://127.0.0.1:3017 bun run
test:browser` also passed with zero page or console errors. Port 3017 kept the
  run isolated from an unrelated application already listening on port 3000.
- Responsive topology acceptance passed at 390 × 1000, 768 × 1000, and
  1440 × 1000. At each width, Fit returned the topology to 100%, every rendered
  graph node remained inside the canvas, zoom reached its bounded 130% state,
  and the zoomed canvas exposed scrolling rather than clipping its controls or
  nodes. The broader run also checked the 1568 × 1000 desktop composition and
  390 × 844 phone/reader flow, with no horizontal document overflow or uncaught
  browser runtime errors.
- Fresh desktop, phone, and specification-reader screenshots were inspected at
  native size. The 390-pixel console keeps the view toggle and full zoom control
  group visible by wrapping them within the topology panel.
- Contributor feedback acceptance exercised a descriptively named segment
  control, verified that both it and the overlapping whole-result control use
  native disabled semantics after submission, and observed the result through
  the polite live region.
- WebMCP: Chrome did not expose `document.modelContext`; native
  registration/execution remains unverified. The application feature-detects
  the optional API and retains its graphical workflows. This is not a claim of
  verified native WebMCP support.

## Remaining native and manual qualification gaps

- The Rust engine tests, workspace Clippy gate, Tauri launch, and desktop
  packaging were not rerun for this browser-only layout correction. This record
  does not extend the browser result into native-engine or packaged-desktop
  qualification.
- CPU runtime construction, model downloads, live text or image inference,
  accelerator behavior, signing, notarization, and installer behavior were not
  exercised and remain unqualified.
- The responsive interaction run used headless Chrome. Safari, Firefox,
  physical touch input, keyboard-only traversal of the entire studio, screen
  readers, browser zoom/text enlargement, and a manual packaged-Tauri visual
  pass remain unqualified.
- Native WebMCP registration and execution still require a browser/runtime that
  exposes `document.modelContext`.

## Design comparison

The full-screen generated studio and reader concepts were inspected before implementation. Rendered desktop and phone screenshots were inspected at their native size.

1. The cool gray left navigation, white canvas, and teal accent match the chosen direction.
2. The dominant composition remains a live dotted topology and adjacent node inspector.
3. The node field hierarchy, strength meter, jitter control, and edit action remain visible and operable.
4. The conversation, attribution feedback, and prompt form sit directly beneath the graph; the final desktop adjustment keeps the prompt within the 1000-pixel composition.
5. The specification reader retains a chapter rail, search, chapter counter, long-form text, and Markdown download. Actual specification wording is preserved rather than abbreviated to fit the concept.

Deliberate differences: responsive navigation becomes a drawer on phones; graph positions are computed from actual node count; the reference reader flows its full text vertically; the browser profile exposes real state and results instead of seeded response mockups. The visual synthesis is explicitly labeled as a fixed-feature visual study.

The accessibility pass followed the fetched Vercel Web Interface Guidelines: named controls, real buttons/links, associated form labels, visible focus, reduced-motion handling, error announcements, keyboard-supported dialogs, and URL-addressable reference views.

The broader specification remains distinct from this browser implementation. See RUNTIME-PROFILE.md for exact executable mechanisms and limits.
