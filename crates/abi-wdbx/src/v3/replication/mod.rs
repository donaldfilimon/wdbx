//! Quorum-fenced replication for the v3 episode ledger.
//!
//! This module adds a *replicated shard* around the single-writer episode
//! ledger ([`crate::v3::episode::EpisodeStore`]). One shard owns three
//! voters; each voter is an independent replica with its own directory, its
//! own writer lock, and its own ledger. A write becomes canonical only when
//! a quorum of the voters has durably accepted it, and the shard that mints
//! the canonical acknowledgement is fenced by a monotonically increasing
//! term.
//!
//! It answers the global constraint that each tenant/scope has one
//! quorum-fenced canonical writer in a three-voter replicated shard, and
//! that partitioned minorities cannot acknowledge canonical writes.
//!
//! # Why a minority cannot acknowledge
//!
//! Three properties combine, and each is enforced by the type system and the
//! control flow rather than by a test:
//!
//! 1. **A receipt needs a quorum.** [`ShardReceipt`] holds a private
//!    [`QuorumProof`], and the only constructor of that proof requires
//!    acknowledgements from at least [`QUORUM_SIZE`] *distinct* members
//!    that all carry the shard's current term and the same digest.
//!    [`ShardReceipt`] has no public constructor and no `Deserialize`
//!    implementation, so a receipt cannot be built — or rebuilt — outside
//!    the crate. The certification step that mints one is crate-internal as
//!    well, so [`QuorumShard::propose`] is the only public path to a
//!    receipt: a caller outside the crate cannot certify acknowledgements
//!    of its own choosing.
//! 2. **A minority cannot produce a quorum.** [`MinorityShard`] holds fewer
//!    members than a quorum needs. Its `propose` returns a
//!    [`MinorityOutcome`], which carries no digest, no sequence, and no
//!    proof, and has no conversion into a receipt. The minority can accept
//!    writes on the members it can reach; it cannot acknowledge them.
//! 3. **The term fences stale writers.** The shard's term is durable and
//!    monotonic, and every acknowledgement carries the term it was accepted
//!    under. A write proposed under an older term is refused, and an
//!    acknowledgement carrying an older term is refused, by the shard and by
//!    the voter that produced it.
//!
//! The third property is what makes "one canonical writer per scope"
//! enforceable across a partition: the minority that keeps writing under an
//! old term produces acknowledgements the shard will not certify, and the
//! majority that holds the current term is the only writer whose
//! acknowledgements count.
//!
//! # What a minority write is
//!
//! A minority write is *durable* and *not canonical*. The minority's own
//! member holds it, and the shard never acknowledged it. When the
//! partition heals, the majority re-proposes the write: the member that
//! already holds it acknowledges idempotently, and the write becomes
//! canonical with the digest it was first accepted under. Nothing is
//! appended twice.
//!
//! # How a member failure is treated
//!
//! The three member states are told apart, because they mean different
//! things:
//!
//! - **Unreachable** (the directory or the ledger cannot be opened): the
//!   member's content is unknown, not known-bad. The surviving quorum still
//!   holds every acknowledged write, so the shard keeps acknowledging. This
//!   is the "one voter fails" case: no quorum-acknowledged write is lost.
//! - **Diverged** (the member is reachable but does not hold a write the
//!   shard acknowledged): the shard refuses to acknowledge anything new,
//!   and it stops reporting the write as acknowledged. Refusing is the
//!   safer direction: the shard will not promise what it cannot show.
//! - **Damaged** (the member's ledger does not replay): the member's
//!   content is broken, so the shard cannot show its agreement either, and
//!   it refuses to acknowledge anything new.
//!
//! # What this is not
//!
//! This is not a consensus protocol, and it is not a federation.
//!
//! - **No Byzantine fault tolerance.** A voter is assumed to store what it
//!   accepted and to report it honestly. A voter that lies about accepting
//!   a write is outside the model; this slice cannot detect that.
//! - **No leader election.** The term advances when an operator advances
//!   it, not through a vote. There is no election timeout, no log
//!   matching, and no commit index.
//! - **No anti-entropy.** A member that has lost a write is *detected* and
//!   the shard then refuses to acknowledge new writes; nothing here
//!   repairs it. Catch-up replication is not implemented.
//! - **The agreement check is linear.** Every acknowledged write is looked
//!   up in every reachable member's ledger, so the check costs the
//!   acknowledged count times the ledger length. It is a slice's worth of
//!   work, not a replicated index.
//! - **No network.** The voters are local directories in one process. A
//!   partitioned minority is modelled by giving one member its own shard
//!   view; the transport, and the failure modes a real transport has, are
//!   not implemented.
//! - **Local disks only.** Durability is the episode store's file-level
//!   durability plus the shard's atomic sidecar replacement. This is not
//!   power-loss evidence, and the other platforms keep file-only
//!   synchronization, exactly as the episode store does.
//! - **One scope per shard.** A shard is bound to a single tenant/scope at
//!   open time and refuses a write for any other scope. Cross-scope
//!   mixing is prevented by the shard, and the episode store's own guild
//!   scoping still applies underneath.
//!
//! # Evidence
//!
//! `crates/abi-wdbx/tests/v3_quorum_shard.rs` pins the behaviour this
//! module claims: quorum formation, minority refusal, term fencing, a
//! failed member, restart recovery, divergence detection, scope isolation,
//! and ledger agreement. It is deterministic and local: no network, no
//! wall-clock dependence, and explicit term numbers.

mod error;
mod receipt;
mod shard;
mod sidecar;
mod state;
mod term;
mod voter;

#[cfg(test)]
mod tests;

pub use error::ShardError;
pub use receipt::{QuorumProof, ShardReceipt, VoterAck};
pub use shard::{
    AcknowledgedSummary, AgreementState, MinorityOutcome, MinorityShard, QUORUM_SIZE, QuorumShard,
    VOTER_COUNT, VoterStatus,
};
pub use term::{ShardTerm, VoterId};
pub use voter::Voter;
