//! Restart recovery, a failed member, and divergence detection.

use super::*;

#[test]
fn a_restart_recovers_the_term_membership_and_acknowledged_writes() {
    let scratch = Scratch::new("restart");
    let receipt;
    {
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        receipt = shard
            .propose(&write("request_1", "operation_1", 1))
            .expect("quorum");
        let term = shard.term();
        let members = shard.members().to_vec();
        drop(shard);
        let mut reopened =
            QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard reopens");
        assert_eq!(reopened.term(), term);
        assert_eq!(reopened.members(), members.as_slice());
        assert!(reopened.is_acknowledged(receipt.episode_digest()));
        assert_eq!(reopened.acknowledged_writes().len(), 1);
        // The recovered shard still acknowledges new writes.
        assert!(
            reopened
                .propose(&write("request_2", "operation_2", 2))
                .is_ok()
        );
    }
}

#[test]
fn a_failed_member_does_not_lose_an_acknowledged_write() {
    let scratch = Scratch::new("failed-member");
    let receipt;
    {
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        receipt = shard
            .propose(&write("request_1", "operation_1", 1))
            .expect("quorum");
    }
    // The member's directory becomes a regular file: the member cannot be
    // opened, exactly as a failed disk would look.
    let failed = scratch.path().join("voter_2");
    let _ = fs::remove_dir_all(&failed);
    fs::write(&failed, b"not a directory").expect("failed member");

    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard reopens");
    assert_eq!(shard.available_voters(), QUORUM_SIZE);
    assert_eq!(shard.agreement_state(), AgreementState::Intact);
    assert!(shard.is_acknowledged(receipt.episode_digest()));
    // The surviving members still hold the write and still form a quorum.
    for member in shard.voters() {
        assert!(member.holds(SCOPE, receipt.episode_digest()));
    }
    assert!(shard.propose(&write("request_2", "operation_2", 2)).is_ok());
}

#[test]
fn a_member_that_lost_an_acknowledged_write_is_detected() {
    let scratch = Scratch::new("diverged");
    {
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        shard
            .propose(&write("request_1", "operation_1", 1))
            .expect("quorum");
    }
    // The member's ledger loses its last line: the member is openable but no
    // longer holds the acknowledged write.
    let ledger = scratch.path().join("voter_2").join("episodes.v1.jsonl");
    let raw = fs::read(&ledger).expect("ledger bytes");
    let mut ends = Vec::new();
    for (index, byte) in raw.iter().enumerate() {
        if *byte == b'\n' {
            ends.push(index + 1);
        }
    }
    // Keep every line but the last one.
    let cut = if ends.len() > 1 {
        ends[ends.len() - 2]
    } else {
        0
    };
    fs::write(&ledger, &raw[..cut]).expect("truncated ledger");

    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard reopens");
    assert_eq!(shard.agreement_state(), AgreementState::Diverged);
    assert!(!shard.is_acknowledged(&[1; 32]));
    assert_eq!(
        shard
            .propose(&write("request_2", "operation_2", 2))
            .unwrap_err(),
        ShardError::Diverged
    );
}

#[test]
fn a_damaged_member_is_not_treated_as_an_unreachable_one() {
    let scratch = Scratch::new("damaged-member");
    {
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        shard
            .propose(&write("request_1", "operation_1", 1))
            .expect("quorum");
    }
    let ledger = scratch.path().join("voter_2").join("episodes.v1.jsonl");
    let raw = fs::read(&ledger).expect("ledger bytes");
    let mutated = String::from_utf8(raw)
        .expect("ledger text")
        .replace("request_1", "request_9");
    fs::write(&ledger, mutated).expect("mutated ledger");

    let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard reopens");
    // The member cannot be replayed, so it is not available; and the shard
    // cannot show its agreement, so it refuses to acknowledge.
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
fn a_corrupt_shard_state_file_is_refused() {
    let scratch = Scratch::new("corrupt-state");
    {
        let mut shard = QuorumShard::open(scratch.path(), SCOPE, policy()).expect("shard opens");
        shard
            .propose(&write("request_1", "operation_1", 1))
            .expect("quorum");
    }
    let state = scratch.path().join("shard.v1.json");
    let mut raw = fs::read(&state).expect("state bytes");
    let index = raw.len() / 2;
    raw[index] = if raw[index] == b'1' { b'2' } else { b'1' };
    fs::write(&state, &raw).expect("mutated state");
    assert_eq!(
        QuorumShard::open(scratch.path(), SCOPE, policy()).unwrap_err(),
        ShardError::Corrupt
    );
}
