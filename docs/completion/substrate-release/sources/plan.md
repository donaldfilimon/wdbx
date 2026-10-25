# WDBX Complete Substrate Release Implementation Plan

> For agentic workers: use superpowers:subagent-driven-development or superpowers:executing-plans. Implement and independently review one task at a time. Track steps with checkboxes.

**Goal:** Deliver a production-qualified WDBX substrate covering all existing subsystems, durable spatial and hybrid retrieval, cryptographic lifecycle, real distributed operation, offline synchronization APIs, and the selected hardware backends.

**Architecture:** WDBX owns canonical storage/evidence; ABI owns authentication, normative service schemas and transport. Host adapters implement hardware execution without dependencies from WDBX back into ABI. New profiles are additive and preserve historical bytes.

**Tech Stack:** Rust 1.99.1, edition 2024; canonical CBOR/COSE; independent Python witnesses; reviewed TFHE-rs; reviewed consensus; ABI authenticated services and native accelerator runtimes.

**Spec:** User-approved 22-task substrate release plan (2026-10-23); docs/completion frozen contracts; supplied WDBX specification revision 6; docs/superpowers/specs/2026-10-20-wdbx-cose-trust-prototype-design.md; ABI member-erasure specification (separately approval-gated).

This is the execution transcription of the approved plan. The original authorization is the user message. It is not an approval receipt for cryptographic publication, privacy implementation, deployment, spending or live-data operations.

## Scope

All five crates; legacy/v2/v3 compatibility; complete episodes and full-payload COSE; trust/admission/lifecycle; actual erasure and backup reconciliation; durable spatial/read views/index generations; vector/lexical/graph/time/hybrid retrieval; codecs/multiway/CPU PCA; production FHE; operational three-voter replication/federation/offline APIs; Metal/CUDA/Vulkan/Core ML/ANE/Windows ML/TPU; macOS ARM64/Intel, Windows x64, Linux x64; 30-day service soak on Linux x64 and macOS ARM64.

Excluded: Studio research/UI/explorer UI, general consumer cutovers, public hosting/signup/billing/commercial SLA. Member custody is an included separately approved cross-repository exception.

## Global Constraints

- Rust 1.99.1; edition 2024; floor 1.99; unsafe_code = deny; existing crate dependency direction; WDBX Rust files <=1000 lines.
- Preserve unrelated dirty work, including crypto and Studio inputs; preserve Studio local path dependencies.
- Prepare local reviewable changes only. No commits, pushes, publication, purchases, infrastructure changes or live-data operations without separate authorization.
- Never test live stores, real member facts, production keys or private backups.
- Preserve legacy encoding, v2 parent order, v3 candidate/edge digests and original signed bytes.
- Keep the historical 163-requirement C0 inventory unchanged. The separate reviewed release contract does not close excluded broader requirements.
- No automatic promotion, golden regeneration, backend substitution or downgrade. Negative research needs explicit specification revision; original failed requirements remain failed/open.
- Missing hardware, independent review, licensing permission or owner approval blocks the affected requirement.
- No second canonical storage authority; no host runtime behavior in abi-compute; no I/O in the pure Studio kernel.

## Review Focus

1. Stale/forged authority cannot disclose or authorize mutations.
2. Torn writes/lost replies/retries cannot duplicate effects or invent noncommit.
3. Deletion survives snapshots, offline clients, imports and backup restore.
4. Incomplete indexes/workers/accelerators cannot claim exact complete results.
5. Crypto validity, semantic evidence, placement and qualification are separate.

## Interfaces and task cycle

Ratify bounded immutable complete-episode encode/decode, P1 verification, structured admission and terminal receipts. Host-issued authentication is authoritative, never client role/tenant/time strings. Read views bind roots/frontier/conflicts/frame/generation to current policy/consent. Query outcomes separate coverage/numeric accuracy/completion/consistency/evidence/execution. Stable operation IDs bind input digest; terminal outcomes are committed/rejected/failed-before-commit/cancelled-before-commit; unknown requires reconciliation. Compute binds operation/model/input/shape/dtype/destination/qualification/budgets/fence/output. Deletion distinguishes requested/local inaccessibility/propagation/backup reconciliation/physical expiry. ABI owns published wire/errors.

Each changed behavior: add direct positive/boundary/refusal tests, capture intended red, implement complete scoped change including recovery/docs, run focused/compatibility/owning gate, independently review exact diff and fix findings. Existing behavior may be qualified without artificial failure. Nonzero discovered test sets are required. No commits.

