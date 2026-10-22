//! The studio's write-through to the canonical episode ledger.
//!
//! # What the write-through does
//!
//! When the studio's store is opened with a canonical adapter, every durable
//! change is published to the ledger before the studio's own snapshot is
//! committed:
//!
//! 1. the studio derives the canonical operations from the change: the new
//!    state, the proposals that were learned, and the erasures;
//! 2. it asks the substrate whether it admits each of them, through the
//!    substrate's own gate, which appends nothing;
//! 3. it commits the admitted ones through the substrate's one write path and
//!    records the substrate's content-free receipts;
//! 4. it keeps the content the ledger committed to in the studio's own blob
//!    store, beside the snapshot, and records the map from every episode to
//!    that content in the studio's journal.
//!
//! The studio's snapshot is then a projection: [`Store::rebuild_from_ledger`]
//! walks the ledger and reconstructs the document the studio holds.
//!
//! # What the write-through refuses to do
//!
//! * It never predicts an episode commitment. The substrate computes it.
//! * It never writes to the ledger by itself. Every append goes through the
//!   substrate's `propose_write`.
//! * It never drops an operation silently. A refusal is reported with the
//!   substrate's own content-free reason, and a refused change leaves the
//!   ledger byte-identical and the studio's canonical state unchanged.
//! * It never invents a tombstone. The studio's erasure rows come from the
//!   same diff that produced the erasure, so the ledger's forget candidates
//!   and the studio's tombstone list are two records of one event.

use super::{Snapshot, Store};
use crate::{
    Result,
    canonical::{
        CanonicalAdapter, CanonicalConfig, CanonicalDecision, CanonicalOperation, Digest,
        ForgetAgreement, JOURNAL_KEY, Journal, JournalEntry, JournalKind, OperationKind,
        RebuildGap, RebuildReport, TombstoneRow, canonical_content_bytes, content_commitment,
    },
    engine, error,
};
use abi_wdbx::v2::V2Mutation;
use serde_json::Value;
use std::collections::BTreeSet;

/// What the studio's last canonical write-through did.
///
/// The report is the studio's own record of the substrate's answer. It
/// carries the substrate's content-free reason for every refusal and the
/// substrate's content-free receipt for every admitted episode, so a caller
/// never has to parse a string to learn what the ledger said.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanonicalReport {
    /// The operations the change produced, in the order they were proposed.
    pub operations: Vec<CanonicalOperationReport>,
}

impl CanonicalReport {
    /// The operations the substrate or the studio refused.
    pub fn refusals(&self) -> impl Iterator<Item = &CanonicalOperationReport> {
        self.operations
            .iter()
            .filter(|operation| operation.decision.is_refused())
    }

    /// Whether every operation was admitted.
    pub fn is_admitted(&self) -> bool {
        self.refusals().next().is_none()
    }
}

/// One operation, and what the substrate said about it.
#[derive(Clone, Debug, PartialEq)]
pub struct CanonicalOperationReport {
    /// What kind of episode the operation proposed.
    pub kind: JournalKind,
    /// The substrate's own answer.
    pub decision: CanonicalDecision,
}

/// What one durable change publishes to the canonical ledger.
pub(super) struct Publication {
    /// The journal after the change.
    pub(super) journal: Journal,
    /// The studio's own mutations: the content blobs and the journal.
    pub(super) mutations: Vec<V2Mutation>,
    /// What the substrate said.
    pub(super) report: CanonicalReport,
}

impl Publication {
    /// The publication of a change the canonical ledger did not see.
    fn unchanged(journal: Journal) -> Self {
        Self {
            journal,
            mutations: Vec::new(),
            report: CanonicalReport::default(),
        }
    }
}

/// One canonical operation the studio derived from a durable change.
enum Plan {
    /// A proposal the studio learned.
    Learn {
        /// The proposal, as the studio holds it.
        proposal: Value,
    },
    /// The new durable state.
    Edit {
        /// The specimen the change produced.
        specimen: Value,
        /// The state this one corrects, when it corrects one.
        supersedes: Option<[u8; 32]>,
    },
    /// The erasure of a proposal the studio dismissed.
    ForgetProposal {
        /// The proposal, as the studio holds it.
        proposal: Value,
        /// The candidate the dismissal erases, when the studio recorded one.
        target: Option<[u8; 32]>,
    },
    /// The erasure of the state that admitted the removed nodes.
    ForgetState {
        /// The candidate that admitted the removed nodes.
        target: [u8; 32],
        /// The studio's own rows for the erasure.
        rows: Vec<TombstoneRow>,
    },
}

