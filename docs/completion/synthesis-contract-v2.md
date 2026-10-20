# Synthesis contract v2 — proposed C0, before experiments

This version supersedes only the ordering/decoder bridge in v1. The v1 document,
weights, operation vectors, worked example and witness remain archived unchanged.
Their historical `contract_vectors.py` reference resolves to archived
`contract_vectors_v1.py`. The active witness is `contract_vectors.py`.

The exact tokenization/128-feature definitions, network weights, numeric execution,
original six operation meanings, safety bounds and rational mutation alignment in
`synthesis-contract-v1.md` remain in force except for the explicit overrides here.
Active operations are `synthesis-operations-v2.json`: original e0..e5 plus e6 for
`report`. No learned weights or invented semantic feature is added.

## Total ordering of serialized scores

Reject nonfinite scores. Treat each supplied binary64 score x as its **exact**
rational value n/d (`float.as_integer_ratio` in Python). The bucket is
`floor(n * 250000 / d)`, using exact signed integer arithmetic and mathematical
floor for negatives. Thus bucket width is the rational 1/250000. Boundaries are
left-closed/right-open in the rational domain. A binary64 representation of a
printed decimal boundary may lie just below it; use its actual bits, not its
printed spelling. No epsilon comparison or pairwise closeness is used.

Sort by bucket descending, then canonical AST UTF-8 bytes ascending. For raw
operation rankings the canonical name bytes are the secondary key. Identical
ASTs deduplicate before ranking. This is a transitive total order over distinct
candidates and is used identically for beam pruning, streaming top-k and final
choice. No exact-score tertiary key overrides a bucket tie.

`synthesis-near-ties-v2.json` includes a three-item 0, 3e-6, 6e-6 chain and negative/
boundary examples. All enumeration permutations produce the same order. The
reference tests execute all permutations of the three-item chain.

Numeric tolerance is an output-comparison allowance, not a ranking tie relation.
Near a bucket boundary different backend output bits may produce different buckets;
such a discrepancy is recorded and ranking falls back to serialized CPU reference
scores. Accelerator tolerance alone never certifies rank parity. Recorded score
bits plus ASTs suffice to replay the exact ordering.

## Public composition corpus adapter

`composition_graph(public_input)` accepts **only** the `input` object from a
composition corpus row, containing exactly `facts` and `query`. Passing the full
row (oracle, family, split or labels) rejects. The grammar is deliberately bounded:

```
what does IDENT own and how many items after adding?
IDENT := one or more ASCII letters, digits or underscores
```

Matching is case-sensitive and whole-string. Unsupported wording returns an
explicit unsupported-query error; no heuristic oracle access is allowed. Parse
IDENT into typed query `{kind:owner_count_after_add, subject:IDENT}`. This belongs
to the arithmetic query family: context feature 96 is 1 and other context flags
are 0 for these fixtures. ATP fields are all 0, familiar=false, seed104729
for the worked example; experiments supply their preregistered seed via the adapter's separate unsigned64 seed argument.

Each `facts[i]` must be `[subject,predicate,object]`, with subject string, predicate
one of `owns`, `contains`, `added`; owns.object is string and other objects are
signed 64-bit integers (booleans are rejected). At most32 triples; each token <=256
UTF-8 bytes and contains no NUL. The supplied fact at array offset i gets stable
source ref `input/facts/i`. Keep these refs through sorting; never infer them from
oracle.required_sources. Sort ingredients by ref UTF-8 bytes, emit all three role/
value tokens from each in subject/predicate/object order, including repeated
subjects. This directly uses v1's exact feature encoder.

## Compatible composite root and executable decoder

For query `owner_count_after_add`, neither a scalar sum nor an owns/contains report
of the original count is complete. Only the following source-backed root is
compatible:

```
["report", owns_ref, ["arithmetic", "+", contains_ref, added_ref]]
```

The owns fact's subject must equal query.subject. Both count facts' subjects must
equal owns.object; their predicates must be contains and added respectively. Sum
uses checked signed64 addition. The completed root contains exactly owner,
object, final count and the union of the three actual source refs. Its rendering
is `OWNER owns OBJECT; OBJECT contains COUNT after adding.`; structured fields
`{owner,object,count}` accompany the text. Each field has direct provenance:
owner/object→owns_ref, count→contains_ref+added_ref. The list of source refs is
UTF-8 sorted, and maps back to oracle indices only during independent evaluation.

Enumerate owns ingredients in ref order with matching query subject, then all
contains ingredients in ref order, then all added ingredients in ref order.
Each typed triple considered counts one expansion, including rejected object
joins/overflow. Stop after256 expansions; preserve complete candidates already
seen. Zero complete candidates returns Abstain with reason, never a guessed count.
The fixed shape has two operation nodes, depth2 and three leaves, within the v1
bounds. Candidate score is `(network_output[6]+network_output[3])/2 - .01` in
binary64, corresponding to mean report/arithmetic score minus the declared one
extra-node cost. Deduplicate canonical ASTs, apply the total order above, keep
beam8 and select its first root. Outputs exceeding512 whitespace tokens reject.
This typed direct enumeration is the full decoder for this query, not a request
to implement an unspecified search. Other query kinds retain their v1 enumeration
rules but must use the new total order.

The corpus public-input adapter never receives or reads oracle fields. Required
source facts, bindings and final fields are computed from facts and parsed query.
Irrelevant entities cannot supply counts. Missing operands yield Abstain; altered
source counts alter the computed result. No predicate or relation is hallucinated.

`synthesis-worked-v2.json` is generated from the exact public input of training
row `composition-000-0` via adapter→encoder→frozen network→decoder. It records the
graph, 128 features, activations, outputs, raw operation order and computed AST,
fields, text, refs and bucket. The witness executes the decoder rather than
hardcoding expected output. Tests verify the worked bytes, changed operand,
missing operand and refusal of full rows containing oracle data. This is one
contract witness, not a run or pass of the efficacy corpus.
