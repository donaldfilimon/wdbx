//! Turn untrusted file text into a specimen and a composition network.
//! A derived network is a seeded sparse decoder (128 inputs, 32 outputs).
//! It is not trained, and it does not write ABI or Abbey stores.
use crate::{
    Result, engine, error,
    host::Host,
    mutate::{self, NodeInput, ResourceInput, SUBSYSTEMS},
    network_edit::{self, NetworkCommand},
    neural::Network,
};
use serde::Serialize;
use serde_json::{Value, json};

const MAX_TEXT: usize = 20 * 1024 * 1024;
const SCAN: usize = 256 * 1024;
const MAX_NODES: usize = 24;
const MAX_MEMORIES: usize = 48;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdaptedImport {
    pub specimen: Value,
    /// `None` means the file did not supply a network. The caller keeps the
    /// composition network already stored on the device.
    pub network: Option<Network>,
    pub summary: String,
    pub kind: String,
}

struct Piece {
    name: String,
    pattern: String,
    action: String,
    body: String,
}

/// A valid decoder whose width follows `node_count`. Same seed, same weights.
pub fn network_for_import(node_count: usize, seed: u64) -> Result<Network> {
    let nodes = node_count.max(1);
    let seed = seed.max(1);
    let mut net = Network::default();
    if nodes > 8 {
        let width = (48 + nodes.min(180) * 2).clamp(48, 256);
        net = network_edit::apply(
            &net,
            NetworkCommand::AddLayer {
                at: 1,
                outputs: width,
                fan_in: 8,
                seed,
            },
        )?;
    }
    let fan = 8 + nodes / 16;
    if fan > 8 {
        net = network_edit::apply(
            &net,
            NetworkCommand::SetConnectivity {
                index: 0,
                fan_in: fan.min(24),
                seed: seed.wrapping_add(3),
            },
        )?;
    }
    network_edit::apply(&net, NetworkCommand::InitWeights { seed })
}

/// Interprets `text` (a specimen, a pattern table, or prose) into a specimen
/// plus a composition network. `filename` only names the result.
pub fn adapt_import(h: &Host, text: &str, filename: &str) -> Result<AdaptedImport> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(error("Invalid", "This file is empty."));
    }
    if trimmed.len() > MAX_TEXT {
        return Err(error(
            "Invalid",
            "Choose a specimen file smaller than 20 MB.",
        ));
    }
    let looks_json = trimmed.starts_with('{') || trimmed.starts_with('[');
    if looks_json {
        let value = serde_json::from_str(trimmed)
            .map_err(|_| error("Invalid", "This file is not valid JSON."))?;
        return from_json(h, &value, filename);
    }
    from_pieces(
        h,
        filename,
        "prose",
        pieces_from_text(prefix(trimmed, SCAN)),
        Vec::new(),
        trimmed.len() > SCAN,
    )
}

fn from_json(h: &Host, value: &Value, filename: &str) -> Result<AdaptedImport> {
    match value {
        Value::Object(_) => from_object(h, value, filename),
        Value::Array(items) => from_array(h, items, filename),
        _ => Err(unsupported()),
    }
}

fn from_object(h: &Host, value: &Value, filename: &str) -> Result<AdaptedImport> {
    let schema = value.get("schema").and_then(|v| v.as_str());
    match schema {
        Some("wdbx.studio.v1") => specimen_import(h, value, value.get("network")),
        Some("wdbx.native.v2") => {
            let specimen = value.get("specimen").ok_or_else(unsupported)?;
            specimen_import(h, specimen, value.get("network"))
        }
        Some(_) => Err(unsupported()),
        None => {
            if let Some(adapted) = specimen_shaped(h, value)? {
                return Ok(adapted);
            }
            let (pieces, memories) = collect_bags(value);
            if pieces.is_empty() && memories.is_empty() {
                return Err(unsupported());
            }
            let name = value
                .get("name")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(filename);
            from_pieces(h, name, "patterns", pieces, memories, false)
        }
    }
}

