# WebPress publication ZIP entry review — 2026-10-03

## Scope and source freeze

**Current verdict: Standards PASS; Spec PASS. Zero open findings.**

The reviewed change fixes the exported `site/index.html` redirect when an
accepted publication contains collection/detail routes but no published `/`.
The scope is only `src/export/publication.ts` and its colocated
`src/export/publication.test.ts` in
`/Users/donaldfilimon/dev/active/webpress`.

The review baseline is the supplied incremental patch, not the entire dirty
checkout's diff from HEAD. WebPress HEAD was
`f3cea13e657b39a2205769c0a1e894f02df7e525`.

- Patch: `.superpowers/sdd/2026-10-03-browser-exports/publication-entry-fix.patch`
- Patch SHA-256:
  `82aaeb27863af2ace00040ecb83c6e88785386d33714f266d8601e90b1ac1dc7`
- `src/export/publication.ts` SHA-256:
  `488fc78a2f2081a19fdacc8287ab66107ccd06f54d75c7afc64835be47db6bac`
- `src/export/publication.test.ts` SHA-256:
  `8614253e9387ebc97f123d11eccbaec30e91a68644a622e89ce58fbaac8a457c`

Reviewer calculations match `publication-entry-receipt.json` for both source
files and the patch. The reverse patch applicability check and scoped
`git diff --check` both exited 0. No product source was edited or tests rerun by
the reviewers; the coordinator owns the full gate and browser interaction.

Standards sources were WebPress `AGENTS.md`, the supplied operator instructions,
existing exporter conventions and the code-review skill's heuristic smell
baseline. The Spec assessment was independently delegated using the explicit
scoped requirements and `publication-entry-fix.md`.

## Standards

**Verdict: PASS. Zero findings.**

| Severity | File:line | Description | Suggestion | Status |
| --- | --- | --- | --- | --- |
| None | `webpress/src/export/publication.ts:102` | The selection reuses the existing sorted, validated route traversal. It stores only the selected route and leaves accepted page bytes, redirect target validation, asset verification and host adapters in their existing paths. No new dependency or parallel publication renderer is introduced. | None. | Accepted. |
| None | `webpress/src/export/publication.ts:151` | The emitted entry uses the selected route under the validated canonical base path and retains HTML escaping. No eligible HTML route produces an inert explanation without a broken redirect. Errors in route or asset validation still reject the export. | None. | Accepted. |
| None | `webpress/src/export/publication.test.ts:39` | Colocated behavioral regressions assert the non-root target and its actual archived HTML, preserve root behavior, exclude feeds/aliases, compare reversed insertion order, and cover empty/feed-only snapshots. Existing asset and malformed-path tests remain intact. | None. | Accepted. |

No documented-standard violation, actionable baseline smell, new lock usage,
`unwrap()` or unnecessary clone was found in the scoped patch. The small
selection state fits the existing function and does not require an additional
abstraction.

## Spec

**Verdict: PASS. Zero findings.**

| Requirement | File:line | Assessment | Status |
| --- | --- | --- | --- |
| Preserve an explicitly published HTML `/`, including a declared root redirect. | `webpress/src/export/publication.ts:109`; tests at `webpress/src/export/publication.test.ts:59` and `:73`. | Root remains preferred and retains its accepted routing behavior. | Current; satisfied. |
| Otherwise select the first sorted non-redirect HTML route; never infer a fixed collection path or select a feed/alias. | `webpress/src/export/publication.ts:102`; tests at `webpress/src/export/publication.test.ts:90`. | The existing traversal supplies order; eligibility excludes non-HTML routes and aliases. Reversed input insertion order produces the same entry. | Current; satisfied. |
| Keep empty/no-HTML snapshots safe. | `webpress/src/export/publication.ts:151`; tests at `webpress/src/export/publication.test.ts:117`. | No selected route emits explanatory HTML without refresh or a navigation target. | Current; satisfied. |
| Preserve accepted page bytes, asset checks and host adapters. | `webpress/src/export/publication.ts:114`, `:125`, `:132` and `:147`. | These paths are unchanged by the patch. `publicationZip` consumes the corrected file map at line 173. | Current; satisfied. |

No missing requirement, incorrect implementation or scope expansion was found.
This is an archive-entry correction using accepted route authority; it does not
introduce a new homepage declaration or modify publication metadata.

## Validation evidence and limits

All evidence paths below are relative to WebPress
`.superpowers/sdd/2026-10-03-browser-exports/`.

| Evidence | Inspected result |
| --- | --- |
| `publication-entry-receipt.json` | Records the original failing regression, initial green run, final focused run, TypeScript/Biome exit codes and the verified source hashes above. |
| `publication-entry-focused.log` | 14 tests passed across the export and local-publication test files. The receipt records exit 0. |
| `publication-entry-typecheck.log` | Both root and server TypeScript configurations invoked; the receipt records exit 0. |
| `publication-entry-biome.log` | Two files checked, no fixes applied; the receipt records exit 0. |
| Reviewer patch and diff checks | Both exit 0, with no source mutation. |

**Partial acceptance boundary:** this independent source review and focused
evidence do not replace the coordinator's full `bun run check` or a fresh ZIP
download followed by navigation from the extracted origin root. Those checks
were assigned separately and their final receipts were not part of this review.
The original failing downloaded archive remains a separate observed specimen.

**Review totals:** Standards: 0 findings. Spec: 0 findings. Worst issue on either
axis: none found in the scoped change.
