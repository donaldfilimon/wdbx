use serde_json::{Value, json};
use specimen_core::persistence::Store;

fn starter() -> Value {
    serde_json::from_str(include_str!("../../../conformance/starter.json")).unwrap()
}

#[test]
fn empty_store_reports_no_snapshot_and_a_valid_audit_dag() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path().join("s")).unwrap();
    let info = serde_json::to_value(store.info().unwrap()).unwrap();
    assert_eq!(info["revision"], 0);
    assert_eq!(info["committedTransactions"], 0);
    assert_eq!(info["snapshotKey"], Value::Null);
    assert_eq!(info["auditDag"], json!({"ok": true, "error": null}));
    assert_eq!(info["records"]["nodes"], 0);
}

#[test]
fn committed_store_reports_heads_key_version_counts_and_files() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(temp.path().join("s")).unwrap();
    let specimen = starter();
    store.edit(0, specimen.clone()).unwrap();
    let mut fewer = specimen.clone();
    let removed = fewer["nodes"].as_array_mut().unwrap().remove(0);
    store.edit(1, fewer).unwrap();

    let info = serde_json::to_value(store.info().unwrap()).unwrap();
    assert_eq!(info["revision"], 2);
    assert_eq!(info["schema"], "wdbx.native.v2");
    assert_eq!(info["committedTransactions"], 2);
    assert_eq!(info["kvCount"], 1);
    // One writer, at sequence 2.
    let heads = info["heads"].as_object().unwrap();
    assert_eq!(heads.len(), 1);
    assert_eq!(heads.values().next().unwrap(), 2);
    assert_eq!(info["writerId"], *heads.keys().next().unwrap());
    let key = &info["snapshotKey"];
    assert_eq!(key["key"], "studio/snapshot");
    assert_eq!(key["sequence"], 2);
    assert_eq!(key["conflicts"], 0);
    assert!(key["bytes"].as_u64().unwrap() > 100);
    assert_eq!(
        info["records"]["nodes"],
        specimen["nodes"].as_array().unwrap().len() - 1
    );
    // The deleted node is kept as a tombstone and listed by name.
    assert_eq!(info["tombstones"][0]["name"], removed["name"]);
    assert_eq!(info["tombstones"][0]["ref"], removed["ref"]);
    assert!(info["tombstones"][0]["deletedAt"].is_string());
    // On-disk usage is measured, not estimated.
    assert!(info["disk"]["store"]["files"].as_u64().unwrap() > 0);
    assert!(info["disk"]["store"]["bytes"].as_u64().unwrap() > 0);
    assert_eq!(info["disk"]["assets"]["files"], 0);
}
