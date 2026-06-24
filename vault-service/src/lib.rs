use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc, RwLock},
};

use vault_core::{Cipher, Entry, EntryId, EntrySummary, KeyDeriver, Result, Store, VaultError};
use vault_crypto::cipher_from_password;
use vault_store::SqliteStore;

/// The verification plaintext encrypted and stored in `vault_meta["verify"]`
/// so we can detect wrong master passwords during unlock.
const VERIFY_PLAINTEXT: &[u8] = b"vault-check-ok";
const SALT_KEY: &str = "salt";
const VERIFY_KEY: &str = "verify";


/// Holds the unlocked store reference. Setting the session to `None` drops
/// the cipher key from memory.
struct UnlockedSession {
    store: Arc<SqliteStore>,
}


pub struct VaultService {
    store_path: PathBuf,
    deriver: Arc<dyn KeyDeriver>,
    session: RwLock<Option<UnlockedSession>>,
    /// When true, title/username/url are shown as "[hidden]" in the entry
    /// list (the real values live inside the encrypted blob).
    pub hide_metadata: AtomicBool,
    /// Extension registry — generators, hooks, importers.
    pub extensions: Arc<vault_ext::Registry>,
}

impl VaultService {
    pub fn new(
        store_path: PathBuf,
        deriver: Arc<dyn KeyDeriver>,
        extensions: Arc<vault_ext::Registry>,
    ) -> Self {
        Self {
            store_path,
            deriver,
            extensions,
            session: RwLock::new(None),
            hide_metadata: AtomicBool::new(false),
        }
    }

    // ── Vault lifecycle ───────────────────────────────────────────────

    /// Check whether a vault database file already exists on disk.
    pub fn vault_exists(&self) -> bool {
        self.store_path.exists()
    }

    /// Create a new vault with the given master password.
    pub fn create_vault(&self, password: &str) -> Result<()> {
        let salt = self.deriver.generate_salt();
        let cipher = cipher_from_password(
            self.deriver.as_ref(),
            password.as_bytes(),
            &salt,
        )?;
        let cipher: Arc<dyn Cipher> = Arc::new(cipher);

        // the store so we don't need to extract it later.
        let verify_blob = cipher.encrypt(VERIFY_PLAINTEXT)?;

        let store = Arc::new(SqliteStore::open(&self.store_path, cipher)?);

        // doesn't leave the vault in an unrecoverable partial state.
        store.transaction(|conn| {
            conn.execute_batch("PRAGMA journal_mode=WAL;")
                .map_err(|e| VaultError::Storage(format!("pragma: {e}")))?;
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS vault_meta (
                    key   TEXT PRIMARY KEY,
                    value BLOB NOT NULL
                );
                CREATE TABLE IF NOT EXISTS entries (
                    rowid          INTEGER PRIMARY KEY AUTOINCREMENT,
                    id             TEXT UNIQUE NOT NULL,
                    title          TEXT NOT NULL DEFAULT '',
                    username       TEXT,
                    url            TEXT,
                    tags           TEXT NOT NULL DEFAULT '[]',
                    encrypted_data BLOB NOT NULL,
                    created_at     INTEGER NOT NULL,
                    updated_at     INTEGER NOT NULL
                );
                CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(
                    title, username, url, tags,
                    content='entries', content_rowid='rowid'
                );
                CREATE TRIGGER IF NOT EXISTS entries_ai AFTER INSERT ON entries BEGIN
                    INSERT INTO entries_fts(rowid, title, username, url, tags)
                    VALUES (new.rowid, new.title, new.username, new.url, new.tags);
                END;
                CREATE TRIGGER IF NOT EXISTS entries_ad AFTER DELETE ON entries BEGIN
                    INSERT INTO entries_fts(entries_fts, rowid, title, username, url, tags)
                    VALUES ('delete', old.rowid, old.title, old.username, old.url, old.tags);
                END;
                CREATE TRIGGER IF NOT EXISTS entries_au AFTER UPDATE ON entries BEGIN
                    INSERT INTO entries_fts(entries_fts, rowid, title, username, url, tags)
                    VALUES ('delete', old.rowid, old.title, old.username, old.url, old.tags);
                    INSERT INTO entries_fts(rowid, title, username, url, tags)
                    VALUES (new.rowid, new.title, new.username, new.url, new.tags);
                END;",
            )
            .map_err(|e| VaultError::Storage(format!("init: {e}")))?;

            conn.execute(
                "INSERT OR REPLACE INTO vault_meta (key, value) VALUES (?1, ?2)",
                rusqlite::params![SALT_KEY, salt.as_slice()],
            )
            .map_err(|e| VaultError::Storage(format!("write salt: {e}")))?;
            conn.execute(
                "INSERT OR REPLACE INTO vault_meta (key, value) VALUES (?1, ?2)",
                rusqlite::params![VERIFY_KEY, verify_blob.as_slice()],
            )
            .map_err(|e| VaultError::Storage(format!("write verify: {e}")))?;

            Ok(())
        })?;

