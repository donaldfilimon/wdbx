# Quesar brand package review — 2026-10-03

**Standards: PASS. Spec: PASS. No actionable findings.**

## Scope and baseline

Reviewed the frozen new offline packager and tests, pinned input manifest,
distribution README/guidelines and actual archive in the canonical
`/Users/donaldfilimon/dev/active/quesar.cloud` checkout. The supplied scoped patch
is the baseline for this review; unrelated dirty work is excluded.

- HEAD: `ac8c24afe618f34a7ed46ea0baed477133064c40`.
- Scoped patch: `notes/verification/2026-10-03-brand-package-scoped.patch`,
  SHA-256 `23d916f3f0feb71bff35f9fc9d576e14389fde505ad49528536fab0e1037f017`.
- Snapshot: `notes/verification/2026-10-03-brand-package-source-snapshot.json`,
  SHA-256 `985697347db4dc06de144f2d4c14ca96947f837da74d77f8c22a927eeeb87ebd`.
- Archive: `notes/verification/brand-distribution-2026-10-03.tar.gz`,
  **842,867 bytes**, SHA-256
  `73b779ac79c37b2e5a0e48cd25d6b8db253af824f00890ea43456b20ca1b7f60`.

Standards sources were the operator/repository instructions, the repo command
and architecture guidance, and the mattpocock code-review heuristic baseline.
Spec sources were the assigned scope and the source-backed local-package
contract in the distribution README/guidelines and acceptance note. Standards
and Spec were assessed in separate passes by this reviewer because all four
agent slots were occupied. No source, archive, film or product output was changed
by the reviewer; no broad gate or media generation was rerun.

## Standards

**Disposition: PASS; 0 findings.** No documented-standard breach or actionable
baseline smell was established. Relevant correctness checks:

- `scripts/package-brand.ts:25` and `:38` reject ambiguous/traversing paths,
  symlink sources and mismatched sizes/hashes. Every copied byte is pinned.
  `:174` rejects case-insensitive duplicate names and file/directory conflicts,
  including reserved generated names. `:249` creates the output exclusively,
  preserving an existing archive on rerun.
- `scripts/package-brand.ts:128` emits sorted regular ustar entries with fixed
  mode, owners and timestamps. Path/field limits fail explicitly. `:157` and
  `:237` bound metadata, file counts and final payload. The implementation uses
  installed TypeScript and Node facilities without adding a dependency.
- `scripts/package-brand.ts:57` exports the pinned component's literal SVG
  geometry without evaluating JSX. The scoped allowlist rejects dynamic
  attributes/children and unexpected tags. No image tracing, font conversion or
  new identity geometry is introduced.
- `scripts/package-brand.test.ts:31` covers deterministic bytes, traversal,
  symlinks, changed/truncated/missing inputs, collisions, reserved names and
  metadata limits; `:116` checks the actual logo source and invalid SVG cases.
  These are retained regressions, not only an ephemeral acceptance script.
- The README's rights section and all three bundled original font licenses
  were checked against their local sources. Font notices and binaries are
  copied unchanged. The package does not invent a license for the surrounding
  brand or claim that repository visibility grants reuse rights.

## Spec

**Disposition: PASS; 0 findings.** The bounded existing-identity package is
implemented and usable within its stated local scope:

- The derived `identity/site-mark.svg` contains the existing 32×32 viewBox,
  literal path and five circles from `src/components/site/logo.tsx:15`, with
  unchanged attributes and `currentColor`. The documentation distinguishes this
  geometry from the CSS badge, live wordmark and separate favicon. It does not
  claim an approved outlined master or invent missing print specifications.
- The distribution guidelines match the current light/dark CSS token blocks,
  Fontsource imports and cinematic mappings. They preserve the Quesar/Abbey
  narrative distinction from `notes/mlai/brand.md`, and explicitly limit older
  manifest colors and film-specific palettes instead of promoting them to
  site-wide authority.
- The archive includes local icons, existing social/room imagery, source
  references, font CSS/binaries and complete original notices. Every CSS font
  URL resolves within the archive. README instructions identify the canonical
  checkout/dependencies needed for reproduction and the extracted directory
  from which to verify `SHA256SUMS`.
- Film/teaser assets are separately labelled references. The packager validates
  copied inputs; the supplied acceptance and this review independently rehash
  the referenced current media. Those references are a snapshot, not a promise
  that later changes to the canonical files are automatically detected by
  archive reproduction. None of the referenced media is duplicated or modified
  by the package.
- Same-input/runtime reproducibility is explicitly bounded to the recorded
  environment. Public release, ownership adjudication, creative approval and
  human film listening remain outside the asserted result.

## Evidence and limits

**Current — independently checked by this reviewer:**

- All six frozen snapshot entries match their actual size and SHA-256.
- All 69 copied inputs match their pins and archived bytes; all 54 media
  references match the current canonical bytes.
- A Python standard-library tar reader opens all 72 sorted regular entries.
  Paths stay beneath `quesar-brand/`; modes are 0644 and uid/gid/mtime are zero.
- All 71 listed payload checksums and all distribution-manifest file entries
  match the extracted in-memory payloads. The archive size/hash match above.
- Font CSS dependencies are present, and all three complete license files
  match their local Fontsource originals byte-for-byte.
- The derived SVG matches the existing six geometry elements. Palette,
  typography and cited motion values were compared with current source.
- The retained full-gate log hash matches the verification JSON:
  `46a177b83795e8e8069b73bd01947414feb7e62168123a87719c849311557bd7`.
  Its contents show formatting, typecheck, lint, **89 files / 744 tests passed**,
  and a completed production build.

**Current — supplied execution receipts:** focused **20 tests passed**;
`bun run check` process 62761 exited 0; a second package was byte-identical;
standard extraction plus `shasum` verified all payloads. The reviewer inspected
the source, receipt and gate log but did not rerun those commands. This review's
read-only archive and hash checks independently confirm the delivered artifact.

**Partial:** independent final creative/claims approval and external rights
approval are not asserted. Font notices establish what accompanies the font
software; this review does not adjudicate authorship or grant public rights.

**Out of scope:** publication, deployment, film regeneration or listening,
unrelated product changes, commits and pushes.

**Review totals:** Standards 0 findings; Spec 0 findings. Both pass for the
frozen local packaging scope.
