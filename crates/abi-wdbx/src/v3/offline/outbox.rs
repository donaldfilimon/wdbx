//! The durable, transactional offline outbox.
//!
//! See the parent module's documentation for what an outbox is and, more
//! importantly, what it is not.

use std::collections::BTreeSet;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::v3::episode::{ActorKind, ActorRef, EpisodeEvent, EpisodeWrite};

use super::asset::{
    AssetChunkOutcome, AssetState, MAX_ASSET_CHUNKS, asset_digest, chunk_count, chunk_digest,
    chunk_length,
};
use super::error::OutboxError;
use super::merge::{EditProposal, MergePlan, merge_edits, validate_edit};
use super::reauthorize::ReauthorizeOutcome;
use super::sidecar::{self, OUTBOX_FILE, OUTBOX_LOCK};
use super::types::{
    AssetProposal, Clock, MAX_OPERATIONS, MAX_POLICY_VERSION_BYTES, MAX_SCOPE_BYTES, NewOperation,
    OperationBinding, OperationId, OperationPayload, OperationReason, OperationReceipt,
    OperationStatus, OperationView, OutboxOperation, OutboxStatus, Resolution, bounded_identifier,
    system_now_ms,
};

/// The greatest token cost one proposed write may carry.
///
/// The canonical store bounds this too. The outbox repeats the bound so a
/// malformed proposal never reaches the durable file.
const MAX_TOKEN_COST: u64 = 1_000_000;

/// The greatest length of an episode write identifier, in bytes.
const MAX_EPISODE_ID_BYTES: usize = 64;

/// A durable, transactional outbox of provisional operations.
///
/// The outbox holds *proposals*, never episodes. It is a queue with
/// dependencies, a durable status per operation, and an explicit
/// reauthorization step on reconnect. It is not a canonical writer: nothing
/// in this type appends to an episode ledger, and nothing in it can bind an
/// episode digest.
pub struct OfflineOutbox {
    /// The file the operations live in.
    path: PathBuf,
    /// The writer lock, held for the outbox's lifetime.
    _lock: File,
    /// The operations, in enqueue order.
    pub(super) operations: Vec<OutboxOperation>,
    /// The sequence number the next enqueued operation takes.
    pub(super) next_sequence: u64,
    /// The clock the enqueue stamps come from.
    clock: Clock,
}

impl std::fmt::Debug for OfflineOutbox {
    /// The proposals are deliberately omitted: the debug output of an outbox
    /// must not carry the work it holds.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OfflineOutbox")
            .field("operations", &self.operations.len())
            .finish_non_exhaustive()
    }
}

