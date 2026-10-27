# Private P1 admission simulator: proposed Slice 4

Synthetic, in memory only. This is not credential authentication, a public wire
contract, an append capability, trusted time, lifecycle policy, or persistence.
Implemented privately after independent Slice 3 review and root authorization;
Slice 4 acceptance still requires independent review and final gates.

## Private typed interfaces

- `evaluate_admission(report: &VerificationReport, context: &AdmissionContext,
  trust: &TrustSnapshot) -> AdmissionDecision` is pure and appends nothing.
- `AdmissionSimulator::new()` starts writer revision zero, with no records.
- `simulate_commit(&mut self, report: VerificationReport,
  decision: &AdmissionDecision, context: &AdmissionContext,
  trust: &TrustSnapshot) -> Result<SimulatedOutcome, AdmissionReason>` serializes
  rechecking and simulated acceptance; it stores only report-owned bytes.
- `SimulatedOutcome::{Accepted(SimulatedReceipt), HistoricalReplay(SimulatedReceipt)}`
  differentiates a new acceptance from a historical receipt without renewed
  authorization. Receipts have private fields and read-only accessors.
- `AdmissionDecision` has immutable private fields, no deserializer, no public
  constructor, and accessors for class/reason. Classes are Eligible, Rejected,
  and Indeterminate. Eligible includes every binding below and current trust
  digest/revision, policy/consent epochs, and writer revision. A decision is
  synthetic evidence, never a production commit capability.

`AdmissionContext` is a private fixture record containing:

- `expected: ExpectedAdmission`: exact profile, domain, version, contract digest,
  namespace, signer, full existing `Identity` (including scope/request/episode
  IDs and contract reference), payload digest and container digest.
- `expected_trust_revision`, `expected_policy_epoch`,
  `expected_consent_epoch`, and `expected_writer_revision`.
- `authority: Option<SyntheticHostAuthority>`: explicitly host-provisioned actor
  (`Actor` principal and kind), scope binding, exact local trust namespace, exact payload digest,
  canonical-writer principal (`OpaqueRef`), and independent `may_write` and
  `may_read_history` decisions. The signed source actor and canonical writer
  principal are distinct concepts; the host fixture associates them explicitly.
- `policy: Option<CurrentPolicy>`: host epoch, exact current policy and safety
  `VersionRef`s. Both must match the signed `Authority` references for new work.
- `consent: Option<CurrentConsent>` with explicit variants
  `NotRequired { epoch }` and `Required { epoch, policy: VersionRef }`.
  NotRequired is a host decision, never inferred from missing signed consent;
  v1 permits signed absence only and rejects signed presence in this branch.
  Required demands signed consent with matching current epoch, policy and scope.
  Missing outer host consent evidence is Indeterminate; no exemption is invented.
- `current_writer_revision` and `budget: HostBudget { max_records,
  max_retained_bytes }`. The simulator caps successful records at 4096 even if
  the host budget is larger; no successful identity is evicted. Tests may
  choose small budgets. Retained payload/container bytes count toward the host
  byte budget; no durable quota implementation is implied.

No transport JSON, serializer, owner error schema, or new normative vocabulary
is introduced. Reuse existing typed Actor/Identity/VersionRef/ConsentRef values.

## Ordering and binding invariants

Check all expected report bindings exactly before grant lookup; mismatch is
Rejected(VerificationBindingMismatch). Compare the authenticated fixture actor
and scope and bound payload digest with the signed report. Missing fixture
inputs yield Indeterminate(MissingAuthority); explicit denial yields Rejected.
The fixture constructor must be named for synthetic host policy; role strings
inside a payload never construct this authorization.

New acceptance requires Valid integrity, current matching trust namespace,
Active signer, exact canonical-writer principal and scoped grant. Unknown
verification material is Indeterminate(VerificationUnavailable); an Invalid
report is Rejected(InvalidVerification). Current policy/consent/writer and trust
revision checks are independent. Snapshot identity from mathematical verification
is retained as provenance; it cannot substitute for current trust authorization.
A key's pending/retired/revoked status or claimed backdated episode times never
permits new admission. Current compromise warning comes from trust revocation
metadata, not a claim that signing occurred before its claimed time.

