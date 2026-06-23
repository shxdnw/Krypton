pub mod registry;

pub use registry::Registry;

use vault_core::{Entry, EntryId, Result};

/// Static metadata describing an extension.
pub struct ExtMeta {
    pub id: &'static str,
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
}

/// Marker trait that every extension must implement.
pub trait Extension: Send + Sync + 'static {
    fn meta(&self) -> ExtMeta;
}


/// Lifecycle hooks that fire on vault and entry events.
///
/// Every method has a default no-op implementation so implementors only
/// override the events they care about.
pub trait HookExt: Extension {
    fn on_entry_created(&self, _entry: &Entry) -> Result<()> {
        Ok(())
    }
    fn on_entry_updated(&self, _entry: &Entry) -> Result<()> {
        Ok(())
    }
    fn on_entry_accessed(&self, _entry: &Entry) -> Result<()> {
        Ok(())
    }
    fn on_entry_deleted(&self, _id: &EntryId) -> Result<()> {
        Ok(())
    }
    fn on_unlock(&self) -> Result<()> {
        Ok(())
    }
    fn on_lock(&self) -> Result<()> {
        Ok(())
    }
}


/// Import entries from external formats (CSV, Bitwarden JSON, etc.).
pub trait ImportExt: Extension {
    /// Return the list of format identifiers this importer supports
    /// (e.g. `["csv", "bitwarden-json"]`).
    fn formats(&self) -> &[&str];

    /// Parse `data` in the given `format` and return a list of entries ready
    /// to be persisted.
    fn import(&self, format: &str, data: &[u8]) -> Result<Vec<Entry>>;
}


/// Configuration for password generation.
pub struct GeneratorConfig {
    pub length: usize,
    pub uppercase: bool,
    pub lowercase: bool,
    pub digits: bool,
    pub symbols: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            length: 20,
            uppercase: true,
            lowercase: true,
            digits: true,
            symbols: true,
        }
    }
}

/// Pluggable password generator.
pub trait GeneratorExt: Extension {
    /// Unique identifier for this generator (used as a lookup key).
    fn generator_id(&self) -> &str;

    /// Generate a password matching the given configuration.
    fn generate(&self, config: &GeneratorConfig) -> Result<String>;
}
