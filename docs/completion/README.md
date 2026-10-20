# Completion contracts, version 1

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
| `synthetic-corpus-v1.jsonl`, `experiments.json` | Frozen cases, grouped splits, seeds, budgets, baselines, thresholds, analysis and result schema. No experiment results. |

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

The corpus has 2,400 synthetic records in eight suites. Its 800 scenario groups
split 60/20/20 per suite; all three variants stay together. Test entities and
numeric ranges differ from training, but generation schemas are shared. This
measures bounded composition, not novel language distributions or general vision.
Repeated deterministic invariants are safety assertions, not independent efficacy
observations. Independent raters and group-level analysis are required for meaning
and groundedness. A public held-out file is a frozen split, not proof developers
have never inspected it; the independent runner must attest contamination controls.

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
