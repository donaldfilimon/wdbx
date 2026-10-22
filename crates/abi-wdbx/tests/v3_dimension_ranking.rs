//! Dimension-separated ranking over the constitutional query.
//!
//! The canonical contract asks for trust to be eight dimensions and never one
//! score. These tests pin the ranking that answers it: every dimension stays
//! separately inspectable, a caller can order by any one of them, missing
//! evidence never reads as a measured zero, the vector arm's own order
//! survives the merge, and the public ranking surface exposes no number that
//! could stand in for the eight.

#[path = "support/dimension_ranking.rs"]
mod support;

use abi_wdbx::v3::episode::{
    ActorKind, ConstitutionalQuery, Disposition, EpisodeSource, EpisodeStore, EvidenceDimension,
    EvidenceLevel, HoldKind, HoldRequest, KeyResolver, RevocationRequest, SignerKeyId,
};
use abi_wdbx::v3::ranking::{
    DimensionRanking, DimensionSignal, DimensionSignals, ExplicitPolicy, MergedRanking,
    RankedResult, SortDirection, VectorHit, merge_vector_arm, rank_constitutional_query,
};
use std::path::Path;
use support::{
    GUILD, build_fixture, digests_of, expected_lifecycle_signal, label_of, labels_of,
    public_fields, rank_fixture, vector_arm,
};

/// The ranking module's own sources, for the structural claim that no public
/// ranking type carries an opaque scalar.
const RANKING_SOURCES: &[&str] = &[
    include_str!("../src/v3/ranking/mod.rs"),
    include_str!("../src/v3/ranking/ordering.rs"),
    include_str!("../src/v3/ranking/signals.rs"),
    include_str!("../src/v3/ranking/merge.rs"),
];

#[test]
fn v3_dimension_ranking_keeps_eight_dimensions_separately() {
    let fixture = build_fixture();
    let ranking = rank_fixture(&fixture);

    assert_eq!(
        ranking.len(),
        11,
        "every record the query answers is ranked, and the forgotten one is not"
    );

    // Every entry carries the eight signals, one per dimension, and the
    // signals agree with the canonical evidence the query reported.
    for entry in ranking.entries() {
        let signals: Vec<DimensionSignal> = EvidenceDimension::all()
            .iter()
            .map(|dimension| entry.dimension_signals.get(*dimension))
            .collect();
        assert_eq!(signals.len(), 8);
        for dimension in EvidenceDimension::all() {
            let signal = entry.dimension_signals.get(dimension);
            assert!(
                !matches!(signal, DimensionSignal::Present(_)) || !signal.label().is_empty(),
                "{dimension:?} must carry a named value"
            );
        }
        // The lifecycle signal agrees with the canonical lifecycle evidence.
        assert_eq!(
            entry.dimension_signals.lifecycle,
            expected_lifecycle_signal(&entry.result)
        );
    }

    // The signals name the states the constitutional query reported.
    assert_eq!(
        label_of(
            &ranking,
            fixture.digest("signed"),
            EvidenceDimension::Cryptographic
        ),
        "valid"
    );
    assert_eq!(
        label_of(
            &ranking,
            fixture.digest("unsigned"),
            EvidenceDimension::Cryptographic
        ),
        "absent"
    );
    assert_eq!(
        label_of(
            &ranking,
            fixture.digest("contradicted"),
            EvidenceDimension::Outcome
        ),
        "contradicted"
    );
    assert_eq!(
        label_of(
            &ranking,
            fixture.digest("stale"),
            EvidenceDimension::Compatibility
        ),
        "policy-stale"
    );
    assert_eq!(
        label_of(
            &ranking,
            fixture.digest("proposal"),
            EvidenceDimension::Source
        ),
        "proposal"
    );
    assert_eq!(
        label_of(
            &ranking,
            fixture.digest("superseded"),
            EvidenceDimension::Lifecycle
        ),
        "superseded"
    );
    assert_eq!(
        label_of(
            &ranking,
            fixture.digest("specified"),
            EvidenceDimension::Confidence
        ),
        "C0"
    );
    assert_eq!(
        label_of(
            &ranking,
            fixture.digest("tombstone"),
            EvidenceDimension::Confidence
        ),
        "not-applicable"
    );
}

