//! Provider registry — maps provider names to endpoint configurations.
//!
//! All OpenAI-compatible providers are defined here as static config entries.
//! The unified `OpenAiCompatibleProvider` uses these configs to connect to any provider.

use bizclaw_core::types::ModelInfo;

/// How to attach auth credentials to requests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthStyle {
    /// `Authorization: Bearer <key>`
    Bearer,
    /// No authentication required (local servers).
    None,
}

/// Static model definition for a provider.
#[derive(Debug, Clone)]
pub struct ModelDef {
    pub id: &'static str,
    pub name: &'static str,
    pub context_length: u32,
    pub max_output_tokens: Option<u32>,
}

impl ModelDef {
    pub fn to_model_info(&self, provider: &str) -> ModelInfo {
        ModelInfo {
            id: self.id.into(),
            name: self.name.into(),
            provider: provider.into(),
            context_length: self.context_length,
            max_output_tokens: self.max_output_tokens,
        }
    }
}

/// Configuration for a single provider.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Provider identifier.
    pub name: &'static str,
    /// Base URL for the API.
    pub base_url: &'static str,
    /// Path for chat completions endpoint (appended to base_url).
    pub chat_path: &'static str,
    /// Path for listing models (appended to base_url).
    pub models_path: &'static str,
    /// Environment variable names to try for the API key (in order).
    pub env_keys: &'static [&'static str],
    /// How to send auth credentials.
    pub auth_style: AuthStyle,
    /// Environment variable to override the base URL (e.g., OLLAMA_HOST).
    pub base_url_env: Option<&'static str>,
    /// Default models to return from `list_models`.
    pub default_models: &'static [ModelDef],
}

// ─── Provider Definitions ────────────────────────────────────────────────────

static OPENAI_MODELS: &[ModelDef] = &[
    ModelDef {
        id: "gpt-4o",
        name: "GPT-4o",
        context_length: 128000,
        max_output_tokens: Some(4096),
    },
    ModelDef {
        id: "gpt-4o-mini",
        name: "GPT-4o Mini",
        context_length: 128000,
        max_output_tokens: Some(4096),
    },
];

static OPENROUTER_MODELS: &[ModelDef] = &[
    ModelDef {
        id: "openai/gpt-4o",
        name: "GPT-4o (OpenRouter)",
        context_length: 128000,
        max_output_tokens: Some(4096),
    },
    ModelDef {
        id: "anthropic/claude-sonnet-4-20250514",
        name: "Claude Sonnet 4 (OpenRouter)",
        context_length: 200000,
        max_output_tokens: Some(8192),
    },
];

static ANTHROPIC_MODELS: &[ModelDef] = &[
    ModelDef {
        id: "claude-sonnet-4-20250514",
        name: "Claude Sonnet 4",
        context_length: 200000,
        max_output_tokens: Some(8192),
    },
    ModelDef {
        id: "claude-3-5-haiku-20241022",
        name: "Claude 3.5 Haiku",
        context_length: 200000,
        max_output_tokens: Some(8192),
    },
    ModelDef {
        id: "claude-3-5-sonnet-20241022",
        name: "Claude 3.5 Sonnet",
        context_length: 200000,
        max_output_tokens: Some(8192),
    },
];

static DEEPSEEK_MODELS: &[ModelDef] = &[
    ModelDef {
        id: "deepseek-chat",
        name: "DeepSeek Chat",
        context_length: 128000,
        max_output_tokens: Some(8192),
    },
    ModelDef {
        id: "deepseek-reasoner",
        name: "DeepSeek R1",
        context_length: 64000,
        max_output_tokens: Some(8192),
    },
];

static GEMINI_MODELS: &[ModelDef] = &[
    ModelDef {
        id: "gemini-2.5-pro",
        name: "Gemini 2.5 Pro",
        context_length: 1048576,
        max_output_tokens: Some(65536),
    },
    ModelDef {
        id: "gemini-2.5-flash",
        name: "Gemini 2.5 Flash",
        context_length: 1048576,
        max_output_tokens: Some(65536),
    },
];

static GROQ_MODELS: &[ModelDef] = &[
    ModelDef {
        id: "llama-3.3-70b-versatile",
        name: "Llama 3.3 70B",
        context_length: 128000,
        max_output_tokens: Some(32768),
    },
    ModelDef {
        id: "llama-3.1-8b-instant",
        name: "Llama 3.1 8B",
        context_length: 128000,
        max_output_tokens: Some(8192),
    },
    ModelDef {
        id: "mixtral-8x7b-32768",
        name: "Mixtral 8x7B",
        context_length: 32768,
        max_output_tokens: Some(8192),
    },
];

static OLLAMA_MODELS: &[ModelDef] = &[ModelDef {
    id: "llama3.2",
    name: "Llama 3.2 (Ollama)",
    context_length: 4096,
    max_output_tokens: Some(4096),
}];

static LLAMACPP_MODELS: &[ModelDef] = &[ModelDef {
    id: "local-model",
    name: "Local llama.cpp Model",
    context_length: 4096,
    max_output_tokens: Some(4096),
}];

