//! Direct assertions for frozen Studio acceptance rows S29, S32 and S33
//! (`docs/completion/studio-acceptance-v1.json`). Synthetic fixtures only.
//! These tests are requirement-level candidate evidence; they do not qualify
//! the requirements (all remain open/C0 in the execution register).
use serde_json::{Value, json};
use specimen_kernel::{engine, host::FixedHost, mutate, neural::Rng};
use std::sync::atomic::AtomicBool;

fn starter() -> Value {
    serde_json::from_str(include_str!("../../../conformance/starter.json")).unwrap()
}

/// First seed, from a spread sequence kept below 2^53 so it survives the
/// settings' f64 round trip, whose first coin flip lands on the requested side.
fn seed_with_first_flip(lands: bool) -> u64 {
    (1u64..)
        .map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 11)
        .find(|&seed| (Rng(seed).sample() < 0.5) == lands)
        .unwrap()
}

/// Two nodes sharing one patternId ("same-ID" peers), an attachment between
/// them, a resource linked to that patternId, and a cycle crediting only `a`.
fn same_pattern_pair(strength_a: u64, flip_lands: bool) -> Value {
    let mut s = starter();
    let mut a = s["nodes"][0].clone();
    let mut b = a.clone();
    a["ref"] = json!("a");
    a["strength"] = json!(strength_a);
    b["ref"] = json!("b");
    b["strength"] = json!(4);
    b["entries"][0]["id"] = json!("b-entry");
    b["entries"][0]["alternatives"][0]["id"] = json!("b-slot");
    let pid = a["patternId"].clone();
    s["nodes"] = json!([a, b]);
    s["attachments"] = json!([{"id":"att","from":"a","to":"b","mode":"sequence"}]);
    s["resources"][0]["resourceId"] = pid;
    s["settings"]["seed"] = json!(seed_with_first_flip(flip_lands));
    s["history"] = json!([{
        "id": "c1", "input": "hi", "createdAt": "2026-01-01T00:00:00.000Z",
        "segments": [{"id":"s1","text":"x","contributors":["a"],"sourceVotes":[],"transformations":[],"color":"green"}],
        "votes": [], "trace": [], "duration": 0, "feedback": [], "status": "complete", "pinned": false
    }]);
    s
}

