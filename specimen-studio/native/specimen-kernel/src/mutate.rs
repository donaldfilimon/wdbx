//! Specimen edits shared by both editions: the browser through WASM, the
//! desktop webview through the same WASM. Behavior and user-facing messages
//! follow the retired `lib/specimen/engine.ts`; pattern IDs, RNG and settings
//! bounds are native (`language::identify`, `neural::Rng`, `engine::validate`).
use crate::{
    Error, Result,
    engine::{self, number, rows, text},
    error,
    host::{Env, Host},
    language,
    neural::Rng,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// Resource subsystems a memory may belong to (mirrors `SUBSYSTEMS`).
pub const SUBSYSTEMS: [&str; 11] = [
    "dictionary",
    "thesaurus",
    "antiThesaurus",
    "chargebook",
    "verbSemantics",
    "conjunctiveRules",
    "negation",
    "morphology",
    "roleMapper",
    "sigils",
    "inhibition",
];

fn invalid(message: &str) -> Error {
    error("Invalid", message)
}

/// Appends an event (newest first, 500 kept) and stamps `updatedAt`.
pub fn log(env: &dyn Env, s: &mut Value, kind: &str, title: &str, detail: &str) {
    let events = s["events"].as_array_mut().expect("events array");
    events.insert(
        0,
        json!({"id": env.uid(), "type": kind, "title": title, "detail": detail, "createdAt": env.now_rfc3339()}),
    );
    events.truncate(500);
    s["updatedAt"] = json!(env.now_rfc3339());
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInput {
    pub name: String,
    pub pattern: String,
    pub action: String,
    pub tone: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub context_id: Option<String>,
    pub entries: Option<Value>,
}

/// Creates a node, or edits the node `node_ref` (keeping its ref, strength,
/// jitter, creation time and sibling entries).
pub fn add_node(
    h: &Host,
    source: &Value,
    input: &NodeInput,
    node_ref: Option<&str>,
) -> Result<Value> {
    let mut s = source.clone();
    let (name, pattern, action) = (input.name.trim(), input.pattern.trim(), input.action.trim());
    if name.is_empty() || pattern.is_empty() || action.is_empty() {
        return Err(invalid("Enter a name, pattern, and action."));
    }
    if name.chars().count() > 100 || pattern.chars().count() > 2000 || action.chars().count() > 8000
    {
        return Err(invalid(
            "Keep names under 100, patterns under 2,000, and actions under 8,000 characters.",
        ));
    }
    let others = |s: &Value| -> Vec<Value> {
        rows(s, "nodes")
            .iter()
            .filter(|n| Some(text(n, "ref")) != node_ref)
            .cloned()
            .collect()
    };
    if others(&s).iter().any(|n| {
        rows(n, "entries")
            .iter()
            .any(|e| text(e, "pattern") == pattern)
    }) {
        return Err(invalid(
            "That exact pattern is already stored. Use a distinct pattern or edit its node.",
        ));
    }
    let inhibited = |p: &str, a: &str| {
        rows(&s, "mutations")
            .iter()
            .any(|m| text(m, "originalPattern") == p && text(m, "originalAction") == a)
    };
    if inhibited(pattern, action) {
        return Err(invalid(
            "This original pairing is inhibited by a previous mutation.",
        ));
    }
    let settings = s["settings"].clone();
    if node_ref.is_none() && rows(&s, "nodes").len() as f64 >= number(&settings, "maxNodes", 1000.)
    {
        return Err(invalid(
            "The node limit is reached. Adjust the limit in Settings.",
        ));
    }
    let old = node_ref.and_then(|r| {
        rows(&s, "nodes")
            .iter()
            .find(|n| text(n, "ref") == r)
            .cloned()
    });
    let first_entry = old
        .as_ref()
        .map(|o| o["entries"][0].clone())
        .unwrap_or(Value::Null);
    let first_alt = first_entry["alternatives"][0].clone();
    let id_or = |v: &Value| v.as_str().map(str::to_owned).unwrap_or_else(|| h.env.uid());
    let mut entries = match &input.entries {
        Some(e) => e.clone(),
        None => {
            let mut alts = vec![json!({
                "id": id_or(&first_alt["id"]),
                "action": action,
                "inhibition": first_alt["inhibition"].as_str().unwrap_or(""),
                "weight": first_alt["weight"].as_f64().unwrap_or(1.),
                "remixed": first_alt["remixed"].as_bool().unwrap_or(false),
            })];
            let mut list = vec![];
            if let Some(o) = &old {
                alts.extend(
                    rows(&o["entries"][0], "alternatives")
                        .iter()
                        .skip(1)
                        .cloned(),
                );
                list.push(json!({"id": id_or(&first_entry["id"]), "pattern": pattern, "alternatives": alts}));
                list.extend(rows(o, "entries").iter().skip(1).cloned());
            } else {
                list.push(json!({"id": h.env.uid(), "pattern": pattern, "alternatives": alts}));
            }
            json!(list)
        }
    };
    let first_pattern = entries[0]["pattern"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let pid = language::identify(&first_pattern);
    if entries.as_array().map_or(0, Vec::len) as f64 > number(&settings, "entryLimit", 20.) {
        return Err(invalid("This node exceeds its entry limit."));
    }
    let mut originals: BTreeSet<String> = others(&s)
        .iter()
        .flat_map(|n| {
            rows(n, "entries")
                .iter()
                .map(|e| text(e, "pattern").to_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    let mut slots = BTreeSet::new();
    for entry in entries.as_array().into_iter().flatten() {
        let p = text(entry, "pattern");
        if p.trim().is_empty() || p.chars().count() > 2000 || !originals.insert(p.to_owned()) {
            return Err(invalid("Original patterns must be nonempty and unique."));
        }
        if language::identify(p).id != pid.id {
            return Err(invalid(
                "Every entry must share this node’s Pattern ID. Move a different pattern to a new node.",
            ));
        }
        let alts = rows(entry, "alternatives");
        if alts.is_empty() || alts.len() > 100 {
            return Err(invalid("Store between 1 and 100 alternatives per entry."));
        }
        for a in alts {
            let w = a["weight"].as_f64().unwrap_or(f64::NAN);
            let act = text(a, "action");
            if act.trim().is_empty()
                || act.chars().count() > 8000
                || !w.is_finite()
                || w < 0.
                || !slots.insert(text(a, "id").to_owned())
            {
                return Err(invalid(
                    "Each vote needs a unique slot, action, and nonnegative finite weight.",
                ));
            }
            if inhibited(p, act) {
                return Err(invalid(
                    "A previous mutation inhibits this original pairing.",
                ));
            }
        }
    }
    if let Some(list) = entries.as_array_mut() {
        for e in list.iter_mut() {
            if e["id"].as_str().is_none_or(str::is_empty) {
                e["id"] = json!(h.env.uid());
            }
        }
    }
    let o = old.clone().unwrap_or(Value::Null);
    let node = json!({
        "ref": o["ref"].as_str().map(str::to_owned).unwrap_or_else(|| h.env.uid()),
        "name": name,
        "patternId": pid.id,
        "resolution": pid.resolution,
        "type": input.kind.clone().unwrap_or_else(|| "pattern".into()),
        "contextId": input.context_id.clone().unwrap_or_default(),
        "strength": o.get("strength").cloned().unwrap_or_else(|| settings["initialStrength"].clone()),
        "jitter": o.get("jitter").cloned().unwrap_or(json!(true)),
        "tone": input.tone.clone().unwrap_or_else(|| "neutral".into()),
        "entries": entries,
        "createdAt": o["createdAt"].as_str().map(str::to_owned).unwrap_or_else(|| h.env.now_rfc3339()),
    });
    let nodes = s["nodes"].as_array_mut().expect("nodes array");
    match node_ref.and_then(|r| nodes.iter().position(|n| text(n, "ref") == r)) {
        Some(i) => nodes[i] = node,
        None => nodes.push(node),
    }
    log(
        h.env,
        &mut s,
        "node",
        if old.is_some() {
            "Node updated"
        } else {
            "Pattern learned"
        },
        name,
    );
    Ok(s)
}

pub fn add_entry(
    h: &Host,
    source: &Value,
    node_ref: &str,
    pattern: &str,
    action: &str,
) -> Result<Value> {
    let mut s = source.clone();
    let limit = number(&s["settings"], "entryLimit", 20.);
    if rows(&s, "nodes").iter().any(|n| {
        rows(n, "entries")
            .iter()
            .any(|e| text(e, "pattern") == pattern)
    }) {
        return Err(invalid("This exact original pattern already exists."));
    }
    let node = s["nodes"]
        .as_array_mut()
        .expect("nodes array")
        .iter_mut()
        .find(|n| text(n, "ref") == node_ref)
        .ok_or_else(|| invalid("Node not found."))?;
    if rows(node, "entries").len() as f64 >= limit {
        return Err(invalid("This node is full. Create a related node."));
    }
    if language::identify(pattern).id != text(node, "patternId") {
        return Err(invalid(
            "The entry must produce the same Pattern ID as its node.",
        ));
    }
    let name = text(node, "name").to_owned();
    node["entries"].as_array_mut().expect("entries").push(json!({
        "id": h.env.uid(),
        "pattern": pattern,
        "alternatives": [{"id": h.env.uid(), "action": action, "inhibition": "", "weight": 1, "remixed": false}],
    }));
    log(h.env, &mut s, "node", "Entry added", &name);
    Ok(s)
}

/// Removes a node, its attachments, and memory links nothing else claims.
pub fn remove_node(h: &Host, source: &Value, node_ref: &str) -> Value {
    let mut s = source.clone();
    let Some(node) = rows(&s, "nodes")
        .iter()
        .find(|n| text(n, "ref") == node_ref)
        .cloned()
    else {
        return s;
    };
    s["nodes"]
        .as_array_mut()
        .expect("nodes")
        .retain(|n| text(n, "ref") != node_ref);
    s["attachments"]
        .as_array_mut()
        .expect("attachments")
        .retain(|a| text(a, "from") != node_ref && text(a, "to") != node_ref);
    let pid = text(&node, "patternId").to_owned();
    if !rows(&s, "nodes")
        .iter()
        .any(|n| text(n, "patternId") == pid)
    {
        for r in s["resources"].as_array_mut().expect("resources") {
            if text(r, "resourceId") == pid {
                r["resourceId"] = json!("");
            }
        }
    }
    log(h.env, &mut s, "node", "Node removed", text(&node, "name"));
    s
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInput {
    pub subsystem: String,
    pub text: String,
    pub value: String,
    #[serde(default)]
    pub resource_id: String,
    pub valence: f64,
    pub intensity: f64,
}

pub fn save_resource(
    h: &Host,
    source: &Value,
    input: &ResourceInput,
    resource_ref: Option<&str>,
) -> Result<Value> {
    let mut s = source.clone();
    if !input.valence.is_finite()
        || input.valence.abs() > 1.
        || !input.intensity.is_finite()
        || !(0. ..=1.).contains(&input.intensity)
    {
        return Err(invalid("Valence must be −1 to 1 and intensity 0 to 1."));
    }
    if resource_ref.is_none() && rows(&s, "resources").len() >= 10000 {
        return Err(invalid("The memory limit of 10,000 entries is reached."));
    }
    if input.text.trim().is_empty() || input.value.trim().is_empty() {
        return Err(invalid("Enter a term and its supporting information."));
    }
    if input.text.chars().count() > 2000 || input.value.chars().count() > 12000 {
        return Err(invalid("This memory entry is too long."));
    }
    if !SUBSYSTEMS.contains(&input.subsystem.as_str()) {
        return Err(invalid("Choose a registered subsystem."));
    }
    if !input.resource_id.is_empty()
        && !rows(&s, "nodes")
            .iter()
            .any(|n| text(n, "patternId") == input.resource_id)
    {
        return Err(invalid("The linked ID must belong to a stored node."));
    }
    let resource = json!({
        "ref": resource_ref.map(str::to_owned).unwrap_or_else(|| h.env.uid()),
        "subsystem": input.subsystem,
        "text": input.text,
        "value": input.value,
        "patternId": language::identify(&input.text).id,
        "resourceId": input.resource_id,
        "valence": input.valence,
        "intensity": input.intensity,
    });
    let list = s["resources"].as_array_mut().expect("resources");
    match resource_ref.and_then(|r| list.iter().position(|x| text(x, "ref") == r)) {
        Some(i) => list[i] = resource,
        None => list.push(resource),
    }
    log(
        h.env,
        &mut s,
        "memory",
        "Memory saved",
        &format!("{} · {}", input.subsystem, input.text),
    );
    Ok(s)
}

/// Applies Right/Wrong feedback to a cycle (whole answer, or one segment
/// color): each contributor changes strength on an independent coin flip.
pub fn feedback(
    h: &Host,
    source: &Value,
    cycle_id: &str,
    right: bool,
    color: Option<&str>,
) -> Result<(Value, String)> {
    let mut s = source.clone();
    let cycle = rows(&s, "history")
        .iter()
        .find(|c| text(c, "id") == cycle_id)
        .cloned()
        .ok_or_else(|| invalid("This answer is no longer available."))?;
    let event = color.unwrap_or("all");
    let given: Vec<String> = rows(&cycle, "feedback")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    if given.iter().any(|g| g == event || g == "all") {
        return Err(invalid(
            "Feedback was already applied to this answer or segment.",
        ));
    }
    let contributors = |filter: &dyn Fn(&Value) -> bool| -> Vec<String> {
        let mut out: Vec<String> = vec![];
        for seg in rows(&cycle, "segments").iter().filter(|seg| filter(seg)) {
            for r in rows(seg, "contributors").iter().filter_map(Value::as_str) {
                if !out.iter().any(|x| x == r) {
                    out.push(r.to_owned());
                }
            }
        }
        out
    };
    let previously: BTreeSet<String> =
        contributors(&|seg| given.iter().any(|g| g == text(seg, "color")))
            .into_iter()
            .collect();
    let refs = contributors(&|seg| color.is_none_or(|c| text(seg, "color") == c));
    if refs.is_empty() {
        return Err(invalid("This segment has no contributing nodes."));
    }
    if refs.iter().any(|r| previously.contains(r)) {
        return Err(invalid(
            "A contributor in this selection already received feedback for this cycle.",
        ));
    }
    let max = number(&s["settings"], "maxStrength", 10.);
    let mut rng = Rng(number(&s["settings"], "seed", 104729.) as u64);
    let mut changed = 0;
    for r in &refs {
        let Some(node) = s["nodes"]
            .as_array_mut()
            .expect("nodes")
            .iter_mut()
            .find(|n| text(n, "ref") == r)
        else {
            continue;
        };
        if rng.sample() < 0.5 {
            let next = (number(node, "strength", 0.) + if right { 1. } else { -1. }).clamp(0., max);
            node["strength"] = json!(next);
            changed += 1;
        }
    }
    s["settings"]["seed"] = json!((rng.0 % u32::MAX as u64).max(1));
    if let Some(c) = s["history"]
        .as_array_mut()
        .expect("history")
        .iter_mut()
        .find(|c| text(c, "id") == cycle_id)
    {
        c["feedback"]
            .as_array_mut()
            .expect("feedback")
            .push(json!(event));
    }
    let spent: Vec<String> = rows(&s, "nodes")
        .iter()
        .filter(|n| number(n, "strength", 1.) == 0.)
        .map(|n| text(n, "ref").to_owned())
        .collect();
    for r in spent {
        s = remove_node(h, &s, &r);
    }
    let message = format!(
        "{changed} of {} contributors changed strength after independent coin flips.",
        refs.len()
    );
    log(
        h.env,
        &mut s,
        "feedback",
        if right {
            "Positive feedback"
        } else {
            "Negative feedback"
        },
        &message,
    );
    Ok((s, message))
}

pub fn toggle_pin(h: &Host, source: &Value, cycle_id: &str) -> Result<Value> {
    let mut s = source.clone();
    let pinned_count = rows(&s, "history")
        .iter()
        .filter(|c| c["pinned"] == true)
        .count() as f64;
    let limit = number(&s["settings"], "pinLimit", 100.);
    let Some(c) = s["history"]
        .as_array_mut()
        .expect("history")
        .iter_mut()
        .find(|c| text(c, "id") == cycle_id)
    else {
        return Ok(s);
    };
    if c["pinned"] != true && pinned_count >= limit {
        return Err(invalid("The pin limit is reached. Unpin a record first."));
    }
    let now = c["pinned"] != true;
    c["pinned"] = json!(now);
    let input = text(c, "input").to_owned();
    engine::trim(&mut s);
    log(
        h.env,
        &mut s,
        "memory",
        if now {
            "Conversation pinned"
        } else {
            "Conversation unpinned"
        },
        &input,
    );
    Ok(s)
}

/// Settings checks with native bounds (from `engine::validate`) plus the type
/// checks the browser profile enforced.
pub fn validate_settings(settings: &Value) -> Result<()> {
    const BOUNDS: [(&str, f64, f64); 18] = [
        ("voteThreshold", -100., 100.),
        ("jitter", 0., 100.),
        ("maxStrength", 1., 1000.),
        ("initialStrength", 1., 1000.),
        ("entryLimit", 1., 1000.),
        ("scanLimit", 1., 1000.),
        ("fanoutLimit", 1., 100.),
        ("historyLimit", 1., 100000.),
        ("pinLimit", 1., 10000.),
        ("maxNodes", 1., 100000.),
        ("maxRafts", 1., 64.),
        ("maxThreads", 1., 64.),
        ("chunkSize", 1., 4096.),
        ("raftThreshold", 1., 10000.),
        ("idleMin", 1., 86400.),
        ("idleMax", 1., 86400.),
        ("transientWidth", 8., 128.),
        ("seed", 1., 4294967295.),
    ];
    for (key, lo, hi) in BOUNDS {
        let v = settings[key].as_f64().unwrap_or(f64::NAN);
        if !v.is_finite() || v < lo || v > hi {
            return Err(invalid(&format!(
                "Set {key} between {} and {}.",
                fmt_bound(lo),
                fmt_bound(hi)
            )));
        }
        if key != "voteThreshold" && key != "jitter" && v.fract() != 0. {
            return Err(invalid(&format!("{key} must be a whole number.")));
        }
    }
    if settings["initialStrength"].as_f64() > settings["maxStrength"].as_f64() {
        return Err(invalid("Initial strength cannot exceed maximum strength."));
    }
    if settings["idleMin"].as_f64() > settings["idleMax"].as_f64() {
        return Err(invalid(
            "The minimum idle interval cannot exceed the maximum.",
        ));
    }
    for key in ["brainstorm", "maintenance", "adaptiveThreading"] {
        if !settings[key].is_boolean() {
            return Err(invalid(&format!("{key} must be on or off.")));
        }
    }
    Ok(())
}

fn fmt_bound(v: f64) -> String {
    if v.fract() == 0. {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// Recomputes node and memory Pattern IDs with the native identifier and
/// remaps memory links, so browser saves from the TypeScript engine load.
pub fn migrate_ids(h: &Host, source: &Value) -> Value {
    let mut s = source.clone();
    let mut remap = BTreeMap::new();
    for n in s["nodes"].as_array_mut().into_iter().flatten() {
        let first = n["entries"][0]["pattern"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let pid = language::identify(&first);
        let old = text(n, "patternId").to_owned();
        if old != pid.id {
            remap.insert(old, pid.id.clone());
        }
        n["patternId"] = json!(pid.id);
        n["resolution"] = json!(pid.resolution);
    }
    let mut changed = !remap.is_empty();
    for r in s["resources"].as_array_mut().into_iter().flatten() {
        let pid = language::identify(text(r, "text")).id;
        if text(r, "patternId") != pid {
            r["patternId"] = json!(pid);
            changed = true;
        }
        if let Some(new) = remap.get(text(r, "resourceId")) {
            r["resourceId"] = json!(new);
        }
    }
    if changed {
        log(
            h.env,
            &mut s,
            "system",
            "Pattern IDs migrated",
            "Recomputed with the native identifier.",
        );
    }
    s
}

/// The starter specimen: eight pattern nodes and twelve supporting memories.
pub fn seed(h: &Host) -> Value {
    let settings = json!({
        "voteThreshold": 62, "jitter": 1.5, "maxStrength": 10, "initialStrength": 5,
        "entryLimit": 20, "scanLimit": 1000, "fanoutLimit": 12, "historyLimit": 100000,
        "pinLimit": 100, "maxNodes": 1000, "maxRafts": 8, "maxThreads": 1, "chunkSize": 256,
        "raftThreshold": 128, "brainstorm": false, "maintenance": false,
        "adaptiveThreading": false, "idleMin": 60, "idleMax": 120, "transientWidth": 32,
        "seed": 104729,
    });
    let mut s = json!({
        "schema": "wdbx.studio.v1", "name": "Specimen 001", "nodes": [], "resources": [],
        "attachments": [], "history": [], "events": [], "mutations": [], "proposals": [],
        "atp": {"valence": 0.2, "intensity": 0.25, "lastUpdate": 0},
        "settings": settings, "updatedAt": h.env.now_rfc3339(),
    });
    let defaults = [
        ("Calculate", "&n &op &n", "&calc(&current_input)"),
        (
            "Greeting",
            "hello",
            "Hello. What would you like to explore?",
        ),
        ("Repeat", "say &text &n times", "&repeat(&current_input)"),
        ("Time", "what time is it", "&time&"),
        ("Recall", "what can you recall", "&recall(&current_input)"),
        ("Imagine", "imagine &text", "&imagine(&current_input)"),
        ("Tone", "how are you feeling", "&tone&"),
        ("Negation", "don't &text", "I will leave that action alone."),
    ];
    for (name, pattern, action) in defaults {
        let input = NodeInput {
            name: name.into(),
            pattern: pattern.into(),
            action: action.into(),
            tone: Some(
                if name == "Greeting" {
                    "warm"
                } else {
                    "neutral"
                }
                .into(),
            ),
            ..Default::default()
        };
        s = add_node(h, &s, &input, None).expect("starter node");
    }
    s["nodes"][0]["strength"] = json!(6);
    let recall = s["nodes"][4]["patternId"].clone();
    let memories = [
        (
            "dictionary",
            "specimen",
            "A persistent collection of pattern nodes and supporting organs.",
        ),
        (
            "dictionary",
            "WDBX",
            "A system for retrieving patterns, collecting gated votes, and composing responses.",
        ),
        ("thesaurus", "hello", "hi, hey, greetings"),
        ("chargebook", "happy", "warm, pleased, content"),
        ("chargebook", "stressed", "tense, concerned"),
        (
            "negation",
            "do not",
            "Inhibit the scoped action rather than execute it.",
        ),
        (
            "verbSemantics",
            "give",
            "subject, predicate, object, recipient",
        ),
        ("morphology", "running", "run"),
        (
            "antiThesaurus",
            "race",
            "Use context to distinguish running from ethnicity.",
        ),
        (
            "conjunctiveRules",
            "and",
            "Combine independent, compatible contributions.",
        ),
        (
            "roleMapper",
            "subject predicate object",
            "Preserve who does what to whom.",
        ),
        (
            "sigils",
            "&current_input",
            "Read the original input; never execute it as arbitrary code.",
        ),
    ];
    for (subsystem, t, value) in memories {
        s["resources"]
            .as_array_mut()
            .expect("resources")
            .push(json!({
                "ref": h.env.uid(),
                "subsystem": subsystem,
                "text": t,
                "value": value,
                "patternId": language::identify(t).id,
                "resourceId": if subsystem == "dictionary" { recall.clone() } else { json!("") },
                "valence": if t == "happy" { 0.7 } else { 0. },
                "intensity": 0.4,
            }));
    }
    s["events"] = json!([]);
    log(
        h.env,
        &mut s,
        "system",
        "Starter specimen created",
        "8 pattern nodes and 12 supporting memories.",
    );
    s
}
