//! Backlink index. It is rebuilt from note bodies, never stored.
//! `update` reapplies the notes a command changed; tests compare that with `full`.
use super::links::{Target, Wikilink, links_in};
use super::model::Note;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    outgoing: BTreeMap<String, Vec<Wikilink>>,
    back: BTreeMap<String, BTreeSet<String>>,
}

impl Index {
    pub fn full(notes: &[Note]) -> Self {
        let mut index = Self {
            outgoing: BTreeMap::new(),
            back: BTreeMap::new(),
        };
        for note in notes {
            index.insert(note);
        }
        index
    }

    /// Replaces the outgoing links of `changed` note ids. A missing note is a deletion.
    ///
    /// Production queries rebuild with [`Index::full`]. This path exists so a
    /// test can prove an incremental update matches that rebuild.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn update(&self, notes: &[Note], changed: &[String]) -> Self {
        let mut next = self.clone();
        for id in changed {
            next.clear(id);
            if let Some(note) = notes.iter().find(|note| &note.id == id) {
                next.insert(note);
            }
        }
        next
    }

    pub fn outgoing_of(&self, id: &str) -> &[Wikilink] {
        self.outgoing.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn ids_for(&self, key: &str) -> impl Iterator<Item = &str> {
        self.back
            .get(key)
            .into_iter()
            .flat_map(|ids| ids.iter().map(String::as_str))
    }

    pub fn linked_from_others(&self, title: &str, id: &str) -> bool {
        let key = Target::Note {
            title: title.to_owned(),
        }
        .key();
        self.ids_for(&key).any(|other| other != id)
    }

    fn insert(&mut self, note: &Note) {
        let links = links_in(&note.body);
        for link in &links {
            self.back
                .entry(link.target.key())
                .or_default()
                .insert(note.id.clone());
        }
        self.outgoing.insert(note.id.clone(), links);
    }

    fn clear(&mut self, id: &str) {
        let Some(links) = self.outgoing.remove(id) else {
            return;
        };
        for link in links {
            let key = link.target.key();
            let empty = self.back.get_mut(&key).is_some_and(|ids| {
                ids.remove(id);
                ids.is_empty()
            });
            if empty {
                self.back.remove(&key);
            }
        }
    }
}
