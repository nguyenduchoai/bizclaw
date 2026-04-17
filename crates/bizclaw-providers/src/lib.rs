//! # BizClaw Providers
//!
//! LLM provider implementations for BizClaw.
//!
//! All OpenAI-compatible providers (OpenAI, Anthropic, DeepSeek, Gemini, Groq,
//! Ollama, LlamaCpp, OpenRouter) are handled by a single `OpenAiCompatibleProvider`.
//! The `BrainProvider` handles local GGUF models separately.

pub mod benchmark;
pub mod brain;
pub mod failover;
pub mod llm_tracing;
pub mod openai_compatible;
pub mod provider_registry;
pub mod router;
pub mod text_tool_calls;

use bizclaw_core::config::BizClawConfig;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Provider;

/// Create a provider from configuration.
///
/// Refactored for Brain Engine Expansion:
/// If Brain is enabled and has providers configured, returns a `BrainRouter`.
/// Otherwise, returns a single provider with optional failover.
pub fn create_provider(config: &BizClawConfig) -> Result<Box<dyn Provider>> {
    // ═══ NEW: Brain Engine Expansion Path ═══
    // If Brain Engine is enabled and has multiple providers or special routing, use BrainRouter.
    if config.brain.enabled
        && (!config.brain.providers.is_empty() || config.brain.model_path != "none")
    {
        tracing::info!("🧠 Initializing Unified Brain Engine (Modular Router)");

        let local_provider: Option<Box<dyn Provider>> = if config.brain.model_path != "none" {
            match brain::BrainProvider::new(config) {
                Ok(p) => Some(Box::new(p)),
                Err(e) => {
                    tracing::warn!("⚠️ Local Brain Engine failed to init: {e}");
                    None
                }
            }
        } else {
            None
        };

        let mut cloud_providers = Vec::new();
        for p_config in &config.brain.providers {
            match create_single_provider(&p_config.name, config) {
                Ok(p) => cloud_providers.push((p_config.clone(), p)),
                Err(e) => tracing::warn!(
                    "⚠️ Cloud provider '{}' in Brain config failed to init: {e}",
                    p_config.name
                ),
            }
        }

        if !cloud_providers.is_empty() || local_provider.is_some() {
            return Ok(Box::new(router::BrainRouter::new(
                config.brain.mode,
                config.brain.routing.clone(),
                cloud_providers,
                local_provider,
            )));
        }
    }

    // ═══ LEGACY / SIMPLE Path ═══
    // Prefer [LLM] section, fallback to legacy top-level field
    let provider_name = if !config.llm.provider.is_empty() {
        config.llm.provider.as_str()
    } else {
        config.default_provider.as_str()
    };

    let primary = create_single_provider(provider_name, config)?;

    // If fallback_provider is configured, wrap in FailoverProvider
    let fallback_name = &config.llm.fallback_provider;
    if !fallback_name.is_empty() && fallback_name != provider_name {
        match create_single_provider(fallback_name, config) {
            Ok(fallback) => {
                tracing::info!(
                    "🔄 Failover chain: {} → {} (CAS state machine active)",
                    provider_name,
                    fallback_name
                );
                return Ok(Box::new(failover::FailoverProvider::with_fallback(
                    primary, fallback,
                )));
            }
            Err(e) => {
                tracing::warn!(
                    "⚠️ Fallback provider '{}' failed to init: {} — running without failover",
                    fallback_name,
                    e
                );
            }
        }
    }

    Ok(primary)
}

/// Create a single provider instance by name (internal helper).
fn create_single_provider(name: &str, config: &BizClawConfig) -> Result<Box<dyn Provider>> {
    match name {
        // Local GGUF engine — not OpenAI-compatible
        "brain" => Ok(Box::new(brain::BrainProvider::new(config)?)),

        // Custom endpoint: "custom:https://my-server.com/v1"
        other if other.starts_with("custom:") => Ok(Box::new(
            openai_compatible::OpenAiCompatibleProvider::custom(other, config)?,
        )),

        // All known OpenAI-compatible providers
        _ => {
            let registry = provider_registry::get_provider_config(name)
                .ok_or_else(|| BizClawError::ProviderNotFound(name.into()))?;
            Ok(Box::new(
                openai_compatible::OpenAiCompatibleProvider::from_registry(registry, config)?,
            ))
        }
    }
}

/// List all available provider names.
pub fn available_providers() -> Vec<&'static str> {
    let mut names = provider_registry::all_provider_names();
    names.push("brain");
    names.push("custom");
    names
}
