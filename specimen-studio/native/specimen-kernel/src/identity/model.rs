use crate::language::Expr;

/// Host-generated concrete node identity. No encoder ID is a concrete reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeRef(pub u128);
/// Host-generated concrete entry identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntryRef(pub u128);
/// Host-generated concrete supporting-resource identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResourceRef(pub u128);
/// Optional logical resource link. Multiple concrete resources may share it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceId(pub String);

/// Population isolation supplied by the host; this is not authentication.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Population {
    pub tenant: String,
    pub specimen: String,
    pub node_type: String,
}
impl Population {
    pub fn new(tenant: &str, specimen: &str, node_type: &str) -> Self {
        Self {
            tenant: tenant.into(),
            specimen: specimen.into(),
            node_type: node_type.into(),
        }
    }
}

/// Complete caller-prepared structural key, including the full SHA-256 digest.
/// This table neither computes an encoder nor interprets existing text-v2 IDs.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PatternKey {
    pub encoder_version: String,
    pub modality: String,
    pub resolution: String,
    pub digest: [u8; 32],
}
impl PatternKey {
    pub fn new(encoder_version: &str, modality: &str, resolution: &str, digest: [u8; 32]) -> Self {
        Self {
            encoder_version: encoder_version.into(),
            modality: modality.into(),
            resolution: resolution.into(),
            digest,
        }
    }
}

/// Prepared action alternative, preserving slot order, inhibition and exact weight bits.
#[derive(Clone, Debug)]
pub struct Alternative {
    pub action: Vec<Expr>,
    pub inhibited: bool,
    pub weight: f64,
}
impl PartialEq for Alternative {
    fn eq(&self, other: &Self) -> bool {
        self.inhibited == other.inhibited
            && self.weight.to_bits() == other.weight.to_bits()
            && trees_equal(&self.action, &other.action)
    }
}
impl Eq for Alternative {}

/// Prepared trees; lexical spelling normalization belongs to the later parser.
/// Equality compares typed structures directly, never serialized JSON.
#[derive(Clone, Debug, Default)]
pub struct ActionAst {
    pub bind: Vec<Expr>,
    pub alternatives: Vec<Alternative>,
}
impl PartialEq for ActionAst {
    fn eq(&self, other: &Self) -> bool {
        trees_equal(&self.bind, &other.bind) && self.alternatives == other.alternatives
    }
}
impl Eq for ActionAst {}
fn trees_equal(a: &[Expr], b: &[Expr]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| match (a, b) {
            (Expr::Text { value: a }, Expr::Text { value: b }) => a == b,
            (Expr::Call { name: a, args: aa }, Expr::Call { name: b, args: ba }) => {
                a == b && aa.len() == ba.len() && aa.iter().zip(ba).all(|(a, b)| trees_equal(a, b))
            }
            _ => false,
        })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeRecord {
    pub population: Population,
    pub reference: NodeRef,
    pub key: PatternKey,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryRecord {
    pub population: Population,
    pub node: NodeRef,
    pub reference: EntryRef,
    pub key: PatternKey,
    /// Original UTF-8 bytes, never normalized by this module.
    pub original: String,
    pub ast: ActionAst,
    /// Logical link, independent of structural keys and concrete refs.
    pub resource: Option<ResourceId>,
}
impl EntryRecord {
    pub(super) fn same_semantics(&self, other: &Self) -> bool {
        self.population == other.population
            && self.original == other.original
            && self.ast == other.ast
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceRecord {
    pub population: Population,
    pub reference: ResourceRef,
    pub key: PatternKey,
    pub original: String,
    pub link: Option<ResourceId>,
}

/// Owned records in insertion order. Internal rebuild input, not a published wire format.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TableSnapshot {
    pub nodes: Vec<NodeRecord>,
    pub entries: Vec<EntryRecord>,
    pub resources: Vec<ResourceRecord>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertOutcome {
    Inserted { node: NodeRef, entry: EntryRef },
    Duplicate { node: NodeRef, entry: EntryRef },
    Full,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum IdentityError {
    #[error("Concrete reference is already in use")]
    DuplicateRef,
    #[error("Node population does not match")]
    PopulationMismatch,
    #[error("Node key does not match")]
    KeyMismatch,
    #[error("Entry does not belong to the supplied node")]
    NodeMismatch,
    #[error("Entry node is absent")]
    MissingNode,
    #[error("Node exceeds twenty entries")]
    NodeCapacity,
    #[error("Duplicate semantic entry in a population")]
    DuplicateEntry,
    #[error("Alternative weight must be finite")]
    InvalidWeight,
    #[error("Prepared tree exceeds existing language bounds or contains an invalid call")]
    InvalidAst,
    #[error("Original exceeds two thousand characters")]
    PatternTooLong,
    #[error("Supporting resource limit exceeded")]
    ResourceLimit,
}
