# WDBX Full Completion Implementation Plan

> **For agentic workers:** Use `superpowers:subagent-driven-development` for implementation and independent task review. Parallelize read-only investigations and independent qualification, not competing implementations.

**Goal:** Complete the constitutional WDBX substrate, full Studio research architecture, automatic offline synchronization, cross-platform desktop delivery, independently administered federation, and private hosted staging. No public launch, signup, billing, or commercial SLA.

**Architecture:** WDBX owns canonical episode semantics. ABI owns authorization, normative wire schemas, and transport. Consumers retain declared projections. Preserve frozen commitments and introduce complete episodes through a new versioned domain. Shared physical tenancy does not share authority or payload keys.

**Tech Stack:** Repository-pinned Rust, Python interoperability witnesses, Studio Rust/WASM kernel, Bun/React/Vinext, Tauri, ABI gateway and Abbey protocol.

**Spec:** User-approved WDBX Full Completion Plan (2026-10-20); `../abi/docs/superpowers/specs/2026-08-22-abbey-system-constitution.md`; `../abi/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md`; `specimen-studio/public/WDBX-Specimen-Architecture-Specification.md`. Current executable source overrides historical implementation descriptions, not normative requirements.

## Global Constraints

- Work in canonical default-branch checkouts. Preserve unrelated dirty work, including `crates/abi-compute/src/cpu.rs`; stage exact paths. No default branches/worktrees beyond these checkouts.
- Never test against live stores or commit runtime data, credentials, or private evidence. No public launch or unrequested service changes.
- Preserve v2 JSON commitments/parent order, v3 candidate/edge digests, original signed bytes and existing corpus provenance. Only ABI's vendor tool refreshes the normative corpus.
- ABI owns wire contracts, gateway and authorization; WDBX owns durable semantics; Studio/Abbey projections declare losses. No second canonical episode writer.
- Shared canonical storage is partitioned by authenticated tenant/scope in every lookup, cache, job, blob and index; tenant-specific key hierarchies support per-object erasure.
- Each tenant/scope has one quorum-fenced canonical writer in a three-voter replicated shard. Partitioned minorities cannot acknowledge canonical writes.
- Federation supports independent organizations with explicit bilateral trust and scoped grants. Foreign evidence retains its origin authority.
- Offline writes are provisional. Reauthorize on reconnect; auto-merge only independent edits; preserve conflicting branches pending authorized resolution.
- No fixed offline expiry. Retain minimal anti-resurrection deletion metadata indefinitely. Resnapshot/revalidate old clients; never silently discard their unsent work.
- Desktop targets: macOS ARM64/x86-64, Windows x86-64, Linux x86-64. Service targets: Linux x86-64 and macOS ARM64.
- Operational acceptance: no loss of quorum-acknowledged writes after one voter fails; service recovery <=15 minutes; backup restore <=4 hours. Capacity is measured, not a commercial SLA.
- WDBX Rust source files <=1000 lines. Studio remains a separate workspace with pinned Git WDBX dependencies.
- Evidence levels C0-C7 are separate. Research failure leaves the relevant completion claim open. Independent crypto review precedes production qualification.

## Review Focus

1. Cross-tenant references and shared caches must never cross authenticated scope.
2. Interrupted writes, retries and duplicate operations must not produce partial or duplicate canonical effects.
3. Deletion must survive backups, conflicting edits, indefinite offline intervals and foreign imports.
4. Stale policy, consent or signer authority must never be silently grandfathered into new writes.
5. Research, native hardware and cross-platform claims must name exact artifacts and direct evidence; fallback and configuration are not execution.

## Task 1: Establish conformance and experiment contracts

- [ ] Record repository revisions, dirty paths, gates and current evidence without private runtime data.
- [ ] Create a machine-readable requirement manifest covering constitutional semantics, federation, hosting, sync, every Studio architecture chapter and each supported platform. Each entry names owner, task, test/evidence and falsifiable completion condition.
- [ ] Resolve Studio section 25.1 algorithms in subsystem designs: language, transforms, synthesis, ATP equations, vision, supervision, mutation and graves. Preserve section 25.3 acceptance cases verbatim in a versioned inventory.
- [ ] Freeze research corpora, held-out splits, baselines, seeds, budgets, metrics and thresholds before experiments. Retain negative outcomes. Define subsystem interfaces and dependency contracts before downstream implementation.
- [ ] Validate manifest closure and source provenance; independently review the contracts. Documentation and experimental specifications do not count as implemented research.

## Task 2: Canonical episodes and cryptographic interoperability

- [ ] Add a complete versioned envelope binding tenant/scope, context, regime, predictions, actions, outcomes, consulted evidence, uncertainty, authority versions, provenance, lifecycle and artifact references.
- [ ] Implement bounded canonical-CBOR reading, COSE interoperability, signer identity, trusted-key administration, rotation, revocation and historical verification without reinterpreting old records.
- [ ] Extend second-language witnesses to decoding, malformed input and signatures. Test tamper/substitution/replay/downgrade cases and unchanged existing goldens.

## Task 3: Admission and lifecycle

- [ ] Structured write decisions, retention classes, authority/consent, quotas and replay protection; minimal mandatory incidents cannot bypass privacy restrictions.
- [ ] Session/TTL expiration, holds, supersession/revocation, redacted derivatives, object-key destruction, tombstones and auditable garbage collection.
- [ ] Cover payloads, provisional/conflicting data, indexes, embeddings, blobs, caches, managed exports and restored backups. Distinguish active erasure, inaccessible backups, physical expiry and disconnected acknowledgements.
- [ ] Prove crash safety, idempotency, no resurrection and no canonical mutation on rejected writes.

