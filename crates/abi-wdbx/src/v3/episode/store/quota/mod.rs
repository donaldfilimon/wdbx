//! Quota administration for the v3 episode ledger.
//!
//! # What a quota adjustment is
//!
//! A guild's budgets — the cumulative token charge and the cumulative
//! serialized byte count the guild may reach — come from the host policy
//! ([`GuildEpisodePolicy::token_budget`](crate::v3::episode::GuildEpisodePolicy)
//! and [`GuildEpisodePolicy::storage_budget_bytes`](crate::v3::episode::GuildEpisodePolicy)).
//! This module lets an authority *adjust* those budgets at runtime, durably,
//! and auditably.
//!
//! The adjustment is not an episode. It is a governance decision *about* a
//! guild's budget, exactly like a hold or a revocation. Putting it in the
//! ledger would mean a new [`EpisodeEvent`](crate::v3::episode::EpisodeEvent)
//! variant, which changes the canonical event encoder, the cross-language
//! Python witness, and every pinned golden with it. So the adjustment lives
//! in a derived sidecar beside the ledger, `episodes.v1.quotas.jsonl`, in
//! the same shape as the retention, redaction, object-key, reclaimed, hold,
//! and revocation indexes:
//!
//! - one line per decision, holding the guild, the two budgets, the host
//!   policy's budgets in force when the decision was made, the authority, and
//!   the instant — never a digest, a payload, an event, or a participant;
//! - a checksum over each line's own fields, so a corrupted or half-written
//!   line is corruption rather than a silently dropped budget;
//! - a header line carrying the number of decision lines and a checksum over
//!   all of them, so a file truncated at a line boundary is corruption too;
//! - a trailing fragment is corruption, not a discarded crash tail: the
//!   writer only ever writes whole lines;
//! - the file is created with the first decision, not with the store, and a
//!   store that never adjusted a budget has no index file at all;
//! - the line is written and synchronized before the receipt that reports it
//!   is returned, so a receipt never claims a budget the store did not
//!   durably record;
//! - the same single writer that appends the ledger writes the index.
//!
//! The sidecar's format is described in the `index` submodule.
//!
//! # The effective budget
//!
//! **The host policy is the outer bound: the effective budget of a guild is
//! the host policy's budget for that guild, lowered to a recorded
//! adjustment's budget only while the policy still states budgets at or below
//! the ones in force when the adjustment was made, and never above the
//! policy's budget.**
//!
//! An adjustment is therefore bound to the policy it was made under. Raising
//! *either* of the guild's policy budgets above the ones in force at that
//! moment ends the adjustment's force, and the policy's budgets apply again.
//!
//! The effective budget is what the write gate uses
//! ([`EpisodeStore::check_quotas`]), so an adjustment changes what the store
//! accepts: the append path, the commitment preview, and the non-mutating
//! admission decision all read the same effective budget and reach the same
//! verdict.
//!
//! Three consequences follow, and they are the point of the rule:
//!
//! - an adjustment may only move a budget *within* the host's bound: it may
//!   lower the budget, and an authority may lift it back towards the
//!   policy's budget, but no request may set a budget above the policy's
//!   budget, whoever makes it. A host that wants a larger budget changes the
//!   policy;
//! - the host always holds the outer bound. A policy lowered below an
//!   adjustment's budget takes effect at once: the effective budget is never
//!   above the policy's;
//! - the host can always restore a guild's capacity, because the policy's
//!   budgets apply again as soon as the host raises them above the budgets
//!   the adjustment was made under, and
//!   [`EpisodeStore::clear_quota_adjustment`] lifts an adjustment outright.
//!   A writer can therefore never freeze a guild's budget permanently: the
//!   budget is a *guild* budget, so a service that lowers it restricts every
//!   writer in that guild, not only itself.
//!
//! # Who may adjust a budget
//!
//! The store's governance vocabulary decides, exactly as it decides who may
//! place a hold or resolve a memory edge:
//!
//! - a **guild owner**, a **guild administrator**, a **guild manager**, and
//!   an **organization owner** may adjust a guild's budgets within the host
//!   policy's bound;
//! - a **service** may adjust a budget only *downward*. A service is the
//!   writer of memory candidates, and a writer that could raise its own
//!   budget could defeat the budget;
//! - a **human subject** may not adjust a budget at all: the subject of a
//!   request is not an authority in this store.
//!
//! Only an authority that may raise a budget may *clear* one: clearing
//! restores the host policy's budget, so it is the same power as raising, and
//! a service may not do it.
//!
//! The refusal does not distinguish "this authority may never adjust" from
//! "this authority may not adjust in this direction": both answer the same
//! closed error, so the decision is not an oracle about the store's
//! governance.
//!
//! # The authority vocabulary is actor-class based *and* guild-scoped
//!
//! [`ActorRef`](crate::v3::episode::ActorRef) carries a principal and a
//! class; it carries no guild. The class rule below therefore decides *which
//! class* may adjust a budget, and the guild-scoped authority map
//! ([`GuildEpisodePolicy::authorized_principals`], read by
//! `super::authority`) decides *whether the principal may act for the target
//! guild at all*. Both are necessary, and neither widens the other: a
//! principal a guild names may still be refused for its class, and a
//! principal of the right class is still refused for a guild that does not
//! name it.
//!
//! Without the map, a principal labelled `GuildOwner` could adjust any guild
//! the store serves. With it, a host that names the principals of a guild
//! confines them to that guild, and a guild that names nobody falls back to
//! the class rule alone — a *compatibility behaviour, not a security
//! guarantee*.
//!
//! The substrate still cannot verify guild membership, and it does not
//! resolve roles from a directory: the host policy is the authority of
//! record, and the store trusts the caller to present the right principal
//! and the right class. A caller that resolves guild membership before
//! presenting an authority is the one that keeps the vocabulary honest; the
//! store does not and cannot do it.
//!
//! # The safety rule
//!
//! An adjustment may never lower a budget *below the usage already recorded
//! for the guild*. Lowering it there would retroactively invalidate writes
//! the ledger already holds: the store would be serving records that its own
//! budget says it may not hold. The store refuses such an adjustment instead
//! of accepting it and living with the inconsistency.
//!
//! # The clock
//!
//! Only the **commit** reads a clock: the adjustment records the instant it
//! was made, so the store refuses an adjustment it cannot date. The
//! non-mutating decision records nothing, so it reads no clock at all — a
//! pure decision is not coupled to an impure source it does not need. The
//! two agree on every outcome except the clock-unavailable one, where the
//! commit refuses and the decision succeeds.
//!
//! The same state is reachable without the adjustment API: a host policy
//! lowered below the recorded usage creates it. The store does not refuse to
//! open in that case — refusing would brick a store whose policy was
//! lowered — it *surfaces* the inconsistency instead:
//! [`EpisodeStore::quota_state`] reports `over_budget` beside the budgets
//! and the usage, and the store detects the state at open, after the ledger
//! replay and after the quota index is read.
//!
//! # What is not implemented here
//!
//! - The store holds no accounting of *what* consumed the budget beyond the
//!   cumulative counters it already keeps; a per-caller or per-class budget
//!   is not modelled.
//! - The substrate cannot verify guild membership; the map says which
//!   principals a guild authorizes, not what role they hold. See the section
//!   above.
//! - An adjustment is not an episode: it carries no canonical commitment and
//!   no signature, exactly like a hold and a revocation. A quota
//!   administration that is itself an episode needs a new event variant and a
//!   coordinated golden regeneration.
//! - The sidecar holds every decision, but the store answers
//!   [`EpisodeStore::quota_history`] for one guild at a time.
//! - The sidecar is not an append-only log: it carries a header and is
//!   rewritten whole on every decision, so a file written by an earlier
//!   build of this slice does not parse. The sidecar is new and unratified.

