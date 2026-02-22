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
        if let Some(v) = brain.get("threads").and_then(|v| v.as_u64()) {
            cfg.brain.threads = v as u32;
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
