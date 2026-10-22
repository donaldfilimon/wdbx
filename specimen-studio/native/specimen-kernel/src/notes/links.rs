//! Wiki links inside note bodies. Code spans and fenced code stay literal
//! because only Markdown text events are scanned.
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Target {
    Note { title: String },
    Node { node_ref: String },
    Pattern { id: String },
    Memory { resource_ref: String },
}

impl Target {
    pub fn key(&self) -> String {
        match self {
            Self::Note { title } => format!("note:{title}"),
            Self::Node { node_ref } => format!("node:{node_ref}"),
            Self::Pattern { id } => format!("pattern:{id}"),
            Self::Memory { resource_ref } => format!("memory:{resource_ref}"),
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Note { .. } => "note",
            Self::Node { .. } => "node",
            Self::Pattern { .. } => "pattern",
            Self::Memory { .. } => "memory",
        }
    }

    pub fn raw(&self) -> &str {
        match self {
            Self::Note { title } => title,
            Self::Node { node_ref } => node_ref,
            Self::Pattern { id } => id,
            Self::Memory { resource_ref } => resource_ref,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wikilink {
    pub target: Target,
    pub label: String,
}

const MARK_START: char = '\u{E000}';
const MARK_END: char = '\u{E001}';

/// Replaces wiki links outside code with private-use placeholders.
///
/// CommonMark treats `[[Title]]` as a reference link, so the brackets never
/// arrive as text. Placeholders stay in text events, and code is untouched.
pub fn protect(body: &str) -> (String, Vec<Wikilink>) {
    let mut out = String::with_capacity(body.len());
    let mut links = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for line in body.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        if let Some((mark, width)) = fence {
            out.push_str(line);
            if closing_fence(content, mark, width) {
                fence = None;
            }
            continue;
        }
        if let Some((mark, width)) = opening_fence(content) {
            fence = Some((mark, width));
            out.push_str(line);
            continue;
        }
        protect_inline(line, &mut out, &mut links);
    }
    (out, links)
}

/// Links that survive as Markdown text: not in code, and not inside raw HTML.
pub fn links_in(body: &str) -> Vec<Wikilink> {
    let (protected, table) = protect(body);
    let mut links = Vec::new();
    let mut code = 0u32;
    let mut html = 0u32;
    for event in Parser::new(&protected) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code += 1,
            Event::End(TagEnd::CodeBlock) => code = code.saturating_sub(1),
            Event::Start(Tag::HtmlBlock) => html += 1,
            Event::End(TagEnd::HtmlBlock) => html = html.saturating_sub(1),
            Event::Text(text) if code == 0 && html == 0 => {
                links.extend(placeholders(&text, &table).into_iter().cloned());
            }
            _ => {}
        }
    }
    links
}

pub fn placeholders<'a>(text: &str, table: &'a [Wikilink]) -> Vec<&'a Wikilink> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(MARK_START) {
        let after = &rest[start + MARK_START.len_utf8()..];
        let Some(end) = after.find(MARK_END) else {
            break;
        };
        if let Ok(index) = after[..end].parse::<usize>()
            && let Some(link) = table.get(index)
        {
            found.push(link);
        }
        rest = &after[end + MARK_END.len_utf8()..];
    }
    found
}

pub fn placeholder(index: usize) -> String {
    format!("{MARK_START}{index}{MARK_END}")
}

pub fn parse_inside(inside: &str) -> Option<Wikilink> {
    if inside.is_empty() || inside.contains('\n') {
        return None;
    }
    let (raw, label) = match inside.split_once('|') {
        Some((target, label)) => (target.trim(), label.trim()),
        None => {
            let trimmed = inside.trim();
            (trimmed, trimmed)
        }
    };
    if raw.is_empty() {
        return None;
    }
    let target = if let Some(rest) = raw.strip_prefix("node:") {
        if rest.is_empty() {
            return None;
        }
        Target::Node {
            node_ref: rest.to_owned(),
        }
    } else if let Some(rest) = raw.strip_prefix("pattern:") {
        if rest.is_empty() {
            return None;
        }
        Target::Pattern {
            id: rest.to_owned(),
        }
    } else if let Some(rest) = raw.strip_prefix("memory:") {
        if rest.is_empty() {
            return None;
        }
        Target::Memory {
            resource_ref: rest.to_owned(),
        }
    } else {
        Target::Note {
            title: raw.to_owned(),
        }
    };
    let label = if label.is_empty() {
        raw.to_owned()
    } else {
        label.to_owned()
    };
    Some(Wikilink { target, label })
}

