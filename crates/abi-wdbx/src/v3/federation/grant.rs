//! Bilateral trust and the scoped sharing grants that carry it.
//!
//! Federation trust is *bilateral*: two organizations each hold a trust
//! record about the other, and each record carries a scoped grant. There is
//! no global allow-list, no transitive trust, and no implicit permission:
//! a peer that is trusted by a trusted peer is still unknown to this
//! organization.
//!
//! # A grant is a scope, not a switch
//!
//! A [`ShareGrant`] names four things: the guild scopes that may be shared,
//! the evidence classes that may be shared, the direction of the exchange
//! (export, import, or both), and the residency class the counterparty must
//! satisfy. A grant that does not cover a record's scope does not permit that
//! record to move, in either direction.
//!
//! # Revocation
//!
//! Revocation is immediate and terminal for a trust record. A revoked record
//! still *exists* (the operator can see that trust was once granted and then
//! withdrawn), but it authorizes nothing: the next export or import is
//! refused. Re-granting trust is a new trust record, not a resurrection of
//! the old one.
//!
//! # Not a trust store
//!
//! The trust store is in-memory. It is not persisted, not audited, and not
//! reconciled with any peer; it is the local administrator's declaration.

use crate::v3::federation::error::FederationError;
use crate::v3::federation::identity::{PeerId, bounded_identifier};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The closed classes of evidence that may be shared between organizations.
///
/// The classes mirror the shape of the v3 episode events, not their content:
/// a memory candidate, a memory edge, or any other operation event. The
/// classification is content-free.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    /// An admitted memory candidate.
    MemoryCandidate,
    /// A quarantine, contradiction, or resolution edge.
    MemoryEdge,
    /// Any other operation lifecycle event.
    Operation,
}

impl EvidenceClass {
    /// The stable label of the class.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::MemoryCandidate => "memory_candidate",
            Self::MemoryEdge => "memory_edge",
            Self::Operation => "operation",
        }
    }

    /// The class an episode event kind belongs to.
    ///
    /// `None` means the event kind is outside the closed vocabulary, and the
    /// caller must not guess a class for it.
    #[must_use]
    pub fn for_event_kind(event_kind: &str) -> Option<Self> {
        match event_kind {
            "memory_candidate" => Some(Self::MemoryCandidate),
            "memory_edge" => Some(Self::MemoryEdge),
            "proposal" | "approval" | "execution" | "compensation" | "terminal" => {
                Some(Self::Operation)
            }
            _ => None,
        }
    }
}

/// The direction a grant authorizes.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ShareDirection {
    /// The peer may send covered records to the counterparty.
    Export,
    /// The peer may receive covered records from the counterparty.
    Import,
    /// Both directions are covered.
    Both,
}

impl ShareDirection {
    /// Whether the grant authorizes this direction.
    #[must_use]
    pub const fn permits(self, direction: Self) -> bool {
        matches!(self, Self::Both)
            || matches!(
                (self, direction),
                (Self::Export, Self::Export) | (Self::Import, Self::Import)
            )
    }
}

/// The scoped permission one side of a bilateral relation holds.
///
/// The grant is the whole permission: a peer that is trusted, with a grant
/// that covers the scope, the evidence class, the direction, and the
/// residency requirement, may exchange exactly those records.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShareGrant {
    /// Guild scopes the grant covers.
    scopes: BTreeSet<String>,
    /// Evidence classes the grant covers.
    evidence: BTreeSet<EvidenceClass>,
    /// The directions the grant authorizes.
    direction: ShareDirection,
    /// The residency class the counterparty must satisfy.
    residency: crate::v3::federation::identity::ResidencyClass,
}

impl ShareGrant {
    /// Build a grant.
    pub fn new(
        scopes: BTreeSet<String>,
        evidence: BTreeSet<EvidenceClass>,
        direction: ShareDirection,
        residency: crate::v3::federation::identity::ResidencyClass,
    ) -> Result<Self, FederationError> {
        if scopes.iter().any(|scope| !bounded_identifier(scope)) {
            return Err(FederationError::InvalidInput);
        }
        Ok(Self {
            scopes,
            evidence,
            direction,
            residency,
        })
    }

    /// The guild scopes the grant covers.
    #[must_use]
    pub fn scopes(&self) -> &BTreeSet<String> {
        &self.scopes
    }

