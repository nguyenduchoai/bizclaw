//! Zalo authentication - Cookie login, QR login, multi-account.
//! Reverse-engineered from zca-js (TypeScript).

use serde::{Deserialize, Serialize};

/// Zalo login credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloCredentials {
    pub cookie: serde_json::Value,
    pub imei: String,
    pub user_agent: String,
}

/// Zalo session after login.
#[derive(Debug, Clone)]
pub struct ZaloSession {
    pub user_id: String,
    pub access_token: String,
    pub cookies: Vec<(String, String)>,
    pub is_active: bool,
}

impl ZaloSession {
    /// Create a new empty session.
    pub fn new() -> Self {
        Self {
            user_id: String::new(),
            access_token: String::new(),
            cookies: vec![],
            is_active: false,
        }
    }
}

impl Default for ZaloSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Login with cookie credentials.
/// Phase 4A: Full implementation with Zalo Web protocol.
pub async fn login_cookie(_creds: &ZaloCredentials) -> Result<ZaloSession, String> {
    tracing::info!("Zalo auth: cookie login (stub)");
    Ok(ZaloSession::new())
}

/// Login with QR code.
pub async fn login_qr() -> Result<(String, ZaloSession), String> {
    tracing::info!("Zalo auth: QR login (stub)");
    Ok(("qr-code-data-stub".into(), ZaloSession::new()))
}
