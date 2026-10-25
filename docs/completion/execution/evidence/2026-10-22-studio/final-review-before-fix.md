# Final independent Studio tranche review — 2026-10-22

## Scope and method

Reviewed the complete `studio-tranche-final-review.diff` against HEAD `36bc99b699de60be6c07b2838d74580084fb5d4c` plus the preserved prior dirty execution-register baseline, using the requested code-reviewer rubric. Read all three bounded task briefs, reports, prior task reviews, current continuation rulings, Studio instructions, new identity implementation/tests, optional native/WASM fixture/adapter/gate/comparator, evidence changes and retained relevant raw outputs. Reviewed the repetitive observation JSON structurally as well as its substantive claims/blockers.

This review did not rerun builds, tests, probes or inventory checkers. Read-only Python inspection independently checked current hashes, frozen-row equality and register deltas. Only this requested review report was written; no source, index, branch, dependency, service or credential changes were made, and no subagents were dispatched.

## Strengths

- The pure table boundary is coherent and respects the existing product boundary. Typed 128-bit concrete refs, complete caller-prepared keys, populations and logical resource links are distinct; exact original bytes and validated typed bind/action structures determine semantic identity. Key/digest equality is not deduplication authority, and semantic comparison spans the population even when a caller changes key projections. No text-v3 encoder, authentication or durable canonical writer is implied (`specimen-studio/native/specimen-kernel/src/identity/model.rs`, `identity/mod.rs`).
- Mutation refusals precede publication. Insertion checks scope/key/ref consistency, finite weights and complete bounded trees before mutation; full-node spill checks a fresh supplied ref before adding the sibling. Rebuild validates the complete replacement and builds derived indexes before publishing either. The direct tests compare snapshots after failures and cover capacity+1, ref collisions, malformed imports, original-byte limits, resource caps, ordered alternatives and population separation (`identity/validate.rs`; `tests/identity.rs`).
- Lookup order comes from owned record order, never hash-map iteration. Both entry and resource collections switch at the specified per-population 64/65 boundary; indexes retain every matching ref. The stronger parity fixture resolves the earlier resource-link count-only observation through exact array/index/rebuild/reverse sequences, while retaining unlinked structural retrieval (`identity/index.rs`; `native/specimen-wasm/src/identity_probe.rs:153`; `tests/identity-parity.mjs:62`).
- The probe executes real table operations and serializes their observed results. Native and WASM call the same fixture; the JavaScript comparator checks exact bytes and independently expected successful properties, rejects a shared error envelope, validates buffer bounds, copies and frees returned bytes, and throws on unexpected host calls. Feature registration and the raw export are opt-in, and isolated build artifacts preserve the normal browser artifact (`native/specimen-wasm/src/lib.rs:129`; `scripts/check-identity-parity.sh`; `tests/identity-parity.mjs`).
- The evidence mapping preserves scientific and owner boundaries. Exactly S01/S02/S03/S04/S17 change from local gap to partial candidate; all 163 rows remain open/C0/unknown. All 46 case and 18 decision frozen rows match verbatim. Each row retains explicit blockers, including scanner/encoder, feedback, production overflow, consumer migration, scheduling and full qualification obligations. Crypto/ABI-owner requirement objects and historical audit fields are unchanged against the preserved baseline.

## Verification assessment

Independent read-only checks found:

- All current identity, parity and evidence files match their respective frozen task snapshots.
- All 571 path/SHA-256 references traversed in `studio-research-v1.json` match current bytes.
- All 26 prior root/P1 paths in `prior-source-reuse.json` match their recorded prior hashes; this is source reuse, not a fresh substrate gate.
- Register object changes are limited to P08.01/P08.10/CH05/CH06 and S01–S46. Only the authorized five dispositions change; no other top-level register value changes. Current counts are 118 candidates, 29 local gaps, 16 external gaps.
- Retained raw logs support the reported 221 browser/unit passes, 151 native passes, successful broader lint and native Clippy, and the explicit feature checks. The native output also contains one ignored existing test; it is not evidence that every optional runtime test executed.
- The parity raw log records matching 4,168-byte outputs with SHA-256 `eb84ea042097bba7786b5c6cf23ae808b8a95f02887c668999934d44760d55d1`, feature WASM execution, default export absence and independent-property success. The negative raw log contains the expected native/WASM byte-comparison assertion failure.
- Retained current/system Python logs and closure script support the reported inventory checks and 27 mutation tests per interpreter. These remain supplied execution records, not independently rerun results.

