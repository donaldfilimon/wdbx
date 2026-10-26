#![cfg(feature = "identity-probe")]

use serde_json::{Value, json};

#[test]
fn fixture_executes_successful_identity_mechanisms() {
    let text = specimen_wasm::identity_probe::run();
    let out: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(out["schema"], "task8-identity-mechanism-probe-v1");
    assert_eq!(out["capacity"]["full"], "Full");
    assert_eq!(out["capacity"]["full_unchanged"], true);
    assert_eq!(out["capacity"]["collision_error"], "DuplicateRef");
    assert_eq!(out["capacity"]["collision_unchanged"], true);
    assert_eq!(out["capacity"]["nodes"], json!([1, 2]));
    assert_eq!(
        out["capacity"]["spill"],
        json!({"kind": "Inserted", "node": 2, "entry": 1021})
    );
    assert_eq!(
        out["capacity"]["repeat"],
        json!({"kind": "Duplicate", "node": 1, "entry": 1001})
    );
    assert_eq!(
        out["capacity"]["refs"],
        json!((1001..=1021).collect::<Vec<_>>())
    );
    assert_eq!(out["collision"]["refs"], json!([2001, 2002]));
    assert_eq!(
        out["collision"]["original_bytes"],
        json!([[195, 169], [101, 204, 129]])
    );
    assert_eq!(
        out["keys"]["lookups"],
        json!([[3001], [3002], [3003], [3004], [3005]])
    );
    assert_eq!(out["keys"]["wrong_key_error"], "KeyMismatch");
    assert_eq!(out["keys"]["unchanged"], true);
    let entries = (4001..=4065).collect::<Vec<_>>();
    assert_eq!(out["lookup"]["array"], json!(&entries[..64]));
    assert_eq!(out["lookup"]["indexed"], json!(entries));
    assert_eq!(out["lookup"]["rebuilt"], out["lookup"]["indexed"]);
    assert_eq!(
        out["lookup"]["reversed"],
        json!((4001..=4065).rev().collect::<Vec<_>>())
    );
    assert_eq!(out["lookup"]["final_original"], "indexed 64");
    assert_eq!(
        out["resources"]["array_links"],
        json!((5001..=5064).collect::<Vec<_>>())
    );
    assert_eq!(
        out["resources"]["indexed_links"],
        json!((5001..=5065).collect::<Vec<_>>())
    );
    assert_eq!(
        out["resources"]["structural"],
        json!((5001..=5066).collect::<Vec<_>>())
    );
    assert_eq!(
        out["resources"]["rebuilt_links"],
        out["resources"]["indexed_links"]
    );
    assert_eq!(
        out["resources"]["reversed_links"],
        json!((5001..=5065).rev().collect::<Vec<_>>())
    );
    assert_eq!(out["resources"]["unlinked_original"], "unlinked original");
    assert_eq!(out["invalid"]["insert_error"], "InvalidWeight");
    assert_eq!(out["invalid"]["import_error"], "DuplicateRef");
    assert_eq!(out["invalid"]["insert_unchanged"], true);
    assert_eq!(out["invalid"]["import_unchanged"], true);
    assert_eq!(
        out["inherited_order"],
        json!(
            (1001..=1065)
                .chain([2001, 2002, 2003, 2001, 2002])
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(text, specimen_wasm::identity_probe::run());
}
