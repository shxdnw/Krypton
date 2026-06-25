pub mod entry_detail;
pub mod entry_edit;
pub mod entry_list;
pub mod help;
pub mod search;
pub mod settings;
pub mod unlock;

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, AppState, ToastKind};

/// Top-level render: dispatches to the correct view and overlays the toast.
pub fn render(app: &App, f: &mut Frame) {
    let accent = app.accent_color();
    let sidebar = app.config.sidebar_enabled;
    match &app.state {
        AppState::FirstRun(state) => unlock::render_first_run(f, state, f.area(), accent),
        AppState::Locked(state) => unlock::render_locked(f, state, f.area(), accent),
        AppState::Unlocked(view) => match view {
            crate::app::View::EntryList(state) => entry_list::render(f, state, f.area(), accent, sidebar),
            crate::app::View::EntryDetail(state) => entry_detail::render(f, state, f.area(), accent),
            crate::app::View::EntryEdit(state) => entry_edit::render(f, state, f.area(), accent),
            crate::app::View::Search(state) => search::render(f, state, f.area(), accent),
            crate::app::View::Settings(state) => settings::render(f, state, f.area(), accent),
            crate::app::View::Help(_) => help::render(f, f.area()),
        },
    }

    // Toast overlay at the bottom of the screen.
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
