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
    /// ⚠️ DEPRECATED for SaaS: Use environment variables instead.
    /// Kept for backward compatibility with single-tenant config files.
    #[serde(default)]
    pub api_key: String,
    /// Custom endpoint URL. Empty = use default endpoint for the provider.
    #[serde(default)]
    pub endpoint: String,
    /// Fallback provider for automatic failover.
    /// When the primary provider fails, requests are routed to this fallback.
    /// Example: "groq" or "deepseek". Empty = no failover.
    #[serde(default)]
    pub fallback_provider: String,
    /// Generation temperature.
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

impl LlmConfig {
    /// Resolve API key with priority: env var → config.toml field.
    /// Never exposes raw key to AI agents — use `key_status()` for that.
    pub fn resolve_api_key(&self) -> String {
        // Provider-specific env vars (industry standard names)
        let provider_env = match self.provider.as_str() {
            "openai" => Some("OPENAI_API_KEY"),
            "anthropic" => Some("ANTHROPIC_API_KEY"),
            "gemini" | "google" => Some("GOOGLE_API_KEY"),
            "deepseek" => Some("DEEPSEEK_API_KEY"),
            "groq" => Some("GROQ_API_KEY"),
            "openrouter" => Some("OPENROUTER_API_KEY"),
            _ => None,
        };

        // 1. Check BIZCLAW_LLM_API_KEY (universal override)
        if let Ok(key) = std::env::var("BIZCLAW_LLM_API_KEY")
            && !key.is_empty()
        {
            return key;
        }
        // 2. Check provider-specific env var
        if let Some(env_name) = provider_env
            && let Ok(key) = std::env::var(env_name)
            && !key.is_empty()
        {
            return key;
        }
        // 3. Fall back to config.toml field
        self.api_key.clone()
    }

    /// Get masked key status for safe display (to Agent or Dashboard).
    /// Returns (is_configured, source, masked_value).
    pub fn key_status(&self) -> (bool, &'static str, String) {
        if let Ok(key) = std::env::var("BIZCLAW_LLM_API_KEY")
            && !key.is_empty()
        {
            return (true, "env", mask_key(&key));
        }
        let provider_env = match self.provider.as_str() {
            "openai" => Some("OPENAI_API_KEY"),
            "anthropic" => Some("ANTHROPIC_API_KEY"),
            "gemini" | "google" => Some("GOOGLE_API_KEY"),
            "deepseek" => Some("DEEPSEEK_API_KEY"),
            "groq" => Some("GROQ_API_KEY"),
            "openrouter" => Some("OPENROUTER_API_KEY"),
            _ => None,
        };
        if let Some(env_name) = provider_env
            && let Ok(key) = std::env::var(env_name)
            && !key.is_empty()
        {
            return (true, "env", mask_key(&key));
        }
        if !self.api_key.is_empty() {
            return (true, "config", mask_key(&self.api_key));
        }
        (false, "none", String::new())
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            api_key: String::new(),
            endpoint: String::new(),
            fallback_provider: String::new(),
            temperature: default_temperature(),
        }
    }
}

