//! Custom OpenAI-compatible provider stub.
use async_trait::async_trait;
use bizclaw_core::config::BizClawConfig;
use bizclaw_core::error::Result;
use bizclaw_core::traits::provider::{GenerateParams, Provider};
use bizclaw_core::types::{Message, ModelInfo, ProviderResponse, ToolDefinition};

pub struct CustomProvider { endpoint: String }
impl CustomProvider {
    pub fn new(_config: &BizClawConfig, endpoint: &str) -> Result<Self> {
        Ok(Self { endpoint: endpoint.strip_prefix("custom:").unwrap_or(endpoint).to_string() })
    }
}
#[async_trait]
impl Provider for CustomProvider {
    fn name(&self) -> &str { "custom" }
    async fn chat(&self, _: &[Message], _: &[ToolDefinition], _: &GenerateParams) -> Result<ProviderResponse> {
        Ok(ProviderResponse::text(format!("[custom-stub] Endpoint: {}", self.endpoint)))
    }
    async fn list_models(&self) -> Result<Vec<ModelInfo>> { Ok(vec![]) }
    async fn health_check(&self) -> Result<bool> { Ok(false) }
}
