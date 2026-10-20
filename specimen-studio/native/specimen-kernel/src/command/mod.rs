//! Specimen commands shared by the composer, the console, and the terminal.
//!
//! `apply` wraps the existing edit functions. A failed command returns the
//! error and leaves the caller's specimen untouched. The returned [`Undo`]
//! restores the previous specimen, including the event the command prepended,
//! so a round trip matches the specimen digest. It stores the previous value
//! of each top-level key the command changed, not a second copy of history.
mod apply;
mod parse;

use crate::engine::{rows, text};
use crate::neural::Network;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use apply::apply;
pub use parse::{SPELLINGS, help_text, parse};

/// One slash-command spelling and the line `/help` prints for it.
pub struct Spelling {
    pub name: &'static str,
    pub summary: &'static str,
}

/// What `parse` understood. Shell directives open a visible editor or chooser;
/// the kernel does not invent that interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Parsed {
    Prompt { text: String },
    Help { text: String },
    Undo,
    Directive { directive: Directive },
    Command { command: Box<Command> },
    Query { query: crate::notes::Query },
}

/// A composer command that must stay a visible, reviewable step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "name", rename_all = "camelCase")]
pub enum Directive {
    AddPattern,
    LoadSpecimen,
    SaveSpecimen,
    /// `id` is a proposal id or exact proposal text. Absent means the first
    /// pending proposal, which the shell resolves.
    Learn {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    ShowProvenance,
}

/// An edit of the specimen or of the composition network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "command",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    AddNode {
        input: crate::mutate::NodeInput,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_ref: Option<String>,
    },
    AddEntry {
        node_ref: String,
        pattern: String,
        action: String,
    },
    RemoveNode {
        node_ref: String,
    },
    SaveResource {
        input: crate::mutate::ResourceInput,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resource_ref: Option<String>,
    },
    RemoveResource {
        resource_ref: String,
    },
    /// An empty `cycle_id` means the latest cycle.
    Feedback {
        #[serde(default)]
        cycle_id: String,
        right: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
    TogglePin {
        cycle_id: String,
    },
    SetSettings {
        settings: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Connect {
        from: String,
        to: String,
        bidirectional: bool,
        hard: bool,
        affinity: f64,
    },
    UpdateAttachment {
        id: String,
        bidirectional: bool,
        hard: bool,
        affinity: f64,
    },
    Disconnect {
        id: String,
    },
    MoveNode {
        node_ref: String,
        x: f64,
        y: f64,
        z: f64,
    },
    UnpinNode {
        node_ref: String,
    },
    EditNetwork {
        edit: crate::network_edit::NetworkCommand,
    },
    /// Produced by `apply`. Not accepted from `parse`.
    Revert {
        undo: Undo,
    },
    CreateNote {
        title: String,
        #[serde(default)]
        body: String,
        #[serde(default)]
        tags: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    EditNote {
        id: String,
        body: String,
    },
    RenameNote {
        id: String,
        title: String,
    },
    DeleteNote {
        id: String,
        #[serde(default)]
        force: bool,
    },
    TagNote {
        id: String,
        tags: Vec<String>,
    },
    DismissProposal {
        id: String,
    },
    ReconsiderProposal {
        id: String,
    },
}

/// Previous values needed to reverse one successful command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Undo {
    /// Top-level specimen keys whose values this command replaced.
    pub restore: Vec<Restore>,
    /// The event id this command prepended, when that was the only event change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drop_event: Option<String>,
    /// Top-level keys that did not exist before the command. Revert removes them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drop_keys: Vec<String>,
    pub updated_at: String,
    /// The composition network from before an `EditNetwork` command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
}

/// One top-level specimen key and the value it held before the command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Restore {
    pub key: String,
    pub value: Value,
}

/// The specimen and, when the command edited it, the next composition network.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    pub state: Value,
    /// `None` when the command did not edit the network. Callers keep their copy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    pub message: String,
    pub undo: Undo,
}

