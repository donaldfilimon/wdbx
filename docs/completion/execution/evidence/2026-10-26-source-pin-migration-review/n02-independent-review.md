# N02 independent review: model `package` and `tokenizer`

Review date: 2026-10-27. Reviewed tree: local main `f94900a`. Reviewer: an
independent read-only review agent (not the N02 implementer). This is the review
that the [source-pin migration review](README.md) records as pending for the N02
part of the `model` growth on `crates/abi-wdbx/src/lib.rs`.

**Overall verdict: `N02: APPROVE WITH REQUIRED FIXES`.** One Important finding
must be fixed: the production decoder and the independent Python witness
disagree on the error family for two tokenizer bytes. No Critical findings.

This record does not migrate a pin, edit `catalog-v1.json`, route `model` into
the substrate release catalog or change any C0 status. That routing is the
owner's decision. Approval of N02 code is not N03–N07 evidence, runtime or
training qualification, platform durability qualification or LYLX replacement.

## Scope

- `55f33a6` (`feat(model): implement bounded native packages and byte BPE`) and
  `8ad18a8` (`test(model): include N02 witnesses in direct runner`).
- The `85c3baa` WIP-preservation delta, merged by `2b4c231`. Against `55f33a6`
  it adds `package/write/preflight.rs`, `package/write/preflight/storage.rs`,
  `package/write/tests.rs`, `package/stage/tests.rs`, sync changes in
  `package/stage.rs`, writer changes in `package/write.rs`/`write/schema.rs`, and
  test edits. **The merged tree at HEAD was reviewed, not only the two commits**,
  because that tree is what a `lib.rs` pin refresh would admit. `git log
  8ad18a8..HEAD -- crates/abi-wdbx/src/model` shows only `85c3baa`.
- Files: `crates/abi-wdbx/src/model/package/**`, `crates/abi-wdbx/src/model/tokenizer/**`,
  the N02 edits to `model/{mod,limits}.rs`, `tests/model_package.rs`,
  `tests/model_package_bounds.rs`, `tests/model_tokenizer.rs`,
  `tests/support/model_package.rs`, `tests/fixtures/model-v1/{bpe-trainer-vectors-v1.json,field-table-errata-v2.json,n02-fixtures.md}`,
  `tools/wdbx_model_reference.py` (`train_bpe`) and
  `tools/tests/test_wdbx_model_reference.py`.
- Baselines: [binding design](../../../../contracts/model/native-package-design-v1.md)
  sections 1–11 and 14–16, [`package-v1.md`](../../../../contracts/model/package-v1.md),
  [`tokenizer-v1.md`](../../../../contracts/model/tokenizer-v1.md),
  [`components-v1.md`](../../../../contracts/model/components-v1.md),
  [`fixture-policy.md`](../../../../contracts/model/fixture-policy.md), and the
  [N01](../../../../plans/2026-10-25-native-model-contracts.md),
  [sequence](../../../../plans/2026-10-25-native-model-cpu.md) and
  [N02](../../../../plans/2026-10-25-native-package.md) plans.

## Checks executed

Pinned `rustc 1.99.1 (58cf8f9c3 2026-10-13)` with the rustup toolchain and
`/usr/bin` first on `PATH`, `CARGO_TARGET_DIR=.work/n02-review/target`,
`CARGO_BUILD_JOBS=2`, `CARGO_INCREMENTAL=0`, stdin `< /dev/null`. The host was
running unrelated builds concurrently, so durations are not performance
evidence.

| Command | Exit | Result |
| --- | --- | --- |
| `cargo test --locked -p abi-wdbx model` | 0 | Passed, but `model` is a test-name filter: integration binaries reported their N02 tests as filtered out, so this alone is not evidence. |
| `cargo test --locked -p abi-wdbx --test model_contracts --test model_package --test model_package_bounds --test model_tokenizer` | 0 | 5 + 15 + 16 + 7 passed, 0 filtered. |
| `cargo test --locked -p abi-wdbx --lib model::` | 0 | 4 passed (`package::write::tests`, `package::stage::tests`). |
| `python3 -I -m unittest discover -s tools/tests -p 'test_wdbx_model_reference.py'` | 0 | 14 tests OK. |
| `shasum -a 256` on `bpe-trainer-vectors-v1.json` and `field-table-errata-v2.json` | 0 | `04eadb1c…6fbd` and `0b5064e3…d678`, matching `n02-fixtures.md`. |
| I1 probe: witness `validate` on three rehashed `inference` variants | 1 (expected refusal) | `InvalidTokenizer:profile` for each. |
| I1 probe: scratch binary in ignored `.work/n02-review/probe` calling `read_bounded_snapshot` → `validate_framing` → `validate_model` on the same files | 0 | `NonCanonical:enum` for each; unmodified `inference` → `ValidatedModelState`. |