impl Plan {
    /// What kind of episode this plan proposes.
    fn kind(&self) -> JournalKind {
        match self {
            Self::Learn { .. } => JournalKind::Learn,
            Self::Edit { .. } => JournalKind::Edit,
            Self::ForgetProposal { .. } | Self::ForgetState { .. } => JournalKind::Forget,
        }
    }

    /// The content this plan commits to, when it commits to one.
    fn content(&self) -> Option<Vec<u8>> {
        match self {
            Self::Learn { proposal } => Some(canonical_content_bytes(proposal)),
            Self::Edit { specimen, .. } => Some(canonical_content_bytes(specimen)),
            Self::ForgetProposal { .. } | Self::ForgetState { .. } => None,
        }
    }

    /// The candidate this plan supersedes.
    fn supersedes(&self) -> Option<[u8; 32]> {
        match self {
            Self::Edit { supersedes, .. } => *supersedes,
            _ => None,
        }
    }

    /// The candidate this plan erases.
    fn forgets(&self) -> Option<[u8; 32]> {
        match self {
            Self::ForgetProposal { target, .. } => *target,
            Self::ForgetState { target, .. } => Some(*target),
            _ => None,
        }
    }

    /// The proposal this plan records, when it records one.
    fn proposal(&self) -> Option<String> {
        match self {
            Self::Learn { proposal } | Self::ForgetProposal { proposal, .. } => {
                Some(engine::text(proposal, "id").to_owned())
            }
            _ => None,
        }
    }

    /// The studio's own rows for the erasure this plan proposes.
    fn rows(&self) -> Vec<TombstoneRow> {
        match self {
            Self::ForgetState { rows, .. } => rows.clone(),
            _ => Vec::new(),
        }
    }

    /// The operation the substrate is asked about.
    fn operation(&self, request_id: &str) -> Result<CanonicalOperation> {
        let kind = match self {
            Self::Learn { proposal } => OperationKind::Learn {
                proposal: proposal.clone(),
                erases: None,
            },
            Self::Edit {
                specimen,
                supersedes,
            } => OperationKind::Edit {
                specimen: specimen.clone(),
                supersedes: *supersedes,
            },
            Self::ForgetProposal { proposal, target } => OperationKind::Learn {
                proposal: proposal.clone(),
                erases: *target,
            },
            Self::ForgetState { target, .. } => OperationKind::Forget { target: *target },
        };
        CanonicalOperation::new(request_id, kind)
    }
}

/// A proposal's state changed between two durable states.
enum Transition {
    /// The proposal was learned.
    Learned(Value),
    /// The proposal was dismissed.
    Dismissed(Value),
}

impl Store {
    /// Open the studio store with the canonical ledger beside it.
    ///
    /// The ledger keeps its own directory under the studio's root and its
    /// own writer lock; the two stores coexist and neither writes into the
    /// other's files. The store without an adapter is unchanged: this is the
    /// opt-in path.
    pub fn open_with_canonical(
        root: impl Into<std::path::PathBuf>,
        config: CanonicalConfig,
    ) -> Result<Self> {
        let root = root.into();
        let mut store = Self::open(root.clone())?;
        let adapter = CanonicalAdapter::open_in(&root, config)?;
        store.canonical = Some(adapter);
        Ok(store)
    }

    /// The canonical ledger, when the studio opened one.
    pub fn canonical(&self) -> Option<&CanonicalAdapter> {
        self.canonical.as_ref()
    }

    /// What the last canonical write-through did.
    ///
    /// `None` when the studio never published anything canonically.
    pub fn canonical_report(&self) -> Option<&CanonicalReport> {
        self.canonical_report.as_ref()
    }

    /// The studio's canonical journal: the map from every episode to the
    /// content it committed to.
    pub fn journal(&self) -> &Journal {
        &self.journal
    }

