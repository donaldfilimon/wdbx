# Task 8 scoring evidence independent review

2026-10-23 UTC. Review of the frozen observation-maintenance packet only.

**Specification compliance: PASS for the bounded evidence task.**

**Evidence/checker quality: PASS for the bounded evidence task.**

No actionable Critical, Important or Minor finding was established in the owned
evidence changes. This does not approve the whole product tranche or promote a
requirement. The previously recorded probe Minor M1 remains the separate,
nonblocking final-triage observation identified by the complete-probe review.

## Scope and method

Read root and Studio instructions, the Task 8 scoring-evidence brief/report and
mapping preflight. Reviewed the exact six-file `register/owned-inventory.diff`,
the resolved mapping, current source manifest, observation freeze, new closure,
input-qualification and validation-capture scripts, validation receipts and
their named supporting runtime/gate/review receipts. Here and below, evidence
paths are relative to
`docs/completion/execution/evidence/2026-10-23-scoring/`.

Performed read-only JSON comparisons, hash checks and log-summary inspection.
Did not rerun a green suite, execute the closure/checkers, re-review product
implementation, install, use the network, stage, commit or modify another
worker's files. The only reviewer write is this document. Product assertion
adequacy relies on the explicitly scoped independent product/probe reviews;
this review checks their routing, identities, dispositions and qualifications.

## Findings

- Critical: none.
- Important: none.
- Minor: none newly established in this evidence task.

## Verified evidence boundaries and strengths

1. The disposition comparison changes exactly S07–S15 from `local_gap` to
   `candidate`, yielding 129/18/16 across 163 requirements. All 163 execution
   claim triples remain `open/C0/unknown`; the 46 S and 18 D frozen rows,
   ordering and baseline observations match the before snapshots. The previous
   15 observation IDs remain, followed by nine unique scoring IDs. The three
   frozen baseline files match their recorded bytes; no claim promotion is
   implied by the candidate label.

2. All 43 observation-freeze pins, all 40 final-source identities, all 80
   preservation-baseline identities, all three frozen baseline identities and
   all 815 historical-evidence identities rehash successfully. Both resolved
   mapping copies are byte-identical. The manifest has no duplicate or unknown
   mapped requirement/decision IDs and its source/test population is completely
   covered by the mapping groups. Standard `path`/`sha256` references in the
   current execution register (5260 occurrences) and Studio research inventory
   (2000 occurrences) match current file bytes. Historical expected hashes and
   old execution identities remain qualified as historical rather than being
   presented as validation of changed sources.

3. S07/S08 are accurately scoped to numeric policy fixtures. S10's distinct
   NodeRef graph support is distinguished from the scorer-only single-node
   fixture. S14's 1501 descriptors and None completions are described as lease
   accounting/drain, while small actual scanner/broker cases are separate.
   S15 describes a shared broker protocol with explicit host-ownership and
   physical-concurrency exclusions. D06–D08/D16, P08.03/P08.09/P08.10 and the
   stated chapter routes retain the incomplete active engine, save/ID,
   host-authority, research, full ATP, scheduling, owner, hardware and installed
   application clauses. No new S16/S18, feedback, ATP-dynamics or raft-range
   completion follows from the mappings.

4. Original orchestration CHANGES REQUIRED and scoped fix1 I1/I2 PASS are both
   retained and accurately described. Canonical-budget and attachment-budget
   scoped approvals are not relabelled as actual target execution. Original
   probe quality CHANGES REQUIRED, rename-only scoped fix1 PASS, and surviving
   M1 are represented candidly. Historical intermediate failures and the older
   209566-byte trace are not substituted for the final witness.

5. The final retained native and feature-WASM semantic objects are equal; both
   projection files are 243932 bytes with the stated SHA256. Their 24 logical
   boundary rows and 68 equal logical diagnostics are present. The architecture
   wrappers are unequal at 250864/250863 bytes. The positive transcript records
   zero host calls, feature runtime execution and default compile/export
   inspection only. Three separate normal-comparator negative transcripts fail
   at effective confidence, cancellation/quarantine and fixed-boundary outcome
   assertions, each with recorded exit 1. The evidence does not equate these
   finite executions with physical-memory, ABI, efficacy or all-target proof.

6. Each of the four fresh Studio gate receipts records exit 0 and 324 source
   identities; all referenced current source bytes and receipt/log hashes match.
   Raw native-test summaries total 311 passed, zero failed and one ignored;
   the browser log records 221 passes. The evidence preserves actual current
   rust-objcopy/SIGABRT/libLLVM warnings in both native logs despite the recorded
   library path, and leaves possible cached replay unadjudicated. It also keeps
   dependency/browser/Bun-versus-CI limitations explicit. No ignored test is
   counted as passing qualification.

