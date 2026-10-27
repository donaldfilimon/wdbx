# WDBX completion tranche 2 (2026-10-27) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the WDBX-local completion slices that no external owner blocks. The slices cover:

- Task 3 lifecycle invariants and anti-resurrection.
- Task 4 admission-first retrieval and deterministic rebuild.
- Task 2 private P1 vectors, parse parity and the trust-command decoder.
- N02 M6/M7 fixtures.
- Federation and outbox format witnesses.
- Studio S27/S28/S37/S43 direct evidence.
- Execution-register hardening and mapping.
- Gate documentation.

All of this is implementation, tests and evidence only. Every requirement stays `proposed`/C0. The execution register keeps every row at `open`/`C0`/`unknown`.

**Architecture:** No new subsystem. Code tasks come first, and each one prefers new files to edits of pinned files. The following ownership rules apply:

- When a code task edits a register-pinned file, it refreshes only that reference's `sha256` in `register-v1.json`, in the same commit. Path sets do not change, so `CANDIDATE_PATHS_SHA256` stays the same. The task records a `reference-refresh.json` entry under its evidence directory.
- Path-set and disposition changes are batched into two serialized register tasks, Task 19 and Task 20. Each one has a single reviewed pin bump.
- All edits to catalog-pinned sources are batched into one reviewed `catalog-v1.json`/`CATALOG_PIN` migration, Task 21. Until then `check_substrate_release.py` live mode is expected to report drift for exactly the listed paths. Nothing merges to `main` before Task 21 is green.

**Tech Stack:** Rust 1.99.1 (edition 2024, pinned rustup toolchain first on PATH), Python 3 standard library `unittest` (3.9 floor), Bun/TypeScript for `specimen-studio/`, and JSON catalogs.

**Spec:** Sources:

- [requirements](../completion/requirements.json), [plans](../completion/plans.json), [designs](../completion/designs.json)
- [execution register](../completion/execution/register-v1.json)
- [P1 design](../superpowers/specs/2026-10-20-wdbx-cose-trust-prototype-design.md) slices 1–3
- [p1-trust-slice](../completion/execution/p1-trust-slice.md)
- [native package design](../contracts/model/native-package-design-v1.md) §§6, 12, 14
- [N02 fixtures](../../crates/abi-wdbx/tests/fixtures/model-v1/n02-fixtures.md) "Remaining M6 items"
- [inhibition contract v2](../completion/inhibition-contract-v2.md)
- [atp-v3 profile](../completion/execution/atp-v3-profile.md)
- [prior tranche](2026-10-27-wdbx-finish-tranche.md)

## Audit baseline (2026-10-27, HEAD `19db4fa`, read-only)

- Branch `task/finish-wdbx-20261027`, clean tree at audit time. The prior tranche recorded its outcome in `fe5bc02` and `19db4fa`.
- `check_completion.py`: 163 C0 valid, 46 cases. `check_completion_execution.py`: 163 reconciled open C0. The live split is 131 candidate / 16 local_gap / 16 external_gap. The "95/68" figure in `progress.md:11-12` is historical.
- Tests observed by scouts:
  - Model suites: `model_contracts` 5, `model_package` 17, `model_package_bounds` 16, `model_tokenizer` 7, model witness 16 OK.
  - Task-04 targets: 56 tests over 5 targets.
  - `specimen-kernel`: 260 passed.
  - Studio `bun test`: 221 pass, run against an unverified prebuilt WASM artifact.
  - Not run: `bun run test:native`, `bun run check`, `check:native`, `test:browser`.
- Pin classification of files this plan edits (grep of the register and the catalog):

| File | Register-pinned | Catalog-pinned |
| --- | --- | --- |
| `v3/ranking/adapter.rs` | yes (C14) | no |
| `v3/ranking/mod.rs` | no | yes |
| `v3/mod.rs` | no | yes (≥7 MOD rows) |
| `v3/episode/store.rs` | no | yes (FORMAT-v3-episode-ledger) |
| `v3/episode/store/deletion/index.rs` | no | yes (FORMAT-deletion-sidecar) |
| `v3/episode/store/deletion.rs` | yes | no |
| `v3/episode/store/expiry.rs` | no | no |
| `complete_episode/crypto/{mod,tests,python_witness_tests}.rs`, `crypto/trust/mod.rs` | yes | no |
| `tools/wdbx_complete_p1_crypto.py`, `tools/tests/test_wdbx_complete_p1_crypto.py` | yes | no |
| `docs/completion/execution/p1-trust-slice.md` | yes | no |
| `tools/check_rust_sizes.sh` | yes (G12 inspected) | no |
| `tools/check.sh`, `docs/agent-guide.md`, model test files, `n02-fixtures.md`, `use-studio.ts` | no | no |

- Size headroom: `store.rs` 986/1000, `offline/outbox.rs` 868/1000, `crypto/trust/tests.rs` 764/1000, `crypto/tests.rs` 404/1000, `model_package.rs` 530/1000.

## Global Constraints

- **Precondition:**
  - The prior finish tranche is merged into local `main`, and its lease release row is in `organize-2026-10-25/leases.md`.
  - This tranche's writer appends its own lease row before the first write.
  - Work on one task branch from local `main`. Rebase before every register or catalog task.
  - No push.
- **Pinned environment:** every cargo command runs with `export PATH="$(dirname $(rustup +1.99.1 which rustc)):/usr/bin:$PATH"` and `export CARGO_TARGET_DIR=<scratchpad>/wf-target-<task>`, and stdin is at EOF (`< /dev/null`). Every Python command runs with `PYTHONDONTWRITEBYTECODE=1`. Do not stage `__pycache__`.
- **No promotion:**
  - `requirements.json` statuses and evidence levels never change.
  - Every register row stays `status: open`, `evidence_level: C0`, `test_result: unknown`, with that literal value.
  - Observed counts go only into evidence READMEs.
  - Existing blocker strings stay byte-identical. New notes are added as new blocker strings.
- **Traceability labels only:** `wdbx-model-package-v1`, `V6-*`, `PLAN-*`, `FORMAT-*`, and ABI `C24`/`C26` are not `requirements.json` IDs. Cite them as design or catalog traceability, and never as register advancement.
- **New files over pinned edits:**
  - Frozen fixtures, goldens and versioned vector files are never edited. New negatives go in new versioned files.
  - Goldens regenerate only behind an explicit opt-in env var. `tools/check.sh` must `unset` every such variable.
- **No new crates and no `unsafe`:** HMAC is built on the existing `sha2` dependency.
- **Studio:**
  - Studio gates run from `specimen-studio/` (`scripts/pinned-cargo.sh`, `bun`). Root gates never qualify Studio.
  - Mutants run only in a scratchpad copy.
- **Opt-in P1 crypto witness:**
  - It needs the isolated PyNaCl 1.6.2/libsodium venv. The known path is `/tmp/wdbx-p1-libsodium.0EvVu6/venv/bin/python`.
  - If the venv is gone, record the step as **unexecuted**. Recreating it downloads PyNaCl, which needs explicit user approval. Never skip silently.
  - `tools/check_p1_crypto.sh` omits `--locked`, so also run the `--locked` form directly.
- **Stop on disagreement:** if a new cross-language vector shows a disagreement, stop, record it as a finding in the task's evidence, add a failing test, and fix only after review. Never pick a side silently.
- **Reviews:** an independent read-only reviewer (fresh context, no conclusion supplied) must record a verdict before any `CANDIDATE_PATHS_SHA256`, `REVIEW_ARTIFACT_PATHS_SHA256` or `CATALOG_PIN` bump.
- **Exclusions:** do not touch root `README.md` (live-pinned release source), `catalog-v1.json` (outside Task 21), or `crates/abi-wdbx/src/model/**`.

## Review Focus

1. **P1 category parity** (Tasks 6–7). Rust maps payload `NonMinimalArgument`/`NonCanonicalEncoding` to `NonCanonicalEncoding` (`crypto/profile.rs:201-203`). Python maps them to `InvalidPayload` (`tools/wdbx_complete_p1_crypto.py:186-190`). Python's Sig_structure size bound (`py:195`) is conservative, while Rust's is exact near 1,049,088. Python protected-header `expect` errors need to collapse to `InvalidHeaderSet` as in Rust (`profile.rs:131-143`). Each of these must surface as a recorded finding with a failing test before either side changes.
2. **Stale-generation refusal** (Task 15). A refused open must mutate no byte. A store whose sidecars are honestly lost must still open. A target that has been forgotten and then reclaimed must still open.
3. **Retention re-bind semantics** (Task 3). Last-wins is pinned as observed behavior, not "fixed". The "bound once" doc contradiction is recorded in the evidence. It is not resolved silently.
4. **Rejection invariance** (Task 2). `apply_gc_plan` with only ineligible digests returns `Ok` with unchanged bytes; it is not a refusal. A forget of a held target is admitted on purpose.
5. **Outbox dangling `depends_on`** (Task 10). `open()` accepts it today. Record this as a finding test, not as a refused vector.
6. **S28 crystallization** (Task 11) is the inferred predicate `strength == max && base >= threshold`. There is no stored field. The reviewer may withdraw S28 routing, as was done for S32.
7. **Register/catalog identity** (Tasks 19–21). Pins change once per task after review. Counts in the execution README and in `test_readme_counts_match_catalog` match the register exactly.

---

### Task 0: Preconditions and baseline

**Files:** `organize-2026-10-25/leases.md` (append only).

- [ ] Step 1: Confirm `git log main` contains `19db4fa`, that the prior lease release row exists, and that `git status --short` is clean. If the precondition fails, stop.
- [ ] Step 2: Append this tranche's lease row. Create the task branch from local `main`.
- [ ] Step 3: Record the baseline in `.work/tranche2-baseline/` (ignored):
  - `bash tools/check.sh < /dev/null` must end with `check: all green`.
  - `python3 tools/check_completion.py`, `python3 tools/check_completion_execution.py`, `python3 tools/check_source_custody.py` and `python3 tools/check_substrate_release.py` (live, `--drift-report`) all pass.

### Task 1: Harden the execution checker (review and artifact identity, README counts)

**Requirements:** G13, P01.01 (C0).

**Files:**
- Create: `docs/completion/execution/review-artifact-paths-v1.json`, `docs/completion/execution/evidence/2026-10-27-register-hardening/` (logs and reviewer verdict).
- Modify: `tools/check_completion_execution.py` (new `REVIEW_ARTIFACT_PATHS_SHA256`, review/artifact multiset identity, catalog closure over the 163 IDs, README count parser), `tools/tests/test_check_completion_execution.py`, `docs/completion/execution/README.md` (only if a count statement is reworded), `docs/completion/execution/progress.md` (append a superseded note; do not rewrite the dated 95/68 line).

