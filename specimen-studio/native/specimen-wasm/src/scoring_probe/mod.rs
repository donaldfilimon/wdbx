//! Finite authored synthetic mechanism witness. No host effects or efficacy claim.
mod budget_probe;
mod entries;
mod numeric;
mod orchestration;
mod project;
use serde_json::{Value, json};

// Actual logical-charge-v1 counters remain a separate labelled diagnostic view.
// Check their cross-target equality without claiming physical memory equivalence.
fn extract_accounting(value: &mut Value, path: &str, observations: &mut Vec<Value>) {
    match value {
        Value::Object(fields) => {
            if fields.contains_key("owned_within_limit")
                && let Some(bytes) = fields.shift_remove("owned_bytes")
            {
                observations.push(json!({
                    "path":path,
                    "owned_bytes":bytes,
                    "configured_limit":16*1024*1024
                }));
            }
            for (key, child) in fields {
                extract_accounting(child, &format!("{path}/{key}"), observations);
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter_mut().enumerate() {
                extract_accounting(child, &format!("{path}/{i}"), observations);
            }
        }
        _ => {}
    }
}
/// Actually runs immutable scoring and private-handle broker APIs. Raw accounting
/// is retained separately from the explicitly projected semantic trace.
pub fn run() -> String {
    let mut semantic = json!({
        "schema":"studio-scoring-parity-v2",
        "profile":"scoring-orchestration-v3-2026-10-23",
        "scanner":"text-scanner-v3",
        "scope":"authored-synthetic-pure-mechanisms",
        "logical_budget":budget_probe::run(),
        "numeric":numeric::cases(),
        "elections":numeric::elections(),
        "seeded":numeric::seeded(),
        "entries":entries::run(),
        "orchestration":orchestration::run(),
    });
    let mut accounting = vec![];
    extract_accounting(&mut semantic, "", &mut accounting);
    json!({
        "schema":"studio-scoring-parity-envelope-v2",
        "semantic":semantic,
        "diagnostics":{
            "schema":"logical-charge-v1-observations",
            "charge_profile":"logical-charge-v1",
            "target_arch":std::env::consts::ARCH,
            "observations":accounting
        }
    })
    .to_string()
}