    /// The studio's own content for one commitment.
    ///
    /// The content lives in the studio's own content-addressed blob store:
    /// the ledger commits to it, it never holds it.
    fn content(&self, commitment: &Digest) -> Option<Vec<u8>> {
        let path = self.root.join("assets").join(commitment.to_hex());
        std::fs::read(path).ok()
    }

    /// Whether the substrate still holds one candidate as live.
    fn is_live(&self, digest: &[u8; 32]) -> bool {
        let Some(adapter) = self.canonical.as_ref() else {
            return false;
        };
        match adapter
            .store()
            .supersession_lineage(adapter.scope(), digest)
        {
            Ok(lineage) => !lineage.forgotten && !lineage.tombstoned,
            Err(_) => false,
        }
    }

    /// Publish one durable change to the canonical ledger.
    ///
    /// The studio asks first and writes second. The pre-flight decision runs
    /// the substrate's own gate and appends nothing; only an admitted
    /// operation is committed, and a refusal anywhere leaves the ledger
    /// byte-identical.
    pub(super) fn publish_canonical(&mut self, next: &mut Snapshot) -> Result<Publication> {
        if self.canonical.is_none() {
            return Ok(Publication::unchanged(self.journal.clone()));
        }
        let previous = self.snapshot.specimen.clone();
        let specimen = next.specimen.clone();
        if previous == specimen {
            return Ok(Publication::unchanged(self.journal.clone()));
        }

        // The studio's own record of the erasure, when the change erased
        // anything. The rows are computed here, once, so the snapshot and
        // the journal name the same event.
        let rows: Vec<TombstoneRow> = removed_nodes(&previous, &specimen)
            .into_iter()
            .map(|node| TombstoneRow {
                node,
                deleted_at: crate::now(),
            })
            .collect();
        for row in &rows {
            next.tombstones.push(row.to_value());
        }

        let plans = self.plan(&previous, &specimen, rows);
        if plans.is_empty() {
            return Ok(Publication::unchanged(self.journal.clone()));
        }
        let revision = next.revision;
        let operations: Vec<CanonicalOperation> = plans
            .iter()
            .enumerate()
            .map(|(index, plan)| plan.operation(&request_id(plan.kind(), revision, index)))
            .collect::<Result<Vec<CanonicalOperation>>>()?;

        // The pre-flight: the substrate's own gate, appending nothing.
        let mut reports = Vec::new();
        let mut refused = false;
        for (plan, operation) in plans.iter().zip(&operations) {
            let decision = self
                .canonical
                .as_ref()
                .expect("the adapter is present")
                .decide(operation);
            refused |= decision.is_refused();
            reports.push(CanonicalOperationReport {
                kind: plan.kind(),
                decision,
            });
        }
        if refused {
            // A refusal leaves the ledger byte-identical and the studio's
            // canonical state unchanged. The studio's own snapshot still
            // advances, and the report names the substrate's reason.
            return Ok(Publication {
                journal: self.journal.clone(),
                mutations: Vec::new(),
                report: CanonicalReport {
                    operations: reports,
                },
            });
        }

        // The commit: the substrate's one write path, and the substrate's own
        // content-free receipts.
        let mut entries = self.journal.entries.clone();
        let mut mutations = Vec::new();
        let mut reports = Vec::new();
        for (plan, operation) in plans.iter().zip(&operations) {
            let decision = self
                .canonical
                .as_mut()
                .expect("the adapter is present")
                .commit(operation);
            let kind = plan.kind();
            if let CanonicalDecision::Admitted {
                receipt: Some(receipt),
                ..
            } = &decision
            {
                let content = plan.content();
                let commitment = content.as_ref().map(|bytes| content_commitment(bytes));
                if let Some(bytes) = &content {
                    // The studio keeps the bytes the ledger commits to in its
                    // own content-addressed blob store. The blob is named by
                    // the digest of its own bytes, which is the commitment
                    // the episode carries.
                    self.put_asset(bytes)?;
                }
                entries.push(JournalEntry {
                    episode: Digest::from_bytes(receipt.episode_digest),
                    sequence: receipt.sequence,
                    request_id: operation.request_id().to_owned(),
                    kind,
                    content: commitment.map(Digest::from_bytes),
                    supersedes: plan.supersedes().map(Digest::from_bytes),
                    forgets: plan.forgets().map(Digest::from_bytes),
                    proposal: plan.proposal(),
                    erases: plan.rows(),
                });
            }
            reports.push(CanonicalOperationReport { kind, decision });
        }

        let journal = Journal { entries };
        mutations.push(V2Mutation::PutKv {
            key: JOURNAL_KEY.to_owned(),
            value: serde_json::to_string(&journal)?,
        });
        Ok(Publication {
            journal,
            mutations,
            report: CanonicalReport {
                operations: reports,
            },
        })
    }

