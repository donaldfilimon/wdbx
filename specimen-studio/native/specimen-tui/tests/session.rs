use ratatui::Terminal;
use ratatui::backend::TestBackend;
use specimen_core::persistence::Store;
use specimen_tui::{Session, draw};

fn frame(session: &Session, input: &str) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
    terminal.draw(|ui| draw(ui, session, input)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

#[test]
fn a_fresh_store_renders_the_seed_and_a_writable_status() {
    let dir = tempfile::tempdir().unwrap();
    let session = Session::open(dir.path()).unwrap();
    assert!(!session.is_read_only());
    assert_eq!(session.revision(), 1);
    let lines = frame(&session, "");
    assert_eq!(lines[0], "Specimen 001  ·  revision 1  ·  15 nodes");
    assert_eq!(
        &lines[1..16],
        [
            "Calculate",
            "Greeting",
            "Repeat",
            "Time",
            "Recall",
            "Imagine",
            "Tone",
            "Negation",
            "Identity",
            "Capability",
            "Help",
            "Teach",
            "Thanks",
            "Farewell",
            "Observer",
        ]
    );
    assert!(lines[16..18].iter().all(|line| line.is_empty()));
    assert_eq!(lines[18], "writable");
    assert_eq!(lines[19], ">");
}

#[test]
fn a_held_writer_lock_opens_read_only_and_refuses_writes() {
    let dir = tempfile::tempdir().unwrap();
    let hold = Store::open(dir.path()).unwrap();
    let mut session = Session::open(dir.path()).unwrap();
    assert!(session.is_read_only());
    assert!(!hold.is_read_only());
    let lines = frame(&session, "");
    assert_eq!(lines[0], "No specimen  ·  revision 0  ·  0 nodes");
    assert_eq!(lines[18], "Read-only · the desktop app holds this store");
    let err = session.submit("/note Locked").unwrap_err();
    assert_eq!(err.code, "ReadOnly");
    assert_eq!(err.message, "This store is open read-only.");
    assert_eq!(session.revision(), 0);
    assert_eq!(hold.snapshot.revision, 0);
    let help = session.submit("/help").unwrap();
    assert!(help.contains("/prompt"));
    assert!(help.contains("/note"));
    let dossier = session.submit("/contributorfractal").unwrap();
    assert_eq!(dossier, "Open this in the desktop app (run dossier).");
    drop(session);
    drop(hold);
    let mut reopened = Session::open(dir.path()).unwrap();
    assert!(!reopened.is_read_only());
    assert_eq!(
        reopened.submit("/note Field notes").unwrap(),
        "Note created."
    );
    assert_eq!(reopened.submit("/undo").unwrap(), "Undone.");
    let err = reopened.submit("/undo").unwrap_err();
    assert_eq!(err.code, "Invalid");
    assert_eq!(err.message, "Nothing to undo.");
}
