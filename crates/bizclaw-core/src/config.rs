//! BizClaw configuration system.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::traits::identity::Identity;

/// LLM provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider name (e.g., "openai", "anthropic", "gemini", "deepseek", "groq", "ollama", "llamacpp", "brain", "openrouter").
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Model identifier (e.g., "gpt-4o-mini", "claude-sonnet-4-20250514").
    #[serde(default = "default_model")]
    pub model: String,
    /// API key for the provider.
    #[serde(default)]
    pub api_key: String,
    /// Custom endpoint URL. Empty = use default endpoint for the provider.
    #[serde(default)]
    pub endpoint: String,
    /// Generation temperature.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            api_key: String::new(),
            endpoint: String::new(),
            temperature: default_temperature(),
        }
    }
}

/// Root configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BizClawConfig {
    #[serde(default = "default_api_key")]
    pub api_key: String,
    /// Custom API base URL (e.g. CLIProxyAPI: http://localhost:8787/v1)
    /// Leave empty to use provider default.
    #[serde(default)]
    pub api_base_url: String,
    #[serde(default = "default_provider")]
    pub default_provider: String,
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default = "default_temperature")]
    pub default_temperature: f32,
    /// LLM provider configuration section.
    #[serde(default, rename = "LLM")]
    pub llm: LlmConfig,
    #[serde(default)]
    pub brain: BrainConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub gateway: GatewayConfig,
    #[serde(default)]
    pub autonomy: AutonomyConfig,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub tunnel: TunnelConfig,
    #[serde(default)]
    pub secrets: SecretsConfig,
    #[serde(default)]
    pub identity: Identity,
    #[serde(default)]
    pub channel: ChannelConfig,
    /// MCP server configurations.
    #[serde(default)]
    pub mcp_servers: Vec<McpServerEntry>,
    /// Quality Gate — optional evaluator for response review.
    #[serde(default)]
    pub quality_gate: Option<QualityGateConfig>,
}

fn default_api_key() -> String {
    String::new()
}
fn default_provider() -> String {
    "openai".into()
}
fn default_model() -> String {
    "gpt-4o-mini".into()
}
fn default_temperature() -> f32 {
    0.7
}

impl Default for BizClawConfig {
    fn default() -> Self {
        Self {
            api_key: default_api_key(),
            api_base_url: String::new(),
            default_provider: default_provider(),
            default_model: default_model(),
            default_temperature: default_temperature(),
            llm: LlmConfig::default(),
            brain: BrainConfig::default(),
            memory: MemoryConfig::default(),
            gateway: GatewayConfig::default(),
            autonomy: AutonomyConfig::default(),
            runtime: RuntimeConfig::default(),
            tunnel: TunnelConfig::default(),
            secrets: SecretsConfig::default(),
            identity: Identity::default(),
            channel: ChannelConfig::default(),
            mcp_servers: vec![],
            quality_gate: None,
        }
    }
}

impl BizClawConfig {
    /// Load config from the default path (~/.bizclaw/config.toml).
    pub fn load() -> Result<Self> {
        let path = Self::default_path();
        if path.exists() {
            Self::load_from(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load config from a specific path.
    pub fn load_from(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::error::BizClawError::Config(format!("Failed to read config: {e}"))
        })?;
        let config: Self = toml::from_str(&content).map_err(|e| {
            crate::error::BizClawError::Config(format!("Failed to parse config: {e}"))
        })?;
        Ok(config)
    }

    /// Save config to the default path.
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| {
            crate::error::BizClawError::Config(format!("Failed to serialize config: {e}"))
        })?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Get the default config path.
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".bizclaw")
            .join("config.toml")
    }

    /// Get the BizClaw home directory.
    pub fn home_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".bizclaw")
    }
}

/// Brain (local LLM) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainConfig {
    #[serde(default = "bool_true")]
    pub enabled: bool,
    #[serde(default = "default_model_path")]
    pub model_path: String,
    #[serde(default = "default_threads")]
    pub threads: u32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_context_length")]
    pub context_length: u32,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
    #[serde(default = "bool_true")]
    pub auto_download: bool,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    #[serde(default)]
    pub json_mode: bool,
    #[serde(default)]
    pub fallback: Option<BrainFallback>,
}

fn bool_true() -> bool {
    true
}
fn default_model_path() -> String {
    "~/.bizclaw/models/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf".into()
}
fn default_threads() -> u32 {
    4
}
fn default_max_tokens() -> u32 {
    256
}
fn default_context_length() -> u32 {
    2048
}
fn default_cache_dir() -> String {
    "~/.bizclaw/cache".into()
}
fn default_top_p() -> f32 {
    0.9
}

