//! Crate-internal tests for the federation slice.
//!
//! The integration suite (`crates/abi-wdbx/tests/v3_federation.rs`) pins the
//! behaviour a host can observe. These tests pin the parts that are easier to
//! assert from inside the crate: the canonical encoding's determinism, the
//! content-freeness of the error vocabulary, and the derivation of peer
//! identities.

use super::*;
use crate::v3::federation::bundle::BUNDLE_PROFILE;
use std::collections::BTreeSet;

fn organization(value: &str) -> OrganizationId {
    OrganizationId::new(value).expect("bounded organization")
}

fn peer(seed: u8, org: &OrganizationId, residency: ResidencyClass) -> (PeerKey, PeerIdentity) {
    let key = PeerKey::from_seed([seed; 32]);
    let identity = PeerIdentity::new(key.verifying(), org.clone(), residency);
    (key, identity)
}

fn grant(scopes: &[&str]) -> ShareGrant {
    ShareGrant::new(
        scopes.iter().map(|scope| (*scope).to_string()).collect(),
        BTreeSet::from([EvidenceClass::MemoryCandidate]),
        ShareDirection::Both,
        ResidencyClass::Unrestricted,
    )
    .expect("bounded grant")
}

#[test]
fn a_peer_identity_is_derived_from_its_key() {
    let (key, identity) = peer(1, &organization("org_a"), ResidencyClass::SameRegion);
    assert_eq!(identity.peer(), &key.peer_id());
    assert!(identity.peer().as_str().starts_with(PEER_ID_PREFIX));
    assert_eq!(
        PeerId::parse(identity.peer().as_str()),
        Some(identity.peer().clone())
    );
    let (other, _) = peer(2, &organization("org_a"), ResidencyClass::SameRegion);
    assert_ne!(other.peer_id(), *identity.peer());
}

#[test]
fn a_record_digest_is_deterministic_and_binds_every_field() {
    let digest = [3_u8; 32];
    let record = ForeignRecord::new(
        organization("org_a"),
        "guild_ref",
        digest,
        None,
        "memory_candidate",
        EvidenceClass::MemoryCandidate,
    )
    .expect("bounded record");
    let again = ForeignRecord::new(
        organization("org_a"),
        "guild_ref",
        digest,
        None,
        "memory_candidate",
        EvidenceClass::MemoryCandidate,
    )
    .expect("bounded record");
    assert_eq!(record.digest(), again.digest());

    let parented = ForeignRecord::new(
        organization("org_a"),
        "guild_ref",
        digest,
        Some(AncestryRef {
            origin: organization("org_a"),
            episode_digest: [4_u8; 32],
        }),
        "memory_candidate",
        EvidenceClass::MemoryCandidate,
    )
    .expect("bounded record");
    assert_ne!(record.digest(), parented.digest());

    let other_scope = ForeignRecord::new(
        organization("org_a"),
        "other_guild",
        digest,
        None,
        "memory_candidate",
        EvidenceClass::MemoryCandidate,
    )
    .expect("bounded record");
    assert_ne!(record.digest(), other_scope.digest());
}

#[test]
fn a_residency_class_is_ordered() {
    assert!(ResidencyClass::OnPremise.satisfies(ResidencyClass::SameRegion));
    assert!(ResidencyClass::SameRegion.satisfies(ResidencyClass::Unrestricted));
    assert!(!ResidencyClass::Unrestricted.satisfies(ResidencyClass::SameRegion));
    assert!(!ResidencyClass::SameRegion.satisfies(ResidencyClass::OnPremise));
}

#[test]
fn a_grant_covers_only_what_it_names() {
    let grant = grant(&["guild_ref"]);
    assert!(grant.permits_scope("guild_ref"));
    assert!(!grant.permits_scope("other_guild"));
    assert!(grant.permits_evidence(EvidenceClass::MemoryCandidate));
    assert!(!grant.permits_evidence(EvidenceClass::MemoryEdge));
    assert!(grant.permits_direction(ShareDirection::Export));
    assert!(grant.permits_direction(ShareDirection::Import));
}

#[test]
fn a_revoked_trust_record_authorizes_nothing() {
    let (_, identity) = peer(1, &organization("org_a"), ResidencyClass::SameRegion);
    let mut trusts = TrustStore::new();
    trusts
        .grant(BilateralTrust::new(
            identity.peer().clone(),
            grant(&["guild_ref"]),
            1,
        ))
        .expect("grant");
    assert!(trusts.live(identity.peer()).is_some());
    trusts.revoke(identity.peer()).expect("revoke");
    assert!(trusts.live(identity.peer()).is_none());
    assert!(trusts.is_revoked(identity.peer()));
    assert!(trusts.get(identity.peer()).is_some());
}

