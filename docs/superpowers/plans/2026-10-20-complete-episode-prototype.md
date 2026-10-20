# Complete Episode Private Prototype Implementation Plan

> Execute task-by-task with fresh implementer and independent specification/quality reviewer. No public interface/domain freeze before ABI agreement.

**Goal:** Implement and test a real crate-private typed complete-episode shape and draft canonical mapping, without exposing a competing ABI contract.
**Architecture:** Add pub(crate) v3::complete_episode with private validated fields and crate-private methods. No serde, transport, store, admission, signing, migration or new dependency. The private prototype exception in shared-types-v1 permits this work; it does not close task02.
**Tech Stack:** Rust1.99.1, edition2024, existing canonical primitives/SHA256; independent standard-library Python draft witness in a subsequent task.
**Spec:** Approved WDBX completion program; normative constitution and canonical episode specification; docs/completion/shared-types-v1.json gate02. The design and correction appendix below are the exact local prototype contract, with corrections taking precedence.

## Global constraints

Preserve existing encoders, v2/v3 store formats, candidate/edge digests, signed bytes and old goldens. No ABI/corpus/Studio changes. No new public consumer API, feature flag, serde or dependencies. Canonical checkout; source changes stay unstaged/uncommitted. Private prototype bytes/domain are provisional and not a published compatibility commitment. Every Rust file <=1000 lines. All diagnostics of new types/errors are redacted. Caller-supplied allocations are outside the bounded mapping guarantee. No live data or external side effects.

## Review focus

Scope equality in every nested local reference; required signer identity without pretending key validation; explicit Unknown versus zero/absence; bounded size counting before allocating the canonical tree; private type diagnostics cannot delegate sensitive legacy Debug; every set identity and action order follow corrected design.

## Task 1: Typed shape and bounded canonical draft

**Files:** new crates/abi-wdbx/src/v3/complete_episode/ modules; additive private module declaration in v3/mod.rs only. Split primitives, references, grouped fields, validation, sizing/canonical mapping and focused tests by responsibility.
**Interface:** crate-private CompleteEpisode::try_new(CompleteEpisodeInput)->Result<CompleteEpisode,CompleteEpisodeError>; canonical_bytes and digest as designed; primitive constructors and immutable fields/accessors. No downstream consumer can import this module.

- Write meaningful failing unit tests for absent API, then implement all fields/variants and corrections below. Unit tests exercise real private production code.
- Validate shape/counts/ranges/uniqueness/scope/kinds before canonical mapping. Exact checked size preflight before constructing CanonicalValue tree and bytes. try_new must reject invalid input; subsequent encoding cannot discover input-invalid states.
- Cover all red-first expectations in design/audit. Draft exact fixture bytes/digests may be pinned as local regression evidence only; independent witness follows Task2.
- Run focused tests, Clippy/format/size checks. Report honest RED/GREEN and exact source SHA inventory. Controller owns final fullgate after combined slice. No commits.

## Task 2: Independent draft witness and decode qualification

After Task1 review, define a test-only synthetic fixture interchange with an independent Python mapper, not a wire API. Independently compute minimal/populated draft bytes/digests and every tagged vocabulary/scoped form; validate malformed bounded new-domain parsing if implemented in this slice. Existing witness commands/modules unchanged. Observe missing/malformed witness as failures, never skips. Freeze an exact brief before dispatch; no public promotion.

## Design appendix (superseded where correction appendix differs)

# Complete episode encoding-input design

Status: proposed implementation design, C0 only. 2026-10-20. This document
specifies a pure additive Rust commitment input; it does not publish an ABI
wire schema or authorize writes. All numeric tags and bounds below are proposed
for this new domain only. Freeze them with independent vectors before release.

## Source and ownership

Read against root AGENTS.md, `crates/abi-wdbx/src/v3/commitment.rs`,
`v3/episode/types.rs`, `v3/episode/store/canonical.rs`, store and signing APIs;
ABI's `2026-08-22-spec-canonical-wdbx-episodes.md` and constitution section 5
and decision register; and the in-progress `docs/completion/sources/` copies.
The copies are completion evidence inputs, not a replacement normative source.
The ABI canonical-episode spec is explicitly proposed, with dated implementation
updates; the current source determines what is shipped. The approved completion
program explicitly requires a new domain and authenticated tenant/scope binding.

Today the five-entry `abbey-cbor-episode-v1` envelope is a general encoding input;
`StoredRecord::computed_digest` maps a smaller operation event into it. Its
single previous-operation digest is not a complete causal/consulted graph.
`EpisodeWrite` is a guild-scoped operation request, not the complete episode.
Existing signatures are detached writer attestations and do not authenticate a
source or establish truth. Do not widen `EpisodeWrite`, `MemoryCandidate`,
`MemoryEdge`, `StoredRecord`, their mapping, or the frozen `EpisodeCommitment`.

