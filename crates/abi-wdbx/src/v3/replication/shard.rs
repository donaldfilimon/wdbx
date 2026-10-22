//! The replicated shard: three voters, one fenced canonical writer.
//!
//! The shard owns the term, the membership, and the only path that mints a
//! canonical acknowledgement. It proposes a write to its members, counts
//! the durable acceptances, and certifies a receipt only when a quorum of
//! *distinct* members accepted the *same* digest under the shard's *current*
//! term.
//!
//! The certification step is crate-internal: [`QuorumShard::propose`] is the
//! only public path to a [`ShardReceipt`], and an acknowledgement can only be
//! obtained from a member that durably accepted the write.
//!
//! See the module documentation for the safety argument and for what this
//! slice deliberately does not implement.

use crate::v3::episode::{EpisodeWrite, StorePolicy};
use crate::v3::replication::error::ShardError;
use crate::v3::replication::receipt::{QuorumProof, ShardReceipt, VoterAck};
use crate::v3::replication::sidecar::prepare_directory;
use crate::v3::replication::state::{
    AcknowledgedEntry, SHARD_STATE_FILE, ShardStateBody, load_shard_state, store_shard_state,
};
use crate::v3::replication::term::{ShardTerm, VoterId, bounded_identifier};
use crate::v3::replication::voter::Voter;
use std::path::{Path, PathBuf};

/// The number of voters a shard holds.
pub const VOTER_COUNT: usize = 3;

/// The number of voters that must accept a write for it to be canonical.
pub const QUORUM_SIZE: usize = 2;

/// The number of acknowledged writes a shard keeps in its durable state.
const MAX_ACKNOWLEDGED: usize = 4_096;

/// Maximum length of a scope identifier.
const MAX_SCOPE: usize = 128;

/// Whether the shard's members still agree with what it acknowledged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgreementState {
    /// Every available member holds every quorum-acknowledged write.
    Intact,
    /// The shard cannot show its agreement: a member is missing a
    /// quorum-acknowledged write, or a member's ledger is damaged.
    Diverged,
}

/// One member of the shard and the state it could be opened in.
#[derive(Debug)]
struct VoterSlot {
    id: VoterId,
    voter: Option<Voter>,
    failure: Option<ShardError>,
}

/// The proposal whose acknowledgements the shard is currently willing to
/// certify.
#[derive(Clone, Debug)]
struct PendingProposal {
    term: ShardTerm,
    request_id: String,
}

/// A three-voter replicated shard with one quorum-fenced canonical writer.
#[derive(Debug)]
pub struct QuorumShard {
    root: PathBuf,
    scope: String,
    policy: StorePolicy,
    term: ShardTerm,
    members: Vec<VoterId>,
    voters: Vec<VoterSlot>,
    acknowledged: Vec<AcknowledgedEntry>,
    pending: Option<PendingProposal>,
}

impl QuorumShard {
    /// Open the shard rooted at `root` and bound to `scope`.
    ///
    /// The shard's term, its membership, and the writes it acknowledged are
    /// recovered from its durable state file. A voter that cannot be opened
    /// is recorded as unavailable: the shard still opens with the members
    /// it can reach, and it refuses to acknowledge a write it cannot form a
    /// quorum for.
    pub fn open(
        root: impl AsRef<Path>,
        scope: &str,
        policy: StorePolicy,
    ) -> Result<Self, ShardError> {
        if !bounded_identifier(scope, MAX_SCOPE) {
            return Err(ShardError::InvalidInput);
        }
        if !policy.guilds.contains_key(scope) {
            return Err(ShardError::ScopeMismatch);
        }
        let root = prepare_directory(root.as_ref())?;
        let state_path = root.join(SHARD_STATE_FILE);
        let existed = state_path.exists();
        let body = load_shard_state(&state_path)?;
        let loaded_term = body.term()?;
        let members = {
            let recovered = body.member_ids()?;
            if recovered.is_empty() {
                default_members()?
            } else {
                recovered
            }
        };
        if members.len() != VOTER_COUNT {
            return Err(ShardError::Corrupt);
        }
        let mut term = loaded_term;
        let mut voters = Vec::with_capacity(VOTER_COUNT);
        for id in &members {
            let directory = root.join(id.as_str());
            match Voter::open(&directory, id.clone(), policy.clone()) {
                Ok(voter) => {
                    term = term.max(voter.term());
                    voters.push(VoterSlot {
                        id: id.clone(),
                        voter: Some(voter),
                        failure: None,
                    });
                }
                Err(error) => voters.push(VoterSlot {
                    id: id.clone(),
                    voter: None,
                    failure: Some(error),
                }),
            }
        }
        let acknowledged = body.acknowledged().to_vec();
        let mut shard = Self {
            root,
            scope: scope.to_string(),
            policy,
            term,
            members,
            voters,
            acknowledged,
            pending: None,
        };
        // The term is the fence. A shard whose state file was lost must not
        // re-open an epoch its members already left behind. The state file
        // is only rewritten when it is absent or when the term moved, so a
        // second shard over the same root cannot overwrite what the first
        // one acknowledged.
        if !existed || term != loaded_term {
            shard.persist_state()?;
        }
        for slot in &mut shard.voters {
            if let Some(voter) = slot.voter.as_mut() {
                voter.observe_term(shard.term)?;
            }
        }
        Ok(shard)
    }

