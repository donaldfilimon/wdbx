//! Raw storage coherence and conservative live allocations before output exists.
use super::{Annotation, Encoder};
use crate::model::package::{Result, err, require};
use crate::model::{
    contract::{
        ModelComponents,
        tensors::{Axis, Dtype, TENSOR_CATALOG, TensorValues},
        training::{OptimizerState, RngOrderState},
    },
    error::ErrorKind as E,
    limits::{LOADING_OVERHEAD_BYTES, LoadLimits},
};
mod storage;
/// Both writer-input and independent decoder phases include borrowed annotations.
#[derive(Debug)]
pub(super) struct Plan {
    pub output_len: usize,
    pub borrowed_annotations: u64,
    pub peak: u64,
}
pub(super) fn calculate(
    c: &ModelComponents,
    annotations: &[Annotation],
    limits: LoadLimits,
) -> Result<Plan> {
    validate_raw(c)?;
    let retained = storage::model(c, false)?;
    let annotation_bytes = storage::annotations(annotations)?;
    let minimum = add(add(retained, annotation_bytes)?, LOADING_OVERHEAD_BYTES)?;
    require(
        minimum <= limits.loading_bytes,
        E::CapacityExceeded,
        "writer-peak-budget",
    )?;
    // Measuring encodes no output bytes and reserves no output/directory buffer.
    let mut measure = Encoder::measure(limits)?;
    measure.zeroes(96 + 80 * (8 + annotations.len()))?;
    for kind in 1..=8 {
        let start = measure.len();
        super::schema::component(kind, c, &mut measure)?;
        if kind == 4 {
            require(
                (measure.len() - start) as u64 <= limits.tokenizer_bytes,
                E::CapacityExceeded,
                "tokenizer-component",
            )?;
        }
    }
    for annotation in annotations {
        measure.bytes(&annotation.bytes)?;
    }
    require(
        measure.out.capacity() == 0,
        E::CapacityExceeded,
        "writer-measuring-allocation",
    )?;
    let output_bytes = measure.len();
    let decoded = storage::model(c, true)?;
    let peak_bytes = add(
        add(
            add(retained.max(decoded), annotation_bytes)?,
            output_bytes as u64,
        )?,
        LOADING_OVERHEAD_BYTES,
    )?;
    require(
        peak_bytes <= limits.loading_bytes,
        E::CapacityExceeded,
        "writer-peak-budget",
    )?;
    Ok(Plan {
        output_len: output_bytes,
        borrowed_annotations: annotation_bytes,
        peak: peak_bytes,
    })
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b)
        .ok_or_else(|| err(E::CapacityExceeded, "writer-storage-sum"))
}
fn count(n: usize, min: usize, max: usize, field: &'static str) -> Result<()> {
    require((min..=max).contains(&n), E::CapacityExceeded, field)
}
fn string(s: &str, max: usize) -> Result<()> {
    count(s.len(), 0, max, "writer-string")
}
fn validate_raw(c: &ModelComponents) -> Result<()> {
    validate_header_and_topology(c)?;
    validate_tensor_storage(c)?;
    count(c.tokenizer.vocabulary.len(), 260, 4096, "writer-vocabulary")?;
    count(c.tokenizer.merges.len(), 0, 3836, "writer-merges")?;
    string(&c.tokenizer.identity, 4096)?;
    for bytes in &c.tokenizer.vocabulary {
        count(bytes.len(), 0, 65536, "writer-token-bytes")?;
    }
    crate::model::tokenizer::validate_contract(&c.tokenizer)?;
    if let OptimizerState::AdamW(state) = &c.optimizer {
        count(state.moments.len(), 16, 16, "writer-moments")?;
        for (moment, tensor) in state.moments.iter().zip(&c.tensors) {
            let n = tensor.data_byte_length / if tensor.dtype == Dtype::F32 { 4 } else { 8 };
            require(
                moment.tensor_id == tensor.id
                    && moment.scalar_count == n
                    && moment.first_moment.len() as u64 == n
                    && moment.second_moment.len() as u64 == n,
                E::InvalidOptimizer,
                "writer-moment-storage",
            )?;
        }
    }
    if let RngOrderState::StepBoundary(state) = &c.rng_order {
        count(state.streams.len(), 5, 5, "writer-streams")?;
        count(state.permutation.len(), 1, 65536, "writer-permutation")?;
    }
    count(c.provenance.source_cohorts.len(), 0, 32, "writer-cohorts")?;
    count(c.provenance.data_roots.len(), 0, 1024, "writer-data-roots")?;
    count(c.provenance.license_digests.len(), 0, 64, "writer-licenses")?;
    for cohort in &c.provenance.source_cohorts {
        string(&cohort.name, 4096)?;
        string(&cohort.revision, 4096)?;
    }
    Ok(())
}

