use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};
use secrecy::ExposeSecret;

use crate::app::{FirstRunState, LockedState};

/// Render the unlock screen (existing vault, enter password).
pub fn render_locked(f: &mut Frame, state: &LockedState, area: Rect, _accent: Color) {
    let area = centered_rect(60, 10, area);
    let block = Block::default()
        .title("krypton")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let input_len = state.input.expose_secret().len();
    let display = if state.hidden {
        "\u{2022}".repeat(input_len)
    } else {
        state.input.expose_secret().clone()
    };

    let mut text = display;
    if text.is_empty() {
        text = "Enter master password...".into();
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(block.inner(area));

    // Render reset flow if active.
    if let Some(ref step) = state.reset_step {
        let (title, display, hint_text) = match step {
            crate::app::ResetStep::TypingConfirm { buffer } => (
                "Reset Vault",
                format!("Type 'I accept the risks' to confirm:\n{buffer}_"),
                "[Esc] cancel",
            ),
            crate::app::ResetStep::Waiting { seconds } => (
                "Reset Vault",
                format!("Vault will be deleted in {seconds} seconds...\nAll data will be permanently lost."),
                "[Esc] cancel",
            ),
        };
        let reset_block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Red));
        let reset_p = Paragraph::new(display)
            .style(Style::default().fg(Color::Red))
            .block(reset_block);
        let hint = Paragraph::new(hint_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        let rchunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Length(1)])
            .split(area);
        f.render_widget(reset_p, rchunks[0]);
        f.render_widget(hint, rchunks[1]);
        return;
    }

    let input_p = Paragraph::new(text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().title("Password").borders(Borders::ALL));

    f.render_widget(block, area);
    f.render_widget(input_p, chunks[0]);

    if let Some(ref err) = state.error {
        let err_p = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::Red));
        f.render_widget(err_p, chunks[1]);
    }

    let hint = Paragraph::new("[Enter] unlock  [Ctrl+H] show/hide  [Ctrl+R] reset vault  [Esc] quit")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(hint, chunks[2]);
}

/// Render the first-run screen (create master password).
pub fn render_first_run(f: &mut Frame, state: &FirstRunState, area: Rect, _accent: Color) {
    let area = centered_rect(60, 12, area);
    let block = Block::default()
        .title("krypton — Create Master Password")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let is_confirm = state.step == crate::app::FirstRunStep::ConfirmPassword;
    let label = if is_confirm {
        "Confirm Password"
    } else {
        "Master Password"
    };
    let input_text = if is_confirm {
        state.confirm.expose_secret()
    } else {
        state.password.expose_secret()
    };

    let display = "\u{2022}".repeat(input_text.len());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(block.inner(area));

    let input_p = Paragraph::new(display)
        .style(Style::default().fg(Color::White))
        .block(Block::default().title(label).borders(Borders::ALL));

    f.render_widget(block, area);
    f.render_widget(input_p, chunks[0]);

    if let Some(ref err) = state.error {
        let err_p = Paragraph::new(err.as_str())
            .style(Style::default().fg(Color::Red));
        f.render_widget(err_p, chunks[1]);
    }

    let hint = if is_confirm {
        "[Enter] create vault  [Esc] quit"
    } else {
        "[Enter] next  [Esc] quit"
    };
    let hint_p = Paragraph::new(hint)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
    f.render_widget(hint_p, chunks[2]);
}

/// Helper: shrink `area` to a given percent width and absolute height,
/// centered horizontally and vertically.
fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Length((r.height.saturating_sub(height)) / 2),
        ])
        .split(r);

    let horz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1]);

    horz[1]
}
