# WDBX consumer compatibility checkpoints

Planning reconciliation for the approved [full completion program](2026-10-20-wdbx-full-completion.md) and [bounded reader slice](2026-10-20-canonical-cbor-reader.md).

Coordination source: `../COORDINATION_PLAN_2026-10-20.md`. This companion adds handoff requirements within WDBX's existing scope; it does not grant consumer implementation, tests, CI, publication, deployment, or history changes. Independently authorized WDBX work continues. Consumer-owned plans remain with their owners.

## Ownership and dependencies

| Authority | Owner chat | Boundary |
| --- | --- | --- |
| WDBX | `01a15872-c366-7cc1-97e6-274eb388ed4b` | Canonical envelope/commitment, durable episodes/evidence, lifecycle, retrieval semantics, Studio |
| ABI | `01a1585e-e74a-7fc1-a327-1e15a89eb2b3` | Normative inter-component schemas/corpus, authentication/authorization, transport, service messages |
| Abbey | `01a15871-5d1f-72b2-b4f7-e9d1a33215a5` | Consumer/projection, user interaction, learning authority, consumer migration/recovery |

Dependency sequence: agree shared contracts -> qualify exact WDBX source -> qualify ABI adapter/service against that source -> qualify Abbey projection against both -> freeze a three-repository candidate -> independently authorize migration/shadow/cutover. A consumer need not adopt a new additive API merely because it exists.

## Required checkpoints

| Checkpoint | WDBX handoff | ABI acceptance | Abbey acceptance |
| --- | --- | --- | --- |
| C-01 Frozen domains | Exact profile/schema IDs, canonical bytes/digests and immutable original signed-byte fixtures; unchanged v2/candidate/edge goldens | Existing fields/tags/corpus bytes preserved; new wire schema negotiated separately | Old projection remains readable; explicit losses and schema identities, no reinterpretation as a complete episode |
| C-02 Reader scope | Strict supported-profile reader, resource limits, errors and independent witness | Syntax success never substitutes for authorization, signer trust or schema admission | Parsed data is not treated as an authorized or trustworthy memory effect |
| C-03 Complete episode | Typed field/absence/unit table, tenant/scope and origin binding, causal versus consulted lineage, new commitment domain | Program 1 approves normative wire/fixtures; paired positive/negative vectors pin mapping | Projection contract declares every dropped field and retains canonical references |
| C-04 Memory service | Typed Retrieve/ProposeWrite/Verify behavior, eight evidence dimensions, dispositions, lifecycle and verification status | Principal-derived tenant/scope; bounded budgets/deadlines; cancellation, idempotency and replay errors; no KV/vector bypass | Render refusal/unknown/conflict explicitly; verify before applicable effects; do not invent success |
| C-05 Lifecycle | Correction/hold/erasure/tombstone semantics across canonical and derived state | Policy/consent checks before mutation; durable deletion acknowledgment and restore watermark | Erase local projections/outboxes/conflicts; replay tombstones before exposing restored data; preserve accepted deletion during rollback |
| C-06 Reconnect/federation | Provisional operation IDs, dependency/schema identities, current authorization checks, indefinite anti-resurrection history and foreign origin | Scoped bilateral trust, current grant/policy/consent checks, fencing/checkpoint validation | Preserve unsent edits; explicit terminal rejections and conflict versions; no canonical local writer or automatic truth resolution |
| C-07 Corpus | Exact ABI corpus source revision and aggregate digest; local canonical/signature vectors remain separately identified | Publish and verify the normative corpus; refresh consumers only through established vendor tooling | Independently qualify matching corpus bytes and adapter semantics; fixture validity does not authorize writes |
| C-08 Dependency revisions | Exact qualified WDBX commit plus dirty-path/content manifest if applicable | Qualify the named WDBX revision and record ABI commit/dirty manifest; preserve sibling crate identity | Qualify the exact paired ABI/WDBX revisions; Studio's Git pin/lock updates wait for an available qualified revision |
| C-09 Migration | Shadow/replay/compare, one-writer cutover, failure/restart/deletion/rollback receipts | Service adapter preserves accepted writes and deletion watermarks through rollback | UI/projection parity and persistence-before-visible-result; no effect replay after interrupted/unknown outcomes |

