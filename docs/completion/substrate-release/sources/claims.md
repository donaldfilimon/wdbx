# WDBX Claim Ledger

**Claim ledger is [`README.md:44`](https://github.com/donaldfilimon/wdbx/blob/main/README.md#L44).**

**Detailed gap analysis at [`abi/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md`](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md).**

---

## Evidence Levels (Constitution C0–C7)

| Level | Description | WDBX Coverage |
|-------|-------------|---------------|
| C0 | Source conforms under test | ✅ `cargo test --workspace` passes |
| C1 | Source evidence + local deterministic replay | ✅ Golden fixtures, episode-store replay tests, and a deterministic local baseline comparison over a frozen synthetic corpus |
| C2 | Cross-language canonicalization | 🟡 Partial: a standard-library Python reimplementation (`tools/abbey_cbor_episode_v1.py`) agrees with Rust on the `abbey-cbor-episode-v1` golden vectors and a differential corpus (`tests/v3_cross_language_commitment.rs`), and re-derives every receipt digest a real `EpisodeStore` appends, all seven event variants with chained parents, from the JSON wire form (`tests/v3_cross_language_episode.rs`), including the two pinned memory goldens. Encoding and digest derivation only; that witness covers no COSE container (a separate, encoding-only COSE witness is described below), no second-language admission rules and no signature verifier |
| C3 | Live provider / Discord evidence | ❌ Not claimed |
| C4 | Hosted service / federation evidence | ❌ Not claimed |
| C5 | Production deployment evidence | ❌ Not claimed |
| C6 | Operator witnessed exact outcome | ❌ Not claimed: `VerifyEpisode` exists on the ABI gateway, but no operator-witnessed outcome is recorded |
| C7 | Reconstructible experiment manifest | ❌ Partial (`abi-telemetry` exists but incomplete) |

## Current Claim Boundary

> This codebase implements most of the **structural** half of the substrate and little of the **evidence** half.

Closing the evidence half is Program 4: `canonical-wdbx-episodes-claims`.
**Per-episode signatures (2026-09-16; amended 2026-10-21): C0/C1 only.** The
v3 episode store can sign each episode digest with Ed25519
(`src/v3/episode/signing.rs`), and replay rejects a tampered signature under
the writer's own key. This is a detached Ed25519 signature over a canonical
digest, verified in Rust tests. The shipped signature is **not** COSE, is not
verified by any other language, and carries no key rotation or revocation.

A private crate-internal `COSE_Sign1` prototype now exists
(`src/v3/episode/cose/`): a bounded tag-18 container whose RFC 9052
`Sig_structure` Ed25519 signature covers the same 32-byte digest, an
`Integrity` axis (Valid/Invalid/Unsigned/Unverifiable) kept apart from the key
axis, and an in-memory `TrustList` with Pending/Active/Retired/Revoked, a
revision counter and terminal revocation, where retired and revoked keys still
resolve for historical verification but never authorize a new admission. A
standard-library-only Python witness (`tools/wdbx_cose_sign1_p1.py`, driven by
`tests/v3_cross_language_cose.rs`) adds *encoding agreement* across two
languages: the same container bytes must parse to the same protected header,
key id, algorithm, payload and RFC 9052 `Sig_structure` bytes, and the same
malformed containers must be refused with the same error names. The Python side
does not verify signatures and cannot — this environment has no Python
cryptography library, and the design guidance forbids hand-rolling
cryptography in Python — so the witness covers bytes only, and the Ed25519
checks in the Rust test are Rust-side checks. It is a prototype at C0/C1
(`tests/v3_episode_cose.rs`, `tests/golden/cose-sign1-p1/`): unratified, with no
cross-language verifier, no production key custody, no admission or
authorization semantics and no independent cryptographic review. It changes no
committed digest, and the detached `EpisodeSignature` ledger format is
unchanged. C2 above remains partial for the canonical *encoding* only
(2026-09-16: the Python witness), never for signatures.

**Supersession lineage (2026-10-21): C0/C1 only.** `EpisodeStore::superseded_by`
and `EpisodeStore::supersession_lineage` (`src/v3/episode/store/lineage.rs`)
report the committed `supersedes` edges as content-free digests and flags, and
`tests/v3_supersession_lineage.rs` pins them. Both maps are rebuilt in
`apply_record`, so live writes and replay-on-open answer alike. This is a
projection of edges the ledger already holds: it establishes no holds and no
revocation of its own. Holds and candidate revocation are a separate slice
(`src/v3/episode/store/holds/`); redaction, object-key destruction and
garbage collection are a separate slice (`src/v3/episode/store/deletion.rs`,
`src/v3/episode/store/gc.rs`).

**Structured admission decision (2026-10-21): C0/C1 only.**
`src/v3/episode/admission.rs` defines `AdmissionClass` (the closed
five-class retention vocabulary), `AdmissionReason` (exactly one reason per
`EpisodeStoreError` variant, plus `Accepted`) and a content-free
`AdmissionDecision`. `EpisodeStore::decide_write`
(`src/v3/episode/store/admission_gate.rs`) runs the same validation, the same
commitment check and the same quota checks as `propose_write` and appends
nothing, and `EpisodeStore::decide_write_outcome` answers the same question
in one content-free value that never fails, carrying the class and the reason
(`Accepted`, or the reason the refusal maps to), so a caller never has to
match on an error string. The three budget checks were extracted into
`EpisodeStore::check_quotas` (`src/v3/episode/store/validate.rs`) and are now
used by `propose_write`, `decide_write` **and** `preview_commitment`, which
closes a previously inconsistent preview path in which a preview could succeed
for a write the append later refused on budget; `check_quotas` also charges
the retention sidecar's own size ceiling, and the decision and the preview
measure the signed line the append writes, so all three paths reach the same
verdict. `EpisodeStoreError` and its
labels are unchanged, so the ABI gateway's status mapping is unaffected, and
`tests/v3_admission_decision.rs` pins that the decision and the append always
agree. The decision runs the same retention gate as the append path — it
refuses a zero window — and it reads the same effective budgets as the
append path.

**Quota administration (2026-10-21): C0/C1 only.**
`src/v3/episode/store/quota/` adds a durable, content-free, auditable quota
override. A guild's budgets still come from the host policy, and an authority
may adjust them at runtime: `EpisodeStore::decide_quota_adjustment` is the
non-mutating decision, `EpisodeStore::commit_quota_adjustment` commits through
the single writer, `EpisodeStore::clear_quota_adjustment` lifts an adjustment,
`EpisodeStore::quota_state` reports the effective budgets, the usage and
whether the usage is above the effective budget (`over_budget`), and
`EpisodeStore::quota_history` reports every decision the sidecar holds for a
guild, in order. The effective budget is the host policy's budget for that
guild, lowered to a recorded adjustment's budget only while the policy still
states budgets at or below the ones in force when the adjustment was made, and
never above the policy's budget. The host policy is therefore the outer
bound: no adjustment may set a budget above it, whoever makes it, and the
host restores a larger budget by changing the policy or by clearing the
override. Only an authority that may raise a budget may clear one, so a
writer can never freeze a guild's budget permanently; a budget is a *guild*
budget, so a service that lowers it restricts every writer in that guild, not
only itself. The effective budget is what `check_quotas` reads, so an
adjustment changes what the store accepts and the change is visible in
`decide_write`, `preview_commitment` and `propose_write` alike. The
adjustment is a governance decision rather than an episode, so it lives in a
derived sidecar beside the ledger, `episodes.v1.quotas.jsonl`, in the
established shape: content-free lines (guild, the two budgets, the policy's
budgets in force when the decision was made, the authority, the instant), a
checksum over each line's own fields, and a header line naming the number of
decisions and a checksum over all of them. The header is what makes a
truncation detectable, and a truncated file, a trailing fragment and a bad
checksum are all refused as corrupt. The file is created with the first
adjustment rather than with the store, every write replaces it atomically,
and it is written by the same single writer; it survives close and reopen. A
guild owner, administrator or manager and an organization owner may adjust a
budget in either direction; a service may only lower one, because a writer
that could raise its own budget could defeat the budget; a human subject may
not adjust a budget at all. That vocabulary is *actor-class* based, and the
class rule is only the first of two necessary conditions: the guild-scoped
authority map below decides whether the principal may act for the target
guild at all. An adjustment may never lower a
budget below the usage the guild has already recorded, because that would
retroactively invalidate writes the ledger already holds; such an adjustment
is refused. The same state is reachable without the adjustment API, when a
host policy is lowered below the recorded usage; the store *surfaces* it
through `quota_state`'s `over_budget` instead of refusing to open, because
refusing would brick a store whose policy was lowered.
`EpisodeStoreError` and its labels are unchanged. `tests/v3_quota_windows.rs`
pins it. This is C1 source and local deterministic test evidence: it
establishes no per-caller or per-class budget, and no quota administration
that is itself an episode — that needs a new event variant and a coordinated
regeneration of every pinned golden.

**Guild-scoped authority map (2026-10-21): C0/C1 only.**
`GuildEpisodePolicy::authorized_principals` is an optional, `#[serde(default)]`
list of the principals that may act as authorities *for that guild*, and the
store reads it in `src/v3/episode/store/authority.rs`. It is host policy and
not episode material: the canonical commitment binds the policy *version*,
never the policy itself, so naming a principal moves no committed digest, the
frozen v3 record format is untouched, and `ActorRef` keeps its two fields.
The rule has two halves and both are necessary. The guild's own map: when a
guild names a non-empty list, only a principal on that list may place a hold,
release a hold, revoke a candidate, adjust a budget, lift an adjustment, or
order a destruction for that guild, whatever class it carries. The store-wide
scope: a principal that *some* guild names is scoped to the guilds that name
it and holds no class-only authority anywhere else — without that, naming the
owner of one guild would leave that owner free to act for every other guild,
which is the hole the map exists to close. What the map closes, for the
principals the host names, is that hole; a principal that no guild names is
not closed by the map at all and still reaches every guild whose policy states
no map of its own, on the class rule alone. The actor *class* remains a
second, necessary condition: a service may never place a hold, revoke a
candidate, raise a budget, or order a destruction, even when the map names it,
and a human subject is never an authority.
The three states of the map are pinned: an absent map leaves the guild to the
class rule alone (a compatibility behaviour, not a security guarantee — a
host that wants a guild closed to unknown principals has to name them), a
non-empty map constrains the guild to the principals it names, and an empty
map is the same state as an absent map, so an empty list is not a closed
guild. `EpisodeStore::actor_may_act(guild_ref, actor)` is the store-side
predicate, and it is the guild-scoping half only. Every refusal answers the
same closed, content-free `EpisodeStoreError::InvalidTransition`, which names
neither the principal nor the guild, so a refusal is indistinguishable between
"not authorized for this guild" and "no such principal".
`EpisodeStoreError` and its labels are unchanged. `tests/v3_guild_authority.rs`
pins it. This is C1 source and local deterministic test evidence. The honest
limit: the substrate still cannot *verify* that a principal really holds a
role; the host policy is the authority of record, and the store trusts the
caller to present the right principal and the right class. The map does not
constrain the canonical *write* path. The map does not reach the lifecycle
`EpisodeEvent::Approval` (`src/v3/episode/store/validate.rs`), whose approver
is class-checked (a valid actor, not a service, not the requester and not the
proposer) but is *not* guild-scoped — no guild check is applied to it, so a
principal labelled `GuildOwner` for guild A may approve an operation in guild
B. That is a missing *guild* check, not a contradiction of the class rule: an
approval is a **workflow** step, not a governance action over a guild's
resources — it accepts one proposal and confers no authority over anything
else — so the rule that a human subject is never an *authority*, a rule about
governance authorities, does not reach it, and a `HumanSubject` is an
admissible approver there by design; what the store does not do is verify that
the approver holds any standing in the guild of the operation it approves. The
map does reach the deletion lifecycle's destruction requests
(`src/v3/episode/store/deletion.rs`): `RedactionRequest` and
`ObjectKeyDestructionRequest` carry an `actor`, every destruction order is
refused unless the principal is one the guild authorizes *and* the class is
one that may destroy (a guild owner, administrator, manager, or an
organization owner — never a service, never a human subject), and the sidecar
lines record the principal, so a destruction order is attributable. The map is
not committed, signed, or replayed: it is host policy, and changing it changes
what the store accepts from the next open onwards — the store binds the policy
at open and never reloads it.

**Per-class retention windows (2026-10-21): C0/C1 only.**
`GuildEpisodePolicy::retention_windows` is an optional per-class map
(`session_ttl_ms`, `operational_ttl_ms`, `durable_ttl_ms`), `#[serde(default)]`,
so a host policy that omits it still parses and still means the previous
behaviour. The precedence is fixed and documented: an explicit per-class
window wins over the single `retention_ttl_ms`, the single TTL remains the
fallback, a class with neither never expires, and `durable` candidates never
expire whatever the policy says. The write gate refuses a zero window — a
single zero TTL or a per-class zero — exactly as before.
`tests/v3_retention_windows.rs` pins it. This is C1 source and local deterministic
test evidence. The window is chosen by the *policy*, never by the candidate: a
candidate-selected window would need a digest-bound field on `MemoryCandidate`,
or a new event variant, and a coordinated regeneration of every pinned golden
and of the cross-language Python witness; that remains the open item.

**Deletion lifecycle (2026-10-21): C0/C1 only.** `src/v3/episode/store/deletion.rs`
and `src/v3/episode/store/gc.rs` add the redaction of derived copies, the
object-key destruction request, and the garbage collection of expired and
forgotten records. `DerivedCopyKind` is a closed, content-free vocabulary;
`EpisodeStore::decide_redaction` and `EpisodeStore::commit_redaction` record a
redaction request for a *forgotten* target in `episodes.v1.redaction.jsonl`,
and `EpisodeStore::redaction_state` reports the kinds a target has.
`ObjectKeyRef` is the v3 episode namespace's own bounded tenant/key reference,
not the complete-episode prototype's private type; the destruction requests
live in `episodes.v1.object_keys.jsonl`. A request naming a live record is
refused, and the refusal does not distinguish a live target from an unknown
digest, so the store is not an existence oracle. Both requests carry the
`actor` that issues them: the store refuses the order unless the guild's
authority map names that principal and the class is one that may destroy — a
guild owner, administrator, manager, or an organization owner, never a service
and never a human subject — and the sidecar lines record the principal, so a
destruction order is attributable (`tests/v3_deletion_authority.rs`). Each
sidecar is a header line plus one line per request with a checksum over the
request lines, so a truncated or mis-checksummed sidecar is refused as
`Corrupt` rather than believed. `EpisodeStore::gc_plan` is
non-mutating and separates the records that may be reclaimed from the
tombstones that are retained forever; `EpisodeStore::apply_gc_plan` rewrites
the ledger without the reclaimable records, keeping every surviving record's
digest byte for byte (the canonical digest never covers the ledger
`sequence`) and keeping every tombstone. The reclaimed index
(`episodes.v1.reclaimed.jsonl`) is content-free, is written before the rewrite
it describes, and is what lets the ledger's tombstones still replay after
their record is gone. The rewrite writes a new ledger beside the old one and
renames it over, so an interrupted collection leaves the original ledger
intact and openable. `tests/v3_deletion_lifecycle.rs` pins it. This is C1
source and local deterministic test evidence: it establishes no destruction of
the consumer's own derived copies or object keys, and no candidate-selected
retention window.

**Holds and candidate revocation (2026-10-21): C0/C1 only.**
`src/v3/episode/store/holds/` adds a hold and a revocation over memory
candidates. A hold freezes a candidate: while it is live, the candidate is not
expired away (`store/expiry.rs`), not reclaimed (`store/gc.rs`), not silently
superseded (`store.rs`), and its payload and derived copies are not destroyed
(`store/deletion.rs`). `HoldKind` is a closed, content-free vocabulary;
`EpisodeStore::decide_hold`, `EpisodeStore::commit_hold`,
`EpisodeStore::release_hold` and `EpisodeStore::hold_state` report it, and only
the placing authority, or one above it in the documented governance order, may
release a hold. A revocation withdraws a candidate by authority:
`GuildMemories::is_live` reports it as not live, so no later `supersedes` may
name it and it is never resurrected, while its record and its anti-resurrection
metadata are retained, because a revocation is a withdrawal from use and not an
erasure. `EpisodeStore::decide_revocation`, `EpisodeStore::commit_revocation`
and `EpisodeStore::revocation_state` report it, and only a guild owner,
administrator, manager or organization owner may revoke. Both decisions live in
content-free derived sidecars beside the ledger
(`episodes.v1.holds.jsonl`, `episodes.v1.revocations.jsonl`), each line
checksummed over its own fields and written by the same single writer; they are
sidecars and not new `EpisodeEvent` variants because a field on
`MemoryCandidate` or a new event variant would change the committed digests and
break the pinned goldens and the Python witness. `tests/v3_holds_revocation.rs`
pins it. This is C1 source and local deterministic test evidence: the
constitutional query now reports both states — its lifecycle dimension carries
the hold state and the revocation state, and `Disposition` has `Held` and
`Revoked` members for them — while the store still holds no derived copy and
destroys none.

**Constitutional query (2026-10-21): C0/C1 only.**
`EpisodeStore::query` (`src/v3/episode/store/query/`) returns the eight
evidence dimensions (E01-E08) as separate fields, each with its own
content-free reason, alongside a closed `Disposition`, the record's lineage,
an `Explanation` and a sanitized receipt. It never collapses the eight into
one number: the module's source declares no floating-point value and no
combined number, and `MemoryResults` carries per-disposition counts only. The
query is guild-scoped, member-scope partitioned and bounded by `MAX_RECEIPTS`,
and it omits nothing: a record whose cryptographic evidence fails is listed as
`rejected` rather than dropped, and a forgotten record is listed as `rejected`
when the caller asks for erased memories (by default it is excluded, because
the tombstone means the content is gone). Dimensions the ledger does not hold
report `Unknown` rather than a fabricated zero. The lifecycle dimension also
reports the hold state and the revocation state, and `Disposition` has `Held`
and `Revoked` members for them.
`tests/v3_constitutional_query.rs` and `tests/v3_constitutional_query_holds.rs`
pin it. This is a read model over
the ledger: it is not wired into the v2 retrieval scoring seam
(`src/temporal.rs`'s `ScoreComponents::combined()` still collapses four factors
multiplicatively), it resolves signing keys
only through a caller-supplied resolver (there is no production key trust),
and it is neither cross-language verified nor an ABI-facing wire contract.

**Second disposition path (2026-10-21): C0/C1 only.** The episodes contract's
section 3 states two disposition rules that differ by kind: a
cryptographically invalid record is rejected, and a validly signed but
semantically uncertain record is retained, inspectable, carrying a quarantine
status or a contradiction edge. The substrate now implements that second path
in the constitutional query read model (`src/v3/episode/store/query/`):
`Disposition` is a closed vocabulary that keeps the two apart. An invalid
signature or a forgotten record reads `Rejected`; a record whose content is
doubted reads `Quarantined`, `Contradicted` or `Uncertain` instead of being
dropped. The query admits those records by default — `QueryConstraints`
includes contradicted and quarantined records unless the caller says otherwise
— and the doubt stays inspectable: the lifecycle dimension carries the open
quarantine edge, the outcome dimension the contradiction, and the lineage the
open contradiction's counterpart. `tests/v3_constitutional_query.rs` pins the
rejected, quarantined and contradicted cases. Two honest limits remain. The
contract's "at reduced retrieval weight" half is **not** implemented: the
substrate refuses to collapse the eight dimensions into one number, and the
query is not wired into the v2 retrieval scoring seam. And the semantic
members take precedence over an unverifiable signature, so an unverifiable
record a quarantine edge names reads `Quarantined` rather than `Unknown`; an
unsigned record with nothing else against it reads `Accepted`, with its
cryptographic dimension reporting `Unsigned`.

**Frozen retrieval baseline comparison (2026-10-21): C1 only, over synthetic
data.** `src/evaluation/` compares a plain-vector arm against the
constitutional query over one frozen synthetic corpus (12 query families, 160
records, synthetic tenant) with frozen embeddings, frozen storage and a fixed
candidate count of 12 before the shared filter. On that corpus the
repeated-failure rate and the stale-policy reuse rate both fell from
**0.666667 (8/12)** to **0.000000 (0/12)** — a relative reduction of 1.0
against a frozen threshold of 0.25, with the paired family-cluster
bootstrap's lower 95% bound strictly positive at **0.416667**. The harness is
able to report "no benefit": suppressing every dimension turns the
constitutional arm back into the plain vector arm exactly, and that is
asserted. The run also carries a negative result, which the verdict keeps: the
constitutional arm handed back an *unattributed* (unsigned) record in **7 of
12** families against **1 of 12** for the plain vector arm, because the
canonical query accepts an unsigned record on its commitment;
`applicability_accuracy` moved only from 0.250000 to 0.416667, and the
criterion `unsigned_reuse_not_worse` is `false`. This is a deterministic local
comparison over synthetic data: it is **not** a research result, **not** a
production claim, and **not** evidence that evidence-weighted retrieval beats
a vector database in production. The preregistered protocol in
`docs/completion/experiments.json` is unmet, no independent runner holds the
test split, and requirement `P04.03` stays `proposed` with an empty evidence
array. The receipt is
`docs/superpowers/plans/2026-10-21-retrieval-baseline-receipt.md`.

**Vector binding index and vector adapter (2026-10-21): C0/C1 only.**
`src/v3/ranking/vector.rs` (with `vector/format.rs` for the file format and
`vector/lock.rs` for the writer exclusion) and `src/v3/ranking/adapter.rs`
add the substrate-side binding between a vector index's identifiers and the
episode digests the constitutional query answers, and the adapter that uses
it. `VectorBindingIndex` is a derived sidecar beside the ledger
(`episodes.v1.vectors.jsonl`). Its record shape matches the episode store's
own sidecars — one line per binding carrying the scope, the vector identifier
and the episode digest and nothing else, a checksum over the line's own
fields, and a whole-file atomic replacement on every write — and its
writer-exclusion discipline now matches them too: an owner file
(`episodes.v1.vectors.lock`) and a lock held for the index's lifetime, so a
second writer is refused as `VectorBindingError::WriterBusy` instead of
racing, and every write re-reads and merges the file on disk, so a binding
another writer added is never lost. The file carries a header (format
version, record count, checksum over the record set), so a truncation is
detected: the index is replaced whole, so a trailing fragment is damage and
not a crash tail, and a truncation at a line boundary is caught by the header;
either is refused as corrupt, never a shorter index. The index is
scope-partitioned — the scope is part of the line and of the lookup key — and
a binding is refused a digest the scope's constitutional query did not admit
(`AdmittedScope`, built only from the query's own answer, whose scope the
answer now carries). That refusal is a check against the *caller's* admission
set: the substrate trusts the caller to hand over the query's own answer and
does not independently prove admission. `HnswVectorSource` (approximate, with
an explicit pinned seed) and `ExactVectorSource` (the deterministic
correctness oracle) both resolve a slot to an episode digest through the
index; a slot no binding resolves is counted, never invented into the
ranking; and `rank_with_vector_source` is the end-to-end path. The adapter
says what it drops: an HNSW search is approximate and may miss a nearest
neighbour, the binding index is separate from the ledger, a vector's payload
is never held, an unbound vector's episode is never named, and an asset
digest is never held at all. `tests/v3_vector_adapter.rs` pins it. This is C1
source and local deterministic test evidence. Still open: latency or memory
measurement, a cross-language witness for the binding sidecar, an append-only
binding log, a writer-exclusion fence against an actor who rewrites the file
wholesale (the lock is advisory, as everywhere else in the repository), a
vector arm over several scopes in one index, cross-language verification, and
an ABI-facing wire contract.

**Offline outbox (2026-10-21): C0/C1 only.** `src/v3/offline/` adds a
durable, single-writer queue of PROPOSALS made while a scope was offline. It
is not a canonical episode writer and must never become one: it stores the
write the canonical store would compute its own commitment from, it refuses a
write that carries a predicted commitment, and it holds no canonical episode
digest. Reauthorization is a separate, explicit step — `drain` takes the
CURRENT policy and the canonical store and answers a per-operation
`ReauthorizeOutcome`, running the store's own non-mutating decision rather
than re-implementing the write gate. No operation is dropped by the outbox's
own logic: a refused or conflicted operation keeps its proposal, a terminal
status and a reason, and only an explicitly authorized caller (guild manager,
administrator, owner, or the organization owner) may requeue, discard, or
remove it; the removal is status-independent (`Resolution::Remove`), so an
acknowledged operation is not immortal and a long-lived client can make room,
and a full outbox refuses new work with `OutboxError::Capacity`, which is
distinct from an invalid input. The merge is deterministic over a small
closed set of mergeable configuration objects and never invents a winner. An
asset operation carries a digest, a length and a chunk size but never the
bytes; the asset digest is numbered by chunk index, so the order a caller
hands its chunks over in never changes it and a correctly transferred asset
is never reported as corrupted. There is no fixed offline expiry. The file is
`outbox.v1.jsonl` beside `outbox.v1.lock`: a header line naming the
operation count and a checksum over the record set, then one checksummed line
per operation, and every write replaces the file atomically; a truncated
file, a trailing fragment and a bad checksum are all refused as corrupt, so
the outbox never opens with fewer operations than were queued, and nothing is
discarded by its own logic. `tests/v3_offline_outbox.rs` pins it. This is C1
source and local deterministic test evidence. Still open: a transport or a
server (the caller supplies an `OperationSink`), a delivery acknowledgement
from a real server, a general CRDT, model-weight distribution, a resnapshot
format, and any expiry or reaping.

**Complete-episode ledger (2026-10-21): C0/C1 only.**
`src/v3/complete_episode/ledger.rs` adds a single-writer, append-only, durable
ledger of complete episodes under `COMPLETE_EPISODE_DOMAIN`
(`wdbx-complete-episode-v1`), in its own file family
(`complete-episodes.v1.jsonl` and `complete-episodes.v1.lock`); the writer
lock is held for the store's lifetime and persistent contention is
`WriterBusy`. One record is one complete episode: the canonical
complete-episode bytes, the SHA-256 digest over exactly those bytes, and the
detached Ed25519 signature beside the record, so signing never moves a
digest. The chain runs across the ledger: record N names record N-1's digest,
and a gap, a fork or a duplicate digest is `Corrupt`. Replay re-derives every
digest, re-decodes and re-validates every record through the complete
episode's own validation, verifies every record that claims this store's own
key, and refuses anything else as `Corrupt`; only an incomplete final line is
discarded, and only after the verified prefix replays. It is a DIFFERENT
store from the v3 episode ledger — different domain, different digests,
different files — and it is not a second canonical episode for the same
content. The prototype's own types are crate-private, and the four handles
the hidden facade re-exports (`CompleteEpisodeReceipt`,
`CompleteEpisodeStoreError`, `PrototypeEpisode`, `PrototypeLedger`) are `pub`
but `#[doc(hidden)]`, unratified, and not to be depended on: `#[doc(hidden)]`
is documentation, not access control. The honest limit of the detached
signature: stripping a signature from a signed record is not detected — an
unsigned record in a signed ledger replays cleanly and is reported as
unsigned, which downgrades the evidence of who wrote it rather than forging
its content. Without a signature the ledger detects accidental mutation, not
an actor who rewrites the whole file. `tests/v3_complete_episode_ledger.rs`
pins it. This is C1 source and local deterministic test evidence for a
private, provisional, crate-internal prototype. Still open: ABI ratification,
a wire contract, quorum or fencing (the v3 quorum shard does not cover it),
federation, cross-language verification of the ledger format, a deletion or
retention lifecycle, a reader projection, and any consumer.

**C6 reason corrected (2026-09-16).** The earlier reason, "No `Verify` RPC", was
stale: `VerifyEpisode` / `ProposeEpisodeWrite` live on the ABI gateway
(`abi/crates/abi-wdbx-gateway/src/service.rs`, `abi wdbx episode verify`). An RPC
existing is not an operator witnessing an outcome, so the level stays ❌.