    /// The canonical operations one durable change produces.
    fn plan(&self, previous: &Value, specimen: &Value, rows: Vec<TombstoneRow>) -> Vec<Plan> {
        let mut plans = Vec::new();
        // The proposals the studio learned or dismissed.
        for transition in proposal_transitions(previous, specimen) {
            match transition {
                Transition::Learned(proposal) => plans.push(Plan::Learn { proposal }),
                Transition::Dismissed(proposal) => {
                    let id = engine::text(&proposal, "id").to_owned();
                    let target = self
                        .journal
                        .episode_for_proposal(&id)
                        .map(|entry| *entry.episode.as_bytes());
                    plans.push(Plan::ForgetProposal { proposal, target });
                }
            }
        }
        // The new state. It corrects the previous one unless the change
        // erased it: an erased state is not corrected, it is forgotten.
        let previous_state = self
            .journal
            .last_edit()
            .map(|entry| *entry.episode.as_bytes());
        let erased = !rows.is_empty();
        let supersedes = match previous_state {
            Some(digest) if !erased && self.is_live(&digest) => Some(digest),
            _ => None,
        };
        plans.push(Plan::Edit {
            specimen: specimen.clone(),
            supersedes,
        });
        // The erasure of the state that admitted the removed nodes.
        if erased
            && let Some(target) = previous_state
            && self.is_live(&target)
        {
            plans.push(Plan::ForgetState { target, rows });
        }
        plans
    }

    /// Rebuild the studio's document from the canonical ledger.
    ///
    /// The rebuild walks the ledger, resolves every episode's payload
    /// commitment back to the studio's content, and reconstructs the
    /// document. It never returns a partial document: a missing content blob
    /// is a declared gap, and the rebuild returns nothing at all.
    pub fn rebuild_from_ledger(&self) -> Result<RebuildReport> {
        let Some(adapter) = self.canonical.as_ref() else {
            return Err(error(
                "CanonicalUnavailable",
                "This store has no canonical ledger.",
            ));
        };
        let projection = adapter.full_projection()?;
        let mut gaps = Vec::new();
        let mut document = None;
        let mut tombstones = Vec::new();
        let mut seen = BTreeSet::new();
        for episode in &projection.episodes {
            let digest = Digest::from_bytes(episode.episode_digest);
            seen.insert(digest);
            let Some(entry) = self.journal.entry(&digest) else {
                gaps.push(RebuildGap::UnindexedEpisode { episode: digest });
                continue;
            };
            match entry.kind {
                JournalKind::Edit => {
                    let Some(content) = entry.content else {
                        gaps.push(RebuildGap::MissingContent {
                            episode: digest,
                            content: Digest::ZERO,
                        });
                        continue;
                    };
                    match self.content(&content) {
                        None => gaps.push(RebuildGap::MissingContent {
                            episode: digest,
                            content,
                        }),
                        Some(bytes) => {
                            let found = Digest::from_bytes(content_commitment(&bytes));
                            if found != content {
                                gaps.push(RebuildGap::ContentMismatch {
                                    episode: digest,
                                    content,
                                    found,
                                });
                                continue;
                            }
                            let value: Value = serde_json::from_slice(&bytes).map_err(|err| {
                                error(
                                    "MalformedSave",
                                    format!("The canonical content is unreadable: {err}"),
                                )
                            })?;
                            if !episode.dimensions.lifecycle.forgotten {
                                document = Some(value);
                            }
                        }
                    }
                }
                JournalKind::Forget => {
                    for row in &entry.erases {
                        tombstones.push(row.to_value());
                    }
                }
                JournalKind::Learn => {}
            }
        }
        for entry in &self.journal.entries {
            if !seen.contains(&entry.episode) {
                gaps.push(RebuildGap::MissingEpisode {
                    episode: entry.episode,
                });
            }
        }
        if document.is_none() && gaps.is_empty() && self.journal.edits().next().is_some() {
            gaps.push(RebuildGap::NoLiveState {
                episodes: projection.episodes.len(),
            });
        }
        if tombstones != self.snapshot.tombstones {
            gaps.push(RebuildGap::TombstoneMismatch {
                rebuilt: tombstones.len(),
                snapshot: self.snapshot.tombstones.len(),
            });
        }
        Ok(RebuildReport {
            document,
            tombstones,
            episodes: projection.episodes.len(),
            gaps,
        })
    }

