# N02: bounded native packages and deterministic byte BPE

Implement the already approved [binding design](../contracts/model/native-package-design-v1.md), sections 1–12 and 14. Requirement mapping and downstream boundaries remain in [the native sequence](2026-10-25-native-model-cpu.md). N01 reviewed base: `cb129cf`.

Own native `model/package/`, `model/tokenizer/`, narrow model type/limit/error edits, dedicated package/tokenizer/bounds tests and additive independent Python witness tests. Preserve all 53 N01 packages, legacy readers, lockfiles and other workers' files.

## Task 1: implement the approved N02 packet

Read the complete binding design sections 1–12, 14–16 before implementation. Exact owned files and discriminating matrix are section 14; no local redefinition.

1. Regression-first owned snapshot, framing and identities; bounded schema decoding and cross-links; resume context.
2. Real canonical writer, byte-identical re-export and no-clobber explicit-directory staging, with cancellation and durable-platform limits.
3. Exact byte BPE encode/decode and deterministic bounded trainer, checked against independent Python vectors.
4. Focused Rust/Python checks, pinned root gate and completion checker; fresh independent specification/code review before dependent N03 work.

Use isolated `.work/` target/stores, Rust 1.99.1, stdin EOF and jobs=1. Record failed and successful commands privately, no performance qualification under concurrent load. No inference, training-network, ABI activation, publication head, credentials, SDK or live-store access.

Rollback disables only new import/export entrypoints; retains task-owned incomplete objects for explicit retention and leaves selected generations unchanged. No package reference confers permission.
