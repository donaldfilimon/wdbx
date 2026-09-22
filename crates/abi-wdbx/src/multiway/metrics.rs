//! Derived structural metrics over a finished multiway run.

use std::collections::HashSet;

use super::{MultiwayResult, Termination};

/// Derived structural metrics over unique transitions.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiwayMetrics {
    /// Number of unique states.
    pub unique_states: u32,
    /// Number of rule applications.
    pub event_count: u32,
    /// Number of distinct source/destination pairs.
    pub unique_transitions: u32,
    /// State discoveries by depth.
    pub states_per_depth: Vec<u32>,
    /// Events by destination depth.
    pub events_per_depth: Vec<u32>,
    /// BFS frontier width by depth.
    pub frontier_width_per_depth: Vec<u32>,
    /// Unique transitions divided by all states.
    pub mean_out_degree: f64,
    /// Largest distinct-destination out-degree.
    pub max_out_degree: u32,
    /// Median distinct-destination out-degree across all states.
    pub median_out_degree: f64,
    /// States with more than one distinct predecessor.
    pub convergent_states: u32,
    /// Unique self-loop transitions.
    pub self_loops: u32,
    /// Whether the unique-transition graph contains a cycle.
    pub has_cycle: bool,
    /// Weakly connected component count.
    pub weakly_connected_components: u32,
    /// Largest payload.
    pub max_payload_bytes: u32,
    /// Mean payload bytes.
    pub mean_payload_bytes: f64,
    /// `states[d+1] / states[d]` while the denominator is nonzero.
    pub growth_rates: Vec<f64>,
    /// Copied stop reason.
    pub termination: Termination,
    /// Whether the frontier was exhausted.
    pub exhaustive: bool,
}

/// Compute deterministic graph and payload metrics.
#[must_use]
pub fn compute_multiway_metrics(result: &MultiwayResult) -> MultiwayMetrics {
    let state_count = result.states.len();
    let transitions: HashSet<(u32, u32)> = result
        .events
        .iter()
        .map(|event| (event.source, event.destination))
        .collect();
    let mut outgoing = vec![HashSet::new(); state_count];
    let mut incoming = vec![HashSet::new(); state_count];
    for &(source, destination) in &transitions {
        if let (Some(out), Some(input)) = (
            outgoing.get_mut(source as usize),
            incoming.get_mut(destination as usize),
        ) {
            out.insert(destination);
            input.insert(source);
        }
    }

    let mut degrees: Vec<u32> = outgoing
        .iter()
        .map(|destinations| bounded_u32(destinations.len()))
        .collect();
    let max_out_degree = degrees.iter().copied().max().unwrap_or(0);
    degrees.sort_unstable();
    let median_out_degree = match degrees.len() {
        0 => 0.0,
        length if length % 2 == 1 => f64::from(degrees[length / 2]),
        length => f64::midpoint(
            f64::from(degrees[length / 2 - 1]),
            f64::from(degrees[length / 2]),
        ),
    };

    let has_cycle = graph_has_cycle(&outgoing);
    let weakly_connected_components =
        weak_component_count(state_count, transitions.iter().copied());
    let total_payload: usize = result.states.iter().map(|state| state.payload.len()).sum();
    let max_payload = result
        .states
        .iter()
        .map(|state| state.payload.len())
        .max()
        .unwrap_or(0);
    let growth_rates = result
        .states_per_depth
        .windows(2)
        .take_while(|pair| pair[0] != 0)
        .map(|pair| f64::from(pair[1]) / f64::from(pair[0]))
        .collect();

    MultiwayMetrics {
        unique_states: bounded_u32(state_count),
        event_count: bounded_u32(result.events.len()),
        unique_transitions: bounded_u32(transitions.len()),
        states_per_depth: result.states_per_depth.clone(),
        events_per_depth: result.events_per_depth.clone(),
        frontier_width_per_depth: result.states_per_depth.clone(),
        mean_out_degree: if state_count == 0 {
            0.0
        } else {
            f64::from(bounded_u32(transitions.len())) / f64::from(bounded_u32(state_count))
        },
        max_out_degree,
        median_out_degree,
        convergent_states: bounded_u32(incoming.iter().filter(|sources| sources.len() > 1).count()),
        self_loops: bounded_u32(
            transitions
                .iter()
                .filter(|(source, destination)| source == destination)
                .count(),
        ),
        has_cycle,
        weakly_connected_components,
        max_payload_bytes: bounded_u32(max_payload),
        mean_payload_bytes: if state_count == 0 {
            0.0
        } else {
            f64::from(bounded_u32(total_payload)) / f64::from(bounded_u32(state_count))
        },
        growth_rates,
        termination: result.termination,
        exhaustive: result.complete,
    }
}

fn graph_has_cycle(outgoing: &[HashSet<u32>]) -> bool {
    fn visit(node: usize, outgoing: &[HashSet<u32>], colors: &mut [u8]) -> bool {
        colors[node] = 1;
        for &child in &outgoing[node] {
            let child = child as usize;
            if colors[child] == 1 || (colors[child] == 0 && visit(child, outgoing, colors)) {
                return true;
            }
        }
        colors[node] = 2;
        false
    }

    let mut colors = vec![0; outgoing.len()];
    (0..outgoing.len()).any(|node| colors[node] == 0 && visit(node, outgoing, &mut colors))
}

fn weak_component_count(state_count: usize, transitions: impl Iterator<Item = (u32, u32)>) -> u32 {
    fn root(parent: &mut [usize], mut node: usize) -> usize {
        while parent[node] != node {
            parent[node] = parent[parent[node]];
            node = parent[node];
        }
        node
    }

    let mut parent: Vec<usize> = (0..state_count).collect();
    for (source, destination) in transitions {
        let source_root = root(&mut parent, source as usize);
        let destination_root = root(&mut parent, destination as usize);
        if source_root != destination_root {
            parent[source_root] = destination_root;
        }
    }
    bounded_u32(
        (0..state_count)
            .filter(|&node| root(&mut parent, node) == node)
            .count(),
    )
}

fn bounded_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