mod index;

pub(super) use index::QuotaIndex;

use super::super::types::{ActorKind, ActorRef};
use super::{EpisodeStore, EpisodeStoreError, MAX_LEDGER_BYTES, bounded_identifier, valid_actor};

/// The greatest cumulative token budget a guild may be given.
///
/// The same bound the host policy itself is held to, so an adjustment cannot
/// buy a budget the policy could not have stated.
const MAX_TOKEN_BUDGET: u64 = 1_000_000_000;

/// One request to adjust a guild's budgets.
///
/// The request names the guild, the budgets the store must use for it from
/// now on, and the authority making the request. It carries no episode
/// content: no digest, no payload, no event, no participant.
///
/// The authority is an *actor class* plus the guild's own authority map: the
/// store checks that the principal is one the target guild authorizes
/// (`GuildEpisodePolicy::authorized_principals`) and that its class may make
/// this adjustment, and it trusts the caller to present a principal that
/// really holds that class in the target guild, because the substrate cannot
/// verify guild membership. See the module documentation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuotaAdjustmentRequest {
    /// The guild whose budgets are adjusted.
    pub guild_ref: String,
    /// The cumulative token budget the store must use for this guild.
    pub token_budget: u64,
    /// The cumulative storage budget, in bytes, the store must use for this
    /// guild.
    pub storage_budget_bytes: u64,
    /// The authority making the adjustment.
    pub authority: ActorRef,
}

