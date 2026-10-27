# Model routing catalog migration review (2026-10-27)

Independent read-only review of the uncommitted `catalog-v1.json` change that
routes `model` to release task 3 and migrates the 38 `lib.rs` rows, before
`CATALOG_PIN` was updated. The reviewer was a fresh-context agent given the
files and the owner decision, not the author's conclusions. Its report follows
verbatim; the disposition is recorded after it.

---

Catalog migration: APPROVE WITH REQUIRED FIXES

The catalog file is correct as written and I found no defects in it, so nobody needs to reopen the JSON. All three required fixes are outside the catalog. If only `CATALOG_PIN` is bumped, two gates still fail: the release test suite and live mode.

**Pin for the catalog as it stands:** `CATALOG_PIN = 6dd4b1c4cdeb156229a763e786e0ec53616d525fe4439ead217ba7108dbfc23f`. The catalog hash and its diff were the same at the start and end of the review, and the diff matches `catalog-migration.diff` byte for byte.

## Findings

### Critical
None.

### Important

**I1. Live mode fails right now, because root `README.md` has changed since the review began.** This is not caused by the catalog.
- The working tree now has a second uncommitted change: `README.md`, last modified at 08:13:24, after the catalog (08:12:25) and after this review started. It adds a "Where things stand" section of 18 lines.
- `README.md` is a catalog live source (`sources[id=readme]`, `live_path: README.md`, pinned at `559dc9c3…`, which is the HEAD content). Its current hash is `058e9cf5…`.
- With the pin overridden in memory, live mode and `--drift-report` exit 1 with one error, `readme: live source drift`, and one drifted path (`README.md`, no declaration rows).
- When I fed the checker the HEAD version of `README.md` in memory, live validation passed with no errors, 0 drifted paths, and `valid` and `complete` both true. So the catalog migration on its own gives zero drift.
- **Required fix:** either keep the `README.md` edit out of the tree until this pin lands, or give it its own reviewed source-snapshot migration. That means updating `sources/readme.md` and `readme.sha256`, which changes the catalog bytes again and therefore needs another `CATALOG_PIN`. The `6dd4b1c4…` pin is only valid for the catalog without that change. Also find out who is writing `README.md`, since one writer per repository is the rule.

**I2. One test still expects the old row count, so bumping the pin alone leaves the suite red.**
- With the pin overridden inside the test module, 51 of 52 tests pass.
- `tools/tests/test_check_substrate_release.py:359` (`test_two_paths_group_module_and_format_rows`) asserts 38 `declaration_ids` on `lib.rs`. It now gets 39, because `MOD-0084` (`pub mod model;`) also sits on `lib.rs`.
- **Required fix:** change the expected count to 39 in the same commit as the pin.

**I3. The release README's counts and drift prose are out of date.**
- `docs/completion/substrate-release/README.md:18` says "83 lexical public-module declarations"; it should be 96.
- Line 20 says "twenty persistence-format families"; it should be 21.
- The "Live drift diagnostics" paragraphs that describe 38 refused `lib.rs` rows need updating, and so does the migration-review README's "Proposed remaining migration (not applied)" section.
- **Required fix:** update these in the same change. Step 3 of the migration review already calls for this.

### Minor

- **m1. The new rows go beyond what the tranche plan lists, but correctly.** Plan step 2 names `lib.rs` plus the six declarations in `model/mod.rs`. The catalog also adds `MOD-0091`..`MOD-0096` for the six `pub mod` lines in `model/contract.rs`. That follows the catalog README's rule that "nested public declarations are conservatively included", and the earlier nested rows for `v3/mod.rs` and `v3/episode/mod.rs` do the same. Record this so the difference from the plan is explained.
- **m2. The `FORMAT-model-package` path is acceptable but only partly protective.** `model/package/mod.rs` holds re-exports and helpers. The actual bytes are defined in `header.rs` (the 96-byte header), `directory.rs` (framing and package digest), `write.rs`, `components/` and `contract.rs` (`MAGIC`, `PACKAGE_ID`). Choosing the module's root file matches the earlier rows: `FORMAT-complete-episode` points to `complete_episode/mod.rs` and `FORMAT-v3-cose` to `cose/mod.rs`. As with those rows, editing the files that actually define the bytes won't trigger drift on this pin. No other single file is clearly better: `directory.rs` fits the plan's phrase "defines the package framing" more closely, but it doesn't cover the header or the components. I'm not asking for a different path. Mention the limitation in the migration README.
- **m3. The private modules in `package/mod.rs` are correctly left out.** `components`, `directory`, `read`, `stage` and `write` are private but re-exported with `pub use`. Across the repo, 61 such modules are uncatalogued; the only ones catalogued are abi-compute's three crate-root modules. Leaving these five out follows that convention.

## Point-by-point verification

1. **Only the intended rows changed.**
   - `version`, `sources`, `items`, `inspection_anchors`, `features` and `missing_sources` are unchanged, and so is the order of the top-level keys.
   - Of the 83 existing module rows, exactly 38 changed. All of them are on `lib.rs`, and the only fields that changed are `sha256` and `line`.
   - All 38 now have `sha256 = afb2a624…`, which equals `shasum` of the current `lib.rs`.
   - Exactly `MOD-0048`..`MOD-0069` (22 rows) moved +1 line. `MOD-0032`..`MOD-0047` (16 rows) changed hash only.
   - For every one of the 38 rows, the declaration text sits on the stated line of `lib.rs`.
   - No existing row was removed or reordered, and none of the 20 existing format rows changed.
