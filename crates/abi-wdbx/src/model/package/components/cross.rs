//! Cross-schema identity joins are separate from valid individual records.
use super::{Result, require};
use crate::model::{
    contract::{
        ModelComponents,
        architecture::{LearningProfile, Purpose},
        provenance::DataRootKind,
        tensors::Axis,
        training::{OptimizerState, RngOrderState},
    },
    error::ErrorKind as E,
};
pub(super) fn validate(c: &ModelComponents) -> Result<()> {
    topology(c)?;
    tensors(c)?;
    let a = &c.architecture;
    require(
        c.tokenizer.vocabulary.len() == a.vocab_count as usize,
        E::InvalidTokenizer,
        "architecture-vocab",
    )?;
    require(
        c.objective.max_graph_candidates <= a.node_capacity,
        E::InvalidObjective,
        "candidate-capacity",
    )?;
    training(c)?;
    provenance(c)
}
fn topology(c: &ModelComponents) -> Result<()> {
    let a = &c.architecture;
    let top = &c.topology;
    require(
        top.nodes.len() == a.node_count as usize
            && top.edges.len() == a.edge_count as usize
            && top.relations.len() == a.relation_count as usize,
        E::InvalidTopology,
        "architecture-counts",
    )?;
    let mut degree = [0u32; 128];
    for edge in &top.edges {
        let source = top.nodes.binary_search_by_key(&edge.source, |n| n.id);
        let dest = top.nodes.binary_search_by_key(&edge.destination, |n| n.id);
        require(
            source.is_ok() && dest.is_ok() && edge.relation.0 < a.relation_count,
            E::InvalidTopology,
            "edge-endpoint-relation",
        )?;
        let slot = dest.expect("validated destination");
        degree[slot] += 1;
        require(
            degree[slot] <= a.max_incoming_degree,
            E::InvalidTopology,
            "incoming-degree",
        )?;
    }
    Ok(())
}

fn tensors(c: &ModelComponents) -> Result<()> {
    let a = &c.architecture;
    let top = &c.topology;
    let mut scalars = 0u64;
    for tensor in &c.tensors {
        for axis in &tensor.axes {
            let ok = match axis {
                Axis::Node(ids) => {
                    ids.len() == top.nodes.len()
                        && ids.iter().zip(&top.nodes).all(|(id, n)| *id == n.id)
                }
                Axis::Edge(ids) => {
                    ids.len() == top.edges.len()
                        && ids.iter().zip(&top.edges).all(|(id, e)| *id == e.id)
                }
                Axis::Relation(n) => *n == a.relation_count,
                Axis::Token(n) => *n == a.vocab_count,
                Axis::HiddenOut(n) | Axis::HiddenIn(n) => *n == a.hidden_width,
                Axis::Xyz => true,
            };
            require(ok, E::InvalidTensor, "axis-identity")?;
        }
        scalars += tensor.data_byte_length
            / if tensor.dtype == crate::model::contract::tensors::Dtype::F32 {
                4
            } else {
                8
            };
    }
    require(
        scalars <= u64::from(a.max_trainable_scalars),
        E::CapacityExceeded,
        "declared-scalar-cap",
    )?;
    Ok(())
}

fn training(c: &ModelComponents) -> Result<()> {
    let a = &c.architecture;
    require(
        matches!(&c.optimizer, OptimizerState::AdamW(_))
            == (a.purpose == Purpose::ResumableTraining),
        E::InvalidOptimizer,
        "purpose",
    )?;
    require(
        matches!(&c.rng_order, RngOrderState::StepBoundary(_))
            == (a.purpose == Purpose::ResumableTraining),
        E::InvalidRngOrder,
        "purpose",
    )?;
    if let (OptimizerState::AdamW(opt), RngOrderState::StepBoundary(rng)) =
        (&c.optimizer, &c.rng_order)
    {
        require(
            opt.moments.iter().zip(&c.tensors).all(|(m, t)| {
                m.scalar_count
                    == t.data_byte_length
                        / if t.dtype == crate::model::contract::tensors::Dtype::F32 {
                            4
                        } else {
                            8
                        }
            }),
            E::InvalidOptimizer,
            "moment-counts",
        )?;
        let obj = &c.objective;
        require(
            opt.successful_step == rng.successful_step
                && obj.dataset_manifest_digest == rng.dataset_manifest_digest
                && obj.split_manifest_digest == rng.split_manifest_digest
                && obj.preparation_digest == rng.preparation_digest
                && rng.batch_examples <= a.max_batch_examples,
            E::InvalidRngOrder,
            "state-crosslinks",
        )?;
        require(
            a.learning_profile != LearningProfile::OfflineExplicit
                || (rng.accepted_topology_proposals == 0 && rng.next_topology_proposal_step == 0),
            E::InvalidRngOrder,
            "offline-proposals",
        )?;
    }
    Ok(())
}

fn provenance(c: &ModelComponents) -> Result<()> {
    let a = &c.architecture;
    let top = &c.topology;
    let p = &c.provenance;
    let covered = |kind, digest| {
        p.data_roots
            .iter()
            .any(|root| root.kind == kind && root.content_digest == digest)
    };
    require(
        covered(
            DataRootKind::TrainingCorpus,
            c.tokenizer.training_corpus_digest,
        ) && covered(
            DataRootKind::SplitManifest,
            c.tokenizer.training_split_digest,
        ) && covered(
            DataRootKind::DatasetManifest,
            c.objective.dataset_manifest_digest,
        ) && covered(
            DataRootKind::SplitManifest,
            c.objective.split_manifest_digest,
        ) && covered(
            DataRootKind::PreparationManifest,
            c.objective.preparation_digest,
        ) && top.nodes.iter().all(|n| {
            n.source_ref
                .is_none_or(|d| covered(DataRootKind::SourceMemory, d))
        }),
        E::InvalidProvenance,
        "source-root-coverage",
    )?;
    require(
        if a.learning_profile == LearningProfile::OnlineV1 {
            p.policy_digest.is_some() && p.policy_epoch > 0
        } else {
            p.policy_digest.is_none() && p.policy_epoch == 0
        },
        E::InvalidProvenance,
        "policy-profile",
    )?;
    Ok(())
}