fn validate_header_and_topology(c: &ModelComponents) -> Result<()> {
    let a = &c.architecture;
    string(&a.architecture, 4096)?;
    string(&a.numerical_profile, 4096)?;
    let dimensions = [
        a.node_count,
        a.edge_count,
        a.hidden_width,
        a.relation_count,
        a.vocab_count,
        a.message_steps,
        a.max_sequence_tokens,
        a.max_generated_tokens,
        a.max_batch_examples,
        a.node_capacity,
        a.edge_capacity,
        a.max_incoming_degree,
        a.max_trainable_scalars,
    ];
    let minima = [2, 1, 1, 1, 260, 1, 2, 1, 1, 2, 1, 1, 1];
    let maxima = [
        128, 1024, 64, 8, 4096, 4, 128, 128, 4, 128, 1024, 16, 1_000_000,
    ];
    for ((value, min), max) in dimensions.into_iter().zip(minima).zip(maxima) {
        require(
            (min..=max).contains(&value),
            E::CapacityExceeded,
            "writer-architecture-count",
        )?;
    }
    count(c.topology.relations.len(), 1, 8, "writer-relations")?;
    count(c.topology.nodes.len(), 2, 128, "writer-nodes")?;
    count(c.topology.edges.len(), 1, 1024, "writer-edges")?;
    for relation in &c.topology.relations {
        string(&relation.label, 64)?;
    }
    Ok(())
}

fn validate_tensor_storage(c: &ModelComponents) -> Result<()> {
    count(c.tensors.len(), 16, 16, "writer-tensors")?;
    let mut total = 0u64;
    for (tensor, spec) in c.tensors.iter().zip(TENSOR_CATALOG) {
        require(
            tensor.id == spec.id
                && tensor.dtype == spec.dtype
                && usize::from(tensor.rank) == tensor.axes.len()
                && (1..=3).contains(&tensor.rank),
            E::InvalidTensor,
            "writer-tensor-descriptor",
        )?;
        let mut scalar_count = 1u64;
        for axis in &tensor.axes {
            let n = match axis {
                Axis::Node(ids) => {
                    count(ids.len(), 1, 128, "writer-node-axis")?;
                    ids.len() as u64
                }
                Axis::Edge(ids) => {
                    count(ids.len(), 1, 1024, "writer-edge-axis")?;
                    ids.len() as u64
                }
                Axis::Relation(n) | Axis::Token(n) | Axis::HiddenOut(n) | Axis::HiddenIn(n) => {
                    u64::from(*n)
                }
                Axis::Xyz => 3,
            };
            require(
                (1..=4096).contains(&n),
                E::InvalidTensor,
                "writer-tensor-axis",
            )?;
            scalar_count = scalar_count
                .checked_mul(n)
                .ok_or_else(|| err(E::CapacityExceeded, "writer-tensor-product"))?;
            require(
                scalar_count <= 1_000_000,
                E::CapacityExceeded,
                "writer-tensor-scalars",
            )?;
        }
        let valid = match (&tensor.data, tensor.dtype) {
            (TensorValues::F32(values), Dtype::F32) => {
                values.len() as u64 == scalar_count && values.iter().all(|v| v.is_finite())
            }
            (TensorValues::F64(values), Dtype::F64) => {
                values.len() as u64 == scalar_count && values.iter().all(|v| v.is_finite())
            }
            _ => false,
        };
        require(valid, E::InvalidTensor, "writer-tensor-storage")?;
        let width = if tensor.dtype == Dtype::F32 { 4 } else { 8 };
        require(
            tensor.data_byte_length == scalar_count * width,
            E::InvalidTensor,
            "writer-tensor-byte-length",
        )?;
        total = add(total, scalar_count)?;
        require(
            total <= 1_000_000,
            E::CapacityExceeded,
            "writer-total-scalars",
        )?;
    }
    Ok(())
}
