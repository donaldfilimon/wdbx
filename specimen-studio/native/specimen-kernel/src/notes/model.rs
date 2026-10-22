//! Note records and the limits the commands enforce.
use crate::{Result, error};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const TITLE_MAX: usize = 200;
pub const BODY_MAX: usize = 20_000;
pub const TAG_MAX: usize = 40;
pub const TAGS_MAX: usize = 16;
pub const NOTES_MAX: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Accepts a specimen that has no notes. A present `notes` array must be well formed.
pub fn validate_notes(state: &Value) -> Result<()> {
    match state.get("notes") {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Array(items)) => {
            if items.len() > NOTES_MAX {
                return Err(error("Invalid", "This specimen already has 200 notes."));
            }
            let mut ids = BTreeSet::new();
            let mut titles = BTreeSet::new();
            for item in items {
                let note: Note = serde_json::from_value(item.clone())
                    .map_err(|_| error("Invalid", "A note is malformed."))?;
                clean_title(&note.title)?;
                clean_body(&note.body)?;
                clean_tags(&note.tags)?;
                if !valid_id(&note.id) || !ids.insert(note.id) {
                    return Err(error("Invalid", "A note id is missing or repeated."));
                }
                if !titles.insert(note.title) {
                    return Err(error("Invalid", "A note title is repeated."));
                }
            }
            Ok(())
        }
        Some(_) => Err(error("Invalid", "Notes must be a list.")),
    }
}

pub fn load(state: &Value) -> Result<Vec<Note>> {
    validate_notes(state)?;
    match state.get("notes") {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                serde_json::from_value(item.clone()).map_err(|err| error("MalformedSave", err))
            })
            .collect(),
        Some(_) => Err(error("Invalid", "Notes must be a list.")),
    }
}

pub fn store(state: &mut Value, notes: &[Note]) -> Result<()> {
    state["notes"] = serde_json::to_value(notes).map_err(|err| error("Invalid", err))?;
    Ok(())
}

pub fn find_mut<'a>(notes: &'a mut [Note], id: &str) -> Result<&'a mut Note> {
    notes
        .iter_mut()
        .find(|note| note.id == id)
        .ok_or_else(|| error("NotFound", "That note is not in the specimen."))
}

pub fn find<'a>(notes: &'a [Note], id_or_title: &str) -> Result<&'a Note> {
    notes
        .iter()
        .find(|note| note.id == id_or_title || note.title == id_or_title)
        .ok_or_else(|| error("NotFound", "That note is not in the specimen."))
}

pub fn clean_title(title: &str) -> Result<String> {
    let title = title.trim();
    if title.is_empty() {
        return Err(error("Invalid", "Enter a title."));
    }
    if title.chars().count() > TITLE_MAX {
        return Err(error(
            "Invalid",
            "A note title can be at most 200 characters.",
        ));
    }
    if title
        .chars()
        .any(|ch| matches!(ch, '|' | '[' | ']' | '\n' | '\r'))
    {
        return Err(error(
            "Invalid",
            "A note title cannot contain brackets or a vertical bar.",
        ));
    }
    Ok(title.to_owned())
}

pub fn clean_body(body: &str) -> Result<String> {
    if body.chars().count() > BODY_MAX {
        return Err(error("Invalid", "A note can be at most 20000 characters."));
    }
    Ok(body.to_owned())
}

pub fn clean_tags(tags: &[String]) -> Result<Vec<String>> {
    if tags.len() > TAGS_MAX {
        return Err(error("Invalid", "A note can have at most 16 tags."));
    }
    let mut out = Vec::with_capacity(tags.len());
    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() || tag.chars().count() > TAG_MAX || tag.chars().any(char::is_control) {
            return Err(error(
                "Invalid",
                "Each tag is required and can be at most 40 characters.",
            ));
        }
        if out.iter().any(|existing: &String| existing == tag) {
            return Err(error("Invalid", "Tags must be unique."));
        }
        out.push(tag.to_owned());
    }
    Ok(out)
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 80 && id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}
