# Validation record

## Release-candidate acceptance ledger

Status: **qualification in progress; no signed release candidate is certified**.
The candidate source commit is not frozen. Local results below concern the
working changes based on `09a65e32f2366926e36225871685a43e56cd380b`; a commit
identifier alone does not identify these uncommitted changes.

| Requirement | Gate / platform | Current result | Evidence / unresolved dependency |
| --- | --- | --- | --- |
| Browser engine, imports and UI contracts | Canonical `bun test`, local macOS | In progress | Scoped Bun discovery and expanded negation regression matrix; final counts recorded after review |
| Browser types / lint / production build | `bunx tsc --noEmit`, `bun run lint:studio`, `bun run build` | In progress | Final working-tree verification follows source review |
| Browser interaction and accessibility | `bun run test:browser`, Chrome / Firefox / WebKit | In progress | `work/browser-checks-*.json`, `work/accessibility-checks-*.json`; candidate SHA qualification pending |
| Native conformance baseline | `cargo test -p specimen-core --locked`, Apple silicon | Passed baseline: 11 unit + 17 conformance, one GPU ignored | Baseline before new native regressions; not a final candidate receipt |
| Native Clippy baseline | `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed baseline | Dependency future-incompatibility notices remain |
| Native UI | Instrumented WebdriverIO, four desktop targets | Local run in progress; hosted matrix pending | Instrumented tests do not qualify the final signed payload |
| Real OCR | Pinned detection/recognition models and HELLO WORLD fixture | Local run in progress | Need actual OCR output and source-bound receipt |
| Real text inference | Qwen3-4B, macOS Intel reference | Historical success only | [Run 33878636044](https://github.com/donaldfilimon/wdbx-specimen-studio/actions/runs/33878636044), source `b2c382d`; desktop job skipped |
| Real image inference | SDXL-Turbo, macOS reference | Historical success only | [Run 33878632925](https://github.com/donaldfilimon/wdbx-specimen-studio/actions/runs/33878632925), source `b2c382d`; new source requires qualification |
| Accelerator | Apple silicon wgpu, tolerance 0.0001 | Passed local parity | Measured max delta 0.00000012; final source-bound receipt pending |
| Installed helper portability | All four desktop targets | Pending candidate matrix | Must start/stop packaged helpers; startup is not inference qualification |
| macOS signed DMGs | Developer ID, notarization, stapling, final payload | Blocked | No local Developer ID Application identity and repository secret inventory empty |
| Windows signed installer | Authenticode and installed payload hashes | Blocked | Repository signing secrets absent; historical runs do not qualify new source |
| Linux packages | AppImage / Debian installed launch and helpers | Pending | Requires Linux runner and candidate artifacts |
| Upgrade / uninstall | Prior package and isolated application data | Pending | Establish prior-package baseline before claiming preservation |
| Manual accessibility | Safari, Firefox, VoiceOver, physical touch, enlarged text | Pending | Automated WebKit is not Safari or physical-device acceptance |
| Optional WebMCP | Real `document.modelContext` registration / invocation | Unsupported in tested headless browsers | Graphical fallback remains available; native API execution unqualified |

Evidence must name the source SHA, successful workflow run, runner and artifact
digest. Signing or repackaging requires a new final-payload digest and installed
verification. Changed source invalidates affected candidate gates. Release
notes and execution status are in `RELEASE-CANDIDATE.md`; this ledger is the
acceptance authority. Publishing and deployment are excluded.

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
