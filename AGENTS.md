# AGENTS.md - WDBX Substrate

Canonical instructions; `CLAUDE.md` redirects here. Executable source wins over
prose. This is the Rust substrate, not the archived Workers MCP namesake.

## Start here / orientation

Short index only; details and rationale live in the sections below (do not
duplicate them here). Executable source wins over prose.

- **Gate (full):** `bash tools/check.sh` (instructions, sizes, fmt, clippy,
  tests). Pointer: `## Gates`.
- **Single test:** `cargo test -p abi-wdbx --test <file> <name>`; use
  `-- --exact` for one test (`## Gates`).
- **Feature flags (abi-wdbx):** `full-fhe`, `experimental-dghv-bootstrap`;
  default gate does not run them (`## Gates`).
- **Generated/hand-maintained files:** golden fixtures under
  `crates/abi-wdbx/tests/golden/`; retrieval baseline regenerable only with
  `WDBX_WRITE_EXPECTED=1 WDBX_REGENERATE_EXPECTED=1` (`## Gates`,
  `## Persistence Boundaries`).
- **Crate map:** `abi-foundation`, `abi-telemetry`, `abi-compute` (no
  workspace deps); `abi-core` (foundation + telemetry); `abi-wdbx`
  (compute + foundation). (`## Layout`).
- **Deep reference:** `## Ownership` onwards.

## Ownership

- WDBX owns durable episodic semantics: trust, supersession, contradiction,
  quarantine, and deletion. Consumers may expose lossy projections only if they
  declare what is dropped; they must not invent a second canonical episode.
  This is architectural ownership, not a claim that every operation is shipped.
- Public source does not make stores, episodes, evidence, credentials, or runtime
  state public. Never use the user's live store for tests or commit runtime data.
- Five local crates: `abi-foundation`, `abi-telemetry`, `abi-compute`, `abi-core`,
  `abi-wdbx`. There are no path dependencies back into ABI or Abbey. `abi-core`
  has no consumers inside this workspace; its runtime consumers live in ABI.
- ABI and Abbey consume `../wdbx/crates/...`. Keep all three repositories as
  siblings; mixing git and path sources creates distinct, non-unifying crates.
  WDBX's own build/CI does not require either consumer checkout.
- Never discard unrelated dirty work; stage exact paths.

## Gates

`rust-toolchain.toml` pins `1.99.1`; edition 2024, Rust floor 1.99.
Run the gate as `bash tools/check.sh`; `.github/workflows/ci.yml` runs it
verbatim on the self-hosted macOS arm64 runner labelled `wdbx` for trusted
events; fork PRs get no job (`docs/SelfHostedRunner.md`). Its steps:

```sh
bash tools/check_instructions.sh # CLAUDE.md == pointer + this file's git block
bash tools/check_rust_sizes.sh   # every tracked/untracked .rs <= 1000 lines
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace < /dev/null
```

- There is no `tools/cargo.sh` here. Select the pinned rustup
  toolchain, not a Homebrew Cargo/rustc pair; keep the toolchain bin directory
  and `/usr/bin` ahead of Homebrew Rust and Swiftly's compiler shims on macOS.
  `rust-toolchain.toml` also requests `rustfmt`, `clippy`, and `rust-src`; use
  `RUSTUP_TOOLCHAIN=1.99.1`.
- Unlike ABI, this clippy gate does not pass `-D warnings`: workspace lints deny
  unsafe code and clippy `all`, but `missing_docs` and pedantic remain warnings.
- The default gate does not exercise `abi-wdbx`'s `full-fhe` or
  `experimental-dghv-bootstrap` features. Feature work needs explicit checks;
  do not report default green as optional FHE runtime evidence. Check each with
  `cargo test -p abi-wdbx --features full-fhe` and
  `cargo test -p abi-wdbx --features experimental-dghv-bootstrap`.
- The 1000-line limit shapes layout: grow a module by splitting it into a
  sibling directory of submodules (as `wal/`, `v2/segment/` and
  `v3/episode/store/` already are), not by lengthening one file.
- Focused test: `cargo test -p abi-wdbx <filter>`; corpus target:
  `cargo test -p abi-wdbx --test abbey_contracts`.
- One integration test file: `cargo test -p abi-wdbx --test v3_episode_store`.
  One test inside it: the same command plus the test's name
  (`cargo test -p abi-wdbx --test v3_episode_store <test_name>`). A bare
  filter matches every test whose name CONTAINS it, so `-- --exact` is the
  reliable single-test form; without it a short name silently runs several
  tests and a green result can belong to a neighbour.
