//! The studio's write-through to the canonical episode ledger.
//!
//! These tests pin the wiring between the studio's own store and the
//! substrate's canonical ledger:
//!
//! * an edit with the adapter present records an episode and leaves the
//!   studio's content alone;
//! * a refused decision leaves the ledger byte-identical and the studio's
//!   canonical state unchanged, and the reason is the substrate's own;
//! * the studio's document is a projection: a rebuild from the ledger
//!   reproduces it, and a missing content blob is a declared gap;
//! * the studio's erasure list and the ledger's forget candidates agree;
//! * the store without the adapter is unchanged.

use abi_wdbx::v3::episode::AdmissionReason;
use serde_json::{Value, json};
use specimen_core::canonical::{
    CanonicalConfig, Digest, JournalKind, LossId, LossManifest, RebuildGap,
    canonical_content_bytes, content_commitment,
};
use specimen_core::persistence::Store;

/// The write-through's own source, for the structural claims.
const WRITE_THROUGH_SOURCE: &str = include_str!("../src/persistence/canonical.rs");

fn starter() -> Value {
    serde_json::from_str(include_str!("../../../conformance/starter.json")).unwrap()
}

/// The ledger's own file, beside the studio's store.
fn ledger_file(root: &std::path::Path) -> std::path::PathBuf {
    root.join("episodes").join("episodes.v1.jsonl")
}

/// The studio's own blob for one canonical commitment.
fn content_file(root: &std::path::Path, commitment: &Digest) -> std::path::PathBuf {
    root.join("assets").join(commitment.to_hex())
}

#[test]
fn an_edit_records_an_episode_and_leaves_the_studio_content_alone() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("studio");
    let mut store = Store::open_with_canonical(&root, CanonicalConfig::default()).unwrap();
    let specimen = starter();

    let snapshot = store.edit(0, specimen.clone()).unwrap();

    // The studio's own content is what it was.
    assert_eq!(snapshot.specimen, specimen);
    assert_eq!(store.snapshot.revision, 1);

    // The ledger holds the episode.
    let ledger = ledger_file(&root);
    assert!(ledger.is_file(), "the studio wrote no canonical episode");
    assert!(!std::fs::read(&ledger).unwrap().is_empty());

    // The studio recorded the substrate's own content-free receipt.
    let report = store.canonical_report().expect("a canonical report");
    assert!(report.is_admitted(), "the substrate refused the edit");
    assert_eq!(report.operations.len(), 1);
    let operation = &report.operations[0];
    assert_eq!(operation.kind, JournalKind::Edit);
    let receipt = operation
        .decision
        .receipt()
        .expect("the substrate returned a receipt");
    assert_eq!(receipt.sequence, 1);
    assert!(receipt.redacted, "the receipt is content-free");
    assert_eq!(receipt.guild_ref, CanonicalConfig::default().scope);

    // The studio's journal maps the episode to the content it committed to.
    let journal = store.journal();
    assert_eq!(journal.entries.len(), 1);
    let entry = &journal.entries[0];
    assert_eq!(entry.episode, Digest::from_bytes(receipt.episode_digest));
    assert_eq!(entry.sequence, receipt.sequence);
    assert_eq!(entry.kind, JournalKind::Edit);
    let commitment = content_commitment(&canonical_content_bytes(&specimen));
    assert_eq!(entry.content, Some(Digest::from_bytes(commitment)));
    assert!(content_file(&root, &Digest::from_bytes(commitment)).is_file());
}

#[test]
fn a_refused_decision_leaves_the_ledger_byte_identical() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("studio");
    let config = CanonicalConfig {
        learning_enabled: false,
        ..CanonicalConfig::default()
    };
    let mut store = Store::open_with_canonical(&root, config).unwrap();
    let ledger = ledger_file(&root);
    let before = std::fs::read(&ledger).unwrap();

    store.edit(0, starter()).unwrap();

    // The ledger is byte-identical.
    assert_eq!(std::fs::read(&ledger).unwrap(), before);
    // The studio's canonical state is unchanged.
    assert!(store.journal().entries.is_empty());
    // The reason is the substrate's own, content-free one.
    let report = store.canonical_report().expect("a canonical report");
    assert!(!report.is_admitted());
    assert_eq!(
        report.operations[0].decision.substrate_reason(),
        Some(AdmissionReason::LearningDisabled)
    );
    assert_eq!(
        report.operations[0]
            .decision
            .substrate_reason()
            .map(|reason| reason.label()),
        Some("learning_disabled")
    );
    // The studio's own behaviour is kept: the edit still landed.
    assert_eq!(store.snapshot.revision, 1);
}

