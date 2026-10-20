# Independent final packet review

Verdict: **CHANGES REQUESTED — one P2 resource-admission finding.** No P0/P1
finding, legacy compatibility regression, unsafe prune, or canonical-byte change
was found. Acceptance is limited to the authorized T00–T06
`spatial-checked-v1` packet.

## Finding

**P2 — Admit radius payload bytes before growing the output buffer.**
`crates/abi-wdbx/src/spatial3d/octree_checked.rs:290–303` checks only
`(hits.len() + 1) * size_of::<CheckedSpatialHit>()` before reserving and pushing a
radius hit. Borrowed payload logical bytes are checked only after traversal at
lines 482–488. `QueryLimits.max_output_bytes` expressly includes those bytes
(`legacy_checked.rs:14`), and v5 T05 step 3 requires output-budget admission
before allocating the output buffer; T06 consumes that shared contract.

For a matching record with one payload byte and a budget of exactly one hit
slot, the adapter reserves and retains the hit even though its known logical
output charge already exceeds the budget. With many matching records it can
continue allocating and traversing after the first irrecoverable byte overrun.
The final error prevents an oversized successful response, and the borrowed
payload is not copied; this is an admission-contract failure, not a claim of
unbounded allocation or incorrect successful results. It affects both the tree
and complete fallback paths because both use `Selection::consider`.

Radius hits are append-only final winners, so the top-k justification for
charging only final winner payloads does not apply. Track cumulative radius
logical bytes with checked arithmetic and reject before `try_reserve_exact` and
`push`. Preserve the existing top-k treatment of replaceable candidates. Add
exact/one-byte-under payload-boundary tests for indexed and fallback radius
queries, plus a focused test that rejection leaves the selection buffer
unchanged before reservation. The current T06 resource test at
`wdbx_3d_octree_parity.rs:222–246` exercises top-k and a zero slot-byte budget,
so it cannot detect this ordering error.

## Reviewed scope and positive conclusions

Reviewed the complete code changes represented by `final-diff.patch`, including
current Rust/Python/test-fixture files, T00–T06 reports, existing task reviews,
progress/acceptance documents, the complete 61-row manifest and source audit.
Controlling review references were supplied v6 §§1, 3, 12–14, 22, 49 and 51,
and v5 §§5.2/5.5 and T00–T06. This review does not claim to repeat the source
audit workers' complete original-authority reads.

- The tracked production changes preserve crate identities, legacy f32 types,
  old method bodies, duplicate IDs and historical ties. The compute change is
  formatting only. No signing, canonical commitment, parent-order or durable
  storage implementation is changed. Compatibility fixtures use isolated
  synthetic scratch data and report the v2 object-layer limitation accurately.
- Private point/radius representations enforce the finite `1e150` domain and
  normalize only new geometric zero. Widening precedes subtraction. Euclidean
  selection uses non-FMA squared XYZ keys before display sqrt, then ID and
  ordinal. The independent complete oracle and bit-derived BigInt oracle are
  separate; explicit disagreement tests prevent conflating defined-float
  semantics with exact-real predicates.
- The transient adapter reuses original 64/8/16 constants and existing midpoint,
  octant and child-bound primitives. Boxes contain assigned represented
  coordinates; componentwise monotone lower bounds use the same reduction order
  as keys. Strict greater-than pruning preserves boundary/tie candidates.
  Manhattan uses its own sound bound; cosine takes the declared complete scan.
  Rebuilding each exclusive borrow prevents stale checked topology after insert.
- Node/scratch reservations are bounded and fallible; depth is capped.
  Observation work remains cumulative through validation, bounding, partitioning,
  failed construction and fallback. Node visits, sorting and heap comparisons
  are explicitly outside that observation-work counter. Per-node/per-observation
  cancellation and the final poll provide cooperative complete-or-error behavior.
  Zero-k avoids population traversal. Invalid legacy records cannot produce a
  successful complete nonzero query.
- Outcomes accurately identify actual path, exhaustive coverage, complete
  selection, defined-float accuracy, local exclusive borrow, empty generations
  and absent continuation. No durable read view, tenant service, physical frame,
  hardware, empirical speedup or production authority is claimed.
- Source qualification separates immutable inventory, reported historical
  claims and execution. Missing original massive-plan/private historical receipt
  bytes, conflicting C0–C7 prose and consumer revision differences are disclosed
  without silently rewriting authorities.

## Evidence and remaining limits

Inspected final-qualified receipts and independently recomputed every listed
source and command-artifact hash: all matched at review time. The retained
qualified default gate reports **683 passed, 0 failed, 0 ignored**, with 29
summaries including five zero-case suites. Python reports **16 passed**; actual
manifest admission and locked online metadata succeeded. The receipt records
unchanged inputs and removal of live-store/Zig opt-ins. Earlier failed format
and offline-metadata attempts remain historical failures, not final evidence.
No tests were rerun by this reviewer.

The existing 11 T06 tests include 1000 bounded Euclidean seeds, 1000 post-insert
checks, 50 Manhattan seeds and nontrivial cosine cases. Those support the
declared corpus, not universal input proof. Missing historical red stub/binary
identity is disclosed; the five executed red assertion failures remain useful
but do not reconstruct that absent artifact.

Before acceptance, close the P2 finding with relevant regression evidence and
refresh changed-source final gate/fingerprints. The root-owned final acceptance
ledger must point at that eventual state. Optional FHE, consumers, hardware,
full durable spatial shapes/frames, controlled in-flight cancellation races,
allocator-fault injection and production operation remain unqualified; their
absence is not silently promoted into a passing result or a later-profile claim.

Only this report was written. No implementation edits, commits, pushes, CI,
live-store operations or subagents were used.

## Scoped re-review after P2 correction

Updated verdict: **ACCEPT for packet specification and implementation quality;
the P2 finding is closed.** The original finding and pre-fix evidence above are
retained as history. Root still owns the refreshed whole-workspace gate and
final changed-source fingerprint after this correction.

Compared the current adapter and parity test against their full original
`final-diff.patch` contents. The production delta adds only a zero-initialized
`radius_output_bytes` accumulator and checked cumulative slot-plus-payload
admission at `octree_checked.rs:294–309`. Over-budget or overflow exits occur
before reservation/push; the accumulator advances only after a successful push.
This applies identically to indexed and fallback radius selections. Top-k
candidate replacement and its final winner-byte accounting remain unchanged.

The new private unit test at `octree_checked.rs:522–546` tests the previously
missing observable boundary: an over-budget first radius hit returns
`BudgetExceeded` with an empty vector and zero capacity. The retained
`p2-radius-red.txt` shows the old implementation returning `Ok(())` and failing
the intended assertion; `p2-radius-unit-green.txt` reports one passed test.
The integration test at `wdbx_3d_octree_parity.rs:427–478` admits two hits at
their exact combined logical charge, rejects one byte less, and explicitly
asserts actual `Octree` and `CompleteScan(IndexBudget)` paths.
`p2-radius-integration-green.txt` records 12 parity, 10 checked-scan and three
compatibility tests passing. Independently recomputed both corrected source
hashes and the parity test binary hash in `p2-radius-sha256.txt`; all match.
No tests were rerun during this re-review.

No new finding remains. All original scope/evidence limitations still apply;
the pre-fix 683-test gate must not substitute for the root's refreshed final
gate.
