use thiserror::Error;

pub type Result<T> = std::result::Result<T, BrowserError>;

#[derive(Error, Debug)]
pub enum BrowserError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("CDP error: {0}")]
    CdpError(String),

    #[error("Command timeout: {0}")]
    Timeout(String),

    #[error("Element not found: {0}")]
    ElementNotFound(String),

    #[error("Navigation failed: {0}")]
    NavigationFailed(String),

    #[error("Screenshot failed: {0}")]
    ScreenshotFailed(String),

    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    #[error("Invalid selector: {0}")]
    InvalidSelector(String),

    #[error("Auth failed: {0}")]
    AuthFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Launch error: {0}")]
    LaunchError(String),

    #[error("No active session")]
    NoSession,
}