#[test]
fn a_rebuild_from_the_ledger_reproduces_the_studio_document() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("studio");
    let mut store = Store::open_with_canonical(&root, CanonicalConfig::default()).unwrap();

    // The first state.
    let first = starter();
    store.edit(0, first.clone()).unwrap();

    // A correction: a second state that supersedes the first.
    let mut second = first.clone();
    second["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"ref":"added-node","name":"Added","patternId":"low:added","resolution":"low","type":"pattern","contextId":"","strength":5,"jitter":true,"tone":"neutral","entries":[{"id":"added-entry","pattern":"added pattern","alternatives":[{"id":"added-alternative","action":"Acknowledge.","inhibition":"","weight":1,"remixed":false}]}],"createdAt":"2026-10-21T00:00:00.000Z"}));
    store.edit(1, second.clone()).unwrap();

    // An erasure: the second state loses a node.
    let mut third = second.clone();
    third["nodes"].as_array_mut().unwrap().remove(0);
    store.edit(2, third.clone()).unwrap();

    // The rebuild reproduces the studio's document and its erasure list.
    let report = store.rebuild_from_ledger().unwrap();
    assert!(
        report.is_complete(),
        "the rebuild found gaps: {:?}",
        report.gaps
    );
    assert_eq!(report.document, Some(third.clone()));
    assert_eq!(report.tombstones, store.snapshot.tombstones);
    assert_eq!(report.tombstones.len(), 1);
    assert_eq!(
        report.tombstones[0]["node"]["ref"],
        first["nodes"][0]["ref"]
    );
    assert!(report.tombstones[0]["deletedAt"].is_string());

    // And the rebuild agrees with the studio's own snapshot.
    assert_eq!(report.document, Some(store.snapshot.specimen.clone()));
}

#[test]
fn a_missing_content_blob_is_a_declared_gap() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("studio");
    let mut store = Store::open_with_canonical(&root, CanonicalConfig::default()).unwrap();
    store.edit(0, starter()).unwrap();

    // The studio loses the content the ledger committed to.
    let commitment = store.journal().entries[0]
        .content
        .expect("the episode committed to content");
    std::fs::remove_file(content_file(&root, &commitment)).unwrap();

    let report = store.rebuild_from_ledger().unwrap();
    assert!(!report.is_complete());
    assert_eq!(report.gaps.len(), 1);
    assert_eq!(
        report.gaps[0],
        RebuildGap::MissingContent {
            episode: store.journal().entries[0].episode,
            content: commitment,
        }
    );
    // The rebuild never produces a partial document.
    assert_eq!(report.document, None);
}

#[test]
fn the_studio_erasure_list_and_the_ledger_forget_candidates_agree() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("studio");
    let mut store = Store::open_with_canonical(&root, CanonicalConfig::default()).unwrap();
    let first = starter();
    store.edit(0, first.clone()).unwrap();

    let mut second = first.clone();
    second["nodes"].as_array_mut().unwrap().remove(0);
    store.edit(1, second.clone()).unwrap();

    let agreement = store.forget_agreement().unwrap();
    assert!(
        agreement.agrees(),
        "the two records disagree: {agreement:?}"
    );
    assert_eq!(agreement.ledger_tombstones.len(), 1);
    assert_eq!(agreement.studio_forgets.len(), 1);
    assert_eq!(agreement.ledger_tombstones, agreement.studio_forgets);
    assert_eq!(agreement.studio_tombstones, 1);
    assert_eq!(agreement.journal_tombstones, 1);
    assert!(agreement.unknown.is_empty());
    assert!(agreement.missing.is_empty());
    assert!(agreement.unerased.is_empty());

    // The ledger itself reports the erased state as forgotten.
    let projection = store
        .canonical()
        .expect("the studio has a ledger")
        .full_projection()
        .unwrap();
    let forgotten: Vec<_> = projection
        .episodes
        .iter()
        .filter(|episode| episode.dimensions.lifecycle.forgotten)
        .collect();
    assert_eq!(forgotten.len(), 1);
    assert_eq!(
        forgotten[0].episode_digest,
        *store.journal().entries[0].episode.as_bytes()
    );
}

#[test]
fn a_store_without_the_adapter_is_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("studio");
    let mut store = Store::open(&root).unwrap();
    let first = starter();
    store.edit(0, first.clone()).unwrap();

    let mut second = first.clone();
    let removed = second["nodes"].as_array_mut().unwrap().remove(0);
    store.edit(1, second.clone()).unwrap();

    // The studio keeps its own behaviour: the tombstone row is still there,
    // and the studio wrote no canonical ledger at all.
    assert_eq!(store.snapshot.tombstones.len(), 1);
    assert_eq!(store.snapshot.tombstones[0]["node"], removed);
    assert!(store.canonical().is_none());
    assert!(store.canonical_report().is_none());
    assert!(store.journal().entries.is_empty());
    assert!(!ledger_file(&root).exists());
    assert!(store.rebuild_from_ledger().is_err());
    assert!(store.forget_agreement().is_err());
}