## Task 1 — Establish the substrate release contract

**Deliverable:** A separately versioned release inventory covering the expanded substrate scope.

- [ ] Record current revisions, dirty-file identities, toolchains and existing receipts.
- [ ] Map all historical requirements to included, external dependency or excluded-from-this-release, with reasons; do not change their historical status.
- [ ] Enumerate every public module, feature, persistence format and specification obligation, including substrate requirements absent from the 163-item inventory.
- [ ] Add validator mutation tests: missing requirement, duplicate routing, altered source, invented success, missing artifact and unjustified exclusion must fail.
- [ ] Record missing source artifacts explicitly. Inherited hashes do not substitute for unavailable original bytes.

**Acceptance:** Every release obligation has an owner, dependency, direct test or experiment, evidence requirement and closure condition.

## Task 2 — Ratify designs and dependency choices

**Deliverable:** Owner-approved schema/profile decisions that unlock implementation.

- [ ] Ratify full-payload P1 COSE; keep legacy digest profiles distinct.
- [ ] Freeze domain/AAD/protected headers/key identity/purpose/reader floors/downgrade/compatibility vectors.
- [ ] Ratify admission ordering, trust admin, receipt visibility, replay retention, privacy, migration/rollback.
- [ ] Review exact dependencies, licenses, advisories, targets and build provenance: OpenRaft stable release line is a candidate, current TFHE is a candidate only.
- [ ] Record actual named-owner approval against exact hashes and independent cryptographic review requirements.

**Acceptance:** No unresolved interface choice remains for dependent implementation. Rejection returns this task for amendment, never implementer-selected replacement.

## Task 3 — Qualify compatibility and supporting crates

- [ ] Qualify all legacy record types/hash encodings/checkpoints/WAL/writer lock/v2 maintenance leases.
- [ ] Pin unchanged v3 commitments/signatures/candidate-edge goldens/Zig hashing.
- [ ] Exercise credential permissions/redaction/telemetry overflow/CPU numeric failures/core scheduler.
- [ ] Test old-reader/new-writer combinations; unsupported combinations refuse or read-only.

**Acceptance:** Unchanged fixtures pass; new production paths do not depend on unsafe historical interpretation.

## Task 4 — Implement durable complete episodes and trust

- [ ] Integrate strict full-payload verification and independent second-language positive/rejection vectors.
- [ ] Persist trust register/activate/rotate/retire/terminal revoke through one ordered writer.
- [ ] Atomically bind trust/policy/consent/quotas/request/episode identity to admission.
- [ ] Preserve original payload/container and verification provenance.
- [ ] Crash-test append/sync/receipt/replay-index boundaries.

**Acceptance:** Stale/revoked decisions refuse; changed identity bytes conflict; authorized exact retry returns original receipt; refusal leaves canonical/usage unchanged; lost replies never duplicate; replay/repeated recovery agree.

## Task 5 — Complete governed lifecycle and deletion

- [ ] Enforce retention/expiry/holds/supersession/revocation/redacted derivatives for ratified complete episodes.
- [ ] Execute actual key-provider destruction with attributable outcomes, not requests alone.
- [ ] Track blob/embedding/index/cache/export/provisional/conflicting cleanup.
- [ ] Restore into quarantine and apply current deletion before serving.
- [ ] Crash-test every boundary; repeat cleanup twice.

**Acceptance:** Holds retain only authorized minimum; rollback/import/reconnect cannot resurrect; unavailable cleanup stays pending; request is not completed destruction.

## Task 6 — Complete approval-gated member custody

- [ ] Obtain required privacy/constitution approval before dependent implementation.
- [ ] Implement exact approved custody/holds/tombstones/receipts/consumer cleanup.
- [ ] Preserve Keychain restrictions/rotation; invent no cross-platform equivalents.
- [ ] Prove synthetic migration parity/replay/recovery/deletion/operator rollback.
- [ ] Unsupported custody providers remain unavailable until separately approved.

**Acceptance:** Attributable non-resurrecting synthetic erasure; live migration/real keys separately authorized.

## Task 7 — Implement durable geometry and read views

- [ ] Checked point/AABB/sphere: finite f64 magnitude <=1e150.
- [ ] Closed predicates and exact dyadic decisions/order over represented inputs.
- [ ] Durable attachments bind full record/version/attachment/frame/validity/provenance.
- [ ] Explicit rigid transforms/scales; reject unsupported/geodetic conversion.
- [ ] Preserve all maximal causal versions and snapshot observation maps.