The full root gate (`bash tools/check.sh`) and `tools/check_completion.py` were
not run; this review needed only the focused checks above.

## Per-file verdicts

| File (under `crates/abi-wdbx/src/model/`) | Verdict | Notes |
| --- | --- | --- |
| `package/mod.rs` | Approve | Re-exports only; `size()` converts with a typed `CapacityExceeded`. |
| `package/read.rs` | Approve | Reads the 96-byte header, checks bounds, reserves once with `try_reserve_exact`, reads exactly `total`, then a 1-byte EOF probe. Short read is `Truncated`, extra byte is `TrailingBytes`, read errors (including `Interrupted`) are `Io`, no retry. Cancellation before every read. |
| `package/header.rs` | Approve | Magic, version, header reserved/flags, artifact cap, directory count 8..32, `96+80*count <= total` and `total + 16 MiB <= loading_bytes` before any allocation. Caller limits can only narrow. |
| `package/directory.rs` | Approve | Strictly increasing nonzero kinds; kinds 1–8 require flags 1 and version (1,0); unknown kinds require flags 0 and keep any version; contiguous positive ranges from `96+80*count` with no underflow (`end <= total` invariant); all component hashes in kind order before any schema decoding. Minor M5. |
| `package/components/mod.rs` | Approve | Fixed kind order, cumulative loading and metadata budget, opaque annotations charged fully, then cross-checks; `StateRoot`/`GenerationId` formulas match design section 2. `check_resume` refuses `InferenceOnly` (`NotResumable`) and compares all caller identities (`ResumeMismatch`). |
| `package/components/cursor.rs` | Approve | Checked `pos+n`, bounds before slicing, every input-sized reservation charged to the budget then `try_reserve_exact`. Families match the witness `Reader`. |
| `package/components/manifest.rs` | Approve | All section 3 bounds, `N<=node_capacity`, `M<=edge_capacity`, radius bit-exact 4.0. |
| `package/components/topology.rs` | Approve | Relation IDs 0..R-1, unique nonempty labels <=64 bytes, strictly increasing nonzero node/edge IDs, high-water `>= max` and `< u64::MAX`, exactly one Input and one Readout, `source_ref` only on Memory, duplicate triples rejected. Minor M2. |
| `package/components/tensors.rs` | Approve | Catalog order, axis kinds and dtype per table, Node/Edge ID payloads bounded before reservation, scalar product and total <= 1,000,000, exact byte length, finite values, coordinates in [-4,4], signed zero preserved. |
| `package/components/tokenizer.rs` | **Fix required** | Important I1 (normalization/special-policy family). Component size cap is checked before parsing. |
| `package/components/objective.rs` | Approve | Profiles, weights (`text_weight` bit-exact 1.0), candidate bound, nonzero digests. |
| `package/components/optimizer.rs` | Approve | Absent is exactly one tag byte; AdamW profile, all hyperparameter ranges, exactly 16 moments in tensor order with checked totals, finite first moments, nonnegative second moments, zero moments at step 0. |
| `package/components/rng.rs` | Approve | Five streams in order, nonzero state, `consumed < u64::MAX`, seed-derived initial state at `consumed == 0` (SplitMix matches design section 9), permutation bijection, `cursor < count`, batch 1..4, proposal-step rule. |
| `package/components/provenance.rs` | Approve | Nonzero identities, lineage pairing, ordered unique cohorts/roots/licenses, restriction bits and Diagnostic/licence rules. |
| `package/components/cross.rs` | Approve | Counts vs architecture, endpoint/relation existence, incoming degree, Node/Edge axis IDs equal topology order, dimension axes, scalar cap, vocab count, candidate capacity, purpose vs optimizer/RNG, moment counts, step/dataset/split/preparation cross-links, offline proposal counters, required root coverage, policy profile. Matches the witness `validate()`. |
| `package/write.rs` | Approve | Preflight before reservation, refuses unplanned growth, writes header last, then re-validates its own output through `validate_framing` and `validate_model`. `export_exact` streams the captured snapshot in 64 KiB chunks; zero-length write is `Io`. |
| `package/write/preflight.rs`, `preflight/storage.rs` | Approve | Raw storage coherence (dtype/variant, rank/axes, shape/length, moment vector lengths) before serialization; a non-allocating measuring pass; peak includes retained capacities, borrowed annotations and the output. |
| `package/write/schema.rs` | Approve | Field order matches sections 3–10; tensor and moment payloads go through `numeric`, vocabulary rows through `vocabulary_blob`; Absent optimizer/RNG write one tag byte. |
| `package/stage.rs` | Approve | Explicit caller directory only; Unix private (mode `& 0o077 == 0`, not a symlink) or typed `Io`/`unsupported-directory-durability`; object name is the hex `PackageDigest` (no path input from the package, no traversal); `create_new` mode-0600 temporary; file sync, no-clobber `hard_link`, directory sync; an existing object is verified by type, size and digest and never overwritten; incomplete files retained. Minor M3, M4. |
| `tokenizer/mod.rs` | Approve | `validate_contract` receives already typed enums, so I1 is not here; the Rust side of I1 is `components/tokenizer.rs`/`cursor.rs`. Profile, special IDs, nonzero digests, 260..4096 and `merges == V-260`, base vocabulary, per-token <= 65536, exact component size <= 1 MiB, merge rank/result/parents/uniqueness/byte concatenation; duplicate token bytes allowed. Minor M1. |
| `tokenizer/encode.rs` | Approve | Byte IDs, then each merge once in rank order, left-to-right, no overlap; 64 KiB input cap; every tested token charged against the 8,000,000-visit cap; literal special-looking text stays bytes; structural IDs only via `encode_sequence`. |
| `tokenizer/decode.rs` | Approve | `decode_bytes` rejects structural and out-of-range IDs and bounds output before allocation. `decode_sequence_bytes` strips one leading BOS and one terminating EOS, allows PAD only after EOS, rejects UNK, internal BOS/EOS and content after EOS. |
| `tokenizer/train.rs` | Approve | Corpus digest per design section 6; size/allocation preflight before any copy; overlapping pair counts within documents only; max count then lowest `(left,right)`; left-to-right non-overlapping replacement; checked work charging; 1 MiB and 65536-byte caps return `CapacityExceeded`; cancellation returns no vocabulary; no file, network or global state. |

