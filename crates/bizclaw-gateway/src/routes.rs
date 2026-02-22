//! API route handlers for the gateway.

use axum::{extract::State, Json};
use std::sync::Arc;

use super::server::AppState;

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
        "channels": {
            "telegram": cfg.channel.telegram.as_ref().map(|t| serde_json::json!({
                "enabled": t.enabled,
                "bot_token_set": !t.bot_token.is_empty(),
                "allowed_chat_ids": t.allowed_chat_ids,
            })),
            "zalo": cfg.channel.zalo.as_ref().map(|z| serde_json::json!({
                "enabled": z.enabled,
                "mode": z.mode,
                "cookie_path": z.personal.cookie_path,
                "imei": z.personal.imei,
                "self_listen": z.personal.self_listen,
                "auto_reconnect": z.personal.auto_reconnect,
                "rate_limit": {
                    "max_per_minute": z.rate_limit.max_messages_per_minute,
                    "max_per_hour": z.rate_limit.max_messages_per_hour,
                },
            })),
            "discord": cfg.channel.discord.as_ref().map(|d| serde_json::json!({
                "enabled": d.enabled,
                "bot_token_set": !d.bot_token.is_empty(),
                "allowed_channel_ids": d.allowed_channel_ids,
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

    // Save to disk
    let content = toml::to_string_pretty(&*cfg).unwrap_or_default();
    match std::fs::write(&state.config_path, &content) {
        Ok(_) => {
            tracing::info!("✅ Config saved to {}", state.config_path.display());
            Json(serde_json::json!({"ok": true, "message": "Config saved"}))
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
            let token = req.get("bot_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
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
            let token = req.get("bot_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
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
            {"name": "email", "type": "messaging", "status": "available", "configured": false},
            {"name": "webhook", "type": "api", "status": "available", "configured": false},
            {"name": "whatsapp", "type": "messaging", "status": "available", "configured": false},
        ]
    }))
}

/// Generate Zalo QR code for login.
pub async fn zalo_qr_code(
    State(_state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    use bizclaw_channels::zalo::client::auth::{ZaloAuth, ZaloCredentials};

    let creds = ZaloCredentials::default();
    let imei = creds.imei.clone();
    let auth = ZaloAuth::new(creds);

    match auth.get_qr_code().await {
        Ok(qr_data) => Json(serde_json::json!({
            "ok": true,
            "qr_code": qr_data,
            "imei": imei,
            "message": "Mở Zalo trên điện thoại → Quét mã QR để đăng nhập",
            "instructions": [
                "1. Mở ứng dụng Zalo trên điện thoại",
                "2. Vào Cài đặt → Đăng nhập trên thiết bị khác",
                "3. Quét mã QR hiển thị bên dưới",
                "4. Sau khi quét xong, cookie sẽ được lưu tự động"
            ]
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e.to_string(),
            "fallback": "Bạn có thể paste Zalo cookie trực tiếp. Cách lấy: Vào chat.zalo.me → F12 → Application → Cookies → Copy tất cả"
        })),
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
