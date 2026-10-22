//! Terminal session over the desktop specimen store.
//!
//! When the desktop app holds the writer lock, the session opens read-only.
//! Write commands then fail with `ReadOnly` and the status line says so.
mod ui;

use serde_json::Value;
use specimen_core::host::NativeHost;
use specimen_core::persistence::Store;
use specimen_kernel::command::{self, Command, Parsed};
use specimen_kernel::engine::{self, rows, text};
use specimen_kernel::neural::Network;
use specimen_kernel::notes::{self, Answer};
use specimen_kernel::{Result, error};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

pub use ui::draw;

const LOG_LIMIT: usize = 24;
const UNDO_LIMIT: usize = 32;

/// Same directory the desktop app opens, including its debug override.
pub fn app_data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("WDBX_STUDIO_TEST_DATA") {
        return PathBuf::from(path);
    }
    data_dir()
}

fn data_dir() -> PathBuf {
    const ID: &str = "com.donaldfilimon.wdbx-studio";
    #[cfg(target_os = "macos")]
    {
        home().join("Library/Application Support").join(ID)
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home().join(".local/share"))
            .join(ID)
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(ID)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        home().join(ID)
    }
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub struct Session {
    store: Store,
    host: NativeHost,
    undo: Vec<command::Undo>,
    log: Vec<String>,
}

