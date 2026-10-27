# Studio ZIP 9 portable-save verification — 2026-10-25

The native portable-save dependency is pinned to ZIP **9.0.5**, with default
features disabled and only `deflate` requested. The importer maps fallible
entry-name decoding to `MalformedSave` and validates local/central headers,
duplicates and overlap before payload processing or durable writes. This
intentionally tightens malformed-input acceptance: valid archives retain their
format and behavior; ambiguous archive bytes are refused.

The implementation retains the exact accepted member namespace, bounded in-memory
reads (`take(max + 1)`), 512 MiB input/aggregate limit, 10,001 indexed-entry
limit, 64 MiB manifest and 32 MiB asset limits. Parser metadata is allocated
before the application's entry-count check; this change does not establish a
parser-allocation bound. It preserves stale-revision refusal, validation before
recovery/assets, snapshot commit order, temporary-file/sync/rename export and
failed-temp cleanup. No filesystem extraction or new mode policy is introduced.
No canonical publication/recovery behavior was changed or qualified here.

The actual ZIP 4 producer and source/fixture hashes are recorded in
`native/specimen-core/tests/fixtures/portable-archive/{README.md,provenance.json}`.
The retained source commit is `81ae29a7c472d42765fd1f09f75c1288b0252d30`;
the task started at `cee3308fe24cc82ae2035559708320ec1c840e3f`.
An isolated helper called that predecessor's actual `Store::export`. A ZIP 9
export is checked with independent Python `zipfile`, comparing semantic entries
and exact decompressed manifest/asset bytes, never compressed archive identity.
The legacy capture contains a referenced asset, fractional jitter and non-ASCII
text; typed snapshot fields and the reopened state are compared.

## Regression and verification results

The corrected pre-change suite had **8 passes and 2 expected failures**:
`duplicate_names_fail_even_when_payloads_match` and
`local_central_name_and_size_mismatches_refuse` both failed with
`malformed archive unexpectedly imported`. Early fixture-harness failures
(ephemeral lease identities and untyped f32 JSON expectations) were corrected
before that RED receipt; those are not claimed as production regressions.

The final archive suite passes **12/12**. It covers identical/different duplicate
manifest/assets, missing members, hash/CRC errors, unknown/unsafe names,
local/central name/size mismatch, overlapping members with matching headers,
invalid UTF-8, truncated payload/directory, streamed overrun, sparse input and
declared entry/count limits, inert symlink/special modes, stale revision and
failed-export cleanup. Refusals preserve snapshot, assets, recovery and durable
files and the reopened state. The suite serializes subprocess witnesses to
avoid inheriting another test's live writer lock. No 512 MiB expansion is built;
the unchanged aggregate arithmetic is not a newly stress-tested boundary.

Existing core conformance passes **19/19**, including the durable roundtrip,
exact fractional-value and referenced OCR asset cases. Package Clippy passes
with `-D warnings`; the pre-existing substrate unused-import warning remains
visible. `bun run test:native` passes **337 tests**, with **1 pre-existing GPU
parity test ignored** because it requires a qualified GPU. Browser/WASM gate,
whole desktop workspace qualification and Windows/Linux runs are deferred to the
integrated tranche; the browser build's literal `target/` assumption was not
worked around with stale output.

The pinned-toolchain `rust-objcopy --version` probe succeeded with the toolchain
library path. The native suite still emitted nested `rust-objcopy` SIGABRT
warnings for missing `libLLVM.dylib`, despite the actual Bun child environment
recording that same library path first in `DYLD_LIBRARY_PATH`. The cause is
unproven. The test process exited 0; this is not clean-loader evidence. The
`block 0.1.6` future-incompatibility warning also remains. No compiler
installation or global environment was changed.

Locked metadata and the all-target reverse ZIP tree pass. The only lock-graph
changes are ZIP 4.6.1 -> 9.0.5, removing its old `arbitrary`/`derive_arbitrary`
closure and adding `nix 0.31.3`/`typed-path 0.12.4`. Tokio **1.54.1**, ctutils
**0.4.4**, and the existing path-patched GLib **0.18.5** selection remain.
The root and fuzz locks and GLib vendor source were not edited.

Studio's actual `cargo audit --file Cargo.lock --json` exits **0**, reporting
zero vulnerability entries with advisory database commit
`21d909e4c8bd4e14c7aea58bf980c881a0053ab1`, with `ignore: []`.
Seven unmaintained advisories remain: paste (`RUSTSEC-2024-0436`),
proc-macro-error (`RUSTSEC-2024-0370`), unic-char-property
(`RUSTSEC-2025-0081`), unic-char-range (`RUSTSEC-2025-0075`), unic-common
(`RUSTSEC-2025-0080`), unic-ucd-ident (`RUSTSEC-2025-0100`), unic-ucd-version
(`RUSTSEC-2025-0098`). Two unsound warnings remain for lru 0.12.5:
`RUSTSEC-2026-0253` and `RUSTSEC-2026-0002`. No ignores were added and this is
not a full security-clearance claim. Historical failing evidence is retained.

## Portable receipt references