impl OfflineOutbox {
    /// Open the outbox in a directory, using the system clock.
    ///
    /// The outbox file is created with the first enqueued operation, not with
    /// the outbox. A writer lock is held for the outbox's lifetime; another
    /// writer is [`OutboxError::WriterBusy`].
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::Io`] when the directory or the file cannot be
    /// prepared, [`OutboxError::WriterBusy`] when another writer holds the
    /// lock, and [`OutboxError::Corrupt`] when a complete line does not
    /// parse or its checksum does not match.
    pub fn open(directory: impl AsRef<Path>) -> Result<Self, OutboxError> {
        Self::open_with_clock(directory, Box::new(system_now_ms))
    }

    /// Open the outbox with an injected clock.
    ///
    /// The clock stamps the enqueue time of an operation. **No outbox
    /// behaviour depends on it**: an operation never expires, and the merge
    /// never reads it. It exists so a caller can observe how long work has
    /// been pending, and so a test can place the outbox arbitrarily far in
    /// the future and see that nothing expires.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::Io`] when the directory or the file cannot be
    /// prepared, [`OutboxError::WriterBusy`] when another writer holds the
    /// lock, and [`OutboxError::Corrupt`] when a complete line does not
    /// parse or its checksum does not match.
    pub fn open_with_clock(directory: impl AsRef<Path>, clock: Clock) -> Result<Self, OutboxError> {
        let directory = sidecar::prepare_directory(directory.as_ref())?;
        let path = directory.join(OUTBOX_FILE);
        let lock_path = directory.join(OUTBOX_LOCK);
        sidecar::reject_symlink_if_present(&path)?;
        sidecar::reject_symlink_if_present(&lock_path)?;
        let lock = sidecar::owner_file(&lock_path)?;
        acquire_writer_lock(&lock)?;
        let operations = sidecar::read(&path)?;
        let next_sequence = operations
            .iter()
            .map(|operation| operation.sequence)
            .max()
            .unwrap_or_default()
            .saturating_add(1);
        Ok(Self {
            path,
            _lock: lock,
            operations,
            next_sequence,
            clock,
        })
    }

    /// The file the operations live in.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Enqueue one operation.
    ///
    /// The operation is durable when this returns: the outbox file has been
    /// written and synchronized. A proposal is bound to the policy, consent
    /// epoch, and contract revision it was written under, and it is
    /// **provisional** until the server accepts it.
    ///
    /// A write proposal may not carry a predicted commitment: the outbox
    /// holds no canonical episode digest, and a digest it carried would be a
    /// second canonical episode.
    ///
    /// The outbox holds at most [`MAX_OPERATIONS`] operations. A full outbox
    /// is [`OutboxError::Capacity`], which is distinct from
    /// [`OutboxError::InvalidInput`]: the proposal is well formed and the
    /// queue is full. Room is made by removing a terminal operation with an
    /// authorized caller, never by the outbox dropping one on its own.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::InvalidInput`] when the binding or the
    /// proposal is outside its bound, [`OutboxError::DuplicateOperation`]
    /// when the identifier is already in the outbox,
    /// [`OutboxError::UnknownDependency`] when the operation depends on an
    /// operation the outbox does not hold, and [`OutboxError::Capacity`]
    /// when the outbox already holds [`MAX_OPERATIONS`] operations.
    ///
    /// The proposal's own defects are reported before the queue's state, so
    /// a malformed proposal is always [`OutboxError::InvalidInput`] and
    /// never [`OutboxError::Capacity`].
    pub fn enqueue(&mut self, operation: NewOperation) -> Result<OperationReceipt, OutboxError> {
        if self
            .operations
            .iter()
            .any(|existing| existing.id == operation.id)
        {
            return Err(OutboxError::DuplicateOperation);
        }
        if let Some(dependency) = &operation.depends_on
            && !self
                .operations
                .iter()
                .any(|existing| &existing.id == dependency)
        {
            return Err(OutboxError::UnknownDependency);
        }
        validate_new(&operation)?;
        if self.operations.len() >= MAX_OPERATIONS {
            return Err(OutboxError::Capacity);
        }
        let kind = operation.payload.kind();
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        let asset =
            matches!(operation.payload, OperationPayload::Asset(_)).then(AssetState::default);
        let record = OutboxOperation {
            id: operation.id,
            depends_on: operation.depends_on,
            binding: operation.binding,
            kind,
            payload: operation.payload,
            status: OperationStatus::Pending,
            reason: None,
            conflict: None,
            asset,
            enqueued_at_ms: (self.clock)(),
            sequence,
        };
        let receipt = receipt_of(&record);
        self.operations.push(record);
        self.persist()?;
        Ok(receipt)
    }

    /// The status surface: the counts of the outbox's operations, per status.
    #[must_use]
    pub fn status(&self) -> OutboxStatus {
        let mut status = OutboxStatus::default();
        for operation in &self.operations {
            match operation.status {
                OperationStatus::Pending => status.pending += 1,
                OperationStatus::InFlight => status.in_flight += 1,
                OperationStatus::Acknowledged => status.acknowledged += 1,
                OperationStatus::Rejected => status.rejected += 1,
                OperationStatus::Conflicted => status.conflicted += 1,
            }
        }
        status
    }

    /// One operation, content-free.
    #[must_use]
    pub fn operation(&self, id: &OperationId) -> Option<OperationView> {
        self.operations
            .iter()
            .find(|operation| &operation.id == id)
            .map(view_of)
    }

    /// Every operation, content-free, in enqueue order.
    #[must_use]
    pub fn operations(&self) -> Vec<OperationView> {
        self.operations.iter().map(view_of).collect()
    }

    /// The deterministic merge of the live edit operations.
    ///
    /// The plan is a pure function of the live operations: the same edits
    /// produce the same plan whatever order they were enqueued in. It
    /// changes nothing.
    #[must_use]
    pub fn merge_plan(&self) -> MergePlan {
        let live: Vec<&OutboxOperation> = self
            .operations
            .iter()
            .filter(|operation| operation.is_live())
            .collect();
        merge_edits(&live)
    }

    /// Apply the merge: record the conflicts and preserve their branches.
    ///
    /// An operation that takes part in a conflict becomes
    /// [`OperationStatus::Conflicted`] and keeps its proposal. The outbox
    /// never resolves a conflict, never picks a winner, and never drops a
    /// branch. An authorized caller resolves it with [`Self::resolve`].
    ///
    /// The operations that merge cleanly are not changed here: they are
    /// merged when the outbox drains.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::Io`] when the outbox cannot be written.
    pub fn merge(&mut self) -> Result<MergePlan, OutboxError> {
        let plan = self.merge_plan();
        self.apply_conflicts(&plan);
        self.persist()?;
        Ok(plan)
    }
    /// Resolve one queued operation.
    ///
    /// A terminal operation is *retained*, never reaped: it stays in the
    /// outbox with its status and its reason until an authorized caller
    /// resolves it. Resolution is one of four things:
    ///
    /// - [`Resolution::Retain`] keeps the operation exactly as it is;
    /// - [`Resolution::Requeue`] puts a refused or a conflicted operation
    ///   back in the pending set, for example after the policy changed and
    ///   the operation would now be authorized;
    /// - [`Resolution::Discard`] removes a refused or a conflicted
    ///   operation;
    /// - [`Resolution::Remove`] removes the operation **whatever its
    ///   status**.
    ///
    /// A removal is always an explicit, authorized act: nothing is reaped
    /// implicitly, and the outbox's own logic never drops an operation.
    /// [`Resolution::Remove`] is the status-independent path, and it exists
    /// because the other resolutions do not cover every operation: an
    /// **acknowledged** operation is the success case and the one that
    /// accumulates fastest, and without this path it could never leave the
    /// outbox, so a long-lived client would wedge its queue at
    /// [`MAX_OPERATIONS`] with no way to make room.
    ///
    /// **A removal is refused while another operation depends on the one
    /// being removed.** A dependent names the operation it depends on, and
    /// removing that operation would leave the dependent with a dependency
    /// the outbox does not hold: every later drain would defer it with
    /// [`OperationReason::DependencyPending`], for ever, because no drain
    /// could ever satisfy a dependency that is not there. The refusal is
    /// [`OutboxError::DependencyPresent`], which names neither the operation
    /// nor the dependent. The caller finds the dependents in the outbox's own
    /// content-free views and resolves them first — the leaves first — and the
    /// removal then succeeds. A removal therefore never leaves behind a
    /// dependency that names an operation the outbox does not hold.
    ///
    /// Only a guild manager, a guild administrator, a guild owner, or the
    /// organization owner may resolve an operation. A service identity may
    /// not: a writer that could discard its own unsent work could discard
    /// work it is not allowed to make, and a writer that could bury its own
    /// acknowledgement could defeat the audit the outbox keeps.
    ///
    /// The receipt of a removal names the status the operation held when it
    /// was removed: the operation itself is gone, and the receipt is the
    /// content-free record of what left.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::AuthorityDenied`] when the authority may not
    /// resolve, [`OutboxError::InvalidState`] when the operation is unknown
    /// or is not in a state the resolution permits,
    /// [`OutboxError::DependencyPresent`] when another operation depends on
    /// the one being removed, and [`OutboxError::Io`] when the outbox cannot
    /// be written.
    pub fn resolve(
        &mut self,
        id: &OperationId,
        resolution: Resolution,
        authority: &ActorRef,
    ) -> Result<OperationReceipt, OutboxError> {
        if !governance_allowed(authority.kind) {
            return Err(OutboxError::AuthorityDenied);
        }
        let index = self
            .operations
            .iter()
            .position(|operation| &operation.id == id)
            .ok_or(OutboxError::InvalidState)?;
        let mut receipt = receipt_of(&self.operations[index]);
        match resolution {
            Resolution::Retain => {}
            Resolution::Requeue => {
                let operation = &mut self.operations[index];
                if !matches!(
                    operation.status,
                    OperationStatus::Rejected | OperationStatus::Conflicted
                ) {
                    return Err(OutboxError::InvalidState);
                }
                operation.status = OperationStatus::Pending;
                operation.reason = Some(OperationReason::Requeued);
                operation.conflict = None;
                receipt.status = OperationStatus::Pending;
                receipt.reason = Some(OperationReason::Requeued);
            }
            Resolution::Discard => {
                let operation = &self.operations[index];
                if !matches!(
                    operation.status,
                    OperationStatus::Rejected | OperationStatus::Conflicted
                ) {
                    return Err(OutboxError::InvalidState);
                }
                self.refuse_removal_with_dependents(index)?;
                self.operations.remove(index);
            }
            Resolution::Remove => {
                // The status-independent removal: the operation leaves the
                // outbox whatever its status, and the receipt names the
                // status it held. A removal another operation depends on is
                // refused: the dependent would name a dependency the outbox
                // does not hold, and it could never be sent.
                self.refuse_removal_with_dependents(index)?;
                self.operations.remove(index);
            }
        }
        self.persist()?;
        Ok(receipt)
    }

    /// Refuse a removal that another operation depends on.
    ///
    /// An operation another operation names as its dependency may not leave
    /// the outbox while that dependent is still queued. The dependent would
    /// name a dependency the outbox does not hold, and every later drain
    /// would defer it with [`OperationReason::DependencyPending`], for ever:
    /// the outbox holds no record of the dependency's outcome, and no drain
    /// could ever satisfy a dependency that is not there.
    ///
    /// The refusal is content-free and names neither the operation nor the
    /// dependent. The caller finds the dependents itself, in the outbox's own
    /// views, and resolves them first — the leaves first — so the removal
    /// becomes possible. A removal therefore never leaves behind a dependency
    /// that names an operation the outbox does not hold.
    fn refuse_removal_with_dependents(&self, index: usize) -> Result<(), OutboxError> {
        let identifier = &self.operations[index].id;
        let depended_on = self
            .operations
            .iter()
            .any(|operation| operation.depends_on.as_ref() == Some(identifier));
        if depended_on {
            return Err(OutboxError::DependencyPresent);
        }
        Ok(())
    }

    /// Supply one chunk of an asset transfer.
    ///
    /// The chunk's bytes are hashed and the digest is recorded; the bytes
    /// themselves are not held. A chunk that was already transferred is
    /// accepted again only when it hashes to the same digest, so a resumed
    /// transfer is idempotent and a changed chunk is refused with
    /// [`OutboxError::ChunkMismatch`].
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::InvalidState`] when the operation is unknown
    /// or is not an asset, [`OutboxError::InvalidInput`] when the chunk
    /// index or the chunk's length is outside the asset's geometry, and
    /// [`OutboxError::ChunkMismatch`] when the chunk contradicts a chunk
    /// already transferred.
    pub fn supply_asset_chunk(
        &mut self,
        id: &OperationId,
        chunk_index: u32,
        bytes: &[u8],
    ) -> Result<AssetChunkOutcome, OutboxError> {
        let index = self
            .operations
            .iter()
            .position(|operation| &operation.id == id)
            .ok_or(OutboxError::InvalidState)?;
        let OperationPayload::Asset(proposal) = &self.operations[index].payload else {
            return Err(OutboxError::InvalidState);
        };
        let total = chunk_count(proposal.length, proposal.chunk_size);
        if chunk_index >= total {
            return Err(OutboxError::InvalidInput);
        }
        let expected = chunk_length(proposal.length, proposal.chunk_size, chunk_index);
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != expected {
            return Err(OutboxError::InvalidInput);
        }
        let digest = chunk_digest(bytes);
        let state = self.operations[index]
            .asset
            .as_ref()
            .ok_or(OutboxError::InvalidState)?;
        if let Some(existing) = state.chunks.get(&chunk_index) {
            if *existing != digest {
                return Err(OutboxError::ChunkMismatch);
            }
            return Ok(AssetChunkOutcome {
                chunk_index,
                already_transferred: true,
                bytes: expected,
            });
        }
        let operation = &mut self.operations[index];
        let state = operation.asset.as_mut().ok_or(OutboxError::InvalidState)?;
        state.chunks.insert(chunk_index, digest);
        self.persist()?;
        Ok(AssetChunkOutcome {
            chunk_index,
            already_transferred: false,
            bytes: expected,
        })
    }

    /// The chunks an asset transfer still needs, in index order.
    ///
    /// A resumed transfer asks for exactly these chunks: the ones already
    /// transferred are never re-requested.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::InvalidState`] when the operation is unknown
    /// or is not an asset.
    pub fn pending_asset_chunks(&self, id: &OperationId) -> Result<Vec<u32>, OutboxError> {
        let operation = self
            .operations
            .iter()
            .find(|operation| &operation.id == id)
            .ok_or(OutboxError::InvalidState)?;
        let OperationPayload::Asset(proposal) = &operation.payload else {
            return Err(OutboxError::InvalidState);
        };
        let state = operation.asset.as_ref().ok_or(OutboxError::InvalidState)?;
        let total = chunk_count(proposal.length, proposal.chunk_size);
        let mut pending = Vec::new();
        for index in 0..total {
            if !state.chunks.contains_key(&index) {
                pending.push(index);
            }
        }
        Ok(pending)
    }

    /// The transfer state of an asset operation.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::InvalidState`] when the operation is unknown
    /// or is not an asset.
    pub fn asset_state(&self, id: &OperationId) -> Result<AssetProgress, OutboxError> {
        let operation = self
            .operations
            .iter()
            .find(|operation| &operation.id == id)
            .ok_or(OutboxError::InvalidState)?;
        let OperationPayload::Asset(proposal) = &operation.payload else {
            return Err(OutboxError::InvalidState);
        };
        let state = operation.asset.as_ref().ok_or(OutboxError::InvalidState)?;
        let total = chunk_count(proposal.length, proposal.chunk_size);
        let completed_bytes = state
            .chunks
            .keys()
            .map(|index| chunk_length(proposal.length, proposal.chunk_size, *index))
            .sum();
        Ok(AssetProgress {
            completed_chunks: state.completed_chunks(),
            total_chunks: total,
            completed_bytes,
            total_bytes: proposal.length,
        })
    }

    /// Verify a completed asset against the digest the operation declared.
    ///
    /// The verification uses the chunk digests the outbox recorded, so it
    /// needs no bytes. A transfer whose chunks do not produce the declared
    /// digest is reported as not matching: a corrupted chunk is detected.
    ///
    /// # Errors
    ///
    /// Returns [`OutboxError::InvalidState`] when the operation is unknown
    /// or is not an asset.
    pub fn verify_asset(&self, id: &OperationId) -> Result<AssetVerification, OutboxError> {
        let operation = self
            .operations
            .iter()
            .find(|operation| &operation.id == id)
            .ok_or(OutboxError::InvalidState)?;
        let OperationPayload::Asset(proposal) = &operation.payload else {
            return Err(OutboxError::InvalidState);
        };
        let state = operation.asset.as_ref().ok_or(OutboxError::InvalidState)?;
        let total = chunk_count(proposal.length, proposal.chunk_size);
        let complete = state.chunks.len() == usize::try_from(total).unwrap_or(usize::MAX);
        let matches_declared =
            complete && asset_digest(&state.ordered_digests()) == proposal.digest;
        Ok(AssetVerification {
            complete,
            matches_declared,
        })
    }

    /// Write the outbox back to disk, atomically.
    pub(super) fn persist(&self) -> Result<(), OutboxError> {
        sidecar::write_atomic(&self.path, &self.operations)
    }

    /// Record the conflicts of a merge plan on the operations they belong to.
    ///
    /// The affected operations are returned, so a caller that keeps a
    /// per-operation report can refresh it.
    pub(super) fn apply_conflicts(&mut self, plan: &MergePlan) -> Vec<usize> {
        let mut affected = Vec::new();
        for conflict in &plan.conflicts {
            for identifier in &conflict.operations {
                let Some(index) = self
                    .operations
                    .iter()
                    .position(|operation| &operation.id == identifier)
                else {
                    continue;
                };
                let operation = &mut self.operations[index];
                operation.status = OperationStatus::Conflicted;
                operation.reason = Some(OperationReason::MergeConflict);
                operation.conflict = Some(conflict.clone());
                affected.push(index);
            }
        }
        affected
    }

    /// Whether every member of a submission group has its dependency
    /// acknowledged.
    pub(super) fn dependencies_satisfied(&self, members: &[usize]) -> bool {
        for member in members {
            let Some(dependency) = &self.operations[*member].depends_on else {
                continue;
            };
            let Some(operation) = self
                .operations
                .iter()
                .find(|operation| &operation.id == dependency)
            else {
                return false;
            };
            if operation.status != OperationStatus::Acknowledged {
                return false;
            }
        }
        true
    }
}

