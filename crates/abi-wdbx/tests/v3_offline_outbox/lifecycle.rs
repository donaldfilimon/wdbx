//! The outbox's lifecycle: enqueue, reopen, drain, refuse, resolve.

use super::*;
use abi_wdbx::v3::episode::EpisodeStore;
use abi_wdbx::v3::offline::{
    OfflineOutbox, OperationKind, OperationReason, OperationStatus, OutboxError,
    ReauthorizeOutcome, RefusalReason, Resolution,
};

/// A queued operation survives a close and a reopen, and is pending.
#[test]
fn queued_operation_survives_a_reopen_and_is_pending() {
    let scratch = Scratch::new("reopen");
    let directory = scratch.child("outbox");
    {
        let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
        let receipt = outbox
            .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
            .expect("enqueue");
        assert_eq!(receipt.status, OperationStatus::Pending);
        assert_eq!(receipt.sequence, 1);
    }
    let outbox = OfflineOutbox::open(&directory).expect("reopen outbox");
    let status = outbox.status();
    assert_eq!(status.pending, 1);
    assert_eq!(status.total(), 1);
    let view = outbox.operation(&op_id("op_1")).expect("operation view");
    assert_eq!(view.kind, OperationKind::Write);
    assert_eq!(view.status, OperationStatus::Pending);
    assert_eq!(view.reason, None);
    assert_eq!(view.guild_ref, SCOPE);
}

/// A drain that authorizes an operation acknowledges it, and never resends
/// it.
#[test]
fn draining_an_authorized_operation_acknowledges_it_and_never_resends_it() {
    let scratch = Scratch::new("acknowledge");
    let directory = scratch.child("outbox");
    let store_directory = scratch.child("episodes");
    let policy = policy();
    let store = EpisodeStore::open(&store_directory, policy.clone()).expect("canonical store");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
        .expect("enqueue");

    let mut sink = RecordingSink::accepting();
    let report = outbox.drain(&policy, &store, &mut sink).expect("drain");
    assert_eq!(report.outcomes.len(), 1);
    assert_eq!(
        report.outcomes[0].reauthorization,
        ReauthorizeOutcome::Authorized
    );
    assert!(report.outcomes[0].submitted);
    assert_eq!(report.submitted, 1);
    assert_eq!(report.accepted, 1);
    assert_eq!(sink.submissions.len(), 1);
    assert_eq!(outbox.status().acknowledged, 1);
    assert_eq!(outbox.status().pending, 0);

    // The second drain has nothing to send: an acknowledged operation is
    // terminal and is never resent.
    let mut again = RecordingSink::accepting();
    let report = outbox.drain(&policy, &store, &mut again).expect("drain");
    assert_eq!(report.submitted, 0);
    assert_eq!(again.submissions.len(), 0);
    assert_eq!(outbox.status().acknowledged, 1);
}

/// A refused operation is preserved with its reason, never dropped.
#[test]
fn a_refused_operation_is_preserved_with_its_reason() {
    let scratch = Scratch::new("refused");
    let directory = scratch.child("outbox");
    let store_directory = scratch.child("episodes");
    let current = learning_disabled_policy();
    let store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
        .expect("enqueue");

    let mut sink = RecordingSink::accepting();
    let report = outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(
        report.outcomes[0].reauthorization,
        ReauthorizeOutcome::Refused(RefusalReason::LearningDisabled)
    );
    assert!(!report.outcomes[0].submitted);
    assert_eq!(report.submitted, 0);
    assert_eq!(sink.submissions.len(), 0);

    // The work is preserved: the operation is still there, with its reason.
    let status = outbox.status();
    assert_eq!(status.rejected, 1);
    assert_eq!(status.pending, 0);
    assert_eq!(status.total(), 1);
    let view = outbox.operation(&op_id("op_1")).expect("operation view");
    assert_eq!(view.status, OperationStatus::Rejected);
    assert_eq!(
        view.reason,
        Some(OperationReason::Refused(RefusalReason::LearningDisabled))
    );
}

/// An authorized caller may requeue a refused operation, and a service
/// identity may not resolve one at all.
#[test]
fn only_a_governance_authority_may_resolve_an_operation() {
    let scratch = Scratch::new("resolve");
    let directory = scratch.child("outbox");
    let store_directory = scratch.child("episodes");
    let current = learning_disabled_policy();
    let store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
        .expect("enqueue");
    let mut sink = RecordingSink::accepting();
    outbox.drain(&current, &store, &mut sink).expect("drain");

    let service = ActorRef {
        principal_id: "abbey_service".into(),
        kind: ActorKind::Service,
    };
    assert!(matches!(
        outbox.resolve(&op_id("op_1"), Resolution::Requeue, &service),
        Err(OutboxError::AuthorityDenied)
    ));
    assert_eq!(outbox.status().rejected, 1);

    let owner = ActorRef {
        principal_id: "guild_owner".into(),
        kind: ActorKind::GuildOwner,
    };
    let receipt = outbox
        .resolve(&op_id("op_1"), Resolution::Requeue, &owner)
        .expect("requeue");
    assert_eq!(receipt.status, OperationStatus::Pending);
    assert_eq!(outbox.status().pending, 1);
}