    /// The scope this shard owns.
    #[must_use]
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// The shard's current fencing epoch.
    #[must_use]
    pub fn term(&self) -> ShardTerm {
        self.term
    }

    /// The shard's members.
    #[must_use]
    pub fn members(&self) -> &[VoterId] {
        &self.members
    }

    /// How many members had to accept a write for it to be canonical.
    #[must_use]
    pub const fn quorum_size(&self) -> usize {
        QUORUM_SIZE
    }

    /// How many members the shard holds.
    #[must_use]
    pub fn voter_count(&self) -> usize {
        self.voters.len()
    }

    /// How many members the shard can currently reach.
    #[must_use]
    pub fn available_voters(&self) -> usize {
        self.voters
            .iter()
            .filter(|slot| slot.voter.is_some())
            .count()
    }

    /// The state of one member.
    ///
    /// The reason is a content-free label: a member that cannot be opened
    /// says why, never where.
    #[must_use]
    pub fn voter_status(&self, id: &VoterId) -> Option<VoterStatus> {
        self.voters
            .iter()
            .find(|slot| &slot.id == id)
            .map(|slot| VoterStatus {
                id: slot.id.clone(),
                available: slot.voter.is_some(),
                failure: slot.failure,
                term: slot.voter.as_ref().map_or(self.term, Voter::term),
            })
    }

    /// The shard's members, read-only.
    pub fn voters(&self) -> impl Iterator<Item = &Voter> {
        self.voters.iter().filter_map(|slot| slot.voter.as_ref())
    }

    /// One member of the shard, mutably.
    #[cfg(test)]
    pub(crate) fn voter_mut(&mut self, id: &VoterId) -> Option<&mut Voter> {
        self.voters
            .iter_mut()
            .find(|slot| &slot.id == id)
            .and_then(|slot| slot.voter.as_mut())
    }

    /// Whether the shard's members still agree with what it acknowledged.
    ///
    /// An available member that does not hold a quorum-acknowledged write
    /// has diverged. An *unreachable* member cannot be checked and does not
    /// by itself break the agreement: the shard keeps serving with the
    /// members it can reach, and reports what it cannot prove. A member
    /// whose ledger is *damaged* is different: its content is broken, so
    /// the shard cannot show its agreement and refuses to acknowledge.
    #[must_use]
    pub fn agreement_state(&self) -> AgreementState {
        if self
            .voters
            .iter()
            .any(|slot| slot.failure == Some(ShardError::Corrupt))
        {
            return AgreementState::Diverged;
        }
        for entry in &self.acknowledged {
            let mut holders = 0_usize;
            for slot in self.voters.iter().filter_map(|slot| slot.voter.as_ref()) {
                if slot.holds(&self.scope, &entry.digest) {
                    holders += 1;
                } else {
                    return AgreementState::Diverged;
                }
            }
            if holders < QUORUM_SIZE {
                return AgreementState::Diverged;
            }
        }
        AgreementState::Intact
    }

    /// Whether the shard acknowledges a digest as canonical.
    ///
    /// The answer is `false` once the shard has detected a divergence: a
    /// write one member has lost is not acknowledged, because the shard
    /// cannot promise what it cannot show.
    #[must_use]
    pub fn is_acknowledged(&self, digest: &[u8; 32]) -> bool {
        self.agreement_state() == AgreementState::Intact
            && self
                .acknowledged
                .iter()
                .any(|entry| &entry.digest == digest)
    }

