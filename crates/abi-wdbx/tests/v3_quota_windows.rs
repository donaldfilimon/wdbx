//! Quota administration for the v3 episode store.
//!
//! What these tests pin:
//!
//! - the host policy is the outer bound: an adjustment may only restrict a
//!   guild within the policy's budget, and no adjustment may set a budget
//!   above the policy's;
//! - an adjustment lowers the effective budget, and the store then refuses a
//!   write it previously accepted;
//! - who may adjust a budget: a guild owner, administrator, or manager and
//!   an organization owner may, a service may only lower one, and a human
//!   subject may not;
//! - an adjustment never lowers a budget below the usage already recorded;
//! - the host always holds the outer bound: reopening with a larger policy
//!   budget restores the effective budget, a policy lowered below an
//!   adjustment takes effect, and an authority may clear an adjustment
//!   outright while a service may not;
//! - `quota_state` reports the effective budgets, the usage, and whether the
//!   recorded usage is above the effective budget; the adjustment survives
//!   close and reopen, and a damaged sidecar is corruption;
//! - `quota_history` reports every decision the store holds for a guild, in
//!   order;
//! - `decide_write` and `propose_write` agree on the post-adjustment
//!   verdict;
//! - the non-mutating quota decision and the quota commit agree on every
//!   outcome: the decision reads no clock, and the commit is the only path
//!   that dates the adjustment;
//! - the quota sidecar holds no episode content.
//!
//! The per-class retention windows are pinned in `v3_retention_windows.rs`.

#[path = "support/quota_windows.rs"]
mod support;

use abi_wdbx::v3::episode::{ActorKind, AdmissionReason, EpisodeStore, EpisodeStoreError};
use std::fs;
use support::{
    LEDGER_FILE, QUOTA_FILE, Scratch, actor, adjustment, append, candidate, digest_encodings,
    holds, memory, owner, policy, rejected, service, store_bytes,
};

#[test]
fn an_adjustment_lowers_the_effective_token_budget() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(10)).expect("open");
    let event = memory("request_1", "memory_1", 5, candidate(1));

    // Without an adjustment the write fits the policy's budget.
    assert!(store.decide_write(&event).is_ok());
    let digest = append(&mut store, event);
    assert_ne!(digest, [0; 32]);

    // The owner lowers the budget to the usage the guild has recorded: no
    // further token fits.
    let receipt = store
        .commit_quota_adjustment(&adjustment("guild_ref", 5, 1024 * 1024, owner()))
        .expect("an owner may adjust the budget");
    assert_eq!(receipt.guild_ref, "guild_ref");
    assert_eq!(receipt.token_budget, 5);
    assert!(!receipt.raises_token_budget);
    assert!(!receipt.raises_storage_budget);
    assert!(matches!(
        rejected(
            &mut store,
            &memory("request_2", "memory_2", 1, candidate(2))
        ),
        EpisodeStoreError::TokenBudget
    ));
    assert!(matches!(
        store
            .decide_write(&memory("request_3", "memory_3", 1, candidate(3)))
            .expect_err("the decision refuses the write on budget"),
        EpisodeStoreError::TokenBudget
    ));

    // The other guild is untouched: the adjustment is per guild.
    let state = store
        .quota_state("other_guild")
        .expect("quota state")
        .expect("a served guild");
    assert_eq!(state.token_budget, 10);
    assert_eq!(state.adjusted_by, None);
}

