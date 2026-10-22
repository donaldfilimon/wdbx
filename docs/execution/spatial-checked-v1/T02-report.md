# T02 compatibility characterization

Prepared and locally verified at detached source baseline `08a69b3f22db61093e06527b3308b10589687022`, with uncommitted parallel packet work. This is characterization evidence, not acceptance of the complete dependent packet or the new checked API. T00/T01 acceptance review remains separate. No production code, existing goldens, canonical checkout, live stores, commits, pushes, or CI changed by this packet.

The new target discovers and passes exactly **3 tests**:

- `legacy_api_order_is_characterized`: 63/64/65 records, duplicate ID 7 with distinct payloads, Euclidean/Manhattan radius and nearest results, including a k cutoff inside the tie. Below 64, ties preserve insertion order (`positive`, `negative`); at/above 64, octant traversal reverses these observations. Cosine remains insertion ordered. This freezes historical behavior rather than requiring the new API to copy it.
- `legacy_reader_is_nonmutating`: repository v1 fixtures are copied into two fresh scratch layouts. The original manifest's highest active epoch is the valid empty epoch 3 checkpoint; the monolithic sample retains two blocks and one spatial record. Public versioned read-only opens verify the chain and preserve every relative file path and byte.
- `signed_bytes_remain_identical`: a deterministic signed v2 Artifact envelope is authenticated with the public object reader; signed v3 ledger replay, receipt retrieval/lookup and Ed25519 signature verification preserve all scratch file bytes. Raw v2 envelope, v3 ledger, v3 episode commitment and repository v1 fixture SHA-256 values are pinned in the compatibility manifest. Synthetic test seed 1 follows existing signing tests and is not an operator key.

The manifest's `report` supplies `unchanged`, `changed`, `blocked`, and `reason`. There is no production `CompatibilityReport` type introduced merely to satisfy proposed vocabulary. Attachment-index operations remain blocked because that API does not exist. The v2 fixture qualifies authenticated object preservation, not complete v2 journal/segment replay, encryption, migration or parent commitment coverage. Existing signing tests remain authority; CLI/MCP goldens remain in ABI.

## Commands and raw evidence

All commands ran in the isolated fixture checkout with Rustup selecting 1.99.1 explicitly, default features and locked dependencies.

| Exact command | Result | Artifact under `evidence/` |
| --- | --- | --- |
| `rustup run 1.99.1 cargo test --locked -p abi-wdbx --test wdbx_3d_compatibility -- --list` | 3 discovered | `t02-list.txt` |
| `/usr/bin/time -l rustup run 1.99.1 cargo test --locked -p abi-wdbx --test wdbx_3d_compatibility` | 3 passed, 0 failed/ignored/filtered | `t02-green.txt` |
| `rustup run 1.99.1 cargo test --locked -p abi-wdbx --test v3_episode_signing` | 7 passed | `t02-v3-signing.txt` |
| `rustup run 1.99.1 cargo test --locked -p abi-wdbx --test real_store parses_` | 3 fixture cases passed, 3 filtered; no live-store case executed | `t02-v1-fixtures.txt` |
| `rustup run 1.99.1 cargo test --locked -p abi-wdbx --test wdbx_3d_compatibility intentionally_unmatched_zero_case` | Cargo exits 0, 0 tests, 3 filtered; does not qualify | `t02-zero-cargo.txt` |
| `python3 -m unittest tools.tests.test_wdbx_3d_evidence.SourceManifestTests.test_zero_tests_do_not_qualify -v` | 1 passed; rejects zero-count evidence despite exit status 0 | `t02-zero-validator.txt` |
| `rustup run 1.99.1 cargo clippy --locked -p abi-wdbx --test wdbx_3d_compatibility` | passed, no warnings | `t02-clippy.txt` |
| `rustup run 1.99.1 rustfmt --edition 2024 --check crates/abi-wdbx/tests/wdbx_3d_compatibility.rs` | passed | `t02-format.txt` |
| `git diff --check` | passed | `t02-diff-check.txt` |

`zero_tests_do_not_qualify` is the root-owned Python validator negative case, not a tautological Rust test. It rejects executed test evidence with `test_count: 0` and accepts the otherwise equivalent positive-count record. The accompanying Cargo log demonstrates why exit status alone is insufficient; this does not claim automatic parsing of that log by the validator.

The first discovery compile exposed unsupported SHA digest hex formatting (`t02-compile-red.txt`), corrected in the test helper. The initial execution (`-- --nocapture`, `t02-baseline.txt`) passed two cases and rejected an incorrect expectation of two blocks in the newest empty checkpoint. The assertion was corrected to actual full-checkpoint behavior; a separate monolithic fixture now checks the populated state. These are test-authoring corrections, not production regressions or manufactured behavior-change red evidence.

`t02-source-hashes.txt` pins the final test and manifest source bytes; `t02-toolchain.txt` records the compiler build. Runtime/resources are retained verbatim in `t02-green.txt`. The full workspace/profile gate is owned by T01/root and is not claimed as this target's independent evidence. Optional FHE features, browser/desktop, migration fault recovery and durable attachment indexes were not executed or qualified here. Rollback removes only this packet's new test, manifest, report and evidence; scratch directories self-clean.
