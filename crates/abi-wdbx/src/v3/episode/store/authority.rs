//! Guild-scoped authority for the v3 episode store.
//!
//! # Why the map lives in the host policy
//!
//! [`ActorRef`](crate::v3::episode::ActorRef) carries a principal and a
//! class and **no guild**, and the canonical encoder
//! (`store/canonical.rs`) binds `principal_id` and `kind` into every episode
//! commitment. A guild field on `ActorRef` would therefore move every
//! candidate digest, break the pinned goldens
//! (`tests/v3_memory_candidate.rs`, `tests/v3_memory_edge.rs`) and the
//! cross-language Python witness (`tools/abbey_cbor_episode_v1.py`), and
//! change the frozen v3 record format.
//!
//! The authority map therefore lives in the **host policy**
//! ([`GuildEpisodePolicy::authorized_principals`]), which is not episode
//! material: the canonical commitment binds the policy *version*, never the
//! policy itself, so naming a principal moves no committed digest.
//!
//! # The rule
//!
//! For a store serving a set of guilds, and an actor presenting the
//! principal `P` for the guild `G`:
//!
//! 1. **The guild's own map.** When `G`'s policy names a non-empty list of
//!    principals, `P` must be one of them. A principal the guild does not
//!    name is refused every governance action for that guild, whatever class
//!    it carries.
//! 2. **The store-wide scope.** When `G`'s policy names no list of its own
//!    (an absent map and an empty map are the same state), the class rule
//!    alone decides — but only for a principal that **no** guild's map
//!    names. A principal some guild names is *scoped* to the guilds that
//!    name it and keeps no class-only authority anywhere else.
//! 3. **The class rule** stays a second, necessary condition, applied per
//!    action: a service may never place a hold, revoke a candidate, raise a
//!    budget, or order a destruction, even when the map names it; a human
//!    subject is never an authority.
//!
//! Rule 2 is the refinement that makes rule 1 mean anything. Without it, a
//! principal named for guild A would keep its class-only authority over
//! every *other* guild, which is exactly the hole this slice closes: a
//! caller presenting the owner of one guild must not reach another guild.
//!
//! The scope is derived from the policy when the store opens
//! ([`scoped_principals`]), so it is a pure function of the host policy: no
//! clock, no state, and the same policy always yields the same verdict.
//!
//! # The three states of one guild's map
//!
//! - **absent** (`None`): the guild states no map of its own, and rule 2
//!   applies. This is a **compatibility behaviour, not a security
//!   guarantee**. A host that wants a guild closed to unknown principals has
//!   to name them.
//! - **non-empty** (`Some(list)` with entries): the guild names its
//!   authorities, and rule 1 applies.
//! - **empty** (`Some(vec![])`): the same state as an absent map. The guild
//!   names nobody, so it constrains nobody of its own, and rule 2 applies.
//!   An empty list is therefore *not* a closed guild: a host that means to
//!   close a guild names at least one principal.
//!
//! # The honest limit
//!
//! The substrate still cannot **verify** that a principal really holds a
//! role. The host policy is the authority of record: it states which
//! principals may act for a guild, and the store enforces that the principal
//! the caller presents is one of them. The store trusts the caller to name
//! the right principal and to present the right class, and it trusts the
//! host to keep the policy honest — a host that lists a principal it does
//! not trust has misconfigured its own policy.
//!
//! What the map closes, **for the principals the host names**, is the hole
//! the class rule alone left open: a caller that presented a `GuildOwner` for
//! a guild it did not own. A principal named for one guild is refused for
//! every guild that does not name it, and a guild that names its principals
//! accepts nobody else. A principal that *no* guild's map names is not
//! closed by the map at all: it still reaches every guild whose policy
//! states no map of its own, on the class rule alone.
//!
//! # What the map reaches
//!
//! The map constrains the **governance** actions: placing a hold, releasing
//! a hold, revoking a candidate, adjusting a budget, lifting an adjustment,
//! and ordering a destruction. The class rule is the second, necessary half
//! of every one of them.
//!
//! It reaches the deletion lifecycle's destruction requests
//! (`store/deletion.rs`). `RedactionRequest` and
//! `ObjectKeyDestructionRequest` carry an `actor`, and `decide_redaction`,
//! `commit_redaction`, `decide_object_key_destruction` and
//! `commit_object_key_destruction` refuse the order unless the principal is
//! one the guild authorizes **and** its class is one that may order a
//! destruction: a guild owner, a guild administrator, a guild manager, or an
//! organization owner. A service may not — it writes the candidates, and a
//! writer that could destroy the evidence it just wrote would defeat the
//! erasure record — and a human subject may not, because the subject of a
//! request is not an authority in this store. The sidecar lines record the
//! principal, so a destruction order is attributable.
//!
//! # What is not implemented here
//!
//! - The map does not constrain the canonical *write* path: a service still
//!   records memory candidates and memory edges for the guild, and a
//!   `resolves` edge is still admitted on the class rule alone (the write
//!   path is a canonical path, and its replay has no policy to read).
//! - The map does not reach the lifecycle `EpisodeEvent::Approval`
//!   (`store/validate.rs`). The approver is class-checked — a valid actor,
//!   not a service, not the requester and not the proposer — but it is
//!   **not** guild-scoped: no guild check is applied to it, so a principal
//!   labelled `GuildOwner` for guild A may approve an operation in guild B.
//!   That is a missing **guild** check, not a contradiction of the class
//!   rule: an approval is a *workflow* step, not a governance action over a
//!   guild's resources. It accepts one proposal and confers no authority
//!   over anything else, so the rule that a human subject is never an
//!   *authority* — a rule about governance authorities — does not reach it,
//!   and a `HumanSubject` is an admissible approver there by design. The
//!   store deliberately admits every non-service class as an approver; what
//!   it does not do is verify that the approver holds any standing in the
//!   guild of the operation it approves.
//! - The map is not committed, signed, or replayed: it is host policy, and
//!   changing it changes what the store accepts from the next open onwards —
//!   the store binds the policy at open and never reloads it. A hold placed
//!   under one map can be released only under a map that still names the
//!   releasing principal.
//! - The store does not verify guild membership, and it does not resolve
//!   roles from a directory. It compares one opaque principal id with the
//!   list the host wrote down.