/// What the store will do with a quota adjustment.
///
/// The decision is the non-mutating half: it runs the same checks
/// [`EpisodeStore::commit_quota_adjustment`] runs and commits nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuotaAdjustmentDecision {
    /// The guild whose budgets are adjusted.
    pub guild_ref: String,
    /// The token budget the store would use for this guild afterwards.
    pub token_budget: u64,
    /// The storage budget the store would use for this guild afterwards.
    pub storage_budget_bytes: u64,
    /// The authority making the adjustment.
    pub authority: ActorRef,
    /// True when the adjustment raises the token budget above the effective
    /// one the store holds right now.
    pub raises_token_budget: bool,
    /// True when the adjustment raises the storage budget above the effective
    /// one the store holds right now.
    pub raises_storage_budget: bool,
}

/// The receipt of a committed quota adjustment.
///
/// Content-free: a guild, two budgets, an authority, an instant, and two
/// flags. It never carries a digest, a payload, or an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuotaAdjustmentReceipt {
    /// The guild whose budgets were adjusted.
    pub guild_ref: String,
    /// The effective token budget the store uses for this guild.
    pub token_budget: u64,
    /// The effective storage budget the store uses for this guild.
    pub storage_budget_bytes: u64,
    /// The authority that made the adjustment.
    pub authority: ActorRef,
    /// The millisecond at which the store recorded the adjustment.
    pub adjusted_at_ms: u64,
    /// True when the adjustment raised the token budget.
    pub raises_token_budget: bool,
    /// True when the adjustment raised the storage budget.
    pub raises_storage_budget: bool,
}

/// The receipt of a committed quota reset.
///
/// Content-free: a guild, two budgets, an authority, and an instant. It
/// never carries a digest, a payload, or an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuotaResetReceipt {
    /// The guild whose adjustment was lifted.
    pub guild_ref: String,
    /// The effective token budget the store uses for this guild afterwards:
    /// the host policy's.
    pub token_budget: u64,
    /// The effective storage budget the store uses for this guild
    /// afterwards: the host policy's.
    pub storage_budget_bytes: u64,
    /// The authority that lifted the adjustment.
    pub authority: ActorRef,
    /// The millisecond at which the store recorded the reset.
    pub cleared_at_ms: u64,
}

/// One recorded quota decision, as the store's audit history reports it.
///
/// Content-free by construction: a guild, two budgets, an authority, and an
/// instant. It never carries a digest, a payload, or an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuotaAdjustmentRecord {
    /// The guild the decision belongs to.
    pub guild_ref: String,
    /// The token budget the decision set, or the one it lifted.
    pub token_budget: u64,
    /// The storage budget the decision set, or the one it lifted.
    pub storage_budget_bytes: u64,
    /// The authority that made the decision.
    pub authority: ActorRef,
    /// The millisecond at which the store recorded the decision.
    pub adjusted_at_ms: u64,
    /// True when the decision lifted an adjustment instead of making one.
    pub cleared: bool,
}

