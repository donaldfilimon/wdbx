# Private typed bounded complete-episode decoder implementation plan

2026-10-20. Approved full-program task-02 continuation. Encoder/witness inventory has been reviewed and committed (dc30a88, incorporated in8c5ed7f). This private inverse grants no ABI ratification or overall task-02 completion. Dependency refresh is a separately serialized prerequisite; its final reviewed lock must be recorded before Cargo execution.

## Goal and owned surface

Decode the exact current provisional complete envelope into retained typed input and a validated complete value. Keep `v3::complete_episode` crate-private and all bytes/domain provisional. No public API, serde, transport, store/admission behavior, signing, key administration, ABI/corpus/Studio changes, new dependencies or feature. No agent or controller commits or pushes under the current direct user instruction; keep reviewed changes on main. Preserve old formats, signatures, goldens and encoders.

Own new `complete_episode/decode.rs` plus sibling `decode/` helper modules and tests; add the minimum module/re-export wiring in `complete_episode/mod.rs` and test declarations. No production type-visibility changes needed. Split Rust files before 1000 lines. Do not edit encoder mapping, field/tag tables, validation rules or hex pins to make decoder tests pass; report any mapping discrepancy for separate review.

## Exact private API

Implement and re-export within `complete_episode`:

```rust
pub(crate) struct DecodedCompleteEpisode {
    input: CompleteEpisodeInput,
    episode: CompleteEpisode,
}
impl DecodedCompleteEpisode {
    pub(crate) fn from_canonical_bytes(
        bytes: &[u8], limits: CanonicalDecodeLimits,
    ) -> std::result::Result<Self, CompleteEpisodeDecodeError>;
    pub(crate) fn input(&self) -> &CompleteEpisodeInput;
    pub(crate) fn episode(&self) -> &CompleteEpisode;
    pub(crate) fn into_parts(self) -> (CompleteEpisodeInput, CompleteEpisode);
}
```

Use a content-free Debug implementation (`DecodedCompleteEpisode { .. }`) and no Display containing values. Fields are private, accessors immutable, consumption explicit. Access intentionally discloses the already private typed input; it does not confer authority. Do not add mutable accessors or blanket public group-field access.

Source check: `CompleteEpisodeInput`, all grouped/reference types and primitive wrappers already derive Clone/PartialEq/Eq; group fields are `pub(super)` and accessible within descendants of `complete_episode`. CompleteEpisode derives Clone/PartialEq/Eq and stores only bytes/digest. Construct typed values through existing constructors (including validated measurement/monotonic/posterior constructors), then retain the input and call `CompleteEpisode::try_new(input.clone())`. This one bounded clone is the minimum change consistent with the current consuming API. Do not refactor try_new ownership to optimize it in this task. No new Clone/Eq derive is required on the decoded wrapper.

## Error contract

Define a new crate-private Clone/Copy/Debug/PartialEq/Eq error enum with `thiserror` and static messages only:

- `Canonical(CanonicalDecodeError)`: transparent static underlying syntax/resource error; preserve the exact existing enum as source.
- `InvalidShape`: wrong type/width/arity, required/unknown field, optional/estimate representation, invalid numeric conversion or other typed structural mismatch.
- `UnsupportedDomain`: present text domain unequal to COMPLETE_EPISODE_DOMAIN (wrong primitive type is InvalidShape).
- `UnsupportedVersion`: present unsigned version unequal to 1 (wrong primitive type is InvalidShape).
- `UnknownVariant`: unknown numeric enum tag, actor label or evidence label after primitive/arity shape is checked.
- `InvalidValue(CompleteEpisodeError)`: transparent underlying static constructor/final-validation failure.
- `NonCanonicalEncoding`: reconstructed canonical bytes differ from complete input.

No offsets, field paths, raw keys, identifiers, values, formatted diagnostics or parser strings in stored errors. Error Debug and complete source chains must remain content-free; both underlying enums currently have static payload-free cases. No permissive fallback or caller-supplied strings. Tests should assert the intended enum for isolated failures, but do not freeze precedence for multiply-invalid input. No new allocation-failure guarantee beyond the generic reader and existing constructors/encoder; do not claim OOM recovery globally.

