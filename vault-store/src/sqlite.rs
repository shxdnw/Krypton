use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};
use vault_core::{
    Cipher, Entry, EntryId, EntrySummary, Result, Store, VaultError,
};

/// SQLite-backed persistent vault storage with FTS5 full-text search.
pub struct SqliteStore {
    conn: Mutex<Connection>,
    cipher: Arc<dyn Cipher>,
}

impl SqliteStore {
    pub fn open(path: &Path, cipher: Arc<dyn Cipher>) -> Result<Self> {
        let conn = Connection::open(path).map_err(|e| {
            VaultError::Storage(format!("cannot open database: {e}"))
        })?;
        Ok(Self {
            conn: Mutex::new(conn),
            cipher,
        })
    }

    /// Read a single key from `vault_meta` without needing a cipher.
    ///
    /// This is used during unlock to fetch the salt *before* the encryption
    /// key has been derived.
    pub fn read_meta_raw(path: &Path, key: &str) -> Result<Vec<u8>> {
        let conn = Connection::open(path).map_err(|e| {
            VaultError::Storage(format!("cannot open database: {e}"))
        })?;
        conn.query_row(
            "SELECT value FROM vault_meta WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .map_err(|_| VaultError::NotFound(format!("vault_meta key: {key}")))
    }
}

impl Store for SqliteStore {
    fn init(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            VaultError::Storage(format!("lock poisoned: {e}"))
        })?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| VaultError::Storage(format!("pragma failed: {e}")))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS vault_meta (
                key   TEXT PRIMARY KEY,
                value BLOB NOT NULL
            );

            CREATE TABLE IF NOT EXISTS entries (
                id             TEXT    PRIMARY KEY,
                title          TEXT    NOT NULL,
                username       TEXT,
                url            TEXT,
                tags           TEXT    NOT NULL DEFAULT '[]',
                encrypted_data BLOB    NOT NULL,
                created_at     INTEGER NOT NULL,
                updated_at     INTEGER NOT NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(
                title, username, url, tags,
                content='entries', content_rowid='rowid'
            );

            -- Keep FTS5 index in sync with the entries table.
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
        .map_err(|e| VaultError::Storage(format!("schema init failed: {e}")))?;

        Ok(())
    }

    fn list_entries(&self) -> Result<Vec<EntrySummary>> {
        let conn = self.conn.lock().map_err(|e| {
            VaultError::Storage(format!("lock poisoned: {e}"))
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, title, username, url, tags, updated_at
                 FROM entries ORDER BY updated_at DESC",
            )
            .map_err(|e| VaultError::Storage(format!("prepare: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let tags_str: String = row.get(4)?;
                Ok((
                    id_str,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    tags_str,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(|e| VaultError::Storage(format!("query: {e}")))?;

        let mut entries = Vec::new();
        for row in rows {
            let (id_str, title, username, url, tags_str, updated_at) =
                row.map_err(|e| VaultError::Storage(format!("row: {e}")))?;
            let tags: Vec<String> = serde_json::from_str(&tags_str)
                .map_err(|e| VaultError::Storage(format!("tags deserialize: {e}")))?;
            let uuid = uuid::Uuid::parse_str(&id_str)
                .map_err(|_| VaultError::Storage(format!("invalid uuid: {id_str}")))?;
            entries.push(EntrySummary {
                id: vault_core::EntryId(uuid),
                title,
                username,
                url,
                tags,
                updated_at,
            });
        }
        Ok(entries)
    }

    fn get_entry(&self, id: &EntryId) -> Result<Entry> {
        let conn = self.conn.lock().map_err(|e| {
            VaultError::Storage(format!("lock poisoned: {e}"))
        })?;

        let id_str = id.to_string();
        let mut stmt = conn
            .prepare("SELECT encrypted_data FROM entries WHERE id = ?1")
            .map_err(|e| VaultError::Storage(format!("prepare: {e}")))?;

        let blob: Vec<u8> = stmt
            .query_row(params![id_str], |row| row.get(0))
            .map_err(|_| VaultError::NotFound(id_str))?;

        let plain = zeroize::Zeroizing::new(self.cipher.decrypt(&blob)?);
        let entry_data: vault_core::EntryData = serde_json::from_slice(&plain)
            .map_err(|e| VaultError::Storage(format!("deserialize: {e}")))?;
        Ok(entry_data.into_entry())
    }

    fn create_entry(&self, entry: &Entry) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            VaultError::Storage(format!("lock poisoned: {e}"))
        })?;

        let data = vault_core::EntryData::from(entry);
        let json = zeroize::Zeroizing::new(
            serde_json::to_vec(&data)
                .map_err(|e| VaultError::Storage(format!("serialize: {e}")))?,
        );
        let blob = self.cipher.encrypt(&json)?;
        drop(data);
        let tags_json = serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".into());

        conn.execute(
            "INSERT INTO entries (id, title, username, url, tags, encrypted_data, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.id.to_string(),
                entry.title,
                entry.username,
                entry.url,
                tags_json,
                blob,
                entry.created_at,
                entry.updated_at,
            ],
        )
        .map_err(|e| VaultError::Storage(format!("insert: {e}")))?;

        Ok(())
    }

    fn update_entry(&self, entry: &Entry) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            VaultError::Storage(format!("lock poisoned: {e}"))
        })?;

        let data = vault_core::EntryData::from(entry);
        let json = zeroize::Zeroizing::new(
            serde_json::to_vec(&data)
                .map_err(|e| VaultError::Storage(format!("serialize: {e}")))?,
        );
        let blob = self.cipher.encrypt(&json)?;
        drop(data);
        let tags_json = serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".into());

        let affected = conn
            .execute(
                "UPDATE entries SET title=?1, username=?2, url=?3, tags=?4,
                         encrypted_data=?5, updated_at=?6
                 WHERE id=?7",
                params![
                    entry.title,
                    entry.username,
                    entry.url,
                    tags_json,
                    blob,
                    entry.updated_at,
                    entry.id.to_string(),
                ],
            )
            .map_err(|e| VaultError::Storage(format!("update: {e}")))?;

        if affected == 0 {
            return Err(VaultError::NotFound(entry.id.to_string()));
        }
        Ok(())
    }

    fn delete_entry(&self, id: &EntryId) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            VaultError::Storage(format!("lock poisoned: {e}"))
        })?;

        let id_str = id.to_string();
        let affected = conn
            .execute("DELETE FROM entries WHERE id = ?1", params![id_str])
            .map_err(|e| VaultError::Storage(format!("delete: {e}")))?;

        if affected == 0 {
            return Err(VaultError::NotFound(id_str));
        }
        Ok(())
    }

    fn search(&self, query: &str) -> Result<Vec<EntrySummary>> {
        let conn = self.conn.lock().map_err(|e| {
            VaultError::Storage(format!("lock poisoned: {e}"))
        })?;

        // prefix matching. Multi-word queries use implicit AND.
        let escaped = query.replace('"', "\"\"");
        let terms: Vec<String> = escaped
            .split_whitespace()
            .map(|t| format!("{t}*"))
            .collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let fts_query = terms.join(" ");

        let mut stmt = conn
            .prepare(
                "SELECT rowid, id, title, username, url, tags, updated_at
                 FROM entries_fts WHERE entries_fts MATCH ?1 ORDER BY rank",
            )
            .map_err(|e| VaultError::Storage(format!("prepare: {e}")))?;

        let rows = stmt.query_map(params![fts_query], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
            ))
        });

        match rows {
            Ok(iter) => {
                let mut results = Vec::new();
                for row in iter {
                    let (id_str, title, username, url, tags_str, updated_at) =
                        row.map_err(|e| VaultError::Storage(format!("row: {e}")))?;
                    let tags: Vec<String> = serde_json::from_str(&tags_str)
                        .map_err(|e| VaultError::Storage(format!("tags deserialize: {e}")))?;
                    let uuid = uuid::Uuid::parse_str(&id_str)
                        .map_err(|_| VaultError::Storage(format!("invalid uuid: {id_str}")))?;
                    results.push(EntrySummary {
                        id: vault_core::EntryId(uuid),
                        title,
                        username,
                        url,
                        tags,
                        updated_at,
                    });
                }
                Ok(results)
            }
            // FTS5 returns an error for empty/invalid queries; return empty results.
            Err(_) => Ok(Vec::new()),
        }
    }

    fn set_vault_meta(&self, key: &str, value: &[u8]) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| {
            VaultError::Storage(format!("lock poisoned: {e}"))
        })?;
        conn.execute(
            "INSERT OR REPLACE INTO vault_meta (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| VaultError::Storage(format!("set_vault_meta: {e}")))?;
        Ok(())
    }

    fn get_vault_meta(&self, key: &str) -> Result<Vec<u8>> {
        let conn = self.conn.lock().map_err(|e| {
            VaultError::Storage(format!("lock poisoned: {e}"))
        })?;
        conn.query_row(
            "SELECT value FROM vault_meta WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .map_err(|_| VaultError::NotFound(format!("vault_meta key: {key}")))
    }
}
