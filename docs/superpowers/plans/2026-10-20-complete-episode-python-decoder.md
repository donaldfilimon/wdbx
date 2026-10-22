# Private complete-episode Python decoder implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans. Execute the checkbox tasks with fresh implementation and independent specification/quality review.

**Goal:** Independently decode and structurally validate the existing private complete-episode bytes in Python, compare retained typed values with Rust, and reject a bounded differential hostile corpus.

**Architecture:** Reuse the independent Python generic CBOR syntax reader, with its legacy-envelope mode disabled, then implement an explicit Python inverse and relational validator. Return the existing synthetic fixture JSON vocabulary as a value-bearing test projection; it is not a new transport or normative episode schema. Existing Rust and Python encoders, domain, digest pins and legacy corpus remain unchanged.

**Tech stack:** Python standard library; Rust 1.99.1 private unit tests and existing serde_json test infrastructure. No new dependency.

**Spec:** `docs/superpowers/plans/2026-10-20-complete-episode-decoder.md`, the existing provisional model/mapping and `docs/superpowers/plans/2026-10-20-complete-episode-prototype.md`. The approved full completion program and ABI ownership boundaries govern this local witness.

## Global constraints

- This is private C1 interoperability evidence. It provides no ABI ratification, admission, consent authorization, signature verification, resolved lineage, storage or production qualification.
- Current Rust decoder implementation is frozen for independent review; this plan does not assume that review passes. Capture its final reviewed hashes before implementation integration; resolve discovered discrepancies by reviewed contract decisions, never silent encoder/pin changes.
- Preserve v2 commitments, v3 candidate/edge goldens, original signed bytes, Python encoder behavior, corpus vendor files, locks and consumer files. No Git writes, commits, pushes, branches or worktrees under current user instructions.
- Python independently implements shape and relational rules from the existing private Rust model. Do not invoke Rust, shell commands or the Python encoder as a relational validation oracle.
- Root Rust mutations and Cargo ownership require controller clearance. Python-only work may proceed under exclusive new-path ownership while another lane holds a root source freeze; Rust test wiring waits for that freeze's release.

## Review focus

1. An unbounded JSON-lines iterator could allocate attacker-sized input before a decoder limit; task 1 pins bounded reads, draining and recovery.
2. Python booleans masquerade as integers and dicts erase duplicate keys; tasks 1–2 reject both explicitly.
3. Encoder roundtrips alone miss relational scope/kind failures; task 3 independently validates every inventoried slot.
4. Sorting received arrays could hide noncanonical semantics; task 2 preserves their order and task 4 rejects reversed set order.
5. A digest self-reference fixed point cannot be manufactured as an honest byte fixture; task 3 tests the predicate separately and reports its actual ingress coverage.

## Owned files and interfaces

Create `tools/wdbx_complete_episode_decode.py` (bounded ingress, public witness entry points), `tools/wdbx_complete_episode_decode_values.py` (explicit inverse of grouped fields/references/vocabulary), and `tools/wdbx_complete_episode_decode_validate.py` (relational validation and reference inventory). Split further only within this unique prefix if size/readability requires it. Create `tools/tests/test_wdbx_complete_episode_decode.py` and, if needed, sibling test modules with the same prefix. Modify only the minimum declarations in `crates/abi-wdbx/src/v3/complete_episode/tests.rs`; create `tests/python_decode.rs` and sibling test helpers under the private complete_episode directory. The existing `tests/draft_adapter.rs::input` is reused unchanged to interpret returned fixture JSON. No public Rust API is needed.

Read-only dependencies: `tools/abbey_cbor_reader_v1.py::Reader`, `DEFAULTS`, `DecodeError`; `tools/abbey_cbor_episode_v1.py` canonical value classes and encoder; `tools/wdbx_complete_episode_draft.py` fixture vocabulary and reencoder. The generic `decode()` returns bytes rather than a tree: instantiate `Reader(data, limits, model, False)` directly, call `value()`, and check complete consumption. Do not change the shared reader to obtain a tree.

Python entry points:

