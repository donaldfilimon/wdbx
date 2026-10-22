#!/usr/bin/env python3
"""Regenerate the frozen retrieval-baseline corpus.

The committed JSON file is the freeze. This script is retained for audit
only: it documents exactly how the frozen numbers were produced. It writes
the corpus to stdout and never overwrites the committed fixture.

The corpus is a deterministic, synthetic, non-production corpus for the
frozen local baseline comparison harness (requirement P04.03). Nothing here
is derived from a live store, a real tenant, or a real conversation.

Shape
-----
Twelve query families. Every family is one "fact topic": a cluster of
near-duplicate memory records about the same fact, plus one query vector.

The query vector is the *old* phrasing of the topic. Records that keep the
old phrasing are the ones a plain vector index ranks first, which is the
repeated-failure / stale-reuse failure mode under test: a semantically
correct answer and a stale, contradicted, quarantined, forgotten or
unattributed answer can be near-identical to the query, and a vector index
cannot tell them apart.

Per-record vector construction (deterministic, no RNG state carried across
records):

    record = u + magnitude * w + 0.001 * jitter

where `u` is the family's unit topic vector, `w` is a unit vector
orthogonal to `u`, and `magnitude` is `0.05 * level`. Because `w` is
orthogonal to `u`, the cosine similarity between the query (`u`) and a
record is `1 / sqrt(1 + magnitude^2)` up to the tiny jitter, so it decreases
strictly with the level. The level ordering is therefore the whole ranking
story, and it is hand-authored per family rather than emergent.

The cross-tenant record of each family sits at level 0 (the closest
position) in the other tenant, so a ranker that forgets the tenant filter
returns a foreign record first.

Run with the standard library only:

    python3 crates/abi-wdbx/tests/fixtures/retrieval-baseline/generate_corpus.py
"""

from __future__ import annotations

import hashlib
import json
import math
import sys

DIMENSION = 16
ROUNDING = 9
JITTER = 0.001
LEVEL_STEP = 0.05
CROSS_TENANT_LEVEL = 0.0
CROSS_TENANT_MAGNITUDE = 0.02

TENANT = "synthetic-guild"
OTHER_TENANT = "synthetic-guild-alt"
CURRENT_POLICY = "policy_v1"
STALE_POLICY = "policy_v0"
CONTRACT_REVISION = 2
CONTRACT_DIGEST_HEX = "09" * 32
SIGNER_SEED = 11
QUERY_MIN_EVIDENCE = "C2"

# Hand-authored mix. Roles carry the whole ground truth; the vector levels
# only decide which of them a pure vector ranker happens to see first.
ROLES = {
    "healthy_current": ("current", "C3", True, "accepted"),
    "incomplete_lineage": ("current", "C3", True, "uncertain"),
    "stale_policy": ("stale", "C3", True, "uncertain"),
    "contradicted": ("current", "C3", True, "contradicted"),
    "quarantined": ("current", "C3", True, "quarantined"),
    "forgotten": ("current", "C3", True, "rejected"),
    "unsigned": ("current", "C3", False, "accepted"),
    "wrong_regime": ("current", "C0", True, "uncertain"),
}

# Template "applicable_present": twelve records, three of them applicable.
TEMPLATE_A = [
    ("h1", "healthy_current"),
    ("h2", "healthy_current"),
    ("h3", "healthy_current"),
    ("l", "incomplete_lineage"),
    ("s1", "stale_policy"),
    ("s2", "stale_policy"),
    ("c1", "contradicted"),
    ("c2", "contradicted"),
    ("q1", "quarantined"),
    ("f1", "forgotten"),
    ("u1", "unsigned"),
    ("r1", "wrong_regime"),
]

