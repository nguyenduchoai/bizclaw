//! Xiaozhi Webhook Bridge — connects BizClaw agents to Xiaozhi voice devices.
//!
//! Architecture: Xiaozhi ESP32 → Xiaozhi Server (STT) → Webhook → BizClaw Agent → Response → TTS → Xiaozhi
//!
//! Supports:
//! - Incoming voice commands (text after STT)
//! - Response with TTS audio
//! - Multi-device routing by MAC address
//! - Session management per device

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Xiaozhi webhook bridge configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XiaozhiConfig {
    /// Whether the Xiaozhi bridge is enabled.
    #[serde(default)]
    pub enabled: bool,
    /// Shared secret for webhook verification.
    #[serde(default)]
    pub secret: String,
    /// TTS provider for responses ("edge", "openai", "elevenlabs").
    #[serde(default = "default_tts")]
    pub tts_provider: String,
    /// TTS voice.
    #[serde(default = "default_voice")]
    pub tts_voice: String,
    /// Default agent to route commands to.
    #[serde(default = "default_agent")]
    pub default_agent: String,
    /// Device-to-agent routing (MAC → agent_name).
    #[serde(default)]
    pub device_routing: HashMap<String, String>,
}

fn default_tts() -> String { "edge".to_string() }
fn default_voice() -> String { "vi-VN-HoaiMyNeural".to_string() }
fn default_agent() -> String { "default".to_string() }

impl Default for XiaozhiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            secret: String::new(),
            tts_provider: default_tts(),
            tts_voice: default_voice(),
            default_agent: default_agent(),
            device_routing: HashMap::new(),
        }
    }
}

/// Incoming request from Xiaozhi Server (after STT).
#[derive(Debug, Clone, Deserialize)]
pub struct XiaozhiRequest {
    /// Text content (after speech-to-text).
    pub content: String,
    /// Device MAC address for routing.
    #[serde(default)]
    pub device_mac: String,
    /// Session ID for conversation continuity.
    #[serde(default)]
    pub session_id: String,
    /// Device name (optional display name).
    #[serde(default)]
    pub device_name: String,
    /// Audio format preference for response.
    #[serde(default = "default_audio_format")]
    pub audio_format: String,
    /// Language code.
    #[serde(default = "default_lang")]
    pub lang: String,
}

fn default_audio_format() -> String { "mp3".to_string() }
fn default_lang() -> String { "vi".to_string() }

/// Response to Xiaozhi Server (for TTS).
#[derive(Debug, Clone, Serialize)]
pub struct XiaozhiResponse {
    /// Success status.
    pub ok: bool,
    /// Text response from agent.
    pub text: String,
    /// Agent name that processed the request.
    pub agent: String,
    /// Session ID for continuity.
    pub session_id: String,
    /// Whether TTS audio is included.
    pub has_audio: bool,
    /// Processing time in milliseconds.
    pub processing_ms: u64,
    /// Timestamp.
    pub timestamp: String,
}

/// Xiaozhi bridge — manages device sessions and routing.
pub struct XiaozhiBridge {
    pub config: XiaozhiConfig,
    /// Active device sessions: MAC → session_id.
    sessions: HashMap<String, DeviceSession>,
}

/// Per-device session state.
#[derive(Debug, Clone)]
struct DeviceSession {
    /// Session ID.
    session_id: String,
    /// Agent assigned to this device.
    agent_name: String,
    /// Total messages in this session.
    message_count: u32,
    /// Last activity timestamp.
    last_active: chrono::DateTime<Utc>,
}

impl XiaozhiBridge {
    /// Create a new Xiaozhi bridge.
    pub fn new(config: XiaozhiConfig) -> Self {
        Self {
            config,
            sessions: HashMap::new(),
        }
    }

    /// Route an incoming request to the appropriate agent.
    /// Returns the agent name to use.
    pub fn route_request(&mut self, req: &XiaozhiRequest) -> String {
        // Check device-specific routing
        let agent = self.config.device_routing
            .get(&req.device_mac)
            .cloned()
            .unwrap_or_else(|| self.config.default_agent.clone());

        // Update or create session
        let session = self.sessions
            .entry(req.device_mac.clone())
            .or_insert_with(|| DeviceSession {
                session_id: if req.session_id.is_empty() {
                    uuid::Uuid::new_v4().to_string()
                } else {
                    req.session_id.clone()
                },
                agent_name: agent.clone(),
                message_count: 0,
                last_active: Utc::now(),
            });

        session.message_count += 1;
        session.last_active = Utc::now();

        // Update session_id if provided
        if !req.session_id.is_empty() {
            session.session_id = req.session_id.clone();
        }

        agent
    }

