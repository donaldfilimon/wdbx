//! Component 8: references constrain provenance; none confer source authority.
use crate::model::ids::{Digest, GenerationId, ModelId, OperationId, PartitionId, StateRoot};

/// Provenance class is not quality qualification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Origin {
    /// Requires diagnostic-only restriction; unknown licensing remains unqualified.
    Diagnostic = 0,
    /// Requires nonempty license references and external authentication.
    Imported = 1,
    /// Requires nonempty licenses; training claim needs external evidence.
    LocallyTrained = 2,
}
impl TryFrom<u8> for Origin {
    type Error = super::super::error::ModelError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Diagnostic),
            1 => Ok(Self::Imported),
            2 => Ok(Self::LocallyTrained),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "Origin",
                None,
                None,
            )),
        }
    }
}

/// External admitted-data reference kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DataRootKind {
    /// Tokenizer training corpus.
    TrainingCorpus = 0,
    /// Objective dataset manifest.
    DatasetManifest = 1,
    /// Tokenizer/objective split manifests.
    SplitManifest = 2,
    /// Topology Memory source references.
    SourceMemory = 3,
    /// Objective preparation manifest.
    PreparationManifest = 4,
}
impl TryFrom<u8> for DataRootKind {
    type Error = super::super::error::ModelError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::TrainingCorpus),
            1 => Ok(Self::DatasetManifest),
            2 => Ok(Self::SplitManifest),
            3 => Ok(Self::SourceMemory),
            4 => Ok(Self::PreparationManifest),
            _ => Err(super::super::error::ModelError::new(
                super::super::error::ErrorKind::NonCanonical,
                "DataRootKind",
                None,
                None,
            )),
        }
    }
}

/// Restriction bit 0; mandatory for Diagnostic origin.
pub const DIAGNOSTIC_ONLY: u32 = 1;
/// Restriction bit 1; restrictions may reduce but never expand permissions.
pub const NO_EXTERNAL_EXPORT: u32 = 2;
/// All accepted restriction bits.
pub const KNOWN_RESTRICTIONS: u32 = 3;

/// Source reference, never an executable location.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceCohort {
    /// Nonempty UTF-8; names strictly increasing by bytes.
    pub name: String,
    /// Nonempty display-only revision.
    pub revision: String,
    /// Nonzero source-tree digest.
    pub source_tree_digest: Digest,
}

/// Root key is (kind, `content_digest`), strictly sorted and unique.
#[derive(Clone, Debug, PartialEq)]
pub struct DataRoot {
    /// Nonzero content identity.
    pub content_digest: Digest,
    /// Nonzero evidence reference, not an embedded permission grant.
    pub consent_or_eligibility_digest: Digest,
    /// Closed root vocabulary.
    pub kind: DataRootKind,
}

/// Unvalidated component 8 in wire order; external authority is always required.
#[derive(Clone, Debug, PartialEq)]
pub struct Provenance {
    /// Nonzero opaque model identity.
    pub model_id: ModelId,
    /// Nonzero audience partition identity.
    pub partition_id: PartitionId,
    /// Diagnostic, imported or locally trained.
    pub origin: Origin,
    /// Present iff `parent_state_root` is present.
    pub parent_generation: Option<GenerationId>,
    /// Parent pair requires `operation_id`.
    pub parent_state_root: Option<StateRoot>,
    /// Optional source of an inference export, never own generation.
    pub derivation_generation: Option<GenerationId>,
    /// Nonzero when present; required with parent pair.
    pub operation_id: Option<OperationId>,
    /// Nonzero claimed audience; external verification required.
    pub audience_digest: Digest,
    /// Online requires present, offline requires absent.
    pub policy_digest: Option<Digest>,
    /// Online >0, offline exactly zero.
    pub policy_epoch: u64,
    /// At most 32.
    pub source_cohorts: Vec<SourceCohort>,
    /// At most 1024; covers all tokenizer/objective/source references.
    pub data_roots: Vec<DataRoot>,
    /// At most 64; strictly increasing nonzero references.
    pub license_digests: Vec<Digest>,
    /// Prior external report; cannot recursively qualify its own package.
    pub validation_report_digest: Option<Digest>,
    /// Only bits 0 and 1; never grants authority.
    pub restrictions: u32,
}
