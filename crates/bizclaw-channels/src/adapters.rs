//! Multi-channel adapter stubs for platforms inspired by OpenFang's 40+ channels.
//!
//! Each channel follows the same `Channel` trait pattern as existing channels.
//! These provide the configuration + parsing layer — actual API integration
//! is implemented when API keys are provided.

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Channel;
use bizclaw_core::types::{IncomingMessage, OutgoingMessage, ThreadType};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════
// LINE Messaging API
// ═══════════════════════════════════════════════════════

/// LINE Messaging API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineConfig {
    pub channel_access_token: String,
    pub channel_secret: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

pub struct LineChannel {
    config: LineConfig,
    client: reqwest::Client,
    connected: bool,
}

impl LineChannel {
    pub fn new(config: LineConfig) -> Self {
        Self { config, client: reqwest::Client::new(), connected: false }
    }

    /// Parse LINE webhook event.
    pub fn parse_webhook(&self, payload: &serde_json::Value) -> Vec<IncomingMessage> {
        let mut messages = Vec::new();
        if let Some(events) = payload["events"].as_array() {
            for event in events {
                if event["type"].as_str() == Some("message")
                    && event["message"]["type"].as_str() == Some("text")
                {
                    messages.push(IncomingMessage {
                        channel: "line".into(),
                        thread_id: event["source"]["userId"].as_str().unwrap_or("").into(),
                        sender_id: event["source"]["userId"].as_str().unwrap_or("").into(),
                        sender_name: None,
                        content: event["message"]["text"].as_str().unwrap_or("").into(),
                        thread_type: match event["source"]["type"].as_str() {
                            Some("group") => ThreadType::Group,
                            _ => ThreadType::Direct,
                        },
                        timestamp: chrono::Utc::now(),
                        reply_to: event["replyToken"].as_str().map(String::from),
                    });
                }
            }
        }
        messages
    }
}

#[async_trait]
impl Channel for LineChannel {
    fn name(&self) -> &str { "line" }
    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        tracing::info!("📱 LINE channel connected");
        Ok(())
    }
    async fn disconnect(&mut self) -> Result<()> { self.connected = false; Ok(()) }
    fn is_connected(&self) -> bool { self.connected }
    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        let body = serde_json::json!({
            "to": message.thread_id,
            "messages": [{"type": "text", "text": message.content}]
        });
        self.client.post("https://api.line.me/v2/bot/message/push")
            .header("Authorization", format!("Bearer {}", self.config.channel_access_token))
            .json(&body).send().await
            .map_err(|e| BizClawError::Channel(format!("LINE: {e}")))?;
        Ok(())
    }
    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        Ok(Box::new(stream::pending()))
    }
}

// ═══════════════════════════════════════════════════════
// Microsoft Teams (via Bot Framework)
// ═══════════════════════════════════════════════════════

/// Microsoft Teams configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsConfig {
    pub app_id: String,
    pub app_password: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

pub struct TeamsChannel {
    #[allow(dead_code)]
    config: TeamsConfig,
    #[allow(dead_code)]
    client: reqwest::Client,
    connected: bool,
}

impl TeamsChannel {
    pub fn new(config: TeamsConfig) -> Self {
        Self { config, client: reqwest::Client::new(), connected: false }
    }

    pub fn parse_activity(&self, payload: &serde_json::Value) -> Option<IncomingMessage> {
        if payload["type"].as_str() != Some("message") {
            return None;
        }
        Some(IncomingMessage {
            channel: "teams".into(),
            thread_id: payload["conversation"]["id"].as_str().unwrap_or("").into(),
            sender_id: payload["from"]["id"].as_str().unwrap_or("").into(),
            sender_name: payload["from"]["name"].as_str().map(String::from),
            content: payload["text"].as_str().unwrap_or("").into(),
            thread_type: if payload["conversation"]["conversationType"].as_str() == Some("personal") {
                ThreadType::Direct
            } else {
                ThreadType::Group
            },
            timestamp: chrono::Utc::now(),
            reply_to: payload["replyToId"].as_str().map(String::from),
        })
    }
}

