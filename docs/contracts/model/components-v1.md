# Required component schemas 1.0

The full ordered field definitions and cross-component rules remain binding in
[design sections 3–10](native-package-design-v1.md#3-component-1-architecture-and-compatibility-schema-10).
The Rust records live under `crates/abi-wdbx/src/model/contract/` and deliberately
represent unvalidated data. Enum conversion rejects all unnamed discriminants.
Rust layout, `Vec` layout and enum layout are never the wire encoding.

| Kind | Schema | Contract module |
| --- | --- | --- |
| 1 | Architecture, compatibility, capacity and work ceilings | architecture |
| 2 | Stable-ID topology and immutable relation axes | topology |
| 3 | Sixteen exact typed tensor members | tensors |
| 4 | Byte vocabulary and ranked BPE merges | tokenizer |
| 5 | Objective weights, reductions and data identities | training |
| 6 | Absent or complete AdamW moments and successful step | training |
| 7 | Absent or complete post-step RNG/order state | training |
| 8 | Provenance, audience, policy and source-root references | provenance |

| ID | Name | Dtype | Ordered axes | AdamW decay |
| --- | --- | --- | --- | --- |
| 1 | b | F32 | Node, HiddenOut | no |
| 2 | p | F64 | Node, XYZ | no |
| 3 | E | F32 | Token, HiddenOut | no |
| 4 | U | F32 | HiddenOut, HiddenIn | yes |
| 5 | R | F32 | HiddenOut, HiddenIn | yes |
| 6 | W | F32 | Relation, HiddenOut, HiddenIn | yes |
| 7 | S | F32 | HiddenOut, HiddenIn | yes |
| 8 | c | F32 | HiddenOut | no |
| 9 | a | F32 | Edge | no |
| 10 | O | F32 | Token, HiddenIn | yes |
| 11 | o | F32 | Token | no |
| 12 | J | F32 | HiddenOut, HiddenIn | yes |
| 13 | Q | F32 | HiddenOut, HiddenIn | yes |
| 14 | B | F32 | HiddenOut, HiddenIn | yes |
| 15 | d_q | F32 | HiddenOut | no |
| 16 | v | F32 | HiddenOut | no |

Node and Edge descriptors include full ordered stable ID lists. Reduction order
is separately `(destination, relation, source, edge)`; it never reorders persistent
edge gate rows. Moment arrays inherit exact tensor membership and row-major axes.
Neural, physical and display positions have distinct types; only learned unitless
coordinates occur in core tensor 2. Topology holds no competing coordinate copy.

Inference-only state requires both absent schemas. Resumable state requires all
sixteen optimizer members, five explicit RNG streams, a valid permutation/cursor,
matching data and step identities, and current external authority. The contract
alone cannot verify historical PRNG consumption, lineage allocation floors,
dataset dependency completeness or license/consent eligibility.

Section 9's epoch and topology proposal counters may capture u64::MAX when the
explicit ordering/profile rules hold. Checked future increments must refuse
before mutation, never wrap or silently change policy. This differs from the
explicit strict upper bounds on successful step, consumed PRNG outputs and
stable-ID high-water marks. The `saturated-counters` boundary fixture records
this interpretation; it makes no claim that further progress is possible.