/// The effective quota state of one guild.
///
/// Content-free by construction: budgets, counters, an authority, and an
/// instant. It never carries a digest, a payload, or an event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuotaState {
    /// The guild the state belongs to.
    pub guild_ref: String,
    /// The effective token budget the write gate uses.
    pub token_budget: u64,
    /// The effective storage budget the write gate uses.
    pub storage_budget_bytes: u64,
    /// The token budget the host policy states.
    pub policy_token_budget: u64,
    /// The storage budget the host policy states.
    pub policy_storage_budget_bytes: u64,
    /// The authority that last adjusted the budget, when an adjustment is in
    /// force.
    pub adjusted_by: Option<ActorRef>,
    /// The millisecond at which the budget was last adjusted, when an
    /// adjustment is in force.
    pub adjusted_at_ms: Option<u64>,
    /// The cumulative token charge recorded for the guild.
    pub tokens_used: u64,
    /// The cumulative byte count recorded for the guild.
    pub bytes_used: u64,
    /// True when the recorded usage is above the effective budget.
    ///
    /// The adjustment API refuses to create that state; a host policy lowered
    /// below the recorded usage can. The store surfaces it here instead of
    /// refusing to open.
    pub over_budget: bool,
}

/// Whether one authority may make one adjustment.
///
/// The governance vocabulary is the store's own: a guild owner, a guild
/// administrator, a guild manager, or an organization owner decides. A
/// service may only *lower* a budget — a writer that could raise its own
/// budget could defeat the budget — and a human subject may not adjust a
/// budget at all.
fn may_adjust(kind: ActorKind, current: (u64, u64), request: &QuotaAdjustmentRequest) -> bool {
    match kind {
        ActorKind::GuildOwner
        | ActorKind::GuildAdministrator
        | ActorKind::GuildManager
        | ActorKind::OrganizationOwner => true,
        ActorKind::Service => {
            request.token_budget <= current.0 && request.storage_budget_bytes <= current.1
        }
        ActorKind::HumanSubject => false,
    }
}

/// Whether one authority may lift an adjustment.
///
/// Lifting an adjustment restores the host policy's budget, so it is the
/// same power as raising a budget: the authorities that may raise may lift,
/// and a service may not.
fn may_clear(kind: ActorKind) -> bool {
    matches!(
        kind,
        ActorKind::GuildOwner
            | ActorKind::GuildAdministrator
            | ActorKind::GuildManager
            | ActorKind::OrganizationOwner
    )
}

/// Whether one quota adjustment request is well formed.
fn validate_adjustment(request: &QuotaAdjustmentRequest) -> Result<(), EpisodeStoreError> {
    if !bounded_identifier(&request.guild_ref, 128)
        || !valid_actor(&request.authority)
        || request.token_budget > MAX_TOKEN_BUDGET
        || request.storage_budget_bytes > MAX_LEDGER_BYTES
    {
        return Err(EpisodeStoreError::InvalidInput);
    }
    Ok(())
}

impl EpisodeStore {
    /// The budgets the write gate uses for one guild.
    ///
    /// The effective budget is the host policy's budget, lowered where a
    /// quota adjustment applies. `None` means the store does not serve that
    /// guild at all.
    pub(super) fn effective_budgets(&self, guild_ref: &str) -> Option<(u64, u64)> {
        self.quotas.effective_budgets(&self.policy, guild_ref)
    }