The unchanged language parser/contact registry was inspected to check prepared-tree validation against actual scope, literal, byte and nesting rules. The existing raw-buffer helpers were inspected to check the new export's ownership contract. No additional production algorithm was changed by the reviewed source diff.

## Issues

### Critical (Must Fix)

None found.

### Important (Should Fix)

None found for this bounded tranche.

### Minor (Nice to Have)

1. **Preserve the insert-result variant in the parity trace.** `specimen-studio/native/specimen-wasm/src/identity_probe.rs:52` maps `Inserted` and `Duplicate` to the same object. Exact refs/order/state are verified, and kernel tests separately assert the variants, but the cross-runtime witness cannot detect a wrong success variant with identical refs. Add a `kind` field and independently assert `Inserted` for the spill and `Duplicate` for the repeat in both the JS and native probe tests before claiming variant-level parity. This is the existing disclosed nonblocking observation, not a newly discovered table defect.

2. **Describe the default-WASM check precisely.** `docs/completion/execution/studio-research.md:83` says the default artifact is separately “instantiated.” In `specimen-studio/tests/identity-parity.mjs:14`, the default artifact is compiled and its exports inspected; only the feature artifact is instantiated. Change the narrative to “separately compiled and its exports inspected.” The existing check is sufficient for the claimed absence of `identity_probe_raw`; this is a documentation accuracy correction, not a need to add another runtime gate.

### Inherited environment observations

The retained native logs show `rust-objcopy` SIGABRT/missing `@rpath/libLLVM.dylib`, and native Clippy retains dependency diagnostics including the `block` future-incompatibility notice. Successful test execution is supported; a pristine toolchain build is not. The packet already discloses the LLVM limitation and local Bun canary rather than CI's Bun version. Keep these limitations in downstream receipts and repair the toolchain separately before claiming warning-free validation. No source fix or expansion of this tranche is required by this observation.

## Recommendations

Accept the bounded foundation and observation packet for continuation, retaining the two Minor follow-ups explicitly. Preserve the existing resource-order observation as resolved by the consumer-side witness. Keep future text-v3/parser, production command/import/migration and scheduler integration work as separate reviewable packets with their own direct tests; none is established merely by exposing `HybridTable` or compiling it into both editions.

## Cannot verify

- Authenticity or timing of earlier execution beyond the retained logs, matching source snapshots and current byte checks; this review reran no suites.
- Independent research adequacy/holdout controls, whole-case efficacy, complete Task 8 acceptance, models/OCR/GPU execution, installed desktop/UI/theme qualification, other platforms/engines or elapsed operational behavior.
- Named ABI-owner approval, crypto ratification, independent cryptographic qualification or production key custody. Those gates remain pending, including the retained PyCA failure.
- Historical non-use of live data or credentials from code review alone. The reviewed new fixture uses only explicit synthetic values and has no I/O/host-service access; no such access occurred during this review.

## Declined to judge

- Text-v3 lexical transforms, grammar/wildcard indexing and parser normalization: explicitly deferred beyond caller-prepared identity structures; blockers are retained.
- Production feedback/attachment integration, saved specimen migration, durable imports and command cutover: explicitly absent from this foundation tranche and must retain existing text-v2 behavior until reviewed integration.
- Scheduler fairness, raft yielding, end-to-end resource arbitration and asymptotic tuning: this tranche implements deterministic lookup and bounded foundation tests, not the separate scheduling/tuning contract or its performance qualification.
- Broad scientific efficacy, hardware/model/platform qualification and owner/crypto approval: separate qualification obligations; no promotion is made here.
- Global toolchain repair: disclosed host environment defect, outside source ownership and unnecessary to assess the executed bounded fixture.

## Assessment

**Spec compliance: Approved for the bounded current tranche.**

**Code quality: Approved with nonblocking Minor follow-ups.**

**Ready to merge? Yes, for this bounded source/evidence scope; this is a technical verdict, not authorization to commit, merge, deploy or close the parent program.**

The implementation is additive, transactional and deterministic, with meaningful direct and cross-runtime mechanism evidence and no blocking correctness/integration defect found. The inventory accurately remains open/C0/unknown; ABI-owner and crypto gates stay pending, and the full WDBX completion program and parent Task 8 remain incomplete.
