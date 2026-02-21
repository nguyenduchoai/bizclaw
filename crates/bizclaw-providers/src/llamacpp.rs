//! LlamaCpp provider stub.
use async_trait::async_trait;
use bizclaw_core::config::BizClawConfig;
use bizclaw_core::error::Result;
use bizclaw_core::traits::provider::{GenerateParams, Provider};
use bizclaw_core::types::{Message, ModelInfo, ProviderResponse, ToolDefinition};

pub struct LlamaCppProvider;
impl LlamaCppProvider {
    pub fn new(_config: &BizClawConfig) -> Result<Self> { Ok(Self) }
}
#[async_trait]
impl Provider for LlamaCppProvider {
    fn name(&self) -> &str { "llamacpp" }
    async fn chat(&self, _: &[Message], _: &[ToolDefinition], _: &GenerateParams) -> Result<ProviderResponse> {
        Ok(ProviderResponse::text("[llamacpp-stub] Coming soon"))
    }
    async fn list_models(&self) -> Result<Vec<ModelInfo>> { Ok(vec![]) }
    async fn health_check(&self) -> Result<bool> { Ok(false) }
}