    /// Whether the studio's snapshot is ahead of the canonical record.
    ///
    /// A refused canonical write leaves the studio's own snapshot ahead of
    /// the ledger: the studio keeps working, and the divergence is named
    /// here rather than left to be discovered. A store whose document and
    /// ledger agree answers `false`.
    pub fn canonical_drift(&self) -> Result<bool> {
        if self.canonical.is_none() {
            return Ok(false);
        }
        let report = self.rebuild_from_ledger()?;
        Ok(report.document.as_ref() != Some(&self.snapshot.specimen))
    }

    /// Whether the studio's erasure list and the ledger's forget candidates
    /// agree.
    ///
    /// The two are separate records of one event: the studio's tombstone
    /// rows name the node that disappeared, and the ledger's forget
    /// candidate names the episode digest that was erased. The ledger holds
    /// no node and no instant, so the studio's journal carries the pairing,
    /// and this check holds that pairing against the ledger's own answer.
    pub fn forget_agreement(&self) -> Result<ForgetAgreement> {
        let Some(adapter) = self.canonical.as_ref() else {
            return Err(error(
                "CanonicalUnavailable",
                "This store has no canonical ledger.",
            ));
        };
        let projection = adapter.full_projection()?;
        let mut agreement = ForgetAgreement::default();
        let mut forgotten = BTreeSet::new();
        for episode in &projection.episodes {
            let digest = Digest::from_bytes(episode.episode_digest);
            if episode.dimensions.lifecycle.forgotten {
                forgotten.insert(digest);
            }
            if episode.dimensions.lifecycle.tombstoned {
                agreement.ledger_tombstones.push(digest);
                if self.journal.entry(&digest).is_none() {
                    agreement.unknown.push(digest);
                }
            }
        }
        for entry in self.journal.forgets() {
            agreement.studio_forgets.push(entry.episode);
            if !agreement.ledger_tombstones.contains(&entry.episode) {
                agreement.missing.push(entry.episode);
            }
            if let Some(target) = entry.forgets
                && !forgotten.contains(&target)
            {
                agreement.unerased.push(target);
            }
        }
        agreement.studio_tombstones = self.snapshot.tombstones.len();
        agreement.journal_tombstones = self.journal.forgets().map(|entry| entry.erases.len()).sum();
        Ok(agreement)
    }
}

/// The idempotency key one canonical operation carries.
///
/// The key names the kind, the revision the change produced, and the
/// operation's place in it, so it is unique for the life of the ledger and
/// it is a bounded identifier the substrate accepts.
fn request_id(kind: JournalKind, revision: u64, index: usize) -> String {
    format!("{}_r{revision}_{index}", kind.label())
}

/// The nodes the change erased.
fn removed_nodes(previous: &Value, next: &Value) -> Vec<Value> {
    engine::rows(previous, "nodes")
        .iter()
        .filter(|node| {
            !engine::rows(next, "nodes")
                .iter()
                .any(|other| other["ref"] == node["ref"])
        })
        .cloned()
        .collect()
}

/// The proposals whose state the change moved.
fn proposal_transitions(previous: &Value, next: &Value) -> Vec<Transition> {
    let mut transitions = Vec::new();
    for proposal in engine::rows(next, "proposals") {
        let id = engine::text(proposal, "id");
        let before = engine::rows(previous, "proposals")
            .iter()
            .find(|other| engine::text(other, "id") == id);
        let before_status = before.map(|other| engine::text(other, "status"));
        let after_status = engine::text(proposal, "status");
        if before_status == Some(after_status) {
            continue;
        }
        match after_status {
            "learned" => transitions.push(Transition::Learned(proposal.clone())),
            "dismissed" => transitions.push(Transition::Dismissed(proposal.clone())),
            _ => {}
        }
    }
    transitions
}
