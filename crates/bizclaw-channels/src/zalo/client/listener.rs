//! Zalo WebSocket event listener — production-ready with auto-reconnect.
//! Handles: message, reaction, undo, group_event, typing.
//! Sends parsed events via mpsc channel for integration with ZaloChannel.

use super::models::ZaloMessage;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::types::IncomingMessage;
use futures::StreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

/// WebSocket event types from Zalo.
#[derive(Debug, Clone)]
pub enum ZaloEvent {
    /// New message received
    Message(ZaloMessage),
    /// Message recalled/undone
    MessageUndo { msg_id: String, thread_id: String },
    /// Reaction on a message
    Reaction {
        msg_id: String,
        reactor_id: String,
        reaction: String,
    },
    /// Typing indicator
    Typing { thread_id: String, user_id: String },
    /// Group member event (join/leave/kicked)
    GroupEvent {
        group_id: String,
        event_type: String,
        data: serde_json::Value,
    },
    /// Connection state changed
    ConnectionState(ConnectionState),
    /// Raw/unknown event
    Raw(serde_json::Value),
}

#[derive(Debug, Clone)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Reconnecting,
}

/// Zalo WebSocket listener — sends IncomingMessage via mpsc channel.
pub struct ZaloListener {
    ws_url: String,
    connected: Arc<AtomicBool>,
    /// Auto-reconnect enabled
    auto_reconnect: bool,
    /// Reconnect delay in milliseconds
    reconnect_delay_ms: u64,
    /// Max reconnect attempts (0 = unlimited)
    max_reconnect_attempts: u32,
    /// Whether to include self-sent messages
    self_listen: bool,
    /// Own user ID (to filter self messages)
    own_uid: String,
    /// Optional webhook URL to forward raw events
    webhook_url: Option<String>,
    /// Cipher key (zpw_enk base64) for decrypting AES-GCM binary payloads
    cipher_key: Option<String>,
}

impl ZaloListener {
    pub fn new(ws_url: &str) -> Self {
        Self {
            ws_url: ws_url.to_string(),
            connected: Arc::new(AtomicBool::new(false)),
            auto_reconnect: true,
            reconnect_delay_ms: 5000,
            max_reconnect_attempts: 0, // unlimited
            self_listen: false,
            own_uid: String::new(),
            webhook_url: None,
            cipher_key: None,
        }
    }

    /// Configure auto-reconnect behavior.
    pub fn with_reconnect(mut self, enabled: bool, delay_ms: u64, max_attempts: u32) -> Self {
        self.auto_reconnect = enabled;
        self.reconnect_delay_ms = delay_ms;
        self.max_reconnect_attempts = max_attempts;
        self
    }

    /// Set own user ID for self-message filtering.
    pub fn with_own_uid(mut self, uid: &str) -> Self {
        self.own_uid = uid.to_string();
        self
    }

    /// Set self-listen mode.
    pub fn with_self_listen(mut self, enabled: bool) -> Self {
        self.self_listen = enabled;
        self
    }

    /// Set webhook URL for forwarding raw events.
    pub fn with_webhook(mut self, url: Option<String>) -> Self {
        self.webhook_url = url;
        self
    }

    /// Set cipher key for decrypting AES-GCM binary WebSocket frames.
    pub fn with_cipher_key(mut self, key: Option<String>) -> Self {
        self.cipher_key = key;
        self
    }