/// An explicit, authorized discard is the only way an operation leaves the
/// outbox.
#[test]
fn only_an_explicit_authorized_discard_removes_an_operation() {
    let scratch = Scratch::new("discard");
    let directory = scratch.child("outbox");
    let store_directory = scratch.child("episodes");
    let current = learning_disabled_policy();
    let store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
        .expect("enqueue");
    let mut sink = RecordingSink::accepting();
    outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(outbox.status().rejected, 1);

    let owner = ActorRef {
        principal_id: "guild_owner".into(),
        kind: ActorKind::GuildOwner,
    };
    outbox
        .resolve(&op_id("op_1"), Resolution::Discard, &owner)
        .expect("discard");
    assert_eq!(outbox.status().total(), 0);
    assert!(outbox.operation(&op_id("op_1")).is_none());
}

/// An acknowledged operation is removed by an authorized caller, and the
/// outbox accepts new work again.
///
/// An acknowledged operation is the success case, and it is the one that
/// accumulates fastest. Without a status-independent removal it would have
/// no way to leave the outbox at all, and a long-lived client would wedge
/// its queue at the bound with no way to make room.
#[test]
fn an_authorized_caller_removes_an_acknowledged_operation() {
    let scratch = Scratch::new("remove");
    let directory = scratch.child("outbox");
    let store_directory = scratch.child("episodes");
    let current = policy();
    let store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    for index in 0..3 {
        let identifier = format!("op_{index}");
        let seed = u8::try_from(index).expect("a seed") + 1;
        outbox
            .enqueue(write_operation(
                &identifier,
                write(&format!("req_{index}"), &identifier, seed),
            ))
            .expect("enqueue");
    }
    let mut sink = RecordingSink::accepting();
    outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(outbox.status().acknowledged, 3);
    assert_eq!(outbox.status().total(), 3);

    // A service identity may not remove anything: it could bury its own
    // acknowledgement or its own refusal.
    let service = ActorRef {
        principal_id: "abbey_service".into(),
        kind: ActorKind::Service,
    };
    assert!(matches!(
        outbox.resolve(&op_id("op_0"), Resolution::Remove, &service),
        Err(OutboxError::AuthorityDenied)
    ));
    assert_eq!(outbox.status().total(), 3);

    // The refusal path still refuses an acknowledged operation: only the
    // status-independent removal takes it.
    let owner = ActorRef {
        principal_id: "guild_owner".into(),
        kind: ActorKind::GuildOwner,
    };
    assert!(matches!(
        outbox.resolve(&op_id("op_0"), Resolution::Discard, &owner),
        Err(OutboxError::InvalidState)
    ));
    assert_eq!(outbox.status().total(), 3);

    // The removal is explicit and authorized, and the receipt names the
    // status the operation held when it left.
    let receipt = outbox
        .resolve(&op_id("op_0"), Resolution::Remove, &owner)
        .expect("remove");
    assert_eq!(receipt.status, OperationStatus::Acknowledged);
    assert_eq!(outbox.status().total(), 2);
    assert_eq!(outbox.status().acknowledged, 2);
    assert!(outbox.operation(&op_id("op_0")).is_none());

    // The removal is durable: the operation does not come back.
    drop(outbox);
    let outbox = OfflineOutbox::open(&directory).expect("reopen outbox");
    assert_eq!(outbox.status().total(), 2);
    assert!(outbox.operation(&op_id("op_0")).is_none());
}

/// The removal is status-independent, so a queued operation leaves too.
#[test]
fn an_authorized_caller_removes_a_queued_operation() {
    let scratch = Scratch::new("remove-queued");
    let directory = scratch.child("outbox");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
        .expect("enqueue");
    assert_eq!(outbox.status().pending, 1);

    let owner = ActorRef {
        principal_id: "guild_owner".into(),
        kind: ActorKind::GuildOwner,
    };
    let receipt = outbox
        .resolve(&op_id("op_1"), Resolution::Remove, &owner)
        .expect("remove");
    assert_eq!(receipt.status, OperationStatus::Pending);
    assert_eq!(outbox.status().total(), 0);

    // The empty outbox reopens as an empty outbox.
    drop(outbox);
    let outbox = OfflineOutbox::open(&directory).expect("reopen outbox");
    assert_eq!(outbox.status().total(), 0);
    assert!(outbox.operation(&op_id("op_1")).is_none());
}