#[test]
fn only_a_governance_authority_may_adjust_a_budget() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(2)).expect("open");

    // A service may not raise a budget: a writer that could raise its own
    // budget could defeat the budget.
    assert!(matches!(
        store
            .decide_quota_adjustment(&adjustment(
                "guild_ref",
                1_000,
                1024 * 1024,
                actor("abbey_service", ActorKind::Service)
            ))
            .expect_err("a service may not raise a budget"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(matches!(
        store
            .commit_quota_adjustment(&adjustment(
                "guild_ref",
                1_000,
                1024 * 1024,
                actor("abbey_service", ActorKind::Service)
            ))
            .expect_err("a service may not raise a budget"),
        EpisodeStoreError::InvalidTransition
    ));

    // A human subject is not an authority in this store.
    assert!(matches!(
        store
            .decide_quota_adjustment(&adjustment(
                "guild_ref",
                1_000,
                1024 * 1024,
                actor("member", ActorKind::HumanSubject)
            ))
            .expect_err("a human subject may not adjust a budget"),
        EpisodeStoreError::InvalidTransition
    ));

    // A guild the store does not serve has no quota to adjust.
    assert!(matches!(
        store
            .decide_quota_adjustment(&adjustment("absent_guild", 10, 10, owner()))
            .expect_err("an unserved guild"),
        EpisodeStoreError::StaleBinding
    ));
    assert!(
        store
            .quota_state("absent_guild")
            .expect("quota state")
            .is_none()
    );
}

#[test]
fn a_service_may_lower_a_budget_and_an_authority_may_set_it_within_the_policy() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1_000)).expect("open");

    // A service may lower a budget: that only restricts the writers of the
    // guild, never raises what they may spend.
    let lowering = adjustment("guild_ref", 100, 1024 * 1024, service());
    let decision = store
        .decide_quota_adjustment(&lowering)
        .expect("a service may lower a budget");
    assert!(!decision.raises_token_budget);
    assert!(!decision.raises_storage_budget);
    assert_eq!(decision.token_budget, 100);
    store
        .commit_quota_adjustment(&lowering)
        .expect("a service may lower a budget");
    assert_eq!(
        store
            .quota_state("guild_ref")
            .expect("quota state")
            .expect("a served guild")
            .token_budget,
        100
    );

    // A guild manager may set the budget again, as long as it stays within
    // the host policy's bound.
    let manager = actor("guild_manager", ActorKind::GuildManager);
    let decision = store
        .decide_quota_adjustment(&adjustment("guild_ref", 500, 1024 * 1024, manager.clone()))
        .expect("a manager may adjust a budget");
    assert!(decision.raises_token_budget);
    store
        .commit_quota_adjustment(&adjustment("guild_ref", 500, 1024 * 1024, manager))
        .expect("a manager may adjust a budget");
    // So may a guild administrator and an organization owner.
    for authority in [
        actor("guild_admin", ActorKind::GuildAdministrator),
        actor("org_owner", ActorKind::OrganizationOwner),
    ] {
        store
            .commit_quota_adjustment(&adjustment(
                "guild_ref",
                500,
                1024 * 1024,
                authority.clone(),
            ))
            .expect("a governance authority may adjust a budget");
        assert_eq!(
            store
                .quota_state("guild_ref")
                .expect("quota state")
                .expect("a served guild")
                .adjusted_by,
            Some(authority)
        );
    }
    // The owner may still adjust.
    assert!(
        store
            .decide_quota_adjustment(&adjustment("guild_ref", 500, 1024 * 1024, owner()))
            .is_ok()
    );
}

#[test]
fn an_adjustment_below_the_recorded_usage_is_refused() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1_000)).expect("open");
    append(
        &mut store,
        memory("request_1", "memory_1", 10, candidate(1)),
    );
    let (tokens_used, bytes_used) = store.guild_usage("guild_ref").expect("recorded usage");
    assert_eq!(tokens_used, 10);
    assert!(bytes_used > 0);

    // Lowering the token budget below the recorded usage is refused: it
    // would retroactively invalidate the writes the ledger already holds.
    let below = adjustment("guild_ref", tokens_used - 1, 1024 * 1024, owner());
    assert!(matches!(
        store
            .decide_quota_adjustment(&below)
            .expect_err("below the recorded usage"),
        EpisodeStoreError::InvalidInput
    ));
    assert!(matches!(
        store
            .commit_quota_adjustment(&below)
            .expect_err("below the recorded usage"),
        EpisodeStoreError::InvalidInput
    ));

    // The same rule holds for the storage budget.
    assert!(matches!(
        store
            .decide_quota_adjustment(&adjustment(
                "guild_ref",
                tokens_used,
                bytes_used - 1,
                owner()
            ))
            .expect_err("below the recorded usage"),
        EpisodeStoreError::InvalidInput
    ));

    // A budget exactly at the recorded usage is allowed, and it is the
    // budget the gate then uses: no further token fits.
    store
        .commit_quota_adjustment(&adjustment("guild_ref", tokens_used, bytes_used, owner()))
        .expect("a budget at the recorded usage is allowed");
    assert!(matches!(
        rejected(
            &mut store,
            &memory("request_2", "memory_2", 1, candidate(2))
        ),
        EpisodeStoreError::TokenBudget
    ));
}