#[test]
fn v3_dimension_ranking_orders_by_any_chosen_dimension() {
    let fixture = build_fixture();
    let ranking = rank_fixture(&fixture);

    for dimension in EvidenceDimension::all() {
        let ordered = ranking.order_by_dimension(dimension, SortDirection::BestFirst);
        assert_eq!(ordered.len(), ranking.len());
        assert_eq!(
            digests_of(&ordered)
                .iter()
                .collect::<std::collections::BTreeSet<_>>(),
            digests_of(&ranking)
                .iter()
                .collect::<std::collections::BTreeSet<_>>(),
            "{dimension:?} must not add or drop a record"
        );
        // Ordering by the same dimension twice gives the same order.
        let again = ordered.order_by_dimension(dimension, SortDirection::BestFirst);
        assert_eq!(digests_of(&again), digests_of(&ordered));
        // The worst-first order puts the weakest value of that dimension
        // first. It is not the mirror image of the best-first order: records
        // that tie keep the order they already had, in both directions.
        let worst = ranking.order_by_dimension(dimension, SortDirection::WorstFirst);
        assert_eq!(worst.len(), ranking.len());
    }

    // The documented orderings hold where the ledger holds evidence.
    let by_crypto =
        ranking.order_by_dimension(EvidenceDimension::Cryptographic, SortDirection::BestFirst);
    let crypto = labels_of(&by_crypto, EvidenceDimension::Cryptographic);
    assert_eq!(
        crypto.first(),
        Some(&"valid"),
        "a verified signature outranks a record that carries none"
    );
    assert_eq!(crypto.last(), Some(&"absent"));

    let by_lifecycle =
        ranking.order_by_dimension(EvidenceDimension::Lifecycle, SortDirection::BestFirst);
    let lifecycle = labels_of(&by_lifecycle, EvidenceDimension::Lifecycle);
    assert_eq!(lifecycle.first(), Some(&"live"));
    assert!(
        lifecycle.iter().position(|label| *label == "superseded")
            < lifecycle.iter().position(|label| *label == "quarantined"),
        "a superseded record outranks a quarantined one"
    );

    let by_confidence =
        ranking.order_by_dimension(EvidenceDimension::Confidence, SortDirection::BestFirst);
    let confidence = labels_of(&by_confidence, EvidenceDimension::Confidence);
    assert_eq!(confidence.last(), Some(&"not-applicable"));
    assert!(
        confidence.iter().position(|label| *label == "C1")
            < confidence.iter().position(|label| *label == "C0"),
        "a more exercised maturity outranks a specification-only one"
    );

    let by_outcome =
        ranking.order_by_dimension(EvidenceDimension::Outcome, SortDirection::BestFirst);
    let outcome = labels_of(&by_outcome, EvidenceDimension::Outcome);
    assert!(
        outcome.iter().position(|label| *label == "absent")
            < outcome.iter().position(|label| *label == "contradicted"),
        "an unobserved outcome outranks a contradicted one"
    );

    let by_compatibility =
        ranking.order_by_dimension(EvidenceDimension::Compatibility, SortDirection::BestFirst);
    let compatibility = labels_of(&by_compatibility, EvidenceDimension::Compatibility);
    assert_eq!(compatibility.first(), Some(&"compatible"));
    assert_eq!(compatibility.last(), Some(&"policy-stale"));

    // The dimensions the ledger holds no evidence for order nothing, and say
    // so: they are unknown for every record.
    for dimension in [EvidenceDimension::Calibration, EvidenceDimension::Regime] {
        let ordered = ranking.order_by_dimension(dimension, SortDirection::BestFirst);
        assert_eq!(digests_of(&ordered), digests_of(&ranking));
        assert!(
            labels_of(&ordered, dimension)
                .iter()
                .all(|label| *label == "unknown")
        );
    }

    // The worst-first order puts the weakest value first.
    let worst_crypto =
        ranking.order_by_dimension(EvidenceDimension::Cryptographic, SortDirection::WorstFirst);
    let crypto = labels_of(&worst_crypto, EvidenceDimension::Cryptographic);
    assert_eq!(crypto.first(), Some(&"absent"));
    assert_eq!(crypto.last(), Some(&"valid"));

    let worst_lifecycle =
        ranking.order_by_dimension(EvidenceDimension::Lifecycle, SortDirection::WorstFirst);
    let lifecycle = labels_of(&worst_lifecycle, EvidenceDimension::Lifecycle);
    assert_eq!(lifecycle.first(), Some(&"quarantined"));
    assert_eq!(lifecycle.last(), Some(&"live"));

    let worst_outcome =
        ranking.order_by_dimension(EvidenceDimension::Outcome, SortDirection::WorstFirst);
    let outcome = labels_of(&worst_outcome, EvidenceDimension::Outcome);
    assert_eq!(outcome.first(), Some(&"contradicted"));
    assert_eq!(outcome.last(), Some(&"absent"));
}