#[test]
fn a_learned_proposal_becomes_a_candidate_and_a_dismissal_becomes_a_forget() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("studio");
    let mut store = Store::open_with_canonical(&root, CanonicalConfig::default()).unwrap();

    let mut first = starter();
    first["proposals"] = json!([{"id":"p1","pattern":"hello there","evidence":3,"status":"pending","pros":[],"cons":[]}]);
    store.edit(0, first.clone()).unwrap();

    // The proposal is learned: the studio records it as a candidate.
    let mut second = first.clone();
    second["proposals"][0]["status"] = json!("learned");
    store.edit(1, second.clone()).unwrap();

    let learn = store
        .journal()
        .learns()
        .next()
        .expect("the studio recorded the learning")
        .episode;
    assert!(learn.as_bytes().iter().any(|byte| *byte != 0));
    let learn_proposal = store
        .journal()
        .learns()
        .next()
        .expect("the studio recorded the learning")
        .proposal
        .clone();
    assert_eq!(learn_proposal.as_deref(), Some("p1"));

    // The proposal is dismissed: the candidate it named is erased.
    let mut third = second.clone();
    third["proposals"][0]["status"] = json!("dismissed");
    store.edit(2, third.clone()).unwrap();

    let agreement = store.forget_agreement().unwrap();
    assert!(
        agreement.agrees(),
        "the two records disagree: {agreement:?}"
    );
    assert_eq!(agreement.ledger_tombstones.len(), 1);
    assert_eq!(agreement.studio_forgets.len(), 1);
    assert!(agreement.unerased.is_empty());

    // The ledger reports the learning candidate as forgotten.
    let projection = store
        .canonical()
        .expect("the studio has a ledger")
        .full_projection()
        .unwrap();
    let forgotten: Vec<_> = projection
        .episodes
        .iter()
        .filter(|episode| episode.dimensions.lifecycle.forgotten)
        .collect();
    assert_eq!(forgotten.len(), 1);
    assert_eq!(forgotten[0].episode_digest, *learn.as_bytes());
}

#[test]
fn the_studio_document_never_drifts_from_the_ledger_without_a_reason() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("studio");
    let mut store = Store::open_with_canonical(&root, CanonicalConfig::default()).unwrap();
    store.edit(0, starter()).unwrap();
    assert!(
        !store.canonical_drift().unwrap(),
        "the studio's document and the ledger agree"
    );

    // A refused canonical write leaves the studio's own snapshot ahead of
    // the ledger, and the studio says so.
    let mut refused = Store::open_with_canonical(
        temp.path().join("refused"),
        CanonicalConfig {
            learning_enabled: false,
            ..CanonicalConfig::default()
        },
    )
    .unwrap();
    refused.edit(0, starter()).unwrap();
    assert!(
        refused.canonical_drift().unwrap(),
        "the studio names the divergence"
    );
}

#[test]
fn the_write_through_never_appends_to_the_ledger_itself() {
    // The write-through asks the adapter, and the adapter calls the
    // substrate's one write path. Nothing else touches the ledger.
    assert!(!WRITE_THROUGH_SOURCE.contains(".propose_write("));
    for forbidden in [
        "episodes.v1",
        "fs::write",
        "File::create",
        "OpenOptions",
        "write_all",
        "remove_file",
        "set_len",
        "fs::rename",
    ] {
        assert!(
            !WRITE_THROUGH_SOURCE.contains(forbidden),
            "{forbidden} appears in the write-through"
        );
    }
}

#[test]
fn the_loss_manifest_names_the_new_declarations() {
    let manifest = LossManifest::native();
    assert!(!manifest.declarations.is_empty());
    for id in [
        LossId::ContentResolution,
        LossId::MissingContent,
        LossId::DeletionFacts,
        LossId::UnsignedEpisodes,
    ] {
        let declaration = manifest
            .declaration(id)
            .unwrap_or_else(|| panic!("the native manifest declares {id:?}"));
        assert!(!declaration.dropped.is_empty());
        assert!(!declaration.reason.is_empty());
    }

    // The unsigned store is declared, so the cryptographic dimension reads
    // Unsigned rather than pretending a signature exists.
    let signature = manifest
        .declaration(LossId::SignatureEvidence)
        .expect("the native manifest declares the signature evidence");
    assert!(signature.reason.contains("Unsigned"));

    // The browser edition still declares what it drops, and the native
    // manifest still names exactly that.
    let browser = LossManifest::browser().unwrap();
    assert!(!browser.declarations.is_empty());
}
