# Next Studio research preflight

2026-10-23. Planning evidence from read-only source/contract inspection. This
report is the only write. No suite, experiment, holdout, runtime, host action or
inventory update was performed. All requirement results remain open/C0/unknown.
The recommendation is conditional on the controller accepting the final current
scoring/orchestration review; this preflight does not certify that tranche.

## Recommendation

The smallest independent next packet is **a private, additive ATP deterministic
state-transition API**, implementing `advance(state, stimulus, elapsed)` from
D10. Target P08.05 / CH14 and the buildup/release/cooldown part of S24. Do not
repeat scoring's rational penalty, confidence jitter, elections, broker or graph.

`plans.json:304-324` puts synthesis/ATP/vision after scanner/vote/provenance, but
does not prescribe a total order between those three. ATP advance consumes a
validated state and one explicit stimulus; it needs neither a synthesis decoder,
image host, learned model, proposal writer nor canonical receipt. It is smaller
than a complete synthesis or vision packet and precedes the supervision layer in
the plan. This is a recommendation about decomposition, not a new requirement or
a claim that an entire frozen acceptance row would thereby pass.

The first packet should accept **one already resolved stimulus**. Do not quietly
add a Chargebook reducer, move the update before scoring, quantize its result
into scoring's rational snapshot, implement reward/autonomy or rewrite text.
Those require additional recorded local rulings described below. S25 and full
P08.05 stay open. A private transition packet can proceed under existing local
implementation authorization; a public/cross-package contract cannot bypass its
named-owner interface gate.

## Binding contracts and precise scope

| Contract | Applicable obligation |
| --- | --- |
| `requirements.json` P08.05 | Specified buildup/release equations **and** meaning-preserving tone; the proposed first packet covers the equations only. |
| CH14 / Architecture sections14.1-14.3 | Persistent bounded tonal state, temporary non-accumulating jitter, conservative baseline, traced autonomy, hard limits and inhibition. |
| S24 | `ATP spans two related cycles`: intended buildup/release persists and last-cycle random jitter does not. Direct state-transition observations are partial until actual cycle/view integration is tested. |
| S25 | Tone beyond tolerance retries within budget or returns original; deliberately outside this packet. |
| D10 / `designs.json:76-87` | Exact selected decay, charging, release, cooldown and urgency behavior below; broader tone/reward/autonomy remain obligations. |
| P08.10 / `plans.json:318-330` | Actual native/WASM fixtures and distinct hardware qualification; source inspection or compilation is insufficient. |
| P01.04 / `experiments.json` | Freeze, budgets, independent holdout controls and retained failures before efficacy execution. |
| `shared-types-v1.json:6-13,145-153` | Local proposed AtpState is finite f32 `v[-1,1]`, `a[0,1]`, `cooldown_seconds[0,30]`; public task08 interface requires owner-reviewed tagged schema/version negotiation and shared positive/malformed vectors. |

The selected deterministic equation (`designs.json:81-84`) is:

1. Reject nonfinite state/input and negative elapsed; clamp admitted elapsed to
   `dt=min(elapsed,3600)`. Use the supplied monotonic elapsed value, no clock.
2. `d=exp(-dt/180)`, `v0=d*v`, `a0=d*a`.
3. If cooldown is positive **at entry**, decay only and set cooldown to
   `max(0,cooldown-dt)`. Crossing zero during this step does not also charge.
4. Otherwise `v1=clamp(v0+.2*stimulus.valence*stimulus.intensity,-1,1)` and
   `a1=clamp(a0+.2*stimulus.intensity+.1*urgency,0,1)`.
5. At `a1>=.85`, release to `a=.5*a1`, `v=.8*v1`, cooldown30 seconds;
   otherwise keep `v1/a1`, cooldown0.

Use libm in this pure kernel. Deterministic state receives no seed, random draw,
tonal-view delta, confidence or strength. Supplied urgency is a local declaration,
not evidence of an active-clause match or authority. Finite/bounded stimulus
construction must precede computation. An immutable return value and explicit
typed refusal prevent partial state mutation.

## Existing source inspected; what it already supplies

Paths in this table are relative to `specimen-studio/`.

