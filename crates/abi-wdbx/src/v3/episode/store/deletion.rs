//! Deletion lifecycle for the v3 episode ledger: redaction of derived copies
//! and object-key destruction requests.
//!
//! # What the store holds
//!
//! The episode ledger holds commitments, not content. A memory candidate's
//! line carries a payload commitment, a payload size, and a class; the
//! payload itself lives outside this store, and so does every copy a consumer
//! derived from it — an embedding, a search-index entry, an export, a cache.
//!
//! This module therefore models the *request* to destroy those copies, not
//! the destruction itself: a redaction request names a forgotten target and
//! the kinds of derived copy to destroy, and an object-key destruction
//! request names a forgotten target and the object key that holds its
//! payload. The store's contract is narrow and explicit:
//!
//! > a redaction request exists for target `X` covering kinds `K1..Kn`.
//!
//! Destroying the consumer's copies is the consumer's obligation, and it is
//! **not implemented here**: this store never holds a derived copy. What the
//! store offers is the durable, content-free evidence that the request was
//! made, so a consumer can prove it was told.
//!
//! # Who may order a destruction
//!
//! A destruction order is a **governance** act over a guild's own content:
//! it orders the destruction of a forgotten candidate's derived copies and
//! of the object key that holds its payload. Both requests therefore carry
//! the `actor` that issues them, and the store refuses the order unless two
//! independent conditions hold:
//!
//! - **the guild-scoped authority map** (`store/authority.rs`): the
//!   principal the caller presents must be one the target guild's host
//!   policy authorizes. The map is the guild-scoping half, and it is read
//!   from the host policy, never from the ledger.
//! - **the class rule**: only a guild manager, a guild administrator, a
//!   guild owner, or an organization owner may order a destruction. A
//!   service may not: the service is the writer of memory candidates, and a
//!   writer that could destroy the evidence it just wrote would defeat the
//!   erasure record. A human subject may not either: the subject of a
//!   request is not an authority in this store.
//!
//! Both checks run **before every other check**, so an unauthorized actor
//! learns nothing: not whether the target is live, forgotten, or unknown,
//! not whether a hold is in force, and not whether the key is already bound
//! to another candidate. Every refusal is the same closed, content-free
//! error, so a refusal is not an oracle about the guild's map or about the
//! digests the store holds.
//!
//! # The unserved guild
//!
//! One check runs before those two: the store refuses a guild it does not
//! serve, and it refuses it as [`EpisodeStoreError::StaleBinding`] — the
//! same answer the holds slice and the quota slice give for the same
//! situation, so all three governance slices answer one question in one
//! voice. The refusal is about the *guild*, never about the target: an
//! unauthorized principal on a guild the store does serve still answers the
//! closed [`EpisodeStoreError::InvalidTransition`], so the two cases are
//! distinguishable only by the guild and never by the principal.
//!
//! # What the store refuses
//!
//! Both requests name a *forgotten* target. A live candidate is refused:
//! destroying the payload of a record the canonical ledger still holds would
//! take content away without the ledger's say. Only a tombstoned target may
//! be redacted, and only a tombstoned target may have its object key
//! destroyed. The refusal does not distinguish "the target is live" from
//! "the target is unknown": both answer the same closed error, so the store
//! is not an existence oracle for digests it does not hold.
//!
//! A target under a live hold is refused as well: a hold freezes the
//! candidate, so neither its derived copies nor the object key that holds
//! its payload are destroyed while the hold is in force.
//!
//! # The sidecars
//!
//! Both requests are recorded in derived indexes beside the ledger, in the
//! same shape as the quota sidecar: a header line and one line per request.
//! The header names the format, the number of request lines, and a SHA-256
//! over those lines' bytes, so a file truncated at a line boundary fails the
//! count or the checksum and is corruption rather than a silently shorter
//! history. Every request line carries a checksum over its own fields and
//! the principal that issued the order — the principal id and its class,
//! which are authority identifiers and not episode content. A trailing
//! fragment is damage rather than a crash tail: the writer only ever writes
//! whole lines, and the whole file is replaced atomically, so a reader sees
//! the old file or the new one and never a mixture.
//!
//! The sidecars are derived state: a store whose sidecar is lost keeps its
//! ledger and simply holds no destruction request. A sidecar written by an
//! earlier build — before the lines carried a principal — is refused as
//! corruption, because a destruction order the store cannot attribute is
//! not one it may trust.
//!
//! The two sidecars' codec lives in the `index` submodule beside this file,
//! exactly as the holds and the quota slices keep theirs.

