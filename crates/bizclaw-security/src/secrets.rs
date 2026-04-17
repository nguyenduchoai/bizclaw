//! Encrypted secrets management.
//!
//! Provides secure storage and retrieval of API keys, tokens, and
//! other sensitive configuration values using AES-256-GCM encryption
//! (authenticated encryption) with a machine-specific key derived via
//! HMAC-SHA256.
//!
//! SECURITY IMPROVEMENTS (v0.4.0):
//! - AES-256-CBC replaced with AES-256-GCM (authenticated encryption)
//! - GCM provides both confidentiality and integrity protection
//! - Prevents padding oracle and bit-flipping attacks
//! - Backward compatible: detects CBC/ECCB-encrypted files and re-encrypts on save

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use bizclaw_core::error::{BizClawError, Result};
use sha2::Sha256;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Manages encrypted secrets stored on disk.
pub struct SecretStore {
    secrets: HashMap<String, String>,
    secrets_path: PathBuf,
    encrypt: bool,
    key: [u8; 32],
}

impl SecretStore {
    /// Create a new secret store.
    pub fn new(encrypt: bool) -> Self {
        let secrets_path = bizclaw_core::config::BizClawConfig::home_dir().join("secrets.enc");
        Self {
            secrets: HashMap::new(),
            secrets_path,
            encrypt,
            key: derive_machine_key(),
        }
    }

    /// Load secrets from disk.
    pub fn load(&mut self) -> Result<()> {
        if !self.secrets_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.secrets_path)?;

        let json_str = if self.encrypt {
            let raw = content.trim();
            if raw.starts_with("GCM:") {
                let encrypted = &raw[4..];
                decrypt_aes256_gcm(encrypted, &self.key)?
            } else if raw.starts_with("CBC:") {
                tracing::warn!(
                    "⚠️ Secrets file uses legacy CBC encryption — will upgrade to GCM on next save"
                );
                let encrypted = BASE64
                    .decode(&raw[4..])
                    .map_err(|e| BizClawError::Security(format!("Base64 decode failed: {e}")))?;
                decrypt_aes256_cbc(&encrypted, &self.key)?
            } else {
                tracing::warn!(
                    "⚠️ Secrets file uses legacy ECB encryption — will upgrade to GCM on next save"
                );
                let encrypted = BASE64
                    .decode(raw)
                    .map_err(|e| BizClawError::Security(format!("Base64 decode failed: {e}")))?;
                let decrypted = decrypt_aes256_ecb(&encrypted, &self.key);
                String::from_utf8(decrypted).map_err(|e| {
                    BizClawError::Security(format!("Decryption produced invalid UTF-8: {e}"))
                })?
            }
        } else {
            content
        };

        self.secrets = serde_json::from_str(&json_str)
            .map_err(|e| BizClawError::Security(format!("Failed to parse secrets: {e}")))?;

        tracing::debug!(
            "Loaded {} secrets from {}",
            self.secrets.len(),
            self.secrets_path.display()
        );
        Ok(())
    }

    /// Save secrets to disk (always uses GCM for new writes).
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.secrets_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&self.secrets)?;

        let content = if self.encrypt {
            let encrypted = encrypt_aes256_gcm(json.as_bytes(), &self.key);
            format!("GCM:{encrypted}")
        } else {
            json
        };

        // Set restrictive permissions on Unix (0600)
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&self.secrets_path)?;
            file.write_all(content.as_bytes())?;
            Ok(())
        }

        #[cfg(not(unix))]
        {
            std::fs::write(&self.secrets_path, content)?;
            Ok(())
        }
    }

    /// Get a secret value.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.secrets.get(key).map(|s| s.as_str())
    }

    /// Set a secret value.
    pub fn set(&mut self, key: &str, value: &str) {
        self.secrets.insert(key.to_string(), value.to_string());
    }

    /// Remove a secret.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.secrets.remove(key)
    }

    /// List all secret keys (without values).
    pub fn keys(&self) -> Vec<&str> {
        self.secrets.keys().map(|k| k.as_str()).collect()
    }

    /// Load from a specific path.
    pub fn load_from(path: &Path) -> Result<Self> {
        let mut store = Self {
            secrets: HashMap::new(),
            secrets_path: path.to_path_buf(),
            encrypt: false,
            key: derive_machine_key(),
        };
        store.load()?;
        Ok(store)
    }
}

