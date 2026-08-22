//! Bounded, native conformance support for WDBX-owned Abbey contracts.

use jsonschema::{Draft, Retrieve, Uri};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

const SOURCE_REPOSITORY: &str = "https://github.com/donaldfilimon/abi";
const CORPUS_ALGORITHM: &str = "abbey-contract-corpus-sha256-v1";
const CORPUS_DOMAIN: &[u8] = b"abbey-contract-corpus-v1\0";
const EPISODE_SCHEMA_PREFIX: &str = "https://abbey.local/contracts/abbey/v1/schemas/episode/";
const MAX_ARTIFACT_BYTES: u64 = 1024 * 1024;
const MAX_CORPUS_BYTES: u64 = 16 * 1024 * 1024;

/// Closed native verifier failures containing only corpus-relative paths.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ContractFailure {
    /// A committed artifact could not be read.
    #[error("artifact_unreadable:{path}")]
    ArtifactUnreadable { path: String },
    /// A committed path was not a bounded regular file or directory.
    #[error("artifact_invalid:{path}")]
    ArtifactInvalid { path: String },
    /// A path was not normalized corpus-relative UTF-8.
    #[error("path_invalid:{path}")]
    PathInvalid { path: String },
    /// A JSON envelope did not have the closed expected shape.
    #[error("json_invalid:{path}")]
    JsonInvalid { path: String },
    /// The vendor lock was not the closed v1 lock.
    #[error("lock_invalid:{path}")]
    LockInvalid { path: String },
    /// The source manifest was not the closed v1 manifest.
    #[error("manifest_invalid:{path}")]
    ManifestInvalid { path: String },
    /// Manifest inventory did not exactly match the vendored bytes.
    #[error("inventory_mismatch:{path}")]
    InventoryMismatch { path: String },
    /// A file length or SHA-256 commitment differed.
    #[error("artifact_digest_mismatch:{path}")]
    ArtifactDigestMismatch { path: String },
    /// The independently computed aggregate commitment differed.
    #[error("aggregate_digest_mismatch:{path}")]
    AggregateDigestMismatch { path: String },
    /// A local episode schema did not compile without external resolution.
    #[error("schema_compile:{path}")]
    SchemaCompile { path: String },
    /// A document failed its WDBX-owned episode-family schema.
    #[error("schema_rejected:{path}")]
    SchemaRejected { path: String },
    /// A document failed a WDBX-owned semantic invariant.
    #[error("semantic_rejected:{path}")]
    SemanticRejected { path: String },
    /// A requested fixture was outside the episode-family scope.
    #[error("fixture_out_of_scope:{path}")]
    FixtureOutOfScope { path: String },
    /// Transport JSON cannot be a durable WDBX episode commitment.
    #[error("canonical_encoding_forbidden:{path}")]
    CanonicalEncodingForbidden { path: String },
    /// An adapter projection cannot be decoded as a canonical episode.
    #[error("projection_forbidden:{path}")]
    ProjectionForbidden { path: String },
}

/// The observed and declared outcome of one episode-family fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureDisposition {
    actual: String,
    expected: String,
}

impl FixtureDisposition {
    /// Construct a closed fixture disposition.
    #[must_use]
    pub fn new(actual: &str, expected: &str) -> Self {
        Self {
            actual: actual.to_owned(),
            expected: expected.to_owned(),
        }
    }

    /// Return the independently observed closed reason code.
    #[must_use]
    pub fn actual(&self) -> &str {
        &self.actual
    }

    /// Return the fixture's declared closed reason code.
    #[must_use]
    pub fn expected(&self) -> &str {
        &self.expected
    }
}

