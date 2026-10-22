//! The studio's canonical adapter over the WDBX v3 episode ledger.
//!
//! # What this adapter is
//!
//! The studio's own store keeps one JSON snapshot of the specimen. That
//! snapshot is a *projection*: it is convenient to read and it is not
//! canonical. This module is the studio's way of recording its episodic
//! semantics in the substrate's canonical ledger instead, so that trust,
//! supersession, contradiction, quarantine, and deletion are decided by the
//! substrate and not by the studio.
//!
//! The adapter owns one [`EpisodeStore`] and maps the studio's semantic
//! operations onto the substrate's episode writes:
//!
//! | Studio operation | Canonical episode |
//! | --- | --- |
//! | a durable edit of the specimen | a memory candidate, class `Summary`, retention `Durable`, whose payload commitment is the SHA-256 over the studio's canonical content bytes |
//! | an erasure request | a memory candidate with `forgets`, `payload_bytes == 0`, an all-zero payload commitment, and no supersession |
//! | a learning proposal, pending | no episode: the proposal is not admitted yet |
//! | a learning proposal, learned | a memory candidate, class `Fact`, retention `Durable`, recorded as a proposal, and therefore `Uncertain` by design: it never reads `Accepted` |
//! | a learning proposal, dismissed | the erasure request for the candidate it named |
//!
//! The class and the retention are the studio's own choice, and they are
//! deliberate. An edit is a *summary* of the specimen's durable state, not a
//! bounded fact and not an embedding vector, and a learned proposal is a
//! *fact*: a pattern the studio now holds. Both are *durable*, because the
//! studio's specimen outlives a session and a correction goes through
//! `supersedes` rather than through an expiry. Nothing is `session` or
//! `operational`: the studio states no time-to-live, and a deadline the
//! studio never agreed to would expire its own edits.
//!
//! # The rules this module keeps
//!
//! * The ledger **commits to** a payload; it never stores one. The
//!   specimen's bytes stay in the studio's own store, and the ledger holds
//!   the digest of the studio's canonical content bytes.
//! * The adapter **calls** the substrate's write path. It never re-derives a
//!   commitment, never predicts an episode digest, and never writes to the
//!   ledger by itself: every append goes through
//!   [`EpisodeStore::propose_write`], the one canonical write path.
//! * The adapter **asks** the substrate about every refusal. The pre-flight
//!   decision runs the substrate's own gate through
//!   [`EpisodeStore::decide_write_outcome`], so a refusal carries the
//!   substrate's own content-free reason instead of a string the studio
//!   matched on.
//! * The adapter **refuses its own malformed operations in its own words**.
//!   An operation is built through [`CanonicalOperation::new`], which refuses
//!   a request identifier the studio cannot bound, and a proposal the studio
//!   cannot read is refused before it ever becomes an episode write. Those
//!   refusals carry the studio's own content-free reason
//!   ([`StudioRejection`]), never a substrate label borrowed for a
//!   studio-level fault.
//! * The projection **keeps the dimensions apart**. It reports the
//!   substrate's disposition and the substrate's eight evidence dimensions,
//!   and it holds no value of its own that could stand in for them.
//! * The studio **declares what it drops**. See [`LossManifest`].
//!
//! # Where the studio's own erasure list fits
//!
//! The studio's snapshot keeps a `tombstones` list, and that list is the
//! studio's own projection of a deletion. The canonical erasure request is
//! the same deletion recorded by the substrate, and the two are separate on
//! purpose: the snapshot's tombstone names the node that disappeared, and
//! the canonical tombstone names the episode digest of the candidate the
//! studio admitted. The studio owns the mapping between them, because the
//! ledger commits to a payload and never stores one. The mapping lives in
//! the studio's [`Journal`], and the studio checks it against the ledger's
//! own answer with [`ForgetAgreement`].
//!
//! # How the studio's store uses this adapter
//!
//! The studio's own store calls this adapter when it is configured with one:
//! see `persistence::Store::open_with_canonical`. The store derives the
//! canonical operations from a durable change, asks the substrate whether it
//! admits them, commits the admitted ones through
//! [`EpisodeStore::propose_write`], and records the substrate's content-free
//! receipts in its own [`Journal`]. The store's snapshot is then a
//! projection: `persistence::Store::rebuild_from_ledger` walks the ledger and
//! reconstructs the document the store holds.
//!
//! # What this module does not do
//!
//! * It does not store the specimen. The ledger commits to the studio's
//!   content; the content stays in the studio's own store.
//! * It does not give the browser edition a ledger. The browser edition
//!   persists to IndexedDB and declares what it drops.
//! * It does not sign anything. Episode signing, key custody, and the trust
//!   list belong to the substrate's signing module, and the studio opens the
//!   store without a signer.
//! * It does not replicate. The substrate's v3 profile is unratified, and
//!   there is no quorum, no federation, and no cross-language verification
//!   behind these episodes: the ledger is one local writer.
//! * It does not decide what the ledger holds. The studio proposes; the
//!   substrate admits or refuses, and the studio records the refusal.