- [ ] Step 1 (RED): Add these tests to `tools/tests/test_check_completion_execution.py`:
  - `test_review_reference_cannot_be_added_without_catalog`: append a hashed `2026-10-26-p1-review` ref to G01.
  - `test_artifact_references_cannot_be_stripped`: strip C14's non-plan artifacts and keep `plans.json`.
  - `test_gap_rows_cannot_gain_review_refs`
  - `test_review_artifact_catalog_requirement_closure`
  - `test_review_artifact_catalog_hash_is_independently_pinned`
  - `test_readme_counts_match_catalog`

  Run `python3 -m unittest discover -s tools/tests -p test_check_completion_execution.py` and confirm the new tests FAIL.
- [ ] Step 2: Resolve the README "14 historical review/receipt pointers" semantics before encoding them. The register holds 21 unique review paths, 7 of them under `evidence/2026-10-23-scoring/`. Encode the rule explicitly: 14 historical paths outside the current scoring reviews, 21 in total. Either add a separate bullet for the 7 current reviews or record the stale count as a finding. Do not change 14 to 21 blindly. The parser compares bullet counts and expands `P05.01–P05.04`/`OS01–OS06` ranges, or compares counts only. State which.
- [ ] Step 3 (GREEN): Generate `review-artifact-paths-v1.json` exactly from the live register. Implement the checks. Do not edit `register-v1.json`.
- [ ] Step 4: Have an independent reviewer verify that the generated catalog pins the status quo. Record the verdict. Set `REVIEW_ARTIFACT_PATHS_SHA256`.
- [ ] Step 5: All of these pass, and the logs are stored in the evidence directory:
  - `python3 tools/check_completion_execution.py`
  - the unittest above
  - `python3 tools/check_completion.py`

  Commit as "chore(execution): pin review/artifact identities and README counts".

### Task 2: Rejected-write and governance-refusal invariance (test only)

**Requirements:** P03.01, P03.04, C20 (C0).

**Files:**
- Create: `crates/abi-wdbx/tests/v3_rejection_invariance.rs` and `crates/abi-wdbx/tests/support/rejection_invariance.rs`. Reuse the `store_bytes` pattern from `tests/support/quota_windows.rs`.

- [ ] Step 1: Write `every_propose_write_refusal_changes_no_byte`. Each row names its setup and asserts:
  - `propose_write` returns `Err` with the same reason as `decide_write`.
  - `store_bytes` (every sidecar included), `retrieve()` receipts, `guild_usage` and `quota_state` are identical before and after.

  Rows:
  - Quiet
  - StaleBinding: policy, consent and contract variants
  - LearningDisabled
  - Replay by request id
  - InvalidInput: unbounded id, malformed candidate, zero-TTL candidate
  - InvalidTransition: supersede of a held target, supersede of a forgotten target or tombstone, forget of an already-forgotten target
  - CommitmentMismatch
  - TokenBudget
  - StorageBudget

  If a row is unreachable, report it as a finding. Do not drop it.
- [ ] Step 2: Write `every_governance_refusal_changes_no_byte`. Use each entry point's real reasons and check every variant against the source. For every row, assert `retrieve()`, the relevant sidecar state getters, and that bytes are unchanged.
  - `commit_hold`: a different authority already holds the hold.
  - `release_hold`: the hold is not live, or `may_release` fails (`holds/mod.rs:492-513`).
  - `commit_redaction` and `commit_object_key_destruction`: the target is live. For `commit_object_key_destruction`, also a key already bound elsewhere.
  - `commit_quota_adjustment`: above the host policy, or a budget lowered below usage (InvalidInput).
  - `clear_quota_adjustment`
  - All of the above: unserved guild (StaleBinding), unauthorized principal, and a service or human-subject class.
  - `apply_gc_plan`: refused for an unserved guild (StaleBinding) or a bad `guild_ref` (InvalidInput).
- [ ] Step 3: Write `idempotent_no_write_outcomes_change_no_byte`. It covers `already_live`, `already_revoked`, `already_requested`, and `apply_gc_plan` with only ineligible digests. That last case gives `Ok`, an empty `reclaimed` list and `bytes_before == bytes_after` (`gc.rs:556-569`).
- [ ] Step 4: Write `a_refused_write_then_an_accepted_write_matches_a_clean_store`.
  - Use a non-TTL accepted write. `bind_retention` reads `expiry::now_ms`.
  - Replay the same prior accepted writes into the fresh store.
  - Compare files by path relative to each store root.
- [ ] Step 5: Run `cargo test --locked -p abi-wdbx --test v3_rejection_invariance < /dev/null`. All rows pass on the current code. If a row writes the ledger or the retention sidecar, stop and report a finding; do not weaken the assertion. Then `bash tools/check.sh < /dev/null` must be green. Commit.

### Task 3: Named crash-boundary matrix (test only; pin retention last-wins)

**Requirements:** P03.04, P03.02, C18, C19 (C0).

**Files:**
- Create: `crates/abi-wdbx/tests/v3_lifecycle_fault_matrix.rs`, `docs/completion/execution/evidence/2026-10-27-task03-fault-matrix/README.md`.
- No edits to `store.rs`, `expiry.rs` or any other `src/` file.

- [ ] Step 1: Write the rows. Each one composes pre-state and post-state file bytes, reopens twice and byte-compares. The probe is reproduced as a named row in this file, not cited.
  - `retention_stamp_synced_ledger_line_missing`:
    - An orphan stamp leaves `retention_state` and `due_expirations` unchanged.
    - A retry with the same digest gives an effective deadline after reopen equal to the retry's stamp (pins last-wins, `expiry.rs:238,286`), and equal to the in-memory value.
    - Two stamp lines are present.
  - `retry_after_crash_between_retention_and_ledger_extends_deadline`: the probe, as a row.
  - `ledger_tail_partial`: builds on `v3_episode_store.rs:438,471`, adding only a second reopen with byte equality.
  - `gc_reclaimed_committed_rename_blocked_then_reopen_then_retry`:
    - Block `gc.tmp` as at `v3_deletion_lifecycle.rs:655`, drop the store, reopen, re-plan, then `apply_gc_plan` returns `Ok`.
    - Compare only the ledger bytes and the single-line reclaimed index against an uninterrupted run. Both runs are built from one copied pre-crash snapshot.
  - `gc_temp_left_after_full_write`
  - `sidecar_tmp_left_behind`: stale `.episodes.v1.object_keys.jsonl.tmp` (`store/deletion/index.rs:368`) and `.episodes.v1.quotas.jsonl.tmp` (`store/quota/index.rs:506`).
  - `tombstone_never_resurrected_after_each_row`: supersede and forget of the erased digest give `InvalidTransition`.
- [ ] Step 2: Run `cargo test --locked -p abi-wdbx --test v3_lifecycle_fault_matrix --test v3_retention_expiry --test v3_retention_windows --test v3_deletion_lifecycle < /dev/null`. All pass.
- [ ] Step 3: The evidence README holds:
  - the row table: boundary, simulated fault, before and after digests, reopen-1 equal to reopen-2;
  - the last-wins vs "bound once" (`store.rs:715-720`) contradiction, recorded as an open design question;
  - the statement that file-composition simulation is not proof of power-loss or hardware durability;
  - confirmation that none of the rows depends on the task-02 envelope;
  - the requirements staying proposed/C0.

  `bash tools/check.sh < /dev/null` green. Commit.

### Task 4: Task-04 disposition matrix residual (test only)

**Requirements:** P04.01. E05/E06 only where a case asserts that dimension. C0.

**Files:**
- Create: `crates/abi-wdbx/tests/v3_query_disposition_matrix.rs` with its own helpers, under 1000 lines.

- [ ] Step 1: First run a probe test that observes whether `EpisodeStore::open` accepts a policy missing the record's guild. Then pin the observed behavior. Do not branch at runtime.
- [ ] Step 2: Write these tests:
  - `stale_contract_is_uncertain_with_binding_contract_stale`: change only the reopen policy's `contract_revision`/`contract_digest` and append nothing after it. Expect `ContractStale`, `Reason::BindingContractStale` and `Uncertain`.
  - `stale_policy_reason_is_binding_policy_stale`
  - `unbound_guild_binding_reads_unknown_or_is_refused`: the pinned outcome from Step 1.
  - `incomplete_lineage_superseder_forgotten_stays_visible`: assert on digests. The superseded record's lineage still names the absent superseder, which is a dangling reference by design.
  - `wrong_regime_proxy_reason_is_no_regime`: name and doc state that this is the C0-maturity proxy, not a measured regime.
  - `answer_scope_equals_queried_guild_and_unserved_guild_is_pinned`
- [ ] Step 3: The module doc cites existing coverage as consolidation, not new evidence:
  - `v3_constitutional_query.rs:585-618` (unverifiable signature)
  - `:794` and `v3_constitutional_query_holds.rs:621-647` (foreign guild)
  - `v3_retrieval_baseline.rs` and the fixture README
- [ ] Step 4: Run `cargo test --locked -p abi-wdbx --test v3_query_disposition_matrix < /dev/null`, then `bash tools/check.sh < /dev/null`, then `python3 tools/check_completion_execution.py`, which must show no drift. Commit.

### Task 5: N02 M6/M7 fixtures (dtype negatives, held-out sentinel with hand-derived BPE, legacy-reader probes)

**Traceability:** `wdbx-model-package-v1`, V6-1215, PLAN-0073, V6-1403, and ABI C24/C26 via design §12. These are labels only; no register row changes.

**Files:**
- Create:
  - `crates/abi-wdbx/tests/fixtures/model-v1/n02-dtype-negatives-v1.json` (`format: "native-model-n02-dtype-negatives-v1"`)
  - `crates/abi-wdbx/tests/fixtures/model-v1/bpe-trainer-vectors-v2.json`
  - `crates/abi-wdbx/tests/model_legacy_readers.rs`
- Modify:
  - `crates/abi-wdbx/tests/model_package.rs` (one new test; existing tests untouched)
  - `crates/abi-wdbx/tests/model_tokenizer.rs`
  - `tools/tests/test_wdbx_model_reference.py`
  - `crates/abi-wdbx/tests/fixtures/model-v1/n02-fixtures.md`

  `n02-fixtures.md` is touched only after the prior writer's lease is released. In it, record the v1 dtype and v2 BPE digests, the worksheets and provenance, and narrow the "Remaining M6 items" sentence at :79.

