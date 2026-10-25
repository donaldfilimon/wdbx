//! Component 2: stable IDs and semantic graph topology, not display positions.
use crate::model::ids::{Digest, EdgeId, NodeId, RelationId};

/// Learned coordinate frame only.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum CoordinateFrame {
    /// Unitless learned x/y/z axes in tensor 2.
    LearnedUnitlessCartesian3 = 1,
}
impl TryFrom<u16> for CoordinateFrame {
    type Error = super::super::error::ModelError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::LearnedUnitlessCartesian3),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "CoordinateFrame",
                None,
                None,
            )),
        }
    }
}

/// Exactly one Input and one Readout are required.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum NodeRole {
    /// Text/query injection node.
    Input = 0,
    /// Hidden state node.
    Hidden = 1,
    /// Eligible role for graph ranking, subject to external authority.
    Memory = 2,
    /// Causal text readout node.
    Readout = 3,
}
impl TryFrom<u8> for NodeRole {
    type Error = super::super::error::ModelError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Input),
            1 => Ok(Self::Hidden),
            2 => Ok(Self::Memory),
            3 => Ok(Self::Readout),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "NodeRole",
                None,
                None,
            )),
        }
    }
}

/// Immutable semantic relation row.
#[derive(Clone, Debug, PartialEq)]
pub struct Relation {
    /// Dense semantic ID 0..R-1.
    pub id: RelationId,
    /// Nonempty unique exact UTF-8 bytes, at most 64 bytes.
    pub label: String,
}

/// Persistent node row.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Nonzero, serialized in strictly increasing ID order.
    pub id: NodeId,
    /// Closed role vocabulary.
    pub role: NodeRole,
    /// Only Memory may reference a declared kind3 data root; never access permission.
    pub source_ref: Option<Digest>,
}

/// Persistent edge row; reduction order is separate.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    /// Nonzero, strictly increasing serialized ID.
    pub id: EdgeId,
    /// Existing source, including self edges.
    pub source: NodeId,
    /// Existing destination; cycles permitted.
    pub destination: NodeId,
    /// Existing immutable relation ID.
    pub relation: RelationId,
}

/// Unvalidated component 2 data, in wire order.
#[derive(Clone, Debug, PartialEq)]
pub struct Topology {
    /// No physical/display positions in core components.
    pub coordinate_frame: CoordinateFrame,
    /// Live maximum through `u64::MAX-1`; lineage floor checks remain N06.
    pub node_id_high_water: u64,
    /// Live maximum through `u64::MAX-1`; never reused across lineage.
    pub edge_id_high_water: u64,
    /// Exactly R rows, IDs in order.
    pub relations: Vec<Relation>,
    /// Exactly N rows in `NodeId` order.
    pub nodes: Vec<Node>,
    /// Exactly M rows in `EdgeId` order; unique source/destination/relation triples.
    pub edges: Vec<Edge>,
}
