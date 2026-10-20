# T03 — Finite geometry and legacy widening

Implemented locally; ready for packet review. CI remains paused. No commit, push, live store, frame binding, shape predicate, or persistence change.

## Sources and scope

Read repository `AGENTS.md`, the T03 brief, supplied v6 sections 12–14 and v5 section 5.2. The explicitly authorized ADR02 scoped choice preserves legacy f32 and introduces only the checked f64 view with exact `1e150` magnitude boundary. This packet does not claim durable spatial-profile completion or exact predicate arithmetic.

## API

The additive public module `abi_wdbx::spatial3d` exports:

- `Point3::try_new(x: f64, y: f64, z: f64) -> Result<Point3, SpatialError>` and `Point3::from_legacy(Point3D) -> Result<Point3, SpatialError>`.
- Private `[f64; 3]` storage; read-only by-value `x()`, `y()`, `z()` and `coordinates() -> [f64; 3]` accessors.
- `Radius::try_new(f64) -> Result<Radius, SpatialError>` and `get() -> f64`, with private storage.
- `SpatialError::{NonFinite, CoordinateOutOfRange, NegativeRadius, InvalidBounds, BudgetExceeded, UnsupportedMetric}`. The latter three reserve the brief's vocabulary; no later behavior is implemented.

Point validation first rejects any nonfinite component, then magnitudes above `1e150`. Radius validation rejects nonfinite, then negative, then values above `1e150`. Constructors normalize signed zero only in new geometry. Finite f32 widening uses `f64::from`; original `Point3D` fields/methods and old storage are untouched. No unchecked/default/serialization constructor bypasses validation. Types derive only Clone, Copy, Debug and PartialEq (errors additionally Eq).

## Behavioral evidence

All commands ran from `/Users/donald/sources/repos/wdbx-spatial-checked-v1` using `rustup run 1.99.1 cargo` (equivalent toolchain selection to `cargo +1.99.1`):

| Command suffix | Result | Raw output |
| --- | --- | --- |
| `test --locked -p abi-wdbx --test wdbx_3d_numeric -- --list` | 4 named cases, nonzero | `evidence/t03-list.txt` |
| `test --locked -p abi-wdbx --test wdbx_3d_numeric` before checks | expected behavioral red: 0 passed, 4 failed; exit 101 | `evidence/t03-red.txt` |
| `test --locked -p abi-wdbx --test wdbx_3d_numeric` after checks | 4 passed, 0 failed | `evidence/t03-green.txt` |
| `test --locked -p abi-wdbx --test wdbx_3d_compatibility` | 3 passed, 0 failed | `evidence/t03-compatibility.txt` |
| `fmt --all --check` | exit 0 | `evidence/t03-format.txt` |

Named cases: `rejects_nonfinite_and_out_of_domain`, `widens_all_finite_f32`, `normalizes_only_new_geometric_zero`, `radius_domain_is_closed`. Tests were written with compilable temporary unchecked constructors. Assertions failed on accepted NaN, accepted negative radius, and negative-zero bits; this was not a missing-type/build failure. The temporary constructors were replaced with validation. Coordinate checks cover every axis, both domain signs, immediate representable value above the bound, finite extremes and subnormal preservation. Legacy widening exercises finite f32 endpoints, normal/subnormal boundaries and rejects nonfinite legacy input. Signed zero tests pin old raw f32 bytes; the compatibility target additionally pins signed v2/v3 bytes and nonmutating legacy reads using synthetic temporary fixtures.

Green runs were wrapped in `/usr/bin/time -l`: numeric 3.08 s real / 579616768 bytes maximum RSS; compatibility 0.54 s real / 163102720 bytes maximum RSS (compiler/test process totals, not a runtime benchmark). Raw resource output is retained. Red/list resource usage was not separately measured. `evidence/t03-identity.json` records HEAD, rustc identity and final source hashes.

## Limits and review

The parent T01 workspace gate precedes this packet. A new full workspace gate, optional FHE features and CI were not run by this worker; the result above is focused numeric/compatibility evidence only. No arithmetic kernels, shapes, frames, durable geometry or feature-specific runtime claims are introduced. Cancelled/AllocationFailure error variants are deferred to T05 if needed. New Rust files are below 1000 lines; no unsafe code. Rollback removes the additive module/export and numeric test before consumers use them.
