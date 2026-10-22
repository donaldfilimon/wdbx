//! Per-class retention windows for the v3 episode store.
//!
//! A host policy states one fallback time-to-live for the expiring retention
//! classes, and may state a window per class. These tests pin the windows
//! themselves:
//!
//! - a per-class window binds the right deadline: a `session` candidate and
//!   an `operational` candidate under the same policy can have different
//!   windows;
//! - a policy JSON that omits every window field still parses and still
//!   means the previous behaviour;
//! - a per-class zero window is refused by the write gate, exactly like the
//!   single zero window;
//! - a `durable` candidate never expires, whatever the windows say.

#[path = "support/quota_windows.rs"]
mod support;

use abi_wdbx::v3::episode::{
    EpisodeStore, EpisodeStoreError, EpisodeWrite, GuildEpisodePolicy, RetentionClass,
    RetentionWindows, StorePolicy,
};
use support::{
    Scratch, append, candidate, memory, operational, policy_with, rejected, session, wall_clock_ms,
};

#[test]
fn a_per_class_window_binds_the_right_deadline() {
    let scratch = Scratch::new();
    let windows = RetentionWindows {
        session_ttl_ms: Some(1_000),
        operational_ttl_ms: Some(120_000),
        durable_ttl_ms: None,
    };
    let mut store = EpisodeStore::open(
        scratch.path(),
        policy_with(1_000, Some(60_000), Some(windows)),
    )
    .expect("open");

    let before = wall_clock_ms();
    let session_digest = append(&mut store, memory("request_1", "memory_1", 1, session(1)));
    let operational_digest = append(
        &mut store,
        memory("request_2", "memory_2", 1, operational(2)),
    );
    let after = wall_clock_ms();

    let session_state = store
        .retention_state("guild_ref", &session_digest)
        .expect("retention state")
        .expect("admitted candidate");
    assert_eq!(session_state.class, RetentionClass::Session);
    let session_deadline = session_state
        .expires_at
        .expect("a session candidate has a deadline");
    assert!(
        session_deadline >= before.saturating_add(1_000)
            && session_deadline <= after.saturating_add(1_000),
        "the session deadline is the arrival plus the session window: {session_deadline}"
    );

    let operational_state = store
        .retention_state("guild_ref", &operational_digest)
        .expect("retention state")
        .expect("admitted candidate");
    assert_eq!(operational_state.class, RetentionClass::Operational);
    let operational_deadline = operational_state
        .expires_at
        .expect("an operational candidate has a deadline");
    assert!(
        operational_deadline >= before.saturating_add(120_000)
            && operational_deadline <= after.saturating_add(120_000),
        "the operational deadline is the arrival plus the operational window: {operational_deadline}"
    );

    assert!(
        session_deadline < operational_deadline,
        "the two classes have different windows under one policy"
    );
}