```python
decode_complete(data: bytes, limits: dict | None = None) -> dict
request(line: bytes) -> dict
run_json_lines(source, sink) -> int
main(argv: list[str]) -> int
```

`decode_complete` returns exactly `{"fixture": ..., "bytes": lower_hex, "digest": lower_sha256_hex}` on success and raises a content-free `CompleteDecodeError(code)` otherwise. It accepts bytes only; it checks its input cap before parsing. `request` wraps success with `"ok": true`, or returns exactly `{"ok": false, "error": code}`. The CLI accepts only `decode-json-lines`; wrong arguments exit 2. Finite per-request rejection leaves process exit 0. Tracebacks, input snippets, identifiers and exception interpolation never appear on stdout/stderr for expected rejection.

Request JSON has exactly `{"cbor_hex": string}` with optional `"limits": object`. No kind discriminator: this command only handles the private complete domain. Hex is even-length ASCII hexadecimal (either case accepted); returned hex is lowercase. Limits are precisely the five existing reader names, omitted entries use DEFAULTS, values have exact Python type int and range 0..2**63-1. Unknown keys, duplicate JSON keys at any nesting, nonfinite constants, floats as limits and bools are RequestError. `max_parent_count` is accepted but inert; causal and consulted typed caps remain separate.

Errors are fixed strings: `RequestError`, `IngressLimit`, `AllocationFailure`; generic reader categories retain their names with `Canonical:` prefix; typed shape/domain/version/variant/order use `InvalidShape`, `UnsupportedDomain`, `UnsupportedVersion`, `UnknownVariant`, `NonCanonicalEncoding`; constructor/relational categories use `InvalidValue:` plus the current CompleteEpisodeError variant name. No global precedence promise for inputs with multiple independent violations. Differential exact-category assertions use single-fault fixtures; ordinary malformed corpus comparisons require agreement on accept/reject.

## Task 1: Bound ingress before parsing or hex allocation

- [ ] Write failing standard-library tests for the absent module/API and bounded stream protocol; run and record actual RED.
- [ ] Fix `MAX_CBOR_BYTES = 1_048_576` and `MAX_LINE_BYTES = 2 * MAX_CBOR_BYTES + 4096`, including a terminating LF when present. Read `source.readline(MAX_LINE_BYTES + 1)`, never iterate unbounded lines or call unbounded read. If the read exceeds the cap, emit exactly one IngressLimit for that logical line; if that chunk does not already end in LF, drain through its LF using bounded readline(4096) calls, then resume. Do not read past that LF into the next request. EOF-terminated lines within cap are accepted; blank lines are RequestError. Oversized last lines produce one error. Do not accumulate drained chunks.
- [ ] Apply the same line-length cap in direct `request` before UTF-8 decoding/json.loads. Perform a string/escape-aware JSON bracket-depth scan capped at 64 before parsing; it only enforces depth, not JSON validity. Use duplicate-key rejection, rejecting parse_constant, rejecting parse_float and a parse_int callback rejecting decimal digit counts above 20 before calling int. Negative sign does not count as a digit. Invalid UTF-8/JSON/recursion returns RequestError.
- [ ] Validate request/limits, then check decoded hex length against min(request max_input_bytes, 1MiB) before bytes.fromhex. Valid hex above this cap yields Canonical:InputLimit. Reject malformed hex as RequestError. `decode_complete` repeats the raw bytes cap check. Caller zero limits retain their meaning.
- [ ] Catch MemoryError at the request boundary as AllocationFailure; this is best-effort categorization, not guaranteed recovery under process exhaustion. No claim of constant memory or peak-memory <= input cap: JSON text, CBOR tree, fixture, reencoding and response coexist, all bounded by finite ingress/tree/list limits.
- [ ] Test a custom stream that asserts every readline/read has an explicit finite bound; overlong line followed by valid line, huge no-newline tail, invalid UTF8, escaped brackets/quotes, depth64/65, digit20/21, repeated keys, all invalid limit forms, no-newline valid request, two requests, exact line cap and cap+1. Spy on json.loads/bytes.fromhex through narrow injectable helper boundaries to prove over-cap rejection precedes those calls. Do not allocate gigabyte fixtures.

