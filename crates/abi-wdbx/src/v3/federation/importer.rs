//! The importing side of a federation exchange.
//!
//! The importer is one organization receiving another organization's
//! evidence. It verifies a bundle *before* anything is admitted, keeps the
//! admitted records in its own foreign namespace, and never writes them into
//! its canonical episode ledger.
//!
//! # The order of the checks
//!
//! Nothing is admitted until every one of these holds:
//!
//! 1. the bundle's profile, scope, and manifest are internally consistent;
//! 2. the exporter is enrolled, and the signature verifies under its key;
//! 3. the exporter is the peer the manifest names, and the origin is the
//!    exporter's organization;
//! 4. a live bilateral trust covers the exporter, and its grant covers the
//!    direction, the scope, the evidence classes, and the residency class;
//! 5. every record's digest re-derives and the manifest describes exactly
//!    the records the bundle carries;
//! 6. every ancestry parent is either in the bundle or already held here;
//! 7. the checkpoint is not a replay of a completed checkpoint and not a
//!    regression behind one, and the origin has not equivocated at it;
//! 8. the deletion watermark is not older than the one already applied.
//!
//! # The origin keeps the authority
//!
//! An imported record keeps the authority of the organization that authored
//! it: its own organization, its own scope, its own canonical episode
//! digest. A local reference to it is an [`ImportReceipt`], never a claim
//! that this organization wrote it.
//!
//! # What the importer does not do
//!
//! It does not fetch anything, and it does not talk to the exporting peer.
//! It has no transport, no handshake, and no key directory: the peer
//! directory and the trust store are local, in-memory declarations.

use crate::v3::federation::bundle::{DeletionEntry, ExportBundle, check_manifest};
use crate::v3::federation::error::FederationError;
use crate::v3::federation::foreign::ForeignLedger;
use crate::v3::federation::grant::{ShareDirection, TrustStore};
use crate::v3::federation::identity::{OrganizationId, PeerDirectory, PeerIdentity};
use crate::v3::federation::record::{AncestryRef, ForeignRecord};
use crate::v3::federation::sidecar::prepare_directory;
use crate::v3::federation::state::{ImportState, STATE_FILE};
use ed25519_dalek::Signature;
use std::path::{Path, PathBuf};

/// The outcome of one import attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportOutcome {
    /// The bundle was admitted.
    Imported {
        /// How many records were newly admitted.
        applied: usize,
        /// How many records were already held and were skipped.
        skipped: usize,
    },
    /// The bundle had already been imported; nothing changed.
    AlreadyImported,
    /// The import stopped at the caller's limit and can be resumed.
    Interrupted {
        /// How many records were newly admitted.
        applied: usize,
        /// How many records were already held and were skipped.
        skipped: usize,
        /// How many records of the bundle are still not held.
        remaining: usize,
    },
}

/// The receipt of one imported record.
///
/// The receipt is the *only* way this organization refers to a foreign
/// record. It names the origin organization, the origin's scope, the
/// origin's canonical episode digest, and the checkpoint the record arrived
/// in. It never claims this organization authored the record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportReceipt {
    /// The organization that authored the record.
    pub origin: OrganizationId,
    /// The authoring organization's guild scope.
    pub scope: String,
    /// The authoring organization's canonical episode digest.
    pub episode_digest: [u8; 32],
    /// The checkpoint the record arrived in.
    pub checkpoint: u64,
    /// The order in which the record was imported.
    pub imported_sequence: u64,
}

/// A local reference to a foreign record.
///
/// This mirrors the complete-episode prototype's foreign-evidence
/// vocabulary: an origin organization, the origin's scope binding, the
/// origin's digest, and the local import receipt. The prototype's own type
/// is crate-private and is not used here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignEvidenceRef {
    /// The organization that authored the evidence.
    pub origin_organization: OrganizationId,
    /// The authoring organization's guild scope.
    pub origin_scope: String,
    /// The authoring organization's canonical episode digest.
    pub episode_digest: [u8; 32],
    /// The local receipt of the import.
    pub import_receipt: ImportReceipt,
}

/// One detected equivocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EquivocationConflict {
    /// The organization that equivocated.
    pub origin: OrganizationId,
    /// The checkpoint both bundles claimed.
    pub checkpoint: u64,
    /// The bundle digest already accepted at that checkpoint.
    pub recorded_digest: [u8; 32],
    /// The bundle digest that contradicted it.
    pub presented_digest: [u8; 32],
}