mod journal;
mod manifest;
mod operations;
mod projection;
mod rebuild;

pub use journal::{Digest, JOURNAL_KEY, Journal, JournalEntry, JournalKind, TombstoneRow};
pub use manifest::{LossDeclaration, LossId, LossManifest};
pub use operations::{
    CanonicalOperation, LearningProposal, MAX_REQUEST_ID_LEN, OperationKind, ProposalStatus,
    canonical_content_bytes, content_commitment,
};
pub use projection::{CanonicalProjection, PROJECTION_LIMIT};
pub use rebuild::{ForgetAgreement, RebuildGap, RebuildReport};

use crate::{Result, error};
use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, AdmissionClass, AdmissionReason, ConstitutionalQuery, EpisodeEvent,
    EpisodeReceipt, EpisodeSource, EpisodeStore, EpisodeWrite, EvidenceLevel, GuildEpisodePolicy,
    MemoryCandidate, MemoryClass, RetentionClass, StorePolicy, classify_write,
};
use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};

/// The directory the canonical ledger lives in, under the studio's root.
pub const LEDGER_DIRECTORY: &str = "episodes";

/// The guild scope the studio records under.
pub const STUDIO_SCOPE: &str = "studio_workspace";

/// The policy version the studio writes under by default.
pub const DEFAULT_POLICY_VERSION: &str = "studio_policy_v1";

/// The contract revision the studio binds itself to.
pub const CONTRACT_REVISION: u64 = 1;

/// The token charge one episode write carries.
///
/// The studio has no token meter, so every episode costs the same single
/// token and the guild's token budget bounds how many episodes the guild may
/// hold. The substrate charges the content bytes against the storage budget
/// separately.
pub const TOKEN_COST: u64 = 1;

/// The principal that records the studio's episodes.
pub const RECORDER: &str = "specimen_studio";

/// The studio's own contract digest.
///
/// The substrate binds every write to the exact contract revision and digest
/// its host is bound to, and refuses a write whose binding does not match the
/// store's. The studio is not bound to the Abbey ABI contract corpus, so it
/// binds its own content contract: this digest is the SHA-256 over the
/// descriptor that names it. It is an opaque binding value to the substrate,
/// and it is **not** the Abbey corpus digest.
pub fn contract_digest() -> [u8; 32] {
    let mut digest = [0u8; 32];
    digest.copy_from_slice(&Sha256::digest(b"wdbx-specimen-studio.contract.v1"));
    digest
}