/// Digest-qualified native view of WDBX's bounded contract subset.
#[derive(Debug, Clone)]
pub struct WdbxContractCorpus {
    root: PathBuf,
    lock: VendorLock,
    schemas: HashMap<String, Value>,
    episode_fixtures: Vec<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct VendorLock {
    source_repository: String,
    source_revision: String,
    contract_major: u32,
    contract_revision: u32,
    aggregate_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    contract_major: u32,
    contract_revision: u32,
    algorithm: String,
    redaction_profile: String,
    artifacts: Vec<ArtifactRow>,
    aggregate_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactRow {
    path: String,
    bytes: u64,
    media_type: String,
    sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    case_id: String,
    schema: String,
    expect: String,
    document: Value,
}

#[derive(Clone)]
struct LocalRetriever {
    schemas: HashMap<String, Value>,
}

impl Retrieve for LocalRetriever {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        self.schemas
            .get(uri.as_str())
            .cloned()
            .ok_or_else(|| "external schema resolution is disabled".into())
    }
}

impl WdbxContractCorpus {
    /// Open and independently verify the repository-vendored corpus.
    pub fn open_from_repo() -> Result<Self, ContractFailure> {
        let destination = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("contracts/abbey");
        Self::open_at(&destination)
    }

    /// Open and independently verify a controlled vendor destination.
    pub fn open_at(destination: &Path) -> Result<Self, ContractFailure> {
        require_directory(destination, "contracts/abbey")?;
        let root = destination.join("corpus");
        require_directory(&root, "corpus")?;

        let lock: VendorLock = parse_json(
            &read_bounded(
                &destination.join("abbey-contracts.lock.json"),
                "abbey-contracts.lock.json",
            )?,
            "abbey-contracts.lock.json",
        )?;
        if lock.source_repository != SOURCE_REPOSITORY
            || lock.source_revision.len() != 40
            || !is_lower_hex(&lock.source_revision)
            || lock.contract_major != 1
            || lock.aggregate_digest.len() != 64
            || !is_lower_hex(&lock.aggregate_digest)
        {
            return Err(ContractFailure::LockInvalid {
                path: "abbey-contracts.lock.json".to_owned(),
            });
        }

        let manifest_bytes = read_bounded(&root.join("manifest.json"), "manifest.json")?;
        let manifest: Manifest = parse_json(&manifest_bytes, "manifest.json")?;
        let mut fixed_manifest_bytes =
            serde_json::to_vec_pretty(&manifest).map_err(|_| ContractFailure::ManifestInvalid {
                path: "manifest.json".to_owned(),
            })?;
        fixed_manifest_bytes.push(b'\n');
        if fixed_manifest_bytes != manifest_bytes {
            return Err(ContractFailure::ManifestInvalid {
                path: "manifest.json".to_owned(),
            });
        }
        if manifest.contract_major != lock.contract_major
            || manifest.contract_revision != lock.contract_revision
            || manifest.algorithm != CORPUS_ALGORITHM
            || manifest.aggregate_digest != lock.aggregate_digest
            || manifest.redaction_profile != "abbey-contract-redaction-v1"
        {
            return Err(ContractFailure::ManifestInvalid {
                path: "manifest.json".to_owned(),
            });
        }

        verify_artifacts(&root, &manifest)?;
        if aggregate_digest(&manifest)? != lock.aggregate_digest {
            return Err(ContractFailure::AggregateDigestMismatch {
                path: "manifest.json".to_owned(),
            });
        }

        let mut schemas = HashMap::new();
        let mut episode_fixtures = Vec::new();
        for row in &manifest.artifacts {
            if let Some(schema_id) = &row.schema_id {
                let value: Value =
                    parse_json(&read_bounded(&root.join(&row.path), &row.path)?, &row.path)?;
                if schemas.insert(schema_id.clone(), value).is_some() {
                    return Err(ContractFailure::SchemaCompile {
                        path: row.path.clone(),
                    });
                }
            } else if row.path.contains("/fixtures/")
                && Path::new(&row.path)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
            {
                let value: Value =
                    parse_json(&read_bounded(&root.join(&row.path), &row.path)?, &row.path)?;
                if value
                    .get("schema")
                    .and_then(Value::as_str)
                    .is_some_and(|schema| schema.starts_with(EPISODE_SCHEMA_PREFIX))
                {
                    episode_fixtures.push(PathBuf::from(&row.path));
                }
            }
        }

        for row in manifest.artifacts.iter().filter(|row| {
            row.schema_id
                .as_deref()
                .is_some_and(|schema| schema.starts_with(EPISODE_SCHEMA_PREFIX))
        }) {
            let schema_id = row.schema_id.as_deref().expect("filtered schema row");
            compile_schema(
                schemas
                    .get(schema_id)
                    .expect("schema indexed from same row"),
                &schemas,
            )
            .map_err(|()| ContractFailure::SchemaCompile {
                path: row.path.clone(),
            })?;
        }
        episode_fixtures.sort();

        Ok(Self {
            root,
            lock,
            schemas,
            episode_fixtures,
        })
    }

