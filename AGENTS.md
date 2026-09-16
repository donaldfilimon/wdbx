# AGENTS.md - WDBX Substrate

Canonical instructions; `CLAUDE.md` redirects here. Executable source wins over
prose. This is the Rust substrate, not the archived Workers MCP namesake.

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

## Gates

`rust-toolchain.toml` pins `nightly-2026-09-01`; edition 2024, Rust floor 1.99.
The exact CI gate (`.github/workflows/ci.yml`, hosted macOS) is:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

- There is no `check.sh` or `tools/cargo.sh` here. Select the pinned rustup
  toolchain, not a Homebrew Cargo/rustc pair; keep the toolchain bin directory
  and `/usr/bin` ahead of Homebrew Rust and Swiftly's compiler shims on macOS.
- Unlike ABI, this clippy gate does not pass `-D warnings`: workspace lints deny
  unsafe code and clippy `all`, but `missing_docs` and pedantic remain warnings.
- The default gate does not exercise `abi-wdbx`'s `full-fhe` or
  `experimental-dghv-bootstrap` features. Feature work needs explicit checks;
  do not report default green as optional FHE runtime evidence.
- Focused test: `cargo test -p abi-wdbx <filter>`; corpus target:
  `cargo test -p abi-wdbx --test abbey_contracts`.
- Docs-only changes: compare claims with source/tests and run `git diff --check`.
  There is no dedicated Markdown validator. Do not call this a full Rust gate.

## Persistence Boundaries

- `crates/abi-wdbx/tests/golden/` owns format fixtures. CLI/MCP output goldens
  and the scheduler integration oracle remain in ABI (`tests/golden/` and
  `crates/abi-cli/tests/golden_scheduler.rs`); do not duplicate them here.
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
- `src/v3/episode/` owns the single-writer episode ledger, canonical commitment
  calculation, policy/consent checks, replay rejection, and content-free receipts.
  Transport JSON and adapter-provided digests are not canonical authority.
- Episode signing lives in `src/v3/episode/signing.rs`, not in the commitment
  encoder: the store signs the 32-byte digest with Ed25519 when opened via
  `EpisodeStore::open_with_signer`. The signature is detached (stored beside the
  record, outside the envelope), so digests are unchanged and unsigned records
  serialize byte-for-byte as before. `signer_key_id` is derived from the key, not
  chosen. Keep the episode key distinct from the v2 segment-signing key. Only the
  canonical writer signs; adapters never do. A ledger holding signed records
  will not open on a build that predates this. Not implemented: COSE_Sign1,
  cross-language verification, key rotation/revocation, source-side signatures.
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

<!-- machine-git-policy -->
## Git Workflow

Use this canonical checkout's default branch. Branch/worktree isolation requires
a concrete need or explicit request; merge back and remove it before completion.
Full policy: `~/.claude/CLAUDE.md`. Never discard unrelated dirty work.
<!-- /machine-git-policy -->