/// The content-free progress of an asset transfer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetProgress {
    /// The number of chunks already transferred.
    pub completed_chunks: u32,
    /// The total number of chunks the transfer needs.
    pub total_chunks: u32,
    /// The number of bytes already transferred.
    pub completed_bytes: u64,
    /// The total number of bytes the asset holds.
    pub total_bytes: u64,
}

/// The verification of one completed asset transfer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetVerification {
    /// Whether every chunk of the asset has been transferred.
    pub complete: bool,
    /// Whether the transferred chunks produce the declared asset digest.
    pub matches_declared: bool,
}

/// The receipt of one operation, as the outbox holds it.
fn receipt_of(operation: &OutboxOperation) -> OperationReceipt {
    OperationReceipt {
        id: operation.id.clone(),
        status: operation.status,
        reason: operation.reason,
        sequence: operation.sequence,
    }
}

/// The content-free view of one operation.
fn view_of(operation: &OutboxOperation) -> OperationView {
    OperationView {
        id: operation.id.clone(),
        kind: operation.kind,
        status: operation.status,
        reason: operation.reason,
        depends_on: operation.depends_on.clone(),
        guild_ref: operation.binding.guild_ref.clone(),
        contract_revision: operation.binding.contract_revision,
        sequence: operation.sequence,
        enqueued_at_ms: operation.enqueued_at_ms,
        conflict: operation
            .conflict
            .as_ref()
            .map(super::merge::MergeConflict::summary),
    }
}

