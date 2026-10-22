//! Holds and candidate revocation for the v3 episode ledger.
//!
//! # What a hold is
//!
//! A hold is a placement on a memory candidate that *freezes* it. While a
//! hold is live, the candidate's content is preserved: it is not reclaimed by
//! the collector, not expired away by the retention engine, not silently
//! superseded, and its payload is not destroyed by a redaction or an
//! object-key destruction request. A hold is lifted by an explicit act: the
//! authority that placed it releases it, or an authority above it in the
//! store's governance order does.
//!
//! # What a revocation is
//!
//! A revocation *withdraws* a candidate by authority. The candidate is not
//! live any more: the store's own liveness logic reports it as not live, no
//! later `supersedes` may name it, and it can never be resurrected. Its
//! record and its anti-resurrection metadata are **retained**: a revocation
//! is a withdrawal from use, not an erasure. Only a `forgets` tombstone
//! erases, and the store refuses to forget a revoked candidate, so a revoked
//! record and its metadata stay in the ledger.
//!
//! # Why these live in sidecars and not in the ledger
//!
//! A hold and a revocation are governance *decisions about* a candidate, not
//! new kinds of episode. Putting them in the ledger would mean either
//!
//! - adding a field to `MemoryCandidate`, which changes every candidate's
//!   canonical digest, breaks the pinned goldens (`tests/v3_memory_candidate.rs`,
//!   `tests/v3_memory_edge.rs`) and the cross-language Python witness
//!   (`tools/abbey_cbor_episode_v1.py`); or
//! - adding a new `EpisodeEvent` variant, which changes the canonical event
//!   encoder in `store/canonical.rs` *and* the Python witness *and* every
//!   pinned golden, all of which must move together.
//!
//! Neither is acceptable for a decision that is not itself an episode. A hold
//! carries no payload, no participant, and no observation: it names a target
//! digest, a kind, an authority, and a state. So this module follows the
//! repository's existing precedent exactly — the retention index
//! (`episodes.v1.retention.jsonl`), the redaction index
//! (`episodes.v1.redaction.jsonl`), the object-key index
//! (`episodes.v1.object_keys.jsonl`), and the reclaimed index
//! (`episodes.v1.reclaimed.jsonl`):
//!
//! - one derived index beside the ledger, per concern;
//! - one line per decision, holding digests and closed labels only, never
//!   episode content;
//! - a SHA-256 checksum over each line's own fields, so a corrupted or
//!   half-written line is corruption rather than a silently dropped hold;
//! - an incomplete final line is discarded and truncated, exactly as the
//!   ledger discards an incomplete final line;
//! - the file is created with the first decision, not with the store, and a
//!   store that never decided anything has no index file at all;
//! - the line is written and synchronized before the receipt that reports it
//!   is returned, so a receipt never claims a decision the store did not
//!   durably record;
//! - the same single writer that appends the ledger writes the index.
//!
//! Like every other index here, the index is derived state: losing it does
//! not lose an episode. A store whose hold index is lost keeps its ledger and
//! simply holds nothing; a store whose revocation index is lost keeps its
//! ledger and reports the revoked candidate as live again. That is the
//! fail-safe direction for a hold (content is preserved rather than
//! destroyed) and the fail-open direction for a revocation, which is why the
//! revocation index is read and applied before the store answers any query.
//!
//! # What the hold binds
//!
//! A hold that binds nothing is not a hold. While a hold is live:
//!
//! - the retention engine does not propose the candidate for expiry
//!   (`expiry.rs`, `due_expirations`), and the emitter emits no `forgets`
//!   write for it;
//! - the collector does not propose the candidate for reclamation (`gc.rs`,
//!   `reclaimable_at`): the payload-bearing record of a held candidate is
//!   kept, whatever the clock says;
//! - the deletion lifecycle refuses a redaction or an object-key destruction
//!   request that names the candidate (`deletion.rs`), so the payload and the
//!   derived copies of a held candidate are not destroyed;
//! - the write gate refuses a `supersedes` that names the candidate
//!   (`store.rs`, `prepare_record`): a held candidate is not silently
//!   superseded, and the correction waits for the explicit resolution that
//!   releases the hold.
//!
//! A `forgets` tombstone is **not** refused by the hold, and this is
//! deliberate. In this store the tombstone is the *request* to erase a
//! candidate and the collector's reclamation is the *destruction* of its
//! record; the deletion lifecycle's requests are what tell a consumer to
//! destroy the derived copies and the object key. A hold freezes that
//! destruction, and the tombstone itself is an explicit act of a caller that
//! the store does not second-guess. Refusing the tombstone as well would
//! make the collector's and the deletion lifecycle's hold bindings
//! unreachable: a tombstone is the precondition of both.
//!
//! # What is not implemented here
//!
//! - The constitutional query (`store/query/`) now reports both: the
//!   lifecycle dimension carries the hold state and the revocation state,
//!   and the [`Disposition`](super::query::Disposition) vocabulary has a
//!   member for each. What this slice still does not do is hold a hold or a
//!   revocation *in the ledger*: the query reads the derived indexes this
//!   module owns, and a store whose index is lost holds nothing and revokes
//!   nothing.
//! - The store holds no payload and destroys no derived copy: the hold
//!   prevents the *request* to destroy a copy, and the consumer still owns
//!   the copies it holds.
//! - A revocation is not an erasure, and the store holds no key custody and no
//!   policy administration. Quota administration lives in its own slice
//!   (`store/quota.rs`); this module does not decide budgets.
//! - The substrate cannot verify guild membership: the guild-scoped
//!   authority map (`GuildEpisodePolicy::authorized_principals`, read by
//!   `super::authority`) says which principals a guild authorizes, and the
//!   store enforces that the principal the caller presents is one of them.
//!   The host policy is the authority of record, and the store trusts the
//!   caller to present the right principal and the right class.

