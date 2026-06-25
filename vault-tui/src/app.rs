use std::cell::Cell;
use std::sync::Arc;

use secrecy::{ExposeSecret, SecretString};
use vault_core::{Entry, EntryId, EntrySummary};
use vault_service::VaultService;

use crate::actions::Action;
use crate::config::KryptonConfig;

// ── ConfirmAction ─────────────────────────────────────────────────────────

/// A pending destructive action that needs a second confirmation.
#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DeleteEntry(EntryId),
}

// ── Toast ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub kind: ToastKind,
}

// ── Screen state structs ─────────────────────────────────────────────────

/// Master-password input on the unlock screen.
///
/// The password is stored as [`SecretString`] so it is redacted in logs and
/// zeroized when the state is dropped.
#[derive(Debug, Clone)]
pub struct LockedState {
    pub input: SecretString,
    pub hidden: bool,
    pub error: Option<String>,
    #[allow(dead_code)]
    pub loading: bool,
    pub reset_step: Option<ResetStep>,
}

#[derive(Debug, Clone)]
pub enum ResetStep {
    TypingConfirm { buffer: String },
    Waiting { seconds: u64 },
}

impl Default for LockedState {
    fn default() -> Self {
        Self {
            input: SecretString::new("".into()),
            hidden: true,
            error: None,
            loading: false,
            reset_step: None,
        }
    }
}

/// Master-password creation during first run.
#[derive(Debug, Clone)]
pub struct FirstRunState {
    pub step: FirstRunStep,
    pub password: SecretString,
    pub confirm: SecretString,
    pub error: Option<String>,
}

impl Default for FirstRunState {
    fn default() -> Self {
        Self {
            step: FirstRunStep::default(),
            password: SecretString::new("".into()),
            confirm: SecretString::new("".into()),
            error: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum FirstRunStep {
    #[default]
    EnterPassword,
    ConfirmPassword,
}

pub struct EntryListState {
    pub entries: Vec<EntrySummary>,
    pub selected: usize,
    /// Layout rect of the table body for mouse hit-testing.
    pub table_rect: Cell<ratatui::layout::Rect>,
    /// Full entry data for the currently selected item (sidebar preview).
    pub preview_entry: Option<Entry>,
}

// Manual impl — Cell is !Debug + !Clone
impl std::fmt::Debug for EntryListState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntryListState")
            .field("entries", &self.entries)
            .field("selected", &self.selected)
            .field("preview_entry", &self.preview_entry)
            .finish()
    }
}
impl Clone for EntryListState {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            selected: self.selected,
            table_rect: Cell::new(self.table_rect.get()),
            preview_entry: self.preview_entry.clone(),
        }
    }
}

impl Default for EntryListState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            selected: 0,
            table_rect: Cell::new(ratatui::layout::Rect::default()),
            preview_entry: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EntryDetailState {
    pub entry: Entry,
    pub show_password: bool,
}

/// Inline entry editor.
///
/// The `password` field is a [`SecretString`]; characters are appended by
/// exposing the current value, appending, and re-wrapping.
#[derive(Debug, Clone)]
pub struct EntryEditState {
    pub id: Option<EntryId>,
    pub title: String,
    pub username: String,
    pub password: SecretString,
    pub url: String,
    pub notes: String,
    pub active_field: usize,
    pub dirty: bool,
    /// The original password when editing an existing entry.
    /// If the user leaves the password field empty, we keep this.
    pub existing_password: SecretString,
    /// The original created_at timestamp for existing entries.
    pub initial_created_at: i64,
}

impl Default for EntryEditState {
    fn default() -> Self {
        Self {
            id: None,
            title: String::new(),
            username: String::new(),
            password: SecretString::new("".into()),
            url: String::new(),
            notes: String::new(),
            active_field: 0,
            dirty: false,
            existing_password: SecretString::new("".into()),
            initial_created_at: 0,
        }
    }
}

pub struct SearchState {
    pub query: String,
    pub results: Vec<EntrySummary>,
    pub selected: usize,
    pub result_rect: Cell<ratatui::layout::Rect>,
}
impl std::fmt::Debug for SearchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchState")
            .field("query", &self.query)
            .field("results", &self.results)
            .field("selected", &self.selected)
            .finish()
    }
}
impl Clone for SearchState {
    fn clone(&self) -> Self {
        Self {
            query: self.query.clone(),
            results: self.results.clone(),
            selected: self.selected,
            result_rect: Cell::new(self.result_rect.get()),
        }
    }
}
impl Default for SearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            results: Vec::new(),
            selected: 0,
            result_rect: Cell::new(ratatui::layout::Rect::default()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingsState {
    /// Working copy of the config being edited.
    pub config: KryptonConfig,
    /// Index of the currently selected setting row.
    pub selected: usize,
    /// When true, we're editing a numeric field (clipboard timeout).
    pub editing_number: bool,
    /// Buffer for numeric input.
    pub number_buffer: String,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            config: KryptonConfig::default(),
            selected: 0,
            editing_number: false,
            number_buffer: String::new(),
        }
    }
}

impl SettingsState {
    /// Number of setting rows.
    pub fn len(&self) -> usize { 12 }

    /// Get the display label and value string for row `i`.
    pub fn row(&self, i: usize) -> (String, String) {
        match i {
            0 => ("Hide metadata in list".into(), fmt_bool(self.config.hide_metadata)),
            1 => ("Clipboard timeout (s)".into(), {
                if self.editing_number && self.selected == 1 {
                    format!("{}_", self.number_buffer)
                } else {
                    self.config.clipboard_timeout_secs.to_string()
                }
            }),
            2 => ("Confirm before delete".into(), fmt_bool(self.config.confirm_before_delete)),
            3 => ("Password length".into(), self.config.password_length.to_string()),
            4 => ("Uppercase".into(), fmt_bool(self.config.password_uppercase)),
            5 => ("Lowercase".into(), fmt_bool(self.config.password_lowercase)),
            6 => ("Digits".into(), fmt_bool(self.config.password_digits)),
            7 => ("Symbols".into(), fmt_bool(self.config.password_symbols)),
            8 => ("Relative timestamps".into(), fmt_bool(self.config.relative_timestamps)),
            9 => ("Sidebar enabled".into(), fmt_bool(self.config.sidebar_enabled)),
            10 => ("Vim keybinds (j/k)".into(), fmt_bool(self.config.vim_keybinds)),
            11 => ("Clipboard tool".into(), self.config.clipboard_tool.clone()),
            _ => ("".into(), "".into()),
        }
    }

