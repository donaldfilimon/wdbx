//! Deletion propagation and the isolation of one organization's records.

use super::*;

#[test]
fn a_deletion_at_the_origin_propagates_and_a_stale_watermark_cannot_resurrect_it() {
    let scratch = Scratch::new("deletion");
    let organizations = organizations();
    let mut store_a = open_store(&scratch.child("store_a"));
    let digest = append_candidate(&mut store_a, "request_1", 1);

    let mut trusts_a = TrustStore::new();
    trusts_a
        .grant(trust(
            organizations.peer_b.peer(),
            &[SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    let mut exporter = exporter_a(&scratch, &organizations, trusts_a);
    let before = exporter
        .export_bundle(
            organizations.peer_b.peer(),
            &[foreign_record(&organizations.org_a, &store_a, &digest)],
            1,
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
    importer.import(&before).expect("import");
    assert!(
        importer
            .served_record(&organizations.org_a, &digest)
            .is_some()
    );

    // The origin forgets the record and the deletion enters its log.
    append_forgetting(&mut store_a, "request_2", digest);
    assert_eq!(exporter.record_deletion(&digest).expect("deletion"), 1);
    let after = exporter
        .export_bundle(
            organizations.peer_b.peer(),
            &[foreign_record(&organizations.org_a, &store_a, &digest)],
            2,
        )
        .expect("export");
    importer.import(&after).expect("import");
    assert!(
        importer
            .served_record(&organizations.org_a, &digest)
            .is_none(),
        "the deleted record is no longer served"
    );
    assert!(
        importer
            .record_state(&organizations.org_a, &digest)
            .expect("state")
            .forgotten
    );
    assert_eq!(importer.applied_deletion_watermark(&organizations.org_a), 1);

    // A stale origin that still carries an older watermark is refused, and
    // the deleted record stays deleted.
    let mut stale_trusts = TrustStore::new();
    stale_trusts
        .grant(trust(
            organizations.peer_b.peer(),
            &[SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    let mut peers = PeerDirectory::new();
    peers.enroll(organizations.peer_b.clone()).expect("enroll");
    let mut stale = FederationExporter::open(
        scratch.child("exporter_stale"),
        organizations.peer_a.clone(),
        organizations.key_a(),
        peers,
        stale_trusts,
    )
    .expect("exporter");
    let stale_bundle = stale
        .export_bundle(
            organizations.peer_b.peer(),
            &[foreign_record(&organizations.org_a, &store_a, &digest)],
            3,
        )
        .expect("export");
    assert_eq!(
        importer
            .import(&stale_bundle)
            .expect_err("a stale watermark is refused"),
        FederationError::StaleWatermark
    );
    assert!(
        importer
            .served_record(&organizations.org_a, &digest)
            .is_none()
    );
}

#[test]
fn one_organizations_records_never_enter_another_organizations_ledger() {
    let scratch = Scratch::new("isolation");
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
    importer.import(&bundle).expect("import");

    // B's canonical ledger is empty: the foreign record is not a B record.
    let mut store_b = open_store(&scratch.child("store_b"));
    assert_eq!(store_b.retrieve(SCOPE, 16).expect("receipts").len(), 0);
    assert_eq!(
        store_b.retrieve(OTHER_SCOPE, 16).expect("receipts").len(),
        0
    );

    // B writes its own record: it is B's, and it does not enter A's ledger
    // or A's foreign namespace.
    let own = append_candidate(&mut store_b, "request_b", 9);
    assert!(
        store_b
            .find_receipt(SCOPE, &own)
            .expect("receipt")
            .is_some()
    );
    assert!(
        store_b
            .find_receipt(SCOPE, &digest)
            .expect("receipt")
            .is_none()
    );

    // The foreign namespace holds A's record under A's organization.
    let served = importer
        .served_records(SCOPE)
        .into_iter()
        .map(|record| record.origin.clone())
        .collect::<Vec<_>>();
    assert_eq!(served, vec![organizations.org_a.clone()]);
}
