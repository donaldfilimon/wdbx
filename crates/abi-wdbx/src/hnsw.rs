//! Layered Hierarchical Navigable Small World (HNSW) vector index.
//!
//! This is the Rust replacement for `hnsw.zig` and `hnsw_storage.zig`. Graph
//! construction uses the frozen WDBX parameters and deterministic level
//! sampling. [`crate::ExactIndex`] remains the correctness oracle for tests.

use crate::{SearchResult, Snapshot, cosine_similarity};
use std::collections::{BTreeMap, HashSet};

/// Maximum number of graph layers, including layer zero.
pub const MAX_LAYERS: usize = 4;
/// Maximum neighbors retained per node and layer.
pub const M: usize = 16;
/// Candidate width used while inserting.
pub const EF_CONSTRUCTION: usize = 40;
/// Minimum candidate width used while searching.
pub const EF_SEARCH: usize = 32;

/// A vector-storage or HNSW graph failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HnswError {
    /// Index dimensions must be within WDBX's supported range.
    InvalidDimensions {
        /// Requested dimensions.
        dimensions: usize,
    },
    /// A vector or query did not match the index dimensions.
    DimensionMismatch {
        /// Dimensions required by the index.
        expected: usize,
        /// Dimensions supplied by the caller.
        found: usize,
    },
    /// The id already has a graph node.
    DuplicateId {
        /// Duplicate vector id.
        id: u64,
    },
    /// A recovered snapshot contains no vector dimensions to infer.
    EmptySnapshot,
    /// An internal graph invariant was violated.
    CorruptGraph {
        /// Human-readable invariant failure.
        reason: String,
    },
}

impl std::fmt::Display for HnswError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDimensions { dimensions } => {
                write!(formatter, "invalid HNSW dimensions: {dimensions}")
            }
            Self::DimensionMismatch { expected, found } => {
                write!(
                    formatter,
                    "vector has {found} dimensions; expected {expected}"
                )
            }
            Self::DuplicateId { id } => write!(formatter, "vector id {id} already exists"),
            Self::EmptySnapshot => {
                formatter.write_str("cannot infer HNSW dimensions from an empty snapshot")
            }
            Self::CorruptGraph { reason } => write!(formatter, "corrupt HNSW graph: {reason}"),
        }
    }
}

impl std::error::Error for HnswError {}

/// Dimension-checked vector storage owned by an HNSW graph.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorStorage {
    dimensions: usize,
    vectors: BTreeMap<u64, Vec<f32>>,
}

impl VectorStorage {
    /// Create empty storage with fixed dimensions.
    pub fn new(dimensions: usize) -> Result<Self, HnswError> {
        Self::new_with_limit(dimensions, crate::wal::MAX_VECTOR_DIMENSIONS)
    }

    /// Create storage with an explicit format-version dimension ceiling.
    pub(crate) fn new_with_limit(
        dimensions: usize,
        max_dimensions: usize,
    ) -> Result<Self, HnswError> {
        if dimensions == 0 || dimensions > max_dimensions {
            return Err(HnswError::InvalidDimensions { dimensions });
        }
        Ok(Self {
            dimensions,
            vectors: BTreeMap::new(),
        })
    }

    /// Fixed vector dimensions.
    #[must_use]
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Number of present vectors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Whether no vectors are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Insert a previously absent vector id.
    pub fn insert(&mut self, id: u64, values: &[f32]) -> Result<(), HnswError> {
        self.validate(values)?;
        if self.vectors.contains_key(&id) {
            return Err(HnswError::DuplicateId { id });
        }
        self.vectors.insert(id, values.to_vec());
        Ok(())
    }

    /// Borrow one stored vector.
    #[must_use]
    pub fn get(&self, id: u64) -> Option<&[f32]> {
        self.vectors.get(&id).map(Vec::as_slice)
    }

    /// Whether an id is present.
    #[must_use]
    pub fn contains(&self, id: u64) -> bool {
        self.vectors.contains_key(&id)
    }

    fn remove(&mut self, id: u64) -> bool {
        self.vectors.remove(&id).is_some()
    }