## Findings

### Critical

None.

### Important

**I1. Tokenizer `normalization`/`special_policy` error family disagrees with the
independent witness.** `crates/abi-wdbx/src/model/package/components/tokenizer.rs:10-11`
decode these bytes with `r.e8()`, which maps an unnamed discriminant to
`NonCanonical:"enum"` (`components/cursor.rs:131-134`; enum conversions in
`contract/tokenizer.rs:11-46`). The witness `tools/wdbx_model_reference.py:201`
checks the same bytes in its tokenizer profile `require` and reports
`InvalidTokenizer:profile`.

Failure scenario: a package whose only defect is tokenizer `normalization = 1`
(or `special_policy = 0` or `2`), with recomputed component and directory
hashes, is refused by both sides but with different frozen families. The
design and the N01 plan require the Rust parser and the independent witness to
agree, and "tests must fail if Python ... disagrees". No N01 or N02 fixture
flips either byte, so the disagreement is invisible to both suites.
Observed: starting from the frozen `inference` fixture, setting tokenizer byte
20 (`normalization`) to 1, or byte 21 (`special_policy`) to 0 or 2, then
recomputing the component-4 and directory hashes, gives
`InvalidTokenizer:profile` from the witness and `NonCanonical:enum` from the
production decoder for all three vectors (see Checks executed).