ABI Program 1 owns language-neutral schemas/fixtures and compatibility. WDBX
Program 4 owns canonical mappings and durable semantics. Before any transport
exposure, ABI must ratify the versioned wire envelope, identifiers, enum tags,
units, missing-value rules, authority references, version negotiation, bounds,
and fixture corpus; then vendor only through ABI's existing vendor tool. A local
Rust API is not implicit permission to create a competing transport schema.

## Minimal independent slice

Add `v3::complete_episode` with `mod.rs`, `types.rs`, `validate.rs`, and
`canonical.rs` (split before 1,000 lines). Introduce private-field validated
newtypes and a `CompleteEpisodeInput`/`CompleteEpisode` pair, plus explicit
canonical mapping and digest. Inputs carry declarations and references, never
an `authorized: true` or `trusted: true` assertion. No serde derives in this
slice; no JSON, transport, filesystem, signer, key resolver, store append or
migration. The existing bounded reader work stays independent; a reader that
expects the old domain must reject the new envelope rather than reinterpret it.

This slice is useful independently: it can prove field coverage, bounded shape,
scope binding of local references, reproducible bytes, and domain separation.
It cannot prove reference existence, authenticated identity, causal DAG validity,
current policy/consent, payload deletion, storage isolation or admission.

## Proposed types and bounds

All fields below are mandatory unless `Option` is explicit. Private fields are
constructed with fallible constructors; validated collections cannot be mutated
through public references. No raw `CanonicalValue`, arbitrary metadata map,
free text summary, untyped JSON or floating-point number enters the public model.

Primitives:

- `EpisodeId([u8;16])`: non-nil RFC 9562 UUID, RFC variant and version 4 or 7;
  encode its 16 bytes, not textual case variants. Task and request IDs use the
  same constrained UUID newtype (separate Rust types).
- `OpaqueRef`: 1..128 ASCII bytes `[A-Za-z0-9._:-]`, no whitespace, paths or URI;
  distinct wrappers for tenant, organization, principal, scope, source node,
  task regime, clock domain, component, action and parameter class. These are
  pseudonymous tokens; validation cannot itself prove de-identification.
- `Digest32([u8;32])`: fixed SHA-256 bytes. Do not reserve zero as deletion or
  absence; every digest value is representable. Optionality uses `Option`.
- `UtcMillis(u64)` and `MonoNanos(u64)` with an explicit opaque clock-domain
  reference. Monotonic start <= end; no comparison across clock domains.
  Acquisition and receipt UTC times need not be ordered (clock skew is evidence).
- `UnitPpm(u32)`: 0..=1_000_000 for probabilities, risk and novelty;
  `SignedMicros(i64)` for utility and dimensional quantities, fixed scale 10^-6.
  Values outside fixed domain fail; no implicit rounding/clamping or NaN path.
- `Measurement { unit: MeasurementUnit, value: SignedMicros }`, where units
  are `Unitless=0, Count=1, Milliseconds=2, Bytes=3, Microjoules=4`.
  Count/bytes require integral nonnegative values. Bounds are checked before
  arithmetic. Additional units require a schema revision.
- `Estimate<T> = Unknown | Known(T)`: explicit uncertainty, separate from zero.
  Canonical `Unknown` is `[0]`; `Known(x)` is `[1,x]`. `Option::None` omits a
  map entry instead; null is never emitted by the new typed mapper.

Binding and references:

- `ScopeBinding { tenant: TenantRef, scope: Scope }`, with
  `Scope = Guild(GuildRef) | GuildMember { guild, member: PrincipalRef } |
  DirectMessage { user: PrincipalRef } | Organization(OrganizationRef)`.
  No global/public fallback. References include the full binding, even when
  equal to the envelope binding, so extraction does not lose authority context.
- `LocalEpisodeRef { binding: ScopeBinding, domain: EpisodeDomain, digest }`;
  `EpisodeDomain = CompleteV1 | LegacyEpisodeV1`. No naked legacy digest implies
  a tenant: a future resolver must bind old guild records to an authenticated
  tenant explicitly. v2 adapters need a separate later mapping.
- `ArtifactRef { binding, digest, byte_length: u64, kind: ArtifactKind,
  format: VersionRef }`. Max byte_length 2^40; descriptors only, never bytes,
  URI, file path, credentials or model weights. Kinds: `StateSummary=0,
  Prediction=1, Outcome=2, Observation=3, Model=4, Calibration=5,
  Provenance=6, ReservoirEmbedding=7, Evidence=8, Authority=9`.
- `VersionRef { component: ComponentRef, revision: OpaqueRef, digest }`:
  exact immutable artifact identity; version text alone is insufficient.
