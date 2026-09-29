# Release candidate execution record

Approved target: browser and native 0.2 profiles; macOS ARM64/Intel,
Windows x64 and Linux x64 candidates. This task owns final consolidation,
qualification, and the authorized non-force push to `github/main`. The custom
`origin` remains unchanged and is not a publication target.
Reference macOS runners qualify real text/image inference; other platforms
qualify helper startup and portability. Explicit architecture extensions remain
outside this release.

## Progress

- Consolidated baseline: canonical main at
  `6d76b0c0c08353ee5537d8c006b38038337b9682`; existing Studio, engine and hosted
  packaging histories are preserved. Previous implementation tasks are inactive.
- Fresh local baseline: 58 Bun tests / 262 assertions, TypeScript, Studio lint,
  production browser build, Rust formatting, and 13 unit + 18 conformance tests
  passed. The normal Rust run ignores one GPU test.
- Explicit GPU test passed on wgpu: 32 outputs, maximum delta 0.00000012,
  required threshold 0.0001.
- Workspace all-target Clippy passed; dependency future
  incompatibility notices remain separate from warnings-denied gate result.
- Evidence-collector consolidation through `1fdbd05`: 78 Bun tests / 338
  assertions passed, with workflow lint and Python compilation. Final-SHA
  hosted qualification remains pending; the collector rejects incomplete
  desktop/browser matrices and retains partial evidence without certifying it.
- Hosted run 33878636044 at b2c382d qualified text-model only; desktop skipped.
- Credential inventory: GitHub repository secret list empty; no local Developer
  ID Application identity. Signed release candidates require credentials.
- Candidate `720125b` was non-force pushed and qualified browser, text, image,
  GPU and both macOS desktop jobs. Linux AppImage qualification exposed an
  incorrect raw-versus-packaged ELF hash comparison. The narrow staging-evidence
  correction requires a new candidate and complete hosted matrix; retain the
  superseded run and local results as history, not final qualification.
- Replacement `25ea8e95` passed all local gates, hosted browser/text/image/OCR/
  GPU, ARM64 macOS desktop and Windows desktop, including exact portable
  save/restore. Linux's AppImage stage passed, but Debian qualification found
  that the final raw executable does not preserve Debian's bundle-specific
  bytes. The first post-bundle target snapshot was also insufficient: Tauri
  restores the original before returning. Candidate `43a101cf` established
  that the independent snapshot must come from Debian's producer `data`
  staging tree. This correction establishes another candidate.
- Actual Firefox 200% text-only enlargement failed with clipped labels and
  trace content. The layout predates this consolidation and is unchanged here;
  retain the observed failure as a pre-existing manual release blocker. Targeted
  page zoom and increased contrast passed, with settings restored exactly;
  the full manual matrix and spoken VoiceOver acceptance remain incomplete.
- Candidate `43a101cf` passed all local gates and hosted browser/text/image/GPU
  qualification. Its hosted OCR setup failed on rustup component-file conflicts
  twice, including the single permitted retry, before inference; local OCR
  remains separate. Preserve this external blocker and all superseded failures.

## Decisions

Use the canonical main checkout as explicitly approved. Root controls final
integration; any delegated task owns only its stated files. Preserve concurrent
work and do not change ABI/Abbey/WDBX siblings.
Historical hosted successes do not qualify a new candidate source commit.
Manual physical touch, VoiceOver and unavailable platform acceptance remain
unresolved until actually exercised. No fabricated acceptance or receipts.

## Frozen source and external receipt

Commit the procedure and known baseline results before freezing the candidate.
After the final non-force GitHub push, every qualifying run and payload receipt
must identify that exact 40-character source SHA. Record final run URLs, artifact
hashes, platform conclusions, local/manual evidence, and credential blockers in
the qualification-summary Actions artifact. Do not make a subsequent documentation
commit merely to add those results: that would change the source being qualified.
Any corrective source commit creates a new candidate and requires fresh affected
local gates plus the complete required final-SHA hosted matrix.

Missing Windows PFX secrets and macOS Developer ID Application/notarytool
credentials block signing, not the independent browser, native, OCR, inference,
GPU and ad-hoc packaging layers. No signed release is certified until all signing,
notarization, installed-payload and manual requirements have actual evidence.
