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

### Superseded candidate and Linux packaging correction

Candidate `720125bca2bd67b4e9a98a0bac224bef41bc589f` was pushed without
force to `github/main`. Its browser matrix, real Qwen text inference,
SDXL-Turbo image inference and wgpu parity passed; downloaded archives and
payload receipts verified locally. Both macOS desktop jobs also passed.
The Linux job in run `34220950934` failed its installed AppImage payload
comparison: linuxdeploy transforms ELF loader metadata after the raw build.
The extracted application and helpers therefore cannot correctly be compared
to raw pre-AppImage whole-file hashes. The failure is retained under
`work/qualification-20260908/hosted/linux-initial-job.log` and is not a
transient retry or a passing Linux qualification.

The corrective gate must bind the retained final AppDir payload separately to
the same source, run, original build identities and installer digest, then
require exact extracted-to-staged hashes. Debian must likewise retain its
bundle-specific payload from the producer's `data` staging tree, not the raw
executable restored when Tauri's bundle command returns.
Expected AppImage hashes must never be derived from the extracted installer
being tested. This source correction supersedes `720125b`; none of its hosted
successes may be relabeled as new-candidate final evidence.

Local gates at that superseded SHA passed: 78 Bun tests / 338 assertions,
TypeScript, Studio lint, browser build, Rust format, 31 Rust tests, Clippy,
both runtime builds, ad-hoc desktop package/seal, production bridge exclusion,
three-engine browser/accessibility matrix, actual OCR and wgpu parity.
The isolated native retry passed four tests in 1m43.8s with a 146ms history
search. Initial frontend fixture timeouts, DMG-wrapper failure and native
driver connection loss remain alongside successful reruns under
`work/qualification-20260908/final/`; no source fix is claimed for those
symptoms. Final candidate results still belong in the external receipt.

The same candidate's OCR run `34221042086` later failed before model download:
the macOS runner refused system-wide Pillow installation under PEP 668. The
workflow now isolates fixture dependencies in an ephemeral virtual environment
and pins Pillow to the locally exercised 12.3.0 version; it does not disable
managed-Python protections. The failed run produced no OCR acceptance artifact.

Windows completed three native scenarios but failed exact portable save/restore.
A local regression reproduced a real JSON parsing defect: the valid stored
fraction `1.8688716782771921` became `1.868871678277192` after archive import.
The workspace enables serde_json's exact floating-point round-trip parser;
the regression covers both archive import and durable reopen. Native E2E
compares complete specimen values with a structural diff and separately checks
the new revision. The Windows failure lacks its original value diff, so the
fresh hosted run must establish whether this correction resolves that case.
Logs remain under `work/qualification-20260908/hosted/windows-initial-job.log`
and `work/qualification-20260908/portable-fractional-*.log`.

Corrective pre-freeze checks: 86 Bun tests / 357 assertions passed, including
28 focused Linux-stage/collector tests with 95 assertions; workflow lint,
Python compilation and diff whitespace checks passed. The exact-float native
regression failed before the parser feature and passed afterward; all 32 Rust
core/conformance tests, formatting and workspace Clippy passed. Independent
review approved the three narrow corrections. Final clean-source local and
hosted reruns are required before accepting the replacement candidate.

### Debian bundle-stage correction after candidate `25ea8e95`

Candidate `25ea8e95cf479172cfd74e38033cfdb8b9904194` passed all local gates:
86 Bun tests / 357 assertions, TypeScript, Studio lint, production browser build,
Rust formatting, 32 core/conformance tests, workspace Clippy, both runtime
builds, production packaging/ad-hoc sealing and bridge exclusion, four native
UI tests (39.6 seconds), real OCR, explicit wgpu parity, and the Chrome/Firefox/
WebKit functional/accessibility matrix at 390/768/1440. The native 100,000-record
history search rendered in 38 ms. A stale preview caused the first browser
attempt to request obsolete assets; restarting only the owned preview resolved
the failure and all engines passed without a source change.

Its hosted browser, text, image, OCR and GPU workflows passed, as did ARM64
macOS and Windows desktop jobs. Windows passed the exact portable save/restore
scenario. These remain historical source-specific results, not evidence for a
subsequent candidate. Logs and downloaded payloads are retained under
`work/qualification-20260908/candidates/25ea8e95cf479172cfd74e38033cfdb8b9904194/`.

