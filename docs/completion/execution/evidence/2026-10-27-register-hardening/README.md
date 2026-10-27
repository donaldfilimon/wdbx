# Register hardening: review/artifact identity and README counts (2026-10-27)

Tranche 2, Task 1 (`docs/plans/2026-10-27-wdbx-completion-tranche-2.md`).
Requirements G13 and P01.01 stay proposed/C0. Every register row stays
open/C0/unknown. `register-v1.json` and `candidate-paths-v1.json` are unchanged.

## What changed

- `docs/completion/execution/review-artifact-paths-v1.json` is new. For each of
  the 163 IDs, in register row order, it lists the exact `artifacts` and `review`
  reference paths in register order. It records path identity only; the sha256
  pins stay in the register. `generate_review_artifact_paths.py` in this
  directory regenerates it byte-for-byte from the live register.
- `tools/check_completion_execution.py` now:
  - pins the new catalog's sha256 (`REVIEW_ARTIFACT_PATHS_SHA256`, still `None`:
    see "Review status");
  - requires the catalog to cover exactly the 163 requirement IDs;
  - compares each row's artifact and review path multisets with the catalog
    (`artifact identity drift`, `review identity drift`);
  - refuses review references on non-candidate rows (`gap cannot carry review`),
    independently of the catalog;
  - compares the README count statements with the two pinned catalogs.
- `docs/completion/execution/README.md` gains one bullet for the 7 scoring-packet
  review pointers and a paragraph describing the new checks.
- `docs/completion/execution/progress.md` gains an appended superseded note.
  The dated 95/68 line is unchanged.

## README count semantics (Step 2)

The register cites 21 unique review paths. 7 lie under
`evidence/2026-10-23-scoring/`, and 14 lie outside it: 7 language reviews,
2 studio reviews and 5 plan/summary receipts. The existing "14 historical
review/receipt pointers" bullet is therefore not stale. It counts the paths
outside the scoring packet. The rule is encoded as:

- historical = unique review paths outside `SCORING_REVIEW_PREFIX` (14);
- scoring-packet = unique review paths inside it (7, new bullet);
- total = 21 ("21 unique review paths").

The parser compares the five bullet integers exactly. It does not compare only
counts for the lists: it expands en-dash ranges (`P05.01–P05.04`,
`P10.01–P10.04`, `OS01–OS06`) and compares the `Local gaps:` and
`External gaps:` lists as ID sets against the candidate catalog's dispositions,
refusing duplicates.

## Logs

- `red-unittest.log`: the six new tests before implementation (3 failures and
  3 errors). Today's checker returned `[]` for the stripped C14 artifacts and
  the added G01 review reference.
- `green-unpinned-unittest.log` (current Python) and
  `green-unpinned-unittest-system-python.log` (`/usr/bin/python3` 3.9.6): 33 tests
  with 2 expected failures, `test_current_register` and
  `test_review_artifact_catalog_hash_is_independently_pinned`. Both fail only
  because the pin is pending review.
- `checker-unpinned.log`: `review/artifact catalog pin pending review`.
- `dryrun-pinned-checker.log` and `dryrun-pinned-unittest.log`: a scratchpad
  copy of the checker with the candidate pin
  `0a753f848ad7aa0816b6b6abe9adac6326eaa2a710404959deb236987cca7fcf`. The checker
  is valid, and all 33 tests pass. The repository pin was not set.
- `mutation-gap-rule-removed.log`: with the gap rule deleted from the scratchpad
  copy, `test_gap_rows_cannot_gain_review_refs` fails. The rule was then restored.
- `regenerate-compare.log`: the generator output is byte-identical to the
  committed catalog.
- `check-completion.log` and `substrate-release-drift.json`: the frozen
  completion checker is valid, and live release drift is empty.

## Review status

Reviewed. An independent read-only reviewer verified range `93dbeab..8abe9df`
and returned PASS/APPROVE with no required fixes. The report is recorded
verbatim in `review.md`, followed by the author disposition of its five minor
notes. `REVIEW_ARTIFACT_PATHS_SHA256` is now set to the reviewed bytes,
`0a753f848ad7aa0816b6b6abe9adac6326eaa2a710404959deb236987cca7fcf`. The catalog was
not changed after review.

Gates rerun after the pin was set:

- `checker-pinned.log`: the execution register is valid (163 reconciled open C0
  requirements).
- `pinned-unittest.log` (current Python) and
  `pinned-unittest-system-python.log` (`/usr/bin/python3` 3.9.6): 33 tests, OK.
- `check-completion-pinned.log`: completion contracts valid.
- `substrate-release-drift-pinned.json`: live release drift is empty.

The earlier unpinned logs remain as the pre-review record. This review covers
only checker and catalog integrity. It is not requirement, review or platform
qualification, and every row stays open/C0/unknown.
