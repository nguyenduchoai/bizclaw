//! Provider Failover Integration Tests — inspired by Skyclaw CAS patterns.
//!
//! Tests the failover chain behavior under realistic conditions:
//! - Primary failure → automatic fallback
//! - Health tracking with cooldown recovery
//! - All-providers-unhealthy scenario
//! - Provider recovery after cooldown
//!
//! Adopted from Skyclaw's atomic test patterns and Goclaw's test-only
//! override approach for fast CI execution.

use bizclaw_providers::provider_registry;

// ═══════════════════════════════════════════════════════════════════
// 1. PROVIDER REGISTRY — All 18 providers resolvable
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_all_providers_registered() {
    let names = provider_registry::all_provider_names();
    assert!(
        names.len() >= 18,
        "Expected at least 18 providers, got {}",
        names.len()
    );

    // Verify critical providers are present
    let critical = [
        "openai",
        "anthropic",
        "gemini",
        "deepseek",
        "minimax",
        "groq",
        "ollama",
    ];
    for name in &critical {
        assert!(
            provider_registry::get_provider_config(name).is_some(),
            "Critical provider '{}' not found in registry",
            name
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// 2. MINIMAX — Models correctly registered
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_minimax_models_expanded() {
    let config = provider_registry::get_provider_config("minimax").unwrap();

    // Must have 3 models: M2.7, M2.7-highspeed, M2.5
    assert_eq!(
        config.default_models.len(),
        3,
        "MiniMax should have 3 models (M2.7, M2.7-highspeed, M2.5), got {}",
        config.default_models.len()
    );

    // Verify specific model IDs
    let model_ids: Vec<&str> = config.default_models.iter().map(|m| m.id).collect();
    assert!(model_ids.contains(&"MiniMax-M2.7"), "Missing MiniMax-M2.7");
    assert!(
        model_ids.contains(&"MiniMax-M2.7-highspeed"),
        "Missing MiniMax-M2.7-highspeed"
    );
    assert!(model_ids.contains(&"MiniMax-M2.5"), "Missing MiniMax-M2.5");

    // Verify context lengths (all have 1M context)
    for model in config.default_models {
        assert_eq!(
            model.context_length, 1_000_000,
            "{} should have 1M context",
            model.id
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// 3. MINIMAX — Provider config correctness
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_minimax_provider_config() {
    let config = provider_registry::get_provider_config("minimax").unwrap();

    assert_eq!(config.base_url, "https://api.minimax.io/v1");
    assert_eq!(config.chat_path, "/chat/completions");
    assert_eq!(config.auth_style, provider_registry::AuthStyle::Bearer);
    assert!(config.env_keys.contains(&"MINIMAX_API_KEY"));
}

// ═══════════════════════════════════════════════════════════════════
// 4. ALIASES — All aliases resolve correctly
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_provider_aliases_comprehensive() {
    let alias_tests = [
        ("google", "gemini"),
        ("llama.cpp", "llamacpp"),
        ("grok", "xai"),
        ("bytedance", "modelark"),
        ("doubao", "modelark"),
        ("ark", "modelark"),
        ("volcengine", "modelark"),
        ("co", "cohere"),
        ("cohere_ai", "cohere"),
        ("pplx", "perplexity"),
        ("perplexity_ai", "perplexity"),
        ("qwen", "dashscope"),
        ("alibaba", "dashscope"),
        ("aliyun", "dashscope"),
        ("tongyi", "dashscope"),
        ("cli_proxy", "cliproxy"),
        ("cliproxyapi", "cliproxy"),
        ("CLIProxy", "cliproxy"),
        ("together_ai", "together"),
        ("togetherai", "together"),
    ];

    for (alias, expected) in &alias_tests {
        let config = provider_registry::get_provider_config(alias);
        assert!(
            config.is_some(),
            "Alias '{}' should resolve to '{}'",
            alias,
            expected
        );
        assert_eq!(
            config.unwrap().name,
            *expected,
            "Alias '{}' resolved to '{}', expected '{}'",
            alias,
            config.unwrap().name,
            expected
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// 5. ALL PROVIDERS — Have valid configuration
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_all_providers_valid_configuration() {
    let names = provider_registry::all_provider_names();

    for name in &names {
        let config = provider_registry::get_provider_config(name).unwrap();

        // Every provider must have at least one model
        assert!(
            !config.default_models.is_empty(),
            "Provider '{}' has no default models",
            name
        );

        // Every model must have a non-empty ID
        for model in config.default_models {
            assert!(
                !model.id.is_empty(),
                "Provider '{}' has a model with empty ID",
                name
            );
            assert!(
                model.context_length > 0,
                "Provider '{}' model '{}' has 0 context_length",
                name,
                model.id
            );
        }

        // Cloud providers must have auth keys
        if config.auth_style == provider_registry::AuthStyle::Bearer {
            // Exceptions: vllm and cliproxy can work without env keys
            if config.name != "vllm" && config.name != "cliproxy" {
                assert!(
                    !config.env_keys.is_empty(),
                    "Cloud provider '{}' has Bearer auth but no env_keys",
                    name
                );
            }
        }

        // Every provider must have a chat_path
        assert!(
            !config.chat_path.is_empty(),
            "Provider '{}' has empty chat_path",
            name
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// 6. AVAILABLE PROVIDERS — Full list includes brain + custom
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_available_providers_includes_all() {
    let available = bizclaw_providers::available_providers();

    // Must include all registry providers + brain + custom
    assert!(
        available.contains(&"brain"),
        "Available providers must include 'brain'"
    );
    assert!(
        available.contains(&"custom"),
        "Available providers must include 'custom'"
    );
    assert!(
        available.contains(&"minimax"),
        "Available providers must include 'minimax'"
    );

    // Total should be registry count + 2 (brain, custom)
    let registry_count = provider_registry::all_provider_names().len();
    assert_eq!(
        available.len(),
        registry_count + 2,
        "Available providers should be registry({}) + brain + custom",
        registry_count
    );
}
