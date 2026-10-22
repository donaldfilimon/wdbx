//! Frozen legacy behavior; this corpus does not prescribe the new checked API.
use abi_wdbx::StorePaths;
use abi_wdbx::spatial::{DistanceMetric, Point3D, SpatialIndex3D};
use abi_wdbx::v2::{
    ObjectKind, ObjectSecurity, VersionedSnapshot, open_object, open_versioned_read_only,
    seal_object,
};
use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeSigner, EpisodeSource, EpisodeStore, EpisodeWrite,
    EvidenceLevel, GuildEpisodePolicy, SignatureStatus, StorePolicy,
};
use ed25519_dalek::SigningKey;
use sha2::{Digest as _, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("wdbx-compatibility-{}", Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn manifest() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/wdbx_3d/compatibility-manifest.json")).unwrap()
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(result, "{byte:02x}").unwrap();
    }
    result
}
fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn pinned(name: &str, bytes: &[u8]) {
    let actual = hash(bytes);
    assert_eq!(
        actual,
        manifest()["sha256"][name].as_str().unwrap(),
        "fixture {name}"
    );
}
fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

#[test]
fn legacy_reader_is_nonmutating() {
    let scratch = Scratch::new();
    let paths = StorePaths::new(&scratch.0);
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    for (source, destination) in [
        ("wdbx-sample.manifest", paths.manifest()),
        ("wdbx-sample.seg.jsonl", paths.segment(0)),
        ("wdbx-sample.seg.1-collected.jsonl", paths.segment(1)),
        ("wdbx-sample.seg.2.jsonl", paths.segment(2)),
        ("wdbx-sample.seg.3.jsonl", paths.segment(3)),
    ] {
        let bytes = fs::read(golden.join(source)).unwrap();
        pinned(source, &bytes);
        fs::write(destination, bytes).unwrap();
    }
    let before = files(&scratch.0);
    for _ in 0..2 {
        let VersionedSnapshot::V1(snapshot) = open_versioned_read_only(&paths).unwrap() else {
            panic!("must remain v1")
        };
        // Highest active epoch 3 is a valid empty checkpoint, not a delta.
        assert_eq!(snapshot.stats.blocks, 0);
        assert_eq!(snapshot.stats.spatial_records, 0);
        snapshot.verify_chain().unwrap();
        assert_eq!(
            files(&scratch.0),
            before,
            "read must preserve every filename and byte"
        );
    }
    let monolithic = Scratch::new();
    let paths = StorePaths::new(&monolithic.0);
    fs::copy(golden.join("wdbx-sample.seg.jsonl"), paths.index()).unwrap();
    let before = files(&monolithic.0);
    let VersionedSnapshot::V1(snapshot) = open_versioned_read_only(&paths).unwrap() else {
        panic!("monolithic fixture must remain v1")
    };
    assert_eq!(snapshot.stats.blocks, 2);
    assert_eq!(snapshot.stats.spatial_records, 1);
    snapshot.verify_chain().unwrap();
    assert_eq!(files(&monolithic.0), before);
}