mod index;

pub(in crate::v3::episode::store) use index::{HoldIndex, RevocationIndex};

use super::super::types::{ActorKind, ActorRef, EpisodeEvent};
use super::{
    EpisodeStore, EpisodeStoreError, LedgerState, StoredRecord, bounded_identifier, valid_actor,
};
use serde::{Deserialize, Serialize};

/// The kinds of hold a governance authority may place on a candidate.
///
/// The vocabulary is closed and content-free: a hold names the *kind* of
/// freeze, never the reason, the evidence, or the incident behind it.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum HoldKind {
    /// A legal or regulatory hold: the content must be preserved.
    Legal,
    /// An investigation is examining the candidate.
    Investigation,
    /// A user asked for the candidate to be preserved.
    UserRequest,
}

impl HoldKind {
    /// The stable label the index records.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Legal => "legal",
            Self::Investigation => "investigation",
            Self::UserRequest => "user_request",
        }
    }
}

/// Whether a hold is in force.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HoldStatus {
    /// The hold is in force: the candidate is frozen.
    Live,
    /// The hold was released: the candidate is free again.
    Released,
}

impl HoldStatus {
    /// The stable label the index records.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Released => "released",
        }
    }
}

/// One hold on one memory candidate.
///
/// Content-free by construction: a target digest, a kind, the authority that
/// placed the hold, the hold's state, and the authority that released it
/// when it was released. It never carries a payload, a participant, or an
/// event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hold {
    /// The candidate the hold names.
    pub target_digest: [u8; 32],
    /// The kind of freeze.
    pub kind: HoldKind,
    /// The authority that placed the hold.
    pub authority: ActorRef,
    /// Whether the hold is in force.
    pub status: HoldStatus,
    /// The authority that released the hold, when it was released.
    pub released_by: Option<ActorRef>,
}

/// The hold state of one memory candidate.
///
/// `Ok(None)` means no hold names that digest in that guild. The state is
/// content-free: digests, kinds, authorities, and flags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldState {
    /// The candidate the holds name.
    pub target_digest: [u8; 32],
    /// The holds on the candidate, in canonical kind order.
    pub holds: Vec<Hold>,
    /// True when at least one hold is live.
    ///
    /// A frozen candidate is not expired away, not reclaimed, not silently
    /// superseded, and its payload is not destroyed.
    pub frozen: bool,
}

/// One request to place a hold on a memory candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldRequest {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The candidate to freeze.
    pub target_digest: [u8; 32],
    /// The kind of freeze.
    pub kind: HoldKind,
    /// The authority placing the hold.
    pub authority: ActorRef,
}

