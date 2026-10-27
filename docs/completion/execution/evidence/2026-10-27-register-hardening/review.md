# Independent review verdict: tranche 2 Task 1 (2026-10-27)

Reviewed range: `93dbeab..8abe9df` on `task/wdbx-tranche2-20261027`.
Verdict as relayed by the workflow controller: **APPROVE** (the report below
states **PASS**). Required fixes: none.

## Reviewer report (verbatim)

## Independent review: WDBX tranche 2 Task 1, commits 93dbeab..8abe9df

**Verdict: PASS.** All six checks hold. The catalog reproduces the live register exactly, and the hash matches. With the candidate pin applied in a scratch copy, the checker passes and all 33 tests pass. I found no required fixes, only minor notes.

Scope of the review:
- **Repository:** `/Volumes/Extreme SSD/Development/dev/repos/local/the-donald-company/company/legacy/wdbx`, branch task/wdbx-tranche2-20261027, HEAD 8abe9df4b648df3d997fc930e6c400394defa281.
- **Working tree:** clean before and after the review (`git status --short` shows 0 lines). I made no edits to the repository; all scratch files are under `/private/tmp/claude-501/-Volumes-Extreme-SSD-Development-dev-repos-local-the-donald-company-company-legacy-wdbx/248bc0ef-829b-404b-a799-84a94584d21f/scratchpad/review-scratch/`.
- **Environment:** pinned 1.99.1 toolchain first on PATH, then /usr/bin; PYTHONDONTWRITEBYTECODE=1; stdin at EOF for every run. `python3` is 3.15.2.

## Checks I ran

**1. Generator reproduces the catalog.**
- Ran `python3 docs/completion/execution/evidence/2026-10-27-register-hardening/generate_review_artifact_paths.py | cmp - docs/completion/execution/review-artifact-paths-v1.json`. Result: identical.
- I read the generator. It loads `register-v1.json` and, for each row in register order, emits `{id: {artifacts: [ref.path...], review: [ref.path...]}}` in reference order, as `json.dumps(indent=2)` plus a newline.
- It reads no other source and adds no filtering or derived data, so it only copies the register's reference paths.
- Its default root, `parents[5]`, resolves to the repository root.

**2. Hash.** `shasum -a 256` gives `0a753f848ad7aa0816b6b6abe9adac6326eaa2a710404959deb236987cca7fcf`, as expected.

**3. Catalog contents against the live register (my own script).**
- 163 register rows, 163 unique IDs and 163 catalog keys.
- Catalog key order equals register row order.
- For every ID, the `artifacts` and `review` lists equal the register's reference paths, both in order and as multisets. There were 0 mismatches, and every entry has exactly the keys {artifacts, review}.
- No row has a duplicate path.
- Every row cites `docs/completion/plans.json`.
- Every cited path exists.
- The candidate catalog splits as 131 candidate, 16 local_gap and 16 external_gap, and its key set equals the register's IDs.

**4. Review pointers.**
- 55 rows carry review references, and all of them are candidates, so no gap row has one.
- There are 21 unique review paths. 7 are under `evidence/2026-10-23-scoring/`: 4 reviews and 3 fix1 rereviews.
- The other 14 are:
  - 7 language reviews under `2026-10-22-language`;
  - 2 studio reviews: `identity-task-review` and `parity-task-review`;
  - 5 receipts: `docs/execution/p1-local-slices-summary.md` and 4 `docs/superpowers/plans/*-receipt.md` files.
- The README's "14 historical" bullet is therefore not stale, and the new "7 scoring-packet" bullet and the "21 unique review paths" total are correct.

**5. Frozen inputs untouched.**
- `git diff 93dbeab 8abe9df --stat` over `register-v1.json`, `candidate-paths-v1.json`, `requirements.json` and `plans.json` is empty.
- No `catalog-v1.json` exists in `docs/completion/execution/`. The only tracked one, `docs/completion/substrate-release/catalog-v1.json`, does not appear in the changed-file list either.
- The range changes 17 files: the execution README, `progress.md` (an appended note only; the dated 95/68 line is unchanged), the new catalog, the checker, the tests and the evidence directory.

**6. Checker logic (`tools/check_completion_execution.py`).** I read the whole file and probed it with mutations against a scratch copy that has the candidate pin set.
- **Unset pin:** with the pin at `None`, the checker emits `review/artifact catalog pin pending review` and exits 1, so it fails closed. The drift comparison is skipped only while the pin is unset. With the pin set, faking the file's sha gives `review/artifact catalog drift`.
- **Closure over 163 IDs:** the checker compares the catalog's key set with the IDs in the contract-pinned `requirements.json`.
  - Dropping an ID gives `review/artifact catalog requirement closure`.
  - An extra key inside an entry fails closed as invalid structure.