    fn validate(&self, values: &[f32]) -> Result<(), HnswError> {
        if values.len() != self.dimensions {
            return Err(HnswError::DimensionMismatch {
                expected: self.dimensions,
                found: values.len(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Node {
    id: u64,
    level: usize,
    edges: [Vec<usize>; MAX_LAYERS],
}

impl Node {
    fn new(id: u64, level: usize) -> Self {
        Self {
            id,
            level,
            edges: std::array::from_fn(|_| Vec::new()),
        }
    }
}

#[derive(Debug, Clone)]
struct EdgeSnapshot {
    node: usize,
    layer: usize,
    edges: Vec<usize>,
}

#[derive(Debug, Clone)]
struct InsertUndo {
    id: u64,
    node_index: usize,
    prior_entry: Option<usize>,
    prior_max_level: usize,
    edges: Vec<EdgeSnapshot>,
}

impl InsertUndo {
    fn new(id: u64, node_index: usize, prior_entry: Option<usize>, prior_max_level: usize) -> Self {
        Self {
            id,
            node_index,
            prior_entry,
            prior_max_level,
            edges: Vec::new(),
        }
    }

    fn has_snapshot(&self, node: usize, layer: usize) -> bool {
        self.edges
            .iter()
            .any(|snapshot| snapshot.node == node && snapshot.layer == layer)
    }
}

#[derive(Debug, Clone, Copy)]
struct Candidate {
    node: usize,
    distance: f32,
}

/// Deterministic layered HNSW index.
#[derive(Debug, Clone)]
pub struct HnswIndex {
    storage: VectorStorage,
    nodes: Vec<Node>,
    entry_node: Option<usize>,
    max_level: usize,
    level_rng: LevelRng,
    last_insert: Option<InsertUndo>,
}

impl HnswIndex {
    /// Create an empty deterministic index.
    pub fn new(dimensions: usize) -> Result<Self, HnswError> {
        Self::with_seed(dimensions, 0)
    }

    /// Create an index with an explicit deterministic level seed.
    pub fn with_seed(dimensions: usize, seed: u64) -> Result<Self, HnswError> {
        Self::with_seed_and_limit(dimensions, seed, crate::wal::MAX_VECTOR_DIMENSIONS)
    }

    /// Construct an index using a format-version-specific dimension ceiling.
    pub(crate) fn with_seed_and_limit(
        dimensions: usize,
        seed: u64,
        max_dimensions: usize,
    ) -> Result<Self, HnswError> {
        Ok(Self {
            storage: VectorStorage::new_with_limit(dimensions, max_dimensions)?,
            nodes: Vec::new(),
            entry_node: None,
            max_level: 0,
            level_rng: LevelRng::new(seed),
            last_insert: None,
        })
    }

    /// Rebuild a graph from a recovered snapshot in ascending id order.
    pub fn from_snapshot(snapshot: &Snapshot) -> Result<Self, HnswError> {
        let dimensions = snapshot
            .vector_dimensions()
            .ok_or(HnswError::EmptySnapshot)?;
        let mut index = Self::new(dimensions)?;
        for (&id, values) in &snapshot.vectors {
            index.insert(id, values)?;
            index.commit_last_insert(id);
        }
        Ok(index)
    }

    /// Fixed vector dimensions.
    #[must_use]
    pub fn dimensions(&self) -> usize {
        self.storage.dimensions()
    }

    /// Number of graph nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether no graph nodes exist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Borrow a stored vector.
    #[must_use]
    pub fn get(&self, id: u64) -> Option<&[f32]> {
        self.storage.get(id)
    }

    /// Insert an absolute WDBX vector id.
    pub fn insert(&mut self, id: u64, values: &[f32]) -> Result<(), HnswError> {
        self.storage.validate(values)?;
        if self.storage.contains(id) {
            return Err(HnswError::DuplicateId { id });
        }

        // A rollback is only meaningful until the next insert begins.
        self.last_insert = None;
        let prior_entry = self.entry_node;
        let prior_max_level = self.max_level;
        let level = self.random_level();
        let new_index = self.nodes.len();

        self.storage.insert(id, values)?;
        self.nodes.push(Node::new(id, level));
        let mut undo = InsertUndo::new(id, new_index, prior_entry, prior_max_level);

        let Some(mut entry) = prior_entry else {
            self.entry_node = Some(new_index);
            self.max_level = level;
            self.last_insert = Some(undo);
            return Ok(());
        };

        // Preserve a deterministic layer-zero navigation backbone. Approximate
        // neighbor pruning may discard long-range links, but every appended node
        // remains reachable through its insertion-order predecessor.
        self.connect_backbone(new_index, &mut undo);

        // Greedy descent through layers the new node does not occupy.
        for layer in ((level + 1)..=prior_max_level).rev() {
            if let Some(best) = self
                .search_layer(values, &[entry], 1, layer)
                .into_iter()
                .next()
            {
                entry = best.node;
            }
        }

        // At each shared layer, search a wider candidate set, select the nearest
        // M nodes, and add bidirectional links with deterministic pruning.
        let highest_shared = level.min(prior_max_level);
        for layer in (0..=highest_shared).rev() {
            let candidates = self.search_layer(values, &[entry], EF_CONSTRUCTION, layer);
            let candidate_nodes = candidates
                .iter()
                .filter(|candidate| candidate.node != new_index)
                .map(|candidate| candidate.node)
                .collect::<Vec<_>>();
            let selected = self.select_diverse_neighbors(values, &candidate_nodes, M);
            for neighbor in selected {
                self.connect_and_prune(new_index, neighbor, layer, &mut undo);
            }
            if let Some(best) = candidates.first() {
                entry = best.node;
            }
        }

        if level > prior_max_level {
            self.entry_node = Some(new_index);
            self.max_level = level;
        }
        self.last_insert = Some(undo);
        Ok(())
    }

    /// Search the graph and return its best approximate cosine matches.
    pub fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult<'_>>, HnswError> {
        self.storage.validate(query)?;
        let Some(mut entry) = self.entry_node else {
            return Ok(Vec::new());
        };
        if limit == 0 {
            return Ok(Vec::new());
        }

        for layer in (1..=self.max_level).rev() {
            if let Some(best) = self
                .search_layer(query, &[entry], 1, layer)
                .into_iter()
                .next()
            {
                entry = best.node;
            }
        }

        let mut candidates = self.search_layer(query, &[entry], EF_SEARCH.max(limit), 0);
        candidates.truncate(limit);
        Ok(candidates
            .into_iter()
            .map(|candidate| {
                let node = &self.nodes[candidate.node];
                SearchResult {
                    id: node.id,
                    score: 1.0 - candidate.distance,
                    vector: self
                        .storage
                        .get(node.id)
                        .expect("graph nodes always have storage"),
                }
            })
            .collect())
    }

    /// Roll back the newest insert when it matches `id`.
    ///
    /// The insertion journal restores every existing adjacency list affected by
    /// neighbor pruning, then removes the new node and vector.
    pub fn rollback_last_insert(&mut self, id: u64) -> bool {
        let Some(undo) = self.last_insert.take() else {
            return false;
        };
        if undo.id != id
            || undo.node_index + 1 != self.nodes.len()
            || self.nodes.last().map(|node| node.id) != Some(id)
        {
            self.last_insert = Some(undo);
            return false;
        }

        for snapshot in undo.edges {
            self.nodes[snapshot.node].edges[snapshot.layer] = snapshot.edges;
        }
        self.nodes.pop();
        self.storage.remove(id);
        self.entry_node = undo.prior_entry;
        self.max_level = undo.prior_max_level;
        true
    }

    /// Discard rollback state after the caller durably commits the insert.
    pub fn commit_last_insert(&mut self, id: u64) -> bool {
        if self.last_insert.as_ref().map(|undo| undo.id) == Some(id) {
            self.last_insert = None;
            true
        } else {
            false
        }
    }

    /// Verify storage, entry-point, layer, degree and symmetric-edge invariants.
    pub fn validate_graph(&self) -> Result<(), HnswError> {
        if self.nodes.len() != self.storage.len() {
            return Self::corrupt("node and vector counts disagree");
        }
        if self.nodes.is_empty() {
            if self.entry_node.is_some() {
                return Self::corrupt("empty graph has an entry point");
            }
            return Ok(());
        }

        let entry = self.entry_node.ok_or_else(|| HnswError::CorruptGraph {
            reason: "non-empty graph has no entry point".to_string(),
        })?;
        if entry >= self.nodes.len() {
            return Self::corrupt("entry point is out of range");
        }
        if self.nodes[entry].level != self.max_level {
            return Self::corrupt("entry point is not on the maximum layer");
        }

        let mut ids = HashSet::with_capacity(self.nodes.len());
        for (node_index, node) in self.nodes.iter().enumerate() {
            if !ids.insert(node.id) || !self.storage.contains(node.id) {
                return Self::corrupt("node ids are duplicate or missing from storage");
            }
            for layer in 0..MAX_LAYERS {
                let edges = &node.edges[layer];
                if layer > node.level && !edges.is_empty() {
                    return Self::corrupt("node has edges above its sampled level");
                }
                if edges.len() > M {
                    return Self::corrupt("node degree exceeds M");
                }
                let mut unique = HashSet::with_capacity(edges.len());
                for &neighbor in edges {
                    if neighbor >= self.nodes.len() || neighbor == node_index {
                        return Self::corrupt("edge target is invalid");
                    }
                    if !unique.insert(neighbor) {
                        return Self::corrupt("duplicate edge");
                    }
                    if self.nodes[neighbor].level < layer {
                        return Self::corrupt("edge reaches a node below the layer");
                    }
                    if !self.nodes[neighbor].edges[layer].contains(&node_index) {
                        return Self::corrupt("edge is not bidirectional");
                    }
                }
            }
        }
        if self.reachable_len() != self.nodes.len() {
            return Self::corrupt("layer-zero graph is disconnected");
        }
        Ok(())
    }

    /// Number of nodes reachable from the entry point through layer-zero edges.
    #[must_use]
    pub fn reachable_len(&self) -> usize {
        let Some(entry) = self.entry_node else {
            return 0;
        };
        let mut visited = HashSet::with_capacity(self.nodes.len());
        let mut pending = vec![entry];
        while let Some(node) = pending.pop() {
            if !visited.insert(node) {
                continue;
            }
            pending.extend(self.nodes[node].edges[0].iter().copied());
        }
        visited.len()
    }

    fn corrupt<T>(reason: &str) -> Result<T, HnswError> {
        Err(HnswError::CorruptGraph {
            reason: reason.to_string(),
        })
    }

    fn random_level(&mut self) -> usize {
        const LEVEL_THRESHOLD: u64 = u64::MAX / 16;
        let mut level = 0;
        while level + 1 < MAX_LAYERS && self.level_rng.next_u64() < LEVEL_THRESHOLD {
            level += 1;
        }
        level
    }

    fn search_layer(
        &self,
        query: &[f32],
        entry_points: &[usize],
        ef: usize,
        layer: usize,
    ) -> Vec<Candidate> {
        let ef = ef.max(1);
        let mut visited = HashSet::with_capacity(ef.saturating_mul(M));
        let mut pending = Vec::new();
        let mut best = Vec::new();

        for &entry in entry_points {
            if entry >= self.nodes.len()
                || self.nodes[entry].level < layer
                || !visited.insert(entry)
            {
                continue;
            }
            let candidate = Candidate {
                node: entry,
                distance: self.distance_to_node(query, entry),
            };
            insert_candidate(&mut pending, candidate, &self.nodes);
            insert_candidate(&mut best, candidate, &self.nodes);
        }

        while !pending.is_empty() {
            let current = pending.remove(0);
            if best.len() >= ef
                && candidate_cmp(
                    &current,
                    best.last().expect("best is non-empty"),
                    &self.nodes,
                )
                .is_gt()
            {
                break;
            }

            for &neighbor in &self.nodes[current.node].edges[layer] {
                if !visited.insert(neighbor) {
                    continue;
                }
                let candidate = Candidate {
                    node: neighbor,
                    distance: self.distance_to_node(query, neighbor),
                };
                let improves = best.len() < ef
                    || candidate_cmp(
                        &candidate,
                        best.last().expect("best is non-empty"),
                        &self.nodes,
                    )
                    .is_lt();
                if improves {
                    insert_candidate(&mut pending, candidate, &self.nodes);
                    insert_candidate(&mut best, candidate, &self.nodes);
                    best.truncate(ef);
                }
            }
        }
        best
    }

    fn distance_to_node(&self, query: &[f32], node: usize) -> f32 {
        let vector = self
            .storage
            .get(self.nodes[node].id)
            .expect("graph nodes always have storage");
        1.0 - cosine_similarity(vector, query)
    }

    fn connect_and_prune(
        &mut self,
        new_node: usize,
        neighbor: usize,
        layer: usize,
        undo: &mut InsertUndo,
    ) {
        if new_node == neighbor
            || self.nodes[new_node].level < layer
            || self.nodes[neighbor].level < layer
        {
            return;
        }
        if !self.nodes[new_node].edges[layer].contains(&neighbor) {
            self.nodes[new_node].edges[layer].push(neighbor);
        }
        if !self.nodes[neighbor].edges[layer].contains(&new_node) {
            self.capture_edges(neighbor, layer, new_node, undo);
            self.nodes[neighbor].edges[layer].push(new_node);
        }
        self.prune_node(neighbor, layer, new_node, undo);
        self.prune_node(new_node, layer, new_node, undo);
    }

    fn connect_backbone(&mut self, new_node: usize, undo: &mut InsertUndo) {
        if new_node == 0 {
            return;
        }
        let previous = new_node - 1;
        self.capture_edges(previous, 0, new_node, undo);
        if !self.nodes[previous].edges[0].contains(&new_node) {
            self.nodes[previous].edges[0].push(new_node);
        }
        if !self.nodes[new_node].edges[0].contains(&previous) {
            self.nodes[new_node].edges[0].push(previous);
        }
        self.prune_node(previous, 0, new_node, undo);
        self.prune_node(new_node, 0, new_node, undo);
    }

    fn prune_node(&mut self, center: usize, layer: usize, new_node: usize, undo: &mut InsertUndo) {
        if self.nodes[center].edges[layer].len() <= M {
            return;
        }

        let center_vector = self
            .storage
            .get(self.nodes[center].id)
            .expect("graph nodes always have storage")
            .to_vec();
        let current = self.nodes[center].edges[layer].clone();
        let protected = current
            .iter()
            .copied()
            .filter(|&neighbor| layer == 0 && center.abs_diff(neighbor) == 1)
            .collect::<Vec<_>>();
        let candidates = current
            .iter()
            .copied()
            .filter(|neighbor| !protected.contains(neighbor))
            .collect::<Vec<_>>();
        let mut kept = protected;
        kept.extend(self.select_diverse_neighbors(
            &center_vector,
            &candidates,
            M.saturating_sub(kept.len()),
        ));
        let removed = current
            .into_iter()
            .filter(|node| !kept.contains(node))
            .collect::<Vec<_>>();

        self.capture_edges(center, layer, new_node, undo);
        self.nodes[center].edges[layer] = kept;
        for removed_neighbor in removed {
            self.capture_edges(removed_neighbor, layer, new_node, undo);
            self.nodes[removed_neighbor].edges[layer].retain(|&node| node != center);
        }
    }

    fn select_diverse_neighbors(
        &self,
        center: &[f32],
        candidates: &[usize],
        limit: usize,
    ) -> Vec<usize> {
        let mut ranked = candidates
            .iter()
            .copied()
            .map(|node| Candidate {
                node,
                distance: self.distance_between(center, node),
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| candidate_cmp(left, right, &self.nodes));
        ranked.dedup_by_key(|candidate| candidate.node);

        let mut selected: Vec<usize> = Vec::with_capacity(limit.min(ranked.len()));
        let mut rejected = Vec::new();
        for candidate in ranked {
            let diverse = selected.iter().all(|&existing| {
                let candidate_vector = self
                    .storage
                    .get(self.nodes[candidate.node].id)
                    .expect("graph nodes always have storage");
                let between = 1.0
                    - cosine_similarity(
                        candidate_vector,
                        self.storage
                            .get(self.nodes[existing].id)
                            .expect("graph nodes always have storage"),
                    );
                between >= candidate.distance
            });
            if diverse && selected.len() < limit {
                selected.push(candidate.node);
            } else {
                rejected.push(candidate.node);
            }
        }
        if selected.len() < limit {
            selected.extend(rejected.into_iter().take(limit - selected.len()));
        }
        selected
    }

    fn distance_between(&self, vector: &[f32], node: usize) -> f32 {
        1.0 - cosine_similarity(
            vector,
            self.storage
                .get(self.nodes[node].id)
                .expect("graph nodes always have storage"),
        )
    }

    fn capture_edges(&self, node: usize, layer: usize, new_node: usize, undo: &mut InsertUndo) {
        if node >= new_node || undo.has_snapshot(node, layer) {
            return;
        }
        undo.edges.push(EdgeSnapshot {
            node,
            layer,
            edges: self.nodes[node].edges[layer].clone(),
        });
    }

    #[cfg(test)]
    fn graph_signature(&self) -> Vec<(u64, usize, [Vec<u64>; MAX_LAYERS])> {
        self.nodes
            .iter()
            .map(|node| {
                (
                    node.id,
                    node.level,
                    std::array::from_fn(|layer| {
                        node.edges[layer]
                            .iter()
                            .map(|&neighbor| self.nodes[neighbor].id)
                            .collect()
                    }),
                )
            })
            .collect()
    }
}

fn candidate_cmp(left: &Candidate, right: &Candidate, nodes: &[Node]) -> std::cmp::Ordering {
    left.distance
        .total_cmp(&right.distance)
        .then_with(|| nodes[left.node].id.cmp(&nodes[right.node].id))
}

fn insert_candidate(candidates: &mut Vec<Candidate>, candidate: Candidate, nodes: &[Node]) {
    let position = candidates
        .binary_search_by(|existing| candidate_cmp(existing, &candidate, nodes))
        .unwrap_or_else(|position| position);
    candidates.insert(position, candidate);
}

#[derive(Debug, Clone, Copy)]
struct LevelRng {
    state: u64,
}

impl LevelRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^= value >> 31;
        value
    }
}

#[cfg(test)]
mod tests;
