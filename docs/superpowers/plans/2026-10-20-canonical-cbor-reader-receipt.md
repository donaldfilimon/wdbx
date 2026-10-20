# Bounded canonical CBOR reader qualification receipt

2026-10-20; canonical checkout. The reader source is committed in `5182bc3a60300e1dd152611025e774c4b906638b`. The original local qualification ran against the uncommitted reader snapshot on base `429318a0b185d4d609dd3f8fcb7c4dcee8152afe`; its seven frozen source hashes below match committed source exactly. This receipt reconciliation does not rerun or reattribute that historical Rust gate.

Implemented the existing restricted `abbey-cbor-episode-v1` reader with explicit finite resource limits, exact canonical syntax/envelope validation, and an independent Python decoding witness. Existing encoders, goldens, v2 commitments, store/signing code and dependencies remain unchanged.

## Evidence

- Rust task: 16 new reader tests plus 40 existing focused compatibility tests pass.
- Python task: four new differential/protocol tests plus four existing witness tests pass; both standalone golden commands pass.
- Final root `bash tools/check.sh`: exit 0, **667 passed, 0 failed, 0 ignored**, 27 summaries. Rust1.99.1; offline Cargo; toolchain bin and `/usr/bin` first; two build jobs. Log `/tmp/wdbx-20261020-reader-final.log`.
- `git diff --check` passes. Independent task reviews and final combined review pass with no findings. Reports reside in `.superpowers/sdd/2026-10-20-canonical-cbor-reader/`; they preserve the historical working-snapshot review evidence.

## Limits and remaining work

This is source/local interoperability evidence. It does not establish complete constitutional episodes, admission semantics, COSE/signature interoperability, key administration, federation, research efficacy, hosted reliability, optional FHE or supported-platform qualification. Existing optional external Zig oracle branches are not exercised without their legacy binary. Studio qualification remains separate.

Resource caps bound declared CBOR resources, not total process heap. The Python JSON-lines command is a trusted local test witness, not bounded hostile network ingress; its request-policy limits target supported 64-bit hosts. Subprocess tests avoid duplex deadlocks but do not contain an arbitrarily hung replacement interpreter. Finite mutations are regression evidence, not exhaustive fuzzing. Future nonzero schemas and duplicate sorted parents receive syntax acceptance only.

## Frozen source SHA-256

- `crates/abi-wdbx/src/v3/commitment.rs`: `7d8abf123dc76b7602f869629f41fa9625441ecbb9c212dc7ab0e62b19b15dbd`
- `crates/abi-wdbx/src/v3/commitment/decode.rs`: `c72781bc25a8aaeb2e9cf6404db8fb41177381551ff0baf42c66c448162378b3`
- `crates/abi-wdbx/tests/v3_canonical_decode.rs`: `f8f31f698889e85eac441626742a40c9962e1925088fa6ec60968c52801e8a43`
- `crates/abi-wdbx/tests/v3_canonical_decode_limits.rs`: `2894154979f39b111ed8d33f8b1cbf22183c3c2b5a715150e7558d5fb8fa9992`
- `tools/abbey_cbor_episode_v1.py`: `da920b0c3f523024de262328a553c80beca398cab30e13825220379049f155b1`
- `tools/abbey_cbor_reader_v1.py`: `6c6fd35fb857d40e838692d718a039462a3187e529a9582e44cb6d9ab4f9eb60`
- `crates/abi-wdbx/tests/v3_cross_language_decode.rs`: `4ff35518f1a1f6a49b444c57d5e8bbc1baa46277e558208eb1a87dc79f17f155`