#[test]
fn v3_dimension_ranking_exposes_no_opaque_scalar() {
    // The structural claim, checked at the source: no public ranking type
    // carries a number that could stand in for the eight dimensions.
    for source in RANKING_SOURCES {
        for forbidden in ["fn score(", "fn combined(", "fn trust(", "fn rank("] {
            assert!(
                !source.contains(forbidden),
                "{forbidden} appears in the ranking module"
            );
        }
        for (name, ty) in public_fields(source) {
            for forbidden in ["score", "rank", "trust", "combined", "weight", "factor"] {
                assert_ne!(
                    name, forbidden,
                    "the ranking exposes a field named {forbidden}"
                );
            }
            if ty.contains("f32") || ty.contains("f64") {
                assert!(
                    name.contains("similarity"),
                    "{name} is a floating-point field that is not a similarity"
                );
                for forbidden in ["score", "trust", "rank", "combined", "weight"] {
                    assert!(!name.contains(forbidden), "{name} reads like a {forbidden}");
                }
            }
        }
    }

    // The only floating-point values the module exposes are the vector arm's
    // own similarities.
    let mut float_fields = Vec::new();
    for source in RANKING_SOURCES {
        for (name, ty) in public_fields(source) {
            if ty.contains("f32") || ty.contains("f64") {
                float_fields.push(name);
            }
        }
    }
    float_fields.sort_unstable();
    assert_eq!(
        float_fields,
        vec!["similarity".to_string(), "vector_similarity".to_string()],
        "the ranking must expose no number but the vector arm's own similarity"
    );

    // The eight dimensions are eight separate fields, not a collection that
    // could be collapsed.
    let mut dimension_fields = 0_usize;
    for source in RANKING_SOURCES {
        for (_name, ty) in public_fields(source) {
            if ty == "DimensionSignal" {
                dimension_fields += 1;
            }
        }
    }
    assert_eq!(
        dimension_fields, 8,
        "the eight dimensions stay eight fields"
    );

    // The ranking types are finite and hold no numeric verdict of their own.
    assert!(std::mem::size_of::<DimensionSignals>() > 0);
    assert!(std::mem::size_of::<DimensionSignals>() < 1_024);
    assert!(std::mem::size_of::<RankedResult>() > 0);
}

#[test]
fn v3_dimension_ranking_keeps_missing_and_zero_apart() {
    let fixture = build_fixture();
    let ranking = rank_fixture(&fixture);

    // The ledger holds no calibration record and no task-regime field: those
    // dimensions were never measured, and they say so.
    for entry in ranking.entries() {
        assert_eq!(
            entry.dimension_signals.calibration,
            DimensionSignal::Unknown
        );
        assert_eq!(entry.dimension_signals.regime, DimensionSignal::Unknown);
    }

    // The outcome dimension *was* measured, and it reads zero observations.
    let unobserved = ranking
        .result(fixture.digest("signed"))
        .expect("the record is ranked");
    assert_eq!(
        unobserved.dimension_signals.outcome,
        DimensionSignal::Absent
    );

    // A gap and a measured zero are different states, and they order
    // differently.
    assert_ne!(DimensionSignal::Unknown, DimensionSignal::Absent);
    assert_ne!(
        DimensionSignal::Unknown.label(),
        DimensionSignal::Absent.label()
    );
    assert!(!DimensionSignal::Unknown.is_measured());
    assert!(DimensionSignal::Absent.is_measured());

    // The missing dimensions sort after the measured ones: an unmeasured
    // dimension cannot be presented as a favourable one.
    let by_outcome =
        ranking.order_by_dimension(EvidenceDimension::Outcome, SortDirection::BestFirst);
    let outcome = labels_of(&by_outcome, EvidenceDimension::Outcome);
    assert!(outcome.contains(&"absent"));
    assert!(!outcome.contains(&"unknown"));
}

