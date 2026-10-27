//! Provenance claims are references, not authorization.
use super::{Cursor, Result};
use crate::model::{
    contract::provenance::{DataRoot, Origin, Provenance, SourceCohort},
    error::ErrorKind as E,
    ids::{Digest, GenerationId, ModelId, OperationId, PartitionId, StateRoot},
};
pub(super) fn parse(r: &mut Cursor<'_, '_>) -> Result<Provenance> {
    let model_id = ModelId(r.nonzero()?);
    let partition_id = PartitionId(r.nonzero()?);
    let origin = r.e8()?;
    let parent_generation = r.opt()?.map(GenerationId);
    let parent_state_root = r.opt()?.map(StateRoot);
    let derivation_generation = r.opt()?.map(GenerationId);
    let operation_id = r.opt()?.map(OperationId);
    let audience_digest = r.digest()?;
    let policy_digest = r.opt()?.map(Digest);
    let policy_epoch = r.u64()?;
    let n = r.count(32)?;
    let mut source_cohorts = r.vec(n)?;
    for _ in 0..n {
        source_cohorts.push(SourceCohort {
            name: r.string(4096)?,
            revision: r.string(4096)?,
            source_tree_digest: r.digest()?,
        });
    }
    let n = r.count(1024)?;
    let mut data_roots = r.vec(n)?;
    for _ in 0..n {
        data_roots.push(DataRoot {
            content_digest: r.digest()?,
            consent_or_eligibility_digest: r.digest()?,
            kind: r.e8()?,
        });
    }
    let n = r.count(64)?;
    let mut license_digests = r.vec(n)?;
    for _ in 0..n {
        license_digests.push(r.digest()?);
    }
    let validation_report_digest = r.opt()?.map(Digest);
    let restrictions = r.u32()?;
    r.end()?;
    r.check(
        parent_generation.is_some() == parent_state_root.is_some()
            && (parent_generation.is_none() || operation_id.is_some()),
        E::InvalidProvenance,
        "lineage",
    )?;
    r.check(
        source_cohorts
            .iter()
            .all(|c| !c.name.is_empty() && !c.revision.is_empty())
            && source_cohorts
                .windows(2)
                .all(|w| w[0].name.as_bytes() < w[1].name.as_bytes()),
        E::InvalidProvenance,
        "source-cohorts",
    )?;
    r.check(
        data_roots.windows(2).all(|w| {
            (w[0].kind as u8, w[0].content_digest) < (w[1].kind as u8, w[1].content_digest)
        }) && license_digests.windows(2).all(|w| w[0] < w[1]),
        E::InvalidProvenance,
        "root-license-order",
    )?;
    r.check(
        restrictions <= 3
            && (origin != Origin::Diagnostic || restrictions & 1 == 1)
            && (origin == Origin::Diagnostic || !license_digests.is_empty()),
        E::InvalidProvenance,
        "origin-restrictions",
    )?;
    Ok(Provenance {
        model_id,
        partition_id,
        origin,
        parent_generation,
        parent_state_root,
        derivation_generation,
        operation_id,
        audience_digest,
        policy_digest,
        policy_epoch,
        source_cohorts,
        data_roots,
        license_digests,
        validation_report_digest,
        restrictions,
    })
}
