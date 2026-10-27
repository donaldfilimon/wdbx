//! Rejected-write and governance-refusal invariance for the v3 episode store.
//!
//! What these tests pin:
//!
//! - every reason `propose_write` refuses a write for is the reason
//!   `decide_write` gives, and the refusal changes no byte under the store
//!   directory (every sidecar included), no receipt, no usage, and no quota
//!   state;
//! - every governance entry point refuses with its real reason, checked
//!   against the source, and the refusal changes no byte and no sidecar
//!   state read;
//! - the idempotent outcomes (`already_live`, `already_revoked`,
//!   `already_requested`) and a garbage-collection plan whose digests are
//!   all ineligible change no byte. The last one is not a refusal:
//!   `apply_gc_plan` returns `Ok` with an empty `reclaimed` list and
//!   `bytes_before == bytes_after`;
//! - a store that refused writes and then accepted one is byte-identical to
//!   a clean store that only accepted the same writes.
//!
//! Two behaviors are pinned as intended rather than as refusals: a forget
//! of a held target is admitted (the tombstone is the explicit erasure
//! request), and a service may lower a quota budget (only raising one is
//! refused for its class).

#[path = "support/rejection_invariance.rs"]
mod support;

use abi_wdbx::v3::episode::{
    ActorKind, EpisodeEvent, EpisodeSource, EpisodeStore, EpisodeWrite, GcPlan, HoldKind,
    HoldRequest, MemoryCandidate, RetentionClass,
};
use support::{
    ADJUSTED_BYTES, ADJUSTED_TOKENS, DISABLED, GUILD, STORAGE_BUDGET, Scratch, TOKEN_BUDGET,
    UNSERVED, actor, adjustment, admin, append, candidate, destruction, fixture, forget, hold,
    memory, operational, owner, policy, redaction, release, revocation, service, session,
    store_bytes, stranger, subject, supersede, write,
};

const QUIET: &str = "episode_quiet";
const STALE: &str = "episode_binding_stale";
const DISABLED_REASON: &str = "episode_learning_disabled";
const REPLAY: &str = "episode_replay";
const INVALID: &str = "episode_input_invalid";
const TRANSITION: &str = "episode_transition_invalid";
const MISMATCH: &str = "episode_commitment_mismatch";
const TOKENS: &str = "episode_token_budget_exhausted";
const STORAGE: &str = "episode_storage_budget_exhausted";

