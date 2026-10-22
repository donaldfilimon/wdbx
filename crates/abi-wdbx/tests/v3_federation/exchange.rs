//! The exchange itself: round trip, replay, regression, deduplication, and
//! the determinism of a bundle.

use super::*;

#[test]
fn a_round_trip_keeps_the_origin_authority() {
    let scratch = Scratch::new("round-trip");
    let organizations = organizations();
    let mut store_a = open_store(&scratch.child("store_a"));
    let digest = append_candidate(&mut store_a, "request_1", 1);
    let record = foreign_record(&organizations.org_a, &store_a, &digest);

    let mut trusts_a = TrustStore::new();
    trusts_a
        .grant(trust(
            organizations.peer_b.peer(),
            &[SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    let mut exporter = exporter_a(&scratch, &organizations, trusts_a);
    let bundle = exporter
        .export_bundle(organizations.peer_b.peer(), &[record], 1)
        .expect("export");

    let mut trusts_b = TrustStore::new();
    trusts_b
        .grant(trust(
            organizations.peer_a.peer(),
            &[SCOPE],
            ShareDirection::Import,
        ))
        .expect("grant");
    let mut importer = importer_b(&scratch, &organizations, trusts_b);
    assert_eq!(
        importer.import(&bundle).expect("import"),
        ImportOutcome::Imported {
            applied: 1,
            skipped: 0
        }
    );

    let served = importer
        .served_record(&organizations.org_a, &digest)
        .expect("the record is served");
    assert_eq!(served.origin, organizations.org_a);
    assert_eq!(served.scope, SCOPE);
    assert_eq!(served.episode_digest, digest);

    // The record is not in B's canonical episode ledger, and B has no
    // canonical record of its own in that scope.
    let store_b = open_store(&scratch.child("store_b"));
    assert_eq!(store_b.retrieve(SCOPE, 16).expect("receipts").len(), 0);
    assert!(
        store_b
            .find_receipt(SCOPE, &digest)
            .expect("receipt lookup")
            .is_none()
    );

    // The local reference is an import receipt that names the origin.
    let receipt = importer
        .import_receipt(&organizations.org_a, &digest)
        .expect("receipt");
    assert_eq!(receipt.origin, organizations.org_a);
    assert_eq!(receipt.scope, SCOPE);
    assert_eq!(receipt.checkpoint, 1);
    let evidence = importer
        .foreign_evidence(&organizations.org_a, &digest)
        .expect("foreign reference");
    assert_eq!(evidence.origin_organization, organizations.org_a);
    assert_eq!(evidence.origin_scope, SCOPE);
    assert_eq!(evidence.episode_digest, digest);
    assert_eq!(evidence.import_receipt, receipt);
}

#[test]
fn a_replayed_bundle_is_a_no_op() {
    let scratch = Scratch::new("replay");
    let organizations = organizations();
    let mut store_a = open_store(&scratch.child("store_a"));
    let digest = append_candidate(&mut store_a, "request_1", 1);
    let record = foreign_record(&organizations.org_a, &store_a, &digest);

    let mut trusts_a = TrustStore::new();
    trusts_a
        .grant(trust(
            organizations.peer_b.peer(),
            &[SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    let mut exporter = exporter_a(&scratch, &organizations, trusts_a);
    let bundle = exporter
        .export_bundle(organizations.peer_b.peer(), &[record], 1)
        .expect("export");

    let mut trusts_b = TrustStore::new();
    trusts_b
        .grant(trust(
            organizations.peer_a.peer(),
            &[SCOPE],
            ShareDirection::Import,
        ))
        .expect("grant");
    let mut importer = importer_b(&scratch, &organizations, trusts_b);
    assert_eq!(
        importer.import(&bundle).expect("first import"),
        ImportOutcome::Imported {
            applied: 1,
            skipped: 0
        }
    );
    assert_eq!(
        importer.import(&bundle).expect("replay"),
        ImportOutcome::AlreadyImported
    );
    assert_eq!(importer.served_records(SCOPE).len(), 1);
    assert_eq!(importer.highest_checkpoint(&organizations.org_a), 1);
}

#[test]
fn a_bundle_behind_the_newest_checkpoint_is_refused() {
    let scratch = Scratch::new("regression");
    let organizations = organizations();
    let mut store_a = open_store(&scratch.child("store_a"));
    let first = append_candidate(&mut store_a, "request_1", 1);
    let second = append_candidate(&mut store_a, "request_2", 2);

    let mut trusts_a = TrustStore::new();
    trusts_a
        .grant(trust(
            organizations.peer_b.peer(),
            &[SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    let mut exporter = exporter_a(&scratch, &organizations, trusts_a);
    let newer = exporter
        .export_bundle(
            organizations.peer_b.peer(),
            &[foreign_record(&organizations.org_a, &store_a, &second)],
            5,
        )
        .expect("export");
    let older = exporter
        .export_bundle(
            organizations.peer_b.peer(),
            &[foreign_record(&organizations.org_a, &store_a, &first)],
            3,
        )
        .expect("export");

    let mut trusts_b = TrustStore::new();
    trusts_b
        .grant(trust(
            organizations.peer_a.peer(),
            &[SCOPE],
            ShareDirection::Import,
        ))
        .expect("grant");
    let mut importer = importer_b(&scratch, &organizations, trusts_b);
    importer.import(&newer).expect("import");
    assert_eq!(
        importer
            .import(&older)
            .expect_err("a regression is refused"),
        FederationError::StaleCheckpoint
    );
    assert_eq!(importer.highest_checkpoint(&organizations.org_a), 5);
    assert!(
        importer
            .served_record(&organizations.org_a, &first)
            .is_none()
    );
    assert!(
        importer
            .served_record(&organizations.org_a, &second)
            .is_some()
    );
}

#[test]
fn the_same_record_under_two_bundles_is_imported_once() {
    let scratch = Scratch::new("dedup");
    let organizations = organizations();
    let mut store_a = open_store(&scratch.child("store_a"));
    let first = append_candidate(&mut store_a, "request_1", 1);
    let second = append_candidate(&mut store_a, "request_2", 2);

    let mut trusts_a = TrustStore::new();
    trusts_a
        .grant(trust(
            organizations.peer_b.peer(),
            &[SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    let mut exporter = exporter_a(&scratch, &organizations, trusts_a);
    let one = exporter
        .export_bundle(
            organizations.peer_b.peer(),
            &[foreign_record(&organizations.org_a, &store_a, &first)],
            1,
        )
        .expect("export");
    let two = exporter
        .export_bundle(
            organizations.peer_b.peer(),
            &[
                foreign_record(&organizations.org_a, &store_a, &first),
                foreign_record(&organizations.org_a, &store_a, &second),
            ],
            2,
        )
        .expect("export");

    let mut trusts_b = TrustStore::new();
    trusts_b
        .grant(trust(
            organizations.peer_a.peer(),
            &[SCOPE],
            ShareDirection::Import,
        ))
        .expect("grant");
    let mut importer = importer_b(&scratch, &organizations, trusts_b);
    importer.import(&one).expect("import");
    assert_eq!(
        importer.import(&two).expect("import"),
        ImportOutcome::Imported {
            applied: 1,
            skipped: 1
        }
    );
    assert_eq!(importer.served_records(SCOPE).len(), 2);
    assert_eq!(importer.highest_checkpoint(&organizations.org_a), 2);
}

#[test]
fn an_export_is_deterministic() {
    let scratch = Scratch::new("determinism");
    let organizations = organizations();
    let mut store_a = open_store(&scratch.child("store_a"));
    let first = append_candidate(&mut store_a, "request_1", 1);
    let second = append_candidate(&mut store_a, "request_2", 2);
    let records = vec![
        foreign_record(&organizations.org_a, &store_a, &first),
        foreign_record(&organizations.org_a, &store_a, &second),
    ];

    let mut trusts_a = TrustStore::new();
    trusts_a
        .grant(trust(
            organizations.peer_b.peer(),
            &[SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    let mut exporter = exporter_a(&scratch, &organizations, trusts_a);
    let one = exporter
        .export_bundle(organizations.peer_b.peer(), &records, 1)
        .expect("export");
    let two = exporter
        .export_bundle(organizations.peer_b.peer(), &records, 1)
        .expect("the re-export is verifiable");
    assert_eq!(
        serde_json::to_vec(&one).expect("serialize"),
        serde_json::to_vec(&two).expect("serialize")
    );
    assert_eq!(one.digest(), two.digest());
}

#[test]
fn a_tampered_bundle_does_not_verify() {
    let scratch = Scratch::new("tampered");
    let organizations = organizations();
    let mut store_a = open_store(&scratch.child("store_a"));
    let digest = append_candidate(&mut store_a, "request_1", 1);
    let record = foreign_record(&organizations.org_a, &store_a, &digest);

    let mut trusts_a = TrustStore::new();
    trusts_a
        .grant(trust(
            organizations.peer_b.peer(),
            &[SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    let mut exporter = exporter_a(&scratch, &organizations, trusts_a);
    let bundle = exporter
        .export_bundle(organizations.peer_b.peer(), &[record], 1)
        .expect("export");

    let mut tampered: ExportBundle = bundle.clone();
    tampered.manifest.checkpoint = 2;
    assert_ne!(
        tampered.digest(),
        bundle.digest(),
        "the digest covers the manifest"
    );

    let mut trusts_b = TrustStore::new();
    trusts_b
        .grant(trust(
            organizations.peer_a.peer(),
            &[SCOPE],
            ShareDirection::Import,
        ))
        .expect("grant");
    let mut importer = importer_b(&scratch, &organizations, trusts_b);
    assert_eq!(
        importer
            .import(&tampered)
            .expect_err("a tampered manifest does not verify"),
        FederationError::SignatureInvalid
    );

    // A manifest that does not describe the records it carries is refused
    // before anything is admitted.
    let mut misdescribed: ExportBundle = bundle.clone();
    misdescribed.manifest.record_digests = vec![[0; 32]];
    assert_eq!(
        importer
            .import(&misdescribed)
            .expect_err("the manifest does not describe the records"),
        FederationError::DigestMismatch
    );
    assert!(
        importer
            .served_record(&organizations.org_a, &digest)
            .is_none()
    );
}