## Decode algorithm and limits

1. Copy caller `CanonicalDecodeLimits`; clamp `max_input_bytes` to `min(caller, MAX_ENCODED_SIZE)` (1 MiB). Do not raise any caller limit. Zero semantics remain existing reader semantics, not an alias for defaults. Use `CanonicalValue::from_canonical_bytes`, not EpisodeCommitment's old-envelope reader. Preserve fixed nesting maximum 32, caller value-count/string/collection caps, shortest arguments, canonical map ordering, duplicate-key rejection and complete input consumption.
2. Consume the bounded owned tree. Require exactly integer keys 0..11, correct provisional domain and version 1. Require exact allowed/required nested text keys. Resolve all numeric tags and textual labels with explicit exhaustive inverse matches, never Rust discriminants. Numeric signed fields accept canonical unsigned nonnegative values only when representable in their signed target type; negative values only for signed slots. Width conversions are checked, bool is not integer, null is not omission.
3. Check typed collections' lengths before allocating typed output vectors, cloning or mapping entries, including nested actions/parameters and posterior. Check bounded token byte lengths before constructor-owned string allocations. Move generic strings/bytes where constructors allow; do not alter constructors for this task. Reuse existing cap values (actions128, parameters32, resources/costs16, capabilities/models/calibrations32, approvals/holds/causal64, edges64, consulted/source/observations/provenance256, posterior1..32) and existing validations. Set identity/slot/scope/ranges remain defined by current source. Resource kind uniqueness limits actual positive realizable resource sets to5.
4. Build the retained CompleteEpisodeInput through existing constructors. Call try_new on its bounded clone for all relational validation, size and self-reference checks. Compare returned canonical bytes against original input, reject any difference, return the retained input and episode only on exact equality. This rejects unsorted semantic arrays that the generic CBOR reader legitimately accepts. Never sort/de-duplicate malformed input to grant success.

`max_parent_count` remains an existing old-envelope-only policy field, inert in generic parsing; the complete decoder uses its own existing causal/consulted caps. Document this explicitly on the new method; do not reinterpret the field or change the old reader. Generic value-count includes map keys and values; string and collection limits apply independently. Typed caps govern typed allocation only: generic parsing already allocates its bounded tree. Peak memory includes generic tree, retained input, clone and canonical buffers, and is not equal to1MiB. No streaming/zero-copy claim.

## Red-first tests and acceptance

Use existing stable Rust `tests/fixtures.rs` minimal/populated builders, reviewed hex pins and focused coverage builders. Tests exercise real production decoder, not a test-only implementation. Once Task2's final corpus is independently reviewed, add all its valid mapped bytes as additional decoder inputs without inventing a new fixture interchange or editing its spellings here.

