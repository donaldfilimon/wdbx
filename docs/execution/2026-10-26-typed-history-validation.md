# Typed ledger review remediation

The full WDBX completion objective remains open. This packet fixes the two
findings from the read-only main-versus-origin/main review before resuming the
existing substrate release plan. The [packet plan](../plans/2026-10-26-typed-history-validation.md)
defines ownership and verification.

## Behavior

Typed inspect and commit now authenticate every retained KV envelope, including
superseded versions, before exposing current rows. `V2Snapshot::kv_versions`
provides a borrowed history slice; `get` still returns maximal causal versions.
No snapshot serialization, V2 commitment bytes, format version or independent
oracle changed. Temporal node and edge populations both cause `StoreCorrupt` in
a typed-only root, alongside the already-refused vector/spatial/audit kinds.

Two new real-binary integration tests construct isolated synthetic roots through
the ordinary V2 writer, so all unkeyed V2 integrity is valid. The history test
copies two correctly authenticated typed envelopes, changes only the superseded
value in its negative case, and requires both inspect and commit to refuse. Its
untampered control still inspects and admits a third commit. The temporal test
mixes a valid typed envelope with a temporal node, then separately an edge.
Refusals leave committed transaction counts unchanged. No live data is used.

## Evidence

Base remains local `main` at `612d357`; the previous frame slice is preserved.
Raw outputs are in ignored `.work/typed-history/`, with Rust build artifacts
reused from this chat's isolated `.work/task7-frames/target`.

- Before the fix: real CLI suite exits 101; nine existing tests pass and the two
  new tests fail because inspect incorrectly succeeds (`red.log`).
- After the fix: all eleven CLI tests pass, exit 0 (`green.log`), including
  refused-commit no-append checks and the authentic-history positive control.
- Frozen release validator: exit 0. Live drift report: expected exit 1, unchanged
  from the preceding frame packet's report (39 declaration failures across
  `lib.rs` and `v2.rs`). Frozen inventories and pins are not repointed.
- `git diff --check`: exit 0.
- Full `bash tools/check.sh`: exit 0, `check: all green`; 1,210 Rust
  tests including doctests pass, 0 fail, 0 ignored. Python witness suites
  pass 52 complete-episode, 28 COSE and 14 native-model cases (`gate.log`).
  Existing spatial and cross-language oracles run unchanged. Optional FHE,
  Studio, other native platforms, hardware and production service acceptance
  were not rerun for this CLI correction.
- HawkScan remains unavailable: `hawk` is absent from PATH; no DAST ran and no
  security qualification is claimed.

## Remaining completion obligations

These fixes do not qualify the typed CLI as a complete authenticated transaction
history or a production host. Independent review and consumer qualification of
the correction remain separate acceptance work. The release register's 22 tasks
remain governed by their existing contracts, including Task 2 owner/interface
ratification, Task 7 durable attachments/full frame registry/causal views, Task 8
verified crash-safe fenced publication, remaining retrieval/compute/lifecycle
work, native/hardware/multi-host/security acceptance and the operational campaign.
The missing v5 companion provenance and reviewed source/declaration-inventory
migration remain open. No C0 promotion, commit, push or live-data operation was
performed.
