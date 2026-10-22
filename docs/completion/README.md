# Completion contracts — inventory v1, preregistration v3

This directory is a **C0 proposal and preregistration**, not implemented research,
product conformance, or a release claim. It is the Task 1 handoff for the
[completion program](../superpowers/plans/2026-10-20-wdbx-full-completion.md).
Independent review is required before accepting the handoff; independent
experiments, crypto review and platform receipts remain future work.

| Artifact | Contract |
| --- | --- |
| `requirements.json` | 163 IDs: 43 program bullets across ten tasks, 13 global constraints, 26 Studio chapters, 46 behavioral cases, 21 constitutional section-5 items, eight evidence dimensions and six platforms. Each binds owner, task, source line/text, plan, evidence expectations and completion condition. |
| `studio-acceptance-v1.json` | All 46 section 25.3 rows and 18 section 25.1 decisions, verbatim and in source order. |
| `sources.json`, `sources/` | Hash-pinned reference snapshots, not an alternative normative ABI corpus. Upstream authority stays with its repository. No fixture refresh or schema ownership changes. |
| `baseline.json` | Repository heads, dirty paths only, source hashes, existing mechanisms versus gaps, gate/environment status. Controller-reported local gate identified as such. |
| `designs.json` | Nine bundles covering all 18 decisions with algorithms, interface shapes, limits, failures and acceptance. All require implementation and falsification. |
| `plans.json` | Ten ordered work packages: interfaces, dependencies, steps, assertions, required evidence and complete requirement routing. |
| `synthetic-corpus-v2.jsonl`, `experiments.json` | Frozen cases, grouped splits, seeds, budgets, baselines, thresholds, analysis and result schema. No experiment results. |

Run from the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 tools/check_completion.py
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tools/tests -p test_check_completion.py
PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3 -m unittest discover -s tools/tests -p test_check_completion.py
git diff --check
```

These are contract checks, not the Rust/Studio gates or hardware qualification.
The checker uses only the standard library and supports Python 3.9. It remains
separate from the existing product gate.

The checker enumerates IDs from independently pinned source bytes; removing a
case from both inventory and manifest still fails. It checks source hashes,
verbatim rows, task routing, chapters/platforms, design decisions, dependencies,
experiment bytes and disjoint family splits. Claims remain `proposed`/`C0` with
empty evidence arrays. Future promotion needs a reviewed schema/version and
an evidence verifier. Pin changes require review, not automatic regeneration.

Mechanical closure cannot judge scientific adequacy or whether prose supplies
an honest completion assertion. Review must inspect algorithms, limits and tests.
A chapter entry is a coverage obligation, not a substitute for mechanism tests.
The constitutional inventory focuses on section 5 and the approved program; it
does not inventory unrelated ABI/Discord requirements.

## Research boundary

The active v2 corpus has 2,400 synthetic records in eight suites. Its 800 scenario
groups split 60/20/20 per suite; all three variants stay together. The prior corpus
and protocol remain byte-for-byte under `synthetic-corpus-v1.jsonl` and
`experiments-v1.json`; they were superseded before experiments after independent
review found insufficient text and retrieval cases.

ATP cases now include source text, requested tone, correct and incorrect
alternatives and proposition oracles. Mutation cases include literal slots,
required meaning, permitted optional variation and novelty/rating rules. Retrieval
has 100 distinct numerical/metadata scenarios, explicit query vectors, 12 candidate
vectors each and three presentation variants per family. Its baseline filtering,
cosine ranking, tie policy, top-1 output and failure/staleness denominators are
frozen in the protocol. Variants are averaged before family bootstrap; merely
relabelling repeated inputs as families is rejected.

Metric-to-input/oracle mappings are checked independently of file hashes. Public
frozen files do not prove that developers never inspected a holdout; independent
contamination controls and blind raters remain required. Shared synthetic schemas
measure bounded generalization, not new natural language distributions. Safety
invariants are not independent efficacy samples.

Vision cases freeze geometric instructions/pixels and actual OCR PBM raster bytes
with transcription labels and per-image digests. The synthetic bitmap font is
intentionally narrow: independently inspect decoded images before execution.
Actual GPU execution, real-world OCR, arbitrary-language synthesis and broad
vision remain open. Passing structural checks cannot close these claims.

Failed, inconclusive or contaminated research leaves its claim open. Retain every
output, timeout, negative result and configuration. Passing synthetic thresholds
does not establish general efficacy; expanded independently held corpora need a
new preregistered version. No live store, private conversation, credential or model
weight is used.

## Interpretation boundaries

New text encoding is versioned; old IDs and signed bytes are not reinterpreted.
Synthesis selects bounded fixed-network AST composition without attention or
learning; arbitrary coherent generation remains an empirical question. ATP uses
bounded buildup/release equations rather than a literal Lorenz system. PHAGY has
a finite job catalog and fencing. Graves never automatically resurrect nodes;
privacy erasure wins over archive restoration. These are proposed choices, not
retroactive descriptions of current source.

Platform and hosted plans require independent hosts and installed bytes. They do
not authorize deployment, signing with user credentials, public ingress or changes
to live stores. Ownership and authorization boundaries apply to downstream work.

## Archived fix-round-1 decision artifacts

- `synthesis-contract-v1.md` defines all 128 feature slots, operation vectors,
  decoder grammar/ordering and rational mutation alignment/ties.
- `synthesis-network-v1.json`, `synthesis-operations-v1.json` and
  `synthesis-worked-v1.json` pin actual weights, operation features and one complete
  input-to-output example. `contract_vectors_v1.py` reproduces them as contract
  witnesses; running it is not an efficacy experiment.
- `inhibition-contract-v1.md` specifies framed HMAC preimages, scoped key ownership,
  historical MAC-epoch matching, wrapping versus MAC rotation, failure behavior
  and minimal retained metadata. `inhibition-vectors-v1.json` uses synthetic keys
  and demonstrates equivalence, erasure, rotation and tenant/scope separation.
  Security/crypto qualification remains independent future work.
- `shared-types-v1.json` defines the ATP fields and terminal receipt tagged union,
  and gates cross-package serialization on owner-reviewed schemas and vectors.
  ABI's normative wire authority is unchanged.

The checker binds task number, ID, owner and position, checks each requirement is
routed exactly once, and enforces mandatory prerequisite edges. All versioned
algorithm/vector artifacts and the superseded v1 protocol/corpus are hash-pinned.

## Active fix-round-2 decision artifacts

Protocol version 3 retains the corpus v2 bytes and archives the preceding protocol
as `experiments-v2.json`. No experiment has run. Active synthesis/inhibition
contracts are `synthesis-contract-v2.md` and `inhibition-contract-v2.md`; v1 files
and vectors remain unchanged. Historical witness code is `contract_vectors_v1.py`;
active `contract_vectors.py` implements the new executable examples.

Synthesis now uses exact rational score buckets followed by canonical byte order,
with a near-tie vector and permutation tests. The public corpus facts/query adapter
feeds an actual composite report decoder, producing owner, object, computed final
count and all three source refs. The worked v2 vector invokes that pipeline; it
does not return a hardcoded expected AST or consume oracle data. Missing operands
abstain and changed operands change the output. This is a contract demonstration,
not an efficacy run.

Permanent inhibition v2 MACs the normalized pair alone. Empty/changed policy codes
therefore cannot evade an old pair ban after payload erasure or MAC-key rotation.
No old code list is needed for matching. Tenant/scope separation, old-epoch
retention and independent crypto review requirements remain in force. New vectors
exercise both changed and empty code sets in each MAC epoch.
