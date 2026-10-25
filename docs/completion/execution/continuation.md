# Local continuation — 2026-10-22

Continues the accepted full completion plan without commits or deployment. The
previous tranche's dirty work is preserved. Historical receipts and raw logs are
retained unchanged; their source hashes identify their own snapshots, not this
continuation's new bytes. All frozen requirements remain open/C0.

## Requirement reconciliation

All 163 IDs now have inspected, typed dispositions: 113 partial source/test
candidates, 34 local gaps and 16 external ownership/platform/operational gaps.
Every ID binds a reason, inspection references and the frozen qualification plan.
No adjacent code is mistaken for direct acceptance coverage, and an external gap
does not assert that no local code exists. The independently pinned catalog
protects dispositions and reference identities. Independent automated review
found no correction. Twenty-seven mutation tests passed on both Python versions
before crypto-source hash refresh; final checks follow once the source stabilizes.

## Private P1 signing/verification and independent witness

The private Rust verifier now signs and strictly verifies the exact full-payload
P1 signature structure. It retains immutable original bytes, full identity,
contract/scope/signer bindings and verification snapshot namespace/revision/digest.
Unknown keys are Unverifiable. Key snapshots reject weak keys, duplicates and
capacity overflow. Neither a valid report nor a snapshot grants admission.

Exact dependency decisions and host-specific binary/metadata/graph observations
are in `p1-dependency-decision.md` and `p1-libsodium-decision.md`. Independent
automated dependency review accepted bounded experimental implementation only.
The available bundled PyCA backend accepts a synthetic low-order-key signature
that Rust strict verification refuses. This failed backend result is retained.

An independently reviewed, wheel-only hash-locked PyNaCl 1.6.2 installation was
created in a fresh `/tmp` virtual environment using bundled Python 3.12.14. No
global or bundled-runtime package was modified. Its native libsodium backend
rejects that vector. An explicit backend/interpreter is required; there is no
automatic fallback. The isolated runtime is disposable, not a distributable
product dependency or installed-platform qualification.

The Python witness independently parses P1 and constructs the signature structure;
all cryptography is delegated to maintained PyCA or PyNaCl/native APIs. Native
Ed25519ph generation/verification reproduces a synthetic fixture, which both
pure-message witnesses and Rust refuse as P1. Tests cover independent positive
signatures, original bytes, header/key/origin binding, malformed points/scalars,
message/AAD changes and legacy/digest transplants. Backend failures produce static
errors and exit 2, never a successful negative-signature result or traceback.

The opt-in `p1-crypto-witness` Cargo feature and `tools/check_p1_crypto.sh` are
separate from the standard-library default gate. PyCA's explicitly invoked checker
retains exit 1 on the known disagreement. The libsodium checker reports only a
bounded unratified differential matrix after the Python and paired Rust suites.
A single backend probe never qualifies parity. Root-gate green alone supplies
none of this independent cryptographic witness evidence.

Independent automated source review found no functional Rust defect. A Python
error-contract defect received failing injected-error tests and a fix. Twelve
Python tests passed per explicit backend and twenty-four Rust crypto-feature
tests passed with libsodium, including three bidirectional synthetic positives.
These are reviewed local implementation results, not independent cryptographic
qualification, general backend equivalence or ABI owner ratification.

## Next local stages and boundaries

The reviewed bounded Slice 2 permits private in-memory Slice 3 trust-model work.
No filesystem trust history, live credentials, remote authority, consumer cutover
or normative ABI schema is introduced. Subsequent admission simulation must keep
original bytes, current grants, replay identity and revision bindings separate.
Durable integration still requires the design's named owner gate and reviewed
ordering/privacy/recovery contract. Research, real platform hosts and elapsed
operation remain separate requirements.

## Private in-memory trust Slice 3

