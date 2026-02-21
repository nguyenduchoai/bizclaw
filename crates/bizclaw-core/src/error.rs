//! Unified error types for BizClaw.

use thiserror::Error;

/// Result type alias using BizClawError.
pub type Result<T> = std::result::Result<T, BizClawError>;

#[derive(Error, Debug)]
pub enum BizClawError {
    // Provider errors
    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("API key not configured for provider: {0}")]
    ApiKeyMissing(String),

    // Channel errors
    #[error("Channel error: {0}")]
    Channel(String),

    #[error("Channel not connected: {0}")]
    ChannelNotConnected(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    // Memory errors
    #[error("Memory backend error: {0}")]
    Memory(String),

    // Brain (local inference) errors
    #[error("Brain engine error: {0}")]
    Brain(String),

    #[error("Model load error: {0}")]
    ModelLoad(String),

    #[error("GGUF parse error: {0}")]
    GgufParse(String),

    #[error("Inference error: {0}")]
    Inference(String),

    // Tool errors
    #[error("Tool execution error: {0}")]
    Tool(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    // Security errors
    #[error("Security violation: {0}")]
    Security(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    // Config errors
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Config file not found: {0}")]
    ConfigNotFound(String),

    // Gateway errors
    #[error("Gateway error: {0}")]
    Gateway(String),

    // General errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("{0}")]
    Other(String),
}

impl BizClawError {
    pub fn provider(msg: impl Into<String>) -> Self {
        Self::Provider(msg.into())
    }

    pub fn channel(msg: impl Into<String>) -> Self {
        Self::Channel(msg.into())
    }

    pub fn brain(msg: impl Into<String>) -> Self {
        Self::Brain(msg.into())
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn security(msg: impl Into<String>) -> Self {
        Self::Security(msg.into())
    }
}