- **Per-row identity:** each row's review and artifact paths must match the catalog as a multiset. My probes:

  | Mutation | Error reported |
  |---|---|
  | Add a review pointer to candidate G03 | `review identity drift: G03` |
  | Move a review pointer from P02.01 to G03 | drift on both rows |
  | Duplicate a review pointer | `review identity drift: P02.01` |
  | Strip artifacts on P02.01, P02.02, P02.03 or C14 | `artifact identity drift: <ID>` |

- **Gap rule:** `not is_candidate and references['review']` is decided from the register's own reconciliation disposition, which is tied to the pinned candidate catalog. It never consults the review catalog. When the catalog is altered to agree with the change, a review pointer added to OS06 still gives `gap cannot carry review: OS06`.
- **README parser:**
  - It requires exactly one match for each of the five bullet integers. It expands en-dash ranges (`P05.01–P05.04`, `P10.01–P10.04`, `OS01–OS06`), refuses duplicates and compares the Local and External gap lists as ID sets against the candidate catalog.
  - Ranges with mismatched prefixes, mismatched digit widths, reversed bounds or malformed tokens raise errors that are caught and reported as drift.
  - My probes all report README count drift: a hyphen instead of an en dash, a duplicated bullet, a removed scoring bullet, and an ID moved from the local list to the external list.
- **No promotion:** nothing writes or promotes anything. Any status other than `open`, any evidence level other than C0 or any test result other than unknown is refused (`unreviewed promotion`). The checker only reads files.

## Gates
- **Unpinned (as committed):**
  - `python3 tools/check_completion_execution.py` exits 1 with only `review/artifact catalog pin pending review`.
  - `python3 -m unittest discover -s tools/tests -p test_check_completion_execution.py` runs 33 tests with 2 failures: `test_current_register` and `test_review_artifact_catalog_hash_is_independently_pinned`. Both fail only because the pin is pending, as the evidence README says.
- **Pinned dry run (scratch copy with only the pin line changed):** the checker reports "Execution register valid: 163 reconciled open C0 requirements", exit 0, and the 33 tests pass.
- **`python3 tools/check_completion.py`:** "Completion contracts valid: 163 C0 requirements, 46 acceptance cases", exit 0.
- **Evidence logs:** I spot-checked them against my runs. They agree: red (3 failures, 3 errors), dry-run pinned OK, regenerate identical, release drift empty, and the gap-rule mutation failing.

## Minor findings (not blocking)
1. Per-row identity is checked as a multiset, so reordering references inside a row of the register is not detected. This matches the plan's "multiset identity", and the catalog's own order is fixed by the sha pin.
2. The add-review test uses G01, a gap row, so the gap rule fires as well. A candidate-row variant would isolate the identity check; I confirmed manually that G03 is caught.
3. The total-review regex is not anchored to a bullet, but it still requires exactly one occurrence.
4. A malformed catalog entry is reported as the generic "invalid register structure" error. It still fails closed.
5. The review request names a `catalog-v1.json` that does not exist under `docs/completion/execution/`.

## Qualification limits
This review covers only the integrity of the execution-inventory checker and catalog. All 163 requirements remain open/C0. It does not establish requirement completion, review qualification or any platform or production claim.

On PASS, the writer can set `REVIEW_ARTIFACT_PATHS_SHA256 = '0a753f848ad7aa0816b6b6abe9adac6326eaa2a710404959deb236987cca7fcf'`, update the evidence README's Review status, rerun the three gates into the evidence directory and commit with a distinct subject.

## Author disposition

- **Pin:** set `REVIEW_ARTIFACT_PATHS_SHA256` in `tools/check_completion_execution.py`
  to the reviewed bytes, `0a753f848ad7aa0816b6b6abe9adac6326eaa2a710404959deb236987cca7fcf`.
  `review-artifact-paths-v1.json` is unchanged since review; its sha256 was
  rechecked before the pin was set.
- **Minor finding 1 (multiset, not order):** accepted as designed. The plan asks
  for multiset identity. Reference order within a register row is not part of
  the contract, and the catalog's own order is fixed by the sha pin. No change.
- **Minor finding 2 (G01 test also trips the gap rule):** accepted as a test
  coverage note. The reviewer manually confirmed a candidate row (G03) is caught
  by the identity check alone. No test change in this task, to keep the reviewed
  test bytes intact; a candidate-row variant can be added in a later task.
- **Minor finding 3 (total regex not bullet-anchored):** accepted. It still
  requires exactly one occurrence, so a duplicate or missing statement fails. No
  change.
- **Minor finding 4 (generic structure error for a malformed entry):** accepted.
  It fails closed. No change.
- **Minor finding 5 (`catalog-v1.json` named in the review request):** the
  review request mentioned `catalog-v1.json` as an untouched frozen input; the
  only one is `docs/completion/substrate-release/catalog-v1.json`, which this
  task did not touch. Wording error in the request only. No change.
- **Scope:** no requirement, register row, candidate catalog or release catalog
  changes. Every row stays open/C0/unknown. This verdict covers only checker and
  catalog integrity; it is not requirement, review or platform qualification.