**Acceptance:** Touching intersects; zero radius coincident; signed-zero equality; NaN/infinity/reversed/out-of-range refuse; durable IDs never narrow into octree local IDs.

## Task 8 — Implement verified index generations

- [ ] Sealed/verified/published/active/retired state machine.
- [ ] Bind roots/maps/frame/model/policy/algorithms/chunks.
- [ ] Durable file/directory publication and competing-builder fencing.
- [ ] Complete deltas for insert/remove/replace/tombstone/visibility changes.
- [ ] Canonical scan or typed failure if exact coverage unavailable.

**Acceptance:** No partial activation after crash/cancel; immutable pinned views; removals cannot leave stale winners; no unqualified pruning.

## Task 9 — Complete authorized vector and hybrid retrieval

- [ ] Authorize before traversal/cache/statistics.
- [ ] Exact vector reference, explicit approximate filtered ANN, bounded multi-vector aggregation.
- [ ] Versioned BM25/deterministic fusion/comparable shard statistics.
- [ ] Typed bounded graphs and half-open valid-time intervals.
- [ ] Authenticated pagination binds query/view/ranking/current authority.

**Acceptance:** No hidden topology/count leaks; equal dimension is not equal space; visible conflicts; no cursor authority bypass; k hits do not prove completeness.

## Task 10 — Complete evidence-aware retrieval qualification

- [ ] Eight separate dimensions with explicit unknowns.
- [ ] Consulted evidence distinct from causal ancestry.
- [ ] Invalid crypto excluded from ordinary protected reads, inspectable by authorized reviewers.
- [ ] Frozen equivalent-vector baseline/ablations/held-out protocol.
- [ ] Preserve negative outcomes and contamination limits.

**Acceptance:** Thresholds pass or explicit reviewed new claim/protocol; original failures stay open.

## Task 11 — Qualify codecs and production FHE

- [ ] Lossless roundtrip/malformed/archive bomb refusal.
- [ ] Lossy reconstruction/retrieval evidence; retain originals for exact rerank.
- [ ] Deprecate educational DGHV/secret-assisted refresh for production, preserve fixtures.
- [ ] Reviewed TFHE operation catalogue, secure keys, bounded parameters/typed failures/key-ciphertext identity.
- [ ] Plaintext independent oracle; wrong-key/malformed/overflow/exhaustion tests.

**Acceptance:** Independent exact-integration crypto review; library use alone is insufficient.

## Task 12 — Complete multiway publication and projection

- [ ] Preserve overlapping matches/event multiplicity/deterministic bounded enumeration.
- [ ] Resume publication bundles without false atomicity claims.
- [ ] PCA binds cohort/space/view/input order/algorithm.
- [ ] Freeze sign/repeated eigenspace rules; byte replay distinct from portable numeric agreement.
- [ ] Partial/resume/hidden-cohort/deletion invalidation tests.

**Acceptance:** Recoverable idempotent publication; projections are not geometry or causal truth.

## Task 13 — Implement qualified compute dispatch

- [ ] Preserve Accelerator and existing capability facts.
- [ ] Separate build/device/driver/operation/model/shape/numeric qualification.
- [ ] Account queued/running/uncertain work/buffers/transfers.
- [ ] Finite acyclic fallback, same authority/model/budgets.
- [ ] Output-violation quarantine and relevant-change invalidation.

**Acceptance:** Cancellation retains uncertain reservations; fallback cannot change privacy/model; detection/compilation is not execution.

## Task 14 — Qualify Metal, CUDA and Vulkan

- [ ] Freeze every supported backend/platform/device-family/driver/shape/precision combination.
- [ ] Real dot/norm/cosine/L2/top-k versus independent CPU reference.
- [ ] Authoritative recomputation for threshold gates/exact predicates.
- [ ] OOM/reset/cancel/stale-buffer/malformed-output/forbidden-fallback tests.
- [ ] Measure cold/warm end-to-end including transfer/verification.

**Acceptance:** Direct device/numeric receipts for every required entry; documented unsupported combinations; missing devices block.

## Task 15 — Qualify neural providers and TPU

- [ ] Freeze synthetic redistributable models/conversions/shapes/reference outputs.
- [ ] Provider adapters without automatic model downloads/activation.
- [ ] Output correctness distinct from placement; CPU does not close NPU.
- [ ] Authenticated TPU remote worker, fixed inventory/scoped export.
- [ ] Proposed isolated Google Cloud TPU/JAX target; no infrastructure/spending authorization.

