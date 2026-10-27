//! Distinct content, lineage, stable-axis and coordinate identities.
//! Raw identity wrappers are unvalidated data, never authority or admission.

macro_rules! identity {
    ($name:ident, $inner:ty, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name(pub $inner);
    };
}
identity!(NodeId, u64, "Persistent node axis, never a dense slot.");
identity!(EdgeId, u64, "Persistent edge axis, never reduction order.");
identity!(RelationId, u32, "Immutable semantic relation axis.");
identity!(
    TokenId,
    u32,
    "Vocabulary row identity, including structural specials."
);
identity!(ModelId, [u8; 16], "Opaque model identity (wire I).");
identity!(
    OperationId,
    [u8; 16],
    "Opaque lineage operation identity (wire I)."
);
identity!(
    Digest,
    [u8; 32],
    "Digest reference; possession never grants authority."
);
identity!(
    PartitionId,
    [u8; 32],
    "Partition binding, independent of numerical state."
);
identity!(
    PackageDigest,
    [u8; 32],
    "Domain-separated whole-package identity."
);
identity!(
    ArtifactSha256,
    [u8; 32],
    "Ordinary SHA-256 of the exact whole artifact."
);
identity!(
    StateRoot,
    [u8; 32],
    "Computational identity of required components 1 through 7."
);
identity!(
    GenerationId,
    [u8; 32],
    "State, provenance, model and partition identity; not a head pointer."
);

/// Learned unitless coordinates; wire values occur only in tensor 2.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeuralPosition(pub [f64; 3]);
/// Physical coordinates are never implicitly converted into learned state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalPosition(pub [f64; 3]);
/// Presentation coordinates are never implicitly converted into learned state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayPosition(pub [f64; 3]);
