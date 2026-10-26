//! Component 1: fixed compatibility vocabulary and bounded declarations.

/// Purpose never implies local permission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Purpose {
    /// Optimizer and RNG components explicitly absent.
    InferenceOnly = 0,
    /// Complete step-boundary optimizer and RNG required.
    ResumableTraining = 1,
}
impl TryFrom<u8> for Purpose {
    type Error = super::super::error::ModelError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::InferenceOnly),
            1 => Ok(Self::ResumableTraining),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "Purpose",
                None,
                None,
            )),
        }
    }
}

/// Explicit amendment; historical manual profiles are unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum LearningProfile {
    /// Trusted caller must authorize explicit training.
    OfflineExplicit = 0,
    /// Requires enabled external online policy.
    OnlineV1 = 1,
}
impl TryFrom<u8> for LearningProfile {
    type Error = super::super::error::ModelError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::OfflineExplicit),
            1 => Ok(Self::OnlineV1),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "LearningProfile",
                None,
                None,
            )),
        }
    }
}

/// Only compiled operators are admitted; never serialized code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum OperatorSet {
    /// Gather, matvec, add, tanh, squared-distance, segment-softmax, scatter-sum, masked cross-entropy and `AdamW`.
    SpatialRecurrentV1 = 1,
}
impl TryFrom<u16> for OperatorSet {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::SpatialRecurrentV1),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "OperatorSet",
                None,
                None,
            )),
        }
    }
}

/// Unvalidated component 1 data in exact wire field order.
#[derive(Clone, Debug, PartialEq)]
pub struct Architecture {
    /// Must equal `ARCHITECTURE_ID`.
    pub architecture: String,
    /// Must equal `NUMERICAL_PROFILE_ID`.
    pub numerical_profile: String,
    /// State purpose.
    pub purpose: Purpose,
    /// Explicit learning profile.
    pub learning_profile: LearningProfile,
    /// Compiled operator vocabulary.
    pub operator_set: OperatorSet,
    /// Exactly 3: causal text and graph ranking.
    pub required_features: u64,
    /// N, 2..=`node_capacity`.
    pub node_count: u32,
    /// M, 1..=`edge_capacity`.
    pub edge_count: u32,
    /// d, 1..=64.
    pub hidden_width: u32,
    /// R, 1..=8.
    pub relation_count: u32,
    /// V, 260..=4096.
    pub vocab_count: u32,
    /// K, 1..=4, shared-weight synchronous layers.
    pub message_steps: u32,
    /// 2..=128 including BOS; input plus generation must fit.
    pub max_sequence_tokens: u32,
    /// 1..=128; does not override remaining context.
    pub max_generated_tokens: u32,
    /// 1..=4.
    pub max_batch_examples: u32,
    /// N..=128.
    pub node_capacity: u32,
    /// M..=1024.
    pub edge_capacity: u32,
    /// 1..=16.
    pub max_incoming_degree: u32,
    /// 1..=1,000,000 across all sixteen tensors.
    pub max_trainable_scalars: u32,
    /// Positive, at most 256 `MiB`, independently estimated before execution.
    pub max_session_working_bytes: u64,
    /// Positive, at most 512 `MiB` including retained snapshot and copies.
    pub max_loaded_and_trainer_bytes: u64,
    /// Positive, at most 500,000,000 per batch.
    pub max_training_scalar_ops: u64,
    /// Finite and strictly positive.
    pub spatial_scale: f64,
    /// Exactly 4.0; learned unitless coordinates only.
    pub coordinate_radius: f64,
}
