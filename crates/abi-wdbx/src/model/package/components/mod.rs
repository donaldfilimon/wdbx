//! Eight closed schemas; no decoding or constructor bypass.
mod cross;
mod cursor;
mod manifest;
mod objective;
mod optimizer;
mod provenance;
mod rng;
mod tensors;
mod tokenizer;
mod topology;
use super::{Result, err, require};
use crate::model::{
    FramedPackage, ResumableState, ResumeContext, ValidatedModelState,
    contract::{GENERATION_DOMAIN, ModelComponents, STATE_DOMAIN, architecture::Purpose},
    error::{ErrorKind as E, ModelError},
    ids::{GenerationId, StateRoot},
    limits::LoadLimits,
};
use cursor::{Budget, Cursor};
use sha2::{Digest as _, Sha256};
/// Decode supported schemas and all cross-component relationships on owned bytes.
pub fn validate_model(framed: FramedPackage, limits: LoadLimits) -> Result<ValidatedModelState> {
    let limits = limits.effective();
    let mut budget = Budget::new(framed.snapshot().as_bytes().len() as u64, limits)?;
    macro_rules! parse {
        ($kind:literal,$parser:ident) => {{
            let e = &framed.directory()[($kind - 1) as usize];
            let bytes = &framed.snapshot().as_bytes()
                [super::size(e.offset)?..super::size(e.offset + e.length)?];
            $parser::parse(&mut Cursor::new(bytes, $kind, e.offset, &mut budget))
                .map_err(|error| with_component(error, $kind, e.offset))?
        }};
    }
    // Explicit scopes preserve deterministic schema order and cumulative accounting.
    let architecture = parse!(1, manifest);
    let topology = parse!(2, topology);
    let tensors = parse!(3, tensors);
    let tokenizer = parse!(4, tokenizer);
    let objective = parse!(5, objective);
    let optimizer = parse!(6, optimizer);
    let rng_order = parse!(7, rng);
    let provenance = parse!(8, provenance);
    for e in &framed.directory()[8..] {
        budget
            .metadata(e.length)
            .map_err(|error| with_component(error, e.kind, e.offset))?;
    }
    let components = ModelComponents {
        architecture,
        topology,
        tensors,
        tokenizer,
        objective,
        optimizer,
        rng_order,
        provenance,
    };
    cross::validate(&components).map_err(|error| {
        let kind = match error.kind {
            E::InvalidTopology => 2,
            E::InvalidTensor => 3,
            E::InvalidTokenizer => 4,
            E::InvalidObjective => 5,
            E::InvalidOptimizer => 6,
            E::InvalidRngOrder => 7,
            E::InvalidProvenance => 8,
            E::CapacityExceeded => 1,
            _ => return error,
        };
        with_component(error, kind, framed.directory()[(kind - 1) as usize].offset)
    })?;
    let mut hash = Sha256::new();
    hash.update(STATE_DOMAIN);
    for entry in &framed.directory()[..7] {
        hash.update(entry.kind.to_le_bytes());
        hash.update(1u16.to_le_bytes());
        hash.update(0u16.to_le_bytes());
        hash.update(entry.component_sha256.0);
    }
    let state_root = StateRoot(hash.finalize().into());
    let p = &components.provenance;
    let mut hash = Sha256::new();
    hash.update(GENERATION_DOMAIN);
    hash.update(p.model_id.0);
    hash.update(p.partition_id.0);
    if let Some(parent) = p.parent_generation {
        hash.update([1]);
        hash.update(parent.0);
    } else {
        hash.update([0]);
    }
    hash.update(state_root.0);
    hash.update(framed.directory()[7].component_sha256.0);
    let generation_id = GenerationId(hash.finalize().into());
    Ok(ValidatedModelState {
        framed,
        components,
        state_root,
        generation_id,
    })
}
/// Compare caller-admitted identities. This is not a credential or an admission grant.
pub fn check_resume(model: ValidatedModelState, context: &ResumeContext) -> Result<ResumableState> {
    require(
        model.components.architecture.purpose == Purpose::ResumableTraining,
        E::NotResumable,
        "purpose",
    )?;
    let obj = &model.components.objective;
    let p = &model.components.provenance;
    require(
        context.dataset_manifest_digest == obj.dataset_manifest_digest
            && context.split_manifest_digest == obj.split_manifest_digest
            && context.preparation_digest == obj.preparation_digest
            && context.objective_digest == model.framed.directory()[4].component_sha256
            && context.local_policy.audience_digest == p.audience_digest
            && context.local_policy.policy_digest == p.policy_digest
            && context.local_policy.policy_epoch == p.policy_epoch,
        E::ResumeMismatch,
        "caller-context",
    )?;
    Ok(ResumableState { model })
}

fn with_component(mut error: ModelError, kind: u32, base: u64) -> ModelError {
    error.component.get_or_insert(kind);
    error.offset.get_or_insert(base);
    error
}
