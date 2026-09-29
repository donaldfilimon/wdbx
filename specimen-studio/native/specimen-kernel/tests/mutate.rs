use serde_json::{Value, json};
use specimen_kernel::{
    engine,
    host::FixedHost,
    language,
    mutate::{self, NodeInput, ResourceInput},
};

fn seeded() -> (FixedHost, Value) {
    let fixed = FixedHost::default();
    let s = mutate::seed(&fixed.host());
    (fixed, s)
}

fn node(name: &str, pattern: &str, action: &str) -> NodeInput {
    NodeInput {
        name: name.into(),
        pattern: pattern.into(),
        action: action.into(),
        ..Default::default()
    }
}

#[test]
fn seed_is_a_valid_native_specimen() {
    let (_, s) = seeded();
    engine::validate(&s).unwrap();
    assert_eq!(s["schema"], "wdbx.studio.v1");
    assert_eq!(s["nodes"].as_array().unwrap().len(), 8);
    assert_eq!(s["resources"].as_array().unwrap().len(), 12);
    assert_eq!(s["nodes"][0]["strength"], 6);
    assert_eq!(s["nodes"][1]["tone"], "warm");
    for n in s["nodes"].as_array().unwrap() {
        let pattern = n["entries"][0]["pattern"].as_str().unwrap();
        assert_eq!(n["patternId"], language::identify(pattern).id);
    }
    // Dictionary memories link to the Recall node's native Pattern ID.
    let recall = s["nodes"][4]["patternId"].clone();
    assert!(
        s["resources"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["subsystem"] == "dictionary")
            .all(|r| r["resourceId"] == recall)
    );
    assert_eq!(s["events"].as_array().unwrap().len(), 1);
    assert_eq!(s["events"][0]["title"], "Starter specimen created");
}

#[test]
fn seed_is_deterministic_under_a_fixed_host() {
    assert_eq!(seeded().1, seeded().1);
}

#[test]
fn add_node_validates_and_logs() {
    let (fixed, s) = seeded();
    let h = fixed.host();
    let err = mutate::add_node(&h, &s, &node("", "x", "y"), None).unwrap_err();
    assert_eq!(err.message, "Enter a name, pattern, and action.");
    let err = mutate::add_node(&h, &s, &node("Dup", "hello", "Hi"), None).unwrap_err();
    assert_eq!(
        err.message,
        "That exact pattern is already stored. Use a distinct pattern or edit its node."
    );
    let next = mutate::add_node(&h, &s, &node("Color", "favorite color", "Teal."), None).unwrap();
    let added = next["nodes"].as_array().unwrap().last().unwrap();
    assert_eq!(added["name"], "Color");
    assert_eq!(added["patternId"], language::identify("favorite color").id);
    assert_eq!(added["strength"], s["settings"]["initialStrength"]);
    assert_eq!(next["events"][0]["title"], "Pattern learned");
    engine::validate(&next).unwrap();
}

#[test]
fn add_node_edit_keeps_ref_strength_and_sibling_entries() {
    let (fixed, s) = seeded();
    let h = fixed.host();
    let calc = s["nodes"][0]["ref"].as_str().unwrap().to_owned();
    let next = mutate::add_node(
        &h,
        &s,
        &node("Calculate", "&n &op &n", "&calc(&current_input)!"),
        Some(&calc),
    )
    .unwrap();
    let edited = &next["nodes"][0];
    assert_eq!(edited["ref"], calc);
    assert_eq!(edited["strength"], 6);
    assert_eq!(
        edited["entries"][0]["alternatives"][0]["action"],
        "&calc(&current_input)!"
    );
    assert_eq!(next["events"][0]["title"], "Node updated");
}

#[test]
fn add_node_rejects_inhibited_pairings_and_the_node_limit() {
    let (fixed, mut s) = seeded();
    let h = fixed.host();
    s["mutations"] = json!([{"originalPattern": "new thing", "originalAction": "act"}]);
    let err = mutate::add_node(&h, &s, &node("N", "new thing", "act"), None).unwrap_err();
    assert_eq!(
        err.message,
        "This original pairing is inhibited by a previous mutation."
    );
    s["mutations"] = json!([]);
    s["settings"]["maxNodes"] = json!(8);
    let err = mutate::add_node(&h, &s, &node("N", "new thing", "act"), None).unwrap_err();
    assert_eq!(
        err.message,
        "The node limit is reached. Adjust the limit in Settings."
    );
}