#[test]
fn a_policy_without_the_new_fields_still_parses_and_means_the_previous_behaviour() {
    // A host policy that omits both new fields still parses.
    let policy_json = r#"{
        "learning_enabled": true,
        "policy_version": "policy_v1",
        "token_budget": 1000,
        "storage_budget_bytes": 1048576,
        "current_consent_epoch": null
    }"#;
    let guild: GuildEpisodePolicy = serde_json::from_str(policy_json).expect("policy parses");
    assert_eq!(guild.retention_ttl_ms, None);
    assert_eq!(guild.retention_windows, None);
    assert_eq!(guild.retention_ttl_for(RetentionClass::Session), None);
    assert_eq!(guild.retention_ttl_for(RetentionClass::Operational), None);
    assert_eq!(guild.retention_ttl_for(RetentionClass::Durable), None);

    // The round trip is explicit: the parsed policy serializes back to a
    // policy that parses to the same value.
    let round_trip: GuildEpisodePolicy =
        serde_json::from_str(&serde_json::to_string(&guild).expect("policy serializes"))
            .expect("the round trip parses");
    assert_eq!(round_trip, guild);

    // A policy that names the single window still parses, and the window is
    // the fallback for every expiring class.
    let with_ttl = r#"{
        "learning_enabled": true,
        "policy_version": "policy_v1",
        "token_budget": 1000,
        "storage_budget_bytes": 1048576,
        "current_consent_epoch": null,
        "retention_ttl_ms": 60000
    }"#;
    let guild: GuildEpisodePolicy = serde_json::from_str(with_ttl).expect("policy parses");
    assert_eq!(guild.retention_ttl_ms, Some(60_000));
    assert_eq!(guild.retention_windows, None);
    assert_eq!(
        guild.retention_ttl_for(RetentionClass::Session),
        Some(60_000)
    );
    assert_eq!(
        guild.retention_ttl_for(RetentionClass::Operational),
        Some(60_000)
    );

    // A policy that names the per-class windows parses too.
    let with_windows = r#"{
        "learning_enabled": true,
        "policy_version": "policy_v1",
        "token_budget": 1000,
        "storage_budget_bytes": 1048576,
        "current_consent_epoch": null,
        "retention_ttl_ms": 60000,
        "retention_windows": { "session_ttl_ms": 1000, "operational_ttl_ms": 120000 }
    }"#;
    let guild: GuildEpisodePolicy = serde_json::from_str(with_windows).expect("policy parses");
    let windows = guild.retention_windows.as_ref().expect("the windows");
    assert_eq!(windows.session_ttl_ms, Some(1_000));
    assert_eq!(windows.operational_ttl_ms, Some(120_000));
    assert_eq!(windows.durable_ttl_ms, None);
    // An explicit per-class window wins over the single window; the single
    // window remains the fallback.
    assert_eq!(
        guild.retention_ttl_for(RetentionClass::Session),
        Some(1_000)
    );
    assert_eq!(
        guild.retention_ttl_for(RetentionClass::Operational),
        Some(120_000)
    );

    // The whole store policy parses the same way: a policy document that
    // omits the new field, and one that names it.
    let store_json = r#"{
        "contract_revision": 2,
        "contract_digest": [1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
        "guilds": {
            "guild_ref": {
                "learning_enabled": true,
                "policy_version": "policy_v1",
                "token_budget": 1000,
                "storage_budget_bytes": 1048576,
                "current_consent_epoch": null
            }
        }
    }"#;
    let store_policy: StorePolicy = serde_json::from_str(store_json).expect("store policy parses");
    assert_eq!(
        store_policy.guilds["guild_ref"].retention_windows, None,
        "an omitted window means the previous behaviour"
    );
    assert_eq!(store_policy.guilds["guild_ref"].retention_ttl_ms, None);

    let store_json_with_windows = r#"{
        "contract_revision": 2,
        "contract_digest": [1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
        "guilds": {
            "guild_ref": {
                "learning_enabled": true,
                "policy_version": "policy_v1",
                "token_budget": 1000,
                "storage_budget_bytes": 1048576,
                "current_consent_epoch": null,
                "retention_windows": { "session_ttl_ms": 1000 }
            }
        }
    }"#;
    let store_policy: StorePolicy =
        serde_json::from_str(store_json_with_windows).expect("store policy parses");
    assert_eq!(
        store_policy.guilds["guild_ref"]
            .retention_windows
            .as_ref()
            .expect("the windows")
            .session_ttl_ms,
        Some(1_000)
    );

    // The behaviour is the previous one: a policy with only the single
    // window gives both expiring classes the same window.
    let scratch = Scratch::new();
    let mut store =
        EpisodeStore::open(scratch.path(), policy_with(1_000, Some(60_000), None)).expect("open");
    let before = wall_clock_ms();
    let session_digest = append(&mut store, memory("request_1", "memory_1", 1, session(1)));
    let operational_digest = append(
        &mut store,
        memory("request_2", "memory_2", 1, operational(2)),
    );
    let after = wall_clock_ms();
    let session_deadline = store
        .retention_state("guild_ref", &session_digest)
        .expect("retention state")
        .expect("admitted candidate")
        .expires_at
        .expect("a deadline");
    let operational_deadline = store
        .retention_state("guild_ref", &operational_digest)
        .expect("retention state")
        .expect("admitted candidate")
        .expires_at
        .expect("a deadline");
    assert!(
        session_deadline >= before.saturating_add(60_000)
            && session_deadline <= after.saturating_add(60_000),
        "the single window still applies to a session candidate"
    );
    assert!(
        operational_deadline >= before.saturating_add(60_000)
            && operational_deadline <= after.saturating_add(60_000),
        "the single window still applies to an operational candidate"
    );
}

#[test]
fn a_per_class_zero_window_is_refused_by_the_write_gate() {
    let scratch = Scratch::new();
    let windows = RetentionWindows {
        session_ttl_ms: Some(0),
        operational_ttl_ms: Some(60_000),
        durable_ttl_ms: None,
    };
    let mut store = EpisodeStore::open(
        scratch.path(),
        policy_with(1_000, Some(60_000), Some(windows)),
    )
    .expect("open");

    // A zero window is the only way a deadline can be closed at admission,
    // so the gate refuses the candidate exactly as it refuses one under a
    // zero single window.
    assert!(matches!(
        rejected(&mut store, &memory("request_1", "memory_1", 1, session(1))),
        EpisodeStoreError::InvalidInput
    ));
    assert!(matches!(
        store
            .decide_write(&memory("request_1b", "memory_1b", 1, session(1)))
            .expect_err("the decision refuses the expired candidate"),
        EpisodeStoreError::InvalidInput
    ));
    // The class with a positive window is admitted.
    let digest = append(
        &mut store,
        memory("request_2", "memory_2", 1, operational(2)),
    );
    assert!(
        store
            .retention_state("guild_ref", &digest)
            .expect("retention state")
            .expect("admitted candidate")
            .expires_at
            .is_some()
    );
}

#[test]
fn a_durable_candidate_never_expires_whatever_the_windows_say() {
    let scratch = Scratch::new();
    let windows = RetentionWindows {
        session_ttl_ms: Some(1_000),
        operational_ttl_ms: Some(1_000),
        durable_ttl_ms: Some(1),
    };
    let mut store = EpisodeStore::open(
        scratch.path(),
        policy_with(1_000, Some(1_000), Some(windows)),
    )
    .expect("open");
    let digest = append(&mut store, memory("request_1", "memory_1", 1, candidate(1)));

    let state = store
        .retention_state("guild_ref", &digest)
        .expect("retention state")
        .expect("admitted candidate");
    assert_eq!(state.class, RetentionClass::Durable);
    assert_eq!(state.expires_at, None, "a durable candidate never expires");
    assert!(!state.expired);
    assert_eq!(
        store
            .due_expirations("guild_ref", u64::MAX)
            .expect("due expirations"),
        Vec::<[u8; 32]>::new()
    );
    assert_eq!(
        store
            .expiry_writes("guild_ref", u64::MAX, "request_exp", "operation_exp")
            .expect("expiry writes"),
        Vec::<EpisodeWrite>::new()
    );
}
