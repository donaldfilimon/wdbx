//! Objective signatures; manifests are not fetched by a decoder.
use super::{Cursor, Result};
use crate::model::{
    contract::training::{
        GraphCandidatePolicy, GraphReduction, Objective, ObjectiveProfile, SequencePolicy,
        TextMaskProfile, TextReduction,
    },
    error::ErrorKind as E,
};
pub(super) fn parse(r: &mut Cursor<'_, '_>) -> Result<Objective> {
    // Schema-specific unsupported profiles retain the published error family.
    let profiles = [r.u16()?, r.u16()?, r.u16()?, r.u16()?];
    r.check(
        profiles == [1; 4],
        E::InvalidObjective,
        "objective-profiles",
    )?;
    let text_weight = r.f64()?;
    let graph_weight = r.f64()?;
    let coordinate_l2_weight = r.f64()?;
    r.check(
        text_weight.to_bits() == 1.0f64.to_bits()
            && graph_weight >= 0.0
            && coordinate_l2_weight >= 0.0,
        E::InvalidObjective,
        "objective-weights",
    )?;
    let a = r.u16()?;
    let b = r.u16()?;
    r.check(a == 1 && b == 1, E::InvalidObjective, "objective-policies")?;
    let max_graph_candidates = r.u32()?;
    r.check(
        (1..=128).contains(&max_graph_candidates),
        E::InvalidObjective,
        "candidate-limit",
    )?;
    let dataset_manifest_digest = r.digest()?;
    let split_manifest_digest = r.digest()?;
    let preparation_digest = r.digest()?;
    r.end()?;
    Ok(Objective {
        objective_profile: ObjectiveProfile::NextTokenGraphCeCoordinateL2V1,
        text_mask_profile: TextMaskProfile::DisallowPadBosUnk,
        text_reduction: TextReduction::ValidTargetTokenMean,
        graph_reduction: GraphReduction::ExampleMean,
        text_weight,
        graph_weight,
        coordinate_l2_weight,
        sequence_policy: SequencePolicy::BosContentEos,
        graph_candidate_policy: GraphCandidatePolicy::ExplicitAuthorizedMemoryIds,
        max_graph_candidates,
        dataset_manifest_digest,
        split_manifest_digest,
        preparation_digest,
    })
}