impl Default for BrainConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model_path: default_model_path(),
            threads: default_threads(),
            max_tokens: default_max_tokens(),
            context_length: default_context_length(),
            cache_dir: default_cache_dir(),
            auto_download: true,
            temperature: default_temperature(),
            top_p: default_top_p(),
            json_mode: false,
            fallback: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainFallback {
    pub provider: String,
    pub model: String,
}

/// Memory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_memory_backend")]
    pub backend: String,
    #[serde(default = "bool_true")]
    pub auto_save: bool,
    #[serde(default = "default_embedding_provider")]
    pub embedding_provider: String,
    #[serde(default = "default_vector_weight")]
    pub vector_weight: f32,
    #[serde(default = "default_keyword_weight")]
    pub keyword_weight: f32,
}

fn default_memory_backend() -> String {
    "sqlite".into()
}
fn default_embedding_provider() -> String {
    "none".into()
}
fn default_vector_weight() -> f32 {
    0.7
}
fn default_keyword_weight() -> f32 {
    0.3
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: default_memory_backend(),
            auto_save: true,
            embedding_provider: default_embedding_provider(),
            vector_weight: default_vector_weight(),
            keyword_weight: default_keyword_weight(),
        }
    }
}

/// Gateway configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "bool_true")]
    pub require_pairing: bool,
}

fn default_port() -> u16 {
    3000
}
fn default_host() -> String {
    "127.0.0.1".into()
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            host: default_host(),
            require_pairing: true,
        }
    }
}

/// Autonomy / security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyConfig {
    #[serde(default = "default_autonomy_level")]
    pub level: String,
    #[serde(default = "bool_true")]
    pub workspace_only: bool,
    #[serde(default = "default_allowed_commands")]
    pub allowed_commands: Vec<String>,
    #[serde(default = "default_forbidden_paths")]
    pub forbidden_paths: Vec<String>,
}

fn default_autonomy_level() -> String {
    "supervised".into()
}
fn default_allowed_commands() -> Vec<String> {
    vec!["git", "npm", "cargo", "ls", "cat", "grep"]
        .into_iter()
        .map(String::from)
        .collect()
}
fn default_forbidden_paths() -> Vec<String> {
    vec![
        "/etc", "/root", "/proc", "/sys", "~/.ssh", "~/.gnupg", "~/.aws",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

impl Default for AutonomyConfig {
    fn default() -> Self {
        Self {
            level: default_autonomy_level(),
            workspace_only: true,
            allowed_commands: default_allowed_commands(),
            forbidden_paths: default_forbidden_paths(),
        }
    }
}

/// Runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    #[serde(default = "default_runtime_kind")]
    pub kind: String,
}

fn default_runtime_kind() -> String {
    "native".into()
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            kind: default_runtime_kind(),
        }
    }
}

/// Tunnel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    #[serde(default = "default_tunnel_provider")]
    pub provider: String,
}

fn default_tunnel_provider() -> String {
    "none".into()
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            provider: default_tunnel_provider(),
        }
    }
}

/// Secrets configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    #[serde(default = "bool_true")]
    pub encrypt: bool,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self { encrypt: true }
    }
}

/// Channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelConfig {
    #[serde(default)]
    pub zalo: Option<ZaloChannelConfig>,
    #[serde(default)]
    pub telegram: Option<TelegramChannelConfig>,
    #[serde(default)]
    pub discord: Option<DiscordChannelConfig>,
    #[serde(default)]
    pub email: Option<EmailChannelConfig>,
    #[serde(default)]
    pub whatsapp: Option<WhatsAppChannelConfig>,
    #[serde(default)]
    pub webhook: Option<WebhookChannelConfig>,
}

/// Zalo channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloChannelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_zalo_mode")]
    pub mode: String,
    #[serde(default)]
    pub personal: ZaloPersonalConfig,
    #[serde(default)]
    pub rate_limit: ZaloRateLimitConfig,
    #[serde(default)]
    pub allowlist: ZaloAllowlistConfig,
}

fn default_zalo_mode() -> String {
    "personal".into()
}

impl Default for ZaloChannelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: default_zalo_mode(),
            personal: ZaloPersonalConfig::default(),
            rate_limit: ZaloRateLimitConfig::default(),
            allowlist: ZaloAllowlistConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloPersonalConfig {
    #[serde(default = "default_cookie_path")]
    pub cookie_path: String,
    #[serde(default)]
    pub imei: String,
    #[serde(default)]
    pub user_agent: String,
    #[serde(default)]
    pub self_listen: bool,
    #[serde(default = "bool_true")]
    pub auto_reconnect: bool,
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_ms: u64,
    #[serde(default)]
    pub proxy: String,
}

fn default_cookie_path() -> String {
    "~/.bizclaw/zalo/cookie.json".into()
}
fn default_reconnect_delay() -> u64 {
    5000
}