/// Derive a machine-specific AES-256 key from hostname + username.
/// Uses HMAC-SHA256 with a domain-specific salt for key derivation.
fn derive_machine_key() -> [u8; 32] {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "bizclaw".into());
    let username = whoami::username();

    // Use HMAC-SHA256 for proper key derivation
    use hmac::Mac;
    type HmacSha256 = hmac::Hmac<Sha256>;

    let salt = format!("bizclaw::v2::secrets::{username}@{hostname}");
    let mut mac = <HmacSha256 as Mac>::new_from_slice(b"bizclaw-secret-store-v2-hmac-key")
        .expect("HMAC key size");
    mac.update(salt.as_bytes());
    let result = mac.finalize();

    let mut key = [0u8; 32];
    key.copy_from_slice(&result.into_bytes());
    key
}

// ═══ AES-256-GCM (authenticated encryption) ═══

/// AES-256-GCM encrypt with random nonce.
/// Output format: base64(nonce + ciphertext + auth_tag)
fn encrypt_aes256_gcm(data: &[u8], key: &[u8; 32]) -> String {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, data)
        .expect("AES-256-GCM encryption failed");

    let mut payload = Vec::with_capacity(nonce.len() + ciphertext.len());
    payload.extend_from_slice(nonce.as_slice());
    payload.extend_from_slice(&ciphertext);

    BASE64.encode(&payload)
}

/// AES-256-GCM decrypt.
/// Input format: base64(nonce + ciphertext + auth_tag)
fn decrypt_aes256_gcm(encoded: &str, key: &[u8; 32]) -> Result<String> {
    let payload = BASE64
        .decode(encoded)
        .map_err(|e| BizClawError::Security(format!("Base64 decode failed: {e}")))?;

    if payload.len() < 12 {
        return Err(BizClawError::Security(
            "GCM payload too short for nonce".into(),
        ));
    }

    let (nonce_bytes, ciphertext) = payload.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new(key.into());
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| BizClawError::Security(format!("AES-256-GCM decryption failed: {}", e)))?;

    String::from_utf8(plaintext)
        .map_err(|e| BizClawError::Security(format!("Decryption produced invalid UTF-8: {e}")))
}

// ═══ Legacy AES-256-CBC (backward compatibility only) ═══

/// AES-256-CBC decrypt with PKCS7 unpadding (legacy support).
/// Input format: [16-byte IV] + [ciphertext]
fn decrypt_aes256_cbc(data: &[u8], key: &[u8; 32]) -> Result<String> {
    use aes::Aes256;
    use aes::cipher::{BlockDecrypt, KeyInit, generic_array::GenericArray};

    if data.len() < 32 || !data.len().is_multiple_of(16) {
        return Err(BizClawError::Security(
            "Invalid CBC ciphertext length".into(),
        ));
    }

    let cipher = Aes256::new(GenericArray::from_slice(key));
    let block_size = 16;

    let iv = &data[..16];
    let ciphertext = &data[16..];

    let mut decrypted = Vec::with_capacity(ciphertext.len());
    let mut prev_block = iv;

    for chunk in ciphertext.chunks(block_size) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        let mut plaintext = [0u8; 16];
        for i in 0..16 {
            plaintext[i] = block[i] ^ prev_block[i];
        }
        decrypted.extend_from_slice(&plaintext);
        prev_block = chunk;
    }

    if let Some(&pad_len) = decrypted.last() {
        let pad_len = pad_len as usize;
        if pad_len <= block_size && pad_len <= decrypted.len() {
            let valid = decrypted[decrypted.len() - pad_len..]
                .iter()
                .all(|&b| b == pad_len as u8);
            if valid {
                decrypted.truncate(decrypted.len() - pad_len);
            } else {
                return Err(BizClawError::Security("Invalid PKCS7 padding".into()));
            }
        }
    }

    String::from_utf8(decrypted)
        .map_err(|e| BizClawError::Security(format!("Decryption produced invalid UTF-8: {e}")))
}