- `cargo test --workspace` includes `v3_cross_language_commitment` and
  `v3_cross_language_episode`, which run `tools/abbey_cbor_episode_v1.py` (a
  standard-library Python reimplementation of the `abbey-cbor-episode-v1`
  encoder and of the store's episode digest derivation) through `python3`,
  or `WDBX_PYTHON` when set. It fails rather than skips without an
  interpreter, so a missing Python reads as red, never as green. Standalone
  forms: `python3 tools/abbey_cbor_episode_v1.py verify-goldens` and
  `... verify-episode-goldens`. Three things must move together or the
  cross-language tests fail, which is the point: `StoredRecord::computed_digest`
  and `canonical_event` in `src/v3/episode/store/canonical.rs` are mirrored by
  `episode_digest` and
  `canonical_event` in the script, and the `GOLDEN_DIGEST` constants in
  `tests/v3_memory_candidate.rs` / `tests/v3_memory_edge.rs` are mirrored by
  `EPISODE_GOLDENS` in the script, so a deliberate golden regeneration updates
  all three.
- The Python witness has SEVEN commands, and only two are named above:
  `decode-corpus` (a bounded syntax-only JSON-lines decoding witness),
  `differential` and `encode` (a JSON envelope in on stdin; the envelope hex
  and digest out, or the Rust error name), and `episode-differential` and
  `episode-digest` (an `EpisodeWrite` wire form plus its `previous_digest`
  in; the digest out). They are the manual way to ask "what does the encoder
  do with this?" without a full Rust build. The golden FIXTURES are
  `tests/golden/abbey-cbor-episode-v1/*.hex` and `*.sha256` (the canonical
  CBOR bytes and their digest) plus `tests/golden/episode_write_memory_candidate.json`
  and `episode_write_memory_edge.json`.
- GOLDEN REGENERATION IS MANUAL, and that is worth knowing before you start.
  There is no blessing script: the `GOLDEN_DIGEST` constants in
  `tests/v3_memory_candidate.rs` and `tests/v3_memory_edge.rs`, the
  `EPISODE_GOLDENS` table in `tools/abbey_cbor_episode_v1.py`, and the
  `.hex`/`.sha256` fixtures are all hand-maintained constants, and a
  deliberate change updates ALL of them together. The Python side's
  `verify-episode-goldens` is the cross-check that catches a partial update
  — it derives the digests from the fixture JSON and compares them against
  what the Rust tests pin — so run it after touching any of them.
- The ONE regenerable expectation is the retrieval baseline, and it needs
  BOTH opt-ins:
  `WDBX_WRITE_EXPECTED=1 WDBX_REGENERATE_EXPECTED=1 cargo test -p abi-wdbx
  --test v3_retrieval_baseline`. `WDBX_WRITE_EXPECTED=1` alone is REFUSED —
  the test fails and the pinned file is left untouched — so a stray
  environment variable cannot repin a metric. The rewrite prints a
  `WARNING:` line on stderr, and `tools/check.sh` unsets both variables
  before `cargo test`, so the gate always asserts the pin and never rewrites
  it. `crates/abi-wdbx/tests/fixtures/retrieval-baseline/README.md` is the
  freeze's own specification and explains the corpus design; regenerate
  deliberately, and never to make a failing number look better.
- The gate runs the complete-episode decode witness's own unittest suite
  (`tools/tests/test_wdbx_complete_episode_decode*.py`) as a dedicated step
  before `cargo test`, via `"${WDBX_PYTHON:-python3}" -m unittest discover`.
  A missing or failing interpreter fails the gate, never skips.
- The gate runs the COSE encoding witness's own unittest suite
  (`tools/tests/test_wdbx_cose_sign1_p1.py`) as a second dedicated step, under
  its own narrow pattern so no unrelated suite is pulled in. A missing or
  failing interpreter fails the gate, never skips.
- Docs-only changes: compare claims with source/tests and run `git diff --check`.
  There is no dedicated Markdown validator. Do not call this a full Rust gate.
- The completion-contract checker is a SEPARATE check, deliberately not a step
  of this gate: `PYTHONDONTWRITEBYTECODE=1 python3 tools/check_completion.py`
  and `python3 -m unittest discover -s tools/tests -p test_check_completion.py`.
  It validates the C0 inventory and its hash-pinned sources; it is a contract
  check, not the Rust gate, not the Studio gate, and not hardware
  qualification. It never promotes a claim — every requirement stays
  `proposed`/`C0` with an empty evidence array, and a promotion needs a
  separately reviewed schema/version migration.
- `tools/tests/` holds FOUR Python suites and this gate runs TWO of them, by
  the narrow patterns above. The other two are the completion-contract suite
  and the 3-D evidence suite, and both are run by their own workstream, not by
  the gate: `python3 -m unittest discover -s tools/tests -p
  test_wdbx_3d_evidence.py`. Do not read "the gate is green" as "the 3-D
  evidence inventory was re-validated" — it was not.

## Layout

- Crate layering: `abi-foundation`, `abi-telemetry` and `abi-compute` depend on
  no workspace crate; `abi-core` uses foundation and telemetry; `abi-wdbx` uses
  only compute and foundation.
- `abi-compute` is deliberately free of storage dependencies, and that is what
  makes the `Accelerator` contract usable at all: `Accelerator`
  (`src/accelerator.rs`) is an object-safe `Send + Sync` trait, and the crate
  keeps record identity OUT of its own API — `ScoredIndex` carries a batch
  position and a similarity score, not a record ID — so an accelerator adapter
  can be implemented outside this workspace without a compute/storage cycle.
  `CapabilityState` is an evidence ladder, not a boolean: `CapabilityEvidence`
  records five INDEPENDENT facts (compiled, available, initialized, executed,
  runtime-verified) through named setters rather than a positional list of
  booleans, and `CapabilityState::new` refuses the invalid combinations —
  `CapabilityInvariantError` names the three: an initialized backend must be
  compiled and available, an executed backend must first be initialized, and a
  runtime-verified backend must have executed. An adapter may only advance a
  state after the real event, so a backend reporting "available" because it
  compiled is exactly the claim this crate exists to prevent; it is the same
  evidence discipline the C0-C7 levels impose at the repository level.
  The CPU lane primitives in `src/cpu.rs` are deterministic and run on STABLE
  Rust: `simd_lanes()` reports the host-selected `f32` lane count (16 on
  AVX-512, 8 on AVX2, otherwise 4), and this checkout carries no
  `#![feature(portable_simd)]` — which is what lets the pinned 1.99.1
  toolchain build it, and what the Studio's path dependency relies on.
- `abi-foundation` carries the ZIG-COMPATIBILITY surface, and one item in it is
  a trap. `src/wyhash.rs` is a faithful port of Zig's `std.hash.Wyhash`
  because NO Rust crate matches it: the `wyhash` crate implements a different
  version of the algorithm and returns entirely different values (for seed `3`
  over `"hel"`, Zig gives `10846395113768030678` and the crate gives
  `490820195397404894`). That distinction is load-bearing rather than
  academic — `textEmbedding` derives each of its 32 vector buckets AND each
  bucket's SIGN from this hash, and the resulting vectors are persisted into
  the user's WDBX store alongside vectors Zig already wrote, so a different
  hash yields embeddings that do not match the existing ones and cosine and
  semantic search across old and new records becomes silently wrong — no
  error, just degraded ranking.
  `crates/abi-foundation/tests/wyhash_zig_refs.txt` holds 188
  `(seed, len, hash)` triples emitted by the pinned Zig toolchain, covering
  every branch (the small-key paths, the `final1`-only range, and the 48-byte
  round loop including its strict boundary), and the module docs carry the Zig
  snippet that regenerates them. Do not "simplify" this to the `wyhash` crate.
  Note the deliberate contrast: `abi-wdbx` DOES depend on the `wyhash` crate,
  for the HE modulus derivation in `src/crypto_he.rs` — that use is unrelated
  to content addressing and is not in conflict.
- `abi-foundation`'s credential layer is a PROVIDER abstraction that never
  shells out. `src/credentials/` holds a JSON-file backend by default and an
  opt-in platform provider (macOS keychain, Linux secret service, Windows
  ACL), with the field list centralized in `CREDENTIAL_FIELDS` — the Zig
  original repeated the provider list in six places and missing one was a
  silent bug, so adding a provider is now a one-line change.
  `SecretTransport` has exactly one variant, `InProcessApi` ("direct Rust/OS
  API calls; no child-process command line is involved"), so no provider ever
  puts a secret on a command line. `abi-wdbx` uses this layer for exactly two
  things, and both concern the raw key FILES rather than a credential store:
  `write_owner_only_file_new` writes the 32-byte v2 key files (create-new, so
  it never replaces an existing path; mode `0o600` set BEFORE the bytes are
  written; and the file is REMOVED on any failure, so no secret material is
  left behind), and `windows_file_is_owner_only` re-checks the permission.
  The v2 keys therefore stay raw file paths named by
  `ABI_WDBX_ENCRYPTION_KEY_FILE`, `ABI_WDBX_SIGNING_KEY_FILE` and
  `ABI_WDBX_VERIFY_KEY_FILE`; routing them through the credential providers
  would change the env contract the store depends on, so it would be a
  deliberate decision, not a cleanup.
- `abi-wdbx` holds three storage generations side by side; know which one you
  are editing:
  - Legacy durable store: `DurableStore` (`src/durable.rs`) over the
    checkpoint/WAL (`wal/`), snapshots (`store/`), compaction (`segments.rs`)
    and HNSW search (`hnsw/`).
  - v2 causal multi-writer journals (`src/v2/`): per-writer append-only
    journals and head files, no shared manifest or writer lock; segments
    use `codecs/`. Owns the frozen v2 JSON commitment domain.
  - The v2 write path has an AUTHENTICATED-OBJECT layer and TWO durability
    steps, and this is the part that is not guessable from the module names.
    `src/v2/security.rs` defines the immutable object family (`ObjectKind`:
    `Segment`, `Journal`, `Artifact`) sealed under the `ABI-WDBX-OBJECT 2`
    magic, with optional XChaCha20-Poly1305 encryption and Ed25519 signing
    (`seal_object` / `open_object`; the key files are named by
    `ABI_WDBX_ENCRYPTION_KEY_FILE`, `ABI_WDBX_SIGNING_KEY_FILE` and
    `ABI_WDBX_VERIFY_KEY_FILE`, and `KeyMaterial`/`ObjectSecurity` redact
    their own `Debug` output). A committed transaction is visible only after
    its hash-covered object is DURABLE *and* its writer head is PUBLISHED, so
    "committed" is two separate facts. The legacy PLAINTEXT JSONL v2 journal
    remains readable, and that is the compatibility path the v1 fixtures
    depend on. `src/v2/lifecycle.rs` owns the versioned-snapshot operations
    (`open_versioned_read_only` / `open_versioned_writable`, `gc_versioned`,
    `rekey_versioned`, `verify_versioned`, `migration_status`). The phrase
    "object-layer scoped" in the 3-D compatibility test means exactly this
    layer — it is not a full journal qualification.
  - v2 still has an EXCLUSIVITY mechanism, and it is not a lock on the store.
    `src/v2/lease.rs` gives each writer a lease file under `leases/`
    (`acquire_writer_lease`) and gives maintenance an exclusive barrier
    (`begin_maintenance` writes a root `MAINTENANCE` marker). A writer
    refuses to open when the marker exists, and re-checks it AFTER taking its
    own lease, so a maintenance that begins mid-open is caught rather than
    raced. Maintenance then scans `leases/*.lease` and `try_lock`s each one: a
    lease it CAN lock belongs to a writer that is gone, so the stale file is
    removed, while a lease it CANNOT lock belongs to a live writer and the
    maintenance is refused. This is how `rekey_versioned` and `gc_versioned`
    in `src/v2/lifecycle.rs` obtain exclusivity in a model that deliberately
    has no shared manifest and no process-wide writer lock — an active writer
    lease must block a rekey. The v2 directory layout is `journals`, `heads`,
    `segments`, `artifacts`, `leases`.
  - v3 episode ledger (`src/v3/`): commitment encoder plus `EpisodeStore`,
    touching v2 only to read the signing-key file.
- `cluster*`, `rest/`, `spatial.rs`, `multiway/`, `fhe.rs`/`tfhe_demo.rs` and
  the compression/entropy codecs are reference subsystems beside these paths.
  Two of them carry a frozen surface a refactor would quietly break:
  `multiway`'s canonical export format is `abi-multiway-v1` with a hard
  ceiling of `MAX_MULTIWAY_RULES` (256) candidate rules, and `abi-telemetry`
  retains counter INSERTION ORDER because the CLI Prometheus exposition is a
  captured compatibility surface. Its table is bounded and never grows
  (`SLOT_CAPACITY` 128 names, `NAME_CAPACITY` 64 bytes each): a name that is
  empty, over-long, or arrives when the table is full is COUNTED rather than
  silently lost, and `abi_telemetry::dropped_events()` is how you read that
  count, so check it before concluding a counter is simply absent.
- The 3-D spatial work is TWO paths, and knowing which is which matters.
  `src/spatial.rs` is the LEGACY `SpatialIndex3D` that the on-disk formats and
  the Zig-compat fixtures depend on; `src/spatial3d/` is an ADDITIVE *checked*
  layer over it and changes no legacy storage: `numeric.rs` is the validated
  primitive set of the `cartesian-f64-v1` profile, `legacy_checked.rs` gives
  complete-or-error Euclidean queries over the legacy point population, and
  `octree_checked.rs` is a TRANSIENT checked adapter of the legacy octree
  (no durable generation or frame). The answers are pinned against an
  independent complete-scan oracle in `tests/support/spatial_oracle.rs`, and
  five test targets carry the evidence: `wdbx_3d_oracle.rs`,
  `wdbx_3d_checked_points.rs`, `wdbx_3d_octree_parity.rs`, `wdbx_3d_numeric.rs`
  and `wdbx_3d_compatibility.rs` (the last pins the LEGACY reader's behaviour —
  selected legacy readers, threshold ties, duplicate IDs and signed bytes —
  and is explicitly object-layer scoped, not full journal qualification). The
  inspection inventory is `docs/claims/wdbx-3d/source-manifest.json`, checked
  by `python3 tools/validate_wdbx_3d_evidence.py
  docs/claims/wdbx-3d/source-manifest.json`; that validator admits DOCUMENT
  STRUCTURE and does not establish source authenticity or product behaviour.
  The bounded packet's handoff, evidence and independent reviews are under
  `docs/execution/spatial-checked-v1/`. Not implemented: the rest of the
  requirements registry beyond this first packet, and the Neural Specimen
  implementation.

## specimen-studio/

- `specimen-studio/` is the WDBX Specimen Studio (Vinext/React Worker plus a
  Tauri/Rust desktop half), folded in with full history on 2026-09-28 from the
  retired `donaldfilimon/wdbx-specimen-studio`. Its own `AGENTS.md` is canonical
  inside it; read it before editing there.
- It is a separate Cargo workspace (`native/specimen-core`, `src-tauri`), excluded
  from this workspace (`[workspace] exclude`) and from `tools/check_rust_sizes.sh`
  (it vendors glib). `tools/check.sh` does not build or test it. From this
  checkout's root, `cargo studio` runs the desktop app, `cargo studio-build`
  builds it, and `cargo studio-test` tests its workspace; all three are
  aliases in the root `.cargo/config.toml`, they point at
  `specimen-studio/Cargo.toml` with `--locked`, and they do not join the root
  workspace. The aliases are relative to the repository root, so from a
  subdirectory use `--manifest-path specimen-studio/Cargo.toml` instead. Its
  gates run from `specimen-studio/`: `bun run check`, `bun run lint`,
  `bun run test:native`, `bun run check:native`.
- It consumes `abi-wdbx` and `abi-compute` by PATH dependency into this
  checkout's `crates/`: `abi-wdbx` in `native/specimen-core/Cargo.toml`;
  `abi-compute` in both `native/specimen-core/Cargo.toml` and
  `native/specimen-kernel/Cargo.toml`. The switch from a pinned git
  revision to path dependencies was a deliberate, recorded decision: the
  pinned revision (`3ac03f0`) required the nightly-only
  `#![feature(portable_simd)]` gate in `abi-compute`, which stable Rust
  1.99.1 rejects (E0554). A path dependency carries no `source =`, so the
  studio's `Cargo.lock` pins nothing for these two crates. Switching back to a
  pinned git revision would be its own recorded decision; the dependency style
  is a deliberate choice, not a cleanup.
- Its CI is `.github/workflows/specimen-studio-*.yml` (subdirectory workflows do not
  run). Workflow-path provenance checks inside the studio name those files.

## Persistence Boundaries

- `crates/abi-wdbx/tests/golden/` owns format fixtures. CLI/MCP output goldens
  and the scheduler integration oracle remain in ABI (`tests/golden/` and
  `crates/abi-cli/tests/golden_scheduler.rs`); do not duplicate them here.
- `tests/golden/wdbx-format.md` is the READ-COMPATIBILITY SPEC, and it is the
  first thing to read before touching `format.rs` or the segment reader. It
  was reverse-engineered from a census of the live store rather than from the
  Zig source, because that store holds the user's real segments and a
  format-incompatible rewrite silently orphans them. The rules in it are not
  guessable from the code: each segment is a COMPLETE CHECKPOINT of the whole
  store rather than a delta, so keys legitimately repeat across segments; the
  parser must accept all SIX record types even though the live store holds
  only three, because the serializer emits the other three; a `kv` value is
  an OPAQUE string that often holds double-encoded JSON, so parsing it
  eagerly breaks on values that are not JSON; and the two hash fields are each
  encoded in one of TWO ways, because Zig's JSON stringify emits a byte array
  as a string when the bytes happen to be valid UTF-8 and as an integer array
  when they are not. So `hash` is always an array, while `prev_hash` is an
  array EXCEPT in a segment's genesis block, where the all-zero digest IS
  valid UTF-8 and serializes as a string (measured over the live store: 4176
  arrays and 0 strings for `hash`, 4136 arrays and 40 strings for
  `prev_hash`). A reader that fixes one encoding per field fails on the
  genesis block of every segment.
- `DurableStore` holds `<base>.writer.lock` for its lifetime. The bounded
  `WouldBlock` retry in `src/durable.rs` handles fork/exec descriptor inheritance;
  persistent contention is `WriterBusy`, not permission to open without a lock.
- `StorePaths::new` accepts a directory; ABI CLI arguments use a base path.
  Abbey's `<state>/wdbx/` directory is `<state>/wdbx/wdbx` to the ABI CLI.
- `crates/abi-foundation/src/env.rs` owns ABI environment names and accessors.
  Tests use its override/reset/locking hooks; empty values count as unset.
- Preserve the frozen v2 JSON commitment domain and parent ordering. The v3
  `abbey-cbor-episode-v1` encoder in `src/v3/commitment.rs` is separate, with
  sorted parent digests; it is not a general CBOR decoder or episode signer.
- `src/v3/episode/` owns the single-writer episode ledger (`store.rs`), canonical
  commitment calculation (`store/canonical.rs`), policy/consent checks and replay
  rejection (`store/validate.rs`), and content-free receipts.
  Transport JSON and adapter-provided digests are not canonical authority.
- Episode signing lives in `src/v3/episode/signing.rs`, not in the commitment
  encoder: the store signs the 32-byte digest with Ed25519 when opened via
  `EpisodeStore::open_with_signer`. The signature is detached (stored beside the
  record, outside the envelope), so digests are unchanged and unsigned records
  serialize byte-for-byte as before. `signer_key_id` is derived from the key, not
  chosen. Keep the episode key distinct from the v2 segment-signing key. Only the
  canonical writer signs; adapters never do. A ledger holding signed records
  will not open on a build that predates this. A private crate-internal
  `COSE_Sign1` prototype now lives in `src/v3/episode/cose/`: a bounded tag-18
  container, an RFC 9052 `Sig_structure` Ed25519 signature over the same 32-byte
  digest, an `Integrity` axis kept apart from the key axis, and an in-memory
  `TrustList` with Pending/Active/Retired/Revoked, a revision counter and
  terminal revocation, where retired and revoked keys still resolve for
  historical verification but never authorize a new admission. A
  standard-library-only Python witness (`tools/wdbx_cose_sign1_p1.py`, driven
  by `tests/v3_cross_language_cose.rs`) now proves *encoding agreement* across
  the two languages: the same container bytes must parse to the same protected
  header, key id, algorithm, payload and RFC 9052 `Sig_structure` bytes, and
  the same malformed containers must be refused with the same error names. The
  Python side does not verify signatures and cannot (this environment has no
  Python cryptography library, and the design guidance forbids hand-rolling
  cryptography in Python), so the witness covers bytes only. The profile is
  unratified, has no cross-language verifier, no production key custody and no
  admission semantics, and it changes no committed digest; the detached
  `EpisodeSignature` ledger format is unchanged. Not implemented: cross-language
  verification, source-side signatures.
- Quarantine, contradiction, and resolution are `EpisodeEvent::MemoryEdge`
  variants (ABI spec `2026-09-16-spec-memory-edge-episodes.md`), not fields on
  `MemoryCandidate`: adding a field there changes every existing candidate
  digest and the pinned golden that abbey-bot reproduces. New memory vocabulary
  goes in as new event variants. A service may quarantine or contradict; only
  guild owner/administrator/manager or organization owner may resolve. Edges
  never hide a record and never block `forgets`. `tests/v3_memory_edge.rs` pins
  the edge golden; `tests/v3_memory_candidate.rs` must keep passing unchanged.
- Supersession lineage reads live in `src/v3/episode/store/lineage.rs`:
  `EpisodeStore::superseded_by()` and `EpisodeStore::supersession_lineage()`
  project the committed `supersedes` edges as content-free digests and flags.
  Both maps are rebuilt in `apply_record`, so live writes and replay-on-open
  answer alike; `tests/v3_supersession_lineage.rs` pins it. Holds and candidate
  revocation live in `src/v3/episode/store/holds/`.
- Structured admission decisions live in `src/v3/episode/admission.rs`:
  `AdmissionClass` (the closed five-class retention vocabulary),
  `AdmissionReason` (exactly one reason per `EpisodeStoreError` variant, plus
  `Accepted`) and a content-free `AdmissionDecision`.
  `EpisodeStore::decide_write` (`store/admission_gate.rs`) runs the same
  validation, commitment check and quota checks as `propose_write` and appends
  nothing; `EpisodeStore::decide_write_outcome` answers the same question in
  one content-free value that never fails, carrying the class and the reason
  (`Accepted`, or the reason the refusal maps to), so a caller never has to
  match on an error string. The three budget checks live in
  `EpisodeStore::check_quotas` (`store/validate.rs`), now shared by
  `propose_write`, `decide_write` and `preview_commitment`, so a preview no
  longer succeeds for a write the append would refuse on budget;
  `check_quotas` also charges the retention sidecar's own size ceiling, at
  the greatest size a retention line can have, and the decision and the
  preview measure the signed line the append writes, so all three paths reach
  the same verdict. `EpisodeStoreError` and its labels are
  unchanged, so the ABI gateway's status mapping is unaffected;
  `tests/v3_admission_decision.rs` pins it. The quota administration slice
  below adds the budget override those checks read.
- Every v3 sidecar writes through the SAME discipline, and it is deliberately
  NOT the shared `abi-foundation::io::write_file_atomic` helper. The
  foundation helper writes a sibling temporary file, `sync_all()`s it and
  renames over the target, but it does NOT synchronize the parent directory.
  The v3 sidecars each write a temporary file, `sync_all()` it, rename it
  over the target, and then `sync_directory(parent)`, so a reader sees the
  old file or the new one and the directory ENTRY is durable too. There are
  SIX near-identical atomic-write implementations —
  `episode/store/quota/index.rs` (`write_atomic`), `episode/store/deletion/index.rs`
  (`write_sidecar`), `federation/sidecar.rs` (`write_atomic`),
  `replication/sidecar.rs` (`write_atomic`), `offline/sidecar.rs`
  (`write_atomic`) and `ranking/vector.rs` (`persist`) — and the duplication
  is deliberate: each is keyed to its OWN error type (`EpisodeStoreError`,
  `FederationError`, `ShardError`, `OutboxError`, `VectorBindingError`), and
  those error labels are frozen because the ABI gateway maps them to status
  codes. Unifying them is therefore not a cleanup, and swapping a v3 sidecar
  write for the foundation helper would silently drop the directory-entry
  durability the ledger depends on. The same per-module split holds for the
  READ side: bounded readers are `read_bounded_file` /
  `read_bounded_file_if_present` / `read_bounded_ledger` in
  `episode/store/filesystem.rs`, and `read_bounded_optional` in the
  federation, replication and offline sidecars. Symlink rejection is NOT
  universal — the episode store exposes `reject_symlink_if_present` in
  `store/filesystem.rs` and the offline and federation sidecars carry their
  own copies, but the quota, deletion, replication and vector writers do not
  call it.
- Quota administration lives in `src/v3/episode/store/quota/` (a directory:
  `quota/mod.rs` for the API, `quota/index.rs` for the sidecar). A guild's
  budgets still come from the host policy, and an authority may *adjust*
  them at runtime: `EpisodeStore::decide_quota_adjustment()` is the
  non-mutating decision — it reads no clock, because the instant is recorded
  only by the commit — `EpisodeStore::commit_quota_adjustment()` commits
  through the single writer, `EpisodeStore::clear_quota_adjustment()` lifts
  an adjustment, `EpisodeStore::quota_state()` reports the effective budgets,
  the usage and whether the usage is above the effective budget
  (`over_budget`), and `EpisodeStore::quota_history()` reports every decision
  the sidecar holds for a guild, in order. The effective budget is the host
  policy's budget for that guild, lowered to a recorded adjustment's budget
  only while the policy still states budgets at or below the ones in force
  when the adjustment was made, and never above the policy's budget. The host
  policy is therefore the outer bound: no adjustment may set a budget above
  it, whoever makes it, and the host restores a larger budget by changing the
  policy or by clearing the override. Only an authority that may raise a
  budget may clear one, and a writer can therefore never freeze a guild's
  budget permanently — a budget is a *guild* budget, so a service that
  lowers it restricts every writer in that guild, not only itself. The
  effective budget is what `check_quotas` reads, so an adjustment changes
  what the store accepts and the change is visible in `decide_write`,
  `preview_commitment` and `propose_write` alike. The adjustment is a
  governance decision, not an episode, so it lives in a derived sidecar
  beside the ledger, `episodes.v1.quotas.jsonl`, in the established shape:
  content-free lines (guild, the two budgets, the policy's budgets in force
  when the decision was made, the authority, the instant), a checksum over
  each line's own fields, and a HEADER line naming the number of decisions
  and a checksum over all of them. The header is what makes a truncation
  detectable, and a truncated file, a trailing fragment and a bad checksum
  are all refused as `Corrupt`: the writer only ever writes whole lines, so a
  fragment is damage rather than a discarded crash tail, and the sidecar is
  never opened with fewer decisions than it holds. The file is created with
  the first adjustment rather than with the store, every write replaces it
  atomically, and it is written by the same single writer; it survives close
  and reopen. Authority: a guild owner, administrator or manager and an
  organization owner may adjust a budget in either direction; a *service*
  may only lower one, because a writer that could raise its own budget could
  defeat the budget; a human subject may not adjust a budget at all. That
  vocabulary is *actor-class* based, and the class rule is only the first of
  two necessary conditions: the guild-scoped authority map below decides
  whether the principal may act for the target guild at all. Safety: an
  adjustment may never lower a budget below the
  usage the guild has already recorded, because that would retroactively
  invalidate writes the ledger already holds; such an adjustment is refused.
  The same state is reachable without the adjustment API, when a host policy
  is lowered below the recorded usage; the store *surfaces* it through
  `quota_state`'s `over_budget` instead of refusing to open, because
  refusing would brick a store whose policy was lowered.
  `EpisodeStoreError` and its labels are unchanged.
  `tests/v3_quota_windows.rs` pins it. Not implemented: a per-caller or
  per-class budget, and quota administration that is itself an episode (that
  needs a new event variant and a coordinated golden regeneration).
- The GUILD-SCOPED AUTHORITY MAP lives in the host policy:
  `GuildEpisodePolicy::authorized_principals` is an optional,
  `#[serde(default)]` list of the principals that may act as authorities
  *for that guild*, and the store reads it in
  `src/v3/episode/store/authority.rs`. It is host policy and not episode
  material: the canonical commitment binds the policy *version*, never the
  policy itself, so naming a principal moves no committed digest, the frozen
  v3 record format is untouched, and `ActorRef` keeps its two fields (a guild
  field on it would move every candidate digest and break the pinned
  goldens and the Python witness). The rule has two halves and both are
  necessary:
  - the guild's own map — when a guild names a non-empty list, only a
    principal on that list may place a hold, release a hold, revoke a
    candidate, adjust a budget, or lift an adjustment for that guild, whatever
    class it carries;
  - the store-wide scope — a principal that *some* guild names is scoped to
    the guilds that name it and holds no class-only authority anywhere else.
    Without this, naming the owner of one guild would leave that owner free
    to act for every other guild, which is the hole the map exists to close.
    The scope is derived from the whole policy when the store opens, so the
    answer is a pure function of the policy: no clock, no state, and the same
    policy always yields the same verdict. What the map closes, for the
    principals the host names, is that hole; a principal that no guild names
    is not closed by the map at all and still reaches every guild whose
    policy states no map of its own, on the class rule alone.
  The actor *class* remains a second, necessary condition: a service may
  never place a hold, revoke a candidate, raise a budget, or order a
  destruction, even when the map names it, and a human subject is never an
  authority. The three states of the map are pinned: an **absent** map leaves
  the guild to the class rule alone (a *compatibility behaviour, not a
  security guarantee* — a host that wants a guild closed to unknown
  principals has to name them), a
  **non-empty** map constrains the guild to the principals it names, and an
  **empty** map is the same state as an absent map (it names nobody and
  constrains nobody of its own, so an empty list is not a closed guild).
  `EpisodeStore::actor_may_act(guild_ref, actor)` is the store-side
  predicate, and it is the guild-scoping half only: the class rule is
  applied per action. Every refusal answers the same closed, content-free
  `EpisodeStoreError::InvalidTransition`, which names neither the principal
  nor the guild, so a refusal is indistinguishable between "not authorized
  for this guild" and "no such principal". `EpisodeStoreError` and its labels
  are unchanged. The honest limit: the substrate still cannot *verify* that
  a principal really holds a role; the host policy is the authority of record
  and the store trusts the caller to present the right principal and the
  right class. `tests/v3_guild_authority.rs` pins it. Not implemented: the
  map does not constrain the canonical *write* path (a service still records
  memory candidates and memory edges, and a `resolves` edge is still admitted
  on the class rule alone, because the write path is canonical and its
  replay has no policy to read); the map does not reach the lifecycle
  `EpisodeEvent::Approval` (`store/validate.rs`), whose approver is
  class-checked (a valid actor, not a service, not the requester and not the
  proposer) but is *not* guild-scoped — no guild check is applied to it, so
  a principal labelled `GuildOwner` for guild A may approve an operation in
  guild B. That is a missing *guild* check, not a contradiction of the class
  rule: an approval is a **workflow** step, not a governance action over a
  guild's resources — it accepts one proposal and confers no authority over
  anything else — so the rule that a human subject is never an *authority*,
  a rule about governance authorities, does not reach it, and a
  `HumanSubject` is an admissible approver there by design; what the store
  does not do is verify that the approver holds any standing in the guild of
  the operation it approves. The map **does** reach the deletion
  lifecycle's destruction requests (`store/deletion.rs`):
  `RedactionRequest` and `ObjectKeyDestructionRequest` now carry an `actor`,
  every destruction order is refused unless the principal is one the guild
  authorizes *and* the class is one that may destroy (a guild owner,
  administrator, manager, or an organization owner — never a service, never
  a human subject), and the sidecar lines record the principal, so a
  destruction order is attributable. The map is not committed, signed, or
  replayed: it is host policy, and changing it changes what the store accepts
  from the next open onwards — the store binds the policy at open and never
  reloads it. A hold
  placed under one map can be released only under a map that still names the
  releasing principal.
- Retention classes and TTL expiry live in `src/v3/episode/store/expiry.rs`.
  `GuildEpisodePolicy::retention_ttl_ms` is the fallback TTL for `session`
  and `operational` candidates; it is optional and `#[serde(default)]`, so a
  host policy that omits it still parses and still means "no expiry".
  `GuildEpisodePolicy::retention_windows` is an optional per-class map
  (`session_ttl_ms`, `operational_ttl_ms`, `durable_ttl_ms`), also
  `#[serde(default)]`: an explicit per-class window wins over the single
  TTL, the single TTL remains the fallback, a class with neither never
  expires, and `durable` candidates never expire whatever the policy says.
  The store binds the deadline
  (`arrival + ttl`) once, when the candidate is admitted, and
  `EpisodeStore::retention_state()`, `EpisodeStore::due_expirations()` and
  `EpisodeStore::expiry_writes()` report it, decide what is due, and emit the
  `forgets` writes. The emitter never writes: the caller routes the emitted
  writes through `propose_write`, the one canonical write path. The v3 record
  format carries no arrival timestamp and is frozen (a new field would change
  every episode digest and break the pinned goldens and the Python witness),
  so the deadline is committed in a derived index beside the ledger,
  `episodes.v1.retention.jsonl`: digests and deadlines only, no episode
  content and no second canonical episode, written by the same single writer
  before the ledger line it describes, and created with the first deadline
  rather than with the store. A candidate with no deadline is never expired
  on a guessed time; it is kept. The write gate refuses a zero window
  outright — a single zero TTL or a per-class zero — the only way a deadline
  can be closed at admission, because the deadline is
  bound from the store's own clock at the instant of admission. A positive TTL
  is admitted and may expire immediately afterwards; the gate never compares a
  candidate's deadline against a caller-supplied clock. A candidate under an
  open quarantine or contradiction is not expired behind an open review.
  `tests/v3_retention_expiry.rs` and `tests/v3_retention_windows.rs` pin it. Not
  implemented: a *candidate-selected* retention window (it would need a
  digest-bound field on `MemoryCandidate`, or a new event variant, and a
  coordinated regeneration of every pinned golden and of the Python witness),
  the destruction of an expired record's payload
  and derived copies (the store reclaims the record and records the request;
  the consumer owns the bytes).
- The deletion lifecycle lives in `src/v3/episode/store/deletion.rs` (with
  `deletion/index.rs` for the two derived indexes) and
  `src/v3/episode/store/gc.rs`. `DerivedCopyKind` is the closed vocabulary of
  the derived copies a consumer may hold (embedding, index entry, export,
  cache); `EpisodeStore::decide_redaction()` and
  `EpisodeStore::commit_redaction()` record a redaction request for a
  *forgotten* target in `episodes.v1.redaction.jsonl`, and
  `EpisodeStore::redaction_state()` reports the kinds a target has. The store
  holds no derived copy: its contract is "a redaction request exists for
  target X covering kinds K1..Kn", and destroying the consumer's copies is
  the consumer's obligation — that is NOT implemented here. `ObjectKeyRef` is
  the v3 episode namespace's own bounded tenant/key reference (not the
  complete-episode prototype's private type);
  `EpisodeStore::decide_object_key_destruction()`,
  `EpisodeStore::commit_object_key_destruction()` and
  `EpisodeStore::object_key_destruction_state()` record and report
  destruction requests in `episodes.v1.object_keys.jsonl`. A request for a
  live record is refused: a live record's payload is not destroyed under the
  canonical ledger, and the refusal does not distinguish a live target from
  an unknown digest. Both requests carry the `actor` that issues them: the
  store refuses the order unless the guild's authority map names that
  principal and the class is one that may destroy (a guild owner,
  administrator, manager, or an organization owner — never a service, never a
  human subject), and the sidecar lines record the principal, so a
  destruction order is attributable. Each sidecar is a header line plus one
  line per request, with a checksum over the request lines, so a truncated
  or mis-checksummed sidecar is refused as `Corrupt` rather than believed.
  `EpisodeStore::gc_plan()` is the non-mutating collection
  plan and `EpisodeStore::apply_gc_plan()` performs the rewrite: a
  payload-bearing record that is forgotten, past its deadline, and under no
  open review is reclaimed, while every tombstone is retained forever. The v3
  ledger is not one global hash chain — `previous_digest` chains a single
  operation and a memory candidate is a single-event operation — so
  reclaiming a record breaks no chain, and the canonical digest never covers
  the ledger `sequence`, so the rewrite re-positions the surviving records and
  leaves every surviving digest unchanged. The reclaimed index
  (`episodes.v1.reclaimed.jsonl`) remembers, content-free, which candidates
  were reclaimed, so the ledger's tombstones still replay after their record
  is gone; it is written before the rewrite it describes. The rewrite is
  crash-safe: the new ledger is written beside the old one and renamed over
  it, so an interrupted collection leaves the original ledger intact and
  openable. `tests/v3_deletion_lifecycle.rs` pins it. Not implemented: the
  consumer's own destruction of derived copies and object keys, and a
  candidate-selected retention window.
- Holds and candidate revocation live in `src/v3/episode/store/holds/`
  (`holds/mod.rs` for the API, `holds/index.rs` for the two derived indexes).
  A *hold* is a placement that freezes a memory candidate: while it is live the
  candidate is not expired away (`store/expiry.rs`, `due_expirations`), not
  reclaimed (`store/gc.rs`, `reclaimable_at`), not silently superseded
  (`store.rs`, `prepare_record`), and its payload and derived copies are not
  destroyed (`store/deletion.rs`, the redaction and object-key requests). The
  `HoldKind` vocabulary is closed (`legal`, `investigation`, `user_request`);
  `EpisodeStore::decide_hold()`, `EpisodeStore::commit_hold()`,
  `EpisodeStore::release_hold()` and `EpisodeStore::hold_state()` report it,
  and only the placing authority or one above it in the documented governance
  order (guild manager < guild administrator < guild owner < organization
  owner) may release a hold. The class rule is only the first of two
  necessary conditions: the guild-scoped authority map below decides whether
  the principal may act for the target guild at all. A `forgets` tombstone is *not* refused by a
  hold: the tombstone is the explicit erasure request, while the collector's
  reclamation and the deletion lifecycle's requests are the destruction the
  hold freezes, and both are defined on a forgotten target.
  A *revocation* withdraws a candidate by authority: `GuildMemories::is_live`
  reports a revoked candidate as not live, so no later `supersedes` may name
  it, it is never resurrected, and the store refuses to forget it. Its record
  and its anti-resurrection metadata are retained, because a revocation is a
  withdrawal from use and not an erasure.
  `EpisodeStore::decide_revocation()`, `EpisodeStore::commit_revocation()` and
  `EpisodeStore::revocation_state()` report it; only a guild owner,
  administrator, manager, or organization owner may revoke, and a service may
  not; the guild-scoped authority map below decides whether the principal may
  act for the target guild at all. Both are *derived sidecars* beside the ledger —
  `episodes.v1.holds.jsonl` and `episodes.v1.revocations.jsonl` — in the same
  shape as the retention, redaction, object-key, and reclaimed indexes:
  content-free lines, a checksum over each line's own fields, created with the
  first decision rather than with the store, and written by the same single
  writer. They are sidecars and not new `EpisodeEvent` variants on purpose: a
  field on `MemoryCandidate` would change every candidate digest and break the
  pinned goldens and the Python witness, and a new event variant would change
  the canonical encoder, the Python witness, and every pinned golden with it.
  The revocations are applied to the replayed state *after* the ledger replay,
  so a ledger written before the revocation still replays.
  `tests/v3_holds_revocation.rs` pins it. The constitutional query now reports
  both states: the lifecycle dimension carries `HoldEvidence` and
  `RevocationEvidence`, `Disposition` has `Held` and `Revoked` members, and
  `tests/v3_constitutional_query_holds.rs` pins it. Not implemented: the
  consumer's own destruction of a held candidate's copies.
- The constitutional query read model lives in `src/v3/episode/store/query/`
  (`mod.rs`, `dimensions.rs`, `verdict.rs`, `result.rs`, `constraints.rs`,
  `project.rs`):
  `EpisodeStore::query` returns the eight evidence dimensions (E01-E08) as
  separate fields with a closed `Disposition`, a per-dimension content-free
  `Reason` and an `Explanation`, and carries no combined score. It is
  guild-scoped, member-scope partitioned, bounded by `MAX_RECEIPTS`, and it
  omits nothing: a record whose cryptographic evidence fails is listed as
  `rejected` rather than dropped, and a forgotten record is listed as
  `rejected` when the caller asks for erased memories (by default it is
  excluded, because the tombstone means the content is gone); dimensions the
  ledger does not hold read `Unknown`, never a fabricated zero. The lifecycle
  dimension also reports the hold state (`HoldEvidence`) and the revocation
  state (`RevocationEvidence`), `Disposition` has `Held` and `Revoked` members
  for them, `Reason` has `Held`, `Revoked` and `HeldAndRevoked`, and
  `QueryConstraints::include_held`/`include_revoked` both default to `true`.
  `tests/v3_constitutional_query.rs` and `tests/v3_constitutional_query_holds.rs`
  pin it. Not implemented: production key trust, cross-language verification,
  an ABI-facing wire contract.
- The dimension-separated ranking lives in `src/v3/ranking/`: it consumes the
  constitutional query's answer and keeps the eight evidence dimensions
  *separately inspectable* instead of collapsing them into one number, which
  is the opaque collapse `temporal.rs`'s `ScoreComponents::combined()` is.
  `DimensionSignal` is `Present(&'static str) | Absent | Unknown |
  NotApplicable`, so a missing dimension (`Unknown`, the ledger holds no such
  evidence) never reads as a measured zero (`Absent`) and a dimension that
  does not apply (`NotApplicable`) is its own state; the per-dimension order
  is a documented sequence of labelled states, never a number.
  `DimensionRanking::order_by_dimension` orders by one chosen dimension and
  keeps the ranking's existing order among ties; `explicit_policy_order` is
  the opt-in composite, an arbitrary policy named as one, never a trust
  measure, and never the default. `merge_vector_arm` merges a plain vector
  arm's own order into the ranking with a stable merge that invents no
  scalar: the vector arm's own position and similarity travel beside the
  dimension signals, and a vector hit the constitutional query did not answer
  is counted, never invented into the ranking. `rank_constitutional_query` is
  the end-to-end path (store -> constitutional query -> ranking).
  `tests/v3_dimension_ranking.rs` pins it, including the structural claim
  that the public ranking surface exposes no number but the vector arm's own
  similarity. The signal is a lossy projection of the canonical
  evidence and says what it drops: the source signal drops the recording role
  and the reputation, the lifecycle signal drops the retention class, the age
  and the kinds of a live hold, the confidence signal drops the recorded
  uncertainty. The lifecycle signal names the most severe lifecycle fact the
  canonical query reports, in the canonical query's own precedence (forgotten,
  revoked, quarantined, held, superseded, tombstone, live), so sorting by the
  lifecycle dimension never contradicts the disposition the query reported; a
  record that is both held and revoked reads `revoked`. The canonical
  evidence stays on every result.
- The vector-index adapter closes two of that ranking's documented gaps, in
  `src/v3/ranking/vector.rs` (with `vector/format.rs` for the file format and
  `vector/lock.rs` for the writer exclusion) and `src/v3/ranking/adapter.rs`.
  `VectorBindingIndex` is the substrate-side binding between a vector index's
  identifiers and the episode digests the constitutional query answers: a
  derived sidecar beside the ledger (`episodes.v1.vectors.jsonl`). Its RECORD
  shape matches the episode store's own sidecars — one line per binding
  carrying the scope, the vector identifier and the episode digest and
  nothing else (no payload, no actor, no event), a checksum over the line's
  own fields, and a whole-file atomic replacement on every write. Its
  WRITER-EXCLUSION discipline now matches them too: the index takes an owner
  file (`episodes.v1.vectors.lock`) and a lock held for the index's lifetime,
  so a second writer is refused as `VectorBindingError::WriterBusy` instead of
  racing, and every write re-reads and merges the file on disk, so a binding
  another writer added is never lost. The file carries a HEADER — a first
  line recording the format version, the number of binding lines and a
  checksum over the record set — so a truncation is detected rather than
  silently believed: the index is replaced whole, so a trailing fragment is
  damage and not a crash tail, and a truncation at a line boundary is caught
  by the header's count and checksum; either is refused as
  `VectorBindingError::Corrupt`, never a shorter index. It is
  *scope-partitioned*: the scope is part of the line and of the lookup key,
  so a lookup for one guild never answers with another guild's digest. A
  binding is refused a digest the scope's constitutional query did not admit
  (`AdmittedScope`, SEALED: the one public way to have one is the store's own
  `EpisodeStore::admitted_scope`, which runs the constitutional query and
  mints the set from that answer — the scope is read from the answer, not
  taken as a separate argument, and a hand-built answer cannot become a set).
  That refusal is a check against the CALLER's admission set: the substrate
  trusts the caller to hand over the query's own answer and does not
  independently prove admission. The binding
  survives a close and a reopen.
  `VectorSource` is the adapter trait: `HnswVectorSource` owns an
  `HnswIndex` whose level sampling is driven by an explicit, pinned seed it
  exposes, and `ExactVectorSource` owns an `ExactIndex` and is the
  deterministic correctness oracle; `is_approximate` says which one a caller
  is holding. Both resolve a slot to an episode digest through the binding
  index, so the ranking's merged arm carries the digest and not only an
  identifier; a slot no binding resolves is counted in
  `VectorArm::hits_without_binding` and in
  `MergedRanking::vector_hits_without_binding`, never invented into the
  ranking. `rank_with_vector_source` is the end-to-end path (store ->
  constitutional query -> vector source -> merged ranking).
  `tests/v3_vector_adapter.rs` pins it: the round trip, the scope isolation,
  the admission refusal, the close/reopen and the truncated and corrupted
  lines, the second writer refused, two snapshots of one file keeping each
  other's bindings, the loader refusing a file the writer could never
  produce, the oracle agreement and the pinned-seed determinism, the digest
  in the merged arm, the unbound hit, and the whole path against a real
  `EpisodeStore`. Not implemented: latency or memory measurement, a
  cross-language witness for the binding sidecar, an append-only binding log
  (every write replaces the whole file), a writer-exclusion fence against an
  actor who rewrites the file wholesale (the lock is advisory, as everywhere
  else in the repository), a vector arm over several scopes in one index,
  cross-language verification, an ABI-facing wire contract. The adapter says
  what it drops: an HNSW search is approximate and may miss a nearest
  neighbour (the exact source is the oracle), the binding index is separate
  from the ledger, a vector's payload is never held, an unbound vector's
  episode is never named, and an asset digest is never held at all — the
  vector arm binds episode digests and nothing else.
- The quorum-fenced replicated shard lives in `src/v3/replication/`: three
  voters, each with its own directory, its own writer lock, and its own
  episode store, under one canonical writer per tenant/scope. A write is
  canonical only when a quorum of two voters has durably accepted it under
  the shard's current term. `QuorumShard::propose` is the only public path to
  a canonical `ShardReceipt`: the certification step (`certify`) and the
  explicit-term variant (`propose_with_term`) are crate-internal, so a caller
  outside the crate cannot certify an acknowledgement set of its own choosing.
  A receipt is minted only from a quorum of the shard's own members'
  acknowledgements, all carrying the shard's current term and the same digest,
  and only for a proposal the shard itself made. The receipt has no public
  constructor and no `Deserialize`. A
  minority partition gets a `MinorityShard`, whose `propose` returns a
  `MinorityOutcome` and can never return a receipt. The shard's term, its
  membership, and the digests it acknowledged live in one content-free,
  checksummed, atomically replaced sidecar (`shard.v1.json`) beside the voters;
  each voter keeps its own durable term. A restart recovers the term, the
  membership, and the acknowledged state; a member that lost an acknowledged
  write is detected and the shard then refuses to acknowledge new writes, and
  it stops reporting that write as acknowledged. One *unreachable* member is
  not the same: the surviving quorum still holds every acknowledged write, so
  the shard keeps serving. `tests/v3_quorum_shard.rs` pins it. Not implemented:
  Byzantine fault tolerance, leader election, anti-entropy or catch-up
  replication, a network transport, cross-scope federation, and a
  production key custody story.
- The frozen retrieval baseline harness lives in `src/evaluation/` with
  fixtures in `tests/fixtures/retrieval-baseline/`: it compares a plain-vector
  arm against the constitutional query over one frozen synthetic corpus with
  frozen embeddings and a fixed candidate count, reports
  `repeated_failure_rate` and `stale_reuse_rate` for both arms, ablates each
  of the eight dimensions, and can report "no benefit" (suppressing every
  dimension reproduces the baseline exactly, which is asserted). The receipt
  is `docs/superpowers/plans/2026-10-21-retrieval-baseline-receipt.md`. It is
  a deterministic local comparison over synthetic data (C1), not a research
  result and not evidence about production traffic. Not implemented: the
  preregistered held-out protocol, an independent runner, latency or memory
  measurement, cross-language verification.
- Independent-organization federation lives in `src/v3/federation/`: a
  `PeerDirectory` of independent organizations whose peer identities are
  *derived* from their keys, explicit bilateral trust records that carry a
  scoped `ShareGrant` (guild scopes, evidence classes, direction, and a
  closed residency class) and are revocable and never transitive, a
  deterministic `ExportBundle` (origin organization, exporting peer, covered
  scope, monotonic checkpoint, deletion watermark, record digests, and the
  exporter's signature), and a `FederationImporter` that verifies the peer,
  the signature, the trust, the grant, every record digest, the ancestry, the
  checkpoint, and the deletion watermark *before* anything is admitted.
  Imported records keep the *origin's* authority and live in the importing
  peer's own foreign namespace, never in its canonical episode ledger; a
  local reference to one is an import receipt. The importer detects
  equivocation (two different bundles at one checkpoint), refuses replays and
  regressions, deduplicates by origin and digest, resumes an interrupted
  import from its last checkpoint, and applies the origin's deletion
  watermark so a deleted record is no longer served. `tests/v3_federation.rs`
  pins it. Not implemented: Byzantine fault tolerance beyond the detected
  equivocation, a transport or a handshake, key custody or a key directory, a
  relay, payload federation (the ledger commits to a payload, it does not
  store one), and a second-language witness for the bundle encoding.
  Deletion propagation is pull-based: a peer that never receives the
  deletion keeps serving the deleted record.
- The offline outbox lives in `src/v3/offline/`. `OfflineOutbox` is a
  durable, single-writer queue of PROPOSALS made while a scope was offline:
  an operation id (its idempotency key), an optional dependency, a kind tag
  (`Write`, `Delete`, `Asset`, `Edit`), a binding (scope, contract revision,
  contract digest, policy version, consent epoch), a proposal, and a status
  from a closed vocabulary (`Pending`, `InFlight`, `Acknowledged`,
  `Rejected`, `Conflicted`). It is NOT a canonical episode writer and must
  never become one: it stores the write the canonical store would compute its
  own commitment from, it refuses a write that carries a predicted
  commitment, and it holds no canonical episode digest. The reauthorization
  is a separate, explicit step: `drain` takes the CURRENT policy and the
  canonical store and answers a per-operation `ReauthorizeOutcome`
  (`Authorized`, `Refused(reason)`, `NeedsConsent`, `SchemaSkew`), running
  the store's own non-mutating decision rather than re-implementing the
  write gate. No operation is dropped by the outbox's own logic: a refused
  or conflicted operation keeps its proposal, a terminal status, and a
  reason, and only an explicitly authorized caller (guild manager,
  administrator, owner, or the organization owner) may requeue, discard, or
  remove it. The removal is status-independent (`Resolution::Remove`), so an
  acknowledged operation is not immortal and a long-lived client can make
  room; a full outbox refuses new work with `OutboxError::Capacity`, which is
  distinct from an invalid input. A removal that would orphan a dependent is
  REFUSED with `OutboxError::DependencyPresent` — the outbox never mutates
  another operation's state as a side effect of an unrelated act, so the
  caller resolves the dependents first (leaves first) and no operation can be
  stranded unsendable. The merge is deterministic
  and explicit over a small closed set of mergeable configuration objects
  (specimen, learning, network, authority): disjoint field edits merge,
  the same field conflicts, incompatible versions conflict whole, and the
  outbox never invents a winner. An asset operation carries a digest, a
  length, and a chunk size but never the bytes; the chunk payload is
  supplied by the caller and the final digest is verified from the recorded
  chunk digests, so a resumed transfer does not re-request a completed chunk.
  The asset digest is numbered by chunk index, so the order a caller hands
  its chunks over in never changes it and a correctly transferred asset is
  never reported as corrupted.
  There is NO fixed offline expiry: nothing in the module reads a clock for
  behaviour, and an operation survives an arbitrary elapsed time. The file is
  `outbox.v1.jsonl` beside `outbox.v1.lock`: a header line naming the
  operation count and a checksum over the record set, then one checksummed
  line per operation, and every write replaces the file atomically. A
  truncated file, a trailing fragment, and a bad checksum are all refused as
  corrupt: the outbox never opens with fewer operations than were queued, and
  nothing is discarded by its own logic. `tests/v3_offline_outbox.rs`
  pins it. Not implemented: a transport or a server (the caller supplies an
  `OperationSink`), a delivery acknowledgement from a real server, a general
  CRDT, model-weight distribution, a resnapshot format, and any expiry or
  reaping.
- The complete-episode prototype's ledger lives in
  `src/v3/complete_episode/ledger.rs`, with `ledger/format.rs` for the
  on-disk record and `ledger/filesystem.rs` for the writer lock. It is a
  PRIVATE PROVISIONAL prototype until named-owner agreement: crate-internal,
  unratified, and reachable only through the hidden
  `v3::complete_episode_prototype` facade that exists for this crate's own
  integration tests. `CompleteEpisodeStore` is a single-writer, append-only
  store of complete episodes under `COMPLETE_EPISODE_DOMAIN`
  (`wdbx-complete-episode-v1`) in its own file family
  (`complete-episodes.v1.jsonl` and `complete-episodes.v1.lock`); the writer
  lock is held for the store's lifetime and persistent contention is
  `WriterBusy`, never permission to open without it. One record is one
  complete episode: the canonical complete-episode bytes, the SHA-256 digest
  over exactly those bytes, and the detached Ed25519 signature beside the
  record, so signing never moves a digest. The chain is across the ledger (a
  complete episode is a single-record operation, unlike the v3 ledger's
  per-operation chain): record N names record N-1's digest, and a gap, a
  fork, or a duplicate digest is `Corrupt`. Replay re-derives every digest,
  re-decodes and re-validates every record through the complete episode's own
  validation, verifies every record that claims this store's own key, and
  refuses anything else as `Corrupt`; only an incomplete final line is
  discarded, and only after the verified prefix replays.
  `CompleteEpisodeReceipt` is content-free. This is a DIFFERENT store from
  the v3 episode ledger: different domain, different digests, different
  files; it never writes a v3 candidate and is not a second canonical episode
  for the same content. The boundary is precise: the prototype's own TYPES
  are crate-private, and the four handles the facade re-exports
  (`CompleteEpisodeReceipt`, `CompleteEpisodeStoreError`, `PrototypeEpisode`,
  `PrototypeLedger`) are `pub` but `#[doc(hidden)]`, unratified, and not to
  be depended on — `#[doc(hidden)]` is documentation, not access control.
  The honest limit of the detached signature: STRIPPING a signature from a
  signed record is not detected. An unsigned record in a signed ledger
  replays cleanly and is reported as unsigned (`SignatureStatus::Unsigned`),
  which is a DOWNGRADE of the evidence of who wrote the record, not a forgery
  of its content; that is inherited from the v3 store's detached-signature
  design, where the signature sits outside the digest. Not implemented: ABI
  ratification, a wire contract, quorum or fencing (the v3 quorum shard does
  not cover it), federation,
  cross-language verification of the ledger format, a deletion or retention
  lifecycle, a reader projection, and any consumer. Without a signature the
  ledger detects accidental mutation, not an actor who rewrites the whole
  file.
- The complete-episode DECODER is a separate Rust↔Python pair, independent of
  the ledger above. `src/v3/commitment/decode.rs` is the generic bounded
  reader over the `abbey-cbor-episode-v1` profile
  (`CanonicalDecodeError`), and `src/v3/complete_episode/decode/` is the typed
  inverse of the provisional complete envelope built on it
  (`containers.rs`, `groups.rs`, `references.rs`, `vocabulary.rs`), wrapping it
  in `CompleteEpisodeDecodeError`. Both are crate-private and neither confers
  admission authority: they validate bounded syntax and typed values and
  nothing else. The Python side is split the same way —
  `tools/wdbx_complete_episode_decode.py` (entry point),
  `..._decode_ingress.py`, `..._decode_values.py`, `..._decode_validate.py`
  and `tools/wdbx_complete_episode_draft.py`, over `tools/abbey_cbor_reader_v1.py`
  and `tools/abbey_cbor_episode_v1.py`.
  The two sides share ONE error vocabulary, which is the load-bearing part:
  the Python `Canonical:` and `InvalidValue:` name sets ARE the Rust
  `CanonicalDecodeError` and `CompleteEpisodeError` variants, and
  `tests/v3_cross_language_decode.rs` requires the same malformed input to
  be refused with the same error name on both sides. Adding a Rust error
  variant without its Python counterpart (or the reverse) is a
  cross-language test failure, not a local one, and the error NAMES are
  part of the witnessed contract — renaming one breaks it. The witness
  proves decode agreement only: it verifies no signature, admits no episode,
  and is not a second-language admission rule.
- `abi-wdbx-gateway` remains in ABI. Its `ProposeEpisodeWrite` / `VerifyEpisode`
  RPCs use this episode store only with configured episode policy. Do not rename
  crates into CSAPS service names without a spec or equate that gate with full
  constitutional MemoryService conformance or production federation.

Paths beginning `src/` above are relative to `crates/abi-wdbx/`.

## Contract Corpus

- `contracts/abbey/abbey-contracts.lock.json` pins the exact ABI corpus revision
  and digest; `crates/abi-wdbx/tests/abbey_contracts.rs` pins qualification too.
  Refresh only through ABI's `tools/vendor_abbey_contracts.py`, never manual
  copying or independently regenerated fixtures.
- Native qualification verifies all artifact bytes/digests but validates only
  `episode/` schemas/fixtures. It does not open `EpisodeStore` or authorize writes.
  Positive canonical-CBOR vectors and episode-store replay tests are separate
  crate-local evidence, not vendored corpus fixtures.
- Preserve Current/Partial/Proposed and C0-C7 evidence boundaries. Source or local
  replay proof is not empirical integrated-system or production multi-host proof.
  See `README.md` for the implementation gaps.

## Claims and Evidence

- `docs/claims.md` is the claim ledger, and it is where the C0-C7 evidence
  levels are defined and tabulated against this codebase. The levels are the
  constitution's, not this repository's; the Current/Partial/Proposed
  vocabulary is the same one `README.md` uses in its "Status: honest" table.
- `docs/claims/<area>/` holds one area's qualification receipts — for example
  `docs/claims/wdbx-3d/source-manifest.json`, a pinned inspection inventory,
  and `docs/claims/wdbx-3d/stable-qualification.json`.
- `docs/contracts/README.md` is the per-capability status table; it mirrors the
  README's table and is the short form of `docs/claims.md`.
- `docs/execution/` holds bounded implementation packets with their evidence
  and handoff, `docs/verification/` holds local measurement records, and
  `docs/reviews/` holds independent reviews. A receipt in any of them is
  evidence about the packet it names, not about the tree.
- The design rationale and the per-slice receipts live under
  `docs/superpowers/specs/` (designs) and `docs/superpowers/plans/` (plans and
  receipts), and this file cites the relevant one by path in most sections.
  A slice's receipt is where its "not implemented" list and its evidence
  boundary are recorded, so read the receipt before repeating a slice's
  claims.
- A claim level moves only by a reviewed schema/version change with a
  recorded evidence verifier, never by editing a status cell, a README table
  row, or a requirements `status` field. `docs/completion/requirements.json`
  keeps every requirement at `proposed`/`C0` with an empty `evidence` array,
  and `tools/check_completion.py` FAILS on any other value, so a promotion is
  a checker error to be reviewed, not a silent edit.

## Completion Contracts

- `docs/completion/` is the C0 inventory and preregistration for the
  completion program — a proposal, not implemented research, product
  conformance, or a release claim. `requirements.json` holds 163 requirement
  IDs bound to an owner, a task, a source line and text, an expected-evidence
  list and a completion condition; `studio-acceptance-v1.json` holds the 46
  acceptance rows and 18 decisions verbatim and in source order;
  `plans.json`, `designs.json` and `baseline.json` hold the work packages,
  the design bundles and the repository baseline.
- `sources.json` and `sources/` hold hash-pinned REFERENCE SNAPSHOTS of the
  program, the Studio specification, the Abbey constitution and the
  canonical-episodes spec. The upstream repository stays normative and the
  snapshot is a local copy: never edit a snapshot to make a check pass, and
  never treat it as a second normative corpus.
- `tools/check_completion.py` validates the inventory against those pins and
  against the source bytes; `tools/tests/test_check_completion.py` is its own
  suite. It is standard-library only, supports Python 3.9, runs from any
  directory, and works WITHOUT the sibling `../abi` checkout — when live
  authoritative sources ARE present their hashes must also agree, so a
  sibling checkout can only make it stricter, never looser.
- The checker enumerates IDs from the independently pinned source bytes, so
  removing a case from both the inventory and the manifest still fails, and
  it checks that each requirement is routed exactly once with its mandatory
  prerequisite edges. A PIN change is a reviewed contract-version change,
  never an automatic repair.
- `experiments.json` and `synthetic-corpus-v2.jsonl` are the preregistered
  protocol and the frozen corpus. NO experiment has run. Do not report a
  result from them: requirement `P04.03` stays `proposed`, and the inventory's
  `evidence` arrays stay empty. Promotion needs a separately reviewed
  schema/version migration and an evidence verifier, and neither exists yet.

<!-- machine-git-policy -->
## Git workflow (machine policy, 2026-08-27)

Work on the default branch in this canonical checkout. Do not create
branches or worktrees by default; they are for tasks that genuinely need
isolation, or when Donald asks. Any worktree or topic branch created here
must be merged back into this checkout's default branch, the worktree
removed, and the branch deleted, before pushing and before the task is
called done. Full policy: `~/.claude/CLAUDE.md` (*Git discipline*).
<!-- /machine-git-policy -->

## Local main readiness (2026-10-25)

Follow [docs/superpowers/plans/2026-10-25-local-main-readiness.md](docs/superpowers/plans/2026-10-25-local-main-readiness.md) for this user-approved local integration task.
Use one manual task branch/worktree per independent task; preserve existing
dirty, untracked, ignored and unpushed work and app-managed worktree lifecycles.
Merge owned changes into local `main` only after writer release, recovery custody
and the documented repository gates pass on the candidate revision. Keep failed
or skipped gates explicit. This task's hold takes precedence over older guidance
requiring immediate merge or worktree deletion. Leave owner-held work in place;
remote publication and deployment require separate scope.
