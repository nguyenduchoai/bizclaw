//! Zalo Bot Platform channel implementation.
//!
//! Uses the official Zalo Bot API at `https://bot-api.zaloplatforms.com/bot<TOKEN>/...`
//! Supports:
//! - Polling mode (getUpdates — for local/dev)
//! - Webhook mode (setWebhook — for production)
//! - sendMessage, sendPhoto, sendSticker, sendChatAction
//!
//! API reference: https://github.com/thuanhuynhh/zalo-bot-skills

use async_trait::async_trait;
use bizclaw_core::circuit_breaker::CircuitBreaker;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Channel;
use bizclaw_core::types::{IncomingMessage, OutgoingMessage, ThreadType};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};

const ZALO_BOT_API: &str = "https://bot-api.zaloplatforms.com/bot";

// ═══════════════════════════════════════════════════════
// Config
// ═══════════════════════════════════════════════════════

/// Zalo Bot Platform configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloBotConfig {
    /// Bot token from Zalo Bot Creator Mini App.
    pub bot_token: String,
    /// Secret token for webhook validation (8-256 chars).
    #[serde(default)]
    pub secret_token: String,
    /// Webhook URL (if using webhook mode).
    #[serde(default)]
    pub webhook_url: String,
    /// Polling timeout in seconds (default 30).
    #[serde(default = "default_poll_timeout")]
    pub poll_timeout: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for ZaloBotConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            secret_token: String::new(),
            webhook_url: String::new(),
            poll_timeout: 30,
            enabled: true,
        }
    }
}

fn default_poll_timeout() -> u64 {
    30
}
fn default_true() -> bool {
    true
}

// ═══════════════════════════════════════════════════════
// API Models
// ═══════════════════════════════════════════════════════

/// Bot info returned by getMe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotInfo {
    pub id: String,
    pub account_name: String,
    #[serde(default)]
    pub account_type: String,
    #[serde(default)]
    pub can_join_groups: bool,
}

/// Zalo Bot API response wrapper.
#[derive(Debug, Deserialize)]
pub struct ZaloBotResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
    pub error_code: Option<i32>,
}

/// User info from webhook/polling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloBotUser {
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub is_bot: bool,
}

/// Chat info from webhook/polling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloBotChat {
    pub id: String,
    #[serde(default)]
    pub chat_type: String, // PRIVATE or GROUP
}

/// Message from webhook/polling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloBotMessage {
    pub from: ZaloBotUser,
    pub chat: ZaloBotChat,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub photo: Option<String>,
    #[serde(default)]
    pub sticker: Option<String>,
    #[serde(default)]
    pub message_id: String,
    #[serde(default)]
    pub date: u64,
}

/// Update from getUpdates or webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloBotUpdate {
    pub message: ZaloBotMessage,
    #[serde(default)]
    pub event_name: String,
}

/// Webhook payload wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloBotWebhookPayload {
    pub ok: bool,
    pub result: ZaloBotUpdate,
}

// ═══════════════════════════════════════════════════════
// Channel Implementation
// ═══════════════════════════════════════════════════════

pub struct ZaloBotChannel {
    config: ZaloBotConfig,
    client: reqwest::Client,
    connected: bool,
    bot_info: Option<BotInfo>,
    circuit_breaker: CircuitBreaker,
}

