use std::sync::Arc;

use rand::Rng;
use vault_core::{Entry, EntryId, Result};

use crate::{ExportExt, ExtMeta, Extension, GeneratorConfig, GeneratorExt, HookExt, ImportExt};

// ── Default password generator ───────────────────────────────────────────

const LOWERCASE: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{};:,.<>?";

struct DefaultGenerator;

impl Extension for DefaultGenerator {
    fn meta(&self) -> ExtMeta {
        ExtMeta {
            id: "builtin.default",
            name: "Default Generator",
            version: env!("CARGO_PKG_VERSION"),
            description: "Built-in password generator using OsRng",
        }
    }
}

impl GeneratorExt for DefaultGenerator {
    fn generator_id(&self) -> &str {
        "default"
    }

    fn generate(&self, config: &GeneratorConfig) -> Result<String> {
        let mut charset = Vec::<u8>::new();
        if config.lowercase {
            charset.extend_from_slice(LOWERCASE);
        }
        if config.uppercase {
            charset.extend_from_slice(UPPERCASE);
        }
        if config.digits {
            charset.extend_from_slice(DIGITS);
        }
        if config.symbols {
            charset.extend_from_slice(SYMBOLS);
        }
        if charset.is_empty() {
            charset.extend_from_slice(LOWERCASE);
        }

        let mut rng = rand::rngs::OsRng;
        let password: String = (0..config.length)
            .map(|_| {
                let idx = rng.gen_range(0..charset.len());
                charset[idx] as char
            })
            .collect();

        Ok(password)
    }
}

// ── Default JSON exporter ─────────────────────────────────────────────────

struct DefaultJsonExporter;

impl Extension for DefaultJsonExporter {
    fn meta(&self) -> ExtMeta {
        ExtMeta {
            id: "builtin.json-export",
            name: "JSON Exporter",
            version: env!("CARGO_PKG_VERSION"),
            description: "Exports entries as a JSON array",
        }
    }
}

impl ExportExt for DefaultJsonExporter {
    fn export_formats(&self) -> &[&str] {
        &["json"]
    }

    fn export(&self, _format: &str, entries: &[vault_core::Entry]) -> vault_core::Result<Vec<u8>> {
        let data: Vec<vault_core::EntryData> =
            entries.iter().map(vault_core::EntryData::from).collect();
        serde_json::to_vec_pretty(&data)
            .map_err(|e| vault_core::VaultError::Storage(format!("export: {e}")))
    }
}

// ── Default JSON importer ─────────────────────────────────────────────────

struct DefaultJsonImporter;

impl Extension for DefaultJsonImporter {
    fn meta(&self) -> ExtMeta {
        ExtMeta {
            id: "builtin.json-import",
            name: "JSON Importer",
            version: env!("CARGO_PKG_VERSION"),
            description: "Imports entries from a JSON array",
        }
    }
}

impl ImportExt for DefaultJsonImporter {
    fn formats(&self) -> &[&str] {
        &["json"]
    }

    fn import(&self, _format: &str, data: &[u8]) -> vault_core::Result<Vec<vault_core::Entry>> {
        let entries_data: Vec<vault_core::EntryData> = serde_json::from_slice(data)
            .map_err(|e| vault_core::VaultError::Storage(format!("import parse: {e}")))?;
        Ok(entries_data.into_iter().map(|d| d.into_entry()).collect())
    }
}

// ── Registry ─────────────────────────────────────────────────────────────

/// Central extension registry.
///
/// Holds registered hooks, generators, exporters, and importers. All hook fire
/// methods log errors via `eprintln!` and never propagate — a misbehaving
/// extension must not take down the vault.
pub struct Registry {
    hooks: std::sync::Mutex<Vec<Arc<dyn HookExt>>>,
    generators: std::sync::Mutex<Vec<Arc<dyn GeneratorExt>>>,
    importers: std::sync::Mutex<Vec<Arc<dyn ImportExt>>>,
    exporters: std::sync::Mutex<Vec<Arc<dyn ExportExt>>>,
}

impl Registry {
    pub fn new() -> Self {
        let reg = Self {
            hooks: std::sync::Mutex::new(Vec::new()),
            generators: std::sync::Mutex::new(Vec::new()),
            importers: std::sync::Mutex::new(Vec::new()),
            exporters: std::sync::Mutex::new(Vec::new()),
        };
        let _ = reg.register_generator(Arc::new(DefaultGenerator));
        let _ = reg.register_exporter(Arc::new(DefaultJsonExporter));
        let _ = reg.register_importer(Arc::new(DefaultJsonImporter));
        reg
    }

