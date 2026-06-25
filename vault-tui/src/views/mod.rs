pub mod entry_detail;
pub mod entry_edit;
pub mod entry_list;
pub mod help;
pub mod search;
pub mod settings;
pub mod unlock;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, AppState, ToastKind};

/// Top-level render: dispatches to the correct view and overlays the toast.
pub fn render(app: &App, f: &mut Frame) {
    let accent = app.accent_color();
    let sidebar = app.config.sidebar_enabled;
    let row_nums = app.config.show_row_numbers;
    match &app.state {
        AppState::FirstRun(state) => unlock::render_first_run(f, state, f.area(), accent),
        AppState::Locked(state) => unlock::render_locked(f, state, f.area(), accent),
        AppState::Unlocked(view) => match view {
            crate::app::View::EntryList(state) => entry_list::render(f, state, f.area(), accent, sidebar, row_nums),
            crate::app::View::EntryDetail(state) => entry_detail::render(f, state, f.area(), accent),
            crate::app::View::EntryEdit(state) => entry_edit::render(f, state, f.area(), accent),
            crate::app::View::Search(state) => search::render(f, state, f.area(), accent),
            crate::app::View::Settings(state) => settings::render(f, state, f.area(), accent),
            crate::app::View::Help(_) => help::render(f, f.area()),
        },
    }

    if let Some(ref dialog) = app.confirm_dialog {
        let area = centered_rect(50, 7, f.area());
        f.render_widget(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)), area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(1), Constraint::Length(2)])
            .split(area);
        let msg = Paragraph::new(dialog.message.as_str())
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);
        f.render_widget(msg, chunks[0]);
        let hint = Paragraph::new("[←/→] choose  [Enter] confirm  [Esc] cancel")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(hint, chunks[1]);
        let yes_style = if dialog.selected_yes {
            Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let no_style = if !dialog.selected_yes {
            Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let buttons = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);
        let yes_p = Paragraph::new(format!("  {}  ", dialog.yes_label)).style(yes_style).alignment(Alignment::Center);
        let no_p = Paragraph::new(format!("  {}  ", dialog.no_label)).style(no_style).alignment(Alignment::Center);
        f.render_widget(yes_p, buttons[0]);
        f.render_widget(no_p, buttons[1]);
    }

    if let Some(ref toast) = app.toast {
        let color = match toast.kind {
            ToastKind::Info => Color::Cyan,
            ToastKind::Success => Color::Green,
            ToastKind::Error => Color::Red,
        };
        let area = Rect {
            y: f.area().bottom().saturating_sub(1),
            height: 1,
            ..f.area()
        };
        let toast_p = Paragraph::new(toast.message.as_str())
            .style(Style::default().fg(Color::White).bg(color))
            .alignment(Alignment::Center);
        f.render_widget(toast_p, area);
    }
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Length((r.height.saturating_sub(height)) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup[1])[1]
}