impl ZaloBotChannel {
    pub fn new(config: ZaloBotConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
            connected: false,
            bot_info: None,
            circuit_breaker: CircuitBreaker::named(
                "zalo_bot",
                5,
                std::time::Duration::from_secs(30),
            ),
        }
    }

    /// Build API URL for a method.
    fn api_url(&self, method: &str) -> String {
        format!("{}{}/{}", ZALO_BOT_API, self.config.bot_token, method)
    }

    /// Call getMe to verify bot token.
    pub async fn get_me(&self) -> Result<BotInfo> {
        let url = self.api_url("getMe");
        let resp: ZaloBotResponse<BotInfo> = self
            .client
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Zalo Bot getMe: {e}")))?
            .json()
            .await
            .map_err(|e| BizClawError::Channel(format!("Zalo Bot getMe parse: {e}")))?;

        if resp.ok {
            resp.result
                .ok_or_else(|| BizClawError::Channel("Zalo Bot getMe: no result".into()))
        } else {
            Err(BizClawError::Channel(format!(
                "Zalo Bot getMe failed: {}",
                resp.description.unwrap_or_default()
            )))
        }
    }

    /// Get updates via long polling (dev mode).
    pub async fn get_updates(&self) -> Result<Vec<ZaloBotUpdate>> {
        let url = self.api_url("getUpdates");
        let resp: ZaloBotResponse<Vec<ZaloBotUpdate>> = self
            .client
            .post(&url)
            .json(&serde_json::json!({"timeout": self.config.poll_timeout.to_string()}))
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Zalo Bot getUpdates: {e}")))?
            .json()
            .await
            .map_err(|e| BizClawError::Channel(format!("Zalo Bot getUpdates parse: {e}")))?;

        if resp.ok {
            Ok(resp.result.unwrap_or_default())
        } else {
            Err(BizClawError::Channel(format!(
                "Zalo Bot getUpdates: {}",
                resp.description.unwrap_or_default()
            )))
        }
    }

    /// Set webhook URL for production mode.
    pub async fn set_webhook(&self, url: &str, secret_token: &str) -> Result<()> {
        let api_url = self.api_url("setWebhook");
        let resp: ZaloBotResponse<serde_json::Value> = self
            .client
            .post(&api_url)
            .json(&serde_json::json!({
                "url": url,
                "secret_token": secret_token,
            }))
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Zalo Bot setWebhook: {e}")))?
            .json()
            .await
            .map_err(|e| BizClawError::Channel(format!("Zalo Bot setWebhook parse: {e}")))?;

        if resp.ok {
            tracing::info!("[zalo-bot] Webhook set: {}", url);
            Ok(())
        } else {
            Err(BizClawError::Channel(format!(
                "Zalo Bot setWebhook: {}",
                resp.description.unwrap_or_default()
            )))
        }
    }

    /// Delete webhook (switch back to polling).
    pub async fn delete_webhook(&self) -> Result<()> {
        let url = self.api_url("deleteWebhook");
        let _resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Zalo Bot deleteWebhook: {e}")))?;
        tracing::info!("[zalo-bot] Webhook deleted");
        Ok(())
    }

    /// Send a text message.
    pub async fn send_message(&self, chat_id: &str, text: &str) -> Result<()> {
        let url = self.api_url("sendMessage");
        let resp: ZaloBotResponse<serde_json::Value> = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": text,
            }))
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Zalo Bot sendMessage: {e}")))?
            .json()
            .await
            .map_err(|e| BizClawError::Channel(format!("Zalo Bot sendMessage parse: {e}")))?;

        if resp.ok {
            Ok(())
        } else {
            Err(BizClawError::Channel(format!(
                "Zalo Bot sendMessage: {}",
                resp.description.unwrap_or_default()
            )))
        }
    }

    /// Send a photo message.
    pub async fn send_photo(
        &self,
        chat_id: &str,
        photo_url: &str,
        caption: Option<&str>,
    ) -> Result<()> {
        let url = self.api_url("sendPhoto");
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "photo": photo_url,
        });
        if let Some(cap) = caption {
            body["caption"] = serde_json::Value::String(cap.to_string());
        }
        let _resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Zalo Bot sendPhoto: {e}")))?;
        Ok(())
    }

    /// Send typing indicator.
    pub async fn send_typing(&self, chat_id: &str) -> Result<()> {
        let url = self.api_url("sendChatAction");
        let _resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "action": "typing",
            }))
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Zalo Bot sendChatAction: {e}")))?;
        Ok(())
    }

    /// Parse incoming webhook payload into IncomingMessage.
    pub fn parse_webhook_update(update: &ZaloBotUpdate) -> Option<IncomingMessage> {
        let msg = &update.message;
        // Only handle text messages for now
        let text = msg.text.as_deref().unwrap_or("");
        if text.is_empty() {
            return None;
        }

        let thread_type = match msg.chat.chat_type.as_str() {
            "GROUP" => ThreadType::Group,
            _ => ThreadType::Direct,
        };

        Some(IncomingMessage {
            channel: "zalo_bot".into(),
            thread_id: msg.chat.id.clone(),
            sender_id: msg.from.id.clone(),
            sender_name: if msg.from.display_name.is_empty() {
                None
            } else {
                Some(msg.from.display_name.clone())
            },
            content: text.to_string(),
            thread_type,
            timestamp: chrono::Utc::now(),
            reply_to: None,
        })
    }

    /// Get bot info reference.
    pub fn bot_info(&self) -> Option<&BotInfo> {
        self.bot_info.as_ref()
    }

    /// Get circuit breaker reference.
    pub fn circuit_breaker(&self) -> &CircuitBreaker {
        &self.circuit_breaker
    }
}

#[async_trait]
impl Channel for ZaloBotChannel {
    fn name(&self) -> &str {
        "zalo_bot"
    }

