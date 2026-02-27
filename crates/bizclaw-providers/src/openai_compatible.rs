//! Unified OpenAI-compatible provider.
//!
//! A single struct that handles chat completions for ALL OpenAI-compatible APIs.
//! Includes Anthropic prompt caching support (cache_control) — inspired by
//! OpenFang's approach to minimize token costs on repeated system prompts.
//! Different providers are distinguished only by endpoint URL, auth style, and API key.

use async_trait::async_trait;
use bizclaw_core::config::BizClawConfig;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::provider::{GenerateParams, Provider};
use bizclaw_core::types::{
    FunctionCall, Message, ModelInfo, ProviderResponse, ToolCall, ToolDefinition, Usage,
};
use serde_json::{Value, json};

use crate::provider_registry::{AuthStyle, ProviderConfig};

/// A unified provider that works with any OpenAI-compatible API.
pub struct OpenAiCompatibleProvider {
    /// Provider name (e.g., "openai", "groq", "deepseek").
    name: String,
    /// API key for authentication.
    api_key: String,
    /// Base URL for the API (e.g., "https://api.openai.com/v1").
    base_url: String,
    /// Path for chat completions (e.g., "/chat/completions").
    chat_path: String,
    /// Path for listing models (e.g., "/models").
    models_path: String,
    /// Authentication style.
    auth_style: AuthStyle,
    /// Default models to return from `list_models`.
    default_models: Vec<ModelInfo>,
    /// HTTP client.
    client: reqwest::Client,
}

impl OpenAiCompatibleProvider {
    /// Create from a known provider config + BizClawConfig.
    ///
    /// Resolution order:
    /// - API key: `config.llm.api_key` > `config.api_key` > env vars > empty
    /// - Base URL: `config.llm.endpoint` > env override > registry default
    pub fn from_registry(registry: &ProviderConfig, config: &BizClawConfig) -> Result<Self> {
        // Resolve API key: config.llm.api_key > config.api_key > env vars > empty
        let api_key = if !config.llm.api_key.is_empty() {
            config.llm.api_key.clone()
        } else if !config.api_key.is_empty() {
            config.api_key.clone()
        } else {
            registry
                .env_keys
                .iter()
                .find_map(|key| std::env::var(key).ok())
                .unwrap_or_default()
        };

        // Resolve base URL: config.llm.endpoint > env override > registry default
        let base_url = if !config.llm.endpoint.is_empty() {
            config.llm.endpoint.clone()
        } else {
            registry
                .base_url_env
                .and_then(|env_key| {
                    let val = std::env::var(env_key).ok()?;
                    // For OLLAMA_HOST / LLAMACPP_HOST, append /v1 if not present
                    if val.ends_with("/v1") {
                        Some(val)
                    } else {
                        Some(format!("{}/v1", val.trim_end_matches('/')))
                    }
                })
                .unwrap_or_else(|| registry.base_url.to_string())
        };

        let default_models = registry
            .default_models
            .iter()
            .map(|m| m.to_model_info(registry.name))
            .collect();

        Ok(Self {
            name: registry.name.to_string(),
            api_key,
            base_url,
            chat_path: registry.chat_path.to_string(),
            models_path: registry.models_path.to_string(),
            auth_style: registry.auth_style,
            default_models,
            client: reqwest::Client::new(),
        })
    }

    /// Create for a custom endpoint (e.g., "custom:https://my-server.com/v1").
    pub fn custom(endpoint: &str, config: &BizClawConfig) -> Result<Self> {
        let base_url = endpoint
            .strip_prefix("custom:")
            .unwrap_or(endpoint)
            .trim_end_matches('/')
            .to_string();

        let api_key = if !config.api_key.is_empty() {
            config.api_key.clone()
        } else {
            std::env::var("CUSTOM_API_KEY").unwrap_or_default()
        };

        let auth_style = if api_key.is_empty() {
            AuthStyle::None
        } else {
            AuthStyle::Bearer
        };

        Ok(Self {
            name: "custom".to_string(),
            api_key,
            base_url,
            chat_path: "/chat/completions".to_string(),
            models_path: "/models".to_string(),
            auth_style,
            default_models: vec![],
            client: reqwest::Client::new(),
        })
    }

    /// Build the auth header for the request.
    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match self.auth_style {
            AuthStyle::Bearer if !self.api_key.is_empty() => {
                req.header("Authorization", format!("Bearer {}", self.api_key))
            }
            _ => req,
        }
    }
}