    /// Decide whether a quota adjustment would be accepted, without recording
    /// it.
    ///
    /// The decision runs the same checks [`Self::commit_quota_adjustment`]
    /// runs: the request must be well formed, name a guild the store serves,
    /// stay within the host policy's bound, come from an authority that may
    /// make this adjustment, and not lower a budget below the usage the
    /// guild has already recorded.
    ///
    /// **The decision reads no clock.** The *commit* records the instant the
    /// adjustment was made, so the commit needs a readable clock and refuses
    /// an adjustment it cannot date. The decision records nothing, so it has
    /// no instant to read and no reason to refuse: a pure decision is not
    /// coupled to an impure source it does not need. The two therefore agree
    /// on every outcome except that one — the commit refuses with
    /// [`EpisodeStoreError::InvalidInput`] and the decision succeeds.
    ///
    /// It records nothing and appends nothing.
    pub fn decide_quota_adjustment(
        &self,
        request: &QuotaAdjustmentRequest,
    ) -> Result<QuotaAdjustmentDecision, EpisodeStoreError> {
        validate_adjustment(request)?;
        let Some(guild) = self.policy.guilds.get(&request.guild_ref) else {
            return Err(EpisodeStoreError::StaleBinding);
        };
        // The guild-scoping half of the authority check: the adjusting
        // principal must be one this guild authorizes. The class rule below
        // is the second, necessary half. The check runs before the budget
        // and usage checks, so a principal this guild does not authorize
        // learns nothing about the guild's budgets.
        self.require_guild_authority(&request.guild_ref, &request.authority)?;
        let policy = (guild.token_budget, guild.storage_budget_bytes);
        let Some(current) = self.effective_budgets(&request.guild_ref) else {
            return Err(EpisodeStoreError::StaleBinding);
        };
        // The host policy is the outer bound: no adjustment may set a budget
        // above it, whoever makes it. A host that wants a larger budget
        // changes the policy.
        if request.token_budget > policy.0 || request.storage_budget_bytes > policy.1 {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        if !may_adjust(request.authority.kind, current, request) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        // The safety rule: an adjustment may never lower a budget below the
        // usage the guild has already recorded. Lowering it there would
        // retroactively invalidate writes the ledger already holds.
        let usage = self
            .state
            .guild_usage
            .get(&request.guild_ref)
            .copied()
            .unwrap_or_default();
        if request.token_budget < usage.tokens || request.storage_budget_bytes < usage.bytes {
            return Err(EpisodeStoreError::InvalidInput);
        }
        Ok(QuotaAdjustmentDecision {
            guild_ref: request.guild_ref.clone(),
            token_budget: request.token_budget,
            storage_budget_bytes: request.storage_budget_bytes,
            authority: request.authority.clone(),
            raises_token_budget: request.token_budget > current.0,
            raises_storage_budget: request.storage_budget_bytes > current.1,
        })
    }

    /// Adjust a guild's budgets.
    ///
    /// This commits: the adjustment is written to the store's quota index by
    /// the same single writer that appends the ledger, and the sidecar is
    /// synchronized before the receipt is returned. The effective budget
    /// changes in the same call, so the very next write is judged against it.
    ///
    /// The request is not idempotent: every adjustment is recorded, because
    /// the index is the audit trail of who moved the budget and when. The
    /// *effective* budget is the one the last adjustment set.
    ///
    /// An adjustment may only restrict a guild within the host policy's
    /// budget: a request above the policy is refused. The host restores a
    /// larger budget by changing the policy, or by clearing the adjustment
    /// with [`Self::clear_quota_adjustment`].
    ///
    /// The commit is the one path that reads a clock, because it is the one
    /// path that records an instant. Without a readable clock the store
    /// cannot say when the adjustment was made, so it refuses rather than
    /// record an untimed decision.
    pub fn commit_quota_adjustment(
        &mut self,
        request: &QuotaAdjustmentRequest,
    ) -> Result<QuotaAdjustmentReceipt, EpisodeStoreError> {
        self.commit_quota_adjustment_with_clock(request, super::expiry::now_ms)
    }

    /// The commit, with the clock it dates the adjustment with.
    ///
    /// The clock is a parameter so that the "no readable clock" case is
    /// reachable: the commit refuses an adjustment it cannot date, while the
    /// decision — which dates nothing — succeeds. The public commit reads
    /// the store's own clock.
    fn commit_quota_adjustment_with_clock(
        &mut self,
        request: &QuotaAdjustmentRequest,
        clock: impl FnOnce() -> Option<u64>,
    ) -> Result<QuotaAdjustmentReceipt, EpisodeStoreError> {
        if self.poisoned {
            return Err(EpisodeStoreError::Io);
        }
        let decision = self.decide_quota_adjustment(request)?;
        // An adjustment is auditable: it records who adjusted the budget and
        // when. Without a readable clock the store cannot say when, so it
        // refuses rather than record an untimed decision.
        let adjusted_at_ms = clock().ok_or(EpisodeStoreError::InvalidInput)?;
        let guild = self
            .policy
            .guilds
            .get(&decision.guild_ref)
            .ok_or(EpisodeStoreError::StaleBinding)?;
        let policy = (guild.token_budget, guild.storage_budget_bytes);
        self.quotas.commit_adjustment(
            &decision.guild_ref,
            decision.token_budget,
            decision.storage_budget_bytes,
            policy,
            &decision.authority,
            adjusted_at_ms,
        )?;
        Ok(QuotaAdjustmentReceipt {
            guild_ref: decision.guild_ref,
            token_budget: decision.token_budget,
            storage_budget_bytes: decision.storage_budget_bytes,
            authority: decision.authority,
            adjusted_at_ms,
            raises_token_budget: decision.raises_token_budget,
            raises_storage_budget: decision.raises_storage_budget,
        })
    }

    /// Lift a guild's quota adjustment.
    ///
    /// The host policy's budget applies to the guild again, and the reset is
    /// recorded in the same sidecar as the adjustment it lifts, so the audit
    /// history shows who lifted it and when.
    ///
    /// Only an authority that may raise a budget may lift one: a service may
    /// not, and a human subject may not.
    pub fn clear_quota_adjustment(
        &mut self,
        guild_ref: &str,
        authority: &ActorRef,
    ) -> Result<QuotaResetReceipt, EpisodeStoreError> {
        if self.poisoned {
            return Err(EpisodeStoreError::Io);
        }
        if !bounded_identifier(guild_ref, 128) || !valid_actor(authority) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        let Some(guild) = self.policy.guilds.get(guild_ref) else {
            return Err(EpisodeStoreError::StaleBinding);
        };
        // The guild-scoping half of the authority check: the clearing
        // principal must be one this guild authorizes. The class rule below
        // is the second, necessary half.
        self.require_guild_authority(guild_ref, authority)?;
        if !may_clear(authority.kind) {
            return Err(EpisodeStoreError::InvalidTransition);
        }
        let cleared_at_ms = super::expiry::now_ms().ok_or(EpisodeStoreError::InvalidInput)?;
        let policy = (guild.token_budget, guild.storage_budget_bytes);
        // The budgets the reset lifts: the effective ones the store holds
        // right now.
        let lifted = self.effective_budgets(guild_ref).unwrap_or(policy);
        self.quotas.commit_clear(
            guild_ref,
            lifted.0,
            lifted.1,
            policy,
            authority,
            cleared_at_ms,
        )?;
        Ok(QuotaResetReceipt {
            guild_ref: guild_ref.to_string(),
            token_budget: policy.0,
            storage_budget_bytes: policy.1,
            authority: authority.clone(),
            cleared_at_ms,
        })
    }

    /// Report the effective quota state of one guild.
    ///
    /// `Ok(None)` means the store does not serve that guild. The state is
    /// content-free: the effective budgets, the policy's budgets, the
    /// authority that last adjusted the budget and when, and the guild's
    /// cumulative usage.
    ///
    /// `over_budget` is true when the recorded usage is above the effective
    /// budget. The adjustment API refuses to create that state; a host policy
    /// lowered below the recorded usage can, and the store reports it here
    /// instead of refusing to open.
    pub fn quota_state(&self, guild_ref: &str) -> Result<Option<QuotaState>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        let Some(guild) = self.policy.guilds.get(guild_ref) else {
            return Ok(None);
        };
        let Some((token_budget, storage_budget_bytes)) = self.effective_budgets(guild_ref) else {
            return Ok(None);
        };
        let recorded = self
            .quotas
            .override_for(guild_ref)
            .filter(|recorded| recorded.in_force((guild.token_budget, guild.storage_budget_bytes)));
        let usage = self
            .state
            .guild_usage
            .get(guild_ref)
            .copied()
            .unwrap_or_default();
        Ok(Some(QuotaState {
            guild_ref: guild_ref.to_string(),
            token_budget,
            storage_budget_bytes,
            policy_token_budget: guild.token_budget,
            policy_storage_budget_bytes: guild.storage_budget_bytes,
            adjusted_by: recorded.map(|recorded| recorded.authority.clone()),
            adjusted_at_ms: recorded.map(|recorded| recorded.adjusted_at_ms),
            tokens_used: usage.tokens,
            bytes_used: usage.bytes,
            over_budget: self.quotas.is_over_budget(guild_ref)
                || usage.tokens > token_budget
                || usage.bytes > storage_budget_bytes,
        }))
    }

