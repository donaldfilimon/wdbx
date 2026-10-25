# WDBX

The provenance-aware episodic substrate beneath ABI, and the foundation layer it
is built on.

Extracted from [`donaldfilimon/abi`](https://github.com/donaldfilimon/abi) on
2026-08-22 with history preserved. Every commit that ever touched these five
crates is present; nothing was squashed or replayed.

This repository is public so ABI, Abbey, and external forks can build the exact
pinned substrate without a cross-repository credential. That visibility covers
source code only: WDBX stores, episodes, evidence payloads, operator state,
credentials, and consumer runtime data stay private to their owners. Nothing in
this repository provides a hosted database or production authority.

## What this is

Under the Abbey System Constitution, ABI is the canonical cognitive and
governance runtime and **WDBX is its provenance-aware memory and evidence
substrate**. The distinction the substrate exists to enforce is:

> memory != database lookup

A vector database retrieves similar content. An episodic substrate must also
preserve context, causal dependencies, outcomes, versions, constraints, and
evidence, so that later it can answer: what happened, what was predicted, what
was done, what followed, and why this record is trusted.

## Crates

| Crate | Role |
| --- | --- |
| `abi-foundation` | Shared primitives: errors, env, time, validation, JSON, logging, credentials, atomic IO, HTTP helpers. No dependency on any other crate here. |
| `abi-telemetry` | Bounded, process-wide counters with insertion order preserved. No dependencies. |
| `abi-compute` | Compute selection, deterministic CPU SIMD primitives, and the `Accelerator` contract that consumers implement. No dependencies. |
| `abi-core` | Config, task scheduler, memory accounting, plugin registry. Depends on `abi-foundation` and `abi-telemetry`. |
| `abi-wdbx` | The substrate itself: on-disk segment format, CRC-framed WAL recovery, checkpoint publication and salvage, MVCC with conflict sets, multi-parent causal audit DAG, exact and layered-HNSW search, 3-D spatial index, cluster replication with read repair, reference quantization/Huffman/rANS/autoencoder codecs, and optional FHE reference paths. |

Crate names keep the `abi-` prefix deliberately. Renaming them would churn every
consumer for no behavioral gain, and the constitution's point is that ABI owns
this layer, not that the layer must be re-branded.

## Status: honest

Measured against the
[CSAPS/WDBX conformance gap analysis](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md), this
codebase implements most of the **structural** half of the substrate and little
of the **evidence** half.

**Current.** Multi-parent causal audit DAG. SHA-256 content addressing. Ed25519
signing over transaction and segment objects. MVCC. WAL plus segments with CRC
framing. Cluster replication and read repair. A pluggable retrieval scoring seam.
Program 4's first C1-only slice adds a pure `abbey-cbor-episode-v1`
deterministic-CBOR envelope encoder, sorted parent commitments, and exact
positive golden vectors. A bounded restricted-profile reader now validates canonical
syntax and envelope shape with finite resource limits, exact reencoding and an
independent Python decoding witness. It adds syntax validation only; see the
[qualification receipt](docs/superpowers/plans/2026-10-20-canonical-cbor-reader-receipt.md).
The next bounded slice adds a single-writer
`EpisodeStore`: it reconstructs that canonical envelope from typed lifecycle
events, computes the commitment inside WDBX, appends linked records, verifies
every complete record on reopen, and returns content-free receipts. New writes
fail closed on contract-revision, policy-version, and consent-epoch drift,
identity conflicts, request or operation replay, commitment mutation, guild
opt-out, `QUIET`, and per-guild token or storage exhaustion, each exercised by
`tests/v3_episode_store.rs`. Policy-version and consent-epoch drift are pinned
by `policy_version_drift_is_rejected_as_stale_binding_without_partial_write`
and `consent_epoch_drift_is_rejected_as_stale_binding_without_partial_write`,
which also assert that the rejected write leaves receipts, usage, and on-disk
bytes unchanged. This is C1 source evidence plus local deterministic
replay test evidence; it is not a claim-registry promotion or deployed
federation evidence.

Supersession lineage reads (`EpisodeStore::superseded_by`,
`EpisodeStore::supersession_lineage`, `src/v3/episode/store/lineage.rs`) project
the committed `supersedes` edges as content-free digests and flags. Both maps
are rebuilt in `apply_record`, so a live write and a replay-on-open answer
alike, and `tests/v3_supersession_lineage.rs` pins that. This is C1 source and
local deterministic replay evidence for a lineage projection of already
committed edges. It adds no holds or revocation of its own; holds and
revocation (`src/v3/episode/store/holds/`), redaction and object-key
destruction (`src/v3/episode/store/deletion.rs`) and garbage collection
(`src/v3/episode/store/gc.rs`) are separate slices.

The write gate now also answers in one content-free value.
`src/v3/episode/admission.rs` defines `AdmissionClass` (the closed
five-class retention vocabulary), `AdmissionReason` (exactly one reason per
`EpisodeStoreError` variant, plus `Accepted`) and `AdmissionDecision`.
`EpisodeStore::decide_write` runs the same validation, commitment check and
quota checks as `propose_write` and appends nothing, and
`EpisodeStore::decide_write_outcome` answers the same question in one
content-free value that never fails: it carries the class and the reason
(`Accepted`, or the reason the refusal maps to), so a caller never has to
match on an error string. The three budget checks
were extracted into `EpisodeStore::check_quotas` and are now shared by the
append path, the decision and the commitment preview, so a preview no longer
succeeds for a write the append would refuse on budget; `check_quotas` also
charges the retention sidecar's own size ceiling, and the decision and the
preview measure the signed line the append writes, so all three paths reach
the same verdict. `EpisodeStoreError`
and its labels are unchanged, so the ABI gateway's status mapping is
unaffected, and `tests/v3_admission_decision.rs` pins that the decision and
the append always agree. This is C1 source and local deterministic test
evidence.

Quota administration lives in `src/v3/episode/store/quota/`. A guild's
budgets still come from the host policy, and an authority may adjust them at
runtime: `EpisodeStore::decide_quota_adjustment` is the non-mutating
decision, `EpisodeStore::commit_quota_adjustment` commits through the single
writer, `EpisodeStore::clear_quota_adjustment` lifts an adjustment,
`EpisodeStore::quota_state` reports the effective budgets, the usage and
whether the usage is above the effective budget (`over_budget`), and
`EpisodeStore::quota_history` reports every decision the sidecar holds for a
guild, in order. The effective budget is the host policy's budget for that
guild, lowered to a recorded adjustment's budget only while the policy still
states budgets at or below the ones in force when the adjustment was made,
and never above the policy's budget. The host policy is the outer bound: no
adjustment may set a budget above it, whoever makes it, and the host restores
a larger budget by changing the policy or by clearing the override. Only an
authority that may raise a budget may clear one, so a writer can never freeze
a guild's budget permanently; a budget is a *guild* budget, so a service that
lowers it restricts every writer in that guild, not only itself. The effective
budget is what `check_quotas` reads, so an adjustment changes what the store
accepts and the change is visible in `decide_write`,
`preview_commitment` and `propose_write` alike. The adjustment is a
governance decision rather than an episode, so it lives in a derived sidecar
beside the ledger, `episodes.v1.quotas.jsonl`, in the established shape:
content-free lines (guild, the two budgets, the policy's budgets in force
when the decision was made, the authority, the instant), a checksum over each
line's own fields, and a header line naming the number of decisions and a
checksum over all of them. The header is what makes a truncation detectable,
and a truncated file, a trailing fragment and a bad checksum are all refused
as corrupt. The file is created with the first adjustment rather than with
the store, every write replaces it atomically, and it is written by the same
single writer; it survives close and reopen. A guild owner, administrator or
manager and an organization owner may adjust a budget in either direction; a
service may only lower one, because a writer that could raise its own budget
could defeat the budget; a human subject may not adjust a budget at all. That
vocabulary is actor-class based, and the class rule is only the first of two
necessary conditions: the guild-scoped authority map below decides whether
the principal may act for the target guild at all. An adjustment may never
lower a budget below the usage the guild has already recorded, because that
would retroactively invalidate writes the ledger already holds. The same
state is reachable when a host policy is lowered below the recorded usage;
the store surfaces it through `quota_state`'s `over_budget` instead of
refusing to open, because refusing would brick a store whose policy was
lowered. `EpisodeStoreError`
and its labels are unchanged. `tests/v3_quota_windows.rs` pins it. This is C1
source and local deterministic test evidence. It adds no per-caller or
per-class budget, and quota administration that is itself an episode is not
implemented: that needs a new event variant and a coordinated regeneration of
every pinned golden.

The guild-scoped authority map lives in the host policy:
`GuildEpisodePolicy::authorized_principals` is an optional, defaulting list of
the principals that may act as authorities *for that guild*, and the store
reads it in `src/v3/episode/store/authority.rs`. It is host policy and not
episode material: the canonical commitment binds the policy *version*, never
the policy itself, so naming a principal moves no committed digest and the
frozen v3 record format is untouched. The rule has two halves and both are
necessary. The guild's own map: when a guild names a non-empty list, only a
principal on that list may place a hold, release a hold, revoke a candidate,
adjust a budget, lift an adjustment, or order a destruction for that guild,
whatever class it carries. The store-wide scope: a principal that *some* guild
names is scoped to the guilds that name it and holds no class-only authority
anywhere else — without that, naming the owner of one guild would leave that
owner free to act for every other guild, which is the hole the map exists to
close. What the map closes, for the principals the host names, is that hole; a
principal that no guild names is not closed by the map at all and still reaches
every guild whose policy states no map of its own, on the class rule alone.
The actor *class* remains a second, necessary condition: a service may never
place a hold, revoke a candidate, raise a budget, or order a destruction, even
when the map names it, and a human subject is never an authority. The three
states of the map are pinned: an absent map leaves the guild to the class rule
alone (a compatibility behaviour, not a security guarantee — a host that wants
a guild closed to unknown principals has to name them), a non-empty map
constrains the guild to the principals it names, and an empty map is the same
state as an absent map, so an empty list is not a closed guild.
`EpisodeStore::actor_may_act(guild_ref, actor)` is the store-side predicate,
and it is the guild-scoping half only. Every refusal answers the same
closed, content-free `EpisodeStoreError::InvalidTransition`, which names
neither the principal nor the guild. The honest limit: the substrate still
cannot *verify* that a principal really holds a role; the host policy is the
authority of record and the store trusts the caller to present the right
principal and the right class. `tests/v3_guild_authority.rs` pins it. The map
does not constrain the canonical *write* path. The map does not reach the
lifecycle `EpisodeEvent::Approval` (`src/v3/episode/store/validate.rs`),
whose approver is class-checked (a valid actor, not a service, not the
requester and not the proposer) but is *not* guild-scoped — no guild check is
applied to it, so a principal labelled `GuildOwner` for guild A may approve an
operation in guild B. That is a missing *guild* check, not a contradiction of
the class rule: an approval is a **workflow** step, not a governance action
over a guild's resources — it accepts one proposal and confers no authority
over anything else — so the rule that a human subject is never an
*authority*, a rule about governance authorities, does not reach it, and a
`HumanSubject` is an admissible approver there by design; what the store does
not do is verify that the approver holds any standing in the guild of the
operation it approves. The map does reach the deletion lifecycle's destruction
requests (`src/v3/episode/store/deletion.rs`): `RedactionRequest` and
`ObjectKeyDestructionRequest` carry an `actor`, every destruction order is
refused unless the principal is one the guild authorizes *and* the class is
one that may destroy (a guild owner, administrator, manager, or an
organization owner — never a service, never a human subject), and the sidecar
lines record the principal, so a destruction order is attributable. The map is
not committed, signed, or replayed: it is host policy, and changing it changes
what the store accepts from the next open onwards — the store binds the policy
at open and never reloads it.

Retention classes and TTL expiry live in `src/v3/episode/store/expiry.rs`.
`GuildEpisodePolicy::retention_ttl_ms` is the fallback TTL for `session` and
`operational` candidates; it is optional and defaults to absent, so a host
policy that omits it still parses and still means "no expiry".
`GuildEpisodePolicy::retention_windows` is an optional per-class map
(`session_ttl_ms`, `operational_ttl_ms`, `durable_ttl_ms`), also optional, so
a host policy that omits it still parses and still means the previous
behaviour. The precedence is fixed: an explicit per-class window wins over
the single TTL, the single TTL remains the fallback for every class without
an explicit window, a class with neither never expires, and `durable`
candidates never expire whatever the policy says.
`durable` candidates never expire. The store binds the deadline
(`arrival + ttl`) once, when the candidate is admitted, and
`EpisodeStore::retention_state`, `EpisodeStore::due_expirations` and
`EpisodeStore::expiry_writes` report it, decide what is due, and emit the
`forgets` writes. The emitter never writes: the caller routes the emitted
writes through `propose_write`, the one canonical write path. The v3 record
format carries no arrival timestamp and is frozen, so the deadline is
committed in a derived index beside the ledger, `episodes.v1.retention.jsonl`:
digests and deadlines only, written by the same single writer before the
ledger line it describes, and created with the first deadline rather than
with the store. A candidate with no deadline is never expired on a guessed
time; it is kept. The write gate refuses a zero window outright — a single
zero TTL or a per-class zero — the only way a
deadline can be closed at admission, because the deadline is bound from the
store's own clock at the instant of admission. A positive TTL is admitted and
may expire immediately afterwards; the gate never compares a candidate's
deadline against a caller-supplied clock. A candidate under an open quarantine
or contradiction is not expired behind an open review.
`tests/v3_retention_expiry.rs` and `tests/v3_retention_windows.rs` pin it. The
window is chosen by the policy, never by the candidate: a candidate-selected
window would need a digest-bound field on `MemoryCandidate`, or a new event
variant, and a coordinated regeneration of every pinned golden and of the
Python witness. The garbage collection of expired records is a separate slice: the
forgotten candidate's tombstone stays in the ledger, and its payload-bearing
record is reclaimed.

The deletion lifecycle lives in `src/v3/episode/store/deletion.rs` and
`src/v3/episode/store/gc.rs`. `DerivedCopyKind` is the closed vocabulary of
the derived copies a consumer may hold (embedding, index entry, export,
cache). `EpisodeStore::decide_redaction` and `EpisodeStore::commit_redaction`
record a redaction request for a forgotten target in
`episodes.v1.redaction.jsonl`, and `EpisodeStore::redaction_state` reports
which kinds a target has. The store holds no derived copy: its contract is
"a redaction request exists for target X covering kinds K1..Kn", and
destroying the consumer's copies is the consumer's obligation, which is not
implemented here. `ObjectKeyRef` is the v3 episode namespace's own bounded
tenant/key reference; `EpisodeStore::decide_object_key_destruction`,
`EpisodeStore::commit_object_key_destruction` and
`EpisodeStore::object_key_destruction_state` record and report destruction
requests in `episodes.v1.object_keys.jsonl`. A request for a live record is
refused, and the refusal does not distinguish a live target from an unknown
digest. Both requests carry the `actor` that issues them: the store refuses
the order unless the guild's authority map names that principal and the class
is one that may destroy — a guild owner, administrator, manager, or an
organization owner, never a service and never a human subject — and the
sidecar lines record the principal, so a destruction order is attributable
(`tests/v3_deletion_authority.rs`). Each sidecar is a header line plus one
line per request with a checksum over the request lines, so a truncated or
mis-checksummed sidecar is refused as `Corrupt` rather than believed.
`EpisodeStore::gc_plan` is the non-mutating collection plan and
`EpisodeStore::apply_gc_plan` performs the rewrite: a payload-bearing record
that is forgotten, past its deadline, and under no open review is reclaimed,
while every tombstone is retained forever. The v3 ledger is not one global
hash chain, and the canonical digest never covers the ledger `sequence`, so
the rewrite re-positions the surviving records and leaves every surviving
digest unchanged. The reclaimed index (`episodes.v1.reclaimed.jsonl`)
remembers, content-free, which candidates were reclaimed, so the ledger's
tombstones still replay after their record is gone; it is written before the
rewrite it describes. The rewrite is crash-safe: the new ledger is written
beside the old one and renamed over it, so an interrupted collection leaves
the original ledger intact and openable. `tests/v3_deletion_lifecycle.rs`
pins it. This is C1 source and local deterministic test evidence. It adds no
destruction of the consumer's own copies, and no candidate-selected retention
window.

Holds and candidate revocation live in `src/v3/episode/store/holds/`. A *hold*
is a placement that freezes a memory candidate: while it is live the candidate
is not expired away, not reclaimed, not silently superseded, and its payload
and derived copies are not destroyed. The `HoldKind` vocabulary is closed
(`legal`, `investigation`, `user_request`);
`EpisodeStore::decide_hold`, `EpisodeStore::commit_hold`,
`EpisodeStore::release_hold` and `EpisodeStore::hold_state` report it, and only
the placing authority, or one above it in the documented governance order
(guild manager < guild administrator < guild owner < organization owner), may
release a hold. A *revocation* withdraws a candidate by authority: the
candidate is not live any more, no later `supersedes` may name it, and it is
never resurrected, while its record and its anti-resurrection metadata are
retained, because a revocation is a withdrawal from use and not an erasure.
`EpisodeStore::decide_revocation`, `EpisodeStore::commit_revocation` and
`EpisodeStore::revocation_state` report it; only a guild owner,
administrator, manager, or organization owner may revoke, and a service may
not. Both decisions are committed in derived sidecars beside the ledger,
`episodes.v1.holds.jsonl` and `episodes.v1.revocations.jsonl`, in the same
shape as the retention, redaction, object-key and reclaimed indexes:
content-free lines, a checksum over each line's own fields, created with the
first decision, and written by the same single writer. They are sidecars and
not new `EpisodeEvent` variants on purpose: a field on `MemoryCandidate` would
change every candidate digest and break the pinned goldens and the Python
witness, and a new event variant would change the canonical encoder, the
Python witness, and every pinned golden with it.
`tests/v3_holds_revocation.rs` pins it. This is C1 source and local
deterministic test evidence. The constitutional query reports both states: the
lifecycle dimension carries the hold state and the revocation state, and
`Disposition` has `Held` and `Revoked` members for them.

`src/v3/episode/store/query/` adds a constitutional query read model.
`EpisodeStore::query` returns the eight evidence dimensions as separate
fields, each with its own content-free reason, alongside a closed
`Disposition`, the record's lineage, an explanation and a sanitized receipt.
There is no combined score and no field that could stand in for one. The query
is guild-scoped, member-scope partitioned and bounded by the store's receipt
bound, and it omits nothing: a record whose cryptographic evidence fails is
listed as `rejected` rather than dropped, and a forgotten record is listed as
`rejected` when the caller asks for erased memories (by default it is
excluded, because the tombstone means the content is gone). Dimensions the
ledger does not hold read `Unknown` rather than a fabricated zero. The
lifecycle dimension also reports the hold state and the revocation state, and
`tests/v3_constitutional_query.rs` and
`tests/v3_constitutional_query_holds.rs` pin it. This is C1 source and local
deterministic test evidence for a read model. It adds
no production key trust, no cross-language
verification and no ABI-facing wire contract, and the query is not wired into
the v2 retrieval scoring seam: `src/temporal.rs`'s `ScoreComponents::combined()`
still collapses semantic, temporal, causal and persona affinity
multiplicatively into one score.

`src/v3/ranking/` adds the dimension-separated ranking and the vector-index
adapter. The ranking consumes the constitutional query's answer and keeps the
eight evidence dimensions separately inspectable instead of collapsing them
into one number, which is the opaque collapse `src/temporal.rs`'s
`ScoreComponents::combined()` is; a missing dimension reads `Unknown` rather
than a measured zero, and the per-dimension order is a sequence of labelled
states, never a number. `VectorBindingIndex` is the substrate-side binding
between a vector index's identifiers and the episode digests the query
answers: a derived sidecar beside the ledger (`episodes.v1.vectors.jsonl`)
whose record shape matches the episode store's own sidecars and whose
writer-exclusion discipline now matches them too — an owner file
(`episodes.v1.vectors.lock`) and a lock held for the index's lifetime, so a
second writer is refused instead of racing, and every write re-reads and
merges the file on disk, so a binding another writer added is never lost. The
file carries a header (format version, record count, checksum over the record
set), so a truncation is detected: the index is replaced whole, so a trailing
fragment is damage and not a crash tail, and a truncation at a line boundary
is caught by the header; either is refused as corrupt, never a shorter index.
A binding is refused a digest the scope's constitutional query did not admit
(`AdmittedScope`, built only from the query's own answer, whose scope the
answer now carries); that refusal is a check against the caller's admission
set, so the substrate trusts the caller to hand over the query's own answer
and does not independently prove admission. `HnswVectorSource` (approximate,
pinned seed) and `ExactVectorSource` (the deterministic oracle) both resolve
a slot to an episode digest through that index, and
`rank_with_vector_source` is the end-to-end path. The adapter says what it
drops: an HNSW search is approximate and may miss a nearest neighbour, the
binding index is separate from the ledger, a vector's payload is never held,
an unbound vector's episode is never named, and an asset digest is never held
at all. `tests/v3_vector_adapter.rs` pins it. This is C1 source and local
deterministic test evidence. Not implemented: latency or memory measurement, a
cross-language witness for the binding sidecar, an append-only binding log, a
writer-exclusion fence against an actor who rewrites the file wholesale (the
lock is advisory, as everywhere else in the repository), a vector arm over
several scopes in one index, cross-language verification, and an ABI-facing
wire contract.

`src/v3/offline/` adds a durable offline outbox. `OfflineOutbox` is a
single-writer queue of PROPOSALS made while a scope was offline: an
operation id, an optional dependency, a kind tag (`Write`, `Delete`, `Asset`,
`Edit`), a binding (scope, contract revision, contract digest, policy
version, consent epoch), a proposal, and a status from a closed vocabulary.
It is not a canonical episode writer and must never become one: it stores the
write the canonical store would compute its own commitment from, it refuses a
write carrying a predicted commitment, and it holds no canonical episode
digest. Reauthorization is a separate, explicit step: `drain` takes the
CURRENT policy and the canonical store and answers a per-operation
`ReauthorizeOutcome`, running the store's own non-mutating decision rather
than re-implementing the write gate. No operation is dropped by the outbox's
own logic: a refused or conflicted operation keeps its proposal, a terminal
status and a reason, and only an explicitly authorized caller (guild manager,
administrator, owner, or the organization owner) may requeue, discard, or
remove it; the removal is status-independent (`Resolution::Remove`), so an
acknowledged operation is not immortal and a long-lived client can make room,
and a full outbox refuses new work with `OutboxError::Capacity`, which is
distinct from an invalid input. The merge is deterministic and explicit over
a small closed set of mergeable configuration objects, and the outbox never
invents a winner. An asset operation carries a digest, a length and a chunk
size but never the bytes; the chunk payload is supplied by the caller and the
final digest is verified from the recorded chunk digests, so a resumed
transfer does not re-request a completed chunk. The asset digest is numbered
by chunk index, so the order a caller hands its chunks over in never changes
it and a correctly transferred asset is never reported as corrupted. There is
no fixed offline expiry. The file is `outbox.v1.jsonl` beside
`outbox.v1.lock`: a header line naming the operation count and a checksum
over the record set, then one checksummed line per operation, and every write
replaces the file atomically. A truncated file, a trailing fragment, and a bad
checksum are all refused as corrupt: the outbox never opens with fewer
operations than were queued, and nothing is discarded by its own logic.
`tests/v3_offline_outbox.rs` pins it. This is C1 source and local
deterministic test evidence. Not implemented: a transport or a server (the
caller supplies an `OperationSink`), a delivery acknowledgement from a real
server, a general CRDT, model-weight distribution, a resnapshot format, and
any expiry or reaping.

`src/evaluation/` adds a frozen local baseline comparison harness, with
frozen fixtures in `tests/fixtures/retrieval-baseline/`. It compares a
plain-vector arm against the constitutional query over one frozen synthetic
corpus with frozen embeddings, frozen storage and a fixed candidate count,
reports `repeated_failure_rate` and `stale_reuse_rate` for both arms, and
ablates each of the eight dimensions. On that corpus both headline rates fell
from 0.666667 to 0.000000, and the harness is able to report "no benefit":
suppressing every dimension reproduces the baseline exactly, which is
asserted. The run also carries a negative result — the constitutional arm
returned an unattributed (unsigned) record in 7 of 12 families against 1 of 12
for the plain arm, because the query accepts an unsigned record on its
commitment. The receipt is the
[baseline comparison receipt](docs/superpowers/plans/2026-10-21-retrieval-baseline-receipt.md).
This is a deterministic local C1 comparison over synthetic data: not a
research result, not a production claim, and not evidence that
evidence-weighted retrieval beats a vector database in production. The
preregistered protocol in `docs/completion/experiments.json` is not met and
requirement `P04.03` stays `proposed`.

Ledger recovery charges original JSONL byte lengths, not reserialized sizes,
so equivalent JSON whitespace and escapes cannot evade guild storage budgets.
On Unix, initialization synchronizes ledger directory entries before admitting
writes; other platforms retain file-only synchronization and do not gain that
directory-entry durability guarantee.
Episode writer-lock acquisition tolerates transient inherited descriptors with
a bounded 50 ms contention retry; persistent contention remains `WriterBusy`.
A `compensated` terminal status requires observed exact prior-state restoration;
an unsuccessful compensation attempt may still be recorded and terminated as
failed. Previously admitted chains claiming successful compensation after a
failed restore now fail replay rather than being rewritten. These are source
and local regression-test guarantees, not empirical power-loss or
deployed-system qualification.

**Private prototype.** A crate-private complete-episode model now has bounded
canonical encoding, a private typed complete decoder
(`src/v3/complete_episode/`) and an independent Python encoding and decoding
witness. Its domain remains provisional pending ABI agreement; it adds no
admission, no cross-language verification of the ledger format, and no
ratified contract, but it
does add a private provisional ledger (`src/v3/complete_episode/ledger.rs`)
with persistence and detached-signature verification under the store's own
key. See the
[prototype qualification receipt](docs/superpowers/plans/2026-10-20-complete-episode-prototype-receipt.md).

The complete-episode ledger (`src/v3/complete_episode/ledger.rs`) is a
single-writer, append-only store of complete episodes under
`COMPLETE_EPISODE_DOMAIN` (`wdbx-complete-episode-v1`), in its own file
family (`complete-episodes.v1.jsonl` and `complete-episodes.v1.lock`); the
writer lock is held for the store's lifetime and persistent contention is
`WriterBusy`. One record is one complete episode: the canonical
complete-episode bytes, the SHA-256 digest over exactly those bytes, and the
detached Ed25519 signature beside the record, so signing never moves a
digest. The chain runs across the ledger: record N names record N-1's
digest, and a gap, a fork or a duplicate digest is `Corrupt`. Replay
re-derives every digest, re-decodes and re-validates every record through
the complete episode's own validation, verifies every record that claims
this store's own key, and refuses anything else as `Corrupt`; only an
incomplete final line is discarded, and only after the verified prefix
replays. It is a DIFFERENT store from the v3 episode ledger: different
domain, different digests, different files; it never writes a v3 candidate
and is not a second canonical episode for the same content. The prototype's
own types are crate-private; the four handles the hidden facade re-exports
(`CompleteEpisodeReceipt`, `CompleteEpisodeStoreError`, `PrototypeEpisode`,
`PrototypeLedger`) are `pub` but `#[doc(hidden)]`, unratified, and not to be
depended on — `#[doc(hidden)]` is documentation, not access control. The
honest limit: stripping a signature from a signed record is not detected — an
unsigned record in a signed ledger replays cleanly and is reported as
unsigned, which downgrades the evidence of who wrote it rather than forging
its content; that is inherited from the v3 store's detached-signature
design. `tests/v3_complete_episode_ledger.rs` pins it. This is C1 source and
local deterministic test evidence. Not implemented: ABI ratification, a wire
contract, quorum or fencing, federation, cross-language verification of the
ledger format, a deletion or retention lifecycle, a reader projection, and
any consumer. Without a signature the ledger detects accidental mutation, not
an actor who rewrites the whole file.

**Not implemented, and not claimed.** No complete constitutional v3
`EpisodeBlock` or general canonical-CBOR reader. The shipped episode signature
is a detached per-record Ed25519 signature over the v3 digest. A private
crate-internal `COSE_Sign1` prototype (`src/v3/episode/cose/`) encodes and
verifies a bounded tag-18 container carrying an RFC 9052 `Sig_structure`
Ed25519 signature over that same digest, and keeps an `Integrity` axis
(Valid/Invalid/Unsigned/Unverifiable) apart from the key axis. Its in-memory
`TrustList` models Pending/Active/Retired/Revoked with a revision counter and
terminal revocation: retired and revoked keys still resolve for historical
verification but never authorize a new admission. It is a prototype, unratified
and crate-internal, with no cross-language verifier, no production key custody,
no admission or authorization semantics and no independent cryptographic
review. It changes no committed digest, and the detached ledger signature
format is unchanged. Not implemented: a ratified COSE profile, cross-language
signature verification, source-side signatures.
The COSE container *encoding* does have a second-language witness:
`tools/wdbx_cose_sign1_p1.py`, driven by `tests/v3_cross_language_cose.rs`,
requires both languages to read the same container bytes into the same
protected header, key id, algorithm, payload and RFC 9052 `Sig_structure`
bytes, and to refuse the same malformed containers with the same error names.
That is encoding agreement only: the Python side does not verify signatures and
cannot, because this environment has no Python cryptography library and
hand-rolled cryptography is forbidden here. The Ed25519 checks in the Rust
test are Rust-side checks.
The `abbey-cbor-episode-v1` encoder and the episode digest do have a
second-language witness: a standard-library Python reimplementation
(`tools/abbey_cbor_episode_v1.py`) must agree with the Rust encoder on the
golden vectors and a differential corpus, and must reproduce, from each
write's JSON wire form plus its parent, every receipt digest a real
`EpisodeStore` appends across all seven event variants, under `cargo test`.
The Python witness also independently decodes the restricted profile and checks
malformed inputs against Rust; it does not implement the store's admission
rules or any COSE container, and it verifies no signature. The existing v2 commit digest remains `serde_json` bytes with parents in insertion
order; it is intentionally unchanged for compatibility. No `policy_version`,
`signer_key_id`, `task_regime`, or `regime_posterior` exists in the older v2
record format; the new episode ledger binds `policy_version` but does not
reinterpret v2. Contradiction, quarantine, and resolution edges exist only
as single-event memory-edge episodes over v3 memory candidates (spec
`2026-09-16-spec-memory-edge-episodes.md` in ABI); they never hide a record,
and the constitutional query reads them as the outcome and lifecycle
dimensions; the dimension-separated ranking carries that state as a labelled
signal beside a vector arm's own order, but the v2 retrieval scoring seam
still does not weigh them. No evidence-weighted retrieval *ranking* in the v2
scoring seam: the constitutional query exposes the eight evidence dimensions
separately and never collapses them, but the retrieval scoring seam still
combines semantic, temporal, causal, and persona affinity multiplicatively
into one score, which is the opaque collapse the constitution's invariant I3
forbids, and the constitutional query is not wired into it. No full
*block-level* retention, redaction, deletion, or hosted
service semantics: the v3 retention, redaction-request, object-key-destruction
and garbage-collection slices are candidate-level and content-free, and the
store holds no derived copy and destroys none; no ratified or cross-language
signature verification.
The structured admission decision classifies a write and explains a refusal;
it runs the same retention gate as the append path, and the budgets it reads
are the effective ones: the host policy's budget for that guild, lowered to a
recorded adjustment's budget only while the policy still states budgets at or
below the ones in force when the adjustment was made, and never above the
policy's budget. The constitutional query is a
read model over the episode ledger: it is not wired into the v2 retrieval
scoring seam (`src/temporal.rs`'s `ScoreComponents::combined()` still
collapses four factors multiplicatively into one score), it resolves signing
keys only through a caller-supplied resolver, and it is
not a cross-language or ABI-facing contract. The frozen retrieval baseline
harness compares the two arms over one synthetic corpus; it is not the
preregistered experiment, no independent runner holds the test split, and it
measures no latency, memory or production traffic.

Closing those gaps is Program 4, `canonical-wdbx-episodes-claims`.

### Program 1 contract qualification

**Current at C1, data-only.** WDBX vendors ABI's 113-artifact Abbey contract
corpus from immutable source revision
`63e6d6a79d0b8745a652803887d07665245ddb39`, aggregate digest
`3ffd487bdc497b7ce54b8c29978a3686dcbffdb66a85957a0ee4f99ba576cdfd`.
Its native Rust integration test independently verifies the closed lock,
artifact inventory and bytes, per-file SHA-256 commitments, aggregate digest,
local-only episode-schema compilation, and the episode proposal, evidence,
claim, retention, and tombstone fixtures. Mutation coverage rejects changed
artifact bytes, extra files, changed line endings, adapter-supplied episode
digests, missing deletion keys, invalid retention, transport JSON as a durable
commitment, and adapter projections as canonical episodes.

Corpus qualification alone does not authorize the separate runtime episode
write gate. The corpus still contains no positive `abbey-cbor-episode-v1`
fixture; WDBX's crate-local positive canonical-CBOR vectors and episode-store
replay tests are separate implementation evidence. Production federation,
deployment, and live Discord evidence remain separately authorized and
unclaimed.

## Gate

```
bash tools/check.sh   # instruction + size checks, then the three steps below
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

`tools/check.sh` runs the agent-instruction pointer check (`CLAUDE.md` must
stay the pointer form of `AGENTS.md`), the Rust source size limit (1000 lines
per `.rs`), and then the three cargo commands; CI runs the same script. Run it against the
current checkout before making a green-gate claim. Extraction-era test totals are deliberately not a current acceptance
contract. The workspace denies `unsafe_code` and all of clippy, and warns on
`missing_docs` and clippy pedantic.

## Consumers

`abi` and `abbey` consume these crates by relative path as siblings under
`~/dev/active`. That layout is load-bearing: a git-dependency for one consumer
and a path dependency for another would give Cargo two distinct copies of
`abi-wdbx`, and types would stop unifying where they cross.
