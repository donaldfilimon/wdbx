//! The studio's canonical adapter over the WDBX v3 episode ledger.
//!
//! These tests pin the adapter's contract:
//!
//! * the studio's operations map onto the substrate's episode writes, and
//!   the substrate's own gate answers every refusal;
//! * the pre-flight decision and the commit path agree;
//! * the projection keeps the eight evidence dimensions apart and holds no
//!   value that could stand in for them;
//! * the adapter writes to the ledger only through the substrate's one write
//!   path, and a refused operation leaves the ledger byte-identical;
//! * the losses are declared, in this edition and in the browser edition.

use abi_wdbx::v3::episode::{
    AdmissionReason, Disposition, EpisodeEvent, EpisodeSource, EvidenceDimension, MemoryCandidate,
    MemoryClass, RetentionClass,
};
use serde_json::{Value, json};
use specimen_core::canonical::{
    CanonicalAdapter, CanonicalConfig, CanonicalDecision, CanonicalOperation, DeferredStage,
    LossId, LossManifest, OperationKind, StudioRejection, canonical_content_bytes,
    content_commitment,
};
use specimen_core::persistence::Store;

/// The adapter's own source, for the structural claims.
///
/// The adapter is a directory of submodules, so the whole module is read:
/// one file would hide a forbidden token in another.
const CANONICAL_SOURCE: &str = concat!(
    include_str!("../src/canonical/mod.rs"),
    include_str!("../src/canonical/operations.rs"),
    include_str!("../src/canonical/manifest.rs"),
    include_str!("../src/canonical/projection.rs"),
    include_str!("../src/canonical/journal.rs"),
    include_str!("../src/canonical/rebuild.rs"),
);

fn starter() -> Value {
    serde_json::from_str(include_str!("../../../conformance/starter.json")).unwrap()
}

fn proposal(status: &str) -> Value {
    json!({"id":"p1","pattern":"hello","evidence":3,"status":status,"pros":[],"cons":[]})
}

fn edit(request_id: &str, specimen: Value) -> CanonicalOperation {
    CanonicalOperation::new(
        request_id,
        OperationKind::Edit {
            specimen,
            supersedes: None,
        },
    )
    .expect("the request identifier is a bounded identifier")
}

fn learn(request_id: &str, status: &str) -> CanonicalOperation {
    CanonicalOperation::new(
        request_id,
        OperationKind::Learn {
            proposal: proposal(status),
            erases: None,
        },
    )
    .expect("the request identifier is a bounded identifier")
}

/// The candidate one operation proposes, or a failure.
fn candidate_of(adapter: &CanonicalAdapter, operation: &CanonicalOperation) -> MemoryCandidate {
    let write = adapter
        .write_for(operation)
        .expect("the operation proposes a write");
    let EpisodeEvent::MemoryCandidate { candidate, .. } = write.event else {
        panic!("the operation is a memory candidate");
    };
    candidate
}

#[test]
fn an_edit_becomes_a_candidate_over_the_studio_content_and_is_accepted() {
    let temp = tempfile::tempdir().unwrap();
    let mut adapter = CanonicalAdapter::open(temp.path(), CanonicalConfig::default()).unwrap();
    let specimen = starter();
    let operation = edit("request_edit_1", specimen.clone());

    let candidate = candidate_of(&adapter, &operation);
    let bytes = canonical_content_bytes(&specimen);
    assert_eq!(
        candidate.payload_commitment,
        content_commitment(&bytes),
        "the commitment is the digest of the studio's content blob"
    );
    assert_eq!(candidate.payload_bytes, bytes.len() as u64);
    assert_eq!(candidate.class, MemoryClass::Summary);
    assert_eq!(candidate.retention, RetentionClass::Durable);
    assert_eq!(candidate.forgets, None);
    assert_eq!(candidate.supersedes, None);
    assert!(!candidate.member_scoped);

    let write = adapter.write_for(&operation).unwrap();
    assert_eq!(write.source_type, EpisodeSource::LocalRuntime);
    assert_eq!(
        write.expected_commitment, None,
        "the studio never predicts the episode commitment"
    );

    // The substrate accepts it and answers with its own receipt.
    let decision = adapter.commit(&operation);
    assert!(decision.is_admitted(), "the substrate refused the edit");
    let receipt = decision
        .receipt()
        .expect("the substrate returned a receipt");
    assert_eq!(receipt.guild_ref, write.guild_ref);
    assert_eq!(receipt.event_kind, "memory_candidate");
    assert!(receipt.redacted);
}