use super::super::types::{ActorRef, StorePolicy};
use super::{EpisodeStore, EpisodeStoreError};
use std::collections::BTreeSet;

/// The principals that some guild's authority map names.
///
/// A named principal is **scoped** to the guilds that name it: it holds no
/// class-only authority anywhere else, so naming the owner of one guild is
/// what stops that owner from acting for another.
///
/// The set is derived from the host policy when the store opens. It is a
/// pure function of the policy, so the store's answer never depends on a
/// clock, on the order the policy was written, or on anything the ledger
/// holds.
pub(super) fn scoped_principals(policy: &StorePolicy) -> BTreeSet<String> {
    policy
        .guilds
        .values()
        .filter_map(|guild| guild.authorized_principal_list())
        .flatten()
        .cloned()
        .collect()
}

impl EpisodeStore {
    /// Whether the principal the caller presents may act as an authority for
    /// one guild.
    ///
    /// This is the guild-scoping half of the store's authority check, and it
    /// answers exactly one question: is the principal one that *this* guild's
    /// host policy authorizes?
    ///
    /// It is **not** the whole check. The actor *class* is a second,
    /// necessary condition, applied per action: a service may never place a
    /// hold, revoke a candidate, raise a budget, or order a destruction,
    /// whatever this answers, and a human subject is never an authority. A
    /// caller that consults this may therefore still be refused by the
    /// action itself.
    ///
    /// `false` also covers a guild the store does not serve: the store
    /// cannot act for a guild it holds no policy for.
    ///
    /// The answer is content-free and leaks nothing: it says whether the
    /// principal is authorized, never what the guild's map holds.
    #[must_use]
    pub fn actor_may_act(&self, guild_ref: &str, actor: &ActorRef) -> bool {
        let Some(policy) = self.policy.guilds.get(guild_ref) else {
            return false;
        };
        match policy.authorized_principal_list() {
            // The guild names its authorities: only they may act for it.
            Some(principals) => principals
                .iter()
                .any(|authorized| authorized == &actor.principal_id),
            // The guild states no map of its own, so the class rule alone
            // decides — but only for a principal no guild names. A principal
            // some guild names is scoped to the guilds that name it.
            None => !self.scoped_principals.contains(&actor.principal_id),
        }
    }

    /// The guild-scoping half of the authority check, as a refusal.
    ///
    /// The refusal is the closed, content-free
    /// [`EpisodeStoreError::InvalidTransition`], and it is the same refusal
    /// for every authority failure: a principal the guild does not name, a
    /// principal that belongs to another guild, and a principal the guild
    /// names but whose class forbids the action all answer the same closed
    /// error, so the refusal is not an oracle about the guild's map.
    ///
    /// The error carries neither the principal id nor the guild reference.
    pub(crate) fn require_guild_authority(
        &self,
        guild_ref: &str,
        actor: &ActorRef,
    ) -> Result<(), EpisodeStoreError> {
        if self.actor_may_act(guild_ref, actor) {
            Ok(())
        } else {
            Err(EpisodeStoreError::InvalidTransition)
        }
    }
}
