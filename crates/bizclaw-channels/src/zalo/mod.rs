//! Zalo channel module — Zalo Personal + OA.
//! Wraps the client sub-modules into the Channel trait.
//! Now with real WebSocket listening, auto-reconnect, cookie health checks,
//! and Circuit Breaker protection for API calls.

pub mod client;
pub mod official;
pub mod personal;

use async_trait::async_trait;
use bizclaw_core::circuit_breaker::CircuitBreaker;
use bizclaw_core::config::ZaloChannelConfig;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Channel;
use bizclaw_core::types::{IncomingMessage, OutgoingMessage};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::Stream;

use self::client::auth::{ZaloAuth, ZaloCredentials};
use self::client::listener::ZaloListener;
use self::client::messaging::{ThreadType as ZaloThreadType, ZaloMessaging};
use self::client::session::SessionManager;

/// Zalo channel implementation — routes to Personal or OA mode.
/// Now with real WebSocket listening and auto-reconnect.
pub struct ZaloChannel {
    config: ZaloChannelConfig,
    auth: ZaloAuth,
    messaging: ZaloMessaging,
    session: SessionManager,
    connected: bool,
    cookie: Option<String>,
    /// WebSocket URLs from login response
    ws_urls: Vec<String>,
    /// Own user ID (for self-message filtering)
    own_uid: String,
    /// Listener reference (kept alive for WebSocket connection)
    listener: Option<Arc<ZaloListener>>,
    /// Shared message receiver — wrapped in Mutex for safe access
    msg_receiver: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<IncomingMessage>>>>,
    /// Circuit breaker — prevents cascading failures when Zalo API is down.
    circuit_breaker: CircuitBreaker,
    /// Cipher key (zpw_enk) for WebSocket binary frame decryption
    cipher_key: Option<String>,
}

impl ZaloChannel {
    pub fn new(config: ZaloChannelConfig) -> Self {
        let creds = ZaloCredentials {
            imei: config.personal.imei.clone(),
            cookie: None,
            phone: None,
            user_agent: if config.personal.user_agent.is_empty() {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:135.0) Gecko/20100101 Firefox/135.0"
                    .into()
            } else {
                config.personal.user_agent.clone()
            },
            proxy: if config.personal.proxy.is_empty() {
                None
            } else {
                Some(config.personal.proxy.clone())
            },
        };

        let proxy_opt = if config.personal.proxy.is_empty() {
            None
        } else {
            Some(config.personal.proxy.clone())
        };

        // Derive session persist path from cookie_path (e.g. ~/.bizclaw/zalo_cookie → ~/.bizclaw/zalo_session.json)
        let session_path = if !config.personal.cookie_path.is_empty() {
            let p = std::path::Path::new(&config.personal.cookie_path);
            let parent = p.parent().unwrap_or(std::path::Path::new("."));
            let session_file = parent.join("zalo_session.json");
            session_file.to_string_lossy().to_string()
        } else {
            String::new()
        };

        let session = if session_path.is_empty() {
            SessionManager::new()
        } else {
            SessionManager::with_persist_path(&session_path)
        };