    /// Return the immutable ABI source revision from the qualified lock.
    #[must_use]
    pub fn source_revision(&self) -> &str {
        &self.lock.source_revision
    }

    /// Return the independently verified aggregate corpus digest.
    #[must_use]
    pub fn aggregate_digest(&self) -> &str {
        &self.lock.aggregate_digest
    }

    /// Return the qualified contract major.
    #[must_use]
    pub fn contract_major(&self) -> u32 {
        self.lock.contract_major
    }

    /// Return the qualified additive contract revision.
    #[must_use]
    pub fn contract_revision(&self) -> u32 {
        self.lock.contract_revision
    }

    /// Return only fixture paths that declare an episode-family schema.
    #[must_use]
    pub fn episode_fixture_paths(&self) -> Vec<PathBuf> {
        self.episode_fixtures.clone()
    }

    /// Validate one committed episode-family fixture natively.
    pub fn validate_fixture(&self, relative: &Path) -> Result<FixtureDisposition, ContractFailure> {
        let path = normalize_relative(relative)?;
        if !self.episode_fixtures.iter().any(|item| item == relative) {
            return Err(ContractFailure::FixtureOutOfScope { path });
        }
        let fixture: Fixture = parse_json(&read_bounded(&self.root.join(relative), &path)?, &path)?;
        let _ = &fixture.case_id;
        let actual = match self.validate_document(&fixture.schema, &fixture.document) {
            Ok(()) => "valid",
            Err(ContractFailure::SchemaRejected { .. }) => "schema_invalid",
            Err(ContractFailure::SemanticRejected { path }) => match path.as_str() {
                "mandatory_controls_missing" => "mandatory_controls_missing",
                "evidence_overclaim" => "evidence_overclaim",
                _ => "semantic_invalid",
            },
            Err(failure) => return Err(failure),
        };
        Ok(FixtureDisposition::new(actual, &fixture.expect))
    }

    /// Validate an in-memory document against a local WDBX-owned schema.
    pub fn validate_document(
        &self,
        schema_id: &str,
        document: &Value,
    ) -> Result<(), ContractFailure> {
        if !schema_id.starts_with(EPISODE_SCHEMA_PREFIX) {
            return Err(ContractFailure::FixtureOutOfScope {
                path: "schema_family".to_owned(),
            });
        }
        if let Some(reason) = pre_schema_semantic(schema_id, document) {
            return Err(ContractFailure::SemanticRejected {
                path: reason.to_owned(),
            });
        }
        let schema = self
            .schemas
            .get(schema_id)
            .ok_or_else(|| ContractFailure::SchemaCompile {
                path: schema_label(schema_id),
            })?;
        let validator =
            compile_schema(schema, &self.schemas).map_err(|()| ContractFailure::SchemaCompile {
                path: schema_label(schema_id),
            })?;
        if !validator.is_valid(document) {
            return Err(ContractFailure::SchemaRejected {
                path: schema_label(schema_id),
            });
        }
        if let Some(reason) = post_schema_semantic(schema_id, document) {
            return Err(ContractFailure::SemanticRejected {
                path: reason.to_owned(),
            });
        }
        Ok(())
    }

