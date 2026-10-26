# Wave 1 tranche 1 (WDBX) Implementation Plan

> **For agentic workers:** executed natively by the tranche-1 lane (no subagents; the controller reviews). Steps use checkbox (`- [ ]`) syntax.

**Goal:** Honest green baseline for WDBX: Studio `build:wasm` on the pinned toolchain, the unused `AdmissionSimulator` re-export warning removed, and three of the 18 local-gap requirements (S29, S32, S33) moved to direct-test candidate mappings.

**Architecture:** Script-level toolchain pinning in `specimen-studio/package.json`; a `#[cfg(test)]` gate on a test-only re-export; new kernel integration tests that assert the frozen S-case rows, followed by the repository's established register procedure (catalog disposition, checker pin, register row, README counts, evidence packet).

**Tech Stack:** Rust 1.99.1 (rustup), bun, Python 3 standard-library validators.

**Spec:** `$DEV_HOME/projects/finish-all-repos/notes/2026-10-26-wave1-tranche1-design.md` (section "legacy/wdbx") and program rules §3 of `notes/2026-10-26-finish-all-repos-design.md`.

## Global Constraints

- Pinned stable Rust 1.99.1 via rustup; never Homebrew cargo. `CARGO_BUILD_JOBS=2`. Stdin at EOF for tests.
- Root gate `bash tools/check.sh`; Studio gate `bun run check` in `specimen-studio/`; plus `cargo test --locked -p abi-wdbx`.
- Register claim policy is `C0-only; no promotion`: every row stays `open` / `C0` / `unknown`. "Closing" a local gap in this tranche means local_gap -> candidate with direct tests cited; it is not qualification.
- Frozen contracts (`requirements.json`, `plans.json`, `designs.json`, `studio-acceptance-v1.json`) are never edited.
- Hash-pinned references that drift are refreshed only explicitly, with a recorded refresh file (precedent: commit 1557c3b).
- abi-wdbx public API unchanged (legacy/abi and legacy/abbey compile against it).
- No push, network, signing, dirty-worktree edits, reset, branch/worktree deletion.

## Review Focus

- `CARGO_TARGET_DIR` set in the caller's environment: `build:wasm` copies from `target/`; the copy must follow `${CARGO_TARGET_DIR:-target}`.
- Non-test build of abi-wdbx must stay warning-free after gating the re-export (verify with `cargo build -p abi-wdbx` and clippy all-targets).
- S32 boundary: a remixed alternative beyond the shortening count must block the mutation (history preservation) and leave pools byte-identical.
- S29 boundary: removing the last node of a patternId clears linked resource IDs; removing one of two peers keeps them.
- Register checker drift: any source edit pinned by the register must be refreshed and the three validators plus the checker unittest rerun.

---

### Task 1: Studio `build:wasm` on the pinned toolchain

**Files:** Modify `specimen-studio/package.json` (`build:wasm` script).

- [ ] Run `bun run build:wasm` with Homebrew cargo first on PATH; expect E0463 (reproduces Wave 0 failure).
- [ ] Change the script to `rustup run 1.99.1 cargo build --locked -p specimen-wasm --target wasm32-unknown-unknown --release && mkdir -p lib/specimen/wasm && cp -f "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/specimen_wasm.wasm" lib/specimen/wasm/`.
- [ ] Run the full Studio gate detached: `bun run check < /dev/null`, log `.work/wave1/gate-<UTC>.log`, expect `EXIT=0` (or a recorded non-toolchain cause).
- [ ] Run the PyCA probe `python3 tools/wdbx_complete_p1_crypto.py backend-probe --qualify` and record its deliberate non-zero result as the retained qualification failure.
- [ ] Commit `fix(studio): build wasm through the pinned rustup toolchain`.

### Task 2: Remove the unused `AdmissionSimulator` re-export warning

**Files:** Modify `crates/abi-wdbx/src/v3/complete_episode/crypto/admission/mod.rs:11`; refresh pins in `docs/completion/execution/register-v1.json`; add `docs/completion/execution/evidence/2026-10-26-wave1-tranche1/reference-refresh.json`.

- [ ] Confirm the warning: `rustup run 1.99.1 cargo build -p abi-wdbx 2>&1 | grep 'unused import'`.
- [ ] Gate the re-export with `#[cfg(test)]` (only `scenarios.rs`, itself `cfg(test)`, uses it). `pub(super)` is not public API.
- [ ] Rebuild (non-test) and `cargo clippy -p abi-wdbx --all-targets`: no `unused import`.
- [ ] Run `python3 tools/check_completion_execution.py`; refresh only the drifted pins it names; record the refresh; rerun the three validators and `tools/tests/test_check_completion_execution.py`.
- [ ] Commit `fix(abi-wdbx): gate test-only AdmissionSimulator re-export`.

### Task 3: Direct tests for S29, S32, S33 and register move

**Files:** Create `specimen-studio/native/specimen-kernel/tests/lifecycle_local_gaps_v1.rs`; modify `docs/completion/execution/candidate-paths-v1.json`, `tools/check_completion_execution.py` (`CANDIDATE_PATHS_SHA256`), `docs/completion/execution/register-v1.json`, `docs/completion/execution/README.md`; evidence in `docs/completion/execution/evidence/2026-10-26-wave1-tranche1/`.

Tests (FixedHost, Markov fixture from `specimen-core/tests/conformance.rs` mutation test):
- `s29_spent_same_pattern_node_is_removed_and_peers_remain`: two nodes share a `patternId` (refs `a`, `b`); negative feedback takes `a` from 1 to 0; `a` and its attachments removed, `b` byte-identical, linked resource keeps `resourceId`.
- `s29_removing_last_same_pattern_node_unlinks_resources` (boundary).
- `s29_unflipped_node_at_one_is_not_removed` (negative: coin flip does not apply).
- `s32_five_weak_with_four_strong_shortens_recipient_to_four`: recipient has 4 alternatives after commit; slot ids of first four preserved; donor unchanged.
- `s32_remixed_tail_blocks_shortening` (boundary/negative: 5th alternative `remixed: true` -> no mutation, nodes unchanged).
- `s33_four_weak_with_five_strong_keeps_four`: recipient remains 4, no fifth alternative; donor still 5.

Register move per ID: catalog disposition `candidate`, `implementation` = `engine.rs` (S32/S33) or `mutate.rs` (S29), `tests` = the new file; `inspected` extended with them; repin checker; register rows get sha256 references and rewritten blockers (still open/C0/unknown); README counts 132/15/16.

- [ ] Write tests; run `rustup run 1.99.1 cargo test --locked -p specimen-kernel --test lifecycle_local_gaps_v1`; all pass (behavior already implemented; the tests are the missing direct qualification). Mutation-check: temporarily invert one assertion to confirm each test can fail, then restore.
- [ ] Commit tests; then register move with evidence (`requirement-mapping.json`, logs); validators green; commit.

### Task 4: Final gates and ledger

- [ ] Detached on final HEAD: `bash tools/check.sh`, `cargo test --locked -p abi-wdbx`, Studio `bun run check`, `bun run test:native`, the register validators. `EXIT=` appended to each log.
- [ ] Append `## Wave 1 tranche 1` to the program ledger; write the report.