impl Default for ZaloPersonalConfig {
    fn default() -> Self {
        Self {
            cookie_path: default_cookie_path(),
            imei: String::new(),
            user_agent: String::new(),
            self_listen: false,
            auto_reconnect: true,
            reconnect_delay_ms: default_reconnect_delay(),
            proxy: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloRateLimitConfig {
    #[serde(default = "default_max_per_minute")]
    pub max_messages_per_minute: u32,
    #[serde(default = "default_max_per_hour")]
    pub max_messages_per_hour: u32,
    #[serde(default = "default_cooldown")]
    pub cooldown_on_error_ms: u64,
}

fn default_max_per_minute() -> u32 {
    20
}
fn default_max_per_hour() -> u32 {
    200
}
fn default_cooldown() -> u64 {
    30000
}

impl Default for ZaloRateLimitConfig {
    fn default() -> Self {
        Self {
            max_messages_per_minute: default_max_per_minute(),
            max_messages_per_hour: default_max_per_hour(),
            cooldown_on_error_ms: default_cooldown(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloAllowlistConfig {
    #[serde(default)]
    pub user_ids: Vec<String>,
    #[serde(default)]
    pub group_ids: Vec<String>,
    #[serde(default = "bool_true")]
    pub block_strangers: bool,
}

impl Default for ZaloAllowlistConfig {
    fn default() -> Self {
        Self {
            user_ids: vec![],
            group_ids: vec![],
            block_strangers: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramChannelConfig {
    pub enabled: bool,
    pub bot_token: String,
    #[serde(default)]
    pub allowed_chat_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordChannelConfig {
    pub enabled: bool,
    pub bot_token: String,
    #[serde(default)]
    pub allowed_channel_ids: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailChannelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub imap_host: String,
    #[serde(default = "default_imap_port_cfg")]
    pub imap_port: u16,
    #[serde(default)]
    pub smtp_host: String,
    #[serde(default = "default_smtp_port_cfg")]
    pub smtp_port: u16,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
}

fn default_imap_port_cfg() -> u16 {
    993
}
fn default_smtp_port_cfg() -> u16 {
    587
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppChannelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub phone_number_id: String,
    #[serde(default)]
    pub webhook_verify_token: String,
    #[serde(default)]
    pub business_id: String,
}

/// Generic Webhook channel configuration.
/// Allows external systems (Zapier, n8n, custom APIs) to send messages to BizClaw
/// and optionally receive outbound replies via a callback URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookChannelConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Shared secret for HMAC-SHA256 signature verification on inbound webhooks.
    #[serde(default)]
    pub secret: String,
    /// URL to POST outbound replies/messages to.
    #[serde(default)]
    pub outbound_url: String,
}

/// MCP server entry — one per [[mcp_servers]] in config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    /// Display name for this server.
    pub name: String,
    /// Command to start the MCP server process.
    pub command: String,
    /// Arguments to the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables to set.
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    /// Whether this server is enabled.
    #[serde(default = "default_mcp_enabled")]
    pub enabled: bool,
}

fn default_mcp_enabled() -> bool {
    true
}

/// Quality Gate configuration — evaluator reviews agent responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityGateConfig {
    /// Evaluator system prompt (e.g., "Check for accuracy and completeness").
    #[serde(default)]
    pub evaluator_prompt: String,
    /// Model to use for evaluation (defaults to agent's model).
    #[serde(default)]
    pub evaluator_model: Option<String>,
    /// Maximum revision rounds before accepting response.
    #[serde(default)]
    pub max_revisions: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = BizClawConfig::default();
        assert_eq!(config.default_provider, "openai");
        assert_eq!(config.default_model, "gpt-4o-mini");
        assert!((config.default_temperature - 0.7).abs() < 0.01);
        assert_eq!(config.identity.name, "BizClaw");
    }

    #[test]
    fn test_config_from_toml() {
        let toml_str = r#"
            default_provider = "ollama"
            default_model = "llama3.2"
            default_temperature = 0.5

            [identity]
            name = "TestBot"
            persona = "A test assistant"
            system_prompt = "You are a test bot."
        "#;

        let config: BizClawConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.default_provider, "ollama");
        assert_eq!(config.default_model, "llama3.2");
        assert_eq!(config.identity.name, "TestBot");
    }

    #[test]
    fn test_config_missing_fields_use_defaults() {
        let toml_str = "";
        let config: BizClawConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.default_provider, "openai");
        assert_eq!(config.gateway.port, 3000);
    }

    #[test]
    fn test_home_dir() {
        let home = BizClawConfig::home_dir();
        assert!(home.to_string_lossy().contains("bizclaw"));
    }
}