#[test]
fn add_entry_requires_the_same_pattern_id() {
    let (fixed, s) = seeded();
    let h = fixed.host();
    let calc = s["nodes"][0]["ref"].as_str().unwrap().to_owned();
    let next = mutate::add_entry(&h, &s, &calc, "9 * 9", "&calc(&current_input)").unwrap();
    assert_eq!(next["nodes"][0]["entries"].as_array().unwrap().len(), 2);
    let err = mutate::add_entry(&h, &s, &calc, "tell me a story", "x").unwrap_err();
    assert_eq!(
        err.message,
        "The entry must produce the same Pattern ID as its node."
    );
}

#[test]
fn remove_node_drops_attachments_and_orphaned_links() {
    let (fixed, mut s) = seeded();
    let h = fixed.host();
    let a = s["nodes"][0]["ref"].clone();
    let b = s["nodes"][1]["ref"].clone();
    s["attachments"] =
        json!([{"id":"x","from":a,"to":b,"bidirectional":false,"hard":false,"affinity":0.5}]);
    let recall = s["nodes"][4]["ref"].as_str().unwrap().to_owned();
    let next = mutate::remove_node(&h, &s, a.as_str().unwrap());
    assert_eq!(next["nodes"].as_array().unwrap().len(), 7);
    assert!(next["attachments"].as_array().unwrap().is_empty());
    let next = mutate::remove_node(&h, &next, &recall);
    assert!(
        next["resources"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["resourceId"] == "")
    );
    assert_eq!(next["events"][0]["title"], "Node removed");
}

#[test]
fn save_resource_validates_and_sets_native_ids() {
    let (fixed, s) = seeded();
    let h = fixed.host();
    let input = ResourceInput {
        subsystem: "dictionary".into(),
        text: "teal".into(),
        value: "A blue-green color.".into(),
        resource_id: String::new(),
        valence: 0.1,
        intensity: 0.2,
    };
    let next = mutate::save_resource(&h, &s, &input, None).unwrap();
    let saved = next["resources"].as_array().unwrap().last().unwrap();
    assert_eq!(saved["patternId"], language::identify("teal").id);
    assert_eq!(saved["subsystem"], "dictionary");
    let bad = ResourceInput {
        valence: 2.0,
        ..input.clone()
    };
    assert_eq!(
        mutate::save_resource(&h, &s, &bad, None)
            .unwrap_err()
            .message,
        "Valence must be −1 to 1 and intensity 0 to 1."
    );
    let bad = ResourceInput {
        subsystem: "nope".into(),
        ..input
    };
    assert_eq!(
        mutate::save_resource(&h, &s, &bad, None)
            .unwrap_err()
            .message,
        "Choose a registered subsystem."
    );
}

fn with_cycle(s: &mut Value, contributors: Value) {
    s["history"] = json!([{
        "id": "c1", "input": "hi", "createdAt": "2026-01-01T00:00:00.000Z",
        "segments": [{"id":"s1","text":"x","contributors":contributors,"sourceVotes":[],"transformations":[],"color":"green"}],
        "votes": [], "trace": [], "duration": 0, "feedback": [], "status": "complete", "pinned": false
    }]);
}