#[async_trait]
impl Provider for OpenAiCompatibleProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        params: &GenerateParams,
    ) -> Result<ProviderResponse> {
        // For providers that require auth, check API key
        if self.auth_style != AuthStyle::None && self.api_key.is_empty() {
            return Err(BizClawError::ApiKeyMissing(self.name.clone()));
        }

        let is_anthropic = self.name == "anthropic" || self.base_url.contains("anthropic");

        // Build request body — standard OpenAI format
        let mut body = json!({
            "model": params.model,
            "temperature": params.temperature,
            "max_tokens": params.max_tokens,
        });

        // ═══════════════════════════════════════
        // Anthropic Prompt Caching — cache_control
        // ═══════════════════════════════════════
        if is_anthropic {
            // Anthropic uses top-level "system" field (not messages[0])
            // with cache_control for prompt caching
            let mut non_system_msgs: Vec<Value> = Vec::new();
            let mut system_blocks: Vec<Value> = Vec::new();

            for msg in messages {
                if msg.role == bizclaw_core::types::Role::System {
                    system_blocks.push(json!({
                        "type": "text",
                        "text": msg.content,
                        "cache_control": { "type": "ephemeral" }
                    }));
                } else {
                    non_system_msgs.push(serde_json::to_value(msg).unwrap_or_default());
                }
            }

            if !system_blocks.is_empty() {
                body["system"] = Value::Array(system_blocks);
            }
            body["messages"] = Value::Array(non_system_msgs);

            tracing::debug!(
                "🧊 Anthropic prompt caching enabled (system blocks with cache_control)"
            );
        } else {
            body["messages"] = serde_json::to_value(messages).unwrap_or_default();
        }

        // Add tools if present
        if !tools.is_empty() {
            let tool_defs: Vec<Value> = tools
                .iter()
                .map(|t| {
                    let mut def = json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    });
                    // Cache tool definitions for Anthropic (they rarely change)
                    if is_anthropic {
                        def["cache_control"] = json!({ "type": "ephemeral" });
                    }
                    def
                })
                .collect();
            body["tools"] = Value::Array(tool_defs);
        }

        // Send request
        let url = format!("{}{}", self.base_url, self.chat_path);
        let req = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body);
        let req = self.apply_auth(req);

        let resp = req.send().await.map_err(|e| {
            BizClawError::Http(format!("{} connection failed ({}): {}", self.name, url, e))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();

            // Auto-retry WITHOUT tools if model doesn't support function calling
            // (e.g., tinyllama, phi, etc. on Ollama)
            if status.as_u16() == 400
                && !tools.is_empty()
                && (text.contains("does not support tools")
                    || text.contains("tool_use is not supported")
                    || text.contains("does not support function"))
            {
                tracing::warn!(
                    "⚠️ Model '{}' doesn't support tools — retrying without tools",
                    params.model
                );
                // Remove tools from body and retry
                body.as_object_mut().map(|m| m.remove("tools"));
                let retry_req = self
                    .client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .json(&body);
                let retry_req = self.apply_auth(retry_req);
                let retry_resp = retry_req.send().await.map_err(|e| {
                    BizClawError::Http(format!("{} retry failed: {}", self.name, e))
                })?;
                if !retry_resp.status().is_success() {
                    let rs = retry_resp.status();
                    let rt = retry_resp.text().await.unwrap_or_default();
                    return Err(BizClawError::Provider(format!(
                        "{} API error {} (retry without tools): {}",
                        self.name, rs, rt
                    )));
                }
                // Parse the retry response (same flow as below)
                let json: Value = retry_resp
                    .json()
                    .await
                    .map_err(|e| BizClawError::Http(e.to_string()))?;
                let choice = json["choices"]
                    .get(0)
                    .ok_or_else(|| BizClawError::Provider("No choices in retry response".into()))?;
                let content = choice["message"]["content"].as_str().map(String::from);
                let usage = json["usage"].as_object().map(|u| Usage {
                    prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    completion_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                });
                return Ok(ProviderResponse {
                    content,
                    tool_calls: vec![], // No tools available
                    finish_reason: choice["finish_reason"].as_str().map(String::from),
                    usage,
                });
            }

            return Err(BizClawError::Provider(format!(
                "{} API error {}: {}",
                self.name, status, text
            )));
        }

        // Parse response — standard OpenAI format
        let json: Value = resp
            .json()
            .await
            .map_err(|e| BizClawError::Http(e.to_string()))?;

        let choice = json["choices"]
            .get(0)
            .ok_or_else(|| BizClawError::Provider("No choices in response".into()))?;

        let content = choice["message"]["content"].as_str().map(String::from);

        let tool_calls = if let Some(tc) = choice["message"]["tool_calls"].as_array() {
            tc.iter()
                .filter_map(|t| {
                    Some(ToolCall {
                        id: t["id"].as_str().unwrap_or("").to_string(),
                        r#type: "function".to_string(),
                        function: FunctionCall {
                            name: t["function"]["name"].as_str()?.to_string(),
                            arguments: t["function"]["arguments"].as_str()?.to_string(),
                        },
                    })
                })
                .collect()
        } else {
            vec![]
        };

        let usage = json["usage"].as_object().map(|u| Usage {
            prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        });

        Ok(ProviderResponse {
            content,
            tool_calls,
            finish_reason: choice["finish_reason"].as_str().map(String::from),
            usage,
        })
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        // Try to fetch models from the API
        let url = format!("{}{}", self.base_url, self.models_path);
        let req = self.client.get(&url);
        let req = self.apply_auth(req);

        match req.send().await {
            Ok(r) if r.status().is_success() => {
                let json: Value = r.json().await.unwrap_or_default();
                let models: Vec<ModelInfo> = json["data"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|m| {
                                Some(ModelInfo {
                                    id: m["id"].as_str()?.to_string(),
                                    name: m["id"].as_str()?.to_string(),
                                    provider: self.name.clone(),
                                    context_length: 4096,
                                    max_output_tokens: Some(4096),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                if models.is_empty() {
                    Ok(self.default_models.clone())
                } else {
                    Ok(models)
                }
            }
            _ => Ok(self.default_models.clone()),
        }
    }

    async fn health_check(&self) -> Result<bool> {
        if self.auth_style != AuthStyle::None {
            // For cloud providers, just check if API key is set
            return Ok(!self.api_key.is_empty());
        }

        // For local servers (ollama, llamacpp), try to connect
        let url = format!("{}{}", self.base_url, self.models_path);
        let resp = self.client.get(&url).send().await;
        Ok(resp.is_ok())
    }
}
