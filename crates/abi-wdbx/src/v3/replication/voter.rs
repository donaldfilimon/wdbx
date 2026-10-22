//! One replica of the shard.
//!
//! A voter owns a directory and its own single-writer episode store. It
//! accepts a proposed write, appends it durably through the store's own
//! write gate, and acknowledges it locally with the term it accepted it
//! under.
//!
//! A voter is not a canonical writer. It acknowledges *to* the shard, and
//! the shard is the only thing that can turn acknowledgements into a
//! canonical receipt.

use crate::v3::episode::{EpisodeStore, EpisodeStoreError, EpisodeWrite, StorePolicy};
use crate::v3::replication::error::ShardError;
use crate::v3::replication::receipt::VoterAck;
use crate::v3::replication::sidecar::prepare_directory;
use crate::v3::replication::state::{VOTER_TERM_FILE, load_voter_term, store_voter_term};
use crate::v3::replication::term::{ShardTerm, VoterId};
use std::path::{Path, PathBuf};

/// The number of receipts the store returns in one window.
///
/// The store bounds a retrieval window; the voter searches that window for
/// a replayed request identifier. A request older than the window is
/// refused rather than acknowledged with a guess.
const RECEIPT_WINDOW: usize = 2_048;

/// A replica of the shard.
#[derive(Debug)]
pub struct Voter {
    id: VoterId,
    directory: PathBuf,
    store: EpisodeStore,
    term: ShardTerm,
}

impl Voter {
    /// Open (or create) the voter rooted at `directory`.
    pub(crate) fn open(
        directory: &Path,
        id: VoterId,
        policy: StorePolicy,
    ) -> Result<Self, ShardError> {
        let directory = prepare_directory(directory)?;
        let term = load_voter_term(&directory.join(VOTER_TERM_FILE))?;
        let store =
            EpisodeStore::open(&directory, policy).map_err(|error| map_store_error(&error))?;
        Ok(Self {
            id,
            directory,
            store,
            term,
        })
    }

    /// The member's identity inside the shard.
    #[must_use]
    pub fn id(&self) -> &VoterId {
        &self.id
    }

    /// The highest term this voter has acknowledged under.
    #[must_use]
    pub fn term(&self) -> ShardTerm {
        self.term
    }

    /// The directory this voter owns.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The voter's own episode store.
    #[must_use]
    pub fn store(&self) -> &EpisodeStore {
        &self.store
    }

    /// Adopt a newer term.
    ///
    /// A term below the voter's own is refused: that is the fence. A term
    /// above it is adopted and made durable before the voter acknowledges
    /// anything under it.
    pub(crate) fn observe_term(&mut self, term: ShardTerm) -> Result<(), ShardError> {
        if term < self.term {
            return Err(ShardError::StaleTerm);
        }
        if term == self.term {
            return Ok(());
        }
        store_voter_term(&self.directory.join(VOTER_TERM_FILE), term)?;
        self.term = term;
        Ok(())
    }

    /// Accept a proposed write and acknowledge it locally.
    ///
    /// The write goes through the store's own write gate, so policy,
    /// consent, replay, and budget rules apply exactly as they do for a
    /// single-writer store. The acknowledgement is only produced after the
    /// append is durable.
    pub(crate) fn accept(
        &mut self,
        write: &EpisodeWrite,
        term: ShardTerm,
    ) -> Result<VoterAck, ShardError> {
        if term < self.term {
            return Err(ShardError::StaleTerm);
        }
        if term > self.term {
            self.observe_term(term)?;
        }
        match self.store.propose_write(write) {
            Ok(receipt) => Ok(VoterAck::new(
                self.id.clone(),
                term,
                receipt.episode_digest,
                receipt.sequence,
                write.request_id.clone(),
            )),
            Err(EpisodeStoreError::Replay) => self.replayed_ack(write, term),
            Err(error) => Err(map_store_error(&error)),
        }
    }

    /// Acknowledge a write this voter already accepted.
    ///
    /// The store refuses a replayed request identifier. The acknowledgement
    /// is rebuilt from the receipt the store still holds, so a retried
    /// write is acknowledged with the digest it was first accepted under
    /// and is never appended twice.
    ///
    /// The store's receipt window is bounded, so a request older than that
    /// window cannot be looked up. Such a replay is refused rather than
    /// acknowledged with a guess: the safe direction is to withhold an
    /// acknowledgement, never to invent one.
    fn replayed_ack(&self, write: &EpisodeWrite, term: ShardTerm) -> Result<VoterAck, ShardError> {
        let receipts = self
            .store
            .retrieve(&write.guild_ref, RECEIPT_WINDOW)
            .map_err(|error| map_store_error(&error))?;
        let found = receipts
            .into_iter()
            .find(|receipt| receipt.request_id == write.request_id)
            .ok_or(ShardError::VoterRejected)?;
        Ok(VoterAck::new(
            self.id.clone(),
            term,
            found.episode_digest,
            found.sequence,
            write.request_id.clone(),
        ))
    }

    /// Whether this voter holds one digest for a guild.
    pub(crate) fn holds(&self, guild_ref: &str, digest: &[u8; 32]) -> bool {
        self.store
            .find_receipt(guild_ref, digest)
            .ok()
            .flatten()
            .is_some()
    }
}

/// Map an episode-store failure onto a content-free shard failure.
fn map_store_error(error: &EpisodeStoreError) -> ShardError {
    match error {
        EpisodeStoreError::Io => ShardError::Io,
        EpisodeStoreError::WriterBusy => ShardError::WriterBusy,
        EpisodeStoreError::Corrupt => ShardError::Corrupt,
        EpisodeStoreError::TokenBudget | EpisodeStoreError::StorageBudget => {
            ShardError::BudgetExhausted
        }
        EpisodeStoreError::Canonical(_)
        | EpisodeStoreError::InvalidInput
        | EpisodeStoreError::StaleBinding
        | EpisodeStoreError::LearningDisabled
        | EpisodeStoreError::Quiet
        | EpisodeStoreError::Replay
        | EpisodeStoreError::InvalidTransition
        | EpisodeStoreError::CommitmentMismatch => ShardError::VoterRejected,
    }
}
