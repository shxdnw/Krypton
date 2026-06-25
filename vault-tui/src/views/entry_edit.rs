use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use secrecy::ExposeSecret;

use crate::app::EntryEditState;

pub fn render(f: &mut Frame, state: &EntryEditState, area: Rect, accent: Color) {
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

    let pw_display = render_password(state.password.expose_secret());

    // Build the form inside the bordered block.
    let fields: [(u16, &str, String); 6] = [
        (0, "Title", state.title.clone()),
        (1, "Username", state.username.clone()),
        (2, "Password", pw_display),
        (3, "URL", state.url.clone()),
        (4, "Notes", state.notes.clone()),
        (5, "Tags", state.tags.clone()),
    ];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(inner);

    for (idx, (field_idx, label, value)) in fields.iter().enumerate() {
        let border_color = if *field_idx == state.active_field as u16 {
            accent
        } else {
            Color::DarkGray
        };
        let display = if value.is_empty() {
            format!("<{label}>")
        } else {
            value.clone()
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
        // Password strength bar below the password field.
        if *field_idx == 2 && !state.password.expose_secret().is_empty() {
            let pw = state.password.expose_secret();
            let score = password_strength(pw);
            let (bar, color) = match score {
                0..=1 => ("\u{2588}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591} Weak", Color::Red),
                2 => ("\u{2588}\u{2588}\u{2588}\u{2588}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591} Fair", Color::Yellow),
                3 => ("\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2591}\u{2591}\u{2591} Good", Color::Cyan),
                _ => ("\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588}\u{2588} Strong", Color::Green),
            };
            let bar_text = Line::from(Span::styled(bar, Style::default().fg(color)));
            let bar_area = Rect { y: chunks[idx].bottom(), height: 1, x: chunks[idx].x + 1, width: chunks[idx].width.saturating_sub(2) };
            f.render_widget(Paragraph::new(bar_text), bar_area);
        }
    }

    // Bottom bar.
    let hint = Paragraph::new(
        "[Tab] next  [Shift+Tab] prev  [Ctrl+G] generate pw  [Ctrl+S] save  [Esc] discard",
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

fn password_strength(pw: &str) -> u8 {
    let len = pw.len();
    let has_upper = pw.chars().any(|c| c.is_uppercase());
    let has_lower = pw.chars().any(|c| c.is_lowercase());
    let has_digit = pw.chars().any(|c| c.is_ascii_digit());
    let has_sym = pw.chars().any(|c| c.is_ascii_punctuation());
    let classes = has_upper as u8 + has_lower as u8 + has_digit as u8 + has_sym as u8;
    if len >= 16 && classes >= 3 { 4 }
    else if len >= 12 && classes >= 2 { 3 }
    else if len >= 8 && classes >= 1 { 2 }
    else { 1 }
}

fn render_password(pw: &str) -> String {
    if pw.is_empty() {
        String::new()
    } else {
        "\u{2022}".repeat(pw.len())
    }
}
