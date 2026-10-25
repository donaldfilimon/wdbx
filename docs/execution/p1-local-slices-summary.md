# Private P1 local slices 2–4: owner-review supplement

2026-10-22. This supplements the
[owner-review packet](../completion/execution/owner-review.md) and the
[authoritative private design](../superpowers/specs/2026-10-20-wdbx-cose-trust-prototype-design.md).
It records locally implemented and independently reviewed private mechanisms.
**Named ABI-owner Slice 5 approval remains pending.** No normative schema,
public consumer interface, legacy domain or completion evidence level changes.
All 163 completion requirements remain open/C0.

## Reviewed local mechanisms

Slice 2 signs/verifies the exact detached full-payload P1 `Sig_structure` through
Rust's maintained strict pure-message Ed25519 API. The immutable report retains
original payload/container/protected bytes and profile/domain/version, contract,
scope, actor/authority, signer and verification-snapshot provenance. An unknown
key is Unverifiable; mathematical validity conveys no current admission right.
Public-key snapshot construction rejects weak/duplicate keys and overflow.
Independent automated Rust review accepted the bounded private implementation.
A Python backend-exception disclosure finding was reproduced, fixed and retested;
backend failures are static errors and fail closed.

The independent Python witness parses the closed P1 structure and reconstructs
the exact message. Maintained backend APIs perform every signature operation;
no application-written point arithmetic or automatic fallback is introduced.
The initial bounded paired matrix passed with the explicitly selected PyNaCl/
libsodium interpreter (24 Rust and 12 Python tests at that recorded snapshot).
Those counts do not describe a later combined source tree with added trust or
admission tests; exact final commands/results belong to its continuation receipt.

The available PyCA backend accepted a retained synthetic low-order-key signature
that Rust rejected. That failed qualification result remains evidence; the
explicit PyCA checker still fails on it. Selecting the separately reviewed
libsodium backend does not erase that disagreement. Neither a single backend
probe nor default Rust gate establishes cross-language parity.

Slice 3 adds [synthetic in-memory trust administration](../completion/execution/p1-trust-slice.md): exact
namespace/principal/scoped grants; authorized command replay; Pending/Active/
Retired/Revoked lifecycle; atomic registered-replacement rotation; bounded retained
public keys/history; checked revision; immutable trust snapshots distinct from
mathematical key snapshots. Current authorization is checked for new operations
and retries, including access to original history scopes. Twelve focused tests,
exact command/snapshot vectors and feature all-target Clippy passed. Independent
automated specification/quality review accepted this private slice without findings.

Slice 4 adds a separately reviewed pure admission evaluator and synthetic
in-memory acceptance simulator. It binds the immutable verification report to
expected identity, signer, scope, payload/container, current trust and explicit
host actor/policy/consent/writer evidence. It distinguishes eligible, rejected
and indeterminate decisions without granting a production append capability.
Both request and episode replay identities retain exact original bytes/receipts;
current receipt-read authorization precedes historical replay, and changed bytes
or signatures conflict. Commit simulation rechecks current revisions, denies
revoked/stale decisions and preserves state on refusals. Deleted originals and
nondeleted unavailable originals remain distinct refusals.

Independent automated review accepted Slice 4 after a missing-originals branch
finding was reproduced and fixed with a nondeleted-original-loss regression.
Twenty-one focused tests and feature Clippy passed for that reviewed snapshot.
This is a private simulator, not durable admission, real credential authentication,
trusted time, key destruction or producer/consumer adoption.

## Dependency and compatibility boundaries

[Initial dependency decision](../completion/execution/p1-dependency-decision.md) retains the observed
PyCA failure; [libsodium decision](../completion/execution/p1-libsodium-decision.md) records the independently
reviewed bounded wheel-only experiment. Exact host binary/metadata, locked graph,
wheel hashes and advisory query observations are retained. The isolated `/tmp`
interpreter is disposable experimental infrastructure, not a distributable product
dependency or all-platform qualification. No global package or bundled runtime was
changed. Hash agreement does not authenticate an upstream build provenance chain.

The existing v2 commitments, legacy v3 digest signatures and old COSE-digest
prototype remain separate. The private full-payload profile is not a retroactive
interpretation of any old signed record. Synthetic trust authorization is an
explicit host fixture; it performs no credential authentication, custody, remote
administration or filesystem persistence. No reviewed slice supplies an append
capability or establishes historical signing time.

## Requirement reconciliation after explicit final source stability

The following paths are reconciled as **partial candidate references** after
explicit final source stability and independent local admission review. Exact
hashes are recorded in the execution register. Existing frozen contracts and
historical receipts stay untouched.

| Existing requirements | Candidate references and retained limits |
| --- | --- |
| P02.02, C10–C11, E01 | `crypto/verify.rs`, its focused tests, and `crypto/trust/` types, encoder, transitions, snapshots and tests. These establish private integrity/trust mechanisms, not owner ratification or durable lifecycle. |
| P02.03 | `crypto/python_witness_tests.rs`, `tools/wdbx_complete_p1_crypto.py`, its unittest suite and `tools/check_p1_crypto.sh`; retain explicit backend selection and PyCA failure. |
| P02.02–P02.03 artifacts | The two dependency decisions, private design, trust slice review/status and reproducible command/snapshot vector builder/JSON. Dependency review and encoding vectors do not establish production cryptographic qualification. |
| P02.02, C11, E01 | Reviewed private `crypto/admission/` evaluator, simulator, tests/scenarios and typed SPEC. This supplies bounded synthetic current-authority/replay behavior, not persistent lifecycle or production authorization. |

Here `crypto/` denotes
`crates/abi-wdbx/src/v3/complete_episode/crypto/`. The independently pinned candidate
catalog and register hashes are updated together in the explicit final
reconciliation. The supplement supplies no evidence-level promotion.

## Owner decisions and remaining work

The named owner must still decide and approve the exact full-payload profile,
domain/AAD/header/key representation, purpose-specific keys, signature requirement,
trust/admin/receipt vocabulary, replay/privacy retention and compatibility/cutover.
Approval must identify the owner and reviewed schema/profile/source/vector/
dependency hashes. Independent production crypto review remains additional.

Durable single-writer ordering/recovery, real host policy/consent integration,
key custody and administration recovery, consumer adoption,
platform delivery and operational qualification remain separate requirements.
Local slice acceptance does not authorize publication, deployment or live-store use.