    /// Build response.
    pub fn build_response(
        &self,
        text: &str,
        agent_name: &str,
        device_mac: &str,
        processing_ms: u64,
        has_audio: bool,
    ) -> XiaozhiResponse {
        let session_id = self.sessions
            .get(device_mac)
            .map(|s| s.session_id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        XiaozhiResponse {
            ok: true,
            text: text.to_string(),
            agent: agent_name.to_string(),
            session_id,
            has_audio,
            processing_ms,
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    /// Verify webhook signature (HMAC-SHA256).
    pub fn verify_signature(&self, payload: &str, signature: &str) -> bool {
        if self.config.secret.is_empty() {
            return true; // No secret = skip verification
        }
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.config.secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());
        let expected = format!("{:x}", mac.finalize().into_bytes());
        expected == signature
    }

    /// Get active device count.
    pub fn active_devices(&self) -> usize {
        self.sessions.len()
    }

    /// Get session info for a device.
    pub fn device_info(&self, mac: &str) -> Option<DeviceInfo> {
        self.sessions.get(mac).map(|s| DeviceInfo {
            session_id: s.session_id.clone(),
            agent_name: s.agent_name.clone(),
            message_count: s.message_count,
            last_active: s.last_active.to_rfc3339(),
        })
    }

    /// Clean up idle sessions (older than 30 minutes).
    pub fn cleanup_idle_sessions(&mut self) {
        let cutoff = Utc::now() - chrono::Duration::minutes(30);
        self.sessions.retain(|_, s| s.last_active > cutoff);
    }

    /// Check if enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Device info for API responses.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub session_id: String,
    pub agent_name: String,
    pub message_count: u32,
    pub last_active: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let bridge = XiaozhiBridge::new(XiaozhiConfig::default());
        assert!(!bridge.is_enabled());
        assert_eq!(bridge.active_devices(), 0);
    }

    #[test]
    fn test_route_request() {
        let mut bridge = XiaozhiBridge::new(XiaozhiConfig {
            enabled: true,
            default_agent: "assistant".to_string(),
            ..Default::default()
        });

        let req = XiaozhiRequest {
            content: "Xin chào".to_string(),
            device_mac: "AA:BB:CC:DD:EE:FF".to_string(),
            session_id: "s1".to_string(),
            device_name: "Xiaozhi-1".to_string(),
            audio_format: "mp3".to_string(),
            lang: "vi".to_string(),
        };

        let agent = bridge.route_request(&req);
        assert_eq!(agent, "assistant");
        assert_eq!(bridge.active_devices(), 1);
    }

    #[test]
    fn test_device_routing() {
        let mut routing = HashMap::new();
        routing.insert("AA:BB:CC:DD:EE:FF".to_string(), "sales-agent".to_string());

        let mut bridge = XiaozhiBridge::new(XiaozhiConfig {
            enabled: true,
            device_routing: routing,
            ..Default::default()
        });

        let req = XiaozhiRequest {
            content: "Doanh thu hôm nay".to_string(),
            device_mac: "AA:BB:CC:DD:EE:FF".to_string(),
            session_id: String::new(),
            device_name: String::new(),
            audio_format: "mp3".to_string(),
            lang: "vi".to_string(),
        };

        assert_eq!(bridge.route_request(&req), "sales-agent");
    }

    #[test]
    fn test_verify_signature() {
        let bridge = XiaozhiBridge::new(XiaozhiConfig {
            secret: "mysecret".to_string(),
            ..Default::default()
        });

        // Empty secret = always pass
        let bridge_nosecret = XiaozhiBridge::new(XiaozhiConfig::default());
        assert!(bridge_nosecret.verify_signature("anything", "anything"));

        // With secret, wrong sig should fail
        assert!(!bridge.verify_signature("payload", "wrong_signature"));
    }

    #[test]
    fn test_build_response() {
        let mut bridge = XiaozhiBridge::new(XiaozhiConfig::default());
        let req = XiaozhiRequest {
            content: "test".to_string(),
            device_mac: "MAC1".to_string(),
            session_id: "s1".to_string(),
            device_name: String::new(),
            audio_format: "mp3".to_string(),
            lang: "vi".to_string(),
        };
        bridge.route_request(&req);

        let resp = bridge.build_response("Hello", "agent1", "MAC1", 150, false);
        assert!(resp.ok);
        assert_eq!(resp.text, "Hello");
        assert_eq!(resp.session_id, "s1");
        assert_eq!(resp.processing_ms, 150);
    }

    #[test]
    fn test_cleanup_sessions() {
        let mut bridge = XiaozhiBridge::new(XiaozhiConfig::default());
        let req = XiaozhiRequest {
            content: "test".to_string(),
            device_mac: "MAC1".to_string(),
            session_id: String::new(),
            device_name: String::new(),
            audio_format: "mp3".to_string(),
            lang: "vi".to_string(),
        };
        bridge.route_request(&req);
        assert_eq!(bridge.active_devices(), 1);
        bridge.cleanup_idle_sessions();
        assert_eq!(bridge.active_devices(), 1); // Still active
    }
}
