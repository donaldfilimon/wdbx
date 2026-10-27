# N02 fixes independent review (2026-10-27)

Independent read-only review of `9d386dd..8e00bbb` on branch
`task/finish-wdbx-20261027` (N02 I1 fix `dc54126`, M3 fix `2603bf7`, M6 matrix
vectors and clarification 1.0.2 `8e00bbb`). The reviewer was a fresh-context
agent given the code, the prior [N02 independent review](n02-independent-review.md)
and the questions only, not the author's conclusions. Its report follows
verbatim. Disposition of its minors is recorded after it.

---

**N02 fixes: APPROVE**

**Condition 1 is met.** I1 is fixed. The new vector file `tokenizer-profile-negatives-v1.json` is versioned. It is derived from frozen `inference.hex` without changing it: `git diff --stat 9d386dd..HEAD -- crates/abi-wdbx/tests/fixtures/` shows only `n02-fixtures.md` and the two new JSON files, with no `.hex` or `manifest.json` touched. Both suites assert it: the Rust test `model_package.rs::tokenizer_profile_bytes_have_tokenizer_family_like_the_witness` and the witness test `test_tokenizer_profile_bytes_have_tokenizer_family`. Conditions 2 (routing) and 3 are unaffected by this range.

## Answers

**1. I1.** `components/tokenizer.rs:12-15` now maps both bytes to `InvalidTokenizer:"tokenizer-profile"`, matching the witness at `wdbx_model_reference.py:201`. The `TryFrom` change in `contract/tokenizer.rs:17,39` has no other parser user: the only other caller, `train.rs:104-105`, uses the enum variants directly.

I tested values the fixture does not include, with my own scratch worksheet and a scratch Rust probe crate:

| Vector | Rust | Witness |
| --- | --- | --- |
| `normalization` = 2, 127, 255 | `InvalidTokenizer` | `InvalidTokenizer` |
| `special_policy` = 0, 3, 255 | `InvalidTokenizer` | `InvalidTokenizer` |
| Both bytes changed (1 and 0) | `InvalidTokenizer` | `InvalidTokenizer` |
| `reserved` = 1 | `InvalidTokenizer` | `InvalidTokenizer` |

`special_policy = 0`, which the original review named, is not in the fixture, but both sides agree on it.

**2. M3 is correct and safe.**
- Success still requires the object to be linked or verified, and the directory sync at `stage.rs:96-98` to succeed. A real link failure still returns `Err` (`stage.rs:88-95`).
- Only the post-link `remove_file` at `stage.rs:102` is ignored. The second directory sync (`stage.rs:103-105`) still reports its errors, so no false success is possible.
- The normal case still removes the temporary: the happy-path test at `model_package.rs:279` asserts one file left in the directory.
- The new test fails on the old code (old line 99 returned `staging-temporary-unlink`). It is meaningful but weak (see Minor m4).

**3. The eight matrix vectors are genuine.** I regenerated all 10 vectors (8 matrix plus 2 tokenizer) independently from the base `.hex` files, recomputing the component and directory SHA-256. All were byte-identical to the fixtures and every artifact hash matched. Each vector fires the check its name claims, in both parsers:

| Vector | Rust | Witness |
| --- | --- | --- |
| RNG cursor = count | `rng.rs:45-51` `order-bounds` (`cursor < count`) | `permutation/cursor` |
| Objective profile tag 2 | `objective-profiles` | `objective-profiles` |
| Objective policy tag 2 | `objective-policies` | `policies` |
| Objective dataset / split / preparation mismatch (3 vectors) | `cross.rs:126-136` `state-crosslinks` | `state-crosslinks` |
| First moment NaN / infinite (2 vectors) | `optimizer.rs:83-86` `moment-values` | `moments` |

On "both suites assert them": both check the file digest, the artifact hash and the error family. Only the Python tests check that a single field changed, and for the matrix file they use a subset check (`assertLessEqual`) rather than equality.

**4. The Clarification 1.0.2 docs are accurate overall.**
- M1 is accurate: `train.rs:27` gives `InvalidTokenizer`, `tokenizer/mod.rs:80-84` gives `InvalidProvenance`, and the witness matches each (`:388` and `Reader.nonzero`).
- M5 is accurate: `directory.rs:22-26` and the witness `frame()` both hash the directory first.
- M3 is accurate.
- M4 is literally true: `unsafe_code = "deny"` is inherited through `crates/abi-wdbx/Cargo.toml:36-37` `[lints] workspace = true`, and there is no direct libc dependency. Its reasoning is misleading (m3).
- One sentence in `tokenizer-v1.md` is too broad (m1).

**5. No regressions found.**

## Findings

### Critical
None.

### Important
None.

### Minor

- **m1. One sentence overstates the shared check order** (`docs/contracts/model/tokenizer-v1.md`, Clarification 1.0.1). It says "The check happens while component 4 is read, before its trailing-byte check, as in the witness." The same paragraph lists `identity`, `reserved`, the special IDs and `minimum_pair_frequency` as `InvalidTokenizer` fields. In Rust those are checked in `validate_contract` after `r.end()` (`components/tokenizer.rs:39` then `:56`). The witness checks them while reading.
  - Scenario: `reserved = 1` plus a trailing byte gives `NonCanonical` from Rust and `InvalidTokenizer` from the witness.
  - The probe shows the different path: `extra-reserved-1` reports Rust offset 6870, against 6891 and 6892 for the two enum bytes.
  - Fix: limit the sentence to `normalization` and `special_policy`, or point to the M2 note.