/// Mask a secret key for safe display: show first 3 and last 3 chars.
/// e.g., "sk-proj-abc123xyz789" → "sk-***789"
pub fn mask_key(key: &str) -> String {
    if key.len() <= 6 {
        return "***".to_string();
    }
    let prefix = &key[..3];
    let suffix = &key[key.len() - 3..];
    format!("{}***{}", prefix, suffix)
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
    /// Enable extended thinking / deep reasoning mode.
    #[serde(default)]
    pub extended_thinking: bool,
    /// Thinking budget in tokens (Anthropic/DashScope). 0 = provider default.
    #[serde(default)]
    pub thinking_budget_tokens: u32,
    /// Reasoning effort for OpenAI-compatible ("low", "medium", "high"). Empty = default.
    #[serde(default)]
    pub reasoning_effort: String,
    /// Enterprise SSO configuration (SAML/OIDC).
    #[serde(default)]
    pub sso: SsoConfig,
    /// Analytics and metrics configuration.
    #[serde(default)]
    pub analytics: AnalyticsConfig,
    /// LLM fine-tuning pipeline configuration.
    #[serde(default)]
    pub fine_tuning: FineTuningConfig,
    /// Edge/IoT gateway configuration.
    #[serde(default)]
    pub edge_gateway: EdgeGatewayConfig,
    /// Plugin marketplace configuration.
    #[serde(default)]
    pub plugin_marketplace: PluginMarketplaceConfig,
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
            extended_thinking: false,
            thinking_budget_tokens: 0,
            reasoning_effort: String::new(),
            sso: SsoConfig::default(),
            analytics: AnalyticsConfig::default(),
            fine_tuning: FineTuningConfig::default(),
            edge_gateway: EdgeGatewayConfig::default(),
            plugin_marketplace: PluginMarketplaceConfig::default(),
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

// ─────────────────────────────────────────────────────────────
// Hot-Reload Config — DeerFlow-inspired mtime-based auto-reload
// ─────────────────────────────────────────────────────────────

/// A config wrapper that automatically reloads when the file changes.
///
/// Inspired by DeerFlow 2.0's `get_app_config()` which caches the parsed config
/// and automatically reloads when the file's mtime increases. This provides
/// zero-downtime config changes without requiring process restarts.
///
/// ## Usage
/// ```rust,no_run
/// use bizclaw_core::config::HotConfig;
///
/// let hot = HotConfig::new();
/// // First call loads from disk, subsequent calls return cached version
/// let config = hot.get().unwrap();
/// // After user edits config.toml, next call auto-detects the change
/// let fresh_config = hot.get().unwrap();
/// ```
pub struct HotConfig {
    inner: std::sync::Mutex<HotConfigInner>,
}

struct HotConfigInner {
    cached: Option<BizClawConfig>,
    path: PathBuf,
    last_mtime: Option<std::time::SystemTime>,
}

impl HotConfig {
    /// Create a HotConfig watching the default config path.
    pub fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(HotConfigInner {
                cached: None,
                path: BizClawConfig::default_path(),
                last_mtime: None,
            }),
        }
    }

    /// Create a HotConfig watching a specific path.
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            inner: std::sync::Mutex::new(HotConfigInner {
                cached: None,
                path: path.into(),
                last_mtime: None,
            }),
        }
    }

    /// Get the current config, auto-reloading if the file has changed.
    ///
    /// Returns a clone of the cached config for thread safety.
    pub fn get(&self) -> Result<BizClawConfig> {
        let mut inner = self.inner.lock().map_err(|e| {
            crate::error::BizClawError::Config(format!("Config lock poisoned: {e}"))
        })?;

        let current_mtime = std::fs::metadata(&inner.path)
            .and_then(|m| m.modified())
            .ok();

        let needs_reload = match (&inner.cached, &inner.last_mtime, &current_mtime) {
            (None, _, _) => true,                      // No cached config
            (_, None, Some(_)) => true,                // File appeared
            (_, Some(old), Some(new)) => new > old,    // File modified
            (_, Some(_), None) => false,               // File disappeared, keep cached
            (_, None, None) => inner.cached.is_none(), // No file, no cache
        };

        if needs_reload {
            let config = if inner.path.exists() {
                BizClawConfig::load_from(&inner.path)?
            } else {
                BizClawConfig::default()
            };

            tracing::info!("🔄 Config hot-reload: loaded from {}", inner.path.display());

            inner.cached = Some(config);
            inner.last_mtime = current_mtime;
        }

        Ok(inner.cached.clone().unwrap_or_default())
    }

    /// Force a reload regardless of mtime.
    pub fn reload(&self) -> Result<BizClawConfig> {
        let mut inner = self.inner.lock().map_err(|e| {
            crate::error::BizClawError::Config(format!("Config lock poisoned: {e}"))
        })?;

        let config = if inner.path.exists() {
            BizClawConfig::load_from(&inner.path)?
        } else {
            BizClawConfig::default()
        };

        let mtime = std::fs::metadata(&inner.path)
            .and_then(|m| m.modified())
            .ok();

        inner.cached = Some(config.clone());
        inner.last_mtime = mtime;

        tracing::info!("🔄 Config force-reload from {}", inner.path.display());

        Ok(config)
    }

    /// Check if a reload is pending (file changed but not yet loaded).
    pub fn is_stale(&self) -> bool {
        let inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };

        let current_mtime = std::fs::metadata(&inner.path)
            .and_then(|m| m.modified())
            .ok();

        match (&inner.last_mtime, &current_mtime) {
            (Some(old), Some(new)) => new > old,
            (None, Some(_)) => true,
            _ => false,
        }
    }
}