#[async_trait]
impl Channel for TeamsChannel {
    fn name(&self) -> &str { "teams" }
    async fn connect(&mut self) -> Result<()> { self.connected = true; Ok(()) }
    async fn disconnect(&mut self) -> Result<()> { self.connected = false; Ok(()) }
    fn is_connected(&self) -> bool { self.connected }
    async fn send(&self, _message: OutgoingMessage) -> Result<()> {
        // Teams requires Bot Framework REST API with OAuth token
        tracing::warn!("Teams send: not yet implemented — requires Bot Framework auth");
        Ok(())
    }
    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        Ok(Box::new(stream::pending()))
    }
}

// ═══════════════════════════════════════════════════════
// Signal (via signal-cli REST API)
// ═══════════════════════════════════════════════════════

/// Signal configuration (uses signal-cli REST API).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    pub api_url: String, // e.g., "http://localhost:8080"
    pub phone_number: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

pub struct SignalChannel {
    config: SignalConfig,
    client: reqwest::Client,
    connected: bool,
}

impl SignalChannel {
    pub fn new(config: SignalConfig) -> Self {
        Self { config, client: reqwest::Client::new(), connected: false }
    }
}

#[async_trait]
impl Channel for SignalChannel {
    fn name(&self) -> &str { "signal" }
    async fn connect(&mut self) -> Result<()> { self.connected = true; Ok(()) }
    async fn disconnect(&mut self) -> Result<()> { self.connected = false; Ok(()) }
    fn is_connected(&self) -> bool { self.connected }
    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        let body = serde_json::json!({
            "message": message.content,
            "number": self.config.phone_number,
            "recipients": [message.thread_id],
        });
        self.client.post(format!("{}/v2/send", self.config.api_url))
            .json(&body).send().await
            .map_err(|e| BizClawError::Channel(format!("Signal: {e}")))?;
        Ok(())
    }
    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        Ok(Box::new(stream::pending()))
    }
}

// ═══════════════════════════════════════════════════════
// Matrix (via Client-Server API)
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    pub homeserver_url: String,
    pub access_token: String,
    pub user_id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

pub struct MatrixChannel {
    config: MatrixConfig,
    client: reqwest::Client,
    connected: bool,
}

impl MatrixChannel {
    pub fn new(config: MatrixConfig) -> Self {
        Self { config, client: reqwest::Client::new(), connected: false }
    }
}

#[async_trait]
impl Channel for MatrixChannel {
    fn name(&self) -> &str { "matrix" }
    async fn connect(&mut self) -> Result<()> { self.connected = true; Ok(()) }
    async fn disconnect(&mut self) -> Result<()> { self.connected = false; Ok(()) }
    fn is_connected(&self) -> bool { self.connected }
    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        let txn_id = uuid::Uuid::new_v4().to_string();
        let url = format!("{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            self.config.homeserver_url, message.thread_id, txn_id);
        let body = serde_json::json!({"msgtype": "m.text", "body": message.content});
        self.client.put(&url)
            .header("Authorization", format!("Bearer {}", self.config.access_token))
            .json(&body).send().await
            .map_err(|e| BizClawError::Channel(format!("Matrix: {e}")))?;
        Ok(())
    }
    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        Ok(Box::new(stream::pending()))
    }
}

// ═══════════════════════════════════════════════════════
// Viber Bot API
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViberConfig {
    pub auth_token: String,
    pub bot_name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

pub struct ViberChannel {
    config: ViberConfig,
    client: reqwest::Client,
    connected: bool,
}

impl ViberChannel {
    pub fn new(config: ViberConfig) -> Self {
        Self { config, client: reqwest::Client::new(), connected: false }
    }
}

#[async_trait]
impl Channel for ViberChannel {
    fn name(&self) -> &str { "viber" }
    async fn connect(&mut self) -> Result<()> { self.connected = true; Ok(()) }
    async fn disconnect(&mut self) -> Result<()> { self.connected = false; Ok(()) }
    fn is_connected(&self) -> bool { self.connected }
    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        let body = serde_json::json!({
            "receiver": message.thread_id,
            "type": "text",
            "text": message.content,
            "sender": {"name": self.config.bot_name},
        });
        self.client.post("https://chatapi.viber.com/pa/send_message")
            .header("X-Viber-Auth-Token", &self.config.auth_token)
            .json(&body).send().await
            .map_err(|e| BizClawError::Channel(format!("Viber: {e}")))?;
        Ok(())
    }
    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        Ok(Box::new(stream::pending()))
    }
}