fn refs(s: &Value) -> Vec<&str> {
    s["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["ref"].as_str().unwrap())
        .collect()
}

#[test]
fn s29_spent_same_pattern_node_is_removed_and_peers_remain() {
    let fixed = FixedHost::default();
    let s = same_pattern_pair(1, true);
    let (next, _) = mutate::feedback(&fixed.host(), &s, "c1", false, None).unwrap();
    assert_eq!(refs(&next), ["b"]);
    assert_eq!(next["nodes"][0], s["nodes"][1], "peer record is untouched");
    assert_eq!(
        next["attachments"],
        json!([]),
        "only the removed record's links go"
    );
    assert_eq!(
        next["resources"][0]["resourceId"], s["nodes"][1]["patternId"],
        "a surviving same-ID peer keeps resource links"
    );
}

#[test]
fn s29_removing_last_same_pattern_node_unlinks_resources() {
    let fixed = FixedHost::default();
    let mut s = same_pattern_pair(1, true);
    s["nodes"].as_array_mut().unwrap().truncate(1);
    s["attachments"] = json!([]);
    let (next, _) = mutate::feedback(&fixed.host(), &s, "c1", false, None).unwrap();
    assert!(refs(&next).is_empty());
    assert_eq!(next["resources"][0]["resourceId"], "");
}

#[test]
fn s29_unflipped_node_at_one_is_not_removed() {
    let fixed = FixedHost::default();
    let s = same_pattern_pair(1, false);
    let (next, _) = mutate::feedback(&fixed.host(), &s, "c1", false, None).unwrap();
    assert_eq!(next["nodes"], s["nodes"]);
    assert_eq!(next["attachments"], s["attachments"]);
}

const PATTERN: &str = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty";

fn alt(id: &str, action: &str) -> Value {
    json!({"id": id, "action": action, "inhibition": "", "weight": 1, "remixed": false})
}

/// Weak recipient and strong donor whose first alternatives admit a Markov
/// remix (fixture shape from specimen-core conformance), padded with inert
/// plain alternatives to the requested pool sizes.
fn pairing(weak_alts: usize, strong_alts: usize) -> Value {
    let mut s = starter();
    let mut weak = s["nodes"][0].clone();
    let mut donor = weak.clone();
    weak["ref"] = json!("weak");
    weak["strength"] = json!(3);
    weak["entries"][0]["id"] = json!("weak-entry");
    weak["entries"][0]["pattern"] = json!(PATTERN);
    let mut pool = vec![alt(
        "w0",
        "one two alpha beta one two gamma delta together in this place",
    )];
    pool.extend((1..weak_alts).map(|i| alt(&format!("w{i}"), &format!("plain weak reply {i}"))));
    weak["entries"][0]["alternatives"] = json!(pool);
    donor["ref"] = json!("donor");
    donor["strength"] = json!(8);
    donor["entries"][0]["id"] = json!("donor-entry");
    donor["entries"][0]["pattern"] = json!(format!("{PATTERN} extra"));
    let mut pool = vec![alt(
        "d0",
        "one two alpha epsilon one two gamma zeta together in this place",
    )];
    pool.extend(
        (1..strong_alts).map(|i| alt(&format!("d{i}"), &format!("plain strong reply {i}"))),
    );
    donor["entries"][0]["alternatives"] = json!(pool);
    s["nodes"] = json!([weak, donor]);
    s["attachments"] = json!([]);
    s
}

fn mutate_once(s: &Value) -> Value {
    let fixed = FixedHost::default();
    engine::maintain(&fixed.host(), s, "mutation", &AtomicBool::new(false)).unwrap()
}

fn alt_ids(s: &Value, node: usize) -> Vec<&str> {
    s["nodes"][node]["entries"][0]["alternatives"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect()
}

#[test]
fn s32_five_weak_with_four_strong_shortens_recipient_to_four() {
    let s = pairing(5, 4);
    let next = mutate_once(&s);
    assert_eq!(next["mutations"].as_array().unwrap().len(), 1);
    assert_eq!(alt_ids(&next, 0), ["w0", "w1", "w2", "w3"]);
    assert_eq!(
        next["nodes"][0]["entries"][0]["alternatives"][0]["remixed"],
        true
    );
    for i in 1..4 {
        assert_eq!(
            next["nodes"][0]["entries"][0]["alternatives"][i],
            s["nodes"][0]["entries"][0]["alternatives"][i],
            "surviving slots keep their history"
        );
    }
    assert_eq!(next["nodes"][1], s["nodes"][1], "donor is unchanged");
    engine::validate(&next).unwrap();
}

#[test]
fn s32_remixed_tail_blocks_shortening() {
    let mut s = pairing(5, 4);
    s["nodes"][0]["entries"][0]["alternatives"][4]["remixed"] = json!(true);
    let next = mutate_once(&s);
    assert_eq!(
        next["nodes"], s["nodes"],
        "a remixed slot is never discarded"
    );
    assert_eq!(next["mutations"], s["mutations"]);
    assert_eq!(next["events"][0]["title"], "Mutation scan completed");
}

#[test]
fn s33_four_weak_with_five_strong_keeps_four() {
    let s = pairing(4, 5);
    let next = mutate_once(&s);
    assert_eq!(next["mutations"].as_array().unwrap().len(), 1);
    assert_eq!(
        alt_ids(&next, 0),
        ["w0", "w1", "w2", "w3"],
        "no fifth alternative"
    );
    assert_eq!(alt_ids(&next, 1).len(), 5);
    assert_eq!(next["nodes"][1], s["nodes"][1], "donor is unchanged");
    engine::validate(&next).unwrap();
}