/// The content-free reason a verdict leaves behind.
pub(crate) fn reason_for(verdict: ReauthorizeOutcome) -> OperationReason {
    match verdict {
        ReauthorizeOutcome::Authorized => OperationReason::Authorized,
        ReauthorizeOutcome::Refused(reason) => OperationReason::Refused(reason),
        ReauthorizeOutcome::NeedsConsent => OperationReason::ConsentEpochMoved,
        ReauthorizeOutcome::SchemaSkew => OperationReason::SchemaSkew,
    }
}

/// Whether the authority may resolve a queued operation.
///
/// The governance order is the episode store's: a guild manager, a guild
/// administrator, a guild owner, and the organization owner. A service
/// identity and a human subject may not.
fn governance_allowed(kind: ActorKind) -> bool {
    matches!(
        kind,
        ActorKind::GuildManager
            | ActorKind::GuildAdministrator
            | ActorKind::GuildOwner
            | ActorKind::OrganizationOwner
    )
}

/// Check one operation before it is enqueued.
fn validate_new(operation: &NewOperation) -> Result<(), OutboxError> {
    let binding = &operation.binding;
    if !bounded_identifier(&binding.guild_ref, MAX_SCOPE_BYTES)
        || !bounded_identifier(&binding.policy_version, MAX_POLICY_VERSION_BYTES)
        || binding.contract_revision == 0
        || binding.contract_digest == [0; 32]
    {
        return Err(OutboxError::InvalidInput);
    }
    match &operation.payload {
        OperationPayload::Write(write) => validate_write(write, binding, false),
        OperationPayload::Delete(write) => validate_write(write, binding, true),
        OperationPayload::Asset(proposal) => validate_asset(proposal),
        OperationPayload::Edit(edit) => {
            if validate_edit(edit) {
                Ok(())
            } else {
                Err(OutboxError::InvalidInput)
            }
        }
    }
}

