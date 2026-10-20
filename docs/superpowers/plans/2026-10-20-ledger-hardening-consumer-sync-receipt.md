# Ledger hardening and consumer synchronization — 2026-10-20

## Scope and ownership

Canonical implementation stays in WDBX. ABI transports store decisions; Abbey
consumes WDBX and ABI contracts without introducing a second canonical episode.
This is a local dirty-source qualification, not an immutable dependency-pin,
publication, deployment, power-loss, or all-platform receipt.

The inventoried modified/untracked files in the existing detached
`../wdbx-spatial-checked-v1` checkout were byte-identical to files already in
canonical WDBX. No missing contribution was found, no duplicate implementation
was copied, and that pre-existing checkout was not discarded or removed.

Both consumers already resolve substrate dependencies through sibling paths:
ABI uses `../wdbx/crates/`; Abbey uses those same paths directly and through ABI.
No registry/git duplicate was found in their root manifest/lock inventories.
Their immutable CI revision pins were not advanced to an uncommitted tree.

## Implementation and contract changes

- Replay charges persisted JSONL lengths, including whitespace, escapes and
  newlines, plus the existing candidate payload charges. An incomplete tail
  discarded by verified recovery is not charged.
- Unix initialization synchronizes directory entries for the store and ledger.
  Direct store/file symlinks are rejected; ancestor aliases such as macOS `/var`
  remain compatible. Synchronization failures fail initialization on Unix.
  Non-Unix retains file-only synchronization, not directory crash durability.
- Compensation attempts retain their observed restoration outcome. A successful
  `compensated` terminal requires exact restoration. Previously admitted
  contradictory terminal chains now fail replay rather than being rewritten.
- Episode writer-lock acquisition retries contention for a bounded 50 ms, as
  the legacy durable store does for fork/exec-inherited descriptors. Persistent
  contention remains `WriterBusy`; real lock errors return `Io` without retry.
  The acquired handle remains held for the store lifetime.
- REST overload responses now half-close the write side before bounded input
  discard, preventing unread request data from resetting the connection and
  erasing the 503 on the tested host. Discard retains the existing 25 ms / 64 KiB
  ceilings and never routes a rejected request, allocates another worker, or
  consumes a store rate token.

Commitment domains, existing event encodings and signatures are unchanged.

## Consumer evidence

ABI's gateway tests exercise actual WDBX admission before and after reopen:
inexact compensation cannot publish success, refusals leave bytes unchanged
and emit no mutation notice, rejected request IDs remain usable, and persisted
JSON padding cannot evade the storage budget. Stable store errors retain their
existing gRPC status mapping; no adapter-side validation policy was introduced.

Abbey's application memory backend remains the legacy `DurableStore`. Its
feature-gated scratch SEA composition also consumes signed `EpisodeStore`;
the added regression verifies `PersistenceOutcome::Refused` for fabricated
success without changing receipts, usage, ledger bytes or signature validity.
This test does not establish a production episode-ledger cutover.

Owned consumer files:

- `../abi/crates/abi-wdbx-gateway/tests/episodes.rs`
- `../abi/crates/abi-wdbx-gateway/tests/episodes/ledger_hardening.rs`
- `../abi/tasks/todo.md` (bounded qualification status only)
- `../abbey/tests/governed_sea.rs`
- `../abbey/tests/governed_sea/ledger_hardening.rs`
- `../abbey/tests/daemon_memory_reads.rs` (backend-specific assertion diagnostic)

## Verification history

- The preceding WDBX ledger-hardening slice passed `bash tools/check.sh`.
  Later tool resolution showed the unqualified shell selected Homebrew Rust
  1.99.1, not the required rustup installation; those runs are preserved as
  diagnostics, not accepted pinned-toolchain qualification.
- Initial consumer-source inventory and locked Cargo metadata inspection passed.
- ABI's two focused gateway regressions passed.
- Abbey's initial `--locked --features wdbx` test run passed, including the
  pre-existing signed canonical-proposal composition test. This preceded the
  added compensation composition test and is not evidence for that new test.
- Initial full ABI gate stopped because neither dependency scanner was on PATH.
  The existing `~/.cargo/bin/cargo-audit` was located; rerun restores PATH without
  installing software or bypassing the mandatory security policy.
- The next WDBX gate passed formatting and Clippy, then exposed a REST saturation
  response reset in `rest::tests::saturated_rest_readers_get_503_without_store_routing`.
  That failure is retained; a later focused success alone will not replace the
  full-gate requirement.
- Abbey's added signed compensation composition regression passed separately.
- The next broad ABI/Abbey runs reached the tool's 120-second execution limit;
  neither timeout is a completed gate. Both were relaunched without that limit.
- Abbey's unrestricted-duration gate stopped on formatting in concurrently
  edited `src/tui/app.rs` and `src/tui/tests/providers.rs`. Those unrelated owner
  edits were not reformatted or overwritten. No full Abbey gate success is
  claimed for this source set.