    /// Report the quota decisions the store holds for one guild, in order.
    ///
    /// The history is content-free: a guild, two budgets, an authority, an
    /// instant, and whether the decision lifted an adjustment. It is the
    /// audit trail of the sidecar, which is otherwise invisible to a caller.
    ///
    /// `Err(StaleBinding)` means the store does not serve that guild.
    pub fn quota_history(
        &self,
        guild_ref: &str,
    ) -> Result<Vec<QuotaAdjustmentRecord>, EpisodeStoreError> {
        if !bounded_identifier(guild_ref, 128) {
            return Err(EpisodeStoreError::InvalidInput);
        }
        if !self.policy.guilds.contains_key(guild_ref) {
            return Err(EpisodeStoreError::StaleBinding);
        }
        Ok(self
            .quotas
            .history(guild_ref)
            .into_iter()
            .map(|decision| QuotaAdjustmentRecord {
                guild_ref: decision.guild_ref,
                token_budget: decision.token_budget,
                storage_budget_bytes: decision.storage_budget_bytes,
                authority: decision.authority,
                adjusted_at_ms: decision.adjusted_at_ms,
                cleared: decision.cleared,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v3::episode::{GuildEpisodePolicy, StorePolicy};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// A store directory that removes itself.
    struct Scratch(PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The policy the adjustment tests run under.
    fn policy() -> StorePolicy {
        StorePolicy {
            contract_revision: 2,
            contract_digest: [9; 32],
            guilds: BTreeMap::from([(
                "guild_ref".to_string(),
                GuildEpisodePolicy {
                    learning_enabled: true,
                    policy_version: "policy_v1".into(),
                    token_budget: 1_000,
                    storage_budget_bytes: 1024 * 1024,
                    current_consent_epoch: None,
                    retention_ttl_ms: None,
                    retention_windows: None,
                    authorized_principals: None,
                },
            )]),
        }
    }

    /// The owner of the guild: the authority the governance order admits.
    fn owner() -> ActorRef {
        ActorRef {
            principal_id: "guild_owner".into(),
            kind: ActorKind::GuildOwner,
        }
    }

    /// The decision reads no clock, so it succeeds when the clock is
    /// unavailable; the commit records the instant, so it refuses.
    ///
    /// The clock is the only thing that separates the two: with a readable
    /// clock they agree, and every other refusal the decision reports is a
    /// refusal the commit reports too.
    #[test]
    fn the_decision_reads_no_clock_and_the_commit_refuses_without_one() {
        let scratch = Scratch(
            std::env::temp_dir().join(format!("wdbx-quota-clock-{}", uuid::Uuid::new_v4())),
        );
        let mut store = EpisodeStore::open(&scratch.0, policy()).expect("open the store");
        let request = QuotaAdjustmentRequest {
            guild_ref: "guild_ref".into(),
            token_budget: 500,
            storage_budget_bytes: 1024 * 1024,
            authority: owner(),
        };

        // With a readable clock the two agree: the decision succeeds, and
        // the commit records the adjustment.
        let decision = store
            .decide_quota_adjustment(&request)
            .expect("the decision reads no clock");
        assert_eq!(decision.token_budget, 500);
        store
            .commit_quota_adjustment(&request)
            .expect("the commit records the adjustment");

        // With no readable clock the decision still succeeds — it dates
        // nothing — and the commit refuses, because it cannot say when the
        // adjustment was made.
        let decision = store
            .decide_quota_adjustment(&request)
            .expect("the decision reads no clock");
        assert_eq!(decision.token_budget, 500);
        assert!(matches!(
            store.commit_quota_adjustment_with_clock(&request, || None),
            Err(EpisodeStoreError::InvalidInput)
        ));

        // The refusal changed nothing: the effective budget is still the
        // one the earlier commit set.
        let state = store
            .quota_state("guild_ref")
            .expect("quota state")
            .expect("a served guild");
        assert_eq!(state.token_budget, 500);
    }
}
