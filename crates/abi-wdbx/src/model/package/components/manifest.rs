//! Architecture schema 1.0.
use super::{Cursor, Result};
use crate::model::{
    contract::{ARCHITECTURE_ID, NUMERICAL_PROFILE_ID, architecture::Architecture},
    error::ErrorKind as E,
};
pub(super) fn parse(r: &mut Cursor<'_, '_>) -> Result<Architecture> {
    let architecture = r.string(4096)?;
    let numerical_profile = r.string(4096)?;
    r.check(
        architecture == ARCHITECTURE_ID && numerical_profile == NUMERICAL_PROFILE_ID,
        E::NonCanonical,
        "architecture-profile",
    )?;
    let purpose = r.e8()?;
    let learning_profile = r.e8()?;
    let operator_set = r.e16()?;
    let required_features = r.u64()?;
    r.check(required_features == 3, E::NonCanonical, "required-features")?;
    let mut dims = [0; 13];
    let mins = [2, 1, 1, 1, 260, 1, 2, 1, 1, 2, 1, 1, 1];
    let maxs = [
        128, 1024, 64, 8, 4096, 4, 128, 128, 4, 128, 1024, 16, 1_000_000,
    ];
    for ((value, min), max) in dims.iter_mut().zip(mins).zip(maxs) {
        *value = r.u32()?;
        r.check(
            (min..=max).contains(value),
            E::CapacityExceeded,
            "architecture-dimension",
        )?;
    }
    let mut resources = [0; 3];
    for (value, max) in resources
        .iter_mut()
        .zip([268_435_456, 536_870_912, 500_000_000])
    {
        *value = r.u64()?;
        r.check(
            (1..=max).contains(value),
            E::CapacityExceeded,
            "architecture-resource",
        )?;
    }
    let spatial_scale = r.f64()?;
    let coordinate_radius = r.f64()?;
    r.check(
        spatial_scale > 0.0
            && coordinate_radius.to_bits() == 4.0f64.to_bits()
            && dims[0] <= dims[9]
            && dims[1] <= dims[10],
        E::NonCanonical,
        "architecture-bounds",
    )?;
    r.end()?;
    let [
        node_count,
        edge_count,
        hidden_width,
        relation_count,
        vocab_count,
        message_steps,
        max_sequence_tokens,
        max_generated_tokens,
        max_batch_examples,
        node_capacity,
        edge_capacity,
        max_incoming_degree,
        max_trainable_scalars,
    ] = dims;
    let [
        max_session_working_bytes,
        max_loaded_and_trainer_bytes,
        max_training_scalar_ops,
    ] = resources;
    Ok(Architecture {
        architecture,
        numerical_profile,
        purpose,
        learning_profile,
        operator_set,
        required_features,
        node_count,
        edge_count,
        hidden_width,
        relation_count,
        vocab_count,
        message_steps,
        max_sequence_tokens,
        max_generated_tokens,
        max_batch_examples,
        node_capacity,
        edge_capacity,
        max_incoming_degree,
        max_trainable_scalars,
        max_session_working_bytes,
        max_loaded_and_trainer_bytes,
        max_training_scalar_ops,
        spatial_scale,
        coordinate_radius,
    })
}
