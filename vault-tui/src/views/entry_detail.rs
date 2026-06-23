use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use secrecy::ExposeSecret;

use crate::app::EntryDetailState;

pub fn render(f: &mut Frame, state: &EntryDetailState, area: Rect) {
    let block = Block::default()
        .title(state.entry.title.as_str())
        .borders(Borders::ALL);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(block.inner(area));

    f.render_widget(block, area);

    let field_style = Style::default().fg(Color::White);

    let username = state.entry.username.as_deref().unwrap_or("\u{2014}");
    render_field(f, chunks[0], "Username", username, field_style);

    let pw_display = if state.show_password {
        state.entry.password.expose_secret().clone()
    } else {
        "\u{2022}".repeat(12)
    };
    render_field(f, chunks[1], "Password", &pw_display, field_style);

    let url = state.entry.url.as_deref().unwrap_or("\u{2014}");
    render_field(f, chunks[2], "URL", url, field_style);

    let notes = state.entry.notes.as_deref().unwrap_or("\u{2014}");
    render_field(f, chunks[3], "Notes", notes, field_style);

    let tags = if state.entry.tags.is_empty() {
        "\u{2014}".into()
    } else {
        state.entry.tags.join(", ")
    };
    render_field(f, chunks[4], "Tags", &tags, field_style);

    let hint = Paragraph::new(
        "[p] show pw  [e] edit  [y] copy pw  [u] copy user  [Esc/q] back",
    )
    .style(Style::default().fg(Color::Gray))
    .alignment(Alignment::Center);
    let hint_area = Rect {
        y: area.bottom().saturating_sub(1),
        height: 1,
        ..area
    };
    f.render_widget(hint, hint_area);
}

fn render_field(
    f: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    value_style: Style,
) {
    let content = format!("{label}: {value}");
    let p = Paragraph::new(content).style(value_style);
    f.render_widget(
        p,
        Block::default().borders(Borders::NONE).inner(area),
    );
}