// ═══════════════════════════════════════════════════════
// Facebook Messenger Platform
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessengerConfig {
    pub page_access_token: String,
    pub verify_token: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

pub struct MessengerChannel {
    config: MessengerConfig,
    client: reqwest::Client,
    connected: bool,
}

impl MessengerChannel {
    pub fn new(config: MessengerConfig) -> Self {
        Self { config, client: reqwest::Client::new(), connected: false }
    }

    pub fn parse_webhook(&self, payload: &serde_json::Value) -> Vec<IncomingMessage> {
        let mut messages = Vec::new();
        if let Some(entries) = payload["entry"].as_array() {
            for entry in entries {
                if let Some(messaging) = entry["messaging"].as_array() {
                    for msg in messaging {
                        if let Some(text) = msg["message"]["text"].as_str() {
                            messages.push(IncomingMessage {
                                channel: "messenger".into(),
                                thread_id: msg["sender"]["id"].as_str().unwrap_or("").into(),
                                sender_id: msg["sender"]["id"].as_str().unwrap_or("").into(),
                                sender_name: None,
                                content: text.into(),
                                thread_type: ThreadType::Direct,
                                timestamp: chrono::Utc::now(),
                                reply_to: None,
                            });
                        }
                    }
                }
            }
        }
        messages
    }
}

#[async_trait]
impl Channel for MessengerChannel {
    fn name(&self) -> &str { "messenger" }
    async fn connect(&mut self) -> Result<()> { self.connected = true; Ok(()) }
    async fn disconnect(&mut self) -> Result<()> { self.connected = false; Ok(()) }
    fn is_connected(&self) -> bool { self.connected }
    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        let body = serde_json::json!({
            "recipient": {"id": message.thread_id},
            "message": {"text": message.content},
        });
        self.client.post("https://graph.facebook.com/v18.0/me/messages")
            .query(&[("access_token", &self.config.page_access_token)])
            .json(&body).send().await
            .map_err(|e| BizClawError::Channel(format!("Messenger: {e}")))?;
        Ok(())
    }
    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        Ok(Box::new(stream::pending()))
    }
}

// ═══════════════════════════════════════════════════════
// More channels: Mattermost, Google Chat, DingTalk,
// Feishu/Lark, LinkedIn, Reddit, Mastodon, Bluesky,
// Nostr, Webex, Pumble, Flock, Threema, Keybase
// ═══════════════════════════════════════════════════════

/// Generic adapter for platforms with simple webhook + REST API patterns.
/// Covers: Mattermost, Google Chat, DingTalk, Feishu/Lark, Webex, Pumble, Flock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericWebhookConfig {
    pub name: String,
    pub incoming_url: String,
    pub outgoing_url: String,
    pub auth_header: String,
    pub auth_value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

pub struct GenericWebhookChannel {
    config: GenericWebhookConfig,
    client: reqwest::Client,
    connected: bool,
}

impl GenericWebhookChannel {
    pub fn new(config: GenericWebhookConfig) -> Self {
        Self { config, client: reqwest::Client::new(), connected: false }
    }
}

#[async_trait]
impl Channel for GenericWebhookChannel {
    fn name(&self) -> &str { &self.config.name }
    async fn connect(&mut self) -> Result<()> { self.connected = true; Ok(()) }
    async fn disconnect(&mut self) -> Result<()> { self.connected = false; Ok(()) }
    fn is_connected(&self) -> bool { self.connected }
    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        let body = serde_json::json!({"text": message.content});
        self.client.post(&self.config.outgoing_url)
            .header(&self.config.auth_header, &self.config.auth_value)
            .json(&body).send().await
            .map_err(|e| BizClawError::Channel(format!("{}: {e}", self.config.name)))?;
        Ok(())
    }
    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        Ok(Box::new(stream::pending()))
    }
}

