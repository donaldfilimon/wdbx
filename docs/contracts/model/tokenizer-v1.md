# Byte BPE contract 1.0

[Design section 6](native-package-design-v1.md#6-component-4-tokenizerencoder-schema-10)
is the exact grammar and algorithm. IDs 0–3 are empty structural tokens; IDs
4–259 contain bytes 0–255. Merge rank r creates ID 260+r from two prior non-special
IDs, with a unique ordered pair and exact concatenated bytes. Different valid
merge decompositions may produce identical byte strings; rows must not be
deduplicated. Vocabulary size is 260–4096, individual bytes at most 65536, and the
complete component at most 1 MiB.

Encoding begins with byte IDs and applies stored merges once in rank order,
left-to-right without overlap. Literal special-looking text remains ordinary
bytes. Structural sequence encoding and decoding are separate typed operations;
no external tokenizer dependency or generic tokenizer dispatch is introduced.

Training counts adjacent pairs, selects greatest frequency then lexicographically
smallest pair, and merges without crossing documents. The training corpus digest
binds ordered length-prefixed UTF-8 documents. The admitted training split is
external; tokenizer training cannot inspect evaluation documents. Work limits
remain independent of memory limits: 100,000,000 trainer pair/replacement visits,
8,000,000 encoder visits, and 128 MiB live tokenizer training allocation.

N01's Python reference includes only a slow diagnostic encoder to witness literal
BPE IDs. Production bounded training, encoding, decoding, cancellation and staged
publication remain N02 and later tasks. A known vocabulary does not establish
its training-corpus provenance or permission.

## Clarification 1.0.1: tokenizer profile error family (2026-10-27)

This clarification changes no wire byte, field, version or accepted package; it
pins an error family the design left unassigned. Design section 2 says only that
unspecified enum discriminants fail, and section 11 lists the families without
mapping these two bytes.

Fixed-value profile fields of component 4 (`identity`, `normalization`,
`special_policy`, `reserved`, the four special IDs and `minimum_pair_frequency`)
report `InvalidTokenizer`, so `normalization != 0` or `special_policy != 1` is
`InvalidTokenizer`, not `NonCanonical`. This follows the existing rule for other
components with a domain family: objective, optimizer and RNG profile tags report
`InvalidObjective`, `InvalidOptimizer` and `InvalidRngOrder`. Structural enums
inside records (topology node role, tensor dtype and axis kind, manifest
purpose, learning profile and operator set, provenance kinds) and components
without a domain family keep `NonCanonical`; the frozen `bad-role`,
`unsupported-operator` and `unsupported-numeric-profile` vectors are unchanged.
The check happens while component 4 is read, before its trailing-byte check, as
in the witness.

`tokenizer-profile-negatives-v1.json` pins one semantic negative per byte (both
derived from frozen `inference` with recomputed component and directory hashes);
the Rust package corpus and the Python witness both assert `InvalidTokenizer`.
