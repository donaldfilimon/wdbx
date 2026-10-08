# Abbey Bot learning-quality fix3 review — 2026-10-03

**Standards: PASS. Spec: PASS. No open source findings in this repair.**

LQ5-1, LQ5-2 and LQ5-3 from the external review are closed in the frozen source.
This approves the narrow repair for the coordinator-owned strict gate; it does
not mark Task 5 complete. Task 5 remains **Partial** pending qualification of
this source and the separate acceptance layers below.

## Scope and identity

Canonical checkout: `/Users/donaldfilimon/dev/active/abbey-bot`.
Reviewed exactly the four modified files and one new test module:

- `src/brain/dqn.rs`
- `src/brain/replay.rs`
- `src/brain/quality_evaluation.rs`
- `src/learning_quality_cli.rs`
- `src/brain/dqn/import_tests.rs`

Evidence directory:
`.superpowers/sdd/2026-10-01-mlai-learning-quality/task5-quality-evidence-20261003/fix3/`.
The review read `repair-report.md`, `repair-review.diff`, `source-freeze.json`,
`focused-results.json`, the referenced logs and actual source. The explicit
acceptance source is `/tmp/abbey-all-20261002-learning-quality-review.md`,
LQ5-1 through LQ5-3, together with the coordinator's frozen-scope brief.

- Recorded HEAD: `e742375ff9b3d20b97df09c7fad10c1225ae957c`.
- Declared scoped aggregate SHA-256:
  `89b3a50fb3171dc9bc898d7e3e4707c5a56e8e4c2b49e1b2d7132a35f1886975`.
- Reviewer-computed `repair-review.diff` SHA-256:
  `6bd6c860abd94a1836e68da2b07a9be3b6e0861a9a8a7ed1919f968babe14092`.
- Both reviewers independently matched all five current file hashes and lengths
  to `source-freeze.json`; this reviewer also matched recorded line counts.

Standards followed the operator/repo instructions, module contracts and the
mattpocock code-review heuristic baseline. A separate read-only agent reviewed
Spec independently. Reviewers made no Bot source edits and ran no broad gate,
provider, service, deployment or media operation. Only this review file was
written by the primary reviewer.

## Standards

**Disposition: PASS; 0 actionable findings.**

The repair stays in the intended pure-policy and CLI seams. No clock, network,
new production `unwrap()`, lock acquisition, dependency, training constant or
learning-erasure change was added. The test-only unwraps assert fixed fixture
setup. Cloning imported transitions is required to own the replacement buffer
while accepting a borrowed snapshot; cloning the online network preserves the
existing target synchronization contract. No unnecessary clone finding was
established.

At `src/brain/dqn.rs:272`, topology and layer-shape validation, finite-value
validation and every action check occur before any destination mutation.
Replacement replay is constructed locally at `:322`; mutation begins at `:329`
only after all typed rejection paths have completed. The destination capacity
is already positive by the constructor invariant. No rejection path touches
the RNG, target, replay cursor or other private state.

At `src/brain/quality_evaluation.rs:256`, the public shape contract is enforced
in one place. CLI evaluation delegates to it at `src/learning_quality_cli.rs:51`,
preserving the closed error and the independent opened-file/byte limits. All
case validation completes before report counters, policy construction or
grounding begin. The private subset helper serves focused fixtures without
becoming another public admission path.

The new external test module is 152 lines; all five owned files are below the
repository's 800-line review threshold and 1,000-line cap. Regression assertions
cover the observable failure scenarios and private state necessary to detect
partial restore; they do not merely restate the newly added branches.

## Spec

**Disposition: PASS. Independent Spec review closes the three external findings;
0 new findings.**

### LQ5-1 — Invalid imported action could panic during training

- **Severity:** Original High (P1).
- **File:line:** `src/brain/dqn.rs:311`; JSON adapter at `src/runtime.rs:107`;
  regression at `src/brain/dqn/import_tests.rs:38` and `:62`.
- **Description:** Import now returns typed `ReplayActionOutOfRange` for an
  action equal to the output count or larger. It checks every row, including
  rows that will subsequently be skipped for legacy state-width mismatch.
  JSON import delegates to this same checked path and returns false on refusal.
