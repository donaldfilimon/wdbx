# Bounded Studio scoring/orchestration v3 profile

2026-10-23 UTC. Implements frozen scoring D06-D08 and the local queue/attachment
parts of S07-S15. Frozen contracts remain binding. These are opt-in pure kernel
APIs; legacy IDs, saves, engine/settings, default dispatch, existing language,
scanner and identity semantics remain unchanged. No host effects, credentials,
scientific efficacy, durable ATP updates or operational concurrency are certified.

## Exact confidence and randomness

Keep scanner_v3 unchanged. Preserve its exact rational S/D/difference and clamp.
Validate supplied Evidence names/weights/fractions/totals/version and guards;
externally constructible data is not an authenticated permit. Provide an explicitly
numeric checked-balance API for policy tests, distinguished from actual scanning.
A production-oriented local adapter must scan immutable PreparedAction inputs and
retain their guards rather than accepting caller-forged executable constraints.

Use checked i128 Rational arithmetic through confidence, ATP modulation, jitter,
endpoint/threshold comparisons and clamping. Supplied ATP is an immutable rational
snapshot v in [-1,1], a in [0,1], each denominator <=1,000,000. No implicit float
quantization or update of legacy ATP. Node strength is integer0..100; threshold
integer[-100,100], default50; maximum strength100. Confidence is not probability.

Cbase=clamp(Cscan-10*a*max(0,-v),-100,100). Strength does not modify confidence;
strength>0 is a separate admission gate. Determine baseline-qualified via >=.
No jitter when disabled, Cbase exactly +/-100, or strength100 and baseline-qualified.
Otherwise apply independent sign and magnitude, then clamp and gate inclusively.
Default magnitude cap1; brainstorm5; explicit cap rational[0,5], denominator<=1e6.
Record baseline, eligibility reason, delta, effective value and every refusal.
Jitter never feeds back into baseline or durable state. All arithmetic overflow is
explicit, with no partial vote/election result.

Host supplies u32 draws plus seed/stream identity. Magnitude is uniform on the
closed discrete grid u32/u32::MAX, scaled by cap; independent next draw's low bit
sets sign (0 negative,1 positive). Label both actual draws. Exceptions/zero cap
consume no jitter draws. A named deterministic seeded helper is allowed, uses
only explicitly supplied seed/state and bounded draws; no entropy/clock/I/O.

Weights are finite nonnegative f64. Validate before drawing; invalid inputs fail
atomically. Entry alternative election is proportional in stable slot order,
excluding inhibited/ineligible alternatives. Zero total means no action (no
uniform fallback, no draw). A singleton positive eligible alternative needs no
draw. For larger pools draw once on the same closed grid; cumulative half-open
intervals with exact upper endpoint selecting the last positive slot. Use a
stable finite proportional algorithm; record original weights/draw/winner and
refused slots. f64 is confined to these declared sampling weights, not confidence.

## Contributions and exact redundancy

Canonical selected action is the bounded typed identity Expr tree, compared by
explicit structural tags, original literal/name bytes and ordered args. It is an
inert declared legacy-tree projection; no automatic conversion into v3 scripts,
execution or save format is asserted. Exclude election weight and concrete node
identity from canonical action equality. Keep complete original action/inhibition
metadata in provenance; never rewrite literal text to determine equality.

Contribution construction is validated/private, bounded, no Deserialize bypass.
Retain concrete node/entry/slot, scanner version, seed/draw trace, original source
and clause/action/fanout IDs, captures and guards. Denied/unresolved/refuted input
or inhibited target alternatives cannot produce an executable contribution.
A Declared similarity-bypass route can skip only the numeric threshold; retain
actual Ceff for arbitration (never silently assign100) and every other local gate.
A blocked automaton is never a successful route or bypass authorization.

Chunk root=(input ID,clause ID,action ID). Fanout siblings share this lineage root;
individual fanout IDs/rule refs remain provenance. An explicit host parent-lineage
identity distinguishes independent upstream branches. Redundancy requires equal
chunk root+parent lineage+canonical selected action+sorted inhibition set+ordered
named typed capture values (spans remain provenance, not equality). Input and
target inhibition facts have explicit distinct codes; no guessed equivalence.
Existing alternative inhibition remains a boolean. Additional typed facts are
bounded host declarations for provenance/equality, not an executable inhibition
interpreter or capability authorization.

Attached groups and IntermediateFact contributions are exempt. Different dependency
sets remain complementary. Contributions referenced as a dependency are retained,
preventing a redundant election from dangling a downstream edge. All other
contributions survive, including multiple qualified entries in the same node.
For each actual redundant group select with max(0,Ceff)*strength converted to finite
f64 only for arbitration; zero total selects no action. Record all candidates,
weights, draw and winner. No global election. Stable input order determines output
and tie ordering. Reject duplicate contribution IDs and inconsistent references.

Collection/elections validate all data and charge lengths/tree/string/vector work
before cloning. Fixed maxima:100,000 contributions,1,000,000 logical work units,
16MiB owned payload,100,000 draw calls; caller may lower budgets. Tree depth16,
4096 nodes/tree and text/name bytes8000 per leaf; reject overlarge inputs before
materializing. Caller input itself may be larger; checked preflight stops early.