# Template "no_applicable_candidate": fourteen records, none of them
# applicable. The two healthy records sit at the two farthest levels, so the
# frozen candidate cut excludes them and the constitutional arm has nothing
# applicable to offer. This is the honest negative case.
TEMPLATE_B = [
    ("h1", "healthy_current"),
    ("h2", "healthy_current"),
    ("l", "incomplete_lineage"),
    ("s1", "stale_policy"),
    ("s2", "stale_policy"),
    ("s3", "stale_policy"),
    ("s4", "stale_policy"),
    ("c1", "contradicted"),
    ("c2", "contradicted"),
    ("q1", "quarantined"),
    ("q2", "quarantined"),
    ("f1", "forgotten"),
    ("u1", "unsigned"),
    ("r1", "wrong_regime"),
]

# Hand-authored level orders. Level 1 is the nearest to the query.
LEVEL_ORDERS = {
    # The defective near-duplicates come first: the query is the old
    # phrasing, and the old phrasing is what the store kept.
    "P1": ["s1", "c1", "q1", "f1", "u1", "r1", "s2", "c2", "l", "h3", "h1", "h2"],
    # The current phrasing comes first: the plain ranker is right here.
    "P2": ["h1", "h2", "h3", "s1", "c1", "q1", "f1", "u1", "r1", "s2", "c2", "l"],
    # Mixed: a stale record first, a healthy record second.
    "P3": ["s1", "h1", "c1", "q1", "h2", "f1", "u1", "r1", "s2", "c2", "l", "h3"],
    # The unattributed record comes first: not a headline failure, but the
    # plain ranker hands back a record nobody signed.
    "P5": ["u1", "h1", "h2", "s1", "c1", "q1", "f1", "r1", "s2", "c2", "l", "h3"],
    # No applicable record at all.
    "B": ["s1", "c1", "q1", "u1", "f1", "s2", "r1", "c2", "s3", "s4", "l", "q2", "h1", "h2"],
}

FAMILY_PLAN = [
    ("f00", "applicable_present", "P1"),
    ("f01", "applicable_present", "P2"),
    ("f02", "applicable_present", "P3"),
    ("f03", "applicable_present", "P1"),
    ("f04", "applicable_present", "P2"),
    ("f05", "no_applicable_candidate", "B"),
    ("f06", "applicable_present", "P1"),
    ("f07", "applicable_present", "P2"),
    ("f08", "applicable_present", "P3"),
    ("f09", "no_applicable_candidate", "B"),
    ("f10", "applicable_present", "P1"),
    ("f11", "applicable_present", "P5"),
]


class Rng:
    """A 64-bit LCG. Deterministic, seeded, and never seeded from entropy."""

    def __init__(self, seed: int) -> None:
        self.state = seed & 0xFFFFFFFFFFFFFFFF

    def next_u64(self) -> int:
        self.state = (self.state * 6364136223846793005 + 1442695040888963407) & 0xFFFFFFFFFFFFFFFF
        return self.state

    def unit(self) -> float:
        return ((self.next_u64() >> 11) / float(1 << 53)) * 2.0 - 1.0

    def vector(self, count: int) -> list[float]:
        return [self.unit() for _ in range(count)]


def normalize(values: list[float]) -> list[float]:
    norm = math.sqrt(sum(value * value for value in values))
    return [value / norm for value in values]


def orthogonal(base: list[float], rng: Rng) -> list[float]:
    """A unit vector orthogonal to `base`, deterministically."""
    raw = rng.vector(len(base))
    projection = sum(a * b for a, b in zip(base, raw))
    return normalize([a - projection * b for a, b in zip(raw, base)])


def record_seed(record_id: str) -> int:
    """FNV-1a over the record id, so a record's jitter never depends on
    how many records were generated before it."""
    digest = hashlib.sha256(record_id.encode("utf-8")).digest()
    return int.from_bytes(digest[:8], "big")


def round_vector(values: list[float]) -> list[float]:
    return [round(value, ROUNDING) for value in values]


