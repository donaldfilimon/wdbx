# Task 8 canonical-budget correction report

2026-10-23 UTC. Finite local correction only. All 163 requirements remain open/C0/unknown. No portable adoption or physical memory qualification is claimed.

## Result and ownership

Replaced every target-layout charge used by the new scoring and orchestration budget mechanisms with one private, version-labelled concrete-type table. Changed only `specimen-studio/native/specimen-kernel/src/scoring_v3/budget.rs`, `src/orchestration_v3/budget.rs`, new `src/logical_charge_v1.rs`, one private module registration in `src/lib.rs`, and new `tests/scoring_orchestration_budget_v1.rs`. Evidence is newly added under `docs/completion/execution/evidence/2026-10-23-scoring/canonical-budget/`.

No producer, scanner, identity, legacy engine/save, probe, manifest, lock, substrate or frozen-contract source was changed. Existing dirty work was preserved; no staging, commit, installation, network, holdout or live store access occurred. The three edited pre-existing files' snapshots match the root's pre-edit `baseline-canonical-budget` snapshots. `owned-review.diff` records the bounded change, including the two newly added Rust files.

The root approved the scheme in principle, then accepted the measured-reference check: constants must be at least the inspected supported 64-bit reference; production admission must never derive them from the compiling layout. The table below is the final logical-charge-v1 table. The concrete trait bound makes any newly traversed unregistered type a compile error. There is no blanket implementation, public constructor, wire/schema/save/domain change, layout trick or external dependency.

## Reviewed fixed table

| Concrete type | Native arm64 reference bytes | Logical v1 unit bytes |
| --- | ---: | ---: |
| u8 | 1 | 1 |
| f64 | 8 | 8 |
| usize | 8 | 8 |
| u128 | 16 | 16 |
| Span | 16 | 16 |
| String | 24 | 24 |
| Vec<Expr> | 24 | 24 |
| Alternative | 40 | 40 |
| Expr | 48 | 48 |
| InhibitionFact | 32 | 32 |
| DrawRecord | 48 | 48 |
| Definition | 64 | 64 |
| Charge | 56 | 56 |
| Capture | 80 | 80 |
| RouteCapture | 96 | 96 |
| Token | 144 | 160 |
| EntryRequest<'_> | 256 | 384 |
| Contribution | 1952 | 2048 |
| &Contribution | 8 | 8 |

`CONTRIBUTION_BYTES_V1=2048` is shared by the trait's collection charge and the direct scalar completion charge. Units are fixed literals. The reference-only `reference-layout.rs` executable was built with pinned rustc against the existing current kernel rlib (exact path and command in `reference-layout-build.json`); its output is `reference-layout.log`. That evidence uses `size_of` only to record a native reference. No production or new regression-test expectation uses it. This worker did not run a new WASM layout measurement; actual native/WASM decision checks remain the root's following witness step.

These are logical accounted bytes. Nested payload/string/collection charges continue to reserve their separately traversed values, as before; units do not measure allocator overhead, capacities or process peak memory. Conservative fixed reserves intentionally permit earlier refusal on smaller-layout targets; Token, EntryRequest and Contribution also round up from the inspected native reference. Maintaining the explicit table when types evolve is a continuing cost.

## Preserved semantics

The only decision change is the metadata multiplier/scalar unit. All global caps remain unchanged. Scoring still charges collection work by item count and payload by the fixed metadata unit, while orchestration still charges metadata units to both operation work and logical bytes. UTF-8 byte lengths, leaf8000, tree depth16, tree4096, collection100000, checked multiplication/subtraction/addition and existing refusals remain in place. Every unit is positive, preserving the prior orchestration `.max(1)` intent without a layout expression.

The broker's `R = 4*C + 4*J + E` cleanup reservation, retain/release symmetry, completion/attachment atomic headroom checks, RNG transactionality, visits and lease/active/quarantine fencing protocols were not edited. Direct tests verify failed completion preserves counts and an active lease, which can subsequently complete a no-vote result. At exact cleanup capacity an additional cycle refuses without mutation; restart and retirement release retained accounting to the original broker274 bytes.

## TDD and independent public boundaries

Before implementation the new test target ran: three controls passed and five tests failed for expected over-admission under the old native charges. `red.log` and `red.command.json` retain that exit101 run. A preceding compile-only attempt exposed `Expr` lacking `PartialEq`; its assertion was corrected to pattern-match the actual action, and `red-compile.log` is retained. No implementation was changed before the successful behavioral red.

