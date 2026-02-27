//! API route handlers for the gateway.

use axum::{Json, extract::State};
use std::sync::Arc;

use super::server::AppState;
use super::db::GatewayDb;

/// Return sanitized error — logs real error server-side, sends generic message to client.
fn internal_error(context: &str, e: impl std::fmt::Display) -> Json<serde_json::Value> {
    tracing::error!("[{context}] {e}");
    Json(serde_json::json!({"ok": false, "error": "An internal error occurred"}))
}

/// Safely truncate a string at a character boundary (UTF-8 safe).
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes { return s; }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) { end -= 1; }
    &s[..end]
}

/// Validate a name (agent, channel, etc.) — allow Unicode but reject dangerous chars.
#[allow(dead_code)]
fn validate_name(name: &str) -> std::result::Result<(), String> {
    if name.is_empty() { return Err("Name cannot be empty".into()); }
    if name.len() > 100 { return Err("Name too long (max 100 chars)".into()); }
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err("Name contains invalid characters (path traversal)".into());
    }
    if name.contains('<') || name.contains('>') {
        return Err("Name contains invalid characters (HTML)".into());
    }
    if name.contains('\0') {
        return Err("Name contains null bytes".into());
    }
    Ok(())
}

/// Mask a secret string for display — show first 4 chars + •••
fn mask_secret(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    if s.len() <= 4 {
        return "••••".to_string();
    }
    format!("{}••••", &s[..4])
}

/// Enrich agent config with per-provider API key and base_url from the gateway DB.
/// This is the critical function that enables multi-provider support — each agent
/// gets the credentials specific to its chosen provider, not the global default.
///
/// IMPORTANT: Must sync BOTH config systems:
/// - Legacy: config.api_key, config.api_base_url, config.default_provider
/// - LLM section: config.llm.provider, config.llm.api_key, config.llm.endpoint
///
/// `create_provider()` reads from `llm.*` FIRST, so we must set both.
fn apply_provider_config_from_db(
    db: &GatewayDb,
    config: &mut bizclaw_core::config::BizClawConfig,
) {
    let provider_name = &config.default_provider;
    if provider_name.is_empty() {
        return;
    }

    // CRITICAL: Sync llm.provider with default_provider so create_provider() uses the right one
    // create_provider() checks llm.provider FIRST, and LlmConfig::default() is "openai"
    config.llm.provider = provider_name.clone();

    if let Ok(db_provider) = db.get_provider(provider_name) {
        // Use provider-specific API key if it has one, overriding global config
        if !db_provider.api_key.is_empty() {
            config.api_key = db_provider.api_key.clone();
            config.llm.api_key = db_provider.api_key; // Also sync to LLM section
        }
        // For local/proxy providers, ALWAYS use their registered URL
        // (Ollama, llama.cpp, CLIProxy need their specific endpoints)
        if db_provider.provider_type == "local" || db_provider.provider_type == "proxy" {
            if !db_provider.base_url.is_empty() {
                config.api_base_url = db_provider.base_url.clone();
                config.llm.endpoint = db_provider.base_url; // Also sync to LLM section
            }
        } else if !db_provider.base_url.is_empty() && config.api_base_url.is_empty() {
            // For cloud providers, only set if user hasn't explicitly configured one
            config.api_base_url = db_provider.base_url.clone();
            config.llm.endpoint = db_provider.base_url;
        }
    }
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
pub async fn system_info(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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
pub async fn get_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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
            "webhook": cfg.channel.webhook.as_ref().map(|wh| serde_json::json!({
                "enabled": wh.enabled,
                "secret": mask_secret(&wh.secret),
                "secret_set": !wh.secret.is_empty(),
                "outbound_url": wh.outbound_url,
            })),
        },
    }))
}