    /// The writes the shard acknowledged.
    ///
    /// The summaries are rebuilt from the durable entries; the public view
    /// never exposes the internal entry type.
    #[must_use]
    pub fn acknowledged_writes(&self) -> Vec<AcknowledgedSummary> {
        self.acknowledged
            .iter()
            .map(|entry| AcknowledgedSummary {
                digest: entry.digest,
                sequence: entry.sequence,
                term: ShardTerm::from_u64(entry.term).unwrap_or(self.term),
            })
            .collect()
    }

    /// Propose a write and acknowledge it if a quorum accepts it.
    ///
    /// This is the **only public path** to a canonical acknowledgement. The
    /// certification step that mints a receipt is crate-internal
    /// ([`Self::certify`]), so a caller outside this crate cannot turn a
    /// hand-picked set of acknowledgements into a receipt: it can only ask
    /// the shard to propose a write, and the shard proposes it itself.
    ///
    /// The proposal is refused when fewer than a quorum of members durably
    /// accepted it, when the members accepted different digests, or when a
    /// member has diverged.
    pub fn propose(&mut self, write: &EpisodeWrite) -> Result<ShardReceipt, ShardError> {
        let term = self.term;
        self.propose_with_term(write, term)
    }

    /// Propose a write under an explicit term.
    ///
    /// A term that is not the shard's current term is refused. This is the
    /// fence a stale writer runs into.
    ///
    /// This is crate-internal on purpose: [`Self::propose`] is the ordinary
    /// public path, and it proposes under the shard's current term. A caller
    /// that could name its own term could also name a stale one, and the
    /// public surface would then offer a second way to reach a receipt.
    pub(crate) fn propose_with_term(
        &mut self,
        write: &EpisodeWrite,
        term: ShardTerm,
    ) -> Result<ShardReceipt, ShardError> {
        if term != self.term {
            return Err(ShardError::StaleTerm);
        }
        let acks = self.collect_acks(write)?;
        self.certify(&acks)
    }

    /// Collect the members' durable acceptances of one write.
    ///
    /// The acknowledgements are the shard's own proposal receipts; they are
    /// not a canonical acknowledgement. Nothing here can produce a receipt
    /// on its own: the certification step ([`Self::certify`]) is crate-
    /// internal, and a [`VoterAck`] has no public constructor, so an
    /// acknowledgement can only come from a member that durably accepted the
    /// write.
    pub fn collect_acks(&mut self, write: &EpisodeWrite) -> Result<Vec<VoterAck>, ShardError> {
        self.check_scope(write)?;
        self.check_agreement()?;
        let term = self.term;
        let mut acks = Vec::new();
        for slot in &mut self.voters {
            let Some(voter) = slot.voter.as_mut() else {
                continue;
            };
            match voter.accept(write, term) {
                Ok(ack) => acks.push(ack),
                // A member whose term is ahead of the shard's is a fence the
                // shard has to learn about, not a write it may ignore.
                Err(ShardError::StaleTerm) => return Err(ShardError::StaleTerm),
                // Every other refusal means this member did not accept the
                // write. It is not counted towards the quorum.
                Err(_) => {}
            }
        }
        self.pending = Some(PendingProposal {
            term,
            request_id: write.request_id.clone(),
        });
        Ok(acks)
    }

