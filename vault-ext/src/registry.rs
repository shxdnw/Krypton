use std::sync::Arc;

use rand::Rng;
use vault_core::{Entry, EntryId, Result};

use crate::{ExtMeta, Extension, GeneratorConfig, GeneratorExt, HookExt, ImportExt};

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

// ── Registry ─────────────────────────────────────────────────────────────

/// Central extension registry.
///
/// Holds registered hooks, generators, and importers. All hook fire methods
/// log errors via `eprintln!` and never propagate — a misbehaving extension
/// must not take down the vault.
pub struct Registry {
    hooks: std::sync::Mutex<Vec<Arc<dyn HookExt>>>,
    generators: std::sync::Mutex<Vec<Arc<dyn GeneratorExt>>>,
    importers: std::sync::Mutex<Vec<Arc<dyn ImportExt>>>,
}

impl Registry {
    pub fn new() -> Self {
        let reg = Self {
            hooks: std::sync::Mutex::new(Vec::new()),
            generators: std::sync::Mutex::new(Vec::new()),
            importers: std::sync::Mutex::new(Vec::new()),
        };
        // Auto-register the built-in password generator.
        let _ = reg.register_generator(Arc::new(DefaultGenerator));
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

    /// Iterate over registered generators.
    pub fn generators(&self) -> Vec<Arc<dyn GeneratorExt>> {
        self.generators
            .lock()
            .map(|g| g.clone())
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