#[test]
fn every_propose_write_refusal_changes_no_byte() {
    let mut fx = fixture();
    let (held, forgotten, tombstone) = (fx.held, fx.forgotten, fx.tombstone);

    let quiet = EpisodeWrite {
        quiet: true,
        ..memory("quiet", candidate(20))
    };
    fx.assert_write_refused("quiet", &quiet, QUIET);

    let stale_policy = EpisodeWrite {
        policy_version: "policy_v0".into(),
        ..memory("stale_policy", candidate(21))
    };
    fx.assert_write_refused("stale policy version", &stale_policy, STALE);
    let stale_revision = EpisodeWrite {
        contract_revision: 3,
        ..memory("stale_revision", candidate(22))
    };
    fx.assert_write_refused("stale contract revision", &stale_revision, STALE);
    let stale_digest = EpisodeWrite {
        contract_digest: [8; 32],
        ..memory("stale_digest", candidate(23))
    };
    fx.assert_write_refused("stale contract digest", &stale_digest, STALE);
    // A proposal, so the refusal is the consent binding's and does not
    // depend on the voice binding running before the event-shape check.
    let stale_consent = EpisodeWrite {
        source_type: EpisodeSource::DiscordVoice,
        consent_epoch: Some(6),
        ..write(
            "stale_consent",
            "op_stale_consent",
            EpisodeEvent::Proposal {
                requested_by: actor("member", ActorKind::HumanSubject),
                proposed_by: service(),
            },
        )
    };
    fx.assert_write_refused("stale consent epoch", &stale_consent, STALE);
    let unserved = EpisodeWrite {
        guild_ref: UNSERVED.into(),
        ..memory("unserved", candidate(24))
    };
    fx.assert_write_refused("unserved guild", &unserved, STALE);

    let disabled = EpisodeWrite {
        guild_ref: DISABLED.into(),
        ..memory("disabled", candidate(25))
    };
    fx.assert_write_refused("learning disabled", &disabled, DISABLED_REASON);

    // The request id of an accepted write, under a fresh operation id.
    let replay = EpisodeWrite {
        operation_id: "op_replay".into(),
        ..memory("held", candidate(26))
    };
    fx.assert_write_refused("replay by request id", &replay, REPLAY);

    let unbounded = memory(&"r".repeat(65), candidate(27));
    fx.assert_write_refused("unbounded request id", &unbounded, INVALID);
    let malformed = memory(
        "malformed",
        MemoryCandidate {
            payload_bytes: 0,
            ..candidate(28)
        },
    );
    fx.assert_write_refused("malformed candidate", &malformed, INVALID);
    fx.assert_write_refused("zero-TTL candidate", &memory("zero", session(29)), INVALID);

    let rows = [
        ("supersede of a held target", supersede(30, held)),
        ("supersede of a forgotten target", supersede(31, forgotten)),
        ("supersede of a tombstone", supersede(32, tombstone)),
        ("forget of a forgotten target", forget(forgotten)),
    ];
    for (index, (row, candidate)) in rows.into_iter().enumerate() {
        let event = memory(&format!("transition_{index}"), candidate);
        fx.assert_write_refused(row, &event, TRANSITION);
    }

    // TTL-bound candidates: admitting any of them would add a retention
    // line, so the refusal must come before the retention binding.
    let mismatch = EpisodeWrite {
        expected_commitment: Some([1; 32]),
        ..memory("mismatch", operational(33))
    };
    fx.assert_write_refused("commitment mismatch", &mismatch, MISMATCH);
    let tokens = EpisodeWrite {
        token_cost: ADJUSTED_TOKENS + 1,
        ..memory("tokens", operational(34))
    };
    fx.assert_write_refused("token budget", &tokens, TOKENS);
    let storage = memory(
        "storage",
        MemoryCandidate {
            payload_bytes: ADJUSTED_BYTES + 1,
            ..operational(35)
        },
    );
    fx.assert_write_refused("storage budget", &storage, STORAGE);
}

