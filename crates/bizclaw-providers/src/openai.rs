//! OpenAI / OpenRouter provider implementation.

use async_trait::async_trait;
use bizclaw_core::config::BizClawConfig;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::provider::{GenerateParams, Provider};
use bizclaw_core::types::{Message, ModelInfo, ProviderResponse, ToolDefinition};

pub struct OpenAiProvider {
    api_key: String,
    api_url: String,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(config: &BizClawConfig) -> Result<Self> {
        let api_key = if config.api_key.is_empty() {
            std::env::var("OPENAI_API_KEY")
                .or_else(|_| std::env::var("OPENROUTER_API_KEY"))
                .unwrap_or_default()
        } else {
            config.api_key.clone()
        };

        let api_url = if config.default_provider == "openrouter" {
            "https://openrouter.ai/api/v1".into()
        } else {
            std::env::var("OPENAI_API_BASE")
                .unwrap_or_else(|_| "https://api.openai.com/v1".into())
        };

        Ok(Self {
            api_key,
            api_url,
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str { "openai" }

    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        params: &GenerateParams,
    ) -> Result<ProviderResponse> {
        if self.api_key.is_empty() {
            return Err(BizClawError::ApiKeyMissing("openai".into()));
        }

        let mut body = serde_json::json!({
            "model": params.model,
            "messages": messages,
            "temperature": params.temperature,
            "max_tokens": params.max_tokens,
        });

        if !tools.is_empty() {
            let tool_defs: Vec<serde_json::Value> = tools.iter().map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            }).collect();
            body["tools"] = serde_json::Value::Array(tool_defs);
        }

        let resp = self.client
            .post(format!("{}/chat/completions", self.api_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| BizClawError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(BizClawError::Provider(format!("OpenAI API error {status}: {text}")));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| BizClawError::Http(e.to_string()))?;

        let choice = json["choices"].get(0)
            .ok_or_else(|| BizClawError::Provider("No choices in response".into()))?;

        let content = choice["message"]["content"].as_str().map(String::from);
        let tool_calls = if let Some(tc) = choice["message"]["tool_calls"].as_array() {
            tc.iter().filter_map(|t| {
                Some(bizclaw_core::types::ToolCall {
                    id: t["id"].as_str()?.to_string(),
                    r#type: "function".to_string(),
                    function: bizclaw_core::types::FunctionCall {
                        name: t["function"]["name"].as_str()?.to_string(),
                        arguments: t["function"]["arguments"].as_str()?.to_string(),
                    },
                })
            }).collect()
        } else {
            vec![]
        };

        Ok(ProviderResponse {
            content,
            tool_calls,
            finish_reason: choice["finish_reason"].as_str().map(String::from),
            usage: json["usage"].as_object().map(|u| bizclaw_core::types::Usage {
                prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            }),
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(vec![
            ModelInfo { id: "gpt-4o".into(), name: "GPT-4o".into(), provider: "openai".into(), context_length: 128000, max_output_tokens: Some(4096) },
            ModelInfo { id: "gpt-4o-mini".into(), name: "GPT-4o Mini".into(), provider: "openai".into(), context_length: 128000, max_output_tokens: Some(4096) },
        ])
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(!self.api_key.is_empty())
    }
}
