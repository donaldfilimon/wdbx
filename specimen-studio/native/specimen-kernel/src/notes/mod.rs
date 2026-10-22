//! Notes on the specimen document. Links are parsed from each body and are
//! never stored, so they cannot disagree with the text. A missing `notes`
//! key is an empty list.
mod graph;
mod index;
mod links;
mod model;
mod query;
mod render;

pub use graph::Graph;
pub use model::{Note, validate_notes};
pub use query::{Answer, Query, query};

use crate::host::Host;
use crate::mutate::log;
use crate::{Result, error};
use serde_json::Value;

/// Creates a note. The id comes from the host when the caller does not pass one.
pub fn create(
    h: &Host,
    state: &Value,
    title: &str,
    body: &str,
    tags: &[String],
    id: Option<&str>,
) -> Result<(Value, String)> {
    let mut notes = model::load(state)?;
    if notes.len() >= model::NOTES_MAX {
        return Err(error("Invalid", "This specimen already has 200 notes."));
    }
    let title = model::clean_title(title)?;
    if notes.iter().any(|note| note.title == title) {
        return Err(error("Invalid", "A note with that title already exists."));
    }
    let body = model::clean_body(body)?;
    let tags = model::clean_tags(tags)?;
    let id = match id {
        Some(id) => {
            let id = id.to_owned();
            if !model::valid_id(&id) {
                return Err(error("Invalid", "That note id is not valid."));
            }
            if notes.iter().any(|note| note.id == id) {
                return Err(error("Invalid", "A note with that id already exists."));
            }
            id
        }
        None => h.env.uid(),
    };
    let now = h.env.now_rfc3339();
    notes.push(model::Note {
        id,
        title: title.clone(),
        body,
        tags,
        created_at: now.clone(),
        updated_at: now,
    });
    let mut next = state.clone();
    model::store(&mut next, &notes)?;
    log(h.env, &mut next, "note", "Note created", &title);
    Ok((next, "Note created.".into()))
}

/// Replaces one note's body.
pub fn edit(h: &Host, state: &Value, id: &str, body: &str) -> Result<(Value, String)> {
    let mut notes = model::load(state)?;
    let note = model::find_mut(&mut notes, id)?;
    let body = model::clean_body(body)?;
    if note.body == body {
        return Ok((state.clone(), "Note saved.".into()));
    }
    note.body = body;
    note.updated_at = h.env.now_rfc3339();
    let title = note.title.clone();
    save_note(h, state, notes, "Note edited", &title, "Note saved.")
}

/// Renames a note and rewrites `[[old]]` and `[[old|label]]` in every note.
pub fn rename(h: &Host, state: &Value, id: &str, title: &str) -> Result<(Value, String)> {
    let mut notes = model::load(state)?;
    let title = model::clean_title(title)?;
    let old = notes
        .iter()
        .find(|note| note.id == id)
        .ok_or_else(|| error("NotFound", "That note is not in the specimen."))?
        .title
        .clone();
    if notes
        .iter()
        .any(|note| note.id != id && note.title == title)
    {
        return Err(error("Invalid", "A note with that title already exists."));
    }
    if old == title {
        return Ok((state.clone(), "Note renamed.".into()));
    }
    let now = h.env.now_rfc3339();
    for note in &mut notes {
        if note.id == id {
            note.title = title.clone();
            note.updated_at = now.clone();
        }
        let rewritten = links::rewrite_title(&note.body, &old, &title);
        if rewritten != note.body {
            note.body = rewritten;
            note.updated_at = now.clone();
        }
    }
    save_note(h, state, notes, "Note renamed", &title, "Note renamed.")
}

/// Deletes a note. Other notes that still link to its title block the delete
/// unless `force` is set.
pub fn delete(h: &Host, state: &Value, id: &str, force: bool) -> Result<(Value, String)> {
    let mut notes = model::load(state)?;
    let Some(position) = notes.iter().position(|note| note.id == id) else {
        return Err(error("NotFound", "That note is not in the specimen."));
    };
    if !force {
        let title = notes[position].title.clone();
        let linked = index::Index::full(&notes).linked_from_others(&title, id);
        if linked {
            return Err(error(
                "Invalid",
                "That note still has backlinks. Delete it with force to remove it anyway.",
            ));
        }
    }
    let title = notes.remove(position).title;
    save_note(h, state, notes, "Note deleted", &title, "Note deleted.")
}

/// Replaces one note's tags.
pub fn tag(h: &Host, state: &Value, id: &str, tags: &[String]) -> Result<(Value, String)> {
    let mut notes = model::load(state)?;
    let tags = model::clean_tags(tags)?;
    let note = model::find_mut(&mut notes, id)?;
    if note.tags == tags {
        return Ok((state.clone(), "Note tagged.".into()));
    }
    note.tags = tags;
    note.updated_at = h.env.now_rfc3339();
    let title = note.title.clone();
    save_note(h, state, notes, "Note tagged", &title, "Note tagged.")
}

