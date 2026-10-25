# WDBX Documentation

Start here to navigate WDBX's storage, evidence, and neural/spatial design work.
The [repository README](../README.md) and [agent instructions](../AGENTS.md)
explain the current Rust substrate. This index does not replace their detailed
implementation and security boundaries.

## Choose a starting point

| Goal | Read |
| --- | --- |
| Understand the current implementation | [Architecture](architecture.md) and [repository overview](../README.md). |
| Check what a capability claim means | [Claims ledger](claims.md) and [contract overview](contracts/README.md). |
| Build or change the substrate | [AGENTS.md](../AGENTS.md): toolchain, ownership, gates, fixtures, and data safety. |
| Explore Neural Specimen tooling | [Specimen Studio](../specimen-studio/README.md), with its own application and acceptance boundaries. |
| Find local design work | [Local plans/specifications index](superpowers/README.md). |
| Understand the trusted runner | [Self-hosted runner guidance](SelfHostedRunner.md). |

## WDBX in LYLX

In **LYLX by The Donald Company**, WDBX 3D is the neural memory/storage database:
durable records, semantic vectors, physical spatial data, graphs, episodes,
specimens, and provenance. Quesar is the shared model; Abbey/Aviva are its
personality direction; ABI governs dynamic moderation, routing, and authorized
execution.

That ground-up architecture does not erase the current substrate's formats,
crate identities, historical signatures, or compatibility responsibilities.
It also does not establish that the complete Neural Specimen design is already
implemented. Preserve original behavior and record intentional changes through
versioned designs and conformance evidence.

Three meanings of "space" must remain separate: physical 3D coordinates with
units and reference frames; model-specific semantic embedding spaces; and
visualization projections. A close point on a diagram is not proof of semantic
identity, factual truth, or causal influence.

## Canonical state, indexes, and clients

WDBX owns durable episodic semantics and their evidence. Search indexes,
visualizations, and client summaries are derived views, not competing canonical
stores. A consumer must disclose what its projection leaves out.

ABI/Abbey use sibling `../wdbx/crates/...` paths. WDBX itself has no dependency
back into those applications; do not introduce a storage/consumer dependency
cycle. Source visibility does not make user stores, episodes, credentials, or
runtime state public. Never use live user data as a test fixture.

Neural inference, memory admission, specimen growth, training-data eligibility,
and model-weight optimization are separate operations. A successful memory
write cannot be reported as trained Quesar weights, and a storage membership
record cannot grant authority to execute external tools.

## Shared specifications and historical requirements

The ABI repository hosts important shared authorities:

- [Canonical WDBX episodes](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-spec-canonical-wdbx-episodes.md): episode blocks, commitments, trust dimensions, write admission, and deletion semantics.
- [WDBX conformance gap analysis](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/specs/2026-08-22-wdbx-conformance-gap-analysis.md): the structural/evidence distinction and original implementation gaps. Treat this as a dated analysis, not a substitute for current source and receipts.
- [ABI/WDBX architecture](https://github.com/donaldfilimon/abi/blob/main/docs/spec/agent-wdbx-architecture.mdx): consumer interfaces and ownership boundaries.
- [WDBX north-star](https://github.com/donaldfilimon/abi/blob/main/docs/spec/wdbx-north-star.mdx): roadmap and capability-status distinctions.
- [Shared plans/specifications index](https://github.com/donaldfilimon/abi/blob/main/docs/superpowers/README.md).

A specification states required behavior; source and reproducible tests establish
what has been implemented. Neither a roadmap checkbox nor a rendered Studio
view promotes a claim. Qualify storage, specimen behavior, actual accelerator
execution, cross-host operation, and production deployment independently.

## Toolchain and validation

The repository pins **Rust 1.99.1, Edition 2024**. Unlike ABI, WDBX has **no
`tools/cargo.sh` wrapper**. Select the pinned Rust toolchain as directed in
[AGENTS.md](../AGENTS.md), avoiding a shadowing Homebrew compiler or linker shim.

From the repository root, the full substrate gate is:

```bash
bash tools/check.sh
```

The gate includes instruction/size checks, formatting, Clippy, selected Python
witness suites, and workspace Rust tests. Optional FHE features, the separate
completion-contract checker, 3D-evidence inventory, and Studio qualification
have their own gates; a default pass is not proof that those ran.

For **documentation-only** changes, follow the current `AGENTS.md` rule:
compare claims against source/tests and run `git diff --check`. There is no
dedicated Markdown validator. This replaces the older blanket instruction in
this index that every documentation edit required a full Rust build/test run.
Do not describe a documentation review as a completed substrate gate.

## Working with evidence

Keep historical signed bytes and conflict sets intact. Do not regenerate golden
vectors merely to match a changed implementation; the independent witness and
frozen contracts must agree under the repository's explicit regeneration rules.
Report the source revision, commands, platform, and actual outcome. Skipped
hardware tests, cross-compilation, and CI definitions are not runtime evidence.