/// How the studio binds itself to the canonical ledger.
#[derive(Clone, Debug)]
pub struct CanonicalConfig {
    /// The guild scope the studio records under.
    pub scope: String,
    /// The exact policy version the studio writes under.
    pub policy_version: String,
    /// Whether the guild admits durable learning and write records.
    pub learning_enabled: bool,
    /// The guild's token budget.
    pub token_budget: u64,
    /// The guild's storage budget, in bytes.
    ///
    /// The default is the substrate's own whole-ledger ceiling: the studio's
    /// specimen may be larger than any smaller budget, and the substrate
    /// refuses such a write with its own storage reason rather than the
    /// studio truncating the content it commits to.
    pub storage_budget_bytes: u64,
}

impl Default for CanonicalConfig {
    fn default() -> Self {
        Self {
            scope: STUDIO_SCOPE.into(),
            policy_version: DEFAULT_POLICY_VERSION.into(),
            learning_enabled: true,
            token_budget: 1_000_000,
            storage_budget_bytes: 64 * 1024 * 1024,
        }
    }
}

impl CanonicalConfig {
    /// The guild policy this configuration records under.
    ///
    /// The adapter opens the ledger with this policy for the studio's own
    /// scope and holds it, so it never looks a guild up and never answers a
    /// guild question the substrate owns.
    pub fn guild_policy(&self) -> GuildEpisodePolicy {
        GuildEpisodePolicy {
            learning_enabled: self.learning_enabled,
            policy_version: self.policy_version.clone(),
            token_budget: self.token_budget,
            storage_budget_bytes: self.storage_budget_bytes,
            current_consent_epoch: None,
            retention_ttl_ms: None,
            retention_windows: None,
            authorized_principals: None,
        }
    }

    /// The store policy this configuration opens the ledger with.
    pub fn store_policy(&self) -> StorePolicy {
        self.store_policy_for(&self.guild_policy())
    }

    /// The store policy for one guild policy, under the studio's own scope.
    ///
    /// The adapter opens the ledger with the guild policy it holds, so the
    /// two are the same policy by construction and never by agreement.
    fn store_policy_for(&self, guild: &GuildEpisodePolicy) -> StorePolicy {
        let mut guilds = std::collections::BTreeMap::new();
        guilds.insert(self.scope.clone(), guild.clone());
        StorePolicy {
            contract_revision: CONTRACT_REVISION,
            contract_digest: contract_digest(),
            guilds,
        }
    }
}

/// What the substrate says about one studio operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonicalDecision {
    /// The substrate admits the write.
    Admitted {
        /// The class the record is admitted as.
        class: AdmissionClass,
        /// The content-free receipt, once the write was appended.
        receipt: Option<EpisodeReceipt>,
    },
    /// The substrate refuses the write.
    Refused {
        /// The class the refused record would have carried.
        class: AdmissionClass,
        /// The substrate's own content-free reason.
        reason: AdmissionReason,
    },
    /// The studio refuses the operation before the substrate is asked.
    ///
    /// A studio-level refusal is not a substrate answer: the operation never
    /// became an episode write, so the substrate was never asked. The reason
    /// is the studio's own, and it names the shape of the operation and
    /// never its content.
    Rejected {
        /// The studio's own content-free reason.
        reason: StudioRejection,
    },
    /// The operation is not an episode yet, so the substrate was never asked.
    Deferred {
        /// Why the operation is not an episode yet.
        stage: DeferredStage,
    },
}

/// Why the studio refused an operation before the substrate was asked.
///
/// The vocabulary is closed and content-free: it names the shape of the
/// studio's own operation and never the operation's content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StudioRejection {
    /// The learning proposal carries no state the studio knows.
    UnknownProposalState,
}

impl StudioRejection {
    /// The stable label of the rejection.
    pub fn label(self) -> &'static str {
        match self {
            Self::UnknownProposalState => "unknown_proposal_state",
        }
    }
}

