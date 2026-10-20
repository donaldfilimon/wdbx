//! Applies one [`Command`] by calling the existing edit functions.
use super::{Applied, Command, Undo, undo_between};
use crate::engine::{number, rows, text};
use crate::host::Host;
use crate::mutate::{self, log};
use crate::neural::Network;
use crate::{Result, error};
use serde_json::{Value, json};

/// Applies `command`. On `Err`, `state` and `network` are left unchanged
/// because this function never writes through those references.
pub fn apply(
    h: &Host,
    state: &Value,
    network: Option<&Network>,
    command: Command,
) -> Result<Applied> {
    if let Command::Revert { undo } = command {
        return revert(state, undo);
    }
    if let Command::EditNetwork { edit } = command {
        let current = network.ok_or_else(|| {
            error(
                "Invalid",
                "A network command needs the composition network.",
            )
        })?;
        let next = crate::network_edit::apply(current, edit)?;
        let mut undo = undo_between(state, state);
        undo.network = Some(current.clone());
        return Ok(Applied {
            state: state.clone(),
            network: Some(next),
            message: "Network updated.".into(),
            undo,
        });
    }
    let (next, message) = match command {
        Command::AddNode { input, node_ref } => (
            mutate::add_node(h, state, &input, node_ref.as_deref())?,
            if node_ref.is_some() {
                "Node updated."
            } else {
                "Pattern learned."
            }
            .into(),
        ),
        Command::AddEntry {
            node_ref,
            pattern,
            action,
        } => (
            mutate::add_entry(h, state, &node_ref, &pattern, &action)?,
            "Entry added.".into(),
        ),
        Command::RemoveNode { node_ref } => {
            if !rows(state, "nodes")
                .iter()
                .any(|n| text(n, "ref") == node_ref)
            {
                return Err(error("NotFound", "That node is not in the specimen."));
            }
            (
                mutate::remove_node(h, state, &node_ref),
                "Node removed.".into(),
            )
        }
        Command::SaveResource {
            input,
            resource_ref,
        } => (
            mutate::save_resource(h, state, &input, resource_ref.as_deref())?,
            "Memory saved.".into(),
        ),
        Command::RemoveResource { resource_ref } => (
            remove_resource(h, state, &resource_ref)?,
            "Memory removed.".into(),
        ),
        Command::Feedback {
            cycle_id,
            right,
            color,
        } => {
            let id = if cycle_id.is_empty() {
                rows(state, "history")
                    .last()
                    .and_then(|c| c["id"].as_str())
                    .ok_or_else(|| error("Invalid", "Run a cycle before giving feedback."))?
                    .to_owned()
            } else {
                cycle_id
            };
            let (next, message) = mutate::feedback(h, state, &id, right, color.as_deref())?;
            (next, message)
        }
        Command::TogglePin { cycle_id } => (
            mutate::toggle_pin(h, state, &cycle_id)?,
            "Pin updated.".into(),
        ),
        Command::SetSettings { settings, name } => (
            set_settings(h, state, settings, name)?,
            "Settings saved.".into(),
        ),
        Command::Connect {
            from,
            to,
            bidirectional,
            hard,
            affinity,
        } => (
            connect(h, state, &from, &to, bidirectional, hard, affinity)?,
            "Attachment added.".into(),
        ),
        Command::UpdateAttachment {
            id,
            bidirectional,
            hard,
            affinity,
        } => (
            update_attachment(h, state, &id, bidirectional, hard, affinity)?,
            "Attachment updated.".into(),
        ),
        Command::Disconnect { id } => (disconnect(h, state, &id)?, "Attachment removed.".into()),
        Command::MoveNode { node_ref, x, y, z } => {
            crate::place::move_node(h, state, &node_ref, x, y, z)?
        }
        Command::UnpinNode { node_ref } => crate::place::unpin_node(h, state, &node_ref)?,
        Command::CreateNote {
            title,
            body,
            tags,
            id,
        } => crate::notes::create(h, state, &title, &body, &tags, id.as_deref())?,
        Command::EditNote { id, body } => crate::notes::edit(h, state, &id, &body)?,
        Command::RenameNote { id, title } => crate::notes::rename(h, state, &id, &title)?,
        Command::DeleteNote { id, force } => crate::notes::delete(h, state, &id, force)?,
        Command::TagNote { id, tags } => crate::notes::tag(h, state, &id, &tags)?,
        Command::EditNetwork { .. } | Command::Revert { .. } => unreachable!(),
    };
    Ok(Applied {
        undo: undo_between(state, &next),
        state: next,
        network: None,
        message,
    })
}

fn revert(state: &Value, undo: Undo) -> Result<Applied> {
    let mut next = state.clone();
    for item in &undo.restore {
        next[&item.key] = item.value.clone();
    }
    for key in &undo.drop_keys {
        if let Some(map) = next.as_object_mut() {
            map.remove(key);
        }
    }
    if let Some(id) = &undo.drop_event {
        let events = next["events"]
            .as_array_mut()
            .ok_or_else(|| error("Invalid", "The specimen has no event log."))?;
        if events.first().and_then(|e| e["id"].as_str()) != Some(id.as_str()) {
            return Err(error(
                "Invalid",
                "The undo event is no longer the latest change.",
            ));
        }
        events.remove(0);
    }
    next["updatedAt"] = json!(undo.updated_at);
    let network = undo.network.clone();
    Ok(Applied {
        state: next,
        network,
        message: "Undone.".into(),
        undo: Undo {
            restore: Vec::new(),
            drop_event: None,
            drop_keys: Vec::new(),
            updated_at: text(state, "updatedAt").to_owned(),
            network: None,
        },
    })
}