All eight new tests now pass. Their expected costs are hand-itemized literals and field lengths, independent of the private table and native layout. Source=target `hello`, preserved input ID `input`, and selected action `é` exercise the two-byte UTF-8 payload. Public boundaries are:

| Decision | Refuse | Admit exactly |
| --- | ---: | ---: |
| Single scoring payload | 2970 | 2971 |
| Single scoring work (item-count policy preserved) | 2157 | 2158 |
| One-request scoring batch payload | 3354 | 3355 |
| One-contribution arbitration payload | 2791 | 2792 |
| One-job descriptor admission work | 465 | 466 |
| Broker owned completion budget | 2985 | 2986 |
| Broker cleanup/work budget | 2303 | 2304 |

Completion retains2296 logical bytes; broker274+cycle256+job160 gives690 before completion and2986 after. Cleanup is4+4+2296=2304. The suite also preserves explicit checked arithmetic overflow and refuses `usize::MAX` caller payload without advancing the draw cursor. The exact-work control proves metadata unit changes do not silently turn scoring work into byte work. Existing tests cover nested captures, expressions, strings, large collections and attachment RNG/visit retry semantics; none was changed or relaxed.

## Commands and results

All Cargo commands ran from `/Users/donald/sources/repos/wdbx/specimen-studio` with `RUSTUP_TOOLCHAIN=1.99.1`, `CARGO_NET_OFFLINE=true`, pinned toolchain `bin` then `/usr/bin:/bin` ahead of the inherited PATH, and explicit `--offline --locked`. Each named `.command.json` contains exact argv, cwd, PATH, relevant environment, exit and UTC timestamp; corresponding `.log` files contain combined raw stdout/stderr.

- `green`: focused new test target, exit0, 8 passed.
- `affected`: `cargo test --offline --locked -p specimen-kernel --test scoring_orchestration_budget_v1 --test scoring_v3_contributions --test scoring_v3_election --test scoring_v3_policy --test orchestration_v3 --test orchestration_v3_attachments --test orchestration_v3_fix1 --test orchestration_v3_graph`, exit0: 80 tests, comprising8 new+38 existing scoring+34 existing orchestration. The original100000pending/1501drain cases remain in this run.
- `full-kernel`: `cargo test --offline --locked -p specimen-kernel`, exit0: 237 tests, 0 failures, 0 ignored; doc tests0.
- `clippy`: `cargo clippy --offline --locked -p specimen-kernel --all-targets -- -D warnings`, exit0.
- `format` and `format-check`: pinned `rustfmt --edition 2024 [--check] --config skip_children=true` on the five owned Rust files, exit0. No recursive formatting of unrelated files.
- `owned-verification`: direct trailing-whitespace/EOF/conflict checks over all five owned files, absence of layout policy expressions, equality with root pre-edit snapshots, and source/API/test freeze capture, exit0.

The first behavioral-red build completed with a local `rust-objcopy` dyld warning: `@rpath/libLLVM.dylib` was not found. The original warning remains in `red.log`. Subsequent commands explicitly set `DYLD_LIBRARY_PATH=/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/lib`; the library already exists there. `rust-objcopy --version` then actually loaded and exited0, reporting LLVM23.1.1-rust-1.99.1-stable (`objcopy-load.command.json`, `objcopy-load.log`). Focused rebuild, affected tests, full kernel and Clippy produced no warnings or loader error. This is an explicit local environment repair, no installation/config change and no artifact qualification.

One evidence runner initially calculated cwd one parent too high and failed before executing rustfmt; corrected and recorded in `runner-initial-error.txt`. The later formatter and formatter check both exited0.

## Stable handoff and remaining evidence

`source-before.json` and `before/` preserve original owned sources. `source-freeze.json` pins24 current implementation/API/affected-test files; `owned-review.diff` captures this worker's exact Rust delta. The Rust source and tests are frozen at this report; no further changes except an explicit review fix. This report has an identical durable copy in this evidence directory.

Root still owns independent review, profile/ruling updates, subsequent fixed same-limit native/WASM probe cases and actual parity rerun, and the four fresh Studio gates/current evidence. This worker's full kernel test and Clippy results are not the four Studio gates and do not certify real host concurrency, cancellation, physical memory, efficacy or operational adoption. The earlier raw parity diagnostics and failed evidence remain untouched. All163 remain open/C0/unknown.
