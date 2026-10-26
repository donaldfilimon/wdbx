//! Components 5, 6 and 7: objective, optimizer and complete step-boundary state.
use super::tensors::TensorId;
use crate::model::ids::Digest;

/// Fixed combined objectives.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum ObjectiveProfile {
    /// Fixed combined objectives.
    NextTokenGraphCeCoordinateL2V1 = 1,
}
impl TryFrom<u16> for ObjectiveProfile {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::NextTokenGraphCeCoordinateL2V1),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "ObjectiveProfile",
                None,
                None,
            )),
        }
    }
}

/// EOS and byte/merge targets permitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum TextMaskProfile {
    /// EOS and byte/merge targets permitted.
    DisallowPadBosUnk = 1,
}
impl TryFrom<u16> for TextMaskProfile {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::DisallowPadBosUnk),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "TextMaskProfile",
                None,
                None,
            )),
        }
    }
}

/// Mean over valid target tokens across batch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum TextReduction {
    /// Mean over valid target tokens across batch.
    ValidTargetTokenMean = 1,
}
impl TryFrom<u16> for TextReduction {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::ValidTargetTokenMean),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "TextReduction",
                None,
                None,
            )),
        }
    }
}

/// Mean over graph examples, each CE over candidates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum GraphReduction {
    /// Mean over graph examples, each CE over candidates.
    ExampleMean = 1,
}
impl TryFrom<u16> for GraphReduction {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::ExampleMean),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "GraphReduction",
                None,
                None,
            )),
        }
    }
}

/// No packing or truncation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum SequencePolicy {
    /// No packing or truncation.
    BosContentEos = 1,
}
impl TryFrom<u16> for SequencePolicy {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::BosContentEos),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "SequencePolicy",
                None,
                None,
            )),
        }
    }
}

/// Unique sorted Memory IDs with exactly one target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum GraphCandidatePolicy {
    /// Unique sorted Memory IDs with exactly one target.
    ExplicitAuthorizedMemoryIds = 1,
}
impl TryFrom<u16> for GraphCandidatePolicy {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::ExplicitAuthorizedMemoryIds),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "GraphCandidatePolicy",
                None,
                None,
            )),
        }
    }
}

/// Fixed global clip, moments, bias correction, decay, projection, cast order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum OptimizerProfile {
    /// Fixed global clip, moments, bias correction, decay, projection, cast order.
    AdamWGlobalNormMatrixDecayV1 = 1,
}
impl TryFrom<u16> for OptimizerProfile {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::AdamWGlobalNormMatrixDecayV1),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "OptimizerProfile",
                None,
                None,
            )),
        }
    }
}

/// No hidden scheduler counter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum SchedulerProfile {
    /// No hidden scheduler counter.
    ConstantLearningRateV1 = 1,
}
impl TryFrom<u16> for SchedulerProfile {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::ConstantLearningRateV1),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "SchedulerProfile",
                None,
                None,
            )),
        }
    }
}

/// Only PRNG arithmetic wraps modulo 2^64.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum RngProfile {
    /// Only PRNG arithmetic wraps modulo 2^64.
    Xoshiro256StarStarV1 = 1,
}
impl TryFrom<u16> for RngProfile {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Xoshiro256StarStarV1),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "RngProfile",
                None,
                None,
            )),
        }
    }
}

/// Per-stream seed derivation fixed by profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum SeedProfile {
    /// Per-stream seed derivation fixed by profile.
    SplitMix64V1 = 1,
}
impl TryFrom<u16> for SeedProfile {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::SplitMix64V1),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "SeedProfile",
                None,
                None,
            )),
        }
    }
}

/// Rejection sampling consumes every draw.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum SamplerProfile {
    /// Rejection sampling consumes every draw.
    FisherYatesUnbiasedU64V1 = 1,
}
impl TryFrom<u16> for SamplerProfile {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::FisherYatesUnbiasedU64V1),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "SamplerProfile",
                None,
                None,
            )),
        }
    }
}

/// Short final batch, never drop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum BatchPolicy {
    /// Short final batch, never drop.
    OrderedContiguous = 1,
}
impl TryFrom<u16> for BatchPolicy {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::OrderedContiguous),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "BatchPolicy",
                None,
                None,
            )),
        }
    }
}

/// No in-flight tape, gradients or candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum PendingState {
    /// Only complete successful step boundaries serialize.
    None = 0,
}
impl TryFrom<u16> for PendingState {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "PendingState",
                None,
                None,
            )),
        }
    }
}

/// Exactly five streams in ID order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum StreamId {
    /// Initialization stream.
    ParameterInitialization = 1,
    /// Permutation stream.
    DataShuffle = 2,
    /// Preserved even when explicit candidates consume no random draws.
    GraphNegativeSampling = 3,
    /// Copied to request-local sampler; not an inference session checkpoint.
    TextSamplingTemplate = 4,
    /// Permission and schedules remain external.
    TopologyProposal = 5,
}
impl TryFrom<u8> for StreamId {
    type Error = super::super::error::ModelError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::ParameterInitialization),
            2 => Ok(Self::DataShuffle),
            3 => Ok(Self::GraphNegativeSampling),
            4 => Ok(Self::TextSamplingTemplate),
            5 => Ok(Self::TopologyProposal),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "StreamId",
                None,
                None,
            )),
        }
    }
}

