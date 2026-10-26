//! Schema serialization shares no implementation with the independent witness.
use super::{Encoder, Result};
use crate::model::contract::{
    ModelComponents,
    tensors::{Axis, TensorValues},
    training::{OptimizerState, RngOrderState},
};
macro_rules! fields {($e:ident,$method:ident,$record:ident,$($field:ident),+)=>{$($e.$method($record.$field)?;)+}}
pub(super) fn component(kind: u32, c: &ModelComponents, e: &mut Encoder) -> Result<()> {
    match kind {
        1 => architecture(c, e),
        2 => topology(c, e),
        3 => tensors(c, e),
        4 => tokenizer(c, e),
        5 => objective(c, e),
        6 => optimizer(c, e),
        7 => rng_order(c, e),
        8 => provenance(c, e),
        _ => unreachable!("closed required catalog"),
    }
}
fn architecture(c: &ModelComponents, e: &mut Encoder) -> Result<()> {
    let a = &c.architecture;
    e.string(&a.architecture)?;
    e.string(&a.numerical_profile)?;
    e.u8(a.purpose as u8)?;
    e.u8(a.learning_profile as u8)?;
    e.u16(a.operator_set as u16)?;
    e.u64(a.required_features)?;
    fields!(
        e,
        u32,
        a,
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
        max_trainable_scalars
    );
    fields!(
        e,
        u64,
        a,
        max_session_working_bytes,
        max_loaded_and_trainer_bytes,
        max_training_scalar_ops
    );
    fields!(e, f64, a, spatial_scale, coordinate_radius);

    Ok(())
}

fn topology(c: &ModelComponents, e: &mut Encoder) -> Result<()> {
    let t = &c.topology;
    e.u16(t.coordinate_frame as u16)?;
    fields!(e, u64, t, node_id_high_water, edge_id_high_water);
    e.count(t.relations.len())?;
    for r in &t.relations {
        e.u32(r.id.0)?;
        e.string(&r.label)?;
    }
    e.count(t.nodes.len())?;
    for n in &t.nodes {
        e.u64(n.id.0)?;
        e.u8(n.role as u8)?;
        e.opt(n.source_ref.map(|d| d.0))?;
    }
    e.count(t.edges.len())?;
    for edge in &t.edges {
        e.u64(edge.id.0)?;
        e.u64(edge.source.0)?;
        e.u64(edge.destination.0)?;
        e.u32(edge.relation.0)?;
    }

    Ok(())
}

fn tensors(c: &ModelComponents, e: &mut Encoder) -> Result<()> {
    e.count(c.tensors.len())?;
    for t in &c.tensors {
        e.u16(t.id as u16)?;
        e.u8(t.dtype as u8)?;
        e.u8(t.rank)?;
        for a in &t.axes {
            match a {
                Axis::Node(ids) => {
                    e.u8(1)?;
                    e.count(ids.len())?;
                    for id in ids {
                        e.u64(id.0)?;
                    }
                }
                Axis::Edge(ids) => {
                    e.u8(2)?;
                    e.count(ids.len())?;
                    for id in ids {
                        e.u64(id.0)?;
                    }
                }
                Axis::Relation(n) => {
                    e.u8(3)?;
                    e.u32(*n)?;
                }
                Axis::Token(n) => {
                    e.u8(4)?;
                    e.u32(*n)?;
                }
                Axis::HiddenOut(n) => {
                    e.u8(5)?;
                    e.u32(*n)?;
                }
                Axis::HiddenIn(n) => {
                    e.u8(6)?;
                    e.u32(*n)?;
                }
                Axis::Xyz => {
                    e.u8(7)?;
                    e.u32(3)?;
                }
            }
        }
        e.u64(t.data_byte_length)?;
        match &t.data {
            TensorValues::F32(v) => {
                for f in v {
                    e.numeric(&f.to_le_bytes())?;
                }
            }
            TensorValues::F64(v) => {
                for f in v {
                    e.numeric(&f.to_le_bytes())?;
                }
            }
        }
    }

    Ok(())
}

fn tokenizer(c: &ModelComponents, e: &mut Encoder) -> Result<()> {
    let t = &c.tokenizer;
    e.string(&t.identity)?;
    e.u8(t.normalization as u8)?;
    e.u8(t.special_policy as u8)?;
    e.u16(t.reserved)?;
    e.u32(t.pad_id.0)?;
    e.u32(t.bos_id.0)?;
    e.u32(t.eos_id.0)?;
    e.u32(t.unk_id.0)?;
    e.bytes(&t.training_corpus_digest.0)?;
    e.bytes(&t.training_split_digest.0)?;
    fields!(e, u32, t, minimum_pair_frequency, requested_vocab_size);
    e.count(t.vocabulary.len())?;
    for v in &t.vocabulary {
        e.vocabulary_blob(v)?;
    }
    e.count(t.merges.len())?;
    for m in &t.merges {
        e.u32(m.left.0)?;
        e.u32(m.right.0)?;
        e.u32(m.result.0)?;
    }

    Ok(())
}