- First demonstrate meaningful failure for absent decoder; then implement. Minimal/populated decode must retain every typed leaf: compare complete input using existing PartialEq/Eq and compare bytes/digest against pins and original encoder output. Fixture list insertion order may differ from canonical set order: canonicalize only expected set-like fields in an explicit test helper using documented field-specific ordering, not by decoding the expected input or replacing equality with byte equality. Keep action order exact. Add paired permutation tests: differently ordered valid set inputs encode identically and decode to the same complete typed input; distinct action sequences remain distinct. Expected normalization must cover version component ordering, parameter lexical order and posterior regime order as well as canonical-byte-sorted sets.
- Cover every tag/label, all four scopes, both consulted forms/reference domains, all eight evidence slots, every artifact kind, each retention variant, UUID v4/v7, signed boundaries, all-zero digest, absence versus present, Unknown versus knownzero/knownempty. Assert typed identity and values, not merely encode/decode agreement. No semantic trust inference from C0..C7 or signer spelling.
- Raw malformed syntax corpus: duplicate/unordered map keys, nonminimal integers/lengths, indefinite/reserved arguments, tags/floats, malformed UTF8, truncation, trailing data, exhaustion of each caller limit and nesting. Old envelope and wrong domain/version reject. Generic Bool/Null parse capability must never imply typed field acceptance.
- Construct syntactically canonical malformed trees to target typed rules: missing/extra fields at every structure, wrong primitive/byte widths, null optional, estimate and variant arity, unknown label/tag, duplicate/unsorted semantic collections, scope mismatch through every local/artifact slot, wrong artifact kinds, invalid identifiers/UUID/signer spelling, range/expiry/posterior and measurement failures. Unsorted distinct set trees specifically yield NonCanonicalEncoding after validation, not normalized success. Duplicates retain their validation rejection.
- Typed cap and cap+1 using valid unique members for each realizable list; check impossible length declarations fail before generic reservation and typed output allocation starts only after typed cap checks. Smaller injected limits are distinct from production1MiB evidence. Test fixed-depth rejection, extreme declared lengths, zero caller limits, exact actual size success versus actualsize-1 failure. No huge caller limit bypasses complete input cap. Deterministic hostile corpus must not panic.
- Existing self-reference helper injection remains appropriate; no false fixed-point fixture. Diagnostics tests inject sentinel strings/digests/timestamps and cover wrapper Debug, all error cases and source chains. Immutable accessors/into_parts agree with decoded values.
- Run focused complete_episode tests; existing canonical reader, legacy candidate/edge, signing/replay and Python witnesses unchanged; final full `bash tools/check.sh`. Keep exact red/green commands, reviewed artifact hashes and source inventory in task report. No signing/admission claims from decode success.

## Evidence and review boundary

Independent Python encoding plus Rust typed decode is not an independent typed decoding witness. A bounded Python typed decoder and differential malformed raw vectors remain outstanding unless separately specified/reviewed and implemented. Report this explicitly; do not mark task-02 complete. Missing witness interpreter stays a failure under existing witness conventions, never a skip.

Require fresh independent specification/quality review against the final artifact inventory before calling this slice complete. Public promotion still needs ABI schema-owner agreement, signed schema diff, artifact hashes and shared producer/consumer positive/negative vectors per shared-types task-02. This brief grants no agreement, corpus refresh, wire freeze, consumer merge, COSE/trust lifecycle design or production evidence.


# Binding implementation clarifications

The following source preflight resolves ambiguities above. These explicit clarifications take precedence over superseded proposed wording. Additive witness compare-hook ownership is authorized as specified here; old assertions and fixture schemas remain intact.



Retain the proposed crate-private `DecodedCompleteEpisode { input: CompleteEpisodeInput, episode: CompleteEpisode }`, private fields, static redacted Debug, immutable `input()`/`episode()`, consuming `into_parts()`, and associated `from_canonical_bytes(&[u8], CanonicalDecodeLimits) -> std::result::Result<Self, CompleteEpisodeDecodeError>`. Explicitly qualify Result: the parent module's Result alias only supports CompleteEpisodeError. No new model derives or visibility changes are needed. Child modules can access fields declared pub(super) in model modules. Existing CompleteEpisode derives Clone/Eq and owns only bytes/digest, so retaining input requires the proposed bounded clone into `try_new`.

1. Clamp only max_input_bytes to min(caller, MAX_ENCODED_SIZE); generic-reader parse remains the first stage. All other caller limits and zero semantics survive unchanged; max_parent_count is inert here, including zero. Generic reader depth is root=0, rejects depth >32, counts keys and values, validates complete input, and supports Bool/Null only as generic values.
2. Consume owned tree, require exact top-level integer keys 0..11 and exact nested field sets, convert through existing constructors, enforce typed collection caps before typed vector capacity/mapping work.
3. Retain typed input in received order. Invoke `CompleteEpisode::try_new(input.clone())`; compare its canonical bytes with the entire original slice. Return only after equality. No decoder-side sorting, deduplication, or permissive success. Existing constructor/encoder may sort its own representation; final equality prevents that normalization from accepting malformed wire input.
4. Using `episode.canonical_bytes()?` for comparison clones its bounded bytes; document this additional buffer. A descendant can alternatively compare the parent's private byte field without changing visibility; choose the latter if desired, but do not expand the existing API solely for this optimization. Neither path is streaming/zero-copy or global OOM recovery.

