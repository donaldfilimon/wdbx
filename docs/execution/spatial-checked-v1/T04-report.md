# T04 — Independent complete and boundary oracles

Implemented test-only helpers following accepted T03. Sources read: AGENTS.md, T04 brief, supplied v6 sections 12–14 and v5 sections 5.2/8.4. M1 allowance is `defined_float_profile`, not `exact_real_predicate`.

`OraclePoint` retains original validated Point3, test identity and insertion ordinal. Complete radius/kNN enumeration uses no production traversal or distance arithmetic. Coordinates are already f64 (legacy conversion widens before subtraction). Arithmetic is ordinary f64 subtraction, multiplication and left-to-right `(x*x+y*y)+z*z`, no FMA/mul_add; subnormals retain IEEE behavior, including square underflow. Numeric constructors normalize signed zeros. Authoritative ordering uses squared distance before rounded sqrt, then identity and insertion ordinal. k=0 returns immediately; k beyond population returns all. Closed membership compares d2 <= r*r.

The separate BigInt comparator reconstructs finite float bit patterns as integers in units of 2^-1074; its squared keys share scale 2^-2148. Finite f64 requires at most 2098 integer bits before squaring, bounding arithmetic for the fixed corpus. Existing num-bigint is used; no dependency/lock edits. It calls neither scan nor production spatial arithmetic. This is bounded test reference qualification, not a production exact comparator or proof over all inputs.

## Evidence

Commands use `rustup run 1.99.1 cargo`, equivalent to `cargo +1.99.1`, from the task checkout. All cargo tests use `--locked -p abi-wdbx`.

| Command suffix | Result | Raw evidence |
| --- | --- | --- |
| `test --test wdbx_3d_oracle -- --list` before implementation | 5 named tests | `evidence/t04-list.txt` |
| `test --test wdbx_3d_oracle` against compilable empty-scan/constant-exact placeholders | expected behavioral red: 5 failed, exit 101 | `evidence/t04-red.txt` |
| `test --test wdbx_3d_oracle` after implementation | 8 passed | `evidence/t04-green.txt` |
| `test --test wdbx_3d_oracle -- --list` final | 8 named tests | `evidence/t04-green-list.txt` |
| `test --test wdbx_3d_compatibility --test wdbx_3d_numeric` | 3 compatibility + 4 numeric passed | `evidence/t04-compatibility.txt` |
| `fmt --all --check` | passed | `evidence/t04-format.txt` |

All four mandated tests failed behaviorally before implementation. Independence is demonstrated by invoking legacy euclidean_distance on opposite f32 extrema, observing overflow, while the independent radius scan returns the correct finite hit/distance. Known answers pin the six-record source fixture, closed neighboring boundary values, tie permutations and duplicate IDs, zero/empty/k edge cases, maximum profile coordinates and squared keys that differ despite equal rounded sqrt.

Generator v1 uses xorshift64 shifts 13/7/17 with seeds 1 through 1000 inclusive, 16 points per seed, integer coordinates [-32,32], radii [0,32], duplicate IDs 0–7. This exactly representable subdomain supports comparing membership and kNN order to independent dyadic arithmetic without conflating float/exact definitions. Failure assertions print seed and radius-case inputs for reproduction. Explicit disagreements cover `(1,2^-27,0)` at radius 1 and minimum positive subnormal at radius 0: float includes, dyadic excludes. No epsilon changes boundaries.

Green oracle resource use: 0.90 s real, 174194688 bytes maximum RSS; combined compatibility/numeric: 0.08 s, 62259200 bytes. `/usr/bin/time -l` raw output retained. These are command totals, not benchmarks. Source hashes, HEAD and rustc build identity: `evidence/t04-identity.json`.

No production files, live stores, CI, commits or pushes. Full workspace/profile gate remains the root integration worker's responsibility; this packet ran focused tests and format only. Shape predicates, invalid constructors, mutation/index thresholds and production query checks belong to later packets; no completion claim is made for those. Rollback removes these three test files and evidence. During initial file creation an omitted shell cwd briefly placed the three new files in the canonical checkout; only those exact new files were immediately moved into this task checkout before compilation, with no existing files changed.
