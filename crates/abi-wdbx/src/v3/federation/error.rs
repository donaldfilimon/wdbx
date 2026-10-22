//! Closed failure vocabulary for the federation slice.
//!
//! Every variant is a static, content-free label. No variant carries bytes,
//! key identifiers, digests, organization names, scopes, or paths: an error
//! says *what* failed, never *what* it failed to. An error string is safe to
//! log, and it is not a channel for peer-supplied content.

use thiserror::Error;

/// Closed federation failures.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum FederationError {
    /// A federation file or directory operation failed.
    #[error("federation_io")]
    Io,
    /// A durable federation file is malformed, truncated, or fails its checksum.
    #[error("federation_corrupt")]
    Corrupt,
    /// An identifier, record, grant, or argument is outside the bounded profile.
    #[error("federation_invalid_input")]
    InvalidInput,
    /// The peer that signed a bundle is not enrolled in the local directory.
    #[error("federation_unknown_peer")]
    UnknownPeer,
    /// No bilateral trust record exists for the peer.
    #[error("federation_trust_missing")]
    TrustMissing,
    /// The bilateral trust record for the peer has been revoked.
    #[error("federation_trust_revoked")]
    TrustRevoked,
    /// The trust grant does not permit the direction of the exchange.
    #[error("federation_grant_direction_mismatch")]
    GrantDirectionMismatch,
    /// The trust grant does not cover the scope of the exchange.
    #[error("federation_grant_scope_mismatch")]
    GrantScopeMismatch,
    /// The trust grant does not cover the evidence class of the exchange.
    #[error("federation_grant_evidence_mismatch")]
    GrantEvidenceMismatch,
    /// A residency restriction on the trust grant is not satisfied.
    #[error("federation_residency_mismatch")]
    ResidencyMismatch,
    /// The bundle's signature does not verify under the named peer's key.
    #[error("federation_signature_invalid")]
    SignatureInvalid,
    /// The bundle names an exporter other than the one that signed it.
    #[error("federation_exporter_mismatch")]
    ExporterMismatch,
    /// The bundle's origin organization is not the signer's organization.
    #[error("federation_origin_mismatch")]
    OriginMismatch,
    /// A record digest does not re-derive, or the manifest does not describe it.
    #[error("federation_digest_mismatch")]
    DigestMismatch,
    /// The bundle mixes scopes, or names a scope it does not carry.
    #[error("federation_scope_mismatch")]
    ScopeMismatch,
    /// The bundle's checkpoint is older than the newest one already imported.
    #[error("federation_stale_checkpoint")]
    StaleCheckpoint,
    /// The same origin presented two different bundles at one checkpoint.
    #[error("federation_equivocation")]
    Equivocation,
    /// A record names a parent this peer has neither been sent nor seen.
    #[error("federation_ancestry_incomplete")]
    AncestryIncomplete,
    /// The bundle's deletion watermark is older than the one already applied.
    #[error("federation_stale_watermark")]
    StaleWatermark,
    /// The bundle's deletion watermark skips a deletion this peer has not
    /// applied, so applying the watermark would mark an unseen deletion as
    /// applied.
    #[error("federation_deletion_gap")]
    DeletionGap,
    /// A bounded collection, file, or counter limit was reached.
    #[error("federation_budget_exhausted")]
    BudgetExhausted,
}
