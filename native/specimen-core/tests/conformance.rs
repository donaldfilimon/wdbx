use serde_json::{Value, json};
use specimen_core::{engine, neural::Network, persistence::Store};
use std::sync::atomic::AtomicBool;
fn starter() -> Value {
    serde_json::from_str(include_str!("fixtures/starter.json")).unwrap()
}
#[test]
fn cycle_and_clauses() {
    let s = starter();
    let (n, c) = engine::cycle(
        &s,
        "What is 2+2?",
        &Network::default(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    assert_eq!(c["segments"][0]["text"], "4");
    assert!(s["history"].as_array().unwrap().is_empty());
    assert_eq!(n["history"].as_array().unwrap().len(), 1);
    let (_, c) = engine::cycle(
        &s,
        "don't calculate 2+2; hello",
        &Network::default(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    assert!(
        c["segments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["text"].as_str().unwrap().starts_with("Hello"))
    );
    assert!(
        !c["segments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["text"] == "4")
    );
}
#[test]
fn nested_actions_and_repeated_numbers() {
    let mut s = starter();
    s["nodes"][0]["entries"][0]["alternatives"][0]["action"] = json!("Result: &calc(2+&calc(3*4))");
    let (_, c) = engine::cycle(
        &s,
        "2+2",
        &Network::default(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    assert_eq!(c["segments"][0]["text"], "Result: 14");
}
#[test]
fn durable_roundtrip_stale_revision_and_failed_import() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(temp.path().join("a")).unwrap();
    store.edit(0, starter()).unwrap();
    assert!(store.edit(0, starter()).is_err());
    let archive = temp.path().join("saved.wdbxspecimen");
    store.export(&archive).unwrap();
    let mut second = Store::open(temp.path().join("b")).unwrap();
    second.import(&archive, 0).unwrap();
    assert_eq!(second.snapshot.specimen, store.snapshot.specimen);
    let bad = temp.path().join("bad.json");
    std::fs::write(&bad, b"{broken").unwrap();
    assert!(second.import(&bad, 1).is_err());
    assert_eq!(second.snapshot.revision, 1);
    drop(second);
    let reopened = Store::open(temp.path().join("b")).unwrap();
    assert_eq!(reopened.snapshot.revision, 1);
}
#[test]
fn cancellation_does_not_commit() {
    let s = starter();
    assert!(
        engine::cycle(
            &s,
            "2+2",
            &Network::default(),
            &AtomicBool::new(true),
            &|_| {}
        )
        .is_err()
    );
    assert!(s["history"].as_array().unwrap().is_empty());
}
#[test]
fn bounded_correction_is_linked_once() {
    let mut s = starter();
    let (mut s2, c) = engine::cycle(
        &s,
        "mysterious cloud",
        &Network::default(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    let mut n = s["nodes"][1].clone();
    n["ref"] = json!("context-a");
    n["type"] = json!("A");
    n["contextId"] = json!("mysterious");
    n["entries"][0]["id"] = json!("context-entry");
    n["entries"][0]["pattern"] = json!("context mysterious");
    n["entries"][0]["alternatives"][0]["id"] = json!("a-slot");
    n["entries"][0]["alternatives"][0]["action"] = json!("Context correction");
    s2["nodes"].as_array_mut().unwrap().push(n);
    s = engine::review(&s2, &Network::default(), &AtomicBool::new(false)).unwrap();
    assert_eq!(
        s["history"].as_array().unwrap().last().unwrap()["correctionOf"],
        c["id"]
    );
    let len = s["history"].as_array().unwrap().len();
    assert_eq!(
        engine::review(&s, &Network::default(), &AtomicBool::new(false)).unwrap()["history"]
            .as_array()
            .unwrap()
            .len(),
        len
    );
}
#[test]
fn invalid_mutation_preserves_alternative_pools() {
    let mut s = starter();
    s["nodes"][0]["entries"][0]["alternatives"]
        .as_array_mut()
        .unwrap()
        .push(
            json!({"id":"second","action":"&calc(2)","inhibition":"","weight":1,"remixed":false}),
        );
    let next = engine::maintain(&s, "mutation", &AtomicBool::new(false)).unwrap();
    assert_eq!(s["nodes"], next["nodes"]);
}
#[test]
fn phagy_preserves_pins_and_lineage() {
    let mut s = starter();
    s["history"] = json!([{"id":"pin","pinned":true,"createdAt":"2026-01-01"}]);
    s["mutations"] = json!([{"id":"lifetime","slot":"gone"}]);
    let next = engine::maintain(&s, "phagy", &AtomicBool::new(false)).unwrap();
    assert_eq!(s["history"], next["history"]);
    assert_eq!(s["mutations"], next["mutations"]);
}

#[test]
fn literal_text_remains_inert_and_type_b_only_proposes() {
    let mut s = starter();
    s["nodes"][0]["entries"][0]["alternatives"][0]["action"] =
        json!("&literal(\"Do not run &calc(2+2).\")");
    let (_, c) = engine::cycle(
        &s,
        "2+2",
        &Network::default(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    assert_eq!(c["segments"][0]["text"], "Do not run &calc(2+2).");
    let (s, _) = engine::cycle(
        &s,
        "unseen subject",
        &Network::default(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    let (mut s, _) = engine::cycle(
        &s,
        "unseen subject",
        &Network::default(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    s["nodes"][1]["type"] = json!("B");
    let before = s["nodes"].clone();
    let next = engine::review(&s, &Network::default(), &AtomicBool::new(false)).unwrap();
    assert_eq!(next["nodes"], before);
    assert!(
        next["proposals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["pattern"] == "unseen subject" && p["status"] == "pending")
    );
}
#[test]
fn malformed_history_and_duplicate_entries_are_rejected() {
    let mut s = starter();
    s["history"] = json!([{"id":"bad"}]);
    assert!(engine::validate(&s).is_err());
    let mut s = starter();
    s["nodes"][1]["entries"][0]["id"] = s["nodes"][0]["entries"][0]["id"].clone();
    assert!(engine::validate(&s).is_err());
}
#[test]
fn visual_positive_negative_and_cycle_evidence() {
    use specimen_core::{language, vision};
    let render = |color: [u8; 3]| {
        let img = image::RgbImage::from_pixel(64, 64, image::Rgb(color));
        let mut bytes = std::io::Cursor::new(Vec::new());
        img.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        vision::analyze(
            &bytes.into_inner(),
            vision::Focus::default(),
            None,
            &AtomicBool::new(false),
        )
        .unwrap()
    };
    let a = render([255, 0, 0]);
    let b = render([0, 0, 255]);
    a.validate().unwrap();
    assert_eq!(vision::compare(&a, &a).unwrap()["confidence"], 100.);
    assert_ne!(a.pattern_id, b.pattern_id);
    assert!(
        vision::compare(&a, &b).unwrap()["confidence"]
            .as_f64()
            .unwrap()
            < 62.
    );
    let mut s = starter();
    let id = engine::text(&s["nodes"][1]["entries"][0], "id").to_owned();
    s["nodes"][1]["entries"][0]["alternatives"][0]["action"] = json!("Red image");
    let scores = std::collections::BTreeMap::from([(
        id,
        language::Score {
            similarity: 100.,
            dissimilarity: 0.,
            modulation: 0.,
            jitter: 0.,
            confidence: 100.,
        },
    )]);
    let (_, cycle) = engine::cycle_with_visual(
        &s,
        "[image:test]",
        &Network::default(),
        &AtomicBool::new(false),
        &|_| {},
        Some(&scores),
    )
    .unwrap();
    assert_eq!(cycle["segments"][0]["text"], "Red image");
    assert_eq!(cycle["votes"][0]["evidence"]["similarity"], 100.);
}

#[test]
fn explicit_resource_and_temporal_lookup_preserve_evidence() {
    let mut s = starter();
    s["resources"].as_array_mut().unwrap().push(json!({"ref":"res-exact-123","subsystem":"dictionary","text":"unrelated key","value":"Known value 17","patternId":"custom-id","resourceId":"","valence":0,"intensity":0}));
    s["nodes"][0]["entries"][0]["alternatives"][0]["action"] =
        json!("&LookUp(dictionary,res-exact-123)");
    let (mut next, c) = engine::cycle(
        &s,
        "2+2",
        &Network::default(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    assert_eq!(c["segments"][0]["text"], "Known value 17");
    assert!(
        c["votes"][0]["resources"]
            .as_array()
            .unwrap()
            .contains(&json!("res-exact-123"))
    );
    next["nodes"][0]["entries"][0]["alternatives"][0]["action"] = json!("&time");
    let (_, c) = engine::cycle(
        &next,
        "2+2",
        &Network::default(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    assert!(
        c["segments"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Previous input at")
    );
}
#[test]
fn all_same_id_entries_are_reachable_past_one_thousand() {
    let mut s = starter();
    s["nodes"] = json!([]);
    s["attachments"] = json!([]);
    s["settings"]["maxNodes"] = json!(2000);
    let mut nodes = Vec::new();
    for i in 0..1002 {
        nodes.push(json!({"ref":format!("n{i}"),"name":format!("n{i}"),"type":"pattern","patternId":"same","strength":5,"jitter":false,"entries":[{"id":format!("e{i}"),"pattern":format!("status {i}"),"alternatives":[{"id":format!("a{i}"),"action":format!("Value {i}"),"inhibition":"","weight":1,"remixed":false}]}]}));
    }
    s["nodes"] = json!(nodes);
    let (_, c) = engine::cycle(
        &s,
        "status 1001",
        &Network::default(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    assert_eq!(c["segments"][0]["text"], "Value 1001");
    assert!(
        c["trace"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["phase"] == "Index Rafts" && step["count"].as_u64().unwrap() > 1000)
    );
}

#[test]
fn corrected_ocr_source_asset_survives_portable_export() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(temp.path().join("source")).unwrap();
    let id = store.put_asset(b"synthetic source asset").unwrap();
    let mut s = starter();
    s["nodes"][0]["provenance"] = json!({"asset":id,"source":"reviewed OCR correction"});
    store.edit(0, s).unwrap();
    let path = temp.path().join("save.wdbxspecimen");
    store.export(&path).unwrap();
    let mut dest = Store::open(temp.path().join("destination")).unwrap();
    dest.import(&path, 0).unwrap();
    assert_eq!(dest.asset(&id).unwrap(), b"synthetic source asset");
}