    /// Start listening and sending IncomingMessage to the provided sender.
    /// This spawns a background task that auto-reconnects.
    /// Returns the receiver end for consuming messages.
    pub fn start_listening(&self, cookie: String) -> mpsc::Receiver<IncomingMessage> {
        let (tx, rx) = mpsc::channel::<IncomingMessage>(256);

        let ws_url = self.ws_url.clone();
        let connected = self.connected.clone();
        let auto_reconnect = self.auto_reconnect;
        let reconnect_delay_ms = self.reconnect_delay_ms;
        let max_reconnect_attempts = self.max_reconnect_attempts;
        let self_listen = self.self_listen;
        let own_uid = self.own_uid.clone();
        let webhook_url = self.webhook_url.clone();
        let cipher_key = self.cipher_key.clone();

        tokio::spawn(async move {
            let mut attempt = 0u32;

            loop {
                tracing::info!("Zalo WebSocket: connecting to {}...", ws_url);

                match Self::ws_connect_and_listen(
                    &ws_url,
                    &cookie,
                    &tx,
                    &connected,
                    self_listen,
                    &own_uid,
                    webhook_url.as_deref(),
                    cipher_key.as_deref(),
                )
                .await
                {
                    Ok(()) => {
                        tracing::info!("Zalo WebSocket: connection closed normally");
                    }
                    Err(e) => {
                        tracing::error!("Zalo WebSocket error: {e}");
                    }
                }

                connected.store(false, Ordering::SeqCst);

                if !auto_reconnect {
                    tracing::info!("Zalo WebSocket: auto-reconnect disabled, stopping");
                    break;
                }

                attempt += 1;
                if max_reconnect_attempts > 0 && attempt >= max_reconnect_attempts {
                    tracing::error!(
                        "Zalo WebSocket: reached max reconnect attempts ({}), stopping",
                        max_reconnect_attempts
                    );
                    break;
                }

                // Check if the receiver has been dropped (channel closed)
                if tx.is_closed() {
                    tracing::info!("Zalo WebSocket: message channel closed, stopping");
                    break;
                }

                let delay = reconnect_delay_ms * (attempt as u64).min(6); // exponential-ish backoff, max 30s
                tracing::info!(
                    "Zalo WebSocket: reconnecting in {}ms (attempt #{})...",
                    delay,
                    attempt
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
        });

        rx
    }

    /// Internal: connect to WebSocket and process messages.
    async fn ws_connect_and_listen(
        ws_url: &str,
        _cookie: &str,
        tx: &mpsc::Sender<IncomingMessage>,
        connected: &Arc<AtomicBool>,
        self_listen: bool,
        own_uid: &str,
        webhook_url: Option<&str>,
        cipher_key: Option<&str>,
    ) -> Result<()> {
        let (ws_stream, _response) = tokio_tungstenite::connect_async(ws_url)
            .await
            .map_err(|e| BizClawError::Channel(format!("WebSocket connect failed: {e}")))?;

        connected.store(true, Ordering::SeqCst);
        tracing::info!("Zalo WebSocket: connected successfully");

        let (_write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            let json_text_opt = match msg {
                Ok(WsMessage::Text(text)) => Some(text),
                Ok(WsMessage::Binary(data)) => {
                    // Zalo binary frame: [version:1] [cmd:2 LE] [sub_cmd:1] [payload...]
                    if data.len() >= 4 {
                        let _version = data[0];
                        let cmd = u16::from_le_bytes([data[1], data[2]]);
                        let sub_cmd = data[3];

                        // Determine encrypt_type from cmd/sub_cmd (zca-js convention)
                        let encrypt_type: i64 = if sub_cmd == 0 { 0 } else { sub_cmd as i64 };
                        let payload_str = BASE64.encode(&data[4..]);

                        match super::crypto::decode_event_data(
                            &payload_str,
                            encrypt_type,
                            cipher_key,
                        ) {
                            Ok(json) => Some(json.to_string()),
                            Err(e) => {
                                // Fallback: try raw UTF-8 for unencrypted payloads
                                tracing::trace!(
                                    "Binary decode failed (cmd={}, enc={}): {}, trying raw",
                                    cmd,
                                    encrypt_type,
                                    e
                                );
                                String::from_utf8(data[4..].to_vec()).ok()
                            }
                        }
                    } else {
                        None
                    }
                }
                Ok(WsMessage::Ping(data)) => {
                    tracing::trace!("Zalo ping received ({} bytes)", data.len());
                    None
                }
                Ok(WsMessage::Close(frame)) => {
                    tracing::info!("Zalo WebSocket closed: {:?}", frame);
                    let mut is_3000 = false;
                    if let Some(f) = frame {
                        let code: u16 = f.code.into();
                        if code == 3000 {
                            is_3000 = true;
                        }
                    }
                    connected.store(false, Ordering::SeqCst);

                    if is_3000 {
                        let incoming = IncomingMessage {
                            channel: "zalo".to_string(),
                            thread_id: "system".to_string(),
                            sender_id: "system".to_string(),
                            sender_name: Some("System".into()),
                            content: "ERROR_CODE_3000_RELOGIN_REQUIRED".to_string(),
                            thread_type: bizclaw_core::types::ThreadType::Direct,
                            timestamp: chrono::Utc::now(),
                            reply_to: None,
                        };
                        let _ = tx.send(incoming).await;
                        return Err(BizClawError::Channel(
                            "WebSocket closed with Code 3000: Duplicate session".into(),
                        ));
                    }
                    break;
                }
                Err(e) => {
                    tracing::error!("Zalo WebSocket error: {e}");
                    connected.store(false, Ordering::SeqCst);
                    break;
                }
                _ => None,
            };

            if let Some(text) = json_text_opt {
                // 1. Forward to webhook if configured (fire and forget)
                if let Some(url) = webhook_url {
                    let client = reqwest::Client::new();
                    let url_clone = url.to_string();
                    let payload = text.clone();
                    tokio::spawn(async move {
                        let _ = client
                            .post(&url_clone)
                            .header("Content-Type", "application/json")
                            .body(payload)
                            .send()
                            .await;
                    });
                }

                // 2. Parse and handle internal routing
                match Self::parse_ws_event(&text) {
                    Ok(event) => {
                        match event {
                            ZaloEvent::Message(zalo_msg) => {
                                // Skip self-sent messages unless self_listen is on
                                if !self_listen && zalo_msg.sender_id == own_uid {
                                    continue;
                                }

                                let content = match &zalo_msg.content {
                                    super::models::ZaloMessageContent::Text(t) => t.clone(),
                                    super::models::ZaloMessageContent::Attachment(v) => {
                                        format!("[attachment: {}]", v)
                                    }
                                };

                                let incoming = IncomingMessage {
                                    channel: "zalo".to_string(),
                                    thread_id: zalo_msg.thread_id.clone(),
                                    sender_id: zalo_msg.sender_id.clone(),
                                    sender_name: None,
                                    content,
                                    thread_type: bizclaw_core::types::ThreadType::Direct,
                                    timestamp: chrono::DateTime::from_timestamp(
                                        zalo_msg.timestamp as i64,
                                        0,
                                    )
                                    .unwrap_or_else(chrono::Utc::now),
                                    reply_to: None,
                                };

                                if tx.send(incoming).await.is_err() {
                                    tracing::warn!(
                                        "Zalo WebSocket: message channel closed, stopping"
                                    );
                                    return Ok(());
                                }

                                tracing::debug!(
                                    "Zalo: received message from {} in thread {}",
                                    zalo_msg.sender_id,
                                    zalo_msg.thread_id
                                );
                            }
                            ZaloEvent::Typing { thread_id, user_id } => {
                                tracing::trace!("Zalo: typing from {} in {}", user_id, thread_id);
                            }
                            ZaloEvent::Reaction {
                                msg_id, reaction, ..
                            } => {
                                tracing::debug!("Zalo: reaction '{}' on msg {}", reaction, msg_id);
                            }
                            ZaloEvent::MessageUndo { msg_id, .. } => {
                                tracing::debug!("Zalo: message {} recalled", msg_id);
                            }
                            ZaloEvent::GroupEvent {
                                group_id,
                                event_type,
                                ..
                            } => {
                                tracing::debug!(
                                    "Zalo: group event '{}' in {}",
                                    event_type,
                                    group_id
                                );
                            }
                            ZaloEvent::ConnectionState(state) => {
                                tracing::info!("Zalo: connection state: {:?}", state);
                            }
                            ZaloEvent::Raw(json) => {
                                tracing::trace!("Zalo: raw event: {}", json);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse Zalo event: {e}");
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse a WebSocket text message into a ZaloEvent.
    fn parse_ws_event(text: &str) -> Result<ZaloEvent> {
        let json: serde_json::Value = serde_json::from_str(text)
            .map_err(|e| BizClawError::Channel(format!("Invalid JSON: {e}")))?;

        let cmd = json["cmd"].as_i64().unwrap_or(0);

        match cmd {
            501 => {
                // New message
                Ok(ZaloEvent::Message(ZaloMessage {
                    msg_id: json["data"]["msgId"].as_str().unwrap_or("").into(),
                    thread_id: json["data"]["toid"]
                        .as_str()
                        .or_else(|| json["data"]["idTo"].as_str())
                        .unwrap_or("")
                        .into(),
                    sender_id: json["data"]["uidFrom"]
                        .as_str()
                        .or_else(|| json["data"]["uid"].as_str())
                        .unwrap_or("")
                        .into(),
                    content: super::models::ZaloMessageContent::Text(
                        json["data"]["content"]
                            .as_str()
                            .or_else(|| json["data"]["msg"].as_str())
                            .unwrap_or("")
                            .into(),
                    ),
                    timestamp: json["data"]["ts"]
                        .as_u64()
                        .or_else(|| json["data"]["tsMsg"].as_u64())
                        .unwrap_or(0),
                    is_self: false,
                }))
            }
            521 => {
                // Message undo
                Ok(ZaloEvent::MessageUndo {
                    msg_id: json["data"]["msgId"].as_str().unwrap_or("").into(),
                    thread_id: json["data"]["toid"].as_str().unwrap_or("").into(),
                })
            }
            612 => {
                // Reaction
                Ok(ZaloEvent::Reaction {
                    msg_id: json["data"]["msgId"].as_str().unwrap_or("").into(),
                    reactor_id: json["data"]["uidFrom"].as_str().unwrap_or("").into(),
                    reaction: json["data"]["rType"].as_str().unwrap_or("").into(),
                })
            }
            600 | 601 => {
                // Typing indicator
                Ok(ZaloEvent::Typing {
                    thread_id: json["data"]["toid"].as_str().unwrap_or("").into(),
                    user_id: json["data"]["uidFrom"].as_str().unwrap_or("").into(),
                })
            }
            _ => Ok(ZaloEvent::Raw(json)),
        }
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }
}