7. The only execution-checker source change is its candidate-catalog hash
   literal: comparison after replacing that literal makes before/current
   checker text identical. The new closure adds bounded current/historical
   identity, routing, preservation and supplied-receipt assertions without
   relaxing the frozen checker or schema. Its final receipts bind the final
   closure hash under both interpreters. The retained mutable checker and
   27-test outputs are green under each interpreter; the report explicitly
   separates those original six checks from later closure-only assertions.
   The tracked-diff limitation is addressed by a separate owned whitespace
   receipt. Preparation failures remain documented rather than being called
   product failures or erased.

8. Frozen-check reuse is properly qualified: 74 input-file identities match
   their recorded HEAD identities and current bytes; all 28 embedded pins match
   current bytes. The four historical frozen checker/test receipts and raw
   logs match their hashes and show zero exits, with 37 tests per interpreter.
   The qualification explicitly acknowledges the missing contemporaneous full
   runtime-input manifest. It corroborates reuse rather than manufacturing a
   fresh frozen-check execution or scientific result.

## Exact reviewed identities

| File | SHA256 |
|---|---|
| `register/owned-inventory.diff` | `8bccd22a5fb37c53cdc0b4d21c899348374a82281a9ab3cb493f19f44659f221` |
| `evidence/observation-freeze.json` | `cd1bc40b81cb27032c03e457578649b9731487508ba426d76e4c07b8db900b77` |
| `register/current-source-manifest.json` | `109197a0609b2c921ac9b1c51e04cf1c3a31e03f9ed3ebbc0c506dd70d1287f6` |
| `evidence/requirement-mapping.json` and register copy | `196cf8fbd0d57fb380b7ddea96d524d328e1482c20169553c9d4c3d75059b707` |
| `register/check-scoring-closure.py` | `baa5f94e2fcc3ed5b2171a27677909cc3bb5377e46fb0842c63061c302567e55` |
| `register/qualify-inputs.py` | `ab87e214f5a0cb5e3633dca2a2b5808a74415fcd0a268da8727c331763e8a4d6` |
| `evidence/task-8-scoring-evidence-report.md` | `26a67b4470e4f7e82d010a0c5f5d88158f50e4671115e25b688770faa68081f6` |
| `gates/review-fix1/summary.json` | `cfcf1ebeacef0ea65e968fd25546f532ecceecac376daf8ae2f15fd25f0ec976` |
| `parity/review-fix1/verification.json` | `df04fa6fe312ccf7041a64f29b2ff3ce84c46ada5cf20d24921bb4193ac1cacc` |
| Both final semantic projections | `7125a31a9932c95a0df0cf42c505cfe988c29653d9d36f10237f7fa7182be4d8` |

The six owned current file identities are: execution README
`3d99221ada7ffe388eaa60afa1ffafec75bbeb3664765fc5a0fcfcc260a31a39`;
candidate catalog
`b6db4d3143439aacbe6d57970735eaab7d66c789369cdc2f24cf9808b84a0931`;
execution register
`76055025530fd3eb5cd164c5837d3ef759a1f30b1332d537939425c2c31465be`;
Studio research JSON
`784b5204adcfb61cedfb22a2a6c6088a812c8bdbcb52e257aca7489df6bbea5e`;
Studio research Markdown
`b82fbf6f13058a9f7156ccdb00c411bf8bbcf9de9ec5f48e0402335e924df581`;
and `tools/check_completion_execution.py`
`0ca9fdc6a9cea692167cdf940ad838a04c5648d8ad7093aef651a4d592702063`.

## Cannot verify or approve here

Hash agreement establishes byte identity, not independent source authenticity or
historical command authenticity. This review does not reconstruct historical
runtime environments, prove the original frozen-check full input population, or
adjudicate the native diagnostic replay explanation. It does not rerun tests or
independently revalidate product algorithms and probe arithmetic already covered
by the named scoped reviews.

Scientific efficacy/contamination controls/adjudication, full engine adoption,
host authorization/effects and physical fencing/concurrency, physical allocation
ceilings, durable ATP and utilities, save/load migration, ABI owner agreement,
models/OCR/GPU, all-target behavior, installed desktop, actual browser UI/theme/
accessibility and elapsed-operation qualification remain unverified. Final
whole-tranche review and root's final handoff receipt remain separate work. All
163 requirements remain open/C0/unknown.
