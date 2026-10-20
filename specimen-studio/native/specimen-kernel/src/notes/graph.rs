//! Deterministic note layout. Positions come from an id-sorted circle and a
//! fixed number of force steps, using `libm` and `neural::Rng` seed 104729.
use super::index::Index;
use super::links::Target;
use super::model::Note;
use crate::neural::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

const PI: f64 = std::f64::consts::PI;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub fn layout(notes: &[Note], focus: Option<&str>, depth: u32) -> Graph {
    let index = Index::full(notes);
    let selected = select(notes, &index, focus, depth);
    let count = selected.len();
    if count == 0 {
        return Graph {
            nodes: Vec::new(),
            edges: Vec::new(),
        };
    }
    let mut rng = Rng(104729);
    let mut pos = Vec::with_capacity(count);
    for slot in 0..count {
        let angle = 2.0 * PI * (slot as f64) / (count as f64);
        let radius = if count == 1 { 0.0 } else { 160.0 };
        pos.push((
            radius * libm::cos(angle) + (rng.sample() - 0.5) * 0.5,
            radius * libm::sin(angle) + (rng.sample() - 0.5) * 0.5,
        ));
    }
    let mut edges = Vec::new();
    for (from_at, note) in selected.iter().enumerate() {
        for link in index.outgoing_of(&note.id) {
            let Target::Note { title } = &link.target else {
                continue;
            };
            let Some(to_at) = selected.iter().position(|other| &other.title == title) else {
                continue;
            };
            if from_at != to_at {
                edges.push((from_at, to_at));
            }
        }
    }
    for _ in 0..12 {
        let mut disp = vec![(0.0, 0.0); count];
        for left in 0..count {
            for right in (left + 1)..count {
                let dx = pos[left].0 - pos[right].0;
                let dy = pos[left].1 - pos[right].1;
                let dist = libm::sqrt(dx * dx + dy * dy).max(0.01);
                let force = 1200.0 / (dist * dist);
                let ux = dx / dist;
                let uy = dy / dist;
                disp[left].0 += ux * force;
                disp[left].1 += uy * force;
                disp[right].0 -= ux * force;
                disp[right].1 -= uy * force;
            }
        }
        for &(from_at, to_at) in &edges {
            let dx = pos[to_at].0 - pos[from_at].0;
            let dy = pos[to_at].1 - pos[from_at].1;
            let dist = libm::sqrt(dx * dx + dy * dy).max(0.01);
            let force = dist * 0.02;
            let ux = dx / dist;
            let uy = dy / dist;
            disp[from_at].0 += ux * force;
            disp[from_at].1 += uy * force;
            disp[to_at].0 -= ux * force;
            disp[to_at].1 -= uy * force;
        }
        for slot in 0..count {
            let mag = libm::sqrt(disp[slot].0 * disp[slot].0 + disp[slot].1 * disp[slot].1);
            if mag > 0.0 {
                let step = mag.min(8.0);
                pos[slot].0 += disp[slot].0 / mag * step;
                pos[slot].1 += disp[slot].1 / mag * step;
            }
        }
    }
    Graph {
        nodes: selected
            .iter()
            .enumerate()
            .map(|(slot, note)| GraphNode {
                id: note.id.clone(),
                title: note.title.clone(),
                x: pos[slot].0,
                y: pos[slot].1,
            })
            .collect(),
        edges: edges
            .into_iter()
            .map(|(from_at, to_at)| GraphEdge {
                from: selected[from_at].id.clone(),
                to: selected[to_at].id.clone(),
            })
            .collect(),
    }
}

fn select<'a>(notes: &'a [Note], index: &Index, focus: Option<&str>, depth: u32) -> Vec<&'a Note> {
    let mut ordered: Vec<&Note> = notes.iter().collect();
    ordered.sort_by(|left, right| left.id.cmp(&right.id));
    let Some(focus) = focus.filter(|focus| !focus.is_empty()) else {
        return ordered;
    };
    let Some(start) = ordered
        .iter()
        .copied()
        .find(|note| note.id == focus || note.title == focus)
    else {
        return Vec::new();
    };
    let mut seen = BTreeSet::from([start.id.as_str()]);
    let mut out = vec![start];
    let mut layer = VecDeque::from([start]);
    for _ in 0..depth.min(8) {
        let width = layer.len();
        for _ in 0..width {
            let Some(note) = layer.pop_front() else {
                break;
            };
            for neighbor in neighbors(notes, index, note) {
                if seen.insert(neighbor.id.as_str()) {
                    out.push(neighbor);
                    layer.push_back(neighbor);
                }
            }
        }
    }
    out.sort_by(|left, right| left.id.cmp(&right.id));
    out
}

fn neighbors<'a>(notes: &'a [Note], index: &Index, note: &Note) -> Vec<&'a Note> {
    let mut ids = BTreeSet::new();
    for link in index.outgoing_of(&note.id) {
        if let Target::Note { title } = &link.target
            && let Some(other) = notes.iter().find(|other| &other.title == title)
        {
            ids.insert(other.id.clone());
        }
    }
    let key = Target::Note {
        title: note.title.clone(),
    }
    .key();
    for id in index.ids_for(&key) {
        ids.insert(id.to_owned());
    }
    ids.remove(&note.id);
    notes
        .iter()
        .filter(|other| ids.contains(&other.id))
        .collect()
}