- Independent scoped review of writer-lock and both consumer-test changes
  found no blockers.
- ABI's unrestricted-duration `PATH="$HOME/.cargo/bin:$PATH" ./tools/check.sh`
  completed with exit 0 (`check: all green`), including both new gateway
  regressions, required dependency-security checks, sibling contract tests,
  local device-feature checks, benchmark guard, docs and release builds.
  Eight optional central-sync Python tests were skipped and CUDA compilation
  remained unavailable without `nvcc`; neither supplies qualification evidence.
  This run does not qualify subsequent REST changes or a later moving sibling.
- The REST unread-input regression failed before the correction with
  `ConnectionReset` and passed afterward. The implementing agent reports 19
  focused REST tests plus 300 executions across 200 invocations (eight concurrent)
  passing. This is local macOS TCP evidence, not all-platform transport proof.
- WDBX's next `bash tools/check.sh` after the REST correction completed with
  exit 0 (`check: all green`), including the new lock and overload regressions.
  The pinned-toolchain rerun explicitly prepends
  `~/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:/usr/bin` to PATH;
  it completed with exit 0 (`check: all green`). This is the accepted WDBX gate
  result. No compiler installation or pin change was made.
- Abbey's subsequent formatting check passed after its owner edits settled;
  its full gate is rerunning. ABI's full gate also completed again with exit 0
  against the corrected REST source, rather than inheriting the earlier gate
  receipt. The same eight optional Python skips and unavailable CUDA compilation
  remain disclosed.
- Independent review of the REST-only correction found no blockers; it confirms
  finite discard/accept-loop bounds and no store routing. It does not establish
  TCP EOF behavior on untested platforms.
- Abbey's full rerun passed default-mode checks and the WDBX-mode library tests,
  then `tests/daemon_memory_reads.rs` failed because negotiation granted no
  `ReadMemory` capability. The immediate focused locked WDBX-feature rerun
  passed. This is not full-gate success; startup/fixture-race diagnosis remains
  open, and no unrelated product authority was relaxed.
- Startup-order review ruled out a CLI-seeding/probe race: the exact backend
  probe finishes before listener bind. A scratch experiment reproduced missing
  `ReadMemory` with a default-feature daemon paired with a WDBX-feature CLI;
  matching WDBX-feature binaries granted it. This establishes feature skew as
  a concrete candidate, not the proven cause of the earlier failure.
- The assertion now names the backend. No sleeps, grant retries, capability
  widening or product startup changes were added. Abbey's full gate was rerun
  with a newly allocated, task-private `CARGO_TARGET_DIR` and four compile jobs
  to avoid shared-target feature replacement by concurrent builds. Outcome:
  exit 0 (`check.sh: OK`). All four feature modes, warning-denied docs, 98 Python
  tests, claims/instruction checks, desktop IPC codegen drift, Program 3 boundary
  and isolated installer rollback checks completed. Windows/Linux cross-checks
  were skipped because the targets were not installed. The new signed ledger
  regression ran in WDBX mode. The earlier failed gate remains retained.

## Source attribution and handoff limits

All three accepted local gate commands completed successfully. Their inputs
are dirty working trees, not published immutable dependency revisions. No
test output silently promotes later owner changes into an older receipt.

Post-run Rust-input inventory (all tracked/untracked `.rs`, Cargo manifests and
locks, and toolchain files; excludes Studio and historical Zig):

| Repository | HEAD at final inventory | Rust-input aggregate SHA-256 |
| --- | --- | --- |
| WDBX | `8f56e7d2574d889d9bc4efa75360e903c6f26d76` | `34b6a089fd2671125ca724d8a310e2ea26f293e7f0a60f04c15c2031725b3c79` |
| ABI | `4e559a28092cc6f08604e28ecb57d588bd3ea63a` | `fd7be7f8bd4fbc0f01af9e5498282995a9967dc85da05eabcc760801e1ffef1f` |
| Abbey | `25024690fae1b2f64d154490d722ddaa553203ba` | `170496e46c6bf81815306d117c2188f1cd750e6f1f9304bfcb0c8598c5eeb080` |

The aggregate hashes UTF-8 compact JSON of sorted `[relative_path, file_sha256]`
pairs. This is a Rust-input inventory, not a full source/tooling manifest.
WDBX and ABI Rust inputs were unchanged from the pre-isolated-run inventory.
Abbey changed only the task-owned backend diagnostic and an independently owned
`tests/legacy_conversation_migration.rs` delta. The latter was not authored or
reverted here; a frozen final three-repository cohort remains a separate
qualification step. Source-only inventory hashes contain no live store data.

## Remaining boundaries

No task-authored commits, pushes, CI execution, system/user installs, live user
stores, external providers, runtime migrations, corpus regeneration or production
conformance promotion. Installer smoke uses isolated scratch destinations only.
Optional FHE features and non-macOS runtime paths require separate evidence.
Unrelated active decoder, governance, TUI and Studio work remains preserved.
