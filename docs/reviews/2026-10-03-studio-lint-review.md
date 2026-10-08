# Specimen Studio lint closure review — 2026-10-03

## Scope and baseline

- **Current:** reviewed the 15 tracked Studio files and three new regression
  files in `/tmp/wdbx-studio-lint-closure.patch`, against WDBX HEAD
  `e45410356ceb4e96f0dc26a83b02e1c18cf94860`.
- Patch SHA-256:
  `9ad985128e31e94bd0f1bd3d032a13f1f79136d635a5f9a67e00306307ec44a4`.
- `git apply --reverse --check /tmp/wdbx-studio-lint-closure.patch` exited 0,
  confirming the supplied patch matches the inspected working files.
- Acceptance: repair the existing 26 broader lint failures while preserving
  component APIs, ref targets and layout; no production dependencies, generated
  changes or native implementation changes in the Studio slice.
- Standards sources: the supplied machine/repository instructions,
  `specimen-studio/AGENTS.md`, the documented component and test contracts, and
  the code-review skill's heuristic smell baseline. Spec source: the explicit
  scoped acceptance above and Tasks 2 and 7 of
  `docs/superpowers/plans/2026-10-03-ecosystem-parallel-completion.md`.
  No issue-tracker configuration was found; the supplied scope is sufficient for
  this review and no external issue was assumed.
- Standards and Spec were assessed independently; a separate read-only agent
  performed the Spec assessment. No implementation files were changed by this
  review, and no broad gate was rerun.

## Standards

**Verdict: PASS with one nonblocking automation observation.** No blocking
correctness issue, documented-standard violation or actionable baseline smell
was found in the scoped source changes.

### STD-1 — Retain the new interaction checks in routine browser automation

- **Severity:** Low; quality observation, not a hard standards violation.
- **File:line:** `specimen-studio/tests/ui-components.e2e.mjs:1`;
  `specimen-studio/package.json:17`;
  `.github/workflows/specimen-studio-browser.yml:82`.
- **Description:** The new media-query, addon-focus and carousel-subscription
  checks run successfully as a standalone script, but `test:browser` invokes
  only `browser.e2e.mjs` and `accessibility.e2e.mjs`. The browser CI workflow
  invokes that script. A later regression in these isolated components can
  therefore pass the routine browser job without executing the new checks.
  The static `ui-components.test.js` tests are included in `bun test`; the gap
  concerns only the new interaction script.
- **Suggestion:** Append `bun tests/ui-components.e2e.mjs` to `test:browser`, or
  invoke it explicitly in the existing browser job, and run the resulting
  command once.
- **Status:** Open at this reviewed baseline; coordinator has accepted the
  follow-up and owns the source edit.

Correctness observations supporting this verdict:

- `specimen-studio/components/ui/carousel.tsx:61` subscribes to both `select`
  and `reInit`, removes each exact callback in cleanup, and updates the
  subscription when the API instance changes. The bitmask snapshot is a stable
  primitive, avoiding unstable-object snapshots and render loops.
- `specimen-studio/hooks/use-mobile.ts:5` pairs media-query registration and
  cleanup on the same query object. The snapshot is Boolean and the server
  snapshot remains deterministic without accessing `window`.
- `specimen-studio/components/ui/input-group.tsx:61` preserves focus on the
  listed interactive descendants and delegates decoration clicks to an enabled
  input or textarea without adding another tab stop.
- The targeted ARIA exceptions preserve existing DOM/ref contracts, document
  their local rationale and do not disable lint across whole files. Explicit
  label/pagination children preserve prop forwarding; chart key selection
  handles scalar values without coercing function data keys into config names.
- The existing evidence tests retain their mutations and assertions; their
  tuple annotations and unused-binding removal do not alter behavior. No
  production dependency, lock, `unwrap()` or unnecessary clone was introduced.

## Spec

**Verdict: PASS for the narrow lint and interaction source scope. Zero blocking
or nonblocking source findings.**

- The 15 tracked changes and three regression files match the assigned scope.
  Public prop types, ref targets, classes and layout structure are preserved.
  The addon extends its existing focus delegation to textarea and listed
  interactive children; no additional runtime subsystem is introduced.
- Carousel state follows its external API using primitive snapshots, and both
  event subscriptions are cleaned up
  (`specimen-studio/components/ui/carousel.tsx:61`).
- The mobile hook retains deterministic server output
  (`specimen-studio/hooks/use-mobile.ts:15`).
- No production dependency, generated file or Studio native source change is
  present in the reviewed patch.

**Evidence boundary:** the isolated component browser fixture replaces Embla
with an event stub at `specimen-studio/tests/ui-components.e2e.mjs:28`. Its
passing result verifies callback registration, state changes and cleanup, not
real carousel dragging or layout. Native application qualification and the
broader ecosystem completion plan remain separate from this scoped pass.

