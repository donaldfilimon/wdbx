use crate::{
    Result, error,
    host::{Env, Host},
    language::{self, Expr, Scope},
    neural::{Network, Rng},
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::{AtomicBool, Ordering},
};
pub fn rows<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v[key].as_array().map_or(&[], Vec::as_slice)
}
pub fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
pub fn number(v: &Value, key: &str, default: f64) -> f64 {
    v[key].as_f64().unwrap_or(default)
}
fn cancelled(flag: &AtomicBool) -> Result<()> {
    if flag.load(Ordering::Relaxed) {
        Err(error(
            "Cancelled",
            "No partial cycle changes were committed",
        ))
    } else {
        Ok(())
    }
}
pub fn validate(s: &Value) -> Result<()> {
    if s["schema"] != "wdbx.studio.v1" || !s["name"].is_string() || text(s, "name").len() > 100 {
        return Err(error(
            "MalformedSave",
            "Unsupported specimen schema or name",
        ));
    }
    for key in [
        "nodes",
        "resources",
        "attachments",
        "history",
        "events",
        "mutations",
        "proposals",
    ] {
        if !s[key].is_array() {
            return Err(error("MalformedSave", format!("Missing {key}")));
        }
    }
    let settings = &s["settings"];
    for (key, min, max) in [
        ("voteThreshold", -100., 100.),
        ("jitter", 0., 100.),
        ("entryLimit", 1., 1000.),
        ("maxStrength", 1., 1000.),
        ("initialStrength", 1., 1000.),
        ("maxNodes", 1., 100000.),
        ("scanLimit", 1., 1000.),
        ("fanoutLimit", 1., 100.),
        ("historyLimit", 1., 100000.),
        ("pinLimit", 1., 10000.),
        ("maxRafts", 1., 64.),
        ("chunkSize", 1., 4096.),
        ("idleMin", 1., 86400.),
        ("idleMax", 1., 86400.),
    ] {
        let n = settings[key]
            .as_f64()
            .ok_or_else(|| error("MalformedSave", format!("Missing numeric setting {key}")))?;
        if !n.is_finite() || n < min || n > max {
            return Err(error(
                "BudgetExceeded",
                format!("Setting {key} exceeds bounds"),
            ));
        }
    }
    if rows(s, "nodes").len() > number(settings, "maxNodes", 1000.) as usize
        || rows(s, "resources").len() > 10000
        || rows(s, "history").len()
            > (number(settings, "historyLimit", 100000.) + number(settings, "pinLimit", 100.))
                as usize
    {
        return Err(error(
            "BudgetExceeded",
            "Collection exceeds configured limit",
        ));
    }
    if number(settings, "idleMin", 60.) > number(settings, "idleMax", 120.)
        || number(settings, "initialStrength", 5.) > number(settings, "maxStrength", 10.)
    {
        return Err(error("MalformedSave", "Inconsistent configuration limits"));
    }
    for key in ["events", "mutations", "proposals", "attachments"] {
        if rows(s, key).len() > 100000 {
            return Err(error("BudgetExceeded", "Collection exceeds native budget"));
        }
    }
    for h in rows(s, "history") {
        if text(h, "id").is_empty()
            || !h["segments"].is_array()
            || !h["votes"].is_array()
            || !h["trace"].is_array()
            || !h["feedback"].is_array()
        {
            return Err(error("MalformedSave", "Malformed history record"));
        }
    }
    let mut refs = BTreeSet::new();
    let mut originals = BTreeSet::new();
    let mut slots = BTreeSet::new();
    let mut entries_seen = BTreeSet::new();
    for n in rows(s, "nodes") {
        if text(n, "ref").is_empty()
            || !refs.insert(text(n, "ref"))
            || !["pattern", "A", "B"].contains(&text(n, "type"))
            || number(n, "strength", -1.) < 0.
            || number(n, "strength", -1.) > number(settings, "maxStrength", 10.)
        {
            return Err(error(
                "MalformedSave",
                "Invalid node identity, type, or strength",
            ));
        }
        let entries = rows(n, "entries");
        if entries.is_empty() || entries.len() > number(settings, "entryLimit", 20.) as usize {
            return Err(error("BudgetExceeded", "Invalid entry count"));
        }
        for e in entries {
            if text(e, "id").is_empty() || !entries_seen.insert(text(e, "id")) {
                return Err(error("MalformedSave", "Entry identities must be unique"));
            }
            let p = text(e, "pattern");
            if p.is_empty() || p.len() > 2000 || !originals.insert(p) {
                return Err(error(
                    "MalformedSave",
                    "Patterns must be unique and bounded",
                ));
            }
            let alternatives = rows(e, "alternatives");
            if alternatives.is_empty() || alternatives.len() > 100 {
                return Err(error(
                    "BudgetExceeded",
                    "Alternative count must be 1 to 100",
                ));
            }
            for a in alternatives {
                if text(a, "id").is_empty()
                    || !slots.insert(text(a, "id"))
                    || text(a, "action").is_empty()
                    || text(a, "action").len() > 8000
                    || number(a, "weight", -1.) < 0.
                    || !number(a, "weight", -1.).is_finite()
                {
                    return Err(error("MalformedSave", "Invalid alternative"));
                }
                language::parse(text(a, "action"), Scope::NodeAction)?;
            }
        }
    }
    for a in rows(s, "attachments") {
        if !refs.contains(text(a, "from")) || !refs.contains(text(a, "to")) {
            return Err(error(
                "MalformedSave",
                "Attachment refers to a missing node",
            ));
        }
    }
    for r in rows(s, "resources") {
        if text(r, "ref").is_empty()
            || text(r, "value").len() > 12000
            || text(r, "text").len() > 2000
            || number(r, "valence", 2.).abs() > 1.
            || !(0.0..=1.0).contains(&number(r, "intensity", 2.))
        {
            return Err(error("MalformedSave", "Invalid resource"));
        }
    }
    Ok(())
}
fn pick<'a>(items: &'a [Value], key: &str, rng: &mut Rng) -> Option<&'a Value> {
    if items.is_empty() {
        return None;
    }
    let total = items
        .iter()
        .map(|x| number(x, key, 1.).max(0.))
        .sum::<f64>();
    if total <= 0. {
        return items.get((rng.sample() * items.len() as f64) as usize);
    }
    let mut p = rng.sample() * total;
    for item in items {
        p -= number(item, key, 1.).max(0.);
        if p < 0. {
            return Some(item);
        }
    }
    items.last()
}
struct ResourceIndex<'a> {
    by_id: BTreeMap<String, Vec<&'a Value>>,
}
impl<'a> ResourceIndex<'a> {
    fn new(state: &'a Value) -> Self {
        let mut by_id: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
        for resource in rows(state, "resources") {
            for key in [text(resource, "ref"), text(resource, "patternId")] {
                if !key.is_empty() {
                    by_id.entry(key.into()).or_default().push(resource);
                }
            }
            by_id
                .entry(language::identify(text(resource, "text")).id)
                .or_default()
                .push(resource);
            if !text(resource, "resourceId").is_empty() {
                by_id
                    .entry(text(resource, "resourceId").into())
                    .or_default()
                    .push(resource);
            }
        }
        Self { by_id }
    }
}
fn support(index: &ResourceIndex<'_>, input: &str, action: &str, node: &Value) -> Vec<Value> {
    let query = language::normalize(&format!("{input} {action}"));
    fn literals(expr: &[Expr], keys: &mut BTreeSet<String>) {
        for item in expr {
            if let Expr::Call { name, args } = item {
                if name.eq_ignore_ascii_case("LookUp") || name.eq_ignore_ascii_case("memory") {
                    for arg in args {
                        let text = arg
                            .iter()
                            .filter_map(|e| {
                                if let Expr::Text { value } = e {
                                    Some(value.as_str())
                                } else {
                                    None
                                }
                            })
                            .collect::<String>();
                        keys.insert(text.trim().to_owned());
                    }
                }
                for arg in args {
                    literals(arg, keys);
                }
            }
        }
    }
    let mut explicit = BTreeSet::new();
    if let Ok(ast) = language::parse(action, Scope::NodeAction) {
        literals(&ast, &mut explicit);
    }
    let mut keys = language::tokens(&query)
        .iter()
        .map(|t| language::identify(t).id)
        .chain([
            language::identify(input).id,
            language::identify(action).id,
            text(node, "patternId").into(),
        ])
        .collect::<BTreeSet<_>>();
    for key in &explicit {
        keys.insert(key.clone());
        keys.insert(language::identify(key).id);
    }
    let mut seen = BTreeSet::new();
    keys.iter()
        .filter_map(|key| index.by_id.get(key))
        .flatten()
        .filter(|r| {
            seen.insert(text(r, "ref"))
                && ((!text(r, "resourceId").is_empty()
                    && text(r, "resourceId") == text(node, "patternId"))
                    || explicit.contains(text(r, "ref"))
                    || explicit.contains(text(r, "patternId"))
                    || explicit.contains(text(r, "text"))
                    || query.contains(&language::normalize(text(r, "text"))))
        })
        .map(|r| (*r).clone())
        .collect()
}

