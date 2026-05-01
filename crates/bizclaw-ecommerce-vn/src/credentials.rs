//! Credential Management for Vietnamese E-commerce Platforms
//!
//! Securely stores and retrieves API credentials for e-commerce platforms.
//! Uses AES-256-GCM encryption for stored credentials.

use crate::types::EcommercePlatform;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::RwLock;

const ENCRYPTION_KEY_ENV: &str = "BIZCLAW_ECOM_KEY";
const CREDENTIALS_FILE: &str = "vn_ecom_credentials.enc";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlatformCredential {
    #[serde(rename = "sapo")]
    Sapo {
        store: String,
        api_key: String,
        api_secret: String,
    },
    #[serde(rename = "haravan")]
    Haravan {
        access_token: String,
    },
    #[serde(rename = "kiotviet")]
    KiotViet {
        client_id: String,
        client_secret: String,
        retailer: String,
    },
    #[serde(rename = "ladisales")]
    LadiSales {
        api_key: String,
        store_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedCredential {
    pub platform: String,
    pub encrypted_data: String,
    pub nonce: String,
    pub version: u32,
}

pub struct CredentialManager {
    store: RwLock<HashMap<EcommercePlatform, PlatformCredential>>,
    storage_path: PathBuf,
    cipher: Aes256Gcm,
}

impl CredentialManager {
    pub fn new() -> Self {
        let key = Self::get_or_create_key();
        let cipher = Aes256Gcm::new_from_slice(&key)
            .expect("Invalid encryption key length");

        let storage_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("bizclaw")
            .join(CREDENTIALS_FILE);

        Self {
            store: RwLock::new(HashMap::new()),
            storage_path,
            cipher,
        }
    }

    pub fn with_storage_path(path: PathBuf) -> Self {
        let key = Self::get_or_create_key();
        let cipher = Aes256Gcm::new_from_slice(&key)
            .expect("Invalid encryption key length");

        Self {
            store: RwLock::new(HashMap::new()),
            storage_path: path,
            cipher,
        }
    }

    fn get_or_create_key() -> [u8; 32] {
        use std::env;

        if let Ok(key_str) = env::var(ENCRYPTION_KEY_ENV) {
            if let Ok(key) = BASE64.decode(&key_str) {
                if key.len() == 32 {
                    let mut key_array = [0u8; 32];
                    key_array.copy_from_slice(&key);
                    return key_array;
                }
            }
        }

        let key = generate_random_key();
        unsafe { env::set_var(ENCRYPTION_KEY_ENV, BASE64.encode(key)); }
        key
    }

    pub async fn store(
        &self,
        platform: EcommercePlatform,
        credential: &PlatformCredential,
    ) -> Result<()> {
        let mut store = self.store.write().await;
        store.insert(platform, credential.clone());
        drop(store);

        self.persist().await
    }

    pub async fn get(&self, platform: &EcommercePlatform) -> Option<PlatformCredential> {
        let store = self.store.read().await;
        store.get(platform).cloned()
    }

    pub async fn remove(&self, platform: &EcommercePlatform) -> Result<()> {
        let mut store = self.store.write().await;
        store.remove(platform);
        drop(store);

        self.persist().await
    }

    pub async fn list_platforms(&self) -> Vec<EcommercePlatform> {
        let store = self.store.read().await;
        store.keys().cloned().collect()
    }

    pub async fn has_credential(&self, platform: &EcommercePlatform) -> bool {
        let store = self.store.read().await;
        store.contains_key(platform)
    }

    async fn persist(&self) -> Result<()> {
        let store = self.store.read().await;

        if store.is_empty() {
            if self.storage_path.exists() {
                fs::remove_file(&self.storage_path).await?;
            }
            return Ok(());
        }

        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let data = serde_json::to_vec(&*store)
            .context("Failed to serialize credentials")?;

        let nonce_bytes = rand::random::<[u8; 12]>();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self.cipher
            .encrypt(nonce, data.as_ref())
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

        let encrypted = EncryptedCredential {
            platform: "all".to_string(),
            encrypted_data: BASE64.encode(&ciphertext),
            nonce: BASE64.encode(&nonce_bytes),
            version: 1,
        };

        let json = serde_json::to_string_pretty(&encrypted)
            .context("Failed to serialize encrypted data")?;

        fs::write(&self.storage_path, json).await?;

        tracing::debug!("Credentials persisted to {:?}", self.storage_path);
        Ok(())
    }

    pub async fn load(&self) -> Result<()> {
        if !self.storage_path.exists() {
            tracing::debug!("No credentials file found at {:?}", self.storage_path);
            return Ok(());
        }

        let content = fs::read_to_string(&self.storage_path).await?;
        let encrypted: EncryptedCredential = serde_json::from_str(&content)
            .context("Failed to parse encrypted credentials")?;

        let ciphertext = BASE64.decode(&encrypted.encrypted_data)
            .context("Failed to decode ciphertext")?;
        let nonce_bytes = BASE64.decode(&encrypted.nonce)
            .context("Failed to decode nonce")?;

        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = self.cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;

        let store_data: HashMap<EcommercePlatform, PlatformCredential> =
            serde_json::from_slice(&plaintext)
                .context("Failed to deserialize credentials")?;

        let mut store = self.store.write().await;
        *store = store_data;

        tracing::info!("Loaded {} credentials from storage", store.len());
        Ok(())
    }

    pub fn get_sapo_credential(&self, platform: &EcommercePlatform) -> Option<(String, String, String)> {
        match platform {
            EcommercePlatform::Sapo => {
                if let Some(PlatformCredential::Sapo { store, api_key, api_secret }) =
                    self.store.blocking_read().get(platform)
                {
                    Some((store.clone(), api_key.clone(), api_secret.clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn get_haravan_credential(&self) -> Option<String> {
        if let Some(PlatformCredential::Haravan { access_token }) =
            self.store.blocking_read().get(&EcommercePlatform::Haravan)
        {
            Some(access_token.clone())
        } else {
            None
        }
    }

    pub fn get_kiotviet_credential(&self) -> Option<(String, String, String)> {
        if let Some(PlatformCredential::KiotViet { client_id, client_secret, retailer }) =
            self.store.blocking_read().get(&EcommercePlatform::KiotViet)
        {
            Some((client_id.clone(), client_secret.clone(), retailer.clone()))
        } else {
            None
        }
    }
}

impl Default for CredentialManager {
    fn default() -> Self {
        Self::new()
    }
}

fn generate_random_key() -> [u8; 32] {
    use rand::RngCore;
    let mut key = [0u8; 32];
    rand::Rng::fill(&mut rand::thread_rng(), &mut key);
    key
}

mod dirs {
    use std::path::PathBuf;

    pub fn data_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var("APPDATA").ok().map(PathBuf::from)
        }
        #[cfg(target_os = "macos")]
        {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
        }
        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_DATA_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".local").join("share")))
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let manager = CredentialManager::with_storage_path(
            std::env::temp_dir().join("test_credentials.enc"),
        );

        let credential = PlatformCredential::Sapo {
            store: "teststore".to_string(),
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
        };

        manager.store(EcommercePlatform::Sapo, &credential).await.unwrap();

        let retrieved = manager.get(&EcommercePlatform::Sapo).await;
        assert!(retrieved.is_some());

        manager.remove(&EcommercePlatform::Sapo).await.unwrap();
        let removed = manager.get(&EcommercePlatform::Sapo).await;
        assert!(removed.is_none());
    }
}