/// Unvalidated component 5, in exact wire order.
#[derive(Clone, Debug, PartialEq)]
pub struct Objective {
    /// Fixed combined objective.
    pub objective_profile: ObjectiveProfile,
    /// Structural target policy.
    pub text_mask_profile: TextMaskProfile,
    /// Target-token mean.
    pub text_reduction: TextReduction,
    /// Graph-example mean.
    pub graph_reduction: GraphReduction,
    /// Exactly 1.0.
    pub text_weight: f64,
    /// Finite and nonnegative.
    pub graph_weight: f64,
    /// Finite and nonnegative.
    pub coordinate_l2_weight: f64,
    /// BOS, content, EOS.
    pub sequence_policy: SequencePolicy,
    /// Explicit externally authorized Memory IDs.
    pub graph_candidate_policy: GraphCandidatePolicy,
    /// 1..=`node_capacity`.
    pub max_graph_candidates: u32,
    /// Nonzero exact immutable external manifest identity.
    pub dataset_manifest_digest: Digest,
    /// Nonzero exact immutable external split identity.
    pub split_manifest_digest: Digest,
    /// Nonzero exact immutable external preparation identity.
    pub preparation_digest: Digest,
}

/// Component 6 state discriminant (0 absent, 1 `AdamW`); Rust layout is not wire layout.
#[derive(Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum OptimizerState {
    /// `InferenceOnly`: component ends after byte 0.
    Absent = 0,
    /// `ResumableTraining`: complete `AdamW` boundary.
    AdamW(AdamWState) = 1,
}

/// Moments inherit the tensor stable-ID axes and row-major order.
#[derive(Clone, Debug, PartialEq)]
pub struct Moments {
    /// Exactly 1..16 in order, no subset.
    pub tensor_id: TensorId,
    /// Equals corresponding tensor scalar count.
    pub scalar_count: u64,
    /// Finite, zero at step zero (signed zero preserved).
    pub first_moment: Vec<f64>,
    /// Finite and nonnegative, zero at step zero.
    pub second_moment: Vec<f64>,
}

/// Component 6 payload after state byte 1.
#[derive(Clone, Debug, PartialEq)]
pub struct AdamWState {
    /// Fixed `AdamW` rule.
    pub optimizer_profile: OptimizerProfile,
    /// Constant learning rate.
    pub scheduler_profile: SchedulerProfile,
    /// Strictly less than `u64::MAX`.
    pub successful_step: u64,
    /// 0 < lr <= 1.
    pub learning_rate: f64,
    /// 0 <= beta1 < 1.
    pub beta1: f64,
    /// 0 <= beta2 < 1.
    pub beta2: f64,
    /// 0 < epsilon <= 1.
    pub epsilon: f64,
    /// 0 <= decay <= 1; only catalog matrix members.
    pub weight_decay: f64,
    /// 0 < clip <= 1000.
    pub gradient_clip_norm: f64,
    /// Exactly sixteen canonical members, no partial optimizer step.
    pub moments: Vec<Moments>,
}

/// Component 7 state discriminant (0 absent, 1 step boundary).
#[derive(Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum RngOrderState {
    /// `InferenceOnly`: component ends after byte 0.
    Absent = 0,
    /// `ResumableTraining`: full post-step state.
    StepBoundary(Box<StepBoundaryState>) = 1,
}

/// One explicit xoshiro256** stream.
#[derive(Clone, Debug, PartialEq)]
pub struct RandomStream {
    /// Exactly 1..5 ordered.
    pub stream_id: StreamId,
    /// Not all zero; at consumed zero must match root seed derivation.
    pub state_words: [u64; 4],
    /// Strictly less than `u64::MAX`; counts rejected sampler draws too.
    pub consumed_u64: u64,
}

/// Component 7 payload after state byte 1.
#[derive(Clone, Debug, PartialEq)]
pub struct StepBoundaryState {
    /// Exact PRNG transition.
    pub rng_profile: RngProfile,
    /// Exact stream initializer.
    pub seed_profile: SeedProfile,
    /// Seed, not a replacement for full stream state.
    pub root_seed: u64,
    /// Exactly five streams in ID order.
    pub streams: Vec<RandomStream>,
    /// Unbiased Fisher-Yates.
    pub sampler_profile: SamplerProfile,
    /// Ordered contiguous, short final batch.
    pub batch_policy: BatchPolicy,
    /// Always zero.
    pub pending_state: PendingState,
    /// Exactly matches component 5.
    pub dataset_manifest_digest: Digest,
    /// Exactly matches component 5.
    pub split_manifest_digest: Digest,
    /// Exactly matches component 5.
    pub preparation_digest: Digest,
    /// Exactly matches component 6.
    pub successful_step: u64,
    /// Checked counter; next epoch normalized before checkpoint.
    pub epoch: u64,
    /// 1..=65536.
    pub example_count: u32,
    /// Strictly below `example_count`; never serialize end cursor.
    pub next_example_cursor: u32,
    /// 1..=architecture.`max_batch_examples`.
    pub batch_examples: u32,
    /// Bijection of 0..`example_count`, actual order not only seed.
    pub permutation: Vec<u32>,
    /// Checked counter; `OfflineExplicit` requires zero.
    pub accepted_topology_proposals: u64,
    /// Zero disabled, otherwise greater than `successful_step`; `OfflineExplicit` zero.
    pub next_topology_proposal_step: u64,
}
