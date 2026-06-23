use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::SettingsState;

pub fn render(f: &mut Frame, state: &SettingsState, area: Rect, accent: Color) {
    let block = Block::default()
        .title("Settings")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = (0..state.len())
        .map(|i| {
            let (label, value) = state.row(i);
            let line = format!("  {}: {}", label, value);
            if i == state.selected {
                ListItem::new(line).style(
                    Style::default()
                        .fg(accent)
                        .add_modifier(Modifier::REVERSED),
                )
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let list =
        List::new(items).block(Block::default().borders(Borders::NONE));

    f.render_widget(list, inner);

    // Bottom bar.
    let hint = if state.editing_number {
        "[Enter] confirm  [Esc] cancel"
    } else {
        "[j/k] navigate  [Enter/Space] toggle/edit  [Ctrl+S] save  [Esc] back"
    };
    let hint_p = Paragraph::new(hint)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    let hint_area = Rect {
        y: area.bottom().saturating_sub(1),
        height: 1,
        ..area
    };
    f.render_widget(hint_p, hint_area);
}
