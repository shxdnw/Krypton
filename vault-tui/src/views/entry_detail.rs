use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use secrecy::ExposeSecret;

use crate::app::EntryDetailState;

pub fn render(f: &mut Frame, state: &EntryDetailState, area: Rect, _accent: Color) {
    let block = Block::default()
        .title(state.entry.title.as_str())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(_accent));

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

    // Username
    let username = state.entry.username.as_deref().unwrap_or("\u{2014}");
    render_field(f, chunks[0], "Username", username, field_style);

    // Password
    let pw_display = if state.show_password {
        state.entry.password.expose_secret().clone()
    } else {
        "\u{2022}".repeat(12)
    };
    render_field(f, chunks[1], "Password", &pw_display, field_style);

    // URL
    let url = state.entry.url.as_deref().unwrap_or("\u{2014}");
    render_field(f, chunks[2], "URL", url, field_style);

    // Notes
    let notes = state.entry.notes.as_deref().unwrap_or("\u{2014}");
    render_field(f, chunks[3], "Notes", notes, field_style);

    // Tags
    let tags = if state.entry.tags.is_empty() {
        "\u{2014}".into()
    } else {
        state.entry.tags.join(", ")
    };
    render_field(f, chunks[4], "Tags", &tags, field_style);

    // Custom fields
    let mut cf_y = chunks[4].bottom();
    for cf in &state.entry.custom_fields {
        let value = match &cf.value {
            vault_core::FieldValue::Text(v) => v.clone(),
            vault_core::FieldValue::Secret(s) => {
                if state.show_password { s.expose_secret().clone() }
                else { "\u{2022}".repeat(8) }
            }
            vault_core::FieldValue::Totp(secret) => {
                let code = totp_code(secret);
                format!("{code} (TOTP)")
            }
        };
        let cf_content = format!("{}: {value}", cf.label);
        let cf_p = Paragraph::new(cf_content).style(Style::default().fg(Color::White));
        let cf_area = Rect { y: cf_y, height: 1, x: area.x + 1, width: area.width.saturating_sub(2) };
        f.render_widget(cf_p, cf_area);
        cf_y += 1;
    }

    // Timestamps
    let created = format_time(state.entry.created_at);
    let updated = format_time(state.entry.updated_at);
    let timestamp_text = format!("Created: {created}  |  Updated: {updated}");
    let ts_p = Paragraph::new(timestamp_text).style(Style::default().fg(Color::DarkGray));
    let ts_area = Rect { y: cf_y, height: 1, ..area };
    f.render_widget(ts_p, ts_area);

    // Bottom bar.
    let pw_hint = if state.show_password { "hide pw" } else { "show pw" };
    let hint = Paragraph::new(format!(
        "[p] {pw_hint}  [e] edit  [y] copy pw  [u] copy user  [c] copy url  [d] delete  [Esc/q] back"
    ))
    .style(Style::default().fg(Color::Gray))
    .alignment(Alignment::Center);
    let hint_area = Rect {
        y: area.bottom().saturating_sub(1),
        height: 1,
        ..area
    };
    f.render_widget(hint, hint_area);
}

fn totp_code(_secret: &str) -> String {
    "------".into() // placeholder — totp-rs needed for real codes
}

fn format_time(ts: i64) -> String {
    if ts <= 0 { return "\u{2014}".into(); }
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "\u{2014}".into())
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