mod index;

pub(in crate::v3::episode::store) use index::{ObjectKeyIndex, RedactionIndex};

use super::super::types::{ActorKind, ActorRef};
use super::{EpisodeStore, EpisodeStoreError, bounded_identifier, valid_actor};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The greatest number of derived-copy kinds one request may name.
const MAX_REDACTION_KINDS: usize = 16;
/// The greatest length of a tenant identifier.
const MAX_TENANT_BYTES: usize = 64;
/// The greatest length of an object-key identifier.
const MAX_KEY_ID_BYTES: usize = 128;

/// The kinds of derived copy a consumer may hold of one memory candidate.
///
/// The vocabulary is closed and content-free. A request names the *kind* of
/// copy to destroy, never the copy: the store has no embedding, no index
/// entry, no export, and no cache to destroy, because it never holds one.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DerivedCopyKind {
    /// A vector embedding computed from the candidate's payload.
    Embedding,
    /// An entry in a search, vector, or graph index.
    IndexEntry,
    /// An exported copy of the candidate.
    Export,
    /// A cached copy of the candidate.
    Cache,
}

impl DerivedCopyKind {
    /// The stable label the sidecar records.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Embedding => "embedding",
            Self::IndexEntry => "index_entry",
            Self::Export => "export",
            Self::Cache => "cache",
        }
    }
}

/// One request to redact the derived copies of a forgotten candidate.
///
/// The request names the target and the kinds of copy to destroy, and the
/// authority that issues the order. It carries no payload and no event: it is
/// a content-free instruction to a consumer that holds the copies.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RedactionRequest {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The forgotten candidate whose derived copies must go.
    pub target_digest: [u8; 32],
    /// The kinds of derived copy to destroy.
    ///
    /// Bounded and non-empty. Duplicates are collapsed, so a request that
    /// names the same kind twice asks for one destruction, not two.
    pub kinds: Vec<DerivedCopyKind>,
    /// The authority issuing the destruction order.
    ///
    /// The principal must be one the guild authorizes, and its class must be
    /// one that may order a destruction. The store refuses the order
    /// otherwise, and records the principal in the sidecar.
    pub actor: ActorRef,
}

/// The redaction requests the store holds for one forgotten target.
///
/// Content-free by construction: a target digest and the kinds of copy a
/// request covers. It never carries the copies themselves, because the store
/// never held them, and it reports coverage rather than attribution: the
/// principal that issued a request is recorded in the sidecar line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedactionState {
    /// The forgotten target the requests name.
    pub target_digest: [u8; 32],
    /// The kinds of derived copy a request covers, in canonical order.
    pub kinds: Vec<DerivedCopyKind>,
}

/// What the store will do with a redaction request.
///
/// The decision is the non-mutating half: it runs the same checks
/// [`EpisodeStore::commit_redaction`] runs and commits nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedactionDecision {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The forgotten candidate whose derived copies must go.
    pub target_digest: [u8; 32],
    /// The kinds this request records.
    pub requested: Vec<DerivedCopyKind>,
    /// The kinds a previous request already recorded.
    pub already_requested: Vec<DerivedCopyKind>,
}

/// The receipt of a committed redaction request.
///
/// The receipt is content-free: it names the target and the kinds of copy
/// the store now holds a request for. It does not claim that any copy was
/// destroyed; the consumer owns the copies and owns their destruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedactionReceipt {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The forgotten candidate whose derived copies must go.
    pub target_digest: [u8; 32],
    /// The kinds this request recorded.
    pub requested: Vec<DerivedCopyKind>,
    /// The kinds a previous request already recorded.
    pub already_requested: Vec<DerivedCopyKind>,
}

/// A bounded object-key reference: the tenant that owns the key and the key's
/// opaque identifier.
///
/// This is the v3 episode namespace's own type, **not** the complete-episode
/// prototype's private `ObjectKeyRef`: that prototype is crate-internal and
/// is not reused across module boundaries. Both parts are bounded
/// identifiers.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct ObjectKeyRef {
    /// The tenant that owns the object key.
    pub tenant: String,
    /// The opaque object-key identifier.
    pub key_id: String,
}

impl ObjectKeyRef {
    /// Whether both parts of the reference are bounded identifiers.
    fn is_valid(&self) -> bool {
        bounded_identifier(&self.tenant, MAX_TENANT_BYTES)
            && bounded_identifier(&self.key_id, MAX_KEY_ID_BYTES)
    }
}

