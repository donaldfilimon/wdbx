//! Reconciliation: equivocation, ancestry, and a resumable import.

use super::*;

/// The first host of organization A.
fn first_host(scratch: &Scratch, organizations: &Organizations) -> FederationExporter {
    let mut peers = PeerDirectory::new();
    peers.enroll(organizations.peer_b.clone()).expect("enroll");
    let mut trusts = TrustStore::new();
    trusts
        .grant(trust(
            organizations.peer_b.peer(),
            &[SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    FederationExporter::open(
        scratch.child("exporter_a1"),
        organizations.peer_a.clone(),
        organizations.key_a(),
        peers,
        trusts,
    )
    .expect("exporter")
}

/// A second host of organization A: its own key, its own directory, and its
/// own export bindings.
fn second_host(
    scratch: &Scratch,
    organizations: &Organizations,
) -> (PeerIdentity, FederationExporter) {
    let (_, peer_a2) = peer_with(3, &organizations.org_a, ResidencyClass::Unrestricted);
    let mut peers = PeerDirectory::new();
    peers.enroll(organizations.peer_b.clone()).expect("enroll");
    let mut trusts = TrustStore::new();
    trusts
        .grant(trust(
            organizations.peer_b.peer(),
            &[SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    let exporter = FederationExporter::open(
        scratch.child("exporter_a2"),
        peer_a2.clone(),
        PeerKey::from_seed([3; 32]),
        peers,
        trusts,
    )
    .expect("exporter");
    (peer_a2, exporter)
}

#[test]
fn two_bundles_at_one_checkpoint_are_refused_as_an_equivocation() {
    let scratch = Scratch::new("equivocation");
    let organizations = organizations();
    let mut store_a = open_store(&scratch.child("store_a"));
    let first = append_candidate(&mut store_a, "request_1", 1);
    let second = append_candidate(&mut store_a, "request_2", 2);

    // Two hosts of organization A, each with its own key and its own state.
    let (peer_a2, mut exporter_a2) = second_host(&scratch, &organizations);
    let mut exporter_a1 = first_host(&scratch, &organizations);

    let bundle_one = exporter_a1
        .export_bundle(
            organizations.peer_b.peer(),
            &[foreign_record(&organizations.org_a, &store_a, &first)],
            1,
        )
        .expect("export");
    let bundle_two = exporter_a2
        .export_bundle(
            organizations.peer_b.peer(),
            &[foreign_record(&organizations.org_a, &store_a, &second)],
            1,
        )
        .expect("export");

    let mut peers_b = PeerDirectory::new();
    peers_b
        .enroll(organizations.peer_a.clone())
        .expect("enroll");
    peers_b.enroll(peer_a2.clone()).expect("enroll");
    let mut trusts_b = TrustStore::new();
    trusts_b
        .grant(trust(
            organizations.peer_a.peer(),
            &[SCOPE],
            ShareDirection::Import,
        ))
        .expect("grant");
    trusts_b
        .grant(trust(peer_a2.peer(), &[SCOPE], ShareDirection::Import))
        .expect("grant");
    let mut importer = FederationImporter::open(
        scratch.child("importer_b"),
        organizations.peer_b.clone(),
        peers_b,
        trusts_b,
    )
    .expect("importer");

    importer.import(&bundle_one).expect("import");
    assert_eq!(
        importer
            .import(&bundle_two)
            .expect_err("the second bundle at one checkpoint is refused"),
        FederationError::Equivocation
    );
    let conflicts = importer.equivocation_conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].origin, organizations.org_a);
    assert_eq!(conflicts[0].checkpoint, 1);
    assert_ne!(conflicts[0].recorded_digest, conflicts[0].presented_digest);

    // The disputed checkpoint is withheld from the serving view.
    assert!(
        importer
            .served_record(&organizations.org_a, &first)
            .is_none()
    );
    assert!(
        importer
            .served_record(&organizations.org_a, &second)
            .is_none()
    );
    assert!(
        importer
            .record_state(&organizations.org_a, &first)
            .expect("state")
            .conflicted
    );
}

#[test]
fn an_incomplete_ancestry_is_refused_and_a_delivered_parent_is_kept() {
    let scratch = Scratch::new("ancestry");
    let organizations = organizations();
    let mut store_a = open_store(&scratch.child("store_a"));
    let parent = append_candidate(&mut store_a, "request_1", 1);
    let child = append_superseding(&mut store_a, "request_2", 2, parent);

    let mut trusts_a = TrustStore::new();
    trusts_a
        .grant(trust(
            organizations.peer_b.peer(),
            &[SCOPE],
            ShareDirection::Export,
        ))
        .expect("grant");
    let mut exporter = exporter_a(&scratch, &organizations, trusts_a);
    let child_only = exporter
        .export_bundle(
            organizations.peer_b.peer(),
            &[foreign_record(&organizations.org_a, &store_a, &child)],
            1,
        )
        .expect("export");
    let with_parent = exporter
        .export_bundle(
            organizations.peer_b.peer(),
            &[
                foreign_record(&organizations.org_a, &store_a, &child),
                foreign_record(&organizations.org_a, &store_a, &parent),
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
    assert_eq!(
        importer
            .import(&child_only)
            .expect_err("the parent was never sent"),
        FederationError::AncestryIncomplete
    );
    assert!(
        importer
            .served_record(&organizations.org_a, &child)
            .is_none()
    );

    importer.import(&with_parent).expect("import");
    assert_eq!(
        importer.ancestry(&organizations.org_a, &child),
        Some(AncestryRef {
            origin: organizations.org_a.clone(),
            episode_digest: parent,
        })
    );
    assert_eq!(
        importer.ancestry(&organizations.org_a, &parent),
        None,
        "the parent itself has no parent"
    );
}

#[test]
fn an_interrupted_import_resumes_without_duplicating_or_losing() {
    let scratch = Scratch::new("resume");
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
    let bundle = exporter
        .export_bundle(
            organizations.peer_b.peer(),
            &[
                foreign_record(&organizations.org_a, &store_a, &first),
                foreign_record(&organizations.org_a, &store_a, &second),
            ],
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
    {
        let mut importer = importer_b(&scratch, &organizations, trusts_b.clone());
        assert_eq!(
            importer
                .import_limited(&bundle, 1)
                .expect("the first record is admitted"),
            ImportOutcome::Interrupted {
                applied: 1,
                skipped: 0,
                remaining: 1,
            }
        );
        assert_eq!(importer.served_records(SCOPE).len(), 1);
        assert_eq!(
            importer.highest_checkpoint(&organizations.org_a),
            0,
            "an incomplete bundle does not advance the checkpoint"
        );
    }

    // The state survives the restart, and the import resumes.
    let mut importer = importer_b(&scratch, &organizations, trusts_b);
    assert_eq!(
        importer.import(&bundle).expect("the import resumes"),
        ImportOutcome::Imported {
            applied: 1,
            skipped: 1,
        }
    );
    assert_eq!(importer.served_records(SCOPE).len(), 2);
    assert_eq!(importer.highest_checkpoint(&organizations.org_a), 1);
}