| Source seam | Observation and next-packet implication |
| --- | --- |
| `native/specimen-kernel/src/scoring_v3/policy.rs:7-25,76-122` | Accepts immutable Rational ATP valence/intensity, denominators at most1000000, and computes the selected exact confidence penalty. This is a snapshot consumer, not an ATP state machine. Keep it intact; no implicit f32-to-rational bridge. |
| `native/specimen-kernel/src/orchestration_v3/compose.rs:24-33` | CompletePlan is expressly inert and excludes tone, aftercare and durable ATP. It supplies no cycle-update timing or persistent transition. |
| `native/specimen-kernel/src/language_v3/model.rs:209-227,267-338` | Immutable Charge exposes f64 valence/intensity, span and rule ref; PreparedAction exposes origin, tokens, guards and charges. These are useful future reducer inputs, not an aggregate stimulus or a validated urgency receipt. |
| `native/specimen-kernel/src/language_v3/prepare.rs:370-382` | Emits Chargebook evidence while visiting matched token positions. A downstream consumer must select eligible active spans and deduplicate fanout/overlap explicitly; it must not assume `charges()` already embodies ATP reduction policy. |
| `native/specimen-kernel/src/engine.rs:934-954` | Legacy update uses wall-millis, clips backwards elapsed to0, has no3600-second cap, matches whole normalized input substrings, adds `.2*valence` without stimulus-intensity multiplication, releases at `>.85`, leaves valence unreleased and stores absolute cooldownUntil. Preserve this compatibility path; it is not the selected equation. |
| `native/specimen-kernel/src/engine.rs:505-552`; `native/specimen-core/tests/conformance.rs:481` | Existing tone handler and protected-number/negation test are adjacent useful source. They do not establish three-attempt retry exhaustion, full semantic equivalence or the frozen held-out tone thresholds. |
| `native/specimen-kernel/src/neural.rs:60-198,200-217`; `engine.rs:421-490` | Existing sparse network/CPU fallback/libm activation and imagination path exist. Existing encode is token hashing plus support lengths with ATP at120/121, not the frozen role/value signed feature map with ATP112-127; no active v2 typed composition decoder is exposed here. Do not label it D09 implementation. |
| `native/specimen-core/src/vision.rs:12-64,87-145` | Vision is host-owned image decode/analysis with its own fields and8192 image limit, crop focus and background-color-distance mask. This is not the selected4096/64x64 luminance/marching-squares profile; moving it wholesale into the pure kernel would import the wrong boundary. |
| `native/specimen-kernel/src/engine.rs:1018-1095`; `native/specimen-core/tests/conformance.rs:144` | Existing A review has parent linkage/one-correction handling; B finds repeated unmet input. These do not implement selected60-second/next-cycle expiry or typed relation-join proposals/current-authority approval. |

Existing test source was inspected only. No cited test was run here. The older
`execution/studio-research-v1.json` observations were read as pointers, not as
current passing results. The current scoring operational profile explicitly
leaves full ATP dynamics, tone, synthesis, aftercare and save migration separate.

## Concrete first-packet acceptance assertions

All fixtures must be newly authored synthetic mechanisms; no frozen efficacy
case/oracle is needed. Pin numeric policy before creating expected outputs.

- Valid bounded state/stimulus returns a bounded state; NaN/infinity in each
  numeric input, out-of-range state/stimulus and negative elapsed refuse without
  changing any caller value. State field names remain `v/a/cooldown_seconds`.
- Zero stimulus over known elapsed values follows exponential decay; repeated
  idle steps approach baseline. Elapsed3600 and elapsed greater than3600 take
  the same capped step; negative elapsed is never silently treated as0.
- From zero with valence1/intensity1/no urgency and dt0, obtain v=.2/a=.2;
  with zero stimulus intensity there is no valence charge. Explicit urgency adds
  .1 to a before release/clamp and does not alter permissions or work limits.
- Isolate the inclusive release edge: a=.85 at dt0 with zero stimulus releases
  to .425, v=.8*v0, cooldown30; an admissible value strictly below the edge does
  not release. Choose the bit-exact fixture according to the reviewed f32
  arithmetic policy rather than assuming decimal .85 is exactly representable.
- Entry cooldown30 with elapsed10 decays and leaves20, ignoring even urgent
  stimulus. Entry cooldown1 with elapsed2 decays and leaves0 without charging;
  the next separate step can charge. dt0 cannot consume cooldown.
- Two successive state transitions demonstrate intended buildup and release.
  Repeated transitions with identical states/stimuli/elapsed give identical
  results independent of any external seed/jitter trace. The API must not accept
  an ephemeral view as a durable state through an unchecked conversion. This
  establishes a mechanism prerequisite, not full two-cycle S24 integration.