## Resolve error ambiguities before dispatch

Keep proposed payload-free error enum and static thiserror messages. Freeze these isolated-case mappings:

- Generic syntax/limit errors propagate as `Canonical(original_error)` without remapping. A CBOR negative below i64::MIN therefore fails in generic parsing, not typed conversion.
- Missing/unknown field, nontext nested key, wrong primitive/byte width, wrong arity, null optional, integer overflow into the destination Rust type: InvalidShape.
- Present text domain mismatch: UnsupportedDomain. Present unsigned version other than 1, including zero: UnsupportedVersion. Wrong primitive is InvalidShape. Exact root-shape checks may reject old five-member envelopes before checking domain; do not promise old-envelope error precedence.
- For normal enum arrays, first enforce generic family shape: unit family exactly `[unsigned]`; Scope/Consulted exactly `[unsigned,map]`; Retention one or two members with unsigned first and map second if present. Unknown tags in otherwise valid family shapes give UnknownVariant; known tags enforce their exact arity/fields. Actor/evidence wrong text label gives UnknownVariant; wrong primitive gives InvalidShape.
- Estimate is a special representation: accept only `[0]` or `[1,value]`; any other tag/shape gives InvalidShape. This resolves the brief's conflict between “estimate representation” and “unknown numeric enum tag.” Do not treat Estimate as a regular enum fields-map.
- Invalid SignerKeyId::parse spelling maps to InvalidValue(InvalidIdentifier), consistent with other identifier constructors; parse returns Option, not CompleteEpisodeError. Signer is mandatory, not the superseded optional design field.
- Ordinary typed list cap violation maps to InvalidValue(CollectionLimit); posterior length outside 1..32 maps to InvalidValue(InvalidPosterior), matching its existing constructor. Invalid UnitPpm within u32 but >1,000,000 maps to InvalidValue(InvalidRange); unsigned >u32::MAX is InvalidShape. UUID semantic failures map to InvalidValue(InvalidUuid); fixed byte width failures are InvalidShape.
- Constructor/final relational failures map to InvalidValue(original_error), including duplicates, scope/kind/range and self-reference errors. Unsorted otherwise-valid semantic sets give NonCanonicalEncoding. Do not freeze precedence for multiply-invalid values.

`#[error(transparent)]` forwards the inner error's source chain rather than guaranteeing that the wrapped enum itself appears as a source. Both current underlying error enums are payload-free. Tests must check whatever complete chain actually exists for redaction; do not require a nonempty source chain or change legacy errors.

## Exact mapping reminders

Use actual canonical.rs and canonical/mapping.rs, not JSON fixture spelling. Numeric unit variants are singleton arrays, not bare tags. Scope Guild/Organization fields are guild/organization; both consulted forms contain reference; posterior Known payload is a bare array of `[regime_text, unsigned_ppm]` pairs. Actor labels are snake_case while fixture actor names are PascalCase. Evidence labels are C0..C7. Source refs permit every defined artifact kind; all eight evidence slots require Evidence. Foreign origin organization equality is checked only for an Organization origin scope; import receipt always uses local binding.

Ordinary sets sort by **lexicographic whole canonical item bytes**, NOT CBOR map-key length-first order and not `(encoded_length, bytes)`. Version sets sort component ASCII bytes; parameter classes sort token ASCII bytes; posterior sorts regime ASCII bytes; actions retain sequence order and may repeat. Preserve all these independently in expected-value normalization.

## Test ownership correction and finite corpus integration

The brief currently owns only new decoder files plus declarations, yet asks to exercise the final finite witness corpus. Its cases are local variables in draft_corpus tests, so the smallest correct integration is explicitly authorizing an additive hook in existing `tests/draft_witness.rs::compare` after witness/Rust bytes agree. Preserve all existing assertions and fixture/interchange behavior. Pass its already-produced bytes through the production typed decoder, compare bytes/digest, and compare retained typed input against an independently normalized copy of `draft_adapter::input(v)`. This reaches both vocabulary/boundary and permutation/action suites without duplicating corpus construction or changing Python. Own a new test-only normalization helper and a minimal import/call in compare. Pin source inventory before/after this intentional hook change.

