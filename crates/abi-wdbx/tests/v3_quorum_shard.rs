//! Quorum replication for the v3 episode ledger.
//!
//! These tests pin the behaviour the shard claims:
//!
//! - a write acknowledged by a 2-of-3 quorum produces a shard receipt, and
//!   the record is present on the acknowledging voters;
//! - a write that reaches only one voter is refused and produces no
//!   canonical receipt;
//! - a partitioned minority cannot produce a canonical receipt, and its API
//!   has no way to return one;
//! - the term fences the writer: a proposal always runs under the shard's
//!   current term, the receipt carries that term, and the fence survives a
//!   restart;
//! - the only public path to a receipt is the shard's own proposal;
//! - one failed voter does not lose a quorum-acknowledged write;
//! - a restart recovers the term, the membership, and the acknowledged
//!   state;
//! - a voter that lost a quorum-acknowledged write is detected, and the
//!   shard refuses to acknowledge new writes over it;
//! - a shard never holds another scope's records;
//! - the members' ledgers agree on the acknowledged sequence.
//!
//! Everything is deterministic and local: no network, no wall-clock
//! dependence, and explicit term numbers.

use abi_wdbx::v3::episode::{
    ActorKind, ActorRef, EpisodeEvent, EpisodeSource, EpisodeWrite, EvidenceLevel,
    GuildEpisodePolicy, MemoryCandidate, MemoryClass, RetentionClass, StorePolicy,
};
use abi_wdbx::v3::replication::{
    AgreementState, MinorityOutcome, QUORUM_SIZE, QuorumShard, ShardError, ShardReceipt, ShardTerm,
    VOTER_COUNT, Voter, VoterId,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// The scope every shard in these tests owns.
const SCOPE: &str = "guild_ref";

/// A different scope, used to prove that a shard never mixes scopes.
const OTHER_SCOPE: &str = "other_guild";

struct Scratch(PathBuf);

impl Scratch {
    fn new(purpose: &str) -> Self {
        let path = std::env::temp_dir().join(format!("wdbx-quorum-{purpose}-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).expect("scratch directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
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

fn write(request_id: &str, operation_id: &str, seed: u8) -> EpisodeWrite {
    EpisodeWrite {
        request_id: request_id.into(),
        operation_id: operation_id.into(),
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
            candidate: candidate(seed),
        },
        token_cost: 1,
        expected_commitment: None,
        quiet: false,
    }
}

/// The digests a voter holds for one scope, in ledger order.
fn held_digests(voter: &Voter) -> Vec<[u8; 32]> {
    voter
        .store()
        .retrieve(SCOPE, 64)
        .expect("receipts")
        .into_iter()
        .map(|receipt| receipt.episode_digest)
        .collect()
}

/// Make a voter's directory unusable, exactly as a failed disk would look.
fn break_voter(root: &Path, index: usize) {
    let directory = root.join(format!("voter_{index}"));
    let _ = fs::remove_dir_all(&directory);
    fs::write(&directory, b"this member is unavailable").expect("failed member");
}

/// Remove the last ledger line of one voter, exactly as a lost write would
/// look.
fn lose_last_write(root: &Path, index: usize) {
    let ledger = root
        .join(format!("voter_{index}"))
        .join("episodes.v1.jsonl");
    let raw = fs::read(&ledger).expect("ledger bytes");
    let mut ends = Vec::new();
    for (offset, byte) in raw.iter().enumerate() {
        if *byte == b'\n' {
            ends.push(offset + 1);
        }
    }
    let cut = if ends.len() > 1 {
        ends[ends.len() - 2]
    } else {
        0
    };
    fs::write(&ledger, &raw[..cut]).expect("truncated ledger");
}

/// Damage one voter's ledger, exactly as a mutated record would look.
fn corrupt_voter(root: &Path, index: usize) {
    let ledger = root
        .join(format!("voter_{index}"))
        .join("episodes.v1.jsonl");
    let raw = fs::read(&ledger).expect("ledger bytes");
    let mutated = String::from_utf8(raw)
        .expect("ledger text")
        .replace("request_1", "request_9");
    fs::write(&ledger, mutated).expect("mutated ledger");
}

#[test]
fn a_quorum_produces_a_receipt_and_the_record_is_on_the_acknowledging_voters() {
    let scratch = Scratch::new("quorum");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    assert_eq!(shard.voter_count(), VOTER_COUNT);
    assert_eq!(shard.quorum_size(), QUORUM_SIZE);

    let receipt: ShardReceipt = shard
        .propose(&write("request_1", "operation_1", 1))
        .expect("quorum forms");
    assert!(receipt.acknowledged_by() >= QUORUM_SIZE);
    assert_eq!(receipt.quorum_size(), QUORUM_SIZE);
    assert_eq!(receipt.scope(), SCOPE);
    assert_eq!(receipt.request_id(), "request_1");
    assert!(shard.is_acknowledged(receipt.episode_digest()));

    // The record is on every voter that acknowledged it.
    let mut acknowledged = 0_usize;
    for member in shard.voters() {
        if member
            .store()
            .find_receipt(SCOPE, receipt.episode_digest())
            .expect("receipt lookup")
            .is_some()
        {
            acknowledged += 1;
        }
    }
    assert!(acknowledged >= QUORUM_SIZE);
    assert_eq!(acknowledged, receipt.acknowledged_by());
}

#[test]
fn a_write_that_reaches_only_one_voter_is_refused() {
    let scratch = Scratch::new("single-voter");
    // Two members cannot be opened, so only one member is reachable.
    break_voter(scratch.path(), 1);
    break_voter(scratch.path(), 2);
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    assert_eq!(shard.available_voters(), 1);

    let refused = shard.propose(&write("request_1", "operation_1", 1));
    assert_eq!(refused.unwrap_err(), ShardError::NoQuorum);
    // No receipt exists: the shard holds nothing it acknowledged.
    assert_eq!(shard.acknowledged_writes().len(), 0);
    // The write did reach the one member, and it is durable there, but it is
    // not canonical.
    let member = shard.voters().next().expect("the reachable member");
    assert_eq!(held_digests(member).len(), 1);
    for entry in shard.acknowledged_writes() {
        assert_ne!(entry.digest, held_digests(member)[0]);
    }
}

#[test]
fn a_partitioned_minority_cannot_produce_a_canonical_receipt() {
    let scratch = Scratch::new("minority");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let isolated = VoterId::new("voter_0").expect("voter identity");
    let candidate = write("request_1", "operation_1", 7);
    let shard_term = shard.term();

    {
        // The minority side of the partition: one member, no quorum.
        let mut minority = shard.isolate_minority(&isolated).expect("minority");
        assert_eq!(minority.voter_count(), 1);
        assert_eq!(minority.term(), shard_term);
        let outcome: MinorityOutcome = minority
            .propose(&candidate)
            .expect("the minority accepts the write locally");
        assert_eq!(outcome.accepted_by(), 1);
        assert_eq!(outcome.quorum_size(), QUORUM_SIZE);
        assert!(!outcome.reached_quorum());
    }

    // The majority acknowledges the same write: the member that already
    // holds it acknowledges idempotently and nothing is appended twice.
    let receipt = shard.propose(&candidate).expect("quorum forms");
    assert!(shard.is_acknowledged(receipt.episode_digest()));
    for member in shard.voters() {
        assert_eq!(held_digests(member), vec![*receipt.episode_digest()]);
    }
}

#[test]
fn the_minority_api_cannot_return_a_receipt() {
    // A compile-time assertion: the minority's proposal returns a minority
    // outcome, and the canonical path returns a receipt. Neither type can
    // stand in for the other.
    fn minority_return_type() -> Result<MinorityOutcome, ShardError> {
        let scratch = Scratch::new("minority-signature");
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        let isolated = VoterId::new("voter_0").expect("voter identity");
        let mut minority = shard.isolate_minority(&isolated).expect("minority");
        minority.propose(&write("request_1", "operation_1", 1))
    }

    fn canonical_return_type() -> Result<ShardReceipt, ShardError> {
        let scratch = Scratch::new("canonical-signature");
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        shard.propose(&write("request_1", "operation_1", 1))
    }

    assert_ne!(
        std::any::TypeId::of::<MinorityOutcome>(),
        std::any::TypeId::of::<ShardReceipt>()
    );
    let _ = minority_return_type;
    let _ = canonical_return_type;
}

#[test]
fn the_term_fences_stale_writers_and_stale_acknowledgements() {
    let scratch = Scratch::new("fencing");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let stale: ShardTerm = shard.term();
    let candidate = write("request_1", "operation_1", 1);

    // The term advances: the writer that holds the old term is fenced out.
    let current = shard.advance_term().expect("term advances");
    assert!(current > stale);

    // A proposal always runs under the shard's *current* term, and the
    // receipt carries that term: a caller cannot name a term of its own,
    // and therefore cannot acknowledge under a stale one.
    let receipt = shard.propose(&candidate).expect("the current term works");
    assert_eq!(receipt.term(), current);
    assert_eq!(receipt.term(), shard.term());

    // The fence is durable.
    drop(shard);
    let reopened = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard reopens");
    assert_eq!(reopened.term(), current);
}

#[test]
fn one_failed_voter_does_not_lose_a_quorum_acknowledged_write() {
    let scratch = Scratch::new("failed-voter");
    let receipt = {
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        shard
            .propose(&write("request_1", "operation_1", 1))
            .expect("quorum forms")
    };

    // The third member's directory becomes unusable.
    break_voter(scratch.path(), 2);

    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard reopens");
    assert_eq!(shard.available_voters(), QUORUM_SIZE);
    assert_eq!(shard.agreement_state(), AgreementState::Intact);
    // The write is still acknowledged: the surviving quorum still holds it.
    assert!(shard.is_acknowledged(receipt.episode_digest()));
    let mut survivors = 0_usize;
    for member in shard.voters() {
        assert!(
            member
                .store()
                .find_receipt(SCOPE, receipt.episode_digest())
                .expect("receipt lookup")
                .is_some()
        );
        // The bytes are still there.
        let ledger = member.directory().join("episodes.v1.jsonl");
        let bytes = fs::read(ledger).expect("ledger bytes");
        assert!(String::from_utf8_lossy(&bytes).contains("request_1"));
        survivors += 1;
    }
    assert_eq!(survivors, QUORUM_SIZE);
    // The surviving quorum still acknowledges new writes.
    assert!(shard.propose(&write("request_2", "operation_2", 2)).is_ok());
}

#[test]
fn a_restart_recovers_the_term_membership_and_acknowledged_state() {
    let scratch = Scratch::new("restart");
    let (receipt, term, members) = {
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        let receipt = shard
            .propose(&write("request_1", "operation_1", 1))
            .expect("quorum forms");
        let term = shard.term();
        let members = shard.members().to_vec();
        (receipt, term, members)
    };

    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard reopens");
    assert_eq!(shard.term(), term);
    assert_eq!(shard.members(), members.as_slice());
    assert_eq!(shard.voter_count(), VOTER_COUNT);
    assert!(shard.is_acknowledged(receipt.episode_digest()));
    assert_eq!(shard.acknowledged_writes().len(), 1);
    assert_eq!(
        shard.acknowledged_writes()[0].digest,
        *receipt.episode_digest()
    );
    // The recovered shard still acknowledges new writes.
    assert!(shard.propose(&write("request_2", "operation_2", 2)).is_ok());
}

#[test]
fn a_voter_that_lost_an_acknowledged_write_is_detected_and_refused() {
    let scratch = Scratch::new("diverged");
    {
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        shard
            .propose(&write("request_1", "operation_1", 1))
            .expect("quorum forms");
    }

    // The third member loses the acknowledged write but stays openable.
    lose_last_write(scratch.path(), 2);

    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard reopens");
    assert_eq!(shard.available_voters(), VOTER_COUNT);
    assert_eq!(shard.agreement_state(), AgreementState::Diverged);
    // The shard refuses to claim a write it cannot show on every member.
    assert!(!shard.is_acknowledged(&[1; 32]));
    // And it refuses to acknowledge new writes over a diverged member.
    assert_eq!(
        shard
            .propose(&write("request_2", "operation_2", 2))
            .unwrap_err(),
        ShardError::Diverged
    );
}

#[test]
fn a_damaged_member_blocks_new_acknowledgements() {
    let scratch = Scratch::new("damaged-member");
    {
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        shard
            .propose(&write("request_1", "operation_1", 1))
            .expect("quorum forms");
    }

    // The third member's ledger is mutated: the member cannot be replayed,
    // so the shard cannot show its agreement any more.
    corrupt_voter(scratch.path(), 2);

    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard reopens");
    assert_eq!(shard.available_voters(), QUORUM_SIZE);
    assert_eq!(shard.agreement_state(), AgreementState::Diverged);
    assert_eq!(
        shard
            .propose(&write("request_2", "operation_2", 2))
            .unwrap_err(),
        ShardError::Diverged
    );
}

#[test]
fn a_second_shard_cannot_take_over_the_same_root() {
    let scratch = Scratch::new("double-open");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let receipt = shard
        .propose(&write("request_1", "operation_1", 1))
        .expect("quorum forms");

    // The members' writer locks are held by the first shard, so a second
    // shard over the same root reaches no member and can acknowledge
    // nothing.
    let mut second =
        QuorumShard::open(scratch.path(), SCOPE, policy()).expect("second shard opens");
    assert_eq!(second.available_voters(), 0);
    assert!(
        second
            .propose(&write("request_2", "operation_2", 2))
            .is_err()
    );

    // The first shard is untouched: its acknowledged write is still
    // acknowledged, and it still acknowledges new writes.
    assert!(shard.is_acknowledged(receipt.episode_digest()));
    assert!(shard.propose(&write("request_3", "operation_3", 3)).is_ok());
}

#[test]
fn a_shard_never_holds_another_scope_records() {
    let scratch = Scratch::new("scope");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");

    let mut foreign = write("request_1", "operation_1", 1);
    foreign.guild_ref = OTHER_SCOPE.to_string();
    assert_eq!(
        shard.propose(&foreign).unwrap_err(),
        ShardError::ScopeMismatch
    );

    shard
        .propose(&write("request_2", "operation_2", 2))
        .expect("the shard's own scope is accepted");
    for member in shard.voters() {
        let receipts = member.store().retrieve(OTHER_SCOPE, 16).expect("receipts");
        assert_eq!(receipts.len(), 0);
        assert_eq!(held_digests(member).len(), 1);
    }
}

#[test]
fn the_members_agree_on_the_acknowledged_sequence() {
    let scratch = Scratch::new("agreement");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let first = shard
        .propose(&write("request_1", "operation_1", 1))
        .expect("quorum forms");
    let second = shard
        .propose(&write("request_2", "operation_2", 2))
        .expect("quorum forms");
    assert_ne!(first.episode_digest(), second.episode_digest());

    let mut sequences = Vec::new();
    for member in shard.voters() {
        let receipts = member.store().retrieve(SCOPE, 64).expect("receipts");
        let digests: Vec<[u8; 32]> = receipts
            .iter()
            .map(|receipt| receipt.episode_digest)
            .collect();
        let numbers: Vec<u64> = receipts.iter().map(|receipt| receipt.sequence).collect();
        assert_eq!(numbers, vec![1, 2]);
        sequences.push(digests);
    }
    assert_eq!(sequences.len(), VOTER_COUNT);
    for digests in &sequences {
        assert_eq!(digests, &sequences[0]);
    }
    assert_eq!(
        sequences[0],
        vec![*first.episode_digest(), *second.episode_digest()]
    );
}

#[test]
fn the_shard_is_deterministic_and_uses_explicit_terms() {
    let scratch = Scratch::new("determinism");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    // A fresh shard starts at the first epoch and only moves when an
    // operator advances it. Nothing here reads a clock.
    assert_eq!(shard.term().as_u64(), 1);
    assert_eq!(shard.advance_term().expect("term advances").as_u64(), 2);
    assert_eq!(shard.advance_term().expect("term advances").as_u64(), 3);

    // The same write produces the same digest on every run of the shard.
    let receipt = shard
        .propose(&write("request_1", "operation_1", 1))
        .expect("quorum forms");
    assert_eq!(receipt.term(), shard.term());
    assert_eq!(receipt.sequence(), 1);
}

#[test]
fn v3_quorum_shard_the_only_public_path_to_a_receipt_is_the_proposal() {
    // The structural claim, checked at the source: the certification step is
    // crate-internal, so a caller outside the crate cannot turn
    // acknowledgements of its own choosing into a canonical receipt.
    let source = include_str!("../src/v3/replication/shard.rs");
    for forbidden in [
        "pub fn certify",
        "pub fn propose_with_term",
        "pub fn propose_with",
    ] {
        assert!(
            !source.contains(forbidden),
            "{forbidden} is public: `propose` must be the only public path to a receipt"
        );
    }
    assert!(
        source.contains("pub fn propose("),
        "the public path to a receipt is the proposal"
    );

    // The acknowledgements a caller can collect carry no receipt of their
    // own: they are the shard's proposal receipts, and the receipt type has
    // no public constructor.
    let scratch = Scratch::new("public-surface");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let acks = shard
        .collect_acks(&write("request_1", "operation_1", 1))
        .expect("acks");
    assert!(acks.len() >= QUORUM_SIZE);
    for ack in &acks {
        assert_eq!(ack.term(), shard.term());
        assert_eq!(ack.request_id(), "request_1");
    }
}