/// A moved consent epoch yields `NeedsConsent`, which is distinct from a
/// policy refusal.
#[test]
fn a_moved_consent_epoch_yields_needs_consent() {
    let scratch = Scratch::new("consent");
    let directory = scratch.child("outbox");
    let store_directory = scratch.child("episodes");
    let mut binding = binding();
    binding.consent_epoch = Some(7);
    let mut proposal = voice_write("req_1", "op_1", 7);
    proposal.consent_epoch = Some(7);

    let current = moved_consent_policy();
    let store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(NewOperation {
            id: op_id("op_1"),
            depends_on: None,
            binding,
            payload: OperationPayload::Write(Box::new(proposal)),
        })
        .expect("enqueue");

    let mut sink = RecordingSink::accepting();
    let report = outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(
        report.outcomes[0].reauthorization,
        ReauthorizeOutcome::NeedsConsent
    );
    assert!(!report.outcomes[0].submitted);
    assert_eq!(sink.submissions.len(), 0);
    let view = outbox.operation(&op_id("op_1")).expect("operation view");
    assert_eq!(view.status, OperationStatus::Rejected);
    assert_eq!(view.reason, Some(OperationReason::ConsentEpochMoved));
    assert_eq!(outbox.status().rejected, 1);
}

/// A contract revision that is no longer current yields `SchemaSkew`, which
/// is distinct from a policy refusal and from `NeedsConsent`.
#[test]
fn a_contract_revision_that_moved_yields_schema_skew() {
    let scratch = Scratch::new("skew");
    let directory = scratch.child("outbox");
    let store_directory = scratch.child("episodes");
    // The operation was written under revision 2; the server now serves 3.
    let current = skewed_policy();
    let store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
        .expect("enqueue");

    let mut sink = RecordingSink::accepting();
    let report = outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(
        report.outcomes[0].reauthorization,
        ReauthorizeOutcome::SchemaSkew
    );
    assert!(!report.outcomes[0].submitted);
    assert_eq!(sink.submissions.len(), 0);
    let view = outbox.operation(&op_id("op_1")).expect("operation view");
    assert_eq!(view.status, OperationStatus::Rejected);
    assert_eq!(view.reason, Some(OperationReason::SchemaSkew));
}

/// A delete proposal is reauthorized by the same gate, and it is only
/// accepted while its target is live.
#[test]
fn a_delete_proposal_is_reauthorized_by_the_same_gate() {
    let scratch = Scratch::new("delete");
    let directory = scratch.child("outbox");
    let store_directory = scratch.child("episodes");
    let current = policy();
    let mut store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let receipt = store
        .propose_write(&write("req_live", "op_live", 7))
        .expect("admit the record");
    let target = receipt.episode_digest;

    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(delete_operation(
            "op_forget",
            forget("req_1", "op_forget", target),
        ))
        .expect("enqueue");

    let mut sink = RecordingSink::accepting();
    let report = outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(
        report.outcomes[0].reauthorization,
        ReauthorizeOutcome::Authorized
    );
    assert_eq!(report.submitted, 1);
    assert_eq!(outbox.status().acknowledged, 1);

    // A delete of a digest the ledger never admitted is refused by the
    // canonical gate, and the operation is preserved.
    let mut outbox = OfflineOutbox::open(scratch.child("outbox_2")).expect("open outbox");
    outbox
        .enqueue(delete_operation(
            "op_forget",
            forget("req_2", "op_forget", [3; 32]),
        ))
        .expect("enqueue");
    let mut sink = RecordingSink::accepting();
    let report = outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(
        report.outcomes[0].reauthorization,
        ReauthorizeOutcome::Refused(RefusalReason::InvalidTransition)
    );
    assert_eq!(report.submitted, 0);
    assert_eq!(outbox.status().rejected, 1);
}

/// A dependent operation waits for its dependency, and the unsent work is
/// preserved.
#[test]
fn a_dependent_operation_waits_for_its_dependency() {
    let scratch = Scratch::new("dependency");
    let directory = scratch.child("outbox");
    let store_directory = scratch.child("episodes");
    let current = policy();
    let store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
        .expect("enqueue");
    let mut dependent = write_operation("op_2", write("req_2", "op_2", 2));
    dependent.depends_on = Some(op_id("op_1"));
    outbox.enqueue(dependent).expect("enqueue");

    // The dependency is not acknowledged yet, so the dependent operation
    // waits and is preserved.
    let mut sink = RecordingSink {
        submissions: Vec::new(),
        verdict: Some(SubmissionVerdict::Unavailable),
    };
    let report = outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(report.submitted, 1);
    assert_eq!(report.unavailable, 1);
    let dependent_outcome = report
        .outcomes
        .iter()
        .find(|outcome| outcome.operation_id == op_id("op_2"))
        .expect("dependent outcome");
    assert!(!dependent_outcome.submitted);
    assert_eq!(
        dependent_outcome.reason,
        Some(OperationReason::DependencyPending)
    );
    let view = outbox.operation(&op_id("op_2")).expect("operation view");
    assert_eq!(view.status, OperationStatus::Pending);
    assert_eq!(view.depends_on, Some(op_id("op_1")));
}