impl CanonicalDecision {
    /// Whether the substrate admits the operation.
    pub fn is_admitted(&self) -> bool {
        matches!(self, Self::Admitted { .. })
    }
    /// Whether the operation was refused, by the substrate or by the studio.
    pub fn is_refused(&self) -> bool {
        matches!(self, Self::Refused { .. } | Self::Rejected { .. })
    }
    /// The substrate's own reason, when the substrate refused the write.
    pub fn substrate_reason(&self) -> Option<AdmissionReason> {
        match self {
            Self::Refused { reason, .. } => Some(*reason),
            _ => None,
        }
    }
    /// The studio's own reason, when the studio refused the operation.
    pub fn studio_reason(&self) -> Option<StudioRejection> {
        match self {
            Self::Rejected { reason } => Some(*reason),
            _ => None,
        }
    }
    /// The content-free receipt, once the write was appended.
    pub fn receipt(&self) -> Option<&EpisodeReceipt> {
        match self {
            Self::Admitted { receipt, .. } => receipt.as_ref(),
            _ => None,
        }
    }
    /// The episode digest the substrate assigned, once it appended one.
    pub fn episode_digest(&self) -> Option<[u8; 32]> {
        self.receipt().map(|receipt| receipt.episode_digest)
    }
    /// The ledger sequence the substrate assigned, once it appended one.
    pub fn sequence(&self) -> Option<u64> {
        self.receipt().map(|receipt| receipt.sequence)
    }
}

/// Why an operation is not an episode yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeferredStage {
    /// The proposal is pending: the studio has not proposed it to the ledger.
    PendingProposal,
    /// The proposal is dismissed and names no candidate to erase.
    NothingToErase,
}

/// The studio's adapter over the canonical episode ledger.
pub struct CanonicalAdapter {
    store: EpisodeStore,
    directory: PathBuf,
    scope: String,
    policy: StorePolicy,
    /// The guild policy the ledger was opened with.
    ///
    /// The adapter writes under this guild, so it holds the policy it opened
    /// with. It never looks a guild up and never answers a guild question:
    /// whether the guild exists is the substrate's own gate, and the
    /// substrate answers it.
    guild: GuildEpisodePolicy,
}

impl CanonicalAdapter {
    /// Open the canonical ledger in the studio's own directory.
    ///
    /// The ledger keeps its own directory beside the studio's snapshot store
    /// and its assets: the two stores coexist, and neither writes into the
    /// other's files.
    pub fn open_in(root: impl AsRef<Path>, config: CanonicalConfig) -> Result<Self> {
        Self::open(root.as_ref().join(LEDGER_DIRECTORY), config)
    }

    /// Open the canonical ledger in its own directory.
    pub fn open(directory: impl AsRef<Path>, config: CanonicalConfig) -> Result<Self> {
        let directory = directory.as_ref().to_path_buf();
        let guild = config.guild_policy();
        let policy = config.store_policy_for(&guild);
        let store =
            EpisodeStore::open(&directory, policy.clone()).map_err(|e| error("Storage", e))?;
        Ok(Self {
            store,
            directory,
            scope: config.scope,
            policy,
            guild,
        })
    }

    /// The directory the ledger lives in.
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The policy the ledger was opened with.
    pub fn policy(&self) -> &StorePolicy {
        &self.policy
    }

    /// The scope the ledger records under.
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// The substrate's own store, for inspection.
    ///
    /// The handle is read-only: a caller may ask the substrate what it would
    /// admit, and it may not append anything but through this adapter.
    pub fn store(&self) -> &EpisodeStore {
        &self.store
    }

    /// The episode write this operation proposes, when it proposes one.
    ///
    /// A pending proposal, and a dismissal that names no candidate, propose
    /// nothing: they are not episodes yet.
    pub fn write_for(&self, operation: &CanonicalOperation) -> Option<EpisodeWrite> {
        match self.prepare(operation) {
            Prepared::Write(write) => Some(*write),
            Prepared::Deferred(_) | Prepared::Rejected(_) => None,
        }
    }