- `ForeignEvidenceRef { origin_organization, origin_binding, domain, digest,
  import_receipt: LocalEpisodeRef }` is a separate consulted-evidence variant.
  The import receipt must have the envelope's local binding. This preserves
  foreign origin without treating foreign IDs as local authority. This type
  records a claim only; enabling resolution waits for federation validation.

`CompleteEpisodeInput` has these exact grouped fields:

| Group | Fields |
| --- | --- |
| identity | `episode_id: EpisodeId`, `request_id: RequestId`, `binding: ScopeBinding`, `contract: VersionRef` |
| time | `acquired_at: UtcMillis`, `received_at: UtcMillis`, `monotonic: Option<MonotonicRange { clock, start, end }>` |
| origin | `source_node: NodeRef`, `actor: { principal: PrincipalRef, kind: ActorKind }`, `signer: Option<SignerKeyId>` |
| context | `task_id: TaskId`, `regime: RegimeRef`, `regime_posterior: Estimate<RegimeDistribution>`, `habituation: Estimate<UnitPpm>`, `summary: Option<ArtifactRef>`, `resources: Vec<ResourceReading>`, `reservoir: Option<ArtifactRef>` |
| decision | `route: Route`, `prediction: Estimate<ArtifactRef>`, `actions: Vec<ActionDescriptor>`, `uncertainty: Estimate<UnitPpm>`, `risk: Estimate<UnitPpm>`, `novelty: Estimate<UnitPpm>` |
| outcome | `observed: Estimate<ArtifactRef>`, `utility: Estimate<Measurement>`, `safety_effect: SafetyEffect`, `costs: Vec<ResourceReading>`, `rollback: RollbackState` |
| authority | `policy: VersionRef`, `safety_policy: VersionRef`, `capabilities: Vec<VersionRef>`, `models: Vec<VersionRef>`, `calibrations: Vec<VersionRef>`, `approvals: Vec<LocalEpisodeRef>`, `consent: Option<ConsentRef { binding, epoch: u64, policy: VersionRef, attestation: LocalEpisodeRef }>` |
| evidence | `maturity: EvidenceLevel`, `source_refs: Vec<ArtifactRef>`, `observation_refs: Vec<ArtifactRef>`, `dimensions: RecordedEvidence` |
| lifecycle | `retention: RetentionIntent`, `holds: Vec<LocalEpisodeRef>`, `deletion_key: Option<ObjectKeyRef { binding, key_id: OpaqueRef }>`, `edges: Vec<LifecycleLink>` |
| provenance | `causal_parents: Vec<LocalEpisodeRef>`, `consulted_parents: Vec<ConsultedRef>`, `records: Vec<ArtifactRef>` |

Existing `ActorKind` and `EvidenceLevel` may be reused as values but mapped
explicitly; do not derive commitment tags from Rust discriminant position.
`SignerKeyId` is the existing validated Ed25519 identifier; including it binds
an intended writer identity, not proof of possession or source attribution.

Closed vocabulary:

- `Route`: `NoModel=0, LocalModel=1, CloudModel=2, Specialist=3, Deterministic=4`.
- `ActionDescriptor { capability: VersionRef, action: ActionRef,
  parameter_classes: Vec<ParameterClassRef>, phase: Proposed=0|Performed=1 }`.
  Action order is meaningful. No argument values, prompts or response text.
- `ResourceReading { resource: ResourceKind, amount: u64 }` with
  `Tokens=0, CpuMicros=1, MemoryBytes=2, NetworkBytes=3, EnergyMicrojoules=4`;
  values are measurements, not budgets or authorization.
- `RegimeDistribution`: 1..32 unique `(RegimeRef, UnitPpm)` entries, sum exactly
  1_000_000 using checked u64 arithmetic; sorted by ASCII regime bytes.
- `SafetyEffect`: `Unknown=0, NoViolationObserved=1, InterlockTriggered=2,
  ViolationObserved=3`. `RollbackState`: `NotApplicable=0, NotAttempted=1,
  Attempted=2, ExactRestoreObserved=3, Failed=4, Unknown=5`.
- `RecordedEvidence`: eight separate named `Estimate<ArtifactRef>` fields:
  `cryptographic`, `source`, `calibration`, `outcome`, `compatibility`, `regime`,
  `lifecycle`, `confidence`. They reference recorded evidence, not authoritative
  current verification results or a combined trust score. Cryptographic evidence
  cannot attest this envelope's own signature; self-verification is computed
  externally after the commitment exists.
- `RetentionIntent`: `Ephemeral=0`, `Session { session: OpaqueRef,
  expires_at: UtcMillis }=1`, `Operational { expires_at }=2`,
  `Durable { privacy_basis: LocalEpisodeRef }=3`,
  `MandatoryIncident { obligation: LocalEpisodeRef }=4`.
  Encoding ephemeral input is possible for preview; later admission must reject
  persistence. Deadline must exceed receipt time. No automatic hold or retention
  privilege is inferred by these declarations.