fn from_array(h: &Host, items: &[Value], filename: &str) -> Result<AdaptedImport> {
    let (pieces, memories) = split_rows(items);
    if pieces.is_empty() && memories.is_empty() {
        return Err(error("Invalid", "This file has no patterns to import."));
    }
    from_pieces(h, filename, "patterns", pieces, memories, false)
}

fn specimen_shaped(h: &Host, value: &Value) -> Result<Option<AdaptedImport>> {
    let Some(nodes) = value.get("nodes").and_then(|n| n.as_array()) else {
        return Ok(None);
    };
    if !nodes.iter().any(|n| n.get("entries").is_some()) {
        return Ok(None);
    }
    let mut shell = blank(
        h,
        &specimen_name(
            value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Imported specimen"),
        ),
    );
    for key in [
        "nodes",
        "resources",
        "attachments",
        "history",
        "events",
        "mutations",
        "proposals",
        "atp",
        "settings",
        "notes",
    ] {
        if value.get(key).is_some() {
            shell[key] = value[key].clone();
        }
    }
    if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
        shell["name"] = json!(clip(name, 100));
    }
    match accept_specimen(h, &shell) {
        Ok(specimen) => Ok(Some(finish_specimen(specimen, value.get("network"))?)),
        Err(_) => Ok(None),
    }
}

fn specimen_import(h: &Host, specimen: &Value, network: Option<&Value>) -> Result<AdaptedImport> {
    let specimen = accept_specimen(h, specimen)?;
    finish_specimen(specimen, network)
}

fn finish_specimen(specimen: Value, embedded: Option<&Value>) -> Result<AdaptedImport> {
    let nodes = specimen["nodes"].as_array().map(Vec::len).unwrap_or(0);
    let name = specimen["name"].as_str().unwrap_or("Specimen");
    let (network, summary) = if let Some(network) = embedded.and_then(parse_network) {
        let layers = network.layers.len();
        (
            Some(network),
            format!(
                "Loaded {name} ({nodes} nodes). Kept the file's composition network ({layers} layers). It was not trained here."
            ),
        )
    } else if embedded.is_some() {
        let seed = mix(&format!("{name}:{nodes}"));
        let network = network_for_import(nodes, seed)?;
        let layers = network.layers.len();
        (
            Some(network),
            format!(
                "Loaded {name} ({nodes} nodes). The file's network was not a valid 32-output decoder, so a {layers}-layer network was derived. It is not trained."
            ),
        )
    } else {
        (
            None,
            format!(
                "Loaded {name} ({nodes} nodes). The composition network already on this device is unchanged."
            ),
        )
    };
    Ok(AdaptedImport {
        specimen,
        network,
        summary,
        kind: "specimen".into(),
    })
}

