//! The portable record: what one organization may show another.
//!
//! A [`ForeignRecord`] is a *content-free projection* of one episode record.
//! It carries the origin's authority, the origin's guild scope, the origin's
//! canonical episode digest, the ancestry link, and the content-free
//! classification of the evidence. It never carries episode content: the v3
//! episode ledger commits to a payload, it does not store the payload, so
//! there is nothing else to share.
//!
//! # The origin keeps the authority
//!
//! Every record names the organization that authored it. An imported record
//! is *not* a record of the importing organization, and it is never written
//! into the importing organization's canonical episode ledger: the importing
//! organization holds it in its own foreign namespace, addressed by an
//! import receipt.
//!
//! # Ancestry
//!
//! A record that was derived from another record keeps the parent's origin
//! and the parent's digest. The parent is never invented: if the importing
//! organization has neither received the parent nor seen it before, the
//! import is refused as incomplete.

use crate::v3::episode::EpisodeReceipt;
use crate::v3::federation::canonical::{RECORD_DOMAIN, put_bool, put_digest, put_text, sha256};
use crate::v3::federation::error::FederationError;
use crate::v3::federation::grant::EvidenceClass;
use crate::v3::federation::identity::{OrganizationId, bounded_identifier};
use serde::{Deserialize, Serialize};

/// A reference to the record a foreign record was derived from.
///
/// The reference keeps the *origin's* authority: a record derived from
/// another organization's record names that other organization, not the one
/// that imported it.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AncestryRef {
    /// The organization that authored the parent record.
    pub origin: OrganizationId,
    /// The canonical episode digest of the parent record.
    pub episode_digest: [u8; 32],
}

/// One record as it travels between organizations.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ForeignRecord {
    /// The organization that authored the record.
    pub origin: OrganizationId,
    /// The authoring organization's guild scope.
    pub scope: String,
    /// The authoring organization's canonical episode digest.
    pub episode_digest: [u8; 32],
    /// The record this one was derived from, if any.
    pub parent: Option<AncestryRef>,
    /// The content-free event label of the record.
    pub event_kind: String,
    /// The content-free class of the record.
    pub evidence: EvidenceClass,
}

impl ForeignRecord {
    /// Build a foreign record.
    pub fn new(
        origin: OrganizationId,
        scope: &str,
        episode_digest: [u8; 32],
        parent: Option<AncestryRef>,
        event_kind: &str,
        evidence: EvidenceClass,
    ) -> Result<Self, FederationError> {
        if !bounded_identifier(scope) {
            return Err(FederationError::InvalidInput);
        }
        Ok(Self {
            origin,
            scope: scope.to_string(),
            episode_digest,
            parent,
            event_kind: event_kind.to_string(),
            evidence,
        })
    }

    /// Project a local episode receipt into a foreign record.
    ///
    /// The projection is content-free: the receipt is already content-free,
    /// and the parent is the origin's supersession target, which the caller
    /// reads from the local store's lineage.
    pub fn from_receipt(
        origin: &OrganizationId,
        receipt: &EpisodeReceipt,
        parent: Option<AncestryRef>,
    ) -> Result<Self, FederationError> {
        let evidence = EvidenceClass::for_event_kind(&receipt.event_kind)
            .ok_or(FederationError::InvalidInput)?;
        Self::new(
            origin.clone(),
            &receipt.guild_ref,
            receipt.episode_digest,
            parent,
            &receipt.event_kind,
            evidence,
        )
    }

    /// The federation digest of this record.
    ///
    /// The digest binds the origin, the scope, the canonical episode digest,
    /// the ancestry link, the event label, and the evidence class. It is the
    /// value the manifest lists and the importer re-derives.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut body = Vec::new();
        put_text(&mut body, self.origin.as_str());
        put_text(&mut body, &self.scope);
        put_digest(&mut body, &self.episode_digest);
        match &self.parent {
            Some(parent) => {
                put_bool(&mut body, true);
                put_text(&mut body, parent.origin.as_str());
                put_digest(&mut body, &parent.episode_digest);
            }
            None => put_bool(&mut body, false),
        }
        put_text(&mut body, &self.event_kind);
        put_text(&mut body, self.evidence.label());
        sha256(&[RECORD_DOMAIN, &body])
    }
}