    /// Toggle the boolean at row `i`, if it's a boolean field.
    pub fn toggle(&mut self, i: usize) {
        match i {
            0 => self.config.hide_metadata = !self.config.hide_metadata,
            2 => self.config.confirm_before_delete = !self.config.confirm_before_delete,
            4 => self.config.password_uppercase = !self.config.password_uppercase,
            5 => self.config.password_lowercase = !self.config.password_lowercase,
            6 => self.config.password_digits = !self.config.password_digits,
            7 => self.config.password_symbols = !self.config.password_symbols,
            8 => self.config.relative_timestamps = !self.config.relative_timestamps,
            9 => self.config.sidebar_enabled = !self.config.sidebar_enabled,
            10 => self.config.vim_keybinds = !self.config.vim_keybinds,
            11 => {
                // Cycle clipboard tool: auto → wl-copy → xclip → arboard → auto
                self.config.clipboard_tool = match self.config.clipboard_tool.as_str() {
                    "auto" => "wl-copy".into(),
                    "wl-copy" => "xclip".into(),
                    "xclip" => "arboard".into(),
                    _ => "auto".into(),
                };
            }
            _ => {}
        }
    }

    /// Enter number editing mode for rows that support it.
    pub fn start_edit(&mut self) {
        if self.selected == 1 {
            self.editing_number = true;
            self.number_buffer = self.config.clipboard_timeout_secs.to_string();
        } else if self.selected == 3 {
            self.editing_number = true;
            self.number_buffer = self.config.password_length.to_string();
        }
    }

    /// Commit number editing and update the config.
    pub fn commit_number(&mut self) {
        if let Ok(v) = self.number_buffer.parse::<u32>() {
            match self.selected {
                1 => self.config.clipboard_timeout_secs = v.clamp(5, 300),
                3 => self.config.password_length = v.clamp(4, 128) as usize,
                _ => {}
            }
        }
        self.editing_number = false;
        self.number_buffer.clear();
    }
}

fn fmt_bool(b: bool) -> String {
    if b { "[x]" } else { "[ ]" }.into()
}

// ── View enum ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum View {
    EntryList(EntryListState),
    EntryDetail(EntryDetailState),
    EntryEdit(EntryEditState),
    Search(SearchState),
    Settings(SettingsState),
    Help(Box<View>),
}

// ── AppState ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum AppState {
    FirstRun(FirstRunState),
    Locked(LockedState),
    Unlocked(View),
}

// ── App ──────────────────────────────────────────────────────────────────

pub struct App {
    pub service: Arc<VaultService>,
    pub config: KryptonConfig,
    pub state: AppState,
    pub should_quit: bool,
    pub toast: Option<Toast>,
    pub toast_ticks: u8,
    /// Handle to abort the clipboard-clear timer so we can wipe on lock/quit.
    clipboard_abort: Option<tokio::task::AbortHandle>,
    /// Pending confirmation for a destructive action.
    pub confirm_action: Option<ConfirmAction>,
}

impl App {
    pub fn new(
        service: Arc<VaultService>,
        config: KryptonConfig,
        initial_state: AppState,
    ) -> Self {
        Self {
            service,
            config,
            state: initial_state,
            should_quit: false,
            toast: None,
            toast_ticks: 0,
            clipboard_abort: None,
            confirm_action: None,
        }
    }

    pub fn show_toast(&mut self, msg: impl Into<String>, kind: ToastKind) {
        self.toast = Some(Toast {
            message: msg.into(),
            kind,
        });
        self.toast_ticks = 60; // ~3 seconds at 50ms ticks
    }

    pub fn tick(&mut self) {
        if self.toast_ticks > 0 {
            self.toast_ticks -= 1;
            if self.toast_ticks == 0 {
                self.toast = None;
            }
        }
    }

