# Final language/text/scanner tranche review

## Assessment

**Spec compliance: needs fixes. Code quality: needs fixes. Ready to merge: with fixes.** Three Important findings remain in the combined interfaces: quoted condition attestation, validated utility control semantics, and query-wide bounded work. No Critical finding. The additive architecture, exact scanner arithmetic and evidence boundaries are otherwise sound for the explicitly local profile; this is not approval of production adoption or scientific efficacy.

Reviewed the exact `final-language-source-review.diff` and the crosscutting portions of `task-8-language-evidence-review.diff`, against the current continuation rulings and frozen identity/language/scoring design bundles. Source snapshot 33/33 and evidence snapshot 13/13 current hashes match their review manifests. Earlier task approvals remain valid for their stated scopes; the findings below arise from combined invariant checks and additional bounded counterexamples.

## Strengths

- Prepared actions and compiled scripts have private validated state and immutable views. Legacy language/identity dispatch and saved schemas are not replaced; the WASM witness remains feature-gated.
- The shared rewrite validator, invocation-wide work allowance, recursive script output sizing, exact arithmetic-step accounting and body-scoped scanner polarity directly implement the recorded fixes. Script contacts are checked through nested capability intersections and complete registry content equality before computation.
- Text-v3 uses framed full SHA256 keys, while scanning consumes validated full AST leaves independently of key resolution. Checked i128 fractions preserve S and D separately, subtract once, and clamp only the final difference. Missing-feature penalties remain visible.
- Route joins retain concrete entry identity, distinct origins, captures and constraints; blocked automata are labelled unexecuted. The actual optional native/WASM witness independently checks transforms and fractions rather than treating byte equality alone as semantic proof.
- The observation refresh changes only S05/S06 dispositions to partial candidates, preserves all 163 open/C0/unknown claims, and retains explicit integration, efficacy, host, hardware and owner blockers. The checker policy is unchanged apart from its reviewed catalog pin.

## Issues

### Critical

None found in the supplied tranche.

### Important 1 — Condition attestation folds opaque quoted content

**Location:** `specimen-studio/native/specimen-kernel/src/language_v3/prepare.rs:63-69`.

Both the preserved full antecedent slice and each context condition are lowercased without regard to quotes. With context `conditions = [("\"allow\"", true)]`, preparing `if "ALLOW" then go` marks the body Satisfied/executable. `execute_for_action` then successfully evaluates a script for that body, even though the AST retains the distinct literal `ALLOW`.

This violates both selected-profile rules: lowercase applies only to unquoted lexical interpretation, and context attests the exact full antecedent. The error crosses the preparation/execution boundary: an attestation for one literal activates a condition containing another literal. Pure plans limit the present consequence, but the guard is incorrect before any host integration.

**Fix:** use exact full antecedent bytes under the stated exact-text contract, or a separately explicit quote-aware canonical comparison that preserves every quoted byte and applies the same rules to both sides. Do not reuse unconditional `to_lowercase`. Add mismatched quoted case and Unicode literal regressions, an exact positive, and a guarded execution refusal for the mismatch; preserve whole compound-antecedent matching.

### Important 2 — Accepted utility snapshots can erase control semantics

**Locations:** `specimen-studio/native/specimen-kernel/src/language_v3/tables.rs:158`; `prepare.rs:194-208`, `prepare.rs:514-520`, `prepare.rs:494-508`.

The constructor accepts all `Role` and `Conjunction` effects without checking their control semantics against the reserved vocabulary or each other. Two concrete accepted snapshots break the immutable-result invariant:

1. A Grammar row `{ phrase: "halt", effect: Role(Negation) }` prepares `halt go` with a negation-role token, but `inhibited=false`. `make_ast` removes the token because of its role, and the returned AST is affirmative `Action(go)`. Guarded execution succeeds.
2. A Conjunction row `{ phrase: "don't", effect: Conjunction(And) }` prepares `please don't go` as two executable actions. `split` consumes the built-in negator as a connective before the negation pass can see it. The final `go` action is uninhibited and guarded execution succeeds.

These are ordinary public constructor inputs, not forged Prepared values. The same validation boundary must also handle overlaps between custom negation phrases and conjunction phrases and prevent reserved If/Then semantics from being silently reassigned. A custom vocabulary must not let a validated snapshot defeat the frozen requirement to preserve negation/quotes and conservative inhibition.