impl Default for HotConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BrainMode {
    CloudFirst,
    LocalFirst,
    CloudOnly,
    LocalOnly,
}

impl Default for BrainMode {
    fn default() -> Self {
        Self::LocalFirst
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    RoundRobin,
    LeastLatency,
    CostAware,
    PriorityBased,
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        Self::PriorityBased
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_rpm")]
    pub requests_per_minute: u32,
    #[serde(default = "default_tpm")]
    pub tokens_per_minute: u32,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: u32,
}

fn default_rpm() -> u32 {
    60
}
fn default_tpm() -> u32 {
    100000
}
fn default_max_concurrent() -> u32 {
    10
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: default_rpm(),
            tokens_per_minute: default_tpm(),
            max_concurrent: default_max_concurrent(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainProviderConfig {
    pub name: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
    #[serde(default = "default_provider_priority")]
    pub priority: u32,
}

fn default_provider_priority() -> u32 {
    100
}

impl Default for BrainProviderConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            api_key_env: None,
            base_url: None,
            models: vec![],
            rate_limit: None,
            priority: default_provider_priority(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainRoutingConfig {
    #[serde(default = "default_routing_strategy")]
    pub strategy: RoutingStrategy,
    #[serde(default)]
    pub fallback_chain: Vec<String>,
    #[serde(default = "default_local_fallback")]
    pub local_fallback: bool,
    #[serde(default)]
    pub enable_cost_tracking: bool,
}

fn default_routing_strategy() -> RoutingStrategy {
    RoutingStrategy::default()
}
fn default_local_fallback() -> bool {
    true
}

impl Default for BrainRoutingConfig {
    fn default() -> Self {
        Self {
            strategy: default_routing_strategy(),
            fallback_chain: vec![],
            local_fallback: default_local_fallback(),
            enable_cost_tracking: false,
        }
    }
}

/// Brain (local LLM) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainConfig {
    #[serde(default = "bool_true")]
    pub enabled: bool,
    #[serde(default = "default_brain_mode")]
    pub mode: BrainMode,
    #[serde(default)]
    pub providers: Vec<BrainProviderConfig>,
    #[serde(default)]
    pub routing: BrainRoutingConfig,
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
fn default_brain_mode() -> BrainMode {
    BrainMode::LocalFirst
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
            mode: default_brain_mode(),
            providers: vec![],
            routing: BrainRoutingConfig::default(),
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
    /// DEPRECATED: Pairing code auth removed — SaaS uses JWT from Platform login.
    /// Kept for backward compatibility when loading old config files.
    #[serde(default, skip_serializing)]
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
            require_pairing: false,
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
    /// Tools that require human approval before execution (enterprise).
    /// Example: ["shell", "http_request"]
    #[serde(default)]
    pub approval_required_tools: Vec<String>,
    /// Auto-deny approval after this many seconds (0 = never, default 300).
    #[serde(default = "default_approval_timeout")]
    pub auto_approve_timeout_secs: u64,
    /// explicitly allowed tool names. If empty, all tools are allowed except those in forbidden.
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// explicitly forbidden tool names. Overrides allowed_tools.
    #[serde(default = "default_forbidden_tools")]
    pub forbidden_tools: Vec<String>,
}

fn default_autonomy_level() -> String {
    "supervised".into()
}
fn default_approval_timeout() -> u64 {
    300
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
fn default_forbidden_tools() -> Vec<String> {
    vec![]
}

impl Default for AutonomyConfig {
    fn default() -> Self {
        Self {
            level: default_autonomy_level(),
            workspace_only: true,
            allowed_commands: default_allowed_commands(),
            forbidden_paths: default_forbidden_paths(),
            approval_required_tools: vec![],
            auto_approve_timeout_secs: default_approval_timeout(),
            allowed_tools: vec![],
            forbidden_tools: default_forbidden_tools(),
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

/// Channel configuration — supports multiple instances per channel type.
///
/// TOML config examples:
///
/// Single instance (backward compatible):
/// ```toml
/// [channel.telegram]
/// name = "Main Bot"
/// bot_token = "123:ABC"
/// ```
///
/// Multiple instances:
/// ```toml
/// [[channel.telegram]]
/// name = "Sales Bot"
/// bot_token = "123:ABC"
///
/// [[channel.telegram]]
/// name = "Support Bot"
/// bot_token = "456:DEF"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelConfig {
    #[serde(default, deserialize_with = "deserialize_one_or_many")]
    pub zalo: Vec<ZaloChannelConfig>,
    #[serde(default, deserialize_with = "deserialize_one_or_many")]
    pub telegram: Vec<TelegramChannelConfig>,
    #[serde(default, deserialize_with = "deserialize_one_or_many")]
    pub discord: Vec<DiscordChannelConfig>,
    #[serde(default, deserialize_with = "deserialize_one_or_many")]
    pub email: Vec<EmailChannelConfig>,
    #[serde(default, deserialize_with = "deserialize_one_or_many")]
    pub whatsapp: Vec<WhatsAppChannelConfig>,
    #[serde(default, deserialize_with = "deserialize_one_or_many")]
    pub webhook: Vec<WebhookChannelConfig>,
}

/// Deserialize helper: accept either a single object or an array.
/// This allows backward compatibility with old configs that have `[channel.telegram]`
/// while also supporting `[[channel.telegram]]` for multiple instances.
fn deserialize_one_or_many<'de, D, T>(deserializer: D) -> std::result::Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    use serde::de;

    struct OneOrMany<T>(std::marker::PhantomData<T>);

    impl<'de, T: serde::Deserialize<'de>> de::Visitor<'de> for OneOrMany<T> {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a single object or an array of objects")
        }

        fn visit_seq<A>(self, seq: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            Vec::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }

        fn visit_map<M>(self, map: M) -> std::result::Result<Self::Value, M::Error>
        where
            M: de::MapAccess<'de>,
        {
            T::deserialize(de::value::MapAccessDeserializer::new(map)).map(|v| vec![v])
        }
    }

    deserializer.deserialize_any(OneOrMany(std::marker::PhantomData))
}

/// Zalo channel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloChannelConfig {
    /// Instance name (e.g., "Zalo Cá Nhân", "Zalo OA Shop").
    #[serde(default = "default_zalo_name")]
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_zalo_mode")]
    pub mode: String,
    #[serde(default)]
    pub personal: ZaloPersonalConfig,
    #[serde(default)]
    pub official: ZaloOfficialConfig,
    #[serde(default)]
    pub rate_limit: ZaloRateLimitConfig,
    #[serde(default)]
    pub allowlist: ZaloAllowlistConfig,
    /// Zalo OA access token (from developers.zalo.me) — for notification dispatch.
    #[serde(default)]
    pub oa_access_token: String,
    /// Zalo user_id to receive notifications (admin recipient).
    #[serde(default)]
    pub notify_user_id: String,
}

fn default_zalo_mode() -> String {
    "personal".into()
}

impl Default for ZaloChannelConfig {
    fn default() -> Self {
        Self {
            name: default_zalo_name(),
            enabled: false,
            mode: default_zalo_mode(),
            personal: ZaloPersonalConfig::default(),
            official: ZaloOfficialConfig::default(),
            rate_limit: ZaloRateLimitConfig::default(),
            allowlist: ZaloAllowlistConfig::default(),
            oa_access_token: String::new(),
            notify_user_id: String::new(),
        }
    }
}

/// Zalo Official Account (OA) API configuration — server-side, no Android needed.
/// Register at https://oa.zalo.me to get credentials.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZaloOfficialConfig {
    /// Zalo App ID (from developers.zalo.me).
    #[serde(default)]
    pub app_id: String,
    /// Zalo App Secret — used for webhook MAC signature verification.
    #[serde(default)]
    pub app_secret: String,
    /// OA Access Token — for sending messages via OA API.
    #[serde(default)]
    pub access_token: String,
    /// OA Refresh Token — for auto-renewal of access_token.
    #[serde(default)]
    pub refresh_token: String,
    /// OA ID — your Official Account identifier.
    #[serde(default)]
    pub oa_id: String,
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
    #[serde(default)]
    pub webhook_url: String,
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
            webhook_url: String::new(),
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
    /// Instance name (e.g., "Bot Bán Hàng", "Bot Support").
    #[serde(default = "default_telegram_name")]
    pub name: String,
    pub enabled: bool,
    pub bot_token: String,
    #[serde(default)]
    pub allowed_chat_ids: Vec<i64>,
}

impl TelegramChannelConfig {
    /// Resolve bot token: env var → config.toml.
    pub fn resolve_bot_token(&self) -> String {
        std::env::var("BIZCLAW_TELEGRAM_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.bot_token.clone())
    }

    pub fn key_status(&self) -> (bool, &'static str, String) {
        if let Ok(key) = std::env::var("BIZCLAW_TELEGRAM_BOT_TOKEN")
            && !key.is_empty()
        {
            return (true, "env", mask_key(&key));
        }
        if !self.bot_token.is_empty() {
            return (true, "config", mask_key(&self.bot_token));
        }
        (false, "none", String::new())
    }
}

fn default_telegram_name() -> String {
    "Telegram".into()
}
fn default_zalo_name() -> String {
    "Zalo".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordChannelConfig {
    /// Instance name.
    #[serde(default = "default_discord_name")]
    pub name: String,
    pub enabled: bool,
    pub bot_token: String,
    #[serde(default)]
    pub allowed_channel_ids: Vec<u64>,
}

impl DiscordChannelConfig {
    /// Resolve bot token: env var → config.toml.
    pub fn resolve_bot_token(&self) -> String {
        std::env::var("BIZCLAW_DISCORD_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.bot_token.clone())
    }

    pub fn key_status(&self) -> (bool, &'static str, String) {
        if let Ok(key) = std::env::var("BIZCLAW_DISCORD_BOT_TOKEN")
            && !key.is_empty()
        {
            return (true, "env", mask_key(&key));
        }
        if !self.bot_token.is_empty() {
            return (true, "config", mask_key(&self.bot_token));
        }
        (false, "none", String::new())
    }
}

fn default_discord_name() -> String {
    "Discord".into()
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

impl WhatsAppChannelConfig {
    /// Resolve access token: env var → config.toml.
    pub fn resolve_access_token(&self) -> String {
        std::env::var("BIZCLAW_WHATSAPP_ACCESS_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.access_token.clone())
    }

    pub fn key_status(&self) -> (bool, &'static str, String) {
        if let Ok(key) = std::env::var("BIZCLAW_WHATSAPP_ACCESS_TOKEN")
            && !key.is_empty()
        {
            return (true, "env", mask_key(&key));
        }
        if !self.access_token.is_empty() {
            return (true, "config", mask_key(&self.access_token));
        }
        (false, "none", String::new())
    }
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

// ═══ Enterprise SSO Configuration ═══

/// SSO authentication configuration (SAML 2.0 / OpenID Connect).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoConfig {
    /// Enable SSO authentication.
    #[serde(default)]
    pub enabled: bool,
    /// SSO provider type: "saml" or "oidc".
    #[serde(default = "default_sso_provider")]
    pub provider: String,
    /// OIDC: Issuer URL (e.g. https://accounts.google.com).
    #[serde(default)]
    pub issuer_url: String,
    /// OIDC: Client ID.
    #[serde(default)]
    pub client_id: String,
    /// OIDC: Client Secret.
    #[serde(default)]
    pub client_secret: String,
    /// OIDC: Redirect URI after authentication.
    #[serde(default)]
    pub redirect_uri: String,
    /// OIDC: Scopes to request.
    #[serde(default = "default_sso_scopes")]
    pub scopes: Vec<String>,
    /// SAML: IdP Metadata URL.
    #[serde(default)]
    pub idp_metadata_url: String,
    /// SAML: SP Entity ID.
    #[serde(default)]
    pub sp_entity_id: String,
    /// Allow local password login alongside SSO.
    #[serde(default = "bool_true")]
    pub allow_local_login: bool,
    /// Auto-create users on first SSO login.
    #[serde(default = "bool_true")]
    pub auto_provision: bool,
    /// Default role for auto-provisioned users.
    #[serde(default = "default_sso_role")]
    pub default_role: String,
}

fn default_sso_provider() -> String {
    "oidc".into()
}
fn default_sso_scopes() -> Vec<String> {
    vec!["openid".into(), "email".into(), "profile".into()]
}
fn default_sso_role() -> String {
    "user".into()
}

impl Default for SsoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_sso_provider(),
            issuer_url: String::new(),
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: String::new(),
            scopes: default_sso_scopes(),
            idp_metadata_url: String::new(),
            sp_entity_id: String::new(),
            allow_local_login: true,
            auto_provision: true,
            default_role: default_sso_role(),
        }
    }
}

// ═══ Analytics Configuration ═══

/// Analytics and metrics tracking configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    /// Enable analytics collection.
    #[serde(default = "bool_true")]
    pub enabled: bool,
    /// Data retention period in days.
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    /// Track token usage per conversation.
    #[serde(default = "bool_true")]
    pub track_tokens: bool,
    /// Track tool usage statistics.
    #[serde(default = "bool_true")]
    pub track_tools: bool,
    /// Track channel activity metrics.
    #[serde(default = "bool_true")]
    pub track_channels: bool,
    /// Track response latency.
    #[serde(default = "bool_true")]
    pub track_latency: bool,
    /// Export format: "json", "csv", "prometheus".
    #[serde(default = "default_export_format")]
    pub export_format: String,
    /// Prometheus metrics endpoint path.
    #[serde(default = "default_metrics_path")]
    pub metrics_path: String,
    /// Daily report recipients (email addresses).
    #[serde(default)]
    pub report_recipients: Vec<String>,
}

fn default_retention_days() -> u32 {
    90
}
fn default_export_format() -> String {
    "json".into()
}
fn default_metrics_path() -> String {
    "/metrics".into()
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_days: default_retention_days(),
            track_tokens: true,
            track_tools: true,
            track_channels: true,
            track_latency: true,
            export_format: default_export_format(),
            metrics_path: default_metrics_path(),
            report_recipients: vec![],
        }
    }
}

// ═══ Fine-Tuning Pipeline Configuration ═══

/// LLM fine-tuning pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuningConfig {
    /// Enable fine-tuning pipeline.
    #[serde(default)]
    pub enabled: bool,
    /// Provider for fine-tuning: "openai", "together", "fireworks".
    #[serde(default = "default_ft_provider")]
    pub provider: String,
    /// API key for fine-tuning provider (if different from main).
    #[serde(default)]
    pub api_key: String,
    /// Base model to fine-tune.
    #[serde(default = "default_ft_base_model")]
    pub base_model: String,
    /// Training dataset path or directory.
    #[serde(default = "default_ft_dataset_dir")]
    pub dataset_dir: String,
    /// Number of training epochs.
    #[serde(default = "default_ft_epochs")]
    pub epochs: u32,
    /// Learning rate multiplier.
    #[serde(default = "default_ft_lr")]
    pub learning_rate_multiplier: f32,
    /// Batch size.
    #[serde(default = "default_ft_batch")]
    pub batch_size: u32,
    /// Auto-collect training data from conversations.
    #[serde(default)]
    pub auto_collect: bool,
    /// Minimum rating to include in training data (1-5).
    #[serde(default = "default_ft_min_rating")]
    pub min_rating: u32,
    /// Max training samples to collect.
    #[serde(default = "default_ft_max_samples")]
    pub max_samples: u32,
}

