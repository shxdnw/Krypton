use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};

use crate::actions::Action;
use crate::app::{App, AppState, View};

/// Terminal guard that restores the terminal state on drop, even if the
/// application panics.
struct TerminalGuard;

impl TerminalGuard {
    fn setup() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

/// Run the main event loop.
///
/// Polls for keyboard events every 50ms, maps keys to actions, dispatches
/// them through [`App::handle_action`], and re-renders on every iteration.
pub async fn run(app: &mut App) -> color_eyre::Result<()> {
    let _guard = TerminalGuard::setup()?;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(
        io::stdout(),
    ))?;

    loop {
        terminal.draw(|f| crate::views::render(app, f))?;

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if let Some(action) = map_key_to_action(&app.state, key) {
                    app.handle_action(action);
                }
            }
        } else {
            app.handle_action(Action::Tick);
        }
    }

    Ok(())
}

/// Translate a [`KeyEvent`] into an [`Action`] based on the current app state.
fn map_key_to_action(state: &AppState, key: KeyEvent) -> Option<Action> {
    match state {
        AppState::FirstRun(_) | AppState::Locked(_) => {
            map_locked_or_firstrun(key)
        }
        AppState::Unlocked(view) => match view {
            View::EntryList(_) => map_entry_list(key),
            View::EntryDetail(_) => map_entry_detail(key),
            View::EntryEdit(_) => map_entry_edit(key),
            View::Search(_) => map_search(key),
        },
    }
}


fn map_locked_or_firstrun(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char(c) => Some(Action::CharInput(c)),
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Enter => Some(Action::Submit),
        KeyCode::Esc => Some(Action::Quit),
        _ => {
            // Ctrl+H or Ctrl+V to toggle visibility.
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('h') | KeyCode::Char('v') => {
                        Some(Action::ToggleVisibility)
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
    }
}


fn map_entry_list(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
        KeyCode::Enter => Some(Action::Select),
        KeyCode::Char('n') => Some(Action::NewEntry),
        KeyCode::Char('e') => Some(Action::EditEntry),
        KeyCode::Char('d') => Some(Action::DeleteEntry),
        KeyCode::Char('y') => Some(Action::CopyPassword),
        KeyCode::Char('/') => Some(Action::StartSearch),
        KeyCode::Char('L') => Some(Action::Lock),
        KeyCode::Char('g') => Some(Action::PageUp),   // top of list
        KeyCode::Char('G') => Some(Action::PageDown), // bottom of list
        KeyCode::PageUp => Some(Action::PageUp),
        KeyCode::PageDown => Some(Action::PageDown),
        _ => None,
    }
}


fn map_entry_detail(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('p') => Some(Action::ToggleVisibility),
        KeyCode::Char('e') => Some(Action::EditEntry),
        KeyCode::Char('y') => Some(Action::CopyPassword),
        KeyCode::Char('u') => Some(Action::CopyUsername),
        KeyCode::Esc | KeyCode::Char('q') => Some(Action::Back),
        _ => None,
    }
}


fn map_entry_edit(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Tab => Some(Action::NextField),
        KeyCode::BackTab => Some(Action::PrevField),
        KeyCode::Esc => Some(Action::Back),
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Enter => Some(Action::NextField),
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    's' => Some(Action::SaveEntry),
                    _ => Some(Action::CharInput(c)),
                }
            } else {
                Some(Action::CharInput(c))
            }
        }
        _ => None,
    }
}


fn map_search(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc => Some(Action::Back),
        KeyCode::Backspace => Some(Action::Backspace),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::Down),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::Up),
        KeyCode::Enter => Some(Action::Select),
        KeyCode::Char(c) => Some(Action::CharInput(c)),
        _ => None,
    }
}