The bounded reader is independent of unresolved Studio research efficacy and introduces no new wire format. Complete-envelope transport exposure remains gated on C-03; source-only Rust types do not settle ABI schemas.

## Handoff receipt

Each checkpoint requires:

- Producer/consumer owner, repository, commit, branch, owned paths and SHA-256 content manifest for any dirty files participating in qualification.
- Contract/profile/schema/version and corpus source revision/digest; original-byte and projection-loss identities where relevant.
- Accepted scenario and expected observation, command, toolchain/environment, exit status and explicit skips.
- Artifact digests, independent reviewer scope, unresolved failures, allowed evidence claim, and rollback/deletion conditions.
- Consumer-impact classification: unchanged frozen bytes; additive API; negotiated new schema; or migration/cutover required. New types cannot silently force one of the latter two.

A moving sibling HEAD is not a qualification target. Read-only planning snapshots do not reserve files; owners must reconcile reservations before cross-repository edits or freeze-window qualification. This WDBX execution does not edit ABI's active governed-host files or Abbey's active runtime/learning files.

## Current evidence and open gates

- WDBX baseline: `08a69b3f22db61093e06527b3308b10589687022` plus the pre-existing CPU formatting change; `bash tools/check.sh` completed locally with 647 passed, zero failed/ignored. This is baseline source evidence, not qualification of future reader/complete-envelope changes or of consumer revisions.
- The 163-requirement contract handoff is independently approved at C0 after two fix rounds; 36 checker/vector tests pass on Python 3.9 and 3.15. The approved working snapshot's 24 hashes and review receipts are in the execution ledger/report. Research efficacy and product qualification remain open.
- The Rust reader implementation has started on its reserved paths using offline/locked local checks. Existing encoder/goldens stay frozen; decoder qualification and its independent Python witness remain pending.
- ABI and Abbey source/lockfiles are concurrently changing under their owners. No paired exact-revision gate or consumer cutover is claimed here.
- The completion manifest's experiment-integrity checker is separate from measured efficacy. Independent crypto review, separate-host federation, platform installs/signing, private staging and elapsed C7 operation remain separate gates.
- Public launch, signup, billing and commercial SLA remain deferred.

This receipt is local planning state. It does not send a message to another chat or claim an owner has accepted a checkpoint.

## Bounded implementation checkpoint (09:17 UTC continuation)

Selected from the existing program: the strict reader for the frozen `abbey-cbor-episode-v1` domain. Owned implementation paths are `crates/abi-wdbx/src/v3/commitment.rs`, new `crates/abi-wdbx/src/v3/commitment/decode.rs`, and new `crates/abi-wdbx/tests/v3_canonical_decode*.rs`; its separate Python witness follows after Rust review. No new competing program or consumer edits.

Acceptance: exact golden reencoding/digests; shortest encodings and full input consumption; bounded allocation, depth and values; compound keys; preserved duplicate sorted parents; content-free malformed-input rejection. Baseline evidence is the 647-test source gate above. Source implementation and independent review are still pending at this checkpoint's creation.

Subsequent work stays uncommitted for this handoff, with exact path/content hashes in the execution ledger and frozen review packages. Earlier commits/history remain intact. No push, CI, dependency install, destructive operation or deployment is introduced. The reader adds syntax validation only: no wire schema, store migration, authorization, signature trust, efficacy or federation claim.

The separate [Zig/stable-Rust checkpoint](2026-10-20-wdbx-zig-to-stable-rust.md) records the root's existing 1.99.1 toolchain, retained historical oracles and the selected Studio alignment prerequisites. It introduces no consumer source update or extra implementation writer.
