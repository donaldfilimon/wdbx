//! The exporting side of a federation exchange.
//!
//! The exporter is one organization's canonical writer preparing records for
//! another organization. It holds the organization's key, its peer directory,
//! its bilateral trust records, and its durable deletion log.
//!
//! # What the exporter refuses
//!
//! - a destination the organization does not know, or has no live bilateral
//!   trust with;
//! - a record whose scope the trust grant does not cover;
//! - a record whose evidence class the trust grant does not cover;
//! - a record the organization did not author: an organization cannot export
//!   another organization's record under its own authority, so there is no
//!   transitive federation and no relay;
//! - a second export at a checkpoint it has already bound to different
//!   content: the origin refuses to equivocate against itself.
//!
//! # What the exporter does not do
//!
//! It does not fetch anything, talk to anybody, or decide what the peer will
//! do with the bundle. It produces bytes; the importing organization decides
//! whether to admit them.

use crate::v3::federation::bundle::{
    BUNDLE_PROFILE, BundleManifest, BundleSignature, ExportBundle,
};
use crate::v3::federation::error::FederationError;
use crate::v3::federation::grant::{ShareDirection, TrustStore};
use crate::v3::federation::identity::{PeerDirectory, PeerId, PeerIdentity, PeerKey};
use crate::v3::federation::origin::{DeletionLog, ExportBindings, export_bindings_path};
use crate::v3::federation::record::ForeignRecord;
use crate::v3::federation::sidecar::prepare_directory;
use std::path::{Path, PathBuf};

/// One organization's exporting peer.
pub struct FederationExporter {
    directory: PathBuf,
    local: PeerIdentity,
    key: PeerKey,
    peers: PeerDirectory,
    trusts: TrustStore,
    deletions: DeletionLog,
    bindings: ExportBindings,
}

impl FederationExporter {
    /// Open (or create) the exporting peer's durable state.
    ///
    /// The key must be the peer's own key: the exporter signs with it, and
    /// a bundle signed by another key would not carry this organization's
    /// authority.
    pub fn open(
        directory: impl AsRef<Path>,
        local: PeerIdentity,
        key: PeerKey,
        peers: PeerDirectory,
        trusts: TrustStore,
    ) -> Result<Self, FederationError> {
        if local.peer() != &key.peer_id() {
            return Err(FederationError::InvalidInput);
        }
        let directory = prepare_directory(directory.as_ref())?;
        let deletions = DeletionLog::open(&directory)?;
        let bindings = ExportBindings::load(&export_bindings_path(&directory))?;
        Ok(Self {
            directory,
            local,
            key,
            peers,
            trusts,
            deletions,
            bindings,
        })
    }

    /// The exporting peer's own identity.
    #[must_use]
    pub fn local(&self) -> &PeerIdentity {
        &self.local
    }

    /// The origin's current deletion watermark.
    #[must_use]
    pub fn deletion_watermark(&self) -> u64 {
        self.deletions.watermark()
    }

    /// Record a deletion the origin has issued.
    ///
    /// The origin's deletion watermark advances only when a new deletion is
    /// issued. A repeated notice is idempotent, so a bundle never carries
    /// the same deletion twice under two numbers.
    pub fn record_deletion(&mut self, episode_digest: &[u8; 32]) -> Result<u64, FederationError> {
        self.deletions.record(episode_digest)
    }

    /// The bundle digest this origin bound to one checkpoint.
    #[must_use]
    pub fn export_binding(&self, checkpoint: u64) -> Option<[u8; 32]> {
        self.bindings
            .get(checkpoint)
            .map(|binding| binding.bundle_digest)
    }

    /// Build the bundle one peer may hand to another.
    ///
    /// The bundle is a pure function of its inputs: the same records, the
    /// same scope, the same checkpoint, and the same deletion state produce
    /// byte-identical bundles, and the exporter's own signature is
    /// deterministic.
    pub fn export_bundle(
        &mut self,
        destination: &PeerId,
        records: &[ForeignRecord],
        checkpoint: u64,
    ) -> Result<ExportBundle, FederationError> {
        if records.is_empty() || checkpoint == 0 {
            return Err(FederationError::InvalidInput);
        }
        let peer = self
            .peers
            .get(destination)
            .ok_or(FederationError::UnknownPeer)?;
        let trust = self
            .trusts
            .get(destination)
            .ok_or(FederationError::TrustMissing)?;
        if trust.revoked() {
            return Err(FederationError::TrustRevoked);
        }
        let grant = trust.grant();
        if !grant.permits_direction(ShareDirection::Export) {
            return Err(FederationError::GrantDirectionMismatch);
        }
        let scope = records
            .first()
            .map(|record| record.scope.clone())
            .ok_or(FederationError::InvalidInput)?;
        if records.iter().any(|record| record.scope != scope) {
            return Err(FederationError::ScopeMismatch);
        }
        if !grant.permits_scope(&scope) {
            return Err(FederationError::GrantScopeMismatch);
        }
        if records
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
        if records
            .iter()
            .any(|record| record.origin != *self.local.organization())
        {
            return Err(FederationError::OriginMismatch);
        }

        let mut ordered = records.to_vec();
        ordered.sort_by_key(|record| record.episode_digest);
        let mut record_digests: Vec<[u8; 32]> = ordered.iter().map(ForeignRecord::digest).collect();
        record_digests.sort_unstable();
        let manifest = BundleManifest {
            origin: self.local.organization().clone(),
            exporter: self.local.peer().clone(),
            scope: scope.clone(),
            checkpoint,
            deletion_watermark: self.deletions.watermark(),
            record_digests,
            deletions: self.deletions.entries().to_vec(),
        };
        let mut bundle = ExportBundle {
            profile: BUNDLE_PROFILE.to_string(),
            manifest,
            records: ordered,
            signature: BundleSignature {
                signer: self.local.peer().clone(),
                signature: [0; 64],
            },
        };
        let digest = bundle.digest();
        bundle.signature.signature = self.key.sign(&digest);
        if let Some(binding) = self.bindings.get(checkpoint) {
            if binding.bundle_digest != digest {
                return Err(FederationError::Equivocation);
            }
            if binding.scope != scope {
                return Err(FederationError::ScopeMismatch);
            }
        }
        self.bindings.bind(checkpoint, &scope, digest);
        ExportBindings::store(&self.bindings, &export_bindings_path(&self.directory))?;
        Ok(bundle)
    }
}