Commands ran from `specimen-studio/`, stdin at EOF, Rust/Cargo 1.99.1 with the
resolved toolchain bin and `/usr/bin` first, `ABI_WDBX_PERSIST` unset and
`ABI_WDBX_PATH=:memory:`. Build output used the task's independently cloned
`.work/zip9/target`, `CARGO_INCREMENTAL=0`, and DEV/TEST debug=0. The resolved
toolchain lib was prepended to child `DYLD_LIBRARY_PATH`. Disk checks remained
above the 10 GiB floor; no assets/caches were deleted. Private ignored receipts
under `.work/zip9/` retain exact command, paths, environment, start/end/exit,
source/lock hashes and output hashes. Public references below are SHA-256 of
those receipts; paths in this document are portable.

| Receipt | Executed command | Exit | Receipt SHA-256 |
| --- | --- | --- | --- |
| `legacy-producer-final` | `cargo run --locked --manifest-path ../.work/zip9/legacy/Cargo.toml --offline -- ../specimen-studio/native/specimen-core/tests/fixtures/portable-archive ../.work/zip9/legacy-store-final ../../../005-consumer-dependencies/wdbx/specimen-studio/conformance/starter.json` | 0 | `3e7771793939f9fd0cb51ea73052f4fa7cc8bf5eb87f87440e2222bdf87c910c` |
| `red-portable-corrected` | `cargo test --locked -p specimen-core --test portable_archive` | 101 | `026065887ad419ee29f6eb552fd337f6642dd06b954c9900a3c63af9e3bda14a` |
| `portable-archive-self-review` | `cargo test --locked -p specimen-core --test portable_archive` | 0 | `278cf41afff083814a936425931b7aa08c9014c63ef11c2bf49c6cf6a4213ae6` |
| `conformance` | `cargo test --locked -p specimen-core --test conformance` | 0 | `3eb53c542ddf98379bd624596843c9acc4f367d51eb122d39ef8c6ac50139a4c` |
| `package-clippy-self-review` | `cargo clippy --locked -p specimen-core --all-targets -- -D warnings` | 0 | `95c3ef3a42895c26895e62ca09aca85f00e6a73e7190a5563d2a7356d5267fda` |
| `native-four-crates` | `bun run test:native` | 0 | `0ab182d7d3a84736acc78f2b95833016e60add8cb1f64ed2b6d77a12eea9a2d8` |
| `locked-metadata` | `cargo metadata --locked --format-version 1` | 0 | `a925d272c17c5477ea7b820b7b5b2be094d96b23c421ef91ba6b4ae4b13771db` |
| `zip-reverse-tree` | `cargo tree --locked --target all -i zip@9.0.5` | 0 | `f45f825d584813bd3cb235a1541b289cee304473e795751d8f59372d52c58b13` |
| `studio-audit` | `cargo audit --file Cargo.lock --json` | 0 | `5f4f8a7f010e8e12618845156f5e9815b9abda1b57ed5ceaaeb018b6bff75816` |

Reverting this packet restores the prior ZIP reader/dependency. No live-store
migration or production signing/deployment happened. These native archive checks
do not qualify models, hardware, canonical recovery, desktop UI or release.


## Review correction — overlap witness

Review I1 found that the initial overlap fixture omitted the manifest's original
referenced asset. Its refusal could therefore survive loss of overlap validation.
Adding a successful non-overlapping control first exposed that defect: the exact
focused test failed with `MalformedSave: Archive is missing a referenced asset`.
The corrected fixture includes that asset, verifies that the control imports and
both asset payloads are readable, and redirects only the manifest's central
offset into the embedded local header. An independent raw stored-member reader
checks header agreement, CRCs and identical logical member payloads before and
after the redirect; explicit ranges distinguish disjoint control members from
the nested overlap. The overlapping form refuses with the existing full
unchanged-state assertions. Production and locks are unchanged.

The final archive suite passes 12/12, package Clippy and exact-path rustfmt pass.
The initial fix Clippy attempt rejected a needless range loop; the iterator fix
and final passing receipts are retained. Native and audit gates were not rerun
for this test-only correction. Existing CRC witness specificity (review M1) stays
nonblocking and deferred: it establishes fail-closed refusal, while both CRC and
asset content address disagree. Existing loader/audit/platform limits above stay
unchanged. Concurrent root/Studio AGENTS.md edits were not authored, reverted or
staged by this task.

| Receipt | Executed command | Exit | Receipt SHA-256 |
| --- | --- | --- | --- |
| `fix1-red-overlap-control` | `cargo test --locked -p specimen-core --test portable_archive overlapping_members_with_matching_headers_refuse -- --exact` | 101 | `d0ca866cacca2fccec83d666f670e0a26fcdbb02ce2a3dc0218c83e78614f7af` |
| `fix1-portable-suite-final` | `cargo test --locked -p specimen-core --test portable_archive` | 0 | `666944ae468d9c2e5067e4959f1e832e4023c2fb38952ede30e75d47188f3b98` |
| `fix1-package-clippy-final` | `cargo clippy --locked -p specimen-core --all-targets -- -D warnings` | 0 | `599aebdd8e6499011dbb930e76ab09c9917d152c7063334f0ad58635637fd8aa` |
| `fix1-rustfmt` | `rustfmt --edition 2024 --check native/specimen-core/tests/portable_archive.rs native/specimen-core/tests/support/portable_archive.rs` | 0 | `776946583f8b417e08ce152db6449ea14265fe05c3dd35b587fb2db507b1fb64` |