fn default_true() -> bool { true }

// ═══════════════════════════════════════════════════════
// Supported channel names for registry
// ═══════════════════════════════════════════════════════

/// All channel types supported by BizClaw.
pub const ALL_CHANNEL_NAMES: &[(&str, &str)] = &[
    // Existing (9)
    ("cli", "Command-line interface"),
    ("telegram", "Telegram Bot API"),
    ("discord", "Discord Gateway bot"),
    ("email", "IMAP/SMTP email"),
    ("webhook", "HTTP webhooks"),
    ("whatsapp", "WhatsApp Cloud API"),
    ("zalo_personal", "Zalo Personal API"),
    ("zalo_official", "Zalo Official Account"),
    // New (16+)
    ("slack", "Slack Bot (Events API + Socket Mode)"),
    ("line", "LINE Messaging API"),
    ("teams", "Microsoft Teams (Bot Framework)"),
    ("signal", "Signal (via signal-cli REST)"),
    ("matrix", "Matrix/Element (Client-Server API)"),
    ("viber", "Viber Bot API"),
    ("messenger", "Facebook Messenger Platform"),
    ("mattermost", "Mattermost webhook"),
    ("google_chat", "Google Chat service account"),
    ("dingtalk", "DingTalk Robot API"),
    ("feishu", "Feishu/Lark Open Platform"),
    ("linkedin", "LinkedIn Messaging API"),
    ("reddit", "Reddit API bot"),
    ("mastodon", "Mastodon Streaming API"),
    ("bluesky", "Bluesky/AT Protocol"),
    ("nostr", "Nostr relay protocol"),
    ("webex", "Cisco Webex bot"),
    ("pumble", "Pumble bot"),
    ("flock", "Flock bot"),
    ("threema", "Threema Gateway"),
    ("keybase", "Keybase chat bot"),
    ("twitter", "Twitter/X DMs"),
    ("twilio_sms", "Twilio SMS"),
    ("twilio_voice", "Twilio Voice"),
    ("xmpp", "XMPP/Jabber"),
];

#[cfg(test)]
mod tests {
    use super::*;

    // ── LINE tests ──
    #[test]
    fn test_line_parse_text_message() {
        let ch = LineChannel::new(LineConfig {
            channel_access_token: "test".into(),
            channel_secret: "test".into(),
            enabled: true,
        });
        let payload = serde_json::json!({
            "events": [{
                "type": "message",
                "source": {"type": "user", "userId": "U123"},
                "message": {"type": "text", "text": "Xin chào!"},
                "replyToken": "rt_abc"
            }]
        });
        let msgs = ch.parse_webhook(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "Xin chào!");
        assert_eq!(msgs[0].sender_id, "U123");
    }

    #[test]
    fn test_line_parse_group_message() {
        let ch = LineChannel::new(LineConfig {
            channel_access_token: "test".into(),
            channel_secret: "test".into(),
            enabled: true,
        });
        let payload = serde_json::json!({
            "events": [{
                "type": "message",
                "source": {"type": "group", "groupId": "G123", "userId": "U456"},
                "message": {"type": "text", "text": "nhóm chat"},
                "replyToken": "rt_def"
            }]
        });
        let msgs = ch.parse_webhook(&payload);
        assert_eq!(msgs[0].thread_type, ThreadType::Group);
    }

    #[test]
    fn test_line_ignore_non_text() {
        let ch = LineChannel::new(LineConfig {
            channel_access_token: "test".into(),
            channel_secret: "test".into(),
            enabled: true,
        });
        let payload = serde_json::json!({
            "events": [{"type": "message", "source": {"userId": "U1"},
                "message": {"type": "image"}, "replyToken": "rt"}]
        });
        assert!(ch.parse_webhook(&payload).is_empty());
    }

