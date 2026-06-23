use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::SearchState;

pub fn render(f: &mut Frame, state: &SearchState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Search bar.
    let query_display = if state.query.is_empty() {
        "Type to search...".into()
    } else {
        state.query.clone()
    };
    let search_bar = Paragraph::new(query_display)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Search")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
    f.render_widget(search_bar, chunks[0]);

    // Results list.
    let items: Vec<ListItem> = state
        .results
        .iter()
        .map(|e| {
            let title = &e.title;
            let username = e.username.as_deref().unwrap_or("");
            ListItem::new(format!("{title}  ({username})"))
        })
        .collect();

    let highlight = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::REVERSED);

    let list = List::new(items)
        .highlight_style(highlight)
        .highlight_symbol("> ")
        .block(Block::default().borders(Borders::ALL).title("Results"));

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(state.selected));

    f.render_stateful_widget(list, chunks[1], &mut list_state);

    // Bottom bar.
    let hint = Paragraph::new("[Esc] back  [j/k] navigate  [Enter] open")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    let hint_area = Rect {
        y: area.bottom().saturating_sub(1),
        height: 1,
        ..area
    };
    f.render_widget(hint, hint_area);
}
