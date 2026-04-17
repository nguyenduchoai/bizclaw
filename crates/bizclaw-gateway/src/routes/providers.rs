//! Provider management API route handlers.
//!
//! Extracted from api_webhooks.rs to reduce file size.
//! Handles: list_providers, create_provider, delete_provider,
//!          update_provider, fetch_provider_models, list_channels,
//!          ollama_models, brain_scan_models.

use super::helpers::internal_error;
use crate::server::AppState;
use axum::{Json, extract::State};
use std::sync::Arc;

/// List available providers (from DB) — fully self-describing, no hardcoded metadata.
pub async fn list_providers(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
    let active = cfg.default_provider.clone();
    drop(cfg);

    match state.db.list_providers(&active) {
        Ok(providers) => {
            let provider_json: Vec<serde_json::Value> = providers
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "name": p.name,
                        "label": p.label,
                        "icon": p.icon,
                        "type": p.provider_type,
                        "status": if p.is_active { "active" } else { "available" },
                        "models": p.models,
                        "api_key_set": !p.api_key.is_empty(),
                        "base_url": p.base_url,
                        "chat_path": p.chat_path,
                        "models_path": p.models_path,
                        "auth_style": p.auth_style,
                        "env_keys": p.env_keys,
                        "enabled": p.enabled,
                    })
                })
                .collect();
            Json(serde_json::json!({ "providers": provider_json }))
        }
        Err(e) => internal_error("list_providers", e),
    }
}

