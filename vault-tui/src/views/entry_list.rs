use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::EntryListState;

/// Render the entry list as a table with highlight on the selected row.
pub fn render(f: &mut Frame, state: &EntryListState, area: Rect, accent: Color) {
    // Store table body rect for mouse hit-testing (skip header row + hint bar).
    state.table_rect.set(ratatui::layout::Rect {
        y: area.y + 2, // header + border
        height: area.height.saturating_sub(3), // header + hint
        ..area
    });
    let header = Row::new(["Title", "Username", "Updated"])
        .style(Style::default().fg(accent).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = state
        .entries
        .iter()
        .map(|e| {
            let username = e.username.clone().unwrap_or_default();
            let updated = relative_time(e.updated_at);
            Row::new(vec![e.title.clone(), username, updated])
        })
        .collect();

    let highlight = Style::default()
        .fg(accent)
        .add_modifier(Modifier::REVERSED);

    let mut table_state = TableState::default();
    table_state.select(Some(state.selected));

    let widths = [
        Constraint::Percentage(40),
        Constraint::Percentage(30),
        Constraint::Percentage(30),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title("Entries")
                .borders(Borders::ALL),
        )
        .row_highlight_style(highlight)
        .highlight_symbol("> ");

    f.render_stateful_widget(table, area, &mut table_state);

    // Bottom bar.
    let hint = Paragraph::new(
        "[Enter] open  [n] new  [e] edit  [d] delete  [y] copy pw  [/] search  [s] settings  [L] lock  [q] quit",
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