impl Session {
    /// Opens the store. A held writer lock becomes a read-only session.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let store = match Store::open(&root) {
            Ok(store) => store,
            Err(err) if err.code == "WriterBusy" => Store::open_read_only(&root)?,
            Err(err) => return Err(err),
        };
        let mut session = Self {
            store,
            host: NativeHost::new(),
            undo: Vec::new(),
            log: Vec::new(),
        };
        session.ensure_specimen()?;
        Ok(session)
    }

    pub fn is_read_only(&self) -> bool {
        self.store.is_read_only()
    }

    pub fn revision(&self) -> u64 {
        self.store.snapshot.revision
    }

    pub fn log(&self) -> &[String] {
        &self.log
    }

    pub fn status(&self) -> &'static str {
        if self.is_read_only() {
            "Read-only · the desktop app holds this store"
        } else {
            "writable"
        }
    }

    pub fn title(&self) -> String {
        let specimen = &self.store.snapshot.specimen;
        let name = specimen
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("No specimen");
        format!(
            "{name}  ·  revision {}  ·  {} nodes",
            self.revision(),
            rows(specimen, "nodes").len()
        )
    }

    pub fn body_lines(&self) -> Vec<String> {
        let mut lines = rows(&self.store.snapshot.specimen, "nodes")
            .iter()
            .map(|node| text(node, "name").to_owned())
            .collect::<Vec<_>>();
        if !self.log.is_empty() {
            lines.push(String::new());
            lines.extend(self.log.iter().cloned());
        }
        lines
    }

    /// Parses one line, applies it, and records the outcome in the log.
    pub fn submit(&mut self, line: &str) -> Result<String> {
        let outcome = self.dispatch(line);
        let shown = match &outcome {
            Ok(message) => message.clone(),
            Err(err) => format!("{}: {}", err.code, err.message),
        };
        self.push_log(&shown);
        outcome
    }

    fn dispatch(&mut self, line: &str) -> Result<String> {
        match command::parse(line)? {
            Parsed::Help { text } => Ok(text),
            Parsed::Query { query } => {
                let answer = notes::query(&self.store.snapshot.specimen, &query)?;
                Ok(answer_text(&answer).to_owned())
            }
            Parsed::Directive { directive } => Ok(format!(
                "Open this in the desktop app ({}).",
                directive_name(&directive)
            )),
            Parsed::Prompt { text } => self.prompt(&text),
            Parsed::Undo => self.undo(),
            Parsed::Command { command } => self.command(*command),
        }
    }

    fn prompt(&mut self, input: &str) -> Result<String> {
        self.require_write()?;
        let before = self.store.snapshot.specimen.clone();
        let network = self.store.snapshot.network.clone();
        let cancel = AtomicBool::new(false);
        let (next, cycle) = {
            let host = self.host.host();
            engine::cycle(&host, &before, input, &network, &cancel, &|_| {})?
        };
        let undo = command::undo_between(&before, &next);
        self.persist(next, None)?;
        self.push_undo(undo);
        Ok(cycle["segments"]
            .as_array()
            .and_then(|segments| segments.first())
            .map(|segment| text(segment, "text").to_owned())
            .filter(|answer| !answer.is_empty())
            .unwrap_or_else(|| "Cycle completed.".into()))
    }

    fn undo(&mut self) -> Result<String> {
        self.require_write()?;
        let Some(undo) = self.undo.pop() else {
            return Err(error("Invalid", "Nothing to undo."));
        };
        let applied = {
            let host = self.host.host();
            match command::apply(
                &host,
                &self.store.snapshot.specimen,
                Some(&self.store.snapshot.network),
                Command::Revert { undo: undo.clone() },
            ) {
                Ok(applied) => applied,
                Err(err) => {
                    self.undo.push(undo);
                    return Err(err);
                }
            }
        };
        if let Err(err) = self.persist(applied.state, applied.network.clone()) {
            self.undo.push(undo);
            return Err(err);
        }
        Ok(applied.message)
    }

    fn command(&mut self, command: Command) -> Result<String> {
        self.require_write()?;
        let applied = {
            let host = self.host.host();
            command::apply(
                &host,
                &self.store.snapshot.specimen,
                Some(&self.store.snapshot.network),
                command,
            )?
        };
        self.persist(applied.state, applied.network)?;
        self.push_undo(applied.undo);
        Ok(applied.message)
    }

    fn require_write(&self) -> Result<()> {
        if self.is_read_only() {
            Err(error("ReadOnly", "This store is open read-only."))
        } else {
            Ok(())
        }
    }

    fn ensure_specimen(&mut self) -> Result<()> {
        if self.is_read_only() {
            return Ok(());
        }
        if self
            .store
            .snapshot
            .specimen
            .get("schema")
            .and_then(|v| v.as_str())
            == Some("wdbx.studio.v1")
        {
            return Ok(());
        }
        let seeded = {
            let host = self.host.host();
            specimen_kernel::mutate::seed(&host)
        };
        self.persist(seeded, None)
    }

    fn persist(&mut self, specimen: Value, network: Option<Network>) -> Result<()> {
        let expected = self.store.snapshot.revision;
        let mut next = self.store.snapshot.clone();
        next.specimen = specimen;
        if let Some(network) = network {
            next.network = network;
        }
        self.store.commit(expected, next)?;
        Ok(())
    }

    fn push_undo(&mut self, undo: command::Undo) {
        self.undo.insert(0, undo);
        self.undo.truncate(UNDO_LIMIT);
    }

    fn push_log(&mut self, text: &str) {
        for line in text.lines() {
            self.log.push(line.to_owned());
        }
        let extra = self.log.len().saturating_sub(LOG_LIMIT);
        if extra > 0 {
            self.log.drain(0..extra);
        }
    }
}

fn answer_text(answer: &Answer) -> &str {
    match answer {
        Answer::Notes { summary, .. }
        | Answer::Links { summary, .. }
        | Answer::Unresolved { summary, .. }
        | Answer::Graph { summary, .. }
        | Answer::Document { summary, .. } => summary,
    }
}

fn directive_name(directive: &specimen_kernel::command::Directive) -> &'static str {
    use specimen_kernel::command::Directive;
    match directive {
        Directive::AddPattern => "add pattern",
        Directive::LoadSpecimen => "load specimen",
        Directive::SaveSpecimen => "save specimen",
        Directive::Learn { .. } => "learn",
        Directive::ShowProvenance => "run dossier",
    }
}