        Self {
            auth: ZaloAuth::new(creds),
            messaging: ZaloMessaging::with_proxy(proxy_opt),
            config,
            session,
            connected: false,
            cookie: None,
            ws_urls: Vec::new(),
            own_uid: String::new(),
            listener: None,
            msg_receiver: Arc::new(Mutex::new(None)),
            circuit_breaker: CircuitBreaker::named("zalo", 5, std::time::Duration::from_secs(30)),
            cipher_key: None,
        }
    }

    /// Get circuit breaker reference for monitoring.
    pub fn circuit_breaker(&self) -> &CircuitBreaker {
        &self.circuit_breaker
    }

    /// Login with cookie from config or parameter.
    async fn login_cookie(&mut self, cookie: &str) -> Result<()> {
        let login_data = self.auth.login_with_cookie(cookie).await?;

        // Apply service map to messaging client (critical for correct API URLs)
        if let Some(ref map) = login_data.zpw_service_map_v3 {
            let service_map = client::messaging::ZaloServiceMap::from_login_data(map);
            self.messaging.set_service_map(service_map);
            tracing::info!("Zalo: service map applied from login response");
        }

        // Set login credentials on messaging client
        self.messaging
            .set_login_info(&login_data.uid, login_data.zpw_enk.as_deref());

        // Store WebSocket URLs for listener
        if let Some(ref ws_urls) = login_data.zpw_ws {
            self.ws_urls = ws_urls.clone();
            tracing::info!("Zalo: got {} WebSocket URLs", self.ws_urls.len());
        }

        // Extract secret_key (zpw_sek) from cookie for API param encryption
        let secret_key = extract_cookie_value(cookie, "zpw_sek");
        // Wire secret_key into messaging client for send_text encryption
        if let Some(ref sk) = secret_key {
            self.messaging.set_secret_key(sk);
        }

        // Store own UID
        self.own_uid = login_data.uid.clone();

        self.session
            .set_session(
                login_data.uid.clone(),
                login_data.zpw_enk,
                login_data.zpw_key,
                secret_key,
                Some(cookie.to_string()),
                Some(self.config.personal.imei.clone()),
            )
            .await;
        self.cookie = Some(cookie.to_string());
        // Store cipher_key for WebSocket listener (needs to be sync-accessible)
        self.cipher_key = self.session.get_session().await.zpw_enk.clone();
        tracing::info!("Zalo logged in: uid={}", login_data.uid);
        Ok(())
    }

    /// Get QR code for login.
    pub async fn get_qr_code(&mut self) -> Result<client::auth::QrCodeResult> {
        self.auth.get_qr_code().await
    }

    /// Start WebSocket listener after login.
    fn start_ws_listener(&mut self) -> Result<()> {
        let ws_url = self
            .ws_urls
            .first()
            .ok_or_else(|| {
                BizClawError::Channel(
                    "No WebSocket URL from login. Zalo may have changed API.".into(),
                )
            })?
            .clone();

        let cookie = self.cookie.clone().unwrap_or_default();

        let webhook_url = if self.config.personal.webhook_url.is_empty() {
            None
        } else {
            Some(self.config.personal.webhook_url.clone())
        };

        let listener = ZaloListener::new(&ws_url)
            .with_reconnect(
                self.config.personal.auto_reconnect,
                self.config.personal.reconnect_delay_ms,
                0, // unlimited reconnect attempts
            )
            .with_own_uid(&self.own_uid)
            .with_self_listen(self.config.personal.self_listen)
            .with_webhook(webhook_url)
            .with_cipher_key(self.cipher_key.clone());

        let rx = listener.start_listening(cookie);

        self.listener = Some(Arc::new(listener));
        *self.msg_receiver.blocking_lock() = Some(rx);

        tracing::info!(
            "Zalo: WebSocket listener started (ws_url={}, auto_reconnect={})",
            ws_url,
            self.config.personal.auto_reconnect
        );

        Ok(())
    }

    /// Start a background health watchdog that monitors session state.
    /// Detects stale sessions (no heartbeat), logs alerts for re-authentication.
    fn start_health_watchdog(&self) {
        let session = self.session.get_arc();
        let uid = self.own_uid.clone();

        tokio::spawn(async move {
            let check_interval = std::time::Duration::from_secs(60);
            let stale_threshold = 300; // 5 minutes without heartbeat = stale

            loop {
                tokio::time::sleep(check_interval).await;

                let s = session.read().await;
                if !s.active {
                    tracing::info!("Zalo watchdog: session inactive for uid={}, stopping", uid);
                    break;
                }

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let age = now.saturating_sub(s.last_heartbeat);

                if age > stale_threshold {
                    tracing::warn!(
                        "⚠️ Zalo watchdog: session STALE for uid={} ({}s since last heartbeat). Re-auth may be needed.",
                        s.uid,
                        age
                    );
                } else {
                    tracing::trace!(
                        "Zalo watchdog: session healthy for uid={} ({}s)",
                        s.uid,
                        age
                    );
                }
            }
        });

        tracing::info!("Zalo: health watchdog started for uid={}", self.own_uid);
    }
}

#[async_trait]
impl Channel for ZaloChannel {
    fn name(&self) -> &str {
        "zalo"
    }