#[test]
fn legacy_api_order_is_characterized() {
    for count in [63_u32, 64, 65] {
        let mut index = SpatialIndex3D::new();
        // Insert positive octant first. Equal distances retain scan or octant traversal order.
        index.insert(
            7,
            Point3D {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            b"positive",
        );
        index.insert(
            7,
            Point3D {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
            b"negative",
        );
        for id in 2..count {
            index.insert(
                id,
                Point3D {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                b"center",
            );
        }
        assert_eq!(index.count(), usize::try_from(count).unwrap());
        for metric in [DistanceMetric::Euclidean, DistanceMetric::Manhattan] {
            let results = index.radius_search(Point3D::default(), 1.0, metric);
            let duplicates: Vec<_> = results
                .iter()
                .filter(|hit| hit.id == 7 && hit.distance.to_bits() == 1.0_f32.to_bits())
                .map(|hit| hit.payload)
                .collect();
            let expected: Vec<&[u8]> = if count < 64 {
                vec![b"positive", b"negative"]
            } else {
                vec![b"negative", b"positive"]
            };
            assert_eq!(duplicates, expected, "radius threshold {count}, {metric:?}");
            let nearest = index.nearest_neighbors(
                Point3D::default(),
                usize::try_from(count).unwrap(),
                metric,
            );
            assert_eq!(
                nearest
                    .iter()
                    .filter(|hit| hit.id == 7 && hit.distance.to_bits() == 1.0_f32.to_bits())
                    .map(|hit| hit.payload)
                    .collect::<Vec<_>>(),
                expected
            );
            // k cuts inside the tie, freezing which observation survives.
            let nearest = index.nearest_neighbors(
                Point3D::default(),
                usize::try_from(count - 1).unwrap(),
                metric,
            );
            assert_eq!(nearest.last().unwrap().payload, expected[0]);
        }
        let cosine = index.radius_search(Point3D::default(), 1.0, DistanceMetric::Cosine);
        assert_eq!(cosine[0].payload, b"positive");
        assert_eq!(cosine[1].payload, b"negative");
    }
}

fn policy() -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([(
            "guild_ref".into(),
            GuildEpisodePolicy {
                learning_enabled: true,
                policy_version: "policy_v1".into(),
                token_budget: 1000,
                storage_budget_bytes: 1024 * 1024,
                current_consent_epoch: Some(7),
                retention_ttl_ms: None,
                retention_windows: None,
                authorized_principals: None,
            },
        )]),
    }
}
fn proposal() -> EpisodeWrite {
    EpisodeWrite {
        request_id: "request_1".into(),
        operation_id: "operation_1".into(),
        contract_revision: 2,
        contract_digest: [9; 32],
        guild_ref: "guild_ref".into(),
        consent_epoch: Some(7),
        source_type: EpisodeSource::DiscordVoice,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C2,
        event: EpisodeEvent::Proposal {
            requested_by: ActorRef {
                principal_id: "requester_ref".into(),
                kind: ActorKind::HumanSubject,
            },
            proposed_by: ActorRef {
                principal_id: "abbey_service".into(),
                kind: ActorKind::Service,
            },
        },
        token_cost: 3,
        expected_commitment: None,
        quiet: false,
    }
}

#[test]
fn signed_bytes_remain_identical() {
    let scratch = Scratch::new();
    // Synthetic public test seed only, never an operator key. Use the public permission-checked loader.
    let key_path = scratch.0.join("test-signing-key");
    fs::write(&key_path, [1_u8; 32]).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let security = ObjectSecurity::from_key_files(None, Some(&key_path), None).unwrap();
    // This tests the frozen v2 authenticated object layer, not a second journal format.
    let payload = br#"{"parents":["second","first"],"point":[-0.0,1.0,2.0]}"#;
    let encoded =
        seal_object(ObjectKind::Artifact, "compatibility-v2", payload, &security).unwrap();
    let object_path = scratch.0.join("signed-v2-object");
    fs::write(&object_path, &encoded).unwrap();
    let signer = EpisodeSigner::new(SigningKey::from_bytes(&[1; 32]));
    let key = signer.verifying_key();
    let key_id = signer.key_id().clone();
    let directory = scratch.0.join("v3");
    let mut store = EpisodeStore::open_with_signer(&directory, policy(), signer).unwrap();
    let digest = store.propose_write(&proposal()).unwrap().episode_digest;
    drop(store);
    let before = files(&scratch.0);
    for _ in 0..2 {
        let raw = fs::read(&object_path).unwrap();
        let opened = open_object(&raw, &security, true).unwrap();
        assert!(opened.signed);
        assert_eq!(opened.kind, ObjectKind::Artifact);
        assert_eq!(opened.plaintext, payload);
        let store = EpisodeStore::open(&directory, policy()).unwrap();
        assert_eq!(store.retrieve("guild_ref", 10).unwrap().len(), 1);
        assert!(store.find_receipt("guild_ref", &digest).unwrap().is_some());
        assert_eq!(
            store
                .signature_status("guild_ref", &digest, |id| (id == &key_id).then_some(key))
                .unwrap(),
            Some(SignatureStatus::Valid(key_id.clone()))
        );
        drop(store);
        assert_eq!(files(&scratch.0), before);
    }
    pinned("signed-v2-object", &encoded);
    pinned(
        "signed-v3-ledger",
        &fs::read(directory.join("episodes.v1.jsonl")).unwrap(),
    );
    assert_eq!(
        hex(&digest),
        manifest()["v3_episode_digest"].as_str().unwrap()
    );
}
