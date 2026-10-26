# Substrate release execution — 2026-10-23

The 22-task substrate release is **incomplete**. This record describes accepted local deliverables and pending gates, not production qualification or C0 claim promotion. Work remains uncommitted on the canonical main checkout.

## Delivered and independently reviewed

- Task 1: separate C0 substrate-release inventory, source snapshots, explicit profile routing, coverage anchors, missing-input blockers and a mutation-tested validator. Independent review's four Important findings were fixed and accepted. The historical 163-requirement contract, execution inventory and receipts are unchanged.
- Task 2 preparation: thirteen owner decisions covering P1, authenticated authority, trust/admission, receipts, privacy, read views, query outcomes, compute and deletion. Fifty-six input hashes were verified. Independent preparation review passed. The approval template is empty and non-authorizing. **Actual Task 2 is not complete.**
- Task 3 baseline: root gate, both optional FHE feature checks, private explicit libsodium P1 witness and separate contract/evidence suites passed locally. These do not qualify all platforms, old executable combinations, independent cryptography or production operation.
- Task 7 numerical slice 7a: additive checked AABB/sphere predicates and exact represented-binary-input squared-distance ordering. Independent numerical review approved the implementation; its Minor regression-test improvement was implemented and accepted. **Parent Task 7 is not complete:** durable attachments, frames, causal read views and publication remain open.

The numerical implementation is in `crates/abi-wdbx/src/spatial3d/{shapes,exact}.rs` with exports in `mod.rs` and tests in `crates/abi-wdbx/tests/wdbx_3d_shapes.rs`. Original f64 mantissas/exponents are aligned in exact integer arithmetic. Tests cover closed boundaries, degenerate boxes, subnormals, extreme coordinates, rounded radius sums, exact near-tie ordering and a 600-case independent oracle corpus. Legacy search/storage and fixtures are unchanged.

## Verification boundaries

- Initial root `bash tools/check.sh`: 1,136 Rust tests passed, no failures/ignored; existing warnings retained.
- Initial `full-fhe`: 868 Rust tests passed. Initial `experimental-dghv-bootstrap`: 868 passed. These are separate baseline runs, not production FHE acceptance.
- Existing isolated libsodium P1 witness: 12 Python tests plus 57 selected Rust tests passed; unratified bounded matrix only. The retained PyCA disagreement is not erased.
- Frozen-contract suite: 37 passed; existing execution-register suite: 27 passed; spatial-evidence suite: 16 passed.
- New release validator suite: 39 mutation tests passed. Live/snapshot release checks and both historical validators passed.
- Six geometry targets: 39 Rust tests passed, including the two new test functions and their 600-case oracle loop.
- Root gate after numerical implementation: 1,138 Rust tests passed, no failures/ignored. Independent review then strengthened only one test; that current two-test target passed. Production source hashes did not change. The earlier whole-gate test snapshot and later focused snapshot are distinguished in the task report; a second whole-gate run is not claimed.
- Source preservation check: every initially present WDBX file is byte-identical except the intended additive spatial module entrypoint. No staging, commits, branches, pushes, dependencies, goldens, live stores or credentials changed.

Raw logs, exact source hashes, implementation reports, independent reviews, fix deltas and the controller ledger live in `.superpowers/sdd/2026-10-23-wdbx-complete-substrate-release/`. These are local, ignored execution records; they are not published attestations. Historical spatial source manifests were deliberately not repinned: their old entrypoint hashes do not bind the additive current entrypoint.

## Remaining gates

Read `docs/completion/substrate-decisions/README.md` and its exact proposal/source manifest. Task 2 requires actual named ABI/WDBX owners, normative schema/profile/error/receipt/authority diffs, shared positive/malformed producer-consumer vectors, privacy decisions and the required independent reviews. Those artifacts and approvals are absent. The user was asked to identify the ABI schema owner; no answer or acceptance is inferred.

Only independently specified numerical work proceeded around that gate. No durable P1 integration, custody implementation or publication was performed. The unavailable v5 companion and module-specific behavior-contract gaps remain explicit in the release inventory.

Tasks 4–6 and dependent portions of 7–19 remain unfinished behind their applicable design/approval gates; existing neighboring/reference code is not promoted by this work. Whole-release security acceptance, every required native platform/device combination, multi-host/two-administration qualification and the 30-day Linux/macOS service campaign remain outstanding (Tasks 20–22). This record does not state that all hardware is absent; it states that required qualification has not been delivered.

No public or production release is claimed. Infrastructure changes, purchases, live migrations, actual key destruction and publication retain their separate authorization requirements.
