//! Every op the desktop dispatcher handles must be accepted by the protocol
//! gate, which deserializes each request before dispatch. An arm without a
//! `Request` variant is unreachable (this is how `storeInfo` shipped broken).
use specimen_core::protocol::Request;

fn dispatched_ops() -> Vec<String> {
    let main = include_str!("../../../src-tauri/src/main.rs");
    let start = main.find("match op {").expect("dispatcher match");
    let body = &main[start..];
    let body = &body[..body.find("_ => Err(").expect("fallback arm")];
    let mut ops = Vec::new();
    for line in body.lines() {
        let line = line.trim_start();
        if !line.starts_with('"') || !line.contains("=>") {
            continue;
        }
        for alt in line[..line.find("=>").unwrap()].split('|') {
            let alt = alt.trim();
            if let Some(op) = alt.strip_prefix('"').and_then(|a| a.strip_suffix('"')) {
                ops.push(op.to_owned());
            }
        }
    }
    ops
}

#[test]
fn every_dispatched_op_has_a_protocol_variant() {
    let ops = dispatched_ops();
    assert!(ops.len() > 15, "parsed too few ops: {ops:?}");
    for op in ops {
        // Missing fields are fine here; an unknown variant is not.
        if let Err(e) = serde_json::from_value::<Request>(serde_json::json!({ "op": op })) {
            assert!(
                !e.to_string().contains("unknown variant"),
                "op `{op}` is dispatched but the protocol rejects it: {e}"
            );
        }
    }
}

#[test]
fn store_info_passes_the_gate() {
    serde_json::from_value::<Request>(serde_json::json!({"op": "storeInfo"})).unwrap();
}