/// Previous values of the top-level keys `after` changed, plus a single
/// prepended event when that is the whole event-log difference.
pub fn undo_between(before: &Value, after: &Value) -> Undo {
    let mut restore = Vec::new();
    let mut drop_event = None;
    let mut drop_keys = Vec::new();
    let mut keys: Vec<&String> = Vec::new();
    for side in [before, after] {
        if let Some(map) = side.as_object() {
            for key in map.keys() {
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
    }
    for key in keys {
        if key == "updatedAt" || before[key] == after[key] {
            continue;
        }
        if before.get(key).is_none() {
            drop_keys.push((*key).clone());
            continue;
        }
        if key == "events" && prepended_only(before, after) {
            drop_event = after["events"][0]["id"].as_str().map(str::to_owned);
            continue;
        }
        restore.push(Restore {
            key: key.clone(),
            value: before[key].clone(),
        });
    }
    Undo {
        restore,
        drop_event,
        drop_keys,
        updated_at: text(before, "updatedAt").to_owned(),
        network: None,
    }
}

fn prepended_only(before: &Value, after: &Value) -> bool {
    let previous = rows(before, "events");
    let next = rows(after, "events");
    if next.is_empty() {
        return false;
    }
    let kept = next.len() - 1;
    kept <= previous.len() && next[1..] == previous[..kept]
}

pub fn digest_of(value: &Value) -> String {
    crate::digest(serde_json::to_string(value).unwrap_or_default().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::FixedHost;
    use crate::mutate::{self, NodeInput, ResourceInput};
    use crate::neural::Network;
    use serde_json::json;
    use std::sync::atomic::AtomicBool;

    fn host_state() -> (FixedHost, Value) {
        let fixed = FixedHost::default();
        let state = mutate::seed(&fixed.host());
        (fixed, state)
    }

    fn add(name: &str, pattern: &str, action: &str) -> Command {
        Command::AddNode {
            input: NodeInput {
                name: name.into(),
                pattern: pattern.into(),
                action: action.into(),
                ..Default::default()
            },
            node_ref: None,
        }
    }

    fn round_trip(fixed: &FixedHost, before: &Value, command: Command) -> Applied {
        let h = fixed.host();
        let applied = apply(&h, before, None, command).unwrap();
        let undone = apply(
            &h,
            &applied.state,
            None,
            Command::Revert {
                undo: applied.undo.clone(),
            },
        )
        .unwrap();
        assert_eq!(
            digest_of(before),
            digest_of(&undone.state),
            "undo did not restore the specimen"
        );
        assert!(applied.undo.network.is_none());
        applied
    }

    #[test]
    fn help_lists_every_spelling() {
        let text = help_text();
        for spelling in SPELLINGS {
            assert!(text.contains(spelling.name), "missing {}", spelling.name);
        }
    }

    #[test]
    fn a_rejected_command_does_not_mutate() {
        let (fixed, state) = host_state();
        let snapshot = state.clone();
        let err = apply(&fixed.host(), &state, None, add("", "x", "y")).unwrap_err();
        assert_eq!(err.code, "Invalid");
        assert_eq!(err.message, "Enter a name, pattern, and action.");
        assert_eq!(state, snapshot);
        let err = parse("/nope").unwrap_err();
        assert_eq!(err.code, "UnknownCommand");
        let err = parse("/right green extra").unwrap_err();
        assert_eq!(err.code, "ParseError");
    }

    #[test]
    fn add_remove_and_settings_round_trip() {
        let (fixed, state) = host_state();
        let added = round_trip(&fixed, &state, add("Color", "favorite color", "Teal."));
        assert_eq!(added.state["events"][0]["title"], "Pattern learned");
        let node_ref = added.state["nodes"].as_array().unwrap().last().unwrap()["ref"]
            .as_str()
            .unwrap()
            .to_owned();
        round_trip(
            &fixed,
            &added.state,
            Command::RemoveNode {
                node_ref: node_ref.clone(),
            },
        );
        let missing = apply(
            &fixed.host(),
            &state,
            None,
            Command::RemoveNode {
                node_ref: "missing".into(),
            },
        )
        .unwrap_err();
        assert_eq!(missing.code, "NotFound");

        let mut settings = state["settings"].clone();
        settings["voteThreshold"] = json!(70);
        let saved = round_trip(
            &fixed,
            &state,
            Command::SetSettings {
                settings,
                name: Some("Renamed specimen".into()),
            },
        );
        assert_eq!(saved.state["name"], "Renamed specimen");
        assert_eq!(saved.state["settings"]["voteThreshold"], 70);
    }

    #[test]
    fn resources_attachments_and_feedback_round_trip() {
        let (fixed, state) = host_state();
        let h = fixed.host();
        let saved = round_trip(
            &fixed,
            &state,
            Command::SaveResource {
                input: ResourceInput {
                    subsystem: "dictionary".into(),
                    text: "teal".into(),
                    value: "A blue-green.".into(),
                    valence: 0.2,
                    intensity: 0.4,
                    ..Default::default()
                },
                resource_ref: None,
            },
        );
        let resource_ref = saved.state["resources"].as_array().unwrap().last().unwrap()["ref"]
            .as_str()
            .unwrap()
            .to_owned();
        round_trip(
            &fixed,
            &saved.state,
            Command::RemoveResource { resource_ref },
        );

        let from = state["nodes"][0]["ref"].as_str().unwrap().to_owned();
        let to = state["nodes"][1]["ref"].as_str().unwrap().to_owned();
        let linked = round_trip(
            &fixed,
            &state,
            Command::Connect {
                from: from.clone(),
                to: to.clone(),
                bidirectional: true,
                hard: false,
                affinity: 0.7,
            },
        );
        assert_eq!(linked.state["events"][0]["title"], "Nodes attached");
        let id = linked.state["attachments"][0]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        round_trip(
            &fixed,
            &linked.state,
            Command::UpdateAttachment {
                id: id.clone(),
                bidirectional: false,
                hard: true,
                affinity: 0.4,
            },
        );
        round_trip(&fixed, &linked.state, Command::Disconnect { id });
        let missing = apply(
            &h,
            &state,
            None,
            Command::Connect {
                from: "nope".into(),
                to,
                bidirectional: false,
                hard: false,
                affinity: 0.5,
            },
        )
        .unwrap_err();
        assert_eq!(missing.code, "NotFound");
        let same = apply(
            &h,
            &state,
            None,
            Command::Connect {
                from: from.clone(),
                to: from,
                bidirectional: false,
                hard: false,
                affinity: 0.5,
            },
        )
        .unwrap_err();
        assert_eq!(same.message, "Choose two different nodes.");

        let (cycled, _) = crate::engine::cycle(
            &h,
            &state,
            "What is 2 + 2?",
            &Network::default(),
            &AtomicBool::new(false),
            &|_| {},
        )
        .unwrap();
        let answered = round_trip(
            &fixed,
            &cycled,
            Command::Feedback {
                cycle_id: String::new(),
                right: true,
                color: None,
            },
        );
        assert!(answered.message.contains("contributors"));
        let cycle_id = cycled["history"].as_array().unwrap().last().unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        round_trip(&fixed, &cycled, Command::TogglePin { cycle_id });
        let none = apply(
            &h,
            &state,
            None,
            Command::Feedback {
                cycle_id: String::new(),
                right: false,
                color: None,
            },
        )
        .unwrap_err();
        assert_eq!(none.message, "Run a cycle before giving feedback.");
    }

    #[test]
    fn dismiss_proposal_marks_and_errors() {
        let (fixed, mut state) = host_state();
        state["proposals"] =
            json!([{"id":"p1","pattern":"foo bar","evidence":2,"status":"pending"}]);
        let dismissed = apply(
            &fixed.host(),
            &state,
            None,
            Command::DismissProposal { id: "p1".into() },
        )
        .unwrap();
        assert_eq!(dismissed.state["proposals"][0]["status"], "dismissed");
        let err = apply(
            &fixed.host(),
            &dismissed.state,
            None,
            Command::DismissProposal { id: "p1".into() },
        )
        .unwrap_err();
        assert!(err.message.contains("pending"));
        let notfound = apply(
            &fixed.host(),
            &state,
            None,
            Command::DismissProposal { id: "nope".into() },
        )
        .unwrap_err();
        assert_eq!(notfound.code, "NotFound");
    }

    #[test]
    fn reconsider_proposal_roundtrips() {
        let (fixed, mut state) = host_state();
        state["proposals"] =
            json!([{"id":"p1","pattern":"foo bar","evidence":2,"status":"dismissed"}]);
        let reconsidered = apply(
            &fixed.host(),
            &state,
            None,
            Command::ReconsiderProposal { id: "p1".into() },
        )
        .unwrap();
        assert_eq!(reconsidered.state["proposals"][0]["status"], "pending");
        let err = apply(
            &fixed.host(),
            &reconsidered.state,
            None,
            Command::ReconsiderProposal { id: "p1".into() },
        )
        .unwrap_err();
        assert!(err.message.contains("dismissed"));
    }

    #[test]
    fn network_edit_undo_restores_the_previous_network() {
        let (fixed, state) = host_state();
        let network = Network::default();
        let applied = apply(
            &fixed.host(),
            &state,
            Some(&network),
            Command::EditNetwork {
                edit: crate::network_edit::NetworkCommand::InitWeights { seed: 7 },
            },
        )
        .unwrap();
        assert_ne!(applied.network.as_ref(), Some(&network));
        assert_eq!(digest_of(&applied.state), digest_of(&state));
        let undone = apply(
            &fixed.host(),
            &applied.state,
            applied.network.as_ref(),
            Command::Revert { undo: applied.undo },
        )
        .unwrap();
        assert_eq!(undone.network.as_ref(), Some(&network));
        assert_eq!(digest_of(&undone.state), digest_of(&state));
    }

    #[test]
    fn parse_keeps_the_composer_spellings() {
        assert_eq!(
            parse("What is 2 + 2?").unwrap(),
            Parsed::Prompt {
                text: "What is 2 + 2?".into()
            }
        );
        assert_eq!(
            parse("/prompt  What is 2 + 2?").unwrap(),
            Parsed::Prompt {
                text: "What is 2 + 2?".into()
            }
        );
        match parse("/wrong red").unwrap() {
            Parsed::Command { command } => match *command {
                Command::Feedback {
                    cycle_id,
                    right,
                    color,
                } => {
                    assert!(cycle_id.is_empty());
                    assert!(!right);
                    assert_eq!(color.as_deref(), Some("red"));
                }
                other => panic!("unexpected {other:?}"),
            },
            other => panic!("unexpected {other:?}"),
        }
        match parse("/wrng").unwrap() {
            Parsed::Command { command } => {
                assert!(matches!(*command, Command::Feedback { right: false, .. }))
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(
            parse("/learn proposal text").unwrap(),
            Parsed::Directive {
                directive: Directive::Learn {
                    id: Some("proposal text".into())
                }
            }
        );
        assert!(matches!(parse("/learn").unwrap(), Parsed::Directive { .. }));
        assert!(matches!(
            parse("/addPattern").unwrap(),
            Parsed::Directive { .. }
        ));
        match parse("/dismiss abc-123").unwrap() {
            Parsed::Command { command } => match *command {
                Command::DismissProposal { id } => assert_eq!(id, "abc-123"),
                other => panic!("unexpected {other:?}"),
            },
            other => panic!("unexpected {other:?}"),
        }
        match parse("/reconsider p1").unwrap() {
            Parsed::Command { command } => match *command {
                Command::ReconsiderProposal { id } => assert_eq!(id, "p1"),
                other => panic!("unexpected {other:?}"),
            },
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            parse("/contributorfractal").unwrap(),
            Parsed::Directive {
                directive: Directive::ShowProvenance
            }
        ));
        assert!(matches!(parse("/undo").unwrap(), Parsed::Undo));
        assert!(matches!(parse("/help").unwrap(), Parsed::Help { .. }));
    }
}
