//! Acknowledgements, quorum proofs, and the canonical shard receipt.
//!
//! The receipt is the only canonical acknowledgement this module produces.
//! It is deliberately *unforgeable from outside the crate*:
//!
//! - [`VoterAck`] has private fields and no public constructor, so an
//!   acknowledgement can only come from a voter that durably accepted a
//!   write;
//! - [`QuorumProof`] has private fields and no public constructor, so a
//!   proof can only come from certifying a quorum of acknowledgements;
//! - [`ShardReceipt`] holds its proof privately and has no `Deserialize`
//!   implementation, so a receipt cannot be rebuilt from bytes either.
//!
//! The certification step itself is crate-internal too: the shard's
//! [`propose`](super::QuorumShard::propose) is the only public path to a
//! receipt, and it always proposes the write it certifies.

use crate::v3::replication::term::{ShardTerm, VoterId};
use serde::Serialize;

/// One voter's durable acceptance of one write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoterAck {
    voter: VoterId,
    term: ShardTerm,
    digest: [u8; 32],
    sequence: u64,
    request_id: String,
}

impl VoterAck {
    pub(crate) fn new(
        voter: VoterId,
        term: ShardTerm,
        digest: [u8; 32],
        sequence: u64,
        request_id: String,
    ) -> Self {
        Self {
            voter,
            term,
            digest,
            sequence,
            request_id,
        }
    }

    /// The member that durably accepted the write.
    #[must_use]
    pub fn voter(&self) -> &VoterId {
        &self.voter
    }

    /// The term the acceptance was made under.
    #[must_use]
    pub fn term(&self) -> ShardTerm {
        self.term
    }

    /// The canonical episode digest the voter committed.
    #[must_use]
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }

    /// The ledger sequence the write took on that voter.
    #[must_use]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// The request identifier the write carried.
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
}

/// The proof that a quorum of distinct members accepted one write.
///
/// The proof is opaque: it can be inspected, not built. The only way to
/// obtain one is to certify a quorum of acknowledgements that all carry the
/// shard's current term and the same digest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct QuorumProof {
    term: ShardTerm,
    digest: [u8; 32],
    members: Vec<VoterId>,
}

impl QuorumProof {
    pub(crate) fn new(term: ShardTerm, digest: [u8; 32], members: Vec<VoterId>) -> Self {
        Self {
            term,
            digest,
            members,
        }
    }

    /// The term the quorum was formed under.
    #[must_use]
    pub fn term(&self) -> ShardTerm {
        self.term
    }

    /// The canonical episode digest the quorum accepted.
    #[must_use]
    pub fn digest(&self) -> &[u8; 32] {
        &self.digest
    }

    /// The distinct members that formed the quorum.
    #[must_use]
    pub fn members(&self) -> &[VoterId] {
        &self.members
    }
}

/// The canonical acknowledgement of one write.
///
/// A receipt exists only for a write that a quorum of the shard's members
/// durably accepted under the shard's current term. It is content-free: it
/// carries digests, identifiers, and counts, never episode content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShardReceipt {
    proof: QuorumProof,
    scope: String,
    sequence: u64,
    request_id: String,
    acknowledged_by: usize,
    quorum_size: usize,
}

impl ShardReceipt {
    pub(crate) fn new(
        proof: QuorumProof,
        scope: String,
        sequence: u64,
        request_id: String,
        acknowledged_by: usize,
        quorum_size: usize,
    ) -> Self {
        Self {
            proof,
            scope,
            sequence,
            request_id,
            acknowledged_by,
            quorum_size,
        }
    }

    /// The quorum proof behind this acknowledgement.
    #[must_use]
    pub fn proof(&self) -> &QuorumProof {
        &self.proof
    }

    /// The term the acknowledgement was formed under.
    #[must_use]
    pub fn term(&self) -> ShardTerm {
        self.proof.term()
    }

    /// The scope the shard owns.
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// The canonical episode digest of the acknowledged write.
    #[must_use]
    pub fn episode_digest(&self) -> &[u8; 32] {
        self.proof.digest()
    }

    /// The ledger sequence the write took on the members.
    #[must_use]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// The request identifier the acknowledged write carried.
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// How many members durably accepted the write.
    #[must_use]
    pub fn acknowledged_by(&self) -> usize {
        self.acknowledged_by
    }

    /// How many members had to accept the write.
    #[must_use]
    pub fn quorum_size(&self) -> usize {
        self.quorum_size
    }

    /// The distinct members that formed the quorum.
    #[must_use]
    pub fn members(&self) -> &[VoterId] {
        self.proof.members()
    }
}
