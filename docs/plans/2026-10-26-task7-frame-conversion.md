# Task 7b: bounded Cartesian frame conversion

Base: `612d357`, clean canonical `main`. This chat owns the additive frame
module, its integration tests and this packet's documentation. No other active
WDBX writer was observed. Build output uses `.work/task7-frames/target`.

## Contract and boundary

Implements a local numerical subset of release Task 7 and v6 section 12.3:
frame-bound points, explicit source/target references, proper signed permutation
rotations, positive unit scales, target-unit translations and output validation.
Frame references are caller-supplied immutable definition digests, not evidence
of registration, authenticity or authority. Legacy widening stays unbound and
cannot enter conversion. No serialization, durable interface or wire freeze.

The supported rotation subset has exactly one `+1` or `-1` in each row/column
and determinant `+1`; validation is integer-exact with zero tolerance. Other
rotations, reflections, shears and singular matrices refuse. Conversion is
`target[i] = (sign[i] * source[axis[i]]) * scale + translation[i]` with separate
IEEE f64 multiplication and addition. It does not promise exact physical pose,
uncertainty propagation, inverse round trips or arbitrary rigid rotations.

Tests cover all 24 proper signed permutations against independent integer
examples, invalid matrices/scales, source/frame-version mismatch, explicit
legacy-unbound refusal, signed zero and transformed numeric-domain rejection.
Run the focused target, existing spatial/compatibility oracles and full
`bash tools/check.sh` with pinned Rust 1.99.1 first on PATH and stdin at EOF.
Preserve frozen contracts, pins, independent witnesses and goldens; run snapshot
release validation and record live drift separately. No commits or publication.

## Open work

Full frame registry (units/origin/axes/handedness/version/validity/scope/provenance),
registry digest construction/authentication, parent-cycle checking, approved
legacy bindings, arbitrary reviewed rotations and uncertainty policy, shape
conversion, geodetic conversion and index invalidation remain open. Durable
attachments, observation identities, all-maximal causal read views, complete
deltas and fenced/crash-safe publication remain open. Task 2 shared interfaces
and owner approvals, independent acceptance and all release qualification gates
remain required. This packet closes neither Task 7 nor source-pin migration.

## Execution

Implemented and locally verified; see the
[execution packet](../execution/2026-10-26-task7-frame-conversion.md) for red/green
results, full gate counts, unchanged-oracle checks and open qualification gates.
Changes remain uncommitted on local `main` for review.
