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

    // Orchestration errors
    #[error("Delegation error: {0}")]
    Delegation(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("No permission: {0}")]
    NoPermission(String),

    #[error("Team error: {0}")]
    Team(String),

    #[error("Handoff error: {0}")]
    Handoff(String),

    #[error("Evaluate loop error: {0}")]
    EvaluateLoop(String),

    #[error("Quality gate failed: {0}")]
    QualityGate(String),

    // Database errors
    #[error("Database error: {0}")]
    Database(String),

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = BizClawError::Provider("timeout".into());
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_error_constructors() {
        let e1 = BizClawError::provider("test");
        assert!(matches!(e1, BizClawError::Provider(_)));

        let e2 = BizClawError::channel("test");
        assert!(matches!(e2, BizClawError::Channel(_)));

        let e3 = BizClawError::brain("test");
        assert!(matches!(e3, BizClawError::Brain(_)));

        let e4 = BizClawError::security("test");
        assert!(matches!(e4, BizClawError::Security(_)));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: BizClawError = io_err.into();
        assert!(matches!(err, BizClawError::Io(_)));
    }

    #[test]
    fn test_json_error_conversion() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let err: BizClawError = json_err.into();
        assert!(matches!(err, BizClawError::Json(_)));
    }

    #[test]
    fn test_all_error_variants_display() {
        let errors: Vec<BizClawError> = vec![
            BizClawError::Provider("p".into()),
            BizClawError::ProviderNotFound("p".into()),
            BizClawError::ModelNotFound("m".into()),
            BizClawError::ApiKeyMissing("k".into()),
            BizClawError::Channel("c".into()),
            BizClawError::ChannelNotConnected("c".into()),
            BizClawError::AuthFailed("a".into()),
            BizClawError::Memory("m".into()),
            BizClawError::Brain("b".into()),
            BizClawError::ModelLoad("l".into()),
            BizClawError::GgufParse("g".into()),
            BizClawError::Inference("i".into()),
            BizClawError::Tool("t".into()),
            BizClawError::ToolNotFound("t".into()),
            BizClawError::Security("s".into()),
            BizClawError::PermissionDenied("d".into()),
            BizClawError::Config("c".into()),
            BizClawError::ConfigNotFound("f".into()),
            BizClawError::Gateway("g".into()),
            BizClawError::Http("h".into()),
            BizClawError::Timeout("t".into()),
            BizClawError::RateLimited("r".into()),
            BizClawError::Delegation("d".into()),
            BizClawError::AgentNotFound("a".into()),
            BizClawError::NoPermission("n".into()),
            BizClawError::Team("t".into()),
            BizClawError::Handoff("h".into()),
            BizClawError::EvaluateLoop("e".into()),
            BizClawError::QualityGate("q".into()),
            BizClawError::Database("d".into()),
            BizClawError::Other("o".into()),
        ];

        for err in &errors {
            let display = err.to_string();
            assert!(!display.is_empty(), "Error should have display: {:?}", err);
        }
        // There should be 31 variants
        assert_eq!(errors.len(), 31);
    }

    #[test]
    fn test_error_is_debug() {
        let err = BizClawError::Provider("test".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Provider"));
    }

    #[test]
    fn test_config_constructor() {
        let err = BizClawError::config("bad config");
        assert_eq!(err.to_string(), "Configuration error: bad config");
    }

    #[test]
    fn test_result_type_alias() {
        fn returns_ok() -> Result<i32> { Ok(42) }
        fn returns_err() -> Result<i32> { Err(BizClawError::Other("fail".into())) }
        assert_eq!(returns_ok().unwrap(), 42);
        assert!(returns_err().is_err());
    }
}
