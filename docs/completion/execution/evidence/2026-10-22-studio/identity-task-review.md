### Spec Compliance

- ✅ Spec compliant for the bounded identity/table foundation described by `task-8-identity-brief.md`. No blocking missing, extra, or misunderstood implementation was found in the supplied whole-task diff. The public boundary remains prepared keys and typed `Expr` trees (`specimen-studio/native/specimen-kernel/src/identity/mod.rs:1`); the diff changes only the four owned identity modules, module registration and the two new focused test targets.
- ✅ Structural equality retains original UTF-8, the complete bind/action tree, argument/alternative order, inhibition and exact finite weight bits (`specimen-studio/native/specimen-kernel/src/identity/model.rs:55`, `:82`, `:112`). Population-wide semantic comparison is independent of supplied keys (`specimen-studio/native/specimen-kernel/src/identity/mod.rs:72`); complete keys are derived `Eq`/`Hash` structures with all four required dimensions (`specimen-studio/native/specimen-kernel/src/identity/model.rs:35`).
- ✅ Scoped node ownership, concrete-ref collision checks, full-node spill allocation and refusal paths precede owned-record mutation (`specimen-studio/native/specimen-kernel/src/identity/mod.rs:31`). Rebuild validates the full replacement before publishing records or indexes (`:225`; `specimen-studio/native/specimen-kernel/src/identity/validate.rs:77`). Supporting resources retain separate concrete refs, optional nonunique logical links and structural lookup (`specimen-studio/native/specimen-kernel/src/identity/mod.rs:170`, `:194`).
- ✅ Array/hash crossover is based on collection size within each population; multi-value vectors are built by record iteration and their order is returned directly (`specimen-studio/native/specimen-kernel/src/identity/index.rs:16`). Direct fixtures assert all ordered refs beyond multiple candidate-sized regions and at 64/65 (`specimen-studio/native/specimen-kernel/tests/identity.rs:409`), with separate resource cap/rebuild coverage (`:433`).
- ⚠️ Cannot verify from diff: actual native/WASM parity. The deterministic reusable fixture exists (`specimen-studio/native/specimen-kernel/tests/identity_fixture.rs:7`), but the report explicitly leaves the cross-target execution open (`task-8-identity-report.md:135`). Controller should retain this blocker for the later parity packet.
- ⚠️ Cannot verify from diff: unchanged source/save/UI/command behavior at runtime, production migration, text-v3 encoding, explicit grammar/wildcard indexes, full frozen-case conformance, hardware/model efficacy, any C0-C7 promotion, independent crypto qualification or named-owner approval. This packet appropriately implements or claims none of those (`specimen-studio/native/specimen-kernel/src/identity/mod.rs:1`; `task-8-identity-report.md:127`, `:134-136`). Controller must keep the corresponding claims open and use the relevant later evidence; no runtime efficacy or approval is inferred here.

### Strengths

- Typed owned structures prevent caller JSON from becoming canonical equality authority, while preserving exact source bytes and tree order (`specimen-studio/native/specimen-kernel/src/identity/model.rs:69`, `:82`, `:112`). The explicit host scope contract avoids implying credential authentication (`specimen-studio/native/specimen-kernel/src/identity/mod.rs:11`).
- Transactional refusal is implemented in the actual mutation methods, with import corruption validation sharing the same entry validation (`specimen-studio/native/specimen-kernel/src/identity/mod.rs:31`, `:225`; `specimen-studio/native/specimen-kernel/src/identity/validate.rs:13`, `:77`). Tests compare before-state snapshots after invalid mutations/imports and explicitly exercise capacity+1 and ref collisions (`specimen-studio/native/specimen-kernel/tests/identity.rs:84`, `:278`, `:350`).
- Index construction never exposes hash iteration order, and resources remain multi-value even when originals, keys and logical links repeat (`specimen-studio/native/specimen-kernel/src/identity/index.rs:16`; `specimen-studio/native/specimen-kernel/tests/identity_fixture.rs:38`).
- The implementation separates model, validation, derived indexes and mutation/lookup API into focused files. No I/O, clocks, threads, host random generator, GPU dependency, dependency changes or production-adapter changes appear in the diff (`specimen-studio/native/specimen-kernel/src/identity/mod.rs:16`; `src/identity/index.rs:1`).

### Issues

#### Critical (Must Fix)

- None found.

#### Important (Should Fix)

- None found for this bounded task.

#### Minor (Nice to Have)

- `task-8-identity-report.md:137`: validation output is not pristine: test linking reports a `rust-objcopy` SIGABRT caused by unavailable `@rpath/libLLVM.dylib`. This does not contradict the reported passing executed tests or warnings-denied Clippy, but it prevents claiming clean linker/toolchain validation. Controller should track the host-toolchain issue separately and retain the diagnostic in evidence; avoid concealing it or expanding this packet into unrequested environment/service changes.
- `specimen-studio/native/specimen-kernel/tests/identity.rs:433`: the large-resource crossover test compares exact ordered refs for structural lookup but checks only the length for logical-link lookup. An implementation returning the wrong linked refs or reversing them could pass that assertion. Strengthen the logical-link assertion at 64, 65 and 10,000 to compare the expected even-indexed `ResourceRef` vector; the current implementation itself builds these vectors correctly (`specimen-studio/native/specimen-kernel/src/identity/index.rs:48`).

### Assessment

**Task quality:** Approved, with the two minor follow-ups above.

**Reasoning:** The bounded kernel contract is coherent, validates before mutation, preserves collision-safe structural identity, and returns complete insertion-stable results from both lookup modes. Direct synthetic tests exercise the requested failure/capacity/crossover boundaries, while the report correctly leaves parser/encoder, migration, cross-target qualification and efficacy claims open.

**Checks performed:** Read the brief, parent Task 8 scope, implementer report and supplied whole-task diff; reviewed the implementation and tests without rerunning reported tests or mutating code/index/branch state. A focused outside-diff check addressed the named risk of accepting invalid prepared language trees: inspected `specimen-studio/native/specimen-kernel/src/language.rs:239-410` to compare `Expr`, scope/contact registration, literal parsing and parser depth/byte bounds against `identity/validate.rs:27`. Existing call syntax has no general arity schema; the new validator checks complete trees, registered scope, finite weights, literal shape and bounded work without inventing a new language grammar. The reported focused GREEN, 80-test kernel result and warnings-denied Clippy are evidence supplied by the implementer (`task-8-identity-report.md:102-124`), not independently rerun results. No live store, runtime data, credentials, private evidence, public launch or service operation was used. This review writes only this requested review artifact.
