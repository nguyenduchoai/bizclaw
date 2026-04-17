//! Chat message types and roles.

use serde::{Deserialize, Serialize};

/// Role in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<super::ToolCall>>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            name: None,
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
        }
    }
}

/// Incoming message from a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    pub channel: String,
    pub thread_id: String,
    pub sender_id: String,
    pub sender_name: Option<String>,
    pub content: String,
    pub thread_type: ThreadType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub reply_to: Option<String>,
}

/// Outgoing message to a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    pub thread_id: String,
    pub content: String,
    pub thread_type: ThreadType,
    pub reply_to: Option<String>,
}

/// Thread type for channel messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThreadType {
    Direct,
    Group,
}

/// Response from an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<super::ToolCall>,
    pub finish_reason: Option<String>,
    pub usage: Option<Usage>,
}

impl ProviderResponse {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tool_calls: vec![],
            finish_reason: Some("stop".into()),
            usage: None,
        }
    }

    pub fn with_tool_calls(tool_calls: Vec<super::ToolCall>) -> Self {
        Self {
            content: None,
            tool_calls,
            finish_reason: Some("tool_calls".into()),
            usage: None,
        }
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    /// Anthropic prompt caching: tokens written to cache this request.
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    /// Anthropic prompt caching: tokens read from cache (saved cost).
    #[serde(default)]
    pub cache_read_input_tokens: u32,
    /// Extended thinking: tokens consumed by the thinking process.
    #[serde(default)]
    pub thinking_tokens: u32,
}

impl Default for Usage {
    fn default() -> Self {
        Self {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            thinking_tokens: 0,
        }
    }
}

impl Usage {
    /// Estimate cost in USD based on provider pricing (per 1M tokens).
    /// Returns (input_cost, output_cost, total_cost).
    pub fn estimate_cost_usd(&self, provider: &str, model: &str) -> (f64, f64, f64) {
        // Pricing per 1M tokens (input, output) — updated 2026-Q1
        let (input_rate, output_rate) = match (provider, model) {
            // Local — FREE
            ("ollama", _) | ("llamacpp", _) | ("brain", _) => (0.0, 0.0),
            // DeepSeek — cheapest cloud
            ("deepseek", "deepseek-chat") => (0.27, 1.10),
            ("deepseek", "deepseek-reasoner") => (0.55, 2.19),
            // Groq — free tier available
            ("groq", _) => (0.05, 0.08),
            // OpenAI
            ("openai", "gpt-4o-mini") => (0.15, 0.60),
            ("openai", "gpt-4o") => (2.50, 10.00),
            // Anthropic
            ("anthropic", m) if m.contains("haiku") => (0.80, 4.00),
            ("anthropic", _) => (3.00, 15.00),
            // Gemini
            ("gemini", m) if m.contains("flash") => (0.075, 0.30),
            ("gemini", _) => (1.25, 5.00),
            // DashScope (Qwen)
            ("dashscope", _) => (0.20, 0.60),
            // Default conservative estimate
            _ => (1.00, 3.00),
        };

        let input_cost = self.prompt_tokens as f64 / 1_000_000.0 * input_rate;
        let output_cost = self.completion_tokens as f64 / 1_000_000.0 * output_rate;
        (input_cost, output_cost, input_cost + output_cost)
    }

    /// Estimate cost in VND (Vietnamese Dong).
    /// Exchange rate: ~25,500 VND/USD (configurable via env var BIZCLAW_USD_VND_RATE).
    pub fn estimate_cost_vnd(&self, provider: &str, model: &str) -> f64 {
        let rate: f64 = std::env::var("BIZCLAW_USD_VND_RATE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(25_500.0);
        let (_, _, total_usd) = self.estimate_cost_usd(provider, model);
        total_usd * rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_constructors() {
        let sys = Message::system("You are helpful.");
        assert_eq!(sys.role, Role::System);
        assert_eq!(sys.content, "You are helpful.");

        let user = Message::user("Hello");
        assert_eq!(user.role, Role::User);

        let asst = Message::assistant("Hi!");
        assert_eq!(asst.role, Role::Assistant);
    }

    #[test]
    fn test_role_display() {
        assert_eq!(Role::System.to_string(), "system");
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Assistant.to_string(), "assistant");
        assert_eq!(Role::Tool.to_string(), "tool");
    }

    #[test]
    fn test_message_json_roundtrip() {
        let msg = Message::user("test message");
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.content, "test message");
        assert_eq!(parsed.role, Role::User);
    }

    #[test]
    fn test_provider_response() {
        let resp = ProviderResponse::text("hello");
        assert_eq!(resp.content, Some("hello".into()));
        assert!(resp.tool_calls.is_empty());
    }
}