Logical metadata charges use the private versioned logical-charge-v1 table, not
compiling-target size_of or pointer layout. Fixed units: byte1; f64/usize/reference8;
u128/Span16; String/Vec<Expr>24; InhibitionFact32; Alternative40; Expr/DrawRecord48;
Charge56; Definition64; Capture80; RouteCapture96; Token160; EntryRequest384;
Contribution2048; AttachmentContext320; Visit80; Arc context reference8. Attachment
retention counts inherited RouteCapture elements at96 each using checked
multiplication; the redundant Evidence Span predicate uses the same fixed16.
Recursive element counts and UTF-8 payload bytes remain separate
checked charges. The table is conservative logical accounting, not a measured
allocator/physical-memory limit. Earlier refusal on smaller-layout targets is
intentional; caller limits and global maxima are unchanged. Pin decision boundaries
using fixed literal/profile-derived limits, never target-adapted diagnostic values.

## Pure orchestration and queue contract

One owned broker per specimen/runtime domain, shared explicitly by foreground,
attachments and maintenance: maximum1000 active comparisons,100000 pending.
A caller supplying two independent brokers does not establish global enforcement;
host ownership integration remains required. Scope/population/cycle/job identities
and fencing generation are explicit, bounded and validated before queue access.
Batch admission is atomic, all admitted comparisons queued, otherwise Backpressure.
The pending cap is an upper bound; owned metadata/payload16MiB can refuse earlier.
Logical work1,000,000 and trace100,000 are per-operation limits, not unbounded
lifetime logs; events may be returned for host retention. Cleanup/completion cannot
become impossible because an internal historical trace filled. Reserve a
conservative bound for future linear cleanup before accepting owned metadata,
including nested completed/attachment values, within the caller operation-work
limit. Restart transitions cycles once and clears each global queue once; it
does not repeatedly scan queues per cycle. Keep reservation for retained
cancelled/quarantined metadata until it is actually dropped or retired. Headroom
refusal is atomic and explicit, never a silent no-vote or partial mutation. Per-cycle admission
limits are explicit and checked before acceptance; the model does not own or
measure external host snapshot memory.
Drained/fenced cycles may be explicitly retired to release owned metadata. Reused
cycle IDs receive a private incarnation binding. Mutating cycle-control calls
require a private incarnation-bearing creation handle or equivalent receipt,
including discovery close, seal, cancel, retire and compose. Stale host commands,
leases and issuer receipts cannot authorize a new incarnation. Retirement must not
release other cycles or quarantined capacity.
No first-hit or top-N truncation. Foreground includes attachments. Dispatch8
foreground quanta to1 maintenance while both exist, FIFO within each queue.
One lease represents one actual comparison; private unforgeable generation-bound
lease tokens complete once, stale/duplicate/wrong-scope completion refuses without
mutation. Normal completion releases its active permit once. Cancellation removes pending
work and revokes active leases, retaining them as counted quarantined capacity
until a trusted host fence acknowledgement (or observed termination) bound to both
the current owner generation and revoked worker generation releases each exactly
once. Old-worker acknowledgement remains possible after restart, but a stale
owner command cannot release capacity. Revoked results cannot publish or revive a cycle.
Expose quarantined counts and resulting backpressure; absence of acknowledgement
leaves capacity blocked. Restart advances generation but cannot clear surviving
physical work without acknowledgement. The pure model enforces this protocol;
actual host fencing, parallelism and cancellation execution remain unqualified.

Voter completion requires sealed admission plus no pending/active work for cycle;
attachment work admitted during processing must finish too. Reject new work after
seal/cancel. Cancellation/deadline is explicit partial status, never complete plan.
Model host time/deadline by supplied monotonic values only; no clock in kernel.
Finish all admitted work before arbitration/composition, no hidden recursive reopen.

Attachments trigger only a qualified issuer. Hard schedules target evaluation;
soft affinity is explicit rational[0,1] sampled on closed draw grid, with endpoints
0 never/1 always and no draw. A host-supplied remainder is a validated UTF8 subspan
of original input with inherited guards/captures and concrete lineage; it is not
inferred by subtracting scanner overlap. It may be empty (no target scheduling).
Never widen guards, reset depth, or force target vote. Default/max depth4; track
(cycle,target,remainder span+bytes) and reject repeated non-progressing handoff.
Pure bounded plans enqueue rather than recurse; scheduling uses the same broker.
Attachment group identity/dependencies remain on actually produced contributions.
Semantic correctness/completeness of host's remainder partition remains unqualified.

Compose only completed qualified contributions into a stable topological graph,
with explicit host-supplied dependencies and attachment group records; missing
references/cycles/duplicate IDs fail atomically. Preserve canonical action,
intermediate facts, captures, inhibition, source IDs and arbitration lineage.
No semantic chain is invented from text, no actual script/host action executed.
Support retrieval, Transient Memory, tone variation, aftercare, durable learning,
full ATP dynamics, adaptive raft tuning and active-engine/save migration remain
separate packages. This graph is a pure plan, not the entire production cycle.

## Evidence and review limits

Use newly authored synthetic mechanism fixtures, including S07-S15 direct tests,
not frozen efficacy holdouts. Actual opt-in native/WASM traces must independently
check equations, draw consumption, grouping and queue transitions. Compare exact
bytes of the labelled semantic projection; retain per-target raw diagnostics and
validate their own limits independently. Historical prototype size_of charges varied with pointer width and could change
tight-limit decisions. Those raw failed/projection runs stay historical; final
portable logical accounting requires the fixed charge table and same-limit
minus-one/exact-boundary scenarios. Target tags remain diagnostics. Neither equal
logical charges nor the E=0 cleanup scenario establishes allocator parity. Pure state
counts prove model invariants only, not measured host concurrency/fairness/cancel.
All163 requirements stay open/C0/unknown; partial candidates do not close cases.
Changes to this operational profile require recorded rulings and independent review.
Cost if these choices are wrong: revise the local profile/API/fixtures before
consumer adoption; no legacy saved data or IDs are reinterpreted.