2. **Coverage of the `model` declarations is complete.**
   - Every `pub mod` under `crates/abi-wdbx/src/model/` is covered: `mod.rs` lines 17–22 and `contract.rs` lines 4–9. `MOD-0084` covers `lib.rs:47`.
   - There are no `cfg`-gated, inline or `pub(crate)` module declarations in that tree. All other declarations there are private (`mod`).
   - Repo-wide, no `pub mod` in `crates/` is missing from the catalog, and every new row is a real `pub mod`.
   - For each new row, the path, line, declaration text and `source_assertion` match the file, and each `sha256` matches its file's actual hash. The hashes are `228a8ee5…` (`mod.rs`), `4bf1b063…` (`contract.rs`) and `544b68bf…` (`package/mod.rs`).
3. **FORMAT row:** see m2.
4. **Row shape and formatting.**
   - All 14 new rows are `declaration-coverage-only`, owned by `WDBX compatibility maintainer` on task 3. That matches the task-03 protocol owner and the plan's "Task 3 — Qualify compatibility and supporting crates".
   - Each blocker is `open`, with the same owner and task as its row and the one standard condition string used across the catalog.
   - `behavior_links` is empty in every new row. IDs are unique: 96 modules and 21 formats.
   - The key order matches the existing rows, and the file is byte-identical to `json.dumps(indent=2, ensure_ascii=False) + "\n"`.
5. **No claims are promoted.** No inventory, protocol, scope or baseline file changed; there are no new `behavior_links`; and every blocker is still open. With the pin in place, `--snapshot-only` passes, which includes the checks that every obligation stays `proposed`/C0 with empty evidence.
6. **Checker runs:** see "Checks executed".

## Checks executed

All git commands were read-only. Python ran with `PYTHONDONTWRITEBYTECODE=1`, and the `__pycache__` folders already existed before this session (last modified 03:36). Scratch files are under `/private/tmp/claude-501/-Volumes-Extreme-SSD-Development-dev-repos-local-the-donald-company-company-legacy-wdbx/248bc0ef-829b-404b-a799-84a94584d21f/scratchpad/`.

| Check | Result |
| --- | --- |
| `git diff docs/completion/substrate-release/catalog-v1.json \| cmp - .../catalog-migration.diff` (at start and at 08:16) | SAME both times |
| `.../scratchpad/cmp.py` (field-level comparison of HEAD and working catalog, line and hash checks against the files, formatting round-trip) | Results as in points 1, 2 and 4; pin `6dd4b1c4…` |
| `grep` over all `mod` declarations under `model/`; Python scan of all `pub mod` in `crates/` against the catalog | 0 `pub mod` missing from the catalog |
| Scan for private modules with `pub use` re-exports | 61 uncatalogued repo-wide, including the 5 in `model/package` (consistent with convention) |
| Checker with pin overridden in memory, live mode | exit 1; `readme: live source drift` |
| Same, `--snapshot-only` | exit 0; "Substrate release C0 contract valid…" |
| Same, `--drift-report` | exit 1; 1 drifted path, `README.md` (`058e9cf5…` against expected `559dc9c3…`) |
| `validate(live=True)` with the pin overridden and HEAD's `README.md` substituted in memory | `errors []`, 0 drifted paths, `valid` true, `complete` true |
| `git show HEAD:README.md \| shasum -a 256` | `559dc9c3…`, equal to the catalog's `readme` hash |
| Unmodified checker, live and `--snapshot-only` | both exit 1, "catalog changed; independently reviewed version/pin migration required" (expected while the pin is not updated) |
| `python3 -m unittest discover -s tools/tests -p test_check_substrate_release.py` | 52 run, 3 failures (`test_valid`, `test_unhandled_reference_kind`, `test_two_paths_group_module_and_format_rows`); expected before the pin update |
| Same suite with the pin overridden on the test module's `checker` | 52 run, 1 failure: line 359, `38 != 39` (I2) |
| `git status --short` at the end | `M README.md`, `M docs/completion/substrate-release/catalog-v1.json` |

---

## Disposition (author, 2026-10-27)

- I1 fixed: the uncommitted `README.md` edit was this session's own draft index.
  `README.md` is a pinned live source, so the edit was reverted (README is
  byte-identical to HEAD, `559dc9c3…`) and the index moved to the unpinned
  `docs/README.md`. No other writer was active.
- I2 fixed: `test_two_paths_group_module_and_format_rows` now expects 39 `lib.rs`
  declaration IDs; `test_model_rows_refuse_live_drift` was added for the new
  model rows.
- I3 fixed: release README counts are 96 modules and twenty-one formats, and a
  2026-10-27 paragraph records this migration and the m2 limitation. The
  "Applied" section below supersedes "Proposed remaining migration (not applied)".
- m1 recorded: the `model/contract.rs` rows follow the conservative nested rule.
- m2 recorded in the release README. m3 needs no change.
- `CATALOG_PIN` was set to `6dd4b1c4cdeb156229a763e786e0ec53616d525fe4439ead217ba7108dbfc23f`,
  the digest the reviewer computed for the unchanged reviewed catalog.