    /// The evidence classes the grant covers.
    #[must_use]
    pub fn evidence(&self) -> &BTreeSet<EvidenceClass> {
        &self.evidence
    }

    /// The directions the grant authorizes.
    #[must_use]
    pub fn direction(&self) -> ShareDirection {
        self.direction
    }

    /// The residency class the counterparty must satisfy.
    #[must_use]
    pub fn residency(&self) -> crate::v3::federation::identity::ResidencyClass {
        self.residency
    }

    /// Whether the grant covers a direction.
    #[must_use]
    pub fn permits_direction(&self, direction: ShareDirection) -> bool {
        self.direction.permits(direction)
    }

    /// Whether the grant covers a guild scope.
    #[must_use]
    pub fn permits_scope(&self, scope: &str) -> bool {
        self.scopes.contains(scope)
    }

    /// Whether the grant covers an evidence class.
    #[must_use]
    pub fn permits_evidence(&self, evidence: EvidenceClass) -> bool {
        self.evidence.contains(&evidence)
    }
}

/// One side of a bilateral trust relation.
///
/// The record names one peer and the grant that governs what may be exchanged
/// with it. It is local: it says what *this* organization authorizes, and it
/// says nothing about what the other organization authorizes.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BilateralTrust {
    /// The peer this organization trusts.
    peer: PeerId,
    /// What may be exchanged with that peer.
    grant: ShareGrant,
    /// When the relation was granted, in epoch milliseconds.
    granted_at: u64,
    /// Whether the relation has been withdrawn.
    revoked: bool,
}

impl BilateralTrust {
    /// Build a trust record.
    #[must_use]
    pub fn new(peer: PeerId, grant: ShareGrant, granted_at: u64) -> Self {
        Self {
            peer,
            grant,
            granted_at,
            revoked: false,
        }
    }

    /// The trusted peer.
    #[must_use]
    pub fn peer(&self) -> &PeerId {
        &self.peer
    }

    /// The grant that governs the relation.
    #[must_use]
    pub fn grant(&self) -> &ShareGrant {
        &self.grant
    }

    /// When the relation was granted, in epoch milliseconds.
    #[must_use]
    pub fn granted_at(&self) -> u64 {
        self.granted_at
    }

    /// Whether the relation has been withdrawn.
    #[must_use]
    pub fn revoked(&self) -> bool {
        self.revoked
    }

    /// Withdraw the relation.
    pub fn revoke(&mut self) {
        self.revoked = true;
    }
}

/// The local organization's bilateral trust records.
#[derive(Clone, Debug, Default)]
pub struct TrustStore {
    trusts: BTreeMap<PeerId, BilateralTrust>,
}

impl TrustStore {
    /// An empty trust store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a trust relation, replacing any previous record for the peer.
    ///
    /// Replacing a revoked record with a live one is a *new* grant, and the
    /// caller has to say so explicitly.
    pub fn grant(&mut self, trust: BilateralTrust) -> Result<(), FederationError> {
        self.trusts.insert(trust.peer().clone(), trust);
        Ok(())
    }

    /// Withdraw the trust relation with one peer.
    ///
    /// A peer with no relation at all is not an error: there is nothing to
    /// withdraw, and the peer was never authorized.
    pub fn revoke(&mut self, peer: &PeerId) -> Result<(), FederationError> {
        if let Some(trust) = self.trusts.get_mut(peer) {
            trust.revoke();
        }
        Ok(())
    }

    /// The trust record for one peer, revoked or not.
    #[must_use]
    pub fn get(&self, peer: &PeerId) -> Option<&BilateralTrust> {
        self.trusts.get(peer)
    }

    /// The *live* trust record for one peer.
    ///
    /// A revoked record is not live and authorizes nothing.
    #[must_use]
    pub fn live(&self, peer: &PeerId) -> Option<&BilateralTrust> {
        self.trusts.get(peer).filter(|trust| !trust.revoked())
    }

    /// Whether the relation with one peer has been revoked.
    #[must_use]
    pub fn is_revoked(&self, peer: &PeerId) -> bool {
        self.trusts.get(peer).is_some_and(BilateralTrust::revoked)
    }

    /// How many relations are recorded.
    #[must_use]
    pub fn len(&self) -> usize {
        self.trusts.len()
    }

    /// Whether no relation is recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.trusts.is_empty()
    }
}
