//! Component 3: fixed semantic tensor membership, dtype, axes and decay.
use crate::model::ids::{EdgeId, NodeId};

/// Stored numeric dtype; finite values and exact signed-zero bits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Dtype {
    /// IEEE-754 little-endian binary32.
    F32 = 1,
    /// IEEE-754 little-endian binary64.
    F64 = 2,
}
impl TryFrom<u8> for Dtype {
    type Error = super::super::error::ModelError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::F32),
            2 => Ok(Self::F64),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "Dtype",
                None,
                None,
            )),
        }
    }
}

/// Descriptor order is semantic even when dimensions happen to match.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AxisKind {
    /// Full ordered stable `NodeIds`.
    Node = 1,
    /// Full ordered stable `EdgeIds`.
    Edge = 2,
    /// Implicit immutable relation IDs 0..R-1.
    Relation = 3,
    /// Implicit vocabulary IDs 0..V-1.
    Token = 4,
    /// Output coordinate 0..d-1; vectors use this axis.
    HiddenOut = 5,
    /// Input coordinate 0..d-1.
    HiddenIn = 6,
    /// Exactly x,y,z in that order.
    Xyz = 7,
}
impl TryFrom<u8> for AxisKind {
    type Error = super::super::error::ModelError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Node),
            2 => Ok(Self::Edge),
            3 => Ok(Self::Relation),
            4 => Ok(Self::Token),
            5 => Ok(Self::HiddenOut),
            6 => Ok(Self::HiddenIn),
            7 => Ok(Self::Xyz),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "AxisKind",
                None,
                None,
            )),
        }
    }
}

/// Exactly sixteen required trainable tensors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum TensorId {
    /// b; fixed catalog row.
    BaseStates = 1,
    /// p; fixed catalog row.
    LearnedCoordinates = 2,
    /// E; fixed catalog row.
    TokenEmbedding = 3,
    /// U; fixed catalog row.
    InputProjection = 4,
    /// R; fixed catalog row.
    RecurrentProjection = 5,
    /// W; fixed catalog row.
    RelationMessages = 6,
    /// S; fixed catalog row.
    SelfProjection = 7,
    /// c; fixed catalog row.
    MessageBias = 8,
    /// a; fixed catalog row.
    EdgeGateLogits = 9,
    /// O; fixed catalog row.
    TextHead = 10,
    /// o; fixed catalog row.
    TextBias = 11,
    /// J; fixed catalog row.
    GraphQueryProjection = 12,
    /// Q; fixed catalog row.
    GraphNodeProjection = 13,
    /// B; fixed catalog row.
    GraphQueryToScoreProjection = 14,
    /// `d_q`; fixed catalog row.
    GraphScoreBias = 15,
    /// v; fixed catalog row.
    GraphScoreVector = 16,
}
impl TryFrom<u16> for TensorId {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::BaseStates),
            2 => Ok(Self::LearnedCoordinates),
            3 => Ok(Self::TokenEmbedding),
            4 => Ok(Self::InputProjection),
            5 => Ok(Self::RecurrentProjection),
            6 => Ok(Self::RelationMessages),
            7 => Ok(Self::SelfProjection),
            8 => Ok(Self::MessageBias),
            9 => Ok(Self::EdgeGateLogits),
            10 => Ok(Self::TextHead),
            11 => Ok(Self::TextBias),
            12 => Ok(Self::GraphQueryProjection),
            13 => Ok(Self::GraphNodeProjection),
            14 => Ok(Self::GraphQueryToScoreProjection),
            15 => Ok(Self::GraphScoreBias),
            16 => Ok(Self::GraphScoreVector),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "TensorId",
                None,
                None,
            )),
        }
    }
}

/// Stable IDs occur only on Node and Edge axes; no dense-slot conversion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Axis {
    /// Ordered full node axis.
    Node(Vec<NodeId>),
    /// Ordered full edge axis (not reduction order).
    Edge(Vec<EdgeId>),
    /// Implicit immutable relation IDs.
    Relation(u32),
    /// Implicit token IDs.
    Token(u32),
    /// Hidden output dimension.
    HiddenOut(u32),
    /// Hidden input dimension.
    HiddenIn(u32),
    /// Fixed x/y/z order.
    Xyz,
}
/// Row-major numerical values, with last axis varying fastest.
#[derive(Clone, Debug, PartialEq)]
pub enum TensorValues {
    /// Parameter binary32 values.
    F32(Vec<f32>),
    /// Coordinate binary64 values.
    F64(Vec<f64>),
}

