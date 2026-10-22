//! Bilateral trust and the scoped grants that carry it.

use super::*;

#[test]
fn a_peer_without_a_live_bilateral_trust_cannot_import() {
    let scratch = Scratch::new("no-trust");
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

    // B knows A and holds no trust record for it at all.
    let mut importer = importer_b(&scratch, &organizations, TrustStore::new());
    assert_eq!(
        importer.import(&bundle).expect_err("no trust, no import"),
        FederationError::TrustMissing
    );
    assert!(
        importer
            .served_record(&organizations.org_a, &digest)
            .is_none()
    );
    assert_eq!(importer.highest_checkpoint(&organizations.org_a), 0);
}

#[test]
fn a_grant_that_does_not_cover_the_scope_permits_neither_export_nor_import() {
    let scratch = Scratch::new("scope");
    let organizations = organizations();
    let mut store_a = open_store(&scratch.child("store_a"));
    let digest = append_candidate(&mut store_a, "request_1", 1);
    let record = foreign_record(&organizations.org_a, &store_a, &digest);

    // A trusts B for another scope only: the export is refused.
    let mut trusts_a = TrustStore::new();
    trusts_a
        .grant(trust(
            organizations.peer_b.peer(),
            &[OTHER_SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    let mut exporter = exporter_a(&scratch, &organizations, trusts_a);
    assert_eq!(
        exporter
            .export_bundle(
                organizations.peer_b.peer(),
                std::slice::from_ref(&record),
                1,
            )
            .expect_err("the scope is not covered"),
        FederationError::GrantScopeMismatch
    );

    // B trusts A for another scope only: the import is refused.
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
            &[OTHER_SCOPE],
            ShareDirection::Import,
        ))
        .expect("grant");
    let mut importer = importer_b(&scratch, &organizations, trusts_b);
    assert_eq!(
        importer
            .import(&bundle)
            .expect_err("the scope is not covered"),
        FederationError::GrantScopeMismatch
    );
    assert!(
        importer
            .served_record(&organizations.org_a, &digest)
            .is_none()
    );
}

#[test]
fn a_grant_that_permits_only_one_direction_permits_only_that_direction() {
    let scratch = Scratch::new("direction");
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

    // B trusts A for export only: an import is not covered.
    let mut trusts_b = TrustStore::new();
    trusts_b
        .grant(trust(
            organizations.peer_a.peer(),
            &[SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    let mut importer = importer_b(&scratch, &organizations, trusts_b);
    assert_eq!(
        importer
            .import(&bundle)
            .expect_err("the direction is not covered"),
        FederationError::GrantDirectionMismatch
    );
}

#[test]
fn a_residency_restriction_is_enforced_against_both_ends() {
    let scratch = Scratch::new("residency");
    let organizations = organizations();
    let mut store_a = open_store(&scratch.child("store_a"));
    let digest = append_candidate(&mut store_a, "request_1", 1);
    let record = foreign_record(&organizations.org_a, &store_a, &digest);

    let mut trusts_a = TrustStore::new();
    trusts_a
        .grant(BilateralTrust::new(
            organizations.peer_b.peer().clone(),
            grant_with(&[SCOPE], ShareDirection::Both, ResidencyClass::SameRegion),
            1,
        ))
        .expect("grant");
    let mut exporter = exporter_a(&scratch, &organizations, trusts_a);
    // Both peers are unrestricted, so neither satisfies a same-region rule.
    assert_eq!(
        exporter
            .export_bundle(organizations.peer_b.peer(), &[record], 1)
            .expect_err("the residency class does not satisfy the grant"),
        FederationError::ResidencyMismatch
    );
}

#[test]
fn a_revoked_trust_stops_authorizing_the_next_import() {
    let scratch = Scratch::new("revoked");
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
            &[foreign_record(&organizations.org_a, &store_a, &second)],
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

    let mut revoked = TrustStore::new();
    revoked
        .grant(trust(
            organizations.peer_a.peer(),
            &[SCOPE],
            ShareDirection::Import,
        ))
        .expect("grant");
    revoked.revoke(organizations.peer_a.peer()).expect("revoke");
    let mut importer = importer_b(&scratch, &organizations, revoked);
    assert_eq!(
        importer
            .import(&two)
            .expect_err("a revoked trust authorizes nothing"),
        FederationError::TrustRevoked
    );
    assert!(
        importer
            .served_record(&organizations.org_a, &second)
            .is_none()
    );
}

#[test]
fn an_unknown_peer_cannot_sign_a_bundle_this_organization_accepts() {
    let scratch = Scratch::new("unknown-peer");
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

    // B's directory does not know A at all.
    let mut importer = FederationImporter::open(
        scratch.child("importer_b"),
        organizations.peer_b.clone(),
        PeerDirectory::new(),
        TrustStore::new(),
    )
    .expect("importer");
    assert_eq!(
        importer.import(&bundle).expect_err("the peer is unknown"),
        FederationError::UnknownPeer
    );
}
