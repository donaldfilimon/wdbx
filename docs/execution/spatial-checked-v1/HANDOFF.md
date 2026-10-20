# spatial-checked-v1 handoff

## Identity and boundary

Reviewable implementation is uncommitted in the detached isolated worktree
`/Users/donald/sources/repos/wdbx-spatial-checked-v1`, based on WDBX
`08a69b3f22db61093e06527b3308b10589687022`. No rollback to an earlier revision
was performed. Canonical checkout remains `/Users/donald/sources/repos/wdbx`;
ABI is its sibling `/Users/donald/sources/repos/abi` (inspected baseline
`ca4a725c07ff75f885eb60bd59fe9797e12e4d2c`). Active Studio is WDBX's embedded
`specimen-studio/`, separate workspace, with its existing pinned git dependency;
no dependency sources were changed.

The 61-row immutable inspection inventory is
`../../claims/wdbx-3d/source-manifest.json`. It pins commits, SHA-256 digests,
authority read coverage, reported versus executed evidence and explicit scoped
inspection. The original v5/v6/requirements files were not overwritten.
`../../superpowers/specs/2026-10-20-wdbx-3d-source-audit.md` records reconciliation,
ADRs and discrepancies. The inventory validator admits document structure; it
does not establish source authenticity or product behavior.

Canonical WDBX and ABI were initially clean. Later unrelated dirty state was
preserved: canonical WDBX's cpu.rs formatting change, and ABI hook/skill/task
files plus additions to `agent-wdbx-architecture.mdx`. Exact observed statuses,
HEADs and dirty-file hashes are in `evidence/external-dirty-boundary.json`.
The architecture diff was inspected and does not change this bounded packet's
contracts. The manifest remains the pinned inspection baseline, not a claim
those external workspaces are still clean. No external dirty files were reset,
cleaned, staged or overwritten. Worktree cleanup/merge is deferred because the
user authorized an isolated uncommitted result and forbade commits and pushes.

## Task and implementation status

T00–T06 attempted; task-level independent reviews accepted all seven. Final
whole-diff review and changed-source verification are recorded below. T07 and
later are not started. This completes only the first bounded packet, not the
Neural Specimen implementation or the entire requirements registry.

- T00: source audit, discrepancy register and evidence validator; 16 Python tests.
- T01: exact stable Rust 1.99.1 qualification and baseline default gate (647
  harness cases); formatting-only prerequisite in compute cpu.rs.
- T02: three compatibility cases and synthetic fixtures. Selected legacy readers,
  threshold ties, duplicate IDs and signed bytes are pinned. The v2 fixture is
  explicitly object-layer scoped, not full journal qualification.
- T03: private finite f64 Point3/Radius, domain <=1e150, checked constructors,
  explicit widening of legacy f32 and four numeric cases.
- T04: eight independent reference cases, complete scan and distinct bit-derived
  dyadic arithmetic; 1000 bounded seeds and explicit float/exact disagreements.
- T05: ten cases for additive fallible insertion and complete-or-error checked
  radius/top-k scans, limits, stable key/ID/insertion-ordinal ordering and
  cooperative cancellation. Duplicate legacy IDs remain distinct observations.
- T06: transient bounded adapter reusing original octree primitives and actual
  threshold64/leaf8/depth16; Euclidean and Manhattan bounds, explicit cosine
  scan, truthful outcome metadata, cumulative work and build/scratch caps,
  insertion rebuild and fallback. Twelve integration cases plus one internal
  admission regression; seeded radius/top-k comparisons include 1000 insertions.

Production additions are `crates/abi-wdbx/src/spatial3d/{mod,numeric,legacy_checked,octree_checked}.rs`,
additive exports in lib.rs and checked insertion/shared internal visibility in
spatial.rs. Existing query/builder bodies remain unchanged. cpu.rs is formatting
only. New tests are the five `wdbx_3d_*.rs` targets, independent support modules
and compatibility fixture inventory. Tooling changes are the evidence validator
and its tests. The remaining new files are audit, claim/evidence and execution
artifacts. Existing lockfiles, stored formats and goldens are unchanged.

## Verification and review

