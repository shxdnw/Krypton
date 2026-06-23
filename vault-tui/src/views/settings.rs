use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{SettingValue, SettingsState};

pub fn render(f: &mut Frame, state: &SettingsState, area: Rect) {
    let block = Block::default()
        .title("Settings")
        .borders(Borders::ALL);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = state
        .fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let value_str = match &field.value {
                SettingValue::Bool(v) => {
                    if *v { "[x]" } else { "[ ]" }.to_string()
                }
                SettingValue::Number(n) => n.to_string(),
                SettingValue::String(s) => s.clone(),
                SettingValue::Choice { options, selected } => {
                    format!("{}", options.get(*selected).unwrap_or(&"?"))
                }
            };
            let line = format!("  {}: {}", field.label, value_str);
            if i == state.selected {
                ListItem::new(line).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::REVERSED),
                )
            } else {
                ListItem::new(line)
            }
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(list, inner);

    let hint = Paragraph::new(
        "[j/k] navigate  [Enter/Space] toggle  [Ctrl+S] save  [Esc] back",
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