/// The state of one imported record in this organization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignRecordState {
    /// The origin's deletion is known here, so the record is not served.
    pub forgotten: bool,
    /// The checkpoint the record arrived in is disputed, so it is not served.
    pub conflicted: bool,
    /// The checkpoint the record arrived in.
    pub checkpoint: u64,
}

/// One organization's importing peer.
pub struct FederationImporter {
    directory: PathBuf,
    local: PeerIdentity,
    peers: PeerDirectory,
    trusts: TrustStore,
    foreign: ForeignLedger,
    state: ImportState,
}

impl FederationImporter {
    /// Open (or create) the importing peer's durable state.
    pub fn open(
        directory: impl AsRef<Path>,
        local: PeerIdentity,
        peers: PeerDirectory,
        trusts: TrustStore,
    ) -> Result<Self, FederationError> {
        let directory = prepare_directory(directory.as_ref())?;
        let foreign = ForeignLedger::open(&directory)?;
        let state = ImportState::load(&directory.join(STATE_FILE))?;
        Ok(Self {
            directory,
            local,
            peers,
            trusts,
            foreign,
            state,
        })
    }

    /// The importing peer's own identity.
    #[must_use]
    pub fn local(&self) -> &PeerIdentity {
        &self.local
    }

    /// The newest checkpoint completed for one origin.
    #[must_use]
    pub fn highest_checkpoint(&self, origin: &OrganizationId) -> u64 {
        self.state.highest_checkpoint(origin)
    }

    /// The newest deletion watermark applied for one origin.
    #[must_use]
    pub fn applied_deletion_watermark(&self, origin: &OrganizationId) -> u64 {
        self.state
            .applied_watermark(origin)
            .max(self.foreign.watermark(origin))
    }

    /// The equivocations this organization has detected.
    #[must_use]
    pub fn equivocation_conflicts(&self) -> Vec<EquivocationConflict> {
        self.state
            .conflicts()
            .iter()
            .map(|conflict| EquivocationConflict {
                origin: conflict.origin.clone(),
                checkpoint: conflict.checkpoint,
                recorded_digest: conflict.recorded_digest,
                presented_digest: conflict.presented_digest,
            })
            .collect()
    }

    /// Import one bundle, admitting every record it carries.
    pub fn import(&mut self, bundle: &ExportBundle) -> Result<ImportOutcome, FederationError> {
        self.import_limited(bundle, usize::MAX)
    }

    /// Import one bundle, admitting at most `max_records` new records.
    ///
    /// The limit exists so an interrupted import can be resumed: the records
    /// already admitted stay admitted, the checkpoint does not advance until
    /// the bundle is complete, and the next import of the same bundle skips
    /// what is already held and admits the rest.
    pub fn import_limited(
        &mut self,
        bundle: &ExportBundle,
        max_records: usize,
    ) -> Result<ImportOutcome, FederationError> {
        match self.verify(bundle)? {
            Verified::AlreadyImported => Ok(ImportOutcome::AlreadyImported),
            Verified::Admit(plan) => {
                let origin = bundle.manifest.origin.clone();
                let mut applied = 0_usize;
                let mut skipped = 0_usize;
                for record in &plan.records {
                    if self.foreign.contains(&origin, &record.episode_digest) {
                        skipped += 1;
                        continue;
                    }
                    if applied >= max_records {
                        return Ok(ImportOutcome::Interrupted {
                            applied,
                            skipped,
                            remaining: plan.records.len() - applied - skipped,
                        });
                    }
                    self.foreign.append_record(record, plan.checkpoint)?;
                    applied += 1;
                }
                for deletion in &plan.deletions {
                    self.foreign.append_forget(
                        &origin,
                        &deletion.episode_digest,
                        deletion.sequence,
                    )?;
                }
                self.state.complete(
                    &origin,
                    plan.checkpoint,
                    plan.bundle_digest,
                    plan.deletion_watermark,
                );
                self.state.store(&self.directory.join(STATE_FILE))?;
                Ok(ImportOutcome::Imported { applied, skipped })
            }
        }
    }

