# Abbey concept-film renderer review — 2026-10-03

## Scope and baseline

**Standards: PASS. Spec: PASS. No open findings.**
The manifest-isolation defect below is closed after source, regression and
artifact re-review. Delivered media and historical render provenance remain
unchanged. Human audiovisual acceptance remains Partial.

Reviewed WebPress `examples/abbey-trailer/render.py`, `RENDERING.md`, the
original `output/renderer-review.diff`, the metadata follow-up patch
`/tmp/webpress-trailer-metadata-fix.patch`, the current manifest/reports and
`/tmp/webpress-trailer-metadata-fix-receipt.json`. The original renderer is new
source in this slice; the metadata follow-up is included in this assessment.
Closure also covers `/tmp/webpress-trailer-output-scope-fix.patch`, its receipt,
the final retained `test_render.py`, and the subsequent documented test command
and generated metadata refresh.

- Current renderer SHA-256:
  `038c6fa772d55e9316b7905a26c9a8f128bdefd3b87eff9446c1b7e7811332c6`
- Historical render-producing source SHA-256:
  `d61f22f882782126e806d175fac755f4dff86667fd06218f0f16e3cb48193ccf`
- Metadata patch SHA-256:
  `0f91fd6ec34aae0380206452bd8aaf4e254d9f1bedeed97d96cb4c3ef03fca7e`
- Output-isolation patch SHA-256:
  `fa1e2e0c997831e01d18d92380d1f2f7bc0810741c7ec81bd689c2e1303c94e3`
- Final `RENDERING.md` SHA-256:
  `947159d57d0df6ace1fcf5369cfc9515d4a4a551acdd6d485ff5f547e735e689`
- Retained `test_render.py` SHA-256:
  `f5c0064a5be3297c0941c51bb8e29b5d75eb42e08218b68a09635f2a14d799d0`

Standards sources: WebPress `AGENTS.md`, the supplied operator instructions,
existing example conventions and the code-review skill's heuristic baseline.
Spec source: the assigned local-render/media-validation scope and `RENDERING.md`.
A separate read-only agent performed the original Spec assessment. The closure
was reassessed separately against Standards and Spec by this reviewer; all four
agent slots were occupied. No product/example source was changed by the
reviewers, and no film or broad gate was rerun.

## Standards

### STD-1 — Isolate the manifest with its delivery output

- **Severity:** Medium; correctness/provenance, not a style preference.
- **File:line:** Original finding at `webpress/examples/abbey-trailer/render.py:413`,
  `:84` and `:107`; repaired at current `render.py:82`, `:99` and `:426`.
- **Description (original):** Rendering honors `--output` for media and reports but always
  writes the shared `ROOT/asset-manifest.json`. Metadata refresh also always
  reads/writes that shared manifest. Running the documented reproduction into
  `output-reproduction` therefore overwrites the original delivery's manifest
  and renderer provenance. The source hash already differs between the
  historical renderer and current metadata tool, so provenance changes even
  when reproduced media bytes happen to match. If media bytes differ in another
  environment, default `--verify-only` can subsequently compare the original
  media against the reproduction's hashes and reject the wrong delivery.
- **Suggestion:** Store/resolve a manifest per output directory. Retain an
  explicit compatibility path for the existing default delivery without
  letting alternate outputs replace its manifest. Cover alternate-output
  render, verification and refresh with a scratch regression asserting the
  original manifest and reports remain unchanged.
- **Status:** Closed. `manifest_for_output` resolves the original `output/`
  directory to the legacy root manifest and all other deliveries to their own
  manifest. Rendering and refresh use the same resolver. `output_hashes`
  excludes the manifest and checksum list, avoiding a self-referential hash and
  preserving idempotence when the manifest is inside an alternate output.

Reviewer confirmation used disposable scratch input/output paths and the real
`main()` routing, with synthesis, encoding, media validation and formatting
stubbed. A separate output directory was created and the original media stayed
unchanged, while the shared manifest was overwritten and the historical renderer
field replaced. This is a path/metadata reproduction only; it is not evidence
of a new media render. All scratch data was removed afterward.

No other actionable correctness or baseline-smell finding was established.
The existing renderer bounds the frame/sample counts and codec thread count,
streams screenshots directly to FFmpeg and handles capture/encoder failures.
This review does not claim a hard wall-clock execution deadline: subprocess
waits do not currently impose one.

## Spec

### SPEC-1 — The documented reproduction does not preserve earlier provenance