fn remove_resource(h: &Host, state: &Value, resource_ref: &str) -> Result<Value> {
    let mut next = state.clone();
    let list = next["resources"]
        .as_array_mut()
        .ok_or_else(|| error("Invalid", "The specimen has no memory."))?;
    let Some(index) = list.iter().position(|r| text(r, "ref") == resource_ref) else {
        return Err(error("NotFound", "That memory is not in the specimen."));
    };
    let removed = list.remove(index);
    log(
        h.env,
        &mut next,
        "memory",
        "Memory removed",
        text(&removed, "text"),
    );
    Ok(next)
}

fn endpoint<'a>(state: &'a Value, node_ref: &str) -> Result<&'a Value> {
    rows(state, "nodes")
        .iter()
        .find(|n| text(n, "ref") == node_ref)
        .ok_or_else(|| error("NotFound", "Choose a node that exists."))
}

fn connect(
    h: &Host,
    state: &Value,
    from: &str,
    to: &str,
    bidirectional: bool,
    hard: bool,
    affinity: f64,
) -> Result<Value> {
    if from == to {
        return Err(error("Invalid", "Choose two different nodes."));
    }
    endpoint(state, from)?;
    endpoint(state, to)?;
    if !affinity.is_finite() || !(0. ..=1.).contains(&affinity) {
        return Err(error("Invalid", "Attachment affinity must be from 0 to 1."));
    }
    if rows(state, "attachments")
        .iter()
        .any(|a| text(a, "from") == from && text(a, "to") == to)
    {
        return Err(error("Invalid", "Those nodes are already attached."));
    }
    let mut next = state.clone();
    next["attachments"]
        .as_array_mut()
        .ok_or_else(|| error("Invalid", "The specimen has no attachments."))?
        .push(json!({
            "id": h.env.uid(),
            "from": from,
            "to": to,
            "bidirectional": bidirectional,
            "hard": hard,
            "affinity": affinity,
        }));
    log(
        h.env,
        &mut next,
        "attachment",
        "Nodes attached",
        "A semantic handoff was configured.",
    );
    Ok(next)
}

fn update_attachment(
    h: &Host,
    state: &Value,
    id: &str,
    bidirectional: bool,
    hard: bool,
    affinity: f64,
) -> Result<Value> {
    if !affinity.is_finite() || !(0. ..=1.).contains(&affinity) {
        return Err(error("Invalid", "Attachment affinity must be from 0 to 1."));
    }
    let mut next = state.clone();
    let list = next["attachments"]
        .as_array_mut()
        .ok_or_else(|| error("Invalid", "The specimen has no attachments."))?;
    let Some(attachment) = list.iter_mut().find(|a| text(a, "id") == id) else {
        return Err(error("NotFound", "That attachment is not in the specimen."));
    };
    attachment["bidirectional"] = json!(bidirectional);
    attachment["hard"] = json!(hard);
    attachment["affinity"] = json!(affinity);
    log(
        h.env,
        &mut next,
        "attachment",
        "Attachment updated",
        "Affinity and handoff were revised.",
    );
    Ok(next)
}

fn disconnect(h: &Host, state: &Value, id: &str) -> Result<Value> {
    let mut next = state.clone();
    let list = next["attachments"]
        .as_array_mut()
        .ok_or_else(|| error("Invalid", "The specimen has no attachments."))?;
    let before = list.len();
    list.retain(|a| text(a, "id") != id);
    if list.len() == before {
        return Err(error("NotFound", "That attachment is not in the specimen."));
    }
    log(
        h.env,
        &mut next,
        "attachment",
        "Attachment removed",
        "The handoff was disconnected.",
    );
    Ok(next)
}

fn set_settings(h: &Host, state: &Value, settings: Value, name: Option<String>) -> Result<Value> {
    mutate::validate_settings(&settings)?;
    let mut next = state.clone();
    if let Some(name) = name {
        let name = name.trim();
        if name.is_empty() {
            return Err(error("Invalid", "Enter a specimen name."));
        }
        if name.chars().count() > 100 {
            return Err(error(
                "Invalid",
                "Keep the specimen name under 100 characters.",
            ));
        }
        next["name"] = json!(name);
    }
    let entry_limit = number(&settings, "entryLimit", 20.);
    let max_strength = number(&settings, "maxStrength", 10.);
    if rows(&next, "nodes").iter().any(|n| {
        rows(n, "entries").len() as f64 > entry_limit || number(n, "strength", 0.) > max_strength
    }) {
        return Err(error(
            "Invalid",
            "Existing nodes exceed those limits. Adjust them before reducing the limits.",
        ));
    }
    next["settings"] = settings;
    log(
        h.env,
        &mut next,
        "settings",
        "Settings saved",
        "Boundaries updated.",
    );
    Ok(next)
}