- **m2. Three matrix vectors fail two checks.** The objective dataset, split and preparation mismatch vectors in `n02-matrix-negatives-v1.json` change one field but break two rules. My probe confirmed that none of the changed digests is among the provenance data roots, so each would also fail `source-root-coverage` (`cross.rs:150-178`; witness `:349-350`). They hit `state-crosslinks` only because both parsers happen to run that check first, and design §11 does not fix the order within the cross-component stage. That contradicts the "single-defect" wording in `n02-fixtures.md` and Clarification 1.0.2.
  - Fix: add or replace them with versions that change component 7 instead (offsets 228, 260 and 292). The RNG digests are not in the coverage set. I tested this: both sides return `InvalidRngOrder:state-crosslinks`.

- **m3. The M4 reasoning is wrong** (`docs/contracts/model/package-v1.md`, M4 bullet). `O_NOFOLLOW` can be set through the safe `std::os::unix::fs::OpenOptionsExt::custom_flags` without any `unsafe` code. Only the platform constant needs libc or a hard-coded value. Checking the opened handle's own metadata against the earlier path check is another safe option. The conclusion (rely on the precondition) is fine. Fix the wording.

- **m4. The M3 test never sees the unlink fail** (`stage/tests.rs:143-157`). `TemporaryVanishes` deletes the temporary itself, so the test would still pass if `remove_file` were moved before the first directory sync. It also never covers a temporary that is actually left behind.
  - Fix: assert that exactly one directory entry remains after the first import. Optionally add a case where the unlink fails and the temporary stays.

- **m5. A leftover temporary is silent** (`stage.rs:102`). `StagedObject` has no field telling the caller a temporary was left behind, and its name (`.wdbx-stage-{pid}-{seq}.partial`) is never reported. On the `AlreadyExists` path the leftover is a separate full copy, up to 64 MiB. So "retained for explicit cleanup" depends on the caller searching the directory.
  - Fix: return the retained path or a flag. Otherwise, document the name pattern as the cleanup contract.

- **m6. Error offset points one byte past the bad byte** (`components/tokenizer.rs:13,15`). `r.error` runs after `r.u8()`, so the reported offset is past the faulty byte: a bad `normalization` reports the `special_policy` byte. The old `e8` path did the same. Offsets are not part of any fixture.

## Checks executed

All Rust runs used `rustc 1.99.1 (58cf8f9c3 2026-10-13)` with the pinned toolchain first on PATH, `CARGO_TARGET_DIR=<scratchpad>/review-target` and stdin `< /dev/null`. HEAD was `8e00bbb`, and the working tree stayed clean.

| Command | Result |
| --- | --- |
| `cargo test --locked -p abi-wdbx --test model_contracts --test model_package --test model_package_bounds --test model_tokenizer` | 5, 17, 16 and 7 passed, 0 failed, 0 filtered out |
| `cargo test --locked -p abi-wdbx --lib model::` | 5 passed, including `stage_reports_success_when_only_temporary_unlink_fails` |
| `PYTHONDONTWRITEBYTECODE=1 python3 -I -m unittest discover -s tools/tests -p 'test_wdbx_model_reference.py'` | 16 tests OK |
| `cargo clippy --locked -p abi-wdbx --all-targets` | No errors. Warnings only, none in the changed files. That matches the warnings-only policy at `tools/check.sh:7-9`. |
| `cargo fmt --check -p abi-wdbx` | No output |
| `shasum -a 256` on both new fixture files | `d6feb7eb…8e4a` and `1a724aa2…c966`, matching `n02-fixtures.md` and both tests |
| Scratch worksheet `.../scratchpad/probe/gen.py` | All 10 vectors `regen==fixture artsha_ok`; witness results as listed above. Also: 7 extra tokenizer values, `reserved` = 1, 3 component-7 mismatches, and a root-coverage probe (`mutated-root-in-provenance: False` for all three objective digests) |
| Scratch Rust probe `.../scratchpad/probe/rs`, a path dependency on `abi-wdbx` run with `cargo run --offline` | Results for all 21 vectors as listed in Answers 1 and 3 |

Not run: `bash tools/check.sh` (the full root gate) and `tools/check_completion.py`.

All scratch files are under `/private/tmp/claude-501/-Volumes-Extreme-SSD-Development-dev-repos-local-the-donald-company-company-legacy-wdbx/248bc0ef-829b-404b-a799-84a94584d21f/scratchpad/` (the `probe/` folder and the `review-target/` build directory).

---

## Disposition (author, 2026-10-27)

- m1 fixed: tokenizer clarification 1.0.1 now limits the "while reading" order
  to `normalization` and `special_policy` and points to the precedence note.
- m2 fixed: the three digest-mismatch vectors now change component 7 (offsets
  228, 260, 292), which is not a provenance data root, so each violates only
  `state-crosslinks` in both parsers. The fixture keeps its name and format
  (it never left this branch); its SHA-256 is now
  `b279d24f3c07a469c5cab76864b18ae34041b2d8d20d1c7c899fd20bdeda2a8c`. The witness
  test now asserts the exact set of changed non-hash bytes.
- m3 fixed: the M4 note names the safe alternatives and states that the
  precondition is the 1.0 mitigation.
- m4 fixed: `failed_temporary_unlink_after_durable_link_is_retained_not_reported`
  makes `remove_file` really fail and asserts the temporary is retained; it
  fails against the pre-M3 `stage.rs` (`staging-temporary-unlink`).
- m5 deferred and documented: the retained name pattern is now the stated
  cleanup contract in package clarification 1.0.2; no API change.
- m6 deferred: error offsets are not part of any fixture or contract.
