# Private ATP advance profile · logicalf32-v1

This is a local implementation ruling for the approved completion plan's D10,
P08.05/CH14 and partial S24 mechanism. Frozen completion contracts remain binding.
The task-08 named-owner interface gate remains open: this prototype introduces
no public kernel ATP API, shared serialized command/result, version negotiation,
wire freeze, consumer cutover or owner approval. Tests and an opt-in white-box
diagnostic may privately include the same source; that is not producer/consumer
interoperability. Legacy engine, saves, IDs and existing goldens stay unchanged.

The private immutable state uses finite f32 `v` in [-1,1], `a` in [0,1] and
`cooldown_seconds` in [0,30]. A validated stimulus uses finite f32 valence in
[-1,1], intensity in [0,1], and supplied boolean urgency. Constructors and outputs
canonicalize signed zero to positive zero. No Deserialize or unchecked field
construction crosses the module boundary. Negative or nonfinite f32 elapsed is
refused; negative zero is zero. Admitted elapsed is capped at 3600 seconds.

Every named arithmetic operation and literal below is f32, with explicit
intermediate rounding, no `mul_add`, f64 intermediate or decimal rounding:

1. `dt=min(elapsed,3600)`; `d=libm::expf((-dt)/180)`;
   `v0=d*v`, `a0=d*a`.
2. If cooldown is positive at entry, return only decayed `v0/a0` and
   `max(0,cooldown-dt)`. Crossing zero does not charge in this transition.
3. Otherwise `vcharge=(.2*valence)*intensity` and
   `v1=clamp(v0+vcharge,-1,1)`;
   `abase=a0+(.2*intensity)` and `a1=clamp(abase+(.1*urgency),0,1)`.
4. At `a1>=.85f32`, release to `v=.8*v1`, `a=.5*a1`, cooldown30.
   Otherwise retain `v1/a1`, cooldown0.

The result identifies applied elapsed, entry-cooldown and release disposition.
One transition has constant scalar work and no allocation, clock, entropy, host,
thread, I/O or effect. Repeated calls with identical inputs are identical. No
seed or ephemeral tonal-view input is accepted into this state transition.
This does not implement a tonal view or demonstrate integrated zero-jitter drift.

The chosen f32 operation order is a controller ruling, not ratified shared
numeric semantics. Its cost is later reconciliation if the named owner selects
another numeric profile. Source-included native/WASM fixtures must execute the
same prototype, compare exact f32 bits, independently check equations with a
stated libm-versus-host-exp tolerance, and reject corrupted normal-comparator
inputs. No efficacy corpus is used for these synthetic mechanism checks.

Stimulus aggregation/deduplication, active/quoted/negated span selection, urgency
inference, f32-to-rational scoring conversion, authoritative cycle timing,
transactional persistence/migration, tonal views, equivalence-constrained tone,
reward/autonomy and S25 are separate work. Actual hosts must supply monotonic
elapsed and current authorization; supplied urgency widens no scope or budget.
All163 requirements remain open/C0/unknown. Scientific efficacy, independent
contamination controls/adjudication, installed systems, OCR/models/GPU and elapsed
operation need their own evidence. Recorded arithmetic is not qualification of
any of these broader clauses.

The diagnostic uses an isolated offline temporary Cargo workspace with exact
libm/serde_json versions from Studio's unchanged lockfile and original source
paths. Its feature/default artifacts are standalone research artifacts, not
Studio runtime artifacts. No existing dependency graph or probe is changed.