- [ ] Step 1 (dtype negatives): Build the vectors with a standalone scratch `struct`/`hashlib` worksheet that imports neither the witness nor the Rust writer. Each vector records `{name, base, edits, artifact_sha256, hex, expected_kind, expected_field}`.
  - (a) `dtype-unnamed`: dtype byte 3. Expect `NonCanonical`. Compare the kind across languages; compare the field only once confirmed.
  - (b) `dtype-f32-as-f64`: payload widened exactly from F32 to F64, length doubled. It passes the data-length and finite checks and fails `semantic-axes`, giving `InvalidTensor`.
  - (c) `dtype-relabel-length-unchanged`: fails the data-length check. Rust reports `tensor-data-length` and the witness reports `data-length`. Map these names explicitly.
- [ ] Step 2: Write `n02_dtype_negatives_have_the_same_family_as_the_witness` (Rust) and the matching witness test. Both pin the file digest. The witness test compares the parsed header, the directory entries and each component, and asserts that only the declared fields differ: component 3's length and hash, the offsets of components 4–8, `total_bytes` and the directory hash.
- [ ] Step 3 (hand-derived BPE v2):
  - Two or three new corpora, each with at most 4 merges, a tie and an overlapping run, and requested vocabulary `260 + merges`. Do not restate `["aaaa","aaaa"]` or `["abab","cdcd"]`.
  - Write the merges, result IDs, encode IDs and corpus-digest preimage layout step by step in `n02-fixtures.md`, from design §6.
  - Compute the SHA-256 values with a hashlib worksheet only.
  - Omit `visits` unless it is hand-derived.
  - Use split digest `[7;32]`.
- [ ] Step 4 (sentinel):
  - Write `held_out_sentinel_bytes_never_enter_trained_vocabulary`. The training documents contain none of `q`, `z`, `x`, `v`. The absence check runs over merged rows (ID ≥ 260) only. `corpus_digest` covers the training documents only.
  - Write `sentinel_positive_control_produces_pinned_sentinel_merges`, required: the `qzxvqzxvqzxv` corpus gives hand-derived pinned merges.
  - The notes state that hand derivation is not author-independent (M7 stays open), that this is not split-isolation enforcement (N04), and that C26 stays C0.
- [ ] Step 5 (legacy readers): Write `model_legacy_readers.rs` with these tests:
  - `model_bytes_are_refused_by_every_public_byte_decoder`:
    - all six valid fixtures through `v2::open_object` (both flag values)
    - `v3::episode::cose_prototype::decode_container`
    - `CanonicalValue::from_canonical_bytes` and `EpisodeCommitment::from_canonical_bytes` (confirm the public path)
    - `PrototypeEpisode::from_canonical_bytes` (doc-hidden prototype; note this)

    Assert `is_err()` and no panic. Do not pin variants unless they have been observed.
  - `model_bytes_in_a_fresh_tempdir_are_not_loaded_as_data`:
    - `segment::read_segment`, `wal::Wal::read`, `store::load` (`StorePaths::new(tempdir)`), `v2::read_v2_root`
    - Freeze the observed result per reader. An `Ok(empty)` documents the existing missing-manifest behavior and is not a refusal.
  - `legacy_bytes_are_refused_by_model_framing`: a plaintext `v2::seal_object` output and `tests/golden/cose-sign1-p1/*` bytes. Pin the exact framing error per input.

  The module doc states that this is not Specimen-reader coverage (Studio-owned) and that V6-1067 is a non-claim.
- [ ] Step 6: Run `cargo test --locked -p abi-wdbx --test model_package --test model_tokenizer --test model_legacy_readers < /dev/null`. All pass with 0 filtered: model_package ≥18, model_tokenizer ≥9. Then:
  - `python3 -I -m unittest discover -s tools/tests -p test_wdbx_model_reference.py` OK, ≥17.
  - `bash tools/check.sh < /dev/null` green.

  If Rust and the witness disagree, stop and record a finding. Commit.

### Task 6: Private full-payload P1 vectors v1 and stage-split category parity

**Requirements:** P02.02, P02.03, C10 (C0).

