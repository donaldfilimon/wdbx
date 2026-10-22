# Complete-episode ABI ratification dossier

2026-10-20. Prepared for actual ABI schema-owner review. **Unratified comparison dossier, not a new normative schema, wire freeze, approval receipt or task-02 completion.** No ABI files changed. The crate-private prototype remains provisional. The decoder contract is under implementation; final decoder vectors, source hashes and independent review are **pending**.

## Authority and review target

ABI Program 1 owns normative language-neutral schemas, fixtures and compatibility; WDBX owns canonical episode mappings and durable semantics. [Shared task-02 gate](../../completion/shared-types-v1.json) requires a named owner signing the exact schema diff, shared producer/consumer positive and negative decode vectors, exact artifact hashes and independent review before dependent merge or wire freeze. Private prototyping does not satisfy that gate.

The [constitution §5](../../../../abi/docs/superpowers/specs/2026-08-22-abbey-system-constitution.md#5-wdbx-episodic-and-evidence-contract) is ratified at the boundary; its opening status explicitly leaves mechanism contracts proposed. The [canonical episode spec §§1–2](../../../../abi/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md) supplies a proposed EpisodeBlock vocabulary and canonical CBOR/COSE-compatible direction, not approval of this concrete map. Existing [ABI proposal](../../../../abi/contracts/abbey/v1/schemas/episode/proposal.schema.json), [evidence](../../../../abi/contracts/abbey/v1/schemas/episode/evidence.schema.json), [claim](../../../../abi/contracts/abbey/v1/schemas/episode/claim.schema.json) and [tombstone](../../../../abi/contracts/abbey/v1/schemas/episode/tombstone.schema.json) schemas remain authoritative for their existing messages. An adapter proposal is not the complete episode.

No inspected existing approval artifact satisfies task-02 for this mapping. Boundary ratification, old schemas, private encoder review and successful local tests cannot be recorded as that approval.

## Exact current private map

Source: [mapping.rs](../../../crates/abi-wdbx/src/v3/complete_episode/canonical/mapping.rs), [canonical.rs](../../../crates/abi-wdbx/src/v3/complete_episode/canonical.rs), [types.rs](../../../crates/abi-wdbx/src/v3/complete_episode/types.rs). The root is a CBOR map with integer keys, not a tuple or JSON object. Nested keys are text. Root values are:

| Root key | Current value | Constitutional / canonical-spec correspondence; owner decision |
|---|---|---|
| 0 | text `wdbx-complete-episode-v1` | Distinct draft domain; ABI/WDBX must agree exact publication identity and downgrade rules. |
| 1 | unsigned `1` | Schema version; interaction with identity.contract and negotiation unresolved. |
| 2 | Identity | UUID/schema/scope; adds request_id; payload hash is outside these fields as derived digest. |
| 3 | Time | Acquisition/receipt time plus optional monotonic range, rather than spec's created_at name. |
| 4 | Origin | Source/actor/signer; mandatory signer identity does not mean signature exists or verifies. |
| 5 | Context | Combines canonical-spec task and state groups. |
| 6 | Decision | Route, prediction, ordered redacted action descriptions and judgment estimates. |
| 7 | Outcome | Observed result, utility, safety, costs and rollback separated from decision. |
| 8 | Authority | Policy/model/calibration/capability/approval/consent bindings; structural references do not authorize. |
| 9 | Evidence | References and eight explicit recorded dimensions, plus maturity label. |
| 10 | Lifecycle | Retention, holds, deletion key and relations. |
| 11 | Provenance | Separates causal and actually consulted parents; artifact provenance references. |

The following inventory records every current map key and typed leaf/group. `Option` fields are omitted when absent; all other listed fields are present, including empty sets and Unknown estimates. These are observations of private source, not proposed ABI acceptance by implication.

| Private structure | Exact nested map keys and Rust types | Comparison / unresolved normative issue |
|---|---|---|
| ScopeBinding | `tenant: TenantRef`; `scope: Scope` | Constitution pseudonymous scope; existing proposal has scope_digest, not this tenant plus tagged scope. |
| VersionRef | `component: ComponentRef`; `revision: OpaqueRef`; `digest: Digest32` | Constitution versions; ABI source_version and policy/grant versions are not automatically this component/revision/digest triple. |
| LocalEpisodeRef | `binding: ScopeBinding`; `domain: EpisodeDomain`; `digest: Digest32` | New binding/domain/raw digest object; existing proposal target_id is opaque text. Confirm cross-domain resolution and same-scope requirements. |
| ArtifactRef | `binding: ScopeBinding`; `digest: Digest32`; `byte_length: u64`; `kind: ArtifactKind`; `format: VersionRef` | Canonical spec requires digest references for large artifacts. Approve size units, format identity, kind taxonomy and reference access authority. |
| ForeignEvidenceRef | `origin_organization: OrganizationRef`; `origin_binding: ScopeBinding`; `domain: EpisodeDomain`; `digest: Digest32`; `import_receipt: LocalEpisodeRef` | Preserve origin authority; local import receipt must not replace foreign signature. Federation contract remains separate. |
| Actor | `principal: PrincipalRef`; `kind: ActorKind` | Principal plus role label is a recorded claim; ABI must bind authenticated identity and scope, not trust supplied role. |
| Measurement | `unit: MeasurementUnit`; `value: SignedMicros` | SignedMicros with explicit unit; count/bytes nonnegative integral units enforced by divisible-by-1,000,000 raw value. |
| MonotonicRange | `clock: ClockRef`; `start: MonoNanos`; `end: MonoNanos` | Approve clock identity/comparability and restart rules; start <= end only is structurally enforced. |
| ResourceReading | `resource: ResourceKind`; `amount: u64` | Resource kind chooses units; unsigned amount. Confirm measured versus estimated semantics. |
| ActionDescriptor | `capability: VersionRef`; `action: ActionRef`; `parameter_classes: Vec<ParameterClassRef>`; `phase: ActionPhase` | Constitution redacted action and parameter classes; no raw executable parameters. ABI capability binding/version resolution required. |
| ConsentRef | `binding: ScopeBinding`; `epoch: u64`; `policy: VersionRef`; `attestation: LocalEpisodeRef` | Optional authority consent plus epoch/policy/attestation. Presence does not establish present consent; absence interpretation requires ABI policy. |
| ObjectKeyRef | `binding: ScopeBinding`; `key_id: OpaqueRef` | Content-free binding/key ID; not key material. Existing proposal deletion_key is opaque text. |
| LifecycleLink | `relation: LifecycleRelation`; `target: LocalEpisodeRef`; `basis: LocalEpisodeRef` | Explicit relation,target,basis; existing proposal links kind,target_id differs and lacks resolution/redaction/forget variants. |
| Identity | `episode_id: EpisodeId`; `request_id: RequestId`; `binding: ScopeBinding`; `contract: VersionRef` | Constitution UUID/schema/scope. Request ID retry/replay uniqueness and contract version semantics need owner agreement. |
| Time | `acquired_at: UtcMillis`; `received_at: UtcMillis`; `monotonic: Option<MonotonicRange>` | UTC unsigned milliseconds; existing ABI utc_instant is text with seconds. No implicit conversion or pre-epoch support. |
| Origin | `source_node: NodeRef`; `actor: Actor`; `signer: SignerKeyId` | Constitution signer identity/optional signature. Signature bytes/COSE are absent from this map. |
| Context | `task_id: TaskId`; `regime: RegimeRef`; `regime_posterior: Estimate<RegimeDistribution>`; `habituation: Estimate<UnitPpm>`; `summary: Option<ArtifactRef>`; `resources: Vec<ResourceReading>`; `reservoir: Option<ArtifactRef>` | Spec task/state vocabulary combined; summary/reservoir artifact refs optional, other unknowns use Estimate. |
| Decision | `route: Route`; `prediction: Estimate<ArtifactRef>`; `actions: Vec<ActionDescriptor>`; `uncertainty: Estimate<UnitPpm>`; `risk: Estimate<UnitPpm>`; `novelty: Estimate<UnitPpm>` | Prediction versus Unknown explicit. Actions are an ordered sequence that may repeat; uncertainty/risk/novelty units ppm need approval. |
| Outcome | `observed: Estimate<ArtifactRef>`; `utility: Estimate<Measurement>`; `safety_effect: SafetyEffect`; `costs: Vec<ResourceReading>`; `rollback: RollbackState` | Known versus unknown observed/utility; no inference of success from enum label. Existing receipt semantics require explicit adapter mapping. |
| Authority | `policy: VersionRef`; `safety_policy: VersionRef`; `capabilities: Vec<VersionRef>`; `models: Vec<VersionRef>`; `calibrations: Vec<VersionRef>`; `approvals: Vec<LocalEpisodeRef>`; `consent: Option<ConsentRef>` | Version and approval refs structurally scoped; current-policy/consent checks are outside prototype. No grant/approval authority inferred. |
| RecordedEvidence | `cryptographic: Estimate<ArtifactRef>`; `source: Estimate<ArtifactRef>`; `calibration: Estimate<ArtifactRef>`; `outcome: Estimate<ArtifactRef>`; `compatibility: Estimate<ArtifactRef>`; `regime: Estimate<ArtifactRef>`; `lifecycle: Estimate<ArtifactRef>`; `confidence: Estimate<ArtifactRef>` | All eight required dimensions explicit but values are Unknown or Evidence artifact refs, not inline verified dispositions. |
| Evidence | `maturity: EvidenceLevel`; `source_refs: Vec<ArtifactRef>`; `observation_refs: Vec<ArtifactRef>`; `dimensions: RecordedEvidence` | Maturity C0–C7 is recorded data, not conformance promotion; existing ABI evidence states use a different closed object. |
| Lifecycle | `retention: RetentionIntent`; `holds: Vec<LocalEpisodeRef>`; `deletion_key: Option<ObjectKeyRef>`; `edges: Vec<LifecycleLink>` | Retention intentions are not executed deletion/admission/holds; existing retention classes need explicit migration/projection mapping. |
| Provenance | `causal_parents: Vec<LocalEpisodeRef>`; `consulted_parents: Vec<ConsultedRef>`; `records: Vec<ArtifactRef>` | Causal and consulted ancestry kept separate; no ancestry resolution/verification in prototype. |

## Exact scalar, variant and ordering observations

* OpaqueRef and its tenant/organization/principal/guild/node/regime/clock/component/action/parameter wrappers are 1–128 ASCII bytes, alphabet alphanumeric plus `._:-`. Existing [v1 common definitions](../../../../abi/contracts/abbey/v1/schemas/common/definitions.schema.json#/$defs/opaque_id) uses text pattern `[a-z][a-z0-9_-]{0,63}`. These are different contracts; do not alias them.
* EpisodeId/RequestId/TaskId are 16-byte CBOR byte strings, nonzero RFC-variant UUID v4/v7; Digest32 is 32 raw bytes (including zero). Existing common digest_id is textual `sha256:` plus 64 lowercase hex. Signer is text parsed by existing SignerKeyId (`ed25519:` key-derived identity); source role is independent of signer trust.
* UtcMillis, MonoNanos, resource amounts and consent epoch encode unsigned; SignedMicros uses canonical signed integer; UnitPpm is unsigned 0..1,000,000. ABI utc_instant and duration_ms are strings. Unit/precision/overflow and conversion choices need explicit approval.
* `Estimate::Unknown` is `[0]`; `Known(value)` is `[1,value]`. Missing optional map key, Unknown, known zero and known empty remain distinct. Null is not omission. RegimeDistribution Known payload is an array of `[regime_text,ppm]` pairs, 1..32 unique regimes, sum exactly 1,000,000; zero probabilities allowed.
* Unit enums are singleton arrays `[tag]`. EpisodeDomain CompleteV1=0, LegacyEpisodeV1=1. Route NoModel=0,LocalModel=1,CloudModel=2,Specialist=3,Deterministic=4. ActionPhase Proposed=0,Performed=1. MeasurementUnit Unitless=0,Count=1,Milliseconds=2,Bytes=3,Microjoules=4. ResourceKind Tokens=0,CpuMicros=1,MemoryBytes=2,NetworkBytes=3,EnergyMicrojoules=4.
* ArtifactKind StateSummary=0,Prediction=1,Outcome=2,Observation=3,Model=4,Calibration=5,Provenance=6,ReservoirEmbedding=7,Evidence=8,Authority=9. SafetyEffect Unknown=0,NoViolationObserved=1,InterlockTriggered=2,ViolationObserved=3. RollbackState NotApplicable=0,NotAttempted=1,Attempted=2,ExactRestoreObserved=3,Failed=4,Unknown=5. LifecycleRelation Supersedes=0,Contradicts=1,Revokes=2,Quarantines=3,Resolves=4,Redacts=5,Forgets=6.
* Scope variants are `[0,{guild}]`, `[1,{guild,member}]`, `[2,{user}]`, `[3,{organization}]`. ConsultedRef is `[0,{reference:LocalEpisodeRef}]` or `[1,{reference:ForeignEvidenceRef}]`.
* Retention is `[0]` Ephemeral, `[1,{session,expires_at}]` Session, `[2,{expires_at}]` Operational, `[3,{privacy_basis}]` Durable, `[4,{obligation}]` MandatoryIncident. Existing proposal retention_class is ephemeral/short/policy_bound/mandatory_incident; spelling and semantics are not interchangeable. Its holds none/active/released also differ from the private hold-reference set.
* Actor kind text is human_subject/organization_owner/guild_owner/guild_administrator/guild_manager/service; maturity text C0..C7. JSON fixture PascalCase is not the canonical spelling.
* Ordinary sets sort lexicographically by entire encoded item bytes. Version lists instead sort component ASCII bytes; parameter classes sort token ASCII bytes; posterior sorts regime ASCII bytes. Actions retain order/repetition. Canonical map-key ordering is separate from these semantic set rules.

## Structural bounds and governance gaps

[validate.rs](../../../crates/abi-wdbx/src/v3/complete_episode/validate.rs) checks actions<=128, parameters<=32/action, resources/costs<=16, version lists<=32/category, approvals/holds/causal parents<=64, edges<=64, consulted/source/observation/provenance lists<=256. Resource uniqueness means only five distinct resource kinds are realizable, not sixteen valid distinct readings. Artifact length <=2^40 bytes. Encoded output <=1MiB. Same-scope binding applies to local refs and local artifact slots; foreign origin is separate and its import receipt local. All eight evidence dimension artifacts require Evidence kind; source refs intentionally allow every artifact kind. Expiry must exceed receipt time for Session/Operational; monotonic start<=end; duplicates and self-reference reject. These checks do not resolve references, run current policy, grant consent, verify signatures or perform retention.

The [private decoder contract](2026-10-20-complete-episode-decoder.md) requires exact integer root keys0..11, exact nested field sets, explicit inverse variants, typed bounds, existing bounded generic parser then full canonical re-encode equality. Generic depth cap32, supplied independent limits and a clamped <=1MiB input apply; generic parsing allocates an owned tree before typed caps. Final vectors/hashes/review pending. Rust decoding of Python-encoded bytes is not an independent Python typed decoder; that witness remains outstanding.

## Exact existing ABI definitions to reconcile

| Existing authority | Actual scope | Required owner decision |
|---|---|---|
| v1 common definitions opaque_id/digest_id/utc_instant/duration_ms | Text forms noted above | Reuse through an explicit lossless conversion or publish new versioned definitions; do not broaden old schemas silently. |
| v1 episode proposal properties | Closed proposal with observation_method/data_class/claim_class/payload_mode/minimized/redacted/priority_class/deletion_required | These governance fields are not all explicit private complete fields. Decide admission metadata versus canonical binding/artifact, and whether any omissions undermine constitutional privacy/authority audit. |
| v1 episode evidence properties | integrity_state/provenance_state/semantic_state/truth_state plus source/version/time/payload digest | Decide artifact content contract and producer/consumer mapping for eight dimensions; record losses. No conversion from a signature to truth. |
| v1 episode claim/tombstone | Existing claim and deletion messages | Define relation to complete lifecycle/provenance without rewriting existing bytes/digests. |
| v2 authorization [grant](../../../../abi/contracts/abbey/v2/schemas/authorization/grant.schema.json), [approval](../../../../abi/contracts/abbey/v2/schemas/authorization/approval.schema.json), [policy-decision](../../../../abi/contracts/abbey/v2/schemas/authorization/policy-decision.schema.json) | ABI grants, policy versions, revocation epochs and approval/request binding | Resolve complete VersionRef/LocalEpisodeRef references to exact authenticated authority snapshots. Private Actor kind is not authorization. |
| v2 receipt [outcome-receipt](../../../../abi/contracts/abbey/v2/schemas/receipt/outcome-receipt.schema.json) | Existing terminal execution outcome | Decide canonical receipt/operation/writer-epoch representation and cancellation-after-commit behavior with ABI; not present in this private model. |

## Ratification decisions and approval checklist

ABI owner plus WDBX canonical owner must resolve the following against the exact snapshot below. Entries are open decisions, not recommended schema amendments:

1. Domain/version allocation, old-domain rejection, migration/read coexistence and retention of original signed bytes; contract VersionRef versus root version and feature/version negotiation.
2. Identifier encodings, pseudonymization, authenticated tenant/scope binding, organization/direct-message scope semantics, cross-domain/foreign reference resolution and canonical receipt identities.
3. Every tag/text enum, unknown-variant policy, ordered versus set semantics, normalization and future extension rules.
4. Units/precision/range and clock epochs; Unknown versus absent versus zero; whether each estimate's epistemic meaning is recorded sufficiently.
5. Capability/policy/model/calibration/approval/consent version identities; current versus historical authority; consent absence, revoked/stale input and policy epoch semantics; recorded role cannot confer power.
6. Admission metadata absent from the private shape (proposal fields above), retention-class correspondence, holds, incident minimization/privacy, deletion key hierarchy and content-free tombstones.
7. Canonical profile and resource/input/collection bounds, decoder rejection categories and negotiation without accepting unknown variants. Confirm inline versus digest-referenced evidence/artifact contents and eight-dimensional inspectability.
8. COSE profile: detached payload, protected headers/algorithm/key ID, digest-versus-payload signature scope, external AAD, stored original bytes and substitution/downgrade rules. Prototype has no COSE signature container.
9. Trusted-key identity/history, activation/rotation/revocation effective time, key compromise and historical verification versus permission for new admission. Existing payload KEK rotation is a different operation.
10. Shared receipt/authority schema, exact producer/consumer positive and malformed vectors, Rust plus independent typed second-language decoder/signature witness and reviewed dependency selection. No handwritten Python crypto.

Approval receipt must name owner/reviewer, exact schema/domain/profile/vector/source hashes, explicit decisions and bounded evidence, link the ABI-owned schema diff and shared vector results, and state compatibility/migration consequences. Independent crypto review is additionally required before production qualification. Until then no public interface promotion, dependent merge, normative corpus refresh or consumer wire freeze. Only ABI's vendor tool may refresh the corpus after its authoritative update.

## Snapshot hashes

These SHA-256 values identify read source inputs only, not approved schema or decoder output. Decoder source/vector final hashes remain PENDING. The approved baseline encoder pins are listed for review; no new golden was generated here.

| Input | SHA-256 |
|---|---|
| `crates/abi-wdbx/src/v3/complete_episode/mod.rs` | `7cbc3e2a64e8856cdc7d99cca71d7ee093149d9a26519d2a5a19475dee3852ad` |
| `crates/abi-wdbx/src/v3/complete_episode/primitives.rs` | `913fa1f392325aa43e07db713bdea064319a5d28f69dd6b6db9e226429d604ab` |
| `crates/abi-wdbx/src/v3/complete_episode/types.rs` | `dd709ac43d64400a4eaa90be16277111397cd43dcaa17a7cb5d8e577cdc8eaa1` |
| `crates/abi-wdbx/src/v3/complete_episode/validate.rs` | `4fcae5471fad604d3cc5e9b2ebbdd171be27e757474ed177856d7a9002f4c3b5` |
| `crates/abi-wdbx/src/v3/complete_episode/canonical.rs` | `931af7aa3863bbe1459fecbfb8ac126481867e69f474309d021295d321ee469b` |
| `crates/abi-wdbx/src/v3/complete_episode/canonical/mapping.rs` | `c16dad612896339a02e282cb81973c3caf791e3bfc6f18f21de6772590525cf5` |
| `crates/abi-wdbx/src/v3/complete_episode/tests/provisional-minimal.hex` | `35609d42b28084de403d0ba0517a577977c5f31e61f542786c25953c56d621c3` |
| `crates/abi-wdbx/src/v3/complete_episode/tests/provisional-populated.hex` | `6c3f9efeaa6a3741f48fc79da573b3be83854735dc3102fa5daa322a50fd660a` |
| `docs/superpowers/plans/2026-10-20-complete-episode-decoder.md` | `b1884299a4b9157ed440728f688d86c1c270e4b2ac7ef335e3a02f6ec17032ba` |
| `docs/completion/shared-types-v1.json` | `5d51d33a6ce4adc8628f780bd913520f65e9d7f5ae6b875fe03027bd408d527b` |
| `../abi/docs/superpowers/specs/2026-08-22-abbey-system-constitution.md` | `7b3f932a08f55bb4bcf0dfe2b45ea356da84bc79d6103101f70900738b4c5bcc` |
| `../abi/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md` | `8c0c6295d9956290b45f0180a13ddfd42546911ad5bf5bc83e57f06868681526` |
| `../abi/contracts/abbey/v1/schemas/common/definitions.schema.json` | `e3ccda4ce5c999842c0a5f2535c2a9c843213f824d0cf25fe809dd3e3503f530` |
| `../abi/contracts/abbey/v1/schemas/episode/proposal.schema.json` | `d965e8740f3728c0bb1a455a64de9b0c084dd292d3e03db81003c91893a79a35` |
| `../abi/contracts/abbey/v1/schemas/episode/evidence.schema.json` | `ee81e175b4413f9e98e00da5e441f5b55fad2e629da658280d76be23964370fb` |
| `../abi/contracts/abbey/v1/schemas/episode/claim.schema.json` | `30dae8dd338393c29677f8eb32c1030ee6503bd0a7aa327a544ffd2e50caf75f` |
| `../abi/contracts/abbey/v1/schemas/episode/tombstone.schema.json` | `dcc0b5eed552286ce370821afb26c61674182761e67c9682e304e08515f1fbd4` |
| `../abi/contracts/abbey/v2/schemas/authorization/grant.schema.json` | `7d0b9b5b1c5866fa7e565dddda1466106e6dc3e5beb20b698bbe2c79af3f971f` |
| `../abi/contracts/abbey/v2/schemas/authorization/approval.schema.json` | `c03bbd6b31c4b5c9530734954f1c121196bba56f9bb9d6c5772bdbfd195c5909` |
| `../abi/contracts/abbey/v2/schemas/authorization/policy-decision.schema.json` | `6b4af26256743912807545933c914314bf5068423da1d4174c18355132dff211` |
| `../abi/contracts/abbey/v2/schemas/receipt/outcome-receipt.schema.json` | `d378ff41f3b1ae2a32a0a0ed7b24534054fb288890bbcc7c08aa520bba017d40` |

No Cargo, network, installs, source changes, schema approval or commits performed in preparing this dossier. Public launch remains deferred.