/// Create or update a provider — accepts all self-describing fields.
pub async fn create_provider(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("").trim();
    if name.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Provider name is required"}));
    }
    let label = body["label"].as_str().unwrap_or(name);
    let icon = body["icon"].as_str().unwrap_or("🤖");
    let provider_type = body["type"].as_str().unwrap_or("cloud");
    let api_key = body["api_key"].as_str().unwrap_or("");
    let base_url = body["base_url"].as_str().unwrap_or("");
    let chat_path = body["chat_path"].as_str().unwrap_or("/chat/completions");
    let models_path = body["models_path"].as_str().unwrap_or("/models");
    let auth_style = body["auth_style"].as_str().unwrap_or("bearer");
    let env_keys: Vec<String> = body["env_keys"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let models: Vec<String> = body["models"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    match state.db.upsert_provider(
        name,
        label,
        icon,
        provider_type,
        api_key,
        base_url,
        chat_path,
        models_path,
        auth_style,
        &env_keys,
        &models,
    ) {
        Ok(p) => Json(serde_json::json!({
            "ok": true,
            "provider": {
                "name": p.name, "label": p.label, "icon": p.icon,
                "type": p.provider_type, "base_url": p.base_url, "models": p.models
            },
        })),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

/// Delete a provider.
pub async fn delete_provider(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    match state.db.delete_provider(&name) {
        Ok(()) => {
            Json(serde_json::json!({"ok": true, "message": format!("Provider '{}' deleted", name)}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

/// Update provider config (API key, base URL).
pub async fn update_provider(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let api_key = body["api_key"].as_str();
    let base_url = body["base_url"].as_str();

    match state.db.update_provider_config(&name, api_key, base_url) {
        Ok(()) => {
            Json(serde_json::json!({"ok": true, "message": format!("Provider '{}' updated", name)}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
    }
}

/// Fetch live models from a provider's API endpoint.
/// This calls the actual provider API (e.g., OpenAI /models, Ollama /api/tags)
/// and caches the result in DB.
pub async fn fetch_provider_models(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    // Get provider from DB
    let provider = match state.db.get_provider(&name) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("[security] Provider lookup failed for '{}': {e}", name);
            return Json(
                serde_json::json!({"ok": false, "error": "Provider not found or unavailable"}),
            );
        }
    };

    // Special case: Ollama uses /api/tags not /v1/models
    if name == "ollama" {
        let ollama_base = provider.base_url.replace("/v1", "");
        let url = format!("{}/api/tags", ollama_base.trim_end_matches('/'));
        match reqwest::Client::new()
            .get(&url)
            .timeout(std::time::Duration::from_secs(8))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<serde_json::Value>().await {
                    let models: Vec<String> = body["models"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|m| m["name"].as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    // Cache in DB
                    if let Err(e) = state.db.update_provider_models(&name, &models) {
                        tracing::warn!("Failed to cache models for provider '{name}': {e}");
                    }
                    return Json(serde_json::json!({
                        "ok": true,
                        "provider": name,
                        "models": models,
                        "source": "live_api",
                    }));
                }
            }
            Ok(resp) => {
                let status = resp.status();
                return Json(serde_json::json!({
                    "ok": false,
                    "error": format!("Ollama returned HTTP {status}"),
                    "models": provider.models,
                    "source": "cached",
                }));
            }
            Err(e) => {
                return Json(serde_json::json!({
                    "ok": false,
                    "error": format!("Ollama not reachable: {e}"),
                    "models": provider.models,
                    "source": "cached",
                }));
            }
        }
    }

    // Special case: Brain — scan filesystem for GGUF files
    if name == "brain" {
        let config_dir = state
            .config_path
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let scan_dirs = vec![
            config_dir.join("models"),
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join(".bizclaw")
                .join("models"),
        ];
        let mut models = Vec::new();
        for dir in &scan_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(ext) = path.extension()
                        && (ext == "gguf" || ext == "bin")
                        && let Some(name) = path.file_name()
                    {
                        models.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
        if !models.is_empty()
            && let Err(e) = state.db.update_provider_models("brain", &models)
        {
            tracing::warn!("Failed to cache brain models in DB: {e}");
        }
        return Json(serde_json::json!({
            "ok": true,
            "provider": "brain",
            "models": models,
            "source": "filesystem",
        }));
    }

    // Generic OpenAI-compatible provider — call /models endpoint
    if provider.base_url.is_empty() || provider.models_path.is_empty() {
        return Json(serde_json::json!({
            "ok": false,
            "error": "Provider has no base_url or models_path configured",
            "models": provider.models,
            "source": "cached",
        }));
    }

    let url = format!(
        "{}{}",
        provider.base_url.trim_end_matches('/'),
        provider.models_path
    );
    let client = reqwest::Client::new();

    // Apply auth — detect API key from provider config or env vars
    let api_key = if !provider.api_key.is_empty() {
        provider.api_key.clone()
    } else {
        // Try env vars
        provider
            .env_keys
            .iter()
            .find_map(|key| std::env::var(key).ok())
            .unwrap_or_default()
    };

    // Build request with provider-specific auth handling
    let req = if name == "anthropic" {
        // Anthropic uses x-api-key header (not Bearer)
        let mut r = client.get(&url).timeout(std::time::Duration::from_secs(10));
        if !api_key.is_empty() {
            r = r
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01");
        }
        r
    } else if name == "gemini" {
        // Gemini uses ?key= query param
        let full_url = if !api_key.is_empty() {
            format!("{}?key={}", url, api_key)
        } else {
            url.clone()
        };
        client
            .get(&full_url)
            .timeout(std::time::Duration::from_secs(10))
    } else {
        let mut r = client.get(&url).timeout(std::time::Duration::from_secs(10));
        if provider.auth_style == "bearer" && !api_key.is_empty() {
            r = r.header("Authorization", format!("Bearer {}", api_key));
        }
        r
    };

    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                let models: Vec<String> = if let Some(arr) = body["data"].as_array() {
                    // OpenAI / Anthropic format
                    arr.iter()
                        .filter_map(|m| m["id"].as_str().map(String::from))
                        .collect()
                } else if let Some(arr) = body["models"].as_array() {
                    // Gemini / Google format
                    arr.iter()
                        .filter_map(|m| {
                            let name = m["name"].as_str()?;
                            Some(name.strip_prefix("models/").unwrap_or(name).to_string())
                        })
                        .collect()
                } else {
                    vec![]
                };
                if !models.is_empty() {
                    // Cache in DB
                    if let Err(e) = state.db.update_provider_models(&name, &models) {
                        tracing::warn!("Failed to cache models for provider '{name}': {e}");
                    }
                }
                let is_live = !models.is_empty();
                let result_models = if is_live { models } else { provider.models };
                Json(serde_json::json!({
                    "ok": true,
                    "provider": name,
                    "models": result_models,
                    "source": if is_live { "live_api" } else { "cached" },
                }))
            } else {
                Json(serde_json::json!({
                    "ok": false,
                    "error": "Failed to parse models response",
                    "models": provider.models,
                    "source": "cached",
                }))
            }
        }
        Ok(resp) => {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            Json(serde_json::json!({
                "ok": false,
                "error": format!("API returned HTTP {status}: {}", text.chars().take(200).collect::<String>()),
                "models": provider.models,
                "source": "cached",
            }))
        }
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": format!("Connection failed: {e}"),
            "models": provider.models,
            "source": "cached",
        })),
    }
}

/// List available channels with config status.
pub async fn list_channels(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
    Json(serde_json::json!({
        "channels": [
            {"name": "cli", "type": "interactive", "status": "active", "configured": true},
            {"name": "telegram", "type": "messaging", "status": if cfg.channel.telegram.iter().any(|t| t.enabled) { "active" } else { "disabled" }, "configured": !cfg.channel.telegram.is_empty(), "count": cfg.channel.telegram.len()},
            {"name": "zalo", "type": "messaging", "status": if cfg.channel.zalo.iter().any(|z| z.enabled) { "active" } else { "disabled" }, "configured": !cfg.channel.zalo.is_empty(), "count": cfg.channel.zalo.len()},
            {"name": "discord", "type": "messaging", "status": if cfg.channel.discord.iter().any(|d| d.enabled) { "active" } else { "disabled" }, "configured": !cfg.channel.discord.is_empty(), "count": cfg.channel.discord.len()},
            {"name": "email", "type": "messaging", "status": if cfg.channel.email.iter().any(|e| e.enabled) { "active" } else { "disabled" }, "configured": !cfg.channel.email.is_empty(), "count": cfg.channel.email.len()},
            {"name": "webhook", "type": "api", "status": if cfg.channel.webhook.iter().any(|wh| wh.enabled) { "active" } else { "disabled" }, "configured": !cfg.channel.webhook.is_empty(), "count": cfg.channel.webhook.len()},
            {"name": "whatsapp", "type": "messaging", "status": if cfg.channel.whatsapp.iter().any(|w| w.enabled) { "active" } else { "disabled" }, "configured": !cfg.channel.whatsapp.is_empty(), "count": cfg.channel.whatsapp.len()},
        ]
    }))
}

/// List installed Ollama models.
pub async fn ollama_models() -> Json<serde_json::Value> {
    let url = "http://localhost:11434/api/tags";
    match reqwest::Client::new()
        .get(url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                let models: Vec<serde_json::Value> = body
                    .get("models")
                    .and_then(|m| m.as_array())
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|m| {
                        let name = m
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let size_bytes = m.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
                        let size = if size_bytes > 1_000_000_000 {
                            format!("{:.1} GB", size_bytes as f64 / 1e9)
                        } else {
                            format!("{} MB", size_bytes / 1_000_000)
                        };
                        let family = m
                            .get("details")
                            .and_then(|d| d.get("family"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        serde_json::json!({"name": name, "size": size, "family": family})
                    })
                    .collect();
                Json(serde_json::json!({"ok": true, "models": models}))
            } else {
                Json(serde_json::json!({"ok": true, "models": []}))
            }
        }
        Err(e) => {
            tracing::warn!("[security] Ollama connection failed: {e}");
            Json(
                serde_json::json!({"ok": false, "error": "Local AI service is not running or unreachable"}),
            )
        }
    }
}

/// Scan for GGUF model files in standard directories.
pub async fn brain_scan_models(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let config_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let models_dir = config_dir.join("models");

    // Scan paths: ~/.bizclaw/models/, cwd, common locations
    let scan_dirs = vec![
        models_dir.clone(),
        config_dir.to_path_buf(),
        std::path::PathBuf::from("/root/.bizclaw/models"),
        std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".bizclaw")
            .join("models"),
    ];

    let mut found_models: Vec<serde_json::Value> = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for dir in &scan_dirs {
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension()
                    && (ext == "gguf" || ext == "bin")
                {
                    let abs = path.canonicalize().unwrap_or(path.clone());
                    if seen_paths.contains(&abs) {
                        continue;
                    }
                    seen_paths.insert(abs.clone());

                    let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    let size_str = if size_bytes > 1_000_000_000 {
                        format!("{:.1} GB", size_bytes as f64 / 1e9)
                    } else {
                        format!("{} MB", size_bytes / 1_000_000)
                    };
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    found_models.push(serde_json::json!({
                        "name": name,
                        "path": abs.display().to_string(),
                        "size": size_str,
                        "size_bytes": size_bytes,
                    }));
                }
            }
        }
    }

    // Sort by name
    found_models.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });

    Json(serde_json::json!({
        "ok": true,
        "models": found_models,
        "models_dir": models_dir.display().to_string(),
        "scan_dirs": scan_dirs.iter().filter(|d| d.exists()).map(|d| d.display().to_string()).collect::<Vec<_>>(),
    }))
}

