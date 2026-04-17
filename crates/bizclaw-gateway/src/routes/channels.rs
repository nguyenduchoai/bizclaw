//! Channel management API route handlers.
//!
//! Extracted from mod.rs to reduce God File size.
//! Handles: update_channel, channel instances CRUD, agent-channel bindings.

use axum::{Json, extract::State};
use std::sync::Arc;

use super::api_webhooks::spawn_telegram_polling;
use super::internal_error;
use super::mask_secret;
use crate::server::AppState;

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
    let mut cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());

    match channel_type {
        "telegram" => {
            let token_val = req.get("bot_token").and_then(|v| v.as_str()).unwrap_or("");
            let instance_name = req
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Telegram")
                .to_string();
            let token = if token_val.contains('•') {
                cfg.channel
                    .telegram
                    .first()
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
            let new_cfg = bizclaw_core::config::TelegramChannelConfig {
                name: instance_name.clone(),
                enabled,
                bot_token: token,
                allowed_chat_ids: chat_ids,
            };
            // Upsert by name
            if let Some(pos) = cfg
                .channel
                .telegram
                .iter()
                .position(|t| t.name == instance_name)
            {
                cfg.channel.telegram[pos] = new_cfg;
            } else {
                cfg.channel.telegram.push(new_cfg);
            }
        }
        "zalo" => {
            let mut zalo_cfg = cfg.channel.zalo.first().cloned().unwrap_or_default();
            let instance_name = req
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&zalo_cfg.name)
                .to_string();
            zalo_cfg.name = instance_name.clone();
            zalo_cfg.enabled = enabled;
            if let Some(v) = req.get("cookie").and_then(|v| v.as_str()) {
                let cookie_dir = state
                    .config_path
                    .parent()
                    .unwrap_or(std::path::Path::new("."));
                let cookie_path = cookie_dir.join("zalo_cookie.txt");
                if let Err(e) = std::fs::write(&cookie_path, v) {
                    tracing::warn!(
                        "Failed to save Zalo cookie to {}: {e}",
                        cookie_path.display()
                    );
                }
                zalo_cfg.personal.cookie_path = cookie_path.display().to_string();
            }
            if let Some(v) = req.get("imei").and_then(|v| v.as_str()) {
                zalo_cfg.personal.imei = v.to_string();
            }
            if let Some(pos) = cfg
                .channel
                .zalo
                .iter()
                .position(|z| z.name == instance_name)
            {
                cfg.channel.zalo[pos] = zalo_cfg;
            } else {
                cfg.channel.zalo.push(zalo_cfg);
            }
        }
        "discord" => {
            let instance_name = req
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Discord")
                .to_string();
            let token_val = req.get("bot_token").and_then(|v| v.as_str()).unwrap_or("");
            let token = if token_val.contains('•') {
                cfg.channel
                    .discord
                    .first()
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
            let new_cfg = bizclaw_core::config::DiscordChannelConfig {
                name: instance_name.clone(),
                enabled,
                bot_token: token,
                allowed_channel_ids: ids,
            };
            if let Some(pos) = cfg
                .channel
                .discord
                .iter()
                .position(|d| d.name == instance_name)
            {
                cfg.channel.discord[pos] = new_cfg;
            } else {
                cfg.channel.discord.push(new_cfg);
            }
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
                    .first()
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
            let new_cfg = bizclaw_core::config::EmailChannelConfig {
                enabled,
                smtp_host,
                smtp_port,
                email: email_addr,
                password,
                imap_host,
                imap_port: 993,
            };
            if cfg.channel.email.is_empty() {
                cfg.channel.email.push(new_cfg);
            } else {
                cfg.channel.email[0] = new_cfg;
            }
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
                    .first()
                    .map(|w| w.access_token.clone())
                    .unwrap_or_default()
            } else {
                token_val.to_string()
            };
            let new_cfg = bizclaw_core::config::WhatsAppChannelConfig {
                enabled,
                phone_number_id: phone_val,
                access_token: token,
                webhook_verify_token: req
                    .get("webhook_verify_token")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                business_id: req
                    .get("business_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            };
            if cfg.channel.whatsapp.is_empty() {
                cfg.channel.whatsapp.push(new_cfg);
            } else {
                cfg.channel.whatsapp[0] = new_cfg;
            }
        }
        "webhook" => {
            let secret_val = req
                .get("webhook_secret")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let secret = if secret_val.contains('•') {
                cfg.channel
                    .webhook
                    .first()
                    .map(|wh| wh.secret.clone())
                    .unwrap_or_default()
            } else {
                secret_val.to_string()
            };
            let outbound_url = req
                .get("webhook_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let new_cfg = bizclaw_core::config::WebhookChannelConfig {
                enabled,
                secret,
                outbound_url,
            };
            if cfg.channel.webhook.is_empty() {
                cfg.channel.webhook.push(new_cfg);
            } else {
                cfg.channel.webhook[0] = new_cfg;
            }
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
                    "telegram": cfg.channel.telegram.iter().map(|t| serde_json::json!({
                        "name": t.name,
                        "enabled": t.enabled,
                        "bot_token_set": !t.bot_token.is_empty(),
                        "allowed_chat_ids": t.allowed_chat_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", "),
                    })).collect::<Vec<_>>(),
                    "zalo": cfg.channel.zalo.iter().map(|z| serde_json::json!({
                        "name": z.name,
                        "enabled": z.enabled,
                        "mode": z.mode,
                    })).collect::<Vec<_>>(),
                    "discord": cfg.channel.discord.iter().map(|d| serde_json::json!({
                        "name": d.name,
                        "enabled": d.enabled,
                        "bot_token_set": !d.bot_token.is_empty(),
                    })).collect::<Vec<_>>(),
                    "email": cfg.channel.email.iter().map(|e| serde_json::json!({
                        "enabled": e.enabled,
                        "smtp_host": e.smtp_host,
                        "smtp_port": e.smtp_port,
                        "email": e.email,
                    })).collect::<Vec<_>>(),
                    "whatsapp": cfg.channel.whatsapp.iter().map(|w| serde_json::json!({
                        "enabled": w.enabled,
                        "phone_number_id": w.phone_number_id,
                    })).collect::<Vec<_>>(),
                    "webhook": cfg.channel.webhook.iter().map(|wh| serde_json::json!({
                        "enabled": wh.enabled,
                        "secret_set": !wh.secret.is_empty(),
                        "outbound_url": wh.outbound_url,
                    })).collect::<Vec<_>>(),
                });
                let sync_path = parent.join("channels_sync.json");
                if let Err(e) = std::fs::write(
                    &sync_path,
                    serde_json::to_string_pretty(&channels_json).unwrap_or_default(),
                ) {
                    tracing::warn!(
                        "Failed to write channels sync to {}: {e}",
                        sync_path.display()
                    );
                }
            }
            Json(serde_json::json!({"ok": true, "message": format!("{channel_type} config saved")}))
        }
        Err(e) => internal_error("gateway", e),
    }
}

