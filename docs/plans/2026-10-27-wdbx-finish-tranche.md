# WDBX finish tranche (2026-10-27) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every WDBX-local blocker that can be closed without an external owner. That means landing the N02 I1 fix, clearing the N02 minor findings, routing `model` into the substrate release catalog and migrating the 38 refused `lib.rs` pins, so that the release checker goes green on live sources. Then organize the repo's entrypoints and record exactly what remains.

**Architecture:** This tranche contains no new subsystem. Fixes stay inside `crates/abi-wdbx/src/model/` and its independent Python witness. The catalog edit follows the documented mechanism: independent review, edit `catalog-v1.json`, then bump `CATALOG_PIN`. RED is `test_valid`; GREEN is the reviewed pin. Every requirement stays `proposed`/C0.

**Tech Stack:** Rust 1.99.1 (edition 2024, pinned rustup toolchain first on PATH), Python 3 standard library `unittest`, JSON catalogs.

**Spec:** [N02 independent review](../completion/execution/evidence/2026-10-26-source-pin-migration-review/n02-independent-review.md) (findings I1, M1–M8, admission conditions) and [source-pin migration review](../completion/execution/evidence/2026-10-26-source-pin-migration-review/README.md) ("Proposed remaining migration").

## Audit baseline (2026-10-27, HEAD `9d386dd`)

