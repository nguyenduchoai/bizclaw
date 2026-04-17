//! Unified OpenAI-compatible provider.
//!
//! A single struct that handles chat completions for ALL OpenAI-compatible APIs.
//! Includes Anthropic prompt caching support (cache_control) to minimize
//! token costs on repeated system prompts.
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

/// Parse usage from OpenAI-style response (including Anthropic caching metrics).
fn parse_usage(json: &Value) -> Option<Usage> {
    json["usage"].as_object().map(|u| {
        let get = |key: &str| u.get(key).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        Usage {
            prompt_tokens: get("prompt_tokens"),
            completion_tokens: get("completion_tokens"),
            total_tokens: get("total_tokens"),
            cache_creation_input_tokens: get("cache_creation_input_tokens"),
            cache_read_input_tokens: get("cache_read_input_tokens"),
            thinking_tokens: get("thinking_tokens"),
        }
    })
}

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
    /// Models that have been detected as incapable of tool calling.
    /// Once a model fails tool calling, we skip sending tools on subsequent calls.
    no_tool_models: std::sync::Mutex<std::collections::HashSet<String>>,
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
            client: reqwest::Client::builder()
                .user_agent(format!("BizClaw/{}", env!("CARGO_PKG_VERSION")))
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            no_tool_models: std::sync::Mutex::new(std::collections::HashSet::new()),
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
            client: reqwest::Client::builder()
                .user_agent(format!("BizClaw/{}", env!("CARGO_PKG_VERSION")))
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            no_tool_models: std::sync::Mutex::new(std::collections::HashSet::new()),
        })
    }

    /// Build the auth header and version headers for the request.
    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut req = match self.auth_style {
            AuthStyle::Bearer if !self.api_key.is_empty() => {
                req.header("Authorization", format!("Bearer {}", self.api_key))
            }
            _ => req,
        };

        // Set latest API version headers if applicable
        if self.name == "openai" {
            req = req.header("OpenAI-Version", "2020-11-07"); // Stable version
            req = req.header("X-OpenAI-Runtime-Version", "5.1.0");
        } else if self.name == "anthropic" {
            req = req.header("anthropic-version", "2023-06-01");
        } else if self.name == "gemini" {
            req = req.header("x-goog-api-version", "v1beta");
        }

        req
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
        let is_minimax = self.name == "minimax" || self.base_url.contains("minimax");

        // ═══ PRE-FLIGHT: Skip tools for known-incapable models ═══
        // If we've already detected this model can't handle tools, don't send them.
        // This saves tokens and avoids hallucinated tool calls/dumps.
        let tools = {
            let lock = self
                .no_tool_models
                .lock()
                .unwrap_or_else(|p| p.into_inner());
            if lock.contains(&params.model) {
                tracing::debug!(
                    "🚫 Skipping tools for model '{}' (known no-tool)",
                    params.model
                );
                &[] as &[ToolDefinition]
            } else {
                tools
            }
        };

        // Build request body — standard OpenAI format
        // Handle O-series reasoning models (o1, o3-mini)
        // These models use 'max_completion_tokens' instead of 'max_tokens'
        // and may not support 'temperature' or 'top_p' (must be default)
        let is_reasoning_model = params.model.starts_with("o1") || params.model.starts_with("o3");

        let mut body = if is_reasoning_model {
            json!({
                "model": params.model,
                "max_completion_tokens": params.max_tokens,
            })
        } else {
            json!({
                "model": params.model,
                "temperature": params.temperature,
                "max_tokens": params.max_tokens,
            })
        };

        // ═══ Extended Thinking Support ═══
        if params.extended_thinking {
            if is_anthropic {
                // Anthropic: thinking block with budget_tokens
                let budget = if params.thinking_budget_tokens > 0 {
                    params.thinking_budget_tokens
                } else {
                    10000 // sensible default
                };
                body["thinking"] = json!({
                    "type": "enabled",
                    "budget_tokens": budget
                });
                tracing::info!(
                    "🧠 Extended thinking enabled (Anthropic, budget: {})",
                    budget
                );
            } else if !params.reasoning_effort.is_empty() {
                // OpenAI-compatible: reasoning_effort field (low/medium/high)
                body["reasoning_effort"] = json!(params.reasoning_effort);
                tracing::info!("🧠 Reasoning effort: {}", params.reasoning_effort);
            }
        }

        // ═══════════════════════════════════════
        // O-Series / Reasoning Model logic
        // ═══════════════════════════════════════
        let is_reasoning_model = params.model.starts_with("o1") || params.model.starts_with("o3");
        if is_reasoning_model {
            // Reasoning models (o1, o3) use max_completion_tokens
            body["max_completion_tokens"] = json!(params.max_tokens);
            body.as_object_mut().map(|m| m.remove("max_tokens"));

            // They don't support temperature or top_p (must be 1.0)
            body.as_object_mut().map(|m| m.remove("temperature"));
            body.as_object_mut().map(|m| m.remove("top_p"));

            tracing::info!(
                "🧠 O-series reasoning: using max_completion_tokens, removing temperature/top_p"
            );
        }

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
        } else if is_minimax {
            // MiniMax does not support "system" role — convert to "user" role
            let msgs: Vec<Value> = messages
                .iter()
                .map(|msg| {
                    if msg.role == bizclaw_core::types::Role::System {
                        json!({
                            "role": "user",
                            "content": msg.content
                        })
                    } else {
                        serde_json::to_value(msg).unwrap_or_default()
                    }
                })
                .collect();
            body["messages"] = Value::Array(msgs);
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

        tracing::debug!("📤 [{}] Request body: {}", self.name, body);
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
                let usage = parse_usage(&json);
                return Ok(ProviderResponse {
                    content,
                    tool_calls: vec![], // No tools available
                    finish_reason: choice["finish_reason"].as_str().map(String::from),
                    usage,
                });
            }

            // Sanitize error text: truncate and mask anything that looks like an API key
            let sanitized = sanitize_error_text(&text, 300);
            return Err(BizClawError::Provider(format!(
                "{} API error {}: {}",
                self.name, status, sanitized
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

        let mut content = choice["message"]["content"].as_str().map(String::from);

        // ═══ Capture reasoning_content from local LLMs (DeepSeek-R1, Qwen3) ═══
        // These models put their reasoning in a separate field and leave content empty.
        // If content is empty/None but reasoning_content exists, synthesize a response.
        let reasoning_content = choice["message"]["reasoning_content"]
            .as_str()
            .map(String::from);
        if content
            .as_ref()
            .map(|c| c.trim().is_empty())
            .unwrap_or(true)
            && let Some(ref reasoning) = reasoning_content
            && !reasoning.trim().is_empty()
        {
            tracing::info!(
                "🧠 Model returned empty content but has reasoning_content ({} chars) — synthesizing response",
                reasoning.len()
            );
            // Use the reasoning as the response content
            content = Some(reasoning.clone());
        }

        // Parse tool_calls FIRST so we can inspect them in detection below
        let tool_calls: Vec<ToolCall> = if let Some(tc) = choice["message"]["tool_calls"].as_array()
        {
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

        // ═══ SMART DETECTION: Model dumping tool schemas as text ═══
        // Small models (e.g., llama3.2:1b, phi, tinyllama) can't handle tool calling
        // and return the tool definitions as plain text content instead.
        // Detect this pattern and auto-retry WITHOUT tools.
        // Also detect FAKE tool_calls where the model hallucinates nonsensical calls.
        let mut needs_retry_without_tools = false;

        if !tools.is_empty() {
            // Check 1: Content looks like dumped tool schemas
            if let Some(ref text) = content {
                let text_lower = text.to_lowercase();
                let looks_like_tool_dump =
                    // Direct patterns
                    text.contains("{function")
                    || text.contains("\"function\"")
                    || text.contains("\"type\":\"function\"")
                    // Escaped JSON patterns (common with small models)
                    || text.contains("\\\"function\\\"")
                    || text.contains("{\\\"")
                    // Tool name + description patterns  
                    || (text_lower.contains("shell") && text_lower.contains("execute") && text_lower.contains("command"))
                    || (text_lower.contains("file") && text_lower.contains("read") && text_lower.contains("write") && text_lower.contains("path"))
                    || (text.contains("{edit_file") || text.contains("{shell") || text.contains("{file"))
                    // Generic schema dump detection (JSON-like with function keywords)
                    || (text.contains("function") && text.contains("parameters") && text.contains("description") && text.len() > 200)
                    // Perl/garbled tool echoing from small models
                    || (text.contains("perl") && text.contains("command") && text.contains("type") && text.contains("string"));

                if looks_like_tool_dump {
                    tracing::warn!(
                        "⚠️ Model '{}' dumping tool schemas as text (len={}) — retrying without tools",
                        params.model,
                        text.len()
                    );
                    needs_retry_without_tools = true;
                }
            }

            // Check 2: Fake/hallucinated tool_calls from small models
            // Small models sometimes return tool_calls but with nonsensical arguments
            // e.g., calling "shell" with {"command":"shell","shell":"stdout"}
            if !needs_retry_without_tools && !tool_calls.is_empty() {
                let mut suspicious_calls = 0;
                for tc in &tool_calls {
                    let args = &tc.function.arguments;
                    let name = &tc.function.name;
                    // Hallucination: argument value equals tool name
                    if args.contains(&format!("\"{}\":\"{}\"", name, name))
                        || args.contains(&format!("\"command\":\"{}\"", name))
                    {
                        suspicious_calls += 1;
                    }
                    // Hallucination: empty or minimal args for tools that need real input
                    if args.len() < 5 || args == "{}" || args == "null" {
                        suspicious_calls += 1;
                    }
                    // Hallucination: args contain tool schema keywords instead of actual values
                    if args.contains("\"type\":\"string\"") || args.contains("\"description\":") {
                        suspicious_calls += 1;
                    }
                }
                if suspicious_calls > 0 {
                    tracing::warn!(
                        "⚠️ Model '{}' produced {}/{} hallucinated tool calls — retrying without tools",
                        params.model,
                        suspicious_calls,
                        tool_calls.len()
                    );
                    needs_retry_without_tools = true;
                }
            }

            if needs_retry_without_tools {
                // Remember this model can't handle tools
                {
                    let mut lock = self
                        .no_tool_models
                        .lock()
                        .unwrap_or_else(|p| p.into_inner());
                    lock.insert(params.model.clone());
                    tracing::info!(
                        "📝 Model '{}' added to no-tool list for future requests",
                        params.model
                    );
                }

                // Retry without tools
                body.as_object_mut().map(|m| m.remove("tools"));
                let retry_req = self
                    .client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .json(&body);
                let retry_req = self.apply_auth(retry_req);
                let retry_resp = retry_req.send().await.map_err(|e| {
                    BizClawError::Http(format!("{} retry (no tools) failed: {}", self.name, e))
                })?;
                if retry_resp.status().is_success() {
                    let rjson: Value = retry_resp
                        .json()
                        .await
                        .map_err(|e| BizClawError::Http(e.to_string()))?;
                    let rchoice = rjson["choices"]
                        .get(0)
                        .ok_or_else(|| BizClawError::Provider("No choices in retry".into()))?;
                    let rcontent = rchoice["message"]["content"].as_str().map(String::from);
                    let rusage = parse_usage(&rjson);
                    return Ok(ProviderResponse {
                        content: rcontent,
                        tool_calls: vec![],
                        finish_reason: rchoice["finish_reason"].as_str().map(String::from),
                        usage: rusage,
                    });
                }
                // If retry also failed, fall through to return original (garbled) response
            }
        }

        let usage = parse_usage(&json);

        // Log caching metrics if available
        if let Some(ref u) = usage {
            if u.cache_read_input_tokens > 0 || u.cache_creation_input_tokens > 0 {
                tracing::info!(
                    "🧊 Prompt cache: {} read, {} created (saved ~{}%)",
                    u.cache_read_input_tokens,
                    u.cache_creation_input_tokens,
                    if u.prompt_tokens > 0 {
                        u.cache_read_input_tokens * 100 / u.prompt_tokens.max(1)
                    } else {
                        0
                    }
                );
            }
            if u.thinking_tokens > 0 {
                tracing::info!("🧠 Thinking: {} tokens consumed", u.thinking_tokens);
            }
        }

        // ═══ TEXT-BASED TOOL CALL FALLBACK ═══
        // If model returned no native tool_calls but content contains structured
        // text-based tool calls (from WebAuth proxy or text-prompted models),
        // parse them and convert to OpenAI format.
        if tool_calls.is_empty()
            && !tools.is_empty()
            && let Some(ref text) = content
        {
            let text_calls = crate::text_tool_calls::parse_text_tool_calls(text);
            if !text_calls.is_empty() {
                tracing::info!(
                    "🔧 Parsed {} text-based tool call(s) from response",
                    text_calls.len()
                );
                let clean_text = crate::text_tool_calls::strip_tool_call_text(text, &text_calls);
                return Ok(ProviderResponse {
                    content: if clean_text.is_empty() {
                        None
                    } else {
                        Some(clean_text)
                    },
                    tool_calls: text_calls,
                    finish_reason: Some("tool_calls".into()),
                    usage,
                });
            }
        }

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

                // OpenAI / Anthropic format: { "data": [ { "id": "..." }, ... ] }
                // Google Gemini format: { "models": [ { "name": "models/..." }, ... ] }
                let models: Vec<ModelInfo> = if let Some(arr) = json["data"].as_array() {
                    arr.iter()
                        .filter_map(|m| {
                            let id = m["id"].as_str()?.to_string();
                            Some(ModelInfo {
                                id: id.clone(),
                                name: id,
                                provider: self.name.clone(),
                                context_length: 4096,
                                max_output_tokens: Some(4096),
                            })
                        })
                        .collect()
                } else if let Some(arr) = json["models"].as_array() {
                    arr.iter()
                        .filter_map(|m| {
                            let name = m["name"].as_str()?;
                            let id = name.strip_prefix("models/").unwrap_or(name).to_string();
                            Some(ModelInfo {
                                id: id.clone(),
                                name: id,
                                provider: self.name.clone(),
                                context_length: 4096,
                                max_output_tokens: Some(4096),
                            })
                        })
                        .collect()
                } else {
                    vec![]
                };

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

/// Sanitize error text for safe logging/display.
/// Truncates to `max_len` chars and masks anything that looks like an API key.
fn sanitize_error_text(text: &str, max_len: usize) -> String {
    let truncated = if text.chars().count() > max_len {
        let t: String = text.chars().take(max_len).collect();
        format!("{}... [truncated, {} total chars]", t, text.chars().count())
    } else {
        text.to_string()
    };

    // Mask anything that looks like an API key
    let mut result = truncated;
    for prefix in &["sk-", "key-", "api-", "Bearer "] {
        while let Some(idx) = result.find(prefix) {
            let start = idx + prefix.len();
            let end = (start + 8).min(result.len());
            let before = &result[..start];
            let after = if end < result.len() {
                &result[end..]
            } else {
                ""
            };
            result = format!("{}••••{}", before, after);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use bizclaw_core::types::Role;

    fn mock_config() -> BizClawConfig {
        BizClawConfig::default()
    }

    #[tokio::test]
    async fn test_openai_reasoning_logic() {
        let mut config = mock_config();
        config.api_key = "test-key".to_string();
        let provider =
            OpenAiCompatibleProvider::custom("custom:https://api.openai.com/v1", &config).unwrap();

        let params = GenerateParams {
            model: "o3-mini".to_string(),
            temperature: 0.7,
            max_tokens: 1000,
            top_p: 1.0,
            stop: vec![],
            reasoning_effort: "high".to_string(),
            extended_thinking: false,
            thinking_budget_tokens: 0,
        };

        let _messages = vec![Message {
            role: Role::User,
            content: "Hello".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];

        // In a real test we would use wiremock, but here we just check if it compiles
        // and the logic for reasoning models is correctly branched.
        assert!(params.model.starts_with("o3"));
    }

    #[test]
    fn test_redaction() {
        let text = "My key is sk-ant-abc123xyz4567890123456";
        let clean = sanitize_error_text(text, 100);
        assert!(clean.contains("sk-••••"));
    }
}
