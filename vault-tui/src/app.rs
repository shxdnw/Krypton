use std::sync::Arc;

use secrecy::{ExposeSecret, SecretString};
use vault_core::{Entry, EntryId, EntrySummary};
use vault_service::VaultService;

use crate::actions::Action;


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
}

impl Default for LockedState {
    fn default() -> Self {
        Self {
            input: SecretString::new("".into()),
            hidden: true,
            error: None,
            loading: false,
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

#[derive(Debug, Clone)]
pub struct EntryListState {
    pub entries: Vec<EntrySummary>,
    pub selected: usize,
}

impl Default for EntryListState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            selected: 0,
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
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub query: String,
    pub results: Vec<EntrySummary>,
    pub selected: usize,
}


#[derive(Debug, Clone)]
pub enum View {
    EntryList(EntryListState),
    EntryDetail(EntryDetailState),
    EntryEdit(EntryEditState),
    Search(SearchState),
}


#[derive(Debug, Clone)]
pub enum AppState {
    FirstRun(FirstRunState),
    Locked(LockedState),
    Unlocked(View),
}


pub struct App {
    pub service: Arc<VaultService>,
    pub state: AppState,
    pub should_quit: bool,
    pub toast: Option<Toast>,
    pub toast_ticks: u8,
    /// Handle to abort the clipboard-clear timer so we can wipe on lock/quit.
    clipboard_abort: Option<tokio::task::AbortHandle>,
}

impl App {
    pub fn new(service: Arc<VaultService>, initial_state: AppState) -> Self {
        Self {
            service,
            state: initial_state,
            should_quit: false,
            toast: None,
            toast_ticks: 0,
            clipboard_abort: None,
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

    /// Central dispatch: maps an [`Action`] to a state transition.
    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Tick => self.tick(),
            Action::Quit => {
                self.clear_clipboard_now();
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
                    match self.service.list_entries() {
                        Ok(entries) => {
                            self.state =
                                AppState::Unlocked(View::EntryList(
                                    EntryListState {
                                        entries,
                                        selected: 0,
                                    },
                                ));
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

        match action {
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
                    Ok(()) => match self.service.list_entries() {
                        Ok(entries) => {
                            self.state =
                                AppState::Unlocked(View::EntryList(
                                    EntryListState {
                                        entries,
                                        selected: 0,
                                    },
                                ));
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

        match action {
            Action::Up => {
                if state.selected > 0 {
                    state.selected -= 1;
                }
            }
            Action::Down => {
                if state.selected + 1 < state.entries.len() {
                    state.selected += 1;
                }
            }
            Action::PageUp => {
                state.selected = state.selected.saturating_sub(10);
            }
            Action::PageDown => {
                let len = state.entries.len().saturating_sub(1);
                state.selected = (state.selected + 10).min(len);
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
                        self.copy_to_clipboard(&pw);
                        self.show_toast(
                            "Password copied — clears in 30s",
                            ToastKind::Success,
                        );
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
                            self.copy_to_clipboard(u);
                            self.show_toast(
                                "Username copied",
                                ToastKind::Success,
                            );
                        }
                    }
                }
            }
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
            _ => {}
        }
    }

    // ── Entry detail ─────────────────────────────────────────────────

    fn handle_entry_detail(&mut self, action: Action) {
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
                self.copy_to_clipboard(&pw);
                self.show_toast(
                    "Password copied — clears in 30s",
                    ToastKind::Success,
                );
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
                    self.copy_to_clipboard(u);
                    self.show_toast(
                        "Username copied",
                        ToastKind::Success,
                    );
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
            Action::SaveEntry => {
                if state.title.trim().is_empty() {
                    self.show_toast(
                        "Title is required",
                        ToastKind::Error,
                    );
                    return;
                }
                let now = chrono::Utc::now().timestamp();
                let entry = Entry {
                    id: state
                        .id
                        .clone()
                        .unwrap_or_else(vault_core::EntryId::new),
                    title: std::mem::take(&mut state.title),
                    username: if state.username.is_empty() {
                        None
                    } else {
                        Some(std::mem::take(&mut state.username))
                    },
                    password: std::mem::replace(
                        &mut state.password,
                        SecretString::new("".into()),
                    ),
                    url: if state.url.is_empty() {
                        None
                    } else {
                        Some(std::mem::take(&mut state.url))
                    },
                    notes: if state.notes.is_empty() {
                        None
                    } else {
                        Some(std::mem::take(&mut state.notes))
                    },
                    tags: Vec::new(),
                    custom_fields: Vec::new(),
                    created_at: if state.id.is_some() {
                        0
                    } else {
                        now
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
                        self.show_toast(
                            "Entry saved",
                            ToastKind::Success,
                        );
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

    // ── Helpers ──────────────────────────────────────────────────────

    fn pop_to_list(&mut self) {
        match self.service.list_entries() {
            Ok(entries) => {
                self.state =
                    AppState::Unlocked(View::EntryList(
                        EntryListState {
                            entries,
                            selected: 0,
                        },
                    ));
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
            }
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
    /// 30 seconds. Any previously scheduled clear is aborted first.
    fn copy_to_clipboard(&mut self, text: &str) {
        if let Some(handle) = self.clipboard_abort.take() {
            handle.abort();
        }

        if let Ok(mut board) = arboard::Clipboard::new() {
            if board.set_text(text).is_ok() {
                let owned = text.to_string();
                let handle = tokio::spawn(async move {
                    tokio::time::sleep(
                        std::time::Duration::from_secs(30),
                    )
                    .await;
                    if let Ok(mut b) = arboard::Clipboard::new() {
                        let _ = b.set_text("");
                    }
                    drop(owned);
                });
                self.clipboard_abort = Some(handle.abort_handle());
            }
        }
    }

    /// Immediately clear the clipboard and abort any scheduled clear task.
    fn clear_clipboard_now(&mut self) {
        if let Some(handle) = self.clipboard_abort.take() {
            handle.abort();
        }
        if let Ok(mut board) = arboard::Clipboard::new() {
            let _ = board.set_text("");
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.clear_clipboard_now();
    }
}