    /// Resolve the configured accent color to a ratatui [`Color`].
    pub fn accent_color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self.config.accent_color.as_str() {
            "Green" => Color::Green,
            "Yellow" => Color::Yellow,
            "Blue" => Color::Blue,
            "Magenta" => Color::Magenta,
            "White" => Color::White,
            _ => Color::Cyan,
        }
    }

    /// Central dispatch: maps an [`Action`] to a state transition.
    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Tick => self.tick(),
            Action::Quit => {
                self.clear_clipboard_now();
                self.service.lock();
                self.confirm_action = None;
                self.should_quit = true;
            }
            _ => self.handle_action_inner(action),
        }
    }

    fn handle_action_inner(&mut self, action: Action) {
        match &self.state {
            AppState::FirstRun(_) => self.handle_first_run(action),
            AppState::Locked(_) => self.handle_locked(action),
            AppState::Unlocked(view) => match view {
                View::EntryList(_) => self.handle_entry_list(action),
                View::EntryDetail(_) => self.handle_entry_detail(action),
                View::EntryEdit(_) => self.handle_entry_edit(action),
                View::Search(_) => self.handle_search(action),
                View::Settings(_) => self.handle_settings(action),
                View::Help(_) => self.handle_help(action),
            },
        }
    }

    // ── First run ────────────────────────────────────────────────────

    fn handle_first_run(&mut self, action: Action) {
        let AppState::FirstRun(state) = &mut self.state else {
            return;
        };

        match action {
            Action::CharInput(c) => {
                let target = match state.step {
                    FirstRunStep::EnterPassword => &mut state.password,
                    FirstRunStep::ConfirmPassword => &mut state.confirm,
                };
                let mut current = target.expose_secret().clone();
                current.push(c);
                *target = SecretString::new(current.into());
            }
            Action::Backspace => {
                let target = match state.step {
                    FirstRunStep::EnterPassword => &mut state.password,
                    FirstRunStep::ConfirmPassword => &mut state.confirm,
                };
                let mut current = target.expose_secret().clone();
                current.pop();
                *target = SecretString::new(current.into());
            }
            Action::Submit => match state.step {
                FirstRunStep::EnterPassword => {
                    if state.password.expose_secret().is_empty() {
                        state.error = Some("Password cannot be empty".into());
                        return;
                    }
                    state.step = FirstRunStep::ConfirmPassword;
                    state.error = None;
                }
                FirstRunStep::ConfirmPassword => {
                    if state.password.expose_secret()
                        != state.confirm.expose_secret()
                    {
                        state.error =
                            Some("Passwords do not match".into());
                        state.confirm = SecretString::new("".into());
                        return;
                    }
                    let pw = state.password.expose_secret().clone();
                    if let Err(e) = self.service.create_vault(&pw) {
                        state.error =
                            Some(format!("Failed to create vault: {e}"));
                        return;
                    }
                    match Self::make_entry_list_state_from(&self.service) {
                        Ok(list_state) => {
                            self.state =
                                AppState::Unlocked(View::EntryList(list_state));
                        }
                        Err(e) => {
                            state.error = Some(format!(
                                "Failed to load entries: {e}"
                            ));
                        }
                    }
                }
            },
            Action::ToggleVisibility => { /* not applicable here */ }
            _ => {}
        }
    }

    // ── Locked ───────────────────────────────────────────────────────

    fn handle_locked(&mut self, action: Action) {
        let AppState::Locked(state) = &mut self.state else {
            return;
        };

        // Handle vault reset flow.
        if state.reset_step.is_some() {
            match action {
                Action::Back => {
                    state.reset_step = None;
                    return;
                }
                Action::Tick => {
                    if let Some(ResetStep::Waiting { seconds }) = &mut state.reset_step {
                        if *seconds > 0 {
                            *seconds -= 1;
                        }
                        if *seconds == 0 {
                            // Delete the vault.
                            let _ = std::fs::remove_file(&self.service.store_path);
                            self.state = AppState::FirstRun(FirstRunState::default());
                        }
                    }
                    return;
                }
                Action::CharInput(c) => {
                    if let Some(ResetStep::TypingConfirm { buffer }) = &mut state.reset_step {
                        buffer.push(c);
                        if buffer == "I accept the risks" {
                            state.reset_step = Some(ResetStep::Waiting { seconds: 10 });
                        }
                    }
                    return;
                }
                Action::Backspace => {
                    if let Some(ResetStep::TypingConfirm { buffer }) = &mut state.reset_step {
                        buffer.pop();
                    }
                    return;
                }
                _ => return,
            }
        }

        match action {
            Action::StartReset => {
                state.reset_step = Some(ResetStep::TypingConfirm { buffer: String::new() });
            }
            Action::CharInput(c) => {
                let mut current = state.input.expose_secret().clone();
                current.push(c);
                state.input = SecretString::new(current.into());
            }
            Action::Backspace => {
                let mut current = state.input.expose_secret().clone();
                current.pop();
                state.input = SecretString::new(current.into());
            }
            Action::ToggleVisibility => state.hidden = !state.hidden,
            Action::Submit => {
                let pw = state.input.expose_secret().clone();
                match self.service.unlock(&pw) {
                    Ok(()) => match Self::make_entry_list_state_from(&self.service) {
                        Ok(list_state) => {
                            self.state =
                                AppState::Unlocked(View::EntryList(list_state));
                        }
                        Err(e) => {
                            state.error = Some(format!(
                                "Failed to load entries: {e}"
                            ));
                        }
                    },
                    Err(_) => {
                        state.error =
                            Some("Wrong master password".into());
                        state.input = SecretString::new("".into());
                    }
                }
            }
            _ => {}
        }
    }

    // ── Entry list ───────────────────────────────────────────────────

    fn handle_entry_list(&mut self, action: Action) {
        let AppState::Unlocked(View::EntryList(state)) = &mut self.state
        else {
            return;
        };

        // Clear pending confirmation on any action other than DeleteEntry.
        if !matches!(action, Action::DeleteEntry) {
            self.confirm_action = None;
        }

        match action {
            Action::Up => {
                if state.selected > 0 {
                    state.selected -= 1;
                }
                state.preview_entry = state
                    .entries
                    .get(state.selected)
                    .and_then(|s| self.service.get_entry(&s.id).ok());
            }
            Action::Down => {
                if state.selected + 1 < state.entries.len() {
                    state.selected += 1;
                }
                state.preview_entry = state
                    .entries
                    .get(state.selected)
                    .and_then(|s| self.service.get_entry(&s.id).ok());
            }
            Action::PageUp => {
                state.selected = state.selected.saturating_sub(10);
                state.preview_entry = state
                    .entries
                    .get(state.selected)
                    .and_then(|s| self.service.get_entry(&s.id).ok());
            }
            Action::PageDown => {
                let len = state.entries.len().saturating_sub(1);
                state.selected = (state.selected + 10).min(len);
                state.preview_entry = state
                    .entries
                    .get(state.selected)
                    .and_then(|s| self.service.get_entry(&s.id).ok());
            }
            Action::Click(_col, row) => {
                let r = state.table_rect.get();
                if r.height > 0 && row >= r.y && row < r.y + r.height {
                    let idx = (row - r.y) as usize;
                    if idx < state.entries.len() {
                        state.selected = idx;
                        state.preview_entry = state
                            .entries
                            .get(state.selected)
                            .and_then(|s| self.service.get_entry(&s.id).ok());
                    }
                }
            }
            Action::Select => {
                if let Some(summary) =
                    state.entries.get(state.selected)
                {
                    match self.service.get_entry(&summary.id) {
                        Ok(entry) => {
                            self.state =
                                AppState::Unlocked(View::EntryDetail(
                                    EntryDetailState {
                                        entry,
                                        show_password: false,
                                    },
                                ));
                        }
                        Err(e) => self.show_toast(
                            format!("Error: {e}"),
                            ToastKind::Error,
                        ),
                    }
                }
            }
            Action::NewEntry => {
                self.state = AppState::Unlocked(View::EntryEdit(
                    EntryEditState::default(),
                ));
            }
            Action::EditEntry => {
                if let Some(summary) =
                    state.entries.get(state.selected)
                {
                    match self.service.get_entry(&summary.id) {
                        Ok(entry) => {
                            let created = entry.created_at;
                            let existing_pw = entry.password.clone();
                            self.state =
                                AppState::Unlocked(View::EntryEdit(
                                    EntryEditState {
                                        id: Some(entry.id.clone()),
                                        title: entry.title.clone(),
                                        username: entry
                                            .username
                                            .clone()
                                            .unwrap_or_default(),
                                        password: SecretString::new(
                                            "".into(),
                                        ),
                                        url: entry
                                            .url
                                            .clone()
                                            .unwrap_or_default(),
                                        notes: entry
                                            .notes
                                            .clone()
                                            .unwrap_or_default(),
                                        active_field: 0,
                                        dirty: false,
                                        existing_password: existing_pw,
                                        initial_created_at: created,
                                    },
                                ));
                        }
                        Err(e) => self.show_toast(
                            format!("Error: {e}"),
                            ToastKind::Error,
                        ),
                    }
                }
            }
            Action::DeleteEntry => {
                if !self.config.confirm_before_delete {
                    // No confirmation needed — delete immediately.
                    if let Some(summary) =
                        state.entries.get(state.selected)
                    {
                        match self.service.delete_entry(&summary.id) {
                            Ok(()) => {
                                self.show_toast(
                                    "Entry deleted",
                                    ToastKind::Success,
                                );
                                self.reload_entries();
                            }
                            Err(e) => self.show_toast(
                                format!("Error: {e}"),
                                ToastKind::Error,
                            ),
                        }
                    }
                } else if let Some(summary) =
                    state.entries.get(state.selected).cloned()
                {
                    match self.confirm_action.take() {
                        Some(ConfirmAction::DeleteEntry(ref expected_id))
                            if expected_id == &summary.id =>
                        {
                            // Second press — confirmed.
                            match self.service.delete_entry(&summary.id) {
                                Ok(()) => {
                                    self.show_toast(
                                        "Entry deleted",
                                        ToastKind::Success,
                                    );
                                    self.reload_entries();
                                }
                                Err(e) => self.show_toast(
                                    format!("Error: {e}"),
                                    ToastKind::Error,
                                ),
                            }
                        }
                        _ => {
                            // First press — ask for confirmation.
                            self.confirm_action =
                                Some(ConfirmAction::DeleteEntry(
                                    summary.id.clone(),
                                ));
                            self.show_toast(
                                "Press d again to confirm delete, Esc to cancel",
                                ToastKind::Info,
                            );
                        }
                    }
                }
                return; // skip the confirm_action clear below
            }
            Action::CopyPassword => {
                if let Some(summary) =
                    state.entries.get(state.selected)
                {
                    if let Ok(entry) =
                        self.service.get_entry(&summary.id)
                    {
                        let pw =
                            entry.password.expose_secret().clone();
                        if self.copy_to_clipboard(&pw) {
                            self.show_toast(
                                "Password copied — clears in 30s",
                                ToastKind::Success,
                            );
                        } else {
                            self.show_toast(
                                "Clipboard unavailable (is wl-clipboard installed?)",
                                ToastKind::Error,
                            );
                        }
                    }
                }
            }
            Action::CopyUsername => {
                if let Some(summary) =
                    state.entries.get(state.selected)
                {
                    if let Ok(entry) =
                        self.service.get_entry(&summary.id)
                    {
                        if let Some(ref u) = entry.username {
                            if self.copy_to_clipboard(u) {
                                self.show_toast(
                                    "Username copied",
                                    ToastKind::Success,
                                );
                            } else {
                                self.show_toast(
                                    "Clipboard unavailable",
                                    ToastKind::Error,
                                );
                            }
                        }
                    }
                }
            }
            Action::OpenSettings => self.open_settings(),
            Action::StartSearch => {
                self.state = AppState::Unlocked(View::Search(
                    SearchState::default(),
                ));
            }
            Action::Lock => {
                self.clear_clipboard_now();
                self.service.lock();
                self.state =
                    AppState::Locked(LockedState::default());
            }
            Action::Help => self.open_help(),
            Action::Export => self.handle_export(),
            Action::Import => self.handle_import(),
            Action::CopyUrl => {
                if let Some(summary) = state.entries.get(state.selected) {
                    if let Ok(entry) = self.service.get_entry(&summary.id) {
                        if let Some(ref u) = entry.url {
                            if self.copy_to_clipboard(u) {
                                self.show_toast("URL copied", ToastKind::Success);
                            }
                        }
                    }
                }
            }
            Action::DuplicateEntry => {
                if let Some(summary) = state.entries.get(state.selected) {
                    if let Ok(mut entry) = self.service.get_entry(&summary.id) {
                        entry.id = vault_core::EntryId::new();
                        entry.title = format!("{} (copy)", entry.title);
                        entry.created_at = chrono::Utc::now().timestamp();
                        entry.updated_at = entry.created_at;
                        match self.service.create_entry(&entry) {
                            Ok(()) => { self.show_toast("Entry duplicated", ToastKind::Success); self.reload_entries(); }
                            Err(e) => self.show_toast(format!("Error: {e}"), ToastKind::Error),
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // ── Entry detail ─────────────────────────────────────────────────

    fn handle_entry_detail(&mut self, action: Action) {
        // Extract data we need before mutating self.
        match action {
            Action::Back => self.pop_to_list(),
            Action::ToggleVisibility => {
                let AppState::Unlocked(View::EntryDetail(state)) =
                    &mut self.state
                else {
                    return;
                };
                state.show_password = !state.show_password;
            }
            Action::EditEntry => {
                let edit_state = {
                    let AppState::Unlocked(View::EntryDetail(state)) =
                        &self.state
                    else {
                        return;
                    };
                    EntryEditState {
                        id: Some(state.entry.id.clone()),
                        title: state.entry.title.clone(),
                        username: state
                            .entry
                            .username
                            .clone()
                            .unwrap_or_default(),
                        password: SecretString::new("".into()),
                        url: state
                            .entry
                            .url
                            .clone()
                            .unwrap_or_default(),
                        notes: state
                            .entry
                            .notes
                            .clone()
                            .unwrap_or_default(),
                        active_field: 0,
                        dirty: false,
                        existing_password: state.entry.password.clone(),
                        initial_created_at: state.entry.created_at,
                    }
                };
                self.state = AppState::Unlocked(View::EntryEdit(edit_state));
            }
            Action::CopyPassword => {
                let pw = {
                    let AppState::Unlocked(View::EntryDetail(ref state)) =
                        &self.state
                    else {
                        return;
                    };
                    state.entry.password.expose_secret().clone()
                };
                if self.copy_to_clipboard(&pw) {
                    self.show_toast(
                        "Password copied — clears in 30s",
                        ToastKind::Success,
                    );
                } else {
                    self.show_toast(
                        "Clipboard unavailable (is wl-clipboard installed?)",
                        ToastKind::Error,
                    );
                }
            }
            Action::Help => self.open_help(),
            Action::CopyUrl => {
                let url = {
                    let AppState::Unlocked(View::EntryDetail(ref state)) = &self.state else { return; };
                    state.entry.url.clone()
                };
                if let Some(ref u) = url {
                    if self.copy_to_clipboard(u) {
                        self.show_toast("URL copied", ToastKind::Success);
                    }
                }
            }
            Action::DeleteEntry => {
                let id = {
                    let AppState::Unlocked(View::EntryDetail(ref state)) = &self.state else { return; };
                    state.entry.id.clone()
                };
                if self.config.confirm_before_delete {
                    match self.confirm_action.take() {
                        Some(ConfirmAction::DeleteEntry(ref expected)) if expected == &id => {
                            match self.service.delete_entry(&id) {
                                Ok(()) => { self.show_toast("Entry deleted", ToastKind::Success); self.pop_to_list(); }
                                Err(e) => self.show_toast(format!("Error: {e}"), ToastKind::Error),
                            }
                        }
                        _ => {
                            self.confirm_action = Some(ConfirmAction::DeleteEntry(id));
                            self.show_toast("Press d again to confirm delete, Esc to cancel", ToastKind::Info);
                        }
                    }
                } else {
                    match self.service.delete_entry(&id) {
                        Ok(()) => { self.show_toast("Entry deleted", ToastKind::Success); self.pop_to_list(); }
                        Err(e) => self.show_toast(format!("Error: {e}"), ToastKind::Error),
                    }
                }
            }
            Action::DuplicateEntry => {
                let new_entry = {
                    let AppState::Unlocked(View::EntryDetail(ref state)) = &self.state else { return; };
                    let mut e = state.entry.clone();
                    e.id = vault_core::EntryId::new();
                    e.title = format!("{} (copy)", state.entry.title);
                    e.created_at = chrono::Utc::now().timestamp();
                    e.updated_at = e.created_at;
                    e
                };
                match self.service.create_entry(&new_entry) {
                    Ok(()) => { self.show_toast("Entry duplicated", ToastKind::Success); self.pop_to_list(); }
                    Err(e) => self.show_toast(format!("Error: {e}"), ToastKind::Error),
                }
            }
            Action::CopyUsername => {
                let username = {
                    let AppState::Unlocked(View::EntryDetail(ref state)) =
                        &self.state
                    else {
                        return;
                    };
                    state.entry.username.clone()
                };
                if let Some(ref u) = username {
                    if self.copy_to_clipboard(u) {
                        self.show_toast(
                            "Username copied",
                            ToastKind::Success,
                        );
                    } else {
                        self.show_toast(
                            "Clipboard unavailable",
                            ToastKind::Error,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    // ── Entry edit ───────────────────────────────────────────────────

    fn handle_entry_edit(&mut self, action: Action) {
        let AppState::Unlocked(View::EntryEdit(state)) = &mut self.state
        else {
            return;
        };

        match action {
            Action::Back => self.pop_to_list(),
            Action::NextField => {
                state.active_field = (state.active_field + 1).min(4);
            }
            Action::PrevField => {
                state.active_field =
                    state.active_field.saturating_sub(1);
            }
            Action::CharInput(c) => {
                state.dirty = true;
                match state.active_field {
                    0 => state.title.push(c),
                    1 => state.username.push(c),
                    2 => {
                        let mut current =
                            state.password.expose_secret().clone();
                        current.push(c);
                        state.password =
                            SecretString::new(current.into());
                    }
                    3 => state.url.push(c),
                    4 => state.notes.push(c),
                    _ => {}
                }
            }
            Action::Backspace => {
                state.dirty = true;
                match state.active_field {
                    0 => {
                        state.title.pop();
                    }
                    1 => {
                        state.username.pop();
                    }
                    2 => {
                        let mut current =
                            state.password.expose_secret().clone();
                        current.pop();
                        state.password =
                            SecretString::new(current.into());
                    }
                    3 => {
                        state.url.pop();
                    }
                    4 => {
                        state.notes.pop();
                    }
                    _ => {}
                }
            }
            Action::Help => self.open_help(),
            Action::GeneratePassword => {
                if state.active_field == 2 {
                    let gen_config = vault_ext::GeneratorConfig {
                        length: self.config.password_length,
                        uppercase: self.config.password_uppercase,
                        lowercase: self.config.password_lowercase,
                        digits: self.config.password_digits,
                        symbols: self.config.password_symbols,
                    };
                    let generators = self.service.extensions.generators();
                    if let Some(gen) = generators.first() {
                        match gen.generate(&gen_config) {
                            Ok(pw) => {
                                state.password =
                                    SecretString::new(pw.into());
                                state.dirty = true;
                                self.show_toast(
                                    "Password generated",
                                    ToastKind::Success,
                                );
                            }
                            Err(e) => self.show_toast(
                                format!("Generator error: {e}"),
                                ToastKind::Error,
                            ),
                        }
                    }
                }
            }
            Action::SaveEntry => {
                if state.title.trim().is_empty() {
                    self.show_toast(
                        "Title is required",
                        ToastKind::Error,
                    );
                    return;
                }
                let now = chrono::Utc::now().timestamp();
                let is_new = state.id.is_none();
                // Clone fields to build the entry — don't consume state before
                // the DB call succeeds, or data is lost on error.
                let pw_empty = state.password.expose_secret().is_empty();
                let password = if pw_empty && !is_new {
                    state.existing_password.clone()
                } else {
                    state.password.clone()
                };
                let entry = Entry {
                    id: state.id.clone().unwrap_or_else(vault_core::EntryId::new),
                    title: state.title.clone(),
                    username: if state.username.is_empty() {
                        None
                    } else {
                        Some(state.username.clone())
                    },
                    password,
                    url: if state.url.is_empty() {
                        None
                    } else {
                        Some(state.url.clone())
                    },
                    notes: if state.notes.is_empty() {
                        None
                    } else {
                        Some(state.notes.clone())
                    },
                    tags: Vec::new(),
                    custom_fields: Vec::new(),
                    created_at: if is_new {
                        now
                    } else {
                        state.initial_created_at
                    },
                    updated_at: now,
                };

                let result = if state.id.is_some() {
                    self.service.update_entry(&entry)
                } else {
                    self.service.create_entry(&entry)
                };

                match result {
                    Ok(()) => {
                        self.show_toast("Entry saved", ToastKind::Success);
                        self.reload_entries();
                        self.pop_to_list();
                    }
                    Err(e) => self.show_toast(
                        format!("Error: {e}"),
                        ToastKind::Error,
                    ),
                }
            }
            _ => {}
        }
    }

    // ── Search ───────────────────────────────────────────────────────

    fn handle_search(&mut self, action: Action) {
        match action {
            Action::Back => self.pop_to_list(),
            Action::Up | Action::Down => {
                let AppState::Unlocked(View::Search(state)) =
                    &mut self.state
                else {
                    return;
                };
                match action {
                    Action::Up => {
                        if state.selected > 0 {
                            state.selected -= 1;
                        }
                    }
                    Action::Down => {
                        if state.selected + 1 < state.results.len() {
                            state.selected += 1;
                        }
                    }
                    _ => {}
                }
            }
            Action::CharInput(c) => {
                let query = {
                    let AppState::Unlocked(View::Search(
                        ref mut state,
                    )) = &mut self.state
                    else {
                        return;
                    };
                    state.query.push(c);
                    state.query.clone()
                };
                self.run_search_with_query(&query);
            }
            Action::Backspace => {
                let query = {
                    let AppState::Unlocked(View::Search(
                        ref mut state,
                    )) = &mut self.state
                    else {
                        return;
                    };
                    state.query.pop();
                    state.query.clone()
                };
                self.run_search_with_query(&query);
            }
            Action::Click(_col, row) => {
                // Clone the index we want, then mutate state after the borrow.
                let target_idx = {
                    let AppState::Unlocked(View::Search(ref state)) = self.state
                    else { return };
                    let r = state.result_rect.get();
                    if r.height > 0 && row >= r.y && row < r.y + r.height {
                        let idx = (row - r.y) as usize;
                        if idx < state.results.len() { Some(idx) } else { None }
                    } else { None }
                };
                if let Some(idx) = target_idx {
                    let AppState::Unlocked(View::Search(ref mut s)) = self.state
                    else { return };
                    if idx < s.results.len() {
                        s.selected = idx;
                    }
                }
            }
            Action::Help => self.open_help(),
            Action::CopyPassword => {
                let pw = {
                    if let AppState::Unlocked(View::Search(ref s)) = self.state {
                        s.results.get(s.selected).and_then(|summary| {
                            self.service.get_entry(&summary.id).ok().map(|e| {
                                e.password.expose_secret().clone()
                            })
                        })
                    } else { None }
                };
                if let Some(pw) = pw {
                    if self.copy_to_clipboard(&pw) {
                        let secs = self.config.clipboard_timeout_secs;
                        self.show_toast(format!("Password copied — clears in {secs}s"), ToastKind::Success);
                    } else {
                        self.show_toast("Clipboard unavailable", ToastKind::Error);
                    }
                }
            }
            Action::CopyUsername => {
                if let AppState::Unlocked(View::Search(ref s)) = self.state {
                    if let Some(summary) = s.results.get(s.selected) {
                        if let Ok(entry) = self.service.get_entry(&summary.id) {
                            if let Some(ref u) = entry.username {
                                if self.copy_to_clipboard(u) { self.show_toast("Username copied", ToastKind::Success); }
                            }
                        }
                    }
                }
            }
            Action::EditEntry => {
                let summary_opt = if let AppState::Unlocked(View::Search(ref s)) = self.state {
                    s.results.get(s.selected).cloned()
                } else { None };
                if let Some(summary) = summary_opt {
                    if let Ok(entry) = self.service.get_entry(&summary.id) {
                        self.state = AppState::Unlocked(View::EntryEdit(EntryEditState {
                            id: Some(entry.id.clone()),
                            title: entry.title.clone(),
                            username: entry.username.clone().unwrap_or_default(),
                            password: SecretString::new("".into()),
                            url: entry.url.clone().unwrap_or_default(),
                            notes: entry.notes.clone().unwrap_or_default(),
                            active_field: 0,
                            dirty: false,
                            existing_password: entry.password.clone(),
                            initial_created_at: entry.created_at,
                        }));
                    }
                }
            }
            Action::DeleteEntry => {
                let summary_opt = if let AppState::Unlocked(View::Search(ref s)) = self.state {
                    s.results.get(s.selected).cloned()
                } else { None };
                if let Some(summary) = summary_opt {
                    if self.config.confirm_before_delete {
                        match self.confirm_action.take() {
                            Some(ConfirmAction::DeleteEntry(ref expected)) if expected == &summary.id => {
                                match self.service.delete_entry(&summary.id) {
                                    Ok(()) => { self.show_toast("Entry deleted", ToastKind::Success); self.pop_to_list(); }
                                    Err(e) => self.show_toast(format!("Error: {e}"), ToastKind::Error),
                                }
                            }
                            _ => {
                                self.confirm_action = Some(ConfirmAction::DeleteEntry(summary.id.clone()));
                                self.show_toast("Press d again to confirm delete, Esc to cancel", ToastKind::Info);
                            }
                        }
                    } else {
                        match self.service.delete_entry(&summary.id) {
                            Ok(()) => { self.show_toast("Entry deleted", ToastKind::Success); self.pop_to_list(); }
                            Err(e) => self.show_toast(format!("Error: {e}"), ToastKind::Error),
                        }
                    }
                }
            }
            Action::DuplicateEntry => {
                let summary_opt = if let AppState::Unlocked(View::Search(ref s)) = self.state {
                    s.results.get(s.selected).cloned()
                } else { None };
                if let Some(summary) = summary_opt {
                    if let Ok(mut entry) = self.service.get_entry(&summary.id) {
                        entry.id = vault_core::EntryId::new();
                        entry.title = format!("{} (copy)", entry.title);
                        entry.created_at = chrono::Utc::now().timestamp();
                        entry.updated_at = entry.created_at;
                        match self.service.create_entry(&entry) {
                            Ok(()) => { self.show_toast("Entry duplicated", ToastKind::Success); self.pop_to_list(); }
                            Err(e) => self.show_toast(format!("Error: {e}"), ToastKind::Error),
                        }
                    }
                }
            }
            Action::Select => {
                let summary_opt = {
                    let AppState::Unlocked(View::Search(ref state)) =
                        &self.state
                    else {
                        return;
                    };
                    state.results.get(state.selected).cloned()
                };
                if let Some(summary) = summary_opt {
                    match self.service.get_entry(&summary.id) {
                        Ok(entry) => {
                            self.state =
                                AppState::Unlocked(View::EntryDetail(
                                    EntryDetailState {
                                        entry,
                                        show_password: false,
                                    },
                                ));
                        }
                        Err(e) => self.show_toast(
                            format!("Error: {e}"),
                            ToastKind::Error,
                        ),
                    }
                }
            }
            _ => {}
        }
    }

    // ── Help ────────────────────────────────────────────────────────

    fn open_help(&mut self) {
        let prev = match &self.state {
            AppState::Unlocked(view) => Box::new(view.clone()),
            _ => Box::new(View::EntryList(EntryListState::default())),
        };
        self.state = AppState::Unlocked(View::Help(prev));
    }

    fn handle_export(&mut self) {
        match self.service.export_all("json") {
            Ok(data) => {
                let ts = chrono::Utc::now().format("%Y%m%d-%H%M%S");
                let name = format!("krypton-export-{ts}.json");
                let dir = dirs::data_dir()
                    .map(|d| d.join("krypton"))
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                let _ = std::fs::create_dir_all(&dir);
                let path = dir.join(&name);
                match write_private(&path, &data) {
                    Ok(()) => self.show_toast(
                        format!("Exported {} to {}", self.service.list_entries().map(|v| v.len()).unwrap_or(0), name),
                        ToastKind::Success,
                    ),
                    Err(e) => self.show_toast(format!("Export failed: {e}"), ToastKind::Error),
                }
            }
            Err(e) => self.show_toast(format!("Export error: {e}"), ToastKind::Error),
        }
    }

    fn handle_import(&mut self) {
        // For simplicity: read from a fixed path. The user places the JSON
        // file at ~/.local/share/krypton/import.json and presses Ctrl+I.
        let path = dirs::data_dir()
            .map(|d| d.join("krypton").join("import.json"))
            .unwrap_or_else(|| std::path::PathBuf::from("import.json"));
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                self.show_toast(format!("Import file not found: {e} — place import.json in ~/.local/share/krypton/"), ToastKind::Error);
                return;
            }
        };
        match self.service.import_entries("json", &data) {
            Ok(entries) => {
                let count = entries.len();
                for entry in &entries {
                    if let Err(e) = self.service.create_entry(entry) {
                        self.show_toast(format!("Import failed at entry: {e}"), ToastKind::Error);
                        return;
                    }
                }
                self.show_toast(format!("Imported {count} entries"), ToastKind::Success);
                self.reload_entries();
            }
            Err(e) => self.show_toast(format!("Import error: {e}"), ToastKind::Error),
        }
    }

    fn handle_help(&mut self, action: Action) {
        match action {
            Action::Back => {
                if let AppState::Unlocked(View::Help(prev)) = &mut self.state {
                    self.state = AppState::Unlocked(*prev.clone());
                }
            }
            _ => {}
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────

    /// Build an EntryListState from the given service. A free function so it
    /// can be called while `self.state` is mutably borrowed (it only borrows
    /// `self.service` via field access).
    fn make_entry_list_state_from(
        service: &Arc<VaultService>,
    ) -> Result<EntryListState, vault_core::VaultError> {
        let entries = service.list_entries()?;
        let preview = entries
            .first()
            .and_then(|s| service.get_entry(&s.id).ok());
        Ok(EntryListState {
            entries,
            selected: 0,
            table_rect: Cell::new(ratatui::layout::Rect::default()),
            preview_entry: preview,
        })
    }

    fn pop_to_list(&mut self) {
        match Self::make_entry_list_state_from(&self.service) {
            Ok(list_state) => {
                self.state = AppState::Unlocked(View::EntryList(list_state));
            }
            Err(e) => {
                self.show_toast(
                    format!("Error loading entries: {e}"),
                    ToastKind::Error,
                );
            }
        }
    }

    fn reload_entries(&mut self) {
        if let Ok(entries) = self.service.list_entries() {
            if let AppState::Unlocked(View::EntryList(
                ref mut list,
            )) = self.state
            {
                let old_selected = list.selected;
                list.entries = entries;
                if old_selected >= list.entries.len() {
                    list.selected =
                        list.entries.len().saturating_sub(1);
                } else {
                    list.selected = old_selected;
                }
                // Refresh preview after reload.
                list.preview_entry = list
                    .entries
                    .get(list.selected)
                    .and_then(|s| self.service.get_entry(&s.id).ok());
            }
        }
    }

    // ── Settings ─────────────────────────────────────────────────────

    fn open_settings(&mut self) {
        self.state = AppState::Unlocked(View::Settings(SettingsState {
            config: self.config.clone(),
            ..SettingsState::default()
        }));
    }

    fn handle_settings(&mut self, action: Action) {
        let AppState::Unlocked(View::Settings(state)) = &mut self.state
        else { return };

        match action {
            Action::Back => {
                if state.editing_number {
                    state.editing_number = false;
                    state.number_buffer.clear();
                } else {
                    self.pop_to_list();
                }
            }
            Action::Up => {
                if !state.editing_number && state.selected > 0 {
                    state.selected -= 1;
                }
            }
            Action::Down => {
                if !state.editing_number && state.selected + 1 < state.len() {
                    state.selected += 1;
                }
            }
            Action::ToggleSetting => {
                if state.editing_number {
                    state.commit_number();
                } else if matches!(state.selected, 1 | 3) {
                    state.start_edit();
                } else {
                    state.toggle(state.selected);
                }
            }
            Action::CharInput(c) => {
                if state.editing_number {
                    state.number_buffer.push(c);
                }
            }
            Action::Backspace => {
                if state.editing_number {
                    state.number_buffer.pop();
                }
            }
            Action::Help => self.open_help(),
            Action::SaveEntry => {
                if state.editing_number {
                    state.commit_number();
                }
                // Persist to disk.
                if let Err(e) = state.config.save() {
                    self.show_toast(format!("Save failed: {e}"), ToastKind::Error);
                } else {
                    self.config = state.config.clone();
                    self.service.hide_metadata.store(
                        self.config.hide_metadata,
                        std::sync::atomic::Ordering::Relaxed,
                    );
                    self.show_toast("Settings saved", ToastKind::Success);
                    self.pop_to_list();
                }
            }
            _ => {}
        }
    }

    fn run_search_with_query(&mut self, query: &str) {
        let AppState::Unlocked(View::Search(ref mut state)) =
            self.state
        else {
            return;
        };
        if query.is_empty() {
            state.results.clear();
            state.selected = 0;
            return;
        }
        match self.service.search(query) {
            Ok(results) => {
                state.results = results;
                state.selected = 0;
            }
            Err(_) => {
                state.results.clear();
            }
        }
    }

    // ── Clipboard ────────────────────────────────────────────────────

    /// Copy text to the system clipboard and schedule an auto-clear after
    /// the configured timeout. Respects the `clipboard_tool` config.
    /// Returns `true` on success so the caller can show feedback.
    fn copy_to_clipboard(&mut self, text: &str) -> bool {
        // Abort any pending clear.
        if let Some(handle) = self.clipboard_abort.take() {
            handle.abort();
        }

        let tool = self.config.clipboard_tool.clone();
        let success = try_clipboard_set(text, &tool);
        if !success {
            return false;
        }

        let timeout = self.config.clipboard_timeout_secs;
        let owned = zeroize::Zeroizing::new(text.to_string());
        let tool_clone = tool.clone();
        let handle = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(timeout as u64)).await;
            let _ = try_clipboard_clear(&tool_clone);
            drop(owned);
        });
        self.clipboard_abort = Some(handle.abort_handle());
        true
    }

    /// Immediately clear the clipboard and abort any scheduled clear task.
    fn clear_clipboard_now(&mut self) {
        if let Some(handle) = self.clipboard_abort.take() {
            handle.abort();
        }
        let tool = self.config.clipboard_tool.clone();
        let _ = try_clipboard_clear(&tool);
    }
}

// ── Clipboard helpers (free functions) ───────────────────────────────

/// Pipe `text` via stdin to a clipboard command — never via argv, avoiding
/// `/proc/*/cmdline` exposure.
fn try_clipboard_cmd(cmd: &str, text: &str) -> bool {
    use std::io::Write;
    std::process::Command::new(cmd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()
        .and_then(|mut child| {
            let mut stdin = child.stdin.take()?;
            stdin.write_all(text.as_bytes()).ok()?;
            drop(stdin); // close pipe so child sees EOF
            child.wait().ok().map(|s| s.success())
        })
        .unwrap_or(false)
}

/// Try to set the clipboard based on the configured tool.
/// "auto": arboard first, then wl-copy, then xclip.
/// "arboard": arboard only.
/// "wl-copy": wl-copy only.
/// "xclip": xclip only.
fn try_clipboard_set(text: &str, tool: &str) -> bool {
    match tool {
        "arboard" => {
            if let Ok(mut board) = arboard::Clipboard::new() {
                board.set_text(text).is_ok()
            } else {
                false
            }
        }
        "wl-copy" => try_clipboard_cmd("wl-copy", text),
        "xclip" => try_clipboard_cmd("xclip", text),
        _ => {
            // "auto" — try arboard first, then wl-copy, then xclip.
            if let Ok(mut board) = arboard::Clipboard::new() {
                if board.set_text(text).is_ok() {
                    return true;
                }
            }
            if try_clipboard_cmd("wl-copy", text) {
                return true;
            }
            try_clipboard_cmd("xclip", text)
        }
    }
}

/// Clear the system clipboard using the configured tool.
fn try_clipboard_clear(tool: &str) -> bool {
    match tool {
        "arboard" => {
            if let Ok(mut board) = arboard::Clipboard::new() {
                board.set_text("").is_ok()
            } else {
                false
            }
        }
        "wl-copy" => std::process::Command::new("wl-copy")
            .arg("--clear")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
        "xclip" => true, // xclip doesn't hold clipboard; clearing is a no-op
        _ => {
            // "auto" — try arboard + wl-copy.
            let mut ok = false;
            if let Ok(mut board) = arboard::Clipboard::new() {
                ok |= board.set_text("").is_ok();
            }
            if std::process::Command::new("wl-copy")
                .arg("--clear")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                ok = true;
            }
            ok
        }
    }
}

/// Write data to a file with owner-only permissions (0o600). Used for
/// credential exports to prevent other local users from reading them.
fn write_private(path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    std::io::Write::write_all(&mut f, data)?;
    Ok(())
}

impl Drop for App {
    fn drop(&mut self) {
        self.clear_clipboard_now();
    }
}
