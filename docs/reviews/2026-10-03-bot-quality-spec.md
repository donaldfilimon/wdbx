# Abbey Bot Task 5 SPEC review — 2026-10-03

**SPEC verdict: PASS for the bounded source deliverable; Task 5 acceptance remains Partial.** No blocking implementation mismatch or unauthorized scope expansion found. Independent of the standards review.

Authority: local learning-quality design lines 24, 74–77; plan Task 5 lines 209–246, 264–267; progress.md Task5 integration ruling line 104. Reviewed the 13-path owned-review.diff, including new files, against live source at HEAD `e742375ff9b3d20b97df09c7fad10c1225ae957c`; unrelated dirty work was outside review ownership. No issue-tracker authority was supplied.

Current source coverage:

- Corpus bytes independently checked: 100 distinct IDs, five classes of 20, SHA256 `20237fe9320d68dd1ff60af329c308ec7f67322817e9cf86087fabd308b0fb0f`.
- `src/brain/quality_evaluation.rs:153–176` implements the spec's “current cited source records only” and `Verdict::is_grounded` conditions; declared contradiction causes abstention. This proves lexical policy, not semantic truth. Lines 179–219 reject invalid identities/references; 122–129 and 238–255 count actual TP/FP/TN/FN and correction decisions. Lines 229–266 use a seeded untrained policy without learning or persistence.
- `src/main.rs:276–278` exits before initialization; `src/learning_quality_cli.rs:46–88,92–127` enforces schema/provenance, 100/20 counts, 1 MiB reads and closed reports/errors. Actual-process tests exist at `scripts/test-learning-quality-startup.py:20–81`.
- `src/brain/dqn.rs:256–308` rejects topology/shape/nonfinite imports before mutation. `quality_evaluation.rs:386–509` covers deterministic seeds, refusal atomicity and scoped rollback/reset preserving serialized canonical facts and the other scope. No production training, provider calls or policy tuning added.

| Severity | File:line | Description | Suggestion | Status |
|---|---|---|---|---|
| P2 acceptance gap | tests/fixtures/learning-quality-v1.json:3 | Design:74 requires “human-curated expected” labels. Current labels are agent-authored, honestly declared under progress:104; fixed-provider/manual support evidence is also absent. | Obtain independent human adjudication and operator-reviewed fixed-provider measurements before tuning; retain synthetic-only claims. | Partial; nonblocking source review |
| P3 evidence gap | docs/verification/2026-10-03-learning-quality-v1.md:3 | Plan:246 requires recording evidence in `docs/MLAI-LIVE-ACCEPTANCE.md`; its Task5 entry is absent. Strict terminal/stability evidence remains pending. | Append exact reviewed-source/gate evidence and unresolved layers after the running gate finishes. | Open completion step |

Read-only review and lightweight corpus inspection only; no tests/builds, bot edits, commit, deployment or live acceptance performed.