The expected normalization helper must list every collection field explicitly: context.resources; outcome.costs; authority capabilities/models/calibrations and approvals; evidence source_refs/observation_refs; lifecycle holds/edges; provenance causal_parents/consulted_parents/records; each action.parameter_classes; known posterior entries. Use existing test encoder only to derive item-byte sort keys for ordinary sets; direct ASCII comparison for the three exceptions. Never derive expected input by decoding output, and never replace typed Eq with only byte Eq. Preserve actions including repeated descriptors. Existing dedicated Rust fixtures still provide primary minimal/populated typed equality plus immutable accessor/into_parts tests.

All existing finite corpus cases are designed valid; `compare` already calls the real constructor and fails on invalid cases. This additive integration is Rust typed decode over independently encoded Python bytes, NOT independent Python typed-decoder evidence. Do not claim exhaustive malformed cross-language differential coverage or completion of task-02.

## Allocation and boundary evidence corrections

Generic parsing allocates its bounded owned tree before typed checks. Typed cap checks can only govern subsequent typed collection allocation/traversal. Prefer one shared checked-array helper taking cap plus the element mapper, checking length before reserving typed output or invoking the mapper. Tests can count mapper calls and confirm zero on cap+1; that proves no child conversion, not the absence of every allocation in the entire decode. If claiming reservation sequencing, use a narrow test-only observation at that helper's reservation boundary or inspect the control flow and report source evidence. Do not add unsafe/global allocator instrumentation. Generic impossible-declared-length pre-reservation checks already exist in Reader::collection; raw decode error results alone do not dynamically prove no reservation.

Realizable positive caps: actions128 (sequence repetitions allowed), parameters32 unique, versions32/category unique components, approvals/holds/causal64 unique, edges64 unique complete links, consulted/source/observations/provenance256 unique, posterior32 with total1,000,000 and zero probabilities permitted. Resource/cost sets have only five unique kinds; test five succeeds, six with repeated kind fails duplicate, and17 fails collection limit before converting entries. Do not claim a valid16-resource boundary.

Separate exact-size caller-limit tests (`bytes.len()` succeeds, one less fails InputLimit) from production-cap proof. A raw input of 1MiB+1 with huge caller max_input_bytes must produce Canonical(InputLimit) before syntax parsing, demonstrating the complete-decoder input clamp; this need not be a valid complete episode and must not be described as an encoder production-size boundary fixture. Test every independent generic limit and max_parent_count=0 explicitly. Overlarge declarations, reserved/indefinite forms, depth33, trailing bytes, and malformed scalar cases belong in raw hostile tests; typed caps and semantic ordering require syntactically canonical trees.

No OOM-inducing allocation-failure test is required. All error variants can be directly constructed for redaction testing; do not claim every variant was induced through real ingress. Deterministic hostile corpus nonpanic evidence is finite, not fuzzing/exhaustive proof. Existing self_reference_probe::run can check decoder propagation of final constructor rejection without a synthetic hash fixed point.

## Suggested bounded implementation deliverable

One additive private decoder module split into helpers for scalar/container extraction, vocabulary/references, and ten-group construction; new decoder tests split into roundtrip/vocabulary, malformed shape/semantics, bounds/diagnostics; minimal wiring and the explicit witness compare hook above. Keep each Rust file <=1000 lines. Do not alter model validation, existing encoders, pins, Python fixture schema, legacy formats, dependency locks, or consumer files.

After controller clears lock reconciliation: establish meaningful absent-API RED, implement, run focused complete_episode tests including the witness hook and raw hostile cases; fresh independent spec/quality review; controller owns full tools/check.sh plus unchanged legacy evidence. Report exact commands/status/artifact hashes, historical-versus-new evidence clearly, and remaining independent typed Python decoder/public ratification boundaries. No Cargo was run in this preflight.