/// What the store will do with a hold request.
///
/// The decision is the non-mutating half: it runs the same checks
/// [`EpisodeStore::commit_hold`] runs and commits nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldDecision {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The candidate the hold names.
    pub target_digest: [u8; 32],
    /// The kind of freeze.
    pub kind: HoldKind,
    /// The authority placing the hold.
    pub authority: ActorRef,
    /// True when a live hold of this kind already names the target.
    ///
    /// The request is then idempotent: it records nothing new.
    pub already_live: bool,
}

/// The receipt of a committed hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldReceipt {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The candidate the hold names.
    pub target_digest: [u8; 32],
    /// The kind of freeze.
    pub kind: HoldKind,
    /// The authority that placed the hold.
    pub authority: ActorRef,
    /// True when a live hold of this kind already named the target.
    pub already_live: bool,
}

/// One request to release a hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldReleaseRequest {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The candidate the hold names.
    pub target_digest: [u8; 32],
    /// The kind of hold to release.
    pub kind: HoldKind,
    /// The authority performing the release.
    pub authority: ActorRef,
}

/// The receipt of a released hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoldReleaseReceipt {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The candidate the hold named.
    pub target_digest: [u8; 32],
    /// The kind of hold that was released.
    pub kind: HoldKind,
    /// The authority that placed the hold.
    pub released: ActorRef,
    /// The authority that released the hold.
    pub released_by: ActorRef,
}

/// One request to revoke a memory candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevocationRequest {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The candidate to withdraw.
    pub target_digest: [u8; 32],
    /// The authority revoking the candidate.
    pub authority: ActorRef,
}

/// What the store will do with a revocation request.
///
/// The decision is the non-mutating half: it runs the same checks
/// [`EpisodeStore::commit_revocation`] runs and commits nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevocationDecision {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The candidate the request names.
    pub target_digest: [u8; 32],
    /// True when the candidate is already revoked.
    ///
    /// The request is then idempotent: it records nothing new.
    pub already_revoked: bool,
}

/// The receipt of a committed revocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevocationReceipt {
    /// The guild the target belongs to.
    pub guild_ref: String,
    /// The candidate the revocation names.
    pub target_digest: [u8; 32],
    /// The authority that revoked the candidate.
    pub authority: ActorRef,
    /// True when the candidate was already revoked.
    pub already_revoked: bool,
}

/// The revocation state of one memory candidate.
///
/// `Ok(None)` means the candidate was never revoked in that guild.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevocationState {
    /// The candidate the revocation names.
    pub target_digest: [u8; 32],
    /// The authority that revoked the candidate.
    pub authority: ActorRef,
}

/// Whether one authority class may place a hold or revoke a candidate.
///
/// The store's governance vocabulary is the one the memory edges already use:
/// a guild owner, a guild administrator, a guild manager, or an organization
/// owner decides. A service does not: the service is the writer of memory
/// candidates, and a writer that could freeze or withdraw its own records
/// could hide evidence. A human subject does not either: the subject of a
/// request is not an authority in this store.
fn governance_authority(kind: ActorKind) -> bool {
    matches!(
        kind,
        ActorKind::GuildOwner
            | ActorKind::GuildAdministrator
            | ActorKind::GuildManager
            | ActorKind::OrganizationOwner
    )
}

/// The store's documented governance order.
///
/// The order is fixed and documented here, because it decides who may
/// release a hold another authority placed. It runs from the guild's
/// day-to-day manager to the organization's owner.
fn authority_rank(kind: ActorKind) -> u8 {
    match kind {
        ActorKind::GuildManager => 1,
        ActorKind::GuildAdministrator => 2,
        ActorKind::GuildOwner => 3,
        ActorKind::OrganizationOwner => 4,
        _ => 0,
    }
}

/// Whether one authority may release a hold another authority placed.
///
/// The placing authority may always release its own hold. An authority
/// strictly above the placing one in the governance order may release it
/// too, because that order is the store's answer to who may undo whose
/// decision. Nobody else may, and a service never may.
fn may_release(owner: &ActorRef, releaser: &ActorRef) -> bool {
    if !governance_authority(releaser.kind) {
        return false;
    }
    if releaser.principal_id == owner.principal_id && releaser.kind == owner.kind {
        return true;
    }
    authority_rank(releaser.kind) > authority_rank(owner.kind)
}

