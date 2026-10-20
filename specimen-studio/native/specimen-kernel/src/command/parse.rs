//! Slash-command parser. Spellings match the studio composer, including `/wrng`.
use super::{Command, Directive, Parsed, Spelling};
use crate::notes::Query;
use crate::{Result, error};

/// Composer and console spellings, in `/help` order.
pub const SPELLINGS: &[Spelling] = &[
    Spelling {
        name: "/prompt",
        summary: "Run a cycle. Text after the command is the prompt.",
    },
    Spelling {
        name: "/right",
        summary: "Positive feedback for the latest answer, or one segment color.",
    },
    Spelling {
        name: "/wrong",
        summary: "Negative feedback for the latest answer, or one segment color.",
    },
    Spelling {
        name: "/wrng",
        summary: "Alias of /wrong.",
    },
    Spelling {
        name: "/contributorfractal",
        summary: "Show the run dossier for the latest answer.",
    },
    Spelling {
        name: "/saveSpecimen",
        summary: "Save or download the specimen.",
    },
    Spelling {
        name: "/loadSpecimen",
        summary: "Load a specimen.",
    },
    Spelling {
        name: "/addPattern",
        summary: "Open the pattern editor.",
    },
    Spelling {
        name: "/learn",
        summary: "Open a learning proposal by id or exact text, or the first pending one.",
    },
    Spelling {
        name: "/dismiss",
        summary: "Dismiss a learning proposal by id (or exact pattern text).",
    },
    Spelling {
        name: "/dismissProposal",
        summary: "Alias of /dismiss.",
    },
    Spelling {
        name: "/reconsider",
        summary: "Reconsider (re-activate) a dismissed learning proposal by id.",
    },
    Spelling {
        name: "/reconsiderProposal",
        summary: "Alias of /reconsider.",
    },
    Spelling {
        name: "/dismissAll",
        summary: "Dismiss all pending learning proposals.",
    },
    Spelling {
        name: "/dismissAllProposals",
        summary: "Alias of /dismissAll.",
    },
    Spelling {
        name: "/reconsiderAll",
        summary: "Reconsider all dismissed learning proposals.",
    },
    Spelling {
        name: "/reconsiderAllProposals",
        summary: "Alias of /reconsiderAll.",
    },
    Spelling {
        name: "/undo",
        summary: "Undo the last command.",
    },
    Spelling {
        name: "/note",
        summary: "Create a note. The rest of the line is the title.",
    },
    Spelling {
        name: "/backlinks",
        summary: "List notes that link to a title or a node:, pattern:, or memory: target.",
    },
    Spelling {
        name: "/links",
        summary: "List links from a note id or title.",
    },
    Spelling {
        name: "/unresolved",
        summary: "List links whose target is not in the specimen.",
    },
    Spelling {
        name: "/search",
        summary: "Find notes by text. Wrap the text in slashes for a regular expression.",
    },
    Spelling {
        name: "/graph",
        summary: "Describe the note graph. Optional text focuses one note.",
    },
    Spelling {
        name: "/help",
        summary: "List commands.",
    },
];

pub fn help_text() -> String {
    let mut lines = Vec::with_capacity(SPELLINGS.len());
    for spelling in SPELLINGS {
        lines.push(format!("{} — {}", spelling.name, spelling.summary));
    }
    lines.join("\n")
}