    async fn connect(&mut self) -> Result<()> {
        tracing::info!("[zalo-bot] Connecting...");
        // Verify token via getMe
        match self.get_me().await {
            Ok(info) => {
                tracing::info!(
                    "[zalo-bot] Connected as {} (id={}, type={})",
                    info.account_name,
                    info.id,
                    info.account_type
                );
                self.bot_info = Some(info);
                self.connected = true;

                // Set webhook if configured
                if !self.config.webhook_url.is_empty() {
                    if let Err(e) = self
                        .set_webhook(&self.config.webhook_url, &self.config.secret_token)
                        .await
                    {
                        tracing::warn!("[zalo-bot] Failed to set webhook: {e}");
                    }
                }
                Ok(())
            }
            Err(e) => {
                tracing::error!("[zalo-bot] Connection failed: {e}");
                Err(e)
            }
        }
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        self.bot_info = None;
        tracing::info!("[zalo-bot] Disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        if !self.circuit_breaker.can_execute() {
            return Err(BizClawError::Channel(
                "Zalo Bot circuit breaker Open — message rejected".into(),
            ));
        }

        match self
            .send_message(&message.thread_id, &message.content)
            .await
        {
            Ok(()) => {
                self.circuit_breaker.record_success();
                tracing::debug!("[zalo-bot] Message sent to {}", message.thread_id);
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                tracing::error!(
                    "[zalo-bot] Send failed: {e} (CB: {})",
                    self.circuit_breaker.summary()
                );
                Err(e)
            }
        }
    }

    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        // Webhook mode — messages come via POST /api/v1/webhook/zalo-bot
        // Return pending stream; actual messages are dispatched by the webhook handler
        Ok(Box::new(futures::stream::pending::<IncomingMessage>()))
    }

    async fn send_typing(&self, thread_id: &str) -> Result<()> {
        self.send_typing(thread_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_url() {
        let ch = ZaloBotChannel::new(ZaloBotConfig {
            bot_token: "123456:abc".into(),
            ..Default::default()
        });
        assert_eq!(
            ch.api_url("getMe"),
            "https://bot-api.zaloplatforms.com/bot123456:abc/getMe"
        );
    }

    #[test]
    fn test_parse_text_message() {
        let update = ZaloBotUpdate {
            message: ZaloBotMessage {
                from: ZaloBotUser {
                    id: "user123".into(),
                    display_name: "Ted".into(),
                    is_bot: false,
                },
                chat: ZaloBotChat {
                    id: "user123".into(),
                    chat_type: "PRIVATE".into(),
                },
                text: Some("Hello".into()),
                photo: None,
                sticker: None,
                message_id: "msg_123".into(),
                date: 1750316131602,
            },
            event_name: "message.text.received".into(),
        };
        let msg = ZaloBotChannel::parse_webhook_update(&update).unwrap();
        assert_eq!(msg.channel, "zalo_bot");
        assert_eq!(msg.content, "Hello");
        assert_eq!(msg.sender_id, "user123");
        assert_eq!(msg.thread_type, ThreadType::Direct);
        assert_eq!(msg.sender_name, Some("Ted".into()));
    }

    #[test]
    fn test_parse_group_message() {
        let update = ZaloBotUpdate {
            message: ZaloBotMessage {
                from: ZaloBotUser {
                    id: "user456".into(),
                    display_name: "Alice".into(),
                    is_bot: false,
                },
                chat: ZaloBotChat {
                    id: "group789".into(),
                    chat_type: "GROUP".into(),
                },
                text: Some("Hi group!".into()),
                photo: None,
                sticker: None,
                message_id: "msg_456".into(),
                date: 0,
            },
            event_name: "message.text.received".into(),
        };
        let msg = ZaloBotChannel::parse_webhook_update(&update).unwrap();
        assert_eq!(msg.thread_type, ThreadType::Group);
        assert_eq!(msg.thread_id, "group789");
    }

    #[test]
    fn test_ignore_empty_text() {
        let update = ZaloBotUpdate {
            message: ZaloBotMessage {
                from: ZaloBotUser {
                    id: "u".into(),
                    display_name: "".into(),
                    is_bot: false,
                },
                chat: ZaloBotChat {
                    id: "c".into(),
                    chat_type: "PRIVATE".into(),
                },
                text: None,
                photo: Some("http://img.jpg".into()),
                sticker: None,
                message_id: "m".into(),
                date: 0,
            },
            event_name: "message.image.received".into(),
        };
        assert!(ZaloBotChannel::parse_webhook_update(&update).is_none());
    }

    #[test]
    fn test_config_default() {
        let cfg = ZaloBotConfig::default();
        assert_eq!(cfg.poll_timeout, 30);
        assert!(cfg.enabled);
        assert!(cfg.bot_token.is_empty());
    }
}