    /// Certify a quorum of acknowledgements into a canonical receipt.
    ///
    /// The acknowledgements must come from at least a quorum of *distinct*
    /// members of this shard, must all carry the shard's current term, must
    /// all carry the same digest, and must all belong to the proposal the
    /// shard last made. Anything else is refused.
    ///
    /// This step is crate-internal on purpose: it is the only place a
    /// [`ShardReceipt`] is minted, and keeping it inside the crate is what
    /// makes [`Self::propose`] the only public path to one. The checks below
    /// hold independently of how the acknowledgements were obtained: a
    /// certification always requires a pending proposal this shard actually
    /// made, a quorum of *distinct* members, the shard's current term, and
    /// one identical digest.
    pub(crate) fn certify(&mut self, acks: &[VoterAck]) -> Result<ShardReceipt, ShardError> {
        self.check_agreement()?;
        let term = self.term;
        // A stale acknowledgement is refused on its term alone, before the
        // shard looks at anything else.
        for ack in acks {
            if ack.term() != term {
                return Err(ShardError::StaleTerm);
            }
        }
        let pending = self.pending.clone().ok_or(ShardError::InvalidInput)?;
        if pending.term != term {
            return Err(ShardError::StaleTerm);
        }
        let mut members: Vec<VoterId> = Vec::new();
        let mut digest: Option<[u8; 32]> = None;
        let mut sequence: Option<u64> = None;
        for ack in acks {
            if !self.members.iter().any(|member| member == ack.voter()) {
                return Err(ShardError::InvalidInput);
            }
            if ack.request_id() != pending.request_id {
                return Err(ShardError::Conflict);
            }
            if members.iter().any(|member| member == ack.voter()) {
                continue;
            }
            match digest {
                None => {
                    digest = Some(*ack.digest());
                    sequence = Some(ack.sequence());
                }
                Some(found) if found != *ack.digest() => return Err(ShardError::Conflict),
                Some(_) => {}
            }
            members.push(ack.voter().clone());
        }
        if members.len() < QUORUM_SIZE {
            return Err(ShardError::NoQuorum);
        }
        let digest = digest.ok_or(ShardError::NoQuorum)?;
        let sequence = sequence.unwrap_or_default();
        if !self.acknowledged.iter().any(|entry| entry.digest == digest) {
            if self.acknowledged.len() >= MAX_ACKNOWLEDGED {
                return Err(ShardError::BudgetExhausted);
            }
            self.acknowledged.push(AcknowledgedEntry {
                digest,
                sequence,
                request_id: pending.request_id.clone(),
                term: term.as_u64(),
            });
            self.persist_state()?;
        }
        Ok(ShardReceipt::new(
            QuorumProof::new(term, digest, members.clone()),
            self.scope.clone(),
            sequence,
            pending.request_id,
            members.len(),
            QUORUM_SIZE,
        ))
    }

    /// Advance the shard's fencing epoch.
    ///
    /// The new term is durable before any member learns about it, so a
    /// crash leaves the shard ahead of its members rather than behind them.
    /// A writer holding the previous term is fenced out from that moment.
    pub fn advance_term(&mut self) -> Result<ShardTerm, ShardError> {
        let highest = self
            .voters
            .iter()
            .filter_map(|slot| slot.voter.as_ref())
            .map(Voter::term)
            .max()
            .unwrap_or(self.term);
        let next = if highest >= self.term {
            highest.next()
        } else {
            self.term.next()
        };
        self.term = next;
        self.pending = None;
        self.persist_state()?;
        for slot in &mut self.voters {
            if let Some(voter) = slot.voter.as_mut() {
                voter.observe_term(next)?;
            }
        }
        Ok(next)
    }

    /// Isolate one member as the minority side of a partition.
    ///
    /// The returned shard holds that one member and can reach no other. It
    /// can accept writes locally; it can never acknowledge them, because its
    /// API has no way to return a receipt.
    pub fn isolate_minority(&mut self, id: &VoterId) -> Result<MinorityShard<'_>, ShardError> {
        let index = self
            .voters
            .iter()
            .position(|slot| &slot.id == id)
            .ok_or(ShardError::InvalidInput)?;
        let term = self.term;
        let scope = self.scope.clone();
        let (head, tail) = self.voters.split_at_mut(index);
        let (target, rest) = tail.split_at_mut(1);
        let _ = (head, rest);
        let mut voters = Vec::new();
        for slot in target {
            if let Some(voter) = slot.voter.as_mut() {
                voters.push(voter);
            }
        }
        Ok(MinorityShard {
            term,
            scope,
            voters,
        })
    }

    /// Try to open a member that could not be opened before.
    ///
    /// A member that comes back with a *different* history than the shard
    /// acknowledged makes the shard diverge, and the shard then refuses to
    /// acknowledge new writes until the divergence is repaired.
    pub fn reopen_voter(&mut self, id: &VoterId) -> Result<(), ShardError> {
        let slot = self
            .voters
            .iter_mut()
            .find(|slot| &slot.id == id)
            .ok_or(ShardError::InvalidInput)?;
        if slot.voter.is_some() {
            return Ok(());
        }
        let directory = self.root.join(id.as_str());
        let mut voter = Voter::open(&directory, id.clone(), self.policy.clone())?;
        voter.observe_term(self.term)?;
        slot.voter = Some(voter);
        slot.failure = None;
        Ok(())
    }

    fn check_scope(&self, write: &EpisodeWrite) -> Result<(), ShardError> {
        if write.guild_ref != self.scope {
            return Err(ShardError::ScopeMismatch);
        }
        Ok(())
    }

    fn check_agreement(&self) -> Result<(), ShardError> {
        if self.agreement_state() == AgreementState::Diverged {
            return Err(ShardError::Diverged);
        }
        Ok(())
    }

    fn persist_state(&self) -> Result<(), ShardError> {
        let body = ShardStateBody::new(self.term, &self.members, &self.acknowledged);
        store_shard_state(&self.root.join(SHARD_STATE_FILE), &body)
    }
}