#[test]
fn the_quota_state_reports_the_effective_budgets_and_survives_a_reopen() {
    let scratch = Scratch::new();
    let store_policy = policy(10);
    let adjusted_at_ms;
    {
        let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
        append(&mut store, memory("request_1", "memory_1", 1, candidate(1)));

        // Before any adjustment the effective budget is the policy's.
        let state = store
            .quota_state("guild_ref")
            .expect("quota state")
            .expect("a served guild");
        assert_eq!(state.guild_ref, "guild_ref");
        assert_eq!(state.token_budget, 10);
        assert_eq!(state.storage_budget_bytes, 1024 * 1024);
        assert_eq!(state.policy_token_budget, 10);
        assert_eq!(state.policy_storage_budget_bytes, 1024 * 1024);
        assert_eq!(state.adjusted_by, None);
        assert_eq!(state.adjusted_at_ms, None);
        assert_eq!(state.tokens_used, 1);
        assert!(state.bytes_used > 0);
        assert!(!state.over_budget);

        let receipt = store
            .commit_quota_adjustment(&adjustment("guild_ref", 5, 512 * 1024, owner()))
            .expect("an owner may adjust the budget");
        adjusted_at_ms = receipt.adjusted_at_ms;

        // After the adjustment the effective budget is the adjusted one,
        // and the policy's budget is still reported beside it.
        let state = store
            .quota_state("guild_ref")
            .expect("quota state")
            .expect("a served guild");
        assert_eq!(state.token_budget, 5);
        assert_eq!(state.storage_budget_bytes, 512 * 1024);
        assert_eq!(state.policy_token_budget, 10);
        assert_eq!(state.policy_storage_budget_bytes, 1024 * 1024);
        assert_eq!(state.adjusted_by, Some(owner()));
        assert_eq!(state.adjusted_at_ms, Some(adjusted_at_ms));
        assert_eq!(state.tokens_used, 1);
    }

    // The adjustment survives close and reopen: the sidecar is the store's
    // durable record of it.
    let reopened = EpisodeStore::open(scratch.path(), store_policy).expect("verified reopen");
    let state = reopened
        .quota_state("guild_ref")
        .expect("quota state")
        .expect("a served guild");
    assert_eq!(state.token_budget, 5);
    assert_eq!(state.storage_budget_bytes, 512 * 1024);
    assert_eq!(state.adjusted_by, Some(owner()));
    assert_eq!(state.adjusted_at_ms, Some(adjusted_at_ms));
    assert_eq!(state.tokens_used, 1);

    // A guild reference that is not a bounded identifier is refused.
    assert!(matches!(
        reopened.quota_state("Not A Guild").expect_err("bounded"),
        EpisodeStoreError::InvalidInput
    ));

    // The host policy is the outer bound: reopening with a larger policy
    // budget restores the guild's effective budget to the policy's.
    drop(reopened);
    let relarger = EpisodeStore::open(scratch.path(), policy(9_000)).expect("verified reopen");
    let state = relarger
        .quota_state("guild_ref")
        .expect("quota state")
        .expect("a served guild");
    assert_eq!(state.policy_token_budget, 9_000);
    assert_eq!(
        state.token_budget, 9_000,
        "the host's larger policy applies again"
    );
    assert_eq!(
        state.adjusted_by, None,
        "the adjustment is no longer in force"
    );
}