Required fix (owner's choice of side): pick one family for these two bytes,
align the other implementation, and add a new versioned negative vector (do not
repurpose N01 fixtures) that both the Rust corpus test and the witness assert.
`NonCanonical` matches how the witness already treats other enum bytes (for
example the frozen `bad-role` fixture); `InvalidTokenizer` matches the witness's
tokenizer profile grouping.

### Minor

**M1. Zero training-split digest has different families on the two tokenizer
entry points.** `tokenizer/train.rs:27` returns `InvalidTokenizer` for a zero
split digest; the package decoder (`components/cursor.rs:167-178`, via
`tokenizer.rs:17-18`) and `ByteBpe::from_contract` (`tokenizer/mod.rs:80-84`)
return `InvalidProvenance`. The witness agrees with each side respectively
(`train_bpe` and `Reader.nonzero`), so this is consistent but surprising.
Document it or align it.

**M2. Schema-versus-trailing-bytes precedence differs between parsers.**
`topology.rs:41`, `rng.rs:44`, `provenance.rs:44` and `tokenizer.rs:35` call
`r.end()` before their semantic checks, while `manifest.rs:55`, `tensors.rs:94`,
`optimizer.rs:65` and `objective.rs:40` check inline before `end()`. The witness
runs `end()` after each parser, and its tokenizer checks profile, special IDs,
pair frequency and requested size while reading. A component with both trailing
bytes and a semantic fault (for example a wrong tokenizer identity plus a
trailing byte) gets `NonCanonical` from Rust and `InvalidTokenizer` from the
witness. Both reject; the design's "deterministic first-error order" is only
pinned between stages, not within a component. Pin or document it.

**M3. Temporary unlink failure is reported after a durable success.**
`package/stage.rs:99`: if `remove_file(temporary)` fails after `hard_link` and
the directory sync succeeded, the call returns `Io` although the object is linked
and durable. A retry is idempotent (the existing-object path verifies it), so
this is not data loss.

**M4. Existing-object check and open are separate path operations.**
`package/stage.rs:113-120` checks `symlink_metadata` then `File::open` follows
the path again. The documented precondition is a caller-owned private
directory, so this is acceptable; without that precondition it is a TOCTOU
window. An `O_NOFOLLOW` open plus `fstat` would close it.

**M5. Directory hash is checked before entry canonicality.**
`package/directory.rs:22-26` verifies `directory_sha256` before reading entries,
so a tampered noncanonical directory reports `IntegrityMismatch`, not
`NonCanonical`. The witness uses the same order (`frame()`), so this is
consistent; design section 11 lists "directory canonicality/version" before
"component hashes" and is silent on the directory hash itself.

**M6. Section 14 matrix items without a test or fixture.** The code paths exist
and read correctly, but nothing exercises: `cursor == count` (`rng.rs:48`),
objective profile/policy tags other than 1 (`objective.rs:13-30`), a
`dataset`/`split`/`preparation` mismatch between components 5 and 7
(`cross.rs:128-136`; only the step mismatch has a fixture), NaN/Inf first
moments, held-out sentinel documents in trainer input, and decoder-side dtype
mismatch. Only one legacy reader (`v2::open_object`) is probed with model
bytes; design section 14 asks for the existing format and Specimen readers
where supported.

**M7. Witness independence is implementation-level only.** The trainer vectors
in `bpe-trainer-vectors-v1.json` come from `train_bpe`, added to
`tools/wdbx_model_reference.py` in the same commit, by the same author, as the
Rust trainer. The Python is a separate slow implementation (dictionary counts,
`min` by `(-count, pair)`), which satisfies the fixture policy's wording, but it
is not author-independent evidence.

**M8. Status documents are stale on main.** `docs/plans/2026-10-25-native-model-cpu.md`
says N02 is "in progress in the isolated N02 worktree", and
`docs/contracts/model/README.md` still says the packet "supplies no production
parser, tokenizer". Both predate the `2b4c231` merge that put N02 on main as
public `abi_wdbx::model::{package,tokenizer}`. Update them when the owner
records this review's outcome.

## Review questions answered

- **Parsing safety for untrusted bytes.** All header, directory, offset, length,
  count and product arithmetic is checked before slicing or reserving; every
  attacker-sized vector is charged to the loading budget and reserved with
  `try_reserve_exact`. A few reservations precede the matching `take` (tensor
  values, moments, permutation), but each is already capped (1,000,000 scalars,
  65,536 entries) and budgeted. No `unwrap`/`panic` in non-test code; the
  remaining `expect`s are on fixed-width conversions. There is no zip or tar
  handling and no path inside the package: entries are byte ranges, and the only
  filesystem name is the hex package digest.
- **Tokenizer against its contract.** Encode, decode, sequence handling and the
  trainer match design section 6, including overlapping counts, tie order,
  duplicate token bytes, document boundaries and the work and size caps. The
  encoder cap is contractual: 64 KiB input with a full 3836-merge vocabulary can
  exceed 8,000,000 visits and then returns `CapacityExceeded`. There is no
  normalization (exact bytes) and no textual recognition of special tokens.
- **Error labels.** `ErrorKind` has exactly the 21 families of design
  section 11. Rust and the witness agree on all 53 frozen fixtures and on the
  `Reader` primitives; the exception is I1, with precedence notes M1, M2, M5.
  Errors carry only static field names, component kind and offset, never
  payload bytes.
- **Side effects.** Outside `package/stage.rs` there is no filesystem, network,
  environment, process, thread or global mutable state. `stage.rs` writes only
  in the explicit caller directory and never deletes anything except its own
  uniquely named, fully synced temporary after linking. No current-head,
  journal, registry or provider mutation.
- **Release scope and C0.** Neither commit nor the WIP delta changes
  `tools/check_substrate_release.py`, `catalog-v1.json`, `tools/check.sh` or
  `tools/check_completion.py`, and none claims runtime, training, platform
  durability or release qualification. The documentation keeps the no-claim
  wording (`n02-fixtures.md`: "neither a generation-head journal nor a
  supported-platform qualification"). The open release-scope issue is the one
  already recorded in the README: the merge put this public API on main before
  this review, without catalog routing. This record does not resolve that.

## Conditions for admitting `package` and `tokenizer` to a pin

1. Fix I1 with a new versioned negative vector asserted by both Rust and the
   witness, and get that fix reviewed.
2. The owner decides routing (release task or explicit exclusion) for `model`,
   per step 1 of the README's proposed remaining migration.
3. M1–M8 are not blocking. M6 and M8 should be done before N03 starts relying on
   these types.