    /// Refuse transport JSON and known adapter projections as canonical episodes.
    pub fn validate_canonical_episode(&self, transport: &Value) -> Result<(), ContractFailure> {
        let _ = self;
        let is_projection = transport.as_object().is_some_and(|object| {
            object.get("format").and_then(Value::as_str) == Some("# ABI-WDBX v1")
                || object.contains_key("memory_facts")
                || object.contains_key("vectors")
        });
        if is_projection {
            Err(ContractFailure::ProjectionForbidden {
                path: "adapter_projection".to_owned(),
            })
        } else {
            Err(ContractFailure::CanonicalEncodingForbidden {
                path: "episode_transport_json".to_owned(),
            })
        }
    }
}

fn require_directory(path: &Path, display: &str) -> Result<(), ContractFailure> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ContractFailure::ArtifactUnreadable {
        path: display.to_owned(),
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ContractFailure::ArtifactInvalid {
            path: display.to_owned(),
        });
    }
    Ok(())
}

fn read_bounded(path: &Path, display: &str) -> Result<Vec<u8>, ContractFailure> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ContractFailure::ArtifactUnreadable {
        path: display.to_owned(),
    })?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_ARTIFACT_BYTES
    {
        return Err(ContractFailure::ArtifactInvalid {
            path: display.to_owned(),
        });
    }
    fs::read(path).map_err(|_| ContractFailure::ArtifactUnreadable {
        path: display.to_owned(),
    })
}

fn parse_json<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
    path: &str,
) -> Result<T, ContractFailure> {
    serde_json::from_slice(bytes).map_err(|_| ContractFailure::JsonInvalid {
        path: path.to_owned(),
    })
}

fn verify_artifacts(root: &Path, manifest: &Manifest) -> Result<(), ContractFailure> {
    let actual = discover(root)?;
    let actual_names: BTreeSet<String> = actual
        .iter()
        .map(|path| normalize_relative(path))
        .collect::<Result<_, _>>()?;
    let mut listed = BTreeSet::new();
    let mut total = 0_u64;
    for row in &manifest.artifacts {
        validate_manifest_path(&row.path)?;
        if !listed.insert(row.path.clone()) {
            return Err(ContractFailure::InventoryMismatch {
                path: row.path.clone(),
            });
        }
        let bytes = read_bounded(&root.join(&row.path), &row.path)?;
        total = total.saturating_add(bytes.len() as u64);
        if row.bytes != bytes.len() as u64 || row.sha256 != sha256_hex(&bytes) {
            return Err(ContractFailure::ArtifactDigestMismatch {
                path: row.path.clone(),
            });
        }
    }
    if total > MAX_CORPUS_BYTES || listed != actual_names {
        return Err(ContractFailure::InventoryMismatch {
            path: "manifest.json".to_owned(),
        });
    }
    Ok(())
}

fn discover(root: &Path) -> Result<Vec<PathBuf>, ContractFailure> {
    fn visit(
        root: &Path,
        relative: &Path,
        output: &mut Vec<PathBuf>,
    ) -> Result<(), ContractFailure> {
        let display = if relative.as_os_str().is_empty() {
            "corpus".to_owned()
        } else {
            normalize_relative(relative)?
        };
        for entry in
            fs::read_dir(root.join(relative)).map_err(|_| ContractFailure::ArtifactUnreadable {
                path: display.clone(),
            })?
        {
            let entry = entry.map_err(|_| ContractFailure::ArtifactUnreadable {
                path: display.clone(),
            })?;
            let child = relative.join(entry.file_name());
            let child_display = normalize_relative(&child)?;
            let file_type = entry
                .file_type()
                .map_err(|_| ContractFailure::ArtifactInvalid {
                    path: child_display.clone(),
                })?;
            if file_type.is_symlink() {
                return Err(ContractFailure::ArtifactInvalid {
                    path: child_display,
                });
            }
            if file_type.is_dir() {
                visit(root, &child, output)?;
            } else if file_type.is_file() && child != Path::new("manifest.json") {
                output.push(child);
            } else if !file_type.is_file() {
                return Err(ContractFailure::ArtifactInvalid {
                    path: child_display,
                });
            }
        }
        Ok(())
    }

    let mut paths = Vec::new();
    visit(root, Path::new(""), &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn normalize_relative(path: &Path) -> Result<String, ContractFailure> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let text = part.to_str().ok_or_else(|| ContractFailure::PathInvalid {
                    path: "non_utf8".to_owned(),
                })?;
                if text.contains('\\') {
                    return Err(ContractFailure::PathInvalid {
                        path: "backslash".to_owned(),
                    });
                }
                parts.push(text);
            }
            _ => {
                return Err(ContractFailure::PathInvalid {
                    path: "non_relative".to_owned(),
                });
            }
        }
    }
    Ok(parts.join("/"))
}

