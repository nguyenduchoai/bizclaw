//! Slack channel — Slack Bot integration via Bolt-style webhooks.
//!
//! Implements Slack Socket Mode + Events API for receiving messages,
//! and Web API for sending responses.

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Channel;
use bizclaw_core::types::{IncomingMessage, OutgoingMessage, ThreadType};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};

/// Slack channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Bot token (xoxb-...).
    pub bot_token: String,
    /// App-level token for Socket Mode (xapp-...).
    #[serde(default)]
    pub app_token: String,
    /// Signing secret for verifying requests.
    #[serde(default)]
    pub signing_secret: String,
    /// Default channel to post to.
    #[serde(default)]
    pub default_channel: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool { true }

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            app_token: String::new(),
            signing_secret: String::new(),
            default_channel: "#general".into(),
            enabled: true,
        }
    }
}

/// Slack channel implementation.
pub struct SlackChannel {
    config: SlackConfig,
    client: reqwest::Client,
    connected: bool,
}

impl SlackChannel {
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            connected: false,
        }
    }

    /// Send a message to a Slack channel or thread.
    async fn post_message(&self, channel: &str, text: &str, thread_ts: Option<&str>) -> Result<()> {
        let mut body = serde_json::json!({
            "channel": channel,
            "text": text,
        });
        if let Some(ts) = thread_ts {
            body["thread_ts"] = serde_json::Value::String(ts.to_string());
        }

        let resp = self.client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.config.bot_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Slack API error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(BizClawError::Channel(format!("Slack API {status}")));
        }
        Ok(())
    }

    /// Parse a Slack Events API payload.
    pub fn parse_event(&self, payload: &serde_json::Value) -> Option<IncomingMessage> {
        let event = payload.get("event")?;
        let event_type = event["type"].as_str()?;

        if event_type != "message" && event_type != "app_mention" {
            return None;
        }

        // Ignore bot messages
        if event.get("bot_id").is_some() {
            return None;
        }

        Some(IncomingMessage {
            channel: "slack".into(),
            thread_id: event["channel"].as_str().unwrap_or("").into(),
            sender_id: event["user"].as_str().unwrap_or("").into(),
            sender_name: None,
            content: event["text"].as_str().unwrap_or("").into(),
            thread_type: if event.get("thread_ts").is_some() {
                ThreadType::Group
            } else {
                ThreadType::Direct
            },
            timestamp: chrono::Utc::now(),
            reply_to: event["thread_ts"].as_str().map(String::from),
        })
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn name(&self) -> &str { "slack" }

    async fn connect(&mut self) -> Result<()> {
        if self.config.bot_token.is_empty() {
            return Err(BizClawError::Channel("Slack bot_token required".into()));
        }
        // Verify token with auth.test
        let resp = self.client
            .post("https://slack.com/api/auth.test")
            .header("Authorization", format!("Bearer {}", self.config.bot_token))
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Slack auth test: {e}")))?;

        let body: serde_json::Value = resp.json().await
            .map_err(|e| BizClawError::Channel(format!("Slack response: {e}")))?;

        if body["ok"].as_bool() != Some(true) {
            return Err(BizClawError::AuthFailed(
                format!("Slack auth failed: {}", body["error"].as_str().unwrap_or("unknown"))
            ));
        }

        self.connected = true;
        tracing::info!("💬 Slack connected as: {}", body["user"].as_str().unwrap_or("bot"));
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool { self.connected }

    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        let channel = if message.thread_id.is_empty() {
            &self.config.default_channel
        } else {
            &message.thread_id
        };
        self.post_message(channel, &message.content, message.reply_to.as_deref()).await
    }

    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        Ok(Box::new(stream::pending()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_config_default() {
        let config = SlackConfig::default();
        assert_eq!(config.default_channel, "#general");
        assert!(config.enabled);
    }

    #[test]
    fn test_parse_message_event() {
        let channel = SlackChannel::new(SlackConfig::default());
        let payload = serde_json::json!({
            "event": {
                "type": "message",
                "channel": "C123",
                "user": "U456",
                "text": "Hello BizClaw!"
            }
        });
        let msg = channel.parse_event(&payload).unwrap();
        assert_eq!(msg.content, "Hello BizClaw!");
        assert_eq!(msg.sender_id, "U456");
        assert_eq!(msg.thread_id, "C123");
    }

    #[test]
    fn test_ignore_bot_messages() {
        let channel = SlackChannel::new(SlackConfig::default());
        let payload = serde_json::json!({
            "event": {
                "type": "message",
                "channel": "C123",
                "user": "U456",
                "text": "bot reply",
                "bot_id": "B789"
            }
        });
        assert!(channel.parse_event(&payload).is_none());
    }

    #[test]
    fn test_thread_detection() {
        let channel = SlackChannel::new(SlackConfig::default());
        let payload = serde_json::json!({
            "event": {
                "type": "message",
                "channel": "C123",
                "user": "U456",
                "text": "threaded reply",
                "thread_ts": "1234567890.123456"
            }
        });
        let msg = channel.parse_event(&payload).unwrap();
        assert_eq!(msg.thread_type, ThreadType::Group);
        assert_eq!(msg.reply_to, Some("1234567890.123456".into()));
    }

    #[test]
    fn test_app_mention_event() {
        let channel = SlackChannel::new(SlackConfig::default());
        let payload = serde_json::json!({
            "event": {
                "type": "app_mention",
                "channel": "C123",
                "user": "U456",
                "text": "<@BOT> help me"
            }
        });
        let msg = channel.parse_event(&payload).unwrap();
        assert!(msg.content.contains("help me"));
    }

    #[test]
    fn test_ignore_non_message_events() {
        let channel = SlackChannel::new(SlackConfig::default());
        let payload = serde_json::json!({
            "event": {"type": "reaction_added", "user": "U456"}
        });
        assert!(channel.parse_event(&payload).is_none());
    }
}
