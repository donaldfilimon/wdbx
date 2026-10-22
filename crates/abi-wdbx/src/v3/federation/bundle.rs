//! The portable bundle: manifest, records, and the exporter's signature.
//!
//! A bundle is what one organization hands to another. It is
//! self-describing and self-verifying:
//!
//! - the manifest names the **origin organization**, the **exporting peer**,
//!   the covered **scope**, the **checkpoint**, the **deletion watermark**,
//!   the digests of the included records, and the deletions the origin has
//!   issued;
//! - the records themselves travel with the bundle;
//! - the exporter signs the bundle digest, so the origin, the exporter, the
//!   scope, the checkpoint, and the records are all covered by one
//!   signature.
//!
//! # Determinism
//!
//! A bundle is a pure function of its inputs. The records are ordered by
//! their canonical episode digest, the manifest's digest list is sorted, the
//! deletions are ordered by sequence, and Ed25519 signatures are
//! deterministic. The same records, scope, checkpoint, and deletion state
//! therefore produce byte-identical bundles, which is what makes a re-export
//! checkable rather than merely plausible.
//!
//! # Not a wire format
//!
//! These bytes are a prototype profile. Nothing has been ratified, no
//! transport is specified, and no consumer may depend on the encoding.

use crate::v3::federation::canonical::{
    BUNDLE_DOMAIN, MANIFEST_DOMAIN, put_digest, put_digest_list, put_text, put_u64, sha256,
};
use crate::v3::federation::error::FederationError;
use crate::v3::federation::identity::{OrganizationId, PeerId};
use crate::v3::federation::record::ForeignRecord;
use serde::{Deserialize, Serialize};

/// The profile these bytes belong to.
pub const BUNDLE_PROFILE: &str = "wdbx-federation-bundle-v1";

/// One deletion the origin has issued.
///
/// The sequence number is the origin's deletion watermark counter: it orders
/// the origin's deletions and lets an importing organization apply them
/// exactly once, in order.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeletionEntry {
    /// The origin's deletion sequence number.
    pub sequence: u64,
    /// The canonical episode digest the deletion erases.
    pub episode_digest: [u8; 32],
}

/// The manifest of a bundle.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BundleManifest {
    /// The organization the records originate from.
    pub origin: OrganizationId,
    /// The peer that produced the bundle.
    pub exporter: PeerId,
    /// The authoring organization's guild scope the bundle covers.
    pub scope: String,
    /// The origin's export checkpoint. It counts snapshots of one scope.
    pub checkpoint: u64,
    /// The origin's deletion watermark at the time of the export.
    pub deletion_watermark: u64,
    /// The federation digests of the included records, ascending.
    pub record_digests: Vec<[u8; 32]>,
    /// The deletions the origin has issued, ordered by sequence.
    pub deletions: Vec<DeletionEntry>,
}

impl BundleManifest {
    /// The manifest digest.
    ///
    /// The digest covers the origin, the exporter, the scope, the checkpoint,
    /// the deletion watermark, the record digests, and the deletions.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut body = Vec::new();
        put_text(&mut body, self.origin.as_str());
        put_text(&mut body, self.exporter.as_str());
        put_text(&mut body, &self.scope);
        put_u64(&mut body, self.checkpoint);
        put_u64(&mut body, self.deletion_watermark);
        put_digest_list(&mut body, &self.record_digests);
        put_u64(
            &mut body,
            u64::try_from(self.deletions.len()).unwrap_or(u64::MAX),
        );
        for deletion in &self.deletions {
            put_u64(&mut body, deletion.sequence);
            put_digest(&mut body, &deletion.episode_digest);
        }
        sha256(&[MANIFEST_DOMAIN, &body])
    }
}

/// The exporter's detached signature over the bundle digest.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BundleSignature {
    /// The peer that signed the bundle.
    pub signer: PeerId,
    /// The 64-byte Ed25519 signature over the bundle digest.
    #[serde(with = "signature_bytes")]
    pub signature: [u8; 64],
}

/// A portable bundle of records.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportBundle {
    /// The profile these bytes belong to.
    pub profile: String,
    /// The manifest.
    pub manifest: BundleManifest,
    /// The included records, ordered by canonical episode digest.
    pub records: Vec<ForeignRecord>,
    /// The exporter's signature.
    pub signature: BundleSignature,
}

impl ExportBundle {
    /// The bundle digest.
    ///
    /// The digest covers the manifest and the included records, so one
    /// signature over it binds the whole exchange.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut body = Vec::new();
        put_digest(&mut body, &self.manifest.digest());
        put_digest_list(&mut body, &self.records_digests());
        sha256(&[BUNDLE_DOMAIN, &body])
    }

    /// The federation digests of the included records, ascending.
    #[must_use]
    pub fn records_digests(&self) -> Vec<[u8; 32]> {
        self.records.iter().map(ForeignRecord::digest).collect()
    }

    /// The manifest's claim about the record digests matches the records.
    #[must_use]
    pub fn manifest_matches_records(&self) -> bool {
        let mut claimed = self.manifest.record_digests.clone();
        let mut actual = self.records_digests();
        claimed.sort_unstable();
        actual.sort_unstable();
        claimed == actual
    }

    /// The number of records the bundle carries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the bundle carries no record.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Whether the bundle belongs to the expected profile.
    #[must_use]
    pub fn has_profile(&self) -> bool {
        self.profile == BUNDLE_PROFILE
    }
}

/// Refuse a bundle whose manifest is internally inconsistent.
///
/// The check is cheap and local: the manifest must name the scope its
/// records belong to, its deletions must not run past its own watermark, and
/// its record digests must describe exactly the records it carries.
pub(crate) fn check_manifest(bundle: &ExportBundle) -> Result<(), FederationError> {
    if !bundle.has_profile() {
        return Err(FederationError::InvalidInput);
    }
    if bundle.records.is_empty() {
        return Err(FederationError::InvalidInput);
    }
    if !bundle.manifest_matches_records() {
        return Err(FederationError::DigestMismatch);
    }
    for record in &bundle.records {
        if record.scope != bundle.manifest.scope {
            return Err(FederationError::ScopeMismatch);
        }
        if record.origin != bundle.manifest.origin {
            return Err(FederationError::OriginMismatch);
        }
    }
    for deletion in &bundle.manifest.deletions {
        if deletion.sequence > bundle.manifest.deletion_watermark {
            return Err(FederationError::InvalidInput);
        }
    }
    Ok(())
}

mod signature_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(
        bytes: &[u8; 64],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(bytes.iter())
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; 64], D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("bundle signature must be 64 bytes"))
    }
}
