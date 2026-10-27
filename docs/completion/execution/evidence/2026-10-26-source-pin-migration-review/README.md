# Substrate release source-pin migration review — 2026-10-26

Review of the 39 live declaration-drift rows refused by
`tools/check_substrate_release.py`. Reviewed tree: `e5fd43b` (local main). This
record changes no pin, catalog, inventory, contract or C0 status. Live validation
still exits **1**; `--snapshot-only` exits **0**. No migration is applied.

## Structural finding: migration is per file, never per row

The catalog binds each declaration row to the full-file SHA-256 of its `path`.
All 38 `MOD-0032`..`MOD-0069` rows share one `crates/abi-wdbx/src/lib.rs`
identity and `FORMAT-v2-journals` alone carries `crates/abi-wdbx/src/v2.rs`.
Rows on one path with different expected hashes refuse
(`conflicting expected hashes`). A row whose own declaration is unchanged
therefore cannot be migrated independently of every other change in its file.

## Provenance of the pinned identities

The catalog was frozen in `96c4b13`. Its `lib.rs` hash `ca24c1bf…` is the
content from `31f1d57`; its `v2.rs` hash `28a6283b…` is the content from
`b83f142`. Transitions to the current tree:

| Path | Commit | Change | Resulting SHA-256 |
| --- | --- | --- | --- |
| `crates/abi-wdbx/src/lib.rs` | `a043e4f` (reached main through the `2b4c231`/`da4ba2b` integration merges) | `+ pub mod model;` at line 47 | `41d3ab4b67988f363b1e7713e9d11e85826f6a0e013dd45ca39d7e88203ba02d` |
| `crates/abi-wdbx/src/lib.rs` | `4c01514` | `read_v2_root` added to `pub use v2::{...}` | `afb2a62420f0fa8632248ddda78d9329331efc22d50cf40e50a09d0d9bcf9210` |
| `crates/abi-wdbx/src/v2.rs` | `4c01514` | `+ pub fn read_v2_root` with doc comment (added lines 452–468) | `80e50d5537e148d0f6b36842881764c29f0ee3921c98cd7fb906e91c8fe62759` |

`433ce50` (cited in the company plan) already contained `pub mod model;`; only
the `read_v2_root` changes postdate it. Verified with `git show <rev>:<path> |
shasum -a 256` for `96c4b13`, `da4ba2b`, `2b4c231`, `433ce50` and `4c01514`, and
`git diff 31f1d57 4c01514` / `git diff b83f142 4c01514` on the two paths.

## Classification

- **Identity-only**: the row's own declaration text is byte-identical at HEAD.
  For rows after `manifest` the declaration moved one line down because of the
  inserted `pub mod model;`, so the catalog `line` anchor is stale (+1).
- **Additive export**: the only change on the path adds a new public item;
  existing items, encoders and writers are unchanged.
- **Semantic change**: new public surface or format with no catalog routing.

File-level verdicts:

- `lib.rs` carries **semantic** growth: `pub mod model;` exposes a new public
  module tree (`contract`, `error`, `ids`, `limits` from `a043e4f`; `package`,
  `tokenizer` from `55f33a6`) and the native model package format
  (`tests/fixtures/model-v1/`). The catalog has no row for it. Neither
  `protocols-v1.json` nor `sources/plan.md` names a release task or owner for the
  native model package (N02), so routing it is a scope decision. Refreshing the
  `lib.rs` hash would also make the README's "83 lexical public-module
  declarations" statement false. The `read_v2_root` reexport is additive.
- `v2.rs` is **additive export**: `read_v2_root` checks the existing
  `ABI-WDBX 2\n` `VERSION` marker and calls the existing `recover`. It takes no
  writer lease and changes no write path, journal layout, encoder or marker.
  Journal bytes are unchanged. Tests: `crates/abi-wdbx/src/v2/tests.rs`,
  `crates/abi-wdbx/tests/typed_cli.rs`.

## Row table