**Fix:** define and enforce the allowed role/control combinations at construction (for example, reject control roles in Grammar and reject conflicting or overlapping control vocabulary), or make parsing consistently preserve the stricter safety meaning. A typed refusal is appropriate for contradictory snapshots. Add direct regressions for both examples, custom negation/conjunction overlap, and reserved conditional vocabulary; test the resulting execution guard as well as AST/tokens. Keep the existing cross-boundary rewrite checks and no-partial-result behavior.

### Important 3 — Candidate lookup copies the entire table outside its work budget

**Location:** `specimen-studio/native/specimen-kernel/src/scanner_v3/routes/mod.rs:297-303` (and the analogous registration copy at `:63`). Confirming unchanged dependency: `identity/mod.rs:219-220` deep-clones the complete `TableSnapshot`; `identity/validate.rs:77-108` bounds resources and entries per node but has no total entry/node ceiling.

`candidates` constructs `Work`, then deep-clones every entry, AST, original and resource in every population and builds an all-entry BTreeMap before charging any work. With an empty Prepared input, an empty route index and `work_steps=1`, a 257-entry table still returns `Ok([])` after those operations. Increasing the number/size of unrelated entries increases uncharged work without a bound imposed by this request. The 4096 route-reference limit and per-action charges do not cover this work.

This is a local query-budget defect, independent of the explicitly deferred global scan scheduler. It defeats the advertised complete-or-budget-error interface and makes a tiny/no-op query allocate in proportion to the entire host table. This is a static allocation/work counterexample, not a measured latency or exhaustion claim.

The repair must also cover materializing selected targets: `routes/mod.rs:405-406` clones `record.original` and the complete `record.ast`, while the preceding `append` charge covers only query token spans/capture payloads. Unchanged `identity/validate.rs:13-22` validates each alternative's finite weight and bounded tree but does not bound the total alternative count. Thus removing the initial snapshot alone still permits one selected entry to cause an arbitrarily large uncharged alternative-vector clone. This is a defect in the new query accounting, not a request to change the already reviewed identity acceptance rules. Count/charge through borrowed data before copying, with early budget refusal during traversal.

**Fix:** inspect borrowed records through a bounded accessor and charge traversal/materialization before doing it, or preflight a trustworthy complete cost bound before any snapshot/all-reference allocation. Avoid cloning unrelated resources/populations. An additive immutable identity accessor is reasonable if necessary, with IDs, saved formats and existing identity semantics preserved and its changes included in the final fix diff. Structural `find_all` materialization also needs consideration when accounting for complete query cost. Add a deterministic small-budget regression against a larger table, including an empty query, without timing assertions. Registration should likewise avoid an unnecessary complete-table clone even though it has no caller query budget.

### Minor 1 — Independent parity comparator does not assert component names

**Location:** `specimen-studio/tests/language-parity.mjs:239`.

Retained from parity review. Names are emitted but never checked; weights/fractions/reasons/spans are checked by position. Pin the ordered four names (and component count) to prevent a misleading trace label from passing. This does not invalidate the existing full-digest negative witness.

### Minor 2 — Public complexity getter disagrees with text-v3 classification

**Locations:** `specimen-studio/native/specimen-kernel/src/language_v3/model.rs:334-335`; `specimen-studio/native/specimen-kernel/src/text_v3/mod.rs:315`.

For `alpha, beta gamma delta epsilon zeta`, `PreparedAction::complexity_token_count()` returns 7 while the text key reports 6/Low, because the getter counts punctuation and the encoder excludes it. The encoder follows the selected profile, but callers of the exposed complexity API receive contradictory evidence. Share the lexical counting definition or rename/document the broader count and expose the profile count explicitly; add a direct agreement assertion.

### Minor 3 — Registry accepts call names the compiler cannot parse

**Locations:** `specimen-studio/native/specimen-kernel/src/language_v3/script.rs:106-110`, `:540-546`.

`Registry::new` accepts `my-text` as a declaration/alias name, but compiling `my-text("x")` returns Syntax because the parser accepts only alphanumeric/underscore call-name characters. Align call-name validation with parsing (separate identity/subsystem naming if needed). A validated registry should not silently contain an uncallable declaration.

### Minor 4 — Retained environment qualifications

**Location:** `docs/completion/execution/evidence/2026-10-22-language/gates/summary.json`, `limitations`.

The missing-libLLVM stripping diagnostics, dependency/build-script/future-compatibility notices, browser-build limitations and Bun 1.4.4 canary versus CI 1.4.0 remain disclosed. They are not new product blockers or hidden failing tests. Keep them attached to these executions; environment repair and CI-matching qualification are separate work.

## Focused counterexample evidence