#[test]
fn a_damaged_quota_sidecar_is_corruption() {
    let scratch = Scratch::new();
    {
        let mut store = EpisodeStore::open(scratch.path(), policy(1_000)).expect("open");
        store
            .commit_quota_adjustment(&adjustment("guild_ref", 100, 1024 * 1024, owner()))
            .expect("the first adjustment");
        store
            .commit_quota_adjustment(&adjustment("guild_ref", 200, 1024 * 1024, owner()))
            .expect("the second adjustment");
    }
    let path = scratch.path().join(QUOTA_FILE);
    let raw = fs::read(&path).expect("read the sidecar");
    assert_eq!(
        String::from_utf8_lossy(&raw).lines().count(),
        3,
        "a header line and one line per adjustment"
    );

    // A trailing fragment is corruption: the writer only ever writes whole
    // lines, so a fragment is damage rather than a crash tail.
    fs::write(&path, &raw[..raw.len() - 12]).expect("truncate the sidecar");
    assert!(matches!(
        EpisodeStore::open(scratch.path(), policy(1_000)).expect_err("a truncated sidecar"),
        EpisodeStoreError::Corrupt
    ));

    // A file truncated at a line boundary is corruption too: the header
    // carries the number of decisions and a checksum over all of them.
    let head_end = raw
        .iter()
        .position(|byte| *byte == b'\n')
        .expect("a header line");
    fs::write(&path, &raw[..=head_end]).expect("truncate the sidecar");
    assert!(matches!(
        EpisodeStore::open(scratch.path(), policy(1_000)).expect_err("a truncated sidecar"),
        EpisodeStoreError::Corrupt
    ));

    // A complete line that does not parse is corruption: the store refuses
    // to open rather than silently forgetting a budget.
    fs::write(&path, b"{\"guild_ref\":\"guild_ref\"}\n").expect("corrupt the sidecar");
    assert!(matches!(
        EpisodeStore::open(scratch.path(), policy(1_000)).expect_err("corrupt sidecar"),
        EpisodeStoreError::Corrupt
    ));
}

#[test]
fn the_decision_and_the_append_agree_after_an_adjustment() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(10)).expect("open");
    let event = memory("request_1", "memory_1", 5, candidate(1));

    // Before the adjustment every path accepts the write.
    assert_eq!(
        store.decide_write_outcome(&event).reason(),
        AdmissionReason::Accepted
    );
    assert!(store.decide_write(&event).is_ok());

    // The owner lowers the budget below the write's charge: every path
    // refuses it, for the same reason.
    store
        .commit_quota_adjustment(&adjustment("guild_ref", 2, 1024 * 1024, owner()))
        .expect("an owner may adjust the budget");
    assert_eq!(
        store.decide_write_outcome(&event).reason(),
        AdmissionReason::TokenBudget
    );
    assert!(matches!(
        store.decide_write(&event).expect_err("refused on budget"),
        EpisodeStoreError::TokenBudget
    ));
    assert!(matches!(
        rejected(&mut store, &event),
        EpisodeStoreError::TokenBudget
    ));
}

#[test]
fn a_service_may_freeze_a_guild_and_the_host_restores_the_budget() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1_000)).expect("open");
    append(
        &mut store,
        memory("request_1", "memory_1", 10, candidate(1)),
    );
    let (tokens_used, _) = store.guild_usage("guild_ref").expect("recorded usage");
    assert_eq!(tokens_used, 10);

    // A service lowers the guild's budget to the usage the guild has already
    // recorded. The budget is a guild budget, so every writer in the guild
    // is refused from here on.
    store
        .commit_quota_adjustment(&adjustment(
            "guild_ref",
            tokens_used,
            1024 * 1024,
            service(),
        ))
        .expect("a service may lower a budget");
    let state = store
        .quota_state("guild_ref")
        .expect("quota state")
        .expect("a served guild");
    assert_eq!(state.token_budget, tokens_used);
    assert!(matches!(
        rejected(
            &mut store,
            &memory("request_2", "memory_2", 1, candidate(2))
        ),
        EpisodeStoreError::TokenBudget
    ));

    // The host reopens the store with a larger policy budget. The host
    // policy is the outer bound, so the service's downward cap cannot hold
    // the guild below it: the guild's capacity is restored.
    drop(store);
    let reopened = EpisodeStore::open(scratch.path(), policy(9_000)).expect("verified reopen");
    let state = reopened
        .quota_state("guild_ref")
        .expect("quota state")
        .expect("a served guild");
    assert_eq!(state.policy_token_budget, 9_000);
    assert_eq!(
        state.token_budget, 9_000,
        "the host's larger policy restores the guild's budget"
    );
    assert!(
        reopened
            .decide_write(&memory("request_3", "memory_3", 1, candidate(3)))
            .is_ok(),
        "the guild writes again"
    );
}

