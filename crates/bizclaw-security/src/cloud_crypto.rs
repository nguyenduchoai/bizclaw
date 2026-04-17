//! Cloud Crypto for Multi-Tenant Data.
//!
//! Provides secure storage and retrieval of API keys, tokens, and
//! other sensitive data using AES-256-GCM encryption (Authenticated Encryption).
//!
//! Unlike local vault ECB/CBC implementations, GCM ensures both confidentiality
//! and cryptographic integrity, which is required for database-stored secrets
//! in a multi-tenant cloud environment to prevent padding oracle attacks.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use bizclaw_core::error::{BizClawError, Result};
use rand::RngCore;

const GCM_PREFIX: &str = "GCM:";

/// Encrypts plaintext using AES-256-GCM with a specific 32-byte key.
/// Returns a Base64-encoded string containing the prefix, nonce, and ciphertext.
/// Result Format: `GCM:<base64(Nonce(12 bytes) + Ciphertext + AuthTag(16 bytes))>`
pub fn encrypt_aes256_gcm(data: &[u8], key: &[u8; 32]) -> Result<String> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per encryption

    let ciphertext = cipher
        .encrypt(&nonce, data)
        .map_err(|e| BizClawError::Security(format!("AES-256-GCM encryption failed: {}", e)))?;

    // Prepend the nonce to the ciphertext
    let mut payload = Vec::with_capacity(nonce.len() + ciphertext.len());
    payload.extend_from_slice(nonce.as_slice());
    payload.extend_from_slice(&ciphertext);

    Ok(format!("{}{}", GCM_PREFIX, BASE64.encode(&payload)))
}

/// Decrypts an AES-256-GCM Base64-encoded string using the provided 32-byte key.
/// Expects the format `GCM:<base64(Nonce + Ciphertext + AuthTag)>`.
pub fn decrypt_aes256_gcm(encoded: &str, key: &[u8; 32]) -> Result<String> {
    if !encoded.starts_with(GCM_PREFIX) {
        return Err(BizClawError::Security(
            "Ciphertext missing GCM: prefix".into(),
        ));
    }

    let b64_payload = &encoded[GCM_PREFIX.len()..];
    let payload = BASE64
        .decode(b64_payload)
        .map_err(|e| BizClawError::Security(format!("Base64 decode failed: {}", e)))?;

    // The nonce is 12 bytes
    if payload.len() < 12 {
        return Err(BizClawError::Security(
            "Payload too short to contain a valid GCM nonce".into(),
        ));
    }

    let (nonce_bytes, ciphertext) = payload.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new(key.into());
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| BizClawError::Security(format!("AES-256-GCM decryption failed: {}", e)))?;

    String::from_utf8(plaintext).map_err(|e| {
        BizClawError::Security(format!("Decrypted payload produced invalid UTF-8: {}", e))
    })
}

/// Helper function to randomly generate a full new AES-256 32-byte key.
/// Especially useful for dynamic tenant rotation.
pub fn generate_tenant_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_256_gcm_encrypt_decrypt() {
        let key = generate_tenant_key();
        let secret = "sk-ant-api03-xxx-very-sensitive-org-key";

        let encrypted = encrypt_aes256_gcm(secret.as_bytes(), &key).expect("Encryption failed");

        assert!(encrypted.starts_with("GCM:"));
        assert_ne!(encrypted, secret); // Should not expose plaintext

        let decrypted = decrypt_aes256_gcm(&encrypted, &key).expect("Decryption failed");
        assert_eq!(decrypted, secret);
    }

    #[test]
    fn test_aes_256_gcm_tamper_detection() {
        let key = generate_tenant_key();
        let secret = "secret_payment_info";

        let encrypted = encrypt_aes256_gcm(secret.as_bytes(), &key).unwrap();

        // Mess with the base64 payload to simulate tampering
        let tampered = encrypted.clone().replace("A", "B").replace("1", "2");

        let result = decrypt_aes256_gcm(&tampered, &key);
        // Deserialization or decryption tag validation should fail
        assert!(result.is_err());
    }

    #[test]
    fn test_different_encryption_produces_different_ciphertexts() {
        let key = generate_tenant_key();
        let data = "consistent_data";

        let enc1 = encrypt_aes256_gcm(data.as_bytes(), &key).unwrap();
        let enc2 = encrypt_aes256_gcm(data.as_bytes(), &key).unwrap();

        assert_ne!(enc1, enc2); // GCM uses random nonce, ensuring uniqueness
    }
}