Linux run `34226103601` passed the independent AppImage payload and launch
checks, then failed the installed Debian application's raw-build hash check.
The log records Tauri patching the executable for Debian before producing the
`.deb`, then patching the same executable for RPM and AppImage. The installed
Debian application therefore differs from the final raw executable; its two
helpers match the raw helper hashes. This is an evidence-stage defect, not a
transient runner failure. The first correction attempted to snapshot the target
immediately after Debian bundling, then bind it to the source, run, base receipt
and installer. The later `43a101cf` attempt disproved that snapshot boundary,
as detailed below. Expected payload hashes must never be derived from the
installed or extracted Debian package under test.

Actual Firefox 155.0.1 passed a targeted 200% page-zoom feedback path and an
increased-contrast dialog/navigation path. Display accessibility preferences
were restored and complete before/after accessibility records matched exactly.
However, 200% **text-only** enlargement clipped labels and trace content. The
relevant layout predates `ee99735` and is unchanged during this consolidation;
this failed manual case remains a pre-existing release blocker outside the
attributable-fix scope. VoiceOver caption access timed out again; spoken
acceptance is unverified and VoiceOver was restored off. These observations
must retain their actual observed SHA in the external evidence, not be
relabeled as a new-candidate manual pass.

Debian correction pre-freeze checks passed: 91 Bun tests / 369 assertions,
including 33 focused Linux-stage/collector tests / 107 assertions; TypeScript,
Studio lint, Python compilation, workflow lint and whitespace validation also
passed. Independent review approved the correction. The second Linux step uses
`tauri bundle` against the built executable, avoiding a second compilation.
Final hosted qualification must still prove the split-bundle sequence and
installed payload on Linux.

The completed `25ea8e95` desktop run ultimately passed both macOS architectures
and Windows; Linux retained the Debian-stage failure described above. The next
local candidate `6aba808e` passed production/native/OCR/GPU gates and Chrome/
Firefox acceptance, but WebKit's accessibility test exposed a harness focus
race: it waited for the mobile drawer's closed class, then focused Add pattern
before the application's animation-frame focus return completed. Enter could
therefore reopen navigation instead of opening the dialog. The test now waits
for the required Open navigation focus-return state before proceeding. The
focused WebKit accessibility rerun passed 390/768/1440; the initial failure is
retained under that candidate's `browser-webkit-initial-focus.log`. No application
behavior or timing delay was introduced. A new source commit and fresh final
gates are required for this test-only correction as well.

### Debian producer staging and retained diagnostics after `43a101cf`

Candidate `43a101cf371ea41e24703e244ee4197751eb6ae8` was non-force pushed after
all local gates passed: 91 Bun tests / 369 assertions, TypeScript, Studio lint,
production browser build, Rust formatting/32 tests/Clippy, both runtime builds,
production package/seal/bridge exclusion, four native tests (13 seconds;
100,000-record history rendered in 50 ms), real OCR, wgpu parity and all three
browser/accessibility engines at 390/768/1440. Hosted browser, text, image and
GPU passed, as did both macOS desktop jobs. None of these results qualifies a
later source commit.

Linux's verifier still rejected Debian in run `34231752495`. Downloaded artifact
`10059490942` and exact Tauri CLI v2.11.4 source revealed the omitted behavior:
Tauri saves the original executable, temporarily patches it during each bundle,
then restores the original before the command returns. The post-command target
snapshot was therefore raw (`4763ac…`), not Debian's archived application
(`7d7952…`). The earlier diagnosis based only on patch log ordering was incomplete.
The correct independent snapshot is Debian's actual producer staging directory,
`target/release/bundle/deb/<package_base>/data`, populated before its archive is
created and retained afterward. Capture its unique application and helpers,
not a subsequently extracted installer or the restored target executable.
Keep the source/run/base-receipt/installer binding and exact installed-byte
comparison. The checker now prints captured child-command output before
rethrowing failures; a red/green regression verifies that diagnostics survive.

Hosted OCR failed before inference twice on this candidate: initial run
`34231803091` encountered a rustup `bin/cargo-clippy` conflict; the single allowed
fresh-run retry `34232250581` encountered `bin/cargo-fmt`. Fixture creation and
verified OCR model downloads passed. Local recognition passed, but final-SHA
hosted recognition did not occur. Retain this as a persistent external toolchain
provisioning blocker, not a passing OCR result; no additional same-candidate
retry was made.

Actual Firefox 200% text-only enlargement was repeated on `43a101cf` and failed
again with clipped dossier/trace labels and overlapping topology controls.
Normal zoom segment feedback disabled both segment and overlapping whole-result
controls, and AX live-region text was captured. Zoom/text-only settings were
restored and only the audit tab closed. Evidence is under that candidate's
`firefox-text-200-final.png` and `firefox-feedback-final.txt`. The pre-existing
manual layout blocker and signing/VoiceOver checkpoints remain unresolved.

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