    pub fn register_hook(&self, hook: Arc<dyn HookExt>) -> Result<()> {
        self.hooks
            .lock()
            .map_err(|e| vault_core::VaultError::Extension {
                name: "registry".into(),
                reason: format!("lock poisoned: {e}"),
            })?
            .push(hook);
        Ok(())
    }

    pub fn register_generator(&self, gen: Arc<dyn GeneratorExt>) -> Result<()> {
        self.generators
            .lock()
            .map_err(|e| vault_core::VaultError::Extension {
                name: "registry".into(),
                reason: format!("lock poisoned: {e}"),
            })?
            .push(gen);
        Ok(())
    }

    pub fn register_importer(&self, imp: Arc<dyn ImportExt>) -> Result<()> {
        self.importers
            .lock()
            .map_err(|e| vault_core::VaultError::Extension {
                name: "registry".into(),
                reason: format!("lock poisoned: {e}"),
            })?
            .push(imp);
        Ok(())
    }

    pub fn register_exporter(&self, exp: Arc<dyn ExportExt>) -> Result<()> {
        self.exporters
            .lock()
            .map_err(|e| vault_core::VaultError::Extension {
                name: "registry".into(),
                reason: format!("lock poisoned: {e}"),
            })?
            .push(exp);
        Ok(())
    }

    /// Iterate over registered generators.
    pub fn generators(&self) -> Vec<Arc<dyn GeneratorExt>> {
        self.generators
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Iterate over registered exporters.
    pub fn exporters(&self) -> Vec<Arc<dyn ExportExt>> {
        self.exporters
            .lock()
            .map(|e| e.clone())
            .unwrap_or_default()
    }

    /// Iterate over registered importers.
    pub fn importers(&self) -> Vec<Arc<dyn ImportExt>> {
        self.importers
            .lock()
            .map(|i| i.clone())
            .unwrap_or_default()
    }

    // ── Hook firing ──────────────────────────────────────────────────

    fn fire<F>(hooks: &std::sync::Mutex<Vec<Arc<dyn HookExt>>>, name: &str, f: F)
    where
        F: Fn(&Arc<dyn HookExt>),
    {
        let hooks = match hooks.lock() {
            Ok(h) => h,
            Err(e) => {
                eprintln!("[registry] lock poisoned in {name}: {e}");
                return;
            }
        };
        for hook in hooks.iter() {
            f(hook);
        }
    }

    pub fn fire_entry_created(&self, entry: &Entry) {
        Self::fire(&self.hooks, "on_entry_created", |h| {
            if let Err(e) = h.on_entry_created(entry) {
                eprintln!(
                    "[registry] hook '{}' on_entry_created: {e}",
                    h.meta().id
                );
            }
        });
    }

    pub fn fire_entry_updated(&self, entry: &Entry) {
        Self::fire(&self.hooks, "on_entry_updated", |h| {
            if let Err(e) = h.on_entry_updated(entry) {
                eprintln!(
                    "[registry] hook '{}' on_entry_updated: {e}",
                    h.meta().id
                );
            }
        });
    }

    pub fn fire_entry_accessed(&self, entry: &Entry) {
        Self::fire(&self.hooks, "on_entry_accessed", |h| {
            if let Err(e) = h.on_entry_accessed(entry) {
                eprintln!(
                    "[registry] hook '{}' on_entry_accessed: {e}",
                    h.meta().id
                );
            }
        });
    }

    pub fn fire_entry_deleted(&self, id: &EntryId) {
        Self::fire(&self.hooks, "on_entry_deleted", |h| {
            if let Err(e) = h.on_entry_deleted(id) {
                eprintln!(
                    "[registry] hook '{}' on_entry_deleted: {e}",
                    h.meta().id
                );
            }
        });
    }

    pub fn fire_unlock(&self) {
        Self::fire(&self.hooks, "on_unlock", |h| {
            if let Err(e) = h.on_unlock() {
                eprintln!("[registry] hook '{}' on_unlock: {e}", h.meta().id);
            }
        });
    }

    pub fn fire_lock(&self) {
        Self::fire(&self.hooks, "on_lock", |h| {
            if let Err(e) = h.on_lock() {
                eprintln!("[registry] hook '{}' on_lock: {e}", h.meta().id);
            }
        });
    }
}