#[test]
fn v3_dimension_ranking_is_deterministic() {
    let first = build_fixture();
    let first_ranking = rank_fixture(&first);
    let second = build_fixture();
    let second_ranking = rank_fixture(&second);

    // Two independently built ledgers hold the same records, so they rank the
    // same way, in the same order.
    assert_eq!(digests_of(&first_ranking), digests_of(&second_ranking));

    // Ranking the same ledger twice gives the identical order.
    let again = rank_fixture(&first);
    assert_eq!(digests_of(&first_ranking), digests_of(&again));

    // Ordering by a dimension is deterministic too.
    for dimension in EvidenceDimension::all() {
        let one = first_ranking.order_by_dimension(dimension, SortDirection::BestFirst);
        let two = second_ranking.order_by_dimension(dimension, SortDirection::BestFirst);
        assert_eq!(digests_of(&one), digests_of(&two));
    }
}

#[test]
fn v3_dimension_ranking_merges_the_vector_arm_without_collapsing() {
    let fixture = build_fixture();
    let ranking = rank_fixture(&fixture);
    let (arm, _) = vector_arm(&fixture);

    let merged: MergedRanking = merge_vector_arm(&ranking, &arm);

    let merged_order = digests_of(&merged.ranking);
    let constitutional_order = digests_of(&ranking);
    let arm_order: Vec<[u8; 32]> = arm.iter().map(|hit| hit.episode_digest).collect();

    // Every record the constitutional arm answered is in the merged list,
    // exactly once.
    assert_eq!(merged_order.len(), constitutional_order.len());
    let mut unique = merged_order.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        merged_order.len(),
        "no record is listed twice"
    );
    for digest in &constitutional_order {
        assert!(merged_order.contains(digest));
    }

    // The constitutional arm's own order survives the merge for the records
    // only the constitutional arm holds.
    let constitutional_only: Vec<[u8; 32]> = constitutional_order
        .iter()
        .filter(|digest| !arm_order.contains(digest))
        .copied()
        .collect();
    assert_eq!(
        constitutional_only.len(),
        2,
        "the stale and the tombstoned record hold no vector"
    );
    let projected: Vec<[u8; 32]> = merged_order
        .iter()
        .filter(|digest| constitutional_only.contains(digest))
        .copied()
        .collect();
    assert_eq!(
        projected, constitutional_only,
        "the constitutional arm's own order must survive the merge"
    );

    // The vector arm's own order is never lost: every record the arm holds
    // keeps the arm's own position, so sorting the merged ranking by that
    // position reproduces the vector arm's order exactly.
    let mut by_vector_rank: Vec<([u8; 32], Option<usize>)> = merged
        .ranking
        .entries()
        .iter()
        .map(|entry| (entry.result.episode_digest, entry.vector_rank))
        .collect();
    by_vector_rank.sort_by_key(|(_, rank)| rank.unwrap_or(usize::MAX));
    let recovered: Vec<[u8; 32]> = by_vector_rank
        .iter()
        .filter(|(_, rank)| rank.is_some())
        .map(|(digest, _)| *digest)
        .collect();
    let expected: Vec<[u8; 32]> = arm_order
        .iter()
        .filter(|digest| constitutional_order.contains(digest))
        .copied()
        .collect();
    assert_eq!(
        recovered, expected,
        "the vector arm's order must survive the merge"
    );

    // A vector hit the constitutional query did not answer is never invented
    // into the ranking.
    let vector_only: Vec<[u8; 32]> = arm_order
        .iter()
        .filter(|digest| !constitutional_order.contains(digest))
        .copied()
        .collect();
    assert_eq!(
        vector_only.len(),
        1,
        "the forgotten record is the only extra hit"
    );
    let merged_vector_only: Vec<[u8; 32]> = merged_order
        .iter()
        .filter(|digest| vector_only.contains(digest))
        .copied()
        .collect();
    assert!(
        merged_vector_only.is_empty(),
        "a hit the constitutional query did not answer is never invented into the ranking"
    );

    // Every record that the vector arm holds shows the vector arm's own
    // position and similarity beside its dimension signals.
    assert_vector_arm_positions(&merged, &ranking, &arm);

    // A vector hit the constitutional query did not answer is counted, never
    // invented into the ranking.
    assert_eq!(
        merged.vector_hits_without_constitutional_result, 1,
        "the forgotten record is in the vector arm but not in the answer"
    );
    let forged = [7_u8; 32];
    let mut extended = arm.clone();
    extended.push(VectorHit {
        episode_digest: forged,
        similarity: 1.0,
    });
    let merged = merge_vector_arm(&ranking, &extended);
    assert_eq!(merged.vector_hits_without_constitutional_result, 2);
    assert!(!digests_of(&merged.ranking).contains(&forged));
}