- `LifecycleLink { relation, target: LocalEpisodeRef, basis: LocalEpisodeRef }`
  with `Supersedes=0, Contradicts=1, Revokes=2, Quarantines=3, Resolves=4,
  Redacts=5, Forgets=6`. Immutable links are declarations; future store validation
  checks target existence, legal relation, authorization and replay. No mutable
  `superseded_by` or current quarantine flag is committed into historical data.

All local refs, artifacts, deletion key, consent and lifecycle refs must exactly
match the envelope binding, including member/DM identity. Broader organization
or guild authority is represented by same-scope authority receipt references;
proof that the authority actually applies is a later validator responsibility.
Foreign refs are allowed only in consulted evidence and retain explicit origin.
An episode may occur in both causal and consulted sets: the two meanings differ.

Bounds: actions <=128; parameter classes <=32 each; resources/costs <=16 each;
versions <=32 per category; approvals/holds/edges <=64 each; causal parents
<=64; consulted parents <=256; source/observation/provenance refs <=256 each.
Every set rejects duplicates before canonical sorting; actions preserve order.
Resource kinds and version components are unique within their list. An empty
list means known empty, not unknown. Reject reference-to-own computed digest
in the new domain after computing bytes; this local check does not establish
acyclicity or parent availability. Inputs are structurally limited before
mapping; encoded output <=1 MiB with checked sizing before allocation. A
post-allocation length check alone is not a bounded encoder guarantee.

## Exact mapping rule

`COMPLETE_EPISODE_DOMAIN = "wdbx-complete-episode-v1"` is a proposed new domain.
Construct `CanonicalValue::Map` directly, reuse its deterministic primitive
encoding, and hash its bytes with SHA-256. Never call `EpisodeCommitment::new`
for this type; changing only schema_version within the old domain is insufficient.

Top-level integer-key map: `0: domain`, `1: 1` (schema version), `2: identity`,
`3: time`, `4: origin`, `5: context`, `6: decision`, `7: outcome`,
`8: authority`, `9: evidence`, `10: lifecycle`, `11: provenance`.
Nested struct keys are text field names exactly as shown in the table/type
spellings; fields of each named group become its map members. Descriptor/group
names are not extra wrapper keys. Integer values use shortest CBOR forms;
negative signed values use `Negative`, zero/positive signed values use
`Unsigned`. UUIDs/digests are byte strings; opaque refs are ASCII text;
existing signer key IDs use validated canonical text; existing actor/evidence
levels use their explicit current labels. Every enum with a numeric tag above
encodes `[tag, fields-map]` for data variants and `[tag]` for unit variants.
Scope tags: Guild=0, GuildMember=1, DirectMessage=2, Organization=3.
EpisodeDomain tags: CompleteV1=0, LegacyEpisodeV1=1. ConsultedRef tags:
Local=0, Foreign=1; each has one `reference` map member holding its descriptor.
Other structs become maps by the same field-name rule. `Estimate` follows its
special mapping above. No unknown fields or null-valued options are emitted.

Sort set-like collections by their item's canonical CBOR bytes, except the
posterior's defined regime ordering. Reject duplicate canonical encodings. In
addition reject two local refs with the same `(binding, domain, digest)` even
if future descriptors add fields; do not deduplicate silently. Sort version
sets by component ASCII bytes after uniqueness validation. Actions retain input
order and parameter classes sort lexically. Deterministic map order is supplied
by `CanonicalValue`, not insertion order or serde output.

`block_id` is the UUID, never the computed digest. `episode_digest` is derived
output, not a field. The external result may carry `canonical_bytes` and digest.
There is no whole-payload hash, detached signature or self-referential signature
status inside this map. Artifact digests describe separately scoped objects,
never bytes of this same envelope. Actual signatures/COSE wrappers belong to a
later layer and must bind the new domain and the declared signer identity.

## API and failure surface

Proposed signatures (not transport methods):

```rust
impl CompleteEpisode {
    pub fn try_new(input: CompleteEpisodeInput) -> Result<Self, CompleteEpisodeError>;
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CompleteEpisodeError>;
    pub fn digest(&self) -> Result<[u8; 32], CompleteEpisodeError>;
}
```

All new primitives, groups, inputs and outputs use custom redacted `Debug`:
type/variant names and bounded counts only. Do not derive Debug through nested
identities, timestamps, digests, private references or serialized CBOR. Errors
are fixed enums such as `InvalidIdentifier`, `InvalidUuid`, `InvalidRange`,
`DuplicateReference`, `ScopeMismatch`, `CollectionLimit`, `EncodedSizeLimit`,
`InvalidPosterior`, `SelfReference`, `InvalidArtifactKind`; a closed field code
may identify the failing slot, never a supplied value. Existing CanonicalValue
has content-bearing Debug, so keep the mapped value private and do not include
it in errors. A digest can correlate private content and is not a redaction key.
Explicit byte retrieval is an intentional data API, never a diagnostic receipt.