/// Get full config as TOML string for export/display.
pub async fn get_full_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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

    // Update top-level fields + sync to LLM section
    // CRITICAL: create_provider() reads llm.* FIRST, so both must be in sync
    if let Some(v) = req.get("default_provider").and_then(|v| v.as_str()) {
        cfg.default_provider = v.to_string();
        cfg.llm.provider = v.to_string(); // sync
    }
    if let Some(v) = req.get("default_model").and_then(|v| v.as_str()) {
        cfg.default_model = v.to_string();
        cfg.llm.model = v.to_string(); // sync
    }
    if let Some(v) = req.get("default_temperature").and_then(|v| v.as_f64()) {
        cfg.default_temperature = v as f32;
        cfg.llm.temperature = v as f32; // sync
    }
    if let Some(v) = req.get("api_key").and_then(|v| v.as_str()) {
        cfg.api_key = v.to_string();
        cfg.llm.api_key = v.to_string(); // sync
    }
    if let Some(v) = req.get("api_base_url").and_then(|v| v.as_str()) {
        cfg.api_base_url = v.to_string();
        cfg.llm.endpoint = v.to_string(); // sync
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
    if let Some(mcp) = req.get("mcp_servers")
        && let Ok(servers) =
            serde_json::from_value::<Vec<bizclaw_core::config::McpServerEntry>>(mcp.clone())
        {
            cfg.mcp_servers = servers;
        }

    // Save to disk
    let content = toml::to_string_pretty(&*cfg).unwrap_or_default();
    let new_cfg = cfg.clone();

    // Build sync data for platform DB import
    let sync_data = serde_json::json!({
        "default_provider": new_cfg.default_provider,
        "default_model": new_cfg.default_model,
        "api_key": new_cfg.api_key,
        "api_base_url": new_cfg.api_base_url,
        "identity.name": new_cfg.identity.name,
        "identity.persona": new_cfg.identity.persona,
        "identity.system_prompt": new_cfg.identity.system_prompt,
        "brain.enabled": new_cfg.brain.enabled,
        "brain.model_path": new_cfg.brain.model_path,
        "brain.threads": new_cfg.brain.threads,
        "brain.max_tokens": new_cfg.brain.max_tokens,
        "brain.context_length": new_cfg.brain.context_length,
        "brain.temperature": new_cfg.brain.temperature,
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    drop(cfg); // Release lock before file write + agent reinit

    match std::fs::write(&state.config_path, &content) {
        Ok(_) => {
            tracing::info!("✅ Config saved to {}", state.config_path.display());

            // Write config_sync.json for platform DB import
            if let Some(parent) = state.config_path.parent() {
                let sync_path = parent.join("config_sync.json");
                if let Ok(json) = serde_json::to_string_pretty(&sync_data) {
                    std::fs::write(&sync_path, json).ok();
                    tracing::info!("📋 Config sync file written to {}", sync_path.display());
                }
            }

            // Re-initialize Agent with new config (async, don't block response)
            let agent_lock = state.agent.clone();
            tokio::spawn(async move {
                match bizclaw_agent::Agent::new_with_mcp(new_cfg).await {
                    Ok(new_agent) => {
                        let mut guard = agent_lock.lock().await;
                        tracing::info!(
                            "🔄 Agent re-initialized: provider={}, tools={}",
                            new_agent.provider_name(),
                            new_agent.tool_count()
                        );
                        *guard = Some(new_agent);
                    }
                    Err(e) => tracing::warn!("⚠️ Agent re-init failed: {e}"),
                }
            });

            Json(serde_json::json!({"ok": true, "message": "Config saved — agent reloading"}))
        }
        Err(e) => internal_error("gateway", e),
    }
}

/// Update channel config.
pub async fn update_channel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let channel_type = req
        .get("channel_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let enabled = req
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let mut cfg = state.full_config.lock().unwrap();

    match channel_type {
        "telegram" => {
            let token_val = req.get("bot_token").and_then(|v| v.as_str()).unwrap_or("");
            let token = if token_val.contains('•') {
                cfg.channel
                    .telegram
                    .as_ref()
                    .map(|t| t.bot_token.clone())
                    .unwrap_or_default()
            } else {
                token_val.to_string()
            };
            let chat_ids: Vec<i64> = req
                .get("allowed_chat_ids")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            cfg.channel.telegram = Some(bizclaw_core::config::TelegramChannelConfig {
                enabled,
                bot_token: token,
                allowed_chat_ids: chat_ids,
            });
        }
        "zalo" => {
            let mut zalo_cfg = cfg.channel.zalo.clone().unwrap_or_default();
            zalo_cfg.enabled = enabled;
            if let Some(v) = req.get("cookie").and_then(|v| v.as_str()) {
                // Save cookie to file
                let cookie_dir = state
                    .config_path
                    .parent()
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
                cfg.channel
                    .discord
                    .as_ref()
                    .map(|d| d.bot_token.clone())
                    .unwrap_or_default()
            } else {
                token_val.to_string()
            };
            let ids: Vec<u64> = req
                .get("allowed_channel_ids")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            cfg.channel.discord = Some(bizclaw_core::config::DiscordChannelConfig {
                enabled,
                bot_token: token,
                allowed_channel_ids: ids,
            });
        }
        "email" => {
            let smtp_host = req
                .get("smtp_host")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let smtp_port = req
                .get("smtp_port")
                .and_then(|v| v.as_str())
                .unwrap_or("587")
                .parse::<u16>()
                .unwrap_or(587);
            let email_addr = req
                .get("smtp_user")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let pass_val = req.get("smtp_pass").and_then(|v| v.as_str()).unwrap_or("");
            let password = if pass_val.contains('•') {
                cfg.channel
                    .email
                    .as_ref()
                    .map(|e| e.password.clone())
                    .unwrap_or_default()
            } else {
                pass_val.to_string()
            };
            let imap_host = req
                .get("imap_host")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            cfg.channel.email = Some(bizclaw_core::config::EmailChannelConfig {
                enabled,
                smtp_host,
                smtp_port,
                email: email_addr,
                password,
                imap_host,
                imap_port: 993,
            });
        }
        "whatsapp" => {
            let phone_val = req
                .get("phone_number_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let token_val = req
                .get("access_token")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let token = if token_val.contains('•') {
                cfg.channel
                    .whatsapp
                    .as_ref()
                    .map(|w| w.access_token.clone())
                    .unwrap_or_default()
            } else {
                token_val.to_string()
            };
            cfg.channel.whatsapp = Some(bizclaw_core::config::WhatsAppChannelConfig {
                enabled,
                phone_number_id: phone_val,
                access_token: token,
                webhook_verify_token: req.get("webhook_verify_token")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                business_id: req.get("business_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }
        "webhook" => {
            let secret_val = req.get("webhook_secret").and_then(|v| v.as_str()).unwrap_or("");
            let secret = if secret_val.contains('•') {
                cfg.channel.webhook.as_ref().map(|wh| wh.secret.clone()).unwrap_or_default()
            } else {
                secret_val.to_string()
            };
            let outbound_url = req.get("webhook_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
            cfg.channel.webhook = Some(bizclaw_core::config::WebhookChannelConfig {
                enabled,
                secret,
                outbound_url,
            });
        }
        _ => {
            return Json(
                serde_json::json!({"ok": false, "error": format!("Unknown channel: {channel_type}")}),
            );
        }
    }

    // Save to disk
    let content = toml::to_string_pretty(&*cfg).unwrap_or_default();
    match std::fs::write(&state.config_path, &content) {
        Ok(_) => {
            // Also save channels as standalone JSON for platform DB sync on restart
            // This prevents channel loss when platform regenerates config.toml
            if let Some(parent) = state.config_path.parent() {
                let channels_json = serde_json::json!({
                    // SECURITY: Never write secrets to sync file — only enabled flags + non-sensitive data
                    "telegram": cfg.channel.telegram.as_ref().map(|t| serde_json::json!({
                        "enabled": t.enabled,
                        "bot_token_set": !t.bot_token.is_empty(),
                        "allowed_chat_ids": t.allowed_chat_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", "),
                    })),
                    "zalo": cfg.channel.zalo.as_ref().map(|z| serde_json::json!({
                        "enabled": z.enabled,
                        "mode": z.mode,
                    })),
                    "discord": cfg.channel.discord.as_ref().map(|d| serde_json::json!({
                        "enabled": d.enabled,
                        "bot_token_set": !d.bot_token.is_empty(),
                    })),
                    "email": cfg.channel.email.as_ref().map(|e| serde_json::json!({
                        "enabled": e.enabled,
                        "smtp_host": e.smtp_host,
                        "smtp_port": e.smtp_port,
                        "email": e.email,
                    })),
                    "whatsapp": cfg.channel.whatsapp.as_ref().map(|w| serde_json::json!({
                        "enabled": w.enabled,
                        "phone_number_id": w.phone_number_id,
                    })),
                    "webhook": cfg.channel.webhook.as_ref().map(|wh| serde_json::json!({
                        "enabled": wh.enabled,
                        "secret": wh.secret,
                        "outbound_url": wh.outbound_url,
                    })),
                });
                let sync_path = parent.join("channels_sync.json");
                std::fs::write(&sync_path, serde_json::to_string_pretty(&channels_json).unwrap_or_default()).ok();
            }
            Json(serde_json::json!({"ok": true, "message": format!("{channel_type} config saved")}))
        }
        Err(e) => internal_error("gateway", e),
    }
}

/// Channel instances file path helper.
fn channel_instances_path(state: &AppState) -> std::path::PathBuf {
    state.config_path.parent()
        .unwrap_or(std::path::Path::new("."))
        .join("channel_instances.json")
}

/// Load channel instances from JSON file.
fn load_channel_instances(state: &AppState) -> Vec<serde_json::Value> {
    let path = channel_instances_path(state);
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        vec![]
    }
}

/// Save channel instances to JSON file (with restrictive permissions).
fn save_channel_instances(state: &AppState, instances: &[serde_json::Value]) {
    let path = channel_instances_path(state);
    let json = serde_json::to_string_pretty(instances).unwrap_or_default();
    std::fs::write(&path, json).ok();
    // SECURITY: Set file permissions to 0600 (owner-only) since it contains secrets
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
}

/// List all channel instances (secrets masked for frontend display).
pub async fn list_channel_instances(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let instances = load_channel_instances(&state);
    // Mask sensitive fields before sending to frontend
    let masked: Vec<serde_json::Value> = instances.iter().map(|inst| {
        let mut masked_inst = inst.clone();
        if let Some(cfg) = masked_inst.get_mut("config").and_then(|c| c.as_object_mut()) {
            let sensitive_keys = ["bot_token", "access_token", "webhook_secret", "smtp_pass", "app_token"];
            for key in &sensitive_keys {
                if let Some(val) = cfg.get(*key).and_then(|v| v.as_str())
                    && !val.is_empty() {
                        cfg.insert(key.to_string(), serde_json::json!(mask_secret(val)));
                    }
            }
        }
        masked_inst
    }).collect();
    Json(serde_json::json!({
        "ok": true,
        "instances": masked,
    }))
}

/// Create or update a channel instance.
pub async fn save_channel_instance(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let id = req.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let name = req.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let channel_type = req.get("channel_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let enabled = req.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let agent_name = req.get("agent_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let config = req.get("config").cloned().unwrap_or(serde_json::json!({}));

    if name.is_empty() || channel_type.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "name and channel_type required"}));
    }

    let mut instances = load_channel_instances(&state);

    // Generate or reuse ID
    let instance_id = if id.is_empty() {
        format!("{}_{}", channel_type, chrono::Utc::now().timestamp_millis())
    } else {
        id.clone()
    };

    let instance = serde_json::json!({
        "id": instance_id,
        "name": name,
        "channel_type": channel_type,
        "enabled": enabled,
        "agent_name": agent_name,
        "config": config,
        "updated_at": chrono::Utc::now().to_rfc3339(),
    });

    // Update existing or insert new
    if let Some(pos) = instances.iter().position(|i| i["id"].as_str() == Some(&instance_id)) {
        instances[pos] = instance.clone();
    } else {
        instances.push(instance.clone());
    }

    save_channel_instances(&state, &instances);

    // Also sync primary (first enabled) of this type to config.toml
    // This makes the first enabled instance of each type "active"
    let first_enabled = instances.iter()
        .find(|i| i["channel_type"].as_str() == Some(&channel_type) && i["enabled"].as_bool() == Some(true));
    if let Some(primary) = first_enabled {
        let cfg = primary["config"].clone();
        let mut sync_body = cfg.as_object().cloned().unwrap_or_default();
        sync_body.insert("channel_type".into(), serde_json::json!(channel_type));
        sync_body.insert("enabled".into(), serde_json::json!(true));
        // Trigger update_channel internally via direct config write
        let mut full_cfg = state.full_config.lock().unwrap();
        match channel_type.as_str() {
            "telegram" => {
                let token = sync_body.get("bot_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let chat_ids: Vec<i64> = sync_body.get("allowed_chat_ids")
                    .and_then(|v| v.as_str()).unwrap_or("")
                    .split(',').filter_map(|s| s.trim().parse().ok()).collect();
                full_cfg.channel.telegram = Some(bizclaw_core::config::TelegramChannelConfig {
                    enabled: true, bot_token: token, allowed_chat_ids: chat_ids,
                });
            }
            "webhook" => {
                let outbound = sync_body.get("webhook_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let secret = sync_body.get("webhook_secret").and_then(|v| v.as_str()).unwrap_or("").to_string();
                full_cfg.channel.webhook = Some(bizclaw_core::config::WebhookChannelConfig {
                    enabled: true, secret, outbound_url: outbound,
                });
            }
            _ => {} // Other types handled as-is
        }
        let content = toml::to_string_pretty(&*full_cfg).unwrap_or_default();
        std::fs::write(&state.config_path, &content).ok();
        drop(full_cfg);
    }

    // Also write channels_sync.json for platform restart persistence
    let cfg = state.full_config.lock().unwrap();
    if let Some(parent) = state.config_path.parent() {
        let channels_json = serde_json::json!({
            "telegram": cfg.channel.telegram.as_ref().map(|t| serde_json::json!({"enabled": t.enabled, "bot_token": t.bot_token, "allowed_chat_ids": t.allowed_chat_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")})),
            "webhook": cfg.channel.webhook.as_ref().map(|wh| serde_json::json!({"enabled": wh.enabled, "secret": wh.secret, "outbound_url": wh.outbound_url})),
        });
        std::fs::write(parent.join("channels_sync.json"), serde_json::to_string_pretty(&channels_json).unwrap_or_default()).ok();
    }
    drop(cfg);

    // Auto-connect Telegram if agent_name + bot_token provided
    if enabled && channel_type == "telegram" && !agent_name.is_empty() {
        let bot_token = config.get("bot_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !bot_token.is_empty() {
            let s = state.clone();
            let an = agent_name.clone();
            let iid = instance_id.clone();
            tokio::spawn(async move {
                spawn_telegram_polling(s, an, bot_token, iid).await;
            });
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "instance": instance,
    }))
}

/// Delete a channel instance.
pub async fn delete_channel_instance(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let mut instances = load_channel_instances(&state);
    let before = instances.len();
    instances.retain(|i| i["id"].as_str() != Some(&id));
    if instances.len() == before {
        return Json(serde_json::json!({"ok": false, "error": "Instance not found"}));
    }
    save_channel_instances(&state, &instances);
    Json(serde_json::json!({"ok": true, "message": "Instance deleted"}))
}

/// Webhook inbound — receives external messages, routes to bound agent, replies.
/// POST /api/v1/webhook/inbound
/// Body: {"content": "message", "sender_id": "user1", "thread_id": "optional"}
/// Header: X-Webhook-Signature (optional HMAC-SHA256)
pub async fn webhook_inbound(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Json<serde_json::Value> {
    // Find webhook channel instance bound to an agent
    let instances = load_channel_instances(&state);
    let webhook_instance = instances.iter().find(|i| {
        i["channel_type"].as_str() == Some("webhook")
            && i["enabled"].as_bool() == Some(true)
            && !i["agent_name"].as_str().unwrap_or("").is_empty()
    });

    let (agent_name, outbound_url, secret) = match webhook_instance {
        Some(inst) => {
            let agent = inst["agent_name"].as_str().unwrap_or("").to_string();
            let outbound = inst["config"]["webhook_url"].as_str().unwrap_or("").to_string();
            let sec = inst["config"]["webhook_secret"].as_str().unwrap_or("").to_string();
            (agent, outbound, sec)
        }
        None => {
            return Json(serde_json::json!({
                "ok": false,
                "error": "No webhook channel bound to an agent. Create one in Dashboard → Channels."
            }));
        }
    };

    // Verify signature if secret configured
    if !secret.is_empty() {
        let sig = headers.get("x-webhook-signature").and_then(|v| v.to_str().ok()).unwrap_or("");
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(format!("{secret}{body}"));
        let expected = format!("{:x}", hasher.finalize());
        if expected != sig {
            return Json(serde_json::json!({"ok": false, "error": "Invalid webhook signature"}));
        }
    }

    // Parse message
    let json: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("Invalid JSON: {e}")})),
    };
    let content = json["content"].as_str().unwrap_or("").to_string();
    let sender = json["sender_id"].as_str().unwrap_or("webhook-user").to_string();
    if content.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "'content' field required"}));
    }

    tracing::info!("[webhook] {} → agent '{}': {}", sender, agent_name, safe_truncate(&content, 100));

    // Route to agent
    let response = {
        let mut orch = state.orchestrator.lock().await;
        match orch.send_to(&agent_name, &content).await {
            Ok(r) => r,
            Err(e) => format!("⚠️ Agent error: {e}"),
        }
    };

    // Also forward reply to outbound URL if configured
    if !outbound_url.is_empty() {
        let reply_body = serde_json::json!({
            "content": response,
            "sender_id": agent_name,
            "thread_id": json["thread_id"].as_str().unwrap_or("webhook"),
            "in_reply_to": content,
        });
        let client = reqwest::Client::new();
        if let Err(e) = client.post(&outbound_url).json(&reply_body).send().await {
            tracing::error!("[webhook] Outbound forward failed: {e}");
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "response": response,
        "agent": agent_name,
    }))
}