        let mut session = self.session.write().map_err(|e| {
            VaultError::Storage(format!("session lock poisoned: {e}"))
        })?;
        *session = Some(UnlockedSession { store });
        Ok(())
    }

    /// Unlock an existing vault with a master password.
    pub fn unlock(&self, password: &str) -> Result<()> {
        let salt_raw = SqliteStore::read_meta_raw(&self.store_path, SALT_KEY)?;
        let salt: [u8; 32] = salt_raw
            .try_into()
            .map_err(|_| VaultError::Storage("salt is wrong length".into()))?;

        let cipher = cipher_from_password(
            self.deriver.as_ref(),
            password.as_bytes(),
            &salt,
        )?;
        let cipher: Arc<dyn Cipher> = Arc::new(cipher);

        // compare against the stored one. We need a temporary cipher since
        // the main one is about to be moved into the store.
        let verify_cipher = cipher_from_password(
            self.deriver.as_ref(),
            password.as_bytes(),
            &salt,
        )?;
        let stored_verify = SqliteStore::read_meta_raw(&self.store_path, VERIFY_KEY)
            .map_err(|_| VaultError::Storage("no verify blob in vault_meta".into()))?;
        let verify_plain = verify_cipher
            .decrypt(&stored_verify)
            .map_err(|_| VaultError::WrongPassword)?;

        if verify_plain != VERIFY_PLAINTEXT {
            return Err(VaultError::WrongPassword);
        }

        let store = Arc::new(SqliteStore::open(&self.store_path, cipher)?);
        store.init()?;

        {
            let mut session = self.session.write().map_err(|e| {
                VaultError::Storage(format!("session lock poisoned: {e}"))
            })?;
            *session = Some(UnlockedSession { store });
        }
        self.extensions.fire_unlock();
        Ok(())
    }

    /// Lock the vault, dropping the cipher key from memory.
    pub fn lock(&self) {
        if let Ok(mut s) = self.session.write() {
            *s = None;
        }
        self.extensions.fire_lock();
    }

    pub fn is_locked(&self) -> bool {
        self.session
            .read()
            .map(|s| s.is_none())
            .unwrap_or(true)
    }

    // ── CRUD ──────────────────────────────────────────────────────────

    pub fn list_entries(&self) -> Result<Vec<EntrySummary>> {
        let mut entries = self.require_session()?.list_entries()?;
        if self.hide_metadata.load(std::sync::atomic::Ordering::Relaxed) {
            for e in &mut entries {
                e.title = "[hidden]".into();
                e.username = None;
                e.url = None;
            }
        }
        Ok(entries)
    }

    pub fn get_entry(&self, id: &EntryId) -> Result<Entry> {
        let store = self.require_session()?;
        let entry = store.get_entry(id)?;
        self.extensions.fire_entry_accessed(&entry);
        Ok(entry)
    }

    pub fn create_entry(&self, entry: &Entry) -> Result<()> {
        let store = self.require_session()?;
        store.create_entry(entry)?;
        self.extensions.fire_entry_created(entry);
        Ok(())
    }

    pub fn update_entry(&self, entry: &Entry) -> Result<()> {
        let store = self.require_session()?;
        store.update_entry(entry)?;
        self.extensions.fire_entry_updated(entry);
        Ok(())
    }

    pub fn delete_entry(&self, id: &EntryId) -> Result<()> {
        let store = self.require_session()?;
        store.delete_entry(id)?;
        self.extensions.fire_entry_deleted(id);
        Ok(())
    }

    pub fn search(&self, query: &str) -> Result<Vec<EntrySummary>> {
        self.require_session()?.search(query)
    }

    // ── Helpers ───────────────────────────────────────────────────────

    fn require_session(&self) -> Result<Arc<SqliteStore>> {
        let session = self
            .session
            .read()
            .map_err(|e| VaultError::Storage(format!("session lock poisoned: {e}")))?;
        session
            .as_ref()
            .map(|s| s.store.clone())
            .ok_or(VaultError::Locked)
    }
}
