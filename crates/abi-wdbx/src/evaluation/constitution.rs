//! The constitutional side: the canonical constitutional query over the
//! episode ledger.
//!
//! The constitutional reading comes from
//! [`EpisodeStore::query`](crate::v3::episode::EpisodeStore::query), the
//! canonical read model of the v3 episode ledger. The harness owns no second
//! constitutional implementation: it asks the query once per run, indexes the
//! answers by episode digest, and reads the disposition, the eight
//! dimensions, the lineage and the explanation out of them.
//!
//! There is exactly one place where the harness restates the constitutional
//! rules: [`disposition_with`], the ablation view. The canonical query has no
//! way to suppress one dimension, so an ablation has to recompute a
//! disposition with a dimension ignored. The harness therefore checks, on
//! every record of every family, that the ablation view with nothing
//! suppressed reproduces the canonical disposition exactly. A change in the
//! canonical rules shows up as a failed check, never as a quietly different
//! number.
//!
//! ## What the ledger does not hold
//!
//! The ledger holds no calibration record, no task-regime field and no
//! uncertainty interval, so those three dimensions read `Unknown` for every
//! record. The harness reports that as a finding, not as a measurement: an
//! unknown dimension carries no benefit, and the ablation says so.

use crate::v3::episode::{
    CompatibilityEvidence, ConstitutionalQuery, ConstitutionalResult, Disposition, EpisodeSigner,
    EpisodeStore, EpisodeStoreError, EvidenceDimension, EvidenceDimensions, KeyResolver,
    OutcomeEvidence, QueryConstraints, RevocationEvidence, SignatureEvidence, SignerKeyId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The greatest number of results the canonical query will return.
const MAX_RESULTS: usize = 2_048;

/// The retrieval tier a disposition occupies in the constitutional arm's
/// ranking.
///
/// Lower is better. The tiers are named states, not a product of factors:
/// the constitution forbids collapsing the eight dimensions into one
/// number, and this ordering does not do that either. A rejected record has
/// no place in the ranking at all; its tier is defined so that every
/// disposition has one.
#[must_use]
pub const fn tier(disposition: Disposition) -> u8 {
    match disposition {
        Disposition::Accepted => 0,
        Disposition::Uncertain
        | Disposition::Contradicted
        | Disposition::Quarantined
        | Disposition::Held
        | Disposition::Revoked => 1,
        Disposition::Unknown => 2,
        Disposition::Rejected => 3,
    }
}

/// Whether a disposition presents the record as usable.
///
/// Only `Accepted` does. The canonical query accepts a record on its
/// commitment; an uncertain, contradicted, quarantined, held, revoked or
/// unknown record is retained and visible but is not usable as it stands,
/// and a rejected one is not offered at all.
#[must_use]
pub const fn presents_as_usable(disposition: Disposition) -> bool {
    matches!(disposition, Disposition::Accepted)
}

/// The constitutional reading of every record of one guild, by digest.
///
/// # Errors
///
/// Returns [`EpisodeStoreError`] when the store refuses the query.
pub fn read(
    store: &EpisodeStore,
    guild_ref: &str,
    signer: &EpisodeSigner,
) -> Result<BTreeMap<[u8; 32], ConstitutionalResult>, EpisodeStoreError> {
    let key = signer.verifying_key();
    let key_id = signer.key_id().clone();
    let resolver =
        KeyResolver::new(move |requested: &SignerKeyId| (requested == &key_id).then_some(key));
    let mut query = ConstitutionalQuery::new(guild_ref, MAX_RESULTS).with_key_resolver(resolver);
    // The query lists everything the ledger holds, so that nothing is
    // hidden: a contradicted, quarantined, forgotten, held, or revoked record
    // is answered with the disposition that says why, not with silence.
    query.constraints = QueryConstraints {
        include_contradicted: true,
        include_quarantined: true,
        include_forgotten: true,
        include_held: true,
        include_revoked: true,
        member_scope: None,
        min_evidence_level: None,
    };
    let results = store.query(&query)?;
    Ok(results
        .results
        .into_iter()
        .map(|result| (result.episode_digest, result))
        .collect())
}

/// The disposition the canonical rules imply, with the listed dimensions
/// ignored.
///
/// This is the harness's ablation view of the canonical disposition rules.
/// It is not a second canonical implementation: [`read`] is where the
/// canonical answer comes from, and the harness checks this view against the
/// canonical disposition on every record before it reports any ablation.
#[must_use]
pub fn disposition_with(
    dimensions: &EvidenceDimensions,
    dropped: &[EvidenceDimension],
) -> Disposition {
    let skip = |dimension: EvidenceDimension| dropped.contains(&dimension);
    if !skip(EvidenceDimension::Cryptographic)
        && dimensions.cryptographic.signature == SignatureEvidence::Invalid
    {
        return Disposition::Rejected;
    }
    if !skip(EvidenceDimension::Lifecycle) && dimensions.lifecycle.forgotten {
        return Disposition::Rejected;
    }
    if !skip(EvidenceDimension::Lifecycle)
        && dimensions.lifecycle.revocation == RevocationEvidence::Revoked
    {
        return Disposition::Revoked;
    }
    if !skip(EvidenceDimension::Lifecycle) && dimensions.lifecycle.open_quarantine.is_some() {
        return Disposition::Quarantined;
    }
    if !skip(EvidenceDimension::Outcome) && dimensions.outcome == OutcomeEvidence::Contradicted {
        return Disposition::Contradicted;
    }
    if !skip(EvidenceDimension::Lifecycle) && dimensions.lifecycle.hold.is_held() {
        return Disposition::Held;
    }
    if !skip(EvidenceDimension::Lifecycle) && dimensions.lifecycle.superseded_by.is_some() {
        return Disposition::Uncertain;
    }
    if !skip(EvidenceDimension::Compatibility)
        && dimensions.compatibility != CompatibilityEvidence::Compatible
    {
        return Disposition::Uncertain;
    }
    if !skip(EvidenceDimension::Confidence) && dimensions.confidence.maturity == C0 {
        return Disposition::Uncertain;
    }
    if !skip(EvidenceDimension::Cryptographic)
        && dimensions.cryptographic.signature == SignatureEvidence::Unverifiable
    {
        return Disposition::Unknown;
    }
    Disposition::Accepted
}

/// The committed evidence maturity that means "specified only".
const C0: crate::v3::episode::EvidenceLevel = crate::v3::episode::EvidenceLevel::C0;

/// One dimension, its value and the canonical reason for it.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DimensionReport {
    /// The dimension's canonical label.
    pub dimension: String,
    /// The value of the dimension.
    pub value: String,
    /// The canonical reason the dimension reads that way.
    pub note: String,
}

