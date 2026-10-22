//! The origin's durable state: its deletion log and its export bindings.
//!
//! # The deletion log
//!
//! The origin records every deletion it issues, in order, with a sequence
//! number that is the origin's deletion watermark. A bundle carries the
//! watermark and the deletions up to it, so an importing organization can
//! apply them exactly once and in order. The log is content-free: a
//! sequence number and a canonical episode digest.
//!
//! # The export bindings
//!
//! The origin binds each checkpoint to the bundle digest it produced. The
//! binding is what stops *that origin* from equivocating against itself: a
//! second export at the same checkpoint with different content is refused.
//! The binding is per origin *host*, so two hosts of one organization can
//! still equivocate against each other; that equivocation is what an
//! importing organization detects.

use crate::v3::federation::bundle::DeletionEntry;
use crate::v3::federation::error::FederationError;
use crate::v3::federation::sidecar::{
    Appender, checksum, complete_lines, prepare_directory, read_bounded_optional, write_atomic,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The origin's deletion log file.
pub(crate) const DELETION_FILE: &str = "federation.v1.deletions.jsonl";

/// The origin's export binding file.
pub(crate) const EXPORT_FILE: &str = "federation.v1.exports.json";

/// The profile the export binding file belongs to.
const PROFILE: &str = "wdbx-federation-exports-v1";

/// One line of the deletion log.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DeletionLine {
    /// The origin's deletion sequence number.
    sequence: u64,
    /// The canonical episode digest the deletion erases.
    episode_digest: [u8; 32],
}

/// The origin's ordered deletion log.
pub(crate) struct DeletionLog {
    appender: Appender,
    entries: Vec<DeletionEntry>,
    watermark: u64,
}

impl DeletionLog {
    /// Open (and create) the origin's deletion log.
    pub(crate) fn open(directory: &Path) -> Result<Self, FederationError> {
        let directory = prepare_directory(directory)?;
        let path = directory.join(DELETION_FILE);
        let raw = read_bounded_optional(&path)?.unwrap_or_default();
        let appender = Appender::open(&path)?;
        let mut log = Self {
            appender,
            entries: Vec::new(),
            watermark: 0,
        };
        for line in complete_lines(&raw) {
            if line.is_empty() {
                continue;
            }
            let parsed: DeletionLine =
                serde_json::from_slice(line).map_err(|_| FederationError::Corrupt)?;
            if parsed.sequence == 0 || parsed.sequence <= log.watermark {
                return Err(FederationError::Corrupt);
            }
            log.watermark = parsed.sequence;
            log.entries.push(DeletionEntry {
                sequence: parsed.sequence,
                episode_digest: parsed.episode_digest,
            });
        }
        Ok(log)
    }

    /// The origin's current deletion watermark.
    pub(crate) fn watermark(&self) -> u64 {
        self.watermark
    }

    /// The deletions the origin has issued, in order.
    pub(crate) fn entries(&self) -> &[DeletionEntry] {
        &self.entries
    }

    /// Record one deletion and return the new watermark.
    ///
    /// The same digest is not recorded twice: the watermark advances only
    /// when a *new* deletion is issued, so a repeated notice cannot make an
    /// importing organization re-apply an old deletion under a new number.
    pub(crate) fn record(&mut self, episode_digest: &[u8; 32]) -> Result<u64, FederationError> {
        if self
            .entries
            .iter()
            .any(|entry| &entry.episode_digest == episode_digest)
        {
            return Ok(self.watermark);
        }
        let sequence = self.watermark.saturating_add(1);
        if sequence == 0 {
            return Err(FederationError::BudgetExhausted);
        }
        let line = DeletionLine {
            sequence,
            episode_digest: *episode_digest,
        };
        let mut bytes = serde_json::to_vec(&line).map_err(|_| FederationError::Corrupt)?;
        bytes.shrink_to_fit();
        self.appender.append_line(&bytes)?;
        self.entries.push(DeletionEntry {
            sequence,
            episode_digest: *episode_digest,
        });
        self.watermark = sequence;
        Ok(sequence)
    }
}

/// One checkpoint binding of the origin.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExportBinding {
    /// The scope the checkpoint covers.
    pub(crate) scope: String,
    /// The bundle digest produced at that checkpoint.
    pub(crate) bundle_digest: [u8; 32],
}

/// The body of the export binding file.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ExportBody {
    profile: String,
    bindings: BTreeMap<String, ExportBinding>,
}

/// A sidecar file with a checksum over its own body.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Checksummed<T> {
    body: T,
    checksum: [u8; 32],
}

/// The origin's checkpoint-to-bundle bindings.
#[derive(Clone, Debug, Default)]
pub(crate) struct ExportBindings {
    bindings: BTreeMap<u64, ExportBinding>,
}

impl ExportBindings {
    /// Read the bindings, or the bindings of a fresh origin.
    pub(crate) fn load(path: &Path) -> Result<Self, FederationError> {
        let Some(raw) = read_bounded_optional(path)? else {
            return Ok(Self::default());
        };
        let file: Checksummed<ExportBody> =
            serde_json::from_slice(&raw).map_err(|_| FederationError::Corrupt)?;
        let body = serde_json::to_vec(&file.body).map_err(|_| FederationError::Corrupt)?;
        if file.checksum != checksum(&body) {
            return Err(FederationError::Corrupt);
        }
        if file.body.profile != PROFILE {
            return Err(FederationError::Corrupt);
        }
        let mut bindings = BTreeMap::new();
        for (key, value) in file.body.bindings {
            let checkpoint = key.parse::<u64>().map_err(|_| FederationError::Corrupt)?;
            bindings.insert(checkpoint, value);
        }
        Ok(Self { bindings })
    }

    /// Replace the bindings atomically.
    pub(crate) fn store(&self, path: &Path) -> Result<(), FederationError> {
        let body = ExportBody {
            profile: PROFILE.to_string(),
            bindings: self
                .bindings
                .iter()
                .map(|(checkpoint, binding)| (checkpoint.to_string(), binding.clone()))
                .collect(),
        };
        let encoded = serde_json::to_vec(&body).map_err(|_| FederationError::Corrupt)?;
        let file = Checksummed {
            body,
            checksum: checksum(&encoded),
        };
        let mut bytes = serde_json::to_vec(&file).map_err(|_| FederationError::Corrupt)?;
        bytes.push(b'\n');
        write_atomic(path, &bytes)
    }

    /// The binding at one checkpoint.
    pub(crate) fn get(&self, checkpoint: u64) -> Option<&ExportBinding> {
        self.bindings.get(&checkpoint)
    }

    /// Bind one checkpoint to a bundle digest.
    pub(crate) fn bind(&mut self, checkpoint: u64, scope: &str, bundle_digest: [u8; 32]) {
        self.bindings.insert(
            checkpoint,
            ExportBinding {
                scope: scope.to_string(),
                bundle_digest,
            },
        );
    }
}

/// The path of the export binding file inside a peer directory.
pub(crate) fn export_bindings_path(directory: &Path) -> PathBuf {
    directory.join(EXPORT_FILE)
}