/// Check one proposed episode write against its binding.
fn validate_write(
    write: &EpisodeWrite,
    binding: &OperationBinding,
    must_forget: bool,
) -> Result<(), OutboxError> {
    // The outbox holds no canonical episode digest.
    if write.expected_commitment.is_some() {
        return Err(OutboxError::InvalidInput);
    }
    if write.quiet {
        return Err(OutboxError::InvalidInput);
    }
    if write.guild_ref != binding.guild_ref
        || write.contract_revision != binding.contract_revision
        || write.contract_digest != binding.contract_digest
        || write.policy_version != binding.policy_version
        || write.consent_epoch != binding.consent_epoch
    {
        return Err(OutboxError::InvalidInput);
    }
    if !bounded_identifier(&write.request_id, MAX_EPISODE_ID_BYTES)
        || !bounded_identifier(&write.operation_id, MAX_EPISODE_ID_BYTES)
        || write.token_cost > MAX_TOKEN_COST
    {
        return Err(OutboxError::InvalidInput);
    }
    let forgets = matches!(
        &write.event,
        EpisodeEvent::MemoryCandidate { candidate, .. } if candidate.forgets.is_some()
    );
    if forgets != must_forget {
        return Err(OutboxError::InvalidInput);
    }
    if !bounded_event(&write.event) {
        return Err(OutboxError::InvalidInput);
    }
    Ok(())
}