## P0.1 portable libm (2026-09-28)

**Byte-exact native/WASM parity on all three probe cases.** On Donald's
decision, every transcendental in `native/specimen-kernel` now goes through the
`libm` crate (0.2.16, `default-features = false`, so no `arch` intrinsics):
`libm::sin`/`cos` for the visual arrays, `libm::exp` for the atp decay,
`libm::expf` for the sigmoid layer, `libm::pow` for the `^` operator. `sqrt`
and the basic arithmetic stay native because IEEE 754 rounds them exactly.
`cargo tree -i libm` shows it enters only through the kernel.

- `p0-probe.sha256` unchanged (`9577ece5…`): the arithmetic path has no
  transcendental that survives rounding.
- `p0-probe-imagine.sha256` re-blessed from native, `8f99f4ea…` to
  `51ae9bb4…`. That is the digest WASM produced before this change: Rust's
  wasm32 standard library already used the same musl-derived routines, so
  native moved to match the browser.
- `p0-probe-pow.sha256` (new, `142bcfcb…`): `2 ^ 0.5` with a finite decay;
  a native test asserts the result starts `1.41421356`.
- `tests/wasm-probe.test.js`: all three digests match as plain `test`s (the
  former `test.failing` was promoted after being shown red).

Desktop results change in the last bit wherever these functions run. This was
a deliberate re-bless; no stored specimen changes, only newly computed values.
Finding 3 below is resolved by this section. The lane-width caveat (finding 2)
still applies to x86 hosts.

## P0 kernel WASM feasibility (2026-09-28)

**Verdict: GO for building on WASM; NOT byte-identical.** The pure specimen
rules (`native/specimen-kernel`) build for `wasm32-unknown-unknown` and run a
full cycle there. Output matches native byte for byte on the arithmetic path,
but differs in the last bit wherever the engine calls `f64::sin` or a finite
`f64::exp`. Byte-exact cross-target parity is not yet achieved; how to achieve
it is an open decision (finding 3).

Evidence, on aarch64 macOS 27.2, `rustc 1.100.0-nightly (0dfb098f3 2026-08-31)`,
Bun 1.4.0. Two goldens, both recorded from native with `FixedHost`:

- `conformance/p0-probe.sha256` = `9577ece5…a9edcb`: starter specimen, input
  `What is 2 + 2?` (status `complete`, answer `4`, 1 vote, 6 trace steps).
  This path runs no neural layer and no visual trigonometry, and its only
  `exp` underflows to exactly zero (`atp.lastUpdate` 0). Native and WASM
  (`tests/wasm-probe.test.js`) both reproduce it; the WASM test fails against a
  corrupted digest.
- `conformance/p0-probe-imagine.sha256` = `8f99f4ea…15cc313`: starter with
  `atp.lastUpdate` 61.234 s before the fixed clock, input
  `imagine a happy blue square`. It reaches `f64::sin`/`cos` (visual arrays),
  the sigmoid layer (`f32::exp`) and a finite `f64::exp` decay; a native test
  asserts that coverage. WASM produces `51ae9bb4…fe45b9`. The Bun test is
  `test.failing` so the gate records the divergence instead of hiding it; it
  turns red once the digests match and must then become a plain `test`.
- Probe size, release, no `wasm-opt`: 1,598,696 bytes raw, 529,256 bytes
  `gzip -9`. Rebuild of the kernel and probe with dependencies cached: 2.2 s
  wall. A fully cold dependency build was not measured.

Findings:

1. **JSON map ordering diverged between editions (fixed).** The pinned
   `abi-wdbx`/`abi-foundation` enable `serde_json/preserve_order`, so the
   desktop engine serialized JSON objects in insertion order while a kernel
   built without `specimen-core` (the browser) serialized them sorted. The
   engine iterates no JSON objects, so only serialized bytes were affected.
   The first golden (`05ae6c2f…`) was recorded in that non-shipping
   configuration. The kernel now enables `preserve_order` itself; every build
   set produces `9577ece5…`.
2. **Lane width is a parity input.** `abi-compute` dot products accumulate in
   4 lanes for both NEON and scalar (the WASM path). The imagine case's `f32`
   synthesis values match between native and WASM on this host, which is the
   evidence for this host only. An x86_64 desktop with AVX2 (8 lanes) or
   AVX-512 (16 lanes) sums in a different order and may differ; not measured,
   no x86 host was available.