Replay keys are `(binding, profile, request_id)` and `(binding, profile,
episode_id)`. Both indexes must agree on the same successful record. A fresh
scoped receipt-read authorization is required before historical result lookup.
Report mathematics must still be Valid (retired/revoked public keys remain
resolvable). Exact replay requires equal payload/container digests AND byte-identical original
payload/container. Return the original receipt before current write permission,
current signer status, or a now-lower quota could renew or invalidate a historical
write. Changed bytes/signature under either identity is ReplayConflict. A failed
new attempt reserves neither identity. No replay tombstone is evicted.

Simulation commit rechecks all eligible decision bindings and current revisions;
a revoke between evaluation and commit or another simulated write invalidates
an earlier decision. All mutations happen only after these checks and resource
checks; every refusal/indeterminate leaves writer revision, both indexes,
receipts, original bytes and usage unchanged. Checked revision overflow refuses.

A trusted synthetic host erasure entrypoint may install deleted/tombstoned or erased-original
states, increments the writer revision on the first erasure only, and drops
original payload/container bytes while retaining both indexes and digests. A tombstone refuses resurrection even with a fresh valid signature.
Erased original bytes yield MissingOriginalBytes rather than falsely confirming
byte-identical replay; a retained receipt may still be reported as historical
local simulation evidence. No fresh verification is reconstructed from a digest.
There is no real erasure, retention decision, or deletion privacy policy here.

## Acceptance tests

- Full expected binding substitution matrix, signed actor principal/kind and
  scope mismatch; invalid/unverifiable mathematical reports; missing authority,
  policy and consent evidence; explicit denial; correct scope and grant.
- Current policy/safety and required consent mismatch, stale trust/policy/consent/
  writer revisions; ordinary acceptance and original bytes retained.
- Exact replay after retire/revoke, unauthorized replay, changed signature under
  the same request, request and episode collisions, rejection reserves no ID.
- Revoke and competing-write interleavings after evaluation; no partial mutation.
- Zero/one/4096 budgets and byte ceilings, no eviction, overflow refusal.
- Backdated claimed times do not authorize; compromise warnings stay separate;
  deleted or erased-original snapshot cannot resurrect a record.
- Static redacted diagnostics and no append on any Rejected/Indeterminate result.

This file is a private implementation proposal. The specification's Slice 5
owner gate still precedes durable integration.

## Implemented closed prototype choices

Simulation requires fresh scoped receipt-read authority before consulting either
idempotency index, including new-write attempts; it does not reveal whether an
unreadable identity exists. Historical lookup additionally requires the supplied
current trust namespace and original receipt bindings to match the report.
Authorized changed bytes under an existing identity are a conflict before the
Valid-mathematics requirement for an exact historical return; no conflict returns
a receipt or appends anything. Current compromise metadata is read only from the
same namespace as the report.

`simulate_host_erasure(expected, expected_writer_revision)` is an explicitly
trusted synthetic fixture operation, not a lifecycle authorization mechanism.
It drops only this simulator's owned byte copies, increments its revision once,
and retains identities/tombstones. External caller copies and allocations are
outside its erasure model; no cryptographic deletion or filesystem effect follows.

The 4096 boundary test admits 4096 individually signed and strictly verified
synthetic records with distinct request/episode IDs before checking refusal and
unchanged state. No artificial populated-index state stands in for that test.
Input-byte bounds and count limits do not bound all heap usage or guarantee OOM
recovery. All host policy/consent epochs are trusted synthetic inputs; there is no
operational authentication, policy reload, concurrency service, clock attestation,
key custody, durable atomicity, power-loss evidence or public wire surface here.

`simulate_host_originals_unavailable` is a separate trusted synthetic fault
transition. It drops owned originals while keeping `deleted=false`, both indexes,
digests and the original receipt. Its first transition advances the writer
revision; repeating it is inert. It cannot reverse a deletion. A subsequent valid
exact-byte candidate returns MissingOriginalBytes without reconstructing originals,
returning an exact historical receipt, mutating state, or appending. This models
unavailable evidence only; it is not corruption recovery or a real erasure API.
