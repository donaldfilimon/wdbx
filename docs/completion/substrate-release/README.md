# Substrate release contract, version 1

This separate inventory proposes the 2026-10-23 substrate release scope. Every
obligation remains `proposed`, `C0`, with an empty `evidence` array. Validating
this directory proves contract structure and frozen identities only. It proves
no implementation, approval, hardware execution, security acceptance or release.
The historical 163-item program remains unchanged and open.

## Read and validate

Run `PYTHONDONTWRITEBYTECODE=1 python3 tools/check_substrate_release.py` from the
repository root, and run `python3 -m unittest discover -s tools/tests -p
test_check_substrate_release.py`. `--snapshot-only` checks the portable frozen
contract without comparing current WDBX source identities. Neither command
runs the Rust, Studio, hardware, crypto or operational gates.

- `catalog-v1.json` freezes scope routing, source hashes, inspection anchors,
  83 lexical public-module declarations (including abi-compute's three private
  implementation modules exposed through public reexports), four Cargo feature
  selections, and twenty persistence-format families. Cfg-gated and nested
  public declarations are conservatively included; this is not a rustdoc reachability audit.
- `inventory-v1.json` holds the 163 historical routes, all 22 release tasks,
  task checklist/acceptance clauses, normative source-clause candidates,
  source-linked behavior obligations. Module, feature and format declarations live separately in the catalog. Each has a concrete owner role,
  real task prerequisites, its exact source assertion, direct proposed
  experiment/oracle reference, evidence requirements and closure reference. Declaration coverage anchors cannot close a behavior.
- `protocols-v1.json` shares the 22 task-specific experiment procedures,
  evidence requirements and blocking conditions. Resolve a row's `protocol`
  and its source-specific `assertion` together; the protocol alone cannot
  accept any row. Named actual owner/reviewer acceptance remains future work.
- `baseline-v1.json` records revisions, initial dirty-file content identities,
  toolchain/manifests and existing receipt identities. Historical receipt
  presence is not current acceptance. This snapshot is deliberately not
  refreshed when unrelated implementation work changes.
- `sources/` contains byte-exact source snapshots; sibling source identities
  use repository-relative names. The member-erasure original is available,
  revision 3, explicitly unapproved. No implementation permission is inferred.
- `validation/` contains contract-test controls only, never product evidence.

## Coverage and interpretation

The catalog contains a separate conservative inspection-anchor inventory for
nonblank source body lines. These anchors preserve coverage for owner review;
they are not fabricated behavioral requirements or passing acceptance cases.
The v6 prose is classified by complete section/profile context in
`scope-review-v1.json`, independently pinned by the checker. Observable prose
is inventoried even without modal keywords. Schema/example syntax, table
headers, bibliography and task-index references remain justified inspection
anchors. Explorer UI and Specimen-only equality/voting/graves/ATP decisions
are excluded; mixed decisions apply only to their named affected profile.
Supporting-original keyword candidates remain proposed discovery inputs and
require actual owner interpretation before downstream conformance.

Declarations are separate coverage anchors, never behavioral obligations.
Each module/feature/format carries an open, owner/task-bound missing-behavior
blocker. Curated module domains link concrete v6 behavioral clauses where
available; these links do not pretend to prove the module implements those
clauses. Module-specific positive/refused oracles and independent procedures
must be frozen before acceptance. No declaration, compilation or generic root
gate can discharge a missing behavioral contract.

The release plan's checklist and acceptance clauses are directly inventoried.
Historical composite requirements remain external/excluded where only a
substrate subset is included: Studio research/UI and general consumer cutover
are not closed by API compatibility. The normative v6 source, supporting
originals and references are source-locked without pretending this mechanical
inventory independently ratifies their meaning. The v5 companion original was
not found. Its inherited checksum is explicitly unverified; obtaining it and
reconciling its requirements remains an open prerequisite, not a waived gate.

Catalog, protocol and baseline hashes are independent constants in the checker.
Coordinated inventory/catalog shrinkage therefore fails. Any scope/source/
protocol refresh requires deliberate independent review and a version/pin
migration; there is no blessing or promotion command. Unknown inventory fields,
duplicate JSON keys, forged success, source/hash drift, missing references,
missing protocols, path escapes, symlink components and oversized reads refuse.
All paths below the caller's trusted checkout root are bounded regular files;
the checker is a local document validator, not a hostile concurrent filesystem
service and makes no race-free adversarial path-resolution claim.

## Live drift diagnostics

Run `python3 tools/check_substrate_release.py --drift-report` for JSON diagnostics
on stdout. This live mode is mutually exclusive with `--snapshot-only`; it keeps
exit status 1 for drift or invalid contracts and omits the normal success sentence.
The version-1 report contains `mode`, `valid`, `complete`, `errors` and
`drifted_paths`. Each drifted path appears once with `expected_sha256`,
`actual_sha256`, affected `declaration_ids` and `source_ids`, and `read_error`.
Missing, unreadable, oversized and symlink-refused files have a null actual hash
and an explicit read error. Unique live paths are read once per validation.
Conflicting expected hashes refuse. Unpinned or malformed catalogs yield an
incomplete invalid report; an empty path list in such a report proves nothing
about live identity. `complete` means validation reached its end, not acceptance
or qualification; inspect `valid`, `errors` and the exit status together.

At the 2026-10-26 continuation, live validation refuses 39 declaration rows grouped
under `crates/abi-wdbx/src/lib.rs` (38 module rows) and
`crates/abi-wdbx/src/v2.rs` (one format row). The changed source introduces the
`model` declaration and `read_v2_root` in addition to byte drift. Resolving this
requires independently reviewed source/version and declaration-inventory
migration, not a blanket refresh of two hashes. Diagnostics never modify pins,
contracts or source snapshots and do not approve that migration. Isolated tests
materialize synthetic live declarations and mock only their temporary catalog
pin; the real immutable-pin and promotion-refusal tests remain enforced.

The per-row review of these 39 rows (16 identity-only, 22 identity-only with a
+1 line anchor, 1 additive export) is recorded in the review linked below. After
that review and an independent review, only `FORMAT-v2-journals` was migrated
(its `sha256` and `CATALOG_PIN`). Live validation now refuses exactly the 38
`lib.rs` module rows, which stay blocked on routing for the uncatalogued `model`
growth. The remaining migration is proposed, not applied, in
[the source-pin migration review](../execution/evidence/2026-10-26-source-pin-migration-review/README.md).