#[test]
fn every_governance_refusal_changes_no_byte() {
    let mut fx = fixture();
    let (held, forgotten, ttl_live) = (fx.held, fx.forgotten, fx.ttl_live);
    let second = fx.second_forgotten;
    let unserved = |mut request: HoldRequest| {
        request.guild_ref = UNSERVED.into();
        request
    };

    // commit_hold.
    fx.assert_refused("hold: another authority holds it", TRANSITION, |s| {
        s.commit_hold(&hold(held, HoldKind::Legal, admin()))
    });
    fx.assert_refused("hold: unserved guild", STALE, |s| {
        s.commit_hold(&unserved(hold(held, HoldKind::Investigation, owner())))
    });
    for (row, who) in [
        ("hold: unauthorized principal", stranger()),
        ("hold: service class", service()),
        ("hold: human-subject class", subject()),
    ] {
        fx.assert_refused(row, TRANSITION, |s| {
            s.commit_hold(&hold(held, HoldKind::Investigation, who))
        });
    }

    // release_hold.
    fx.assert_refused("release: never placed", TRANSITION, |s| {
        s.release_hold(&release(held, HoldKind::Investigation, owner()))
    });
    fx.assert_refused("release: already released", TRANSITION, |s| {
        s.release_hold(&release(ttl_live, HoldKind::UserRequest, owner()))
    });
    fx.assert_refused("release: below the placing authority", TRANSITION, |s| {
        s.release_hold(&release(held, HoldKind::Legal, admin()))
    });
    fx.assert_refused("release: unserved guild", STALE, |s| {
        let mut request = release(held, HoldKind::Legal, owner());
        request.guild_ref = UNSERVED.into();
        s.release_hold(&request)
    });
    for (row, who) in [
        ("release: unauthorized principal", stranger()),
        ("release: service class", service()),
        ("release: human-subject class", subject()),
    ] {
        fx.assert_refused(row, TRANSITION, |s| {
            s.release_hold(&release(held, HoldKind::Legal, who))
        });
    }

    // commit_redaction.
    fx.assert_refused("redaction: live target", TRANSITION, |s| {
        s.commit_redaction(&redaction(ttl_live, owner()))
    });
    fx.assert_refused("redaction: unserved guild", STALE, |s| {
        let mut request = redaction(second, owner());
        request.guild_ref = UNSERVED.into();
        s.commit_redaction(&request)
    });
    for (row, who) in [
        ("redaction: unauthorized principal", stranger()),
        ("redaction: service class", service()),
        ("redaction: human-subject class", subject()),
    ] {
        fx.assert_refused(row, TRANSITION, |s| {
            s.commit_redaction(&redaction(second, who))
        });
    }

    // commit_object_key_destruction.
    fx.assert_refused("object key: live target", TRANSITION, |s| {
        s.commit_object_key_destruction(&destruction(ttl_live, "key_e", owner()))
    });
    fx.assert_refused("object key: key bound elsewhere", TRANSITION, |s| {
        s.commit_object_key_destruction(&destruction(second, "key_b", owner()))
    });
    fx.assert_refused("object key: unserved guild", STALE, |s| {
        let mut request = destruction(second, "key_e", owner());
        request.guild_ref = UNSERVED.into();
        s.commit_object_key_destruction(&request)
    });
    for (row, who) in [
        ("object key: unauthorized principal", stranger()),
        ("object key: service class", service()),
        ("object key: human-subject class", subject()),
    ] {
        fx.assert_refused(row, TRANSITION, |s| {
            s.commit_object_key_destruction(&destruction(second, "key_e", who))
        });
    }

    // commit_quota_adjustment.
    fx.assert_refused("quota: above the host policy", TRANSITION, |s| {
        s.commit_quota_adjustment(&adjustment(TOKEN_BUDGET + 1, ADJUSTED_BYTES, owner()))
    });
    fx.assert_refused("quota: lowered below usage", INVALID, |s| {
        s.commit_quota_adjustment(&adjustment(0, ADJUSTED_BYTES, owner()))
    });
    fx.assert_refused("quota: unserved guild", STALE, |s| {
        let mut request = adjustment(ADJUSTED_TOKENS, ADJUSTED_BYTES, owner());
        request.guild_ref = UNSERVED.into();
        s.commit_quota_adjustment(&request)
    });
    fx.assert_refused("quota: unauthorized principal", TRANSITION, |s| {
        s.commit_quota_adjustment(&adjustment(ADJUSTED_TOKENS, ADJUSTED_BYTES, stranger()))
    });
    // A service may lower a budget; the class rule refuses it a raise.
    fx.assert_refused("quota: service raises", TRANSITION, |s| {
        s.commit_quota_adjustment(&adjustment(TOKEN_BUDGET, ADJUSTED_BYTES, service()))
    });
    fx.assert_refused("quota: human-subject class", TRANSITION, |s| {
        s.commit_quota_adjustment(&adjustment(ADJUSTED_TOKENS, ADJUSTED_BYTES, subject()))
    });

    // clear_quota_adjustment.
    fx.assert_refused("clear: unserved guild", STALE, |s| {
        s.clear_quota_adjustment(UNSERVED, &owner())
    });
    for (row, who) in [
        ("clear: unauthorized principal", stranger()),
        ("clear: service class", service()),
        ("clear: human-subject class", subject()),
    ] {
        fx.assert_refused(row, TRANSITION, |s| s.clear_quota_adjustment(GUILD, &who));
    }

    // apply_gc_plan.
    let plan = |guild_ref: &str| GcPlan {
        guild_ref: guild_ref.into(),
        now_ms: u64::MAX,
        reclaimable: vec![forgotten],
        retained_tombstones: Vec::new(),
    };
    fx.assert_refused("gc: unserved guild", STALE, |s| {
        s.apply_gc_plan(&plan(UNSERVED))
    });
    fx.assert_refused("gc: bad guild_ref", INVALID, |s| {
        s.apply_gc_plan(&plan("Bad Guild"))
    });
}

