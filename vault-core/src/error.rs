use std::io;

/// Central error type for all vault operations.
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("wrong master password")]
    WrongPassword,

    #[error("vault is locked")]
    Locked,

    #[error("entry not found: {0}")]
    NotFound(String),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("extension '{name}' error: {reason}")]
    Extension { name: String, reason: String },

    #[error(transparent)]
    Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, VaultError>;
