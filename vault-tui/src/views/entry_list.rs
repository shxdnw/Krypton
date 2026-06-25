use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame,
};
use secrecy::ExposeSecret;

use crate::app::EntryListState;

/// Render the entry list with an optional preview sidebar on the left.
pub fn render(
    f: &mut Frame,
    state: &EntryListState,
    area: Rect,
    accent: Color,
    sidebar_enabled: bool,
    show_row_numbers: bool,
) {
    let hint_height = 1u16;

    if sidebar_enabled {
        let main_area = Rect {
            height: area.height.saturating_sub(hint_height),
            ..area
        };
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
            .split(main_area);

        render_preview(f, state, chunks[0], accent);

        render_table(f, state, chunks[1], accent, show_row_numbers);

        state.table_rect.set(ratatui::layout::Rect {
            y: chunks[1].y + 2, // header + border
            height: chunks[1].height.saturating_sub(3), // header + hint
            x: chunks[1].x,
            width: chunks[1].width,
        });
    } else {
        let table_area = Rect {
            height: area.height.saturating_sub(hint_height),
            ..area
        };
        render_table(f, state, table_area, accent, show_row_numbers);

        state.table_rect.set(ratatui::layout::Rect {
            y: table_area.y + 2,
            height: table_area.height.saturating_sub(3),
            ..table_area
        });
    }

    let hint = Paragraph::new(
        "[Enter] open  [n] new  [e] edit  [d] delete  [y] pw  [u] user  [c] url  [/] search  [s] settings  [L] lock  [q] quit",
    )
    .style(Style::default().fg(Color::Gray))
    .alignment(Alignment::Center);
    let hint_area = Rect {
        y: area.bottom().saturating_sub(hint_height),
        height: hint_height,
        ..area
    };
    f.render_widget(hint, hint_area);
}

/// Render the entries table on the right side.
fn render_table(f: &mut Frame, state: &EntryListState, area: Rect, accent: Color, show_row_numbers: bool) {
    let block = Block::default()
        .title("Entries")
        .borders(Borders::ALL);

    if state.entries.is_empty() {
        let msg = Paragraph::new("No entries yet. Press [n] to create one.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(block);
        f.render_widget(msg, area);
    } else if show_row_numbers {
        let header = Row::new(["#", "Title", "Username", "Updated"])
            .style(Style::default().fg(accent).add_modifier(Modifier::BOLD));
        let rows: Vec<Row> = state.entries.iter().enumerate().map(|(i, e)| {
            let username = e.username.clone().unwrap_or_default();
            Row::new(vec![(i+1).to_string(), truncate(&e.title, 22), truncate(&username, 16), relative_time(e.updated_at)])
        }).collect();
        let widths = [Constraint::Length(4), Constraint::Percentage(38), Constraint::Percentage(28), Constraint::Percentage(30)];
        let mut ts = TableState::default(); ts.select(Some(state.selected));
        let t = Table::new(rows, widths).header(header).block(block)
            .row_highlight_style(Style::default().fg(accent).add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
        f.render_stateful_widget(t, area, &mut ts);
    } else {
        let header = Row::new(["Title", "Username", "Updated"])
            .style(Style::default().fg(accent).add_modifier(Modifier::BOLD));
        let rows: Vec<Row> = state.entries.iter().map(|e| {
            let username = e.username.clone().unwrap_or_default();
            let updated = relative_time(e.updated_at);
            Row::new(vec![truncate(&e.title, 24), truncate(&username, 18), updated])
        }).collect();
        let highlight = Style::default().fg(accent).add_modifier(Modifier::REVERSED);
        let mut table_state = TableState::default(); table_state.select(Some(state.selected));
        let widths = [Constraint::Percentage(40), Constraint::Percentage(30), Constraint::Percentage(30)];
        let table = Table::new(rows, widths).header(header).block(block)
            .row_highlight_style(highlight).highlight_symbol("> ");
        f.render_stateful_widget(table, area, &mut table_state);
    }
}

/// Render the preview sidebar on the left.
fn render_preview(
    f: &mut Frame,
    state: &EntryListState,
    area: Rect,
    accent: Color,
) {
    let block = Block::default()
        .title("Preview")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let entry = match &state.preview_entry {
        Some(e) => e,
        None => {
            let placeholder = Paragraph::new("No entry selected")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(placeholder, inner);
            return;
        }
    };

    let label_style = Style::default().fg(Color::DarkGray);
    let value_style = Style::default().fg(Color::White);
    let title_style = Style::default().fg(accent).add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        truncate(&entry.title, 28),
        title_style,
    )));
    lines.push(Line::from("")); // spacer

    let username = entry.username.as_deref().unwrap_or("\u{2014}");
    lines.push(Line::from(vec![
        Span::styled(" Username: ", label_style),
        Span::styled(username, value_style),
    ]));

    let pw_display = "\u{2022}".repeat(entry.password.expose_secret().len().max(1));
    lines.push(Line::from(vec![
        Span::styled(" Password: ", label_style),
        Span::styled(pw_display, value_style),
    ]));

    let url = entry.url.as_deref().unwrap_or("\u{2014}");
    lines.push(Line::from(vec![
        Span::styled(" URL:      ", label_style),
        Span::styled(truncate(url, 24), value_style),
    ]));

    let notes = entry.notes.as_deref().unwrap_or("\u{2014}");
    let notes_preview = notes.lines().next().unwrap_or("\u{2014}");
    lines.push(Line::from(vec![
        Span::styled(" Notes:    ", label_style),
        Span::styled(truncate(notes_preview, 24), value_style),
    ]));

    let tags = if entry.tags.is_empty() {
        "\u{2014}".into()
    } else {
        entry.tags.join(", ")
    };
    lines.push(Line::from(vec![
        Span::styled(" Tags:     ", label_style),
        Span::styled(truncate(&tags, 24), value_style),
    ]));

    let preview = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(preview, inner);
}

/// Truncate a string to `max` chars, appending "…" if it was cut.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        format!("{}\u{2026}", s.chars().take(max.saturating_sub(1)).collect::<String>())
    } else {
        s.to_string()
    }
}

/// Convert a Unix timestamp to a relative time string.
fn relative_time(ts: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let diff = now - ts;
    if diff < 60 {
        "just now".into()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}