    /// The record this organization serves for one origin digest.
    ///
    /// `None` means the record is unknown, or it is not served: either the
    /// origin's deletion is known here, or the checkpoint it arrived in is
    /// disputed by an equivocation.
    #[must_use]
    pub fn served_record(
        &self,
        origin: &OrganizationId,
        episode_digest: &[u8; 32],
    ) -> Option<&ForeignRecord> {
        let stored = self.foreign.get(origin, episode_digest)?;
        if self.foreign.is_forgotten(origin, episode_digest) {
            return None;
        }
        if self.state.is_conflicted(origin, stored.checkpoint) {
            return None;
        }
        Some(&stored.record)
    }

    /// Every record this organization serves for one scope.
    ///
    /// The scope is the *origin's* scope: this organization does not merge a
    /// foreign scope into one of its own.
    #[must_use]
    pub fn served_records(&self, scope: &str) -> Vec<&ForeignRecord> {
        self.foreign
            .records_in_scope(scope)
            .into_iter()
            .filter(|stored| {
                !self
                    .foreign
                    .is_forgotten(&stored.record.origin, &stored.record.episode_digest)
                    && !self
                        .state
                        .is_conflicted(&stored.record.origin, stored.checkpoint)
            })
            .map(|stored| &stored.record)
            .collect()
    }

    /// The state of one imported record.
    #[must_use]
    pub fn record_state(
        &self,
        origin: &OrganizationId,
        episode_digest: &[u8; 32],
    ) -> Option<ForeignRecordState> {
        let stored = self.foreign.get(origin, episode_digest)?;
        Some(ForeignRecordState {
            forgotten: self.foreign.is_forgotten(origin, episode_digest),
            conflicted: self.state.is_conflicted(origin, stored.checkpoint),
            checkpoint: stored.checkpoint,
        })
    }

    /// The record one foreign record was derived from.
    ///
    /// `None` means the record has no parent, or the record is unknown here.
    /// The parent is never invented: an import with an unknown parent is
    /// refused, so a parent that is reported here was either delivered in the
    /// bundle or was already held.
    #[must_use]
    pub fn ancestry(
        &self,
        origin: &OrganizationId,
        episode_digest: &[u8; 32],
    ) -> Option<AncestryRef> {
        self.foreign
            .get(origin, episode_digest)
            .and_then(|stored| stored.record.parent.clone())
    }

    /// The receipt of one imported record.
    #[must_use]
    pub fn import_receipt(
        &self,
        origin: &OrganizationId,
        episode_digest: &[u8; 32],
    ) -> Option<ImportReceipt> {
        let stored = self.foreign.get(origin, episode_digest)?;
        Some(ImportReceipt {
            origin: origin.clone(),
            scope: stored.record.scope.clone(),
            episode_digest: *episode_digest,
            checkpoint: stored.checkpoint,
            imported_sequence: stored.sequence,
        })
    }

    /// The local reference to one foreign record.
    #[must_use]
    pub fn foreign_evidence(
        &self,
        origin: &OrganizationId,
        episode_digest: &[u8; 32],
    ) -> Option<ForeignEvidenceRef> {
        let receipt = self.import_receipt(origin, episode_digest)?;
        Some(ForeignEvidenceRef {
            origin_organization: origin.clone(),
            origin_scope: receipt.scope.clone(),
            episode_digest: *episode_digest,
            import_receipt: receipt,
        })
    }

