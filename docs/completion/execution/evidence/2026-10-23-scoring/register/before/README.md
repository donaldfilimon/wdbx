# Execution reconciliation v1

Every one of the 163 frozen requirements is reconciled to its owner, task,
qualification plan and inspected local references. All remain **open/C0**.
This is an execution inventory, not an evidence promotion schema.

- **120 candidate mappings** have direct, hash-pinned partial implementation and
  test references. Candidate code/tests do not establish complete requirement
  coverage, successful current execution, research efficacy or product conformance.
- **27 local gaps** identify missing direct qualification or local implementation
  work, with inspection references and exact frozen design/acceptance obligations.
  A gap does not claim no adjacent code exists.
- **16 external gaps** require ABI/consumer owners, installed platform artifacts,
  signing authority, separate hosts or elapsed operational evidence. Inspecting
  local workflows or sibling gateway source does not satisfy those obligations.
- **14 historical review/receipt pointers** remain unqualified; current review
  conclusions are recorded separately. No review is inferred from source presence.

Local gaps: G01, G02, G05, G12; CH01, CH02, CH04, CH26; S07–S16, S18, S24,
S28, S29, S32, S33, S36, S37, S41. Each acceptance gap repeats its exact frozen
case and requires direct positive/boundary/negative assertions.

External gaps: G10, G11; P05.01–P05.04; P10.01–P10.04; OS01–OS06.
All other IDs have candidate mappings. These classifications are routing and
inspection results, not completion findings. Subsequent implementation packages
must attach direct requirement-level source, tests, artifacts and independent
review; evidence promotion requires a separately reviewed schema and verifier.

## Historical source audit

The audit compares all listed files in three existing source manifests: old-profile
reader 7/7 matching hashes, private Rust complete-episode decoder 11/13, and Python
decoder 9/9. Mismatches are recorded individually. Current module-entrypoint/test
changes are explicitly observed against retained historical expected hashes.
Agreement establishes byte identity only. It does not rerun gates, qualify
unlisted dependencies, verify external raw logs, establish ABI agreement or
prove behavior. A source change invalidates its observation until explicit audit;
never automatically repin to hide drift.

Studio keeps local substrate path dependencies. Old git-pin guidance and gate
receipts do not override that decision. Inspection included Studio's AGENTS.md,
native canonical adapter, browser storage, kernel/conformance tests, platform
collector and local ABI gateway source. The native adapter is opt-in unsigned;
browser canonical and full cross-repository complete-envelope adoption remain
open. The old digest-COSE witness remains encoding-only. Separate private full-payload
P1 signing/verification, synthetic trust and admission simulation now have
partial candidate mappings; the bounded libsodium matrix and retained PyCA
failure do not establish ratification, persistence or production qualification.
No live store is used.

Studio observation inventory `studio-research-v1.json` records all 46 S cases and
18 D decisions verbatim, with current source identities, source-only baseline
limits and explicit blockers. Six reviewed identity/table mechanisms and one
synthetic native/WASM witness remain separate from whole-case qualification.
The artifact promotes no claim; all 163 requirements remain open/C0/unknown.

## Validation

Run separately from product gates:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/check_completion.py
PYTHONDONTWRITEBYTECODE=1 python3 tools/check_completion_execution.py
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/tests -p test_check_completion_execution.py
```

The standard-library verifier rejects missing/duplicate IDs, routing drift,
contract/source hash drift, escaping paths/symlinks, malformed reference shapes,
missing audits, false promotion and invented current test results. Every ID
requires a typed reconciliation, reason, inspection and qualification-plan refs.
Candidate entries require source and test mappings; gap entries cannot imply
candidate coverage.

The checker independently pins the frozen contracts and `candidate-paths-v1.json`.
That catalog fixes each ID's disposition and exact source/test/inspection/plan
identities, so repinning the register or substituting an unrelated valid plan hash
cannot silently shrink coverage. Catalog changes require explicit review and a
checker-pin update. Neither validator assesses scientific or cryptographic
adequacy. Run the original completion checker too; frozen C0 inputs are unchanged.

## Current language observation packet

The current packet is `evidence/2026-10-22-language/final-fix/`. Its
`requirement-mapping.json` and `current-source-manifest.json` route the additive
language/text/scanner/route APIs and direct tests to frozen S/D, P08 and chapter
requirements. The earlier observation wave moved only S05/S06 from local gap to
partial candidate; this final fix changes no dispositions. Counts remain
120 candidates / 27 local gaps / 16 external gaps, with all 163 requirements
open/C0/unknown, 46 S rows and 18 D rows verbatim, and the same 15 observation IDs.

The combined review's three Important findings are addressed by quote-opaque full
condition comparison, typed refusal of contradictory utility controls, and
borrowed table/target preflights before query materialization. Three small API/
assertion Minors are addressed: ordered scanner component names/count, shared
lexical complexity counting, and callable registry names/aliases. Independent
scoped rereview remains pending; this is not preapproval or scientific acceptance.

The root ran four separate Studio gates after absolute product-source freeze;
`final-fix/gates/summary.json` records all exit 0 and unchanged source hashes:
221 browser/unit passes, 228 native passes plus one existing ignored test, lint
and native all-target Clippy. Actual feature-WASM/native language traces match
at 151812 bytes, SHA256
`743d07418aadbe60c704d0d934c2bffb237e62e1e39c162ad731aba7d4a8e4d9`.
The independent comparator rejects digest and component-label corruption with
exit 1; default WASM is only compiled/export-inspected. Existing identity probe
parity is unchanged at 4205 bytes and its original hash. See the new raw runtime
folders, commands and `check-final-closure.py`; the old closure remains historical.

The original 23-file legacy baseline is never repinned: 22 files are unchanged,
and one exact reviewed immutable O(1) `HybridTable::entries()` accessor is added
to identity/mod.rs. Removing exactly that accessor reproduces the original bytes;
IDs, identity acceptance, saved schemas, locks and goldens remain unchanged.
All earlier manifests/logs/receipts remain byte-preserved historical evidence.

Read `language-v3-profile.md` for the bounded interpretation. Exact canonical
decimal capture comparisons remain separate from finite f64 arithmetic; +20/-20
missing-component outcomes are unchanged. Logical work reservations are not
elapsed-time or global-scheduler qualification. Bun 1.4.4 canary versus CI 1.4.0,
missing-libLLVM stripping diagnostics, dependency/build-script/future-compatibility
warnings and browser-build limitations remain attached to these executions.
Active engine/ID/save migration, durable utilities, real host authorization/effects,
full research controls/efficacy, scheduler/ATP/voting, ABI-owner receipt and
hardware/platform/UI/elapsed-operation qualification remain open.