/// Parses one composer or console line.
///
/// Text that does not start with `/` is a prompt. An unknown slash command is
/// `UnknownCommand` and must not be run as a cycle.
pub fn parse(line: &str) -> Result<Parsed> {
    let line = line.trim();
    if line.is_empty() {
        return Err(error("ParseError", "Enter a command or a prompt."));
    }
    if !line.starts_with('/') {
        return Ok(Parsed::Prompt {
            text: line.to_owned(),
        });
    }
    let (token, rest) = split_token(line);
    match token {
        "/prompt" => Ok(Parsed::Prompt {
            text: rest.to_owned(),
        }),
        "/right" | "/wrong" | "/wrng" => {
            let color = one_optional(token, rest)?;
            Ok(Parsed::Command {
                command: Box::new(Command::Feedback {
                    cycle_id: String::new(),
                    right: token == "/right",
                    color,
                }),
            })
        }
        "/contributorfractal" => {
            no_args(token, rest)?;
            Ok(Parsed::Directive {
                directive: Directive::ShowProvenance,
            })
        }
        "/saveSpecimen" => {
            no_args(token, rest)?;
            Ok(Parsed::Directive {
                directive: Directive::SaveSpecimen,
            })
        }
        "/loadSpecimen" => {
            no_args(token, rest)?;
            Ok(Parsed::Directive {
                directive: Directive::LoadSpecimen,
            })
        }
        "/addPattern" => {
            no_args(token, rest)?;
            Ok(Parsed::Directive {
                directive: Directive::AddPattern,
            })
        }
        "/learn" => Ok(Parsed::Directive {
            directive: Directive::Learn {
                id: if rest.is_empty() {
                    None
                } else {
                    Some(rest.to_owned())
                },
            },
        }),
        "/dismiss" | "/dismissProposal" => {
            if rest.is_empty() {
                return Err(error(
                    "ParseError",
                    "Enter a proposal id or exact pattern text to dismiss.",
                ));
            }
            Ok(Parsed::Command {
                command: Box::new(Command::DismissProposal {
                    id: rest.to_owned(),
                }),
            })
        }
        "/reconsider" | "/reconsiderProposal" => {
            if rest.is_empty() {
                return Err(error(
                    "ParseError",
                    "Enter a proposal id or exact pattern text to reconsider.",
                ));
            }
            Ok(Parsed::Command {
                command: Box::new(Command::ReconsiderProposal {
                    id: rest.to_owned(),
                }),
            })
        }
        "/dismissAll" | "/dismissAllProposals" => {
            no_args(token, rest)?;
            Ok(Parsed::Command {
                command: Box::new(Command::DismissAllProposals),
            })
        }
        "/reconsiderAll" | "/reconsiderAllProposals" => {
            no_args(token, rest)?;
            Ok(Parsed::Command {
                command: Box::new(Command::ReconsiderAllProposals),
            })
        }
        "/undo" => {
            no_args(token, rest)?;
            Ok(Parsed::Undo)
        }
        "/help" => {
            no_args(token, rest)?;
            Ok(Parsed::Help { text: help_text() })
        }
        "/note" => {
            if rest.is_empty() {
                return Err(error("ParseError", "Enter a title."));
            }
            Ok(Parsed::Command {
                command: Box::new(Command::CreateNote {
                    title: rest.to_owned(),
                    body: String::new(),
                    tags: Vec::new(),
                    id: None,
                }),
            })
        }
        "/backlinks" => {
            if rest.is_empty() {
                return Err(error(
                    "ParseError",
                    "Enter a note title or a node, pattern, or memory target.",
                ));
            }
            Ok(Parsed::Query {
                query: Query::Backlinks {
                    target: rest.to_owned(),
                },
            })
        }
        "/links" => {
            if rest.is_empty() {
                return Err(error("ParseError", "Enter a note id or title."));
            }
            Ok(Parsed::Query {
                query: Query::Outgoing {
                    note: rest.to_owned(),
                },
            })
        }
        "/unresolved" => {
            no_args(token, rest)?;
            Ok(Parsed::Query {
                query: Query::Unresolved,
            })
        }
        "/search" => Ok(Parsed::Query {
            query: search_query(rest)?,
        }),
        "/graph" => Ok(Parsed::Query {
            query: Query::Graph {
                focus: if rest.is_empty() {
                    None
                } else {
                    Some(rest.to_owned())
                },
                depth: if rest.is_empty() { 0 } else { 2 },
            },
        }),
        other => Err(error(
            "UnknownCommand",
            format!("Unknown command {other}. Run /help for the list."),
        )),
    }
}

fn split_token(line: &str) -> (&str, &str) {
    match line.find(char::is_whitespace) {
        Some(i) => (&line[..i], line[i + 1..].trim()),
        None => (line, ""),
    }
}

fn no_args(token: &str, rest: &str) -> Result<()> {
    if rest.is_empty() {
        Ok(())
    } else {
        Err(error(
            "ParseError",
            format!("{token} does not take further text."),
        ))
    }
}

fn search_query(rest: &str) -> Result<crate::notes::Query> {
    if rest.is_empty() {
        return Err(error("ParseError", "Enter text to search for."));
    }
    let regex = rest.len() > 2 && rest.starts_with('/') && rest.ends_with('/');
    let text = if regex {
        &rest[1..rest.len() - 1]
    } else {
        rest
    };
    if text.is_empty() {
        return Err(error("ParseError", "Enter text to search for."));
    }
    Ok(crate::notes::Query::Search {
        text: text.to_owned(),
        regex,
    })
}

fn one_optional(token: &str, rest: &str) -> Result<Option<String>> {
    if rest.is_empty() {
        return Ok(None);
    }
    if rest.split_whitespace().nth(1).is_some() {
        return Err(error(
            "ParseError",
            format!("{token} takes one optional color."),
        ));
    }
    Ok(Some(rest.to_owned()))
}
