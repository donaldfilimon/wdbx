//! Pinned positions on specimen nodes. A node with no position fields is
//! unpinned, and the view lays it out. Moving stores finite coordinates and
//! pins the node; unpinning removes those fields again.
use crate::engine::text;
use crate::host::Host;
use crate::mutate::log;
use crate::{Result, error};
use serde_json::{Value, json};

const LIMIT: f64 = 100.;

/// Accepts a node that has no position, and a pinned node with finite coordinates.
pub fn validate_position(node: &Value) -> Result<()> {
    let present = node.get("pinned").is_some()
        || node.get("x").is_some()
        || node.get("y").is_some()
        || node.get("z").is_some();
    if !present {
        return Ok(());
    }
    if node.get("pinned") != Some(&json!(true)) || !coords_ok(node) {
        return Err(error(
            "Invalid",
            "A node position must be pinned, with each coordinate from -100 to 100.",
        ));
    }
    Ok(())
}

fn coords_ok(node: &Value) -> bool {
    ["x", "y", "z"].into_iter().all(|key| {
        node.get(key)
            .and_then(Value::as_f64)
            .is_some_and(|n| n.is_finite() && n.abs() <= LIMIT)
    })
}

fn node_mut<'a>(state: &'a mut Value, node_ref: &str) -> Result<&'a mut Value> {
    let nodes = state["nodes"]
        .as_array_mut()
        .ok_or_else(|| error("Invalid", "The specimen has no nodes."))?;
    nodes
        .iter_mut()
        .find(|node| text(node, "ref") == node_ref)
        .ok_or_else(|| error("NotFound", "That node is not in the specimen."))
}

/// Stores a position and pins the node.
pub fn move_node(
    h: &Host,
    state: &Value,
    node_ref: &str,
    x: f64,
    y: f64,
    z: f64,
) -> Result<(Value, String)> {
    if ![x, y, z]
        .into_iter()
        .all(|n| n.is_finite() && n.abs() <= LIMIT)
    {
        return Err(error(
            "Invalid",
            "A node position must be pinned, with each coordinate from -100 to 100.",
        ));
    }
    let mut next = state.clone();
    let node = node_mut(&mut next, node_ref)?;
    if node.get("pinned") == Some(&json!(true))
        && node["x"].as_f64() == Some(x)
        && node["y"].as_f64() == Some(y)
        && node["z"].as_f64() == Some(z)
    {
        return Ok((state.clone(), "Node position kept.".into()));
    }
    let name = text(node, "name").to_owned();
    node["x"] = json!(x);
    node["y"] = json!(y);
    node["z"] = json!(z);
    node["pinned"] = json!(true);
    log(h.env, &mut next, "topology", "Node moved", &name);
    Ok((next, "Node moved.".into()))
}

/// Drops a pinned position so the view lays the node out again.
pub fn unpin_node(h: &Host, state: &Value, node_ref: &str) -> Result<(Value, String)> {
    let mut next = state.clone();
    let node = node_mut(&mut next, node_ref)?;
    let had = node.get("pinned").is_some()
        || node.get("x").is_some()
        || node.get("y").is_some()
        || node.get("z").is_some();
    if !had {
        return Ok((state.clone(), "Node unpinned.".into()));
    }
    let name = text(node, "name").to_owned();
    if let Some(map) = node.as_object_mut() {
        map.remove("x");
        map.remove("y");
        map.remove("z");
        map.remove("pinned");
    }
    log(h.env, &mut next, "topology", "Node unpinned", &name);
    Ok((next, "Node unpinned.".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{self, Command};
    use crate::host::FixedHost;
    use crate::mutate;

    fn host_state() -> (FixedHost, Value) {
        let fixed = FixedHost::default();
        let state = mutate::seed(&fixed.host());
        (fixed, state)
    }

    #[test]
    fn move_pins_and_unpin_restores_the_specimen() {
        let (fixed, state) = host_state();
        let node_ref = state["nodes"][0]["ref"].as_str().unwrap().to_owned();
        assert!(state["nodes"][0].get("pinned").is_none());
        let moved = command::apply(
            &fixed.host(),
            &state,
            None,
            Command::MoveNode {
                node_ref: node_ref.clone(),
                x: 1.5,
                y: -2.,
                z: 3.,
            },
        )
        .unwrap();
        let node = &moved.state["nodes"][0];
        assert_eq!(node["pinned"], json!(true));
        assert_eq!(node["x"], json!(1.5));
        assert_eq!(node["y"], json!(-2.));
        assert_eq!(node["z"], json!(3.));
        assert_eq!(moved.message, "Node moved.");
        mutate::validate_records(&moved.state).unwrap();
        let undone = command::apply(
            &fixed.host(),
            &moved.state,
            None,
            Command::Revert { undo: moved.undo },
        )
        .unwrap();
        assert_eq!(undone.state, state);

        let unpinned = command::apply(
            &fixed.host(),
            &moved.state,
            None,
            Command::UnpinNode {
                node_ref: node_ref.clone(),
            },
        )
        .unwrap();
        assert!(unpinned.state["nodes"][0].get("x").is_none());
        assert!(unpinned.state["nodes"][0].get("pinned").is_none());
        let restored = command::apply(
            &fixed.host(),
            &unpinned.state,
            None,
            Command::Revert {
                undo: unpinned.undo,
            },
        )
        .unwrap();
        assert_eq!(restored.state["nodes"][0]["pinned"], json!(true));
    }

    #[test]
    fn a_missing_node_and_a_bad_position_do_not_change_the_specimen() {
        let (fixed, state) = host_state();
        let err = command::apply(
            &fixed.host(),
            &state,
            None,
            Command::MoveNode {
                node_ref: "missing".into(),
                x: 0.,
                y: 0.,
                z: 0.,
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "NotFound");
        let err = command::apply(
            &fixed.host(),
            &state,
            None,
            Command::MoveNode {
                node_ref: state["nodes"][0]["ref"].as_str().unwrap().into(),
                x: 1000.,
                y: 0.,
                z: 0.,
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "Invalid");
        assert_eq!(state["nodes"][0].get("pinned"), None);
    }

    #[test]
    fn a_missing_connect_endpoint_is_not_found_and_leaves_attachments() {
        let (fixed, state) = host_state();
        let before = state["attachments"].clone();
        let err = command::apply(
            &fixed.host(),
            &state,
            None,
            Command::Connect {
                from: "missing".into(),
                to: state["nodes"][0]["ref"].as_str().unwrap().into(),
                bidirectional: false,
                hard: true,
                affinity: 0.5,
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "NotFound");
        assert_eq!(state["attachments"], before);
    }
}