## Task 2: Explicit typed inverse and value-bearing projection

- [ ] Independently map the exact twelve top-level unsigned keys 0..11 and ten group maps, strict required/optional sets, exact scalar widths, tuple arities and all current numeric/string enum tags. Reference the existing canonical mapping files as the field/tag inventory; do not derive an inverse by introspecting/calling the forward mapper.
- [ ] Use existing canonical value classes with exact class checks, so Bool/Null cannot be accepted as Unsigned. Enforce integer widths, token ASCII `[A-Za-z0-9._:-]{1,128}`, UUID nonzero RFC variant/version4-or7, 16/32-byte lengths, and `ed25519:` plus 64 lowercase hex signer syntax. All-zero digest remains valid. Actor wire strings map back to existing fixture enum names. Optional absent fields stay omitted, never become null. Unknown estimates return `{"unknown": true}`; known zero and known empty artifact remain distinct.
- [ ] Return fixture keys/enum names exactly as `wdbx_complete_episode_draft.py` accepts today. Byte fields become lowercase hex strings, integers remain integers, estimates retain explicit wrappers, arrays preserve received order. This test projection deliberately exposes synthetic values; no real user records enter fixtures or logs.
- [ ] Check each typed list cap before typed child conversion/output-list creation: actions128, parameter_classes32, resource/cost readings16, each version category32, approvals/holds/causal parents64, lifecycle edges64, consulted/source/observation/provenance256, posterior1..32. The generic syntax tree already exists; report only the sequencing actually proved. Posterior cap failures use InvalidPosterior, ordinary typed list caps CollectionLimit.
- [ ] Reencode the validated projected fixture with the unchanged independent Python forward encoder and require exact equality to original bytes; mismatch yields NonCanonicalEncoding. Do not use that reencoder to replace independent inverse checks or relational validation. It is the independent Python implementation of existing canonical ordering, not a third schema.
- [ ] Pin ordinary set order to whole canonical-item-byte lexical order, not CBOR map-key length-first order. Version lists sort by component ASCII; parameter classes and posterior regime labels use ASCII. Action sequence order and repeated actions are meaningful and remain permitted. Validate duplicates separately before reencoding.
- [ ] Unit-test every variant/tag and every field with missing/extra/wrong kind, all optional presence/absence, UUID4/7, signed endpoints, unknown/known zero, zero digest, all scopes, foreign refs, resource labels, measurement units and C0–C7. Test typed cap+1 with malformed children and an observed mapper count of zero. Valid resource maximum is five unique kinds, not sixteen.

## Task 3: Independently implement complete structural relational validation

- [ ] Implement known posterior probability bounds 0..1,000,000, unique regimes, checked total1,000,000 and 1..32 entries; monotonic start<=end; Count/Bytes measurements nonnegative and divisible by1,000,000, other units signed i64; Session/Operational expiration strictly greater than received_at.
- [ ] Reject duplicate parameter class, resource kind, version component within each category, and full-value duplicates within each remaining set. Distinct records with distinct complete values remain valid unless an existing special key rule applies. Preserve the current validator's structural semantics; do not invent semantic equivalence or policy restrictions.
- [ ] Implement an explicit local/artifact walk, independently reviewed against `complete_episode/validate.rs::walk`: context summary StateSummary, reservoir ReservoirEmbedding; known prediction Prediction; known observed Outcome; every known evidence dimension Evidence; source_refs any defined artifact kind; observation_refs Observation; provenance records Provenance. Each artifact has local identity binding and length<=2**40.
- [ ] Every local reference must match identity binding: approvals, holds, causal parents, consent attestation, edge target/basis, Durable privacy_basis, MandatoryIncident obligation, consulted Local, Foreign import_receipt. Consent binding and deletion-key binding also match. A Foreign origin Organization scope must equal origin_organization; its origin binding otherwise need not equal the local binding.
- [ ] Compute SHA256 over original canonical bytes, then reject each walked CompleteV1 local reference with that digest and every walked artifact digest equal to it. LegacyEpisodeV1 local references are not subject to the CompleteV1 digest comparison. Foreign origin digest is not itself a local reference; only the import receipt enters the local walk.
- [ ] Test the self-reference predicate directly with an explicit digest argument and synthetic references, including legacy and foreign exceptions. Test that the production entry point supplies the real SHA256 through a narrow test-only observer/mock wrapping the predicate. Do not fabricate a real accepted-byte hash fixed point or claim it was rejected at byte ingress. Other relational failures must be real canonical-byte ingress cases, independently constructed without asking the old forward fixture mapper to validate them first.
- [ ] Negative tests mutate each inventoried slot individually for scope/kind mismatch, artifact length, each duplicate family, retention boundary and monotonic/utility/posterior conditions. Include wrong local tenant with equal scope and wrong scope with equal tenant. Existing encoder accepting a structurally relational-invalid fixture is expected and must not be silently changed.