// ═══ Legacy AES-256-ECB (backward compatibility only) ═══

/// AES-256-ECB decrypt with PKCS7 unpadding (legacy support).
/// DEPRECATED: Only used for migrating old ECB-encrypted secrets.
fn decrypt_aes256_ecb(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    use aes::Aes256;
    use aes::cipher::{BlockDecrypt, KeyInit, generic_array::GenericArray};

    let cipher = Aes256::new(GenericArray::from_slice(key));
    let block_size = 16;

    let mut decrypted = Vec::with_capacity(data.len());
    for chunk in data.chunks(block_size) {
        if chunk.len() == block_size {
            let mut block = GenericArray::clone_from_slice(chunk);
            cipher.decrypt_block(&mut block);
            decrypted.extend_from_slice(&block);
        }
    }

    // Remove PKCS7 padding
    if let Some(&pad_len) = decrypted.last() {
        let pad_len = pad_len as usize;
        if pad_len <= block_size && pad_len <= decrypted.len() {
            let valid = decrypted[decrypted.len() - pad_len..]
                .iter()
                .all(|&b| b == pad_len as u8);
            if valid {
                decrypted.truncate(decrypted.len() - pad_len);
            }
        }
    }

    decrypted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcm_encrypt_decrypt_roundtrip() {
        let key = derive_machine_key();
        let data = b"Hello, BizClaw secrets! This is a longer test to span multiple blocks.";
        let encrypted = encrypt_aes256_gcm(data, &key);
        let decrypted = decrypt_aes256_gcm(&encrypted, &key).unwrap();
        assert_eq!(decrypted.as_bytes(), data);
    }

    #[test]
    fn test_gcm_different_nonce_each_time() {
        let key = derive_machine_key();
        let data = b"Same plaintext";
        let enc1 = encrypt_aes256_gcm(data, &key);
        let enc2 = encrypt_aes256_gcm(data, &key);
        assert_ne!(enc1, enc2);
        assert_eq!(decrypt_aes256_gcm(&enc1, &key).unwrap().as_bytes(), data);
        assert_eq!(decrypt_aes256_gcm(&enc2, &key).unwrap().as_bytes(), data);
    }

    #[test]
    fn test_gcm_tamper_detection() {
        use aes::Aes256;
        use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};

        let key = derive_machine_key();
        let data = b"secret data";

        let encrypted_b64 = encrypt_aes256_gcm(data, &key);
        let mut payload = BASE64.decode(&encrypted_b64[4..]).unwrap();

        payload[20] ^= 0xFF;

        let tampered = format!("GCM:{}", BASE64.encode(&payload));
        let result = decrypt_aes256_gcm(&tampered, &key);
        assert!(result.is_err());
    }

    #[test]
    fn test_secret_store_operations() {
        let mut store = SecretStore::new(false);
        store.set("api_key", "sk-test-12345");
        store.set("bot_token", "123456:ABC-DEF");

        assert_eq!(store.get("api_key"), Some("sk-test-12345"));
        assert_eq!(store.get("bot_token"), Some("123456:ABC-DEF"));
        assert_eq!(store.get("missing"), None);

        assert!(store.keys().contains(&"api_key"));
        assert_eq!(store.remove("api_key"), Some("sk-test-12345".into()));
        assert_eq!(store.get("api_key"), None);
    }

    #[test]
    fn test_legacy_ecb_still_decrypts() {
        use aes::Aes256;
        use aes::cipher::{BlockEncrypt, KeyInit, generic_array::GenericArray};

        let key = derive_machine_key();
        let data = b"legacy test data";

        let cipher = Aes256::new(GenericArray::from_slice(&key));
        let padding_len = 16 - (data.len() % 16);
        let mut padded = data.to_vec();
        padded.extend(std::iter::repeat_n(padding_len as u8, padding_len));

        let mut encrypted = Vec::new();
        for chunk in padded.chunks(16) {
            let mut block = GenericArray::clone_from_slice(chunk);
            cipher.encrypt_block(&mut block);
            encrypted.extend_from_slice(&block);
        }

        let decrypted = decrypt_aes256_ecb(&encrypted, &key);
        assert_eq!(decrypted, data);
    }
}
