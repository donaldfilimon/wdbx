//! Native WDBX qualification for the canonical Abbey episode contract subset.

#[path = "support/abbey_contracts.rs"]
mod support;

use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use support::{ContractFailure, FixtureDisposition, WdbxContractCorpus};

const QUALIFIED_ABI_REVISION: &str = "63e6d6a79d0b8745a652803887d07665245ddb39";
const QUALIFIED_AGGREGATE_DIGEST: &str =
    "3ffd487bdc497b7ce54b8c29978a3686dcbffdb66a85957a0ee4f99ba576cdfd";
static SCRATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct ScratchCorpus {
    root: std::path::PathBuf,
}

impl ScratchCorpus {
    fn copy_from_repo() -> Self {
        let sequence = SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "wdbx-abbey-contracts-{}-{sequence}",
            std::process::id()
        ));
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("contracts/abbey");
        copy_tree(&source, &root);
        Self { root }
    }
}

impl Drop for ScratchCorpus {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir(destination).expect("scratch destination must be new");
    for entry in fs::read_dir(source).expect("source directory must read") {
        let entry = entry.expect("source entry must read");
        let target = destination.join(entry.file_name());
        if entry.file_type().expect("entry type must read").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("artifact must copy");
        }
    }
}

fn corpus() -> WdbxContractCorpus {
    WdbxContractCorpus::open_from_repo().expect("the vendored corpus must qualify")
}

fn fixture_document(relative: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("contracts/abbey/corpus")
        .join(relative);
    serde_json::from_slice::<Value>(&fs::read(path).expect("fixture must be readable"))
        .expect("fixture must be JSON")["document"]
        .clone()
}

#[test]
fn corpus_lock_and_manifest_match_the_qualified_abi_source() {
    let corpus = corpus();
    assert_eq!(corpus.source_revision(), QUALIFIED_ABI_REVISION);
    assert_eq!(corpus.aggregate_digest(), QUALIFIED_AGGREGATE_DIGEST);
    assert_eq!(corpus.contract_major(), 2);
    assert_eq!(corpus.contract_revision(), 2);
}

#[test]
fn every_episode_family_fixture_has_its_declared_native_disposition() {
    let corpus = corpus();
    let paths = corpus.episode_fixture_paths();
    assert_eq!(
        paths.len(),
        8,
        "the v1 WDBX subset is intentionally bounded"
    );

    for path in paths {
        let disposition = corpus
            .validate_fixture(&path)
            .unwrap_or_else(|failure| panic!("{}: {failure}", path.display()));
        assert_eq!(
            disposition.actual(),
            disposition.expected(),
            "{}",
            path.display()
        );
    }
}

#[test]
fn valid_episode_evidence_claim_retention_and_link_shapes_decode() {
    let corpus = corpus();
    for path in [
        "v1/fixtures/valid/episode-proposal.json",
        "v1/fixtures/valid/episode-mandatory-incident.json",
        "v1/fixtures/valid/episode-evidence.json",
        "v1/fixtures/valid/episode-claim.json",
        "v1/fixtures/valid/episode-tombstone.json",
    ] {
        assert_eq!(
            corpus.validate_fixture(Path::new(path)),
            Ok(FixtureDisposition::new("valid", "valid")),
            "{path}"
        );
    }
}

#[test]
fn transport_json_is_never_accepted_as_a_canonical_episode_commitment() {
    let corpus = corpus();
    let proposal = fixture_document("v1/fixtures/valid/episode-proposal.json");
    assert_eq!(
        corpus.validate_canonical_episode(&proposal),
        Err(ContractFailure::CanonicalEncodingForbidden {
            path: "episode_transport_json".to_owned(),
        })
    );
}

#[test]
fn adapter_supplied_episode_digest_is_rejected() {
    let corpus = corpus();
    let mut proposal = fixture_document("v1/fixtures/valid/episode-proposal.json");
    proposal.as_object_mut().expect("proposal object").insert(
        "episode_digest".to_owned(),
        json!("sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
    );
    assert_eq!(
        corpus.validate_document(
            "https://abbey.local/contracts/abbey/v1/schemas/episode/proposal.schema.json",
            &proposal,
        ),
        Err(ContractFailure::SchemaRejected {
            path: "episode/proposal".to_owned(),
        })
    );
}

#[test]
fn deletion_key_and_retention_policy_are_fail_closed() {
    let corpus = corpus();
    let schema = "https://abbey.local/contracts/abbey/v1/schemas/episode/proposal.schema.json";
    let mut missing_key = fixture_document("v1/fixtures/valid/episode-proposal.json");
    missing_key
        .as_object_mut()
        .expect("proposal object")
        .remove("deletion_key");
    assert_eq!(
        corpus.validate_document(schema, &missing_key),
        Err(ContractFailure::SchemaRejected {
            path: "episode/proposal".to_owned(),
        })
    );

    let mut invalid_retention = fixture_document("v1/fixtures/valid/episode-proposal.json");
    invalid_retention
        .as_object_mut()
        .expect("proposal object")
        .insert("retention_class".to_owned(), json!("forever"));
    assert_eq!(
        corpus.validate_document(schema, &invalid_retention),
        Err(ContractFailure::SchemaRejected {
            path: "episode/proposal".to_owned(),
        })
    );
}

#[test]
fn adapter_projection_cannot_decode_as_a_canonical_episode() {
    let corpus = corpus();
    let projection = json!({
        "format": "# ABI-WDBX v1",
        "memory_facts": [{"key": "synthetic", "value": "redacted"}],
        "vectors": []
    });
    assert_eq!(
        corpus.validate_canonical_episode(&projection),
        Err(ContractFailure::ProjectionForbidden {
            path: "adapter_projection".to_owned(),
        })
    );
}

#[test]
fn artifact_byte_mutation_and_extra_inventory_fail_native_verification() {
    let mutated = ScratchCorpus::copy_from_repo();
    let proposal = mutated
        .root
        .join("corpus/v1/fixtures/valid/episode-proposal.json");
    let mut bytes = fs::read(&proposal).expect("proposal must read");
    let offset = bytes
        .iter()
        .position(|byte| *byte == b'a')
        .expect("fixture contains a mutation byte");
    bytes[offset] = b'b';
    fs::write(&proposal, bytes).expect("scratch mutation must write");
    assert_eq!(
        WdbxContractCorpus::open_at(&mutated.root).expect_err("mutated byte must fail"),
        ContractFailure::ArtifactDigestMismatch {
            path: "v1/fixtures/valid/episode-proposal.json".to_owned(),
        }
    );

    let extra = ScratchCorpus::copy_from_repo();
    fs::write(extra.root.join("corpus/extra.json"), b"{}\n")
        .expect("scratch extra file must write");
    assert_eq!(
        WdbxContractCorpus::open_at(&extra.root).expect_err("extra artifact must fail"),
        ContractFailure::InventoryMismatch {
            path: "manifest.json".to_owned(),
        }
    );
}

#[test]
fn manifest_line_ending_mutation_is_not_normalized_away() {
    let mutated = ScratchCorpus::copy_from_repo();
    let manifest = mutated.root.join("corpus/manifest.json");
    let bytes = fs::read(&manifest).expect("manifest must read");
    let changed = String::from_utf8(bytes)
        .expect("manifest is UTF-8")
        .replace('\n', "\r\n");
    fs::write(&manifest, changed).expect("scratch mutation must write");
    assert_eq!(
        WdbxContractCorpus::open_at(&mutated.root).expect_err("changed line endings must fail"),
        ContractFailure::ManifestInvalid {
            path: "manifest.json".to_owned(),
        }
    );
}
