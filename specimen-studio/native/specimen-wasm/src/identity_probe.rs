//! Opt-in synthetic runtime witness, not a save format or production command.
//! No caller input, corpus, I/O, encoder or host service is involved.
use serde_json::{Value, json};
use specimen_kernel::identity::*;
use specimen_kernel::language::Expr;

#[path = "../../specimen-kernel/tests/identity_fixture.rs"]
mod reviewed_fixture;

fn population() -> Population {
    Population::new("probe-tenant", "probe-specimen", "pattern")
}
fn key() -> PatternKey {
    PatternKey::new("prepared-probe-v1", "text", "low", [0xa5; 32])
}
fn entry(node: u128, reference: u128, original: String) -> EntryRecord {
    EntryRecord {
        population: population(),
        node: NodeRef(node),
        reference: EntryRef(reference),
        key: key(),
        original,
        ast: ActionAst {
            bind: vec![Expr::Text {
                value: "bind".into(),
            }],
            alternatives: vec![Alternative {
                action: vec![Expr::Text {
                    value: "result".into(),
                }],
                inhibited: false,
                weight: 0.5,
            }],
        },
        resource: None,
    }
}
fn refs(table: &HybridTable, key: &PatternKey) -> Vec<u128> {
    table
        .find_all(&population(), key)
        .iter()
        .map(|r| r.0)
        .collect()
}
fn links(table: &HybridTable) -> Vec<u128> {
    table
        .find_resource_links(&population(), &ResourceId("shared".into()))
        .iter()
        .map(|r| r.0)
        .collect()
}
fn outcome(value: InsertOutcome) -> Value {
    match value {
        InsertOutcome::Inserted { node, entry } => {
            json!({ "kind": "Inserted", "node": node.0, "entry": entry.0 })
        }
        InsertOutcome::Duplicate { node, entry } => {
            json!({ "kind": "Duplicate", "node": node.0, "entry": entry.0 })
        }
        InsertOutcome::Full => json!("Full"),
    }
}
fn capacity() -> Value {
    let mut table = HybridTable::default();
    for i in 0..20 {
        table
            .insert(entry(1, 1001 + i, format!("original {i}")), None)
            .unwrap();
    }
    let before = table.snapshot();
    let next = entry(1, 1021, "original 20".into());
    let full = table.insert(next.clone(), None).unwrap();
    let full_unchanged = table.snapshot() == before;
    let collision = table.insert(next.clone(), Some(NodeRef(1001))).unwrap_err();
    let collision_unchanged = table.snapshot() == before;
    let spill = table.insert(next, Some(NodeRef(2))).unwrap();
    let repeat = table
        .insert(entry(3, 9000, "original 0".into()), None)
        .unwrap();
    json!({
        "full": outcome(full), "full_unchanged": full_unchanged,
        "collision_error": format!("{collision:?}"), "collision_unchanged": collision_unchanged,
        "spill": outcome(spill), "repeat": outcome(repeat),
        "nodes": table.snapshot().nodes.iter().map(|n| n.reference.0).collect::<Vec<_>>(),
        "refs": refs(&table, &key()),
    })
}
fn collision() -> Value {
    let mut table = HybridTable::default();
    table.insert(entry(11, 2001, "é".into()), None).unwrap();
    table
        .insert(entry(12, 2002, "e\u{301}".into()), None)
        .unwrap();
    let original_bytes = [
        table
            .get(&population(), NodeRef(11), EntryRef(2001))
            .unwrap()
            .original
            .as_bytes(),
        table
            .get(&population(), NodeRef(12), EntryRef(2002))
            .unwrap()
            .original
            .as_bytes(),
    ];
    json!({ "refs": refs(&table, &key()), "original_bytes": original_bytes })
}
fn complete_keys() -> Value {
    let mut keys = vec![key(); 5];
    keys[1].encoder_version = "prepared-probe-v2".into();
    keys[2].modality = "image".into();
    keys[3].resolution = "high".into();
    keys[4].digest[31] ^= 1;
    let mut table = HybridTable::default();
    for (i, key) in keys.iter().enumerate() {
        let mut value = entry(21 + i as u128, 3001 + i as u128, format!("key {i}"));
        value.key = key.clone();
        table.insert(value, None).unwrap();
    }
    let before = table.snapshot();
    let mut invalid = entry(21, 3999, "invalid key".into());
    invalid.key = keys[1].clone();
    let error = table.insert(invalid, None).unwrap_err();
    json!({
        "lookups": keys.iter().map(|key| refs(&table, key)).collect::<Vec<_>>(),
        "wrong_key_error": format!("{error:?}"), "unchanged": table.snapshot() == before,
    })
}
fn lookup() -> Value {
    let mut table = HybridTable::default();
    let mut array = Vec::new();
    for i in 0..65 {
        table
            .insert(entry(101 + i, 4001 + i, format!("indexed {i}")), None)
            .unwrap();
        if i == 63 {
            array = refs(&table, &key());
        }
    }
    let indexed = refs(&table, &key());
    let final_original = table
        .get(&population(), NodeRef(165), EntryRef(4065))
        .unwrap()
        .original
        .clone();
    let snapshot = table.snapshot();
    let mut rebuilt = HybridTable::default();
    rebuilt.rebuild(snapshot.clone()).unwrap();
    let rebuilt_refs = refs(&rebuilt, &key());
    let mut reverse = snapshot;
    reverse.entries.reverse();
    rebuilt.rebuild(reverse).unwrap();
    json!({ "array": array, "indexed": indexed, "rebuilt": rebuilt_refs,
        "reversed": refs(&rebuilt, &key()), "final_original": final_original })
}
fn resources() -> Value {
    let mut table = HybridTable::default();
    let mut array_links = Vec::new();
    for i in 0..65 {
        table
            .insert_resource(ResourceRecord {
                population: population(),
                reference: ResourceRef(5001 + i),
                key: key(),
                original: "same linked original".into(),
                link: Some(ResourceId("shared".into())),
            })
            .unwrap();
        if i == 63 {
            array_links = links(&table);
        }
    }
    let indexed_links = links(&table);
    table
        .insert_resource(ResourceRecord {
            population: population(),
            reference: ResourceRef(5066),
            key: key(),
            original: "unlinked original".into(),
            link: None,
        })
        .unwrap();
    let structural = table
        .find_resources(&population(), &key())
        .iter()
        .map(|r| r.0)
        .collect::<Vec<_>>();
    let unlinked_original = table
        .get_resource(&population(), ResourceRef(5066))
        .unwrap()
        .original
        .clone();
    let snapshot = table.snapshot();
    let mut rebuilt = HybridTable::default();
    rebuilt.rebuild(snapshot.clone()).unwrap();
    let rebuilt_links = links(&rebuilt);
    let mut reverse = snapshot;
    reverse.resources.reverse();
    rebuilt.rebuild(reverse).unwrap();
    json!({ "array_links": array_links, "indexed_links": indexed_links,
        "structural": structural, "unlinked_original": unlinked_original,
        "rebuilt_links": rebuilt_links, "reversed_links": links(&rebuilt) })
}
fn invalid() -> Value {
    let mut table = HybridTable::default();
    table
        .insert(entry(601, 6001, "valid".into()), None)
        .unwrap();
    let before = table.snapshot();
    let mut bad = entry(602, 6002, "invalid".into());
    bad.ast.alternatives[0].weight = f64::NAN;
    let insert_error = table.insert(bad, None).unwrap_err();
    let insert_unchanged = table.snapshot() == before;
    let mut bad = before.clone();
    bad.nodes.push(bad.nodes[0].clone());
    let import_error = table.rebuild(bad).unwrap_err();
    json!({ "insert_error": format!("{insert_error:?}"), "insert_unchanged": insert_unchanged,
        "import_error": format!("{import_error:?}"), "import_unchanged": table.snapshot() == before })
}

/// Executes the bounded fixture and returns its actual mechanism results.
/// Panics on unexpected refusals; no shared error envelope can compare green.
pub fn run() -> String {
    json!({
        "schema": "task8-identity-mechanism-probe-v1",
        "capacity": capacity(), "collision": collision(), "keys": complete_keys(),
        "lookup": lookup(), "resources": resources(), "invalid": invalid(),
        "inherited_order": reviewed_fixture::ordered_refs_fixture(),
    })
    .to_string()
}
