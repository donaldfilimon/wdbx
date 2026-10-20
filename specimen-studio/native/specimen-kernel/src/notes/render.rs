//! Markdown blocks for both editions. Raw HTML events are dropped, so the
//! render never carries a tag from the note body.
use super::links::{self, Target, Wikilink};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Block {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Paragraph {
        inlines: Vec<Inline>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Inline>>,
    },
    Code {
        text: String,
    },
    Quote {
        inlines: Vec<Inline>,
    },
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Inline {
    Text {
        text: String,
    },
    Code {
        text: String,
    },
    Em {
        inlines: Vec<Inline>,
    },
    Strong {
        inlines: Vec<Inline>,
    },
    Break,
    Link {
        label: String,
        href: String,
        kind: String,
        target: String,
        resolved: bool,
    },
}

enum Stop {
    Paragraph,
    Heading(HeadingLevel),
    Emphasis,
    Strong,
    Link,
    Image,
    Strike,
}

pub fn render(body: &str, resolve: impl Fn(&Target) -> bool) -> Vec<Block> {
    let (protected, links) = links::protect(body);
    let events: Vec<Event<'_>> = Parser::new(&protected).collect();
    let mut blocks = Vec::new();
    let mut index = 0;
    while index < events.len() {
        match &events[index] {
            Event::Start(Tag::Paragraph) => {
                index += 1;
                let (inlines, next) =
                    read_inlines(&events, index, &Stop::Paragraph, &links, &resolve);
                index = next;
                blocks.push(Block::Paragraph { inlines });
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let stop = Stop::Heading(*level);
                let level = *level as u8;
                index += 1;
                let (inlines, next) = read_inlines(&events, index, &stop, &links, &resolve);
                index = next;
                blocks.push(Block::Heading { level, inlines });
            }
            Event::Start(Tag::CodeBlock(_)) => {
                index += 1;
                let (text, next) = read_code(&events, index);
                index = next;
                blocks.push(Block::Code { text });
            }
            Event::Start(Tag::List(start)) => {
                let ordered = start.is_some();
                index += 1;
                let (items, next) = read_list(&events, index, &links, &resolve);
                index = next;
                blocks.push(Block::List { ordered, items });
            }
            Event::Start(Tag::BlockQuote(_)) => {
                index += 1;
                let (inlines, next) = read_quote(&events, index, &links, &resolve);
                index = next;
                blocks.push(Block::Quote { inlines });
            }
            Event::Start(Tag::HtmlBlock) => index = skip_balanced(&events, index),
            Event::Rule => {
                index += 1;
                blocks.push(Block::Rule);
            }
            Event::Html(_) | Event::InlineHtml(_) => index += 1,
            _ => index += 1,
        }
    }
    blocks
}

fn read_inlines(
    events: &[Event<'_>],
    mut index: usize,
    stop: &Stop,
    links: &[Wikilink],
    resolve: &impl Fn(&Target) -> bool,
) -> (Vec<Inline>, usize) {
    let mut buf = Vec::new();
    while index < events.len() {
        if let Event::End(tag) = &events[index]
            && stops(tag, stop)
        {
            return (buf, index + 1);
        }
        match &events[index] {
            Event::Text(text) => {
                push_text(&mut buf, text, links, resolve);
                index += 1;
            }
            Event::Code(text) => {
                buf.push(Inline::Code {
                    text: text.to_string(),
                });
                index += 1;
            }
            Event::SoftBreak | Event::HardBreak => {
                buf.push(Inline::Break);
                index += 1;
            }
            Event::Html(_) | Event::InlineHtml(_) => index += 1,
            Event::Start(Tag::Emphasis) => {
                index += 1;
                let (inlines, next) = read_inlines(events, index, &Stop::Emphasis, links, resolve);
                index = next;
                buf.push(Inline::Em { inlines });
            }
            Event::Start(Tag::Strong) => {
                index += 1;
                let (inlines, next) = read_inlines(events, index, &Stop::Strong, links, resolve);
                index = next;
                buf.push(Inline::Strong { inlines });
            }
            Event::Start(Tag::Strikethrough) => {
                index += 1;
                let (inlines, next) = read_inlines(events, index, &Stop::Strike, links, resolve);
                index = next;
                buf.extend(inlines);
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                let href = dest_url.to_string();
                index += 1;
                let (inlines, next) = read_inlines(events, index, &Stop::Link, links, resolve);
                index = next;
                buf.push(Inline::Link {
                    label: plain(&inlines),
                    target: href.clone(),
                    href,
                    kind: "url".into(),
                    resolved: true,
                });
            }
            Event::Start(Tag::Image { .. }) => {
                index += 1;
                let (inlines, next) = read_inlines(events, index, &Stop::Image, links, resolve);
                index = next;
                buf.extend(inlines);
            }
            _ => index += 1,
        }
    }
    (buf, index)
}

fn stops(tag: &TagEnd, stop: &Stop) -> bool {
    match (tag, stop) {
        (TagEnd::Paragraph, Stop::Paragraph)
        | (TagEnd::Emphasis, Stop::Emphasis)
        | (TagEnd::Strong, Stop::Strong)
        | (TagEnd::Link, Stop::Link)
        | (TagEnd::Image, Stop::Image)
        | (TagEnd::Strikethrough, Stop::Strike) => true,
        (TagEnd::Heading(level), Stop::Heading(want)) => level == want,
        _ => false,
    }
}

fn read_code(events: &[Event<'_>], mut index: usize) -> (String, usize) {
    let mut text = String::new();
    while index < events.len() {
        match &events[index] {
            Event::End(TagEnd::CodeBlock) => return (text, index + 1),
            Event::Text(part) => text.push_str(part),
            Event::SoftBreak | Event::HardBreak => text.push('\n'),
            _ => {}
        }
        index += 1;
    }
    (text, index)
}

fn read_list(
    events: &[Event<'_>],
    mut index: usize,
    links: &[Wikilink],
    resolve: &impl Fn(&Target) -> bool,
) -> (Vec<Vec<Inline>>, usize) {
    let mut items = Vec::new();
    while index < events.len() {
        match &events[index] {
            Event::End(TagEnd::List(_)) => return (items, index + 1),
            Event::Start(Tag::Item) => {
                index += 1;
                let (item, next) = read_item(events, index, links, resolve);
                index = next;
                items.push(item);
            }
            _ => index += 1,
        }
    }
    (items, index)
}

fn read_item(
    events: &[Event<'_>],
    mut index: usize,
    links: &[Wikilink],
    resolve: &impl Fn(&Target) -> bool,
) -> (Vec<Inline>, usize) {
    let mut buf = Vec::new();
    let mut depth = 1u32;
    while index < events.len() {
        match &events[index] {
            Event::Start(Tag::Item) => {
                depth += 1;
                index += 1;
            }
            Event::End(TagEnd::Item) => {
                depth -= 1;
                index += 1;
                if depth == 0 {
                    return (buf, index);
                }
            }
            Event::Start(Tag::Paragraph) if depth == 1 => {
                index += 1;
                let (inlines, next) = read_inlines(events, index, &Stop::Paragraph, links, resolve);
                index = next;
                if !buf.is_empty() {
                    buf.push(Inline::Break);
                }
                buf.extend(inlines);
            }
            _ => index += 1,
        }
    }
    (buf, index)
}

fn read_quote(
    events: &[Event<'_>],
    mut index: usize,
    links: &[Wikilink],
    resolve: &impl Fn(&Target) -> bool,
) -> (Vec<Inline>, usize) {
    let mut buf = Vec::new();
    let mut depth = 1u32;
    while index < events.len() {
        match &events[index] {
            Event::Start(Tag::BlockQuote(_)) => {
                depth += 1;
                index += 1;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                depth -= 1;
                index += 1;
                if depth == 0 {
                    return (buf, index);
                }
            }
            Event::Start(Tag::Paragraph) if depth == 1 => {
                index += 1;
                let (inlines, next) = read_inlines(events, index, &Stop::Paragraph, links, resolve);
                index = next;
                if !buf.is_empty() {
                    buf.push(Inline::Break);
                }
                buf.extend(inlines);
            }
            Event::Html(_) | Event::InlineHtml(_) => index += 1,
            _ => index += 1,
        }
    }
    (buf, index)
}

fn skip_balanced(events: &[Event<'_>], start: usize) -> usize {
    let mut index = start + 1;
    let mut depth = 1u32;
    while index < events.len() && depth > 0 {
        match &events[index] {
            Event::Start(_) => depth += 1,
            Event::End(_) => depth -= 1,
            _ => {}
        }
        index += 1;
    }
    index
}

fn push_text(
    buf: &mut Vec<Inline>,
    text: &str,
    links: &[Wikilink],
    resolve: &impl Fn(&Target) -> bool,
) {
    let marker = '\u{E000}';
    let end_mark = '\u{E001}';
    let mut rest = text;
    while let Some(start) = rest.find(marker) {
        if start > 0 {
            buf.push(Inline::Text {
                text: rest[..start].to_owned(),
            });
        }
        let after = &rest[start + marker.len_utf8()..];
        let Some(end) = after.find(end_mark) else {
            buf.push(Inline::Text {
                text: rest[start..].to_owned(),
            });
            return;
        };
        if let Ok(slot) = after[..end].parse::<usize>()
            && let Some(link) = links.get(slot)
        {
            buf.push(Inline::Link {
                kind: link.target.kind_name().into(),
                target: link.target.raw().to_owned(),
                label: link.label.clone(),
                href: String::new(),
                resolved: resolve(&link.target),
            });
        }
        rest = &after[end + end_mark.len_utf8()..];
    }
    if !rest.is_empty() {
        buf.push(Inline::Text {
            text: rest.to_owned(),
        });
    }
}

pub fn plain(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text: part }
            | Inline::Code { text: part }
            | Inline::Link { label: part, .. } => text.push_str(part),
            Inline::Em { inlines } | Inline::Strong { inlines } => text.push_str(&plain(inlines)),
            Inline::Break => text.push(' '),
        }
    }
    text
}