## RED-first acceptance tests

Write these tests before implementation and observe failure due to absent API,
then rerun after implementation. No live stores or private fixtures.

1. Synthetic minimally complete and fully populated fixtures pin exact bytes and
   SHA-256, independently recomputed (Python witness in a new profile module;
   do not regenerate old fixtures). Include all enum/data shapes over the suite.
2. Domain substitution test demonstrates new envelope bytes/digest differ from
   the old profile with equivalent content. Old-profile reader rejects it.
3. Mutate each mandatory field, each optional value, tenant, scope kind, member,
   signer ID, evidence dimension and authority reference: bytes/digest change.
4. Zero differs from Unknown and absent optional fields; negative utility and
   integer-size boundaries encode exactly. No floating-point input constructor.
5. Reverse each set: same bytes; duplicate entries reject; action permutation
   changes bytes. Moving a parent from causal to consulted changes commitment.
6. Guild, member, DM and tenant substitution in every nested local reference
   rejects. Foreign consulted reference preserves origin; local import receipt
   mismatch rejects. These prove structural binding, not authenticated tenancy.
7. Maximum-size boundary succeeds; +1 field/list/output fails before unbounded
   allocation. Invalid UUID, identifier, posterior sum, repeated regime,
   negative count measurement, expiry and monotonic range reject deterministically.
8. Predicted and observed unknown values remain distinct from successful outcomes;
   evidence Unknown never becomes Valid. Two conflicting lifecycle declarations
   can be represented without claiming authorized state transition.
9. Sentinel strings in every private leaf never occur in `Debug`, `Display` or
   error chains; exact digests/UUID bytes and timestamps also never leak.
10. Old v2 tests, memory-candidate/edge goldens, signing/replay and cross-language
    witnesses pass unchanged. Run full `bash tools/check.sh` for Rust changes;
    documentation-only design validation is only `git diff --check`.

## Deliberately incomplete after this slice

No stored complete episodes; no new database, journal, CAS or quorum writer.
No reader/transport mapping for the new shape, negotiation or ABI corpus update.
No COSE, new signing behavior, signer administration, rotation, revocation,
reference resolution, authenticated scope or authorization; no source truth.
No selective admission, current policy validation, incident retention exemption,
TTL enforcement, object encryption/erasure, tombstone propagation or artifact
liveness. No causal graph traversal, evidence-weighted retrieval, claim ledger,
federation grants, sync, projection rebuild or migration. No C3 efficacy or
C4-C7 deployment claim. Existing operation ledger remains authoritative for its
existing event domain; new pure input values do not create a parallel writer.

## Correction appendix (binding for Task1)

# Complete-episode contract audit

2026-10-20. Read-only source audit; the only output is this report. C0 design evidence, not implementation, cryptographic approval, ABI ratification, or a completed task-02 gate.

## Recommendation

Do not implement the proposed public canonical serializer as a gate-cleared task-02 deliverable yet. `docs/completion/plans.json` task-02 explicitly requires agreement with the ABI schema owner **before writing the canonical serializer**. `shared-types-v1.json` task-02 permits private prototypes but forbids dependent merge or wire freeze until the named owner signs the schema diff and producer/consumer vectors and artifact hashes are reviewed. Omitting serde does not remove this sequencing constraint: public canonical bytes are a cross-package contract too.

Proceed now with this design correction and, if useful, a clearly private prototype whose bytes are provisional. A public `v3::complete_episode` API and frozen new-domain vectors require the documented gate or an explicit scoped change to that gate. The proposed slice is usefully independent of transport implementation, storage, signing, and authorization; that independence does not itself satisfy agreement with the ABI owner. Do not claim the entire task-02 complete after this slice: decoding, COSE, trust-key history, and signature witnesses remain separate work.

## Sources checked

- Root `AGENTS.md` and `.superpowers/sdd/2026-10-20-wdbx-full-completion/complete-envelope-design.md`.
- `docs/completion/shared-types-v1.json` task-02 and `docs/completion/plans.json` task-02.
- Sibling ABI `docs/superpowers/specs/2026-08-22-abbey-system-constitution.md`, section 5, decisions 23–30, Program 1/4 ownership, and amendment rules; `2026-08-22-spec-canonical-wdbx-episodes.md` proposed envelope/commitment/evidence/deletion sections and dated implementation corrections.
- Actual `crates/abi-wdbx/src/v3/commitment.rs`, `episode/types.rs`, `episode/signing.rs`, and `episode/store/canonical.rs`.
- Actual ABI corpus `contracts/abbey/v1/schemas/episode/{proposal,evidence}.schema.json` and `authorization/scope.schema.json`.

