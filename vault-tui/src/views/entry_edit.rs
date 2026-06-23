use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::EntryEditState;

pub fn render(f: &mut Frame, state: &EntryEditState, area: Rect) {
    let title_text = if state.id.is_some() {
        "Edit Entry"
    } else {
        "New Entry"
    };
    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let fields: [(u16, &str, &str); 5] = [
        (0, "Title", &state.title),
        (1, "Username", &state.username),
        (2, "Password", &render_password(&state.password)),
        (3, "URL", &state.url),
        (4, "Notes", &state.notes),
    ];

    // We use a simple vertical stack — one paragraph per field.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(inner);

    for (idx, (field_idx, label, value)) in fields.iter().enumerate() {
        let border_color = if *field_idx == state.active_field as u16 {
            Color::Yellow
        } else {
            Color::DarkGray
        };
        let display = if value.is_empty() {
            format!("<{label}>")
        } else {
            value.to_string()
        };
        let p = Paragraph::new(display)
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .title(*label)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            );
        if idx < chunks.len() {
            f.render_widget(p, chunks[idx]);
        }
    }

    let hint = Paragraph::new(
        "[Tab] next field  [Shift+Tab] prev  [Ctrl+S] save  [Esc] discard",
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

fn render_password(pw: &str) -> String {
    if pw.is_empty() {
        String::new()
    } else {
        "•".repeat(pw.len())
    }
}