/// Render one canonical result as eight separate dimension reports.
#[must_use]
pub fn dimension_reports(result: &ConstitutionalResult) -> Vec<DimensionReport> {
    let dimensions = &result.dimensions;
    let reason = |dimension: EvidenceDimension| {
        result
            .explanation
            .entries
            .iter()
            .find(|(entry, _)| *entry == dimension)
            .map_or_else(
                || "no reason recorded".to_string(),
                |(_, reason)| format!("{reason:?}"),
            )
    };
    let lifecycle = if dimensions.lifecycle.forgotten {
        "forgotten"
    } else if dimensions.lifecycle.revocation == RevocationEvidence::Revoked {
        "revoked"
    } else if dimensions.lifecycle.open_quarantine.is_some() {
        "quarantined"
    } else if dimensions.lifecycle.hold.is_held() {
        "held"
    } else if dimensions.lifecycle.superseded_by.is_some() {
        "superseded"
    } else if dimensions.lifecycle.tombstoned {
        "tombstone"
    } else {
        "live"
    };
    vec![
        DimensionReport {
            dimension: EvidenceDimension::Cryptographic.label().to_string(),
            value: format!("{:?}", dimensions.cryptographic.signature).to_lowercase(),
            note: reason(EvidenceDimension::Cryptographic),
        },
        DimensionReport {
            dimension: EvidenceDimension::Source.label().to_string(),
            value: format!("{:?}", dimensions.source.source_type).to_lowercase(),
            note: reason(EvidenceDimension::Source),
        },
        DimensionReport {
            dimension: EvidenceDimension::Calibration.label().to_string(),
            value: "unknown".to_string(),
            note: reason(EvidenceDimension::Calibration),
        },
        DimensionReport {
            dimension: EvidenceDimension::Outcome.label().to_string(),
            value: format!("{:?}", dimensions.outcome).to_lowercase(),
            note: reason(EvidenceDimension::Outcome),
        },
        DimensionReport {
            dimension: EvidenceDimension::Compatibility.label().to_string(),
            value: format!("{:?}", dimensions.compatibility).to_lowercase(),
            note: reason(EvidenceDimension::Compatibility),
        },
        DimensionReport {
            dimension: EvidenceDimension::Regime.label().to_string(),
            value: "unknown".to_string(),
            note: reason(EvidenceDimension::Regime),
        },
        DimensionReport {
            dimension: EvidenceDimension::Lifecycle.label().to_string(),
            value: lifecycle.to_string(),
            note: reason(EvidenceDimension::Lifecycle),
        },
        DimensionReport {
            dimension: EvidenceDimension::Confidence.label().to_string(),
            value: format!("{:?}", dimensions.confidence.maturity).to_lowercase(),
            note: reason(EvidenceDimension::Confidence),
        },
    ]
}

/// Render one canonical result's explanation as one line per dimension.
#[must_use]
pub fn explanation(result: &ConstitutionalResult) -> Vec<String> {
    result
        .explanation
        .entries
        .iter()
        .map(|(dimension, reason)| format!("{}: {reason:?}", dimension.label()))
        .collect()
}