#[test]
fn a_policy_lowered_below_an_override_takes_effect() {
    let scratch = Scratch::new();
    {
        let mut store = EpisodeStore::open(scratch.path(), policy(1_000)).expect("open");
        store
            .commit_quota_adjustment(&adjustment("guild_ref", 800, 1024 * 1024, owner()))
            .expect("an owner may adjust the budget");
    }

    // The host lowers the policy below the adjustment: the host always
    // holds the outer bound, so the effective budget is the policy's.
    let reopened = EpisodeStore::open(scratch.path(), policy(500)).expect("verified reopen");
    let state = reopened
        .quota_state("guild_ref")
        .expect("quota state")
        .expect("a served guild");
    assert_eq!(state.policy_token_budget, 500);
    assert_eq!(
        state.token_budget, 500,
        "the host's lower policy takes effect"
    );
}

#[test]
fn an_owner_may_clear_an_adjustment_and_a_service_may_not() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1_000)).expect("open");
    store
        .commit_quota_adjustment(&adjustment("guild_ref", 100, 1024 * 1024, owner()))
        .expect("an owner may adjust the budget");
    assert_eq!(
        store
            .quota_state("guild_ref")
            .expect("quota state")
            .expect("a served guild")
            .token_budget,
        100
    );

    // A service may not clear: clearing restores the host policy's budget,
    // which is the same power as raising one.
    assert!(matches!(
        store
            .clear_quota_adjustment("guild_ref", &service())
            .expect_err("a service may not clear an adjustment"),
        EpisodeStoreError::InvalidTransition
    ));

    // The owner may, and the host policy's budget applies again.
    let receipt = store
        .clear_quota_adjustment("guild_ref", &owner())
        .expect("an owner may clear an adjustment");
    assert_eq!(receipt.guild_ref, "guild_ref");
    assert_eq!(receipt.token_budget, 1_000);
    assert_eq!(receipt.storage_budget_bytes, 1024 * 1024);
    assert_eq!(receipt.authority, owner());
    let state = store
        .quota_state("guild_ref")
        .expect("quota state")
        .expect("a served guild");
    assert_eq!(
        state.token_budget, 1_000,
        "the host policy's budget applies again"
    );
    assert_eq!(state.adjusted_by, None);

    // The reset is recorded in the same audit trail as the adjustment it
    // lifts.
    let history = store.quota_history("guild_ref").expect("the quota history");
    assert_eq!(history.len(), 2);
    assert!(!history[0].cleared);
    assert!(history[1].cleared);
    assert_eq!(history[1].authority, owner());

    // A guild the store does not serve has no adjustment to clear.
    assert!(matches!(
        store
            .clear_quota_adjustment("absent_guild", &owner())
            .expect_err("an unserved guild"),
        EpisodeStoreError::StaleBinding
    ));
}