## Validation evidence inspected

| Evidence | Observed result and boundary |
| --- | --- |
| `/tmp/wdbx-studio-check-20261003.log` | Browser gate log: 190 tests passed, 0 failed; production build completed. This gate runs `lint:studio`, not broader `lint`. |
| `/tmp/wdbx-studio-lint-final.log` | Separate broader lint invocation, `$ oxlint`, with no diagnostics. Coordinator reports successful completion; this text log does not itself encode the process exit status. |
| `/tmp/wdbx-studio-focused.log` | 57 focused tests passed, 0 failed, across four files, including the six new static component cases. |
| `/tmp/wdbx-studio-browser-final.log` | Chrome core workflows passed for desktop and mobile with zero runtime errors; accessibility checks passed at 390, 768 and 1440 pixels. |
| `/tmp/wdbx-studio-ui-regression-after.log` | Isolated component browser regressions passed. Embla is stubbed as described above. |
| Reviewer `git diff --check -- specimen-studio` | Exit 0. |
| Reviewer reverse patch applicability check | Exit 0; source matches supplied patch. |

## Appendix: ABI public Core ML fixture write

**Standards verdict: PASS. Spec verdict: PASS. Zero findings.** Scope is only
the removal of `.completeFileProtection` from the temporary fixture write at
`/Users/donaldfilimon/dev/active/abi/crates/abi-gpu/native/metal_dot.swift:346`.

- **Severity:** None.
- **File:line:** `abi/crates/abi-gpu/native/metal_dot.swift:346`.
- **Description:** The write continues to use `.atomic`, a fresh UUID temporary
  pathname and deferred cleanup. Its input is the deterministic model generated
  by `abi/crates/abi-gpu/build.rs:121`, implementing `output = 2 * input + 1`.
  The change does not touch a user model, store, credential or persisted runtime
  data. Compile, load, prediction and failure handling are unchanged. The
  temporary public fixture does not need the removed protection option to
  satisfy the probe's correctness contract.
- **Suggestion:** None for the scoped change.
- **Status:** Accepted. The coordinator's isolated reproduction reports
  Foundation 513/POSIX EPERM with complete protection and success using atomic
  publication alone; that reproduction was not rerun by this reviewer.

The inspected `/tmp/abi-20261003-coreml-fixed.log` records 21 passing GPU tests,
including Core ML inference and evidence-boundary tests. The inspected full
`/tmp/abi-20261003-ecosystem-gate-fixed.log` ends with `check: all green`.
Reviewer parsing confirms 987 Rust test executions passed, 0 failed and 0
ignored, plus 144 Python tests. Its SHA-256 is
`929ada1a26f0c50cbe7a58ee8fb29dd094885a2f00ee432b63cf3a7a5943d9aa`, matching
`docs/verification/2026-10-03-ecosystem-local-evidence.json`, which records
gate exit 0. This is local qualification under the requested Core ML compute
policy; it does not establish ANE residency or remote release acceptance.

**Review totals:** Standards: one Low automation observation, no blocking
findings. Spec: zero findings. ABI appendix: zero findings on both axes.

## Final appendix: Studio automation closure

**Current verdict: Standards PASS; Spec PASS. Zero open findings in this
reviewed Studio slice.** This appendix supersedes STD-1's open status and the
earlier review totals; the original entries retain their baseline context.

- **Severity:** Low observation, now resolved.
- **File:line:** `specimen-studio/package.json:17`;
  `.github/workflows/specimen-studio-browser.yml:82`.
- **Description:** The current `test:browser` command appends
  `&& bun tests/ui-components.e2e.mjs`. The existing browser CI step invokes
  `bun run test:browser`, so the interaction checks now participate in routine
  browser automation and their failure propagates through the command chain.
  The package diff contains only this script addition.
- **Suggestion:** None remaining for STD-1.
- **Status:** Closed after source and receipt inspection. No source files were
  changed by this re-review.

The coordinator supplied direct command receipts recording exit 0 for
`bun tests/ui-components.e2e.mjs` and the separate broader `bun run lint`.
The inspected `/tmp/wdbx-studio-ui-gate-final.log` contains
`UI component browser regressions passed`; the inspected
`/tmp/wdbx-studio-root-lint-20261003.log` contains only `$ oxlint`, with no lint
diagnostics. Empty direct-command stdout is expected when output is redirected
to these logs. The exit statuses come from the coordinator's direct execution
receipts, rather than being inferred from empty output or encoded in the text
logs themselves.

The appended command was run separately; this re-review did not rerun the full
combined browser chain or a broad gate. The earlier core-browser and
accessibility receipts remain distinct from this new component receipt.
The Embla-stub limitation still applies. Native tests and native Clippy remain
separate qualification evidence and are not closed by this appendix.