/// Every merged entry carries the vector arm's own position and similarity,
/// and the dimension signals travel through the merge untouched.
fn assert_vector_arm_positions(
    merged: &MergedRanking,
    ranking: &DimensionRanking,
    arm: &[VectorHit],
) {
    for entry in merged.ranking.entries() {
        let position = arm
            .iter()
            .position(|hit| hit.episode_digest == entry.result.episode_digest);
        if let Some(index) = position {
            assert_eq!(entry.vector_rank, Some(index + 1));
            assert_eq!(entry.vector_similarity, Some(arm[index].similarity));
        } else {
            assert_eq!(entry.vector_rank, None);
            assert_eq!(entry.vector_similarity, None);
        }
        assert_eq!(
            entry.dimension_signals,
            ranking
                .result(entry.result.episode_digest)
                .expect("the record is ranked")
                .dimension_signals
        );
    }
}

#[test]
fn v3_dimension_ranking_answers_end_to_end_from_a_real_store() {
    let fixture = build_fixture();
    let store = fixture.store();
    let query = ConstitutionalQuery::new(GUILD, 64);
    let ranking = rank_constitutional_query(&store, &query).expect("the query answers");

    // The ranking agrees with the canonical query it was built from.
    let canonical = store.query(&query).expect("the canonical query answers");
    assert_eq!(ranking.len(), canonical.results.len());
    for result in &canonical.results {
        let ranked = ranking
            .result(result.episode_digest)
            .expect("every canonical result is ranked");
        assert_eq!(ranked.result.disposition, result.disposition);
        assert_eq!(ranked.result.dimensions, result.dimensions);
        assert_eq!(ranked.result.explanation, result.explanation);
    }

    // The dispositions the constitutional query reported are all present, so
    // the ranking hides nothing.
    let counts = ranking.disposition_counts();
    assert_eq!(counts.get(&Disposition::Accepted), Some(&4));
    assert_eq!(counts.get(&Disposition::Uncertain), Some(&4));
    assert_eq!(counts.get(&Disposition::Contradicted), Some(&2));
    assert_eq!(counts.get(&Disposition::Quarantined), Some(&1));

    // The explicit policy order is an ordering the caller asked for by name.
    let policy = ExplicitPolicy::EvidenceFirst;
    let ordered = ranking.explicit_policy_order(&policy);
    assert_eq!(ordered.len(), ranking.len());
    assert_eq!(
        digests_of(&ordered)
            .iter()
            .collect::<std::collections::BTreeSet<_>>(),
        digests_of(&ranking)
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
    );
    let criteria = policy.criteria();
    assert_eq!(criteria[0].0, EvidenceDimension::Cryptographic);
    let by_crypto =
        ranking.order_by_dimension(EvidenceDimension::Cryptographic, SortDirection::BestFirst);
    assert_eq!(
        digests_of(&ordered)[0],
        digests_of(&by_crypto)[0],
        "the policy starts with the cryptographic dimension"
    );

    // A caller-chosen policy is the same machinery, applied to the criteria
    // the caller names: the first criterion decides the classes, the second
    // breaks the ties the first one leaves.
    let chosen = ExplicitPolicy::Criteria(vec![
        (EvidenceDimension::Lifecycle, SortDirection::BestFirst),
        (EvidenceDimension::Cryptographic, SortDirection::BestFirst),
    ]);
    let ordered = ranking.explicit_policy_order(&chosen);
    assert_eq!(ordered.len(), ranking.len());
    let first = ordered.entries().first().expect("the ranking is not empty");
    assert_eq!(
        first.dimension_signals.lifecycle,
        DimensionSignal::Present("live")
    );
    assert_eq!(
        first.dimension_signals.cryptographic,
        DimensionSignal::Present("valid")
    );
    let by_lifecycle =
        ranking.order_by_dimension(EvidenceDimension::Lifecycle, SortDirection::BestFirst);
    assert_eq!(
        labels_of(&by_lifecycle, EvidenceDimension::Lifecycle).first(),
        Some(&"live")
    );
}