#[test]
fn no_adjustment_may_raise_a_budget_above_the_policy() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1_000)).expect("open");

    // A service may not raise a budget at all.
    assert!(matches!(
        store
            .decide_quota_adjustment(&adjustment("guild_ref", 2_000, 1024 * 1024, service()))
            .expect_err("a service may not raise a budget"),
        EpisodeStoreError::InvalidTransition
    ));

    // Nor may an authority: the host policy is the outer bound, and a host
    // that wants a larger budget changes the policy.
    assert!(matches!(
        store
            .decide_quota_adjustment(&adjustment("guild_ref", 2_000, 1024 * 1024, owner()))
            .expect_err("above the policy"),
        EpisodeStoreError::InvalidTransition
    ));
    assert!(matches!(
        store
            .commit_quota_adjustment(&adjustment("guild_ref", 2_000, 1024 * 1024, owner()))
            .expect_err("above the policy"),
        EpisodeStoreError::InvalidTransition
    ));

    // The storage budget has the same bound.
    assert!(matches!(
        store
            .commit_quota_adjustment(&adjustment("guild_ref", 1_000, 2 * 1024 * 1024, owner()))
            .expect_err("above the policy"),
        EpisodeStoreError::InvalidTransition
    ));

    // Nothing was recorded: the guild still holds the policy's budget.
    let state = store
        .quota_state("guild_ref")
        .expect("quota state")
        .expect("a served guild");
    assert_eq!(state.token_budget, 1_000);
    assert_eq!(state.adjusted_by, None);
}

#[test]
fn a_policy_lowered_below_the_recorded_usage_is_reported_not_refused() {
    let scratch = Scratch::new();
    {
        let mut store = EpisodeStore::open(scratch.path(), policy(1_000)).expect("open");
        append(
            &mut store,
            memory("request_1", "memory_1", 100, candidate(1)),
        );
    }

    // The host lowers the policy below the usage the guild has recorded. The
    // store surfaces the inconsistency instead of refusing to open: refusing
    // would brick a store whose policy was lowered.
    let reopened = EpisodeStore::open(scratch.path(), policy(10)).expect("the store still opens");
    let state = reopened
        .quota_state("guild_ref")
        .expect("quota state")
        .expect("a served guild");
    assert_eq!(state.tokens_used, 100);
    assert_eq!(state.token_budget, 10);
    assert!(
        state.over_budget,
        "the store reports the usage above its effective budget"
    );
}

#[test]
fn the_quota_history_reports_every_decision_in_order() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1_000)).expect("open");
    store
        .commit_quota_adjustment(&adjustment("guild_ref", 500, 1024 * 1024, owner()))
        .expect("the first adjustment");
    let manager = actor("guild_manager", ActorKind::GuildManager);
    store
        .commit_quota_adjustment(&adjustment("guild_ref", 200, 1024 * 1024, manager.clone()))
        .expect("the second adjustment");

    // Both decisions are visible, in the order the store recorded them.
    let history = store.quota_history("guild_ref").expect("the quota history");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].guild_ref, "guild_ref");
    assert_eq!(history[0].token_budget, 500);
    assert_eq!(history[0].storage_budget_bytes, 1024 * 1024);
    assert_eq!(history[0].authority, owner());
    assert!(!history[0].cleared);
    assert_eq!(history[1].token_budget, 200);
    assert_eq!(history[1].authority, manager);
    assert!(history[1].adjusted_at_ms >= history[0].adjusted_at_ms);

    // The other guild has no decisions.
    assert_eq!(
        store
            .quota_history("other_guild")
            .expect("the quota history"),
        Vec::new()
    );
    // A guild the store does not serve has no history at all.
    assert!(matches!(
        store
            .quota_history("absent_guild")
            .expect_err("an unserved guild"),
        EpisodeStoreError::StaleBinding
    ));

    // The audit trail is durable: it survives close and reopen.
    drop(store);
    let reopened = EpisodeStore::open(scratch.path(), policy(1_000)).expect("verified reopen");
    let history = reopened
        .quota_history("guild_ref")
        .expect("the quota history");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].token_budget, 500);
    assert_eq!(history[1].token_budget, 200);
}

