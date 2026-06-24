use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration persisted to `~/.config/krypton/config.json`.
///
/// All fields are plain preferences — no secrets are stored here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KryptonConfig {
    // ── Security ──────────────────────────────────────────────────────
    /// When true, title/username/url are hidden from plaintext columns in the
    /// entry list view (shown as "[hidden]"). The real values are still stored
    /// inside the encrypted blob. Disables FTS5 search.
    #[serde(default, alias = "encrypt_metadata")]
    pub hide_metadata: bool,

    /// Seconds before the clipboard is automatically cleared after a copy.
    #[serde(default = "default_clipboard_timeout")]
    pub clipboard_timeout_secs: u32,

    /// Show a confirmation prompt before deleting an entry.
    #[serde(default)]
    pub confirm_before_delete: bool,

    // ── Password generator defaults ──────────────────────────────────
    #[serde(default = "default_pw_length")]
    pub password_length: usize,

    #[serde(default = "default_true")]
    pub password_uppercase: bool,

    #[serde(default = "default_true")]
    pub password_lowercase: bool,

    #[serde(default = "default_true")]
    pub password_digits: bool,

    #[serde(default = "default_true")]
    pub password_symbols: bool,

    // ── UI ───────────────────────────────────────────────────────────
    /// Relative timestamps in entry list ("2m ago") vs absolute.
    #[serde(default = "default_true")]
    pub relative_timestamps: bool,

    /// Accent colour name used for highlights and active fields.
    /// Valid values: Cyan, Green, Yellow, Blue, Magenta, White.
    #[serde(default = "default_accent")]
    pub accent_color: String,

    /// Show line numbers or row indicators in lists.
    #[serde(default)]
    pub show_row_numbers: bool,

    // ── Layout ─────────────────────────────────────────────────────────
    /// Show the preview sidebar next to the entries table.
    #[serde(default = "default_true")]
    pub sidebar_enabled: bool,

    // ── Input ──────────────────────────────────────────────────────────
    /// Enable vim-style j/k navigation keys alongside arrow keys.
    #[serde(default = "default_true")]
    pub vim_keybinds: bool,

    // ── Clipboard ──────────────────────────────────────────────────────
    /// Clipboard backend: "auto", "wl-copy", "xclip", or "arboard".
    /// "auto" tries arboard first, then falls back to wl-copy / xclip.
    #[serde(default = "default_clipboard_tool")]
    pub clipboard_tool: String,
}

impl Default for KryptonConfig {
    fn default() -> Self {
        Self {
            hide_metadata: false,
            clipboard_timeout_secs: 30,
            confirm_before_delete: false,
            password_length: 20,
            password_uppercase: true,
            password_lowercase: true,
            password_digits: true,
            password_symbols: true,
            relative_timestamps: true,
            accent_color: "Cyan".into(),
            show_row_numbers: false,
            sidebar_enabled: true,
            vim_keybinds: true,
            clipboard_tool: "auto".into(),
        }
    }
}

// ── Serde default helpers ────────────────────────────────────────────

fn default_clipboard_timeout() -> u32 {
    30
}
fn default_pw_length() -> usize {
    20
}
fn default_true() -> bool {
    true
}
fn default_accent() -> String {
    "Cyan".into()
}
fn default_clipboard_tool() -> String {
    "auto".into()
}

// ── Load / save ──────────────────────────────────────────────────────

impl KryptonConfig {
    /// Resolve the config path: `~/.config/krypton/config.json`.
    pub fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("krypton").join("config.json"))
    }

    /// Load config from disk, falling back to defaults if the file is
    /// missing or unreadable.
    pub fn load() -> Self {
        let path = match Self::path() {
            Some(p) => p,
            None => return Self::default(),
        };
        match std::fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Persist the current config to disk, creating parent directories
    /// if needed.
    #[allow(dead_code)] // used by settings UI (Step 5)
    pub fn save(&self) -> Result<(), String> {
        let path = Self::path().ok_or("no config directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("serialize: {e}"))?;
        std::fs::write(&path, json).map_err(|e| format!("write: {e}"))?;
        Ok(())
    }
}
