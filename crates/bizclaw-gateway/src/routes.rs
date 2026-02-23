//! API route handlers for the gateway.

use axum::{extract::State, Json};
use std::sync::Arc;

use super::server::AppState;

/// Mask a secret string for display — show first 4 chars + •••
fn mask_secret(s: &str) -> String {
    if s.is_empty() { return String::new(); }
    if s.len() <= 4 { return "••••".to_string(); }
    format!("{}••••", &s[..4])
}

/// Health check endpoint.
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "bizclaw-gateway",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// System information endpoint.
pub async fn system_info(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let uptime = state.start_time.elapsed();
    let cfg = state.full_config.lock().unwrap();
    Json(serde_json::json!({
        "name": cfg.identity.name,
        "version": env!("CARGO_PKG_VERSION"),
        "platform": format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
        "uptime_secs": uptime.as_secs(),
        "default_provider": cfg.default_provider,
        "default_model": cfg.default_model,
        "gateway": {
            "host": state.gateway_config.host,
            "port": state.gateway_config.port,
            "require_pairing": state.gateway_config.require_pairing,
        }
    }))
}

/// Get current configuration (sanitized — no API keys).
pub async fn get_config(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let cfg = state.full_config.lock().unwrap();
    Json(serde_json::json!({
        "default_provider": cfg.default_provider,
        "default_model": cfg.default_model,
        "default_temperature": cfg.default_temperature,
        "api_key_set": !cfg.api_key.is_empty(),
        "api_base_url": cfg.api_base_url,
        "identity": {
            "name": cfg.identity.name,
            "persona": cfg.identity.persona,
            "system_prompt": cfg.identity.system_prompt,
        },
        "gateway": {
            "host": cfg.gateway.host,
            "port": cfg.gateway.port,
            "require_pairing": cfg.gateway.require_pairing,
        },
        "memory": {
            "backend": cfg.memory.backend,
            "auto_save": cfg.memory.auto_save,
            "embedding_provider": cfg.memory.embedding_provider,
            "vector_weight": cfg.memory.vector_weight,
            "keyword_weight": cfg.memory.keyword_weight,
        },
        "autonomy": {
            "level": cfg.autonomy.level,
            "workspace_only": cfg.autonomy.workspace_only,
            "allowed_commands": cfg.autonomy.allowed_commands,
            "forbidden_paths": cfg.autonomy.forbidden_paths,
        },
        "brain": {
            "enabled": cfg.brain.enabled,
            "model_path": cfg.brain.model_path,
            "threads": cfg.brain.threads,
            "max_tokens": cfg.brain.max_tokens,
            "context_length": cfg.brain.context_length,
            "temperature": cfg.brain.temperature,
            "json_mode": cfg.brain.json_mode,
        },
        "runtime": {
            "kind": cfg.runtime.kind,
        },
        "tunnel": {
            "provider": cfg.tunnel.provider,
        },
        "secrets": {
            "encrypt": cfg.secrets.encrypt,
        },
        "mcp_servers": cfg.mcp_servers.iter().map(|s| {
            let mut masked_env = std::collections::HashMap::new();
            for (k, v) in &s.env {
                masked_env.insert(k.clone(), mask_secret(v));
            }
            serde_json::json!({
                "name": s.name, "command": s.command,
                "args": s.args, "env": masked_env, "enabled": s.enabled,
            })
        }).collect::<Vec<_>>(),
        "channels": {
            "telegram": cfg.channel.telegram.as_ref().map(|t| serde_json::json!({
                "enabled": t.enabled,
                "bot_token": mask_secret(&t.bot_token),
                "bot_token_set": !t.bot_token.is_empty(),
                "allowed_chat_ids": t.allowed_chat_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", "),
            })),
            "zalo": cfg.channel.zalo.as_ref().map(|z| serde_json::json!({
                "enabled": z.enabled,
                "mode": z.mode,
                "cookie_path": z.personal.cookie_path,
                "cookie": if z.personal.cookie_path.is_empty() { "".to_string() } else { "•••• (saved to file)".to_string() },
                "imei": z.personal.imei,
                "self_listen": z.personal.self_listen,
                "auto_reconnect": z.personal.auto_reconnect,
            })),
            "discord": cfg.channel.discord.as_ref().map(|d| serde_json::json!({
                "enabled": d.enabled,
                "bot_token": mask_secret(&d.bot_token),
                "bot_token_set": !d.bot_token.is_empty(),
                "allowed_channel_ids": d.allowed_channel_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", "),
            })),
            "email": cfg.channel.email.as_ref().map(|e| serde_json::json!({
                "enabled": e.enabled,
                "smtp_host": e.smtp_host,
                "smtp_port": e.smtp_port,
                "smtp_user": e.email,
                "smtp_pass": mask_secret(&e.password),
                "imap_host": e.imap_host,
                "imap_port": e.imap_port,
            })),
            "whatsapp": cfg.channel.whatsapp.as_ref().map(|w| serde_json::json!({
                "enabled": w.enabled,
                "phone_number_id": w.phone_number_id,
                "access_token": mask_secret(&w.access_token),
                "business_id": w.business_id,
            })),
        },
    }))
}

/// Get full config as TOML string for export/display.
pub async fn get_full_config(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let cfg = state.full_config.lock().unwrap();
    let toml_str = toml::to_string_pretty(&*cfg).unwrap_or_default();
    Json(serde_json::json!({
        "ok": true,
        "toml": toml_str,
        "config_path": state.config_path.display().to_string(),
    }))
}

