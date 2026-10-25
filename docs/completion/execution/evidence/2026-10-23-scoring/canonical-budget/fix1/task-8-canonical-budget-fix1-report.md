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