/// Whether every free-text identifier the event carries is bounded.
///
/// The outbox file must never hold free text: a proposal is a bounded
/// identifier and a commitment, never a rendered episode. The episode
/// vocabulary names a principal in every event and an embedding version in
/// a candidate, and both are bounded here exactly as the canonical store
/// bounds them.
fn bounded_event(event: &EpisodeEvent) -> bool {
    let actor_is_bounded = |actor: &ActorRef| bounded_identifier(&actor.principal_id, 64);
    match event {
        EpisodeEvent::Proposal {
            requested_by,
            proposed_by,
        } => actor_is_bounded(requested_by) && actor_is_bounded(proposed_by),
        EpisodeEvent::Approval { approved_by } => actor_is_bounded(approved_by),
        EpisodeEvent::Execution { executed_by, .. } => actor_is_bounded(executed_by),
        EpisodeEvent::Compensation { compensated_by, .. } => actor_is_bounded(compensated_by),
        EpisodeEvent::Terminal { .. } => true,
        EpisodeEvent::MemoryCandidate {
            recorded_by,
            candidate,
        } => {
            actor_is_bounded(recorded_by)
                && candidate
                    .embedding_version
                    .as_deref()
                    .is_none_or(|version| bounded_identifier(version, 64))
        }
        EpisodeEvent::MemoryEdge { recorded_by, .. } => actor_is_bounded(recorded_by),
    }
}