fn from_pieces(
    h: &Host,
    filename: &str,
    kind: &str,
    pieces: Vec<Piece>,
    memories: Vec<ResourceInput>,
    truncated: bool,
) -> Result<AdaptedImport> {
    let mut pieces = pieces;
    pieces.truncate(MAX_NODES);
    let mut memories = memories;
    memories.truncate(MAX_MEMORIES);
    if pieces.is_empty() && memories.is_empty() {
        return Err(error("Invalid", "This file has no patterns to import."));
    }
    let name = specimen_name(filename);
    let mut specimen = blank(h, &name);
    if pieces.is_empty() {
        specimen = mutate::add_node(
            h,
            &specimen,
            &NodeInput {
                name: "Recall".into(),
                pattern: "what can you recall".into(),
                action: "&recall(&current_input)".into(),
                ..Default::default()
            },
            None,
        )?;
    }
    let mut stored = 0usize;
    for piece in &pieces {
        let input = NodeInput {
            name: piece.name.clone(),
            pattern: piece.pattern.clone(),
            action: piece.action.clone(),
            tone: Some("neutral".into()),
            ..Default::default()
        };
        if let Ok(next) = mutate::add_node(h, &specimen, &input, None) {
            specimen = next;
            stored += 1;
            if piece.body.chars().count() > 80
                && let Ok((noted, _)) = crate::notes::create(
                    h,
                    &specimen,
                    &note_title(&piece.name),
                    &clip(&piece.body, 20_000),
                    &["import".into()],
                    None,
                )
            {
                specimen = noted;
            }
        }
    }
    let recall = specimen["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|n| n["name"] == "Recall"))
        .map(|n| n["patternId"].as_str().unwrap_or("").to_owned())
        .unwrap_or_default();
    let mut remembered = 0usize;
    for mut memory in memories {
        if memory.subsystem == "dictionary" && !recall.is_empty() {
            memory.resource_id = recall.clone();
        }
        if let Ok(next) = mutate::save_resource(h, &specimen, &memory, None) {
            specimen = next;
            remembered += 1;
        }
    }
    if specimen["nodes"].as_array().is_none_or(Vec::is_empty) {
        return Err(error("Invalid", "This file has no patterns to import."));
    }
    accept_specimen(h, &specimen)?;
    let nodes = specimen["nodes"].as_array().map(Vec::len).unwrap_or(0);
    let seed = mix(&format!("{name}:{stored}:{remembered}"));
    let network = network_for_import(nodes, seed)?;
    let layers = network.layers.len();
    let mut summary = format!(
        "Read {stored} patterns and {remembered} memories from {name}. Derived a {layers}-layer composition network (128 inputs, 32 outputs). It is not trained."
    );
    if truncated {
        summary.push_str(" Only the first part of a large file was read.");
    }
    if stored == MAX_NODES {
        summary.push_str(" Extra patterns were left out.");
    }
    mutate::log(h.env, &mut specimen, "system", "File imported", &summary);
    Ok(AdaptedImport {
        specimen,
        network: Some(network),
        summary,
        kind: kind.into(),
    })
}

fn pieces_from_text(text: &str) -> Vec<Piece> {
    let markdown = markdown_sections(text);
    if !markdown.is_empty() {
        return markdown;
    }
    let qa = qa_pairs(text);
    if !qa.is_empty() {
        return qa;
    }
    let csv = csv_rows(text);
    if !csv.is_empty() {
        return csv;
    }
    let mut paragraphs = paragraphs(text);
    if paragraphs.is_empty() && !text.trim().is_empty() {
        paragraphs.push(finish_piece("", text, text));
    }
    paragraphs
}

fn markdown_sections(text: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut title = String::new();
    let mut body = String::new();
    let mut saw = false;
    for line in text.lines() {
        if let Some(heading) = heading(line) {
            if saw {
                push_section(&mut out, &title, &body);
            }
            saw = true;
            title = heading;
            body.clear();
        } else if saw {
            if !body.is_empty() {
                body.push('\n');
            }
            body.push_str(line);
        }
    }
    if saw {
        push_section(&mut out, &title, &body);
    }
    out
}

fn heading(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    // `#Title` is not a heading; Markdown requires a space after the marks.
    let rest = trimmed.get(hashes..)?;
    if !rest.starts_with(' ') {
        return None;
    }
    let title = rest.trim();
    if title.is_empty() {
        None
    } else {
        Some(title.to_owned())
    }
}

fn push_section(out: &mut Vec<Piece>, title: &str, body: &str) {
    let body = body.trim();
    let action = if body.is_empty() { title } else { body };
    let mut piece = finish_piece(title, title, action);
    piece.body = body.to_owned();
    out.push(piece);
}

fn qa_pairs(text: &str) -> Vec<Piece> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if let Some(question) = strip_label(lines[i], &["Q:", "Question:"]) {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if let Some(answer) = lines
                .get(j)
                .and_then(|line| strip_label(line, &["A:", "Answer:"]))
            {
                out.push(finish_piece("", &question, &answer));
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn csv_rows(text: &str) -> Vec<Piece> {
    let rows: Vec<Vec<String>> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(csv_fields)
        .filter(|row| row.len() >= 2)
        .collect();
    if rows.len() < 2 {
        return Vec::new();
    }
    let header = rows[0].iter().map(|c| c.to_lowercase()).collect::<Vec<_>>();
    let (start, name_at, pattern_at, action_at) = if header
        .iter()
        .any(|c| c == "pattern" || c == "question" || c == "action" || c == "answer")
    {
        let pattern_at = header
            .iter()
            .position(|c| c == "pattern" || c == "question" || c == "prompt")
            .unwrap_or(0);
        let action_at = header
            .iter()
            .position(|c| c == "action" || c == "answer" || c == "response")
            .unwrap_or(1);
        let name_at = header.iter().position(|c| c == "name");
        (1, name_at, pattern_at, action_at)
    } else {
        (0, None, 0, 1)
    };
    rows.into_iter()
        .skip(start)
        .filter_map(|row| {
            let pattern = row.get(pattern_at)?.trim();
            let action = row.get(action_at)?.trim();
            if pattern.is_empty() || action.is_empty() {
                return None;
            }
            let name = name_at
                .and_then(|i| row.get(i))
                .map(String::as_str)
                .unwrap_or("");
            Some(finish_piece(name, pattern, action))
        })
        .collect()
}

fn paragraphs(text: &str) -> Vec<Piece> {
    text.split("\n\n")
        .map(str::trim)
        .filter(|part| part.chars().count() >= 12)
        .map(|part| finish_piece("", part, part))
        .collect()
}

fn collect_bags(value: &Value) -> (Vec<Piece>, Vec<ResourceInput>) {
    let mut pieces = Vec::new();
    let mut memories = Vec::new();
    for key in [
        "patterns",
        "nodes",
        "items",
        "entries",
        "pairs",
        "memories",
        "resources",
    ] {
        if let Some(rows) = value.get(key).and_then(|v| v.as_array()) {
            let (next_pieces, next_memories) = split_rows(rows);
            pieces.extend(next_pieces);
            memories.extend(next_memories);
        }
    }
    (pieces, memories)
}

fn split_rows(items: &[Value]) -> (Vec<Piece>, Vec<ResourceInput>) {
    let mut pieces = Vec::new();
    let mut memories = Vec::new();
    for item in items {
        if let Some(memory) = as_memory(item) {
            memories.push(memory);
            continue;
        }
        if let Some(piece) = as_piece(item) {
            pieces.push(piece);
        }
    }
    (pieces, memories)
}

fn as_memory(value: &Value) -> Option<ResourceInput> {
    let subsystem = value.get("subsystem")?.as_str()?;
    if !SUBSYSTEMS.contains(&subsystem) {
        return None;
    }
    let text = value
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let stored = value
        .get("value")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if text.is_empty() || stored.is_empty() {
        return None;
    }
    Some(ResourceInput {
        subsystem: subsystem.to_owned(),
        text: clip(text, 2000),
        value: clip(stored, 12_000),
        valence: 0.0,
        intensity: 0.4,
        ..Default::default()
    })
}

fn as_piece(value: &Value) -> Option<Piece> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        return Some(finish_piece("", text, text));
    }
    if value.get("subsystem").and_then(|v| v.as_str()).is_some() {
        return None;
    }
    let pattern = first_str(value, &["pattern", "question", "prompt", "input"])?;
    let action = first_str(value, &["action", "answer", "response", "output"])?;
    let name = first_str(value, &["name"]).unwrap_or("");
    Some(finish_piece(name, pattern, action))
}

fn finish_piece(name: &str, pattern: &str, action: &str) -> Piece {
    let pattern = one_line(pattern, 2000);
    let action = clip(action, 8000);
    let name = {
        let given = one_line(name, 80);
        if given.is_empty() {
            name_from(&pattern)
        } else {
            given
        }
    };
    Piece {
        name,
        pattern,
        action,
        body: String::new(),
    }
}

fn first_str<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
    })
}

