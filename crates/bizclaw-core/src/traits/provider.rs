//! LLM Provider trait — swappable AI backends.

use async_trait::async_trait;

use crate::error::Result;
use crate::types::{Message, ModelInfo, ProviderResponse, ToolDefinition};

/// Configuration for generation parameters.
#[derive(Debug, Clone)]
pub struct GenerateParams {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: f32,
    pub stop: Vec<String>,
}

impl Default for GenerateParams {
    fn default() -> Self {
        Self {
            model: String::new(),
            temperature: 0.7,
            max_tokens: 4096,
            top_p: 0.9,
            stop: vec![],
        }
    }
}

/// Provider trait — every LLM backend implements this.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider identifier (e.g., "openai", "anthropic", "brain").
    fn name(&self) -> &str;

    /// Send a chat completion request.
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        params: &GenerateParams,
    ) -> Result<ProviderResponse>;

    /// List available models for this provider.
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;

    /// Check if the provider is available and configured.
    async fn health_check(&self) -> Result<bool>;
}