    /// Decide, without writing, whether the substrate admits the operation.
    ///
    /// The decision runs the substrate's own gate and appends nothing, so a
    /// caller may show the refusal reason before the studio acts on it. The
    /// reason is the substrate's own content-free label, unless the studio
    /// refused the operation before the substrate was asked: then the reason
    /// is the studio's own.
    pub fn decide(&self, operation: &CanonicalOperation) -> CanonicalDecision {
        match self.prepare(operation) {
            Prepared::Write(write) => {
                let decision = self.store.decide_write_outcome(&write);
                match decision.reason() {
                    AdmissionReason::Accepted => CanonicalDecision::Admitted {
                        class: decision.class(),
                        receipt: None,
                    },
                    reason => CanonicalDecision::Refused {
                        class: decision.class(),
                        reason,
                    },
                }
            }
            Prepared::Deferred(stage) => CanonicalDecision::Deferred { stage },
            Prepared::Rejected(reason) => CanonicalDecision::Rejected { reason },
        }
    }

    /// Commit the operation through the substrate's one canonical write path.
    ///
    /// The adapter appends nothing itself: the write goes to the substrate,
    /// and the substrate's own content-free receipt comes back.
    pub fn commit(&mut self, operation: &CanonicalOperation) -> CanonicalDecision {
        match self.prepare(operation) {
            Prepared::Write(write) => {
                let class = classify_write(&write);
                match self.store.propose_write(&write) {
                    Ok(receipt) => CanonicalDecision::Admitted {
                        class,
                        receipt: Some(receipt),
                    },
                    Err(error) => CanonicalDecision::Refused {
                        class,
                        reason: AdmissionReason::from(&error),
                    },
                }
            }
            Prepared::Deferred(stage) => CanonicalDecision::Deferred { stage },
            Prepared::Rejected(reason) => CanonicalDecision::Rejected { reason },
        }
    }

    /// Read the ledger back as the studio's projection.
    ///
    /// The projection keeps the substrate's disposition and the substrate's
    /// eight dimensions for every episode, and it holds no value of its own
    /// that could stand in for them.
    pub fn projection(&self) -> Result<CanonicalProjection> {
        let query = ConstitutionalQuery::new(self.scope.clone(), PROJECTION_LIMIT);
        let results = self.store.query(&query).map_err(|e| error("Storage", e))?;
        Ok(CanonicalProjection {
            scope: results.scope,
            truncated: results.results.len() >= PROJECTION_LIMIT,
            episodes: results.results,
            disposition_counts: results.disposition_counts,
        })
    }

    /// Read the ledger back as the studio's projection, erased records included.
    ///
    /// The default projection leaves a forgotten record out, because a
    /// tombstone means the content is gone. A rebuild needs the whole
    /// ledger: it has to see which state the studio erased to know which
    /// state is current, and it has to count the erasures it finds.
    pub fn full_projection(&self) -> Result<CanonicalProjection> {
        let mut query = ConstitutionalQuery::new(self.scope.clone(), PROJECTION_LIMIT);
        query.constraints.include_forgotten = true;
        let results = self.store.query(&query).map_err(|e| error("Storage", e))?;
        Ok(CanonicalProjection {
            scope: results.scope,
            truncated: results.results.len() >= PROJECTION_LIMIT,
            episodes: results.results,
            disposition_counts: results.disposition_counts,
        })
    }

    /// What the studio's projection drops relative to the canonical evidence.
    pub fn loss_manifest(&self) -> LossManifest {
        LossManifest::native()
    }

