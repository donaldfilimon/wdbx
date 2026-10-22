//! The importing organization's foreign namespace.
//!
//! Imported records live here, in their own append-only file, and nowhere
//! else. They are deliberately *not* in the importing organization's
//! canonical episode ledger: that ledger has one writer, and its writer is
//! the importing organization. Letting a foreign record into that ledger
//! would let the importing organization author, replay, or re-sign an
//! episode that belongs to another organization.
//!
//! The namespace holds:
//!
//! - the imported records, with the checkpoint they arrived in;
//! - the tombstones the origin's deletions have produced;
//! - the deletion watermark applied per origin.
//!
//! Every entry is content-free: digests, scopes, and event labels. The file
//! carries no episode payload, because the v3 episode ledger commits to a
//! payload rather than storing it, so there is nothing to carry.

use crate::v3::federation::error::FederationError;
use crate::v3::federation::identity::OrganizationId;
use crate::v3::federation::record::ForeignRecord;
use crate::v3::federation::sidecar::{
    Appender, complete_lines, prepare_directory, read_bounded_optional,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The foreign namespace file of one importing peer.
pub(crate) const FOREIGN_FILE: &str = "federation.v1.foreign.jsonl";

/// One imported record, with the checkpoint it arrived in.
#[derive(Clone, Debug)]
pub(crate) struct StoredForeign {
    /// The imported record.
    pub(crate) record: ForeignRecord,
    /// The checkpoint the record arrived in.
    pub(crate) checkpoint: u64,
    /// The order in which the record was imported.
    pub(crate) sequence: u64,
}

/// One line of the foreign namespace file.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ForeignLine {
    /// One imported record.
    Record {
        /// The record.
        record: ForeignRecord,
        /// The checkpoint the record arrived in.
        checkpoint: u64,
    },
    /// One deletion the origin issued.
    Forget {
        /// The organization that issued the deletion.
        origin: OrganizationId,
        /// The canonical episode digest the deletion erases.
        episode_digest: [u8; 32],
        /// The origin's deletion sequence number.
        sequence: u64,
    },
}

/// The importing organization's foreign namespace.
pub(crate) struct ForeignLedger {
    appender: Appender,
    records: BTreeMap<(OrganizationId, [u8; 32]), StoredForeign>,
    forgotten: BTreeSet<(OrganizationId, [u8; 32])>,
    watermarks: BTreeMap<OrganizationId, u64>,
    sequence: u64,
}

impl ForeignLedger {
    /// Open (and create) the foreign namespace of one peer.
    pub(crate) fn open(directory: &Path) -> Result<Self, FederationError> {
        let directory = prepare_directory(directory)?;
        let path = directory.join(FOREIGN_FILE);
        let raw = read_bounded_optional(&path)?.unwrap_or_default();
        let appender = Appender::open(&path)?;
        let mut ledger = Self {
            appender,
            records: BTreeMap::new(),
            forgotten: BTreeSet::new(),
            watermarks: BTreeMap::new(),
            sequence: 0,
        };
        for line in complete_lines(&raw) {
            if line.is_empty() {
                continue;
            }
            let parsed: ForeignLine =
                serde_json::from_slice(line).map_err(|_| FederationError::Corrupt)?;
            ledger.apply(parsed);
        }
        Ok(ledger)
    }

    /// Apply one line to the in-memory state.
    fn apply(&mut self, line: ForeignLine) {
        match line {
            ForeignLine::Record { record, checkpoint } => {
                let key = (record.origin.clone(), record.episode_digest);
                if self.records.contains_key(&key) {
                    return;
                }
                let sequence = self.sequence;
                self.sequence = self.sequence.saturating_add(1);
                self.records.insert(
                    key,
                    StoredForeign {
                        record,
                        checkpoint,
                        sequence,
                    },
                );
            }
            ForeignLine::Forget {
                origin,
                episode_digest,
                sequence,
            } => {
                self.forgotten.insert((origin.clone(), episode_digest));
                let watermark = self.watermarks.entry(origin).or_insert(0);
                *watermark = (*watermark).max(sequence);
            }
        }
    }

    /// Whether the namespace already holds one record.
    pub(crate) fn contains(&self, origin: &OrganizationId, digest: &[u8; 32]) -> bool {
        self.records.contains_key(&(origin.clone(), *digest))
    }

    /// The stored form of one record.
    pub(crate) fn get(&self, origin: &OrganizationId, digest: &[u8; 32]) -> Option<&StoredForeign> {
        self.records.get(&(origin.clone(), *digest))
    }

    /// Whether the origin's deletion is known here.
    pub(crate) fn is_forgotten(&self, origin: &OrganizationId, digest: &[u8; 32]) -> bool {
        self.forgotten.contains(&(origin.clone(), *digest))
    }

    /// The deletion watermark applied for one origin.
    pub(crate) fn watermark(&self, origin: &OrganizationId) -> u64 {
        self.watermarks.get(origin).copied().unwrap_or(0)
    }

    /// Every stored record of one scope, ordered by origin and digest.
    pub(crate) fn records_in_scope(&self, scope: &str) -> Vec<&StoredForeign> {
        self.records
            .values()
            .filter(|stored| stored.record.scope == scope)
            .collect()
    }

    /// Append one imported record.
    pub(crate) fn append_record(
        &mut self,
        record: &ForeignRecord,
        checkpoint: u64,
    ) -> Result<(), FederationError> {
        let line = ForeignLine::Record {
            record: record.clone(),
            checkpoint,
        };
        let mut bytes = serde_json::to_vec(&line).map_err(|_| FederationError::Corrupt)?;
        bytes.shrink_to_fit();
        self.appender.append_line(&bytes)?;
        self.apply(line);
        Ok(())
    }

    /// Append one deletion applied from the origin.
    pub(crate) fn append_forget(
        &mut self,
        origin: &OrganizationId,
        digest: &[u8; 32],
        sequence: u64,
    ) -> Result<(), FederationError> {
        let line = ForeignLine::Forget {
            origin: origin.clone(),
            episode_digest: *digest,
            sequence,
        };
        let mut bytes = serde_json::to_vec(&line).map_err(|_| FederationError::Corrupt)?;
        bytes.shrink_to_fit();
        self.appender.append_line(&bytes)?;
        self.apply(line);
        Ok(())
    }
}
