//! The outbox's deterministic independent-edit merge.

use super::*;
use abi_wdbx::v3::episode::EpisodeStore;
use abi_wdbx::v3::offline::{ConflictKind, OfflineOutbox, OperationReason, OperationStatus};

/// Two edits over the same field conflict, and both branches are preserved.
#[test]
fn two_edits_over_one_field_conflict_and_are_preserved() {
    let scratch = Scratch::new("conflict");
    let directory = scratch.child("outbox");
    let object = target(MergeObjectKind::Specimen, "specimen_1");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(edit_operation("op_a", object.clone(), &[("alpha", 1)]))
        .expect("enqueue");
    outbox
        .enqueue(edit_operation("op_b", object.clone(), &[("alpha", 2)]))
        .expect("enqueue");

    let plan = outbox.merge_plan();
    assert_eq!(plan.merged.len(), 0);
    assert_eq!(plan.conflicts.len(), 1);
    let conflict = &plan.conflicts[0];
    assert_eq!(conflict.kind, ConflictKind::DivergentValues);
    assert_eq!(conflict.field.as_deref(), Some("alpha"));
    assert_eq!(conflict.value_digests, vec![[1; 32], [2; 32]]);
    assert_eq!(conflict.operations, vec![op_id("op_a"), op_id("op_b")]);

    // Applying the merge records the conflict on both branches.
    outbox.merge().expect("merge");
    let status = outbox.status();
    assert_eq!(status.conflicted, 2);
    assert_eq!(status.pending, 0);
    for identifier in ["op_a", "op_b"] {
        let view = outbox
            .operation(&op_id(identifier))
            .expect("operation view");
        assert_eq!(view.status, OperationStatus::Conflicted);
        assert_eq!(view.reason, Some(OperationReason::MergeConflict));
        let summary = view.conflict.expect("conflict summary");
        assert_eq!(summary.object_kind, MergeObjectKind::Specimen);
        assert_eq!(summary.field.as_deref(), Some("alpha"));
        assert_eq!(summary.value_digests, 2);
        assert_eq!(summary.operations, 2);
    }

    // The conflict survives a reopen: it is recorded, not resolved away.
    drop(outbox);
    let outbox = OfflineOutbox::open(&directory).expect("reopen outbox");
    assert_eq!(outbox.status().conflicted, 2);
}

/// Two edits over disjoint fields merge automatically, and the merged
/// proposal is what the drain submits.
#[test]
fn two_edits_over_disjoint_fields_merge_automatically() {
    let scratch = Scratch::new("disjoint");
    let directory = scratch.child("outbox");
    let store_directory = scratch.child("episodes");
    let object = target(MergeObjectKind::Learning, "learning_1");
    let current = policy();
    let store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(edit_operation("op_a", object.clone(), &[("alpha", 1)]))
        .expect("enqueue");
    outbox
        .enqueue(edit_operation("op_b", object.clone(), &[("beta", 2)]))
        .expect("enqueue");

    let plan = outbox.merge_plan();
    assert_eq!(plan.conflicts.len(), 0);
    assert_eq!(plan.merged.len(), 1);
    let merged = &plan.merged[0];
    assert_eq!(merged.target, object);
    assert_eq!(merged.fields.len(), 2);
    assert_eq!(merged.fields[0].key, "alpha");
    assert_eq!(merged.fields[1].key, "beta");
    assert_eq!(merged.contributors, vec![op_id("op_a"), op_id("op_b")]);

    // The drain submits one merged proposal, not two.
    let mut sink = RecordingSink::accepting();
    let report = outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(report.submitted, 1);
    assert_eq!(sink.submissions.len(), 1);
    assert_eq!(sink.submissions[0].contributors.len(), 2);
    let OperationPayload::Edit(edit) = &sink.submissions[0].payload else {
        panic!("the merged submission is an edit");
    };
    assert_eq!(edit.fields.len(), 2);
    assert_eq!(outbox.status().acknowledged, 2);
}

/// The drain records a conflict, preserves both branches, and submits
/// neither of them.
#[test]
fn the_drain_records_a_conflict_and_submits_neither_branch() {
    let scratch = Scratch::new("drain-conflict");
    let directory = scratch.child("outbox");
    let store_directory = scratch.child("episodes");
    let object = target(MergeObjectKind::Authority, "authority_1");
    let current = policy();
    let store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(edit_operation("op_a", object.clone(), &[("alpha", 1)]))
        .expect("enqueue");
    outbox
        .enqueue(edit_operation("op_b", object.clone(), &[("alpha", 2)]))
        .expect("enqueue");

    let mut sink = RecordingSink::accepting();
    let report = outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(report.merge.conflicts.len(), 1);
    assert_eq!(report.merge.merged.len(), 0);
    assert_eq!(report.submitted, 0);
    assert_eq!(sink.submissions.len(), 0);
    assert_eq!(outbox.status().conflicted, 2);
    for outcome in &report.outcomes {
        assert_eq!(outcome.status, OperationStatus::Conflicted);
        assert_eq!(outcome.reason, Some(OperationReason::MergeConflict));
        assert!(!outcome.submitted);
    }
}

/// Two edits over incompatible versions of one object are preserved whole
/// rather than merged into a hybrid nobody wrote.
#[test]
fn two_edits_over_incompatible_versions_conflict_whole() {
    let scratch = Scratch::new("versions");
    let directory = scratch.child("outbox");
    let object = target(MergeObjectKind::Authority, "authority_1");
    let mut outbox = OfflineOutbox::open(&directory).expect("open outbox");
    outbox
        .enqueue(edit_operation("op_a", object.clone(), &[("alpha", 1)]))
        .expect("enqueue");
    let mut second = edit_operation("op_b", object.clone(), &[("beta", 2)]);
    // The second edit was written under another policy version.
    second.binding.policy_version = "policy_v2".into();
    outbox.enqueue(second).expect("enqueue");

    let plan = outbox.merge_plan();
    assert_eq!(plan.merged.len(), 0);
    assert_eq!(plan.conflicts.len(), 1);
    let conflict = &plan.conflicts[0];
    assert_eq!(conflict.kind, ConflictKind::IncompatibleVersion);
    assert_eq!(conflict.field, None);
    assert_eq!(conflict.operations, vec![op_id("op_a"), op_id("op_b")]);

    outbox.merge().expect("merge");
    assert_eq!(outbox.status().conflicted, 2);
    assert_eq!(outbox.status().pending, 0);
}

/// The merge is deterministic, and the order of independent operations does
/// not change the merged outcome.
#[test]
fn the_merge_is_deterministic_and_order_independent() {
    let scratch = Scratch::new("determinism");
    let first = scratch.child("first");
    let second = scratch.child("second");
    let object = target(MergeObjectKind::Network, "network_1");

    let mut left = OfflineOutbox::open(&first).expect("open outbox");
    left.enqueue(edit_operation("op_a", object.clone(), &[("alpha", 1)]))
        .expect("enqueue");
    left.enqueue(edit_operation("op_b", object.clone(), &[("beta", 2)]))
        .expect("enqueue");

    let mut right = OfflineOutbox::open(&second).expect("open outbox");
    right
        .enqueue(edit_operation("op_b", object.clone(), &[("beta", 2)]))
        .expect("enqueue");
    right
        .enqueue(edit_operation("op_a", object.clone(), &[("alpha", 1)]))
        .expect("enqueue");

    // The same operations in the same order produce the same result.
    assert_eq!(left.merge_plan(), left.merge_plan());
    // The order of independent operations does not change the outcome.
    assert_eq!(left.merge_plan(), right.merge_plan());
}