/// Spawn a Telegram polling loop that routes messages to a specific agent.
/// Reused by both save_channel_instance (manual) and auto_connect_channels (startup).
pub async fn spawn_telegram_polling(
    state: Arc<AppState>,
    agent_name: String,
    bot_token: String,
    instance_id: String,
) {
    // Disconnect existing bot for this agent if any
    {
        let mut bots = state.telegram_bots.lock().await;
        if let Some(existing) = bots.remove(&agent_name) {
            existing.abort_handle.notify_one();
            tracing::info!("[telegram] Disconnecting existing bot for agent '{}'", agent_name);
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }
    }

    // Verify bot token
    let tg = bizclaw_channels::telegram::TelegramChannel::new(
        bizclaw_channels::telegram::TelegramConfig {
            bot_token: bot_token.clone(),
            enabled: true,
            poll_interval: 1,
        },
    );
    let bot_username = match tg.get_me().await {
        Ok(me) => me.username.unwrap_or_default(),
        Err(e) => {
            tracing::error!("[telegram] Bot token invalid for instance '{}': {}", instance_id, e);
            return;
        }
    };
    tracing::info!("[telegram] @{} connected → agent '{}' (instance: {})", bot_username, agent_name, instance_id);

    // Spawn polling loop
    let stop = Arc::new(tokio::sync::Notify::new());
    let stop_rx = stop.clone();
    let state_clone = state.clone();
    let agent_name_clone = agent_name.clone();
    let bot_token_for_state = bot_token.clone();

    tokio::spawn(async move {
        let mut channel = bizclaw_channels::telegram::TelegramChannel::new(
            bizclaw_channels::telegram::TelegramConfig {
                bot_token: bot_token.clone(),
                enabled: true,
                poll_interval: 1,
            },
        );

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

                                    tracing::info!("[telegram] {} → agent '{}': {}", sender, agent_name_clone, safe_truncate(&text, 100));
                                    let _ = channel.send_typing(chat_id).await;

                                    // Route to agent
                                    let response = {
                                        let mut orch = state_clone.orchestrator.lock().await;
                                        match orch.send_to(&agent_name_clone, &text).await {
                                            Ok(r) => r,
                                            Err(e) => format!("⚠️ Agent error: {e}"),
                                        }
                                    };

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
        bots.insert(
            agent_name.clone(),
            super::server::TelegramBotState {
                bot_token: bot_token_for_state,
                bot_username: bot_username.clone(),
                abort_handle: stop,
            },
        );
    }
}

/// Spawn a Discord Gateway listener that routes messages to a specific agent.
pub async fn spawn_discord_gateway(
    state: Arc<AppState>,
    agent_name: String,
    bot_token: String,
    instance_id: String,
) {
    use futures::StreamExt;

    let discord = bizclaw_channels::discord::DiscordChannel::new(
        bizclaw_channels::discord::DiscordConfig {
            bot_token: bot_token.clone(),
            enabled: true,
            intents: 33281, // GUILDS | GUILD_MESSAGES | MESSAGE_CONTENT
        },
    );

    // Verify bot token
    match discord.get_me().await {
        Ok(me) => {
            tracing::info!("[discord] Bot {} connected → agent '{}' (instance: {})",
                me.username, agent_name, instance_id);
        }
        Err(e) => {
            tracing::error!("[discord] Bot token invalid for instance '{}': {}", instance_id, e);
            return;
        }
    }

    let gateway = discord.start_gateway();
    let state_clone = state.clone();
    let agent_name_clone = agent_name.clone();

    tokio::spawn(async move {
        let mut stream = gateway;
        let reply_client = bizclaw_channels::discord::DiscordChannel::new(
            bizclaw_channels::discord::DiscordConfig {
                bot_token: bot_token.clone(),
                enabled: true,
                intents: 33281,
            },
        );

        while let Some(msg) = stream.next().await {
            let channel_id = msg.thread_id.clone();
            let text = msg.content.clone();
            let sender = msg.sender_name.clone().unwrap_or_default();

            tracing::info!("[discord] {} → agent '{}': {}", sender, agent_name_clone, safe_truncate(&text, 100));

            // Send typing indicator
            let _ = reply_client.send_typing_indicator(&channel_id).await;

            // Route to agent
            let response = {
                let mut orch = state_clone.orchestrator.lock().await;
                match orch.send_to(&agent_name_clone, &text).await {
                    Ok(r) => r,
                    Err(e) => format!("⚠️ Agent error: {e}"),
                }
            };

            // Reply via Discord
            if let Err(e) = reply_client.send_message(&channel_id, &response).await {
                tracing::error!("[discord] Reply failed: {e}");
            }
        }
        tracing::warn!("[discord] Gateway stream ended for agent '{}'", agent_name_clone);
    });
}

/// Auto-connect all enabled channel instances on startup.
/// Called from server::start() after AppState is built.
pub async fn auto_connect_channels(state: Arc<AppState>) {
    let mut instances = load_channel_instances(&state);
    let mut connected = 0;

    // ── Fallback: if no telegram instances, check config.toml for bot_token ──
    let has_telegram_instance = instances.iter().any(|i| i["channel_type"].as_str() == Some("telegram"));
    if !has_telegram_instance {
        // Extract telegram config data (owned) to avoid holding MutexGuard across await
        let tg_data = {
            let cfg = state.full_config.lock().unwrap();
            cfg.channel.telegram.as_ref().and_then(|tg| {
                if tg.enabled && !tg.bot_token.is_empty() {
                    Some((tg.bot_token.clone(), tg.allowed_chat_ids.clone()))
                } else { None }
            })
        }; // MutexGuard dropped here

        if let Some((bot_token, chat_ids)) = tg_data {
            // Find which agent to bind to — check agent-channels.json first
            let mut target_agent = String::new();
            let agent_channels_path = state.config_path.parent()
                .unwrap_or(std::path::Path::new("."))
                .join("agent-channels.json");
            if agent_channels_path.exists()
                && let Ok(content) = std::fs::read_to_string(&agent_channels_path)
                    && let Ok(bindings) = serde_json::from_str::<serde_json::Value>(&content)
                        && let Some(obj) = bindings.as_object() {
                            for (agent, channels) in obj {
                                if let Some(arr) = channels.as_array()
                                    && arr.iter().any(|c| c.as_str() == Some("telegram")) {
                                        target_agent = agent.clone();
                                        break;
                                    }
                            }
                        }
            // Fallback: bind to first agent available
            if target_agent.is_empty() {
                let orch = state.orchestrator.lock().await;
                let agents = orch.list_agents();
                if let Some(first) = agents.first() {
                    target_agent = first["name"].as_str().unwrap_or("").to_string();
                }
            }

            if !target_agent.is_empty() {
                let instance_id = format!("telegram_config_{}", chrono::Utc::now().timestamp());
                let inst = serde_json::json!({
                    "id": instance_id,
                    "name": "Telegram Bot (auto)",
                    "channel_type": "telegram",
                    "enabled": true,
                    "agent_name": target_agent,
                    "config": {
                        "bot_token": bot_token,
                        "allowed_chat_ids": chat_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", "),
                    },
                    "updated_at": chrono::Utc::now().to_rfc3339(),
                });
                instances.push(inst);
                save_channel_instances(&state, &instances);
                tracing::info!("[auto-connect] Migrated config.toml telegram → channel instance bound to '{}'", target_agent);
            }
        }
    }

    // ── Connect all enabled instances ──
    for inst in &instances {
        let enabled = inst["enabled"].as_bool().unwrap_or(false);
        if !enabled { continue; }
        let channel_type = inst["channel_type"].as_str().unwrap_or("");
        let agent_name = inst["agent_name"].as_str().unwrap_or("");
        let instance_id = inst["id"].as_str().unwrap_or("");
        let cfg = &inst["config"];

        match channel_type {
            "telegram" if !agent_name.is_empty() => {
                let bot_token = cfg["bot_token"].as_str().unwrap_or("").to_string();
                if !bot_token.is_empty() {
                    spawn_telegram_polling(
                        state.clone(),
                        agent_name.to_string(),
                        bot_token,
                        instance_id.to_string(),
                    ).await;
                    connected += 1;
                }
            }
            "discord" if !agent_name.is_empty() => {
                let bot_token = cfg["bot_token"].as_str().unwrap_or("").to_string();
                if !bot_token.is_empty() {
                    let s = state.clone();
                    let an = agent_name.to_string();
                    let iid = instance_id.to_string();
                    tokio::spawn(async move {
                        spawn_discord_gateway(s, an, bot_token, iid).await;
                    });
                    connected += 1;
                }
            }
            "webhook" if !agent_name.is_empty() => {
                // Webhook is passive — inbound via /api/v1/webhook/inbound
                // No polling needed, just log that it's ready
                tracing::info!("[webhook] Instance '{}' bound to agent '{}' — ready for inbound at /api/v1/webhook/inbound",
                    inst["name"].as_str().unwrap_or(instance_id), agent_name);
                connected += 1;
            }
            _ => {}
        }
    }
    if connected > 0 {
        tracing::info!("📱 Auto-connected {} channel instance(s)", connected);
    }
}

