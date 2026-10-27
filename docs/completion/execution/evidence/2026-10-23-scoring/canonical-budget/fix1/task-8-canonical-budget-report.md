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


## Integration fix1 — remaining attachment charges, 2026-10-23 UTC

The root's actual post-correction WASM integration run found remaining target-layout charges outside the original owned budget modules: attachment preflight used the physical `AttachmentContext`+`Visit` headers, and retained attachment metadata added those headers, an Arc reference and `size_of_val` of the inherited capture slice. A redundant supplied-evidence Span bound in `scoring_v3/policy.rs` also still named the target layout. The original report's general wording about every target-layout charge is superseded by this wider, explicit new-module audit. The original native/WASM diagnostics, strict comparator and earlier failed run were preserved; no probe change or relaxed equality was made by this worker.

The root explicitly extended ownership to `orchestration_v3/attachments.rs`, private `logical_charge_v1.rs`, the redundant Span check in `scoring_v3/policy.rs`, one new public integration-test file and new fix1 evidence. Only those four Rust files changed in this round. All21 other files from the original24-file source/test freeze remain byte-identical. No public API, scalar scoring math, scanner/producer, legacy save/ID, probe, manifest, lock, substrate or frozen-contract edit occurred. No staging, commit, installation, network, holdout or live store access occurred. Original evidence under `canonical-budget/` remains immutable; all new raw evidence is in `canonical-budget/fix1/`.

### Final fixed attachment table

| Category | Observed actual native arm64 reference | Fixed v1 reserve |
| --- | ---: | ---: |
| AttachmentContext | 240 | 320 |
| Visit | 64 | 80 |
| Arc reference to AttachmentContext | 8 | 8 |
| RouteCapture (inherited attachment slice) | 96 | 96 |
| Span | 16 | 16 |

The root approved the fixed320/80/8/96/16 table after observing these actual reference fits. At the root's explicit direction, a temporary internal `#[cfg(test)]` observation measured the actual private types through the pinned compiler. `layout-reference.log` and `.command.json` record exit0 and output; `temporary-layout-observer.rs` preserves its exact text. The temporary function was removed and the attachment source restored byte-for-byte before the behavioral-red tests and correction. It is absent from final production and test builds.

Inherited attachment captures are `RouteCapture`, so their slices share its existing canonical96 unit. The producer's different `Capture` type remains80 and is not used to charge an inherited `RouteCapture` slice. Production now uses only named versioned constants for attachment headers/reference, checked count multiplication for the slice, and checked additions for each retained payload/name/parent/stream amount. Existing per-value text/capture validation precedes copying and retention. The Span bound now uses `Span::UNIT_BYTES` (16); its existing one-million-span cap already makes the byte branch redundant on both targets, so this substitution changes no accepted input under that cap.

The same fixed headers apply in attachment preflight and retention. Selected and declined visits still reserve the same metadata extras, as before. Global caps, work/retention policy, `R=4C+4J+E`, binding/guard/containment checks, affinity sampling, visits, leases and quarantine/fencing were not otherwise changed. Rounded logical reserves can cause earlier refusals than the prior native headers; they establish no allocator, capacity, physical-memory, host-resource or operational claim.

### Public TDD boundaries and atomic behavior

New target `scoring_orchestration_attachment_budget_v1` contains3 tests covering10 boundary scenarios. All3 failed before implementation with the expected erroneous success at the new minus-one limits (`red.log`, exit101), then passed unchanged (`green.log`, exit0). Expectations use only hand-itemized fixed charges, literal field lengths and actual public state; no expected limit is computed from target layout or a production budget helper.

The new fixture retains one inherited `RouteCapture`: name `cap` (3 bytes), value `é` (2 UTF-8 bytes). The root contribution therefore retains2397=original2296+RouteCapture96+payload5 logical bytes. Broker274+cycle256+job160+contribution2397 gives3087 owned bytes before attachment. The extra retained attachment metadata is521=Context320+Visit80+reference8+RouteCapture96+payload5+parent6+stream6. A selected target additionally reserves job160 owned bytes and4 cleanup units; a declined target reserves the metadata without a job.

