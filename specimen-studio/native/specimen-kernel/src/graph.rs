//! Graph-view policy. The webview calls this through WASM. The Tauri process
//! does not expose graphView. Camera and density choices never change stored
//! weights.

use std::collections::BTreeSet;

/// The next orbit preset. Unknown values restart at orbit.
pub fn next_camera(current: &str) -> &'static str {
    match current {
        "orbit" => "top",
        "top" => "side",
        "side" => "orbit",
        _ => "orbit",
    }
}

/// Keep a stored camera name, or fall back to orbit.
pub fn camera(current: &str) -> &'static str {
    match current {
        "top" => "top",
        "side" => "side",
        _ => "orbit",
    }
}

/// Drawn connections stay inside the viewport budget.
pub fn clamp_edge_budget(n: i64) -> i64 {
    n.clamp(80, 600)
}

/// Nodes to draw. `local` keeps the selection and its neighbors; anything else keeps all.
pub fn visible_refs(
    all: &[String],
    selected: Option<&str>,
    edges: &str,
    links: &[(String, String)],
) -> Vec<String> {
    if edges != "local" {
        return all.to_vec();
    }
    let Some(selected) = selected.filter(|s| !s.is_empty()) else {
        return all.to_vec();
    };
    let mut keep = BTreeSet::new();
    keep.insert(selected.to_string());
    for (from, to) in links {
        if from == selected {
            keep.insert(to.clone());
        }
        if to == selected {
            keep.insert(from.clone());
        }
    }
    all.iter().filter(|r| keep.contains(*r)).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_cycle_and_budget_are_bounded() {
        assert_eq!(next_camera("orbit"), "top");
        assert_eq!(next_camera("side"), "orbit");
        assert_eq!(next_camera("nope"), "orbit");
        assert_eq!(clamp_edge_budget(10), 80);
        assert_eq!(clamp_edge_budget(900), 600);
    }

    #[test]
    fn local_view_keeps_neighbors() {
        let all = vec!["a".into(), "b".into(), "c".into()];
        let links = vec![("a".into(), "b".into())];
        assert_eq!(
            visible_refs(&all, Some("a"), "local", &links),
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(visible_refs(&all, None, "local", &links), all);
    }
}
