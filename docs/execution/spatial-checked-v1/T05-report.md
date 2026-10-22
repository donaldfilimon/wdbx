# T05 — Checked access to the legacy point index

Implemented locally; ready for review. CI paused. No commit, push, canonical bytes, durable identities, frame bindings, production runtime, or live store use.

Read AGENTS.md, T05 brief, supplied v6 §§12–14/22 and v5 §§5.2/5.5. Root accepted T03/T04 before implementation. Applied test-driven-development skill: eight compilable incorrect-stub tests failed on behavior before implementation. Two supplemental qualifications expand the final target to ten tests.

## API and scope

`SpatialIndex3D::try_insert(id: u32, point: Point3D, payload: &[u8]) -> Result<(), SpatialError>` validates first, copies payload with fallible reservation, reserves the record slot fallibly, and appends only after both succeed. Validation/allocation errors leave the logical records, count, and old query state unchanged. Legacy insert/query implementations are unchanged.

`try_radius_search_checked(&self, center: Point3D, radius: f32, limits: QueryLimits)` and `try_nearest_neighbors_checked(&self, center: Point3D, k: usize, limits: QueryLimits)` return `Result<Vec<CheckedSpatialHit<'_>>, SpatialError>`. Corresponding `_with_cancel` methods append `cancel: &AtomicBool`. `SpatialError::Cancelled` was a root-approved narrow addition to numeric.rs. Each hit exposes id, insertion_ordinal, original point, rounded distance_f64, borrowed payload; a private comparison-key field supports allocation-free ordering. IDs and ordinals identify observations only within this index.

`QueryLimits { max_work, max_results, max_output_bytes }` uses usize ceilings. Work counts population observation examinations: successful nonzero-k and radius paths examine every observation once for validation/admission and once for execution, requiring checked `2*n` before allocation. This is not CPU cycles or a deadline. Heap selection and sorting also consume finite bounded work, bounded by population/result ceilings: top-k O(n log min(k,n)), final sorting O(m log m). k=0 validates the query and cancellation, then returns empty without population traversal or allocation, including when stored legacy points are invalid. k>population selects all available observations when result/byte caps permit.

Result capacity and arithmetic are checked before output/scratch allocation. Radius first counts all matching observations and charges bytes; any result/byte cap fails instead of truncating. Top-k reserves only min(k,n) hit slots after slot-byte admission; an in-place max heap selects winners, then charges final winner payload lengths before returning. Nonwinning large payloads cannot cause a false output-byte rejection. Both paths use try_reserve_exact, allocation failure maps to BudgetExceeded. Output-byte charge is `sum(size_of::<CheckedSpatialHit>() + payload.len())`, including borrowed payload logical bytes, excluding allocator metadata/rounding and pre-existing stored allocations. The heap buffer becomes the output, with no second n-hit allocation. Radius sorting is in-place and top-k does not allocate a full population when k is small.

Both paths poll cancellation at entry, per population examination, and before returning. Errors expose no partial Vec. Cancellation is cooperative, not a hard wall-clock guarantee; sort/heap operations are not interruptible within a single operation.

Numeric semantics are **defined_float_profile**, exhaustive complete-or-error Euclidean scans. Each finite f32 coordinate widens before subtraction. The authoritative squared f64 key is `(dx*dx + dy*dy) + dz*dz`, ordinary multiply/add, no FMA, IEEE subnormals preserved; membership compares to widened radius squared with a closed `<=` boundary. Ordering is squared key, ID, ordinal; sqrt is display only. Signed zero geometry is normalized only in the validated view; original points/payload bytes remain unchanged. Finite f32 extrema have finite widened differences/squares. No exact-real predicate claim is made: rounded squared keys can disagree with the stronger dyadic profile. T07 owns that stronger profile.

Internal shared helpers exposed under spatial3d are `squared`, `hit`, `hit_with_distance`, `order`, `sift_down`, and pub(crate) SpatialIndex3D::checked_records. The separate key/display constructor lets T06 reuse hit storage/order without forcing sqrt for another metric; T05 public methods remain Euclidean only.

## Evidence

All cargo commands selected rustup toolchain 1.99.1 from `/Users/donald/sources/repos/wdbx-spatial-checked-v1`. Source hashes, HEAD and rustc identity: `evidence/t05-identity.json`. Commands and raw outputs:

| Command | Result | Raw output |
| --- | --- | --- |
| `rustup run 1.99.1 cargo test --locked -p abi-wdbx --test wdbx_3d_checked_points -- --list` | Initial 8 / final 10 named tests | `t05-list.txt`, `t05-list-final.txt` |
| Same target without `-- --list`, incorrect compilable stubs | 0 passed, 8 failed, cargo exit 101; expected assertion failures | `t05-red.txt` |
| Same target after initial implementation | 8 passed | `t05-green.txt` |
| `rustup run 1.99.1 cargo test --locked -p abi-wdbx --test wdbx_3d_checked_points --test wdbx_3d_compatibility --test wdbx_3d_numeric --test wdbx_3d_oracle` | 9 checked, 3 compatibility, 4 numeric, 8 oracle passed | `t05-focused.txt` |
| `bash tools/check.sh` (wrapped `/usr/bin/time -l`) | All green, then all green again after test lint cleanup | `t05-gate.txt`, `t05-gate-final.txt` |
| Final target after helper visibility/constructor and rounded-display qualification | 10 passed, 0 failed | `t05-final-target.txt` |
| `rustup run 1.99.1 cargo fmt --all --check`; `git diff --check` | exit 0 | `t05-format-final.txt`, `t05-diff-check.txt` |

Evidence paths are under `docs/execution/spatial-checked-v1/evidence/`. The full default gate ran before final helper exposure and additional qualification test; final target/format ran after those changes. Root owns the final combined T06 workspace gate. Existing pedantic/missing-doc warnings from other files remain; no new production warnings. Latest full gate: 26.92 s real, 176111616 bytes maximum RSS. Final focused target: 1.55 s real, 463093760 bytes maximum RSS. These include compiler/test process totals, not runtime benchmark evidence.

The five exact brief names are retained: invalid_insert_is_atomic; zero_k_returns_no_hits; equal_distance_uses_checked_total_order; legacy_duplicates_are_retained; invalid_legacy_population_is_not_silently_skipped. Other cases cover exact work/result/byte boundaries, caps on radius versus top-k, invalid query/radius, empty indexes, pre-cancellation, k>population, signed zero, f32 extrema/subnormals, closed boundary adjacency, duplicate IDs, nine seeded 3D populations across 63/64/65 with three centers and seven k values against T04's independent complete scan. A crafted `d2=1` versus `d2=1+2^-52` pair has equal displayed sqrt but preserves squared ordering and radius exclusion.

No optional FHE tests, CI, allocation-fault injection, deterministic in-flight cancellation race, durable service outcome metadata, frame registry, or indexed pruning evidence is claimed. New Rust files remain below 1000 lines. Rollback switches consumers back to existing methods under their historical semantics; no stored bytes change.