    /// Verify a bundle and decide what it would admit.
    fn verify(&mut self, bundle: &ExportBundle) -> Result<Verified, FederationError> {
        check_manifest(bundle)?;
        let signer = &bundle.signature.signer;
        let peer = self.peers.get(signer).ok_or(FederationError::UnknownPeer)?;
        if &bundle.manifest.exporter != signer {
            return Err(FederationError::ExporterMismatch);
        }
        let digest = bundle.digest();
        let signature = Signature::from_bytes(&bundle.signature.signature);
        if peer
            .key()
            .as_ed25519()
            .verify_strict(&digest, &signature)
            .is_err()
        {
            return Err(FederationError::SignatureInvalid);
        }
        if peer.organization() != &bundle.manifest.origin {
            return Err(FederationError::OriginMismatch);
        }
        let trust = self
            .trusts
            .get(signer)
            .ok_or(FederationError::TrustMissing)?;
        if trust.revoked() {
            return Err(FederationError::TrustRevoked);
        }
        let grant = trust.grant();
        if !grant.permits_direction(ShareDirection::Import) {
            return Err(FederationError::GrantDirectionMismatch);
        }
        if !grant.permits_scope(&bundle.manifest.scope) {
            return Err(FederationError::GrantScopeMismatch);
        }
        if bundle
            .records
            .iter()
            .any(|record| !grant.permits_evidence(record.evidence))
        {
            return Err(FederationError::GrantEvidenceMismatch);
        }
        if !peer.residency().satisfies(grant.residency())
            || !self.local.residency().satisfies(grant.residency())
        {
            return Err(FederationError::ResidencyMismatch);
        }
        self.check_ancestry(bundle)?;
        match self.check_checkpoint(bundle, digest)? {
            CheckpointVerdict::Proceed => {}
            CheckpointVerdict::AlreadyImported => return Ok(Verified::AlreadyImported),
        }
        let origin = &bundle.manifest.origin;
        let watermark = bundle.manifest.deletion_watermark;
        let applied = self.applied_deletion_watermark(origin);
        if watermark < applied {
            return Err(FederationError::StaleWatermark);
        }
        // Every deletion this peer has not applied yet must be present in the
        // bundle. A watermark that skipped one would mark it applied without
        // ever seeing it, and the deleted record could survive here forever.
        for sequence in applied.saturating_add(1)..=watermark {
            if !bundle
                .manifest
                .deletions
                .iter()
                .any(|deletion| deletion.sequence == sequence)
            {
                return Err(FederationError::DeletionGap);
            }
        }
        Ok(Verified::Admit(VerifiedPlan {
            checkpoint: bundle.manifest.checkpoint,
            bundle_digest: digest,
            deletion_watermark: watermark,
            records: bundle.records.clone(),
            deletions: bundle.manifest.deletions.clone(),
        }))
    }

    /// Every ancestry parent must be in the bundle or already held.
    fn check_ancestry(&self, bundle: &ExportBundle) -> Result<(), FederationError> {
        for record in &bundle.records {
            let Some(parent) = &record.parent else {
                continue;
            };
            let in_bundle = bundle.records.iter().any(|candidate| {
                candidate.origin == parent.origin
                    && candidate.episode_digest == parent.episode_digest
            });
            if in_bundle {
                continue;
            }
            if self
                .foreign
                .contains(&parent.origin, &parent.episode_digest)
            {
                continue;
            }
            return Err(FederationError::AncestryIncomplete);
        }
        Ok(())
    }

    /// The checkpoint must be new, and the origin must not have equivocated.
    fn check_checkpoint(
        &mut self,
        bundle: &ExportBundle,
        digest: [u8; 32],
    ) -> Result<CheckpointVerdict, FederationError> {
        let origin = bundle.manifest.origin.clone();
        let checkpoint = bundle.manifest.checkpoint;
        let highest = self.state.highest_checkpoint(&origin);
        let Some(entry) = self.state.checkpoint_entry(&origin, checkpoint) else {
            if checkpoint <= highest {
                return Err(FederationError::StaleCheckpoint);
            }
            return Ok(CheckpointVerdict::Proceed);
        };
        if entry.conflicted {
            return Err(FederationError::Equivocation);
        }
        if entry.bundle_digest == digest {
            return if checkpoint == highest {
                Ok(CheckpointVerdict::AlreadyImported)
            } else {
                Err(FederationError::StaleCheckpoint)
            };
        }
        self.state
            .record_conflict(&origin, checkpoint, entry.bundle_digest, digest);
        self.state.store(&self.directory.join(STATE_FILE))?;
        Err(FederationError::Equivocation)
    }
}

/// What the checkpoint check decided.
enum CheckpointVerdict {
    /// The checkpoint is new and may be admitted.
    Proceed,
    /// The bundle was already imported at this checkpoint.
    AlreadyImported,
}

/// What a verified bundle would admit.
enum Verified {
    Admit(VerifiedPlan),
    AlreadyImported,
}

/// The records, checkpoint, and deletions a verified bundle carries.
struct VerifiedPlan {
    checkpoint: u64,
    bundle_digest: [u8; 32],
    deletion_watermark: u64,
    records: Vec<ForeignRecord>,
    deletions: Vec<DeletionEntry>,
}
