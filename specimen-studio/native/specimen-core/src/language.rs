use crate::{Result, error};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clause {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub negated: bool,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub origin: usize,
}
pub fn re(s: &str) -> Regex {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::BTreeMap<String, Regex>>> =
        std::sync::OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    cache
        .entry(s.to_owned())
        .or_insert_with(|| Regex::new(s).expect("static regular expression"))
        .clone()
}
pub fn normalize(s: &str) -> String {
    let s = s
        .to_lowercase()
        .replace('’', "'")
        .replace("multiplied by", "*")
        .replace("divided by", "/");
    let s=re(r"\b(zero|one|two|three|four|five|six|seven|eight|nine|ten|plus|minus|times|equals|hi|hey|greetings)\b").replace_all(&s, |c:&regex::Captures<'_>| match &c[0] {"zero"=>"0","one"=>"1","two"=>"2","three"=>"3","four"=>"4","five"=>"5","six"=>"6","seven"=>"7","eight"=>"8","nine"=>"9","ten"=>"10","plus"=>"+","minus"=>"-","times"=>"*","equals"=>"=",_=>"hello"}).to_string();
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
pub fn tokens(s: &str) -> Vec<String> {
    re(r"&\w+|[\p{L}\p{N}_]+|[+*/^%=()-]")
        .find_iter(&normalize(s))
        .map(|m| m.as_str().to_owned())
        .collect()
}
pub fn arithmetic_input(s: &str) -> String {
    re(r"\bwhat(?: is|'s)|\bcalculate|\bcompute|\bsolve|\bplease|\bthe result of|\bequal to")
        .replace_all(&normalize(s), "")
        .replace(['?', '='], "")
        .trim()
        .to_owned()
}
pub fn clauses(s: &str, limit: usize) -> Vec<Clause> {
    let split = re(r"(?i);|\s+(?:and then|then|and|but)\s+");
    let mut out = Vec::new();
    let mut start = 0;
    for end in split
        .find_iter(s)
        .map(|m| (m.start(), m.end()))
        .chain(std::iter::once((s.len(), s.len())))
    {
        let raw = &s[start..end.0];
        let t = raw.trim();
        let offset = raw.len() - raw.trim_start().len();
        if !t.is_empty() && out.len() < limit {
            let words = t.split_whitespace().collect::<Vec<_>>();
            let negated = re(r"\b(don't|do not|never|not)\b").is_match(&normalize(t));
            out.push(Clause {
                text: t.into(),
                start: start + offset,
                end: start + offset + t.len(),
                negated,
                subject: "user".into(),
                predicate: words.first().unwrap_or(&"").to_string(),
                object: words.iter().skip(1).copied().collect::<Vec<_>>().join(" "),
                origin: out.len(),
            });
        }
        start = end.1;
    }
    out
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternId {
    pub id: String,
    pub resolution: String,
    pub features: String,
}
pub fn identify(s: &str) -> PatternId {
    let t = tokens(s);
    let n = normalize(s);
    let a = arithmetic_input(s);
    let features = if n == "&n &op &n"
        || (!a.is_empty()
            && re(r"^[\d.\s()+*/%^\-]+$").is_match(&a)
            && re(r"[+*/%^]|\d\s*-").is_match(&a))
    {
        "arithmetic".into()
    } else if n.starts_with("imagine ") {
        "imagine".into()
    } else if n.starts_with("say ") && n.ends_with(" *") {
        "repeat".into()
    } else {
        let mut feature_tokens = t
            .iter()
            .map(|w| {
                if w.chars().all(|x| x.is_numeric()) {
                    "#".into()
                } else {
                    w.clone()
                }
            })
            .collect::<Vec<String>>();
        feature_tokens.sort();
        feature_tokens.dedup();
        feature_tokens.join(" ")
    };
    // Complexity belongs to the source structure; short arithmetic binds share their low-resolution class.
    let resolution = if t.len() <= 6 {
        "low"
    } else if t.len() <= 16 {
        "medium"
    } else {
        "high"
    };
    PatternId {
        id: format!(
            "text-v2:{resolution}:{}",
            &crate::digest(features.as_bytes())[..24]
        ),
        resolution: resolution.into(),
        features,
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Binding {
    pub matched: bool,
    pub numbers: Vec<String>,
    pub text: String,
    pub operator: String,
    pub remainder: String,
}
pub fn bind(pattern: &str, input: &str) -> Binding {
    let p = normalize(pattern);
    if let Some(code) = pattern
        .strip_prefix("&automata(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return Binding {
            matched: tape(code, input).is_ok_and(|s| s.bytes().any(|b| b != 0)),
            ..Default::default()
        };
    }
    let raw = normalize(input);
    let text = if p == "&n &op &n" {
        arithmetic_input(input)
    } else {
        raw.clone()
    };
    let mut expression = String::new();
    let mut pos = 0;
    let mut kinds = Vec::new();
    let sigils = re(r"&(?:n\b|text\b|op\b|equals\b|\*?[\p{L}\p{N}_]+\*)");
    for m in sigils.find_iter(&p) {
        expression.push_str(&regex::escape(&p[pos..m.start()]).replace(" ", r"\s*"));
        let kind = m.as_str();
        kinds.push(kind.to_owned());
        expression.push_str(match kind {
            "&n" => r"([-+]?\d+(?:\.\d+)?)",
            "&op" => r"([+*/%^\-])",
            "&equals" => r"(=)",
            "&text" => r"(.+?)",
            _ => r"(.*?)",
        });
        if kind.ends_with('*') {
            expression.push_str(&regex::escape(kind.trim_matches(['&', '*'])));
            expression.push_str(".*?");
        }
        pos = m.end();
    }
    expression.push_str(&regex::escape(&p[pos..]).replace(" ", r"\s*"));
    if kinds.is_empty() {
        return Binding {
            matched: p == raw,
            ..Default::default()
        };
    }
    let Ok(regex) = Regex::new(&format!("^{expression}$")) else {
        return Binding::default();
    };
    let Some(c) = regex.captures(&text) else {
        return Binding::default();
    };
    let mut b = Binding {
        matched: true,
        ..Default::default()
    };
    for (i, k) in kinds.iter().enumerate() {
        let value = c.get(i + 1).map_or("", |m| m.as_str()).to_owned();
        match k.as_str() {
            "&n" => b.numbers.push(value),
            "&op" => b.operator = value,
            "&text" => {
                b.text = value.clone();
                b.remainder = value;
            }
            _ => b.remainder = value,
        }
    }
    b
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Score {
    pub similarity: f64,
    pub dissimilarity: f64,
    pub modulation: f64,
    pub jitter: f64,
    pub confidence: f64,
}
pub fn score(pattern: &str, input: &str) -> Score {
    let a = tokens(pattern).into_iter().collect::<BTreeSet<_>>();
    let b = tokens(input).into_iter().collect::<BTreeSet<_>>();
    let union = a.union(&b).count();
    let similarity = if bind(pattern, input).matched {
        100.0
    } else if union == 0 {
        0.0
    } else {
        100.0 * a.intersection(&b).count() as f64 / union as f64
    };
    let dissimilarity = 100.0 - similarity;
    Score {
        similarity,
        dissimilarity,
        modulation: 0.0,
        jitter: 0.0,
        confidence: (similarity - dissimilarity).clamp(-100.0, 100.0),
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    RelationalTriple,
    PatternBind,
    NodeAction,
    Vote,
    Orchestration,
    Reserved,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Expr {
    Text { value: String },
    Call { name: String, args: Vec<Vec<Expr>> },
}
#[derive(Debug, Serialize)]
pub struct Contact {
    pub name: &'static str,
    pub scopes: Vec<Scope>,
    pub reads: Vec<&'static str>,
    pub writes: Vec<&'static str>,
}
pub fn ledger() -> Vec<Contact> {
    [
        "current_input",
        "n",
        "text",
        "op",
        "equals",
        "calc",
        "combineSlots",
        "repeat",
        "automata",
        "time",
        "recall",
        "tone",
        "imagine",
        "LookUp",
        "memory",
        "literal",
    ]
    .iter()
    .map(|&name| Contact {
        name,
        scopes: match name {
            "n" | "text" | "op" | "equals" => vec![Scope::PatternBind, Scope::NodeAction],
            "current_input" => vec![
                Scope::RelationalTriple,
                Scope::PatternBind,
                Scope::NodeAction,
                Scope::Vote,
                Scope::Orchestration,
            ],
            "LookUp" | "automata" => {
                vec![Scope::PatternBind, Scope::NodeAction, Scope::Orchestration]
            }
            _ => vec![Scope::NodeAction, Scope::Orchestration],
        },
        reads: match name {
            "LookUp" | "memory" | "recall" => vec!["resources", "history"],
            "tone" => vec!["atp"],
            _ => vec!["cycle"],
        },
        writes: vec![],
    })
    .collect()
}
pub fn parse(s: &str, scope: Scope) -> Result<Vec<Expr>> {
    if s.len() > 8000 {
        return Err(error("BudgetExceeded", "Action exceeds 8,000 bytes"));
    }
    fn inner(s: &str, scope: Scope, depth: usize) -> Result<Vec<Expr>> {
        if depth > 16 {
            return Err(error("BudgetExceeded", "Sigil nesting exceeds 16"));
        }
        let mut out = Vec::new();
        let mut pos = 0;
        while let Some(relative) = s[pos..].find('&') {
            let start = pos + relative;
            if start > pos {
                out.push(Expr::Text {
                    value: s[pos..start].into(),
                });
            }
            let rest = &s[start + 1..];
            let len = rest
                .bytes()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == b'_')
                .count();
            if len == 0 {
                return Err(error(
                    "InvalidScope",
                    "An ampersand must start a registered sigil",
                ));
            }
            let name = &rest[..len];
            if !ledger()
                .iter()
                .any(|x| x.name.eq_ignore_ascii_case(name) && x.scopes.contains(&scope))
            {
                return Err(error(
                    "InvalidScope",
                    format!("{name} is not allowed in {scope:?}"),
                ));
            }
            let mut at = start + 1 + len;
            let mut args = Vec::new();
            if s.as_bytes().get(at) == Some(&b'(') {
                at += 1;
                let begin = at;
                let mut level = 1;
                let mut quote = None;
                let mut split = begin;
                let mut escape = false;
                while at < s.len() {
                    let ch = s.as_bytes()[at];
                    if escape {
                        escape = false;
                        at += 1;
                        continue;
                    }
                    if ch == b'\\' && quote.is_some() {
                        escape = true;
                        at += 1;
                        continue;
                    }
                    if let Some(q) = quote {
                        if ch == q {
                            quote = None;
                        }
                    } else if ch == b'\'' || ch == b'"' {
                        quote = Some(ch);
                    } else if ch == b'(' {
                        level += 1;
                    } else if ch == b')' {
                        level -= 1;
                        if level == 0 {
                            if name.eq_ignore_ascii_case("literal") {
                                let value: String = serde_json::from_str(s[begin..at].trim())
                                    .map_err(|_| {
                                        error("MalformedAction", "literal requires a JSON string")
                                    })?;
                                args.push(vec![Expr::Text { value }]);
                            } else {
                                args.push(inner(
                                    s[split..at].trim().trim_matches(['\'', '"']),
                                    scope,
                                    depth + 1,
                                )?);
                            }
                            break;
                        }
                    } else if ch == b',' && level == 1 && !name.eq_ignore_ascii_case("literal") {
                        args.push(inner(
                            s[split..at].trim().trim_matches(['\'', '"']),
                            scope,
                            depth + 1,
                        )?);
                        split = at + 1;
                    }
                    at += 1;
                }
                if level != 0 || quote.is_some() {
                    return Err(error("MalformedAction", "Unclosed sigil argument"));
                }
                at += 1;
            } else if s.as_bytes().get(at) == Some(&b'&') {
                at += 1;
            }
            out.push(Expr::Call {
                name: name.to_owned(),
                args,
            });
            pos = at;
        }
        if pos < s.len() {
            out.push(Expr::Text {
                value: s[pos..].into(),
            });
        }
        Ok(out)
    }
    inner(s, scope, 0)
}
pub fn calculate(input: &str) -> Result<f64> {
    let text = arithmetic_input(input);
    if text.len() > 2000 {
        return Err(error("BudgetExceeded", "Calculation exceeds 2,000 bytes"));
    }
    let mut t = Vec::new();
    let mut at = 0;
    let bytes = text.as_bytes();
    while at < bytes.len() {
        let c = bytes[at];
        if c.is_ascii_whitespace() {
            at += 1;
            continue;
        }
        if c.is_ascii_digit() || c == b'.' {
            let start = at;
            at += 1;
            while at < bytes.len() && (bytes[at].is_ascii_digit() || bytes[at] == b'.') {
                at += 1;
            }
            t.push(text[start..at].to_owned());
        } else if b"()+-*/%^".contains(&c) {
            t.push((c as char).to_string());
            at += 1;
        } else {
            return Err(error("MalformedAction", "Invalid arithmetic character"));
        }
    }
    fn expr(t: &[String], i: &mut usize, min: u8, steps: &mut usize) -> Result<f64> {
        *steps += 1;
        if *steps > 500 {
            return Err(error("BudgetExceeded", "Calculation is too complex"));
        }
        let token = t
            .get(*i)
            .ok_or_else(|| error("MalformedAction", "Missing number"))?;
        *i += 1;
        let mut left = match token.as_str() {
            "(" => {
                let n = expr(t, i, 0, steps)?;
                if t.get(*i).map(String::as_str) != Some(")") {
                    return Err(error("MalformedAction", "Missing closing parenthesis"));
                }
                *i += 1;
                n
            }
            "-" => -expr(t, i, 4, steps)?,
            "+" => expr(t, i, 4, steps)?,
            _ => token
                .parse::<f64>()
                .map_err(|_| error("MalformedAction", "Invalid number"))?,
        };
        while let Some(op) = t.get(*i) {
            let prec = match op.as_str() {
                "+" | "-" => 1,
                "*" | "/" | "%" => 2,
                "^" => 3,
                _ => 0,
            };
            if prec == 0 || prec < min {
                break;
            }
            *i += 1;
            let right = expr(t, i, prec + u8::from(op != "^"), steps)?;
            if (op == "/" || op == "%") && right == 0.0 {
                return Err(error("MalformedAction", "Division by zero"));
            }
            left = match op.as_str() {
                "+" => left + right,
                "-" => left - right,
                "*" => left * right,
                "/" => left / right,
                "%" => left % right,
                _ => left.powf(right),
            };
            if !left.is_finite() {
                return Err(error("BudgetExceeded", "Nonfinite arithmetic result"));
            }
        }
        Ok(left)
    }
    let mut i = 0;
    let value = expr(&t, &mut i, 0, &mut 0)?;
    if i != t.len() {
        return Err(error("MalformedAction", "Unexpected arithmetic token"));
    }
    Ok(value)
}
pub fn tape(code: &str, input: &str) -> Result<String> {
    let ops = code
        .bytes()
        .filter(|b| b"><+-.,[]".contains(b))
        .collect::<Vec<_>>();
    let mut jumps = vec![0; ops.len()];
    let mut stack = Vec::new();
    for (i, c) in ops.iter().enumerate() {
        if *c == b'[' {
            stack.push(i);
        } else if *c == b']' {
            let j = stack
                .pop()
                .ok_or_else(|| error("MalformedAction", "Unmatched tape bracket"))?;
            jumps[i] = j;
            jumps[j] = i;
        }
    }
    if !stack.is_empty() {
        return Err(error("MalformedAction", "Unmatched tape bracket"));
    }
    let mut cells = [0u8; 1024];
    let (mut pc, mut ptr, mut read, mut steps) = (0, 0, 0, 0);
    let mut out = String::new();
    while pc < ops.len() {
        steps += 1;
        if steps > 10000 {
            return Err(error(
                "BudgetExceeded",
                "Tape exhausted 10,000 instructions",
            ));
        }
        match ops[pc] {
            b'>' => ptr = (ptr + 1) % 1024,
            b'<' => ptr = (ptr + 1023) % 1024,
            b'+' => cells[ptr] = cells[ptr].wrapping_add(1),
            b'-' => cells[ptr] = cells[ptr].wrapping_sub(1),
            b'.' => out.push(cells[ptr] as char),
            b',' => {
                cells[ptr] = *input.as_bytes().get(read).unwrap_or(&0);
                read += 1;
            }
            b'[' if cells[ptr] == 0 => pc = jumps[pc],
            b']' if cells[ptr] != 0 => pc = jumps[pc],
            _ => {}
        }
        pc += 1;
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collision_is_not_confidence() {
        assert_eq!(identify("2+2").id, identify("9+9").id);
        assert!(score("2+2", "9+9").confidence < 62.0);
        assert_eq!(score("&n &op &n", "What is 2+2?").confidence, 100.0);
    }
    #[test]
    fn independent_negation() {
        let c = clauses("don't calculate 2+2; say hello", 12);
        assert!(c[0].negated);
        assert!(!c[1].negated);
        assert_eq!(
            &"don't calculate 2+2; say hello"[c[1].start..c[1].end],
            "say hello"
        );
    }
    #[test]
    fn scope_and_nesting() {
        assert!(parse("&calc(2+&calc(1+2))", Scope::NodeAction).is_ok());
        assert!(parse("&calc(1)", Scope::PatternBind).is_err());
        assert!(parse("&unknown()", Scope::NodeAction).is_err());
        assert!(parse("&calc(1", Scope::NodeAction).is_err());
    }
    #[test]
    fn bounded_math() {
        assert_eq!(calculate("2^3^2").unwrap(), 512.0);
        assert!(calculate("1/0").is_err());
        assert!(tape("+[+]", "").is_ok());
        assert!(tape("+[]", "").is_err());
    }
}