- Gate `bash tools/check.sh` on the dirty tree (prior writer's uncommitted I1 fix included): see Task 1 for the result.
- `check_completion.py`: 163 C0 requirements valid. `check_completion_execution.py`: 163 reconciled open C0. `check_source_custody.py`: verified.
- `check_substrate_release.py` live mode: **fails**, 38 `MOD-*` source identity drifts, all on `crates/abi-wdbx/src/lib.rs` (`ca24c1…` → `afb2a6…`), caused by the uncatalogued `pub mod model;`.
- No first-party `todo!`/`unimplemented!`. All 84 hits are vendored glib under `specimen-studio/native/vendor/`.
- Requirements by owner: WDBX tasks 1–4 = 57; Studio kernel/native (task 8) = 82; ABI/Abbey/Studio-sync/federation (5, 6, 7, 9) = 14; platform operators (10) = 10.

## Decisions (Donald, 2026-10-27, in chat)

- This session takes over the wdbx writer lease (row appended to `organize-2026-10-25/leases.md`).
- Adopt the prior writer's uncommitted I1 fix if the full gate passes. It is preserved first at `.work/2026-10-27-takeover/` (patch sha256 `4529a713…`).
- Route `model` to a release task: add declaration rows and a `FORMAT-model-package` row with open `missing_behavior_blocker`, then migrate the 38 `lib.rs` rows.

## Global Constraints

- Pinned stable Rust 1.99.1, edition 2024; tests run with stdin at EOF (`< /dev/null`).
- Keep the five-crate layering. Add no new dependencies (no `libc`), and keep `unsafe_code` denied.
- `CLAUDE.md` stays the AGENTS.md pointer form (`tools/check_instructions.sh`). Rust files stay within `tools/check_rust_sizes.sh` limits.
- Never repurpose frozen N01/N02 fixtures; new negative vectors are new, versioned files.
- Never regenerate goldens or the catalog automatically. A pin changes only after an independent review recorded in the evidence directory.
- No requirement promotion. Identity pins, gates and local tests do not move anything past C0.
- Work on branch `task/finish-wdbx-20261027`, merge back into local `main`, delete the branch. No push.

## Review Focus

1. A package whose only defect is a tokenizer profile byte must get the same error family (`InvalidTokenizer:profile`) from Rust and the witness. Task 1 pins this via `tokenizer-profile-negatives-v1.json`.
2. `stage` must not report failure for an object that is already durably linked. Task 2 covers this (M3).
3. Section-14 matrix gaps (M6) must be refused by both parsers with the same family. Task 2 adds them as a new versioned negatives file.
4. When a later edit touches a newly pinned model file, the checker must refuse it rather than silently pass. Task 3 tests this.
5. Catalog counts (modules/formats) in README and tests must match the edited catalog exactly. Task 3 covers this.

---

### Task 1: Adopt the N02 I1 fix

**Files:** the six modified files and the new fixture in `git status` at takeover (`model/contract/tokenizer.rs`, `model/package/components/tokenizer.rs`, `tests/model_package.rs`, `tests/fixtures/model-v1/n02-fixtures.md`, `tests/fixtures/model-v1/tokenizer-profile-negatives-v1.json`, `docs/contracts/model/tokenizer-v1.md`, `tools/tests/test_wdbx_model_reference.py`).

- [ ] Step 1: Full gate on the dirty tree with the pinned toolchain and an isolated `CARGO_TARGET_DIR`. Expected: `check: all green`.
- [ ] Step 2: Create branch `task/finish-wdbx-20261027` and commit exactly those seven paths (explicit `git add` paths) as "fix(model): align tokenizer profile byte errors with witness (N02 I1)". State in the body that it is the preserved prior-writer work.

### Task 2: N02 minor findings

**Files:**
- Modify: `crates/abi-wdbx/src/model/package/stage.rs` (M3)
- Create: `crates/abi-wdbx/tests/fixtures/model-v1/n02-matrix-negatives-v1.json` (M6)
- Modify: `crates/abi-wdbx/tests/model_package.rs`, `tools/tests/test_wdbx_model_reference.py` (M6 assertions)
- Modify: `docs/contracts/model/tokenizer-v1.md` and the package contract doc (M1, M2, M4, M5 documented precedence/preconditions)
- Modify: `docs/plans/2026-10-25-native-model-cpu.md`, `docs/contracts/model/README.md` (M8 stale status)

- [ ] Step 1 (M3): Write the test `stage_reports_success_when_only_temporary_unlink_fails`. It uses a `Durability` test double, or a pre-removed temporary, to make the post-link unlink fail, and asserts `Ok(StagedObject{created: true})` with the object present and verifiable. Run it and confirm it fails.
- [ ] Step 2 (M3): After a successful link and directory sync, treat a temporary-unlink failure as best-effort: ignore the error, still sync the directory and return `Ok`. Run the test and confirm it passes.
- [ ] Step 3 (M6): Add versioned negatives for every M6 item that the frozen inference fixture can express by byte edit plus recomputed hashes: rng `cursor == count`, objective profile/policy tag ≠ 1, component 5/7 dataset/split/preparation mismatch, NaN/Inf first moment, and decoder-side dtype mismatch. Each vector records `{name, base, edits, expected_kind, expected_field}`. Assert all of them in both the Rust corpus test and the witness test, then run both and confirm they agree. If a vector shows disagreement, stop and report it as a new finding; do not pick a side silently. Record held-out-sentinel trainer input and extra legacy readers as follow-ups unless they fit in an existing test.
- [ ] Step 4 (M1/M2/M4/M5): Document in the contract docs the zero-split-digest family per entry point, the within-component trailing-bytes precedence, the caller-owned private directory precondition for `stage`, and that the directory hash is checked before entry canonicality.
- [ ] Step 5 (M8): Update the two stale status documents to say N02 is on main as public `abi_wdbx::model::{package,tokenizer}`, reviewed APPROVE WITH REQUIRED FIXES, with I1 fixed in Task 1.
- [ ] Step 6: `cargo test --locked -p abi-wdbx --test model_package < /dev/null` and `python3 -m unittest discover -s tools/tests -p test_wdbx_model_reference.py` pass. Commit.

### Task 3: Independent review, then route `model` and migrate the `lib.rs` pins

**Files:**
- Create: `docs/completion/execution/evidence/2026-10-26-source-pin-migration-review/n02-fixes-review.md`
- Modify: `docs/completion/substrate-release/catalog-v1.json`, `tools/check_substrate_release.py` (`CATALOG_PIN`), `docs/completion/substrate-release/README.md` (counts), count-dependent tests in `tools/tests/test_check_substrate_release.py`
- Modify: `docs/completion/execution/evidence/2026-10-26-source-pin-migration-review/README.md` (applied migration record and new drift report)

- [ ] Step 1: Dispatch an independent read-only reviewer (fresh context, no conclusion supplied) over `9d386dd..HEAD`, limited to Task 1 and Task 2 changes. The verdict must say whether condition 1 of the N02 admission conditions is met. Record it verbatim in `n02-fixes-review.md`. Proceed only on APPROVE; fix and re-review otherwise.
- [ ] Step 2: Add catalog rows for `pub mod model;` (`crates/abi-wdbx/src/lib.rs`), the six `pub mod` declarations in `crates/abi-wdbx/src/model/mod.rs`, and a `FORMAT-model-package` row whose source file defines the package framing, in the existing row shape: `classification: declaration-coverage-only`, owner `WDBX compatibility maintainer`, the matching release task, and an open `missing_behavior_blocker`. Update the 38 `lib.rs` rows to `afb2a62420f0fa8632248ddda78d9329331efc22d50cf40e50a09d0d9bcf9210` and shift line anchors by +1 for the rows after `model` (`MOD-0048`..`MOD-0069`). Run `test_valid` and confirm it is RED with "catalog changed".
- [ ] Step 3: Dispatch a second independent reviewer over the catalog diff alone. On APPROVE, set `CATALOG_PIN` to the new digest and update README counts and count-dependent tests.
- [ ] Step 4: `check_substrate_release.py` live exits 0, `--snapshot-only` exits 0, `--drift-report` reports no drift, and the release unittest suite passes. Add a test that edits a pinned model file in an isolated copy and asserts live mode refuses. Save the drift report JSON next to the earlier one and update the migration README. Commit.

### Task 4: Organize entrypoints and record remaining work

**Files:**
- Modify: `README.md` (add a short "Where things stand" index near the top; existing claims prose is unchanged)
- Modify: this plan (record its execution outcome)

- [ ] Step 1: The README index links the completion register, execution register, this plan, the gates (`tools/check.sh`, completion/release checkers, Studio's own gates), the 7 external gates, and the remaining-work table below. `bash tools/check_instructions.sh` passes.
- [ ] Step 2: Full gate on the exact branch head, plus completion, execution, custody and release checkers. Merge `--ff-only` into local `main`, delete the branch, and append a lease release row. Commit.

## Remaining work after this tranche (not done here, with reason)

| Work | Requirements | Why not in this tranche |
| --- | --- | --- |
| Task 2 vNext envelope, COSE detached signature, key admin | P02.*, C01–C16 | Step 1 requires agreeing the new domain and typed envelope with the ABI schema owner before writing a serializer (external gate: ABI normative schema approval). Production claims also need independent crypto review. |
| Task 3 admission/lifecycle/erasure qualification | P03.*, C17–C21 | Much is implemented (EpisodeStore admission, holds, deletion, gc). The fault-injection boundary matrix and backup-quarantine restore are the next local slices, depending on task 2's envelope. |
| Task 4 eight-dimension retrieval and frozen baseline | P04.*, E01–E08 | Depends on tasks 2–3. Held-out efficacy is the research-experiment external gate. |
| `model` behavior beyond identity pins (N03+) | MOD/FORMAT `missing_behavior_blocker`s | Needs frozen expected/refused observations and a direct independent experiment. No training is authorized. |
| Studio kernel/native | 82 task-8 requirements | Separate workspace (`specimen-studio/`) with its own guide and gates. Should be its own plan. |
| ABI/Abbey consumers, federation, Studio sync, platforms | tasks 5, 6, 7, 9, 10 | Owned by other repositories or operators. Needs separate-host federation, signing receipts, staging drills and elapsed C7 operation. |
| HawkScan | — | Documented missing prerequisites (`HAWK_API_KEY` unset). Reported skipped. |
