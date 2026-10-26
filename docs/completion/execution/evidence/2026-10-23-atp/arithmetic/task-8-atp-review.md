# Task 8.atp independent arithmetic review

2026-10-23. Reviewed the supplied four-file, 522-line ownership diff once against
`task-8-atp-brief.md`, the implementation report, root and Studio `AGENTS.md`,
`docs/completion/execution/atp-v3-profile.md`, frozen `designs.json` ATP/D10 and
`shared-types-v1.json` AtpState/task-08. This review covers the private arithmetic
packet only. No source was edited and no green suite was rerun.

**Specification verdict: PASS for the bounded private arithmetic task.**
**Quality verdict: PASS with one nonblocking Minor test improvement.**
Critical findings: none. Important findings: none.

## Findings

- **Minor — add an order-sensitive binary32 charge fixture.**
  `specimen-studio/native/specimen-kernel/tests/atp_v3.rs:124` exercises zero
  intensity, urgency, and fractional charge, but the nonzero charge examples use
  valence/intensity factors 0, ±1 or powers of two. They do not distinguish the
  required `(0.2_f32 * valence) * intensity` from regrouping or a wider
  intermediate. The implementation at `src/atp_v3/advance.rs:54` is correct;
  this is a regression-coverage gap, not a current arithmetic defect. Add an
  independent expected-bit fixture from zero state with elapsed0,
  valence0.1_f32, intensity0.9_f32, urgencyfalse: ordered valence charge is
  `0x3c9374bd`, whereas `0.2_f32 * (valence * intensity)` and a single final
  rounding of the widened products yield `0x3c9374bc`. I derived these values
  with standard-library Python `struct` binary32 rounding in a focused scalar
  calculation; no implementation or test was mutated. An arousal fixture with
  all three nonzero summands would similarly pin the named addition order.
  Controller disposition: carry this M1 into the next private native/WASM
  diagnostic as required future fixture coverage with an independent JavaScript
  `Math.fround` expectation. No arithmetic source change is needed for this
  review. That future fixture has not yet executed and is not credited here.

## Specification and mechanism assessment

The constructor paths in `model.rs:19` and `model.rs:72` reject nonfinite and
out-of-range fields with separate typed errors, normalize negative zero, and
retain private fields with immutable Copy inputs/getters. No unchecked external
constructor, Deserialize or serialized envelope is introduced.

`advance.rs:39` refuses nonfinite/negative elapsed before capping, accepts -0,
normalizes applied elapsed and caps finite elapsed at3600. Lines46–67 use f32
operands/constants, `libm::expf`, separately written decay, charge and arousal
operations, clamp before inclusive `>=0.85_f32` release, and the required release
scales/cooldown. The entry-cooldown branch applies decay and cooldown subtraction
only, even when the step crosses zero. Transition getters expose state,
applied_elapsed, released and cooldown_at_entry without input mutation.
The validated output constructor normalizes underflow-produced negative zero.
All admitted operations remain bounded, so its internal `expect` has no
identified valid-input panic path.

The scalar path has constant work, no dynamic collections/allocation, I/O,
clock, entropy, threads, host/model/GPU call or hidden mutable state. Urgency
remains supplied data. The module uses private source inclusion and pub(super)
access only; a focused check of kernel lib.rs found no atp_v3 registration.
Nothing in this diff adopts the mechanism into the public kernel, command API,
engine cycle, persistence, saves, IDs or scoring conversion.

Frozen D10 is broader than this packet. The report/profile correctly preserve
task-08's named Studio kernel/native and ABI interface gates and the allowance
for private prototyping. The local operation-order ruling does not ratify a
shared numeric or wire contract. There is no S24/S25, CH14, P08.05 or broader
acceptance promotion.

## Evidence assessment

- The four live files, independent review snapshots, arithmetic source copies
  and source-freeze.json hashes all agree. All packet-file hashes agree with
  source-freeze.json; the working and durable reports are byte-identical and
  match its report hash. Review diff SHA-256:
  `815abf56f1fe6146363b9f6a1ce4dfc3af186d4eba8a966b2ae76ed30a0611c6`.
- TDD receipts show assertion failures rather than setup/compiler failure:
  admission RED 0passed/3failed, then GREEN3; transition RED3passed/11failed
  against the retained no-op scaffold, then GREEN14. The 14 final tests
  meaningfully cover each field/refusal, endpoint and signed-zero admission,
  elapsed cap, independent decay bits, dt0 charge, urgency, inclusive release
  and adjacent floats, clamps, entry cooldown/crossing, immutability and repeated
  buildup/release/idle calls. The Minor finding above does not negate these
  behavior checks.
- Kernel-tests.log independently sums to **254passed, 0failed, 0ignored** over
  27 result summaries, including the 14 ATP tests. Kernel Clippy's receipt is
  `--all-targets -- -D warnings`, exit0. Kernel fmt and explicit private-file
  rustfmt receipts exit0. The final command receipts pin Rust1.99.1, offline
  Cargo, toolchain PATH/DYLD_LIBRARY_PATH, and unchanged before/after source
  hashes. No final warning is recorded.
- Both preservation receipts contain118 matching expected/actual hashes; a
  fresh hash-only comparison also matches all118 live files. Tracked-before
  and tracked-after diff hashes match. Existing source content outside the
  bounded review was not re-reviewed.

## Cannot verify / remaining boundaries

This review verified code and retained receipts, not a fresh Rust execution or
the historical ordering of actions beyond those receipts. Native/WASM execution
and exact-bit comparison are future work and are **not complete** here. Root
and full Studio gates, host monotonic timing/cancellation, active-clause and
aggregation rules, tonal views/equivalence/retries, reward/autonomy, transactional
persistence/migration, owner approval, interoperability, efficacy and hardware
qualification cannot be established by this packet. Repeated scalar identity
checks establish no hidden scalar drift, not integrated zero-jitter or S24
acceptance. No frozen efficacy corpus or live store was accessed.