- **Severity:** Medium.
- **File:line:** Original finding at `webpress/examples/abbey-trailer/render.py:413`
  and `RENDERING.md:24`, `:27`; repaired at current `render.py:82`, `:392`, `:426`
  and `RENDERING.md:28`, `:48`.
- **Description (original):** The requirement for truthful per-delivery provenance and the
  statement that reproduction cannot overwrite an earlier delivery are not
  satisfied by the shared manifest path. A reproduction writes fresh report
  and media hashes into the original delivery's manifest despite receiving a
  separate output directory. Metadata refresh/verification then resolve that
  same shared file, rather than the selected delivery's manifest.
- **Suggestion:** Bind manifests to the selected output, including refresh and
  verification. Preserve the historical delivery's manifest explicitly during
  any compatibility migration.
- **Status:** Closed. Alternate render, metadata refresh and verification all
  resolve the selected delivery's manifest. Verification still delegates to
  `refresh_metadata(output)` after media checks. The revised documentation
  describes the compatibility location and selected-output behavior accurately.
  The original independent Spec review confirmed the same underlying defect
  as STD-1; these are separate axis dispositions, not two independent bugs.

The remaining scoped requirements are implemented: fixed frame sampling,
local-file source capture, explicit technical validation, immutable authored
input comparison, original score synthesis and separate historical-renderer
versus metadata-tool hashes. `write_json` compares parsed JSON semantics before
publishing formatted metadata. Current `refresh_metadata` verifies manifest
MP4/WAV hashes before reformatting reports.

## Closure and durable regression coverage

- **Current:** `webpress/examples/abbey-trailer/test_render.py:35`, `:62` and
  `:83` retain the three regressions in the repository: alternate render cannot
  replace the original manifest; alternate refresh preserves provenance and
  remains byte-idempotent; default output retains the public legacy manifest
  location. The source path is relative to the test, so it is portable across
  checkout locations. Real manifest/checksum writes and Biome formatting are
  exercised in temporary directories; media production, decoding and version
  probes are explicitly stubbed.
- **Current:** `RENDERING.md:54` documents the Python 3.11+ command and installed
  Biome requirement. Retaining this targeted runner is appropriate for the
  isolated example; adding FFmpeg or Python execution to the product gate is
  unnecessary. The earlier concern that the regression existed only under
  `/tmp` is resolved.
- **Evidence:** `/tmp/webpress-trailer-output-scope-receipt.json` records the
  scratch regression failing twice before the repair and passing three tests
  afterward. `/tmp/webpress-trailer-durable-regression-20261003.log` records all
  three retained tests passing; the coordinator reports process exit 0. The
  reviewer read the final test source and that log without rerunning media.
- **Evidence:** The output-isolation receipt predates the final test-runner
  documentation. The reviewer independently verified the final documentation
  hash above against `metadata_tool.documentation_sha256` in the manifest,
  render report and verification report, and verified the current renderer
  hash against each `metadata_tool.sha256`. Historical `renderer_sha256` fields
  were preserved.

## Verified evidence and boundaries

- Final reviewer hashing found zero mismatches across all four authored inputs and
  all 15 manifest-listed outputs. Report before/after input hashes equal the
  manifest's input hashes. The checksum list matches the output manifest, and
  both MP4 files and the score WAV still match their pre-repair hashes.
- The manifest and render report retain the historical renderer SHA above.
  The separate metadata-tool SHA equals the currently inspected `render.py`.
- Current reports record 900 frames/30 seconds and 450 frames/15 seconds,
  1920×1080 video, H.264/yuv420p and stereo 48 kHz AAC. Both full-decode receipts
  have exit code 0. No decode was rerun by the reviewer.
- Capture evidence records no page errors or external requests and a matching
  repeated transition frame. This supports the recorded local sample; it does
  not establish cross-platform byte identity or every-frame reproducibility.
- The metadata follow-up receipt records semantic preservation, byte-idempotent
  refresh, Python syntax success, Biome exit 0, unchanged MP4/WAV hashes and
  full verification exit 0. Reviewer hash checks independently confirm the
  currently listed outputs.
- **Partial:** human listening and complete real-time audiovisual approval
  remain unperformed. The coordinator's five-beat image inspection is sampled
  visual evidence.
- **Out of scope:** WebPress product qualification, live Abbey capability
  demonstrations, remote publication, commits and pushes.

**Review totals:** Standards: 1 Medium finding closed, 0 open. Spec: 1 Medium
finding closed, 0 open. Both concerned the same manifest-isolation defect;
current delivered media remains supported by its retained technical evidence.