fn default_ft_provider() -> String {
    "openai".into()
}
fn default_ft_base_model() -> String {
    "gpt-4o-mini-2024-07-18".into()
}
fn default_ft_dataset_dir() -> String {
    "~/.bizclaw/fine-tuning/datasets".into()
}
fn default_ft_epochs() -> u32 {
    3
}
fn default_ft_lr() -> f32 {
    1.8
}
fn default_ft_batch() -> u32 {
    4
}
fn default_ft_min_rating() -> u32 {
    4
}
fn default_ft_max_samples() -> u32 {
    10000
}

impl Default for FineTuningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_ft_provider(),
            api_key: String::new(),
            base_model: default_ft_base_model(),
            dataset_dir: default_ft_dataset_dir(),
            epochs: default_ft_epochs(),
            learning_rate_multiplier: default_ft_lr(),
            batch_size: default_ft_batch(),
            auto_collect: false,
            min_rating: default_ft_min_rating(),
            max_samples: default_ft_max_samples(),
        }
    }
}

// ═══ Edge/IoT Gateway Configuration ═══

/// Edge deployment and IoT gateway configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeGatewayConfig {
    /// Enable edge gateway mode.
    #[serde(default)]
    pub enabled: bool,
    /// Edge node ID (unique per deployment).
    #[serde(default)]
    pub node_id: String,
    /// MQTT broker URL (e.g. mqtt://localhost:1883).
    #[serde(default)]
    pub mqtt_broker: String,
    /// MQTT topic prefix for commands.
    #[serde(default = "default_mqtt_topic")]
    pub mqtt_topic_prefix: String,
    /// CoAP server port for lightweight IoT devices.
    #[serde(default = "default_coap_port")]
    pub coap_port: u16,
    /// Sync interval with cloud (seconds). 0 = disabled.
    #[serde(default = "default_sync_interval")]
    pub sync_interval_secs: u32,
    /// Cloud API endpoint for syncing.
    #[serde(default)]
    pub cloud_endpoint: String,
    /// Offline queue capacity (messages buffered when cloud disconnected).
    #[serde(default = "default_offline_queue")]
    pub offline_queue_size: u32,
    /// Supported device protocols.
    #[serde(default = "default_protocols")]
    pub protocols: Vec<String>,
    /// Xiaozhi voice device integration.
    #[serde(default)]
    pub xiaozhi_enabled: bool,
    /// Xiaozhi Server OTA endpoint.
    #[serde(default)]
    pub xiaozhi_ota_url: String,
}

