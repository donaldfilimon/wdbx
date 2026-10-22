//! Quorum formation and minority refusal.

use super::*;

#[test]
fn a_quorum_of_two_produces_a_receipt_on_both_members() {
    let scratch = Scratch::new("quorum");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let receipt = shard
        .propose(&write("request_1", "operation_1", 1))
        .expect("quorum");
    assert!(receipt.acknowledged_by() >= QUORUM_SIZE);
    assert_eq!(receipt.quorum_size(), QUORUM_SIZE);
    assert!(receipt.members().len() >= QUORUM_SIZE);
    assert_eq!(receipt.scope(), SCOPE);
    assert!(shard.is_acknowledged(receipt.episode_digest()));
    for member in shard.voters() {
        assert!(member.holds(SCOPE, receipt.episode_digest()));
    }
}

#[test]
fn a_minority_accepts_locally_and_never_acknowledges() {
    let scratch = Scratch::new("minority");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let isolated = VoterId::new("voter_0").expect("voter identity");
    let candidate = write("request_1", "operation_1", 7);
    {
        let mut minority = shard.isolate_minority(&isolated).expect("minority");
        let outcome = minority
            .propose(&candidate)
            .expect("the minority accepts locally");
        assert_eq!(outcome.accepted_by(), 1);
        assert_eq!(outcome.quorum_size(), QUORUM_SIZE);
        assert!(!outcome.reached_quorum());
    }
    // The majority can still acknowledge the same write: the member that
    // already holds it acknowledges idempotently, and nothing is appended
    // twice.
    let receipt = shard.propose(&candidate).expect("quorum");
    assert_eq!(receipt.acknowledged_by(), VOTER_COUNT);
    for member in shard.voters() {
        let receipts = member.store().retrieve(SCOPE, 16).expect("receipts");
        assert_eq!(receipts.len(), 1);
    }
}

#[test]
fn the_minority_api_cannot_return_a_receipt() {
    // A compile-time assertion: the minority's proposal returns a minority
    // outcome, and no receipt type appears in its signature.
    fn minority_return_type() -> Result<MinorityOutcome, ShardError> {
        let scratch = Scratch::new("signature");
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        let isolated = VoterId::new("voter_0").expect("voter identity");
        let mut minority = shard.isolate_minority(&isolated).expect("minority");
        minority.propose(&write("request_1", "operation_1", 1))
    }

    fn canonical_return_type() -> Result<ShardReceipt, ShardError> {
        let scratch = Scratch::new("signature-canonical");
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        shard.propose(&write("request_1", "operation_1", 1))
    }

    // The two return types are distinct.
    assert_ne!(
        std::any::TypeId::of::<MinorityOutcome>(),
        std::any::TypeId::of::<ShardReceipt>()
    );
    // The functions above only compile because their declared return types
    // are the ones the API actually produces.
    let _ = minority_return_type;
    let _ = canonical_return_type;
}

#[test]
fn a_write_for_another_scope_is_refused() {
    let scratch = Scratch::new("scope");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let mut foreign = write("request_1", "operation_1", 3);
    foreign.guild_ref = "other_guild".into();
    assert_eq!(
        shard.propose(&foreign).unwrap_err(),
        ShardError::ScopeMismatch
    );
    for member in shard.voters() {
        let receipts = member
            .store()
            .retrieve("other_guild", 16)
            .expect("receipts");
        assert_eq!(receipts.len(), 0);
    }
}

#[test]
fn acknowledgements_of_another_proposal_are_refused() {
    let scratch = Scratch::new("foreign-acks");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let first = write("request_1", "operation_1", 1);
    let second = write("request_2", "operation_2", 2);
    let stale = shard.collect_acks(&first).expect("acks");
    let _current = shard.collect_acks(&second).expect("acks");
    // The shard certifies only the proposal it last made.
    assert_eq!(shard.certify(&stale).unwrap_err(), ShardError::Conflict);
}

#[test]
fn a_proposal_the_shard_never_made_cannot_be_certified() {
    let scratch = Scratch::new("no-proposal");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    // The shard has made no proposal, so it has nothing to certify: the
    // certification step always requires a pending proposal the shard
    // itself made.
    assert_eq!(shard.certify(&[]).unwrap_err(), ShardError::InvalidInput);
    // And a proposal it has made is the only thing it can certify.
    let acks = shard
        .collect_acks(&write("request_1", "operation_1", 1))
        .expect("acks");
    assert!(acks.len() >= QUORUM_SIZE);
    let receipt = shard
        .certify(&acks)
        .expect("the shard's own proposal certifies");
    assert_eq!(receipt.request_id(), "request_1");
}

#[test]
fn the_members_agree_on_the_acknowledged_sequence() {
    let scratch = Scratch::new("agreement");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let first = shard
        .propose(&write("request_1", "operation_1", 1))
        .expect("quorum");
    let second = shard
        .propose(&write("request_2", "operation_2", 2))
        .expect("quorum");
    let mut digests = Vec::new();
    for member in shard.voters() {
        let receipts = member.store().retrieve(SCOPE, 16).expect("receipts");
        let member_digests: Vec<[u8; 32]> =
            receipts.into_iter().map(|r| r.episode_digest).collect();
        digests.push(member_digests);
    }
    assert_eq!(digests.len(), VOTER_COUNT);
    for member_digests in &digests {
        assert_eq!(member_digests, &digests[0]);
    }
    assert_eq!(
        digests[0],
        vec![*first.episode_digest(), *second.episode_digest()]
    );
}