The ABI proposal schema is an adapter proposal, not a complete episode. Its retention vocabulary (`ephemeral`, `short`, `policy_bound`, `mandatory_incident`) and evidence object are not an existing byte mapping for this design. ABI's authorization scope includes organization, deployment, tenant, and platform scope; the proposed local binding omits deployment and most platform scope variants. Neither schema can silently be reinterpreted as this new envelope. No new ABI schema, corpus fixture, or transport translation was created in this audit.

## Minimal corrections before implementation

1. **State the prototype/release boundary explicitly.** Keep the named domain provisional and the new implementation private until task-02 agreement. Do not describe a public API or local golden freeze as permission to publish a new wire. Ratification must resolve deployment/organization isolation, platform scope coverage, UUID constraints, units, version component identity, and identifiers against actual ABI contracts. Do not mechanically add all ABI authorization fields here without that decision.

2. **Make signer identity present for a complete committed value.** Constitution section 5 lists signer identity and an *optional signature*. The design instead makes signer identity optional. Recommended minimal correction: require the validated intended writer key identifier on `CompleteEpisodeInput`; possession/signing remains future work. If an unsigned pre-writer preview needs no identity, keep it a separate draft input that cannot become `CompleteEpisode` until identity is supplied. Do not insert a fake/zero signer or infer one from actor. Existing `SignerKeyId::parse` validates spelling, not that a key exists or that its fingerprint actually matches a supplied key.

3. **Finish the exact encoding table.** `Guild(GuildRef)` and `Organization(OrganizationRef)` lack named tuple fields despite the rule requiring a fields map. Define them explicitly as `[0,{"guild":…}]` and `[3,{"organization":…}]`; member and DM use the declared `guild`/`member` and `user` names. Define posterior encoding as an array of `[regime_text, ppm_unsigned]` pairs sorted by regime ASCII bytes, without a new wrapper. Declare `GuildRef` as the same bounded opaque-token representation used by other identity wrappers (it is currently referenced but omitted from the primitive list). These are recommended *private prototype* decisions pending ABI agreement, not a wire publication.

4. **Pin enum labels and uniform numeric-tag shapes.** See the inventory below. All numerically tagged unit variants must encode as singleton arrays, including resource kind, measurement unit, relation, phase, and domain; an implementer must not encode some as bare integers. Map reused actor variants with an exhaustive local match: `ActorKind::label` is `pub(super)` inside the existing episode module and is unavailable to a sibling module. Do not broaden old APIs just to access it. Evidence labels are explicitly C0–C7, with no automatic evidence promotion.

5. **Specify artifact-slot kind checks.** Recommended shape rules: summary→StateSummary, reservoir→ReservoirEmbedding, prediction→Prediction, observed→Outcome, observation_refs→Observation, provenance.records→Provenance; recorded evidence dimensions→Evidence. Permit source_refs to contain any defined ArtifactKind, since a source can be a model, observation, or other artifact; document this exception. Standalone descriptors admit every defined kind. These rules make `InvalidArtifactKind` meaningful. Digest/format verification and whether an artifact actually supports a claim remain deferred. Use a slot matrix in code/tests; do not infer kind from free-form component text.

6. **Close collection identity and foreign-reference rules.** All collections except action sequence are sets unless explicitly stated otherwise; posterior has its own ordering. Resource and cost lists are each unique by resource kind; version lists are each unique by component; parameter classes unique by token. Artifact sets reject identical canonical descriptors. Local episode-reference lists reject repeated `(binding,domain,digest)`; lifecycle links reject identical `(relation,target,basis)` but allow different relations/bases naming the same target. Do not apply local-reference deduplication across unrelated fields: one authority receipt may support multiple links, and one parent may be both causal and consulted. Foreign consulted identity is the complete descriptor; different import receipts remain distinguishable claims. Require its import receipt to be local; require origin organization to match origin_binding's organization only when origin scope is explicitly Organization. Other organization-membership relationships cannot be proven from this model. Do not require origin tenant strings to differ: opaque identifiers are meaningful in their issuing context.

7. **Narrow privacy promises to the new diagnostic surfaces.** Existing `SignerKeyId` has content-bearing Debug and Display. Do not change that legacy type in this slice or promise it never prints. New groups/input/output must redact it; expose no derived diagnostic delegation to its Display. Either wrap it in a new redacted type or explicitly limit the guarantee to diagnostics of the new complete-episode types. Make explicit accessor/byte retrieval an intentional disclosure API. Redact timestamp/amount/UUID/digest values as the design requires, not just free text. No blanket `derive(Debug)` on nested newtypes.

