//! Stable topology rows; coordinate data is exclusively tensor 2.
use super::{Cursor, Result};
use crate::model::{
    contract::topology::{CoordinateFrame, Edge, Node, NodeRole, Relation, Topology},
    error::ErrorKind as E,
    ids::{Digest, EdgeId, NodeId, RelationId},
};
pub(super) fn parse(r: &mut Cursor<'_, '_>) -> Result<Topology> {
    let frame = r.u16()?;
    r.check(frame == 1, E::InvalidTopology, "coordinate-frame")?;
    let coordinate_frame = CoordinateFrame::LearnedUnitlessCartesian3;
    let node_id_high_water = r.u64()?;
    let edge_id_high_water = r.u64()?;
    let n = r.count(8)?;
    let mut relations = r.vec(n)?;
    for _ in 0..n {
        relations.push(Relation {
            id: RelationId(r.u32()?),
            label: r.string(64)?,
        });
    }
    let n = r.count(128)?;
    let mut nodes = r.vec(n)?;
    for _ in 0..n {
        nodes.push(Node {
            id: NodeId(r.u64()?),
            role: r.e8()?,
            source_ref: r.opt()?.map(Digest),
        });
    }
    let n = r.count(1024)?;
    let mut edges = r.vec(n)?;
    for _ in 0..n {
        edges.push(Edge {
            id: EdgeId(r.u64()?),
            source: NodeId(r.u64()?),
            destination: NodeId(r.u64()?),
            relation: RelationId(r.u32()?),
        });
    }
    r.end()?;
    r.check(
        relations.iter().enumerate().all(|(i, x)| {
            x.id.0 == u32::try_from(i).expect("bounded relation count") && !x.label.is_empty()
        }) && relations
            .iter()
            .enumerate()
            .all(|(i, x)| relations[..i].iter().all(|p| p.label != x.label)),
        E::InvalidTopology,
        "relations",
    )?;
    r.check(
        !nodes.is_empty()
            && nodes[0].id.0 > 0
            && nodes.windows(2).all(|w| w[0].id < w[1].id)
            && nodes.last().is_some_and(|n| n.id.0 <= node_id_high_water)
            && node_id_high_water < u64::MAX,
        E::InvalidTopology,
        "node-ids",
    )?;
    r.check(
        !edges.is_empty()
            && edges[0].id.0 > 0
            && edges.windows(2).all(|w| w[0].id < w[1].id)
            && edges.last().is_some_and(|e| e.id.0 <= edge_id_high_water)
            && edge_id_high_water < u64::MAX,
        E::InvalidTopology,
        "edge-ids",
    )?;
    r.check(
        nodes.iter().filter(|n| n.role == NodeRole::Input).count() == 1
            && nodes.iter().filter(|n| n.role == NodeRole::Readout).count() == 1,
        E::InvalidTopology,
        "roles",
    )?;
    r.check(
        nodes
            .iter()
            .all(|n| n.source_ref.is_none() || n.role == NodeRole::Memory),
        E::InvalidTopology,
        "source-role",
    )?;
    r.check(
        edges.iter().enumerate().all(|(i, e)| {
            edges[..i].iter().all(|p| {
                (p.source, p.destination, p.relation) != (e.source, e.destination, e.relation)
            })
        }),
        E::InvalidTopology,
        "edge-triples",
    )?;
    Ok(Topology {
        coordinate_frame,
        node_id_high_water,
        edge_id_high_water,
        relations,
        nodes,
        edges,
    })
}
