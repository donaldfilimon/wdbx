use super::*;

#[test]
fn inexact_compensation_cannot_report_success_before_or_after_reopen() {
    let scratch = Scratch::new();
    let store_policy = policy(true, 100, 100_000);
    let mut store = EpisodeStore::open(scratch.path(), store_policy.clone()).expect("open");
    append(&mut store, proposal("request_1", "operation_1"));
    append(
        &mut store,
        write(
            "request_2",
            "operation_1",
            EpisodeEvent::Approval {
                approved_by: actor("admin_ref", ActorKind::GuildAdministrator),
            },
        ),
    );
    append(
        &mut store,
        write(
            "request_3",
            "operation_1",
            EpisodeEvent::Execution {
                executed_by: actor("bot_service", ActorKind::Service),
                voice: None,
            },
        ),
    );
    append(
        &mut store,
        write(
            "request_4",
            "operation_1",
            EpisodeEvent::Compensation {
                compensated_by: actor("bot_service", ActorKind::Service),
                exact_restore_observed: false,
            },
        ),
    );
    let receipts = store.retrieve("guild_ref", 10).expect("receipts");
    let usage = store.guild_usage("guild_ref");
    let bytes = store_bytes(scratch.path());
    assert_eq!(receipts.len(), 4);

    for reopened in [false, true] {
        if reopened {
            drop(store);
            store = EpisodeStore::open(scratch.path(), store_policy.clone())
                .expect("inexact compensation replays");
        }
        for status in [TerminalStatus::Compensated, TerminalStatus::Completed] {
            let terminal = write(
                "request_5",
                "operation_1",
                EpisodeEvent::Terminal {
                    status,
                    reason: TerminalReason::Completed,
                },
            );
            assert!(matches!(
                store.preview_commitment(&terminal),
                Err(EpisodeStoreError::InvalidTransition)
            ));
            assert!(matches!(
                store.propose_write(&terminal),
                Err(EpisodeStoreError::InvalidTransition)
            ));
            assert_eq!(store.retrieve("guild_ref", 10).unwrap(), receipts);
            assert_eq!(store.guild_usage("guild_ref"), usage);
            assert_eq!(store_bytes(scratch.path()), bytes);
        }
    }

    // Rejection must not consume the request identifier or terminate the operation.
    append(
        &mut store,
        write(
            "request_5",
            "operation_1",
            EpisodeEvent::Terminal {
                status: TerminalStatus::Failed,
                reason: TerminalReason::Completed,
            },
        ),
    );
    let receipts = store.retrieve("guild_ref", 10).unwrap();
    drop(store);
    let reopened =
        EpisodeStore::open(scratch.path(), store_policy).expect("failed terminal replays");
    assert_eq!(reopened.retrieve("guild_ref", 10).unwrap(), receipts);
    assert_eq!(receipts.len(), 5);
}
