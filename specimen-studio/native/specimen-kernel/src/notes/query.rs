//! Read-only questions over notes. Queries do not change the specimen.
use super::graph::{self, Graph};
use super::index::Index;
use super::links::{Target, Wikilink, links_in};
use super::model::{self, Note};
use super::render::{self, Block};
use crate::engine::{rows, text};
use crate::{Result, error};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "query",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Query {
    Backlinks {
        target: String,
    },
    Outgoing {
        note: String,
    },
    Unresolved,
    Search {
        text: String,
        #[serde(default)]
        regex: bool,
    },
    Graph {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        focus: Option<String>,
        #[serde(default)]
        depth: u32,
    },
    Render {
        id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteHit {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnresolvedHit {
    pub note_id: String,
    pub note_title: String,
    pub title: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Answer {
    Notes {
        summary: String,
        notes: Vec<NoteHit>,
    },
    Links {
        summary: String,
        links: Vec<Wikilink>,
    },
    Unresolved {
        summary: String,
        links: Vec<UnresolvedHit>,
    },
    Graph {
        summary: String,
        nodes: Vec<super::graph::GraphNode>,
        edges: Vec<super::graph::GraphEdge>,
    },
    Document {
        summary: String,
        blocks: Vec<Block>,
    },
}

pub fn query(state: &Value, query: &Query) -> Result<Answer> {
    let notes = model::load(state)?;
    match query {
        Query::Backlinks { target } => {
            let hits = backlinks(&notes, target);
            let summary = if hits.is_empty() {
                "No backlinks.".into()
            } else {
                hits.iter()
                    .map(|hit| hit.title.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Ok(Answer::Notes {
                summary,
                notes: hits,
            })
        }
        Query::Outgoing { note } => {
            let note = model::find(&notes, note)?;
            let links = links_in(&note.body);
            let summary = if links.is_empty() {
                "No outgoing links.".into()
            } else {
                links
                    .iter()
                    .map(|link| format!("{} → {}", link.label, link.target.key()))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Ok(Answer::Links { summary, links })
        }
        Query::Unresolved => {
            let links = unresolved(state, &notes);
            let summary = if links.is_empty() {
                "No unresolved links.".into()
            } else {
                links
                    .iter()
                    .map(|link| format!("{} (in {})", link.title, link.note_title))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Ok(Answer::Unresolved { summary, links })
        }
        Query::Search { text, regex } => {
            let found = search(&notes, text, *regex)?;
            let summary = if found.is_empty() {
                "No matching notes.".into()
            } else {
                found
                    .iter()
                    .map(|hit| hit.title.clone())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Ok(Answer::Notes {
                summary,
                notes: found,
            })
        }
        Query::Graph { focus, depth } => {
            let graph = graph::layout(&notes, focus.as_deref(), *depth);
            let summary = format!(
                "Note graph: {} notes, {} links.",
                graph.nodes.len(),
                graph.edges.len()
            );
            Ok(graph_answer(summary, graph))
        }
        Query::Render { id } => {
            let note = model::find(&notes, id)?;
            let blocks = render::render(&note.body, |target| resolves(state, target));
            Ok(Answer::Document {
                summary: note.title.clone(),
                blocks,
            })
        }
    }
}

fn graph_answer(summary: String, graph: Graph) -> Answer {
    Answer::Graph {
        summary,
        nodes: graph.nodes,
        edges: graph.edges,
    }
}

pub fn backlinks(notes: &[Note], target: &str) -> Vec<NoteHit> {
    let key = canonical_target(target);
    let index = Index::full(notes);
    let mut hits: Vec<NoteHit> = index
        .ids_for(&key)
        .filter_map(|id| notes.iter().find(|note| note.id == id))
        .map(hit)
        .collect();
    hits.sort_by(|left, right| left.title.cmp(&right.title).then(left.id.cmp(&right.id)));
    hits
}

fn canonical_target(target: &str) -> String {
    let target = target.trim();
    if target.starts_with("node:")
        || target.starts_with("pattern:")
        || target.starts_with("memory:")
        || target.starts_with("note:")
    {
        target.to_owned()
    } else {
        format!("note:{target}")
    }
}

fn unresolved(state: &Value, notes: &[Note]) -> Vec<UnresolvedHit> {
    let mut hits = Vec::new();
    for note in notes {
        for link in links_in(&note.body) {
            if resolves(state, &link.target) {
                continue;
            }
            hits.push(UnresolvedHit {
                note_id: note.id.clone(),
                note_title: note.title.clone(),
                title: link.label,
                target: link.target.key(),
            });
        }
    }
    hits
}

fn search(notes: &[Note], text: &str, regex: bool) -> Result<Vec<NoteHit>> {
    let text = text.trim();
    if text.is_empty() {
        return Err(error("ParseError", "Enter text to search for."));
    }
    if text.chars().count() > 200 {
        return Err(error(
            "ParseError",
            "Search text can be at most 200 characters.",
        ));
    }
    let hits = if regex {
        let pattern = Regex::new(text).map_err(|_| {
            error(
                "ParseError",
                "That search pattern is not a valid regular expression.",
            )
        })?;
        notes
            .iter()
            .filter(|note| note_matches_regex(note, &pattern))
            .map(hit)
            .collect()
    } else {
        let needle = text.to_lowercase();
        notes
            .iter()
            .filter(|note| {
                note.title.to_lowercase().contains(&needle)
                    || note.body.to_lowercase().contains(&needle)
                    || note
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&needle))
            })
            .map(hit)
            .collect()
    };
    Ok(hits)
}

fn note_matches_regex(note: &Note, pattern: &Regex) -> bool {
    pattern.is_match(&note.title)
        || pattern.is_match(&note.body)
        || note.tags.iter().any(|tag| pattern.is_match(tag))
}

fn hit(note: &Note) -> NoteHit {
    NoteHit {
        id: note.id.clone(),
        title: note.title.clone(),
    }
}

pub fn resolves(state: &Value, target: &Target) -> bool {
    match target {
        Target::Note { title } => model::load(state)
            .ok()
            .is_some_and(|notes| notes.iter().any(|note| &note.title == title)),
        Target::Node { node_ref } => rows(state, "nodes")
            .iter()
            .any(|node| text(node, "ref") == node_ref),
        Target::Pattern { id } => pattern_ids(state).contains(id),
        Target::Memory { resource_ref } => rows(state, "resources")
            .iter()
            .any(|resource| text(resource, "ref") == resource_ref),
    }
}

fn pattern_ids(state: &Value) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for node in rows(state, "nodes") {
        let pattern = text(node, "patternId");
        if !pattern.is_empty() {
            ids.insert(pattern.to_owned());
        }
        for entry in rows(node, "entries") {
            let id = text(entry, "id");
            if !id.is_empty() {
                ids.insert(id.to_owned());
            }
        }
    }
    ids
}
