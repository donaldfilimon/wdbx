//! Stable error families with bounded context and no payload disclosure.

/// Deterministic validation failure family.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// Io: see the native package 1.0 contract.
    Io,
    /// Truncated: see the native package 1.0 contract.
    Truncated,
    /// `TrailingBytes`: see the native package 1.0 contract.
    TrailingBytes,
    /// `BadMagic`: see the native package 1.0 contract.
    BadMagic,
    /// `UnsupportedContainerVersion`: see the native package 1.0 contract.
    UnsupportedContainerVersion,
    /// `UnsupportedRequiredComponent`: see the native package 1.0 contract.
    UnsupportedRequiredComponent,
    /// `UnsupportedComponentVersion`: see the native package 1.0 contract.
    UnsupportedComponentVersion,
    /// `NonCanonical`: see the native package 1.0 contract.
    NonCanonical,
    /// `IntegrityMismatch`: see the native package 1.0 contract.
    IntegrityMismatch,
    /// `MissingComponent`: see the native package 1.0 contract.
    MissingComponent,
    /// `CapacityExceeded`: see the native package 1.0 contract.
    CapacityExceeded,
    /// `InvalidTopology`: see the native package 1.0 contract.
    InvalidTopology,
    /// `InvalidTensor`: see the native package 1.0 contract.
    InvalidTensor,
    /// `InvalidTokenizer`: see the native package 1.0 contract.
    InvalidTokenizer,
    /// `InvalidObjective`: see the native package 1.0 contract.
    InvalidObjective,
    /// `InvalidOptimizer`: see the native package 1.0 contract.
    InvalidOptimizer,
    /// `InvalidRngOrder`: see the native package 1.0 contract.
    InvalidRngOrder,
    /// `InvalidProvenance`: see the native package 1.0 contract.
    InvalidProvenance,
    /// `NotResumable`: see the native package 1.0 contract.
    NotResumable,
    /// `ResumeMismatch`: see the native package 1.0 contract.
    ResumeMismatch,
    /// Cancelled: see the native package 1.0 contract.
    Cancelled,
}
/// A field name is static, and context never embeds artifact/source contents.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{kind:?}: {field} (component {component:?}, offset {offset:?})")]
pub struct ModelError {
    /// Stable family.
    pub kind: ErrorKind,
    /// Static schema field name.
    pub field: &'static str,
    /// Component kind if known.
    pub component: Option<u32>,
    /// Bounded offset into the admitted snapshot if known.
    pub offset: Option<u64>,
}
impl ModelError {
    /// Construct payload-free diagnostic context.
    #[must_use]
    pub const fn new(
        kind: ErrorKind,
        field: &'static str,
        component: Option<u32>,
        offset: Option<u64>,
    ) -> Self {
        Self {
            kind,
            field,
            component,
            offset,
        }
    }
}