Pinned compiler selection uses the bin directory returned by
`rustup which --toolchain 1.99.1 rustc`, then `/usr/bin`, ahead of other Rust or
Swift shims; `RUSTUP_TOOLCHAIN=1.99.1`. Live-store and Zig opt-ins were removed.
The retained exact commands and exit codes are in
`evidence/final-reviewed/verification.json`; raw output is adjacent:

```sh
cargo check --workspace --all-targets --locked
bash tools/check.sh
python3 -m unittest discover -s tools/tests -p test_wdbx_3d_evidence.py -v
python3 tools/validate_wdbx_3d_evidence.py docs/claims/wdbx-3d/source-manifest.json
git diff --check
```

The gate itself runs instruction/size checks, fmt, workspace/all-target clippy
and workspace tests. Exact compiler and full locked metadata receipts are in
`evidence/final-qualified/`. Metadata first failed offline for missing cached
locked dependency `aes v0.9.3`; the subsequent locked metadata command succeeded.
An earlier final gate stopped at formatting; its failure is retained under
`evidence/final/`. A worker mistakenly restored compact baseline formatting
in cpu.rs after rustfmt; the exact pinned formatter output was restored.
No compiler substitution, lockfile rewrite or arithmetic change was used.

Expected-red and observed-green artifacts remain separate per-task. The broad
review found one P2: radius payload bytes were admitted after collection. The
fix admits cumulative slot plus payload bytes before reservation/push; the new
unit proves rejected output has zero capacity and no retained hit. Indexed and
fallback boundary tests pass. See `evidence/T06/p2-radius-*` and the appended
verdict in `reviews/final-review.md`. No independent reviewer test execution is
claimed. Original T06 stub identity was not retained; five behavioral red
failures are retained without claiming a reconstructible red binary.

Final changed-source gate: **685 Rust harness cases passed, zero failed or
ignored**, across29 summaries including five zero-case suites. Python: **16
passed**. Locked check, manifest admission and diffcheck all exit0. Fingerprints
remained unchanged. Final independent re-review ACCEPT closed the sole P2.
Counts and acceptance are recorded in acceptance.md and the final verification
receipt. Pedantic/missing-doc warnings remain permitted by repository
policy; passing clippy does not mean zero warnings. Empty harness/doc suites are
separately counted. Opt-in live-store cases returning early supply no live-data
evidence. Source fingerprints include untracked implementation/test files and
must remain unchanged during final verification.

## Contract limits, governance and recovery

M1 is the permitted `defined_float_profile`, not exact-real predicates. Keys
widen before subtraction and use defined non-FMA reduction; rounded sqrt is
presentation only. New geometry is unitless and separate from embeddings or
visual projections. No frame transforms, durable attachments, version-bound
spatial hits, index generations, service authorization or canonical commit
publication are introduced. Local result metadata declares exclusive-borrow
consistency and empty durable generations rather than inventing them.

Work caps count observation inspections, not wall time, node visits, heap work
or sorting. Scratch/output ceilings count specified logical requested bytes,
not total RSS or allocator metadata. Cancellation is cooperative polling;
pre-cancel is tested, a controlled in-flight race and allocator fault injection
are not qualified. Fallback retains consumed work and can reject on the remaining
budget. Corpus coverage is bounded; no universal numerical proof or speedup is
claimed.

The prompt resolves local ADR02/03/18 choices within this additive scope; it
confers no constitutional approval or signed-format change. Missing historical
massive-plan/private receipts and conflicting stale C0–C7 prose remain registered;
constitution definitions govern, no claim level was promoted. The original
117 requirements and all original specimen obligations remain preserved as
future obligations, not implementation passes. Optional FHE, consumers, devices,
benchmarks, deployment, remote execution and operator approval are unverified.
CI remains paused. No live store, model download, key/backup change, commit,
push, PR or production migration occurred.

Disable by stopping new checked-adapter calls; T05 complete scans and original
legacy APIs remain available. No persistent data changed, so no stored-byte
migration/rollback is needed. Retain the isolated diff and evidence for review;
do not reset an external dirty checkout. The next smallest packet is v5 T07
“Specify closed AABB and sphere predicates”, only after reading its complete task and
resolving its applicable review gates; it has not been launched.