3. **`f64::sin` and `f64::exp` differ in the last bit (open).** Reproduced by
   the whole-branch review and by the imagine golden: `cycle.visual.yArray[22]`
   `6.083365791431938` native vs `6.08336579143194` WASM (`sin`), and
   `atp.valence` `0.2823271779908061` vs `0.28232717799080614` (`exp`).
   `f64::cos` and every `f32` synthesis value matched. Options for P1, not
   decided here: route kernel transcendentals through one portable `libm`
   implementation on both targets (byte-exact; changes native values once,
   deliberately re-blessed), or define an explicit last-bit tolerance for
   cross-edition comparison. Existing exact comparisons are not loosened.

Not exercised: the browser Worker running WASM (the probe ran under Bun only),
wasm-bindgen, IndexedDB storage, and any x86 or Linux host. Vision and model
tooling remain native-only by design.

## P2 shell redesign (2026-09-29)

The studio moved from one 3,238-line component to a shell (`app/shell/`),
one module per view (`app/panels/`), dialogs (`app/dialogs/`) and a state
hook (`app/state/use-studio.ts`), with a dark graphite + teal default theme
and the previous white/teal look as the light theme. All colors come from
`app/theme.css`.

Evidence on aarch64 macOS 27.2, Chrome via Playwright, Bun 1.4.0, against a
freshly built `bun start` preview:

- `bun run test:browser` (default dark theme): browser acceptance passed,
  including the new command-palette, side-pane chapter and per-pane search
  steps; accessibility passed at 390, 768 and 1440 px with new
  `activity-dock-open` (all widths) and `split-studio-nodes` (1440) audits.
- `STUDIO_THEME=light bun run test:browser`: same result in the light theme.
- Both accessibility receipts pass `scripts/qualification-summary.py`'s
  `validate_accessibility_widths`.
- `bun run check` (116 Bun tests and the Worker build), `bun run test:native`
  and `bun run check:native` exit 0.

Findings from the whole-branch review, fixed with tests that failed first: both
panes shared one search string; a chapter clicked in a side-pane
Specification replaced the primary view and closed the split; the activity
dock failed axe (`listitem`, `scrollable-region-focusable`); four named
`white` colors had escaped the token migration because the no-literal test
only matched hex and function notations.

Findings during the work: the kit's `CommandDialog` rendered without a cmdk
root (runtime crash on open) and placed its title outside the dialog; the
palette now composes the parts itself. The light theme failed axe contrast on
the sidebar group labels (teal on dark green, 2.54:1) until the sidebar got
its own `--sidebar-accent` and muted tokens.

Not exercised: Firefox and WebKit engines, the native desktop UI suite
(`native.e2e.mjs`), a packaged Tauri build, and screen readers.

## B0 Bun runtime spike (2026-09-29)

Scratch worktree at the P2 head, Bun 1.4.0, Node 26.10.0, aarch64 macOS 27.2.

| Step | Under Bun (`bun --bun`) | Evidence |
|---|---|---|
| `vinext build` (web) | works | exit 0; same 76 output files and identical prerendered HTML size as the Node build (only build IDs and hashes differ); 6.5 s vs 6.2 s |
| `vite build`, desktop config | works | exit 0; warm 1.0-1.15 s under both |
| `vite` desktop dev server | works | page hydrates (`Stored on this device`), dark theme applied |
| `vite` web dev server (vinext + Cloudflare plugin) | fails | page never hydrates: `Failed to fetch dynamically imported module .../virtual:vinext-app-browser-entry`; Node serves that module (200) and hydrates |
| `wrangler dev` (Worker preview) | fails | prints "Ready", but three requests each time out at 20 s; Node wrangler serves the same Bun-built `dist/` (200) |
| Playwright e2e (both suites) | works | browser and accessibility acceptance pass against a Node-served preview |
| Tauri CLI `build --debug --bundles app` | works | `WDBX Specimen Studio.app` bundled in 23 s (warm cache); `beforeBuildCommand` ran the Bun desktop build |

Adopted in `package.json`: `build`, `desktop:dev`, `desktop:build`,
`desktop:package`, `test:browser` on Bun; `dev` and `start` stay on Node, so
`engines` lists both and the browser CI workflow keeps `setup-node`. The
desktop workflow's `tauri build`/`bundle` steps use `bunx --bun`. Not
exercised under Bun: `tauri dev` (interactive), the WebdriverIO native suite,
Linux/Windows hosts.

After the switch, in the canonical checkout: `bun run check` (117 tests, Worker
build) exit 0, `bun run test:native` exit 0, `bun run desktop:build` exit 0,
`bun run test:browser` passes in the dark default and `STUDIO_THEME=light`.

