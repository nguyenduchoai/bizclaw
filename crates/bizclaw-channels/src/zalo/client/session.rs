//! Zalo session management — cookie jar, keep-alive, reconnection, file persistence.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Zalo session state — stores all crypto keys and session info.
/// Serializable for file-based persistence across restarts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZaloSession {
    /// User ID
    pub uid: String,
    /// Encrypted key for WebSocket event decryption (zpw_enk → used as cipher_key in decode_event_data)
    pub zpw_enk: Option<String>,
    /// Service key (zpw_key)
    pub zpw_key: Option<String>,
    /// Secret key from cookie (zpw_sek) — used for AES-CBC request param encryption
    pub secret_key: Option<String>,
    /// Cookie string — needed for HTTP API requests
    pub cookie: Option<String>,
    /// IMEI device fingerprint
    pub imei: Option<String>,
    /// Encrypted ZCID from ParamsEncryptor (sent with auth requests)
    pub zcid: Option<String>,
    /// Derived encrypt key from ParamsEncryptor (for encrypting login payloads)
    pub encrypt_key: Option<String>,
    /// WebSocket URL(s)
    pub ws_url: Option<String>,
    /// Session active flag
    pub active: bool,
    /// Last heartbeat timestamp
    pub last_heartbeat: u64,
}

/// Thread-safe session manager with file persistence.
pub struct SessionManager {
    session: Arc<RwLock<ZaloSession>>,
    /// Path to persist session state (JSON file)
    persist_path: Option<String>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            session: Arc::new(RwLock::new(ZaloSession::default())),
            persist_path: None,
        }
    }

    /// Create with a persistence file path.
    pub fn with_persist_path(path: &str) -> Self {
        Self {
            session: Arc::new(RwLock::new(ZaloSession::default())),
            persist_path: Some(path.to_string()),
        }
    }

    /// Get a cloned Arc to the session for background tasks (e.g. watchdog).
    pub fn get_arc(&self) -> Arc<RwLock<ZaloSession>> {
        self.session.clone()
    }

    /// Update session after login with all required crypto keys.
    pub async fn set_session(
        &self,
        uid: String,
        zpw_enk: Option<String>,
        zpw_key: Option<String>,
        secret_key: Option<String>,
        cookie: Option<String>,
        imei: Option<String>,
    ) {
        let mut session = self.session.write().await;
        session.uid = uid;
        session.zpw_enk = zpw_enk;
        session.zpw_key = zpw_key;
        session.secret_key = secret_key;
        session.cookie = cookie;
        session.imei = imei;
        session.active = true;
        session.last_heartbeat = current_timestamp();
        drop(session);
        // Auto-persist after login
        self.save_to_file().await;
    }

    /// Get the secret key for encrypting API params.
    pub async fn secret_key(&self) -> Option<String> {
        self.session.read().await.secret_key.clone()
    }

    /// Get the cipher key for decrypting WebSocket events (zpw_enk).
    pub async fn cipher_key(&self) -> Option<String> {
        self.session.read().await.zpw_enk.clone()
    }

    /// Get the stored cookie.
    pub async fn cookie(&self) -> Option<String> {
        self.session.read().await.cookie.clone()
    }

    /// Check if session is active.
    pub async fn is_active(&self) -> bool {
        let session = self.session.read().await;
        session.active
    }

    /// Get current user ID.
    pub async fn uid(&self) -> String {
        self.session.read().await.uid.clone()
    }

    /// Update heartbeat timestamp.
    pub async fn heartbeat(&self) {
        let mut session = self.session.write().await;
        session.last_heartbeat = current_timestamp();
    }

    /// Invalidate session.
    pub async fn invalidate(&self) {
        let mut session = self.session.write().await;
        session.active = false;
        drop(session);
        self.save_to_file().await;
    }

    /// Get session clone.
    pub async fn get_session(&self) -> ZaloSession {
        self.session.read().await.clone()
    }

    /// Check if session is stale (no heartbeat for > threshold_secs).
    pub async fn is_stale(&self, threshold_secs: u64) -> bool {
        let session = self.session.read().await;
        if !session.active {
            return true;
        }
        let now = current_timestamp();
        now.saturating_sub(session.last_heartbeat) > threshold_secs
    }

    // ── File Persistence ─────────────────────────────────────

    /// Save session state to JSON file.
    pub async fn save_to_file(&self) {
        let path = match &self.persist_path {
            Some(p) => p.clone(),
            None => return,
        };
        let session = self.session.read().await.clone();
        match serde_json::to_string_pretty(&session) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    tracing::warn!("Failed to persist Zalo session to {}: {}", path, e);
                } else {
                    tracing::debug!("Zalo session persisted to {}", path);
                }
            }
            Err(e) => tracing::warn!("Failed to serialize Zalo session: {}", e),
        }
    }

    /// Load session state from JSON file. Returns true if loaded successfully.
    pub async fn load_from_file(&self) -> bool {
        let path = match &self.persist_path {
            Some(p) => p.clone(),
            None => return false,
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        match serde_json::from_str::<ZaloSession>(&content) {
            Ok(loaded) => {
                if loaded.uid.is_empty() || loaded.cookie.is_none() {
                    tracing::debug!("Zalo session file exists but empty/invalid, skipping");
                    return false;
                }
                let mut session = self.session.write().await;
                *session = loaded;
                tracing::info!(
                    "Zalo session restored from file: uid={}, active={}",
                    session.uid,
                    session.active
                );
                true
            }
            Err(e) => {
                tracing::warn!("Failed to deserialize Zalo session from {}: {}", path, e);
                false
            }
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