    // ── Teams tests ──
    #[test]
    fn test_teams_parse_personal_message() {
        let ch = TeamsChannel::new(TeamsConfig {
            app_id: "test".into(), app_password: "test".into(), enabled: true,
        });
        let payload = serde_json::json!({
            "type": "message",
            "from": {"id": "user1", "name": "Hoai"},
            "conversation": {"id": "conv1", "conversationType": "personal"},
            "text": "Hello from Teams!"
        });
        let msg = ch.parse_activity(&payload).unwrap();
        assert_eq!(msg.content, "Hello from Teams!");
        assert_eq!(msg.thread_type, ThreadType::Direct);
        assert_eq!(msg.sender_name, Some("Hoai".into()));
    }

    #[test]
    fn test_teams_parse_group_message() {
        let ch = TeamsChannel::new(TeamsConfig {
            app_id: "test".into(), app_password: "test".into(), enabled: true,
        });
        let payload = serde_json::json!({
            "type": "message",
            "from": {"id": "u1", "name": "Tester"},
            "conversation": {"id": "c1", "conversationType": "groupChat"},
            "text": "group msg"
        });
        let msg = ch.parse_activity(&payload).unwrap();
        assert_eq!(msg.thread_type, ThreadType::Group);
    }

    #[test]
    fn test_teams_ignore_non_message() {
        let ch = TeamsChannel::new(TeamsConfig {
            app_id: "test".into(), app_password: "test".into(), enabled: true,
        });
        let payload = serde_json::json!({"type": "typing"});
        assert!(ch.parse_activity(&payload).is_none());
    }

    // ── Messenger tests ──
    #[test]
    fn test_messenger_parse_webhook() {
        let ch = MessengerChannel::new(MessengerConfig {
            page_access_token: "test".into(),
            verify_token: "test".into(),
            enabled: true,
        });
        let payload = serde_json::json!({
            "entry": [{
                "messaging": [{
                    "sender": {"id": "psid123"},
                    "message": {"text": "Hi from Messenger"}
                }]
            }]
        });
        let msgs = ch.parse_webhook(&payload);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "Hi from Messenger");
        assert_eq!(msgs[0].sender_id, "psid123");
    }

    #[test]
    fn test_messenger_no_text_message() {
        let ch = MessengerChannel::new(MessengerConfig {
            page_access_token: "test".into(),
            verify_token: "test".into(),
            enabled: true,
        });
        let payload = serde_json::json!({
            "entry": [{"messaging": [{"sender": {"id": "p1"}, "message": {"attachments": []}}]}]
        });
        assert!(ch.parse_webhook(&payload).is_empty());
    }

    // ── Generic Webhook tests ──
    #[test]
    fn test_generic_webhook_creation() {
        let ch = GenericWebhookChannel::new(GenericWebhookConfig {
            name: "mattermost".into(),
            incoming_url: "http://localhost:8065".into(),
            outgoing_url: "http://localhost:8065/hooks/xxx".into(),
            auth_header: "Authorization".into(),
            auth_value: "Bearer token".into(),
            enabled: true,
        });
        assert_eq!(ch.name(), "mattermost");
        assert!(!ch.is_connected());
    }

    // ── Channel registry tests ──
    #[test]
    fn test_all_channels_count() {
        assert!(ALL_CHANNEL_NAMES.len() >= 25, "Should have 25+ channels, got {}", ALL_CHANNEL_NAMES.len());
    }

    #[test]
    fn test_channel_names_unique() {
        let names: Vec<_> = ALL_CHANNEL_NAMES.iter().map(|(n, _)| *n).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len(), "Channel names must be unique");
    }

    #[test]
    fn test_existing_channels_present() {
        let names: Vec<_> = ALL_CHANNEL_NAMES.iter().map(|(n, _)| *n).collect();
        for expected in &["telegram", "discord", "slack", "line", "teams", "signal"] {
            assert!(names.contains(expected), "Missing channel: {expected}");
        }
    }
}