| Public attachment decision | Refuse | Admit exactly |
| --- | ---: | ---: |
| Operation work, 60 declared dependency IDs | 2936 | 2937 |
| Selected retained owned bytes, second empty cycle retained | 4023 | 4024 |
| Declined retained owned bytes, second empty cycle retained | 3863 | 3864 |
| Selected cleanup reservation, C=2,J=2,E=2918 | 2933 | 2934 |
| Declined cleanup reservation, C=2,J=1,E=2918 | 2929 | 2930 |

The work fixture's 60 explicit dependency IDs are inert host metadata used to isolate preflight work; no executable dependency graph or completeness claim is made. Its2937 total is descriptor466+dependency960+headers400+capture101+parent6+stream6+binding38+dependency960. An unrelated active job remains within cleanup headroom, distinguishing this decision from cleanup refusal.

Tests assert no RNG advancement or counts/owned/identity/visit publication on refusal. The same issuer, target, remainder and job ID retry successfully after either reducing the descriptor metadata or retiring exactly the expendable headroom. Both selected and declined successful handoffs record the visit once; repeated handoff refuses without a second draw. The work test retains an unrelated active lease across refusal/admission, completes both target and unrelated lease once, and rejects a duplicate completion without mutation. Exact cleanup saturation refuses another cycle atomically. Original80 affected tests and all237 prior kernel tests were retained unchanged.

### Fresh fix1 checks and source freeze

All commands ran through `fix1/run.py` from `/Users/donald/sources/repos/wdbx/specimen-studio`, with pinned1.99.1 first in PATH, `/usr/bin:/bin` ahead of inherited entries, `RUSTUP_TOOLCHAIN=1.99.1`, `CARGO_NET_OFFLINE=true`, and `DYLD_LIBRARY_PATH=/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/lib`. Cargo commands explicitly used `--offline --locked`. Each `.command.json` records exact argv/cwd/environment/exit/time and each `.log` retains combined raw stdout/stderr. No new loader warnings occurred.

- `layout-reference`: actual temporary internal observation, exit0, then removed.
- `red`: new attachment target before correction, exit101, all3 expected behavioral failures.
- `green`: new attachment target after correction, exit0, all3 tests pass.
- `affected`: prior80 plus3 new attachment tests, exit0, 83 pass/0 fail/0 ignored.
- `full-kernel`: `cargo test --offline --locked -p specimen-kernel`, exit0, 240 pass/0 fail/0 ignored; doc tests0.
- `clippy`: `cargo clippy --offline --locked -p specimen-kernel --all-targets -- -D warnings`, exit0.
- `format` / `format-check`: pinned rustfmt on only the four owned files with `skip_children=true`, both exit0.
- `owned-verification`: direct whitespace/EOF/conflict check, exact root fix1 pre-edit snapshot agreement, original non-owned-source preservation and scoped layout-policy audit, exit0.

`layout-policy-audit.json` inventories14 production Rust files: all new `scoring_v3/`, all new `orchestration_v3/`, and the private logical-charge helper. It finds zero `size_of`, `size_of_val` (covered by the `size_of` pattern), `align_of`, target-pointer-width, target-architecture or `std/core::mem` policy leftovers. The removed temporary observer is also checked absent. No legacy module was scanned or refactored for this audit.

`fix1/source-before.json`, `before/`, and `owned-review.diff` preserve the exact three-file baseline and four-file delta. `fix1/source-freeze.json` pins25 final source/API/affected-test files, SHA256 `87d3c8804c475a0a69f2fa9d642fca1114ff2d28954cdc879e7ce908ffae6516`. Rust source/tests are frozen after this report except an explicit review fix. The original24-file manifest and original durable report remain preserved as historical evidence. This appendix has a new durable copy under `fix1/task-8-canonical-budget-fix1-report.md`; the complete updated workspace report also has a durable fix1 copy.

Root still owns the scoped independent rereview, unchanged-probe actual native/WASM integration rerun, final four Studio gates and evidence aggregation. Native tests and a source audit are not actual WASM parity or the four Studio gates. This is integration fix round1/5, a finite correction of already-present attachment accounting, with no broader feature addition. All163 requirements remain open/C0/unknown.