// ─── Registry ────────────────────────────────────────────────────────────────

/// All known providers.
static PROVIDERS: &[ProviderConfig] = &[
    ProviderConfig {
        name: "openai",
        base_url: "https://api.openai.com/v1",
        chat_path: "/chat/completions",
        models_path: "/models",
        env_keys: &["OPENAI_API_KEY"],
        auth_style: AuthStyle::Bearer,
        base_url_env: Some("OPENAI_API_BASE"),
        default_models: OPENAI_MODELS,
    },
    ProviderConfig {
        name: "openrouter",
        base_url: "https://openrouter.ai/api/v1",
        chat_path: "/chat/completions",
        models_path: "/models",
        env_keys: &["OPENROUTER_API_KEY", "OPENAI_API_KEY"],
        auth_style: AuthStyle::Bearer,
        base_url_env: None,
        default_models: OPENROUTER_MODELS,
    },
    ProviderConfig {
        name: "anthropic",
        base_url: "https://api.anthropic.com/v1",
        chat_path: "/chat/completions",
        models_path: "/models",
        env_keys: &["ANTHROPIC_API_KEY"],
        auth_style: AuthStyle::Bearer,
        base_url_env: None,
        default_models: ANTHROPIC_MODELS,
    },
    ProviderConfig {
        name: "deepseek",
        base_url: "https://api.deepseek.com",
        chat_path: "/chat/completions",
        models_path: "/models",
        env_keys: &["DEEPSEEK_API_KEY"],
        auth_style: AuthStyle::Bearer,
        base_url_env: None,
        default_models: DEEPSEEK_MODELS,
    },
    ProviderConfig {
        name: "gemini",
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        chat_path: "/chat/completions",
        models_path: "/models",
        env_keys: &["GEMINI_API_KEY", "GOOGLE_API_KEY"],
        auth_style: AuthStyle::Bearer,
        base_url_env: None,
        default_models: GEMINI_MODELS,
    },
    ProviderConfig {
        name: "groq",
        base_url: "https://api.groq.com/openai/v1",
        chat_path: "/chat/completions",
        models_path: "/models",
        env_keys: &["GROQ_API_KEY"],
        auth_style: AuthStyle::Bearer,
        base_url_env: None,
        default_models: GROQ_MODELS,
    },
    ProviderConfig {
        name: "ollama",
        base_url: "http://localhost:11434/v1",
        chat_path: "/chat/completions",
        models_path: "/models",
        env_keys: &[],
        auth_style: AuthStyle::None,
        base_url_env: Some("OLLAMA_HOST"),
        default_models: OLLAMA_MODELS,
    },
    ProviderConfig {
        name: "llamacpp",
        base_url: "http://localhost:8080/v1",
        chat_path: "/chat/completions",
        models_path: "/models",
        env_keys: &[],
        auth_style: AuthStyle::None,
        base_url_env: Some("LLAMACPP_HOST"),
        default_models: LLAMACPP_MODELS,
    },
    ProviderConfig {
        name: "cliproxy",
        base_url: "http://localhost:8888/v1",
        chat_path: "/chat/completions",
        models_path: "/models",
        env_keys: &["CLIPROXY_API_KEY"],
        auth_style: AuthStyle::Bearer,
        base_url_env: Some("CLIPROXY_HOST"),
        default_models: &[ModelDef {
            id: "default",
            name: "CLIProxy Model",
            context_length: 128000,
            max_output_tokens: Some(4096),
        }],
    },
    ProviderConfig {
        name: "vllm",
        base_url: "http://localhost:8000/v1",
        chat_path: "/chat/completions",
        models_path: "/models",
        env_keys: &["VLLM_API_KEY"],
        auth_style: AuthStyle::None,
        base_url_env: Some("VLLM_HOST"),
        default_models: &[ModelDef {
            id: "default",
            name: "vLLM Model",
            context_length: 32768,
            max_output_tokens: Some(4096),
        }],
    },
    ProviderConfig {
        name: "together",
        base_url: "https://api.together.xyz/v1",
        chat_path: "/chat/completions",
        models_path: "/models",
        env_keys: &["TOGETHER_API_KEY"],
        auth_style: AuthStyle::Bearer,
        base_url_env: None,
        default_models: &[ModelDef {
            id: "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            name: "Llama 3.3 70B (Together)",
            context_length: 128000,
            max_output_tokens: Some(4096),
        }],
    },
];

/// Look up a provider config by name.
pub fn get_provider_config(name: &str) -> Option<&'static ProviderConfig> {
    // Also match aliases
    let lookup = match name {
        "google" => "gemini",
        "llama.cpp" => "llamacpp",
        "cli_proxy" | "cliproxyapi" | "CLIProxy" => "cliproxy",
        "together_ai" | "togetherai" => "together",
        other => other,
    };
    PROVIDERS.iter().find(|p| p.name == lookup)
}

/// List all known provider names.
pub fn all_provider_names() -> Vec<&'static str> {
    PROVIDERS.iter().map(|p| p.name).collect()
}
