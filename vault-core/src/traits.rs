use crate::{Entry, EntryId, EntrySummary, Result};

/// Persistence layer for vault entries.
pub trait Store: Send + Sync + 'static {
    /// Initialise the store (create tables, run migrations, etc.).
    fn init(&self) -> Result<()>;

    /// Return metadata for every entry, ordered by most recently updated first.
    fn list_entries(&self) -> Result<Vec<EntrySummary>>;

    /// Load the full (decrypted) entry for `id`.
    fn get_entry(&self, id: &EntryId) -> Result<Entry>;

    /// Persist a new entry.
    fn create_entry(&self, entry: &Entry) -> Result<()>;

    /// Overwrite an existing entry.
    fn update_entry(&self, entry: &Entry) -> Result<()>;

    /// Remove an entry by id.
    fn delete_entry(&self, id: &EntryId) -> Result<()>;

    /// Full-text search across title, username, url, and tags.
    fn search(&self, query: &str) -> Result<Vec<EntrySummary>>;
}

/// Symmetric authenticated encryption.
pub trait Cipher: Send + Sync + 'static {
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>>;
    fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>>;
}

/// Password-based key derivation.
pub trait KeyDeriver: Send + Sync + 'static {
    fn derive_key(&self, password: &[u8], salt: &[u8]) -> Result<[u8; 32]>;
    fn generate_salt(&self) -> [u8; 32];
}
