# WDBX historical source custody

On 2026-10-25 this repository received the unchanged bytes of
[WDBX-3D-Neural-Specimen-Master-Plan-v5.md](wdbx-master-v5.md), identified as
`AUDITED-SOURCE-02`. Its SHA-256 is
`f98d61f6f80503a2d00687c8cb6f38ed654cf9ad46cf55407ba5e06765354822` and its length
is exactly **344347 bytes**. Wording, line endings and original whitespace are
preserved. The source is a proposed revision-5 plan prepared on 2026-10-20;
its embedded original-artifact identity refers to its earlier revision, not
to the digest of this recovered revision-5 file.

[custody-v1.json](custody-v1.json) records the read-only recovery origin:
the `2026-10-20-181425` Time Machine snapshot, LYLX's
`docs/requirements/historical-sources/manifest.json` entry `AUDITED-SOURCE-02`,
its content-addressed object and the relative recovery receipt names. These
are provenance references within the independently retained LYLX repository.
They introduce no runtime or checker dependency on another checkout.

Run from this repository:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/check_source_custody.py
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/tests -p test_check_source_custody.py
```

The checker also accepts `--repo-root ROOT` from any working directory. The
default root follows the checker file. It reads bounded regular files, rejects
symlink components (including repository ancestors), checks the closed manifest
shape and independently fixed identity/hash/length, writes nothing, and emits
bounded content-free failure diagnostics, including argument errors. It has no
refresh mode. Its path safeguards use POSIX directory descriptors and
`O_NOFOLLOW`; this is local custody tooling, not cross-platform runtime evidence.

The [2026-10-23 substrate inventory](../completion/substrate-release/README.md),
its [catalog missing-source entry](../completion/substrate-release/catalog-v1.json)
`v5-companion-original`, and its
[baseline receipt](../completion/substrate-release/baseline-v1.json) remain
frozen history. Their earlier `unavailable-original` observation was true for
that dated packet and is preserved. This additive receipt establishes current
byte custody; it neither rewrites that observation nor closes its requirement
review prerequisite.

Remaining work is explicit: reconcile all v5 requirements, task/profile
boundaries, specimen chapter/invariant obligations and compatibility restrictions
against the [frozen v6 source](../completion/substrate-release/sources/specification-v6.md)
and existing substrate inventory, with source-linked decisions for overlaps,
conflicts and omissions. Any resulting inventory/schema/pin migration requires
separate review. The original v5 constraints preserve crate/API identities,
v1/v2/v3 formats and signed bytes, HNSW/octree/causal behavior, and distinct
specimen and optional accelerator evidence. Recovery grants no implementation
or erasure authority.

The new online-profile direction needs its own separately reviewed amendment,
requirements and acceptance evidence. It must remain separate from these
immutable historical bytes and must not imply historical adoption. This packet
adds no requirement numbers or promotion: implementation, trained-model quality,
device execution, performance, production readiness, LYLX replacement and release
qualification remain separate obligations.

Rollback consists of reverting the additive source-custody commit.
