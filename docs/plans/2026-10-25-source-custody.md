# Restore WDBX master source custody

**Goal:** Restore the inherited, hash-matching v5 master to WDBX without rewriting historical evidence or importing a runtime dependency on LYLX.
**Architecture:** An additive repository-owned source record and independent read-only verifier, beside the existing frozen completion inventories.
**Spec:** Approved product modernization master, milestone 1; inherited substrate catalog `missing_sources` and source identity `AUDITED-SOURCE-02` from preserved custody.
**Tech stack:** Standard-library Python, existing WDBX document tooling.

## Global Constraints

- Preserve the five-crate layering and every historical requirement, source pin, receipt and failure.
- Restoring a source document establishes custody only. It does not establish implementation, trained-model quality, native device or release qualification.
- Do not edit the historical substrate catalog, baseline, source snapshots or their checker pins to remove its earlier unavailable-original observation.
- Tracked paths are repository-relative. Recovery source checkout locations belong only in private execution records.
- Preserve the restored master bytes exactly, including original wording and line endings; amendments remain separate.
- Original checkouts, live stores, app-managed worktrees and other writers' edits are not implementation targets.

## Task 1: Add verified v5 source record

**Owned files:** Create `docs/source-records/wdbx-master-v5.md`, `docs/source-records/custody-v1.json`, `docs/source-records/README.md`, `tools/check_source_custody.py`, `tools/tests/test_check_source_custody.py`. The controller already owns this plan. No changes to root README/AGENTS or existing completion inventories in this task.

**Source:** Copy the preserved object whose SHA-256 is `f98d61f6f80503a2d00687c8cb6f38ed654cf9ad46cf55407ba5e06765354822` and size is exactly `344347` bytes; the controller supplies its private input path. Original source identity is `AUDITED-SOURCE-02`, title `WDBX-3D-Neural-Specimen-Master-Plan-v5.md`. Source custody was recovered from a 2026-10-20 Time Machine snapshot into LYLX's historical-source object store. Record this origin and receipt identities, but no absolute machine paths. There is no new requirement numbering or adoption claim in this packet.

**Interfaces:** `python3 tools/check_source_custody.py [--repo-root ROOT]` works from arbitrary cwd, verifies fixed expected source identity/hash/length, manifest shape and safe regular repository paths, writes nothing and returns nonzero for failure. Validation must derive its expected digest from independently fixed code, not trust a coordinated edited manifest and file. Diagnostics are bounded and content-free. No auto-refresh mode. Default root is relative to the checker file, not a sibling checkout.

**Steps:**

- [ ] Read AGENTS and relevant agent-guide completion/claims rules, catalog missing-source entry and custody manifest. Inspect original headings and compatibility restrictions; do not rewrite the original.
- [ ] Add regression tests that reject absent/truncated/modified source, edited hash/size/identity, path escape and symlinked paths, plus valid unchanged bytes from arbitrary cwd. Observe failures before implementing verifier.
- [ ] Copy exact bytes, write the additive custody manifest and verifier. Include read-only origin references by repository identity and relative artifact names. Preserve old unavailable-original receipts as dated history.
- [ ] Document the explicit remaining v5/v6 requirement reconciliation and separate new online-profile amendment, linking existing substrate inventory. No false completion/status changes.
- [ ] Run focused tests, custody verifier, existing completion and substrate checkers, and diff whitespace check excluding the immutable source blob. Record the existing completion-execution AGENTS reference drift separately; do not silently repin it.
- [ ] Commit owned files and report exact hash/size and test evidence.

**Rollback:** Revert this additive task commit. No live state, executable format or prior evidence is mutated.