8. **Specify validation timing and allocation guarantees.** `try_new` must validate the entire immutable object before returning success, including final encoded-size and self-reference checks; later byte/digest calls must not discover input-invalid states. Count exact CBOR size with checked arithmetic before constructing the CanonicalValue tree and encoded buffers. Existing `CanonicalValue::canonical_bytes` is not bounded by byte count and `encode_map` allocates temporary key/value encodings. Reusing it after a successful preflight can bound work for this fixed shape, but does not mean peak memory equals the 1 MiB output bound. Document that distinction; keep the old encoder untouched. Validate caller-owned list lengths before cloning/mapping them. Caller allocations before handing an input to this API are outside its allocation guarantee.

9. **Make self-reference scope and testability precise.** Reject this CompleteV1 digest in any local episode reference, including consent attestations, approvals, holds, lifecycle target/basis, retention basis/obligation, and foreign import receipts, not just parents. Reject an artifact descriptor claiming the current envelope digest as its separate object, as the design already states. A real fixed-point SHA-256 envelope is not a practical test fixture: unit-test the reference-walk helper with an injected candidate digest, then test integration calls that helper. Do not claim a synthetic fixed point or confuse graph acyclicity with this check.

## Field and enum completeness

The ten groups cover the constitutional field categories at the level of declarations/references: identity/time/scope, state/regime/resources, route/prediction/actions/judgment, outcome/safety/cost/rollback, authority versions, evidence, lifecycle, and distinct causal/consulted provenance. The optional signer is the concrete constitutional presence mismatch above. `holds` records hold references, not authoritative current hold state. Evidence references are recorded claims, not the retrieval evidence assessment described in the normative spec. Keeping current hold/trust state external is appropriate for immutable historical commitments, but document that the model alone does not deliver those behaviors.

The old proposed spec's `payload_hash` and inline `signature` should not be copied mechanically: the current constitution permits an optional signature; the whole-envelope digest must stay derived outside its own input. Artifact hashes identify separate payloads. Request/task IDs and source-node identity are additional input fields, not replacements for episode UUID. Omission, Unknown, known zero, and known empty list remain distinct as designed.

| Vocabulary | Required exhaustive mapping |
| --- | --- |
| ActorKind | Text: human_subject, organization_owner, guild_owner, guild_administrator, guild_manager, service |
| EvidenceLevel | Text: C0, C1, C2, C3, C4, C5, C6, C7 |
| Scope | Guild 0; GuildMember 1; DirectMessage 2; Organization 3; named field maps as corrected above |
| EpisodeDomain | CompleteV1 0; LegacyEpisodeV1 1 |
| ConsultedRef | Local 0 / Foreign 1, each `{reference: descriptor}` |
| Route | NoModel 0; LocalModel 1; CloudModel 2; Specialist 3; Deterministic 4 |
| Action phase | Proposed 0; Performed 1 |
| MeasurementUnit | Unitless 0; Count 1; Milliseconds 2; Bytes 3; Microjoules 4 |
| ResourceKind | Tokens 0; CpuMicros 1; MemoryBytes 2; NetworkBytes 3; EnergyMicrojoules 4 |
| ArtifactKind | StateSummary 0; Prediction 1; Outcome 2; Observation 3; Model 4; Calibration 5; Provenance 6; ReservoirEmbedding 7; Evidence 8; Authority 9 |
| SafetyEffect | Unknown 0; NoViolationObserved 1; InterlockTriggered 2; ViolationObserved 3 |
| RollbackState | NotApplicable 0; NotAttempted 1; Attempted 2; ExactRestoreObserved 3; Failed 4; Unknown 5 |
| RetentionIntent | Ephemeral 0; Session 1 `{session,expires_at}`; Operational 2 `{expires_at}`; Durable 3 `{privacy_basis}`; MandatoryIncident 4 `{obligation}` |
| Lifecycle relation | Supersedes 0; Contradicts 1; Revokes 2; Quarantines 3; Resolves 4; Redacts 5; Forgets 6 |
| Estimate | Unknown `[0]`; Known `[1,value]`, intentionally different from enum fields-map rule |

No additional enums or raw metadata escape hatches are needed for this pure slice. In particular, do not import the old proposal schema's `minimized: true`/`redacted: true` flags as evidence that privacy requirements are met.

## Exact test expectations