**Files:**
- Create: `crates/abi-wdbx/tests/golden/cose-sign1-p1-full-payload/v1.json` and its `README.md`.
- Modify (register-pinned; refresh only these references' sha256 in the same commit, plus `evidence/2026-10-27-p1-vectors/reference-refresh.json`): `crates/abi-wdbx/src/v3/complete_episode/crypto/tests.rs`, `.../crypto/python_witness_tests.rs`, `tools/tests/test_wdbx_complete_p1_crypto.py`.

- [ ] Step 1: Write `v1.json`. It is labelled private, provisional and unratified: slice-1/2 vectors, not the slice-5 ABI-shared vectors, and qualifying nothing.
  - Each entry's `expected` is either `{"stage":"parse","category":<ProfileError>}` or `{"stage":"verify","reason":<VerificationReason>}`.
  - **Positive:** a deterministic synthetic seed, plus payload, protected, container and Sig_structure hex with their sha256 values.
  - **Parse malformed:**
    - every truncation boundary, a trailing byte
    - duplicate or unknown header label, tag ≠ 18, non-empty unprotected map, attached payload
    - non-minimal or indefinite length, kid length 71 or 73, non-ASCII kid
    - a 63-byte signature (parse stage)
    - container, payload and protected cap+1, written as generator descriptors (`{base, mutation:"extend_payload_to", length}`), not literal hex
  - **Named downgrade:**
    - `alg -8` substitution
    - payload version 0 and 2
    - payload domain mismatch: define precisely as either parse-stage payload content or verify-stage `DomainMismatch` with explicit context
    - the legacy transplant from `crates/abi-wdbx/tests/golden/cose-sign1-p1/container.hex`
    - Ed25519ph, referenced read-only from `docs/completion/execution/evidence/2026-10-22-continuation/libsodium-dependencies/ph-vector.json`: verify stage, `InvalidSignature`
  - Tampered r/S values are verify stage.
- [ ] Step 2 (RED): Write `crypto::tests::p1_full_payload_vectors_v1_pin_exact_bytes_and_categories`. This is the default build with no Python. `sign_p1` reproduces the positive bytes. `parse_p1` returns the exact `ProfileError` for each parse-stage entry. `verify_p1` returns the exact `VerificationReason` for each verify-stage entry.
  - Run `cargo test --locked -p abi-wdbx --lib v3::complete_episode::crypto < /dev/null`.
  - Any entry where Rust differs from the expected value is a finding. Record it, and do not edit the expectation to match.
- [ ] Step 3: Replace the `ok==false`-only check in `python_witness_tests.rs:120-141` in place with `malformed_and_downgrade_vectors_refuse_with_identical_categories`. It compares Python `error` for the parse stage and Python `reason` for the verify stage. The old fixed indices stay as regression entries. Add `test_frozen_vectors_v1_categories` to the Python suite.
- [ ] Step 4: Run the opt-in gate:
  - `WDBX_CRYPTO_PYTHON=<isolated PyNaCl 1.6.2 venv python> WDBX_CRYPTO_BACKEND=libsodium bash tools/check_p1_crypto.sh`
  - `cargo test --locked -p abi-wdbx --features p1-crypto-witness --lib v3::complete_episode::crypto < /dev/null`

  The expected first outcome is a recorded divergence on the noncanonical-payload entry (Review Focus 1), plus possibly one on the Sig_structure bound. Record each in `evidence/2026-10-27-p1-vectors/findings.md` with its failing test marked expected-fail. Leave both witness and profile unchanged pending review. If the venv is unavailable, record the step as unexecuted.
- [ ] Step 5: Legacy pins unchanged: `cargo test --locked -p abi-wdbx --test v3_canonical_commitment --test v3_episode_cose --test v3_cross_language_cose < /dev/null`. Then `bash tools/check.sh < /dev/null` green and `python3 tools/check_completion_execution.py` valid after the hash refresh. Commit.

### Task 7: Stdlib-only `parse` op so P1 parser parity runs in the default gate

**Requirements:** P02.03 (C0). **Depends on:** Task 6 and its reviewed findings.

**Files:**
- Create: `crates/abi-wdbx/src/v3/complete_episode/crypto/witness_support.rs` (`#[cfg(test)]`, not feature-gated; shared hex, script, interpreter and subprocess helpers).
- Modify (register-pinned; hash refresh): `crypto/mod.rs` (module declaration), `crypto/tests.rs`, `crypto/python_witness_tests.rs` (use the shared helpers), `tools/wdbx_complete_p1_crypto.py`, `tools/tests/test_wdbx_complete_p1_crypto.py`.
- Modify: `tools/check.sh`. Add one narrow step that runs only the `TestParseOpNoBackend` class.

- [ ] Step 1: Decision recorded in the evidence:
  - The Python witness changes. It wraps each protected-header `expect` call to raise `InvalidHeaderSet`.
  - The payload mapping follows the Task 6 review ruling: align Python to `NonCanonicalEncoding`, or explicitly exclude payload-noncanonical vectors.
  - Witness behavior changes are recorded as such.
- [ ] Step 2 (RED): Write the Python class `TestParseOpNoBackend` with:
  - `test_parse_op_needs_no_crypto_dependency`: patch `sys.modules` (`cryptography`, `nacl` set to `None`) and `importlib.metadata.version` to raise. Parse answers. Verify and sign return their observed backend-unavailable outcome, checked before asserting an exit code.
  - `test_parse_response_has_no_valid_field_and_carries_scope_marker`
  - `test_protected_header_type_error_is_invalid_header_set`: e.g. `protected[0]=0x1c`.

  setUp does not call `backend_identity()`. Confirm the new tests FAIL.
- [ ] Step 3 (GREEN): Add the op `{op:"parse", container_hex, payload_hex}`. It never calls `_crypto()` and returns `{ok, error}` or `{ok:true, kid, protected_hex, sig_structure_sha256, scope:"parse-only, not signature verification"}`. The digest is hashlib over `_structure()`.
- [ ] Step 4: Write `crypto::tests::default_gate_parser_parity_over_frozen_v1_vectors_and_single_byte_flips`. It covers the v1 parse-stage vectors, every position × all 255 replacement values of the positive container, and every truncation length `0..len-1`. Interpreter resolution is `WDBX_CRYPTO_PYTHON`, then `WDBX_PYTHON`, then `python3`. Also write `a_missing_parse_interpreter_is_a_hard_failure_not_a_skip`, modelled on `v3_cross_language_cose.rs:616`.
- [ ] Step 5: Run `cargo test --locked -p abi-wdbx --lib v3::complete_episode::crypto < /dev/null` with system `python3`. Then `bash tools/check.sh < /dev/null` green, the Task 6 opt-in commands again (or recorded unexecuted), and `python3 tools/check_completion_execution.py` valid. Commit.

### Task 8: Bounded private trust-command wire decoder with malformed vectors and a stdlib witness

**Requirements:** P02.02, P02.03, C10 (C0).

**Files:**
- Create:
  - `crates/abi-wdbx/src/v3/complete_episode/crypto/trust/decode.rs`
  - `.../crypto/trust/decode_tests.rs`
  - `crates/abi-wdbx/tests/golden/trust-command-wire-v1/command-wire-v1.json`
  - `tools/wdbx_trust_command_witness.py`
  - `tools/tests/test_wdbx_trust_command_witness.py`
- Modify (register-pinned; hash refresh with reviewer note): `crypto/trust/mod.rs` (`pub(super)` module declarations only), `docs/completion/execution/p1-trust-slice.md:96` (decoder scope; still no persistence or admission authority).

**Interfaces:** `pub(super) fn from_canonical_bytes(bytes: &[u8], limits: CanonicalDecodeLimits) -> Result<(Namespace, AdminRef, u64 /*expected_revision*/, TrustCommand), TrustDecodeError>`. The outer array is `[domain, namespace, command_id, admin, expected_revision, operation]`, matching `command.rs`. Parsing goes through `CanonicalValue::from_canonical_bytes`, rebuilds through the existing typed constructors (`KeyRegistration::new`, …), then re-encodes with `canonical_bytes`; any difference is `NonCanonical`. Grants are decoded by re-encode comparison through the existing `Wire` path, without widening `complete_episode/decode/references.rs`. There is no public re-export.

- [ ] Step 1 (RED): Write the tests:
  - `trust::decode_tests::frozen_positive_commands_round_trip_exactly`: the five cases plus `claimed_zero_command_hex` from the read-only `docs/completion/execution/evidence/2026-10-22-continuation/trust/commands.json`.
  - `trust::decode_tests::malformed_command_wire_v1_rejected_with_named_categories`, covering:
    - wrong domain, outer arity 5 or 7, unknown op tag 5, operation arity
    - non-minimal ints, indefinite length, trailing bytes, nesting cap
    - public key of 31 or 33 bytes
    - purpose ≠ `Unsigned(0)`, profile ≠ `PROFILE_ID`
    - duplicate or unsorted grants, 65 grants, empty grants
    - empty or non-ASCII `command_id`, revocation reason 2
    - derived-kid mismatch, weak key
    - null vs 0 claimed time staying distinct
  - `trust::decode_tests::decoded_command_confers_no_authority`: `TrustState::apply` still needs `AdminAuthorization`, and decoding never mutates state.

  Run `cargo test --locked -p abi-wdbx --lib v3::complete_episode::crypto::trust < /dev/null`. FAIL.
- [ ] Step 2 (GREEN): Implement the decoder. Rerun until green.
- [ ] Step 3: Write the stdlib witness and `test_wdbx_trust_command_witness`, with identical category names. Then write the Rust test `trust::decode_tests::python_witness_agrees_on_command_wire_v1`, which fails hard without an interpreter. Do not add the Python suite to `check.sh` without reviewer agreement; the Rust shell-out test carries it in the default gate.
- [ ] Step 4: Run:
  - `python3 -m unittest discover -s tools/tests -p test_wdbx_trust_command_witness.py`
  - `bash tools/check.sh < /dev/null`
  - `python3 tools/check_completion.py`
  - `python3 tools/check_completion_execution.py` (valid after the hash refresh)
  - `python3 tools/check_substrate_release.py` (unchanged)

  Commit.

### Task 9: Federation canonical encoding witness and golden bundle

**Requirements:** P06.02, PLAN-0213, PLAN-0215. Encoding evidence only; C0. `FORMAT-federation-sidecar` is not cited: it is `sidecar.rs`, not the bundle encoding.

**Files:**
- Create:
  - `tools/wdbx_federation_canonical_v1.py` (stdlib)
  - `tools/tests/test_wdbx_federation_canonical_v1.py`
  - `crates/abi-wdbx/tests/golden/federation-bundle-v1/{README.md,bundle.json,bundle.sha256,manifest_digest.hex,bundle_digest.hex,record_digests.txt,refused/*.json}`
  - `crates/abi-wdbx/tests/v3_cross_language_federation.rs`
- Modify: `tools/check.sh`. Add one `-p 'test_wdbx_federation_canonical_v1.py'` step, and `unset` the new golden opt-in variable.
- Do not edit `canonical.rs`, `bundle.rs`, `exporter.rs` or `importer.rs`.

- [ ] Step 1: Generate the golden through the public API behind the opt-in variable `WDBX_WRITE_FEDERATION_GOLDEN`. Use a fixed-seed exporter, synthetic records and a `std::env::temp_dir()` state directory; add no `tempfile` dependency. The README records:
  - the seeds, organization and peer IDs, scope, checkpoint and deletion entries;
  - the ordering: records by `episode_digest`, `manifest.record_digests` by federation digest;
  - the "ascending" doc inconsistency in `bundle.rs`, recorded and not fixed;
  - the scope: "prototype, encoding-agreement-only, no wire freeze (gate task-06), no Python signature verification".
- [ ] Step 2 (RED): Write the Python tests and confirm they fail before the witness exists:
  - `record_digest_matches_golden`, `manifest_digest_matches_golden`, `bundle_digest_matches_golden`
  - `length_prefix_prevents_field_concatenation_collision`
  - `evidence_class_vocabulary_is_closed`
  - one refused-vector test per family
  - `unknown_field_refused`
- [ ] Step 3: Write the Rust tests:
  - `golden_bundle_digests_agree_across_languages`
  - `golden_bundle_bytes_are_pinned`
  - `refused_bundles_are_refused_with_the_same_family_by_both_sides`, through `FederationImporter::import` with peer, trust and grant configured, and serde for unknown fields. The families are `InvalidInput`, `DigestMismatch`, `ScopeMismatch`, `OriginMismatch` and `parse`. Refusal order follows `check_manifest`. Scope and origin vectors mutate the manifest and recompute `record_digests`.
  - `re_export_is_byte_identical`: byte equality against the golden, beyond `exchange.rs:266`.
  - `witness_scope_is_encoding_agreement_only`
  - `a_missing_federation_witness_interpreter_is_a_hard_failure_not_a_skip`
- [ ] Step 4: Run `cargo test --locked -p abi-wdbx --test v3_cross_language_federation < /dev/null`, the Python discover command, `bash tools/check.sh < /dev/null` and `python3 tools/check_completion_execution.py` (no drift). Commit.

### Task 10: Golden native outbox and refused-vector corpus with a stdlib reader

**Requirements:** P07.01, P07.04, PLAN-0222, PLAN-0228. Native outbox format observations only; browser and ABI scope untouched; `FORMAT-offline-sidecar` stays open; C0.

**Files:**
- Create:
  - `crates/abi-wdbx/tests/golden/outbox-v1/{README.md,outbox.v1.jsonl,outbox.v1.sha256,refused-v1.json}`
  - `crates/abi-wdbx/tests/v3_offline_outbox_golden.rs` (split into a `tests/v3_offline_outbox_golden/` directory if it grows)
  - `tools/wdbx_outbox_v1_reader.py`
  - `tools/tests/test_wdbx_outbox_v1_reader.py`
- Modify: `tools/check.sh`. Add one narrow step, and `unset WDBX_WRITE_OUTBOX_GOLDEN`.
- `sidecar.rs`, `outbox.rs` and all register-pinned offline files stay byte-unchanged.

- [ ] Step 1: Generate the golden with a deterministic `Clock`, ASCII-only synthetic markers, one operation of each kind (Write/Delete/Asset/Edit) and one of each status (Pending/InFlight/Acknowledged/Rejected/Conflicted).
  - `refused-v1.json` covers:
    - header count mismatch, `records_checksum` mismatch, line checksum mismatch
    - trailing fragment or missing final newline
    - unknown field, kind/payload mismatch, duplicate id, wrong format tag
    - `> MAX_LINE_BYTES`, as a rule plus a seed
  - Each vector carries an `expected_cause`.
- [ ] Step 2: Write the Rust tests, reusing the error-family assertions from `format.rs:183-313`:
  - `golden_outbox_reopens_with_every_operation_and_status`
  - `golden_outbox_bytes_are_pinned`
  - `rewrite_of_golden_is_byte_identical`
  - `every_refused_vector_is_refused_as_corrupt`: `Err(OutboxError::Corrupt)`
  - `golden_carries_no_episode_content`: only synthetic markers; no commitment or digest field beyond the binding `contract_digest` and the asset digest
  - `dangling_dependency_is_currently_accepted_finding`: `open()` succeeds; recorded as a finding in the README
- [ ] Step 3: Write the Python reader. It verifies the header, count, `records_checksum` and per-line SHA-256 over `json.dumps(separators=(',',':'), ensure_ascii=False)` with key order preserved. Tests:
  - `golden_header_and_checksums_verify`
  - one test per refused vector checking `expected_cause` exactly
  - `extra_field_refused`
  - `dangling_dependency_is_reported_as_structural_warning`

  If any serde_json formatting differs from Python, stop and record a finding.
- [ ] Step 4: Run `cargo test --locked -p abi-wdbx --test v3_offline_outbox_golden --test v3_offline_outbox < /dev/null`, the Python discover command, `bash tools/check.sh < /dev/null` and `python3 tools/check_completion_execution.py` (no drift). Commit.

### Task 11: Studio direct kernel tests S27/S28/S43 and S24 engine-path evidence (test only)

**Requirements:** S27, S28, S43, S24, CH14. Candidate evidence only; C0.

**Files:**
- Create:
  - `specimen-studio/native/specimen-kernel/tests/lifecycle_local_gaps_v2.rs` (copies `seed_with_first_flip`; the pinned v1 file is not edited)
  - `specimen-studio/native/specimen-kernel/tests/atp_engine_two_cycle_v1.rs` (test-local `Sync` `Env` with an `AtomicI64` clock that wraps `FixedEnv` for `uid`)
  - `docs/completion/execution/evidence/2026-10-27-studio-lifecycle-v2/{kernel-lifecycle-v2-tests.log,atp-engine-two-cycle.log,mutation-check.log}`

- [ ] Step 1: Write `s27_one_contributor_in_two_green_spans_gets_one_trial`:
  - Segments `s1` and `s2` are green with contributors `[a]`, and the contributor is a real node.
  - Strength moves by exactly +1.
  - The message starts with "1 of 1 contributors changed strength after independent coin flips."
  - The seed equals `(Rng advanced once).0 % u32::MAX`, then `.max(1)`.
  - Boundary: two real contributors take two samples.
  - Negative: a repeat feedback gives "already applied", kept as an in-fixture boundary check only.
- [ ] Step 2: Write `s28_fall_from_max_restores_jitter_and_keeps_attachment_mode`.
  - Setup: `strength == maxStrength` exactly, `jitter:true`, `atp.intensity > 0`, `threshold <= base < 100` with margin for ±1.5 jitter, and a `{hard:true}` attachment. The vote is found by `nodeRef`.
  - Before feedback, set `settings.seed = seed_with_first_flip(true)`.
  - At max, `modulation == 0` and `jitter == 0`. After the fall, `strength == max - 1`, and fixed-seed runs before and after show nonzero jitter.
  - The original hard attachment is byte-identical. Keep steps few enough that coactivation (`engine.rs:966-981`) adds no attachment.
  - Negative control: with the coin not flipped, strength stays at max and jitter stays 0.
  - The doc states that crystallization is the inferred predicate.
- [ ] Step 3: Write `s43_pinned_history_survives_rolling_overwrite`:
  - `historyLimit=2`; pin the oldest record; run 3 or more `engine::cycle` calls.
  - After each cycle the unpinned count equals 2 and the pinned record keeps its position.
  - Boundary: `pinLimit=1` with one record already pinned gives "The pin limit is reached".
  - Respect `history.len() <= historyLimit + pinLimit` (`engine.rs:80-81`).
- [ ] Step 4: Write the S24 engine-path tests. Each cycle threads the state returned by the previous one.
  - `s24_engine_buildup_persists_across_two_related_cycles`: reset `atp` explicitly, use the seeded "happy" resource (I=0.4), and expect 0.08 then 0.16 within 1e-12.
  - `s24_engine_release_and_cooldown`: add an I=1.0 resource via `mutate::save_resource`. After at most 5 cycles intensity is halved and `cooldownUntil = now + clamp(atpCooldownSeconds,1,3600)*1000`. A cycle at the same clock before expiry leaves intensity and valence unchanged.
  - `s24_engine_jitter_does_not_persist`: a jitter node driven by `cycle_with_visual`. Jitter is nonzero and differs between the two seeds; `state.atp` is byte-identical.
  - `s24_engine_backwards_clock_divergence_is_recorded`: assert the `max(0)` clamp and the backwards `lastUpdate` as observed legacy behavior, documented as divergence from the monotonic design and not as conformance.

  The header names `tests/atp_v3.rs` as the private-profile coverage. Neither file closes S24 or CH14.
- [ ] Step 5: Mutants, applied by hand in a scratchpad copy of `specimen-studio`. Each must fail a named test; log which one.
  1. Drop the trim pinned guards (`engine.rs:1006,1010`).
  2. Drop the contributor dedup (`mutate.rs:428-436`).
  3. Change the max guard to `max-1` (`engine.rs:757-768`).
  4. Flip `hard` on decrement.
  5. Leave strength unchanged on a `right=false` flip.
  6. Persist jitter into `state["atp"]`.
- [ ] Step 6: From `specimen-studio/`:
  - `CARGO_TARGET_DIR=<scratch>/wf-target-studio sh scripts/pinned-cargo.sh test --locked -p specimen-kernel --test lifecycle_local_gaps_v2 --test atp_engine_two_cycle_v1 </dev/null`
  - the full `sh scripts/pinned-cargo.sh test --locked -p specimen-kernel </dev/null` (260 or more pass)

  Then `python3 tools/check_completion_execution.py` (no pinned file touched). Commit.

### Task 12: Studio idle scheduler draws a fresh interval per completed event (D16 pause/resume)

**Requirements:** S37, CH19 (C0). Merges `s37-idle-fresh-interval` and `studio-s37-idle-fresh-interval`.

**Files:**
- Create: `specimen-studio/lib/specimen/idle-scheduler.ts`, `specimen-studio/tests/idle-scheduler.test.js`.
- Modify: `specimen-studio/app/state/use-studio.ts:197-235` (unpinned).

**Interfaces:**

```ts
createIdleScheduler({ idleMin, idleMax, random, now, setTimeout, clearTimeout,
  onEvent: (mode: "phagy" | "mutation") => Promise<void> })
```

It returns `{ pause(), resume(), dispose() }`.

- It throws on non-finite values, on `idleMin`/`idleMax` outside `[1,86400]`, and on `idleMin > idleMax`, matching `validate_settings` (`mutate.rs:545-564,572-592`). It does not clamp.
- Each wait is a fresh uniform draw.
- The mode is drawn once per event (`random() < 0.5` gives `phagy`) and passed to both `nativeReview(…, mode, signal)` and `maintenance(snapshot, mode)`.
- The next timer is armed only in `.finally()` of the returned promise.

- [ ] Step 1 (RED): Write the tests:
  - `S37 each completed idle event draws a fresh interval`
  - `S37 next timer is not armed until the event promise settles`
  - `S37 each event flips a fair coin between mutation and phagy`: `[0.2, 0.7]` gives phagy then mutation.
  - `S37 foreground activity pauses the remaining delay and resumes without a new draw`: the event fires at the original delay plus the paused time, and the number of `random()` calls is unchanged.
  - `S37 boundary idleMin equals idleMax gives a constant delay`
  - `S37 invalid settings throw`
  - `S37 dispose clears timers and a late settle does not re-arm`

  Run `bun test tests/idle-scheduler.test.js`. FAIL.
- [ ] Step 2 (GREEN): Implement the module. Rewire the hook so that:
  - state and settings are read through refs;
  - the scheduler is keyed only on hydration, the maintenance setting and the settings values;
  - busy and interaction call `pause()`/`resume()`;
  - both branch callbacks return their promises;
  - branch logic is unchanged and `use-studio.ts` no longer calls `Math.random()` for mode.

  Check that `maintenance` has no internal second draw.
- [ ] Step 3: From `specimen-studio/`, run `bunx tsc --noEmit`, `bun run lint:studio`, `bun test tests/idle-scheduler.test.js` and `bun test`. Then `git status --short` must show only the intended files.
- [ ] Step 4: Record as unexecuted: `bun run check` (build:wasm writes artifacts), `bun run test:browser` and the native wdio suite. Record that the defaults match the D16 `[60,120]` s range, or record the mismatch. Record the frozen-row "tag or PHAGY" vs designs.json "mutation/PHAGY" wording discrepancy as unresolved. S38–S41 fairness and foreground stay open. Commit.

### Task 13: Studio research consistency checker over the existing case map

**Requirements:** CH25, P08.10 (C0). Reshaped: it adds no new map file.

**Files:**
- Create: `tools/check_studio_research.py` (stdlib, Python 3.9, `safe_file` pattern from `check_completion_execution.py:17-31`), `tools/tests/test_check_studio_research.py`.
- Not wired into `tools/check.sh`. Record that explicitly.
- Edits none of `register-v1.json`, `requirements.json`, `plans.json`, `studio-acceptance-v1.json` or `studio-research-v1.json`.

- [ ] Step 1 (RED): Write the tests, each over a temporary fixture:
  - `test_current_tree_passes`
  - `test_missing_case_refused`, `test_extra_case_refused`, `test_duplicate_case_refused`
  - `test_renamed_rust_test_refused`: needs `#[test]` immediately above `fn <name>(`.
  - `test_renamed_js_test_refused`: exact `test(`/`it(` title, either quote style.
  - `test_bad_contract_sha_refused`
  - `test_promoted_status_refused`
  - `test_escaping_or_symlink_path_refused`
  - `test_labelled_test_without_reference_warns`: S24, `atp_v3.rs:270,303`.
  - `test_local_gap_case_mapped_without_warning_refused`
- [ ] Step 2 (GREEN): Implement the checker:
  - case IDs equal the 46 in `studio-acceptance-v1.json`;
  - `contracts[].sha256` match;
  - every `direct_test`/`candidate_tests` entry exists;
  - every case is `open`/`C0`/`unknown` with `claim_policy` C0-only;
  - local_gap cases are cross-checked against the register;
  - S24 produces a warning.

  The acceptance digest field is named distinctly from that file's internal `source_sha256`.
- [ ] Step 3: Run `python3 tools/check_studio_research.py` (exit 0, S24 warning printed), `python3 -m unittest discover -s tools/tests -p test_check_studio_research.py`, and `bash tools/check.sh < /dev/null` (unaffected; state that it gives no evidence for this checker). Commit.

### Task 14: Workspace-constraint checker (G12 mechanism)

**Requirements:** G12 (C0). Routing happens in Task 20.

**Files:**
- Create: `tools/check_workspace_constraints.py` (stdlib, 3.9, no `tomllib`; bounded manifest parsing), `tools/tests/test_check_workspace_constraints.py`.
- Modify: `tools/check.sh` (add a step after `check_rust_sizes`).

- [ ] Step 1 (RED): Write the tests:
  - `test_current_tree_passes`
  - `test_git_dependency_refused`, `test_registry_or_version_only_dependency_refused`
  - `test_path_outside_crates_refused`, `test_missing_dependency_refused`, `test_duplicate_dependency_refused`
  - `test_lock_source_refused`: per `[[package]]` block in `specimen-studio/Cargo.lock`.
  - `test_studio_is_not_a_root_workspace_member`
  - `test_size_limit_refused`: copy `check_rust_sizes.sh` into `<tmp>/tools/` of a `git init` repo. A tracked 1001-line `.rs` file exits 1, an untracked non-ignored 1001-line file also exits 1, and a 1000-line file passes.
  - `test_specimen_studio_excluded_from_size_limit`: a deliberate exclusion, not permission.
- [ ] Step 2 (GREEN): Implement the checker.
  - `specimen-core` must declare `abi-wdbx` and `abi-compute` as path dependencies resolving to `crates/abi-wdbx` and `crates/abi-compute`.
  - `specimen-kernel` must declare only `abi-compute`.
  - The lock entries for those crates must have no `source` or `checksum`.
  - `check_rust_sizes.sh` must exist and be invoked by `check.sh`.

  Wire it into `check.sh`.
- [ ] Step 3: Run `python3 tools/check_workspace_constraints.py`, the unittest discover for it, and `bash tools/check.sh < /dev/null` green. Recheck the `inventory-v1.json` and source-custody digests for `check.sh` and record the result; do not edit snapshots. Commit.

### Task 15: Refuse opening a store whose deletion sidecars outlive the replayed ledger

**Requirements:** P03.04 implementation and test evidence only. P03.02 and P03.03 restored-backup handling stays blocked. C0.

**Files:**
- Modify:
  - `crates/abi-wdbx/src/v3/episode/store/deletion/index.rs` (catalog-pinned; Task 21): `RedactionIndex::verify` and `ObjectKeyIndex::verify`, plus one combined helper.
  - `crates/abi-wdbx/src/v3/episode/store.rs` (catalog-pinned; Task 21): one call line beside `reclaimed.verify` (:380-382), before `revocations.inject` and any tail repair. It must stay at or below 995 lines.
  - `crates/abi-wdbx/src/v3/episode/store/deletion.rs` (register-pinned; hash refresh): amend the doc at :89-93. "Sidecar lost, ledger kept" is unchanged; "ledger older than sidecar" is now corruption.
- Create: `crates/abi-wdbx/tests/v3_deletion_stale_generation.rs`.
- No holds or revocation verify in this slice. It is recorded as a follow-up that needs its own invariant proof.

**Rule:** every redaction and object-key target must be in `memories.forgotten` for its guild. Tombstones are never reclaimed. A missing guild gives `EpisodeStoreError::Corrupt`, modelled on `ReclaimedIndex::verify` (`gc.rs:299-310`).

- [ ] Step 1 (RED): Write the tests:
  - `a_pre_forget_ledger_beside_an_object_key_sidecar_is_refused_as_corrupt`: `store_bytes` unchanged across the refused open.
  - `a_pre_forget_ledger_beside_a_redaction_sidecar_is_refused_as_corrupt`
  - `an_object_key_sidecar_naming_a_guild_the_ledger_lacks_is_refused_as_corrupt`
  - `a_lost_deletion_sidecar_still_opens_and_holds_no_request`
  - `a_reclaimed_target_with_a_destruction_request_still_opens`: forget, destroy, apply GC, reopen twice, identical bytes.
  - `the_erased_target_cannot_be_superseded_after_any_reopen`: the inverse of the probe.

  Run `cargo test --locked -p abi-wdbx --test v3_deletion_stale_generation < /dev/null`. The first three FAIL, matching the probe "stale restore OPENED".
- [ ] Step 2 (GREEN): Implement. Run `cargo test --locked -p abi-wdbx --test v3_deletion_stale_generation --test v3_deletion_lifecycle --test v3_deletion_authority --test v3_holds_revocation < /dev/null`.
- [ ] Step 3: `bash tools/check.sh < /dev/null` green, with sizes checked. `python3 tools/check_completion_execution.py` valid after the `deletion.rs` refresh. `check_substrate_release.py --drift-report` lists only `store.rs` and `deletion/index.rs` as expected drift; record that in the commit body. The evidence states that this detects only inconsistent generations, not a consistent old backup. Commit.

### Task 16: Rust `InhibitionCommitmentV1` parity with the pinned synthetic vectors

**Requirements:** P03.02, and P03.04 as supporting evidence only. C0 synthetic parity: no KMS, rotation policy, crypto review or schema approval.

**Files:**
- Create: `crates/abi-wdbx/src/v3/inhibition.rs`. It contains HMAC-SHA256 on `sha2`, a constant-time compare, framing per `inhibition-contract-v2.md` and `contract_vectors.py:209-237`, and `#[cfg(test)]` unit tests that read `docs/completion/inhibition-vectors-v2.json` through `CARGO_MANIFEST_DIR`.
- Modify: `crates/abi-wdbx/src/v3/mod.rs` (catalog-pinned; Task 21): `pub(crate) mod inhibition;`. No public facade.
- `inhibition-vectors-v2.json` and `inhibition-contract-v2.md` are not edited; they are pinned at `check_completion.py:55,59`.

**Interfaces:**
- `InhibitionCommitmentV1::tag(tenant, scope, pair_bytes, epoch_key) -> [u8;32]`
- `InhibitionCommitmentV1::matches(stored_tag, stored_epoch, &epoch_keys) -> bool`

Tenant and scope are ASCII, and anything else fails closed. Class IDs are 1–128 ASCII bytes. Pair bytes are at most 16 KiB. Pair framing takes no policy-code input.

- [ ] Step 1 (RED): Write the tests:
  - `hmac_sha256_rfc4231_case_2_known_answer`
  - `preimage_bytes_match_vector`
  - `epoch_tags_match_vector`
  - `policy_code_variations_cannot_change_the_pair_tag`
  - `epoch_zero_tag_unchanged_after_payload_key_destruction`
  - `old_epoch_tag_matches_after_mac_rotation_only_while_epoch_zero_key_is_kept`
  - `other_tenant_scope_and_non_equivalent_tags_never_match`
  - `non_ascii_and_oversized_inputs_fail_closed`

  Run `cargo test --locked -p abi-wdbx --lib v3::inhibition < /dev/null`. FAIL.
- [ ] Step 2 (GREEN): Implement until green. Rust output must equal `contract_vectors.py inhibition_vectors()`.
- [ ] Step 3: Run `python3 -m unittest discover -s tools/tests -p test_check_completion.py` (pins unchanged) and `bash tools/check.sh < /dev/null`. The commit body names `v3/mod.rs` as expected release drift for Task 21. Commit.

### Task 17: Admission-first vector candidate search

**Requirements:** C14, P04.01, P04.02. Partial evidence: P04.02 "reject invalid crypto" and governed rebuild stay open. C0.

**Files:**
- Create: `crates/abi-wdbx/src/v3/ranking/admitted.rs`, `crates/abi-wdbx/tests/v3_vector_admitted_search.rs`.
- Modify:
  - `crates/abi-wdbx/src/v3/ranking/mod.rs` (catalog-pinned; Task 21): module declaration and re-export.
  - `crates/abi-wdbx/src/v3/ranking/adapter.rs` (register-pinned under C14; hash refresh plus `reference-refresh.json`): `pub(crate)` accessors over stored vectors via `HnswIndex::get` (`hnsw.rs:281`) and `ExactIndex::get` (`index.rs:139`).
- `vector.rs` is untouched.

**Interfaces:** `rank_with_vector_source_admitted(store, query, source, vector, limit)`.

1. Run `store.query` once and mint the scope via `admitted_scope` from that same answer.
2. Map digests to vector IDs (`VectorBindingIndex::vector_id_of`).
3. Score only those IDs with exact cosine.
4. Truncate after admission.

No scalar is added to ranking. `rank_with_vector_source` keeps its pinned contract.

- [ ] Step 1 (RED): Write the tests:
  - `non_admitted_near_duplicate_does_not_consume_top_k`: also asserts that the old path still reports crowding.
  - `other_scope_vectors_are_never_scored`
  - `member_scope_constraint_applies_before_search`
  - `uncertain_and_contradicted_admitted_records_stay_visible`: reuses the `v3_constitutional_query_holds.rs` and `v3_holds_revocation.rs` fixture patterns.
  - `admitted_search_is_deterministic_and_matches_exact_oracle`

  Run `cargo test --locked -p abi-wdbx --test v3_vector_admitted_search < /dev/null`. FAIL.
- [ ] Step 2 (GREEN): Implement. Then run `cargo test --locked -p abi-wdbx --test v3_vector_admitted_search --test v3_vector_adapter --test v3_dimension_ranking < /dev/null`, recounting the existing tests rather than asserting "25".
- [ ] Step 3: Run:
  - `bash tools/check.sh < /dev/null`
  - `python3 tools/check_completion_execution.py`
  - `python3 -m unittest discover -s tools/tests -p test_check_completion_execution.py`
  - `python3 tools/check_completion.py`
  - `python3 tools/check_source_custody.py`

  The adapter hash refresh is reviewed and recorded. Commit, naming `ranking/mod.rs` as expected release drift.

### Task 18: Deterministic disposable index rebuild with a local `IndexDigest`

**Requirements:** P04.02 (the "rebuild disposable indexes deterministically" clause only), with P04.01 supporting. C0. It is a private prototype under the open `docs/completion/shared-types-v1.json#task-04` schema gate, not serialized for consumers, and not a wire freeze. It can land without Task 17.

**Files:**
- Create: `crates/abi-wdbx/src/v3/ranking/rebuild.rs`, `crates/abi-wdbx/tests/v3_index_rebuild.rs`.
- Modify: `crates/abi-wdbx/src/v3/ranking/mod.rs` (catalog-pinned; Task 21).
- `vector.rs` and `adapter.rs` are not edited.

**Interfaces:**
- `#[non_exhaustive] pub enum IndexVersion { V1 }`, encoded as u32 LE 1.
- `rebuild_exact(...)` and `rebuild_hnsw(...)`. Each takes `(version, store, query, embeddings: &BTreeMap<[u8;32], Vec<f32>>, seed: u64, directory: &Path)` and returns `Result<(Source, IndexDigest, RebuildReceipt), RebuildError>`.
- The directory must contain neither `VECTOR_BINDING_FILE` nor `VECTOR_BINDING_LOCK`. The ledger is never written. Insertion is in ascending digest order. Non-admitted or unknown digests are refused and counted.
- `IndexDigest = SHA-256(` the length-prefixed domain tag `"wdbx-local-index-digest-v1"` ‖ version u32 LE ‖ length-prefixed scope ‖ seed u64 LE ‖ for each sorted `(vector_id u64 LE, digest[32], dim u32 LE, f32::to_bits LE…)` `)`. This layout is documented in the module docs and in the evidence note as local and unversioned by ABI.

- [ ] Step 1 (RED): Write the tests:
  - `index_digest_known_answer_for_a_tiny_input`
  - `two_rebuilds_produce_identical_digest_and_binding_file`
  - `embedding_insertion_order_does_not_change_digest`
  - `deleting_binding_file_and_lock_then_rebuilding_reproduces_the_index`: ledger hash equal before and after.
  - `rebuilt_ranking_matches_an_independently_built_index`: inserted in non-sorted order, compared on resolved digests and dispositions via `rank_with_vector_source` and `admitted_scope`.
  - `rebuild_never_hides_uncertainty`
  - `non_admitted_embedding_is_refused_not_bound`
  - `different_version_or_seed_changes_digest`
  - `rebuild_refuses_a_non_empty_directory`

  Run `cargo test --locked -p abi-wdbx --test v3_index_rebuild < /dev/null`. FAIL.
- [ ] Step 2 (GREEN): Implement until green.
- [ ] Step 3: Run `bash tools/check.sh < /dev/null` and `python3 tools/check_completion_execution.py` (no register change). Commit, naming `ranking/mod.rs` as expected release drift.

### Task 19: Register mapping refresh (no disposition change)

**Requirements:** the rows listed below. Every row stays `open`/`C0`/`unknown`.

**Files:**
- Modify: `docs/completion/execution/register-v1.json`, `docs/completion/execution/candidate-paths-v1.json`, `tools/check_completion_execution.py` (`CANDIDATE_PATHS_SHA256` and, if artifacts change, `REVIEW_ARTIFACT_PATHS_SHA256`), `docs/completion/execution/review-artifact-paths-v1.json`, `docs/completion/execution/progress.md` (append).
- Create: `docs/completion/execution/evidence/2026-10-27-register-refresh/{requirement-mapping.json,reference-refresh.json,review.md,*.log}`.

- [ ] Step 1: Rebase on the current `main`. Confirm that every file from Tasks 2–18 exists and is committed. Compute all sha256 values from the committed bytes.
- [ ] Step 2: Write the mapping table in `requirement-mapping.json`, with per-row before and after paths and hashes. Implementation and test refs are also added to `inspected`, and register and catalog lists are edited together.

  Existing suites:
  - C14: `crates/abi-wdbx/tests/v3_vector_adapter.rs`.
  - P03.02: `v3_deletion_authority.rs`, `v3_supersession_lineage.rs`, `v3_retention_windows.rs`.
  - C08: `v3_supersession_lineage.rs`.
  - C18 and C19: `v3_retention_windows.rs`.
  - P08.05 and CH14:
    - implementation `specimen-studio/native/specimen-kernel/src/atp_v3/{mod,model,advance}.rs`, also listed under inspected;
    - tests `specimen-studio/native/specimen-kernel/tests/atp_v3.rs`;
    - artifact `docs/completion/execution/atp-v3-profile.md`;
    - an added blocker: "private, test-included only (no `mod atp_v3` in lib.rs); logicalf32-only; no tone semantics; no named-owner interface".
  - S39, CH20, P08.09: `specimen-studio/native/specimen-kernel/tests/raft_plan.rs`, only where the reviewer confirms it asserts the row. The S39 blocker says it lacks the 0/20/50 starts and per-step yield.

  Gap rows, inspected refs only:
  - G05: `v3_guild_authority.rs`, `v3_deletion_authority.rs`.
  - S37: `use-studio.ts`, `lib/specimen/idle-scheduler.ts`, `lib/specimen/kernel.ts`.
  - S41: `specimen-studio/native/specimen-core/src/scheduler.rs`, `raft_plan.rs`.
  - S24: an added blocker "atp_v3 s24_* (atp_v3.rs:270,303) and engine-path atp_engine_two_cycle_v1.rs are partial mechanism; engine wall-time/max(0) divergence; task-08 named-owner gate open". S24 stays local_gap.

  New tranche tests:
  - P03.*/C20: `v3_rejection_invariance.rs`, `v3_lifecycle_fault_matrix.rs`, `v3_deletion_stale_generation.rs`.
  - P03.02: `v3/inhibition.rs`.
  - P02.02/P02.03/C10: P1 v1 goldens, `crypto/trust/decode.rs`, `decode_tests.rs`, the trust witness.
  - P04.01/P04.02/C14: `v3_query_disposition_matrix.rs`, `ranking/admitted.rs`, `v3_vector_admitted_search.rs`, `ranking/rebuild.rs`, `v3_index_rebuild.rs`.
  - E05/E06: only where a test asserts the dimension.
  - P06.02: `federation/{canonical,bundle,record,origin}.rs` and `v3_cross_language_federation.rs`. Decide P06.01 vs P06.02 for `record.rs`/`origin.rs` explicitly.
  - P07.01: `offline/{draining,sidecar}.rs` and `v3_offline_outbox_golden.rs`.
  - S27, S43: `lifecycle_local_gaps_v2.rs`.
  - Record `federation/{sidecar,foreign}.rs`, `offline/{asset,drain,types,merge}.rs` and the unmapped `v3_federation/*` and `v3_offline_outbox/*` files as mapped or inspected-but-unmapped.

  Do not touch the historical candidate-paths hash in `substrate-release/baseline-v1.json`.
- [ ] Step 3 (RED): `python3 tools/check_completion_execution.py` reports "candidate path catalog drift".
- [ ] Step 4: An independent read-only reviewer checks that each added test asserts its mapped requirement rather than sitting next to it, and records the verdict in `review.md`. Drop any rejected mapping.
- [ ] Step 5 (GREEN): Bump `CANDIDATE_PATHS_SHA256` (and the review-artifact pin if needed) to the reviewed bytes. Then:
  - `python3 tools/check_completion_execution.py` prints "163 reconciled open C0".
  - `python3 -m unittest discover -s tools/tests -p test_check_completion_execution.py` passes, including `test_readme_counts_match_catalog` with counts unchanged at 131/16/16.
  - `python3 tools/check_completion.py` passes.
  - A `python3 -c` scan confirms every row is `open`/`C0`/`unknown`.

  Record the receipts for `cargo test --locked -p abi-wdbx --test v3_vector_adapter --test v3_deletion_authority --test v3_supersession_lineage --test v3_retention_windows < /dev/null` and, from `specimen-studio/`, `sh scripts/pinned-cargo.sh test --locked -p specimen-kernel --test atp_v3 --test raft_plan </dev/null`. Commit.

### Task 20: Register disposition moves (G12, S37, conditional S28) and counts

**Requirements:** G12, S37, S28. All stay `open`/`C0`/`unknown`.

**Files:** the same register, catalog, checker pin, execution `README.md` and `progress.md` set as Task 19. Create `docs/completion/execution/evidence/2026-10-27-disposition-moves/` with the mapping, logs and review.

- [ ] Step 1: Move each row to `candidate`, rewrite its reason, and keep the existing blockers byte-identical:
  - **G12:** implementation `[tools/check_rust_sizes.sh, tools/check_workspace_constraints.py]`, tests `[tools/tests/test_check_workspace_constraints.py]`. Added blocker: "continuous per-revision constraint; checker pass is point-in-time, not completion".
  - **S37:** implementation `[specimen-studio/lib/specimen/idle-scheduler.ts, specimen-studio/app/state/use-studio.ts]`, tests `[specimen-studio/tests/idle-scheduler.test.js]`. Added blockers: the tag-vs-mutation wording discrepancy; foreground and fairness for S38–S41 open; browser e2e unexecuted.
  - **S28:** move only if the Task 19 or Task 20 reviewer accepts the inferred-predicate assertions together with their mutants. Implementation `[engine.rs, mutate.rs]` with unchanged hashes, confirmed. Tests `[lifecycle_local_gaps_v2.rs]`. Added blockers: crystallization has no persisted flag (studio.md:1401 step 6); saved-store, import, browser/WASM parity, UI and independent review remain open.
- [ ] Step 2: Compute the counts from the live register:
  - all three moved: 134 candidate / 13 local_gap / 16 external_gap;
  - S28 withdrawn: 133 / 14 / 16.

  Update the execution README count line and the "Local gaps:" list. The root `README.md` is not edited.
- [ ] Step 3 (RED → review → GREEN): The checker reports drift. An independent reviewer records a verdict. Then bump `CANDIDATE_PATHS_SHA256`, and run `python3 tools/check_completion_execution.py`, the execution unittest (including `test_readme_counts_match_catalog`) and `python3 tools/check_completion.py`. Commit.

### Task 21: Single reviewed release-catalog migration for the touched pinned sources

**Requirements:** catalog identity only; every `missing_behavior_blocker` stays open.

**Files:**
- Modify: `docs/completion/substrate-release/catalog-v1.json`, `tools/check_substrate_release.py` (`CATALOG_PIN`), `docs/completion/substrate-release/README.md` (counts), and the count-dependent tests in `tools/tests/test_check_substrate_release.py`.
- Create: `docs/completion/execution/evidence/2026-10-27-catalog-migration/{review.md,drift-before.json,drift-after.json}`.

- [ ] Step 1: Run `python3 tools/check_substrate_release.py --drift-report`. The drift must be exactly:
  - `v3/episode/store.rs` and `v3/episode/store/deletion/index.rs` (FORMAT rows, Task 15)
  - `v3/mod.rs` (Task 16)
  - `v3/ranking/mod.rs` (Tasks 17–18)

  Any other path is a stop condition.
- [ ] Step 2: Re-hash those rows and shift line anchors after the new declarations. Add declaration-coverage-only rows for `pub(crate) mod inhibition;`, `mod admitted;` and `mod rebuild;`, each with owner `WDBX compatibility maintainer`, the matching release task and an open `missing_behavior_blocker`. Follow the shape of `56fb6a8`. `test_valid` is RED with "catalog changed".
- [ ] Step 3: An independent reviewer checks the catalog diff alone and records a verdict. On APPROVE, set `CATALOG_PIN` and update the README counts and tests.
- [ ] Step 4: Live mode exits 0, `--snapshot-only` exits 0 and `--drift-report` reports no drift. `python3 -m unittest discover -s tools/tests -p test_check_substrate_release.py` passes. Add `test_edit_to_newly_pinned_ranking_mod_is_refused` (an isolated copy). Commit.

### Task 22: Reconcile stale gate documentation

**Requirements:** P01.01 (C0). README.md is excluded.

**Files:**
- Modify:
  - `docs/agent-guide.md`, roughly lines 125–160: the actual `tools/tests/test_*.py` count; the narrow `check.sh` unittest patterns as they stand after Tasks 7, 9, 10 and 14; the non-gate suites; `check_p1_crypto.sh` described as an explicit opt-in private P1 witness gate that fails by design under the default PyCA backend; the completion, execution, custody, substrate-release and 3-D evidence checkers named as outside the gate.
  - `.github/workflows/ci.yml:59-61`: comment only.
- `rust-toolchain.toml` is deliberately excluded (see the table below).

- [ ] Step 1: Measure on the live tree with `grep -n unittest -A1 tools/check.sh` and `ls tools/tests/test_*.py | wc -l`. Record the output in the commit body.
- [ ] Step 2: Edit the docs to match the measurements exactly.
- [ ] Step 3: Run `bash tools/check_instructions.sh`, `git diff --check`, `bash tools/check.sh < /dev/null`, `python3 tools/check_completion_execution.py` and `python3 tools/check_substrate_release.py`. Commit.

### Task 23: Final gates, merge and lease release

- [ ] Step 1: On the exact branch head, all of these pass:
  - `bash tools/check.sh < /dev/null`
  - `check_completion.py`, `check_completion_execution.py`, `check_source_custody.py`
  - `check_substrate_release.py`: live, `--snapshot-only`, `--drift-report`
  - `check_studio_research.py`, `check_workspace_constraints.py`
  - From `specimen-studio/`: the full `sh scripts/pinned-cargo.sh test --locked -p specimen-kernel </dev/null`, `bunx tsc --noEmit`, `bun run lint:studio` and `bun test`.

  Record the opt-in P1 gate's result, or record it as unexecuted. Record every skipped Studio gate explicitly.
- [ ] Step 2: Append an execution outcome section to this plan, listing per task the commits, observed counts, findings and withdrawn routings.
- [ ] Step 3: `git merge --ff-only` into local `main`. Delete the task branch. Append a lease release row. No push.

## Not in this tranche

| Item | Kind | Reason / blocker |
| --- | --- | --- |
| (none) | Refuted slices | No slice was refuted. Every proposed slice survived verification, and the corrections are folded in above. Merged as duplicates: `s37-idle-fresh-interval` and `studio-s37-idle-fresh-interval` (Task 12); `studio-gap-wave2-s24-s28` and `studio-s27-s28-s43-direct` (Task 11, with S24 kept local_gap per Option A); `studio-route-lifecycle-v2`, `p04-register-evidence-refresh`, `exec-catalog-refresh-existing-suites` and `register-mapping-refresh` (Tasks 19–20). Reshaped: `studio-case-map-v1` became a checker over the existing `studio-research-v1.json` (Task 13). |
| vNext typed envelope (`EpisodeEnvelopeVNext` with tenant, regime, prediction, action, outcome, uncertainty, artifacts) and its serializer; any rename of the `wdbx-complete-episode-v1` domain | Blocked | plans.json task-02 step 1 needs ABI schema-owner agreement first; owner-review.md decisions 1–3; ABI Program 1 owns normative wire schemas. |
| Durable trust history and admission (persisted commands, restart/rollback protection, a single canonical writer, all-or-none crash recovery) | Blocked | Design slice 5 needs an ABI-reviewed schema/profile/error/receipt diff, a named owner receipt and independent review. The persistent adapter was withdrawn (progress.md); owner-review.md decision 6. |
| Effective-time key administration (`KeyAdmin.rotate/revoke(KeyId, EffectiveTime, Authority)`) | Blocked | The trust model is revision-ordered by design, and claimed time is evidence only. Temporal authority is an owner decision (owner-review.md decision 3). |
| Default-gate Python signature verification; general Rust/Python crypto parity | Blocked | The crypto dependency decision: system Python has no crypto library, hand-rolled crypto is forbidden, the PyCA weak-key disagreement is retained, and only the isolated PyNaCl/libsodium venv verifies. Task 7 adds parse parity only. |
| Cross-language replay witness | Blocked | It would re-implement the private admission model and an unratified receipt schema (owner-review.md decision 3). Rust replay is already covered. |
| Production/crypto qualification of P1, key custody, promotion of P02.* and C01–C16 | Blocked | Design slice 6 independent crypto review; ABI exact-revision consumer qualification; C0–C7 operational evidence. |
| C11–C16 retrieval/guild/single-writer claims beyond the candidate tests | Blocked | Routed through tasks 3–4 and federation, which depend on the envelope. Efficacy needs research experiments; fencing needs separate-host federation. |
| Per-object payload encryption, tenant-derived keys, custody, executed key destruction (P03.02 step 2) | Blocked | The task-02 vNext envelope (ABI schema approval). The consumer owns the keys (`deletion.rs:1-22`). Crypto review. |
| Executed idempotent derived cleanup across outboxes, indexes, embeddings, caches, blobs and exports; disconnected acknowledgements (P03.03) | Blocked | Consumer-side execution outside WDBX needs ABI/Abbey exact-revision qualification. Disconnected acknowledgements need separate-host federation. |
| Backup restore into quarantine, applying the current deletion ledger, revalidating keys (P03 step 4, acceptance 3) | Blocked | No backup or restore code and no task-03 design entry. The independent deletion ledger depends on the envelope's deletion generation. Restore drills are external private staging. Task 15 is only the inconsistent-generation detector. |
| C17 Ephemeral and C21 mandatory-incident records in a persisted ledger | Blocked | The v3 `RetentionClass` is frozen with goldens and a witness. `RetentionIntent::{Ephemeral, MandatoryIncident}` lives only in complete_episode and depends on the envelope plus ABI approval. |
| Holds/revocation sidecar verify at open | Deferred (local) | The invariant "admitted or reclaimed" is unproven from `holds.rs`, and enforcing it could brick valid stores. It needs its own proof and three tests. |
| Re-admission of the same `payload_commitment` under a new request id after forget | Open design question | Tombstone identity is the episode digest. This is policy for the task-02 envelope or the deletion ledger, not a local defect. |
| Any C1+ evidence or promotion for P03.* and C17–C21 | Blocked | Independent review, ABI consumer qualification, crypto review, private-staging recovery drills. |
| Held-out repeated-failure and stale-reuse reductions (P04.03) | Blocked | Preregistered research efficacy experiments (experiments.json; none run) plus independent held-out analysis. The corpus-v1 harness is synthetic. |
| Measured E06 regime and a true wrong-regime disposition | Blocked | There is no task-regime field. It needs the complete-envelope schema (tasks 2–3) and ABI approval. Task 4 keeps the C0 proxy. |
| Measured E03 calibration, E02 reputation, E08 uncertainty, E07 age | Blocked | There are no such ledger records. Needs complete-envelope query integration and the ABI schema; hardware-health calibration also needs platform receipts. |
| E01 production key status / trust-root semantics | Blocked | Independent crypto review; ABI-owned trust-root schema and consumer qualification. |
| Qualifying any P04 or E row above C0 | Blocked | Independent review, ABI exact-revision qualification, a reviewed schema migration with an evidence verifier. |
| N03 recurrent spatial CPU forward pass and its numerical witness | Blocked | The message-layer equations are absent (deferred to the ignored `.work/drafts/wdbx-native-design.md`). No N03 packet exists. Freezing the equations and the rounding rules is an owner/controller approval (company M2/M4). |
| N04 backward pass, loss, optimizer, training and resume | Blocked | No training is authorized. It depends on N03 and on an undefined dataset manifest and preparation record (design §16). |
| Held-out sentinel enforcement at data preparation | Blocked | The N04 data-preparation contract. Task 5 asserts a trainer-API property only. |
| Author-independent trainer oracle (M7 proper) | Blocked | Needs an oracle author who is not the N02 implementer. Task 5's hand derivation is same-team mitigation. |
| Probing Specimen readers with model bytes | Blocked | They live in the separate `specimen-studio/` workspace; Studio-owned task-8 plan and gates. |
| N05 ABI admission and selection, N06 generation publication, N07 topology migration | Blocked | Need N03/N04 execution, ABI schema approval and consumer qualification, and staging and recovery drills. |
| P05.01–P05.04 and release TASK-19 (Retrieve/ProposeWrite/Verify, KV-bypass refusal, shadow/cutover, tenant rollback; PLAN-0232..0236, V6-1084..1108) | Blocked | ABI normative schema approval, vendor-tool corpus refresh, Abbey/Swift consumer migration and exact-revision qualification. The sibling gateway exposes only legacy RPCs. |
| Outbox `TerminalReceipt` serialization (accepted/rejected/conflicted) | Blocked | The gate task-07 tagged command/result schema with version negotiation, and no wire freeze before sign-off. Divergences for the owner: `SubmissionVerdict` has no receipt or epochs; `RefusalReason` differs from `ReasonCode`; `OperationId` is not 32-hex. |
| P06.01/P06.03 and TASK-17 PLAN-0216 (two independent administrations, partition/rejoin, revoked-peer faults) | Blocked | Separate-host federation with independent identities ("loopback is insufficient"); an independent trust-policy witness. |
| P07.01/P07.04 browser and native transactional outbox, tab lifetime, indefinite disconnect | Blocked | Studio sync owner (IndexedDB/native integration and a fault matrix). Shared vectors are gated on the task-07 schema review. Task 10 covers the native format only. |
| P09.01–P09.03 both-edition save/reopen/conflict/deletion | Blocked | Installed-edition qualification of browser and desktop Studio under Studio's own gates. |
| P10.01–P10.04, OS01–OS06, G10, G11, release TASK-21/22 | Blocked | Platform operators: hardware runners, signing and notarization, private staging, backup/restore drills, 30-day dual-target C7 operation. |
| Release TASK-16 three real voters / quorum-fenced shards | Blocked | Separate hosts plus the ABI transport owner. Local replication is loopback only. |
| Catalog FORMAT row for the `wdbx-federation-bundle-v1` profile | Deferred | Needs its own independently reviewed catalog migration after Task 21; out of this tranche's single migration scope. |
| Second-language verification of bundle and outbox signatures | Blocked | The independent crypto review gate. Stdlib has no Ed25519 and hand-rolled crypto is forbidden. Tasks 9–10 are encoding-agreement only. |
| G01, G02 process constraints | Blocked | They need per-run git, staging and synthetic-only operational evidence. Static checks cannot qualify them. |
| CH01, CH02, CH04, CH26 chapter qualification | Blocked | Each needs a chapter-wide normative-mechanism inventory with direct negatives and an independent chapter review. |
| S32 five-weak/four-strong history-preserving shortening | Blocked | Needs `engine::maintain` changes (ref-order retirement and tombstones, a 47-row pin drift) plus a D18 save-format decision. Withdrawn in `433ce50`. |
| S36 inhibited pairing including fan-out equivalents | Blocked | The fan-out-equivalence definition and the scoped privacy-safe MAC contract are absent, and any MAC needs crypto review. |
| S41 HybridTable / sandbox loop-hash-raft selection | Blocked | No sandbox planner exists. D16's median-latency selection is clock-nondeterministic, and a design ruling on a pure chooser is needed. `search.rs`/`lib.rs` edits drift 30+20 pins. |
| S16, S18 activator keys and the side-resource retrieval trace | Blocked | No activator concept exists. They need trace instrumentation in `engine.rs support()` plus a trace-schema ruling, and editing `engine.rs` drifts 47 pins and may move the WASM golden. |
| S24 closure (engine ATP path conformance) | Blocked | The engine uses wall time and `max(0)`, unlike the proposed monotonic profile. `atp_v3` is private and not integrated; the task-08 named-owner interface gate is open. Task 11 adds engine-path evidence only. |
| S44/S45/D18 versioned save envelope and migration | Blocked | The D18 envelope and migration design, native store qualification, and staging and recovery drills. |
| P08.04–P08.08 and CH12–CH17 efficacy; task-08 "all frozen efficacy thresholds pass" | Blocked | Research efficacy experiments with independent freeze verification and adjudication. No training is authorized. |
| Task-08 step 4 GPU/OCR/model-host qualification; desktop and cross-platform installed acceptance | Blocked | Platform installation, hardware and signing receipts (Developer ID). A configured job is not evidence. |
| Public kernel ATP/command ABI freeze and consumer cutover (atp_v3, scoring_v3, orchestration_v3) | Blocked | ABI normative schema approval / task-08 named-owner interface gate. |
| `rust-toolchain.toml` garbled comment | Deferred by choice | Rewording it would put its sha256 out of step with four historical snapshot records (`current-dependency-handoff.json`, `substrate-decisions/source-manifest-v1.json`, `substrate-release/baseline-v1.json`, `claims/wdbx-3d/source-manifest.json`). Fix it in a dedicated change that records those as intentionally historical. |
| Root `README.md` Gate section omitting the Python steps | Deferred | README is a live-pinned release source (`readme`). It needs a reviewed source-snapshot migration, not this tranche. |
| Opt-in P1 libsodium witness when the `/tmp` venv is gone | External prerequisite | Recreating the venv downloads PyNaCl 1.6.2 and needs explicit user approval. Until then the step is recorded as unexecuted and never skipped silently. |