#[test]
fn a_forget_is_the_tombstone_shape_the_substrate_requires_and_is_accepted() {
    let temp = tempfile::tempdir().unwrap();
    let mut adapter = CanonicalAdapter::open(temp.path(), CanonicalConfig::default()).unwrap();
    let edit_operation = edit("request_edit_1", starter());
    let target = adapter
        .commit(&edit_operation)
        .receipt()
        .expect("the edit was admitted")
        .episode_digest;

    let operation = CanonicalOperation::new("request_forget_1", OperationKind::Forget { target })
        .expect("the request identifier is a bounded identifier");
    let candidate = candidate_of(&adapter, &operation);
    assert_eq!(candidate.payload_bytes, 0);
    assert_eq!(candidate.payload_commitment, [0u8; 32]);
    assert_eq!(candidate.forgets, Some(target));
    assert_eq!(candidate.supersedes, None);

    let decision = adapter.commit(&operation);
    assert!(decision.is_admitted(), "the substrate refused the erasure");

    // The shape matters: a tombstone that carries a payload is refused.
    let mut malformed = adapter
        .write_for(&operation)
        .expect("the operation proposes a write");
    malformed.request_id = "request_forget_2".into();
    malformed.operation_id = "op_request_forget_2".into();
    if let EpisodeEvent::MemoryCandidate { candidate, .. } = &mut malformed.event {
        candidate.payload_bytes = 12;
        candidate.payload_commitment = [7u8; 32];
    }
    let refused = adapter.store().decide_write_outcome(&malformed);
    assert_eq!(refused.reason(), AdmissionReason::InvalidInput);
}

#[test]
fn a_refusal_carries_the_substrates_own_reason_and_writes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let config = CanonicalConfig {
        learning_enabled: false,
        ..CanonicalConfig::default()
    };
    let mut adapter = CanonicalAdapter::open(temp.path(), config).unwrap();
    let ledger = temp.path().join("episodes.v1.jsonl");
    let operation = learn("request_learn_1", "learned");

    let decision = adapter.decide(&operation);
    assert_eq!(
        decision.substrate_reason(),
        Some(AdmissionReason::LearningDisabled),
        "the refusal reason is the substrate's own"
    );
    assert_eq!(
        std::fs::read(&ledger).unwrap(),
        Vec::<u8>::new(),
        "the decision wrote nothing"
    );

    let committed = adapter.commit(&operation);
    assert_eq!(
        committed.substrate_reason(),
        Some(AdmissionReason::LearningDisabled)
    );
    assert_eq!(
        std::fs::read(&ledger).unwrap(),
        Vec::<u8>::new(),
        "the refusal wrote nothing"
    );
}

#[test]
fn decide_and_commit_agree_on_every_operation() {
    let temp = tempfile::tempdir().unwrap();
    let mut adapter = CanonicalAdapter::open(temp.path(), CanonicalConfig::default()).unwrap();
    let admitted = edit("request_edit_1", starter());
    let learned = learn("request_learn_1", "learned");
    let unknown = CanonicalOperation::new(
        "request_forget_1",
        OperationKind::Forget { target: [9u8; 32] },
    )
    .expect("the request identifier is a bounded identifier");
    let pending = learn("request_learn_2", "pending");
    let malformed = learn("request_learn_3", "half-learned");

    for operation in [&admitted, &learned, &unknown, &pending, &malformed] {
        let decided = adapter.decide(operation);
        let committed = adapter.commit(operation);
        match (&decided, &committed) {
            (
                CanonicalDecision::Admitted { class: left, .. },
                CanonicalDecision::Admitted { class: right, .. },
            ) => assert_eq!(left, right, "the classes agree"),
            (
                CanonicalDecision::Refused { reason: left, .. },
                CanonicalDecision::Refused { reason: right, .. },
            ) => assert_eq!(left, right, "the reasons agree"),
            (
                CanonicalDecision::Rejected { reason: left },
                CanonicalDecision::Rejected { reason: right },
            ) => assert_eq!(left, right, "the studio's reasons agree"),
            (
                CanonicalDecision::Deferred { stage: left },
                CanonicalDecision::Deferred { stage: right },
            ) => assert_eq!(left, right, "the stages agree"),
            _ => panic!("decide and commit disagree"),
        }
    }

    // The refusals are the substrate's own reasons, the deferral is the
    // studio's own state, and a malformed proposal is the studio's own
    // refusal: it never became an episode write, so the substrate was never
    // asked about it.
    assert_eq!(
        adapter.decide(&unknown).substrate_reason(),
        Some(AdmissionReason::InvalidTransition)
    );
    assert_eq!(
        adapter.decide(&pending),
        CanonicalDecision::Deferred {
            stage: DeferredStage::PendingProposal
        }
    );
    assert_eq!(
        adapter.decide(&malformed),
        CanonicalDecision::Rejected {
            reason: StudioRejection::UnknownProposalState
        }
    );
    assert_eq!(
        adapter.decide(&malformed).substrate_reason(),
        None,
        "the studio does not borrow a substrate label"
    );
}

