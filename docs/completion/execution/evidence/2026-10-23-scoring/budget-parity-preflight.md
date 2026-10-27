# Budget parity preflight — 2026-10-23

Ruling: retain the current labelled semantic-projection probe as bounded C0
evidence, but do not treat it as portable admission/completion parity. Before a
consumer adopts a same-input/same-limits deterministic contract, replace the
target-dependent charges in these new budget modules with canonical conservative
logical charges. This is a local prerequisite for that contract, not a reason to
invalidate the probe's narrower results or promote any requirement. All 163
requirements remain open/C0; no production adoption is established.

The inspected frozen design permits explicit admission budgets and Backpressure
(`docs/completion/designs.json:47,146-149`); it does not specify a canonical byte
formula. Thus target-local budgets are not independently proven to violate those
lines. They are a legitimate alternative ONLY with an explicit contract that
identical caller budgets can produce different errors, admitted work and results
across targets. The operational profile's current disclaimer at
`docs/completion/execution/scoring-orchestration-v3-profile.md:159-166` excludes
byte-accounting/allocator parity, but should explicitly exclude tight-boundary
decision parity too. Its “logical work” terminology (85,99) otherwise invites a
stronger interpretation than the implementation supports.

Exact decision-bearing sites (paths below relative to
`specimen-studio/native/specimen-kernel/src/`):

- `scoring_v3/budget.rs:67-76`: `Work::items<T>` subtracts
  `len * size_of::<T>()` from payload. Rule-reference strings (81), captures
  (102), expression/argument containers (129,145), alternatives (156), and
  prepared-action metadata (168-191) therefore have layout-dependent payload
  admission. This helper's work charge is only item count, so its immediate
  target-dependent check is payload, not work.
- `orchestration_v3/budget.rs:22-35`: `Meter::items<T>` feeds the same
  layout-dependent product through `add`, which consumes operation work AND
  increases bytes. `contribution` separately charges
  `size_of::<Contribution>()` at 181-182. Nested metadata remains subject to
  the generic charge; replacing only the Contribution charge is insufficient.
- `orchestration_v3/broker.rs:265-273,305-308`: job admission traverses
  comparison metadata using this meter before insertion. Identical descriptors
  can therefore receive success on one target and Backpressure on another at a
  tight operation-work limit even though job storage uses fixed `JOB_BYTES`.
- `orchestration_v3/broker.rs:427-433,454-456`: completion meters a contribution,
  checks owned-byte reservation (219-224) and cumulative cleanup work reservation
  (226-235), then retains the charge. Tight limits can leave one target's lease
  incomplete while the other completes it, affecting subsequent sealing and
  plans. `Counts.owned_bytes` (145-151) merely exposes part of that difference.

If all other checks pass and a limit falls between the two calculated charges,
the smaller-charge target accepts while the larger-charge target refuses. This
is a source-derived risk; no boundary reproduction or suite was run for this
ruling. Removing a diagnostic from JSON cannot remove these branches.

Smallest correction: define explicit fixed per-type metadata charges for the
types traversed by these two budget modules (including string/vector headers,
spans, enum records and Contribution), use checked count multiplication, and
retain existing recursive UTF-8/payload-length charges and bounds. Share the
charge definitions where both modules account for the same type. Choose and
document conservative constants against the supported native and WASM layouts;
do not derive them from the compiling target's `size_of`, padding, or a pointer
width multiplier. Preserve broker reserve/release symmetry. No identity,
persistence, scanner or producer change is required.

Concrete private scheme: a version-labelled `LogicalChargeV1` trait with a named
`UNIT_BYTES` constant for each traversed concrete type, and
`items<T: LogicalChargeV1>` retaining each module's current work/payload policy.
Use a named `CONTRIBUTION_BYTES_V1` for the direct scalar charge. No blanket
implementation using Rust layout. Exact constants require checking the concrete
types; this narrow review does not invent sizes or claim to have inspected their
definitions. In addition to `String`, `RouteCapture`, `Expr`, `Vec<Expr>` and
`Contribution`, the implementation must cover the concrete element types of:

- Scoring: `ActionAst.alternatives` (156); prepared `tokens` (168), token
  `rule_refs` (172), `captures` (177), `operators` (181), `definitions` (186),
  `charges` (191), and `SemanticAst::Action` (206).
- Orchestration: `Origin.rule_refs` (56), capture bytes (75), action
  alternatives (116), comparison token spans/dependencies (133,135), candidate
  spans (166), contribution spans/dependencies/inhibitions (188,190,191), election
  weights/refused alternatives (195-196), score draws (197), and evidence
  component left/right spans (204-205). Capture and expression traversal also
  use the shared helpers (63,86,97).

The compiler's explicit trait bounds make missed concrete types visible without
changing scanner/producers or deriving sizes. Primitive byte payload stays one
unit per byte; strings keep their UTF-8 traversal. Pin named profile charges and
exercise their admission effect at charge-minus-one/exact-charge boundaries on
both targets, including unchanged broker state on refusal. Checking only a table
constant or diagnostic equality would not establish that decision behavior.

Call these logical accounted bytes, not measured physical memory. The existing
model already mixes fixed broker/job/cycle charges with traversal of borrowed
comparison data; `size_of` does not establish allocator overhead, collection
capacity or process peak memory. Canonical accounting restores reproducible
model decisions, not a proven 16 MiB physical-memory ceiling. Its cost is earlier
refusal on smaller-layout targets plus maintaining the explicit charge table.
If a physical ceiling is needed later, qualify separate host measurements and
resource enforcement. Subsequent validation should cover identical lowered
payload/work/owned/cleanup boundary decisions on both targets, including the
atomic refusal state; this preflight performs no kernel edit or tests.
