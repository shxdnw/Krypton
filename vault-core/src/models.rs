use chrono::Utc;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use uuid::Uuid;


/// Unique identifier for a vault entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryId(pub Uuid);

impl EntryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for EntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── FieldValue (with manual serde for SecretString safety) ───────────────

/// Typed value of a custom field.
///
/// The `Secret` variant wraps [`SecretString`] so its contents are redacted
/// from logs and zeroized on drop. Manual `Serialize` / `Deserialize`
/// implementations expose the secret only during the brief window when the
/// entry is being serialised into the encrypted blob.
#[derive(Clone)]
pub enum FieldValue {
    Text(String),
    Secret(SecretString),
    Totp(String),
}

impl fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(v) => f.debug_tuple("Text").field(v).finish(),
            Self::Secret(_) => f.write_str("Secret([REDACTED])"),
            Self::Totp(v) => f.debug_tuple("Totp").field(v).finish(),
        }
    }
}

impl Serialize for FieldValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("FieldValue", 2)?;
        match self {
            FieldValue::Text(v) => {
                s.serialize_field("type", "Text")?;
                s.serialize_field("value", v)?;
            }
            FieldValue::Secret(v) => {
                s.serialize_field("type", "Secret")?;
                s.serialize_field("value", v.expose_secret())?;
            }
            FieldValue::Totp(v) => {
                s.serialize_field("type", "Totp")?;
                s.serialize_field("value", v)?;
            }
        }
        s.end()
    }
}

impl<'de> Deserialize<'de> for FieldValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;

        #[derive(Deserialize)]
        struct Helper {
            #[serde(rename = "type")]
            ty: String,
            value: String,
        }

        let h = Helper::deserialize(deserializer)?;
        match h.ty.as_str() {
            "Text" => Ok(FieldValue::Text(h.value)),
            "Secret" => Ok(FieldValue::Secret(SecretString::new(h.value.into()))),
            "Totp" => Ok(FieldValue::Totp(h.value)),
            other => Err(D::Error::custom(format!("unknown FieldValue type: {other}"))),
        }
    }
}


/// A user-defined extra field attached to an entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub label: String,
    pub value: FieldValue,
}


/// The full decrypted entry held in memory during an unlocked session.
///
/// The password is stored as [`SecretString`] so it is never logged, displayed
/// in debug output, or left in memory after the entry is dropped.
#[derive(Debug, Clone)]
pub struct Entry {
    pub id: EntryId,
    pub title: String,
    pub username: Option<String>,
    pub password: SecretString,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub custom_fields: Vec<CustomField>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Entry {
    pub fn new(title: String, password: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: EntryId::new(),
            title,
            username: None,
            password: SecretString::new(password.into()),
            url: None,
            notes: None,
            tags: Vec::new(),
            custom_fields: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}


/// Metadata-only view of an entry, safe for list rendering (no secrets).
#[derive(Debug, Clone)]
pub struct EntrySummary {
    pub id: EntryId,
    pub title: String,
    pub username: Option<String>,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub updated_at: i64,
}

impl From<&Entry> for EntrySummary {
    fn from(entry: &Entry) -> Self {
        Self {
            id: entry.id.clone(),
            title: entry.title.clone(),
            username: entry.username.clone(),
            url: entry.url.clone(),
            tags: entry.tags.clone(),
            updated_at: entry.updated_at,
        }
    }
}

// ── EntryData (serde mirror, transient, zeroized on drop) ────────────────

/// Serde-compatible mirror of [`Entry`] where the password is a plain
/// [`String`]. This type is *only* used transiently inside encrypted blobs —
/// it is serialised to JSON, encrypted, and then immediately zeroized.
#[derive(Serialize, Deserialize)]
pub struct EntryData {
    pub id: EntryId,
    pub title: String,
    pub username: Option<String>,
    pub password: String,
    pub url: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub custom_fields: Vec<CustomField>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Drop for EntryData {
    fn drop(&mut self) {
        // The password is a bare String (not SecretString), so we zeroize it.
        zeroize::Zeroize::zeroize(&mut self.password);
        // FieldValue::Secret uses SecretString, which zeroizes on its own Drop
        // — no extra handling needed here.
    }
}

impl From<&Entry> for EntryData {
    fn from(entry: &Entry) -> Self {
        Self {
            id: entry.id.clone(),
            title: entry.title.clone(),
            username: entry.username.clone(),
            password: entry.password.expose_secret().clone(),
            url: entry.url.clone(),
            notes: entry.notes.clone(),
            tags: entry.tags.clone(),
            custom_fields: entry.custom_fields.clone(),
            created_at: entry.created_at,
            updated_at: entry.updated_at,
        }
    }
}

impl EntryData {
    /// Reconstruct a runtime [`Entry`] with the password wrapped in
    /// [`SecretString`].
    pub fn into_entry(self) -> Entry {
        Entry {
            id: self.id.clone(),
            title: self.title.clone(),
            username: self.username.clone(),
            password: SecretString::new(self.password.clone().into()),
            url: self.url.clone(),
            notes: self.notes.clone(),
            tags: self.tags.clone(),
            custom_fields: self.custom_fields.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
