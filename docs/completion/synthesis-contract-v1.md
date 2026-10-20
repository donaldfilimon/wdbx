# Synthesis and alignment contract v1 — proposed C0

This resolves D09's deterministic choices. `synthesis-network-v1.json` is the
frozen network artifact; `contract_vectors.py:network` reproduces it. Dimensions,
CSR offsets, columns, IEEE binary32 weights and biases are all explicit. It uses
the existing default topology and seed 104729, not an unspecified runtime network.
The artifact's hash is pinned by the contract checker. Editing a user's network
is a distinct version, not this experiment. No network training is authorized.

## Input and exact feature map

Input is a typed IngredientGraph containing stable source refs, facts
`[subject,predicate,object]` with object a string or signed 64-bit integer,
optional literal spans, query, context flags, and ATP fields. Source refs sort by
UTF-8 byte order. Tokens are emitted from facts in that order, then positions
subject/predicate/object. Each becomes `[role,value]`; integer values use minimal
base-10 spelling. Strings are already normalized by the versioned Language
Governance boundary: synthesis performs **no additional case, Unicode, stemming,
negation or whitespace normalization**. Embedded NUL is rejected. Max 32 facts,
128 tokens, 256 bytes/token, all UTF-8. Corpus entities use ASCII.

For each token compute SHA256(UTF8(role) || 00 || UTF8(value)); bucket is the first
four digest bytes as unsigned big-endian modulo 96; sign is +1 if bit zero of
byte 4 is zero, else -1. Add binary32(sign / max(1,token_count)) in token order,
rounding each addition to binary32. Buckets 0–95 start at zero.

Context slots 96–111 are these exact booleans, converted to 0 or 1:

| Offset | Field |
| --- | --- |
| 96 | query kind `count_after_add` (arithmetic) |
| 97 | query kind `select` |
| 98 | query kind `concatenate` |
| 99 | query kind `equivalent` |
| 100 | query kind `join` |
| 101 | query kind `shape` |
| 102 | explicit request for explanation |
| 103 | explicit request for terse output |
| 104 | explicit request for multiple alternatives |
| 105 | source contains contradiction disposition |
| 106 | source contains uncertain disposition |
| 107 | source contains quoted span |
| 108 | source contains code span |
| 109 | source contains inhibited action |
| 110 | source contains visual ingredient |
| 111 | source contains missing required role |

Exactly one query-kind bit is set. Flags come from explicit typed metadata,
not inferred from strings. Missing flags default false.

ATP slots 112–127 are binary32 values in this order: valence `v` [-1,1]; intensity
`a` [0,1]; min(cooldown_seconds,30)/30; urgency bool; familiar bool; brainstorm
bool; max(v,0); max(-v,0); a*v; bounded reward [-.1,.1]; min(idle_seconds,120)/120;
previous output valence [-1,1]; previous output intensity [0,1]; tone tolerance
[0,1]; autonomy enabled bool; eye arousal [0,1]. Missing optional history/reward/
time fields are zero; tolerance is explicitly supplied (zero in worked vector).
`v/a` are the sole wire field names: scoring's valence/intensity mean `v/a`.
Inputs outside bounds fail, rather than clamp silently.

## Numerical execution and operation table

`contract_vectors.py` defines xorshift64 exactly, including wrap and zero-state
replacement. Two RNG samples choose the two layer activations, sigmoid iff sample
<.75 for familiar, <.25 otherwise. Within each CSR row, multiply and add in column
storage order with a binary32 rounding after each operation, then add bias.
Sigmoid is 1/(1+exp(-clamp(x,-30,30))); ReLU clamp(x,0,16). Rust uses libm; the
Python vector is a mathematical reference. Outputs/operation scores must agree
within absolute 2e-6; byte parity is not claimed for transcendental functions.
Any two candidate scores within 4e-6 are a tie, resolved as below, so comparison
never depends on backend least-significant-bit noise. CPU reference is required;
GPU execution is separately evidenced and may not silently change the ranker.

Operation feature vector i is the 32-dimensional one-hot vector e_i:

| i | Operation | Typed AST and enumeration | Rendering |
| --- | --- | --- | --- |
| 0 | select | `[select,ref]` for each admitted literal/fact ref | literal bytes; fact as `subject predicate object` |
| 1 | concatenate | `[concatenate,left,right]` for ordered distinct admitted spans | left + `; ` + right |
| 2 | equivalent | `[equivalent,ref,equivalence_id]` for exact table-approved replacements | frozen replacement literal |
| 3 | arithmetic | `[arithmetic,+,base,added]` only for query count_after_add whose matching contains/added facts establish operands | minimal decimal sum |
| 4 | join | `[join,left_ref,right_ref]` where left.object == right.subject, predicates owns/contains | `subject owns object; object contains count.` |
| 5 | shape | `[shape,operation,left_ref,right_ref]`, operation union/intersection/subtract requested explicitly | typed JSON instruction AST, not invented image text |

All other coordinates are zero. Raw operation score is dot(output,e_i). No
unspecified learned vectors, words, templates or grammar glue exist. Equivalent
table entries come from the frozen input resources (with source refs); absent
entries produce no candidates. Negation, code, numeric and quoted spans cannot
be replaced. Every operation retains the union of source refs and transformation
nodes, including both contains/added refs for arithmetic.

Enumerate ASTs by depth (max 4), operation index, then lexicographically by
canonical compact JSON UTF-8 bytes (ensure_ascii=false, no floats, sorted object
keys). Leaves are source refs; at each depth apply the six rules to the surviving
beam, including all source leaves. No identical AST repeats. Admit only query-kind
compatible roots; completed count_after_add roots must establish both operands
from source facts, not trust query operands. Check inhibition, scope, type and
source constraints before scoring. Candidate score is mean raw operation scores
for its non-leaf nodes minus .01*(node_count-1). Keep the top 8 per depth, treating
scores within 4e-6 as ties and choosing canonical AST bytes ascending. Stop after
256 expansions (count each attempted AST, including rejected ones), 32 source
spans, 512 output tokens or depth 4. Return highest-scoring completed AST seen;
if none, Abstain with reason. Token count for this bounded decoder is whitespace
separation; shape JSON counts each scalar as a token. No backdoor arbitrary text
model is part of this protocol.

`synthesis-worked-v1.json` gives normalized input, 128 features, both activations,
32 outputs, raw operation ranking, admissibility filter and final AST/text with
source refs. Its metadata is explicit: query count_after_add, all other flags and
ATP fields zero. Only arithmetic is admissible for its query, even if another
operation has a higher raw score. Reproducing this vector establishes a contract
example, not efficacy or independent implementation evidence.

## Mutation slot alignment

Before alignment, order recipient and donor alternatives by immutable slot-ref
UTF-8 bytes. Score literal action pairs using case-sensitive Unicode whitespace
split tokens, preserving punctuation, numbers and negation. Let U be token
multisets and B ordered adjacent-token-pair multisets. Similarity is the exact
rational `100 * (2*J(Ur,Ud) + J(Br,Bd))/3`, with multiset J=sum(min counts)/
sum(max counts); empty/empty J=1, one empty J=0. No floating-point comparison.
Reject edges below 20 and mismatched structured action kinds; patterns separately
must pass the specified scanner threshold 80. Donors must be stronger.

For pools <=8, enumerate all injective donor assignments to the first
min(recipient_count,donor_count) recipient refs; require every mapped edge eligible.
Maximize sum of exact rational similarities (cross-multiply to compare). Equal
sums choose lexicographically smallest vector of donor refs in recipient-ref
order. For larger pools, sort edges by rational score descending, then recipient
ref and donor ref ascending, greedily take disjoint edges; incomplete matching
rejects the event. No donor can be reused in a single alignment. Tail retirement
is by recipient-ref order, not array position; any already-remixed retired slot
rejects shortening. Store selected mapping and permanent retired refs atomically.
The 2-slot remix budget and semantic-validity conditions remain unchanged. A
failed text validation neither consumes the marker nor shortens a pool.