- Independent witness vectors must cover minimal and populated values, all enum variants above, all 4 scope forms, all 8 evidence slots, and both consulted forms. Pin exact bytes and SHA-256 from independent implementations; never regenerate the legacy corpus. Test wrapper arrays explicitly (`[0]` differs from `0`).
- Mutation coverage must change every leaf while keeping shape valid. Changing the envelope binding alone with existing nested refs is a rejection test; for a commitment-change test, change all local bindings consistently. Omitted optional values differ from present values; Unknown differs from known zero and known empty sets.
- Scope rejection must visit every nested local-reference/artifact slot, including authority/consent policy's associated attestations, lifecycle basis, durable privacy basis, mandatory obligation, and foreign import receipt. VersionRef has no binding; do not pretend that comparing its digest authenticates authority scope.
- Positive shape tests allow acquisition later than receipt, equal monotonic endpoints, all-zero digest bytes, signed utility extrema, unknown outcomes, contradictory declarations, and C0–C7 claimed maturity. They do not assert the claims valid for admission. Expiry equal to receipt rejects; receipt+1 succeeds when representable. Count/bytes measurement values must be nonnegative multiples of 1,000,000 in SignedMicros; fractions and negatives reject.
- UUID tests cover non-nil RFC variant v4/v7 acceptance, nil/wrong-variant/other-version rejection. Opaque tokens reject empty, 129 bytes, whitespace, slash, Unicode, and unsupported punctuation; 128 valid ASCII bytes succeeds. Treat the regex as token spelling, not proof of de-identification or universal URI detection (colon is explicitly allowed).
- Posterior tests: 1 and 32 unique entries; 0 and 33 fail; sum 1,000,000 succeeds; sum off by one and repeated regime fail; zero-probability entries remain representable. All arithmetic is checked before conversion.
- Set reversal preserves commitment; duplicate keys/descriptors fail; action reordering changes commitment for distinct actions; equal action descriptors may repeat because a sequence can repeat an action. A resource list cannot have 16 unique entries with only 5 resource kinds: test all 5 unique kinds, then duplicate rejection, rather than claiming a realizable 16-entry success boundary.
- Test precise collection caps using valid unique values, then cap+1. For encoded size, use the counted-size helper with a smaller test limit if no valid input can hit exactly 1 MiB; independently establish the maximal reachable size or furnish a valid over-limit input before claiming the production cap was exercised. Post-allocation rejection is insufficient.
- Redaction tests cover new type Debug, errors and error chains using synthetic identifiers and numeric/digest sentinels; explicitly test signer containment without asserting that the unchanged legacy SignerKeyId diagnostic API is redacted.
- Legacy-domain reader must reject the new map; generic CanonicalValue parsing may accept its primitives and is not evidence of legacy-envelope acceptance. Old v2, candidate/edge, signing/replay and Python witness tests must remain unchanged and pass. Rust implementation requires `bash tools/check.sh`; documentation audit only uses `git diff --check`.

## Boundary after correction

Required now: closed typed shape, spelling/ranges, scoped reference equality, slot-kind compatibility, duplicate rules, deterministic mapping, bounds, redacted diagnostics, digest and domain separation. Deferred: identity authentication, source attribution, tenant/deployment policy mapping, reference existence and graph traversal, issuer/key resolution, signature or COSE verification, admission/replay, active consent, policy applicability, actual hold status, retention enforcement, encryption/erasure, incident minimization, federation authorization, and store migration. The corrected pure model must not reject or approve records based on invented substitutes for those missing systems.

## Actionable private-prototype ruling

A minimal safe path is a real, additive crate-private module (`pub(crate) mod complete_episode` under v3), with crate-private types/functions and private validated fields. No new Cargo feature and no `cfg(test)`-only implementation are necessary. Unit tests inside that module exercise the real code; external integration tests cannot import it, which is the intended consumer boundary. Keep serde, stores, gateway adapters, key administration, and transport exposure absent. Draft vector tests can pin current behavior for regression detection, but mark the fixtures/domain and module documentation explicitly **provisional, subject to ABI ratification, not a published compatibility promise**. Independent Python comparison may run from the unit tests using synthetic draft fixtures. This is the private-prototype exception in `shared-types-v1.json`, not satisfaction of task-02's serializer agreement step.

Before coding, incorporate the nine corrections above into the prototype design. The only missing numeric enum field maps are the tuple-style Guild/Organization variants; ConsultedRef already specifies `reference`, RetentionIntent already names all data fields, GuildMember/DM already name theirs. The other concrete incomplete container mapping is RegimeDistribution. ActorKind and EvidenceLevel use text, not numeric arrays. Signer identity should be required validated text for completed prototype values while actual signature stays external; recorded evidence slots should remain eight separate Unknown/Known artifact references with no cryptographic-validity or trust-result enum inferred from them.

Public promotion remains blocked on ABI agreement; private model/encoder experimentation can proceed after these local decisions are documented. No dependent consumer merge, corpus refresh, or production-domain freeze follows from green prototype tests.

## Execution notes

For private dormant production code, a module-scoped allow(dead_code) with an explicit comment is permitted solely because consumer access is intentionally gated; do not suppress other lints or weaken workspace policy. Unit tests must use actual code. No production cap coverage claim from smaller injected-limit tests; report exact reachable bounds honestly.