    /// What one operation amounts to before the substrate is asked.
    fn prepare(&self, operation: &CanonicalOperation) -> Prepared {
        let (source_type, event) = match operation.kind() {
            OperationKind::Edit {
                specimen,
                supersedes,
            } => {
                let bytes = canonical_content_bytes(specimen);
                (
                    EpisodeSource::LocalRuntime,
                    EpisodeEvent::MemoryCandidate {
                        recorded_by: recorder(),
                        candidate: candidate(
                            MemoryClass::Summary,
                            content_commitment(&bytes),
                            byte_count(bytes.len()),
                            *supersedes,
                            None,
                        ),
                    },
                )
            }
            OperationKind::Forget { target } => (
                EpisodeSource::LocalRuntime,
                EpisodeEvent::MemoryCandidate {
                    recorded_by: recorder(),
                    candidate: candidate(MemoryClass::Summary, [0; 32], 0, None, Some(*target)),
                },
            ),
            OperationKind::Learn { proposal, erases } => match ProposalStatus::from_value(proposal)
            {
                Some(ProposalStatus::Pending) => {
                    return Prepared::Deferred(DeferredStage::PendingProposal);
                }
                Some(ProposalStatus::Dismissed) => {
                    let Some(target) = erases else {
                        return Prepared::Deferred(DeferredStage::NothingToErase);
                    };
                    (
                        EpisodeSource::LocalRuntime,
                        EpisodeEvent::MemoryCandidate {
                            recorded_by: recorder(),
                            candidate: candidate(
                                MemoryClass::Summary,
                                [0; 32],
                                0,
                                None,
                                Some(*target),
                            ),
                        },
                    )
                }
                Some(ProposalStatus::Learned) => {
                    let bytes = canonical_content_bytes(proposal);
                    (
                        EpisodeSource::Proposal,
                        EpisodeEvent::MemoryCandidate {
                            recorded_by: recorder(),
                            candidate: candidate(
                                MemoryClass::Fact,
                                content_commitment(&bytes),
                                byte_count(bytes.len()),
                                None,
                                None,
                            ),
                        },
                    )
                }
                // A proposal the studio cannot read is the studio's own
                // malformed operation: it never became an episode write, so
                // the substrate is never asked about it.
                None => return Prepared::Rejected(StudioRejection::UnknownProposalState),
            },
        };
        Prepared::Write(Box::new(EpisodeWrite {
            request_id: operation.request_id().to_owned(),
            operation_id: format!("op_{}", operation.request_id()),
            contract_revision: self.policy.contract_revision,
            contract_digest: self.policy.contract_digest,
            guild_ref: self.scope.clone(),
            consent_epoch: None,
            source_type,
            policy_version: self.guild.policy_version.clone(),
            evidence_level: EvidenceLevel::C1,
            event,
            token_cost: TOKEN_COST,
            // The studio never predicts the episode commitment: the substrate
            // computes it, and a prediction would be a second opinion the
            // adapter has no standing to hold.
            expected_commitment: None,
            quiet: false,
        }))
    }
}

/// What one operation amounts to before the substrate is asked.
enum Prepared {
    /// A write the substrate may admit.
    Write(Box<EpisodeWrite>),
    /// Not an episode yet.
    Deferred(DeferredStage),
    /// The studio's own operation is malformed.
    Rejected(StudioRejection),
}

/// One memory candidate in the studio's own shape.
///
/// The retention class is always `Durable`: the studio's specimen outlives a
/// session, and the substrate's expiry rules apply to the policy's window,
/// not to the studio's own lifetime. A correction goes through `supersedes`
/// and an erasure through `forgets`.
fn candidate(
    class: MemoryClass,
    payload_commitment: [u8; 32],
    payload_bytes: u64,
    supersedes: Option<[u8; 32]>,
    forgets: Option<[u8; 32]>,
) -> MemoryCandidate {
    MemoryCandidate {
        class,
        retention: RetentionClass::Durable,
        payload_commitment,
        payload_bytes,
        dimension: None,
        embedding_version: None,
        // The studio's specimen is one workspace, not a per-member record.
        member_scoped: false,
        supersedes,
        forgets,
    }
}

/// The service identity that records the studio's episodes.
fn recorder() -> ActorRef {
    ActorRef {
        principal_id: RECORDER.into(),
        kind: ActorKind::Service,
    }
}

/// The byte count the substrate charges for one payload.
fn byte_count(bytes: usize) -> u64 {
    u64::try_from(bytes).unwrap_or(u64::MAX)
}