#[test]
fn feedback_is_deterministic_applies_once_and_advances_the_seed() {
    let (fixed, mut s) = seeded();
    let h = fixed.host();
    let refs: Vec<Value> = s["nodes"].as_array().unwrap()[..4]
        .iter()
        .map(|n| n["ref"].clone())
        .collect();
    with_cycle(&mut s, json!(refs));
    let (a, msg_a) = mutate::feedback(&h, &s, "c1", true, None).unwrap();
    let (b, msg_b) = mutate::feedback(&h, &s, "c1", true, None).unwrap();
    assert_eq!(a["nodes"], b["nodes"]);
    assert_eq!(msg_a, msg_b);
    assert!(msg_a.ends_with("of 4 contributors changed strength after independent coin flips."));
    assert_ne!(a["settings"]["seed"], s["settings"]["seed"]);
    assert_eq!(a["history"][0]["feedback"], json!(["all"]));
    assert_eq!(
        mutate::feedback(&h, &a, "c1", false, Some("green"))
            .unwrap_err()
            .message,
        "Feedback was already applied to this answer or segment."
    );
    assert_eq!(a["events"][0]["title"], "Positive feedback");
}

#[test]
fn feedback_removes_nodes_that_reach_zero() {
    let (fixed, mut s) = seeded();
    let h = fixed.host();
    for n in s["nodes"].as_array_mut().unwrap() {
        n["strength"] = json!(1);
    }
    let refs: Vec<Value> = s["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["ref"].clone())
        .collect();
    with_cycle(&mut s, json!(refs));
    let (next, _) = mutate::feedback(&h, &s, "c1", false, None).unwrap();
    let left = next["nodes"].as_array().unwrap();
    assert!(left.len() < 8, "some coin flips must land");
    assert!(left.iter().all(|n| n["strength"].as_f64().unwrap() > 0.));
}

#[test]
fn toggle_pin_respects_the_pin_limit() {
    let (fixed, mut s) = seeded();
    let h = fixed.host();
    with_cycle(&mut s, json!([]));
    let pinned = mutate::toggle_pin(&h, &s, "c1").unwrap();
    assert_eq!(pinned["history"][0]["pinned"], true);
    assert_eq!(pinned["events"][0]["title"], "Conversation pinned");
    let unpinned = mutate::toggle_pin(&h, &pinned, "c1").unwrap();
    assert_eq!(unpinned["history"][0]["pinned"], false);
    s["settings"]["pinLimit"] = json!(1);
    s["history"][0]["pinned"] = json!(false);
    let mut other = s["history"][0].clone();
    other["id"] = json!("c0");
    other["pinned"] = json!(true);
    s["history"].as_array_mut().unwrap().insert(0, other);
    assert_eq!(
        mutate::toggle_pin(&h, &s, "c1").unwrap_err().message,
        "The pin limit is reached. Unpin a record first."
    );
}

#[test]
fn settings_use_native_bounds_and_types() {
    let (_, s) = seeded();
    let mut settings = s["settings"].clone();
    mutate::validate_settings(&settings).unwrap();
    settings["jitter"] = json!(50);
    mutate::validate_settings(&settings).unwrap();
    settings["jitter"] = json!(101);
    assert_eq!(
        mutate::validate_settings(&settings).unwrap_err().message,
        "Set jitter between 0 and 100."
    );
    settings["jitter"] = json!(1.5);
    settings["entryLimit"] = json!(2.5);
    assert_eq!(
        mutate::validate_settings(&settings).unwrap_err().message,
        "entryLimit must be a whole number."
    );
    settings["entryLimit"] = json!(20);
    settings["brainstorm"] = json!("yes");
    assert_eq!(
        mutate::validate_settings(&settings).unwrap_err().message,
        "brainstorm must be on or off."
    );
}

#[test]
fn migrate_ids_recomputes_patterns_and_remaps_links() {
    let (fixed, mut s) = seeded();
    let h = fixed.host();
    s["nodes"][4]["patternId"] = json!("low:recall");
    for r in s["resources"].as_array_mut().unwrap() {
        r["patternId"] = json!("low:old");
        if r["subsystem"] == "dictionary" {
            r["resourceId"] = json!("low:recall");
        }
    }
    let next = mutate::migrate_ids(&h, &s);
    assert_eq!(
        next["nodes"][4]["patternId"],
        seeded().1["nodes"][4]["patternId"]
    );
    assert!(
        next["resources"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["patternId"] != "low:old")
    );
    assert!(
        next["resources"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["subsystem"] == "dictionary")
            .all(|r| r["resourceId"] == next["nodes"][4]["patternId"])
    );
    engine::validate(&next).unwrap();
}
