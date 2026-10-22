//! Independent-organization federation for the v3 episode ledger.
//!
//! These tests pin the behaviour the federation slice claims:
//!
//! - a peer without a live bilateral trust cannot import;
//! - a grant that does not cover a record's scope permits neither the export
//!   nor the import;
//! - a round trip keeps the *origin's* authority: the imported record is
//!   held in the importer's foreign namespace, under the origin's
//!   organization and the origin's scope, and never in the importer's own
//!   canonical episode ledger;
//! - a replayed bundle is a no-op, and a bundle behind the newest checkpoint
//!   is refused;
//! - two different bundles at one checkpoint are refused as an
//!   equivocation, and the dispute is reported;
//! - the same record under two bundles is imported once;
//! - a record whose parent is neither delivered nor known is refused as
//!   incomplete, and a delivered parent is preserved as ancestry;
//! - an interrupted import resumes from its last checkpoint without
//!   duplicating or losing a record, and its state survives a restart;
//! - a deletion at the origin propagates, and a stale watermark cannot
//!   resurrect what was deleted;
//! - a revoked bilateral trust stops authorizing the next import;
//! - one organization's records never enter another organization's ledger.
//!
//! Everything is deterministic and local: no network, no wall-clock
//! dependence, and explicit checkpoint and watermark numbers.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeSource, EpisodeStore, EpisodeWrite, EvidenceLevel,
    GuildEpisodePolicy, MemoryCandidate, MemoryClass, RetentionClass, StorePolicy,
};
use abi_wdbx::v3::federation::{
    AncestryRef, BilateralTrust, EvidenceClass, ExportBundle, FederationError, FederationExporter,
    FederationImporter, ForeignRecord, ImportOutcome, OrganizationId, PeerDirectory, PeerId,
    PeerIdentity, PeerKey, ResidencyClass, ShareDirection, ShareGrant, TrustStore,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[path = "v3_federation/deletion.rs"]
mod deletion;
#[path = "v3_federation/exchange.rs"]
mod exchange;
#[path = "v3_federation/reconciliation.rs"]
mod reconciliation;
#[path = "v3_federation/trust.rs"]
mod trust;

/// The scope the origin's records live in.
const SCOPE: &str = "guild_ref";

/// A second scope, used to prove that a grant covers one scope only.
const OTHER_SCOPE: &str = "other_guild";

pub(crate) struct Scratch(PathBuf);

impl Scratch {
    pub(crate) fn new(purpose: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("wdbx-federation-{purpose}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("scratch directory");
        Self(path)
    }

    pub(crate) fn child(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn guild_policy() -> GuildEpisodePolicy {
    GuildEpisodePolicy {
        learning_enabled: true,
        policy_version: "policy_v1".into(),
        token_budget: 1_000,
        storage_budget_bytes: 1024 * 1024,
        current_consent_epoch: None,
        retention_ttl_ms: None,
        retention_windows: None,
        authorized_principals: None,
    }
}

fn policy() -> StorePolicy {
    StorePolicy {
        contract_revision: 2,
        contract_digest: [9; 32],
        guilds: BTreeMap::from([
            (SCOPE.to_string(), guild_policy()),
            (OTHER_SCOPE.to_string(), guild_policy()),
        ]),
    }
}

pub(crate) fn organization(value: &str) -> OrganizationId {
    OrganizationId::new(value).expect("bounded organization")
}

/// One organization's peer identity and its key.
pub(crate) fn peer_with(
    seed: u8,
    organization: &OrganizationId,
    residency: ResidencyClass,
) -> (PeerKey, PeerIdentity) {
    let key = PeerKey::from_seed([seed; 32]);
    let identity = PeerIdentity::new(key.verifying(), organization.clone(), residency);
    (key, identity)
}

pub(crate) fn grant(scopes: &[&str], direction: ShareDirection) -> ShareGrant {
    grant_with(scopes, direction, ResidencyClass::Unrestricted)
}

pub(crate) fn grant_with(
    scopes: &[&str],
    direction: ShareDirection,
    residency: ResidencyClass,
) -> ShareGrant {
    ShareGrant::new(
        scopes.iter().map(|scope| (*scope).to_string()).collect(),
        BTreeSet::from([
            EvidenceClass::MemoryCandidate,
            EvidenceClass::MemoryEdge,
            EvidenceClass::Operation,
        ]),
        direction,
        residency,
    )
    .expect("bounded grant")
}

pub(crate) fn trust(peer: &PeerId, scopes: &[&str], direction: ShareDirection) -> BilateralTrust {
    BilateralTrust::new(peer.clone(), grant(scopes, direction), 1)
}

pub(crate) fn open_store(directory: &Path) -> EpisodeStore {
    EpisodeStore::open(directory, policy()).expect("episode store")
}

fn candidate(seed: u8) -> MemoryCandidate {
    MemoryCandidate {
        class: MemoryClass::Fact,
        retention: RetentionClass::Durable,
        payload_commitment: [seed; 32],
        payload_bytes: 64,
        dimension: None,
        embedding_version: None,
        member_scoped: false,
        supersedes: None,
        forgets: None,
    }
}

fn write(request_id: &str, candidate: MemoryCandidate) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: request_id.into(),
        contract_revision: 2,
        contract_digest: [9; 32],
        guild_ref: SCOPE.into(),
        consent_epoch: None,
        source_type: EpisodeSource::DiscordGuild,
        policy_version: "policy_v1".into(),
        evidence_level: EvidenceLevel::C1,
        event: EpisodeEvent::MemoryCandidate {
            recorded_by: ActorRef {
                principal_id: "abbey_service".into(),
                kind: ActorKind::Service,
            },
            candidate,
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}

/// Append one memory candidate and return its canonical digest.
pub(crate) fn append_candidate(store: &mut EpisodeStore, request_id: &str, seed: u8) -> [u8; 32] {
    append(store, write(request_id, candidate(seed)))
}

/// Append one candidate that supersedes another.
pub(crate) fn append_superseding(
    store: &mut EpisodeStore,
    request_id: &str,
    seed: u8,
    parent: [u8; 32],
) -> [u8; 32] {
    let mut superseding = candidate(seed);
    superseding.supersedes = Some(parent);
    append(store, write(request_id, superseding))
}

/// Append the tombstone that forgets one candidate.
pub(crate) fn append_forgetting(
    store: &mut EpisodeStore,
    request_id: &str,
    target: [u8; 32],
) -> [u8; 32] {
    let mut tombstone = candidate(0);
    tombstone.payload_commitment = [0; 32];
    tombstone.payload_bytes = 0;
    tombstone.forgets = Some(target);
    append(store, write(request_id, tombstone))
}

fn append(store: &mut EpisodeStore, mut event: EpisodeWrite) -> [u8; 32] {
    let preview = store
        .preview_commitment(&event)
        .expect("valid write previews");
    event.expected_commitment = Some(preview);
    store
        .propose_write(&event)
        .expect("valid write appends")
        .episode_digest
}

/// Project one local episode record into the portable record that travels.
pub(crate) fn foreign_record(
    origin: &OrganizationId,
    store: &EpisodeStore,
    episode_digest: &[u8; 32],
) -> ForeignRecord {
    let receipt = store
        .find_receipt(SCOPE, episode_digest)
        .expect("receipt lookup")
        .expect("the record is admitted");
    let parent = store
        .supersession_lineage(SCOPE, episode_digest)
        .expect("lineage")
        .supersedes
        .map(|parent| AncestryRef {
            origin: origin.clone(),
            episode_digest: parent,
        });
    ForeignRecord::from_receipt(origin, &receipt, parent).expect("portable record")
}

/// The two organizations of every test, with their keys and identities.
pub(crate) struct Organizations {
    pub(crate) org_a: OrganizationId,
    seed_a: u8,
    peer_a: PeerIdentity,
    peer_b: PeerIdentity,
}

impl Organizations {
    /// A fresh signing key for organization A's peer.
    pub(crate) fn key_a(&self) -> PeerKey {
        PeerKey::from_seed([self.seed_a; 32])
    }
}

pub(crate) fn organizations() -> Organizations {
    let org_a = organization("org_a");
    let org_b = organization("org_b");
    let (_, peer_a) = peer_with(1, &org_a, ResidencyClass::Unrestricted);
    let (_, peer_b) = peer_with(2, &org_b, ResidencyClass::Unrestricted);
    Organizations {
        org_a,
        seed_a: 1,
        peer_a,
        peer_b,
    }
}

/// The exporting peer of organization A, with B enrolled and trusted.
pub(crate) fn exporter_a(
    scratch: &Scratch,
    organizations: &Organizations,
    trusts: TrustStore,
) -> FederationExporter {
    let mut peers = PeerDirectory::new();
    peers.enroll(organizations.peer_b.clone()).expect("enroll");
    FederationExporter::open(
        scratch.child("exporter_a"),
        organizations.peer_a.clone(),
        organizations.key_a(),
        peers,
        trusts,
    )
    .expect("exporter")
}

/// The importing peer of organization B, with A enrolled.
pub(crate) fn importer_b(
    scratch: &Scratch,
    organizations: &Organizations,
    trusts: TrustStore,
) -> FederationImporter {
    let mut peers = PeerDirectory::new();
    peers.enroll(organizations.peer_a.clone()).expect("enroll");
    FederationImporter::open(
        scratch.child("importer_b"),
        organizations.peer_b.clone(),
        peers,
        trusts,
    )
    .expect("importer")
}