## Task 4: Rust/Python differential witness and regression gates

- [ ] Add a new private Rust test driver that starts the selected `WDBX_PYTHON` or python3 command, strictly checks exit status, response count/schema, JSON duplicate keys, hex, digest and no trailing junk. Missing Python is failure, never a skipped green. Do not relax old witness assertions or change old command behavior.
- [ ] For every successful Python response, parse its fixture through unchanged `draft_adapter::input`; compare it with `DecodedCompleteEpisode::input()` from the same original bytes. Independently check returned bytes equal original bytes and both digests equal SHA256(original). This proves value-bearing typed agreement without a second Rust serializer. A forward-only roundtrip or digest-only comparison is insufficient.
- [ ] Positive corpus reuses every existing finite valid draft-witness case plus targeted variants/boundaries above. Expected typed values come from original fixtures and explicit test normalization, never from either decoder under test. Reversed independent set arrays must reject; action reversal may accept with correspondingly different typed order/digest.
- [ ] Add deterministic canonical-tree mutations for every group field/tag/shape, all relational slots, duplicate/order families and typed bounds. Add raw malformed CBOR cases: every minimal-fixture truncation point, nonminimal arguments, reserved/indefinite forms, floats/tags, bad UTF8, map ordering/duplicate keys, trailing bytes, depth33, impossible declared lengths; all five limit zeros and independent limit reductions, max_parent_count0 success for otherwise valid input, raw1MiB+1 clamped despite huge caller cap. The last is input-clamp proof, not a valid maximal episode.
- [ ] A mutation that cannot pass the existing fixture encoder is serialized from an explicitly built generic CanonicalValue tree, preserving canonical CBOR syntax when testing typed semantics. Keep raw syntax cases separate. Ensure at least one real ingress case for each advertised reachable error category; direct enum construction only proves formatting. Do not demand matching error precedence on multiply-invalid inputs.
- [ ] Run `python3 -m unittest discover -s tools/tests -p 'test_wdbx_complete_episode_decode*.py' -v`; `cargo test --offline --locked -p abi-wdbx complete_episode` with the pinned toolchain after Cargo clearance; existing `v3_cross_language_decode`, commitment/episode witnesses and frozen memory candidate/edge tests unchanged. Controller runs `bash tools/check.sh` on the final reviewed inventory, plus `git diff --check`. Optional FHE claims remain separately scoped.

## Acceptance and evidence receipt

Each task requires actual RED/GREEN logs, exact source/lock/tool hashes, finite corpus case counts by class, and independent specification/quality review with bounded fix rounds. Finish with a receipt distinguishing syntax, typed mapping, structural relational agreement and the self-reference predicate seam. No measured security, exhaustive fuzzing, general-CBOR, peak-memory, independent cryptographic review or full constitutional admission claim follows. The implementation must remain explicitly incomplete if a required relational rule is omitted; do not relabel mapping-only evidence as a complete typed validator.

Implementation is serial within owned source paths; read-only review/corpus design can run independently. Persist task/review outcomes in `.superpowers/sdd/2026-10-20-completion-remaining/`. Do not commit or push. ABI named-owner ratification and production crypto review remain external open gates after all local tests pass.
