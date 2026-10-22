//! The importing peer's durable import state.
//!
//! The state is the peer's record of *reconciliation*, not of content: the
//! newest checkpoint it has completed per origin, the bundle digest it
//! completed at each recent checkpoint, the deletion watermark it has
//! applied per origin, and the equivocations it has detected.
//!
//! It is a small, content-free, checksummed file that is replaced
//! atomically, exactly like the quorum shard's sidecar. It holds no episode
//! content, no payload, and no organization name beyond the opaque
//! organization identifiers the peer already knows.
//!
//! # Two files, one order
//!
//! The foreign namespace (the records themselves) is written first and the
//! state sidecar second. A crash between the two leaves records that the
//! state does not yet know about: the next import of the same bundle skips
//! them as duplicates and then completes the checkpoint. Nothing is lost and
//! nothing is imported twice.

use crate::v3::federation::error::FederationError;
use crate::v3::federation::identity::OrganizationId;
use crate::v3::federation::sidecar::{checksum, read_bounded_optional, write_atomic};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The import state file of one importing peer.
pub(crate) const STATE_FILE: &str = "federation.v1.state.json";

/// The profile this file belongs to.
const PROFILE: &str = "wdbx-federation-state-v1";

/// How many recent checkpoints are remembered per origin.
///
/// The bound keeps the sidecar small. A checkpoint that has fallen out of the
/// window is still protected by the monotonic checkpoint rule: a bundle for
/// it is refused as a regression rather than admitted.
const MAX_CHECKPOINTS_PER_ORIGIN: usize = 64;

/// How many detected equivocations are remembered.
const MAX_CONFLICTS: usize = 64;

/// One completed checkpoint of one origin.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CheckpointEntry {
    /// The bundle digest that was accepted at this checkpoint.
    pub(crate) bundle_digest: [u8; 32],
    /// Whether this checkpoint is disputed by an equivocation.
    pub(crate) conflicted: bool,
}

/// The reconciliation state of one origin.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct OriginState {
    /// The newest checkpoint completed for this origin.
    pub(crate) highest_checkpoint: u64,
    /// The recent checkpoints and the bundle digests they accepted.
    pub(crate) checkpoints: BTreeMap<u64, CheckpointEntry>,
    /// The newest deletion watermark applied for this origin.
    pub(crate) applied_watermark: u64,
}

/// One detected equivocation.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConflictRecord {
    /// The organization that equivocated.
    pub(crate) origin: OrganizationId,
    /// The checkpoint both bundles claimed.
    pub(crate) checkpoint: u64,
    /// The bundle digest already accepted at that checkpoint.
    pub(crate) recorded_digest: [u8; 32],
    /// The bundle digest that contradicted it.
    pub(crate) presented_digest: [u8; 32],
}

/// The body of the import state file.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct StateBody {
    profile: String,
    origins: BTreeMap<String, OriginState>,
    conflicts: Vec<ConflictRecord>,
}

/// A sidecar file with a checksum over its own body.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Checksummed<T> {
    body: T,
    checksum: [u8; 32],
}

/// The importing peer's durable reconciliation state.
#[derive(Clone, Debug, Default)]
pub(crate) struct ImportState {
    origins: BTreeMap<OrganizationId, OriginState>,
    conflicts: Vec<ConflictRecord>,
}

impl ImportState {
    /// Read the import state, or the state of a fresh importer.
    pub(crate) fn load(path: &Path) -> Result<Self, FederationError> {
        let Some(raw) = read_bounded_optional(path)? else {
            return Ok(Self::default());
        };
        let file: Checksummed<StateBody> =
            serde_json::from_slice(&raw).map_err(|_| FederationError::Corrupt)?;
        let body = serde_json::to_vec(&file.body).map_err(|_| FederationError::Corrupt)?;
        if file.checksum != checksum(&body) {
            return Err(FederationError::Corrupt);
        }
        if file.body.profile != PROFILE {
            return Err(FederationError::Corrupt);
        }
        let mut origins = BTreeMap::new();
        for (key, value) in file.body.origins {
            let origin = OrganizationId::new(&key).map_err(|_| FederationError::Corrupt)?;
            origins.insert(origin, value);
        }
        Ok(Self {
            origins,
            conflicts: file.body.conflicts,
        })
    }

    /// Replace the import state atomically.
    pub(crate) fn store(&self, path: &Path) -> Result<(), FederationError> {
        let body = StateBody {
            profile: PROFILE.to_string(),
            origins: self
                .origins
                .iter()
                .map(|(origin, state)| (origin.as_str().to_string(), state.clone()))
                .collect(),
            conflicts: self.conflicts.clone(),
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

    /// The newest checkpoint completed for one origin.
    pub(crate) fn highest_checkpoint(&self, origin: &OrganizationId) -> u64 {
        self.origins
            .get(origin)
            .map_or(0, |state| state.highest_checkpoint)
    }

    /// The newest deletion watermark applied for one origin.
    pub(crate) fn applied_watermark(&self, origin: &OrganizationId) -> u64 {
        self.origins
            .get(origin)
            .map_or(0, |state| state.applied_watermark)
    }

    /// The bundle digest accepted at one checkpoint of one origin.
    pub(crate) fn checkpoint_entry(
        &self,
        origin: &OrganizationId,
        checkpoint: u64,
    ) -> Option<&CheckpointEntry> {
        self.origins
            .get(origin)
            .and_then(|state| state.checkpoints.get(&checkpoint))
    }

    /// Whether a checkpoint of one origin is disputed.
    pub(crate) fn is_conflicted(&self, origin: &OrganizationId, checkpoint: u64) -> bool {
        self.checkpoint_entry(origin, checkpoint)
            .is_some_and(|entry| entry.conflicted)
    }

    /// Record a completed checkpoint.
    pub(crate) fn complete(
        &mut self,
        origin: &OrganizationId,
        checkpoint: u64,
        bundle_digest: [u8; 32],
        watermark: u64,
    ) {
        let state = self.origins.entry(origin.clone()).or_default();
        state.highest_checkpoint = state.highest_checkpoint.max(checkpoint);
        state.applied_watermark = state.applied_watermark.max(watermark);
        state.checkpoints.insert(
            checkpoint,
            CheckpointEntry {
                bundle_digest,
                conflicted: false,
            },
        );
        while state.checkpoints.len() > MAX_CHECKPOINTS_PER_ORIGIN {
            let Some(oldest) = state.checkpoints.keys().next().copied() else {
                break;
            };
            state.checkpoints.remove(&oldest);
        }
    }

    /// Record an equivocation and dispute the checkpoint it happened at.
    pub(crate) fn record_conflict(
        &mut self,
        origin: &OrganizationId,
        checkpoint: u64,
        recorded_digest: [u8; 32],
        presented_digest: [u8; 32],
    ) {
        let state = self.origins.entry(origin.clone()).or_default();
        state
            .checkpoints
            .entry(checkpoint)
            .and_modify(|entry| entry.conflicted = true)
            .or_insert(CheckpointEntry {
                bundle_digest: recorded_digest,
                conflicted: true,
            });
        self.conflicts.push(ConflictRecord {
            origin: origin.clone(),
            checkpoint,
            recorded_digest,
            presented_digest,
        });
        while self.conflicts.len() > MAX_CONFLICTS {
            self.conflicts.remove(0);
        }
    }

    /// The detected equivocations, oldest first.
    pub(crate) fn conflicts(&self) -> &[ConflictRecord] {
        &self.conflicts
    }
}