**Acceptance:** All selected provider/platform combinations pass runtime/output/failure/placement requirements.

## Task 16 — Implement operational consensus and replication

- [ ] Reviewed consensus durable storage/state-machine/authenticated transport.
- [ ] Replicate exact canonical commands/authority revisions/deletions.
- [ ] Durable-owner stale-term fencing; minorities cannot acknowledge.
- [ ] Snapshots/dependency-closed reads/anti-entropy/restartable movement.
- [ ] Real separate-host failures, leader loss, disk full, torn log, delay and interrupted snapshot.

**Acceptance:** No loss of quorum-acknowledged writes after one voter fails; no duplicate/revival on recovery.

## Task 17 — Complete independent-organization federation

- [ ] Enrollment/scoped grants/residency/certificate-key lifecycle.
- [ ] Original bytes/ancestry/checkpoint/deletion verification before serving.
- [ ] Equivocation/rollback/replay-conflict detection.
- [ ] Resumable transfer; immutable foreign signatures.
- [ ] Two independent administrations on actual hosts.

**Acceptance:** Revoked peers cannot exchange; rejoin revalidates authority/deletions before disclosure.

## Task 18 — Complete durable offline synchronization APIs

- [ ] Persist IDs/dependencies/base revision/payload refs/retries/terminal receipts.
- [ ] Current policy/consent reauthorization on reconnect.
- [ ] Digest-verified resumable assets.
- [ ] Independent-compatible merge only, explicit conflict branches.
- [ ] Indefinite disconnect/schema skew/resnapshot preserve unsent work.

**Acceptance:** Reorder/duplicate/send-ack crash never duplicates; old clients cannot resurrect or grandfather authority.

## Task 19 — Publish ABI service contracts and migration tools

- [ ] Owner-reviewed Retrieve/ProposeWrite/Verify/status/views/admin schema.
- [ ] Existing ABI authentication/transport boundary.
- [ ] Generic KV/vector cannot bypass canonical governance.
- [ ] Shadow/synthetic checkpoint/writer fencing.
- [ ] Before-after-commit cancel/lost-receipt/negotiation/reader rollback tests.

**Acceptance:** Services match canonical semantics; no general live consumer cutover/access.

## Task 20 — Complete security and adversarial qualification

- [ ] Bounded fuzz codec/container/journal/index/import/network inputs.
- [ ] Scope substitution/role spoof/stale consent/cursor tamper/secret diagnostics.
- [ ] Custody/supply-chain/license/artifact provenance review.
- [ ] Backup+offline, revoked signer, hidden projection, malicious worker scenarios.
- [ ] Fix blockers and rerun affected evidence.

**Acceptance:** Independent exact-artifact security acceptance, no blockers; bounded owner-approved limitations.

## Task 21 — Qualify platform artifacts and compatibility consumers

- [ ] Native builds/execution on all four targets, supported feature tuples.
- [ ] Filesystem durability/permissions/locks/upgrade/recovery/shutdown/uninstall.
- [ ] ABI integration and Studio local-path compatibility without research expansion.
- [ ] Hashes/dependency/license/reader-floor/rollback manifests.
- [ ] No production test bridge/credentials/debug bypass.

**Acceptance:** Cross-compile is insufficient; every target/hardware entry needs direct evidence.

## Task 22 — Run operational acceptance and close the release

- [ ] Exact candidate: 30 consecutive days on Linux x64 and macOS ARM64.
- [ ] Weekly node-loss/restore/revoke/delete-propagation drills.
- [ ] No acknowledged-write loss; service recovery <=15 minutes; backup restore <=4 hours.
- [ ] Publish bounded capacity/resource measurements, no throughput SLA.
- [ ] Per-requirement accepted evidence/operator witness/limitations.

**Acceptance:** Relevant changes restart affected qualification; correctness/privacy/durability failures block and require rerun. Missing prerequisites are not complete. Publication separately authorized.

## Verification and handoff

Run pinned root tools/check.sh; full-fhe and experimental-dghv-bootstrap separately. Contract/execution/spatial/crypto witness checks remain separate. ABI has its own current authoritative gates; affected Studio compatibility requires check/lint/test:native/check:native independently.

Each packet binds requirement/profile, exact source/dependency/configuration, discovered cases, commands/raw outcomes/hashes, positive/boundary/refusal/recovery results, independent review and invalidation rules. No zero-test green result closes a claim.

Completion wording: WDBX substrate release qualified only after Task 22; excluded Studio/broader ecosystem obligations remain open.
