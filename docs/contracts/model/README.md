# Native model contract foundation (N01)

The binding [native package design](native-package-design-v1.md), sections 1–12,
fixes version 1.0. This additive packet supplies Rust contract types, a Python
binary/semantic witness and frozen diagnostic fixtures. N02 later added the bounded
production package parser/writer, staging and byte-level BPE tokenizer
(`abi_wdbx::model::{package,tokenizer}`). There is still no inference engine,
trainer runtime, model provider or publication path.

- [Task sequence and requirement mapping](../../plans/2026-10-25-native-model-cpu.md)
- [Package framing and identities](package-v1.md)
- [Eight component schemas and tensor catalog](components-v1.md)
- [Tokenizer contract](tokenizer-v1.md)
- [Fixture policy and evidence boundary](fixture-policy.md)

`FramedPackage`, `ValidatedModelState` and `ResumableState` have private fields and
no public constructors, `Default` or deserialization implementation. The raw
`ModelComponents` records are unvalidated. N02 must implement bounded snapshot
ownership and full validation before creating these states; N03–N07 own the
remaining execution, training, admission, publication and topology work.

Provenance is never source authority. Diagnostic fixture acceptance proves
contract consistency, not trained quality, successful execution, production
readiness, device support or LYLX replacement. Historical storage, Specimen,
manual-learning profiles and neural codecs retain their existing identities.
