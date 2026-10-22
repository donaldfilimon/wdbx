//! `specimen-tui` opens the desktop app's store and runs the same commands.
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use specimen_core::{Result, error};
use specimen_tui::{Session, app_data_dir, draw};
use std::time::Duration;

fn main() {
    if let Err(err) = run() {
        eprintln!("{}: {}", err.code, err.message);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut session = Session::open(app_data_dir())?;
    let mut terminal = ratatui::try_init().map_err(|err| error("Storage", err))?;
    let result = event_loop(&mut terminal, &mut session);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut ratatui::DefaultTerminal, session: &mut Session) -> Result<()> {
    let mut input = String::new();
    loop {
        terminal.draw(|frame| draw(frame, session, &input))?;
        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
            KeyCode::Esc => break,
            KeyCode::Enter => {
                let line = std::mem::take(&mut input);
                if !line.trim().is_empty() {
                    // The session records the code and message in its log.
                    drop(session.submit(&line));
                }
            }
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Char(ch) => input.push(ch),
            _ => {}
        }
    }
    Ok(())
}