fn save_note(
    h: &Host,
    state: &Value,
    notes: Vec<model::Note>,
    event: &str,
    detail: &str,
    message: &str,
) -> Result<(Value, String)> {
    let mut next = state.clone();
    model::store(&mut next, &notes)?;
    log(h.env, &mut next, "note", event, detail);
    Ok((next, message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{self, Command};
    use crate::host::FixedHost;
    use crate::mutate;
    use crate::notes::index::Index;
    use crate::notes::links::{Target, links_in};
    use crate::notes::render::{Block, Inline};

    fn host_state() -> (FixedHost, Value) {
        let fixed = FixedHost::default();
        let state = mutate::seed(&fixed.host());
        (fixed, state)
    }

    fn create_note(fixed: &FixedHost, state: &Value, title: &str, body: &str) -> (Value, String) {
        create(&fixed.host(), state, title, body, &[], None).unwrap()
    }

    #[test]
    fn a_missing_notes_list_is_empty_and_still_valid() {
        let (fixed, state) = host_state();
        assert!(state.get("notes").is_none());
        assert!(model::load(&state).unwrap().is_empty());
        mutate::validate_records(&state).unwrap();
        let answer = query(&state, &Query::Unresolved).unwrap();
        assert!(matches!(answer, Answer::Unresolved { links, .. } if links.is_empty()));
        let applied = command::apply(
            &fixed.host(),
            &state,
            None,
            Command::CreateNote {
                title: "Studio".into(),
                body: "Hello".into(),
                tags: Vec::new(),
                id: None,
            },
        )
        .unwrap();
        assert!(applied.state.get("notes").is_some());
        let undone = command::apply(
            &fixed.host(),
            &applied.state,
            None,
            Command::Revert { undo: applied.undo },
        )
        .unwrap();
        assert_eq!(
            command::digest_of(&state),
            command::digest_of(&undone.state)
        );
        assert!(undone.state.get("notes").is_none());
    }

    #[test]
    fn rename_rewrites_exact_links_and_delete_waits_for_backlinks() {
        let (fixed, state) = host_state();
        let (state, _) = create_note(&fixed, &state, "Color", "Teal.");
        let (state, _) = create_note(
            &fixed,
            &state,
            "Links",
            "See [[Color]] and [[Color|the swatch]] plus [[Color theory]].",
        );
        let color = model::load(&state)
            .unwrap()
            .into_iter()
            .find(|note| note.title == "Color")
            .unwrap();
        let (renamed, _) = rename(&fixed.host(), &state, &color.id, "Hue").unwrap();
        let notes = model::load(&renamed).unwrap();
        let links = notes.iter().find(|note| note.title == "Links").unwrap();
        assert!(links.body.contains("[[Hue]]"));
        assert!(links.body.contains("[[Hue|the swatch]]"));
        assert!(links.body.contains("[[Color theory]]"));
        assert!(
            !links_in(&links.body)
                .iter()
                .any(|link| { matches!(&link.target, Target::Note { title } if title == "Color") })
        );
        let hue = notes.iter().find(|note| note.title == "Hue").unwrap();
        let err = delete(&fixed.host(), &renamed, &hue.id, false).unwrap_err();
        assert_eq!(err.code, "Invalid");
        assert!(err.message.contains("backlinks"));
        assert_eq!(model::load(&renamed).unwrap().len(), 2);
        delete(&fixed.host(), &renamed, &hue.id, true).unwrap();
    }

    #[test]
    fn html_is_dropped_and_code_spans_are_not_links() {
        let body = "<script>alert(1)</script>\n\nSee **bold** and `[[Secret]]` plus [[Shown]].";
        let blocks = render::render(body, |_| false);
        let dumped = serde_json::to_string(&blocks).unwrap();
        assert!(!dumped.contains("script"));
        assert!(!dumped.contains("alert"));
        assert!(!dumped.contains('<'));
        assert!(blocks.iter().any(|block| {
            matches!(block, Block::Paragraph { inlines } if inlines.iter().any(|inline| {
                matches!(inline, Inline::Strong { .. })
            }))
        }));
        assert!(blocks.iter().any(|block| {
            matches!(block, Block::Paragraph { inlines } if inlines.iter().any(|inline| {
                matches!(inline, Inline::Code { text } if text == "[[Secret]]")
            }))
        }));
        let links = links_in(body);
        assert_eq!(links.len(), 1);
        assert!(matches!(&links[0].target, Target::Note { title } if title == "Shown"));
    }

    #[test]
    fn an_incremental_index_matches_a_full_rebuild() {
        let (fixed, state) = host_state();
        let (state, _) = create_note(&fixed, &state, "A", "Link [[B]]");
        let (state, _) = create_note(&fixed, &state, "B", "Link [[A]]");
        let (state, _) = create_note(&fixed, &state, "C", "Alone");
        let before = model::load(&state).unwrap();
        let full_before = Index::full(&before);
        let b = before.iter().find(|note| note.title == "B").unwrap();
        let (edited, _) = edit(
            &fixed.host(),
            &state,
            &b.id,
            "Now [[C]] and [[node:missing]]",
        )
        .unwrap();
        let after = model::load(&edited).unwrap();
        let incremental = full_before.update(&after, std::slice::from_ref(&b.id));
        assert_eq!(incremental, Index::full(&after));
        let (deleted, _) = delete(&fixed.host(), &edited, &b.id, true).unwrap();
        let remaining = model::load(&deleted).unwrap();
        assert_eq!(
            incremental.update(&remaining, std::slice::from_ref(&b.id)),
            Index::full(&remaining)
        );
    }

    #[test]
    fn graph_positions_do_not_change_between_calls() {
        let (fixed, state) = host_state();
        let (state, _) = create_note(&fixed, &state, "A", "[[B]]");
        let (state, _) = create_note(&fixed, &state, "B", "[[A]]");
        let notes = model::load(&state).unwrap();
        let once = graph::layout(&notes, None, 0);
        let twice = graph::layout(&notes, None, 0);
        assert_eq!(once, twice);
        assert_eq!(once.nodes.len(), 2);
        assert!(!once.edges.is_empty());
    }
}
