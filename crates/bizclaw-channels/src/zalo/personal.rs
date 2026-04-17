//! Zalo Personal mode — wraps the low-level client modules.
//!
//! Provides a high-level Channel interface using reverse-engineered
//! Zalo Web protocol (auth, messaging, WebSocket listener).

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Channel;
use bizclaw_core::types::{IncomingMessage, OutgoingMessage};
use futures::stream::{self, Stream};

use super::client::{
    auth::{ZaloAuth, ZaloCredentials},
    messaging::{ThreadType as ZaloThreadType, ZaloMessaging},
    session::SessionManager,
};

/// Zalo Personal channel — uses cookie/QR login.
pub struct ZaloPersonalChannel {
    auth: ZaloAuth,
    messaging: ZaloMessaging,
    session: SessionManager,
    connected: bool,
    cookie: Option<String>,
}

impl ZaloPersonalChannel {
    pub fn new(imei: &str, user_agent: &str) -> Self {
        let creds = ZaloCredentials {
            imei: imei.to_string(),
            cookie: None,
            phone: None,
            user_agent: user_agent.to_string(),
            proxy: None,
        };
        Self {
            auth: ZaloAuth::new(creds),
            messaging: ZaloMessaging::new(),
            session: SessionManager::new(),
            connected: false,
            cookie: None,
        }
    }

    /// Login with cookie.
    pub async fn login_cookie(&mut self, cookie: &str) -> Result<()> {
        let login_data = self.auth.login_with_cookie(cookie).await?;
        self.session
            .set_session(
                login_data.uid.clone(),
                login_data.zpw_enk,
                login_data.zpw_key,
                None, // secret_key extracted separately if needed
                Some(cookie.to_string()),
                None, // imei
            )
            .await;
        self.cookie = Some(cookie.to_string());
        self.connected = true;
        tracing::info!("Zalo Personal logged in: uid={}", login_data.uid);
        Ok(())
    }
}

#[async_trait]
impl Channel for ZaloPersonalChannel {
    fn name(&self) -> &str {
        "zalo-personal"
    }

    async fn connect(&mut self) -> Result<()> {
        if self.cookie.is_none() {
            return Err(BizClawError::AuthFailed("Call login_cookie() first".into()));
        }
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.session.invalidate().await;
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        let cookie = self
            .cookie
            .as_ref()
            .ok_or_else(|| BizClawError::Channel("Not logged in".into()))?;
        self.messaging
            .send_text(
                &message.thread_id,
                ZaloThreadType::User,
                &message.content,
                cookie,
            )
            .await?;
        Ok(())
    }

    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        Ok(Box::new(stream::pending()))
    }
}