| Row | Declaration | Task | Pinned line → HEAD line | Row classification | Verdict |
| --- | --- | --- | --- | --- | --- |
| `MOD-0032` | `pub mod ans;` | 3 | 31 → 31 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0033` | `pub mod cluster;` | 16 | 32 → 32 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0034` | `pub mod cluster_rpc;` | 16 | 33 → 33 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0035` | `pub mod codecs;` | 3 | 34 → 34 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0036` | `pub mod compression;` | 3 | 35 → 35 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0037` | `pub mod compute;` | 13 | 36 → 36 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0038` | `pub mod crypto_he;` | 3 | 37 → 37 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0039` | `pub mod durable;` | 3 | 38 → 38 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0040` | `pub mod entropy;` | 3 | 39 → 39 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0041` | `pub mod evaluation;` | 3 | 40 → 40 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0042` | `pub mod fhe;` | 3 | 41 → 41 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0043` | `pub mod format;` | 3 | 42 → 42 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0044` | `pub mod hash;` | 3 | 43 → 43 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0045` | `pub mod hnsw;` | 3 | 44 → 44 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0046` | `pub mod index;` | 3 | 45 → 45 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0047` | `pub mod manifest;` | 3 | 46 → 46 | identity-only | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0048` | `pub mod multiway;` | 3 | 47 → 48 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0049` | `pub mod mvcc;` | 3 | 48 → 49 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0050` | `pub mod net_line;` | 3 | 49 → 50 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0051` | `pub mod neural_compress;` | 3 | 50 → 51 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0052` | `pub mod persistence;` | 3 | 51 → 52 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0053` | `pub mod rate_limit;` | 3 | 52 → 53 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0054` | `pub mod record;` | 3 | 53 → 54 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0055` | `pub mod remote_compute;` | 3 | 54 → 55 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0056` | `pub mod rest;` | 3 | 55 → 56 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0057` | `pub mod retrieval;` | 3 | 56 → 57 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0058` | `pub mod segment;` | 3 | 57 → 58 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0059` | `pub mod segments;` | 3 | 58 → 59 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0060` | `pub mod spatial;` | 3 | 59 → 60 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0061` | `pub mod spatial3d;` | 3 | 60 → 61 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0062` | `pub mod store;` | 3 | 61 → 62 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0063` | `pub mod store_path;` | 3 | 62 → 63 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0064` | `pub mod temporal;` | 3 | 63 → 64 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0065` | `pub mod tls_config;` | 3 | 64 → 65 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0066` | `pub mod v2;` | 3 | 65 → 66 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0067` | `pub mod v3;` | 3 | 66 → 67 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0068` | `pub mod versioned;` | 3 | 67 → 68 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `MOD-0069` | `pub mod wal;` | 3 | 68 → 69 | identity-only (line anchor +1) | Not migrated: blocked by the `model` growth on the shared `lib.rs` identity |
| `FORMAT-v2-journals` | journal format bytes, `v2.rs` | 3 | n/a | additive export (`4c01514`) | Not migrated: candidate for a reviewed migration; needs independent review |

Summary: 16 identity-only rows with the same line, 22 identity-only rows with the
line anchor shifted by one, 1 additive-export row, 0 semantic rows. The semantic
change is uncatalogued growth that shares `lib.rs` with the 38 module rows.

## Why nothing was migrated

The checker and release README document only one mechanism: "deliberate
independent review and a version/pin migration" (edit `catalog-v1.json`, update
the checker's `CATALOG_PIN`). There is no blessing command. The release README
explicitly rejects "a blanket refresh of two hashes".

- `lib.rs` (38 rows): migrating the hash would accept uncatalogued module and
  format growth. **Blocked** on owner routing for `model` (named WDBX
  compatibility maintainer and release-scope owner).
- `v2.rs` (1 row): this review classifies it as additive. Applying it still means
  editing the frozen catalog and changing `CATALOG_PIN` on self-review only. That
  is not the independent review the contract requires. **Deferred** to that
  reviewer; this record supplies the evidence.

## Proposed migration (not applied)

1. Owner decision: route `model` (and nested `contract`, `error`, `ids`,
   `limits`, `package`, `tokenizer`) to a release task or record an explicit
   exclusion. Add declaration rows and a `FORMAT-model-package` row for the
   model-v1 format, each with owner/task and an open `missing_behavior_blocker`.
2. Update `MOD-0048`..`MOD-0069` `line` anchors (+1). Update `lib.rs` rows to
   `afb2a62420f0fa8632248ddda78d9329331efc22d50cf40e50a09d0d9bcf9210` and `FORMAT-v2-journals` to `80e50d5537e148d0f6b36842881764c29f0ee3921c98cd7fb906e91c8fe62759`.
3. Bump the catalog version or record the in-place pin migration. Update
   `CATALOG_PIN`, the counts in the release README (83 modules / 20 formats) and
   the 38-row expectation in
   `test_two_paths_group_module_and_format_rows`. The RED step is
   `test_valid` failing on the catalog change; GREEN is the reviewed pin.
4. Re-run live, `--drift-report` and `--snapshot-only` and the release suite.

## Artifacts

- [drift-report.json](drift-report.json): `--drift-report` output at `e5fd43b`,
  SHA-256 `0c4d2ffe4b233c9606d6d8e7538ffbab5057a42734a750833f930417a5238457`.
- [row-classification.json](row-classification.json): the machine-readable form
  of the table above.
- [validation.json](validation.json): commands and exit codes.