    async fn connect(&mut self) -> Result<()> {
        tracing::info!("Zalo channel: connecting in {} mode...", self.config.mode);

        match self.config.mode.as_str() {
            "personal" => {
                tracing::warn!("⚠️  Zalo Personal API is unofficial. Use at your own risk.");

                // Step 0: Try restoring persisted session first (skip fresh login if valid)
                if self.session.load_from_file().await {
                    let restored = self.session.get_session().await;
                    if restored.active && restored.cookie.is_some() {
                        tracing::info!("Zalo: restored persisted session for uid={}", restored.uid);
                        self.own_uid = restored.uid.clone();
                        self.cookie = restored.cookie.clone();
                        self.cipher_key = restored.zpw_enk.clone();
                        self.messaging
                            .set_login_info(&restored.uid, restored.zpw_enk.as_deref());

                        // Re-login to refresh service map + ws URLs
                        if let Some(ref cookie) = restored.cookie {
                            match self.auth.login_with_cookie(cookie).await {
                                Ok(login_data) => {
                                    if let Some(ref map) = login_data.zpw_service_map_v3 {
                                        let service_map =
                                            client::messaging::ZaloServiceMap::from_login_data(map);
                                        self.messaging.set_service_map(service_map);
                                    }
                                    if let Some(ref ws) = login_data.zpw_ws {
                                        self.ws_urls = ws.clone();
                                    }
                                    self.connected = true;
                                    tracing::info!("Zalo: session refreshed successfully");
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Zalo: persisted session expired, doing fresh login: {e}"
                                    );
                                    self.session.invalidate().await;
                                    // Fall through to fresh login below
                                }
                            }
                        }
                    }
                }

                // Step 1: Fresh login if not already connected
                if !self.connected {
                    let cookie = self.try_load_cookie()?;
                    if let Some(cookie) = cookie {
                        self.login_cookie(&cookie).await?;
                        self.connected = true;
                        tracing::info!("Zalo Personal: connected via cookie auth");
                    } else {
                        return Err(BizClawError::AuthFailed(
                            "No Zalo cookie found. Configure cookie_path in config.toml or use QR login via admin dashboard.".into()
                        ));
                    }
                }

                // Step 2: Start WebSocket listener
                if !self.ws_urls.is_empty() {
                    if let Err(e) = self.start_ws_listener() {
                        tracing::warn!(
                            "Zalo: WebSocket listener failed to start: {e}. \
                             Messages will only be received via webhook/polling."
                        );
                    }
                } else {
                    tracing::warn!(
                        "Zalo: No WebSocket URLs from login. \
                         Real-time message receiving unavailable."
                    );
                }

                // Step 3: Start health watchdog
                self.start_health_watchdog();
            }
            "official" => {
                tracing::info!("Zalo OA: connecting via official API...");
                self.connected = true;
                tracing::info!("Zalo OA: connected (official API requires Zalo OA token)");
            }
            _ => {
                return Err(BizClawError::Config(format!(
                    "Unknown Zalo mode: {}",
                    self.config.mode
                )));
            }
        }
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.session.invalidate().await;
        self.connected = false;
        self.listener = None; // Drop listener stops WebSocket
        tracing::info!("Zalo channel: disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        // Take the receiver from the shared slot
        let mut guard = self.msg_receiver.lock().await;
        if let Some(rx) = guard.take() {
            tracing::info!(
                "Zalo listener: active (WebSocket real-time mode, uid={})",
                self.own_uid
            );
            Ok(Box::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
        } else {
            // Fallback: if no WebSocket available (e.g., OA mode), use pending stream
            tracing::info!(
                "Zalo listener: no WebSocket receiver available, using webhook/polling fallback"
            );
            Ok(Box::new(futures::stream::pending::<IncomingMessage>()))
        }
    }

    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        if !self.circuit_breaker.can_execute() {
            return Err(BizClawError::Channel(
                "Zalo circuit breaker Open — message rejected".into(),
            ));
        }

        let cookie = self
            .cookie
            .as_ref()
            .ok_or_else(|| BizClawError::Channel("Zalo not logged in".into()))?;

        match self
            .messaging
            .send_text(
                &message.thread_id,
                ZaloThreadType::User,
                &message.content,
                cookie,
            )
            .await
        {
            Ok(_msg_id) => {
                self.circuit_breaker.record_success();
                tracing::debug!("Zalo: message sent to {}", message.thread_id);
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                tracing::error!(
                    "Zalo send failed: {e} (CB: {})",
                    self.circuit_breaker.summary()
                );
                Err(e)
            }
        }
    }

    async fn send_typing(&self, thread_id: &str) -> Result<()> {
        tracing::debug!(
            "Zalo: typing indicator to {} (not supported by API)",
            thread_id
        );
        Ok(())
    }
}

impl ZaloChannel {
    /// Try to load cookie from cookie_path file.
    fn try_load_cookie(&self) -> Result<Option<String>> {
        let path = &self.config.personal.cookie_path;
        if path.is_empty() {
            return Ok(None);
        }

        // Expand ~ to home dir
        let expanded = if path.starts_with("~/") {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(&path[2..]))
                .unwrap_or_else(|| std::path::PathBuf::from(path))
        } else {
            std::path::PathBuf::from(path)
        };

        if expanded.exists() {
            let content = std::fs::read_to_string(&expanded)
                .map_err(|e| BizClawError::Config(format!("Failed to read cookie file: {e}")))?;

            let trimmed = content.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            // Support JSON format {"cookie": "..."} or raw cookie string
            if trimmed.starts_with('{')
                && let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed)
                && let Some(cookie) = json["cookie"].as_str()
            {
                return Ok(Some(cookie.to_string()));
            }

            Ok(Some(trimmed.to_string()))
        } else {
            Ok(None)
        }
    }

    /// Check cookie health — returns true if cookie is still valid.
    pub async fn check_cookie_health(&self) -> bool {
        if let Some(ref cookie) = self.cookie {
            match self.auth.login_with_cookie(cookie).await {
                Ok(_) => true,
                Err(e) => {
                    tracing::warn!("Zalo cookie health check failed: {e}");
                    false
                }
            }
        } else {
            false
        }
    }

    /// Get current connection info for debugging/dashboard.
    pub fn connection_info(&self) -> serde_json::Value {
        serde_json::json!({
            "connected": self.connected,
            "mode": self.config.mode,
            "uid": self.own_uid,
            "ws_urls": self.ws_urls,
            "has_cookie": self.cookie.is_some(),
            "has_listener": self.listener.is_some(),
            "auto_reconnect": self.config.personal.auto_reconnect,
            "service_info": self.messaging.service_info(),
        })
    }
}

/// Extract a value from a cookie string by key name.
fn extract_cookie_value(cookie_str: &str, key: &str) -> Option<String> {
    cookie_str.split(';').find_map(|pair| {
        let mut kv = pair.trim().splitn(2, '=');
        if kv.next() == Some(key) {
            kv.next().map(String::from)
        } else {
            None
        }
    })
}