#[test]
fn v3_dimension_ranking_the_lifecycle_signal_agrees_with_the_disposition() {
    // A hold and a revocation are governance decisions about a candidate.
    // The canonical query reports them, and the ranking must not contradict
    // the disposition it reported.
    let scratch = support::Scratch::new();
    let (held, revoked);
    {
        let mut store =
            EpisodeStore::open(scratch.path(), support::policy(support::CURRENT_POLICY))
                .expect("open");
        held = support::append(
            &mut store,
            support::write(
                "req_held",
                "op_held",
                support::CURRENT_POLICY,
                EvidenceLevel::C1,
                EpisodeSource::DiscordGuild,
                support::candidate(21),
            ),
        );
        revoked = support::append(
            &mut store,
            support::write(
                "req_revoked",
                "op_revoked",
                support::CURRENT_POLICY,
                EvidenceLevel::C1,
                EpisodeSource::DiscordGuild,
                support::candidate(22),
            ),
        );
        store
            .commit_hold(&HoldRequest {
                guild_ref: GUILD.into(),
                target_digest: held,
                kind: HoldKind::Legal,
                authority: support::actor("guild_owner", ActorKind::GuildOwner),
            })
            .expect("the hold commits");
        store
            .commit_revocation(&RevocationRequest {
                guild_ref: GUILD.into(),
                target_digest: revoked,
                authority: support::actor("guild_owner", ActorKind::GuildOwner),
            })
            .expect("the revocation commits");
    }

    let store =
        EpisodeStore::open(scratch.path(), support::policy(support::CURRENT_POLICY)).expect("open");
    let ranking = rank_constitutional_query(&store, &ConstitutionalQuery::new(GUILD, 64))
        .expect("the query answers");

    let held_entry = ranking.result(held).expect("the held record is ranked");
    assert_eq!(held_entry.result.disposition, Disposition::Held);
    assert_eq!(
        held_entry.dimension_signals.lifecycle,
        DimensionSignal::Present("held"),
        "a held record never reads as live"
    );

    let revoked_entry = ranking
        .result(revoked)
        .expect("the revoked record is ranked");
    assert_eq!(revoked_entry.result.disposition, Disposition::Revoked);
    assert_eq!(
        revoked_entry.dimension_signals.lifecycle,
        DimensionSignal::Present("revoked"),
        "a revoked record never reads as live"
    );

    // The ordering puts them where the canonical precedence puts them: a
    // hold is a live freeze, a revocation is an authority's withdrawal.
    let by_lifecycle =
        ranking.order_by_dimension(EvidenceDimension::Lifecycle, SortDirection::BestFirst);
    let labels = labels_of(&by_lifecycle, EvidenceDimension::Lifecycle);
    assert!(
        labels.iter().position(|label| *label == "held")
            < labels.iter().position(|label| *label == "revoked"),
        "a held record outranks a revoked one: {labels:?}"
    );
    let worst = ranking.order_by_dimension(EvidenceDimension::Lifecycle, SortDirection::WorstFirst);
    let worst_labels = labels_of(&worst, EvidenceDimension::Lifecycle);
    assert_eq!(worst_labels.first(), Some(&"revoked"));
    assert_eq!(worst_labels.last(), Some(&"held"));
}