- **Suggestion:** No further source repair required for this finding.
- **Status:** Closed in fix3. Tests use already-trained destinations with a
  lagging target and a wrapped buffer larger than the 1,000-row exported tail.
  The helper at `import_tests.rs:25` compares online/target networks, full replay
  storage/cursor/capacity, epsilon, step count and RNG. Additional refusal tests
  cover topology, shape and nonfinite values, followed by identical subsequent
  learning on the unchanged control agent at `:129`.

### LQ5-2 — Valid restore appended old and duplicate replay

- **Severity:** Original Medium (P2).
- **File:line:** `src/brain/dqn.rs:322`, `:336`;
  regressions at `src/brain/dqn/import_tests.rs:76`, `:94` and `:105`.
- **Description:** A fresh replay buffer replaces destination replay, preserving
  its capacity. Snapshot rows are processed oldest first, so overflow retains
  the newest compatible rows. Repeating a restore cannot duplicate old rows;
  an empty or legacy-absent `experiences` field clears existing replay.
- **Suggestion:** No further source repair required for this finding.
- **Status:** Closed in fix3. Existing width-skip behavior remains explicit,
  after finite/action validation. Tests verify exact exported snapshot equality
  when rows fit, repeated restore, legacy omission, overflow order and the next
  insertion. The snapshot has no RNG or lagging-target field: valid restore
  deliberately retains destination RNG and synchronizes target to online,
  consistent with the documented previous semantics. This is not a claim that
  snapshots serialize every training-process state component.

### LQ5-3 — Public corpus boundary allowed partial/unbounded evaluation

- **Severity:** Original Medium (P2).
- **File:line:** `src/brain/quality_evaluation.rs:256`, `:276`, `:190`;
  public-shape regression at `:348`.
- **Description:** `evaluate_corpus` now requires exactly 100 cases and all five
  enum classes at 20 each before entering case evaluation. It then validates
  every case's limits, identities, citations and correction labels before
  grounding, result counting or action selection. Existing bounded text/list
  validation from the earlier repair remains intact, including token limits
  before dotted-version expansion. Direct single-case evaluation also retains
  the limit guard at `:158`.
- **Suggestion:** No further source repair required for this finding.
- **Status:** Closed in fix3 when assessed with the retained field/list repair.
  New tests reject empty, one-case, 99-case, 101-case and wrong-distribution
  corpora. The private `evaluate_subset` keeps the meaningful malformed-field
  tests without weakening the public complete-corpus contract. CLI shape checks
  delegate to that public API while preserving its separate 1 MiB ingress bound.

## Evidence and remaining boundaries

**Current — supplied logs inspected by the reviewer:**

| Check | Recorded result |
| --- | --- |
| Pre-repair import regressions | Exit 101; 4 failed, 0 passed |
| Pre-repair public corpus regression | Exit 101; 1 failed, 0 passed |
| Locked Rust 1.98.0 `brain::` tests | Exit 0; 242 passed, 0 failed/ignored |
| Locked Rust 1.98.0 CLI unit tests | Exit 0; 5 passed, 0 failed/ignored |
| Formatting | Exit 0 |
| All-target locked Clippy with `-D warnings` | Exit 0 |
| Worker diff check | Exit 0 |

The reviewer independently ran the scoped `git diff --check`, which exited 0.
No test command above was rerun by the reviewers. Counts agree with the retained
logs; zero-length formatting/diff logs are accompanied by recorded process exit
codes rather than treated as standalone success evidence.

**Partial:** the next strict combined gate and actual-process CLI evidence must
qualify fix3 and its rebuilt artifact. Earlier fix2 results qualify their own
snapshot only. Human adjudication of the synthetic corpus, fixed-provider/manual
answer support before tuning, installed artifact identity and live Discord/runtime
acceptance remain separate and open. This review does not change that status.

**Out of scope:** external Task 4 erasure/reset work, policy tuning, broader
learning features, provider calls, deployment, commits and pushes. The five-file
repair contains no Task 4 erasure/reset changes; this scoped review does not
requalify Task 4 or prove whole-checkout incoming-baseline preservation.

**Review totals:** Standards 0 findings; Spec 3 historical findings closed,
0 open. Both axes pass the frozen fix3 source repair.