#[test]
fn idempotent_no_write_outcomes_change_no_byte() {
    let mut fx = fixture();
    let (held, forgotten, revoked) = (fx.held, fx.forgotten, fx.revoked);

    let before = fx.snapshot();
    let receipt = fx
        .store
        .commit_hold(&hold(held, HoldKind::Legal, owner()))
        .expect("the same hold again");
    assert!(receipt.already_live);
    assert_eq!(fx.snapshot(), before, "already_live changed state");

    let receipt = fx
        .store
        .commit_revocation(&revocation(revoked, owner()))
        .expect("the same revocation again");
    assert!(receipt.already_revoked);
    assert_eq!(fx.snapshot(), before, "already_revoked changed state");

    let receipt = fx
        .store
        .commit_redaction(&redaction(forgotten, owner()))
        .expect("the same redaction again");
    assert!(receipt.requested.is_empty());
    assert_eq!(receipt.already_requested.len(), 1);
    assert_eq!(fx.snapshot(), before, "redaction already_requested");

    let receipt = fx
        .store
        .commit_object_key_destruction(&destruction(forgotten, "key_b", owner()))
        .expect("the same destruction again");
    assert!(receipt.already_requested);
    assert_eq!(fx.snapshot(), before, "object key already_requested");

    // Not a refusal: a plan naming only ineligible digests — a live
    // candidate and a forgotten durable one with no deadline — is applied
    // as `Ok` and reclaims nothing.
    let plan = GcPlan {
        guild_ref: GUILD.into(),
        now_ms: u64::MAX,
        reclaimable: vec![fx.ttl_live, forgotten],
        retained_tombstones: vec![fx.tombstone],
    };
    let report = fx.store.apply_gc_plan(&plan).expect("an ineligible plan");
    assert!(report.reclaimed.is_empty());
    assert_eq!(report.skipped, vec![fx.ttl_live, forgotten]);
    assert_eq!(report.bytes_before, report.bytes_after);
    assert_eq!(report.records_before, report.records_after);
    assert_eq!(fx.snapshot(), before, "an ineligible plan changed state");
}

/// The accepted writes both stores replay. They are durable or tombstones,
/// so none binds a retention deadline and no clock enters the bytes.
fn prior_writes() -> Vec<EpisodeWrite> {
    let first = memory("prior_1", candidate(40));
    let second = memory("prior_2", candidate(41));
    vec![first, second]
}

fn replay_prior(store: &mut EpisodeStore) -> [u8; 32] {
    let digests: Vec<[u8; 32]> = prior_writes()
        .into_iter()
        .map(|event| append(store, event))
        .collect();
    append(store, memory("prior_forget", forget(digests[1])));
    digests[1]
}

#[test]
fn a_refused_write_then_an_accepted_write_matches_a_clean_store() {
    let refused_root = Scratch::new();
    let clean_root = Scratch::new();
    let mut refusing = EpisodeStore::open(refused_root.path(), policy()).expect("open");
    let mut clean = EpisodeStore::open(clean_root.path(), policy()).expect("open");
    let forgotten = replay_prior(&mut refusing);
    assert_eq!(replay_prior(&mut clean), forgotten);

    let refusals = [
        EpisodeWrite {
            quiet: true,
            ..memory("next", candidate(50))
        },
        memory("prior_1", candidate(51)),
        memory("zero", session(52)),
        memory("superseded", supersede(53, forgotten)),
        EpisodeWrite {
            expected_commitment: Some([1; 32]),
            ..memory("mismatch", operational(54))
        },
        EpisodeWrite {
            token_cost: TOKEN_BUDGET,
            ..memory("tokens", operational(55))
        },
        memory(
            "storage",
            MemoryCandidate {
                payload_bytes: STORAGE_BUDGET,
                retention: RetentionClass::Operational,
                ..candidate(56)
            },
        ),
    ];
    for event in &refusals {
        assert!(refusing.propose_write(event).is_err(), "refused");
    }

    let accepted = memory("next", candidate(57));
    let digest = append(&mut refusing, accepted.clone());
    assert_eq!(append(&mut clean, accepted), digest);

    assert_eq!(
        store_bytes(refused_root.path()),
        store_bytes(clean_root.path()),
        "the refusals left a trace in the store directory"
    );
    assert_eq!(
        refusing.retrieve(GUILD, 2_048).expect("retrieve"),
        clean.retrieve(GUILD, 2_048).expect("retrieve")
    );
    assert_eq!(refusing.guild_usage(GUILD), clean.guild_usage(GUILD));
}
