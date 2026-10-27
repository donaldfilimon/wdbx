# Substrate release source-pin migration review — 2026-10-26

Review of the 39 live declaration-drift rows refused by
`tools/check_substrate_release.py`. Reviewed tree: `e5fd43b` (local main). The
catalog was frozen in `96c4b13`, which is the pin baseline for both paths.

**Outcome (fix round 1, 2026-10-27):** only `FORMAT-v2-journals` was migrated,
after this review and an independent review both approved it (see
[v2.rs migration](#v2rs-migration-applied)). The 38 `lib.rs` rows stay
unmigrated and still drifting. Live validation exits **1** with exactly 38 errors
on one path; `--snapshot-only` exits **0**. No inventory, contract or C0 status
changed.

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

- `lib.rs` carries **semantic** growth: `pub mod model;` exposes a public module
  tree and the native model package format (`tests/fixtures/model-v1/`). The
  substrate release catalog has no row for it, and neither `protocols-v1.json`
  nor `sources/plan.md` routes it to a release task. Refreshing the `lib.rs` hash
  would admit that unrouted tree and would make the release README's "83 lexical
  public-module declarations" false. The `read_v2_root` reexport is additive.
  The tree has two parts with different review states:
  - **Reviewed (N01, `a043e4f`):** `pub mod model;` and `contract`, `error`,
    `ids`, `limits`. This commit is an ancestor of the reviewed modernization
    cohort `2221889`, which `da4ba2b` merged. Its independent spec/code review and
    the index-fix re-review are recorded as accepted (`cb129cf`) in
    [`docs/plans/2026-10-25-native-model-cpu.md`](../../../../plans/2026-10-25-native-model-cpu.md)
    and [`docs/plans/2026-10-25-native-model-contracts.md`](../../../../plans/2026-10-25-native-model-contracts.md).
  - **Review pending (N02, `55f33a6`, with `8ad18a8`):** `package` and
    `tokenizer`. These are not in `2221889`. They reached main through the
    WIP-preservation merge `2b4c231` of `85c3baa` (snapshot of the idle
    `012-native-package` working tree). The native-model plan states that N02
    "independent review and integration remain pending"; see
    [`docs/plans/2026-10-25-native-package.md`](../../../../plans/2026-10-25-native-package.md).
  - **Owning plan and contracts:** [`docs/contracts/model/`](../../../../contracts/model/README.md)
    (binding [native-package design](../../../../contracts/model/native-package-design-v1.md),
    `package-v1.md`, `tokenizer-v1.md`), under company master items M2/M4. The
    model work has an owner; what is missing is routing in the *substrate release*
    catalog. That routing should reuse N01's accepted review and wait for N02's
    pending one.
- `v2.rs` is **additive export**: `read_v2_root` checks the existing
  `ABI-WDBX 2\n` `VERSION` marker and calls the existing `recover`. It takes no
  writer lease and changes no write path, journal layout, encoder or marker.
  Journal bytes are unchanged. Tests: `crates/abi-wdbx/src/v2/tests.rs`,
  `crates/abi-wdbx/tests/typed_cli.rs`. In the same window
  `crates/abi-wdbx/src/v2/types.rs` gained the public read-only
  `V2Snapshot::kv_keys` (`4c01514`) and `kv_versions` (`e5fd43b`). Those are
  additive v2-module exports with no format effect. No catalog row pins
  `v2/types.rs`, so they are noted here but do not affect this pin.

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
| `FORMAT-v2-journals` | journal format bytes, `v2.rs` | 3 | n/a | additive export (`4c01514`) | **Migrated** (fix round 1): approved by this review and an independent review |

Summary: 16 identity-only rows with the same line, 22 identity-only rows with the
line anchor shifted by one, 1 additive-export row, 0 semantic rows. The semantic
change has no row of its own by construction, because it is uncatalogued: it is
the `model` growth that shares the `lib.rs` file identity with the 38 module rows
and blocks them.

## Why `lib.rs` was not migrated

The checker and release README document one mechanism: "deliberate independent
review and a version/pin migration" (edit `catalog-v1.json`, update the
checker's `CATALOG_PIN`). There is no blessing command.

- `lib.rs` (38 rows): migrating the hash would accept the uncatalogued `model`
  growth, including the N02 part whose review is pending. **Blocked** on
  release-scope routing for `model` by the WDBX compatibility maintainer and the
  native-model plan owner.

## v2.rs migration (applied)

Two reviews approved moving `FORMAT-v2-journals` alone:

1. This review (`bc6dbfa`) classified the `v2.rs` change as an additive
   read-only export.
2. An independent read-only review of `e5fd43b..bc6dbfa` gave "v2.rs
   migration: APPROVE". It found the full `96c4b13..HEAD` diff of `v2.rs` is one
   17-line addition. `read_v2_root` reuses `V2Error::UnsupportedVersion` and
   `io_error`, writes nothing, takes no lease and leaves the journal format
   unchanged. `FORMAT-v2-journals` is the only catalog row (and no source
   `live_path`) on `v2.rs`, so it can move without touching `lib.rs`.
   Conditions: change only that row's `sha256` and `CATALOG_PIN`; leave `lib.rs`
   rows and line anchors alone; re-run `--snapshot-only`, `--drift-report` and
   the release suite.

Applied, as a pin migration of `catalog-v1.json` in place (catalog `version`
stays 1):

| Item | Before | After |
| --- | --- | --- |
| `FORMAT-v2-journals.sha256` | `28a6283b886b6290ea00cdd7500efa248724ab28d60ae3fa2bdba297eb8e0d64` | `80e50d5537e148d0f6b36842881764c29f0ee3921c98cd7fb906e91c8fe62759` |
| `CATALOG_PIN` (`tools/check_substrate_release.py`) | `d9b2cbe9ea14c65166bfa407686aae0eeffe76608d0bca46f1a246ee0c419248` | `e65441f2a36c099a1c752735ee9d3f109fe0b30c57e71158585e77408b8b955f` |

RED: with only the catalog edited, `test_valid` failed and `--snapshot-only`
refused ("catalog changed; independently reviewed version/pin migration
required"). GREEN: after the pin update, `--snapshot-only` exits 0, the 52 release
tests pass, and `--drift-report` exits 1 with 38 errors on the single path
`crates/abi-wdbx/src/lib.rs`
([drift-report-after-v2-migration.json](drift-report-after-v2-migration.json)).
The row stays `declaration-coverage-only` with its open
`missing_behavior_blocker`. An identity pin does not qualify behaviour or move
anything past C0.

## Proposed remaining migration (not applied)

1. Owner decision: route `model` to a release task or record an explicit
   exclusion. Reuse N01's accepted review for `contract`, `error`, `ids` and
   `limits`. Wait for N02's independent review before admitting `package` and
   `tokenizer`. Add declaration rows and a `FORMAT-model-package` row for
   model-v1, each with owner/task and an open `missing_behavior_blocker`.
2. Update `MOD-0048`..`MOD-0069` `line` anchors (+1) and the `lib.rs` rows to
   `afb2a62420f0fa8632248ddda78d9329331efc22d50cf40e50a09d0d9bcf9210`.
3. Update `CATALOG_PIN`, the release README counts (83 modules / 20 formats) and
   any count-dependent tests. RED is `test_valid` on the catalog edit; GREEN is
   the reviewed pin.
4. Re-run live, `--drift-report`, `--snapshot-only` and the release suite.

## Artifacts

- [drift-report.json](drift-report.json): `--drift-report` output at `e5fd43b`,
  SHA-256 `0c4d2ffe4b233c9606d6d8e7538ffbab5057a42734a750833f930417a5238457`.
- [row-classification.json](row-classification.json): the machine-readable form
  of the table above.
- [drift-report-after-v2-migration.json](drift-report-after-v2-migration.json):
  `--drift-report` after the v2.rs migration (38 errors, 1 path).
- [validation.json](validation.json): commands and exit codes.