/// Channel instances file path helper.
fn channel_instances_path(state: &AppState) -> std::path::PathBuf {
    state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("channel_instances.json")
}

/// Load channel instances from JSON file.
pub fn load_channel_instances(state: &AppState) -> Vec<serde_json::Value> {
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
pub fn save_channel_instances(state: &AppState, instances: &[serde_json::Value]) {
    let path = channel_instances_path(state);
    let json = serde_json::to_string_pretty(instances).unwrap_or_default();
    if let Err(e) = std::fs::write(&path, json) {
        tracing::warn!(
            "Failed to save channel instances to {}: {e}",
            path.display()
        );
    }
    // SECURITY: Set file permissions to 0600 (owner-only) since it contains secrets
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
}

/// List all channel instances (secrets masked for frontend display).
pub async fn list_channel_instances(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let instances = load_channel_instances(&state);
    // Mask sensitive fields before sending to frontend
    let masked: Vec<serde_json::Value> = instances
        .iter()
        .map(|inst| {
            let mut masked_inst = inst.clone();
            if let Some(cfg) = masked_inst
                .get_mut("config")
                .and_then(|c| c.as_object_mut())
            {
                let sensitive_keys = [
                    "bot_token",
                    "access_token",
                    "webhook_secret",
                    "smtp_pass",
                    "app_token",
                    "app_secret",
                    "page_access_token",
                    "refresh_token",
                ];
                for key in &sensitive_keys {
                    if let Some(val) = cfg.get(*key).and_then(|v| v.as_str())
                        && !val.is_empty()
                    {
                        cfg.insert(key.to_string(), serde_json::json!(mask_secret(val)));
                    }
                }
            }
            masked_inst
        })
        .collect();
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
    let id = req
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let name = req
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let channel_type = req
        .get("channel_type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let enabled = req
        .get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let agent_name = req
        .get("agent_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
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
    if let Some(pos) = instances
        .iter()
        .position(|i| i["id"].as_str() == Some(&instance_id))
    {
        instances[pos] = instance.clone();
    } else {
        instances.push(instance.clone());
    }

    save_channel_instances(&state, &instances);

    // Also sync primary (first enabled) of this type to config.toml
    // This makes the first enabled instance of each type "active"
    let first_enabled = instances.iter().find(|i| {
        i["channel_type"].as_str() == Some(&channel_type) && i["enabled"].as_bool() == Some(true)
    });
    if let Some(primary) = first_enabled {
        let cfg = primary["config"].clone();
        let mut sync_body = cfg.as_object().cloned().unwrap_or_default();
        sync_body.insert("channel_type".into(), serde_json::json!(channel_type));
        sync_body.insert("enabled".into(), serde_json::json!(true));
        // Trigger update_channel internally via direct config write
        let mut full_cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
        match channel_type.as_str() {
            "telegram" => {
                let token = sync_body
                    .get("bot_token")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let chat_ids: Vec<i64> = sync_body
                    .get("allowed_chat_ids")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                let new_tg = bizclaw_core::config::TelegramChannelConfig {
                    name: "Telegram".into(),
                    enabled: true,
                    bot_token: token,
                    allowed_chat_ids: chat_ids,
                };
                if full_cfg.channel.telegram.is_empty() {
                    full_cfg.channel.telegram.push(new_tg);
                } else {
                    full_cfg.channel.telegram[0] = new_tg;
                }
            }
            "webhook" => {
                let outbound = sync_body
                    .get("webhook_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let secret = sync_body
                    .get("webhook_secret")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let new_wh = bizclaw_core::config::WebhookChannelConfig {
                    enabled: true,
                    secret,
                    outbound_url: outbound,
                };
                if full_cfg.channel.webhook.is_empty() {
                    full_cfg.channel.webhook.push(new_wh);
                } else {
                    full_cfg.channel.webhook[0] = new_wh;
                }
            }
            _ => {} // Other types handled as-is
        }
        let content = toml::to_string_pretty(&*full_cfg).unwrap_or_default();
        if let Err(e) = std::fs::write(&state.config_path, &content) {
            tracing::warn!("Failed to persist config during channel instance save: {e}");
        }
        drop(full_cfg);
    }

    // Also write channels_sync.json for platform restart persistence
    let cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(parent) = state.config_path.parent() {
        let channels_json = serde_json::json!({
            "telegram": cfg.channel.telegram.iter().map(|t| serde_json::json!({"name": t.name, "enabled": t.enabled, "bot_token": t.bot_token, "allowed_chat_ids": t.allowed_chat_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")})).collect::<Vec<_>>(),
            "webhook": cfg.channel.webhook.iter().map(|wh| serde_json::json!({"enabled": wh.enabled, "secret": wh.secret, "outbound_url": wh.outbound_url})).collect::<Vec<_>>(),
        });
        if let Err(e) = std::fs::write(
            parent.join("channels_sync.json"),
            serde_json::to_string_pretty(&channels_json).unwrap_or_default(),
        ) {
            tracing::warn!("Failed to write channels_sync.json for instance: {e}");
        }
    }
    drop(cfg);

    // Auto-connect Telegram if agent_name + bot_token provided
    if enabled && channel_type == "telegram" && !agent_name.is_empty() {
        let bot_token = config
            .get("bot_token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
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

// ═══════════════════════════════════════════════════════
// Agent-Channel Binding API
// ═══════════════════════════════════════════════════════

/// Bind an agent to one or more channels.
pub async fn agent_bind_channels(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let channels = body["channels"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Store binding in agent-channels.json
    let bindings_path = state
        .config_path
        .parent()
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

    let mut obj = serde_json::Map::new();
    obj.insert("channels".into(), body["channels"].clone());
    if let Some(rag) = body.get("rag_collection") {
        obj.insert("rag_collection".into(), rag.clone());
    }
    if let Some(sql) = body.get("sql_connection") {
        obj.insert("sql_connection".into(), sql.clone());
    }

    bindings.insert(name.clone(), serde_json::Value::Object(obj));

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
pub async fn agent_channel_bindings(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let bindings_path = state
        .config_path
        .parent()
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

// ═══════════════════════════════════════════════════════
// Zalo Session Status API — used by SME Dashboard
// ═══════════════════════════════════════════════════════

/// Get Zalo native session status — reads the persisted session JSON.
/// Used by /channels and /companion dashboard pages.
pub async fn zalo_session_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
    let zalo_cfg = cfg.channel.zalo.first().cloned();
    drop(cfg);

    let Some(zalo) = zalo_cfg else {
        return Json(serde_json::json!({
            "ok": true,
            "connected": false,
            "reason": "no_zalo_config",
            "message": "Zalo chưa được cấu hình trong config.toml",
        }));
    };

    // Find session file next to cookie_path
    let session_path = if !zalo.personal.cookie_path.is_empty() {
        let p = std::path::Path::new(&zalo.personal.cookie_path);
        let parent = p.parent().unwrap_or(std::path::Path::new("."));
        parent.join("zalo_session.json")
    } else {
        state
            .config_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("zalo_session.json")
    };

    if !session_path.exists() {
        return Json(serde_json::json!({
            "ok": true,
            "connected": false,
            "reason": "no_session_file",
            "message": "Chưa có phiên đăng nhập. Cần đăng nhập Zalo bằng cookie.",
            "config": {
                "mode": zalo.mode,
                "enabled": zalo.enabled,
                "has_cookie_path": !zalo.personal.cookie_path.is_empty(),
                "has_imei": !zalo.personal.imei.is_empty(),
            }
        }));
    };

    // Read and parse session
    match std::fs::read_to_string(&session_path) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(session) => {
                let uid = session["uid"].as_str().unwrap_or("");
                let active = session["active"].as_bool().unwrap_or(false);
                let last_heartbeat = session["last_heartbeat"].as_u64().unwrap_or(0);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let age_secs = now.saturating_sub(last_heartbeat);
                let stale = age_secs > 300; // 5 min threshold

                Json(serde_json::json!({
                    "ok": true,
                    "connected": active && !stale,
                    "uid": uid,
                    "active": active,
                    "stale": stale,
                    "last_heartbeat": last_heartbeat,
                    "heartbeat_age_secs": age_secs,
                    "has_cookie": session["cookie"].is_string(),
                    "has_secret_key": session["secret_key"].is_string(),
                    "has_cipher_key": session["zpw_enk"].is_string(),
                    "config": {
                        "mode": zalo.mode,
                        "enabled": zalo.enabled,
                        "auto_reconnect": zalo.personal.auto_reconnect,
                    }
                }))
            }
            Err(_) => Json(serde_json::json!({
                "ok": true,
                "connected": false,
                "reason": "corrupt_session",
                "message": "File phiên bị lỗi. Cần đăng nhập lại.",
            })),
        },
        Err(_) => Json(serde_json::json!({
            "ok": true,
            "connected": false,
            "reason": "unreadable_session",
            "message": "Không đọc được file phiên.",
        })),
    }
}