fn validate_manifest_path(path: &str) -> Result<(), ContractFailure> {
    if path.is_empty() || path.contains('\\') || Path::new(path).is_absolute() {
        return Err(ContractFailure::PathInvalid {
            path: "manifest_entry".to_owned(),
        });
    }
    if Path::new(path)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ContractFailure::PathInvalid {
            path: "manifest_entry".to_owned(),
        });
    }
    Ok(())
}

fn aggregate_digest(manifest: &Manifest) -> Result<String, ContractFailure> {
    let mut zeroed = manifest.clone();
    zeroed.aggregate_digest = "0".repeat(64);
    let mut manifest_bytes =
        serde_json::to_vec_pretty(&zeroed).map_err(|_| ContractFailure::ManifestInvalid {
            path: "manifest.json".to_owned(),
        })?;
    manifest_bytes.push(b'\n');

    let mut entries: Vec<(String, u64, String)> = manifest
        .artifacts
        .iter()
        .map(|row| (row.path.clone(), row.bytes, row.sha256.clone()))
        .collect();
    entries.push((
        "manifest.json".to_owned(),
        manifest_bytes.len() as u64,
        sha256_hex(&manifest_bytes),
    ));
    entries.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));

    let mut hasher = Sha256::new();
    hasher.update(CORPUS_DOMAIN);
    for (path, bytes, digest) in entries {
        hasher.update(path.as_bytes());
        hasher.update([0]);
        hasher.update(bytes.to_string().as_bytes());
        hasher.update([0]);
        hasher.update(digest.as_bytes());
        hasher.update(b"\n");
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn compile_schema(
    schema: &Value,
    schemas: &HashMap<String, Value>,
) -> Result<jsonschema::Validator, ()> {
    jsonschema::options()
        .with_draft(Draft::Draft202012)
        .with_retriever(LocalRetriever {
            schemas: schemas.clone(),
        })
        .build(schema)
        .map_err(|_| ())
}

fn schema_label(schema_id: &str) -> String {
    schema_id
        .strip_prefix(EPISODE_SCHEMA_PREFIX)
        .and_then(|suffix| suffix.strip_suffix(".schema.json"))
        .map_or_else(
            || "episode/schema".to_owned(),
            |name| format!("episode/{name}"),
        )
}

fn pre_schema_semantic(schema: &str, document: &Value) -> Option<&'static str> {
    let object = document.as_object()?;
    if schema.ends_with("/episode/proposal.schema.json")
        && object.get("priority_class").and_then(Value::as_str) == Some("MandatoryIncident")
        && (object.get("minimized").and_then(Value::as_bool) != Some(true)
            || object.get("redacted").and_then(Value::as_bool) != Some(true)
            || object.get("deletion_required").and_then(Value::as_bool) != Some(true)
            || object.get("deletion_key").and_then(Value::as_str).is_none()
            || object.get("retention_class").and_then(Value::as_str) != Some("mandatory_incident")
            || !matches!(
                object.get("hold_state").and_then(Value::as_str),
                Some("active" | "released")
            ))
    {
        return Some("mandatory_controls_missing");
    }
    None
}

fn post_schema_semantic(schema: &str, document: &Value) -> Option<&'static str> {
    if !schema.ends_with("/episode/claim.schema.json") {
        return None;
    }
    let object: &Map<String, Value> = document.as_object()?;
    let level = |field: &str| {
        object
            .get(field)
            .and_then(Value::as_str)
            .and_then(|value| value.strip_prefix('C'))
            .and_then(|value| value.parse::<u8>().ok())
    };
    (level("display_evidence_level") > level("evidence_level")).then_some("evidence_overclaim")
}
