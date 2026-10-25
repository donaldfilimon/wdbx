//! Native packages and deterministic byte BPE, with sealed validation stages.
//!
//! The owned reader, framing and semantic validation, exact writer/staging and
//! tokenizer implement N02. This module supplies no inference, neural training,
//! provider activation, selected generation or authorization service. Raw records
//! remain distinct from validated state; provenance does not confer permission.
//!
//! ```compile_fail
//! use abi_wdbx::model::ValidatedModelState;
//! let invalid = ValidatedModelState::default();
//! ```
//! ```compile_fail
//! use abi_wdbx::model::FramedPackage;
//! let invalid = FramedPackage {};
//! ```

pub mod contract;
pub mod error;
pub mod ids;
pub mod limits;
pub mod package;
pub mod tokenizer;

use contract::{DirectoryEntry, ModelComponents};
use ids::{ArtifactSha256, Digest, GenerationId, PackageDigest, StateRoot};

/// Owned bounded immutable snapshot, constructed only by the bounded reader.
#[derive(Debug)]
pub struct PackageBytes {
    bytes: Vec<u8>,
}
impl PackageBytes {
    /// Borrow the one owned snapshot; no mutable or cloning API is exposed.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Integrity-checked framing only; cannot construct execution or resume state.
#[derive(Debug)]
pub struct FramedPackage {
    snapshot: PackageBytes,
    directory: Vec<DirectoryEntry>,
    artifact_sha256: ArtifactSha256,
    package_digest: PackageDigest,
}
impl FramedPackage {
    /// Owned snapshot underlying every validated range and hash.
    #[must_use]
    pub fn snapshot(&self) -> &PackageBytes {
        &self.snapshot
    }
    /// All entries, including opaque optional annotation versions and ranges.
    #[must_use]
    pub fn directory(&self) -> &[DirectoryEntry] {
        &self.directory
    }
    /// Ordinary whole-file digest for external artifact verification.
    #[must_use]
    pub fn artifact_sha256(&self) -> ArtifactSha256 {
        self.artifact_sha256
    }
    /// Domain-separated package identity (not a semantic validity claim).
    #[must_use]
    pub fn package_digest(&self) -> PackageDigest {
        self.package_digest
    }
}

/// Complete semantic validity; private construction, no deserialization bypass.
/// This still grants no runtime qualification or caller authority.
#[derive(Debug)]
pub struct ValidatedModelState {
    framed: FramedPackage,
    components: ModelComponents,
    state_root: StateRoot,
    generation_id: GenerationId,
}
impl ValidatedModelState {
    /// Immutable hash-checked framing.
    #[must_use]
    pub fn framed(&self) -> &FramedPackage {
        &self.framed
    }
    /// Read-only coherent component data; cannot mutate through this view.
    #[must_use]
    pub fn components(&self) -> &ModelComponents {
        &self.components
    }
    /// Computational state identity, excluding provenance and annotations.
    #[must_use]
    pub fn state_root(&self) -> StateRoot {
        self.state_root
    }
    /// Provenance-bound generation identity, not a selected head or permission.
    #[must_use]
    pub fn generation_id(&self) -> GenerationId {
        self.generation_id
    }
}

/// External caller identities to compare before resume. No data is fetched.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumeContext {
    /// Exact admitted dataset manifest digest.
    pub dataset_manifest_digest: Digest,
    /// Exact admitted split manifest digest.
    pub split_manifest_digest: Digest,
    /// Exact admitted preparation identity.
    pub preparation_digest: Digest,
    /// Exact objective component identity.
    pub objective_digest: Digest,
    /// Caller policy context, checked against current authority by later integration.
    pub local_policy: LocalPolicyContext,
}

/// Context values are claims to check, never an authorization token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalPolicyContext {
    /// Current caller audience identity.
    pub audience_digest: Digest,
    /// Current policy identity; offline profile requires absent.
    pub policy_digest: Option<Digest>,
    /// Current policy epoch, zero only for offline profile.
    pub policy_epoch: u64,
}

/// Resume-context-matched state; caller authorization remains an external check.
/// There is no public constructor, Default, Deserialize or execution API.
#[derive(Debug)]
pub struct ResumableState {
    model: ValidatedModelState,
}
impl ResumableState {
    /// Read-only validated state; no implicit training or publication.
    #[must_use]
    pub fn model(&self) -> &ValidatedModelState {
        &self.model
    }
}