/// The state of one member of the shard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoterStatus {
    /// The member's identity.
    pub id: VoterId,
    /// Whether the member could be opened.
    pub available: bool,
    /// Why it could not be opened, when it could not.
    pub failure: Option<ShardError>,
    /// The term the member last acknowledged under.
    pub term: ShardTerm,
}

/// A content-free summary of one acknowledged write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AcknowledgedSummary {
    /// The canonical episode digest.
    pub digest: [u8; 32],
    /// The ledger sequence the write took.
    pub sequence: u64,
    /// The term the acknowledgement was formed under.
    pub term: ShardTerm,
}

/// The outcome of a proposal made inside a minority partition.
///
/// It is deliberately **not** a receipt: it carries no canonical digest, no
/// sequence, and no quorum proof, and there is no conversion from it into a
/// [`ShardReceipt`]. A minority write is durable on the minority's own
/// members and is *not* canonical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MinorityOutcome {
    accepted_by: usize,
    quorum_size: usize,
}

impl MinorityOutcome {
    /// How many members of the minority durably accepted the write.
    #[must_use]
    pub fn accepted_by(&self) -> usize {
        self.accepted_by
    }

    /// How many members had to accept the write for it to be canonical.
    #[must_use]
    pub fn quorum_size(&self) -> usize {
        self.quorum_size
    }

    /// Whether the minority reached a quorum.
    ///
    /// Always `false` for a minority: it holds fewer members than a quorum
    /// needs.
    #[must_use]
    pub fn reached_quorum(&self) -> bool {
        self.accepted_by >= self.quorum_size
    }
}

/// The minority side of a partition.
///
/// The minority holds fewer members than a quorum needs. It can accept a
/// write on the members it can reach, and it refuses to acknowledge it:
/// `propose` returns a [`MinorityOutcome`], not a receipt, and no method
/// anywhere in this type returns a [`ShardReceipt`].
#[derive(Debug)]
pub struct MinorityShard<'a> {
    term: ShardTerm,
    scope: String,
    voters: Vec<&'a mut Voter>,
}

impl MinorityShard<'_> {
    /// The fencing epoch the minority is in.
    #[must_use]
    pub fn term(&self) -> ShardTerm {
        self.term
    }

    /// How many members the minority can reach.
    #[must_use]
    pub fn voter_count(&self) -> usize {
        self.voters.len()
    }

    /// Propose a write inside the minority.
    ///
    /// The write is durable on the members the minority can reach. It is
    /// *not* canonical, and this method has no way to say that it is: the
    /// return type carries no digest and no proof.
    pub fn propose(&mut self, write: &EpisodeWrite) -> Result<MinorityOutcome, ShardError> {
        if write.guild_ref != self.scope {
            return Err(ShardError::ScopeMismatch);
        }
        let mut accepted = 0_usize;
        for voter in &mut self.voters {
            match voter.accept(write, self.term) {
                Ok(_) => accepted += 1,
                Err(ShardError::StaleTerm) => return Err(ShardError::StaleTerm),
                Err(_) => {}
            }
        }
        Ok(MinorityOutcome {
            accepted_by: accepted,
            quorum_size: QUORUM_SIZE,
        })
    }
}

/// The default membership of a fresh shard.
fn default_members() -> Result<Vec<VoterId>, ShardError> {
    (0..VOTER_COUNT)
        .map(|index| {
            let name = format!("voter_{index}");
            VoterId::new(&name)
        })
        .collect()
}