- No source/store/network/ID/save mutation or host clock, entropy, thread, I/O,
  model, OCR or accelerator call occurs. Work and allocations are constant for
  a single transition; no arbitrary caller collection is traversed.
- Unit assertions identify D10/S24 and compare complete return/refusal values.
  A subsequent reviewed private native/WASM diagnostic must execute the API,
  check the equations independently and compare declared bits/tolerance,
  including refused inputs and boundaries. Merely reusing legacy full-cycle
  goldens or compiling wasm does not meet this assertion.

## Proposed ownership for the implementation brief

This preflight changes none of these files. Suggested first-packet ownership:

- New private `specimen-studio/native/specimen-kernel/src/atp_v3/{mod.rs,model.rs,advance.rs,tests.rs}`.
- One module declaration in `native/specimen-kernel/src/lib.rs`, coordinated with
  its current owner. Keep constructor invariants private and avoid Deserialize
  bypass. In-module tests allow the prototype to remain crate-private.
- A new, separately reviewed operational profile/report outside frozen
  contracts, for the numeric ruling and exact mechanism evidence. Parity probe
  feature/fixture/script files belong to a later explicitly owned parity task.

Leave scoring_v3, scanner_v3, orchestration_v3, language_v3, engine.rs, neural.rs,
vision.rs, command dispatch, settings, existing goldens and saved schema intact.
No root crate, dependency, lockfile, mutable observation inventory or normative
contract change is needed for this first packet. All agents must preserve the
other tranche's edits; integration into shared lib/probe files is serialized.

## Rulings and blockers before broader ATP integration or closure

1. **Private numeric profile:** AtpState declares f32 while equations do not
   specify intermediate rounding/exp width; language Charge is f64 and scoring
   consumes bounded exact rationals. Record f32/f64 intermediate/rounding policy,
   release comparison, libm entry point and parity rule before first-packet
   fixtures. Keep frozen f32 state semantics; any diagnostic wider intermediate
   choice is a reviewed local ruling. No decimal rounding or denominator choice
   into scoring is implied. This is a concrete implementation decision, not a
   request for credentials or normative ABI approval.
2. **Stimulus construction and cycle ownership:** select aggregation of multiple
   matches, overlap/longest-phrase policy, rule/source deduplication across fanout,
   eligible active/quoted/negated/conditional spans, explicit-user urgency
   without charge, cycle timing and one authoritative update. D10 urgency is
   explicit flag OR active Chargebook valence<=-.7/intensity>=.8; unknown phrase
   has zero charge. Preserve provenance. The scorer's snapshot does not decide
   any of these questions, and PreparedAction is not host authentication.
3. **Separate tonal view and text variation:** record seeded magnitude/sign draw
   mapping, endpoint/clamp behavior and consumption for default.01/brainstorm.05;
   these are different units from confidence jitter1/5. Define charge-distance
   metric, ranked-equivalent order and exact three-attempt/fallback handling.
   Preserve numeric/negated/quoted/code/procedural spans. Reward's allowed range
   is specified but how it enters the equation is not; autonomy requires traced
   internal stimulus and existing capabilities/inhibition. Do not invent either.
4. **Interface approval:** `shared-types-v1.json#task-08` requires the named
   Studio kernel/native owner's signed schema diff, exact hashes, independent
   review, versioned tagged results and producer/consumer positive/malformed
   vectors before dependent cross-package merge or wire freeze. Generic local
   implementation authorization does not supply that receipt. Private in-memory
   work can continue. Durable canonical/consumer integration separately retains
   package02's ABI-owner receipt and task09 gate; those do not block this
   isolated arithmetic prototype.
5. **Host and persistence:** actual cycles need monotonic elapsed, cancellation,
   a single update owner and transactional persistence/migration with legacy
   valence/intensity/lastUpdate/cooldownUntil preserved. Save/import/recovery and
   host capability behavior require their own receipts. This packet has no
   physical concurrency, desktop/browser UI or save/reopen evidence.
6. **Scientific closure:** active `experiments.json` is version3 and selects the
   v2 corpus/contracts; `experiments-v2.json` is historical. Tone must meet
   semantic preservation1.0, preferred rate>=.9, safety failures0, paired held-out
   lexical-family trials, independent two-rater/adjudication records, baseline
   and timing/memory limits. Freeze commit/reviewer signoff, independent holdout
   custody and contamination attestation are prerequisites; these mechanism
   fixtures supply none. Preserve all negative/timeout/error results. A bounded
   state machine cannot close S25, CH14 or P08.05 by itself.

