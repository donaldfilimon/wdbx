//! Accessible retained capacities; logical mode bounds the fresh decoded phase.
use super::{Annotation, ModelComponents, Result, add};
use crate::model::contract::{
    tensors::{Axis, TensorValues},
    training::{OptimizerState, RngOrderState},
};
struct Storage {
    bytes: u64,
    logical: bool,
}
impl Storage {
    fn add(&mut self, n: u64) -> Result<()> {
        self.bytes = add(self.bytes, n)?;
        Ok(())
    }
    fn allocation(&mut self, n: usize, width: usize) -> Result<()> {
        let n = n.checked_mul(width).ok_or_else(|| {
            crate::model::package::err(
                crate::model::error::ErrorKind::CapacityExceeded,
                "writer-storage-product",
            )
        })?;
        self.add(n as u64)?;
        self.add(32)
    }
    fn vector<T>(&mut self, v: &Vec<T>) -> Result<()> {
        self.allocation(
            if self.logical { v.len() } else { v.capacity() },
            std::mem::size_of::<T>(),
        )
    }
    fn string(&mut self, s: &String) -> Result<()> {
        self.allocation(if self.logical { s.len() } else { s.capacity() }, 1)
    }
}
pub(super) fn model(c: &ModelComponents, logical: bool) -> Result<u64> {
    let mut size = Storage {
        bytes: std::mem::size_of::<ModelComponents>() as u64,
        logical,
    };
    size.string(&c.architecture.architecture)?;
    size.string(&c.architecture.numerical_profile)?;
    size.vector(&c.topology.relations)?;
    size.vector(&c.topology.nodes)?;
    size.vector(&c.topology.edges)?;
    for relation in &c.topology.relations {
        size.string(&relation.label)?;
    }
    size.vector(&c.tensors)?;
    for tensor in &c.tensors {
        size.vector(&tensor.axes)?;
        for axis in &tensor.axes {
            match axis {
                Axis::Node(v) => size.vector(v)?,
                Axis::Edge(v) => size.vector(v)?,
                _ => {}
            }
        }
        match &tensor.data {
            TensorValues::F32(v) => size.vector(v)?,
            TensorValues::F64(v) => size.vector(v)?,
        }
    }
    size.string(&c.tokenizer.identity)?;
    size.vector(&c.tokenizer.vocabulary)?;
    for token in &c.tokenizer.vocabulary {
        size.vector(token)?;
    }
    size.vector(&c.tokenizer.merges)?;
    if let OptimizerState::AdamW(state) = &c.optimizer {
        size.vector(&state.moments)?;
        for moment in &state.moments {
            size.vector(&moment.first_moment)?;
            size.vector(&moment.second_moment)?;
        }
    }
    if let RngOrderState::StepBoundary(state) = &c.rng_order {
        size.add(std::mem::size_of_val(state.as_ref()) as u64 + 32)?;
        size.vector(&state.streams)?;
        size.vector(&state.permutation)?;
        if logical {
            size.allocation(state.permutation.len(), 1)?;
        }
    }
    size.vector(&c.provenance.source_cohorts)?;
    for cohort in &c.provenance.source_cohorts {
        size.string(&cohort.name)?;
        size.string(&cohort.revision)?;
    }
    size.vector(&c.provenance.data_roots)?;
    size.vector(&c.provenance.license_digests)?;
    Ok(size.bytes)
}
pub(super) fn annotations(annotations: &[Annotation]) -> Result<u64> {
    // A borrowed slice exposes its accessible inline rows, not unrelated storage
    // outside that slice; every accessible payload's full retained capacity counts.
    let mut size = Storage {
        bytes: std::mem::size_of_val(annotations) as u64,
        logical: false,
    };
    for a in annotations {
        size.vector(&a.bytes)?;
    }
    Ok(size.bytes)
}
