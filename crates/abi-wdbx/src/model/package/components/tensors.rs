//! Exact semantic axes and finite numerical payloads.
use super::{Cursor, Result};
use crate::model::{
    contract::tensors::{Axis, AxisKind, Dtype, TENSOR_CATALOG, Tensor, TensorId, TensorValues},
    error::ErrorKind as E,
    ids::{EdgeId, NodeId},
};
pub(super) fn parse(r: &mut Cursor<'_, '_>) -> Result<Vec<Tensor>> {
    let n = r.count(16)?;
    r.check(n == 16, E::InvalidTensor, "tensor-membership")?;
    let mut tensors = r.vec(16)?;
    let mut total = 0u64;
    for spec in TENSOR_CATALOG {
        let ident = r.u16()?;
        let dtype = r.e8::<Dtype>()?;
        let rank = r.u8()?;
        r.check(
            ident == spec.id as u16 && (1..=3).contains(&rank),
            E::InvalidTensor,
            "tensor-id-rank",
        )?;
        let id = spec.id;
        let mut axes = r.vec(rank as usize)?;
        let mut count = 1u64;
        for _ in 0..rank {
            let kind = r.e8::<AxisKind>()?;
            let n = r.u32()?;
            r.check((1..=4096).contains(&n), E::InvalidTensor, "axis-length")?;
            count = count
                .checked_mul(u64::from(n))
                .ok_or_else(|| r.error(E::CapacityExceeded, "tensor-product"))?;
            r.check(count <= 1_000_000, E::CapacityExceeded, "tensor-scalars")?;
            let descriptor = match kind {
                AxisKind::Node => {
                    r.check(n <= 128, E::InvalidTensor, "node-axis-length")?;
                    let mut ids = r.vec(n as usize)?;
                    for _ in 0..n {
                        ids.push(NodeId(r.u64()?));
                    }
                    Axis::Node(ids)
                }
                AxisKind::Edge => {
                    r.check(n <= 1024, E::InvalidTensor, "edge-axis-length")?;
                    let mut ids = r.vec(n as usize)?;
                    for _ in 0..n {
                        ids.push(EdgeId(r.u64()?));
                    }
                    Axis::Edge(ids)
                }
                AxisKind::Relation => Axis::Relation(n),
                AxisKind::Token => Axis::Token(n),
                AxisKind::HiddenOut => Axis::HiddenOut(n),
                AxisKind::HiddenIn => Axis::HiddenIn(n),
                AxisKind::Xyz => {
                    r.check(n == 3, E::InvalidTensor, "xyz-length")?;
                    Axis::Xyz
                }
            };
            axes.push(descriptor);
        }
        total += count;
        r.check(total <= 1_000_000, E::CapacityExceeded, "total-scalars")?;
        let data_byte_length = r.u64()?;
        let width = if dtype == Dtype::F32 { 4 } else { 8 };
        r.check(
            data_byte_length == count * width,
            E::InvalidTensor,
            "tensor-data-length",
        )?;
        let data = values(r, id, dtype, count, data_byte_length)?;
        let kinds = axes.iter().map(|a| match a {
            Axis::Node(_) => AxisKind::Node,
            Axis::Edge(_) => AxisKind::Edge,
            Axis::Relation(_) => AxisKind::Relation,
            Axis::Token(_) => AxisKind::Token,
            Axis::HiddenOut(_) => AxisKind::HiddenOut,
            Axis::HiddenIn(_) => AxisKind::HiddenIn,
            Axis::Xyz => AxisKind::Xyz,
        });
        r.check(
            dtype == spec.dtype && kinds.eq(spec.axes.iter().copied()),
            E::InvalidTensor,
            "semantic-axes",
        )?;
        tensors.push(Tensor {
            id,
            dtype,
            rank,
            axes,
            data_byte_length,
            data,
        });
    }
    r.end()?;
    Ok(tensors)
}

fn values(
    r: &mut Cursor<'_, '_>,
    id: TensorId,
    dtype: Dtype,
    count: u64,
    data_byte_length: u64,
) -> Result<TensorValues> {
    Ok(if dtype == Dtype::F32 {
        let mut v = r.vec(super::super::size(count)?)?;
        for chunk in r
            .take(super::super::size(data_byte_length)?, true)?
            .as_chunks::<4>()
            .0
        {
            let f = f32::from_le_bytes(*chunk);
            r.check(f.is_finite(), E::InvalidTensor, "finite-tensor")?;
            v.push(f);
        }
        TensorValues::F32(v)
    } else {
        let mut v = r.vec(super::super::size(count)?)?;
        for chunk in r
            .take(super::super::size(data_byte_length)?, true)?
            .as_chunks::<8>()
            .0
        {
            let f = f64::from_le_bytes(*chunk);
            r.check(f.is_finite(), E::InvalidTensor, "finite-tensor")?;
            if id == TensorId::LearnedCoordinates {
                r.check(
                    (-4.0..=4.0).contains(&f),
                    E::InvalidTensor,
                    "coordinate-radius",
                )?;
            }
            v.push(f);
        }
        TensorValues::F64(v)
    })
}
