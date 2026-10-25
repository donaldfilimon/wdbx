# Native package 1.0 diagnostic vectors

`manifest.json` is the frozen inventory. Every `.hex` has an ordinary SHA-256
sidecar and an explicit framing/semantic outcome. Positive rows also pin package,
state and generation identities. Auxiliary JSON files pin field offsets and
literal SplitMix64/xoshiro256** vectors.

All positive models use N=3, M=2, d=2, R=1, K=2, V=260 (BPE: 261). Node IDs are
10 Input, 30 Memory and 90 Readout; edges are 7 (10→90) and 42 (90→30), relation
0 `link`. ID reduction order is therefore 42,7 while tensor 9 remains 7,42.
High-water marks are 100 and 50. Every tensor is a literal diagnostic array:
non-coordinate tensor i has `(i % 3 - 1) * 0.125` in each scalar; coordinates are
[-1,0,1], [0,0.5,0], [1,0,-1]. The signed-zero vector changes b's first scalar to
negative zero. These are not trained artifacts.

The resumable fixture has step 3, first moments 0.125, second moments 0.25,
seed 7, explicit five streams (shuffle stream consumed once), epoch 2,
permutation [2,0,1], cursor 1 and batch 1. This is a coherent diagnostic boundary,
not evidence of a historical training run or reproducible training implementation.
The BPE fixture adds pair (101,102) → 260, bytes `ab`; `abab` encodes [260,260].

Model ID is sixteen 0x01 bytes; partition is thirty-two 0x02 bytes (partition
variant: 0x09). Data/split/preparation references use 0x03/0x04/0x05; audience is
0x06, source tree 0x07 and eligibility reference 0x08. The corpus digest represents
an empty ordered corpus. Origin is Diagnostic and restrictions include bit 0.
No reference is an authorization grant. No filesystem or source payload is used.

Negative rows specify exact expected errors. Semantic negatives recompute all
outer hashes. `annotation-cannot-supply-tensor` cannot replace missing graph
state with optional bytes. `missing-core-annotation` cannot replace provenance
with an opaque annotation. `payload-tamper` preserves file size and intentionally
leaves hashes stale. No automatic regeneration is supported.

`saturated-counters` is an OnlineV1 diagnostic with policy digest 0x09 and epoch
1. Its RNG/order epoch, accepted proposal count and next proposal step are
u64::MAX. Section 9 imposes checked future arithmetic and ordering, not a stricter
serialized bound on those fields. This is valid captured data, not a promise of
runnable progress or authorization: future increments must fail before mutation.
`step-exhausted`, `rng-consumed-exhausted` and `node-id-high-water-exhausted`
separately pin the explicit strict limits on successful step, PRNG consumption
and stable-ID high-water marks. All four vectors have freshly computed integrity
hashes, so they reach semantic checks.
