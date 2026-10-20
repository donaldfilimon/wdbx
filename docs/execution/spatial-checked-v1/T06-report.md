# T06 local qualification report

State: implemented; independent acceptance and root full gate pending. No commit,
push, CI, live store, or production data access. Source root is the authorized
`/Users/donald/sources/repos/wdbx-spatial-checked-v1` checkout. Rust is 1.99.1
(58cf8f9c35b355199727627d07ae34a4e2846d67), aarch64-apple-darwin, LLVM 23.1.1.

## Implementation and scope

`spatial3d/octree_checked.rs` adds a transient checked query adapter using the
original octant partition, outward f32 bounds, midpoint, child-bound primitives,
and 64/8/16 threshold/leaf/depth constants. Existing legacy query and builder
bodies are unchanged. Visibility of shared primitives is widened internally only.
`legacy_checked::metric_key` is a root-approved additive internal getter; no T05
behavior changes. The tree is rebuilt for each exclusive local borrow, so an
insertion cannot leave stale checked nodes. All-coincident populations stop at
one leaf; merely occupying one octant does not stop subdivision.

Euclidean squared keys use the shared non-FMA f64 x/y/z reduction; Manhattan
uses the corresponding left-to-right absolute sum. The nearest endpoint of a
containing box gives component differences no larger in magnitude than any
represented descendant. Correctly rounded subtraction, nonnegative square and
addition are monotone, using the same reduction order. Strict greater-than
pruning keeps closed boundaries and tie contenders. The retained authoritative
metric key is used for top-k thresholds, never rounded sqrt. Cosine uses a
complete f64 non-FMA scan, zero norms yield one; it has no qualified tree bound.
This is `defined_float_profile`, not an exact-real predicate claim.

Outcomes separate coverage, completion, numeric accuracy, actual execution path,
ranking, local exclusive-borrow consistency, no partial reason/continuation,
empty observed generations, and explanation. Coordinates remain legacy unitless;
there is no durable read-view ID, canonical index, gateway, storage, or frame
registry claim.

## Resources and cancellation

`QueryLimits.max_work` counts cumulative observation inspections across initial
validation, root bounding, both partition passes and leaf execution. A failed
build never resets work; fallback can fail even when a standalone scan would fit.
Node visits, heap selection, sorting and CPU cycles are not observation work.
A separate `OctreeBuildLimits` caps nodes and cumulative logical requested tree
and index-partition heap bytes before fallible reservations. Recursion is capped
at 16 and fixed per-frame stack is bounded; allocator metadata is excluded.
On scoped build failure the remaining complete scan reports `IndexBudget`.
Successful tree bounds/partition work consumes its budget as well. Invalid legacy
records fail before construction. Every node/record polls cancellation, with a
final poll after ordering; no partial hits escape on errors.

Result slots are admitted against output bytes/results before allocation. Radius payload bytes and hit slots are admitted cumulatively before each
reservation/push. kNN final winner payload bytes are charged after selection. Payloads are borrowed. These
are scoped logical byte ceilings, not total-process RSS measurements or an index
memory interpretation of output bytes. No speedup is claimed.

## Red and green evidence

Raw logs and SHA-256 identities are in `evidence/T06/`. Initial direct
`cargo +1.99.1` attempts hit the local Homebrew Cargo shim (`no such command`);
that environment error is not red behavior evidence. Equivalent pinned commands
used `rustup run 1.99.1 cargo` thereafter.

- Discovery: `rustup run 1.99.1 cargo test --locked -p abi-wdbx --test wdbx_3d_octree_parity -- --list` listed all five named brief cases.
- Behavioral red: the same target with the initial adapter stub returning
  `Err(UnsupportedMetric)` produced five executed failures, zero successes.
  The raw red log is preserved. The initial stub binary/hash was not retained;
  final source/build SHA-256 identities are preserved, without claiming a red
  binary identity.
- Final scoped command: `rustup run 1.99.1 cargo test --locked -p abi-wdbx --test wdbx_3d_octree_parity --test wdbx_3d_checked_points --test wdbx_3d_compatibility`:
  **11 T06 + 10 T05 + 3 compatibility tests passed**, zero ignored/failed.
- `rustup run 1.99.1 cargo clippy --locked -p abi-wdbx --test wdbx_3d_octree_parity` passed with permitted pedantic warnings; raw log retained.
- `git diff --check` passed. Both T06 Rust files remain below 1000 lines.

The bounded generator is LCG/rotate v1, seeds 0..999, fixed 63/64/65 populations.
It exercises coincident, collinear, planar, clustered, skewed, extreme finite
f32, subnormal/signed-zero, and tie/duplicate-ID/payload cases. Each seed executes
radius and kNN against the separately implemented `support/spatial_oracle.rs`,
then inserts a new point and independently checks the rebuilt selection. More
than 1000 initial executions assert actual Octree use. Every case checks output
identity/ordinal/display distance/payload and outcome semantics; successful
resource usage stays inside explicit caps. Fifty additional deterministic 3D
Manhattan seeds compare radius/kNN with an independent complete reference,
including a closed-boundary assertion. Nontrivial cosine known answers cover
axis, (4,3,0), (3,4,0), zero and negative axis with distinct payloads and >64 rows.

Unexecuted: full workspace/default gate (root owns it), optional FHE features,
production/integrated systems, empirical speedup/RSS profiling, and a separately
controlled cancellation race during traversal. Pre-cancellation is tested;
per-node/per-record cooperative polling is directly inspectable. No all-f64
proof or exact dyadic agreement is asserted for the defined-float profile.

Rollback: stop calling the checked tree adapter; retained T05 complete methods
and all prior legacy methods remain available.

## P2 radius payload admission review fix

The review found that radius slot admission preceded reservations but payload
bytes were only checked at final materialization. Radius selection now maintains
a checked cumulative slot-plus-payload count and rejects before reserving or
retaining the first over-budget hit. kNN winner charging remains after selection.

TDD red: `rustup run 1.99.1 cargo test --locked -p abi-wdbx --lib radius_payload_budget_rejects_before_reservation` executed one test and failed as expected:
`consider` returned `Ok(())` for an over-budget payload. Green unit evidence
asserts `BudgetExceeded` with zero vector capacity and no retained hit. An added
integration test checks exact and one-byte-under combined payload budgets on
both actual Octree and explicit IndexBudget complete-scan paths. Raw red/unit
green/integration green logs are `evidence/T06/p2-radius-*.txt`. Only the two T06
Rust files and this report/evidence were modified for the fix; CPU formatting
was not touched. Final scoped results are 1 focused unit + 12 T06 + 10 T05 +
3 compatibility tests passed. Full gate remains root-owned.
