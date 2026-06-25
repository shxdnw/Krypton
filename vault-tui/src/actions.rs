/// Every user interaction is represented as an Action.
///
/// The event loop maps key events to actions, and `App::handle_action`
/// dispatches them to the appropriate state transition.
#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Back,
    Up,
    Down,
    PageUp,
    PageDown,
    Select,
    /// Append a printable character to the active text input.
    CharInput(char),
    Backspace,
    Submit,
    ToggleVisibility,
    NewEntry,
    EditEntry,
    DeleteEntry,
    CopyPassword,
    CopyUsername,
    SaveEntry,
    StartSearch,
    NextField,
    PrevField,
    Lock,
    Tick,
    /// Mouse click at terminal coordinates (column, row).
    Click(u16, u16),
    OpenSettings,
    ToggleSetting,
    GeneratePassword,
    Help,
}