/// One request to destroy the object key that holds a forgotten candidate.
///
/// The request names the target whose payload the key holds, and the
/// authority that issues the order. The store holds no object and destroys
/// none: the request is the durable evidence that the consumer was told to
/// destroy that key.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObjectKeyDestructionRequest {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The forgotten candidate whose payload the key holds.
    pub target_digest: [u8; 32],
    /// The object key to destroy.
    pub key: ObjectKeyRef,
    /// The authority issuing the destruction order.
    ///
    /// The principal must be one the guild authorizes, and its class must be
    /// one that may order a destruction. The store refuses the order
    /// otherwise, and records the principal in the sidecar.
    pub actor: ActorRef,
}

/// The destruction state of one object key.
///
/// Content-free: the guild, the key, and the forgotten target the key was
/// bound to. It never carries the object, its payload, or its location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectKeyDestructionState {
    /// The guild the request was made in.
    pub guild_ref: String,
    /// The object key the request names.
    pub key: ObjectKeyRef,
    /// The forgotten candidate whose payload the key holds.
    pub target_digest: [u8; 32],
}

/// What the store will do with an object-key destruction request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectKeyDestructionDecision {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The forgotten candidate whose payload the key holds.
    pub target_digest: [u8; 32],
    /// The object key the request names.
    pub key: ObjectKeyRef,
    /// True when a previous request already named this key for this target.
    pub already_requested: bool,
}

/// The receipt of a committed object-key destruction request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectKeyDestructionReceipt {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The forgotten candidate whose payload the key holds.
    pub target_digest: [u8; 32],
    /// The object key the request names.
    pub key: ObjectKeyRef,
    /// True when a previous request already named this key for this target.
    pub already_requested: bool,
}

/// Whether one actor class may order a destruction.
///
/// The store's governance vocabulary is the one the holds, the revocations,
/// and the budget adjustments already use: a guild owner, a guild
/// administrator, a guild manager, or an organization owner decides. A
/// service does not: the service is the writer of memory candidates, and a
/// writer that could destroy the evidence it just wrote would defeat the
/// erasure record. A human subject does not either: the subject of a
/// request is not an authority in this store.
fn destruction_authority(kind: ActorKind) -> bool {
    matches!(
        kind,
        ActorKind::GuildOwner
            | ActorKind::GuildAdministrator
            | ActorKind::GuildManager
            | ActorKind::OrganizationOwner
    )
}