No green suite or product gate was rerun. One synthetic standalone probe was compiled with pinned rustc and linked to the most recent retained gate-built kernel rlib. The source mechanisms were also confirmed in the exact reviewed diff; source/evidence snapshot hashes still match. This cache-linked probe is additional bounded counterexample evidence, not a clean rebuild or whole-program qualification.

```sh
/Users/donald/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin/rustc --edition=2024 /tmp/wdbx-language-final-review-probe.rs --extern specimen_kernel=specimen-studio/target/debug/deps/libspecimen_kernel-8d219e0fcb97448d.rlib -L dependency=specimen-studio/target/debug/deps -o /tmp/wdbx-language-final-review-probe
/tmp/wdbx-language-final-review-probe > /tmp/wdbx-language-final-review-probe.log
```

Cwd: `/Users/donald/sources/repos/wdbx`. Compile and probe exit 0. Retain for durable copying:

- Source: `/tmp/wdbx-language-final-review-probe.rs`, SHA256 `c772e4de8a108929af464d427a7e4b08cf9e4c4fe56728263b44c8723f3167be`.
- Raw output: `/tmp/wdbx-language-final-review-probe.log`, SHA256 `94f50d34151b69a45ac7c93802ec55c42e5eb0248e9f1171a1066ea752697620`.
- Quoted mismatch, Grammar negation and conjunction-shadowing cases each print `inhibited=false, executable=true` and `execution=Ok(Text("effect"))`.
- Complexity prints `getter=7, key_count=6, class=Low`; accepted hyphenated declaration prints `Err(Syntax)`.
- Empty query prints `257 records, work_steps=1: Ok(0)`; static source establishes the intervening complete-table copy.

## Retained validation and cannot verify

The retained four Studio gate summary records all exit 0: 221 browser/unit passes, lint, 221 native passes plus one existing ignored test, and native all-target Clippy. The reviewed witness/report records actual native/feature-WASM equality at 133421 bytes, SHA256 `0874bb27c8a593db459c47f06faa46e62cf2c917f31e7eaa78845a5df7475bae`, zero host calls and digest-corruption exit 1. Default WASM was compiled/export-inspected only. These retained checks did not cover the new counterexamples and do not override them.

Cannot independently establish scientific efficacy/frozen-case acceptance, real host effects/authentication, active engine/save/import migration, durable utility ingestion, all-target/installed/UI/hardware/elapsed qualification or ABI-owner acceptance from this tranche. No such claims are made here. Evidence raw-body preservation was already separately reviewed; this review examined its exact mapping/pin/closure changes and current snapshot identities rather than replaying that workflow.

## Declined to judge

- General natural-language understanding beyond the bounded local grammar: explicitly outside this packet; typed conservative refusal is permitted.
- Efficacy/calibration of the selected +20/-20 missing-feature outcomes: a disclosed design choice requiring separate research evidence, not a defect merely because the scores are surprising.
- Full-chunk scanning, ATP/jitter/voting/global permits and production route/effect registration: explicitly deferred; Important 3 concerns the implemented local query budget only.
- Real credential authorization, durable effect idempotency and host execution: the API intentionally returns pure plans and makes no corresponding operational claim.
- Legacy-ID conversion, saved-schema migration, substrate/crypto completion and ABI-owner receipt: separate workstreams; no current-tranche changes or qualification claimed.
- Runtime/toolchain repair and CI-equivalent/installed UI qualification: retained limits, not silently treated as successful evidence.

## Recommended final fix scope

Address all three Important findings together, preferably the three small assertion/API Minors too. Retain prior evidence as immutable; use fresh focused RED/GREEN and affected combined/parity evidence after edits, then refresh only current source mappings/catalog pins and explicitly record final review disposition. Preserve all C0/open/unknown states and default dispatch. One scoped rereview can check this integrated fix and its cross-boundary regressions without redispatching already reviewed historical slices.

## Method

Applied the supplied code-reviewer methodology. Read the current continuation ledger section, task briefs and relevant scoped reviews; read all changed product source/tests/profile/narrative once in bounded diff passes. Inspected the evidence diff's checker/closure/refresh/mapping and deduplicated repeated reference/hash records for crosscutting claim boundaries; recovered truncated narrative output with a bounded deduplicated pass. For the identified table-budget risk only, inspected unchanged identity snapshot/find_all/validation functions. No broad repository crawl, holdout/corpus access, subagents, git mutations, product edits, suite reruns or production effects. Only this requested report and temporary synthetic probe artifacts were written.