#[test]
fn the_quota_sidecar_holds_no_episode_content() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1_000)).expect("open");
    let digest = append(
        &mut store,
        memory("request_1", "memory_1", 1, candidate(0xAB)),
    );
    store
        .commit_quota_adjustment(&adjustment("guild_ref", 500, 1024 * 1024, owner()))
        .expect("an owner may adjust the budget");

    let files = store_bytes(scratch.path());
    let sidecar = files
        .get(&scratch.path().join(QUOTA_FILE))
        .expect("the quota sidecar exists");
    let ledger = files
        .get(&scratch.path().join(LEDGER_FILE))
        .expect("the ledger exists");

    // The sidecar records the guild, the budgets, the authority, and the
    // instant. It holds no episode content at all.
    let text = String::from_utf8_lossy(sidecar);
    assert!(text.contains("guild_ref"));
    assert!(text.contains("guild_owner"));
    assert!(text.contains("500"));
    assert!(text.contains("1000"));

    // The candidate's payload commitment is in the ledger and nowhere else.
    let commitment = [0xAB; 32];
    assert!(
        digest_encodings(&commitment)
            .iter()
            .any(|encoding| holds(ledger, encoding)),
        "the ledger carries the payload commitment"
    );
    for encoding in digest_encodings(&commitment) {
        assert!(
            !holds(sidecar, &encoding),
            "the quota sidecar carries no payload commitment"
        );
    }

    // Nor does it carry the episode digest.
    for encoding in digest_encodings(&digest) {
        assert!(
            !holds(sidecar, &encoding),
            "the quota sidecar carries no episode digest"
        );
    }

    // Nor any of the vocabulary of an episode.
    for forbidden in [
        "payload",
        "candidate",
        "memory",
        "episode",
        "request_1",
        "memory_1",
        "retention",
        "supersedes",
        "forgets",
    ] {
        assert!(
            !text.contains(forbidden),
            "the quota sidecar must not mention {forbidden}: {text}"
        );
    }
}

/// The non-mutating decision and the commit agree on every outcome.
///
/// The clock is the only thing that separates them: the commit records the
/// instant the adjustment was made, so it is the only path that reads a
/// clock, and the decision — which records nothing — reads none at all.
/// Every other refusal is the same refusal in both, with the same label.
///
/// The clock-unavailable case itself is not reachable from here: the store's
/// clock is the system clock, and it is readable on every platform this
/// substrate runs on. The crate's own unit test drives the commit with an
/// unreadable clock and pins that the decision still succeeds while the
/// commit refuses.
#[test]
fn the_decision_and_the_commit_agree_on_every_outcome() {
    let scratch = Scratch::new();
    let mut store = EpisodeStore::open(scratch.path(), policy(1_000)).expect("open");

    // An authorized adjustment: the decision succeeds, and the commit
    // records it.
    let allowed = adjustment("guild_ref", 500, 1024 * 1024, owner());
    assert!(store.decide_quota_adjustment(&allowed).is_ok());
    store
        .commit_quota_adjustment(&allowed)
        .expect("the commit records the adjustment");

    // Every refusal is the same refusal in both.
    let cases = [
        (
            "above the host policy",
            adjustment("guild_ref", 2_000, 1024 * 1024, owner()),
        ),
        (
            "a service may not raise a budget",
            adjustment("guild_ref", 900, 1024 * 1024, service()),
        ),
        (
            "a human subject is not an authority",
            adjustment(
                "guild_ref",
                900,
                1024 * 1024,
                actor("member", ActorKind::HumanSubject),
            ),
        ),
        (
            "a guild the store does not serve",
            adjustment("absent_guild", 10, 10, owner()),
        ),
        (
            "a guild reference that is not a bounded identifier",
            adjustment("Not A Guild", 10, 10, owner()),
        ),
    ];
    for (reason, request) in cases {
        let decision = store.decide_quota_adjustment(&request).expect_err(reason);
        let commit = store.commit_quota_adjustment(&request).expect_err(reason);
        assert_eq!(
            commit.to_string(),
            decision.to_string(),
            "the decision and the commit must agree: {reason}"
        );
    }

    // The refusals left the effective budget where the allowed adjustment
    // put it: nothing was recorded by a refused commit.
    let state = store
        .quota_state("guild_ref")
        .expect("quota state")
        .expect("a served guild");
    assert_eq!(state.token_budget, 500);
    assert_eq!(
        store.quota_history("guild_ref").expect("the history").len(),
        1
    );
}
