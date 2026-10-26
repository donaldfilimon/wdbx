# Task 7b: local Cartesian frame conversion

The bounded implementation adds `spatial3d/frames.rs` and
`tests/wdbx_3d_frames.rs`, exported through the existing spatial entrypoint.
The [plan](../plans/2026-10-26-task7-frame-conversion.md) defines the accepted
local scope and arithmetic. This is a numerical primitive, not completion of
T09's registry packet or Task 7's durable profile.

`CartesianFrameRef` compares all 32 caller-supplied definition-digest bytes.
`CartesianPoint3` keeps a checked point and optional explicit frame reference;
legacy widening preserves its unitless, unbound status. `FrameConversion`
validates all 24 orientation-preserving signed axis permutations, an explicit
positive finite unit scale and checked target-unit translation. Conversion
refuses unbound/mismatched source references and revalidates output coordinates.
Fields are private, no serde defaults or persisted error variants were added,
and the existing numeric/storage APIs remain unchanged.

The rotation subset uses exact integer validation, with zero tolerance.
Conversion uses separate IEEE f64 multiply/add operations; rounding and
underflow remain f64 behavior. This is no exact-pose, uncertainty-propagation,
general-rotation, frame-authentication or inverse-round-trip guarantee.

## Verification

Base revision: `612d3570f9ab0d842519aa4dbc4cb977aa33c9e0`, clean canonical
`main`. No competing active WDBX writer was observed. Changes remain local and
uncommitted; no branches, worktrees, dependencies or live stores were changed.
Raw logs are retained under ignored `.work/task7-frames/`.

- Pinned compiler: Rust 1.99.1; Cargo 1.99.1. The rustup toolchain bin and
  `/usr/bin` precede compiler shims. Build output is isolated in that packet's
  `target/`, and Rust test stdin is at EOF.
- `cargo test --locked -p abi-wdbx --test wdbx_3d_frames`: seven discovered
  cases. With a temporary `convert` stub returning `UnsupportedRotation`, six
  behavior tests failed and one constructor-refusal test passed (`red.log`,
  exit 101). With conversion implemented, seven pass (`green.log`, exit 0).
  The independent integer corpus checks all 24 proper rotations and refuses
  all 24 reflected permutations; expected values do not call production math.
- `python3 tools/check_substrate_release.py --snapshot-only`: exit 0.
- `python3 -m unittest discover -s tools/tests -p
  test_check_substrate_release.py`: 52 pass, exit 0.
- `bash tools/check.sh`: exit 0, `check: all green`. Instruction, source-size,
  format and Clippy checks pass under the repository's existing warning policy.
  Rust totals across 62 test summaries (including doctests): 1,208 passed,
  0 failed, 0 ignored. Python witness suites: 52 complete-episode decode,
  28 COSE encoding and 14 native-model tests passed. All seven spatial targets
  ran, including unchanged independent dyadic/scan and compatibility oracles.
  Optional FHE features, Studio, other native platforms and hardware were not
  rerun for this local frame primitive (`gate.log`, `toolchain.log`).
- `git diff --check`: exit 0. Relative to the clean base, existing tracked
  changes are only the additive exports in `spatial3d/mod.rs` and the execution
  register's continuation link. New files are the frame module, its test,
  this packet and its plan. Independent witnesses, legacy source, golden
  fixtures, frozen contracts, source pins and manifests are unchanged.
  `source-hashes.json` records 39 implementation/test/contract identities;
  `verification-summary.json` records the executed check counts/statuses.
- Live `--drift-report`: expected exit 1 both before and after; JSON and stderr
  diagnostics compare byte-identical. The existing 39 declaration failures
  remain visible, with no pin refresh or inventory migration.
- HawkScan prerequisite check: `hawk` is not installed (command exit 127);
  no DAST scan ran and no security qualification is inferred.

## Exactly what remains open

1. **Frames:** durable registry schema, units, origin convention, handedness,
   axis interpretation, transform version/validity/scope/provenance, canonical
   digest construction and authentication, parent lookup/cycle rejection,
   approved legacy source-set bindings, query-level mixed-frame enforcement,
   arbitrary reviewed rigid rotations/tolerances, uncertainty policy, shape
   conversion, geodetic profile and registry-driven index invalidation. The
   current reference is a caller assertion, not evidence of these properties.
2. **Durable attachments:** full observation/record/version/attachment identity,
   schema/profile/frame/validity/creator/scope/provenance commitments, canonical
   codec/admission transaction, reader gating, restart/corruption/recovery and
   signed-source-byte preservation through actual attachment operations.
3. **Causal read views:** all-maximal-version preservation and conflict modes,
   immutable transaction-digest frontiers, relevant v3 roots, observation maps,
   policy/consent binding, authorized scopes and view/lease lifetime enforcement.
4. **Publication:** verified generation manifests and mappings, complete deltas
   (including removals, tombstones, conflicts, frames and visibility changes),
   sealed/verified/published/active/retired states, file/directory durability,
   competing-builder fencing and crash/cancel recovery without partial activation.
5. **Acceptance:** Task 2 owner/schema/authority receipts and shared vectors for
   dependent interfaces; independent review/acceptance of this subset; all
   remaining release tasks, native/device/multi-host/security and operational
   qualification. No C0 status or evidence array is promoted by these tests.
6. **Release register:** live validation retains the pre-existing 39 declaration
   failures on `lib.rs` (38) and `v2.rs` (1). Reviewed version/declaration-inventory
   migration remains required; blanket hash replacement is not acceptance.

Rollback removes only this additive module, exports, tests and packet docs;
there is no persisted geometry or live-state migration to undo.