impl EpisodeStore {
    /// Decide whether a hold would be accepted, without recording it.
    ///
    /// The decision runs the same checks [`Self::commit_hold`] runs: the
    /// request must be well formed, name a guild the store serves, name a
    /// *candidate* of that guild, and come from an authority that may place
    /// a hold. A service may not, and neither may a human subject.
    ///
    /// The target may be a live candidate or an already forgotten one: a
    /// hold freezes the *destruction* of the content, and a forgotten
    /// candidate's payload, derived copies, and object key still exist
    /// outside the ledger. A digest the store never admitted is refused, and
    /// so is a tombstone, which is an erasure request rather than a
    /// candidate; the refusal is the same closed error either way, so the
    /// decision is not an existence oracle.
    ///
    /// It records nothing and appends nothing.
    pub fn decide_hold(&self, request: &HoldRequest) -> Result<HoldDecision, EpisodeStoreError> {
        validate_hold_request(request)?;
        self.served_guild(&request.guild_ref)?;
        // The guild-scoping half of the authority check: the principal the
        // caller presents must be one this guild authorizes. The class rule
        // below is the second, necessary half.
        self.require_guild_authority(&request.guild_ref, &request.authority)?;
        if !governance_authority(request.authority.kind) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        if !self.holdable_candidate(&request.guild_ref, &request.target_digest) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        let already_live =
            match self
                .holds
                .record(&request.guild_ref, &request.target_digest, request.kind)
            {
                Some(record) if record.status == HoldStatus::Live => {
                    if record.authority.principal_id != request.authority.principal_id {
                        // Another authority already holds this kind of freeze.
                        return Err(EpisodeStoreError::InvalidTransition);
                    }
                    true
                }
                _ => false,
            };
        Ok(HoldDecision {
            guild_ref: request.guild_ref.clone(),
            target_digest: request.target_digest,
            kind: request.kind,
            authority: request.authority.clone(),
            already_live,
        })
    }

    /// Record a hold on a memory candidate.
    ///
    /// This commits: the hold is written to the store's hold index by the
    /// same single writer that appends the ledger, and the line is
    /// synchronized before the receipt is returned.
    ///
    /// The request is idempotent: a live hold of the same kind placed by the
    /// same authority is reported as `already_live` and no second line is
    /// written.
    pub fn commit_hold(&mut self, request: &HoldRequest) -> Result<HoldReceipt, EpisodeStoreError> {
        if self.poisoned {
            return Err(EpisodeStoreError::Io);
        }
        let decision = self.decide_hold(request)?;
        if !decision.already_live {
            self.holds.commit(
                &decision.guild_ref,
                decision.target_digest,
                decision.kind,
                &decision.authority,
                HoldStatus::Live,
                None,
            )?;
        }
        Ok(HoldReceipt {
            guild_ref: decision.guild_ref,
            target_digest: decision.target_digest,
            kind: decision.kind,
            authority: decision.authority,
            already_live: decision.already_live,
        })
    }

    /// Release a hold.
    ///
    /// Only the authority that placed the hold may release it, or an
    /// authority above it in the store's governance order. Any other
    /// authority is refused, and so is a hold that is not live: the refusal
    /// does not distinguish "never placed" from "already released", so the
    /// call is not an existence oracle.
    ///
    /// Releasing a hold is the explicit resolution a held candidate needs
    /// before it may be superseded, forgotten, expired, or reclaimed.
    pub fn release_hold(
        &mut self,
        request: &HoldReleaseRequest,
    ) -> Result<HoldReleaseReceipt, EpisodeStoreError> {
        if self.poisoned {
            return Err(EpisodeStoreError::Io);
        }
        validate_release_request(request)?;
        self.served_guild(&request.guild_ref)?;
        // The guild-scoping half of the authority check: the releasing
        // principal must be one this guild authorizes. The class rule and
        // the governance order below are the other halves.
        self.require_guild_authority(&request.guild_ref, &request.authority)?;
        if !governance_authority(request.authority.kind) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        let owner = {
            let Some(record) =
                self.holds
                    .record(&request.guild_ref, &request.target_digest, request.kind)
            else {
                return Err(EpisodeStoreError::InvalidTransition);
            };
            if record.status != HoldStatus::Live {
                return Err(EpisodeStoreError::InvalidTransition);
            }
            record.authority.clone()
        };
        if !may_release(&owner, &request.authority) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        self.holds.commit(
            &request.guild_ref,
            request.target_digest,
            request.kind,
            &owner,
            HoldStatus::Released,
            Some(&request.authority),
        )?;
        Ok(HoldReleaseReceipt {
            guild_ref: request.guild_ref.clone(),
            target_digest: request.target_digest,
            kind: request.kind,
            released: owner,
            released_by: request.authority.clone(),
        })
    }