## B2 kernel parity and WASM browser engine (2026-09-29)

The browser now runs the Rust kernel as WebAssembly; `lib/specimen/engine.ts`
(1,640 lines) and its tests are deleted. Edits moved into
`native/specimen-kernel/src/mutate.rs`; both editions use them through the
same WASM, and desktop keeps native cycles, reviews and maintenance.

Evidence (aarch64 macOS 27.2, Bun 1.4.0, Chrome via Playwright):

- `cargo test` for the kernel (10 unit + 13 mutation tests), `specimen-wasm`
  (12, including the three unchanged conformance goldens) and `specimen-core`;
  `bun run check:native` exit 0.
- `bun run check` exit 0: 111 Bun tests, including 18 facade tests in
  `tests/kernel.test.js` that load the built WASM.
- `bun run test:browser` passes in dark and `STUDIO_THEME=light`, including the
  pinned Processing / Cancelled / Failed / Completed trace sequence, which now
  runs through the worker behind a cooperative checkpoint.
- `tauri build --debug --bundles app` bundles the WASM and worker.

Findings:

1. **Native `&current_input` returned the lowercased clause** instead of the
   original text the spec requires (ch. 8). Fixed in the kernel with a test
   that failed first; affects desktop too. Goldens unchanged.
2. **vinext rewrites `import.meta.url` to a file URL** in client chunks, so
   `new URL('./worker', import.meta.url)` could not start a worker. The worker
   now loads through Vite's `?worker&url`.
3. **The provenance evidence block overflowed at 390 px** with native vote
   fields; it is now a focusable, labelled scroll region (axe).

Semantic changes adopted (native semantics won; tests rewritten to match):
Pattern IDs, feedback RNG, Type A review, settings bounds, single-operation
arithmetic binding, clause splitting on `then`/`and`/`but`, and the trace
phase list (see `RUNTIME-PROFILE.md`).

Not exercised: the desktop webview loading the WASM under the new CSP (the
native UI suite was not run), Firefox and WebKit.

Whole-branch review fixes (each with a test that failed first):

- **Desktop CSP blocked the WASM fetch** (`connect-src` lacked `'self'`, and
  `default-src` does not apply once `connect-src` is present): every desktop
  edit would have failed. `tests/desktop-config.test.js` pins the directives.
- **The advanced entry editor could store a node with no entries**, which
  made the workspace unloadable on the next start. `add_node` now rejects it.
- **A failure inside the worker left the run in Processing forever.** The
  worker reports errors as kernel errors and restarts its instance;
  `runLong` rejects on worker errors; a trapped main-thread instance is
  replaced and long commands wait for it (`tests/kernel-worker.test.js`
  exercises a real Worker).
- **TypeScript-era saves could load into uneditable or unloadable state.**
  Ruling: migration runs before validation (for loads and imports); it clamps
  settings to native bounds and splits entries that no longer share their
  node's native Pattern ID into new nodes (`<name> · 2`), keeping every entry;
  memory links follow the first node that carried the old ID. All repairs are
  logged. Cost if wrong: a user sees an extra node per split class.
- **Record checks the browser engine had were missing** (ATP bounds,
  self-attachments and affinity, node strength, entry/Pattern ID agreement,
  memory links, history status, visual arrays). Ruling: they run at the
  import boundary (`mutate::validate_records`); `engine::validate`, which also
  guards every desktop edit, is unchanged so existing desktop stores keep
  loading. Cost if wrong: a malformed desktop store is caught later.

## B3 live engine views (2026-09-29)

New Observe view `?view=engine` ("Live engine", `app/panels/engine-panel.tsx`)
over pure derivations in `lib/specimen/insights.ts`: the running trace as a
phase timeline (repeated Index Rafts checkpoints fold into one row with a
coverage `<progress>`), per-node votes against the vote threshold, the recent
cycle series (confidence, duration, matched), and ATP valence and intensity as
native `<meter>` elements decayed with the kernel's `exp(-elapsed_s/180)`.
Every chart has a paired data table behind "Show data".

Evidence (aarch64 macOS 27.2, Bun 1.4.0, Chrome via Playwright):

- `bun run check` exit 0: 135 Bun tests, including `tests/insights.test.js`
  (5) and `tests/engine-panel.test.js` (server-rendered markup).
- `bun run test:browser` passes in dark and `STUDIO_THEME=light` on a freshly
  rebuilt preview, including the new live-engine step and a `live-engine` axe
  state at 390, 768 and 1440 pixels.

Not exercised: the view inside the desktop webview, Firefox and WebKit.
