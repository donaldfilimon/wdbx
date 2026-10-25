//! Typed unvalidated schema records. Rust layout is never the wire encoding.
//! All counts, scalar validity and cross-links still require N02 validation.

pub mod architecture;
pub mod provenance;
pub mod tensors;
pub mod tokenizer;
pub mod topology;
pub mod training;

use super::ids::Digest;
use architecture::Architecture;
use provenance::Provenance;
use tensors::Tensor;
use tokenizer::Tokenizer;
use topology::Topology;
use training::{Objective, OptimizerState, RngOrderState};

/// Exact container magic.
pub const MAGIC: [u8; 8] = *b"WDBXMOD1";
/// Package identity, distinct from every historical WDBX format.
pub const PACKAGE_ID: &str = "wdbx-model-package-v1";
/// Supported native architecture.
pub const ARCHITECTURE_ID: &str = "wdbx-spatial-neural-v1";
/// Exact scalar numerical profile; no device/backend claim.
pub const NUMERICAL_PROFILE_ID: &str = "cpu-scalar-f32params-f64accum-v1";
/// Byte BPE identity.
pub const TOKENIZER_ID: &str = "wdbx-byte-bpe-v1";
/// Explicit online-learning profile identity, subject to external policy.
pub const ONLINE_LEARNING_ID: &str = "wdbx-online-learning-v1";
/// Whole-package content identity domain including its terminator.
pub const PACKAGE_DOMAIN: &[u8] = b"wdbx-model-package-v1\0";
/// Computational state identity domain including its terminator.
pub const STATE_DOMAIN: &[u8] = b"wdbx-model-state-v1\0";
/// Provenance-bound generation identity domain including its terminator.
pub const GENERATION_DOMAIN: &[u8] = b"wdbx-model-generation-v1\0";
/// Ordered training-document corpus domain including its terminator.
pub const BPE_CORPUS_DOMAIN: &[u8] = b"wdbx-bpe-corpus-v1\0";
/// Required causal-text plus graph-ranking feature bits, with no other bits.
pub const REQUIRED_FEATURES: u64 = 3;
/// Directory flag bit 0; all recognized components require exactly this flag.
pub const REQUIRED_COMPONENT: u32 = 1;

/// Explicit version pair. Higher minor versions are not implicitly compatible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Version {
    /// Major version.
    pub major: u16,
    /// Minor version.
    pub minor: u16,
}
/// The only supported container and recognized component version.
pub const V1: Version = Version { major: 1, minor: 0 };

/// One independently versioned mandatory schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentSpec {
    /// Directory kind.
    pub kind: u32,
    /// Human-readable schema name, never executable dispatch.
    pub name: &'static str,
    /// Exact supported version.
    pub version: Version,
}
/// All eight components are required even when optimizer/RNG state is absent.
pub const COMPONENT_CATALOG: [ComponentSpec; 8] = [
    ComponentSpec {
        kind: 1,
        name: "architecture",
        version: V1,
    },
    ComponentSpec {
        kind: 2,
        name: "topology",
        version: V1,
    },
    ComponentSpec {
        kind: 3,
        name: "tensors",
        version: V1,
    },
    ComponentSpec {
        kind: 4,
        name: "tokenizer",
        version: V1,
    },
    ComponentSpec {
        kind: 5,
        name: "objective",
        version: V1,
    },
    ComponentSpec {
        kind: 6,
        name: "optimizer",
        version: V1,
    },
    ComponentSpec {
        kind: 7,
        name: "rng-order",
        version: V1,
    },
    ComponentSpec {
        kind: 8,
        name: "provenance",
        version: V1,
    },
];

/// Unvalidated 96-byte header fields, in exact wire order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Header {
    /// Exactly MAGIC.
    pub magic: [u8; 8],
    /// Exactly (1,0).
    pub version: Version,
    /// Exactly 96.
    pub header_bytes: u32,
    /// Exact file size, with no trailing bytes.
    pub total_bytes: u64,
    /// 8..=32.
    pub component_count: u32,
    /// Exactly zero.
    pub flags: u32,
    /// SHA-256 over directory bytes as stored.
    pub directory_sha256: Digest,
    /// All zero.
    pub reserved: [u8; 32],
}

/// Unvalidated 80-byte directory entry fields, in wire order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryEntry {
    /// Strictly increasing nonzero kind. Unknown optional kinds remain opaque.
    pub kind: u32,
    /// Recognized components require (1,0); optional unknown versions retained.
    pub version: Version,
    /// Bit 0 required, all others zero.
    pub flags: u32,
    /// Zero.
    pub reserved: u32,
    /// Exact next contiguous payload position.
    pub offset: u64,
    /// Strictly positive payload byte count.
    pub length: u64,
    /// Ordinary SHA-256 of the component bytes.
    pub component_sha256: Digest,
    /// All zero.
    pub reserved_tail: [u8; 16],
}

/// Decoded component records are not a validated model and cannot execute.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelComponents {
    /// Component 1.
    pub architecture: Architecture,
    /// Component 2.
    pub topology: Topology,
    /// Component 3 (exactly sixteen members).
    pub tensors: Vec<Tensor>,
    /// Component 4.
    pub tokenizer: Tokenizer,
    /// Component 5.
    pub objective: Objective,
    /// Component 6, including explicit absent state.
    pub optimizer: OptimizerState,
    /// Component 7, including explicit absent state.
    pub rng_order: RngOrderState,
    /// Component 8; never confers authority.
    pub provenance: Provenance,
}