fn strip_label(line: &str, labels: &[&str]) -> Option<String> {
    let trimmed = line.trim();
    for label in labels {
        if let Some(rest) = strip_prefix_ci(trimmed, label) {
            let rest = rest.trim();
            if !rest.is_empty() {
                return Some(rest.to_owned());
            }
        }
    }
    None
}

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let mut text_chars = text.chars();
    for expected in prefix.chars() {
        let got = text_chars.next()?;
        if !got.eq_ignore_ascii_case(&expected) {
            return None;
        }
    }
    Some(text_chars.as_str())
}

fn csv_fields(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for ch in line.chars() {
        match ch {
            '"' => quoted = !quoted,
            ',' if !quoted => {
                out.push(cur.trim().to_owned());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    out.push(cur.trim().to_owned());
    out
}

fn blank(h: &Host, name: &str) -> Value {
    json!({
        "schema": "wdbx.studio.v1",
        "name": clip(name, 100),
        "nodes": [],
        "resources": [],
        "attachments": [],
        "history": [],
        "events": [],
        "mutations": [],
        "proposals": [],
        "atp": {"valence": 0.2, "intensity": 0.25, "lastUpdate": 0},
        "settings": {
            "voteThreshold": 62, "jitter": 1.5, "maxStrength": 10, "initialStrength": 5,
            "entryLimit": 20, "scanLimit": 1000, "fanoutLimit": 12, "historyLimit": 100000,
            "pinLimit": 100, "maxNodes": 1000, "maxRafts": 8, "maxThreads": 1, "chunkSize": 256,
            "raftThreshold": 128, "brainstorm": false, "maintenance": false,
            "adaptiveThreading": false, "idleMin": 60, "idleMax": 120, "transientWidth": 32,
            "seed": 104729,
        },
        "updatedAt": h.env.now_rfc3339(),
    })
}

fn accept_specimen(h: &Host, value: &Value) -> Result<Value> {
    let migrated = mutate::migrate_ids(h, value);
    engine::validate(&migrated)?;
    mutate::validate_settings(&migrated["settings"])?;
    mutate::validate_records(&migrated)?;
    Ok(migrated)
}

fn parse_network(value: &Value) -> Option<Network> {
    let network: Network = serde_json::from_value(value.clone()).ok()?;
    network.validate().ok()?;
    Some(network)
}

fn specimen_name(filename: &str) -> String {
    let base = filename.rsplit(['/', '\\']).next().unwrap_or(filename);
    let stem = base.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(base);
    let cleaned = stem
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == ' ' || ch == '-' {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        "Imported specimen".into()
    } else {
        clip(&cleaned, 80)
    }
}

fn name_from(pattern: &str) -> String {
    let words = pattern
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    let words = if words.is_empty() {
        "Imported".into()
    } else {
        let mut chars = words.chars();
        let first = chars
            .next()
            .unwrap_or('I')
            .to_uppercase()
            .collect::<String>();
        first + chars.as_str()
    };
    clip(&words, 80)
}

fn note_title(title: &str) -> String {
    let cleaned = title
        .chars()
        .map(|ch| {
            if matches!(ch, '|' | '[' | ']' | '\n' | '\r') {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        "Imported note".into()
    } else {
        clip(&cleaned, 200)
    }
}

fn one_line(text: &str, max: usize) -> String {
    clip(&text.split_whitespace().collect::<Vec<_>>().join(" "), max)
}

fn clip(text: &str, max: usize) -> String {
    text.trim().chars().take(max).collect()
}

fn prefix(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn mix(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash.max(1)
}

fn unsupported() -> crate::Error {
    error(
        "Invalid",
        "This file is not a supported WDBX Studio specimen.",
    )
}