struct Context<'a> {
    host: &'a Host<'a>,
    input: &'a str,
    binding: language::Binding,
    state: &'a Value,
    resources: &'a [Value],
    steps: usize,
    network: &'a Network,
    visual: Option<Value>,
    transforms: Vec<String>,
}
fn eval(expr: &[Expr], ctx: &mut Context<'_>) -> Result<String> {
    let mut output = String::new();
    for e in expr {
        ctx.steps += 1;
        if ctx.steps > 1000 {
            return Err(error("BudgetExceeded", "Action call budget reached"));
        }
        match e {
            Expr::Text { value } => output.push_str(value),
            Expr::Call { name, args } => {
                let mut values = Vec::new();
                for arg in args {
                    values.push(eval(arg, ctx)?);
                }
                let arg = values.first().map(String::as_str).unwrap_or(ctx.input);
                let value = match name.to_lowercase().as_str() {
                    "current_input" => ctx.input.into(),
                    "literal" => arg.into(),
                    "n" => ctx.binding.numbers.first().cloned().unwrap_or_default(),
                    "text" => ctx.binding.text.clone(),
                    "op" => ctx.binding.operator.clone(),
                    "equals" => "=".into(),
                    "combineslots" => arg.into(),
                    "calc" => {
                        let parts = arg.split("||").map(str::trim).collect::<Vec<_>>();
                        let n = if parts.len() > 2 {
                            let op = parts.last().unwrap();
                            let nums = parts[..parts.len() - 1]
                                .iter()
                                .map(|p| language::calculate(p))
                                .collect::<Result<Vec<_>>>()?;
                            let mut n = nums[0];
                            for next in &nums[1..] {
                                n = language::calculate(&format!("({n}){op}({next})"))?;
                            }
                            n
                        } else {
                            language::calculate(arg)?
                        };
                        ctx.transforms.push("bounded arithmetic parser".into());
                        n.to_string()
                    }
                    "repeat" => {
                        let normalized = language::normalize(arg);
                        let cap = language::re(r"^say\s+(.+)\s+(\d+)\s+\*$").captures(&normalized);
                        let (value, count) = if let Some(c) = cap {
                            (c[1].to_owned(), c[2].parse::<usize>().unwrap_or(0))
                        } else {
                            (
                                arg.to_owned(),
                                values
                                    .get(1)
                                    .and_then(|s| s.parse::<usize>().ok())
                                    .unwrap_or(1),
                            )
                        };
                        if count == 0 || count > 100 {
                            return Err(error("BudgetExceeded", "Repeat count must be 1 to 100"));
                        }
                        std::iter::repeat_n(value, count)
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                    "automata" => language::tape(arg, ctx.input)?,
                    "lookup" | "memory" => {
                        let found = ctx.resources.iter().find(|r| {
                            if name.eq_ignore_ascii_case("memory") {
                                text(r, "text").eq_ignore_ascii_case(arg)
                            } else {
                                text(r, "subsystem").eq_ignore_ascii_case(arg)
                                    && values.get(1).is_some_and(|id| {
                                        text(r, "ref") == id
                                            || text(r, "patternId") == id
                                            || text(r, "text").eq_ignore_ascii_case(id)
                                    })
                            }
                        });
                        found
                            .map(|r| text(r, "value").to_owned())
                            .unwrap_or_else(|| "No matching supporting resource.".into())
                    }
                    "recall" => {
                        let mut items = ctx
                            .resources
                            .iter()
                            .filter(|r| text(r, "subsystem") == "dictionary")
                            .map(|r| format!("{}: {}", text(r, "text"), text(r, "value")))
                            .collect::<Vec<_>>();
                        items.extend(
                            rows(ctx.state, "history")
                                .iter()
                                .rev()
                                .filter(|h| h["pinned"] == true)
                                .take(3)
                                .map(|h| format!("{}: {}", text(h, "createdAt"), text(h, "input"))),
                        );
                        if items.is_empty() {
                            "No relevant stored memory.".into()
                        } else {
                            items.join("\n\n")
                        }
                    }
                    "time" => {
                        let history = rows(ctx.state, "history").last();
                        format!(
                            "Current time: {}.{}",
                            ctx.host.env.now_rfc3339(),
                            history
                                .map(|h| format!(
                                    " Previous input at {}: {}",
                                    text(h, "createdAt"),
                                    text(h, "input")
                                ))
                                .unwrap_or_default()
                        )
                    }
                    "tone" => format!(
                        "ATP valence {:.2}, intensity {:.2}.",
                        number(&ctx.state["atp"], "valence", 0.),
                        number(&ctx.state["atp"], "intensity", 0.)
                    ),
                    "imagine" => {
                        let ingredients = ctx
                            .resources
                            .iter()
                            .map(|r| text(r, "value").to_owned())
                            .collect::<Vec<_>>();
                        let mut features = crate::neural::encode(
                            arg,
                            &ingredients,
                            number(&ctx.state["atp"], "valence", 0.) as f32,
                            number(&ctx.state["atp"], "intensity", 0.) as f32,
                        );
                        let visual_ingredients = ctx
                            .resources
                            .iter()
                            .filter_map(|r| r["visualFeatures"].as_array())
                            .filter(|v| v.len() == 80)
                            .collect::<Vec<_>>();
                        for ingredient in &visual_ingredients {
                            for (i, value) in ingredient.iter().enumerate() {
                                features[i] = (features[i]
                                    + value.as_f64().unwrap_or(0.) as f32
                                        / visual_ingredients.len() as f32)
                                    .clamp(0., 1.);
                            }
                        }
                        let mut synthesis = ctx.network.run(
                            &features,
                            !ingredients.is_empty(),
                            number(&ctx.state["settings"], "seed", 104729.) as u64,
                            if ctx.state["settings"]["gpu"] == true {
                                ctx.host.accel
                            } else {
                                None
                            },
                        )?;
                        if ctx.state["settings"]["brainstorm"] == true {
                            let mut sampling = Rng(number(&ctx.state["settings"], "seed", 104729.)
                                as u64
                                ^ 0x425241494e);
                            for value in &mut synthesis.values {
                                *value = (*value + (sampling.sample() as f32 - 0.5) * 0.1)
                                    .clamp(0., 16.);
                            }
                            ctx.transforms
                                .push("bounded brainstorm composition sampling".into());
                        }
                        let colors = ["#14786b", "#91c7a7", "#e5ad50", "#8174b7"];
                        let v = &synthesis.values;
                        ctx.visual = Some(
                            json!({"xArray":(0..32).map(|i|(i as f64*2.399).cos()*120.*v[i]as f64).collect::<Vec<_>>(),"yArray":(0..32).map(|i|(i as f64*2.399).sin()*70.*v[i]as f64).collect::<Vec<_>>(),"colorArray":(0..32).map(|i|colors[i%4]).collect::<Vec<_>>(),"brightnessArray":v.iter().map(|x|x.clamp(0.0,1.0)).collect::<Vec<_>>(),"size":{"width":320,"height":180},"position":{"x":0,"y":0},"synthesis":{"backend":synthesis.backend,"activationFunctions":synthesis.activations,"fallback":synthesis.fallback,"networkVersion":ctx.network.version,"seed":ctx.state["settings"]["seed"],"brainstorm":ctx.state["settings"]["brainstorm"]}}),
                        );
                        ctx.transforms.extend([
                            "fixed 128/64/32 sparse network".into(),
                            "bounded visual parameter decoder".into(),
                        ]);
                        format!(
                            "A structural study of {}. {} retrieved ingredients contributed; the composition is transient.",
                            arg.trim_start_matches("imagine "),
                            ingredients.len()
                        )
                    }
                    _ => return Err(error("InvalidScope", "Unregistered action")),
                };
                output.push_str(&value);
            }
        }
        if output.len() > 32768 {
            return Err(error("BudgetExceeded", "Action output exceeds 32 KiB"));
        }
    }
    Ok(output)
}
fn append_event(env: &dyn Env, s: &mut Value, kind: &str, title: &str, detail: &str) {
    let events = s["events"].as_array_mut().unwrap();
    events.insert(0,json!({"id":env.uid(),"type":kind,"title":title,"detail":detail,"createdAt":env.now_rfc3339()}));
    events.truncate(500);
    s["updatedAt"] = json!(env.now_rfc3339());
}
/// Initial conservative lexical profile. Only curated equivalent families may
/// be substituted; resource authors cannot authorize arbitrary semantic changes.
fn vary_literal(
    value: &str,
    tone: &str,
    resources: &[Value],
    atp: &Value,
    rng: &mut Rng,
) -> (String, Option<String>) {
    let tokens = language::tokens(value);
    if tone == "neutral"
        || value.chars().any(|c| c.is_ascii_digit())
        || value.contains(['`', '"', '&'])
        || tokens.iter().any(|t| {
            [
                "not", "no", "never", "without", "dont", "don't", "cannot", "can't",
            ]
            .contains(&t.as_str())
        })
    {
        return (value.into(), None);
    }
    let families: &[&[&str]] = &[
        &["hello", "hi", "hey", "greetings"],
        &["perhaps", "maybe"],
        &["thanks", "thankyou"],
    ];
    for resource in resources
        .iter()
        .filter(|r| text(r, "subsystem") == "thesaurus")
    {
        let term = language::normalize(text(resource, "text"));
        let Some(family) = families.iter().find(|f| f.contains(&term.as_str())) else {
            continue;
        };
        if (number(resource, "valence", 0.) - number(atp, "valence", 0.)).abs() > 0.5
            || (number(resource, "intensity", 0.) - number(atp, "intensity", 0.)).abs() > 0.5
        {
            continue;
        }
        let choices = text(resource, "value")
            .split(',')
            .map(str::trim)
            .filter(|v| *v != term && family.contains(v))
            .collect::<Vec<_>>();
        if choices.is_empty() {
            continue;
        }
        let expression = regex::Regex::new(&format!(r"(?i)\b{}\b", regex::escape(&term))).unwrap();
        if let Some(found) = expression.find(value) {
            let mut replacement =
                choices[(rng.sample() * choices.len() as f64) as usize].to_owned();
            if found.as_str().starts_with(char::is_uppercase) {
                replacement.replace_range(..1, &replacement[..1].to_uppercase());
            }
            let mut result = value.to_owned();
            result.replace_range(found.range(), &replacement);
            return (
                result,
                Some(format!(
                    "conservative {tone} variation via thesaurus {}",
                    text(resource, "ref")
                )),
            );
        }
    }
    (value.into(), None)
}
pub fn cycle(
    host: &Host<'_>,
    source: &Value,
    input: &str,
    network: &Network,
    cancel: &AtomicBool,
    progress: &dyn Fn(Value),
) -> Result<(Value, Value)> {
    cycle_with_visual(host, source, input, network, cancel, progress, None)
}
pub fn cycle_with_visual(
    host: &Host<'_>,
    source: &Value,
    input: &str,
    network: &Network,
    cancel: &AtomicBool,
    progress: &dyn Fn(Value),
    visual_scores: Option<&BTreeMap<String, language::Score>>,
) -> Result<(Value, Value)> {
    validate(source)?;
    if input.trim().is_empty() || input.len() > 8000 {
        return Err(error(
            "BudgetExceeded",
            "Prompt must contain 1 to 8,000 bytes",
        ));
    }
    let start = host.env.monotonic_ms();
    let mut state = source.clone();
    let settings = state["settings"].clone();
    let mut rng = Rng(number(&settings, "seed", 104729.) as u64);
    let mut trace = Vec::new();
    let mut step = |phase: &str, detail: String, count: usize| {
        let value = json!({"id":host.env.uid(),"phase":phase,"detail":detail,"count":count});
        trace.push(value.clone());
        progress(value);
    };
    let clauses = language::clauses(input, number(&settings, "fanoutLimit", 12.) as usize);
    step(
        "Prepare",
        format!(
            "{} clauses with source spans and scoped negation",
            clauses.len()
        ),
        clauses.len(),
    );
    let resources_index = ResourceIndex::new(&state);
    let nodes = rows(&state, "nodes");
    let mut index: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    let mut binds = Vec::new();
    for (ni, n) in nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| text(n, "type") == "pattern")
    {
        for (ei, e) in rows(n, "entries").iter().enumerate() {
            index
                .entry(language::identify(text(e, "pattern")).id)
                .or_default()
                .push((ni, ei));
            if text(e, "pattern").contains('&') {
                binds.push((ni, ei));
            }
        }
    }
    let mut jobs = Vec::new();
    let mut seen = BTreeSet::new();
    for clause in &clauses {
        let mut variants = vec![clause.text.clone(), language::normalize(&clause.text)];
        // Interpret a bounded imperative utterance as its object while retaining
        // the original clause, spans and originating group for independent votes.
        let imperative = language::re(r"(?i)^(?:please\s+)?say\s+(.+)$");
        if let Some(captures) = imperative.captures(clause.text.trim()) {
            let object = captures[1].trim();
            if !language::re(r"(?i)\s+\d+\s+(?:times|\*)$").is_match(object) {
                variants.push(object.to_owned());
            }
        }
        for r in rows(&state, "resources")
            .iter()
            .filter(|r| text(r, "subsystem") == "thesaurus")
        {
            let term = text(r, "text");
            if language::normalize(&clause.text).contains(&language::normalize(term)) {
                for alternative in text(r, "value").split(',') {
                    variants.push(
                        language::normalize(&clause.text)
                            .replace(&language::normalize(term), alternative.trim()),
                    );
                }
            }
        }
        variants.truncate(number(&settings, "fanoutLimit", 12.) as usize);
        for variant in variants {
            let id = language::identify(&variant).id;
            for &(ni, ei) in index
                .get(&id)
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .chain(&binds)
            {
                if seen.insert((ni, ei, variant.clone(), clause.origin)) {
                    jobs.push((
                        ni,
                        ei,
                        variant.clone(),
                        clause.origin,
                        clause.negated,
                        String::new(),
                        0,
                    ));
                }
            }
        }
    }
    if let Some(scores) = visual_scores {
        jobs.clear();
        seen.clear();
        for (ni, node) in nodes.iter().enumerate() {
            for (ei, entry) in rows(node, "entries").iter().enumerate() {
                if scores.contains_key(text(entry, "id")) {
                    jobs.push((ni, ei, input.to_owned(), 0, false, String::new(), 0));
                }
            }
        }
    }
    let compare_entry = |entry: &Value, input: &str| -> language::Score {
        visual_scores
            .and_then(|scores| scores.get(text(entry, "id")))
            .cloned()
            .unwrap_or_else(|| language::score(text(entry, "pattern"), input))
    };
    step(
        "Retrieve",
        format!(
            "{} candidate entries; exact IDs only select candidates",
            jobs.len()
        ),
        jobs.len(),
    );
    let candidate_count = jobs.len();
    let eligible = host.search.find_all(
        jobs.len(),
        &|index| {
            let job = &jobs[index];
            compare_entry(&rows(&nodes[job.0], "entries")[job.1], &job.2).confidence
                + 2.0
                + number(&settings, "jitter", 1.5)
                >= number(&settings, "voteThreshold", 62.0)
        },
        number(&settings, "chunkSize", 256.) as usize,
        (number(&settings, "maxRafts", 8.) as usize)
            .min(number(&settings, "scanLimit", 1000.) as usize),
        cancel,
        &|covered, total| {
            progress(
                json!({"id":host.env.uid(),"phase":"Index Rafts","detail":format!("Checkpoint {covered} of {total}"),"count":covered,"covered":covered,"total":total,"checkpoint":covered}),
            )
        },
    )?;
    jobs = eligible
        .into_iter()
        .map(|index| jobs[index].clone())
        .collect();
    step(
        "Index Rafts",
        format!("{candidate_count} candidates covered by disjoint worker partitions"),
        candidate_count,
    );
    let mut votes = Vec::new();
    let mut i = 0;
    let mut scan_count = 0;
    while i < jobs.len() {
        cancelled(cancel)?;
        let (ni, ei, input, origin, negated, group, depth) = jobs[i].clone();
        i += 1;
        scan_count += 1;
        let node = &nodes[ni];
        let entry = &rows(node, "entries")[ei];
        let pattern = text(entry, "pattern");
        if negated
            && !language::clauses(pattern, 1)
                .first()
                .is_some_and(|c| c.negated)
        {
            continue;
        }
        let mut evidence = compare_entry(entry, &input);
        let base = evidence.confidence;
        let strength = number(node, "strength", 5.);
        let threshold = number(&settings, "voteThreshold", 62.);
        let max = number(&settings, "maxStrength", 10.);
        if base.abs() != 100. && !(strength == max && base >= threshold) {
            evidence.modulation = (number(&state["atp"], "intensity", 0.) * 2.).clamp(0., 2.);
        }
        let modulated = (base + evidence.modulation).clamp(-100., 100.);
        if node["jitter"] == true
            && modulated.abs() != 100.
            && !(strength == max && modulated >= threshold)
        {
            evidence.jitter = (if rng.sample() < 0.5 { -1. } else { 1. })
                * rng.sample()
                * number(&settings, "jitter", 1.5);
        }
        evidence.confidence = (modulated + evidence.jitter).clamp(-100., 100.);
        if evidence.confidence < threshold {
            continue;
        }
        let alternatives = rows(entry, "alternatives")
            .iter()
            .filter(|a| {
                text(a, "inhibition").is_empty()
                    || !language::normalize(&input)
                        .contains(&language::normalize(text(a, "inhibition")))
            })
            .cloned()
            .collect::<Vec<_>>();
        let Some(a) = pick(&alternatives, "weight", &mut rng) else {
            continue;
        };
        let resources = support(&resources_index, &input, text(a, "action"), node);
        let binding = language::bind(pattern, &input);
        votes.push(json!({"id":host.env.uid(),"nodeRef":node["ref"],"entryRef":entry["id"],"action":a["action"],"base":base,"confidence":evidence.confidence,"strength":strength,"input":input,"group":group,"origin":origin,"resources":resources.iter().map(|r|r["ref"].clone()).collect::<Vec<_>>(),"evidence":evidence,"binding":binding}));
        if depth < 4 && !binding.remainder.is_empty() {
            for attach in rows(&state, "attachments") {
                let target = if attach["from"] == node["ref"] {
                    text(attach, "to")
                } else if attach["bidirectional"] == true && attach["to"] == node["ref"] {
                    text(attach, "from")
                } else {
                    continue;
                };
                if attach["hard"] != true && rng.sample() >= number(attach, "affinity", 0.5) {
                    continue;
                }
                if let Some(ti) = nodes.iter().position(|n| text(n, "ref") == target) {
                    for (te, _) in rows(&nodes[ti], "entries").iter().enumerate() {
                        if seen.insert((ti, te, binding.remainder.clone(), origin)) {
                            jobs.push((
                                ti,
                                te,
                                binding.remainder.clone(),
                                origin,
                                negated,
                                text(attach, "id").into(),
                                depth + 1,
                            ));
                        }
                    }
                }
            }
        }
    }
    step(
        "Deep scan",
        format!("{scan_count} independent comparisons; no search truncation"),
        scan_count,
    );
    let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for v in votes {
        let key = if !text(&v, "group").is_empty() {
            text(&v, "id").to_owned()
        } else {
            format!(
                "{}:{}",
                v["origin"],
                language::normalize(text(&v, "action"))
            )
        };
        groups.entry(key).or_default().push(v);
    }
    let mut votes = Vec::new();
    for group in groups.values() {
        let weighted = group
            .iter()
            .map(|v| {
                let mut v = v.clone();
                v["bias"] =
                    json!(number(&v, "confidence", 0.).max(0.) * number(&v, "strength", 0.));
                v
            })
            .collect::<Vec<_>>();
        if let Some(v) = pick(&weighted, "bias", &mut rng) {
            votes.push(v.clone());
        }
    }
    votes.sort_by_key(|v| v["origin"].as_u64().unwrap_or(0));
    step(
        "Vote",
        format!(
            "{} contributions after same-origin arbitration",
            votes.len()
        ),
        votes.len(),
    );
    let mut segments = Vec::new();
    let mut visual = None;
    let colors = ["green", "violet", "amber", "blue", "rose", "cyan"];
    for v in &votes {
        cancelled(cancel)?;
        let node = nodes.iter().find(|n| n["ref"] == v["nodeRef"]).unwrap();
        let resources = support(&resources_index, text(v, "input"), text(v, "action"), node);
        let mut ctx = Context {
            host,
            input: text(v, "input"),
            binding: serde_json::from_value(v["binding"].clone())?,
            state: &state,
            resources: &resources,
            steps: 0,
            network,
            visual: None,
            transforms: vec!["qualified vote".into(), "scoped sigil interpreter".into()],
        };
        let ast = language::parse(text(v, "action"), Scope::NodeAction)?;
        let result = eval(&ast, &mut ctx);
        let (mut value, failed) = match result {
            Ok(x) => (x, false),
            Err(e) => (e.message, true),
        };
        if failed {
            ctx.transforms.push("action rejected safely".into());
        } else if ast.iter().all(|expr| matches!(expr, Expr::Text { .. })) {
            let (varied, evidence) = vary_literal(
                &value,
                text(node, "tone"),
                &resources,
                &state["atp"],
                &mut rng,
            );
            value = varied;
            if let Some(evidence) = evidence {
                ctx.transforms.push(evidence);
            }
        }
        if ctx.visual.is_some() {
            visual = ctx.visual;
        }
        let contributors = std::iter::once(text(node, "ref"))
            .chain(
                resources
                    .iter()
                    .filter_map(|r| r["nodeRef"].as_str())
                    .filter(|r| nodes.iter().any(|n| text(n, "ref") == *r)),
            )
            .collect::<BTreeSet<_>>();
        segments.push(json!({"id":host.env.uid(),"text":value,"contributors":contributors,"sourceVotes":[v["id"]],"transformations":ctx.transforms,"color":colors[segments.len()%6]}));
    }
    let status = if segments.is_empty() {
        if clauses.iter().all(|c| c.negated) {
            "inhibited"
        } else {
            "unmatched"
        }
    } else {
        "complete"
    };
    if segments.is_empty() {
        segments.push(json!({"id":host.env.uid(),"text":if status=="inhibited"{"That action is inhibited."}else{"No stored pattern qualified. Teach this input a response."},"contributors":[],"sourceVotes":[],"transformations":["empty-match fallback"],"color":"green"}));
    }
    step(
        "Compose",
        "Completed actions with explicit contributor lineage".into(),
        segments.len(),
    );
    let time = host.env.now_millis();
    let elapsed =
        ((time as f64 - number(&state["atp"], "lastUpdate", time as f64)) / 1000.).max(0.);
    let mut valence = number(&state["atp"], "valence", 0.) * (-elapsed / 180.).exp();
    let mut intensity = number(&state["atp"], "intensity", 0.) * (-elapsed / 180.).exp();
    let mut cooldown = number(&state["atp"], "cooldownUntil", 0.);
    for r in rows(&state, "resources").iter().filter(|r| {
        time as f64 >= cooldown
            && text(r, "subsystem") == "chargebook"
            && language::normalize(input).contains(&language::normalize(text(r, "text")))
    }) {
        valence = (valence + number(r, "valence", 0.) * 0.2).clamp(-1., 1.);
        intensity = (intensity + number(r, "intensity", 0.) * 0.2).clamp(0., 1.);
    }
    if intensity > 0.85 {
        intensity *= 0.5;
        cooldown =
            time as f64 + number(&settings, "atpCooldownSeconds", 30.).clamp(1., 3600.) * 1000.;
    }
    state["atp"] =
        json!({"valence":valence,"intensity":intensity,"lastUpdate":time,"cooldownUntil":cooldown});
    // Bounded co-activation correlation creates optional attachment edges after repeated evidence.
    if !state["correlations"].is_object() {
        state["correlations"] = json!({});
    }
    let contributors = votes
        .iter()
        .map(|v| text(v, "nodeRef").to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(8)
        .collect::<Vec<_>>();
    for pair in contributors.windows(2) {
        let key = format!("{}:{}", pair[0], pair[1]);
        let count = state["correlations"][&key]
            .as_u64()
            .unwrap_or(0)
            .saturating_add(1)
            .min(100);
        state["correlations"][&key] = json!(count);
        if count >= 3
            && rows(&state, "attachments").len() < 4096
            && !rows(&state, "attachments")
                .iter()
                .any(|a| a["from"] == pair[0] && a["to"] == pair[1])
        {
            state["attachments"].as_array_mut().unwrap().push(json!({"id":host.env.uid(),"from":pair[0],"to":pair[1],"bidirectional":false,"hard":false,"affinity":0.2,"origin":"coactivation","evidence":count}));
        }
    }

    state["settings"]["seed"] = json!((rng.0 % u32::MAX as u64).max(1));
    let mut cycle = json!({"id":host.env.uid(),"input":input,"createdAt":host.env.now_rfc3339(),"segments":segments,"votes":votes,"trace":trace,"duration":host.env.monotonic_ms()-start,"feedback":[],"status":status,"pinned":false,"supervisedUntil":time+90000,"clauses":clauses});
    if let Some(v) = visual {
        cycle["visual"] = v;
    }
    cancelled(cancel)?;
    state["history"].as_array_mut().unwrap().push(cycle.clone());
    trim(&mut state);
    append_event(
        host.env,
        &mut state,
        "cycle",
        "Native cycle completed",
        input,
    );
    Ok((state, cycle))
}
pub fn trim(s: &mut Value) {
    let limit = number(&s["settings"], "historyLimit", 100000.) as usize;
    let history = s["history"].as_array_mut().unwrap();
    let mut remove = history
        .iter()
        .filter(|h| h["pinned"] != true)
        .count()
        .saturating_sub(limit);
    history.retain(|h| {
        if remove > 0 && h["pinned"] != true {
            remove -= 1;
            false
        } else {
            true
        }
    });
}
pub fn review(
    host: &Host<'_>,
    source: &Value,
    network: &Network,
    cancel: &AtomicBool,
) -> Result<Value> {
    let mut s = source.clone();
    let now = host.env.now_millis() as f64;
    let history = rows(&s, "history").to_vec();
    let nodes = rows(&s, "nodes").to_vec();
    for n in &nodes {
        cancelled(cancel)?;
        if text(n, "type") == "A" {
            for h in history.iter().rev().take(20).filter(|h| {
                text(h, "status") == "unmatched"
                    && number(h, "supervisedUntil", 0.) >= now
                    && h["reviewed"] != true
            }) {
                let context = text(n, "contextId");
                if context.is_empty()
                    || !language::normalize(text(h, "input"))
                        .contains(&language::normalize(context))
                {
                    continue;
                }
                let mut projected = s.clone();
                projected["nodes"] = json!([{"ref":n["ref"],"name":n["name"],"type":"pattern","patternId":n["patternId"],"strength":n["strength"],"jitter":false,"entries":[{"id":rows(n,"entries")[0]["id"],"pattern":text(h,"input"),"alternatives":rows(n,"entries")[0]["alternatives"]}]}]);
                projected["attachments"] = json!([]);
                let (_, mut corrected) =
                    cycle(host, &projected, text(h, "input"), network, cancel, &|_| {})?;
                corrected["correctionOf"] = h["id"].clone();
                corrected["supervisedUntil"] = json!(0);
                corrected["reviewed"] = json!(true);
                if let Some(original) = s["history"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|x| x["id"] == h["id"])
                {
                    if original["reviewed"] == true {
                        continue;
                    }
                    original["reviewed"] = json!(true);
                }
                s["history"].as_array_mut().unwrap().push(corrected);
            }
        }
        if text(n, "type") == "B" {
            let mut counts = BTreeMap::<String, usize>::new();
            for h in history
                .iter()
                .rev()
                .take(100)
                .filter(|h| text(h, "status") == "unmatched")
            {
                *counts.entry(text(h, "input").to_owned()).or_default() += 1;
            }
            for (input, count) in counts {
                if count >= 2
                    && !rows(&s, "proposals")
                        .iter()
                        .any(|p| text(p, "pattern") == input)
                {
                    s["proposals"].as_array_mut().unwrap().push(json!({"id":host.env.uid(),"pattern":input,"evidence":count,"status":"pending","pros":["repeated unmet input"],"cons":["no grounded response yet"],"observer":n["ref"]}));
                }
            }
        }
    }
    trim(&mut s);
    append_event(
        host.env,
        &mut s,
        "context",
        "Context review completed",
        "Corrections keep original lineage; inferred knowledge remains proposed.",
    );
    Ok(s)
}
pub fn maintain(host: &Host<'_>, source: &Value, mode: &str, cancel: &AtomicBool) -> Result<Value> {
    let mut s = source.clone();
    cancelled(cancel)?;
    if mode == "phagy" {
        trim(&mut s);
        s["events"].as_array_mut().unwrap().truncate(200);
        let refs = rows(&s, "nodes")
            .iter()
            .map(|n| text(n, "ref").to_owned())
            .collect::<BTreeSet<_>>();
        s["attachments"]
            .as_array_mut()
            .unwrap()
            .retain(|a| refs.contains(text(a, "from")) && refs.contains(text(a, "to")));
        append_event(
            host.env,
            &mut s,
            "maintenance",
            "PHAGY completed",
            "Expired derived references and retention limits checked; learned knowledge and lineage retained.",
        );
        return Ok(s);
    }
    let mut rng = Rng(number(&s["settings"], "seed", 104729.) as u64);
    let nodes = rows(&s, "nodes").to_vec();
    let mut sampled = (0..nodes.len()).collect::<Vec<_>>();
    for i in (1..sampled.len()).rev() {
        let j = (rng.sample() * (i + 1) as f64) as usize;
        sampled.swap(i, j);
    }
    sampled.truncate(8);
    for &wi in &sampled {
        for &di in &sampled {
            cancelled(cancel)?;
            let w = &nodes[wi];
            let d = &nodes[di];
            if text(w, "type") != "pattern"
                || text(d, "type") != "pattern"
                || number(w, "strength", 0.) >= number(d, "strength", 0.)
                || rows(&s, "mutations").iter().any(|m| {
                    (m["recipient"] == w["ref"] && m["donor"] == d["ref"])
                        || (m["recipient"] == d["ref"] && m["donor"] == w["ref"])
                })
            {
                continue;
            }
            for (ei, e) in rows(w, "entries").iter().enumerate() {
                for donor in rows(d, "entries") {
                    if language::score(text(e, "pattern"), text(donor, "pattern")).confidence < 80.
                    {
                        continue;
                    }
                    let count = rows(e, "alternatives")
                        .len()
                        .min(rows(donor, "alternatives").len());
                    if rows(e, "alternatives")[count..]
                        .iter()
                        .any(|a| a["remixed"] == true)
                    {
                        continue;
                    }
                    for (ai, a) in rows(e, "alternatives").iter().take(count).enumerate() {
                        if a["remixed"] == true || text(a, "action").contains('&') {
                            continue;
                        }
                        let b = rows(donor, "alternatives")
                            .iter()
                            .filter(|b| {
                                !text(b, "action").contains('&')
                                    && language::score(text(a, "action"), text(b, "action"))
                                        .confidence
                                        >= 20.
                            })
                            .max_by(|left, right| {
                                language::score(text(a, "action"), text(left, "action"))
                                    .confidence
                                    .total_cmp(
                                        &language::score(text(a, "action"), text(right, "action"))
                                            .confidence,
                                    )
                            });
                        let Some(b) = b else {
                            continue;
                        };
                        let words = text(a, "action")
                            .split_whitespace()
                            .map(str::to_owned)
                            .collect::<Vec<_>>();
                        let donor_words = text(b, "action")
                            .split_whitespace()
                            .map(str::to_owned)
                            .collect::<Vec<_>>();
                        if words.len() < 3 || donor_words.len() < 3 {
                            continue;
                        }
                        let mut transitions: BTreeMap<(String, String), Vec<String>> =
                            BTreeMap::new();
                        for sentence in [&words, &donor_words] {
                            for window in sentence.windows(3) {
                                transitions
                                    .entry((window[0].clone(), window[1].clone()))
                                    .or_default()
                                    .push(window[2].clone());
                            }
                        }
                        let mut output = words[..2].to_vec();
                        for _ in 0..62 {
                            let len = output.len();
                            let Some(next) = transitions
                                .get(&(output[len - 2].clone(), output[len - 1].clone()))
                            else {
                                break;
                            };
                            output.push(next[(rng.sample() * next.len() as f64) as usize].clone());
                        }
                        let action = output.join(" ");
                        if action == text(a, "action")
                            || action == text(b, "action")
                            || action.len() > 8000
                        {
                            continue;
                        }
                        language::parse(&action, Scope::NodeAction)?;
                        let slot = &mut s["nodes"][wi]["entries"][ei]["alternatives"][ai];
                        slot["action"] = json!(action);
                        slot["remixed"] = json!(true);
                        if rng.sample() < 0.5 {
                            slot["weight"] = b["weight"].clone();
                        }
                        s["nodes"][wi]["entries"][ei]["alternatives"]
                            .as_array_mut()
                            .unwrap()
                            .truncate(count);
                        s["mutations"].as_array_mut().unwrap().push(json!({"id":host.env.uid(),"recipient":w["ref"],"donor":d["ref"],"slot":a["id"],"originalPattern":e["pattern"],"originalAction":a["action"],"createdAt":host.env.now_rfc3339()}));
                        validate(&s)?;
                        append_event(
                            host.env,
                            &mut s,
                            "mutation",
                            "Markov remix committed",
                            "Pattern and vote comparisons passed; lifetime slot marker and original-pair inhibition committed atomically.",
                        );
                        return Ok(s);
                    }
                }
            }
        }
    }
    append_event(
        host.env,
        &mut s,
        "maintenance",
        "Mutation scan completed",
        "No valid mutation; no alternative pools were shortened.",
    );
    Ok(s)
}
#[cfg(test)]
mod host_tests {
    use super::*;
    use crate::host::FixedHost;
    use std::sync::atomic::AtomicBool;
    fn starter() -> Value {
        serde_json::from_str(include_str!("../../../conformance/starter.json")).unwrap()
    }
    fn run_once() -> String {
        let fixed = FixedHost::default();
        let (state, cycle) = cycle(
            &fixed.host(),
            &starter(),
            "What is 2 + 2?",
            &Network::default(),
            &AtomicBool::new(false),
            &|_| {},
        )
        .unwrap();
        serde_json::to_string(&(state, cycle)).unwrap()
    }
    #[test]
    fn fixed_host_cycle_is_byte_deterministic() {
        assert_eq!(run_once(), run_once());
    }
    #[test]
    fn gpu_setting_without_accelerator_runs_on_cpu() {
        let mut s = starter();
        s["settings"]["gpu"] = serde_json::json!(true);
        let fixed = FixedHost::default();
        assert!(
            cycle(
                &fixed.host(),
                &s,
                "What is 2 + 2?",
                &Network::default(),
                &AtomicBool::new(false),
                &|_| {},
            )
            .is_ok()
        );
    }
}
