//! Term fencing.

use super::*;

#[test]
fn a_stale_writer_is_refused_after_the_term_advances() {
    let scratch = Scratch::new("fence");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let stale = shard.term();
    let next = shard.advance_term().expect("term advances");
    assert!(next > stale);
    let candidate = write("request_1", "operation_1", 1);
    assert_eq!(
        shard.propose_with_term(&candidate, stale).unwrap_err(),
        ShardError::StaleTerm
    );
    // The current term still works.
    assert!(shard.propose_with_term(&candidate, next).is_ok());
}

#[test]
fn a_stale_acknowledgement_is_refused_by_the_shard() {
    let scratch = Scratch::new("stale-ack");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let candidate = write("request_1", "operation_1", 1);
    let acks = shard.collect_acks(&candidate).expect("acks");
    assert!(acks.len() >= QUORUM_SIZE);
    shard.advance_term().expect("term advances");
    assert_eq!(shard.certify(&acks).unwrap_err(), ShardError::StaleTerm);
}

#[test]
fn a_voter_refuses_an_acknowledgement_under_an_older_term() {
    let scratch = Scratch::new("voter-fence");
    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
    let stale = shard.term();
    shard.advance_term().expect("term advances");
    let member_id = VoterId::new("voter_0").expect("voter identity");
    let member = shard.voter_mut(&member_id).expect("a member");
    let candidate = write("request_1", "operation_1", 1);
    assert_eq!(
        member.accept(&candidate, stale).unwrap_err(),
        ShardError::StaleTerm
    );
}

#[test]
fn a_term_survives_a_restart_and_only_moves_forward() {
    let scratch = Scratch::new("term-restart");
    let advanced;
    {
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        let first = shard.term();
        advanced = shard.advance_term().expect("term advances");
        assert!(advanced > first);
    }
    let mut reopened = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard reopens");
    assert_eq!(reopened.term(), advanced);
    assert!(reopened.advance_term().expect("term advances") > advanced);
}