The bounded trust model implements Pending/Active/Retired/Revoked transitions,
atomic rotation to a registered Pending replacement, terminal revocation and
immutable current snapshots. It binds command namespace, ID, administrator,
expected revision and every operation argument into canonical bytes and a digest.
Current scoped host-fixture authorization is checked before successful-history
lookup and again against the original command's affected scopes. Exact authorized
replays return historical receipts without renewed authority or mutation; conflicts
and failed commands reserve no identity. Historical public keys remain available
for mathematics while current permissions are Active-only.

The model enforces 1,024 keys, 64 grants per key, 4,096 successful commands,
checked revision arithmetic and no eviction. Immutable trust snapshots bind full
metadata and ordered successful history; verification key snapshots remain a
separate mathematical object. Twelve focused tests and independent Python
command/snapshot vectors passed, along with all-target feature Clippy. Independent
automated review accepted this local slice with no actionable defect. There is
no filesystem state, credential authentication, remote authority or admission.
The private admission simulator below consumes this reviewed interface.


## Private admission Slice 4

The evaluator binds every immutable report identity, profile, contract, namespace,
signer and original-byte digest to an explicit synthetic host context. Current
Active grants, policy/safety references, consent state and writer budgets remain
separate from historical signature mathematics. Missing current authority or
consent information never becomes implicit permission. A current snapshot in a
foreign namespace cannot authorize either admission or receipt lookup.

The bounded in-memory simulator rechecks the decision against current revisions
before accepting the report-owned bytes. It preserves both request and episode
indexes without eviction, rejects changed bytes/signatures under reused IDs, and
checks authorized scoped reads before historical lookup. Exact historical replay
requires valid mathematics and original byte equality; it does not renew current
write authority. Capacity, quota and checked-revision failures mutate nothing.

Synthetic deletion and a separate originals-unavailable fault transition drop
owned payload/container bytes while retaining receipt identity. A retry returns
`Deleted` or `MissingOriginalBytes`, respectively, without reconstruction or
resurrection. These are model fixtures, not a real lifecycle authorization or
physical-erasure service. No filesystem append, credentials, network authority or
production admission path is introduced.

Independent automated specification/quality review found one acceptance gap:
missing originals had no distinct exercised transition. A failing regression
preceded its fix. The reviewer independently reran all 21 admission tests,
including 4,096 individually signed and verified unique admissions, and accepted
this bounded slice with no remaining functional finding. All-target feature
Clippy passed. The named ABI-owner gate still precedes durable integration.

## Final local verification

With pinned Rust 1.99.1 and its toolchain first on PATH, the final root gate passed:
1,136 Rust tests/doctests, 52 complete-decode Python tests and 28 COSE-encoding
Python tests. Feature all-target workspace Clippy also passed. These gates cover
the substrate and private local models, not Studio, consumers, optional FHE,
installed platforms or elapsed operations.

The final explicit libsodium checker passed 57 Rust crypto tests and 12 Python
witness tests. The final explicit PyCA run passed its same paired test matrix,
then deliberately exited 1 at the retained raw weak-key disagreement probe.
That result remains a failure of qualification, not a suppressed failure or a
fallback selection. Exact interpreter identities and dependency observations are
retained alongside their decisions.

The frozen completion checker and its 37 mutation tests passed: 163 C0
requirements and 46 acceptance cases. The separate 3-D inventory validator and
16 evidence-validator tests passed; this establishes inventory structure, not
new spatial product evidence. Final execution-register checks are recorded in
`evidence/2026-10-22-continuation/register/`.

Raw logs retain earlier failing attempts as well as final outcomes. In particular,
`p1-slices-2-3.log` is a failed compilation against a moving TDD source tree;
it is not reused as a passing result. Final logs, automated review scope and a
SHA-256 manifest are under `evidence/2026-10-22-continuation/`. The manifest pins
uncommitted file bytes; the baseline Git revision alone does not identify them.
No commit, deployment, public ingress, live-store test or evidence promotion
was performed.

The final execution register and pinned catalog passed independent refresh review.
All 27 mutation tests passed on both Python 3.15 and system Python 3.9, and both
completion checkers passed. The review confirmed the 113/34/16 dispositions and
current source hashes without promoting any whole-requirement claim.