#[test]
fn an_error_never_carries_peer_supplied_content() {
    let errors = [
        FederationError::Io,
        FederationError::Corrupt,
        FederationError::InvalidInput,
        FederationError::UnknownPeer,
        FederationError::TrustMissing,
        FederationError::TrustRevoked,
        FederationError::GrantDirectionMismatch,
        FederationError::GrantScopeMismatch,
        FederationError::GrantEvidenceMismatch,
        FederationError::ResidencyMismatch,
        FederationError::SignatureInvalid,
        FederationError::ExporterMismatch,
        FederationError::OriginMismatch,
        FederationError::DigestMismatch,
        FederationError::ScopeMismatch,
        FederationError::StaleCheckpoint,
        FederationError::Equivocation,
        FederationError::AncestryIncomplete,
        FederationError::StaleWatermark,
        FederationError::DeletionGap,
        FederationError::BudgetExhausted,
    ];
    for error in errors {
        let rendered = error.to_string();
        assert!(rendered.starts_with("federation_"), "{rendered}");
        assert!(!rendered.contains('/'), "{rendered}");
        assert_eq!(rendered, rendered.to_lowercase(), "{rendered}");
    }
}

#[test]
fn a_bundle_manifest_digest_binds_the_checkpoint_and_the_deletions() {
    let manifest = BundleManifest {
        origin: organization("org_a"),
        exporter: PeerId::parse(
            "peer:0000000000000000000000000000000000000000000000000000000000000000",
        )
        .expect("bounded peer"),
        scope: "guild_ref".to_string(),
        checkpoint: 7,
        deletion_watermark: 2,
        record_digests: vec![[1_u8; 32], [2_u8; 32]],
        deletions: Vec::new(),
    };
    let digest = manifest.digest();
    let mut other = manifest.clone();
    other.checkpoint = 8;
    assert_ne!(digest, other.digest());
    let mut third = manifest;
    third.deletion_watermark = 3;
    assert_ne!(digest, third.digest());
}

#[test]
fn the_bundle_profile_is_explicit() {
    assert_eq!(BUNDLE_PROFILE, "wdbx-federation-bundle-v1");
}

/// A disposable directory for one test.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new() -> Self {
        let base = std::env::temp_dir()
            .canonicalize()
            .expect("temporary directory");
        let path = base.join(format!("wdbx-federation-unit-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).expect("scratch directory");
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_deletion_watermark_that_skips_a_deletion_is_refused() {
    let scratch = Scratch::new();
    let org_a = organization("org_a");
    let org_b = organization("org_b");
    let key_a = PeerKey::from_seed([5; 32]);
    let signing = PeerKey::from_seed([5; 32]);
    let peer_a = PeerIdentity::new(
        key_a.verifying(),
        org_a.clone(),
        ResidencyClass::Unrestricted,
    );
    let (_, peer_b) = peer(2, &org_b, ResidencyClass::Unrestricted);

    let mut peers_a = PeerDirectory::new();
    peers_a.enroll(peer_b.clone()).expect("enroll");
    let mut trusts_a = TrustStore::new();
    trusts_a
        .grant(BilateralTrust::new(
            peer_b.peer().clone(),
            grant(&["guild_ref"]),
            1,
        ))
        .expect("grant");
    let mut exporter = FederationExporter::open(
        scratch.0.join("exporter"),
        peer_a.clone(),
        key_a,
        peers_a,
        trusts_a,
    )
    .expect("exporter");

    let record = ForeignRecord::new(
        org_a.clone(),
        "guild_ref",
        [7_u8; 32],
        None,
        "memory_candidate",
        EvidenceClass::MemoryCandidate,
    )
    .expect("bounded record");
    assert_eq!(exporter.record_deletion(&[1_u8; 32]).expect("deletion"), 1);
    assert_eq!(exporter.record_deletion(&[2_u8; 32]).expect("deletion"), 2);
    let bundle = exporter
        .export_bundle(peer_b.peer(), &[record], 1)
        .expect("export");
    assert_eq!(bundle.manifest.deletion_watermark, 2);
    assert_eq!(bundle.manifest.deletions.len(), 2);

    // A bundle that claims the watermark of two deletions but carries only
    // the second one is refused: applying the watermark would mark the
    // unseen deletion as applied.
    let mut gapped = bundle.clone();
    gapped.manifest.deletions.remove(0);
    gapped.signature.signature = signing.sign(&gapped.digest());

    let mut peers_b = PeerDirectory::new();
    peers_b.enroll(peer_a.clone()).expect("enroll");
    let mut trusts_b = TrustStore::new();
    trusts_b
        .grant(BilateralTrust::new(
            peer_a.peer().clone(),
            grant(&["guild_ref"]),
            1,
        ))
        .expect("grant");
    let mut importer = FederationImporter::open(
        scratch.0.join("importer"),
        peer_b.clone(),
        peers_b,
        trusts_b,
    )
    .expect("importer");
    assert_eq!(
        importer.import(&gapped).expect_err("the gap is refused"),
        FederationError::DeletionGap
    );
    assert_eq!(importer.applied_deletion_watermark(&org_a), 0);

    // The complete bundle is admitted.
    importer.import(&bundle).expect("import");
    assert_eq!(importer.applied_deletion_watermark(&org_a), 2);
}
