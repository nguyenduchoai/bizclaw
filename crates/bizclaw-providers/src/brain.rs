//! Brain provider — local LLM via bizclaw-brain.

use async_trait::async_trait;
use bizclaw_core::config::BizClawConfig;
use bizclaw_core::error::Result;
use bizclaw_core::traits::provider::{GenerateParams, Provider};
use bizclaw_core::types::{Message, ModelInfo, ProviderResponse, ToolDefinition};

pub struct BrainProvider {
    engine: bizclaw_brain::BrainEngine,
}

impl BrainProvider {
    pub fn new(config: &BizClawConfig) -> Result<Self> {
        let brain_config = bizclaw_brain::BrainConfig {
            threads: config.brain.threads,
            max_tokens: config.brain.max_tokens,
            context_length: config.brain.context_length,
            temperature: config.brain.temperature,
            top_p: config.brain.top_p,
            json_mode: config.brain.json_mode,
        };
        Ok(Self {
            engine: bizclaw_brain::BrainEngine::new(brain_config),
        })
    }
}

#[async_trait]
impl Provider for BrainProvider {
    fn name(&self) -> &str { "brain" }

    async fn chat(
        &self,
        messages: &[Message],
        _tools: &[ToolDefinition],
        _params: &GenerateParams,
    ) -> Result<ProviderResponse> {
        // Format messages into a prompt
        let prompt = messages.iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let response = self.engine.generate(&prompt, self.engine.config().max_tokens)?;
        Ok(ProviderResponse::text(response))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo {
                id: "tinyllama-1.1b".into(),
                name: "TinyLlama 1.1B (local)".into(),
                provider: "brain".into(),
                context_length: 2048,
                max_output_tokens: Some(256),
            },
        ])
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.engine.is_loaded())
    }
}
