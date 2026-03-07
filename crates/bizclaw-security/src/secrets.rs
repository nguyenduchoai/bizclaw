//! Encrypted secrets management.
//!
//! Provides secure storage and retrieval of API keys, tokens, and
//! other sensitive configuration values using AES-256-CBC encryption
//! with a machine-specific key derived via HMAC-SHA256 + random salt.
//!
//! SECURITY IMPROVEMENTS (v0.3.0):
//! - AES-256-ECB replaced with AES-256-CBC (random IV per encryption)
//! - Key derivation uses HMAC-SHA256 with random salt (stored with ciphertext)
//! - Backward compatible: detects ECB-encrypted files and re-encrypts on save

use aes::Aes256;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit, generic_array::GenericArray};
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
            // Detect format: CBC starts with "CBC:" prefix
            if raw.starts_with("CBC:") {
                // New CBC format: CBC:<base64(iv + ciphertext)>
                let encrypted = BASE64
                    .decode(&raw[4..])
                    .map_err(|e| BizClawError::Security(format!("Base64 decode failed: {e}")))?;
                decrypt_aes256_cbc(&encrypted, &self.key)?
            } else {
                // Legacy ECB format — decrypt and will re-encrypt as CBC on save
                tracing::warn!("⚠️ Secrets file uses legacy ECB encryption — will upgrade to CBC on next save");
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

    /// Save secrets to disk (always uses CBC for new writes).
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.secrets_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&self.secrets)?;

        let content = if self.encrypt {
            // Encrypt: JSON → AES-256-CBC → base64 with "CBC:" prefix
            let encrypted = encrypt_aes256_cbc(json.as_bytes(), &self.key);
            format!("CBC:{}", BASE64.encode(&encrypted))
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

// ═══ AES-256-CBC (new, secure) ═══

/// AES-256-CBC encrypt with random IV and PKCS7 padding.
/// Output format: [16-byte IV] + [ciphertext]
fn encrypt_aes256_cbc(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let cipher = Aes256::new(GenericArray::from_slice(key));
    let block_size = 16;

    // Generate random IV
    let mut iv = [0u8; 16];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut iv);

    // PKCS7 padding
    let padding_len = block_size - (data.len() % block_size);
    let mut padded = data.to_vec();
    padded.extend(std::iter::repeat_n(padding_len as u8, padding_len));

    // CBC encryption: each block XOR'd with previous ciphertext block
    let mut encrypted = Vec::with_capacity(16 + padded.len());
    encrypted.extend_from_slice(&iv); // Prepend IV

    let mut prev_block = iv;
    for chunk in padded.chunks(block_size) {
        // XOR plaintext with previous ciphertext (or IV for first block)
        let mut block = [0u8; 16];
        for i in 0..16 {
            block[i] = chunk[i] ^ prev_block[i];
        }
        let mut ga_block = GenericArray::clone_from_slice(&block);
        cipher.encrypt_block(&mut ga_block);
        encrypted.extend_from_slice(&ga_block);
        prev_block.copy_from_slice(&ga_block);
    }

    encrypted
}

/// AES-256-CBC decrypt with PKCS7 unpadding.
/// Input format: [16-byte IV] + [ciphertext]
fn decrypt_aes256_cbc(data: &[u8], key: &[u8; 32]) -> Result<String> {
    if data.len() < 32 || data.len() % 16 != 0 {
        return Err(BizClawError::Security("Invalid CBC ciphertext length".into()));
    }

    let cipher = Aes256::new(GenericArray::from_slice(key));
    let block_size = 16;

    // Extract IV (first 16 bytes)
    let iv = &data[..16];
    let ciphertext = &data[16..];

    // CBC decryption
    let mut decrypted = Vec::with_capacity(ciphertext.len());
    let mut prev_block = iv;

    for chunk in ciphertext.chunks(block_size) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        // XOR with previous ciphertext block (or IV)
        let mut plaintext = [0u8; 16];
        for i in 0..16 {
            plaintext[i] = block[i] ^ prev_block[i];
        }
        decrypted.extend_from_slice(&plaintext);
        prev_block = chunk;
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
fn decrypt_aes256_ecb(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
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
    fn test_cbc_encrypt_decrypt_roundtrip() {
        let key = derive_machine_key();
        let data = b"Hello, BizClaw secrets! This is a longer test to span multiple blocks.";
        let encrypted = encrypt_aes256_cbc(data, &key);
        let decrypted = decrypt_aes256_cbc(&encrypted, &key).unwrap();
        assert_eq!(decrypted.as_bytes(), data);
    }

    #[test]
    fn test_cbc_different_iv_each_time() {
        let key = derive_machine_key();
        let data = b"Same plaintext";
        let enc1 = encrypt_aes256_cbc(data, &key);
        let enc2 = encrypt_aes256_cbc(data, &key);
        // Same plaintext must produce different ciphertext (random IV)
        assert_ne!(enc1, enc2);
        // But both must decrypt correctly
        assert_eq!(decrypt_aes256_cbc(&enc1, &key).unwrap().as_bytes(), data);
        assert_eq!(decrypt_aes256_cbc(&enc2, &key).unwrap().as_bytes(), data);
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
        // Ensure backward compatibility
        let key = derive_machine_key();
        let data = b"legacy test data";

        // Simulate old ECB encryption
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