fn objective(c: &ModelComponents, e: &mut Encoder) -> Result<()> {
    let o = &c.objective;
    e.u16(o.objective_profile as u16)?;
    e.u16(o.text_mask_profile as u16)?;
    e.u16(o.text_reduction as u16)?;
    e.u16(o.graph_reduction as u16)?;
    fields!(e, f64, o, text_weight, graph_weight, coordinate_l2_weight);
    e.u16(o.sequence_policy as u16)?;
    e.u16(o.graph_candidate_policy as u16)?;
    e.u32(o.max_graph_candidates)?;
    e.bytes(&o.dataset_manifest_digest.0)?;
    e.bytes(&o.split_manifest_digest.0)?;
    e.bytes(&o.preparation_digest.0)?;

    Ok(())
}

fn optimizer(c: &ModelComponents, e: &mut Encoder) -> Result<()> {
    if let OptimizerState::AdamW(o) = &c.optimizer {
        e.u8(1)?;
        e.u16(o.optimizer_profile as u16)?;
        e.u16(o.scheduler_profile as u16)?;
        e.u64(o.successful_step)?;
        fields!(
            e,
            f64,
            o,
            learning_rate,
            beta1,
            beta2,
            epsilon,
            weight_decay,
            gradient_clip_norm
        );
        e.count(o.moments.len())?;
        for m in &o.moments {
            e.u16(m.tensor_id as u16)?;
            e.u64(m.scalar_count)?;
            for f in &m.first_moment {
                e.numeric(&f.to_le_bytes())?;
            }
            for f in &m.second_moment {
                e.numeric(&f.to_le_bytes())?;
            }
        }
    } else {
        e.u8(0)?;
    }

    Ok(())
}

fn rng_order(c: &ModelComponents, e: &mut Encoder) -> Result<()> {
    if let RngOrderState::StepBoundary(r) = &c.rng_order {
        e.u8(1)?;
        e.u16(r.rng_profile as u16)?;
        e.u16(r.seed_profile as u16)?;
        e.u64(r.root_seed)?;
        e.count(r.streams.len())?;
        for s in &r.streams {
            e.u8(s.stream_id as u8)?;
            for word in s.state_words {
                e.u64(word)?;
            }
            e.u64(s.consumed_u64)?;
        }
        e.u16(r.sampler_profile as u16)?;
        e.u16(r.batch_policy as u16)?;
        e.u16(r.pending_state as u16)?;
        e.bytes(&r.dataset_manifest_digest.0)?;
        e.bytes(&r.split_manifest_digest.0)?;
        e.bytes(&r.preparation_digest.0)?;
        fields!(e, u64, r, successful_step, epoch);
        fields!(
            e,
            u32,
            r,
            example_count,
            next_example_cursor,
            batch_examples
        );
        e.count(r.permutation.len())?;
        for i in &r.permutation {
            e.u32(*i)?;
        }
        fields!(
            e,
            u64,
            r,
            accepted_topology_proposals,
            next_topology_proposal_step
        );
    } else {
        e.u8(0)?;
    }

    Ok(())
}

fn provenance(c: &ModelComponents, e: &mut Encoder) -> Result<()> {
    let p = &c.provenance;
    e.bytes(&p.model_id.0)?;
    e.bytes(&p.partition_id.0)?;
    e.u8(p.origin as u8)?;
    e.opt(p.parent_generation.map(|d| d.0))?;
    e.opt(p.parent_state_root.map(|d| d.0))?;
    e.opt(p.derivation_generation.map(|d| d.0))?;
    e.opt(p.operation_id.map(|d| d.0))?;
    e.bytes(&p.audience_digest.0)?;
    e.opt(p.policy_digest.map(|d| d.0))?;
    e.u64(p.policy_epoch)?;
    e.count(p.source_cohorts.len())?;
    for s in &p.source_cohorts {
        e.string(&s.name)?;
        e.string(&s.revision)?;
        e.bytes(&s.source_tree_digest.0)?;
    }
    e.count(p.data_roots.len())?;
    for r in &p.data_roots {
        e.bytes(&r.content_digest.0)?;
        e.bytes(&r.consent_or_eligibility_digest.0)?;
        e.u8(r.kind as u8)?;
    }
    e.count(p.license_digests.len())?;
    for d in &p.license_digests {
        e.bytes(&d.0)?;
    }
    e.opt(p.validation_report_digest.map(|d| d.0))?;
    e.u32(p.restrictions)?;

    Ok(())
}
