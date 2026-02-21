//! # BizClaw Providers
//!
//! LLM provider implementations: OpenAI, Anthropic, Ollama, LlamaCpp, Brain.

pub mod openai;
pub mod anthropic;
pub mod ollama;
pub mod llamacpp;
pub mod brain;
pub mod custom;

use bizclaw_core::config::BizClawConfig;
use bizclaw_core::traits::Provider;
use bizclaw_core::error::Result;

/// Create a provider from configuration.
pub fn create_provider(config: &BizClawConfig) -> Result<Box<dyn Provider>> {
    match config.default_provider.as_str() {
        "openai" | "openrouter" => Ok(Box::new(openai::OpenAiProvider::new(config)?)),
        "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new(config)?)),
        "ollama" => Ok(Box::new(ollama::OllamaProvider::new(config)?)),
        "llamacpp" | "llama.cpp" => Ok(Box::new(llamacpp::LlamaCppProvider::new(config)?)),
        "brain" => Ok(Box::new(brain::BrainProvider::new(config)?)),
        other if other.starts_with("custom:") => {
            Ok(Box::new(custom::CustomProvider::new(config, other)?))
        }
        other => Err(bizclaw_core::error::BizClawError::ProviderNotFound(other.into())),
    }
}