impl EpisodeStore {
    /// Report the redaction requests the store holds for one target.
    ///
    /// `Ok(None)` means no request names that digest in this guild. The state
    /// is content-free: the target digest and the kinds of derived copy a
    /// request covers.
    ///
    /// The report is the store's half of the contract only. It says that a
    /// request exists; it never says that a consumer destroyed anything,
    /// because the store never held a copy to destroy.
    pub fn redaction_state(
        &self,
        guild_ref: &str,
        target_digest: &[u8; 32],
    ) -> Result<Option<RedactionState>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        let kinds = self.redaction.kinds(guild_ref, target_digest);
        if kinds.is_empty() {
            return Ok(None);
        }
        Ok(Some(RedactionState {
            target_digest: *target_digest,
            kinds: kinds.into_iter().collect(),
        }))
    }

    /// Decide whether a redaction request would be accepted, without
    /// recording it.
    ///
    /// The decision runs the same checks [`Self::commit_redaction`] runs: the
    /// principal must be one the guild authorizes and its class must be one
    /// that may order a destruction, the request must be well formed, and it
    /// must name a *forgotten* target. A live target is refused, and so is a
    /// digest the store never admitted; both refusals are the same closed
    /// error, so the decision is not an existence oracle.
    ///
    /// The authority check runs before every check about the target, so an
    /// unauthorized actor learns nothing about the target.
    pub fn decide_redaction(
        &self,
        request: &RedactionRequest,
    ) -> Result<RedactionDecision, EpisodeStoreError> {
        // The guild comes first: a guild the store does not serve is
        // refused as stale, exactly as the holds and the quota slices
        // refuse it. The refusal is about the guild, not about the target.
        self.require_served_guild(&request.guild_ref)?;
        // The guild-scoping half of the authority check: the principal the
        // caller presents must be one this guild authorizes. The class rule
        // below is the second, necessary half. Both run before every other
        // check, so an unauthorized actor learns nothing.
        self.require_guild_authority(&request.guild_ref, &request.actor)?;
        if !destruction_authority(request.actor.kind) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        validate_redaction_request(request)?;
        if !self.forgotten_target(&request.guild_ref, &request.target_digest) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        // A live hold freezes the candidate: its payload is not destroyed
        // while the hold is in force. The check is explicit even though the
        // write gate currently refuses to forget a held candidate, so the
        // invariant does not depend on that gate alone.
        if self.is_held(&request.guild_ref, &request.target_digest) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        let held = self
            .redaction
            .kinds(&request.guild_ref, &request.target_digest);
        let mut requested = Vec::new();
        let mut already_requested = Vec::new();
        for kind in request.kinds.iter().copied().collect::<BTreeSet<_>>() {
            if held.contains(&kind) {
                already_requested.push(kind);
            } else {
                requested.push(kind);
            }
        }
        Ok(RedactionDecision {
            guild_ref: request.guild_ref.clone(),
            target_digest: request.target_digest,
            requested,
            already_requested,
        })
    }

    /// Record a redaction request for a forgotten target.
    ///
    /// This commits: the request is written to the store's redaction index by
    /// the same single writer that appends the ledger, and the line is
    /// synchronized before the receipt is returned. The request is
    /// idempotent per kind: a kind a previous request already recorded is
    /// reported as `already_requested` and no second line is written.
    ///
    /// The order is attributable: the principal that issued it is recorded
    /// in the index, and only an authority the guild authorizes may issue
    /// one. A request that names several kinds is committed kind by kind; a
    /// failure part-way leaves the kinds already written recorded, and the
    /// request is idempotent, so retrying it completes the rest.
    ///
    /// The receipt does not claim that any derived copy was destroyed. The
    /// store holds no derived copy; destroying the consumer's copies is the
    /// consumer's obligation, and it is not implemented here.
    pub fn commit_redaction(
        &mut self,
        request: &RedactionRequest,
    ) -> Result<RedactionReceipt, EpisodeStoreError> {
        // The guild comes first, for the same reason as in the decision
        // above.
        self.require_served_guild(&request.guild_ref)?;
        // The authority check runs before anything else here too, so an
        // unauthorized actor learns nothing — not even whether the store is
        // writable. The decision below repeats it, so the two paths cannot
        // drift apart.
        self.require_guild_authority(&request.guild_ref, &request.actor)?;
        if !destruction_authority(request.actor.kind) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        if self.poisoned {
            return Err(EpisodeStoreError::Io);
        }
        let decision = self.decide_redaction(request)?;
        for kind in &decision.requested {
            self.redaction.commit(
                &decision.guild_ref,
                decision.target_digest,
                *kind,
                &request.actor,
            )?;
        }
        Ok(RedactionReceipt {
            guild_ref: decision.guild_ref,
            target_digest: decision.target_digest,
            requested: decision.requested,
            already_requested: decision.already_requested,
        })
    }

    /// Report the destruction state of one object key.
    ///
    /// `Ok(None)` means no request names that key in this guild. The state is
    /// content-free: the key, the guild, and the forgotten target the key was
    /// bound to. It never carries the object or its payload.
    pub fn object_key_destruction_state(
        &self,
        guild_ref: &str,
        key: &ObjectKeyRef,
    ) -> Result<Option<ObjectKeyDestructionState>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) || !key.is_valid() {
            return Err(EpisodeStoreError::InvalidInput);
        }
        let Some(target) = self.object_keys.target(guild_ref, key) else {
            return Ok(None);
        };
        Ok(Some(ObjectKeyDestructionState {
            guild_ref: guild_ref.to_string(),
            key: key.clone(),
            target_digest: target,
        }))
    }

    /// Decide whether an object-key destruction request would be accepted,
    /// without recording it.
    ///
    /// The principal must be one the guild authorizes and its class must be
    /// one that may order a destruction, and the request must name a
    /// *forgotten* target. A live target is refused: destroying the payload
    /// of a record the canonical ledger still holds would take content away
    /// without the ledger's say.
    ///
    /// A key already bound to another target is refused as well: one key
    /// names one target, so a destruction request may not be re-pointed at a
    /// different candidate.
    ///
    /// The authority check runs before every check about the target or the
    /// key, so an unauthorized actor learns nothing about either.
    pub fn decide_object_key_destruction(
        &self,
        request: &ObjectKeyDestructionRequest,
    ) -> Result<ObjectKeyDestructionDecision, EpisodeStoreError> {
        // The guild comes first, for the same reason as in the redaction
        // decision above.
        self.require_served_guild(&request.guild_ref)?;
        // The guild-scoping half of the authority check: the principal the
        // caller presents must be one this guild authorizes. The class rule
        // below is the second, necessary half. Both run before every other
        // check, so an unauthorized actor learns nothing.
        self.require_guild_authority(&request.guild_ref, &request.actor)?;
        if !destruction_authority(request.actor.kind) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        validate_object_key_request(request)?;
        if !self.forgotten_target(&request.guild_ref, &request.target_digest) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        // A live hold freezes the candidate: the object key that holds its
        // payload is not destroyed while the hold is in force.
        if self.is_held(&request.guild_ref, &request.target_digest) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        let bound = self.object_keys.target(&request.guild_ref, &request.key);
        if let Some(bound) = bound
            && bound != request.target_digest
        {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        Ok(ObjectKeyDestructionDecision {
            guild_ref: request.guild_ref.clone(),
            target_digest: request.target_digest,
            key: request.key.clone(),
            already_requested: bound.is_some(),
        })
    }

    /// Record an object-key destruction request for a forgotten target.
    ///
    /// This commits: the request is written to the store's object-key index
    /// by the same single writer that appends the ledger, and the line is
    /// synchronized before the receipt is returned.
    ///
    /// The order is attributable: the principal that issued it is recorded
    /// in the index, and only an authority the guild authorizes may issue
    /// one.
    ///
    /// The receipt does not claim that the key was destroyed. The store holds
    /// no object; destroying it is the consumer's obligation, and it is not
    /// implemented here.
    pub fn commit_object_key_destruction(
        &mut self,
        request: &ObjectKeyDestructionRequest,
    ) -> Result<ObjectKeyDestructionReceipt, EpisodeStoreError> {
        // The guild comes first, for the same reason as in the decision
        // above.
        self.require_served_guild(&request.guild_ref)?;
        // The authority check runs before anything else here too, so an
        // unauthorized actor learns nothing — not even whether the store is
        // writable. The decision below repeats it, so the two paths cannot
        // drift apart.
        self.require_guild_authority(&request.guild_ref, &request.actor)?;
        if !destruction_authority(request.actor.kind) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        if self.poisoned {
            return Err(EpisodeStoreError::Io);
        }
        let decision = self.decide_object_key_destruction(request)?;
        if !decision.already_requested {
            self.object_keys.commit(
                &decision.guild_ref,
                decision.target_digest,
                &decision.key,
                &request.actor,
            )?;
        }
        Ok(ObjectKeyDestructionReceipt {
            guild_ref: decision.guild_ref,
            target_digest: decision.target_digest,
            key: decision.key,
            already_requested: decision.already_requested,
        })
    }

    /// Whether one digest names a candidate this guild has forgotten.
    ///
    /// A forgotten candidate is a tombstone: its content is gone, and no
    /// operation may bring it back. The check does not distinguish a live
    /// candidate from an unknown digest, so the refusal that uses it leaks
    /// nothing about digests the store does not hold.
    fn forgotten_target(&self, guild_ref: &str, digest: &[u8; 32]) -> bool {
        self.state
            .memories
            .get(guild_ref)
            .is_some_and(|memories| memories.forgotten.contains(digest))
    }

    /// Whether the store serves one guild.
    ///
    /// The governance slices answer one question in one voice: a guild the
    /// store does not serve is refused as
    /// [`EpisodeStoreError::StaleBinding`], the same answer the holds slice
    /// and the quota slice give. The refusal is about the guild alone, so it
    /// tells the caller nothing about the target, the key, or the guild's
    /// authority map.
    fn require_served_guild(&self, guild_ref: &str) -> Result<(), EpisodeStoreError> {
        self.policy
            .guilds
            .get(guild_ref)
            .map(|_| ())
            .ok_or(EpisodeStoreError::StaleBinding)
    }
}

/// Whether one redaction request is well formed.
fn validate_redaction_request(request: &RedactionRequest) -> Result<(), EpisodeStoreError> {
    if !bounded_identifier(&request.guild_ref, 128)
        || !valid_actor(&request.actor)
        || request.kinds.is_empty()
        || request.kinds.len() > MAX_REDACTION_KINDS
    {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(())
}

/// Whether one object-key destruction request is well formed.
fn validate_object_key_request(
    request: &ObjectKeyDestructionRequest,
) -> Result<(), EpisodeStoreError> {
    if !bounded_identifier(&request.guild_ref, 128)
        || !valid_actor(&request.actor)
        || !request.key.is_valid()
    {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(())
}
