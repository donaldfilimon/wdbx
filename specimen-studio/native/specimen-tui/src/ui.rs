//! One frame: specimen summary, log, status banner, command line.
use crate::Session;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::Paragraph;

pub fn draw(frame: &mut Frame, session: &Session, input: &str) {
    let columns = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(frame.area());
    frame.render_widget(Paragraph::new(session.title()), columns[0]);
    let body = session
        .body_lines()
        .into_iter()
        .map(Line::from)
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Text::from(body)), columns[1]);
    let status_style = if session.is_read_only() {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    frame.render_widget(
        Paragraph::new(session.status()).style(status_style),
        columns[2],
    );
    frame.render_widget(Paragraph::new(format!("> {input}")), columns[3]);
}
