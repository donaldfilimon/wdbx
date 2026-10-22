//! Independent-organization federation for the v3 episode ledger.
//!
//! This module adds a *federation slice* beside the episode ledger: two
//! independent organizations, each with its own canonical writer, exchanging
//! evidence across an explicit bilateral trust relation.
//!
//! It answers the global constraint that "federation supports independent
//! organizations with explicit bilateral trust and scoped grants" and that
//! "foreign evidence retains its origin authority".
//!
//! # The shape
//!
//! ```text
//! organization A                          organization B
//! ──────────────                          ──────────────
//! EpisodeStore (canonical)                EpisodeStore (canonical)
//!      │                                        │
//!      │ projection                             │ projection
//!      ▼                                        ▼
//! FederationExporter  ──── ExportBundle ───▶  FederationImporter
//! ```
//!
//! - A [`PeerDirectory`] enrolls the peers this organization knows. A peer's
//!   identity is *derived* from its key, so a peer cannot claim an identity
//!   its key does not match, and a rotated key is a new peer identity.
//! - A [`TrustStore`] holds this organization's *bilateral* trust records.
//!   Each record names one peer and carries a [`ShareGrant`]: the guild
//!   scopes, the evidence classes, the direction (export, import, or both),
//!   and the residency class the counterparty must satisfy. There is no
//!   global allow-list and no transitive trust.
//! - A [`FederationExporter`] turns the organization's own records into a
//!   deterministic [`ExportBundle`]: a manifest naming the origin
//!   organization, the exporting peer, the covered scope, the checkpoint, the
//!   deletion watermark, and the digests of the included records, plus the
//!   records themselves and the exporter's signature.
//! - A [`FederationImporter`] verifies a bundle before anything is admitted
//!   and keeps what it admits in its own foreign namespace.
//!
//! # Origin authority
//!
//! An imported record keeps the authority of the organization that authored
//! it. It is *not* a record of the importing organization, and it is never
//! written into the importing organization's canonical episode ledger: that
//! ledger has one writer, and its writer is the importing organization.
//! Letting a foreign record into that ledger would let the importing
//! organization author, replay, or re-sign an episode that belongs to
//! another organization. A local reference to a foreign record is an
//! [`ImportReceipt`], shaped like the complete-episode prototype's
//! `ForeignEvidenceRef`.
//!
//! # What is verified before admission
//!
//! Nothing is admitted until all of the following hold: the bundle's
//! manifest is internally consistent; the exporter is enrolled and its
//! signature verifies; the exporter is the peer the manifest names and the
//! origin is the exporter's organization; a live bilateral trust covers the
//! exporter and its grant covers the direction, the scope, the evidence
//! classes, and the residency class; every record's digest re-derives and
//! the manifest describes exactly the records the bundle carries; every
//! ancestry parent is either in the bundle or already held; the checkpoint
//! is neither a replay of nor a regression behind a completed checkpoint;
//! and the deletion watermark is not older than the one already applied and
//! does not skip a deletion this peer has not applied.
//!
//! # Equivocation
//!
//! If one origin presents two different bundles at the same checkpoint, the
//! importer refuses the second one, records the conflict, and *withholds the
//! disputed checkpoint from its serving view*: the records that arrived in
//! it are no longer served, because the origin that produced them has been
//! shown to contradict itself. The origin's own exporter refuses to
//! equivocate against itself — a second export at a bound checkpoint with
//! different content is refused — but that binding is per origin *host*, so
//! two hosts of one organization can still equivocate against each other.
//! That is the case the importer detects.
//!
//! # Deletion propagation
//!
//! The origin keeps a deletion log: an ordered, content-free sequence of
//! the deletions it has issued. A bundle carries the origin's deletion
//! watermark and the deletions up to it, and the importer applies them,
//! marking the imported copies forgotten so they are no longer served. A
//! bundle whose watermark is *older* than the watermark already applied is
//! refused, so a stale bundle cannot resurrect a deleted record, and a
//! bundle whose watermark *skips* a deletion the importer has not applied is
//! refused too: applying that watermark would mark an unseen deletion as
//! applied, and the deleted record could survive here forever.
//!
//! **The honest limit:** deletion propagation is pull-based. A peer that
//! never receives a bundle carrying the deletion keeps serving the deleted
//! record, and nothing here can make a peer learn about a deletion it is
//! never told about. There is no gossip, no tombstone broadcast, and no
//! proof that a peer has seen a deletion.
//!
//! # Ancestry
//!
//! A record that was derived from another record keeps the parent's origin
//! and digest. If the importing organization has neither received the parent
//! nor seen it before, the import is refused as incomplete: ancestry is never
//! invented, and a record whose derivation is unknown is not admitted.
//!
//! # What this is not
//!
//! - **It is not a ratified profile.** These bytes have no specification,
//!   no transport, and no stability promise. No consumer may depend on them.
//! - **It is not Byzantine fault tolerant.** An origin that equivocates is
//!   *detected*, and its disputed checkpoint is withheld. An origin that
//!   lies about something else — about a record it never issued, about a
//!   deletion it never made — is outside the model.
//! - **It has no transport.** A bundle is a value passed in process. There
//!   is no network, no TLS, no handshake, no certificate exchange, and no
//!   discovery.
//! - **It has no key custody.** Keys live in memory and are forgotten at
//!   exit. There is no key store, no rotation ceremony, no revocation
//!   service, and no certificate authority. The peer directory and the trust
//!   store are local declarations.
//! - **It does not federate payloads.** The v3 episode ledger commits to a
//!   payload rather than storing it, so a record carries a commitment and
//!   its content-free metadata. Nothing else travels. An importing
//!   organization therefore cannot re-derive the origin's canonical episode
//!   digest from the bundle: the origin's signature over the bundle digest
//!   is what binds the carried digest to the origin's authority.
//! - **It is not a relay.** An organization may not export a record it did
//!   not author, so trust never becomes transitive and evidence never
//!   changes hands twice.
//! - **Residency is a declaration.** The closed residency vocabulary
//!   models the *shape* of a residency restriction; nothing here observes
//!   where a peer actually runs, so the restriction is an administrative
//!   rule, not an enforcement mechanism.
//! - **One host, one writer.** The importing organization keeps one foreign
//!   namespace per directory, and the exporting organization keeps one
//!   deletion log and one set of export bindings per directory. Concurrent
//!   writers to one namespace are not fenced by a term, unlike the quorum
//!   shard's voters.
//!
//! # Evidence
//!
//! `crates/abi-wdbx/tests/v3_federation.rs` pins the behaviour this module
//! claims: trust refusal, grant scope refusal, a round trip that keeps the
//! origin's authority, replay idempotence, regression refusal, equivocation
//! detection, deduplication, ancestry completeness, resumable imports, and
//! deletion propagation. Everything is deterministic and local: no network,
//! no wall-clock dependence, and explicit checkpoint and watermark numbers.

mod bundle;
mod canonical;
mod error;
mod exporter;
mod foreign;
mod grant;
mod identity;
mod importer;
mod origin;
mod record;
mod sidecar;
mod state;

#[cfg(test)]
mod tests;

pub use bundle::{BUNDLE_PROFILE, BundleManifest, BundleSignature, DeletionEntry, ExportBundle};
pub use error::FederationError;
pub use exporter::FederationExporter;
pub use grant::{BilateralTrust, EvidenceClass, ShareDirection, ShareGrant, TrustStore};
pub use identity::{
    OrganizationId, PEER_ID_PREFIX, PeerDirectory, PeerId, PeerIdentity, PeerKey, PeerVerifyingKey,
    ResidencyClass,
};
pub use importer::{
    EquivocationConflict, FederationImporter, ForeignEvidenceRef, ForeignRecordState,
    ImportOutcome, ImportReceipt,
};
pub use record::{AncestryRef, ForeignRecord};
