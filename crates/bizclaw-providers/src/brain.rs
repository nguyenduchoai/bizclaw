use async_trait::async_trait;
use bizclaw_core::config::BizClawConfig;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::provider::{GenerateParams, Provider};
use bizclaw_core::types::{Message, ModelInfo, ProviderResponse, Role, ToolDefinition};
use tokio::sync::Mutex;

pub struct BrainProvider {
    engine: Mutex<bizclaw_brain::BrainEngine>,
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

        let mut engine = bizclaw_brain::BrainEngine::new(brain_config);

        // Try to load model from configured path
        let model_dir = BizClawConfig::home_dir().join("models");
        let model_path = if !config.brain.model_path.is_empty() {
            std::path::PathBuf::from(&config.brain.model_path)
        } else {
            // Auto-detect: find first .gguf file in models directory
            find_gguf_model(&model_dir).unwrap_or_else(|| model_dir.join("model.gguf"))
        };

        if model_path.exists() {
            match engine.load_model(&model_path) {
                Ok(()) => {
                    tracing::info!("Brain provider: model loaded from {}", model_path.display())
                }
                Err(e) => tracing::warn!("Brain provider: failed to load model: {e}"),
            }
        } else {
            tracing::info!(
                "Brain provider: no model found at {}. Use `bizclaw brain download` to get a model.",
                model_path.display()
            );
        }

        Ok(Self {
            engine: Mutex::new(engine),
        })
    }
}

/// Find the first .gguf file in a directory.
fn find_gguf_model(dir: &std::path::Path) -> Option<std::path::PathBuf> {
    if !dir.exists() {
        return None;
    }
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_str()? == "gguf" {
                Some(path)
            } else {
                None
            }
        })
        .next()
}

#[async_trait]
impl Provider for BrainProvider {
    fn name(&self) -> &str {
        "brain"
    }

    async fn chat(
        &self,
        messages: &[Message],
        _tools: &[ToolDefinition],
        params: &GenerateParams,
    ) -> Result<ProviderResponse> {
        if !self.engine.lock().await.is_loaded() {
            return Err(BizClawError::Brain(
                "No model loaded. Place a .gguf file in ~/.bizclaw/models/ or set brain.model_path in config.".into()
            ));
        }

        // Format messages into a chat prompt (Llama-style)
        let prompt = format_chat_prompt(messages);

        let max_tokens = if params.max_tokens > 0 {
            params.max_tokens
        } else {
            256
        };

        let response = self.engine.lock().await.generate(&prompt, max_tokens)?;
        Ok(ProviderResponse::text(response))
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let mut models = vec![];

        if let Some(info) = self.engine.lock().await.model_info() {
            models.push(ModelInfo {
                id: "local-model".into(),
                name: info,
                provider: "brain".into(),
                context_length: 2048,
                max_output_tokens: Some(256),
            });
        }

        // List available models in ~/.bizclaw/models/
        let model_dir = BizClawConfig::home_dir().join("models");
        if model_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&model_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("gguf") {
                        let name = path
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let size_mb = std::fs::metadata(&path)
                            .map(|m| m.len() / 1024 / 1024)
                            .unwrap_or(0);

                        if !models.iter().any(|m: &ModelInfo| m.id == name) {
                            models.push(ModelInfo {
                                id: name.clone(),
                                name: format!("{} ({}MB)", name, size_mb),
                                provider: "brain".into(),
                                context_length: 2048,
                                max_output_tokens: Some(256),
                            });
                        }
                    }
                }
            }
        }

        if models.is_empty() {
            models.push(ModelInfo {
                id: "none".into(),
                name: "No models installed — use `bizclaw brain download`".into(),
                provider: "brain".into(),
                context_length: 0,
                max_output_tokens: None,
            });
        }

        Ok(models)
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.engine.lock().await.is_loaded())
    }
}

/// Format messages into a LLaMA-style chat prompt.
fn format_chat_prompt(messages: &[Message]) -> String {
    let mut prompt = String::new();

    for msg in messages {
        match msg.role {
            Role::System => {
                prompt.push_str(&format!("[INST] <<SYS>>\n{}\n<</SYS>>\n\n", msg.content));
            }
            Role::User => {
                prompt.push_str(&format!("{} [/INST]", msg.content));
            }
            Role::Assistant => {
                prompt.push_str(&format!(" {} </s><s>[INST] ", msg.content));
            }
            Role::Tool => {
                prompt.push_str(&format!("Tool result: {} [/INST]", msg.content));
            }
        }
    }

    prompt
}
