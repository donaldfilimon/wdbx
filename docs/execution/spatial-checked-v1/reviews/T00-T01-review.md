# Independent T00/T01 review

Review date: 2026-10-20. Read-only review except this report. No Cargo/Python test rerun, CI, live-store access, commit or push. Existing raw logs were inspected rather than rerunning passing tests.

## Verdicts

- **T00 spec compliance: PASS within approved first-packet inventory scope. Quality: PASS.** The validator honestly admits inventory shape and declared identity only; its function documentation and CLI explicitly deny independent content/execution verification. The named fixtures, bounded typed ranges, required authorities, external-artifact provenance, reported/executed separation and source/log digest declarations satisfy that scope. The v5 first-packet and lockfile dependency-inventory exceptions are explicit rather than invented full reads.
- **T01 spec compliance: PASS for baseline default workspace only. Quality: PASS with one low export-hygiene note below.** Rust 1.99.1, separate compile-only evidence, default feature tuple, consumer omissions, permitted warning policy, source-size gate and earlier pause/current authorization are correctly distinguished. No eventual implementation, optional FHE, device, SIMD ISA, speedup or consumer qualification follows.

## Findings

**Low — operator-specific path in public qualification artifact.** `docs/claims/wdbx-3d/stable-qualification.json:24` includes an absolute user-home toolchain prefix. V5 §2.3 (line 124) says public source manifests omit operator paths, and the source audit's public-output statement generalizes that hygiene. The actual source manifest is clean; this adjacent public qualification artifact should express the prefix symbolically (for example, `$RUSTUP_HOME/toolchains/<release-target>/bin`) while retaining the exact local command environment in a nonexported receipt if needed. This does not invalidate the observed compiler or gate result and contains no credential or private runtime record.

No blocking correctness or evidence-inflation finding was identified in this packet scope.

## Independent checks and evidence

- Inspected briefs T00/T01, their reports, validator-fix report, complete validator and 16-method test module, source audit, source manifest and stable qualification. Inspected supplied v5 §2.3/shared numeric/interface contracts/T00/T01 and v6 §§1,12–14,49. Authority scope comes from those sources plus the explicit first-packet review instructions; old execution-root planning documents were not treated as canonical audit.
- Independently recomputed SHA-256 for **every Git-backed manifest row** from its declared immutable commit/path: all matched. Recomputed all four external-artifact rows from supplied package bytes: all matched. These checks verify identity, not that another worker read the declared passages. Collective full controlling-source inspection remains attributed to the recorded workers; this reviewer does not claim to have independently repeated it.
- `evidence/t00-fix-redgreen.txt` preserves admission failures, subsequent 16-test success, and external-artifact compatibility success. The validator permits failed execution records as inventory; it does not turn them into a qualification pass or authenticate command semantics. No unsupported cryptographic verification claim was found.
- `evidence/t01-identity.txt` records rustc/cargo 1.99.1 and the CPU diff. Independently compared HEAD and working `crates/abi-compute/src/cpu.rs` after removing whitespace: identical. The sole ordered-fold correction changes line wrapping only; arithmetic/reduction order and signed-zero seed are unchanged.
- Independently parsed `evidence/t01-workspace-gate.txt`: **24 summaries; 647 passed, 0 failed, 0 ignored**, with instruction, source-size, formatting and clippy stages and `T01_GATE_EXIT=0`. The baseline qualification explicitly excludes later source changes. Its original format failure remains in `evidence/workspace-gate.txt`.
- `tools/check.sh` retains the exact repository lint/size contract and does not override toolchain selection. The absence of `--locked` in that canonical script is disclosed, with unchanged lockfile digest; separate check used `--locked`. Optional FHE tuples and all consumer runtime checks remain unverified.
- T00/T01 do not edit frozen v1/v2/v3 formats or signed commitment code. This review confirms the CPU formatting claim; actual compatibility execution belongs to T02 and the final full gate.

## Claims to retain at final acceptance

Require a fresh gate tied to final dirty implementation inputs; keep baseline and final evidence separate. Explicitly remove `ABI_WDBX_REAL_STORE_TEST` and `ABI_ZIG_BIN` before that gate, as T01 already requires: post-run absence alone is not pre-run exclusion evidence. Preserve missing-original-artifact/private-receipt limitations, partial lockfile read scope, untested consumers/features and content-versus-inventory distinction. Do not promote historical ledger or bare harness `ok` to live-store/device evidence.