/// Unvalidated tensor record; all redundant descriptors must agree.
#[derive(Clone, Debug, PartialEq)]
pub struct Tensor {
    /// Strict catalog order 1..16.
    pub id: TensorId,
    /// Must match both catalog and numerical storage.
    pub dtype: Dtype,
    /// Must equal axes length and catalog rank, 1..=3.
    pub rank: u8,
    /// Exact catalog ordering and full persistent stable IDs.
    pub axes: Vec<Axis>,
    /// Checked product of axis lengths times dtype width.
    pub data_byte_length: u64,
    /// Finite values; learned coordinates additionally within [-4,4].
    pub data: TensorValues,
}

/// One immutable catalog entry; all members are trainable in this profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TensorSpec {
    /// Fixed semantic tensor identifier.
    pub id: TensorId,
    /// Mathematical short name.
    pub name: &'static str,
    /// Required storage dtype.
    pub dtype: Dtype,
    /// Required axes in semantic order (output before input).
    pub axes: &'static [AxisKind],
    /// Decoupled `AdamW` matrix decay applies to this member only when true.
    pub adamw_decay: bool,
}
/// Complete frozen schema 1.0 tensor catalog, in persistent order.
pub const TENSOR_CATALOG: [TensorSpec; 16] = [
    TensorSpec {
        id: TensorId::BaseStates,
        name: "b",
        dtype: Dtype::F32,
        axes: &[AxisKind::Node, AxisKind::HiddenOut],
        adamw_decay: false,
    },
    TensorSpec {
        id: TensorId::LearnedCoordinates,
        name: "p",
        dtype: Dtype::F64,
        axes: &[AxisKind::Node, AxisKind::Xyz],
        adamw_decay: false,
    },
    TensorSpec {
        id: TensorId::TokenEmbedding,
        name: "E",
        dtype: Dtype::F32,
        axes: &[AxisKind::Token, AxisKind::HiddenOut],
        adamw_decay: false,
    },
    TensorSpec {
        id: TensorId::InputProjection,
        name: "U",
        dtype: Dtype::F32,
        axes: &[AxisKind::HiddenOut, AxisKind::HiddenIn],
        adamw_decay: true,
    },
    TensorSpec {
        id: TensorId::RecurrentProjection,
        name: "R",
        dtype: Dtype::F32,
        axes: &[AxisKind::HiddenOut, AxisKind::HiddenIn],
        adamw_decay: true,
    },
    TensorSpec {
        id: TensorId::RelationMessages,
        name: "W",
        dtype: Dtype::F32,
        axes: &[AxisKind::Relation, AxisKind::HiddenOut, AxisKind::HiddenIn],
        adamw_decay: true,
    },
    TensorSpec {
        id: TensorId::SelfProjection,
        name: "S",
        dtype: Dtype::F32,
        axes: &[AxisKind::HiddenOut, AxisKind::HiddenIn],
        adamw_decay: true,
    },
    TensorSpec {
        id: TensorId::MessageBias,
        name: "c",
        dtype: Dtype::F32,
        axes: &[AxisKind::HiddenOut],
        adamw_decay: false,
    },
    TensorSpec {
        id: TensorId::EdgeGateLogits,
        name: "a",
        dtype: Dtype::F32,
        axes: &[AxisKind::Edge],
        adamw_decay: false,
    },
    TensorSpec {
        id: TensorId::TextHead,
        name: "O",
        dtype: Dtype::F32,
        axes: &[AxisKind::Token, AxisKind::HiddenIn],
        adamw_decay: true,
    },
    TensorSpec {
        id: TensorId::TextBias,
        name: "o",
        dtype: Dtype::F32,
        axes: &[AxisKind::Token],
        adamw_decay: false,
    },
    TensorSpec {
        id: TensorId::GraphQueryProjection,
        name: "J",
        dtype: Dtype::F32,
        axes: &[AxisKind::HiddenOut, AxisKind::HiddenIn],
        adamw_decay: true,
    },
    TensorSpec {
        id: TensorId::GraphNodeProjection,
        name: "Q",
        dtype: Dtype::F32,
        axes: &[AxisKind::HiddenOut, AxisKind::HiddenIn],
        adamw_decay: true,
    },
    TensorSpec {
        id: TensorId::GraphQueryToScoreProjection,
        name: "B",
        dtype: Dtype::F32,
        axes: &[AxisKind::HiddenOut, AxisKind::HiddenIn],
        adamw_decay: true,
    },
    TensorSpec {
        id: TensorId::GraphScoreBias,
        name: "d_q",
        dtype: Dtype::F32,
        axes: &[AxisKind::HiddenOut],
        adamw_decay: false,
    },
    TensorSpec {
        id: TensorId::GraphScoreVector,
        name: "v",
        dtype: Dtype::F32,
        axes: &[AxisKind::HiddenOut],
        adamw_decay: false,
    },
];