#[test]
fn v3_dimension_ranking_is_compatible_with_the_frozen_baseline_harness() {
    use abi_wdbx::evaluation::{Corpus, constitution, ledger};

    let corpus_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/retrieval-baseline/corpus-v1.json");
    let corpus = Corpus::load(&corpus_path).expect("the frozen corpus loads");
    let built = ledger::build(&corpus).expect("the ledger builds");
    let readings = constitution::read(&built.store, corpus.tenant(), &built.signer)
        .expect("the constitutional query answers");

    // The same key resolver the harness uses, so the ranking reads the same
    // signatures the harness read.
    let key = built.signer.verifying_key();
    let key_id = built.signer.key_id().clone();
    let resolver =
        KeyResolver::new(move |requested: &SignerKeyId| (requested == &key_id).then_some(key));
    let ranking = rank_constitutional_query(
        &built.store,
        &ConstitutionalQuery::new(corpus.tenant(), 2_048).with_key_resolver(resolver),
    )
    .expect("the ranking answers");
    assert!(!ranking.is_empty());

    // The ranking reports exactly what the canonical query reported.
    let mut checked = 0_usize;
    for (digest, result) in &readings {
        let Some(ranked) = ranking.result(*digest) else {
            continue;
        };
        assert_eq!(ranked.result.disposition, result.disposition);
        assert_eq!(ranked.result.dimensions, result.dimensions);
        checked += 1;
    }
    assert!(checked >= 100, "the corpus must be large enough to matter");

    // The per-dimension signals agree with the harness's own rendering of the
    // canonical dimensions: the harness reports the outcome and the lifecycle
    // in words, and the ranking reports the same words.
    for entry in ranking.entries() {
        let reports = constitution::dimension_reports(&entry.result);
        assert_eq!(reports.len(), 8);
        for report in &reports {
            let dimension = EvidenceDimension::all()
                .iter()
                .find(|candidate| candidate.label() == report.dimension)
                .copied()
                .expect("the report names a canonical dimension");
            let signal = entry.dimension_signals.get(dimension);
            // A forgetting tombstone carries no remembered claim, so the
            // confidence dimension does not apply to it. The harness renders
            // the committed maturity anyway; the ranking says the dimension
            // does not apply, and the canonical maturity stays readable on
            // the result.
            if dimension == EvidenceDimension::Confidence
                && entry.result.dimensions.lifecycle.tombstoned
            {
                assert_eq!(signal, DimensionSignal::NotApplicable);
                continue;
            }
            match report.value.as_str() {
                "validated" => assert_eq!(signal, DimensionSignal::Present("validated")),
                "contradicted" => assert_eq!(signal, DimensionSignal::Present("contradicted")),
                "unobserved" | "unsigned" => assert_eq!(signal, DimensionSignal::Absent),
                "live" => assert_eq!(signal, DimensionSignal::Present("live")),
                "superseded" => assert_eq!(signal, DimensionSignal::Present("superseded")),
                "quarantined" => assert_eq!(signal, DimensionSignal::Present("quarantined")),
                "held" => assert_eq!(signal, DimensionSignal::Present("held")),
                "revoked" => assert_eq!(signal, DimensionSignal::Present("revoked")),
                "forgotten" => assert_eq!(signal, DimensionSignal::Present("forgotten")),
                "tombstone" => assert_eq!(signal, DimensionSignal::Present("tombstone")),
                "valid" => assert_eq!(signal, DimensionSignal::Present("valid")),
                "invalid" => assert_eq!(signal, DimensionSignal::Present("invalid")),
                "unverifiable" | "unknown" => assert_eq!(signal, DimensionSignal::Unknown),
                "compatible" => assert_eq!(signal, DimensionSignal::Present("compatible")),
                "contractstale" => assert_eq!(signal, DimensionSignal::Present("contract-stale")),
                "policystale" => assert_eq!(signal, DimensionSignal::Present("policy-stale")),
                "proposal" => assert_eq!(signal, DimensionSignal::Present("proposal")),
                "discordguild" => assert_eq!(signal, DimensionSignal::Present("discord_guild")),
                "discordvoice" => assert_eq!(signal, DimensionSignal::Present("discord_voice")),
                "localruntime" => assert_eq!(signal, DimensionSignal::Present("local_runtime")),
                "c0" => assert_eq!(signal, DimensionSignal::Present("C0")),
                "c1" => assert_eq!(signal, DimensionSignal::Present("C1")),
                "c2" => assert_eq!(signal, DimensionSignal::Present("C2")),
                "c3" => assert_eq!(signal, DimensionSignal::Present("C3")),
                "c4" => assert_eq!(signal, DimensionSignal::Present("C4")),
                "c5" => assert_eq!(signal, DimensionSignal::Present("C5")),
                "c6" => assert_eq!(signal, DimensionSignal::Present("C6")),
                "c7" => assert_eq!(signal, DimensionSignal::Present("C7")),
                other => panic!("the harness reported an unknown value: {other}"),
            }
        }
    }
}