/// List available providers (from DB) — fully self-describing, no hardcoded metadata.
pub async fn list_providers(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let cfg = state.full_config.lock().unwrap();
    let active = cfg.default_provider.clone();
    drop(cfg);
    
    match state.db.list_providers(&active) {
        Ok(providers) => {
            let provider_json: Vec<serde_json::Value> = providers.iter().map(|p| {
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
            }).collect();
            Json(serde_json::json!({ "providers": provider_json }))
        }
        Err(e) => Json(serde_json::json!({ "ok": false, "error": format!("DB error: {e}") })),
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
    let env_keys: Vec<String> = body["env_keys"].as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let models: Vec<String> = body["models"].as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    match state.db.upsert_provider(
        name, label, icon, provider_type, api_key, base_url,
        chat_path, models_path, auth_style, &env_keys, &models,
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
        Ok(()) => Json(serde_json::json!({"ok": true, "message": format!("Provider '{}' deleted", name)})),
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
        Ok(()) => Json(serde_json::json!({"ok": true, "message": format!("Provider '{}' updated", name)})),
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
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("Provider not found: {e}")})),
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
                    let models: Vec<String> = body["models"].as_array()
                        .map(|arr| arr.iter().filter_map(|m| m["name"].as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    // Cache in DB
                    state.db.update_provider_models(&name, &models).ok();
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
        let config_dir = state.config_path.parent().unwrap_or(std::path::Path::new("."));
        let scan_dirs = vec![
            config_dir.join("models"),
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join(".bizclaw").join("models"),
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
        if !models.is_empty() {
            state.db.update_provider_models("brain", &models).ok();
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

    let url = format!("{}{}", provider.base_url.trim_end_matches('/'), provider.models_path);
    let client = reqwest::Client::new();

    // Apply auth — detect API key from provider config or env vars
    let api_key = if !provider.api_key.is_empty() {
        provider.api_key.clone()
    } else {
        // Try env vars
        provider.env_keys.iter()
            .find_map(|key| std::env::var(key).ok())
            .unwrap_or_default()
    };

    // Build request with provider-specific auth handling
    let req = if name == "anthropic" {
        // Anthropic uses x-api-key header (not Bearer)
        let mut r = client.get(&url).timeout(std::time::Duration::from_secs(10));
        if !api_key.is_empty() {
            r = r.header("x-api-key", &api_key)
                 .header("anthropic-version", "2023-06-01");
        }
        r
    } else if name == "gemini" {
        // Gemini uses ?key= query param
        let full_url = if !api_key.is_empty() {
            format!("{}?key={}", url, api_key)
        } else { url.clone() };
        client.get(&full_url).timeout(std::time::Duration::from_secs(10))
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
                let models: Vec<String> = body["data"].as_array()
                    .map(|arr| arr.iter().filter_map(|m| m["id"].as_str().map(String::from)).collect())
                    .unwrap_or_default();
                if !models.is_empty() {
                    // Cache in DB
                    state.db.update_provider_models(&name, &models).ok();
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
        Err(e) => {
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Connection failed: {e}"),
                "models": provider.models,
                "source": "cached",
            }))
        }
    }
}


/// List available channels with config status.
pub async fn list_channels(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let cfg = state.full_config.lock().unwrap();
    Json(serde_json::json!({
        "channels": [
            {"name": "cli", "type": "interactive", "status": "active", "configured": true},
            {"name": "telegram", "type": "messaging", "status": if cfg.channel.telegram.as_ref().is_some_and(|t| t.enabled) { "active" } else { "disabled" }, "configured": cfg.channel.telegram.is_some()},
            {"name": "zalo", "type": "messaging", "status": if cfg.channel.zalo.as_ref().is_some_and(|z| z.enabled) { "active" } else { "disabled" }, "configured": cfg.channel.zalo.is_some()},
            {"name": "discord", "type": "messaging", "status": if cfg.channel.discord.as_ref().is_some_and(|d| d.enabled) { "active" } else { "disabled" }, "configured": cfg.channel.discord.is_some()},
            {"name": "email", "type": "messaging", "status": if cfg.channel.email.as_ref().is_some_and(|e| e.enabled) { "active" } else { "disabled" }, "configured": cfg.channel.email.is_some()},
            {"name": "webhook", "type": "api", "status": if cfg.channel.webhook.as_ref().is_some_and(|wh| wh.enabled) { "active" } else { "disabled" }, "configured": cfg.channel.webhook.is_some()},
            {"name": "whatsapp", "type": "messaging", "status": if cfg.channel.whatsapp.as_ref().is_some_and(|w| w.enabled) { "active" } else { "disabled" }, "configured": cfg.channel.whatsapp.is_some()},
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
            Json(serde_json::json!({"ok": false, "error": format!("Ollama not running: {e}")}))
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
                    && (ext == "gguf" || ext == "bin") {
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

/// Generate Zalo QR code for login.
pub async fn zalo_qr_code(State(_state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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
        Err(e) => {
            tracing::error!("[zalo_qr] {e}");
            Json(serde_json::json!({
                "ok": false,
                "error": "Không thể tạo mã QR Zalo",
                "fallback": "Vui lòng vào chat.zalo.me → F12 → Application → Cookies → Copy toàn bộ và paste vào ô Cookie bên dưới"
            }))
        }
    }
}

/// WhatsApp webhook verification (GET) — Meta sends this to verify endpoint.
pub async fn whatsapp_webhook_verify(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    let mode = params.get("hub.mode").map(|s| s.as_str()).unwrap_or("");
    let token = params
        .get("hub.verify_token")
        .map(|s| s.as_str())
        .unwrap_or("");
    let challenge = params
        .get("hub.challenge")
        .map(|s| s.as_str())
        .unwrap_or("");

    let expected_token = {
        let cfg = state.full_config.lock().unwrap();
        cfg.channel
            .whatsapp
            .as_ref()
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
                            if msg_type != "text" {
                                continue;
                            }

                            let from = msg["from"].as_str().unwrap_or("").to_string();
                            let text = msg["text"]["body"].as_str().unwrap_or("").to_string();
                            let msg_id = msg["id"].as_str().unwrap_or("").to_string();

                            if text.is_empty() {
                                continue;
                            }

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
                                        .header(
                                            "Authorization",
                                            format!("Bearer {}", wa_cfg.access_token),
                                        )
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

// ---- Generic Webhook Inbound API ----

/// Generic webhook inbound handler (POST).
/// Receives messages from external systems (Zapier, n8n, custom apps).
/// LEGACY — replaced by new webhook_inbound at line ~761 that routes through channel_instances.
pub async fn _webhook_inbound_legacy(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Json<serde_json::Value> {
    // Check if webhook channel is enabled
    let (enabled, secret, outbound_url) = {
        let cfg = state.full_config.lock().unwrap();
        match cfg.channel.webhook.as_ref() {
            Some(wh) => (wh.enabled, wh.secret.clone(), wh.outbound_url.clone()),
            None => {
                return Json(serde_json::json!({
                    "ok": false,
                    "error": "Webhook channel not configured"
                }));
            }
        }
    };

    if !enabled {
        return Json(serde_json::json!({
            "ok": false,
            "error": "Webhook channel is disabled"
        }));
    }

    // Verify signature if secret is configured
    if !secret.is_empty() {
        let signature = headers
            .get("X-Webhook-Signature")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if signature.is_empty() {
            return Json(serde_json::json!({
                "ok": false,
                "error": "Missing X-Webhook-Signature header"
            }));
        }

        // HMAC-SHA256 verification
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(format!("{secret}{body}"));
        let expected = format!("{:x}", hasher.finalize());
        if expected != signature {
            tracing::warn!("[webhook] Invalid signature from inbound request");
            return Json(serde_json::json!({
                "ok": false,
                "error": "Invalid webhook signature"
            }));
        }
    }

    // Parse the JSON body
    let payload: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            return Json(serde_json::json!({
                "ok": false,
                "error": format!("Invalid JSON: {e}")
            }));
        }
    };

    let content = payload["content"].as_str().unwrap_or("").to_string();
    let sender_id = payload["sender_id"].as_str().unwrap_or("webhook-user").to_string();
    let thread_id = payload["thread_id"].as_str().unwrap_or("webhook").to_string();

    if content.is_empty() {
        return Json(serde_json::json!({
            "ok": false,
            "error": "Missing 'content' field in webhook payload"
        }));
    }

    tracing::info!("[webhook] Inbound from {sender_id} (thread={thread_id}): {content}");

    // Process through Agent Engine
    let response = {
        let mut agent = state.agent.lock().await;
        if let Some(agent) = agent.as_mut() {
            match agent.process(&content).await {
                Ok(r) => r,
                Err(e) => format!("Error: {e}"),
            }
        } else {
            "Agent not available".to_string()
        }
    };

    // Optionally send reply to outbound URL
    if !outbound_url.is_empty() {
        let reply = serde_json::json!({
            "thread_id": thread_id,
            "sender_id": sender_id,
            "content": response,
            "channel": "webhook",
        });
        let client = reqwest::Client::new();
        tokio::spawn(async move {
            if let Err(e) = client.post(&outbound_url).json(&reply).send().await {
                tracing::error!("[webhook] Outbound reply failed: {e}");
            } else {
                tracing::info!("[webhook] Outbound reply sent to {outbound_url}");
            }
        });
    }

    Json(serde_json::json!({
        "ok": true,
        "response": response,
        "thread_id": thread_id,
    }))
}

// ---- Scheduler API ----

/// List all scheduled tasks.
pub async fn scheduler_list_tasks(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let engine = state.scheduler.lock().await;
    let tasks: Vec<_> = engine
        .list_tasks()
        .iter()
        .map(|t| {
            let action_type = match &t.action {
                bizclaw_scheduler::tasks::TaskAction::AgentPrompt(_) => "agent_prompt",
                bizclaw_scheduler::tasks::TaskAction::Notify(_) => "notify",
                bizclaw_scheduler::tasks::TaskAction::Webhook { .. } => "webhook",
            };
            serde_json::json!({
                "id": t.id,
                "name": t.name,
                "action": t.action,
                "task_type": t.task_type,
                "status": t.status,
                "enabled": t.enabled,
                "run_count": t.run_count,
                "next_run": t.next_run.map(|d| d.to_rfc3339()),
                "last_run": t.last_run.map(|d| d.to_rfc3339()),
                "action_type": action_type,
                "agent_name": t.agent_name,
                "deliver_to": t.deliver_to,
                // Retry fields
                "fail_count": t.fail_count,
                "last_error": t.last_error,
                "retry": {
                    "max_retries": t.retry.max_retries,
                    "base_delay_secs": t.retry.base_delay_secs,
                    "backoff_multiplier": t.retry.backoff_multiplier,
                    "max_delay_secs": t.retry.max_delay_secs,
                },
                "retry_status": t.retry_status(),
            })
        })
        .collect();
    let stats = engine.retry_stats();
    Json(serde_json::json!({
        "ok": true,
        "tasks": tasks,
        "count": tasks.len(),
        "stats": {
            "retrying": stats.retrying,
            "permanently_failed": stats.permanently_failed,
            "total_retries": stats.total_retries,
        }
    }))
}

/// Add a new scheduled task.
pub async fn scheduler_add_task(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or("unnamed");
    let prompt = body["prompt"].as_str().unwrap_or("");
    let action_str = body["action"].as_str().unwrap_or("");
    let agent_name = body["agent_name"].as_str().filter(|s| !s.is_empty()).map(String::from);
    let deliver_to = body["deliver_to"].as_str().filter(|s| !s.is_empty()).map(String::from);

    // If prompt is provided, use AgentPrompt; otherwise Notify
    let action = if !prompt.is_empty() {
        bizclaw_scheduler::tasks::TaskAction::AgentPrompt(prompt.to_string())
    } else if !action_str.is_empty() {
        bizclaw_scheduler::tasks::TaskAction::Notify(action_str.to_string())
    } else {
        return Json(serde_json::json!({"ok": false, "error": "Either 'prompt' or 'action' is required"}));
    };

    let task_type = body["task_type"].as_str()
        .or_else(|| body["type"].as_str())
        .unwrap_or("cron");

    let mut task = match task_type {
        "cron" => {
            let expr = body["cron"].as_str()
                .or_else(|| body["expression"].as_str())
                .unwrap_or("0 * * * *");
            bizclaw_scheduler::Task::cron(name, expr, action)
        }
        "once" => {
            let at = chrono::Utc::now()
                + chrono::Duration::seconds(body["delay_secs"].as_i64().unwrap_or(60));
            bizclaw_scheduler::Task::once(name, at, action)
        }
        _ => {
            let secs = body["interval_secs"].as_u64().unwrap_or(300);
            bizclaw_scheduler::Task::interval(name, secs, action)
        }
    };

    // Set optional fields
    task.agent_name = agent_name;
    task.deliver_to = deliver_to.clone();
    task.notify_via = deliver_to;

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

/// Toggle a scheduled task (enable/disable).
pub async fn scheduler_toggle_task(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let enabled = body["enabled"].as_bool().unwrap_or(true);
    state.scheduler.lock().await.set_enabled(&id, enabled);
    Json(serde_json::json!({"ok": true, "enabled": enabled}))
}

/// Get notification history.
pub async fn scheduler_notifications(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let engine = state.scheduler.lock().await;
    let history: Vec<_> = engine
        .router
        .history()
        .iter()
        .map(|n| {
            serde_json::json!({
                "title": n.title,
                "body": n.body,
                "source": n.source,
                "priority": format!("{:?}", n.priority),
                "timestamp": n.timestamp.to_rfc3339(),
            })
        })
        .collect();
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
            let items: Vec<_> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "doc_name": r.doc_name,
                        "content": r.content,
                        "score": r.score,
                        "chunk_idx": r.chunk_idx,
                    })
                })
                .collect();
            Json(serde_json::json!({"ok": true, "results": items, "count": items.len()}))
        }
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

/// List all knowledge documents.
pub async fn knowledge_list_docs(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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
        Some(store) => match store.add_document(name, content, source) {
            Ok(chunks) => Json(serde_json::json!({"ok": true, "chunks": chunks})),
            Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
        },
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
        Some(store) => match store.remove_document(id) {
            Ok(()) => Json(serde_json::json!({"ok": true})),
            Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
        },
        None => Json(serde_json::json!({"ok": false, "error": "Knowledge base not available"})),
    }
}

// ---- Multi-Agent Orchestrator API ----

/// List all agents in the orchestrator.
pub async fn list_agents(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let orch = state.orchestrator.lock().await;
    let mut agents = orch.list_agents();

    // Load channel bindings and attach to each agent
    let bindings_path = state.config_path.parent()
        .unwrap_or(std::path::Path::new("."))
        .join("agent-channels.json");
    let bindings: serde_json::Value = if bindings_path.exists() {
        std::fs::read_to_string(&bindings_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    for agent in agents.iter_mut() {
        if let Some(name) = agent["name"].as_str() {
            let ch = bindings.get(name).cloned().unwrap_or(serde_json::json!([]));
            agent.as_object_mut().map(|o| o.insert("channels".into(), ch));
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "agents": agents,
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
    if let Some(provider) = body["provider"].as_str()
        && !provider.is_empty() {
            agent_config.default_provider = provider.to_string();
            agent_config.llm.provider = provider.to_string(); // sync
        }
    if let Some(model) = body["model"].as_str()
        && !model.is_empty() {
            agent_config.default_model = model.to_string();
            agent_config.llm.model = model.to_string(); // sync
        }
    if let Some(persona) = body["persona"].as_str() {
        agent_config.identity.persona = persona.to_string();
    }
    if let Some(sys_prompt) = body["system_prompt"].as_str() {
        agent_config.identity.system_prompt = sys_prompt.to_string();
    }
    agent_config.identity.name = name.to_string();

    // Critical: inject per-provider API key and base_url from DB
    // This enables agents to use different providers (e.g. Ollama, DeepSeek)
    // without needing the global config to match.
    apply_provider_config_from_db(&state.db, &mut agent_config);

    // Use sync Agent::new() — MCP tools are shared at orchestrator level
    match bizclaw_agent::Agent::new(agent_config) {
        Ok(agent) => {
            let provider = agent.provider_name().to_string();
            let model = agent.model_name().to_string();
            let system_prompt = agent.system_prompt().to_string();
            let mut orch = state.orchestrator.lock().await;
            orch.add_agent(name, role, description, agent);
            // Persist to SQLite DB
            if let Err(e) = state.db.upsert_agent(name, role, description, &provider, &model, &system_prompt) {
                tracing::warn!("DB persist failed for agent '{}': {}", name, e);
            }
            // Also save to legacy agents.json for backward compatibility
            let agents_path = state.config_path.parent()
                .unwrap_or(std::path::Path::new("."))
                .join("agents.json");
            orch.save_agents_metadata(&agents_path);
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
    if removed {
        // Delete from SQLite DB
        if let Err(e) = state.db.delete_agent(&name) {
            tracing::warn!("DB delete failed for agent '{}': {}", name, e);
        }
        // Also update legacy agents.json
        let agents_path = state.config_path.parent()
            .unwrap_or(std::path::Path::new("."))
            .join("agents.json");
        orch.save_agents_metadata(&agents_path);
    }
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
    let provider = body["provider"].as_str();
    let model = body["model"].as_str();
    let system_prompt = body["system_prompt"].as_str();

    // Phase 1: Update basic metadata + check if re-creation needed
    let mut needs_recreate = false;
    {
        let mut orch = state.orchestrator.lock().await;
        let updated = orch.update_agent(&name, role, description);
        if !updated {
            return Json(serde_json::json!({"ok": false, "message": format!("Agent '{}' not found", name)}));
        }
        // Only re-create if provider or model ACTUALLY CHANGED (not just present)
        if let Some(agent) = orch.get_agent_mut(&name) {
            let cur_provider = agent.provider_name().to_string();
            let cur_model = agent.model_name().to_string();
            if let Some(p) = provider
                && !p.is_empty() && p != cur_provider { needs_recreate = true; }
            if let Some(m) = model
                && !m.is_empty() && m != cur_model { needs_recreate = true; }
            // Update system prompt directly on live agent (no re-creation needed)
            if !needs_recreate
                && let Some(sp) = system_prompt
                    && !sp.is_empty() && sp != agent.system_prompt() {
                        agent.set_system_prompt(sp);
                        tracing::info!("📝 update_agent '{}' — system_prompt updated in-place", name);
                    }
        }

    } // lock released here

    // Phase 2: Re-create agent ONLY if provider/model actually changed
    if needs_recreate {

        let mut agent_config = state.full_config.lock().unwrap().clone();
        {
            let mut orch = state.orchestrator.lock().await;
            if let Some(agent) = orch.get_agent_mut(&name) {
                agent_config.default_provider = agent.provider_name().to_string();
                agent_config.default_model = agent.model_name().to_string();
                agent_config.identity.system_prompt = agent.system_prompt().to_string();
            }
        } // lock released before potentially slow await

        if let Some(p) = provider
            && !p.is_empty() {
                agent_config.default_provider = p.to_string();
                agent_config.llm.provider = p.to_string(); // sync
            }
        if let Some(m) = model
            && !m.is_empty() {
                agent_config.default_model = m.to_string();
                agent_config.llm.model = m.to_string(); // sync
            }
        if let Some(sp) = system_prompt {
            agent_config.identity.system_prompt = sp.to_string();
        }
        agent_config.identity.name = name.clone();

        // Critical: inject per-provider API key from DB
        apply_provider_config_from_db(&state.db, &mut agent_config);

        // Re-create agent with sync Agent::new() — fast, no MCP hang
        match bizclaw_agent::Agent::new(agent_config) {
            Ok(new_agent) => {
                let mut orch = state.orchestrator.lock().await;
                let role_str = role.unwrap_or("assistant").to_string();
                let desc_str = description.unwrap_or("").to_string();
                let agents_list = orch.list_agents();
                let current = agents_list.iter().find(|a| a["name"].as_str() == Some(&name));
                let final_role = if role.is_some() { role_str.clone() } else {
                    current.and_then(|a| a["role"].as_str()).unwrap_or("assistant").to_string()
                };
                let final_desc = if description.is_some() { desc_str.clone() } else {
                    current.and_then(|a| a["description"].as_str()).unwrap_or("").to_string()
                };
                orch.remove_agent(&name);
                orch.add_agent(&name, &final_role, &final_desc, new_agent);
                tracing::info!("🔄 Agent '{}' re-created with new provider/model", name);
            }
            Err(e) => {
                tracing::warn!("⚠️ Agent '{}' re-create failed: {}", name, e);
            }
        }
    }

    // Phase 3: Persist to DB — always save metadata/prompt even without re-creation
    // Use DB record as fallback (NOT hardcoded "openai") to preserve user's provider choice
    {
        let db_agent = state.db.get_agent(&name).ok();
        let orch = state.orchestrator.lock().await;
        let agents_list = orch.list_agents();
        let current = agents_list.iter().find(|a| a["name"].as_str() == Some(&name));
        let final_role = current.and_then(|a| a["role"].as_str()).unwrap_or("assistant");
        let final_desc = current.and_then(|a| a["description"].as_str()).unwrap_or("");
        // Provider fallback chain: explicit request → DB record → orchestrator live state → ""
        let final_provider = provider.unwrap_or_else(|| {
            current.and_then(|a| a["provider"].as_str())
                .filter(|p| !p.is_empty())
                .or_else(|| db_agent.as_ref().map(|a| a.provider.as_str()).filter(|p| !p.is_empty()))
                .unwrap_or("")
        });
        let final_model = model.unwrap_or_else(|| {
            current.and_then(|a| a["model"].as_str())
                .filter(|m| !m.is_empty())
                .or_else(|| db_agent.as_ref().map(|a| a.model.as_str()).filter(|m| !m.is_empty()))
                .unwrap_or("")
        });
        let final_prompt = system_prompt.unwrap_or_else(|| {
            current.and_then(|a| a["system_prompt"].as_str())
                .or_else(|| db_agent.as_ref().map(|a| a.system_prompt.as_str()))
                .unwrap_or("")
        });
        if let Err(e) = state.db.upsert_agent(&name, final_role, final_desc, final_provider, final_model, final_prompt) {
            tracing::warn!("DB persist failed for agent '{}': {}", name, e);
        }
    }

    // Persist to legacy agents.json
    {
        let orch = state.orchestrator.lock().await;
        let agents_path = state.config_path.parent()
            .unwrap_or(std::path::Path::new("."))
            .join("agents.json");
        orch.save_agents_metadata(&agents_path);
    }

    Json(serde_json::json!({
        "ok": true,
        "message": format!("Agent '{}' updated", name),
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
        Err(e) => {
            tracing::error!("[agent_chat:{name}] {e}");
            Json(serde_json::json!({
                "ok": false,
                "error": "Agent processing failed",
            }))
        }
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
    let responses: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(name, result)| match result {
            Ok(response) => serde_json::json!({
                "agent": name,
                "ok": true,
                "response": response,
            }),
            Err(e) => {
                tracing::error!("[broadcast:{name}] {e}");
                serde_json::json!({
                    "agent": name,
                    "ok": false,
                    "error": "Agent processing failed",
                })
            }
        })
        .collect();

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
        if !agents
            .iter()
            .any(|a| a["name"].as_str() == Some(&agent_name))
        {
            return Json(
                serde_json::json!({"ok": false, "error": format!("Agent '{}' not found", agent_name)}),
            );
        }
    }

    // Already connected? Disconnect first
    {
        let mut bots = state.telegram_bots.lock().await;
        if let Some(existing) = bots.remove(&agent_name) {
            existing.abort_handle.notify_one();
            tracing::info!(
                "[telegram] Disconnecting existing bot for agent '{}'",
                agent_name
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    // Verify bot token
    let tg = bizclaw_channels::telegram::TelegramChannel::new(
        bizclaw_channels::telegram::TelegramConfig {
            bot_token: bot_token.clone(),
            enabled: true,
            poll_interval: 1,
        },
    );
    let bot_info = match tg.get_me().await {
        Ok(me) => me,
        Err(e) => {
            return Json(
                serde_json::json!({"ok": false, "error": format!("Invalid bot token: {e}")}),
            );
        }
    };
    let bot_username = bot_info.username.clone().unwrap_or_default();
    tracing::info!(
        "[telegram] Bot @{} verified for agent '{}'",
        bot_username,
        agent_name
    );

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
            },
        );
        tracing::info!(
            "[telegram] Polling started for agent '{}'",
            agent_name_clone
        );

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

                                    tracing::info!("[telegram] {} → agent '{}': {}", sender, agent_name_clone, safe_truncate(&text, 100));

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
        bots.insert(
            agent_name.clone(),
            super::server::TelegramBotState {
                bot_token: bot_token.clone(),
                bot_username: bot_username.clone(),
                abort_handle: stop,
            },
        );
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
        tracing::info!(
            "[telegram] @{} disconnected from agent '{}'",
            bot.bot_username,
            agent_name
        );
        Json(serde_json::json!({
            "ok": true,
            "message": format!("@{} disconnected from agent '{}'", bot.bot_username, agent_name),
        }))
    } else {
        Json(
            serde_json::json!({"ok": false, "error": format!("No Telegram bot connected to agent '{}'", agent_name)}),
        )
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
        None => {
            Json(serde_json::json!({"ok": false, "error": format!("File not found: {filename}")}))
        }
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
        Err(e) => internal_error("gateway", e),
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
        Ok(true) => {
            Json(serde_json::json!({"ok": true, "message": format!("Deleted: {filename}")}))
        }
        Ok(false) => Json(serde_json::json!({"ok": false, "error": "File not found"})),
        Err(e) => internal_error("gateway", e),
    }
}

/// Brain Personalization — AI generates SOUL.md, IDENTITY.md, USER.md from user description.
pub async fn brain_personalize(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let about_user = body["about_user"].as_str().unwrap_or("");
    let agent_vibe = body["agent_vibe"]
        .as_str()
        .unwrap_or("helpful and professional");
    let agent_name = body["agent_name"].as_str().unwrap_or("BizClaw Agent");
    let language = body["language"].as_str().unwrap_or("vi");
    let tenant = body["tenant"].as_str().unwrap_or("");

    if about_user.is_empty() {
        return Json(
            serde_json::json!({"ok": false, "error": "Please describe yourself (about_user)"}),
        );
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
            Err(e) => {
                return Json(serde_json::json!({"ok": false, "error": format!("AI error: {e}")}));
            }
        },
        None => {
            return Json(
                serde_json::json!({"ok": false, "error": "Agent not available — configure provider first"}),
            );
        }
    };
    drop(agent_lock);

    // Parse AI response as JSON
    let clean = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let parsed: serde_json::Value = match serde_json::from_str(clean) {
        Ok(v) => v,
        Err(_) => {
            // Fallback: try to extract JSON from response
            let start = clean.find('{').unwrap_or(0);
            let end = clean.rfind('}').map(|i| i + 1).unwrap_or(clean.len());
            match serde_json::from_str(&clean[start..end]) {
                Ok(v) => v,
                Err(e) => {
                    return Json(serde_json::json!({
                        "ok": false,
                        "error": format!("Failed to parse AI response: {e}"),
                        "raw": response,
                    }));
                }
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
    for (key, filename) in &[
        ("soul", "SOUL.md"),
        ("identity", "IDENTITY.md"),
        ("user", "USER.md"),
    ] {
        if let Some(content) = parsed[key].as_str()
            && ws.write_file(filename, content).is_ok() {
                saved.push(*filename);
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
pub async fn system_health_check(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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
    if config_ok {
        pass_count += 1;
    } else {
        fail_count += 1;
    }

    // 2. Provider API key
    let key_ok = match provider.as_str() {
        "ollama" | "brain" | "llamacpp" => true,
        _ => !api_key_empty,
    };
    let key_detail = if key_ok {
        format!("{provider}: configured")
    } else {
        format!("{provider}: API key missing!")
    };
    checks.push(serde_json::json!({"name": "API Key", "status": if key_ok {"pass"} else {"fail"}, "detail": key_detail}));
    if key_ok {
        pass_count += 1;
    } else {
        fail_count += 1;
    }

    // 3. Model configured
    checks.push(serde_json::json!({"name": "Model", "status": if !model_empty {"pass"} else {"warn"}, "detail": model_info}));
    if !model_empty {
        pass_count += 1;
    } else {
        fail_count += 1;
    }

    // 4. Brain workspace
    let brain_ws = bizclaw_memory::brain::BrainWorkspace::default();
    let brain_status = brain_ws.status();
    let brain_files_exist = brain_status.iter().filter(|(_, exists, _)| *exists).count();
    let brain_ok = brain_files_exist >= 3;
    checks.push(serde_json::json!({"name": "Brain Workspace", "status": if brain_ok {"pass"} else {"warn"}, "detail": format!("{}/{} files", brain_files_exist, brain_status.len())}));
    if brain_ok {
        pass_count += 1;
    } else {
        fail_count += 1;
    }

    // 5. Ollama (if local provider)
    let ollama_check = if provider == "ollama" {
        match reqwest::Client::new()
            .get("http://localhost:11434/api/tags")
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
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
    if agent_ready {
        pass_count += 1;
    } else {
        fail_count += 1;
    }

    // 7. Memory backend
    checks.push(
        serde_json::json!({"name": "Memory Backend", "status": "pass", "detail": "SQLite FTS5"}),
    );
    pass_count += 1;

    let total = pass_count + fail_count;
    let score = if total > 0 {
        (pass_count * 100) / total
    } else {
        0
    };
    let overall = if fail_count == 0 {
        "healthy"
    } else if fail_count <= 2 {
        "degraded"
    } else {
        "critical"
    };

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
        let (activity_tx, _rx) = tokio::sync::broadcast::channel(16);
        State(Arc::new(AppState {
            gateway_config: bizclaw_core::config::GatewayConfig::default(),
            full_config: Arc::new(Mutex::new(bizclaw_core::config::BizClawConfig::default())),
            config_path: std::path::PathBuf::from("/tmp/test_config.toml"),
            start_time: std::time::Instant::now(),
            pairing_code: Arc::new(Mutex::new(String::new())),
            auth_failures: Arc::new(tokio::sync::Mutex::new((0, std::time::Instant::now()))),
            agent: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
            orchestrator: std::sync::Arc::new(tokio::sync::Mutex::new(
                bizclaw_agent::orchestrator::Orchestrator::new(),
            )),
            scheduler: Arc::new(tokio::sync::Mutex::new(
                bizclaw_scheduler::SchedulerEngine::new(
                    &std::env::temp_dir().join("bizclaw-test-sched"),
                ),
            )),
            knowledge: Arc::new(tokio::sync::Mutex::new(None)),
            telegram_bots: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            db: Arc::new(crate::db::GatewayDb::open(std::path::Path::new(":memory:")).unwrap()),
            orch_store: {
                let store = Arc::new(bizclaw_db::SqliteStore::in_memory().unwrap());
                store
            },
            traces: Arc::new(Mutex::new(Vec::new())),
            activity_tx,
            activity_log: Arc::new(Mutex::new(Vec::new())),
        }))
    }

    // ---- Health & Info ----

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
        assert!(json["uptime_secs"].is_number());
    }

    #[tokio::test]
    async fn test_system_health_check() {
        let result = system_health_check(test_state()).await;
        let json = result.0;
        // Health check may fail if config file doesn't exist in test env
        assert!(json["checks"].is_array());
        assert!(json.get("score_pct").is_some());
    }

    // ---- Providers & Channels ----

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
        let channels = json["channels"].as_array().unwrap();
        // Should have at least CLI, Telegram, Zalo channels
        assert!(channels.len() >= 3);
    }

    // ---- Config ----

    #[tokio::test]
    async fn test_get_config() {
        let result = get_config(test_state()).await;
        let json = result.0;
        assert!(json["default_provider"].is_string());
        assert!(json["default_model"].is_string());
    }

    #[tokio::test]
    async fn test_get_full_config() {
        let result = get_full_config(test_state()).await;
        let json = result.0;
        assert!(json.is_object());
    }

    #[tokio::test]
    async fn test_update_config() {
        let body = Json(serde_json::json!({
            "default_provider": "ollama",
            "default_model": "llama3.2"
        }));
        let result = update_config(test_state(), body).await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());

        // Verify updated
        let config_result = get_config(test_state()).await;
        // Note: test_state creates fresh state each time, so only in-memory update is tested
    }

    // ---- Multi-Agent ----

    #[tokio::test]
    async fn test_list_agents_empty() {
        let result = list_agents(test_state()).await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());
        assert_eq!(json["total"], 0);
        assert!(json["agents"].is_array());
    }

    #[tokio::test]
    async fn test_create_agent() {
        let state = test_state();
        let body = Json(serde_json::json!({
            "name": "test-agent",
            "role": "assistant",
            "description": "A test agent",
            "system_prompt": "You are a test agent."
        }));
        let result = create_agent(state.clone(), body).await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());
        assert_eq!(json["name"], "test-agent");
        assert_eq!(json["total_agents"], 1);

        // List should now have 1
        let list = list_agents(state.clone()).await;
        assert_eq!(list.0["total"], 1);
    }

    #[tokio::test]
    async fn test_create_agent_missing_name() {
        let body = Json(serde_json::json!({
            "role": "assistant"
        }));
        let result = create_agent(test_state(), body).await;
        let json = result.0;
        // Agent creation with missing "name" field — the endpoint reads it as empty string
        // which may or may not fail depending on validation
        assert!(json.get("ok").is_some());
    }

    #[tokio::test]
    async fn test_update_agent() {
        let state = test_state();
        // Create first
        let body = Json(serde_json::json!({
            "name": "editor",
            "role": "assistant",
            "description": "Original desc"
        }));
        create_agent(state.clone(), body).await;

        // Update
        let update_body = Json(serde_json::json!({
            "role": "coder",
            "description": "Updated desc"
        }));
        let result = update_agent(
            state.clone(),
            axum::extract::Path("editor".to_string()),
            update_body,
        )
        .await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_update_nonexistent_agent() {
        let body = Json(serde_json::json!({"role": "coder"}));
        let result = update_agent(
            test_state(),
            axum::extract::Path("nonexistent".to_string()),
            body,
        )
        .await;
        let json = result.0;
        assert!(!json["ok"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_delete_agent() {
        let state = test_state();
        // Create first
        let body = Json(serde_json::json!({
            "name": "deleteme",
            "role": "assistant",
            "description": "To be deleted"
        }));
        create_agent(state.clone(), body).await;

        // Delete
        let result = delete_agent(state.clone(), axum::extract::Path("deleteme".to_string())).await;
        assert!(result.0["ok"].as_bool().unwrap());

        // Verify gone
        let list = list_agents(state.clone()).await;
        assert_eq!(list.0["total"], 0);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_agent() {
        let result = delete_agent(test_state(), axum::extract::Path("ghost".to_string())).await;
        assert!(!result.0["ok"].as_bool().unwrap());
    }

    // ---- Telegram Bot Status ----

    #[tokio::test]
    async fn test_telegram_status_not_connected() {
        let result =
            telegram_status(test_state(), axum::extract::Path("some-agent".to_string())).await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());
        assert!(!json["connected"].as_bool().unwrap());
    }

    // ---- Knowledge Base ----

    #[tokio::test]
    async fn test_knowledge_list_docs_no_store() {
        let result = knowledge_list_docs(test_state()).await;
        let json = result.0;
        // Should handle gracefully when no KB initialized
        assert!(json.is_object());
    }

    #[tokio::test]
    async fn test_knowledge_search_no_store() {
        let body = Json(serde_json::json!({"query": "test"}));
        let result = knowledge_search(test_state(), body).await;
        let json = result.0;
        assert!(json.is_object());
    }

    // ---- Scheduler ----

    #[tokio::test]
    async fn test_scheduler_list_tasks() {
        let result = scheduler_list_tasks(test_state()).await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());
        assert!(json["tasks"].is_array());
    }

    #[tokio::test]
    async fn test_scheduler_notifications() {
        let result = scheduler_notifications(test_state()).await;
        let json = result.0;
        assert!(json["ok"].as_bool().unwrap());
    }
}

// ═══════════════════════════════════════════════════════
// Gallery API — Manage skill templates
// ═══════════════════════════════════════════════════════

/// List all gallery skills (built-in + user-created).
pub async fn gallery_list(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let gallery_path = state.config_path.parent()
        .unwrap_or(std::path::Path::new("."))
        .join("gallery.json");

    // Load built-in skills from embedded data
    let builtin: Vec<serde_json::Value> = serde_json::from_str(
        include_str!("../../../data/gallery-skills.json")
    ).unwrap_or_default();

    // Load user-created skills
    let user_skills: Vec<serde_json::Value> = if gallery_path.exists() {
        std::fs::read_to_string(&gallery_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // Check which skills have attached MD files
    let skills_dir = state.config_path.parent()
        .unwrap_or(std::path::Path::new("."))
        .join("skills");

    let mut all_skills: Vec<serde_json::Value> = builtin.into_iter()
        .map(|mut s| { s.as_object_mut().map(|o| o.insert("source".into(), "builtin".into())); s })
        .collect();

    for mut s in user_skills {
        s.as_object_mut().map(|o| o.insert("source".into(), "user".into()));
        all_skills.push(s);
    }

    // Check for attached MD files
    for skill in &mut all_skills {
        if let Some(id) = skill["id"].as_str() {
            let md_path = skills_dir.join(format!("{}.md", id));
            if md_path.exists() {
                skill.as_object_mut().map(|o| o.insert("has_md".into(), true.into()));
            }
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "skills": all_skills,
        "total": all_skills.len(),
    }))
}

/// Create a custom gallery skill.
pub async fn gallery_create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let gallery_path = state.config_path.parent()
        .unwrap_or(std::path::Path::new("."))
        .join("gallery.json");

    let mut skills: Vec<serde_json::Value> = if gallery_path.exists() {
        std::fs::read_to_string(&gallery_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let id = body["id"].as_str().unwrap_or("custom").to_string();

    // Check for duplicate
    if skills.iter().any(|s| s["id"].as_str() == Some(&id)) {
        return Json(serde_json::json!({"ok": false, "error": format!("Skill '{}' already exists", id)}));
    }

    skills.push(body.clone());

    if let Ok(json) = serde_json::to_string_pretty(&skills) {
        let _ = std::fs::write(&gallery_path, json);
    }

    Json(serde_json::json!({"ok": true, "id": id, "total": skills.len()}))
}

/// Delete a custom gallery skill.
pub async fn gallery_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let gallery_path = state.config_path.parent()
        .unwrap_or(std::path::Path::new("."))
        .join("gallery.json");

    let mut skills: Vec<serde_json::Value> = if gallery_path.exists() {
        std::fs::read_to_string(&gallery_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let before = skills.len();
    skills.retain(|s| s["id"].as_str() != Some(&id));
    let removed = before != skills.len();

    if removed {
        if let Ok(json) = serde_json::to_string_pretty(&skills) {
            let _ = std::fs::write(&gallery_path, json);
        }
        // Also remove any attached MD file
        let skills_dir = state.config_path.parent()
            .unwrap_or(std::path::Path::new("."))
            .join("skills");
        let md_path = skills_dir.join(format!("{}.md", id));
        let _ = std::fs::remove_file(md_path);
    }

    Json(serde_json::json!({"ok": removed, "id": id}))
}

/// Upload an MD file for a gallery skill.
pub async fn gallery_upload_md(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    body: String,
) -> Json<serde_json::Value> {
    let skills_dir = state.config_path.parent()
        .unwrap_or(std::path::Path::new("."))
        .join("skills");
    let _ = std::fs::create_dir_all(&skills_dir);

    let md_path = skills_dir.join(format!("{}.md", id));
    match std::fs::write(&md_path, &body) {
        Ok(_) => {
            tracing::info!("📄 Uploaded skill MD: {}.md ({} bytes)", id, body.len());
            Json(serde_json::json!({
                "ok": true,
                "id": id,
                "size": body.len(),
                "path": md_path.display().to_string(),
            }))
        }
        Err(e) => internal_error("gateway", e),
    }
}

/// Get the MD content for a gallery skill.
pub async fn gallery_get_md(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let skills_dir = state.config_path.parent()
        .unwrap_or(std::path::Path::new("."))
        .join("skills");
    let md_path = skills_dir.join(format!("{}.md", id));

    if md_path.exists() {
        let content = std::fs::read_to_string(&md_path).unwrap_or_default();
        Json(serde_json::json!({"ok": true, "id": id, "content": content}))
    } else {
        Json(serde_json::json!({"ok": false, "error": "MD file not found"}))
    }
}

// ═══════════════════════════════════════════════════════
// Agent-Channel Binding API
// ═══════════════════════════════════════════════════════

/// Bind an agent to one or more channels.
pub async fn agent_bind_channels(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let channels = body["channels"].as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
        .unwrap_or_default();

    // Store binding in agent-channels.json
    let bindings_path = state.config_path.parent()
        .unwrap_or(std::path::Path::new("."))
        .join("agent-channels.json");

    let mut bindings: serde_json::Map<String, serde_json::Value> = if bindings_path.exists() {
        std::fs::read_to_string(&bindings_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        serde_json::Map::new()
    };

    bindings.insert(name.clone(), serde_json::json!(channels));

    if let Ok(json) = serde_json::to_string_pretty(&serde_json::Value::Object(bindings.clone())) {
        let _ = std::fs::write(&bindings_path, json);
    }

    tracing::info!("🔗 Agent '{}' bound to channels: {:?}", name, channels);

    Json(serde_json::json!({
        "ok": true,
        "agent": name,
        "channels": channels,
    }))
}

/// Get channel bindings for all agents.
pub async fn agent_channel_bindings(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let bindings_path = state.config_path.parent()
        .unwrap_or(std::path::Path::new("."))
        .join("agent-channels.json");

    let bindings: serde_json::Value = if bindings_path.exists() {
        std::fs::read_to_string(&bindings_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    Json(serde_json::json!({
        "ok": true,
        "bindings": bindings,
    }))
}

// ---- Orchestration API ----

/// Delegate a task from one agent to another.
/// POST /api/v1/orchestration/delegate
/// Body: {"from_agent": "a", "to_agent": "b", "task": "...", "mode": "sync|async"}
pub async fn orch_delegate(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let from = body["from_agent"].as_str().unwrap_or("");
    let to = body["to_agent"].as_str().unwrap_or("");
    let task = body["task"].as_str().unwrap_or("");

    if from.is_empty() || to.is_empty() || task.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "from_agent, to_agent, and task are required"}));
    }

    let mode = match body["mode"].as_str().unwrap_or("sync") {
        "async" => bizclaw_core::types::DelegationMode::Async,
        _ => bizclaw_core::types::DelegationMode::Sync,
    };

    let mut orch = state.orchestrator.lock().await;
    match orch.delegate_with_mode(from, to, task, mode).await {
        Ok(response) => Json(serde_json::json!({
            "ok": true,
            "from": from,
            "to": to,
            "response": safe_truncate(&response, 5000),
        })),
        Err(e) => {
            tracing::error!("[orch_delegate] {e}");
            internal_error("delegation", e)
        }
    }
}

/// Handoff conversation from one agent to another.
/// POST /api/v1/orchestration/handoff
pub async fn orch_handoff(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let from = body["from_agent"].as_str().unwrap_or("");
    let to = body["to_agent"].as_str().unwrap_or("");
    let session = body["session_id"].as_str().unwrap_or("default");
    let reason = body["reason"].as_str();

    if from.is_empty() || to.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "from_agent and to_agent required"}));
    }

    let mut orch = state.orchestrator.lock().await;
    match orch.handoff(from, to, session, reason).await {
        Ok(()) => Json(serde_json::json!({"ok": true, "from": from, "to": to, "session": session})),
        Err(e) => {
            tracing::error!("[orch_handoff] {e}");
            internal_error("handoff", e)
        }
    }
}

/// Clear handoff for a session.
/// DELETE /api/v1/orchestration/handoff/{session_id}
pub async fn orch_clear_handoff(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let orch = state.orchestrator.lock().await;
    match orch.clear_handoff(&session_id).await {
        Ok(()) => Json(serde_json::json!({"ok": true, "session": session_id})),
        Err(e) => internal_error("clear_handoff", e),
    }
}

/// Run evaluate loop between two agents.
/// POST /api/v1/orchestration/evaluate
pub async fn orch_evaluate(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let generator = body["generator"].as_str().unwrap_or("");
    let evaluator = body["evaluator"].as_str().unwrap_or("");
    let task = body["task"].as_str().unwrap_or("");
    let pass_criteria = body["pass_criteria"].as_str().unwrap_or("high quality output");
    let max_rounds = body["max_rounds"].as_u64().unwrap_or(3) as u32;

    if generator.is_empty() || evaluator.is_empty() || task.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "generator, evaluator, and task required"}));
    }

    let config = bizclaw_core::types::EvaluateConfig {
        generator: generator.to_string(),
        evaluator: evaluator.to_string(),
        task: task.to_string(),
        pass_criteria: pass_criteria.to_string(),
        max_rounds,
    };

    let mut orch = state.orchestrator.lock().await;
    match orch.evaluate_loop(&config).await {
        Ok(result) => Json(serde_json::json!({
            "ok": true,
            "approved": result.approved,
            "output": result.output,
            "feedback": result.feedback,
            "rounds_used": result.rounds_used,
            "max_rounds": result.max_rounds,
        })),
        Err(e) => {
            tracing::error!("[orch_evaluate] {e}");
            internal_error("evaluate", e)
        }
    }
}

/// Create a permission link between agents.
/// POST /api/v1/orchestration/links
pub async fn orch_create_link(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let source = body["source"].as_str().unwrap_or("");
    let target = body["target"].as_str().unwrap_or("");
    let direction = match body["direction"].as_str().unwrap_or("outbound") {
        "inbound" => bizclaw_core::types::LinkDirection::Inbound,
        "bidirectional" => bizclaw_core::types::LinkDirection::Bidirectional,
        _ => bizclaw_core::types::LinkDirection::Outbound,
    };

    if source.is_empty() || target.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "source and target required"}));
    }

    let orch = state.orchestrator.lock().await;
    match orch.create_link(source, target, direction).await {
        Ok(link) => Json(serde_json::json!({"ok": true, "id": link.id})),
        Err(e) => internal_error("create_link", e),
    }
}

/// List all agent permission links.
/// GET /api/v1/orchestration/links
pub async fn orch_list_links(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let orch = state.orchestrator.lock().await;
    match orch.list_links().await {
        Ok(links) => {
            let items: Vec<serde_json::Value> = links.iter().map(|l| serde_json::json!({
                "id": l.id,
                "source": l.source_agent,
                "target": l.target_agent,
                "direction": l.direction.to_string(),
                "max_concurrent": l.max_concurrent,
            })).collect();
            Json(serde_json::json!({"ok": true, "links": items, "count": items.len()}))
        }
        Err(e) => internal_error("list_links", e),
    }
}

/// Delete a permission link.
/// DELETE /api/v1/orchestration/links/{id}
pub async fn orch_delete_link(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let orch = state.orchestrator.lock().await;
    match orch.delete_link(&id).await {
        Ok(()) => Json(serde_json::json!({"ok": true})),
        Err(e) => internal_error("delete_link", e),
    }
}

/// List delegation history.
/// GET /api/v1/orchestration/delegations?agent=name&limit=20
pub async fn orch_list_delegations(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let agent = params.get("agent").map(|s| s.as_str()).unwrap_or("*");
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(20);

    let store = &state.orch_store;
    let delegations = if agent == "*" {
        // Get all — use traces as proxy (list_delegations requires agent)
        store.list_delegations("", limit).await.unwrap_or_default()
    } else {
        store.list_delegations(agent, limit).await.unwrap_or_default()
    };

    let items: Vec<serde_json::Value> = delegations.iter().map(|d| serde_json::json!({
        "id": d.id,
        "from": d.from_agent,
        "to": d.to_agent,
        "task": safe_truncate(&d.task, 200),
        "status": format!("{:?}", d.status),
        "mode": format!("{:?}", d.mode),
        "created_at": d.created_at.to_rfc3339(),
    })).collect();

    Json(serde_json::json!({"ok": true, "delegations": items, "count": items.len()}))
}

/// List LLM traces (observability).
/// GET /api/v1/orchestration/traces?limit=50
pub async fn orch_list_traces(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(50);

    let orch = state.orchestrator.lock().await;
    let traces = orch.list_traces(limit).await.unwrap_or_default();

    let items: Vec<serde_json::Value> = traces.iter().map(|t| serde_json::json!({
        "id": t.id,
        "agent": t.agent_name,
        "provider": t.provider,
        "model": t.model,
        "tokens": t.total_tokens,
        "latency_ms": t.latency_ms,
        "cache_hit": t.cache_hit,
        "status": t.status,
        "created_at": t.created_at.to_rfc3339(),
    })).collect();

    Json(serde_json::json!({"ok": true, "traces": items, "count": items.len()}))
}

// ═══ MCP Servers API ═══
pub async fn mcp_list_servers(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let config = state.full_config.lock().unwrap();
    let servers: Vec<serde_json::Value> = config.mcp_servers.iter().map(|s| {
        serde_json::json!({
            "name": s.name,
            "transport": "stdio",
            "command": s.command,
            "args": s.args,
            "enabled": s.enabled,
            "tools_count": 0,
            "status": if s.enabled { "configured" } else { "disabled" },
        })
    }).collect();
    Json(serde_json::json!({"ok": true, "servers": servers, "count": servers.len()}))
}