/// Check one proposed asset transfer.
fn validate_asset(proposal: &AssetProposal) -> Result<(), OutboxError> {
    if proposal.digest == [0; 32]
        || proposal.length == 0
        || proposal.chunk_size == 0
        || chunk_count(proposal.length, proposal.chunk_size) > MAX_ASSET_CHUNKS
    {
        return Err(OutboxError::InvalidInput);
    }
    Ok(())
}

/// Acquire the outbox's writer lock, tolerating a fork that briefly retains
/// an inherited descriptor.
fn acquire_writer_lock(file: &File) -> Result<(), OutboxError> {
    let deadline = Instant::now() + Duration::from_millis(50);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err(OutboxError::WriterBusy);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(std::fs::TryLockError::Error(_)) => return Err(OutboxError::Io),
        }
    }
}

/// The operations the drain groups into submissions.
pub(super) struct SubmissionGroup {
    /// The operations the submission covers.
    pub members: Vec<usize>,
    /// The proposal to submit.
    pub payload: OperationPayload,
}

/// The submission groups of a drain, in enqueue order.
pub(super) fn build_groups(
    operations: &[OutboxOperation],
    authorized: &[usize],
    plan: &MergePlan,
) -> Vec<SubmissionGroup> {
    let mut merged_members: BTreeSet<usize> = BTreeSet::new();
    let mut merged: Vec<(usize, SubmissionGroup)> = Vec::new();
    for object in &plan.merged {
        let members: Vec<usize> = object
            .contributors
            .iter()
            .filter_map(|identifier| {
                operations
                    .iter()
                    .position(|operation| &operation.id == identifier)
            })
            .collect();
        if members.is_empty() {
            continue;
        }
        let first = members.iter().copied().min().unwrap_or_default();
        merged_members.extend(members.iter().copied());
        merged.push((
            first,
            SubmissionGroup {
                members,
                payload: OperationPayload::Edit(EditProposal {
                    target: object.target.clone(),
                    fields: object.fields.clone(),
                }),
            },
        ));
    }
    let mut singles: Vec<(usize, SubmissionGroup)> = Vec::new();
    for &index in authorized {
        if merged_members.contains(&index) {
            continue;
        }
        singles.push((
            index,
            SubmissionGroup {
                members: vec![index],
                payload: operations[index].payload.clone(),
            },
        ));
    }
    let mut groups = singles;
    groups.extend(merged);
    groups.sort_by_key(|(first, _)| *first);
    groups.into_iter().map(|(_, group)| group).collect()
}