    /// Report the hold state of one memory candidate.
    ///
    /// `Ok(None)` means no hold names that digest in this guild. The state is
    /// content-free: the target digest, the kinds of hold, the authorities
    /// that placed them, and whether the candidate is frozen.
    pub fn hold_state(
        &self,
        guild_ref: &str,
        target_digest: &[u8; 32],
    ) -> Result<Option<HoldState>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        let records = self.holds.holds_of(guild_ref, target_digest);
        if records.is_empty() {
            return Ok(None);
        }
        let frozen = records
            .iter()
            .any(|(_, record)| record.status == HoldStatus::Live);
        Ok(Some(HoldState {
            target_digest: *target_digest,
            holds: records
                .into_iter()
                .map(|(kind, record)| Hold {
                    target_digest: *target_digest,
                    kind,
                    authority: record.authority,
                    status: record.status,
                    released_by: record.released_by,
                })
                .collect(),
            frozen,
        }))
    }

    /// Decide whether a revocation would be accepted, without recording it.
    ///
    /// The decision runs the same checks [`Self::commit_revocation`] runs:
    /// the request must be well formed, name a guild the store serves, name a
    /// live candidate of that guild, and come from an authority that may
    /// revoke. Only a guild owner, a guild administrator, a guild manager,
    /// or an organization owner may revoke; a service may not, and neither
    /// may a human subject.
    ///
    /// It records nothing and appends nothing.
    pub fn decide_revocation(
        &self,
        request: &RevocationRequest,
    ) -> Result<RevocationDecision, EpisodeStoreError> {
        validate_revocation_request(request)?;
        self.served_guild(&request.guild_ref)?;
        // The guild-scoping half of the authority check: the revoking
        // principal must be one this guild authorizes. The class rule below
        // is the second, necessary half.
        self.require_guild_authority(&request.guild_ref, &request.authority)?;
        if !governance_authority(request.authority.kind) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        let already_revoked = self
            .revocations
            .authority_of(&request.guild_ref, &request.target_digest)
            .is_some();
        if !already_revoked && !self.live_candidate(&request.guild_ref, &request.target_digest) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        Ok(RevocationDecision {
            guild_ref: request.guild_ref.clone(),
            target_digest: request.target_digest,
            already_revoked,
        })
    }

    /// Revoke a memory candidate.
    ///
    /// This commits: the revocation is written to the store's revocation
    /// index by the same single writer that appends the ledger, and the line
    /// is synchronized before the receipt is returned. The candidate stops
    /// being live in the same call, so no later write may supersede it and
    /// no later query may present it as usable evidence.
    ///
    /// The request is idempotent: a candidate that is already revoked is
    /// reported as `already_revoked` and no second line is written.
    ///
    /// A revocation is not an erasure. The candidate's record and its
    /// anti-resurrection metadata are retained, and the store refuses to
    /// forget a revoked candidate.
    pub fn commit_revocation(
        &mut self,
        request: &RevocationRequest,
    ) -> Result<RevocationReceipt, EpisodeStoreError> {
        if self.poisoned {
            return Err(EpisodeStoreError::Io);
        }
        let decision = self.decide_revocation(request)?;
        if !decision.already_revoked {
            self.revocations.commit(
                &decision.guild_ref,
                decision.target_digest,
                &request.authority,
            )?;
            // The candidate is not live any more. The revocation is part of
            // the state the store answers from, exactly as the admission of a
            // reclaimed candidate is.
            if let Some(memories) = self.state.memories.get_mut(&decision.guild_ref) {
                memories.revoked.insert(decision.target_digest);
            }
        }
        Ok(RevocationReceipt {
            guild_ref: decision.guild_ref,
            target_digest: decision.target_digest,
            authority: request.authority.clone(),
            already_revoked: decision.already_revoked,
        })
    }

    /// Report the revocation state of one memory candidate.
    ///
    /// `Ok(None)` means the candidate was never revoked in this guild. The
    /// state is content-free: the target digest and the authority that
    /// revoked it.
    pub fn revocation_state(
        &self,
        guild_ref: &str,
        target_digest: &[u8; 32],
    ) -> Result<Option<RevocationState>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        let Some(authority) = self.revocations.authority_of(guild_ref, target_digest) else {
            return Ok(None);
        };
        Ok(Some(RevocationState {
            target_digest: *target_digest,
            authority: authority.clone(),
        }))
    }

    /// Whether a live hold names one candidate.
    ///
    /// The retention engine, the collector, and the deletion lifecycle ask
    /// this before they propose to destroy anything.
    pub(super) fn is_held(&self, guild_ref: &str, digest: &[u8; 32]) -> bool {
        self.holds.is_held(guild_ref, digest)
    }

    /// Refuse a write whose `supersedes` names a held candidate.
    ///
    /// A held candidate is not silently superseded: the correction waits for
    /// the explicit resolution that releases the hold. A `forgets` tombstone
    /// is not refused: the tombstone is the explicit erasure request, and
    /// the hold's own bindings — the collector and the deletion lifecycle —
    /// are defined on a forgotten target.
    pub(super) fn refuse_held_target(
        &self,
        record: &StoredRecord,
    ) -> Result<(), EpisodeStoreError> {
        let EpisodeEvent::MemoryCandidate { candidate, .. } = &record.event else {
            return Ok(());
        };
        let Some(target) = candidate.supersedes else {
            return Ok(());
        };
        if self.holds.is_held(&record.guild_ref, &target) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        Ok(())
    }

    /// Rebuild the revoked candidates of a replayed state.
    ///
    /// The collector rewrites the ledger and replays it into a fresh state;
    /// the revocations are not in the ledger, so they are re-applied to the
    /// fresh state, exactly as the reclaimed admissions are.
    pub(super) fn inject_revocations(&self, state: &mut LedgerState) {
        self.revocations.inject(state);
    }

    /// Whether the store serves one guild.
    fn served_guild(&self, guild_ref: &str) -> Result<(), EpisodeStoreError> {
        self.policy
            .guilds
            .get(guild_ref)
            .map(|_| ())
            .ok_or(EpisodeStoreError::StaleBinding)
    }

    /// Whether one digest names a live candidate of this guild.
    ///
    /// The check does not distinguish a forgotten candidate from an unknown
    /// digest, so the refusal that uses it leaks nothing about the digests
    /// the store does not hold.
    fn live_candidate(&self, guild_ref: &str, digest: &[u8; 32]) -> bool {
        self.state
            .memories
            .get(guild_ref)
            .is_some_and(|memories| memories.is_live(digest))
    }

    /// Whether one digest names a candidate a hold may freeze.
    ///
    /// A live candidate and an already forgotten one may both be held: the
    /// hold freezes the destruction of the content, and a forgotten
    /// candidate's payload and derived copies still exist outside the
    /// ledger. A tombstone is not a candidate — it is the erasure request
    /// itself — and is never held.
    fn holdable_candidate(&self, guild_ref: &str, digest: &[u8; 32]) -> bool {
        self.state.memories.get(guild_ref).is_some_and(|memories| {
            memories.admitted.contains_key(digest) && !memories.tombstones.contains(digest)
        })
    }
}

/// Whether one hold request is well formed.
fn validate_hold_request(request: &HoldRequest) -> Result<(), EpisodeStoreError> {
    if !bounded_identifier(&request.guild_ref, 128) || !valid_actor(&request.authority) {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(())
}

/// Whether one hold release request is well formed.
fn validate_release_request(request: &HoldReleaseRequest) -> Result<(), EpisodeStoreError> {
    if !bounded_identifier(&request.guild_ref, 128) || !valid_actor(&request.authority) {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(())
}

/// Whether one revocation request is well formed.
fn validate_revocation_request(request: &RevocationRequest) -> Result<(), EpisodeStoreError> {
    if !bounded_identifier(&request.guild_ref, 128) || !valid_actor(&request.authority) {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(())
}
