//! The durable offline outbox for the v3 episode ledger.
//!
//! These tests pin the behaviour the outbox claims:
//!
//! - a queued operation survives a close and a reopen, and is reported
//!   pending;
//! - draining against a policy that still accepts an operation authorizes
//!   it, the operation leaves the outbox as `Acknowledged`, and it is never
//!   resent;
//! - draining against a policy that refuses an operation yields the right
//!   reauthorization verdict, and the operation is *preserved* with that
//!   reason rather than dropped;
//! - a moved consent epoch yields `NeedsConsent`, and a contract revision
//!   that is no longer current yields `SchemaSkew`; both are distinct from a
//!   policy refusal;
//! - a delete/forget proposal is reauthorized by the same gate, and it is
//!   only accepted while its target is live;
//! - two edits over the same field conflict, and both branches are
//!   preserved; two edits over disjoint fields merge automatically; two
//!   edits over incompatible versions of one object conflict whole;
//! - the merge is deterministic, and the order of independent operations
//!   does not change the merged outcome;
//! - an asset transfer resumes: the completed chunks are not re-transferred,
//!   the final digest is verified, and a corrupted chunk is detected;
//! - an operation survives an arbitrary elapsed time: there is no fixed
//!   offline expiry;
//! - the outbox is not a canonical writer: the canonical store holds exactly
//!   the writes that were authorized and bound, and the outbox holds no
//!   canonical episode digest;
//! - an acknowledged operation is removed by an authorized caller, the count
//!   drops, and the outbox accepts new work again; a service identity may
//!   not remove anything;
//! - a removal another operation depends on is refused: the dependent would
//!   name a dependency the outbox does not hold, and it could never be sent;
//!   the dependent survives the refusal, stays inspectable, and is still
//!   drained once its dependency is acknowledged; the refusal is
//!   content-free;
//! - a full outbox refuses new work with `Capacity`, which is distinct from
//!   an invalid proposal;
//! - a corrupted sidecar is `Corrupt`, and a truncated file is `Corrupt` too:
//!   the outbox never opens with fewer operations than were queued, and a
//!   trailing fragment is refused rather than discarded;
//! - the outbox's file and its status surface carry no episode content.
//!
//! Everything is deterministic and local: no network, and explicit sequence
//! numbers instead of a wall clock.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeSource, EpisodeStore, EpisodeWrite, EvidenceLevel,
    GuildEpisodePolicy, MemoryCandidate, MemoryClass, RetentionClass, StorePolicy,
};
use abi_wdbx::v3::offline::{
    AssetProposal, EditProposal, FieldEdit, MergeObjectKind, MergeTarget, NewOperation,
    OfflineOutbox, OperationBinding, OperationId, OperationPayload, OperationSink, OperationStatus,
    OutboxError, Resolution, Submission, SubmissionVerdict,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[path = "v3_offline_outbox/asset.rs"]
mod asset;
#[path = "v3_offline_outbox/format.rs"]
mod format;
#[path = "v3_offline_outbox/lifecycle.rs"]
mod lifecycle;
#[path = "v3_offline_outbox/merge.rs"]
mod merge;

/// The scope every operation in these tests belongs to.
pub(crate) const SCOPE: &str = "guild_ref";

/// The contract revision the current policy serves.
pub(crate) const REVISION: u64 = 2;

/// The contract digest the current policy serves.
pub(crate) const DIGEST: [u8; 32] = [9; 32];

pub(crate) struct Scratch(PathBuf);

impl Scratch {
    pub(crate) fn new(purpose: &str) -> Self {
        let path = std::env::temp_dir().join(format!("wdbx-offline-{purpose}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("scratch directory");
        Self(path)
    }

    pub(crate) fn child(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A guild policy that accepts durable writes.
pub(crate) fn guild_policy() -> GuildEpisodePolicy {
    GuildEpisodePolicy {
        learning_enabled: true,
        policy_version: "policy_v1".into(),
        token_budget: 1_000,
        storage_budget_bytes: 1024 * 1024,
        current_consent_epoch: None,
        retention_ttl_ms: None,
        retention_windows: None,
        authorized_principals: None,
    }
}

/// The current server policy.
pub(crate) fn policy() -> StorePolicy {
    StorePolicy {
        contract_revision: REVISION,
        contract_digest: DIGEST,
        guilds: BTreeMap::from([(SCOPE.to_string(), guild_policy())]),
    }
}

/// A policy that refuses durable writes.
pub(crate) fn learning_disabled_policy() -> StorePolicy {
    let mut policy = policy();
    if let Some(guild) = policy.guilds.get_mut(SCOPE) {
        guild.learning_enabled = false;
    }
    policy
}

/// A policy whose consent epoch has moved on.
pub(crate) fn moved_consent_policy() -> StorePolicy {
    let mut policy = policy();
    if let Some(guild) = policy.guilds.get_mut(SCOPE) {
        guild.current_consent_epoch = Some(8);
    }
    policy
}

/// A policy that serves a different contract revision.
pub(crate) fn skewed_policy() -> StorePolicy {
    let mut policy = policy();
    policy.contract_revision = REVISION + 1;
    policy
}

/// The binding an operation is written under.
pub(crate) fn binding() -> OperationBinding {
    OperationBinding {
        guild_ref: SCOPE.into(),
        contract_revision: REVISION,
        contract_digest: DIGEST,
        policy_version: "policy_v1".into(),
        consent_epoch: None,
    }
}

/// A bounded operation identifier.
pub(crate) fn op_id(value: &str) -> OperationId {
    OperationId::new(value).expect("bounded operation id")
}

/// A content-free memory candidate.
pub(crate) fn candidate(seed: u8) -> MemoryCandidate {
    MemoryCandidate {
        class: MemoryClass::Fact,
        retention: RetentionClass::Durable,
        payload_commitment: [seed; 32],
        payload_bytes: 64,
        dimension: None,
        embedding_version: None,
        member_scoped: false,
        supersedes: None,
        forgets: None,
    }
}

/// A proposed episode write.
pub(crate) fn write(request_id: &str, operation_id: &str, seed: u8) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        contract_revision: REVISION,
        contract_digest: DIGEST,
        guild_ref: SCOPE.into(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C1,
        event: EpisodeEvent::MemoryCandidate {
            recorded_by: ActorRef {
                principal_id: "abbey_service".into(),
                kind: ActorKind::Service,
            },
            candidate: candidate(seed),
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}

/// A proposed erase of one remembered record.
pub(crate) fn forget(request_id: &str, operation_id: &str, target: [u8; 32]) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        contract_revision: REVISION,
        contract_digest: DIGEST,
        guild_ref: SCOPE.into(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C1,
        event: EpisodeEvent::MemoryCandidate {
            recorded_by: ActorRef {
                principal_id: "abbey_service".into(),
                kind: ActorKind::Service,
            },
            candidate: MemoryCandidate {
                class: MemoryClass::Fact,
                retention: RetentionClass::Durable,
                payload_commitment: [0; 32],
                payload_bytes: 0,
                dimension: None,
                embedding_version: None,
                member_scoped: false,
                supersedes: None,
                forgets: Some(target),
            },
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}

/// A proposed voice write, bound to one consent epoch.
pub(crate) fn voice_write(request_id: &str, operation_id: &str, epoch: u64) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
        contract_revision: REVISION,
        contract_digest: DIGEST,
        guild_ref: SCOPE.into(),
        consent_epoch: Some(epoch),
        source_type: EpisodeSource::DiscordVoice,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C1,
        event: EpisodeEvent::Proposal {
            requested_by: ActorRef {
                principal_id: "requester".into(),
                kind: ActorKind::HumanSubject,
            },
            proposed_by: ActorRef {
                principal_id: "abbey_service".into(),
                kind: ActorKind::Service,
            },
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}

/// A write operation, enqueued.
pub(crate) fn write_operation(id: &str, proposal: EpisodeWrite) -> NewOperation {
    NewOperation {
        id: op_id(id),
        depends_on: None,
        binding: binding(),
        payload: OperationPayload::Write(Box::new(proposal)),
    }
}

/// A delete operation, enqueued.
pub(crate) fn delete_operation(id: &str, proposal: EpisodeWrite) -> NewOperation {
    NewOperation {
        id: op_id(id),
        depends_on: None,
        binding: binding(),
        payload: OperationPayload::Delete(Box::new(proposal)),
    }
}

/// An edit operation over one mergeable object.
pub(crate) fn edit_operation(id: &str, target: MergeTarget, fields: &[(&str, u8)]) -> NewOperation {
    NewOperation {
        id: op_id(id),
        depends_on: None,
        binding: binding(),
        payload: OperationPayload::Edit(EditProposal {
            target,
            fields: fields
                .iter()
                .map(|(key, seed)| FieldEdit {
                    key: (*key).into(),
                    value_digest: [*seed; 32],
                })
                .collect(),
        }),
    }
}

/// An asset operation.
pub(crate) fn asset_operation(id: &str, proposal: AssetProposal) -> NewOperation {
    NewOperation {
        id: op_id(id),
        depends_on: None,
        binding: binding(),
        payload: OperationPayload::Asset(proposal),
    }
}

/// A sink that records what it was given and answers a fixed verdict.
#[derive(Default)]
pub(crate) struct RecordingSink {
    pub(crate) submissions: Vec<Submission>,
    pub(crate) verdict: Option<SubmissionVerdict>,
}

impl RecordingSink {
    pub(crate) fn accepting() -> Self {
        Self {
            submissions: Vec::new(),
            verdict: Some(SubmissionVerdict::Accepted),
        }
    }
}

impl OperationSink for RecordingSink {
    fn submit(&mut self, submission: &Submission) -> SubmissionVerdict {
        self.submissions.push(submission.clone());
        self.verdict.unwrap_or(SubmissionVerdict::Accepted)
    }
}

/// Whether the bytes hold the needle.
pub(crate) fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The lower-case hex of some bytes.
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// A mergeable object of one kind.
pub(crate) fn target(kind: MergeObjectKind, id: &str) -> MergeTarget {
    MergeTarget {
        kind,
        id: id.into(),
    }
}

/// The governance authority the removal tests use.
fn owner() -> ActorRef {
    ActorRef {
        principal_id: "guild_owner".into(),
        kind: ActorKind::GuildOwner,
    }
}

/// The service identity that may never remove anything.
fn service() -> ActorRef {
    ActorRef {
        principal_id: "abbey_service".into(),
        kind: ActorKind::Service,
    }
}

/// An outbox that holds one operation and one operation that depends on it.
fn outbox_with_a_dependent(scratch: &Scratch) -> OfflineOutbox {
    let mut outbox = OfflineOutbox::open(scratch.child("outbox")).expect("open outbox");
    outbox
        .enqueue(write_operation("op_1", write("req_1", "op_1", 1)))
        .expect("enqueue");
    let mut dependent = write_operation("op_2", write("req_2", "op_2", 2));
    dependent.depends_on = Some(op_id("op_1"));
    outbox.enqueue(dependent).expect("enqueue");
    outbox
}

/// A removal another operation depends on is refused.
///
/// The dependent would name a dependency the outbox does not hold, and no
/// drain could ever satisfy it, so the removal is refused instead of
/// applied. The refusal leaves the outbox exactly as it was.
#[test]
fn a_removal_that_another_operation_depends_on_is_refused() {
    let scratch = Scratch::new("dependent-removal");
    let mut outbox = outbox_with_a_dependent(&scratch);

    let error = outbox
        .resolve(&op_id("op_1"), Resolution::Remove, &owner())
        .expect_err("the removal is refused");
    assert_eq!(error, OutboxError::DependencyPresent);

    // Nothing was removed: the refusal is not a removal.
    assert_eq!(outbox.status().total(), 2);
    assert!(outbox.operation(&op_id("op_1")).is_some());
    let dependent = outbox.operation(&op_id("op_2")).expect("the dependent");
    assert_eq!(dependent.status, OperationStatus::Pending);
    assert_eq!(dependent.depends_on, Some(op_id("op_1")));

    // The refusal wrote nothing: the file still holds both operations.
    drop(outbox);
    let outbox = OfflineOutbox::open(scratch.child("outbox")).expect("reopen outbox");
    assert_eq!(outbox.status().total(), 2);
    assert!(outbox.operation(&op_id("op_2")).is_some());
}

/// A discard is a removal too, and it is refused the same way.
#[test]
fn a_discard_that_another_operation_depends_on_is_refused() {
    let scratch = Scratch::new("dependent-discard");
    let store_directory = scratch.child("episodes");
    let current = learning_disabled_policy();
    let store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let mut outbox = outbox_with_a_dependent(&scratch);

    // A policy that refuses leaves both operations refused, so both are
    // discardable in principle.
    let mut sink = RecordingSink::accepting();
    outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(outbox.status().rejected, 2);

    // The discard of the dependency is refused: the dependent names it.
    assert!(matches!(
        outbox.resolve(&op_id("op_1"), Resolution::Discard, &owner()),
        Err(OutboxError::DependencyPresent)
    ));
    assert_eq!(outbox.status().total(), 2);

    // The dependent goes first, and the dependency then goes too.
    outbox
        .resolve(&op_id("op_2"), Resolution::Discard, &owner())
        .expect("discard the dependent");
    outbox
        .resolve(&op_id("op_1"), Resolution::Discard, &owner())
        .expect("discard the dependency");
    assert_eq!(outbox.status().total(), 0);
}

/// The dependent of a refused removal survives it: it is still queued, it
/// still names its dependency, and a drain still sends it once that
/// dependency is acknowledged.
#[test]
fn the_dependent_of_a_refused_removal_is_still_drainable() {
    let scratch = Scratch::new("dependent-drain");
    let store_directory = scratch.child("episodes");
    let current = policy();
    let store = EpisodeStore::open(&store_directory, current.clone()).expect("canonical store");
    let mut outbox = outbox_with_a_dependent(&scratch);

    assert!(matches!(
        outbox.resolve(&op_id("op_1"), Resolution::Remove, &owner()),
        Err(OutboxError::DependencyPresent)
    ));

    // The dependent is untouched, and it is still drainable: the dependency
    // is acknowledged first, and the dependent follows it in the same drain.
    let mut sink = RecordingSink::accepting();
    let report = outbox.drain(&current, &store, &mut sink).expect("drain");
    assert_eq!(report.submitted, 2);
    assert_eq!(report.accepted, 2);
    assert_eq!(sink.submissions.len(), 2);
    assert_eq!(outbox.status().acknowledged, 2);
    assert_eq!(outbox.status().pending, 0);
    assert_eq!(
        outbox
            .operation(&op_id("op_2"))
            .expect("the dependent")
            .status,
        OperationStatus::Acknowledged
    );
}

/// The documented way out of the refusal: the caller resolves the dependent
/// first, and the removal then succeeds. An operation with no dependent
/// leaves the outbox exactly as before.
#[test]
fn the_dependents_are_resolved_first_and_the_removal_then_succeeds() {
    let scratch = Scratch::new("dependent-order");
    let mut outbox = outbox_with_a_dependent(&scratch);

    // The dependency cannot go first.
    assert!(matches!(
        outbox.resolve(&op_id("op_1"), Resolution::Remove, &owner()),
        Err(OutboxError::DependencyPresent)
    ));

    // The dependent has no dependent of its own, so it goes.
    let receipt = outbox
        .resolve(&op_id("op_2"), Resolution::Remove, &owner())
        .expect("the dependent is removed");
    assert_eq!(receipt.status, OperationStatus::Pending);
    assert_eq!(outbox.status().total(), 1);

    // And now the dependency may go too.
    let receipt = outbox
        .resolve(&op_id("op_1"), Resolution::Remove, &owner())
        .expect("the dependency is removed");
    assert_eq!(receipt.status, OperationStatus::Pending);
    assert_eq!(outbox.status().total(), 0);
    assert!(outbox.operation(&op_id("op_1")).is_none());
    assert!(outbox.operation(&op_id("op_2")).is_none());
}

/// A service identity may not remove anything: not the dependency, and not
/// the dependent either.
#[test]
fn a_service_may_not_remove_an_operation_with_a_dependent() {
    let scratch = Scratch::new("dependent-service");
    let mut outbox = outbox_with_a_dependent(&scratch);

    assert!(matches!(
        outbox.resolve(&op_id("op_1"), Resolution::Remove, &service()),
        Err(OutboxError::AuthorityDenied)
    ));
    assert!(matches!(
        outbox.resolve(&op_id("op_2"), Resolution::Remove, &service()),
        Err(OutboxError::AuthorityDenied)
    ));
    assert_eq!(outbox.status().total(), 2);
}

/// The refusal is content-free: it names neither the operation, nor the
/// dependent, nor the scope, nor anything else the outbox holds.
#[test]
fn the_refusal_of_a_removal_with_a_dependent_is_content_free() {
    let scratch = Scratch::new("dependent-content-free");
    let mut outbox = outbox_with_a_dependent(&scratch);

    let error = outbox
        .resolve(&op_id("op_1"), Resolution::Remove, &owner())
        .expect_err("the removal is refused");
    let message = error.to_string();
    assert_eq!(message, "offline_outbox_dependency_present");
    for forbidden in ["op_1", "op_2", SCOPE, "req_1", "req_2"] {
        assert!(
            !message.contains(forbidden),
            "the refusal must not name {forbidden}: {message}"
        );
    }
}