def make_record(
    family_id: str,
    key: str,
    role: str,
    base: list[float],
    direction: list[float],
    magnitude: float,
    tenant: str,
) -> dict:
    policy, evidence, signed, expected = ROLES[role]
    record_id = f"{family_id}-{key}"
    jitter_rng = Rng(record_seed(record_id))
    jitter = orthogonal(base, jitter_rng)
    vector = [
        base[axis] + magnitude * direction[axis] + JITTER * jitter[axis]
        for axis in range(len(base))
    ]
    return {
        "record_id": record_id,
        "role": role,
        "vector": round_vector(vector),
        "tenant": tenant,
        "policy_version": CURRENT_POLICY if policy == "current" else STALE_POLICY,
        "evidence_level": evidence,
        "signed": signed,
        "member_scoped": True,
        "supersedes": None,
        "expected_disposition": expected,
    }


def build_family(family_id: str, template: str, order_key: str) -> dict:
    members = TEMPLATE_A if template == "applicable_present" else TEMPLATE_B
    by_key = dict(members)
    order = LEVEL_ORDERS[order_key]

    family_rng = Rng(record_seed(family_id))
    base = normalize(family_rng.vector(DIMENSION))
    direction = orthogonal(base, family_rng)

    records = []
    for level, key in enumerate(order, start=1):
        role = by_key[key]
        record = make_record(
            family_id,
            key,
            role,
            base,
            direction,
            LEVEL_STEP * level,
            TENANT,
        )
        if key == "q1":
            # The quarantined correction replaces the record that then has
            # no usable current version: its lineage is incomplete.
            record["supersedes"] = f"{family_id}-l"
        records.append(record)

    # The cross-tenant record sits closest to the query of its own family.
    cross = make_record(
        family_id,
        "x",
        "healthy_current",
        base,
        direction,
        CROSS_TENANT_MAGNITUDE,
        OTHER_TENANT,
    )
    cross["role"] = "cross_tenant"
    cross["expected_disposition"] = "filtered_out"

    edges = [
        {
            "kind": "contradicts",
            "target": f"{family_id}-c1",
            "counterpart": f"{family_id}-c2",
            "reason": "conflicting_observation",
        },
        {
            "kind": "quarantines",
            "target": f"{family_id}-q1",
            "counterpart": None,
            "reason": "source_untrusted",
        },
    ]
    if template == "no_applicable_candidate":
        edges.append(
            {
                "kind": "quarantines",
                "target": f"{family_id}-q2",
                "counterpart": None,
                "reason": "policy_violation",
            }
        )

    tombstones = [
        {
            "record_id": f"{family_id}-t1",
            "forgets": f"{family_id}-f1",
            "policy_version": CURRENT_POLICY,
            "evidence_level": "C3",
            "signed": True,
        }
    ]

    return {
        "family_id": family_id,
        "template": template,
        "level_order": order_key,
        "query_vector": round_vector(base),
        "records": records,
        "edges": edges,
        "tombstones": tombstones,
        "cross_tenant_record": cross,
    }


def build_corpus() -> dict:
    families = [build_family(family_id, template, order) for family_id, template, order in FAMILY_PLAN]
    return {
        "schema": "wdbx.evaluation.corpus.v1",
        "corpus_id": "synthetic-guild-retrieval-baseline-v1",
        "tenant": TENANT,
        "other_tenant": OTHER_TENANT,
        "embedding_dimension": DIMENSION,
        "candidate_count": 12,
        "contract": {"revision": CONTRACT_REVISION, "digest_hex": CONTRACT_DIGEST_HEX},
        "policy": {"current": CURRENT_POLICY, "stale": STALE_POLICY},
        "query": {
            "policy_version": CURRENT_POLICY,
            "min_evidence_level": QUERY_MIN_EVIDENCE,
            "scope": "member",
        },
        "signer_seed": SIGNER_SEED,
        "families": families,
    }


def main() -> int:
    corpus = build_corpus()
    json.dump(corpus, sys.stdout, indent=2, sort_keys=False)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