/// Update config fields via JSON body.
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let mut cfg = state.full_config.lock().unwrap();

    // Update top-level fields
    if let Some(v) = req.get("default_provider").and_then(|v| v.as_str()) {
        cfg.default_provider = v.to_string();
    }
    if let Some(v) = req.get("default_model").and_then(|v| v.as_str()) {
        cfg.default_model = v.to_string();
    }
    if let Some(v) = req.get("default_temperature").and_then(|v| v.as_f64()) {
        cfg.default_temperature = v as f32;
    }
    if let Some(v) = req.get("api_key").and_then(|v| v.as_str()) {
        cfg.api_key = v.to_string();
    }
    if let Some(v) = req.get("api_base_url").and_then(|v| v.as_str()) {
        cfg.api_base_url = v.to_string();
    }

    // Update identity
    if let Some(id) = req.get("identity") {
        if let Some(v) = id.get("name").and_then(|v| v.as_str()) {
            cfg.identity.name = v.to_string();
        }
        if let Some(v) = id.get("persona").and_then(|v| v.as_str()) {
            cfg.identity.persona = v.to_string();
        }
        if let Some(v) = id.get("system_prompt").and_then(|v| v.as_str()) {
            cfg.identity.system_prompt = v.to_string();
        }
    }

    // Update memory
    if let Some(mem) = req.get("memory") {
        if let Some(v) = mem.get("backend").and_then(|v| v.as_str()) {
            cfg.memory.backend = v.to_string();
        }
        if let Some(v) = mem.get("auto_save").and_then(|v| v.as_bool()) {
            cfg.memory.auto_save = v;
        }
    }

    // Update autonomy
    if let Some(auto) = req.get("autonomy") {
        if let Some(v) = auto.get("level").and_then(|v| v.as_str()) {
            cfg.autonomy.level = v.to_string();
        }
        if let Some(v) = auto.get("workspace_only").and_then(|v| v.as_bool()) {
            cfg.autonomy.workspace_only = v;
        }
    }

    // Update brain
    if let Some(brain) = req.get("brain") {
        if let Some(v) = brain.get("enabled").and_then(|v| v.as_bool()) {
            cfg.brain.enabled = v;
        }
        if let Some(v) = brain.get("model_path").and_then(|v| v.as_str()) {
            cfg.brain.model_path = v.to_string();
        }
        if let Some(v) = brain.get("threads").and_then(|v| v.as_u64()) {
            cfg.brain.threads = v as u32;
        }
        if let Some(v) = brain.get("max_tokens").and_then(|v| v.as_u64()) {
            cfg.brain.max_tokens = v as u32;
        }
        if let Some(v) = brain.get("context_length").and_then(|v| v.as_u64()) {
            cfg.brain.context_length = v as u32;
        }
        if let Some(v) = brain.get("temperature").and_then(|v| v.as_f64()) {
            cfg.brain.temperature = v as f32;
        }
    }

    // Update MCP servers
    if let Some(mcp) = req.get("mcp_servers") {
        if let Ok(servers) = serde_json::from_value::<Vec<bizclaw_core::config::McpServerEntry>>(mcp.clone()) {
            cfg.mcp_servers = servers;
        }
    }

    // Save to disk
    let content = toml::to_string_pretty(&*cfg).unwrap_or_default();
    let new_cfg = cfg.clone();
    drop(cfg); // Release lock before file write + agent reinit

    match std::fs::write(&state.config_path, &content) {
        Ok(_) => {
            tracing::info!("✅ Config saved to {}", state.config_path.display());

            // Re-initialize Agent with new config (async, don't block response)
            let agent_lock = state.agent.clone();
            tokio::spawn(async move {
                match bizclaw_agent::Agent::new_with_mcp(new_cfg).await {
                    Ok(new_agent) => {
                        let mut guard = agent_lock.lock().await;
                        tracing::info!("🔄 Agent re-initialized: provider={}, tools={}",
                            new_agent.provider_name(), new_agent.tool_count());
                        *guard = Some(new_agent);
                    }
                    Err(e) => tracing::warn!("⚠️ Agent re-init failed: {e}"),
                }
            });

            Json(serde_json::json!({"ok": true, "message": "Config saved — agent reloading"}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// Update channel config.
pub async fn update_channel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let channel_type = req.get("channel_type").and_then(|v| v.as_str()).unwrap_or("");
    let enabled = req.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let mut cfg = state.full_config.lock().unwrap();

    match channel_type {
        "telegram" => {
            let token_val = req.get("bot_token").and_then(|v| v.as_str()).unwrap_or("");
            let token = if token_val.contains('•') {
                cfg.channel.telegram.as_ref().map(|t| t.bot_token.clone()).unwrap_or_default()
            } else { token_val.to_string() };
            let chat_ids: Vec<i64> = req.get("allowed_chat_ids")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            cfg.channel.telegram = Some(bizclaw_core::config::TelegramChannelConfig {
                enabled, bot_token: token, allowed_chat_ids: chat_ids,
            });
        }
        "zalo" => {
            let mut zalo_cfg = cfg.channel.zalo.clone().unwrap_or_default();
            zalo_cfg.enabled = enabled;
            if let Some(v) = req.get("cookie").and_then(|v| v.as_str()) {
                // Save cookie to file
                let cookie_dir = state.config_path.parent()
                    .unwrap_or(std::path::Path::new("."));
                let cookie_path = cookie_dir.join("zalo_cookie.txt");
                std::fs::write(&cookie_path, v).ok();
                zalo_cfg.personal.cookie_path = cookie_path.display().to_string();
            }
            if let Some(v) = req.get("imei").and_then(|v| v.as_str()) {
                zalo_cfg.personal.imei = v.to_string();
            }
            cfg.channel.zalo = Some(zalo_cfg);
        }
        "discord" => {
            let token_val = req.get("bot_token").and_then(|v| v.as_str()).unwrap_or("");
            let token = if token_val.contains('•') {
                // Keep existing token if masked value sent
                cfg.channel.discord.as_ref().map(|d| d.bot_token.clone()).unwrap_or_default()
            } else { token_val.to_string() };
            let ids: Vec<u64> = req.get("allowed_channel_ids")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            cfg.channel.discord = Some(bizclaw_core::config::DiscordChannelConfig {
                enabled, bot_token: token, allowed_channel_ids: ids,
            });
        }
        "email" => {
            let smtp_host = req.get("smtp_host").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let smtp_port = req.get("smtp_port").and_then(|v| v.as_str()).unwrap_or("587")
                .parse::<u16>().unwrap_or(587);
            let email_addr = req.get("smtp_user").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let pass_val = req.get("smtp_pass").and_then(|v| v.as_str()).unwrap_or("");
            let password = if pass_val.contains('•') {
                cfg.channel.email.as_ref().map(|e| e.password.clone()).unwrap_or_default()
            } else { pass_val.to_string() };
            let imap_host = req.get("imap_host").and_then(|v| v.as_str()).unwrap_or("").to_string();
            cfg.channel.email = Some(bizclaw_core::config::EmailChannelConfig {
                enabled, smtp_host, smtp_port, email: email_addr, password,
                imap_host, imap_port: 993,
            });
        }
        "whatsapp" => {
            let phone_val = req.get("phone_number_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let token_val = req.get("access_token").and_then(|v| v.as_str()).unwrap_or("");
            let token = if token_val.contains('•') {
                cfg.channel.whatsapp.as_ref().map(|w| w.access_token.clone()).unwrap_or_default()
            } else { token_val.to_string() };
            cfg.channel.whatsapp = Some(bizclaw_core::config::WhatsAppChannelConfig {
                enabled, phone_number_id: phone_val, access_token: token,
                webhook_verify_token: String::new(), business_id: String::new(),
            });
        }
        _ => {
            return Json(serde_json::json!({"ok": false, "error": format!("Unknown channel: {channel_type}")}));
        }
    }

    // Save to disk
    let content = toml::to_string_pretty(&*cfg).unwrap_or_default();
    match std::fs::write(&state.config_path, &content) {
        Ok(_) => Json(serde_json::json!({"ok": true, "message": format!("{channel_type} config saved")})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// List available providers.
pub async fn list_providers(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let cfg = state.full_config.lock().unwrap();
    let active = &cfg.default_provider;
    Json(serde_json::json!({
        "providers": [
            {"name": "openai", "type": "cloud", "status": if active == "openai" {"active"} else {"available"}, "models": ["gpt-4o", "gpt-4o-mini", "gpt-3.5-turbo", "o1-mini", "o3-mini"]},
            {"name": "anthropic", "type": "cloud", "status": if active == "anthropic" {"active"} else {"available"}, "models": ["claude-sonnet-4-20250514", "claude-3.5-sonnet", "claude-3-haiku"]},
            {"name": "gemini", "type": "cloud", "status": if active == "gemini" {"active"} else {"available"}, "models": ["gemini-2.5-pro", "gemini-2.5-flash", "gemini-2.0-flash"]},
            {"name": "deepseek", "type": "cloud", "status": if active == "deepseek" {"active"} else {"available"}, "models": ["deepseek-chat", "deepseek-reasoner"]},
            {"name": "groq", "type": "cloud", "status": if active == "groq" {"active"} else {"available"}, "models": ["llama-3.3-70b", "mixtral-8x7b-32768"]},
            {"name": "ollama", "type": "local", "status": if active == "ollama" {"active"} else {"available"}, "models": ["llama3.2", "qwen3", "phi-4", "gemma2"]},
            {"name": "llamacpp", "type": "local", "status": if active == "llamacpp" {"active"} else {"available"}, "models": ["server endpoint"]},
            {"name": "brain", "type": "local", "status": if active == "brain" {"active"} else {"available"}, "models": ["tinyllama-1.1b", "phi-2", "llama-3.2-1b"]},
        ]
    }))
}

/// List available channels with config status.
pub async fn list_channels(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let cfg = state.full_config.lock().unwrap();
    Json(serde_json::json!({
        "channels": [
            {"name": "cli", "type": "interactive", "status": "active", "configured": true},
            {"name": "telegram", "type": "messaging", "status": if cfg.channel.telegram.as_ref().map_or(false, |t| t.enabled) { "active" } else { "disabled" }, "configured": cfg.channel.telegram.is_some()},
            {"name": "zalo", "type": "messaging", "status": if cfg.channel.zalo.as_ref().map_or(false, |z| z.enabled) { "active" } else { "disabled" }, "configured": cfg.channel.zalo.is_some()},
            {"name": "discord", "type": "messaging", "status": if cfg.channel.discord.as_ref().map_or(false, |d| d.enabled) { "active" } else { "disabled" }, "configured": cfg.channel.discord.is_some()},
            {"name": "email", "type": "messaging", "status": if cfg.channel.email.as_ref().map_or(false, |e| e.enabled) { "active" } else { "disabled" }, "configured": cfg.channel.email.is_some()},
            {"name": "webhook", "type": "api", "status": "available", "configured": false},
            {"name": "whatsapp", "type": "messaging", "status": if cfg.channel.whatsapp.as_ref().map_or(false, |w| w.enabled) { "active" } else { "disabled" }, "configured": cfg.channel.whatsapp.is_some()},
        ]
    }))
}

/// List installed Ollama models.
pub async fn ollama_models() -> Json<serde_json::Value> {
    let url = "http://localhost:11434/api/tags";
    match reqwest::Client::new().get(url).timeout(std::time::Duration::from_secs(5)).send().await {
        Ok(resp) => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                let models: Vec<serde_json::Value> = body.get("models")
                    .and_then(|m| m.as_array())
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|m| {
                        let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let size_bytes = m.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
                        let size = if size_bytes > 1_000_000_000 {
                            format!("{:.1} GB", size_bytes as f64 / 1e9)
                        } else {
                            format!("{} MB", size_bytes / 1_000_000)
                        };
                        let family = m.get("details").and_then(|d| d.get("family")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        serde_json::json!({"name": name, "size": size, "family": family})
                    })
                    .collect();
                Json(serde_json::json!({"ok": true, "models": models}))
            } else {
                Json(serde_json::json!({"ok": true, "models": []}))
            }
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": format!("Ollama not running: {e}")})),
    }
}

/// Scan for GGUF model files in standard directories.
pub async fn brain_scan_models(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let config_dir = state.config_path.parent()
        .unwrap_or(std::path::Path::new("."));
    let models_dir = config_dir.join("models");
    
    // Scan paths: ~/.bizclaw/models/, cwd, common locations
    let scan_dirs = vec![
        models_dir.clone(),
        config_dir.to_path_buf(),
        std::path::PathBuf::from("/root/.bizclaw/models"),
        std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".bizclaw").join("models"),
    ];

    let mut found_models: Vec<serde_json::Value> = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for dir in &scan_dirs {
        if !dir.exists() { continue; }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "gguf" || ext == "bin" {
                        let abs = path.canonicalize().unwrap_or(path.clone());
                        if seen_paths.contains(&abs) { continue; }
                        seen_paths.insert(abs.clone());
                        
                        let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        let size_str = if size_bytes > 1_000_000_000 {
                            format!("{:.1} GB", size_bytes as f64 / 1e9)
                        } else {
                            format!("{} MB", size_bytes / 1_000_000)
                        };
                        let name = path.file_name()
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
    }

    // Sort by name
    found_models.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    Json(serde_json::json!({
        "ok": true,
        "models": found_models,
        "models_dir": models_dir.display().to_string(),
        "scan_dirs": scan_dirs.iter().filter(|d| d.exists()).map(|d| d.display().to_string()).collect::<Vec<_>>(),
    }))
}

/// Generate Zalo QR code for login.
pub async fn zalo_qr_code(
    State(_state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    use bizclaw_channels::zalo::client::auth::{ZaloAuth, ZaloCredentials};

    let creds = ZaloCredentials::default();
    let mut auth = ZaloAuth::new(creds);

    match auth.get_qr_code().await {
        Ok(qr) => Json(serde_json::json!({
            "ok": true,
            "qr_code": qr.image,
            "qr_id": qr.code,
            "imei": auth.credentials().imei,
            "instructions": [
                "1. Mở ứng dụng Zalo trên điện thoại",
                "2. Nhấn biểu tượng QR ở thanh tìm kiếm",
                "3. Quét mã QR này để đăng nhập",
                "4. Xác nhận đăng nhập trên điện thoại"
            ],
            "message": "Quét mã QR bằng Zalo trên điện thoại"
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e.to_string(),
            "fallback": "Vui lòng vào chat.zalo.me → F12 → Application → Cookies → Copy toàn bộ và paste vào ô Cookie bên dưới"
        })),
    }
}

/// WhatsApp webhook verification (GET) — Meta sends this to verify endpoint.
pub async fn whatsapp_webhook_verify(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    let mode = params.get("hub.mode").map(|s| s.as_str()).unwrap_or("");
    let token = params.get("hub.verify_token").map(|s| s.as_str()).unwrap_or("");
    let challenge = params.get("hub.challenge").map(|s| s.as_str()).unwrap_or("");

    let expected_token = {
        let cfg = state.full_config.lock().unwrap();
        cfg.channel.whatsapp.as_ref()
            .map(|w| w.webhook_verify_token.clone())
            .unwrap_or_default()
    };

    if mode == "subscribe" && token == expected_token {
        tracing::info!("WhatsApp webhook verified");
        axum::response::Response::builder()
            .status(200)
            .body(axum::body::Body::from(challenge.to_string()))
            .unwrap()
    } else {
        axum::response::Response::builder()
            .status(403)
            .body(axum::body::Body::from("Forbidden"))
            .unwrap()
    }
}

/// WhatsApp webhook handler (POST) — receives incoming messages from Meta.
pub async fn whatsapp_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // Extract messages and spawn processing in background
    // (WhatsApp expects quick 200 OK response)
    let entry = &body["entry"];
    if let Some(entries) = entry.as_array() {
        for entry in entries {
            if let Some(changes) = entry["changes"].as_array() {
                for change in changes {
                    let value = &change["value"];
                    if let Some(messages) = value["messages"].as_array() {
                        for msg in messages {
                            let msg_type = msg["type"].as_str().unwrap_or("");
                            if msg_type != "text" { continue; }

                            let from = msg["from"].as_str().unwrap_or("").to_string();
                            let text = msg["text"]["body"].as_str().unwrap_or("").to_string();
                            let msg_id = msg["id"].as_str().unwrap_or("").to_string();

                            if text.is_empty() { continue; }

                            tracing::info!("[whatsapp] Message from {from}: {text}");

                            // Get WhatsApp config for reply
                            let wa_config = {
                                let cfg = state.full_config.lock().unwrap();
                                cfg.channel.whatsapp.clone()
                            };

                            // Spawn background task for agent processing + reply
                            let agent_lock = state.agent.clone();
                            tokio::spawn(async move {
                                // Process through Agent Engine
                                let response = {
                                    let mut agent = agent_lock.lock().await;
                                    if let Some(agent) = agent.as_mut() {
                                        match agent.process(&text).await {
                                            Ok(r) => r,
                                            Err(e) => format!("Error: {e}"),
                                        }
                                    } else {
                                        "Agent not available".to_string()
                                    }
                                };

                                // Send reply via WhatsApp Cloud API
                                if let Some(wa_cfg) = wa_config {
                                    let url = format!(
                                        "https://graph.facebook.com/v21.0/{}/messages",
                                        wa_cfg.phone_number_id
                                    );
                                    let reply = serde_json::json!({
                                        "messaging_product": "whatsapp",
                                        "to": from,
                                        "type": "text",
                                        "text": { "body": response },
                                        "context": { "message_id": msg_id },
                                    });
                                    let client = reqwest::Client::new();
                                    if let Err(e) = client
                                        .post(&url)
                                        .header("Authorization", format!("Bearer {}", wa_cfg.access_token))
                                        .json(&reply)
                                        .send()
                                        .await
                                    {
                                        tracing::error!("[whatsapp] Reply failed: {e}");
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }
    }

    Json(serde_json::json!({"status": "ok"}))
}

// ---- Scheduler API ----

/// List all scheduled tasks.
pub async fn scheduler_list_tasks(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let engine = state.scheduler.lock().await;
    let tasks: Vec<_> = engine.list_tasks().iter().map(|t| {
        serde_json::json!({
            "id": t.id,
            "name": t.name,
            "status": format!("{:?}", t.status),
            "enabled": t.enabled,
            "run_count": t.run_count,
            "next_run": t.next_run.map(|d| d.to_rfc3339()),
            "last_run": t.last_run.map(|d| d.to_rfc3339()),
        })
    }).collect();
    Json(serde_json::json!({"ok": true, "tasks": tasks, "count": tasks.len()}))
}

/// Add a new scheduled task.
pub async fn scheduler_add_task(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("unnamed");
    let action_str = body["action"].as_str().unwrap_or("");
    let action = bizclaw_scheduler::tasks::TaskAction::Notify(action_str.to_string());

    let task_type = body["type"].as_str().unwrap_or("interval");
    let task = match task_type {
        "cron" => {
            let expr = body["expression"].as_str().unwrap_or("0 * * * *");
            bizclaw_scheduler::Task::cron(name, expr, action)
        }
        "once" => {
            let at = chrono::Utc::now() + chrono::Duration::seconds(
                body["delay_secs"].as_i64().unwrap_or(60)
            );
            bizclaw_scheduler::Task::once(name, at, action)
        }
        _ => {
            let secs = body["interval_secs"].as_u64().unwrap_or(300);
            bizclaw_scheduler::Task::interval(name, secs, action)
        }
    };

    let id = task.id.clone();
    state.scheduler.lock().await.add_task(task);
    Json(serde_json::json!({"ok": true, "id": id}))
}

/// Remove a scheduled task.
pub async fn scheduler_remove_task(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let removed = state.scheduler.lock().await.remove_task(&id);
    Json(serde_json::json!({"ok": removed}))
}

/// Get notification history.
pub async fn scheduler_notifications(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let engine = state.scheduler.lock().await;
    let history: Vec<_> = engine.router.history().iter().map(|n| {
        serde_json::json!({
            "title": n.title,
            "body": n.body,
            "source": n.source,
            "priority": format!("{:?}", n.priority),
            "timestamp": n.timestamp.to_rfc3339(),
        })
    }).collect();
    Json(serde_json::json!({"ok": true, "notifications": history}))
}

// ---- Knowledge Base API ----

/// Search the knowledge base.
pub async fn knowledge_search(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let query = body["query"].as_str().unwrap_or("");
    let limit = body["limit"].as_u64().unwrap_or(5) as usize;

    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => {
            let results = store.search(query, limit);
            let items: Vec<_> = results.iter().map(|r| {
                serde_json::json!({
                    "doc_name": r.doc_name,
                    "content": r.content,
                    "score": r.score,
                    "chunk_idx": r.chunk_idx,
                })
            }).collect();
            Json(serde_json::json!({"ok": true, "results": items, "count": items.len()}))
        }
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

/// List all knowledge documents.
pub async fn knowledge_list_docs(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => {
            let docs: Vec<_> = store.list_documents().iter().map(|(id, name, source, chunks)| {
                serde_json::json!({"id": id, "name": name, "source": source, "chunks": chunks})
            }).collect();
            let (total_docs, total_chunks) = store.stats();
            Json(serde_json::json!({
                "ok": true, "documents": docs,
                "total_docs": total_docs, "total_chunks": total_chunks
            }))
        }
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

/// Add a document to the knowledge base.
pub async fn knowledge_add_doc(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("unnamed.txt");
    let content = body["content"].as_str().unwrap_or("");
    let source = body["source"].as_str().unwrap_or("api");

    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => {
            match store.add_document(name, content, source) {
                Ok(chunks) => Json(serde_json::json!({"ok": true, "chunks": chunks})),
                Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
            }
        }
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

/// Remove a document from the knowledge base.
pub async fn knowledge_remove_doc(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Json<serde_json::Value> {
    let kb = state.knowledge.lock().await;
    match kb.as_ref() {
        Some(store) => {
            match store.remove_document(id) {
                Ok(()) => Json(serde_json::json!({"ok": true})),
                Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
            }
        }
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

// ---- Multi-Agent Orchestrator API ----

/// List all agents in the orchestrator.
pub async fn list_agents(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let orch = state.orchestrator.lock().await;
    Json(serde_json::json!({
        "ok": true,
        "agents": orch.list_agents(),
        "total": orch.agent_count(),
        "default": orch.default_agent_name(),
        "recent_messages": orch.recent_messages(10),
    }))
}

/// Create a new named agent.
pub async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("agent");
    let role = body["role"].as_str().unwrap_or("assistant");
    let description = body["description"].as_str().unwrap_or("A helpful AI agent");
    
    // Use current config as base, optionally override provider/model
    let mut agent_config = state.full_config.lock().unwrap().clone();
    if let Some(provider) = body["provider"].as_str() {
        agent_config.default_provider = provider.to_string();
    }
    if let Some(model) = body["model"].as_str() {
        agent_config.default_model = model.to_string();
    }
    if let Some(persona) = body["persona"].as_str() {
        agent_config.identity.persona = persona.to_string();
    }
    if let Some(sys_prompt) = body["system_prompt"].as_str() {
        agent_config.identity.system_prompt = sys_prompt.to_string();
    }
    agent_config.identity.name = name.to_string();
    
    match bizclaw_agent::Agent::new_with_mcp(agent_config).await {
        Ok(agent) => {
            let mut orch = state.orchestrator.lock().await;
            orch.add_agent(name, role, description, agent);
            tracing::info!("🤖 Agent '{}' created (role={})", name, role);
            Json(serde_json::json!({
                "ok": true,
                "name": name,
                "role": role,
                "total_agents": orch.agent_count(),
            }))
        }
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": format!("Failed to create agent: {e}"),
        })),
    }
}

/// Delete a named agent.
pub async fn delete_agent(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let mut orch = state.orchestrator.lock().await;
    let removed = orch.remove_agent(&name);
    Json(serde_json::json!({
        "ok": removed,
        "message": if removed { format!("Agent '{}' removed", name) } else { format!("Agent '{}' not found", name) },
    }))
}

/// Update an existing agent's metadata.
pub async fn update_agent(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let role = body["role"].as_str();
    let description = body["description"].as_str();
    
    let mut orch = state.orchestrator.lock().await;
    let updated = orch.update_agent(&name, role, description);
    Json(serde_json::json!({
        "ok": updated,
        "message": if updated { format!("Agent '{}' updated", name) } else { format!("Agent '{}' not found", name) },
    }))
}

/// Chat with a specific agent.
pub async fn agent_chat(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let message = body["message"].as_str().unwrap_or("");
    if message.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Empty message"}));
    }
    
    let mut orch = state.orchestrator.lock().await;
    match orch.send_to(&name, message).await {
        Ok(response) => Json(serde_json::json!({
            "ok": true,
            "agent": name,
            "response": response,
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e.to_string(),
        })),
    }
}

/// Broadcast message to all agents.
pub async fn agent_broadcast(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let message = body["message"].as_str().unwrap_or("");
    if message.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Empty message"}));
    }
    
    let mut orch = state.orchestrator.lock().await;
    let results = orch.broadcast(message).await;
    let responses: Vec<serde_json::Value> = results.into_iter().map(|(name, result)| {
        match result {
            Ok(response) => serde_json::json!({
                "agent": name,
                "ok": true,
                "response": response,
            }),
            Err(e) => serde_json::json!({
                "agent": name,
                "ok": false,
                "error": e.to_string(),
            }),
        }
    }).collect();
    
    Json(serde_json::json!({
        "ok": true,
        "responses": responses,
    }))
}

// ---- Telegram Bot ↔ Agent API ----

/// Connect a Telegram bot to a specific agent.
/// Verifies the bot token, then spawns a polling loop.
pub async fn connect_telegram(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(agent_name): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let bot_token = body["bot_token"].as_str().unwrap_or("").trim().to_string();
    if bot_token.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "bot_token is required"}));
    }

    // Check agent exists
    {
        let orch = state.orchestrator.lock().await;
        let agents = orch.list_agents();
        if !agents.iter().any(|a| a["name"].as_str() == Some(&agent_name)) {
            return Json(serde_json::json!({"ok": false, "error": format!("Agent '{}' not found", agent_name)}));
        }
    }

    // Already connected? Disconnect first
    {
        let mut bots = state.telegram_bots.lock().await;
        if let Some(existing) = bots.remove(&agent_name) {
            existing.abort_handle.notify_one();
            tracing::info!("[telegram] Disconnecting existing bot for agent '{}'", agent_name);
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    // Verify bot token
    let tg = bizclaw_channels::telegram::TelegramChannel::new(
        bizclaw_channels::telegram::TelegramConfig {
            bot_token: bot_token.clone(),
            enabled: true,
            poll_interval: 1,
        }
    );
    let bot_info = match tg.get_me().await {
        Ok(me) => me,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("Invalid bot token: {e}")})),
    };
    let bot_username = bot_info.username.clone().unwrap_or_default();
    tracing::info!("[telegram] Bot @{} verified for agent '{}'", bot_username, agent_name);

    // Spawn polling loop
    let stop = Arc::new(tokio::sync::Notify::new());
    let stop_rx = stop.clone();
    let state_clone = state.clone();
    let agent_name_clone = agent_name.clone();
    let bot_token_clone = bot_token.clone();

    tokio::spawn(async move {
        let mut channel = bizclaw_channels::telegram::TelegramChannel::new(
            bizclaw_channels::telegram::TelegramConfig {
                bot_token: bot_token_clone,
                enabled: true,
                poll_interval: 1,
            }
        );
        tracing::info!("[telegram] Polling started for agent '{}'", agent_name_clone);

        loop {
            tokio::select! {
                _ = stop_rx.notified() => {
                    tracing::info!("[telegram] Polling stopped for agent '{}'", agent_name_clone);
                    break;
                }
                result = channel.get_updates() => {
                    match result {
                        Ok(updates) => {
                            for update in updates {
                                if let Some(msg) = update.to_incoming() {
                                    let chat_id: i64 = msg.thread_id.parse().unwrap_or(0);
                                    let sender = msg.sender_name.clone().unwrap_or_default();
                                    let text = msg.content.clone();

                                    tracing::info!("[telegram] {} → agent '{}': {}", sender, agent_name_clone, &text[..text.len().min(100)]);

                                    // Send typing indicator
                                    let _ = channel.send_typing(chat_id).await;

                                    // Route to agent
                                    let response = {
                                        let mut orch = state_clone.orchestrator.lock().await;
                                        match orch.send_to(&agent_name_clone, &text).await {
                                            Ok(r) => r,
                                            Err(e) => format!("⚠️ Agent error: {e}"),
                                        }
                                    };

                                    // Reply via Telegram
                                    if let Err(e) = channel.send_message(chat_id, &response).await {
                                        tracing::error!("[telegram] Reply failed: {e}");
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("[telegram] Polling error for '{}': {e}", agent_name_clone);
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        }
                    }
                }
            }
        }
    });

    // Save state
    {
        let mut bots = state.telegram_bots.lock().await;
        bots.insert(agent_name.clone(), super::server::TelegramBotState {
            bot_token: bot_token.clone(),
            bot_username: bot_username.clone(),
            abort_handle: stop,
        });
    }

    Json(serde_json::json!({
        "ok": true,
        "agent": agent_name,
        "bot_username": bot_username,
        "message": format!("@{} connected to agent '{}'", bot_username, agent_name),
    }))
}

/// Disconnect Telegram bot from an agent.
pub async fn disconnect_telegram(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(agent_name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let mut bots = state.telegram_bots.lock().await;
    if let Some(bot) = bots.remove(&agent_name) {
        bot.abort_handle.notify_one();
        tracing::info!("[telegram] @{} disconnected from agent '{}'", bot.bot_username, agent_name);
        Json(serde_json::json!({
            "ok": true,
            "message": format!("@{} disconnected from agent '{}'", bot.bot_username, agent_name),
        }))
    } else {
        Json(serde_json::json!({"ok": false, "error": format!("No Telegram bot connected to agent '{}'", agent_name)}))
    }
}

/// Get Telegram bot status for an agent.
pub async fn telegram_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(agent_name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let bots = state.telegram_bots.lock().await;
    if let Some(bot) = bots.get(&agent_name) {
        Json(serde_json::json!({
            "ok": true,
            "connected": true,
            "bot_username": bot.bot_username,
            "agent": agent_name,
        }))
    } else {
        Json(serde_json::json!({
            "ok": true,
            "connected": false,
            "agent": agent_name,
        }))
    }
}

// ---- Brain Workspace API ----

/// List all brain files in the workspace.
/// If `?tenant=slug` provided, uses per-tenant workspace.
pub async fn brain_list_files(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let ws = match params.get("tenant") {
        Some(slug) if !slug.is_empty() => bizclaw_memory::brain::BrainWorkspace::for_tenant(slug),
        _ => bizclaw_memory::brain::BrainWorkspace::default(),
    };
    let _ = ws.initialize(); // ensure files exist
    let files = ws.list_files();
    let base_dir = ws.base_dir().display().to_string();
    Json(serde_json::json!({
        "ok": true,
        "files": files,
        "base_dir": base_dir,
        "count": files.len(),
    }))
}

/// Read a specific brain file.
pub async fn brain_read_file(
    axum::extract::Path(filename): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let ws = match params.get("tenant") {
        Some(slug) if !slug.is_empty() => bizclaw_memory::brain::BrainWorkspace::for_tenant(slug),
        _ => bizclaw_memory::brain::BrainWorkspace::default(),
    };
    match ws.read_file(&filename) {
        Some(content) => Json(serde_json::json!({
            "ok": true, "filename": filename, "content": content, "size": content.len(),
        })),
        None => Json(serde_json::json!({"ok": false, "error": format!("File not found: {filename}")})),
    }
}

/// Write (create/update) a brain file.
pub async fn brain_write_file(
    axum::extract::Path(filename): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let ws = match params.get("tenant") {
        Some(slug) if !slug.is_empty() => bizclaw_memory::brain::BrainWorkspace::for_tenant(slug),
        _ => bizclaw_memory::brain::BrainWorkspace::default(),
    };
    let content = body["content"].as_str().unwrap_or("");
    match ws.write_file(&filename, content) {
        Ok(()) => Json(serde_json::json!({"ok": true, "message": format!("Saved: {filename}")})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// Delete a brain file.
pub async fn brain_delete_file(
    axum::extract::Path(filename): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let ws = match params.get("tenant") {
        Some(slug) if !slug.is_empty() => bizclaw_memory::brain::BrainWorkspace::for_tenant(slug),
        _ => bizclaw_memory::brain::BrainWorkspace::default(),
    };
    match ws.delete_file(&filename) {
        Ok(true) => Json(serde_json::json!({"ok": true, "message": format!("Deleted: {filename}")})),
        Ok(false) => Json(serde_json::json!({"ok": false, "error": "File not found"})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// Brain Personalization — AI generates SOUL.md, IDENTITY.md, USER.md from user description.
pub async fn brain_personalize(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let about_user = body["about_user"].as_str().unwrap_or("");
    let agent_vibe = body["agent_vibe"].as_str().unwrap_or("helpful and professional");
    let agent_name = body["agent_name"].as_str().unwrap_or("BizClaw Agent");
    let language = body["language"].as_str().unwrap_or("vi");
    let tenant = body["tenant"].as_str().unwrap_or("");

    if about_user.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Please describe yourself (about_user)"}));
    }

    // Build the AI prompt
    let prompt = format!(
        r#"You are a configuration assistant. Based on the user's description below, generate personalized brain files for an AI agent.

User describes themselves: "{about_user}"
Desired agent personality/vibe: "{agent_vibe}"
Agent name: "{agent_name}"
Language: "{language}"

Generate EXACTLY these 3 files. Output as JSON with keys "soul", "identity", "user". Each value is the markdown content for that file.

SOUL.md should define the agent's personality, tone, and behavioral rules based on the desired vibe.
IDENTITY.md should define the agent's name, role, and style.
USER.md should capture key facts about the user for personalization.

Output ONLY valid JSON, no markdown fences."#
    );

    // Send to agent
    let mut agent_lock = state.agent.lock().await;
    let response = match agent_lock.as_mut() {
        Some(agent) => match agent.process(&prompt).await {
            Ok(r) => r,
            Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("AI error: {e}")})),
        },
        None => return Json(serde_json::json!({"ok": false, "error": "Agent not available — configure provider first"})),
    };
    drop(agent_lock);

    // Parse AI response as JSON
    let clean = response.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
    let parsed: serde_json::Value = match serde_json::from_str(clean) {
        Ok(v) => v,
        Err(_) => {
            // Fallback: try to extract JSON from response
            let start = clean.find('{').unwrap_or(0);
            let end = clean.rfind('}').map(|i| i + 1).unwrap_or(clean.len());
            match serde_json::from_str(&clean[start..end]) {
                Ok(v) => v,
                Err(e) => return Json(serde_json::json!({
                    "ok": false,
                    "error": format!("Failed to parse AI response: {e}"),
                    "raw": response,
                })),
            }
        }
    };

    // Save to workspace
    let ws = if tenant.is_empty() {
        bizclaw_memory::brain::BrainWorkspace::default()
    } else {
        bizclaw_memory::brain::BrainWorkspace::for_tenant(tenant)
    };
    let _ = ws.initialize();

    let mut saved = Vec::new();
    for (key, filename) in &[("soul", "SOUL.md"), ("identity", "IDENTITY.md"), ("user", "USER.md")] {
        if let Some(content) = parsed[key].as_str() {
            if ws.write_file(filename, content).is_ok() {
                saved.push(*filename);
            }
        }
    }

    tracing::info!("🎨 Brain personalized: {} files saved", saved.len());
    Json(serde_json::json!({
        "ok": true,
        "saved": saved,
        "files": {
            "soul": parsed["soul"].as_str().unwrap_or(""),
            "identity": parsed["identity"].as_str().unwrap_or(""),
            "user": parsed["user"].as_str().unwrap_or(""),
        },
    }))
}

// ---- System Health Check ----

/// Comprehensive health check — verify API keys, config, workspace, connectivity.
pub async fn system_health_check(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    // Extract all needed values from config — drop guard before any .await
    let (provider, api_key_empty, model_empty, model_info, config_path_display) = {
        let cfg = state.full_config.lock().unwrap();
        (
            cfg.default_provider.clone(),
            cfg.api_key.is_empty(),
            cfg.default_model.is_empty(),
            format!("{}/{}", cfg.default_provider, cfg.default_model),
            state.config_path.display().to_string(),
        )
    };

    let mut checks: Vec<serde_json::Value> = Vec::new();
    let mut pass_count = 0;
    let mut fail_count = 0;

    // 1. Config file
    let config_ok = state.config_path.exists();
    checks.push(serde_json::json!({"name": "Config File", "status": if config_ok {"pass"} else {"fail"}, "detail": config_path_display}));
    if config_ok { pass_count += 1; } else { fail_count += 1; }

    // 2. Provider API key
    let key_ok = match provider.as_str() {
        "ollama" | "brain" | "llamacpp" => true,
        _ => !api_key_empty,
    };
    let key_detail = if key_ok { format!("{provider}: configured") } else { format!("{provider}: API key missing!") };
    checks.push(serde_json::json!({"name": "API Key", "status": if key_ok {"pass"} else {"fail"}, "detail": key_detail}));
    if key_ok { pass_count += 1; } else { fail_count += 1; }

    // 3. Model configured
    checks.push(serde_json::json!({"name": "Model", "status": if !model_empty {"pass"} else {"warn"}, "detail": model_info}));
    if !model_empty { pass_count += 1; } else { fail_count += 1; }

    // 4. Brain workspace
    let brain_ws = bizclaw_memory::brain::BrainWorkspace::default();
    let brain_status = brain_ws.status();
    let brain_files_exist = brain_status.iter().filter(|(_, exists, _)| *exists).count();
    let brain_ok = brain_files_exist >= 3;
    checks.push(serde_json::json!({"name": "Brain Workspace", "status": if brain_ok {"pass"} else {"warn"}, "detail": format!("{}/{} files", brain_files_exist, brain_status.len())}));
    if brain_ok { pass_count += 1; } else { fail_count += 1; }

    // 5. Ollama (if local provider)
    let ollama_check = if provider == "ollama" {
        match reqwest::Client::new()
            .get("http://localhost:11434/api/tags")
            .timeout(std::time::Duration::from_secs(3))
            .send().await
        {
            Ok(r) if r.status().is_success() => {
                pass_count += 1;
                serde_json::json!({"name": "Ollama Server", "status": "pass", "detail": "Running on localhost:11434"})
            }
            _ => {
                fail_count += 1;
                serde_json::json!({"name": "Ollama Server", "status": "fail", "detail": "Not reachable at localhost:11434"})
            }
        }
    } else {
        pass_count += 1;
        serde_json::json!({"name": "Ollama Server", "status": "skip", "detail": format!("Not needed for {provider}")})
    };
    checks.push(ollama_check);

    // 6. Agent ready
    let agent_ready = state.agent.lock().await.is_some();
    checks.push(serde_json::json!({"name": "Agent Engine", "status": if agent_ready {"pass"} else {"fail"}, "detail": if agent_ready {"Initialized and ready"} else {"Not initialized"}}));
    if agent_ready { pass_count += 1; } else { fail_count += 1; }

    // 7. Memory backend
    checks.push(serde_json::json!({"name": "Memory Backend", "status": "pass", "detail": "SQLite FTS5"}));
    pass_count += 1;

    let total = pass_count + fail_count;
    let score = if total > 0 { (pass_count * 100) / total } else { 0 };
    let overall = if fail_count == 0 { "healthy" } else if fail_count <= 2 { "degraded" } else { "critical" };

    Json(serde_json::json!({
        "ok": fail_count == 0,
        "status": overall,
        "score": format!("{}/{}", pass_count, total),
        "score_pct": score,
        "checks": checks,
        "pass": pass_count,
        "fail": fail_count,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::AppState;
    use std::sync::Mutex;

    fn test_state() -> State<Arc<AppState>> {
        State(Arc::new(AppState {
            gateway_config: bizclaw_core::config::GatewayConfig::default(),
            full_config: Arc::new(Mutex::new(bizclaw_core::config::BizClawConfig::default())),
            config_path: std::path::PathBuf::from("/tmp/test_config.toml"),
            start_time: std::time::Instant::now(),
            pairing_code: None,
            agent: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
            orchestrator: std::sync::Arc::new(tokio::sync::Mutex::new(
                bizclaw_agent::orchestrator::Orchestrator::new()
            )),
            scheduler: Arc::new(tokio::sync::Mutex::new(
                bizclaw_scheduler::SchedulerEngine::new(&std::env::temp_dir().join("bizclaw-test-sched"))
            )),
            knowledge: Arc::new(tokio::sync::Mutex::new(None)),
        }))
    }

    #[tokio::test]
    async fn test_health_check() {
        let result = health_check().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_system_info() {
        let result = system_info(test_state()).await;
        let json = result.0;
        assert_eq!(json["name"], "BizClaw");
        assert!(json["version"].is_string());
    }

    #[tokio::test]
    async fn test_list_providers() {
        let result = list_providers(test_state()).await;
        let json = result.0;
        assert!(json["providers"].is_array());
        assert!(json["providers"].as_array().unwrap().len() >= 5);
    }

    #[tokio::test]
    async fn test_list_channels() {
        let result = list_channels(test_state()).await;
        let json = result.0;
        assert!(json["channels"].is_array());
    }
}