/// Replaces exact `[[old]]` and `[[old|label]]` outside code. A longer title stays.
pub fn rewrite_title(body: &str, old: &str, new: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut fence: Option<(char, usize)> = None;
    for line in body.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        if let Some((mark, width)) = fence {
            out.push_str(line);
            if closing_fence(content, mark, width) {
                fence = None;
            }
            continue;
        }
        if let Some((mark, width)) = opening_fence(content) {
            fence = Some((mark, width));
            out.push_str(line);
            continue;
        }
        rewrite_inline(line, old, new, &mut out);
    }
    out
}

fn protect_inline(line: &str, out: &mut String, links: &mut Vec<Wikilink>) {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'`' {
            let width = run(bytes, index, b'`');
            if let Some(end) = find_run(bytes, index + width, b'`', width) {
                out.push_str(&line[index..end + width]);
                index = end + width;
                continue;
            }
        }
        if bytes[index] == b'['
            && index + 1 < bytes.len()
            && bytes[index + 1] == b'['
            && let Some(end) = line[index + 2..].find("]]")
        {
            let inside = &line[index + 2..index + 2 + end];
            if !inside.contains('\n')
                && let Some(link) = parse_inside(inside)
            {
                out.push_str(&placeholder(links.len()));
                links.push(link);
                index += 2 + end + 2;
                continue;
            }
        }
        let ch = line[index..].chars().next().unwrap_or(' ');
        out.push(ch);
        index += ch.len_utf8();
    }
}

fn rewrite_inline(line: &str, old: &str, new: &str, out: &mut String) {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'`' {
            let width = run(bytes, index, b'`');
            if let Some(end) = find_run(bytes, index + width, b'`', width) {
                out.push_str(&line[index..end + width]);
                index = end + width;
                continue;
            }
        }
        if bytes[index] == b'['
            && index + 1 < bytes.len()
            && bytes[index + 1] == b'['
            && let Some(end) = line[index + 2..].find("]]")
        {
            let inside = &line[index + 2..index + 2 + end];
            let title = inside
                .split_once('|')
                .map(|(title, _)| title)
                .unwrap_or(inside);
            if title == old {
                out.push_str("[[");
                out.push_str(new);
                if let Some((_, label)) = inside.split_once('|') {
                    out.push('|');
                    out.push_str(label);
                }
                out.push_str("]]");
                index += 2 + end + 2;
                continue;
            }
        }
        let ch = line[index..].chars().next().unwrap_or(' ');
        out.push(ch);
        index += ch.len_utf8();
    }
}

fn run(bytes: &[u8], start: usize, mark: u8) -> usize {
    bytes[start..]
        .iter()
        .take_while(|byte| **byte == mark)
        .count()
}

fn find_run(bytes: &[u8], start: usize, mark: u8, width: usize) -> Option<usize> {
    let mut index = start;
    while index < bytes.len() {
        if bytes[index] == mark {
            let found = run(bytes, index, mark);
            if found == width {
                return Some(index);
            }
            index += found;
            continue;
        }
        index += 1;
    }
    None
}

fn opening_fence(line: &str) -> Option<(char, usize)> {
    let rest = line.trim_start_matches(' ');
    if line.len() - rest.len() > 3 {
        return None;
    }
    let mark = rest.chars().next()?;
    if mark != '`' && mark != '~' {
        return None;
    }
    let width = rest.chars().take_while(|ch| *ch == mark).count();
    if width < 3 {
        return None;
    }
    Some((mark, width))
}

fn closing_fence(line: &str, mark: char, width: usize) -> bool {
    let trimmed = line.trim();
    let count = trimmed.chars().take_while(|ch| *ch == mark).count();
    count >= width && trimmed.chars().all(|ch| ch == mark)
}