## Task 4: Evidence-aware retrieval

- [ ] Add constitutional queries/results alongside receipt lookup and legacy search. Expose eight dimensions, constraints, disposition, lineage and explanation separately.
- [ ] Reject invalid crypto; preserve uncertain/contradicted/quarantined evidence with explicit applicability. Rebuild disposable indexes deterministically.
- [ ] Test stale policy, wrong regime and incomplete lineage. Compare repeated failures and stale reuse against equivalent vector retrieval with frozen embeddings, storage and candidate counts; retain negative results.

## Task 5: ABI service and consumer migration

- [ ] Implement Retrieve/ProposeWrite/Verify semantics through ABI authorization and existing transport, with typed decisions, negotiation, identity/scope binding, cancellation, idempotency and per-principal budgets.
- [ ] Generic KV/vector methods cannot bypass episode governance. Update normative ABI schemas and refresh vendored qualification only through its tool.
- [ ] Shadow-read, replay, compare, qualify recovery/deletion and cut over one writer in Abbey/applicable adapters. Reconcile Swift stores via explicit migration/projection contracts and synthetic fixtures.
- [ ] Test tenant isolation and rollback without losing accepted writes or deletions.

## Task 6: Independent-organization federation

- [ ] Peer enrollment, bilateral trust, sharing grants, residency, revocation and certificate/key lifecycle.
- [ ] Verified imports/exports, ancestry, equivocation, checkpoints, deduplication, resumable reconciliation and deletion propagation, preserving origin authority.
- [ ] Returning peers validate authorization and deletion watermarks before serving. Separate-host fault tests cover partition/rejoin, revoked peers, malicious ancestry, conflicting policies and rollback.

## Task 7: Automatic offline synchronization

- [ ] Transactional browser/native outboxes: operation IDs, dependencies, schema, retries and terminal receipts. Reauthorize against current server policy/consent on reconnect.
- [ ] Resumable verified asset transfers; model identities sync without automatic weight distribution.
- [ ] Deterministic independent-edit merge; preserve incompatible specimen/learning/network/authority versions. Surface pending/rejected/conflicted/synchronized status.
- [ ] Test reordered/duplicate messages, interrupted blobs, crashes, revoked devices, schema skew and indefinite-disconnection deletion. Browser sync runs while open/on reconnect, not after tab shutdown.

## Task 8: Full Studio research mechanisms

- [ ] Identity/HybridTable: stable references, duplicate semantics, encoders and complete lookup.
- [ ] Language/scripts: language utilities, context/negation, scoped grammar and bounded automata.
- [ ] Scoring/orchestration: confidence evidence, jitter/ATP order, complementary votes and attachment routing.
- [ ] Transient synthesis: fixed non-learning composition, provenance, bounded decoder and held-out efficacy.
- [ ] ATP: specified buildup/release equations and meaning-preserving tone.
- [ ] Vision/fuzzy art: contours/SDF, focus/periphery, OCR and instruction-driven rendering.
- [ ] Type A/B: attributable correction, inference proposals and explicit learning approval.
- [ ] Growth/PHAGY: permanent lineage, unique cleanup jobs, timeout recovery and graves.
- [ ] Scheduling/tuning: global ceilings, fairness, cancellation, complete rafts and measured tuning.
- [ ] Preserve native/WASM parity; qualify actual model/hardware execution separately. All behavioral cases and frozen efficacy thresholds must pass before their claim closes.

## Task 9: Studio canonical integration

- [ ] Canonical adapter for edits/learning/outcomes/lifecycle; snapshots remain projections. Update pinned Git dependency revisions and lockfiles together.
- [ ] Complete busy/cancel/malformed-response/import/storage-failure/recovery behavior; regenerate specification assets after authoritative text changes.
- [ ] Qualify save/reopen/import/export, provenance, conflict resolution, deletion and offline recovery in both editions.

## Task 10: Cross-platform packages and private staging

- [ ] Restore real Intel macOS/Windows/Linux runners and qualify production installs/upgrades/uninstall, helpers, OCR/inference, accessibility and acceleration per platform.
- [ ] Signing/notarization and installed-byte evidence; no production test bridge.
- [ ] Private deployment: tenant administration, credentials, quotas, backups, observability, incidents and reproducible deployment. Test noisy neighbors, compromise, node failure, restore and rollback.
- [ ] Keep public ingress/signup disabled. Produce exact-artifact receipts and measured capacity/recovery results on both service platforms.

## Verification and evidence

Run C1/C2 gates at each work package, C3 before behavior-changing cutover, then profile-specific C4 shadow, C5 canary, C6 operator witness and C7 sustained operation. C7 cannot be manufactured without elapsed operation.

- WDBX: `bash tools/check.sh`; affected optional FHE suites separately.
- Studio: `bun run check`, `bun run lint`, `bun run test:native`, `bun run check:native`; browser/themes, hardware and installed-native qualification separately.
- ABI/Abbey: each checkout's current authoritative gate plus relevant integration suites.

Completion requires requirement-level evidence, all platform artifacts, reproducible research, recovery/deletion drills and migration rollback, with no unresolved release blockers. Infrastructure, credentials, independent review or failed research remain explicitly incomplete. Execution progress lives in this plan's SDD ledger; no task is complete merely because a design was written.
