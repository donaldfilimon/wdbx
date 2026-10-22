//! The studio's semantic operations and their canonical content form.
//!
//! The canonical episode ledger commits to a payload; it never stores one.
//! The studio's specimen therefore stays in the studio's own store, and what
//! the ledger holds is the studio's *episodic semantics*: a memory candidate
//! whose payload commitment is the SHA-256 over the studio's canonical
//! content bytes.
//!
//! The canonical content form is the studio's own encoding of its own JSON:
//! object members ordered by their UTF-8 bytes, no insignificant whitespace,
//! and the standard JSON escaping. It is not a canonical commitment, and the
//! substrate never sees these bytes: it receives a 32-byte digest and a byte
//! count, and it derives the episode commitment itself.

use crate::{Result, error};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// The greatest length of the studio's own request identifier.
///
/// The adapter derives the substrate's operation identifier from the request
/// identifier by prefixing `op_`, and the substrate bounds both identifiers
/// at 64 characters. The studio's own bound leaves room for that prefix and
/// is checked when the operation is built, so an over-long identifier is
/// refused by the studio and never reaches the substrate.
pub const MAX_REQUEST_ID_LEN: usize = 60;

/// One studio operation the canonical ledger may record.
///
/// The operation is built through [`CanonicalOperation::new`], which refuses
/// a request identifier the studio cannot bound. The fields are private for
/// that reason: an operation that was never checked is not one the studio
/// may offer to the ledger.
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalOperation {
    /// The studio's idempotency key for this operation.
    ///
    /// The substrate refuses a repeated request identifier as a replay, so
    /// the same key always means the same request.
    request_id: String,
    /// What the studio wants the ledger to record.
    kind: OperationKind,
}

impl CanonicalOperation {
    /// Build one studio operation, refusing a request identifier the studio
    /// cannot bound.
    ///
    /// The request identifier is the studio's idempotency key, and the
    /// adapter derives the substrate's operation identifier from it by
    /// prefixing `op_`. The substrate bounds both identifiers at 64
    /// characters over its own character set, so a 62-character request
    /// identifier would become a 65-character operation identifier and be
    /// refused by the substrate as an invalid input: a late and confusing
    /// failure. The studio bounds the request identifier at
    /// [`MAX_REQUEST_ID_LEN`] characters instead, one below the substrate's
    /// own limit, and checks it here, at construction: the refusal is the
    /// studio's own, it is content-free, and it arrives before the ledger is
    /// asked.
    pub fn new(request_id: impl Into<String>, kind: OperationKind) -> Result<Self> {
        let request_id = request_id.into();
        if !is_bounded_request_id(&request_id) {
            return Err(error(
                "InvalidRequestId",
                format!(
                    "The request identifier is not a bounded identifier of at most {MAX_REQUEST_ID_LEN} characters over the ledger's own character set."
                ),
            ));
        }
        Ok(Self { request_id, kind })
    }

    /// The studio's idempotency key for this operation.
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// What the studio wants the ledger to record.
    pub fn kind(&self) -> &OperationKind {
        &self.kind
    }
}

/// Whether one request identifier is the studio's own bounded identifier.
///
/// The character set is the substrate's own: lowercase ASCII letters, digits,
/// `_`, `-` and `.`.
fn is_bounded_request_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_REQUEST_ID_LEN
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-' | b'.')
        })
}

/// The operations the studio's own semantics produce.
#[derive(Clone, Debug, PartialEq)]
pub enum OperationKind {
    /// A durable edit of the specimen.
    Edit {
        /// The specimen state the studio is about to persist.
        specimen: Value,
        /// The candidate this edit corrects, when it corrects one.
        supersedes: Option<[u8; 32]>,
    },
    /// An erasure request for a candidate the studio admitted earlier.
    Forget {
        /// The episode digest of the candidate to erase.
        target: [u8; 32],
    },
    /// A learning proposal in one of the studio's three states.
    Learn {
        /// The proposal exactly as the studio holds it.
        proposal: Value,
        /// The candidate a dismissed proposal erases, when it erases one.
        erases: Option<[u8; 32]>,
    },
}

/// The three states of a studio learning proposal.
///
/// The states are the studio's own: a proposal is proposed, learned, or
/// dismissed. The adapter maps them onto the substrate's admission
/// semantics, and it never invents a state of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    /// Proposed, and not yet admitted to the canonical ledger.
    Pending,
    /// Admitted to the canonical ledger as a memory candidate.
    Learned,
    /// Withdrawn: the candidate it named is forgotten.
    Dismissed,
}

impl ProposalStatus {
    /// The studio's own label for the state.
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Learned => "learned",
            Self::Dismissed => "dismissed",
        }
    }
    /// Read the state from the studio's own JSON shape.
    ///
    /// `None` when the proposal carries no known state: the studio's own
    /// contract admits exactly these three, and anything else is malformed
    /// rather than a fourth state.
    pub fn from_value(value: &Value) -> Option<Self> {
        match value.get("status")?.as_str()? {
            "pending" => Some(Self::Pending),
            "learned" => Some(Self::Learned),
            "dismissed" => Some(Self::Dismissed),
            _ => None,
        }
    }
}

/// The studio's learning proposal, as `lib/specimen/types.ts` states it.
///
/// The typed view is what the studio reads; the commitment is taken over the
/// proposal's own JSON, so no field of the proposal is dropped from the
/// content the ledger commits to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningProposal {
    /// The proposal's own identifier.
    pub id: String,
    /// The pattern the proposal suggests.
    pub pattern: String,
    /// How much evidence the studio counted for it.
    pub evidence: u64,
    /// The proposal's state.
    pub status: ProposalStatus,
}

impl LearningProposal {
    /// Read the proposal from the studio's own JSON shape.
    pub fn from_value(value: &Value) -> Result<Self> {
        let status = ProposalStatus::from_value(value).ok_or_else(|| {
            error(
                "InvalidProposal",
                "The proposal carries no known learning state.",
            )
        })?;
        Ok(Self {
            id: text(value, "id"),
            pattern: text(value, "pattern"),
            evidence: value
                .get("evidence")
                .and_then(Value::as_u64)
                .unwrap_or_default(),
            status,
        })
    }
}

/// One string field of the studio's own JSON, or the empty string.
fn text(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The studio's canonical byte form of one JSON value.
///
/// Object members are ordered by their UTF-8 bytes and no insignificant
/// whitespace is emitted, so equal values always produce equal bytes. This is
/// the studio's own encoding of its own content: it is not a commitment, and
/// the substrate never receives these bytes.
pub fn canonical_content_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(&ordered(value)).expect("a JSON value always serializes")
}

/// The value with every object's members ordered by their UTF-8 bytes.
fn ordered(value: &Value) -> Value {
    match value {
        Value::Object(members) => {
            let mut sorted: BTreeMap<String, Value> = BTreeMap::new();
            for (key, item) in members {
                sorted.insert(key.clone(), ordered(item));
            }
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(items) => Value::Array(items.iter().map(ordered).collect()),
        other => other.clone(),
    }
}

/// The SHA-256 over the studio's canonical content bytes.
///
/// The substrate treats this digest as an opaque 32-byte commitment: it
/// checks the shape, charges the byte count against the guild's budget, and
/// never sees the bytes themselves.
pub fn content_commitment(bytes: &[u8]) -> [u8; 32] {
    let mut commitment = [0u8; 32];
    commitment.copy_from_slice(&Sha256::digest(bytes));
    commitment
}