/// Delete a downloaded GGUF model file.
pub async fn brain_delete_model(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let config_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let models_dir = config_dir.join("models");
    let target = models_dir.join(&filename);

    if !target.exists() {
        return Json(serde_json::json!({"ok": false, "error": "Model not found"}));
    }

    match std::fs::remove_file(&target) {
        Ok(_) => Json(serde_json::json!({"ok": true, "message": format!("Deleted {}", filename)})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": format!("Failed to delete: {e}")})),
    }
}

/// Trigger background download of a model.
pub async fn brain_download_model(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let model = body["model"].as_str().unwrap_or("").to_string();
    if model.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Model parameter is required"}));
    }

    let (url, filename) = match model.as_str() {
        "tinyllama-1.1b" | "tinyllama" => (
            "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
            "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
        ),
        "phi-2" => (
            "https://huggingface.co/TheBloke/phi-2-GGUF/resolve/main/phi-2.Q4_K_M.gguf",
            "phi-2.Q4_K_M.gguf",
        ),
        "llama-3.2-1b" | "llama3.2" => (
            "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf",
            "Llama-3.2-1B-Instruct-Q4_K_M.gguf",
        ),
        "gemma4-e2b" | "gemma-4-e2b" => (
            "https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF/resolve/main/gemma-4-E2B-it-Q4_K_M.gguf",
            "gemma-4-E2B-it-Q4_K_M.gguf",
        ),
        "gemma4-e4b" | "gemma-4-e4b" => (
            "https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF/resolve/main/gemma-4-E4B-it-Q4_K_M.gguf",
            "gemma-4-E4B-it-Q4_K_M.gguf",
        ),
        "gemma4-26b" | "gemma-4-26b" | "gemma4-moe" => (
            "https://huggingface.co/unsloth/gemma-4-26B-A4B-it-GGUF/resolve/main/gemma-4-26B-A4B-it-Q4_K_M.gguf",
            "gemma-4-26B-A4B-it-Q4_K_M.gguf",
        ),
        other if other.starts_with("http") => (other, "custom-model.gguf"),
        _ => return Json(serde_json::json!({"ok": false, "error": "Unknown model identifier"})),
    };

    let config_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let models_dir = config_dir.join("models");
    let _ = std::fs::create_dir_all(&models_dir);
    let dest = models_dir.join(filename);
    let status_file = models_dir.join(format!(".dl_{}.json", filename));

    if dest.exists() && dest.metadata().map(|m| m.len()).unwrap_or(0) > 10 * 1024 * 1024 {
        return Json(
            serde_json::json!({"ok": true, "message": "Model already downloaded", "status": "completed"}),
        );
    }

    if status_file.exists() {
        return Json(
            serde_json::json!({"ok": true, "message": "Download already in progress", "status": "downloading"}),
        );
    }

    // Initialize status file
    let _ = std::fs::write(&status_file, r#"{"progress":0, "total":0}"#);

    // Spawn download task
    let url = url.to_string();
    tokio::spawn(async move {
        tracing::info!("⬇️ Starting background download: {}", url);
        let client = reqwest::Client::new();
        if let Ok(resp) = client.get(&url).send().await {
            let total = resp.content_length().unwrap_or(0);
            if let Ok(mut file) = tokio::fs::File::create(&dest).await {
                use futures::StreamExt;
                use tokio::io::AsyncWriteExt;
                let mut stream = resp.bytes_stream();
                let mut downloaded: u64 = 0;
                let mut last_pct = 0;
                while let Some(chunk) = stream.next().await {
                    if let Ok(chunk) = chunk {
                        if file.write_all(&chunk).await.is_ok() {
                            downloaded += chunk.len() as u64;
                            if total > 0 {
                                let pct = (downloaded as f64 / total as f64 * 100.0) as u32;
                                if pct > last_pct {
                                    last_pct = pct;
                                    let status = format!(
                                        r#"{{"progress":{}, "total":{}, "percent":{}}}"#,
                                        downloaded, total, pct
                                    );
                                    let _ = std::fs::write(&status_file, status);
                                }
                            }
                        }
                    }
                }
                let _ = file.flush().await;
            }
        }
        // Remove status file when done
        let _ = std::fs::remove_file(&status_file);
        tracing::info!("✅ Download complete: {}", dest.display());
    });

    Json(serde_json::json!({"ok": true, "message": "Download started", "status": "downloading"}))
}

/// Poll download status for a model.
pub async fn brain_download_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let config_dir = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let models_dir = config_dir.join("models");
    let status_file = models_dir.join(format!(".dl_{}.json", filename));

    if let Ok(content) = std::fs::read_to_string(&status_file) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            return Json(
                serde_json::json!({"ok": true, "status": "downloading", "progress": json}),
            );
        }
    }

    let dest = models_dir.join(&filename);
    if dest.exists() && dest.metadata().map(|m| m.len()).unwrap_or(0) > 10 * 1024 * 1024 {
        return Json(serde_json::json!({"ok": true, "status": "completed"}));
    }

    Json(serde_json::json!({"ok": true, "status": "not_started"}))
}