## Following packets and why they are not folded into this one

- **Synthesis D09/P08.04/CH13/S22-S23/S26:** the next larger independent CPU
  packet is the frozen `owner_count_after_add` public-input adapter, exact128
  feature encoder, pinned network, v2 report/arithmetic decoder and total-order
  ranker. Read v1 plus explicit v2 overrides, operations-v2, network-v1 and
  near-ties-v2. No oracle/full corpus-row access; retain input/facts/i refs;
  checked i64 joins/sums;32 ingredients/128 feature tokens/256 expansions/512
  output tokens/100000 multiply-adds/64KiB graph; abstain on missing/invalid
  complete root. Preserve weights/knowledge hashes and source transformations.
  Old network API is a candidate seam, not the required encoder/rounding/ranker.
  Held-out accuracy>=.9, stronger-baseline gain>=.1, groundedness1.0 and paired
  bootstrap lower gain>0 remain independent efficacy obligations. GPU/model
  execution and fallback receipts cannot be inferred from CPU reproduction.
- **Vision D11/P08.06/CH16/S42:** a pure validated equal-array representation
  would be small but covers only S42's refusal boundary. Full next useful vision
  packet needs selected coordinates/color/luminance/contour/SDF math and CPU
  mechanism fixtures; decoded originals/OCR, scanline cancellation, image voting
  and real GPU execution remain host dependencies. Frozen IoU>=.9, OCR character
  accuracy>=.95 and independent raster/label review remain unqualified.
- **Supervision D12/P08.07/CH17/S26-S31:** a private output-bound one-correction
  window model is feasible later, but the plan places supervision after this
  research group. It must enforce60 seconds OR next user cycle, no self-review,
  parent provenance and no knowledge writes. Type B requires typed compatible
  relation joins, separate bounded100-proposal queue,2 edges/proposal,24h/parent/
  policy expiry, current-authority scoped approval and idempotent operation
  receipt. Existing repeated-input proposals do not meet that contract.
  Correction/proposal precision>=.95 also needs recall>=.8 and independent
  held-out controls; atomic durable learning remains an owner/host boundary.

## Inspected snapshot identities

These hashes identify inspected bytes only; they are not an inventory or a
passing result. Concurrent work may subsequently change them.

```text
f67633ce920f802b75109b18e2786b09648bbd4b6d174cc42995e7d45e6258e9  docs/completion/designs.json
5d51d33a6ce4adc8628f780bd913520f65e9d7f5ae6b875fe03027bd408d527b  docs/completion/shared-types-v1.json
ca85e660fd548f26bdfffccc16a16e246caa13e54006543c63ee785107910e7d  docs/completion/plans.json
511cb9f85f6651143b59f4cd6c213059a2bce505c57d86861b8f22c377c4dec5  docs/completion/studio-acceptance-v1.json
e827c9a49c72b7b074a2a8e68ef46bb6ab8cf5eb79c4f85f8ef89dd210e2afaf  docs/completion/execution/scoring-orchestration-v3-profile.md
c8a387eae542e047da64473c2f3a852e23caa6cf8b56f96d82f20e46d877ab9c  docs/completion/experiments.json
98ce45326436b9ed580a9aaa9f41ab2090f2d3f685fcaea50be0eca86a6ccba2  specimen-studio/native/specimen-kernel/src/engine.rs
bb95769e8d297546d018b70eb490e502727f0039d144b5302e45527ecd14cfbb  specimen-studio/native/specimen-kernel/src/scoring_v3/policy.rs
2ed6ce26fd2040da3db283b50b13aec0b7b478bd520b049b24df1d03c1b24c5f  specimen-studio/native/specimen-kernel/src/language_v3/model.rs
6ca1696da4dbfd1ac96c148f6c7236719c36ed5fb07f19d49aa9a273a4b3050a  specimen-studio/native/specimen-kernel/src/language_v3/prepare.rs
98c0e214f30c626282b45af3c66e7c1a0ba0633b453e79e0fad4904967585506  specimen-studio/native/specimen-kernel/src/orchestration_v3/compose.rs
eb5c197df8679f6172441ca9c6727753c2a2643ccfc87eaacffdaff1e83624b8  specimen-studio/native/specimen-kernel/src/neural.rs
09cd5de6a09aa9f293ea4d35666f6d8645d8681122cac4ebc67d02373d071f64  specimen-studio/native/specimen-core/src/vision.rs
```