fn default_mqtt_topic() -> String {
    "bizclaw/edge".into()
}
fn default_coap_port() -> u16 {
    5683
}
fn default_sync_interval() -> u32 {
    60
}
fn default_offline_queue() -> u32 {
    1000
}
fn default_protocols() -> Vec<String> {
    vec!["mqtt".into(), "http".into(), "websocket".into()]
}

impl Default for EdgeGatewayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            node_id: String::new(),
            mqtt_broker: String::new(),
            mqtt_topic_prefix: default_mqtt_topic(),
            coap_port: default_coap_port(),
            sync_interval_secs: default_sync_interval(),
            cloud_endpoint: String::new(),
            offline_queue_size: default_offline_queue(),
            protocols: default_protocols(),
            xiaozhi_enabled: false,
            xiaozhi_ota_url: String::new(),
        }
    }
}

// ═══ Plugin Marketplace Configuration ═══

/// Plugin marketplace and extension management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketplaceConfig {
    /// Enable plugin marketplace.
    #[serde(default = "bool_true")]
    pub enabled: bool,
    /// Plugin registry URL.
    #[serde(default = "default_plugin_registry")]
    pub registry_url: String,
    /// Local plugin directory.
    #[serde(default = "default_plugin_dir")]
    pub plugin_dir: String,
    /// Auto-update installed plugins.
    #[serde(default)]
    pub auto_update: bool,
    /// Verify plugin signatures before install.
    #[serde(default = "bool_true")]
    pub verify_signatures: bool,
    /// Installed plugins list.
    #[serde(default)]
    pub installed: Vec<PluginEntry>,
}

/// A single installed plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    /// Plugin ID.
    pub id: String,
    /// Plugin version.
    #[serde(default)]
    pub version: String,
    /// Whether plugin is enabled.
    #[serde(default = "bool_true")]
    pub enabled: bool,
    /// Plugin-specific config (JSON).
    #[serde(default)]
    pub config: serde_json::Value,
}

fn default_plugin_registry() -> String {
    "https://plugins.bizclaw.vn/api/v1".into()
}
fn default_plugin_dir() -> String {
    "~/.bizclaw/plugins".into()
}

impl Default for PluginMarketplaceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            registry_url: default_plugin_registry(),
            plugin_dir: default_plugin_dir(),
            auto_update: false,
            verify_signatures: true,
            installed: vec![],
        }
    }
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
