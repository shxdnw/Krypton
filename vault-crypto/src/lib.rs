use argon2::{Argon2, Algorithm, Params, Version};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::RngCore;
use vault_core::{Cipher, KeyDeriver, Result, VaultError};

// ── Argon2id key derivation ──────────────────────────────────────────────

/// Argon2id with parameters tuned for a balance of security and interactive
/// performance: 64 MiB memory, 3 iterations, 4 lanes of parallelism.
pub struct Argon2IdDeriver {
    inner: Argon2<'static>,
}

impl Argon2IdDeriver {
    pub fn new() -> Self {
        let params = Params::new(65536, 3, 4, Some(32))
            .expect("hard-coded Argon2 params are valid");
        Self {
            inner: Argon2::new(Algorithm::Argon2id, Version::V0x13, params),
        }
    }
}

impl Default for Argon2IdDeriver {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyDeriver for Argon2IdDeriver {
    fn derive_key(&self, password: &[u8], salt: &[u8]) -> Result<[u8; 32]> {
        let mut key = zeroize::Zeroizing::new([0u8; 32]);
        self.inner
            .hash_password_into(password, salt, &mut *key)
            .map_err(|e| VaultError::Crypto(format!("key derivation failed: {e}")))?;
        Ok(*key)
    }

    fn generate_salt(&self) -> [u8; 32] {
        let mut salt = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut salt);
        salt
    }
}

// ── ChaCha20-Poly1305 AEAD cipher ────────────────────────────────────────

/// Authenticated encryption using XChaCha20-Poly1305.
///
/// Ciphertext format: `[12-byte nonce][encrypted data + 16-byte tag]`
pub struct ChaCha20Cipher {
    aead: ChaCha20Poly1305,
}

impl ChaCha20Cipher {
    pub fn new(key: &[u8; 32]) -> Self {
        Self {
            aead: ChaCha20Poly1305::new_from_slice(key)
                .expect("ChaCha20Poly1305 key is exactly 32 bytes"),
        }
    }
}

impl Cipher for ChaCha20Cipher {
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .aead
            .encrypt(nonce, plaintext)
            .map_err(|e| VaultError::Crypto(format!("encryption failed: {e}")))?;

        // Prepend nonce to ciphertext.
        let mut out = Vec::with_capacity(12 + ciphertext.len());
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        if ciphertext.len() < 12 {
            return Err(VaultError::Crypto("ciphertext too short".into()));
        }
        let (nonce_bytes, encrypted) = ciphertext.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        self.aead
            .decrypt(nonce, encrypted)
            .map_err(|_| VaultError::Crypto("decryption failed".into()))
    }
}

// ── Convenience constructor ──────────────────────────────────────────────

/// Derive a 256-bit key from a password + salt and wrap it in a
/// [`ChaCha20Cipher`].
pub fn cipher_from_password(
    deriver: &dyn KeyDeriver,
    pw: &[u8],
    salt: &[u8],
) -> Result<ChaCha20Cipher> {
    let key = deriver.derive_key(pw, salt)?;
    Ok(ChaCha20Cipher::new(&key))
}
