# AGENTS.md — WDBX substrate

Canonical instructions for this repository. `CLAUDE.md` is a pointer to this file.

## What this repository is

The provenance-aware episodic substrate beneath ABI, plus the foundation layer it
builds on. Extracted from `donaldfilimon/abi` on 2026-08-22 with history
preserved (39 commits, every commit that touched these crates).

Authority, per the Abbey System Constitution
(`abi/docs/superpowers/specs/2026-08-22-abbey-system-constitution.md`): this
repository **owns durable episodic semantics** — what an episode is, what makes
it trusted, how it is superseded, contradicted, quarantined, or deleted. No
consumer may define a second answer. A consumer may hold a lossy *projection*,
which must declare what it drops.

## The remote does not exist yet

`Cargo.toml` sets `repository = "https://github.com/donaldfilimon/wdbx"`, and
`repository.workspace = true` propagates it into all five crates' metadata. **That
URL is an intent, not a fact** — this repository has no git remote configured and
the GitHub repo has not been created. Creating it is Donald's call, since it is an
outward-facing action. Until then, treat the field as a placeholder: do not cite it
as a source, and do not publish from this workspace.

## Layout

Five crates, in dependency order. Every crate depends only on crates above it.

```
abi-foundation   (no deps)
abi-telemetry    (no deps)
abi-compute      (no deps)
abi-core         -> abi-foundation, abi-telemetry
abi-wdbx         -> abi-foundation, abi-compute
```

`abi-core` is a leaf here: nothing in this repository depends on it. It travels
with the substrate because Donald chose the full dependency closure, and its
consumers (`abi-cli`, `abi-mcp`, `abi-plugins`) live in `abi`.

## Gate

```
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

Green at extraction: 558 tests, 0 failed, 0 clippy diagnostics. The workspace
denies `unsafe_code` and all of clippy.

## Rules that bite

- **Sibling layout is load-bearing.** `abi` and `abbey` consume these crates by
  relative path (`../wdbx/crates/...`). Mixing a git dependency in one consumer
  with a path dependency in another gives Cargo two distinct `abi-wdbx` crates
  whose types do not unify. Keep all three as siblings under `~/dev/active`.
- **Golden fixtures are crate-local now.** `crates/abi-wdbx/tests/golden/` holds
  the WDBX on-disk format samples that used to live at abi's repo root. They moved
  because they describe *this* format. The CLI and MCP output goldens
  (`help-wdbx.txt`, `wdbx-stats.txt`, `wdbx-db-verify.txt`) stayed in `abi`,
  because they describe abi's surfaces.
- **`abi-core`'s golden scheduler test did not come with it.** It asserted
  `abi-core`'s scheduler against fixtures captured from abi's CLI and MCP output,
  which makes it an integration test between substrate and runtime. It lives in
  `abi`, where both sides exist. Do not re-add it here by copying the fixtures;
  that would create two copies that drift.
- **Do not rename crates toward CSAPS service names.** `abi-wdbx-gateway` (which
  stayed in `abi`) resembles the specification's `MemoryService` but is not it:
  no `ProposeWrite` write gate, no `Verify`. Renaming is Program 6 work and needs
  a spec first.

## Honesty

Report **Current** versus **Proposed**. The specification this substrate targets
is a *proposed architecture* whose own status box says the integrated system is
not empirically validated. Most of the evidence half of that specification is
unimplemented here; the README says exactly which parts. Do not describe a
capability as Current above the evidence level (L0-L8) that supports it.