#[test]
fn the_projection_reports_each_episode_with_its_disposition_and_dimensions_apart() {
    let temp = tempfile::tempdir().unwrap();
    let mut adapter = CanonicalAdapter::open(temp.path(), CanonicalConfig::default()).unwrap();
    adapter.commit(&edit("request_edit_1", starter()));
    adapter.commit(&learn("request_learn_1", "learned"));

    let projection = adapter.projection().unwrap();
    assert_eq!(projection.scope, CanonicalConfig::default().scope);
    assert_eq!(projection.episodes.len(), 2);
    assert!(!projection.truncated);

    for episode in &projection.episodes {
        // Every dimension is reported on its own, with its own reason.
        assert_eq!(episode.explanation.entries.len(), 8);
        let dimensions: Vec<EvidenceDimension> = episode
            .explanation
            .entries
            .iter()
            .map(|(dimension, _)| *dimension)
            .collect();
        assert_eq!(dimensions, EvidenceDimension::all().to_vec());
        // The receipt is the substrate's own, and content-free.
        assert!(episode.receipt.redacted);
    }

    // The dispositions are the substrate's own: a local observation is
    // accepted, and a proposal that was never observed is retained as
    // uncertain rather than presented as evidence.
    let dispositions: Vec<Disposition> = projection
        .episodes
        .iter()
        .map(|episode| episode.disposition)
        .collect();
    assert!(dispositions.contains(&Disposition::Accepted));
    assert!(dispositions.contains(&Disposition::Uncertain));
    assert_eq!(
        projection.disposition_counts.get(&Disposition::Accepted),
        Some(&1)
    );
    assert_eq!(
        projection.disposition_counts.get(&Disposition::Uncertain),
        Some(&1)
    );
}

#[test]
fn the_projection_holds_no_value_that_could_stand_in_for_the_dimensions() {
    // The structural claim, checked at the source: the adapter declares no
    // floating-point value and no single number for the eight dimensions.
    for forbidden in ["f32", "f64", "score", "combined"] {
        assert!(
            !CANONICAL_SOURCE.contains(forbidden),
            "{forbidden} appears in the canonical adapter"
        );
    }
    assert!(!CANONICAL_SOURCE.contains("fn combined"));
    assert!(!CANONICAL_SOURCE.contains("fn score"));
}

#[test]
fn the_losses_are_declared_in_this_edition_and_in_the_browser_edition() {
    let manifest = LossManifest::native();
    assert!(!manifest.declarations.is_empty());
    assert_eq!(manifest.edition, "native");

    // The browser edition holds no canonical ledger, and the native manifest
    // names exactly what it drops.
    let browser = LossManifest::browser().unwrap();
    assert!(!browser.declarations.is_empty());
    assert_eq!(browser.edition, "browser");
    let named = manifest
        .declaration(LossId::BrowserEdition)
        .expect("the native manifest names the browser edition");
    for declaration in &browser.declarations {
        assert!(
            named.dropped.contains(declaration.id.label()),
            "the native manifest does not name {}",
            declaration.id.label()
        );
    }

    // The native edition's own losses are named too.
    for id in [
        LossId::PayloadContent,
        LossId::PayloadCommitment,
        LossId::SignatureEvidence,
        LossId::CalibrationEvidence,
        LossId::RegimeEvidence,
        LossId::SourceReputation,
        LossId::RecordedUncertainty,
        LossId::RecordAge,
        LossId::OutcomeObservation,
        LossId::ProposedSource,
        LossId::ReplicationScope,
    ] {
        let declaration = manifest
            .declaration(id)
            .unwrap_or_else(|| panic!("the native manifest declares {id:?}"));
        assert!(!declaration.dropped.is_empty());
        assert!(!declaration.reason.is_empty());
    }

    // The payload commitment is declared, and it says what the projection
    // carries instead: the episode digest, which commits to the payload and
    // its byte count without exposing either.
    let commitment = manifest
        .declaration(LossId::PayloadCommitment)
        .expect("the native manifest declares the payload commitment");
    assert!(commitment.reason.contains("episode digest"));
    assert!(
        !commitment
            .reason
            .contains("carries a digest and a byte count instead"),
        "the payload content declaration must not claim the projection carries a digest and a byte count"
    );
    let content = manifest
        .declaration(LossId::PayloadContent)
        .expect("the native manifest declares the payload content");
    assert!(content.reason.contains("proposed write"));
    assert!(content.reason.contains("episode digest"));

    // A learned proposal is a proposal, so it reads Uncertain by design and
    // never Accepted. The manifest says so.
    let proposed = manifest
        .declaration(LossId::ProposedSource)
        .expect("the native manifest declares the proposed source");
    assert!(proposed.reason.contains("Uncertain"));
    assert!(proposed.reason.contains("Accepted"));
}

