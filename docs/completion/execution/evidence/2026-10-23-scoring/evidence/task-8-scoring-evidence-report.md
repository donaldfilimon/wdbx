# Task 8 scoring evidence report

2026-10-23. Observation maintenance is complete and frozen for independent final
review. No commit or staging was performed. Product code, frozen contracts,
legacy sources and historical raw evidence were not edited by this task. Final
whole-tranche review and root's final handoff receipt remain pending.

## Current inventory and routing

Only S07–S15 change from local_gap to partial mechanism candidate. Counts are
**129 candidate / 18 local_gap / 16 external_gap = 163**. All 163 execution claims
remain **open/C0/unknown**; frozen requirements remain proposed/C0 with empty
evidence. All **46 S cases and 18 D decisions**, frozen text/order and baseline
observations are preserved. The previous 15 bounded observation IDs remain and
nine scoring observations are appended, yielding 24 unique IDs. No other
requirement changes disposition.

Current files are `docs/completion/execution/{register-v1.json,
candidate-paths-v1.json,studio-research-v1.json,studio-research.md,README.md}`.
The only checker change is the candidate catalog SHA256 literal in
`tools/check_completion_execution.py`; validation policy is unchanged. Before
snapshots and `register/owned-inventory.diff` make the exact changes reviewable.

The resolved `evidence/requirement-mapping.json` retains cases,
program_chapter_routes and decisions, and adds explicit source/test groups.
`register/requirement-mapping.json` is byte-identical. It routes S07–S15,
D06–D08/D16, P08.03/P08.09/P08.10 and CH09/10/11 plus bounded CH14/18/20/23.
`register/current-source-manifest.json` covers exactly the final 40 tranche
identities: 25 kernel/API/test files, 13 probe/registration inputs, profile and
rulings, each with unique mapped IDs/decisions. No FINALHASH_PENDING value occurs
in the current inventories or resolved mappings. Current minimal lib/manifest
references are refreshed; historical expected/source/raw identities remain.

Named assertion bodies were inspected. S07/S08 are numeric policy fixtures, with
separate actual scanner/contribution evidence for S09–S13. S10 multi-node support
comes from the orchestration graph and its distinct NodeRefs; the scorer-only
complementary fixture uses one node. S14's 1501 descriptors complete with None:
lease accounting/drain, not 1501 scanner evaluations or physical concurrency.
S15 is a shared broker protocol, not host-wide actual worker enforcement.
Explicit blockers retain these distinctions.

## Supplied final executions and review provenance

Final runtime evidence is `parity/canonical-final/`, followed by the current
rename-only JS fix in `parity/review-fix1/`. Its verification records actual
feature-WASM/native execution with 243932-byte semantic projections, SHA256
`7125a31a9932c95a0df0cf42c505cfe988c29653d9d36f10237f7fa7182be4d8`,
24 fixed-boundary outcomes, 68 equal logical diagnostics and zero host calls.
Architecture-labelled raw wrappers remain unequal at 250864/250863 bytes.
Arithmetic, quarantine and fixed-boundary corruptions are rejected by the normal
comparator in three processes, each exit 1. Default WASM is compiled/export-
inspected; only feature WASM executes. The JS rename reuses hash-qualified native,
feature/default WASM artifacts, re-executes native and feature runtime, and makes
no Rust rebuild. No feature/default execution claim is inferred from compilation.

The original orchestration review is CHANGES REQUIRED; fix1's scoped PASS resolves
I1/I2. Original/intermediate architecture-dependent decisions, historical 209566
semantic projections and attachment delta68 remain immutable. Canonical-budget
fix1's scoped PASS covers fixed attachment Context/Visit/Arc/capture and Span
charges. All 14 new production modules have a recorded clean layout-policy audit;
Contribution scalar/collection charge is 2048. This remains logical-charge-v1,
not physical allocation or ABI acceptance. Original probe review is CHANGES
REQUIRED; rename-only fix1 and scoped rereview PASS resolve Important I1 without
relaxing assertions. Minor M1 defining per-ID input-pin hardening remains
nonblocking final triage. No final combined approval is claimed.

Root's `gates/review-fix1/summary.json` records four separate Studio gates, all
exit 0, with 324 source identities unchanged during each: `bun run check`
(221 browser/unit passes and production build), `bun run lint`, `bun run test:native` (311 passes plus one existing ignored test), and `bun run
check:native` (native all-target Clippy). The evidence worker did not rerun them.
The native-test/native-clippy logs still show rust-objcopy SIGABRT/dyld libLLVM
warnings despite DYLD_LIBRARY_PATH. Possible cache replay is unadjudicated;
isolated successful loader/build repair does not establish clean broad-gate
linking. Existing AdmissionSimulator import/block futurecompat, browser chunk/
static-analysis limitations and Bun1.4.4 canary versus CI1.4.0 remain recorded.
The ignored test is not passing qualification.

## Inventory validation and preservation

Raw argv/cwd/environment/time/status/log receipts are under `register/`. The
current interpreter resolves to Python3.15; system is /usr/bin/python3 (3.9.6).
All commands use PYTHONDONTWRITEBYTECODE=1. Each interpreter passed:

- `tools/check_completion_execution.py` (exit0).
- `-m unittest discover -s tools/tests -p test_check_completion_execution.py`
  (27 tests, OK, exit0).
- `docs/completion/execution/evidence/2026-10-23-scoring/register/check-scoring-closure.py`
  (exit0; final receipts are current-closure-final/system-closure-final).

`validation-summary.json` records the original six passes; the final closure
receipts additionally check the actual 24-row trace and the 14-module recorded
audit. Checker/tests were not repeated after those closure-only assertions.
`git diff --check` passes; because execution files are currently untracked,
`owned-whitespace-check.json` separately verifies their text. No product gate,
frozen-check or 3-D suite was rerun by this evidence task.

Closure rehashes 80/80 preservation files, 3/3 frozen baseline files and 815
historical evidence files without repinning; it preserves baselines/frozen rows
and checks exact catalog-only checker change, current references, routing and
supplied final gate/source/trace identities. Hash agreement establishes byte
identity only, not source authenticity or scientific adequacy.

Existing `contracts/` receipts record frozen checker plus 37-test suite exit0
under both interpreters. `register/frozen-check-reuse-qualification.json`
corroborates reuse with 74 checker/test/frozen input files equal to Git HEAD and
28 embedded live/snapshot/source/contract pins matching. Historical receipts lack
a contemporaneous full runtime input manifest; that limitation is explicit.
Corpus handling for this check was hash-only, without evaluating efficacy data.

Two preparation errors are retained as observations: initial hash qualification
included two generated __pycache__ files absent from HEAD, then excluded runtime
caches; initial development closure exposed missing D06/D07/D08/D16 routing for
P08.10 probe sources, then metadata routing was corrected. Neither involved a
product/frozen gate failure or schema relaxation. Their summaries are
`input-qualification-initial-error.txt` and `closure-initial-error.txt`.

## Remaining work

Active engine/commands/feedback adoption; ID/save/import projection/migration;
durable versioned utilities; real host contact authorization/effects; full
research efficacy, contamination controls and adjudication; durable ATP/full
scheduler/voting/budgets; ABI owner/durable trust boundary; OCR/model/GPU; all
targets, installed desktop, browser UI/theme/accessibility and elapsed operation
remain open. Typed redundancy, inert legacy actions, supplied ATP and local
fencing/accounting do not satisfy those clauses. No live store, holdout evaluation,
network/install, credentials, stage, commit or deployment was used. This report
freezes the observation packet for root's final independent review and receipt.
