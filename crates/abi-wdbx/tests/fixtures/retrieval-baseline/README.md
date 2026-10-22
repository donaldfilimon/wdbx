# Frozen retrieval-baseline corpus

This directory holds the freeze for the **frozen local baseline comparison
harness** for evidence-aware retrieval — the falsification harness
requirement `P04.03` asks for.

Nothing here comes from a live store, a real tenant, a real conversation or
a real credential. Every identifier is synthetic; the guild is
`synthetic-guild`.

| File | What it is |
| --- | --- |
| `corpus-v1.json` | The frozen corpus: the query families, the embeddings, the ledger shape and the hand-authored ground truth. |
| `expected-metrics-v1.json` | The pinned measurement: the metric values, the per-dimension ablation, the paired bootstrap and the verdict, exactly as the harness produced them. |
| `generate_corpus.py` | The deterministic generator, kept for audit. It writes to stdout and never overwrites the committed corpus. |

## The freeze

The corpus is frozen in three senses.

1. **The embeddings are frozen.** Every embedding is a literal list of
   numbers in `corpus-v1.json`. The harness never re-embeds, never learns
   and never re-normalises: it reads these numbers and ranks them.
2. **The storage is frozen.** The harness appends the corpus to a real
   `EpisodeStore` and reads every fact back out of that ledger. The
   digests, the edges, the lineage and the signature states are the
   store's own.
3. **The candidate count is frozen.** Every query family hands exactly
   **12 candidates** to both rankers before the shared filter, and the
   shared filter (tenant, scope, cryptographic rejection) is applied
   identically to both. The shared filter removes the other tenant's
   record, so both arms rank the same 11 survivors.

`expected-metrics-v1.json` is the fourth freeze: the measurement itself is
pinned, so a change in the harness, in the corpus or in the constitutional
query shows up as a failing test rather than as a quietly different number.

Regenerate the pinned expectation after a deliberate change, with **both**
opt-ins, so a stray environment variable cannot repin a metric:

```sh
WDBX_WRITE_EXPECTED=1 WDBX_REGENERATE_EXPECTED=1 \
  cargo test -p abi-wdbx --test v3_retrieval_baseline
```

The rewrite prints a `WARNING:` line on stderr. `WDBX_WRITE_EXPECTED=1` on its
own is refused: the test fails instead of repinning the expectation, and the
pinned file is left untouched. The repository gate (`tools/check.sh`) unsets
both variables before `cargo test`, so the gate always asserts the pin and
never rewrites it.

## The shape of the corpus

Twelve query families. A family is one "fact topic": a cluster of
near-duplicate memory records about the same fact, and one query vector.

The query vector is the **old phrasing** of the topic. The records that
keep the old phrasing are the ones a plain vector index ranks first, which
is the failure mode under test: a semantically correct answer and a stale,
contradicted, quarantined, forgotten or unattributed answer can be
near-identical to the query, and a vector index cannot tell them apart.

Two templates:

* `applicable_present` (ten families) — twelve records, three of them
  applicable.
* `no_applicable_candidate` (two families) — fourteen records, none of them
  applicable. The two healthy records sit at the two farthest levels, so the
  frozen candidate cut excludes them and the constitutional arm has nothing
  applicable to offer. This is the honest negative case.

### The hand-authored mix

| Role | Count per family | What the canonical query answers |
| --- | --- | --- |
| `healthy_current` | 2–3 | `accepted` |
| `stale_policy` | 2–4 | `uncertain` (the policy binding is stale) |
| `contradicted` | 2 | `contradicted` |
| `quarantined` | 1–2 | `quarantined` |
| `forgotten` | 1 | `rejected` (a tombstone erased it) |
| `unsigned` | 1 | `accepted` (the query accepts an unsigned record on its commitment) |
| `incomplete_lineage` | 1 | `uncertain` (its superseder is quarantined, so the chain does not resolve) |
| `wrong_regime` | 1 | `uncertain` (the committed maturity is specification-only) |

Every record carries **one** defect, except the cross-tenant record, which
is the shared filter's test. That isolation is deliberate: it is what lets
the per-dimension ablation say which dimension carries the benefit.

Two entries need a word.

* **The unsigned record is accepted.** The constitutional query accepts an
  unsigned record on its commitment and reports the missing signature in its
  cryptographic dimension. The corpus records that expectation, and the
  harness *measures* the consequence: the evidence-aware arm hands back an
  unattributed record more often than the plain vector arm does. That is a
  finding about the constitutional rules, and the verdict carries it.
* **The wrong-regime record has a committed maturity of `C0`.** The ledger
  holds no task-regime field, so the query's regime dimension reads
  `unknown` for every record. A record below the query's evidence floor is
  therefore expressed the only way the ledger can express it: as a record
  that is only specified. The regime dimension itself carries no benefit on
  this corpus, and the ablation says so.

### The vectors

Every record's vector is

```
record = u + magnitude * w + 0.001 * jitter
```

where `u` is the family's unit topic vector, `w` is a unit vector
orthogonal to `u`, and `magnitude` is `0.05 * level`. Because `w` is
orthogonal to `u`, the cosine similarity between the query and a record is
`1 / sqrt(1 + magnitude^2)` up to the jitter, so it decreases strictly with
the level. The level order is therefore the whole ranking story, and it is
hand-authored per family rather than emergent.

The level orders are the corpus's deliberate design:

| Order | Families | What it means |
| --- | --- | --- |
| `P1` | `f00`, `f03`, `f06`, `f10` | The defective near-duplicates come first: the query is the old phrasing, and the old phrasing is what the store kept. |
| `P2` | `f01`, `f04`, `f07` | The current phrasing comes first: the plain ranker is right here too. |
| `P3` | `f02`, `f08` | A stale record first, a healthy record second. |
| `P5` | `f11` | The unattributed record comes first. |
| `B` | `f05`, `f09` | No applicable record at all. |

The cross-tenant record of every family sits at level 0 — the closest
position — in the other tenant, so a ranker that forgets the tenant filter
returns a foreign record first.

### The generator

`generate_corpus.py` is the audit trail, not the freeze. It uses only the
Python standard library, seeds every random draw from a fixed constant or
from a SHA-256 of the record identifier, and writes the corpus to stdout:

```sh
python3 crates/abi-wdbx/tests/fixtures/retrieval-baseline/generate_corpus.py
```

Re-running it must reproduce `corpus-v1.json` byte for byte. If it does
not, the freeze has been broken and the pinned expectation must be
regenerated deliberately, not silently.

## What this corpus is not

It is not a research corpus, not a held-out evaluation set and not
production data. It is a synthetic, adversarial, deterministic corpus whose
only job is to make the comparison *measurable*: to make the plain vector
arm fail in a way the constitutional arm can be measured against, and to
keep the negative results.

The corpus is adversarial by construction. A real corpus may show no
benefit at all. See the receipt under `docs/superpowers/plans/` for the
measured numbers, the ablation and the evidence boundary.