#[test]
fn a_request_id_the_studio_cannot_bound_is_refused_before_the_substrate_sees_it() {
    // The adapter derives the substrate's operation identifier from the
    // request identifier, and the substrate bounds both at 64 characters: a
    // 61-character request identifier would become a 64-character operation
    // identifier and a 62-character one would be refused by the substrate
    // late, as an invalid input. The studio refuses it early instead.
    let refused = CanonicalOperation::new(
        "a".repeat(61),
        OperationKind::Edit {
            specimen: starter(),
            supersedes: None,
        },
    )
    .expect_err("a 61-character request identifier is refused");
    assert_eq!(refused.code, "InvalidRequestId");
    assert!(
        !refused.message.contains(&"a".repeat(61)),
        "the refusal carries no content"
    );

    // The character set is the substrate's own.
    for bad in ["Request_Edit_1", "request edit 1", "request/edit", ""] {
        let refused = CanonicalOperation::new(
            bad,
            OperationKind::Edit {
                specimen: starter(),
                supersedes: None,
            },
        )
        .expect_err("the request identifier is not a bounded identifier");
        assert_eq!(refused.code, "InvalidRequestId");
    }

    // A 60-character request identifier is accepted, and the substrate
    // admits it.
    let temp = tempfile::tempdir().unwrap();
    let mut adapter = CanonicalAdapter::open(temp.path(), CanonicalConfig::default()).unwrap();
    let operation = CanonicalOperation::new(
        "b".repeat(60),
        OperationKind::Edit {
            specimen: starter(),
            supersedes: None,
        },
    )
    .expect("a 60-character request identifier is bounded");
    assert!(adapter.commit(&operation).is_admitted());
}

#[test]
fn the_adapter_writes_to_the_ledger_only_through_the_substrate() {
    // By construction: the adapter names the substrate's write path once and
    // opens no file of its own.
    assert_eq!(
        CANONICAL_SOURCE.matches(".propose_write(").count(),
        1,
        "the adapter appends through one call"
    );
    for forbidden in [
        "fs::write",
        "File::create",
        "OpenOptions",
        "write_all",
        "remove_file",
        "set_len",
        "fs::rename",
    ] {
        assert!(
            !CANONICAL_SOURCE.contains(forbidden),
            "{forbidden} appears in the canonical adapter"
        );
    }

    // And by behaviour: a refused operation leaves the ledger byte-identical.
    let temp = tempfile::tempdir().unwrap();
    let mut adapter = CanonicalAdapter::open(temp.path(), CanonicalConfig::default()).unwrap();
    adapter.commit(&edit("request_edit_1", starter()));
    let ledger = temp.path().join("episodes.v1.jsonl");
    let before = std::fs::read(&ledger).unwrap();

    let refused = CanonicalOperation::new(
        "request_forget_1",
        OperationKind::Forget { target: [9u8; 32] },
    )
    .expect("the request identifier is a bounded identifier");
    assert!(!adapter.decide(&refused).is_admitted());
    assert!(!adapter.commit(&refused).is_admitted());
    assert_eq!(
        std::fs::read(&ledger).unwrap(),
        before,
        "a refused operation leaves the ledger byte-identical"
    );
}

#[test]
fn the_canonical_ledger_lives_beside_the_snapshot_store_and_leaves_it_alone() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("studio");
    let mut store = Store::open(&root).unwrap();
    store.edit(0, starter()).unwrap();
    let revision = store.snapshot.revision;
    let specimen = store.snapshot.specimen.clone();

    let mut adapter = CanonicalAdapter::open_in(&root, CanonicalConfig::default()).unwrap();
    adapter.commit(&edit("request_edit_1", starter()));

    // The two stores keep their own files.
    assert!(root.join("store").is_dir());
    assert!(root.join("episodes/episodes.v1.jsonl").is_file());
    assert_eq!(adapter.directory(), root.join("episodes"));

    // The snapshot store is untouched by the canonical write.
    let reopened = Store::open_read_only(&root).unwrap();
    assert_eq!(reopened.snapshot.revision, revision);
    assert_eq!(reopened.snapshot.specimen, specimen);
    assert!(reopened.snapshot.tombstones.is_empty());
}
