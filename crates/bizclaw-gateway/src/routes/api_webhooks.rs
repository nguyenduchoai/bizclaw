//! Channel webhooks and inbound message handlers.
//! Handles generic webhooks, Discord, Zalo OA, WhatsApp, Messenger, Xiaozhi.
//! Provider management extracted to routes/providers.rs.

use super::{load_channel_instances, safe_truncate, save_channel_instances};
use crate::server::AppState;
use axum::{Json, extract::State};
use std::sync::Arc;

// Re-export provider functions for backward compat
pub use super::providers::{
    brain_delete_model, brain_download_model, brain_download_status, brain_scan_models,
    create_provider, delete_provider, fetch_provider_models, list_channels, list_providers,
    ollama_models, update_provider,
};

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
            let outbound = inst["config"]["webhook_url"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let sec = inst["config"]["webhook_secret"]
                .as_str()
                .unwrap_or("")
                .to_string();
            (agent, outbound, sec)
        }
        None => {
            return Json(serde_json::json!({
                "ok": false,
                "error": "No webhook channel bound to an agent. Create one in Dashboard → Channels."
            }));
        }
    };

    // Verify signature if secret configured (HMAC-SHA256)
    if !secret.is_empty() {
        let sig = headers
            .get("x-webhook-signature")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(body.as_bytes());
        let expected = format!("{:x}", mac.finalize().into_bytes());
        if expected != sig {
            return Json(serde_json::json!({"ok": false, "error": "Invalid webhook signature"}));
        }
    }

    // Parse message
    let json: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("[security] Invalid JSON from client: {e}");
            return Json(serde_json::json!({"ok": false, "error": "Invalid JSON format"}));
        }
    };
    let content = json["content"].as_str().unwrap_or("").to_string();
    let sender = json["sender_id"]
        .as_str()
        .unwrap_or("webhook-user")
        .to_string();
    if content.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "'content' field required"}));
    }

    tracing::info!(
        "[webhook] {} → agent '{}': {}",
        sender,
        agent_name,
        safe_truncate(&content, 100)
    );

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
            tracing::info!(
                "[telegram] Disconnecting existing bot for agent '{}'",
                agent_name
            );
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
            tracing::error!(
                "[telegram] Bot token invalid for instance '{}': {}",
                instance_id,
                e
            );
            return;
        }
    };
    tracing::info!(
        "[telegram] @{} connected → agent '{}' (instance: {})",
        bot_username,
        agent_name,
        instance_id
    );

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
                                    let parts: Vec<&str> = msg.thread_id.split(':').collect();
                                    let chat_id: i64 = parts[0].parse().unwrap_or(0);
                                    let message_thread_id: Option<i64> = parts.get(1).and_then(|id| id.parse().ok());

                                    let sender = msg.sender_name.clone().unwrap_or_default();
                                    let text = msg.content.clone();

                                    // ── Handoff Auto-Routing (Intercept) ──
                                    let lower = text.to_lowercase();
                                    if lower.contains("gặp nhân viên") || lower.contains("gap nhan vien") || lower.contains("chuyển khách") {
                                        tracing::info!("📞 Handoff hotword detected in Telegram: '{}'", text);
                                        let req = crate::routes::api_handoff::HandoffRequestPayload {
                                            customer: sender.clone(),
                                            channel: Some("Telegram".to_string()),
                                            reason: Some("Khách hàng yêu cầu hỗ trợ từ nhân viên qua Telegram".to_string()),
                                            message: Some(text.clone()),
                                        };
                                        let _ = crate::routes::api_handoff::execute_handoff(state_clone.clone(), req).await;

                                        let reply = crate::routes::api_handoff::load_handoff_settings(&state_clone).greeting;
                                        let _ = channel.send_message(chat_id, message_thread_id, &reply).await;
                                        continue;
                                    }

                                    tracing::info!("[telegram] {} → agent '{}': {}", sender, agent_name_clone, safe_truncate(&text, 100));
                                    let _ = channel.send_typing(chat_id, message_thread_id).await;

                                    // Route to agent
                                    let response = {
                                        let mut orch = state_clone.orchestrator.lock().await;
                                        match orch.send_to(&agent_name_clone, &text).await {
                                            Ok(r) => r,
                                            Err(e) => format!("⚠️ Agent error: {e}"),
                                        }
                                    };

                                    if let Err(e) = channel.send_message(chat_id, message_thread_id, &response).await {
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
            crate::server::TelegramBotState {
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

    let discord =
        bizclaw_channels::discord::DiscordChannel::new(bizclaw_channels::discord::DiscordConfig {
            bot_token: bot_token.clone(),
            enabled: true,
            intents: 33281, // GUILDS | GUILD_MESSAGES | MESSAGE_CONTENT
        });

    // Verify bot token
    match discord.get_me().await {
        Ok(me) => {
            tracing::info!(
                "[discord] Bot {} connected → agent '{}' (instance: {})",
                me.username,
                agent_name,
                instance_id
            );
        }
        Err(e) => {
            tracing::error!(
                "[discord] Bot token invalid for instance '{}': {}",
                instance_id,
                e
            );
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

            // ── Handoff Auto-Routing (Intercept) ──
            let lower = text.to_lowercase();
            if lower.contains("gặp nhân viên")
                || lower.contains("gap nhan vien")
                || lower.contains("chuyển khách")
            {
                tracing::info!("📞 Handoff hotword detected in Discord: '{}'", text);
                let req = crate::routes::api_handoff::HandoffRequestPayload {
                    customer: sender.clone(),
                    channel: Some("Discord".to_string()),
                    reason: Some("Khách hàng yêu cầu hỗ trợ từ nhân viên qua Discord".to_string()),
                    message: Some(text.clone()),
                };
                let _ = crate::routes::api_handoff::execute_handoff(state_clone.clone(), req).await;

                let reply =
                    crate::routes::api_handoff::load_handoff_settings(&state_clone).greeting;
                let _ = reply_client.send_message(&channel_id, &reply).await;
                continue;
            }

            tracing::info!(
                "[discord] {} → agent '{}': {}",
                sender,
                agent_name_clone,
                safe_truncate(&text, 100)
            );

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
        tracing::warn!(
            "[discord] Gateway stream ended for agent '{}'",
            agent_name_clone
        );
    });
}

/// Auto-connect all enabled channel instances on startup.
/// Called from server::start() after AppState is built.
pub async fn resolve_agent_for_channel(state: &AppState, channel: &str) -> Option<String> {
    let agent_channels_path = state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("agent-channels.json");
    if agent_channels_path.exists()
        && let Ok(content) = std::fs::read_to_string(&agent_channels_path)
        && let Ok(bindings) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(obj) = bindings.as_object()
    {
        for (agent, channels) in obj {
            if let Some(arr) = channels.as_array()
                && arr.iter().any(|c| c.as_str() == Some(channel))
            {
                return Some(agent.clone());
            }
        }
    }
    None
}

pub async fn dispatch_to_channel_agent(
    state: Arc<AppState>,
    channel: &str,
    thread_id: Option<&str>,
    content: &str,
) -> Option<String> {
    if let Some(t_id) = thread_id {
        if state.paused_threads.read().await.contains(t_id) {
            tracing::info!("⏸️ Handoff active: ignoring AI reply for thread {}", t_id);
            return None;
        }

        // ── Handoff Auto-Routing (Intercept) ──
        let lower = content.to_lowercase();
        if lower.contains("gặp nhân viên")
            || lower.contains("gap nhan vien")
            || lower.contains("chuyển khách")
        {
            tracing::info!("📞 Handoff hotword detected in {}: '{}'", channel, content);
            let req = crate::routes::api_handoff::HandoffRequestPayload {
                customer: t_id.to_string(),
                channel: Some(channel.to_string()),
                reason: Some(format!(
                    "Khách hàng yêu cầu hỗ trợ từ nhân viên qua {}",
                    channel
                )),
                message: Some(content.to_string()),
            };
            let _ = crate::routes::api_handoff::execute_handoff(state.clone(), req).await;
            return Some(crate::routes::api_handoff::load_handoff_settings(&state).greeting);
        }
    }
    // ── P1.1 FIX: Compute session key for per-thread conversation isolation ──
    let session_key = match thread_id {
        Some(t_id) => format!("{}:{}", channel, t_id),
        None => format!("{}:anon", channel),
    };

    // ── P1.3 FIX: Hybrid Intent Router ──
    // Step 1: Try keyword-based fast-path (0ms, handles ~80% of messages)
    let fast_routed = fast_route_by_keyword(content);

    // Step 2: Check if channel has a specific agent binding (manual override)
    let target = resolve_agent_for_channel(&state, channel).await;

    // Step 3a: If keyword router matched → skip MAMA LLM entirely (fast path)
    if let Some(routed_agent) = fast_routed {
        if target.is_none() {
            let mut orch = state.orchestrator.lock().await;
            // Set session for thread isolation
            if let Some(agent) = orch.get_agent_mut(&routed_agent) {
                agent.set_session(&session_key);
            }
            tracing::info!(
                "⚡ Fast-route '{}' → '{}' (keyword match, session={})",
                if content.len() > 50 {
                    &content[..50]
                } else {
                    content
                },
                routed_agent,
                session_key
            );
            match orch.send_to(&routed_agent, content).await {
                Ok(reply) => return Some(reply),
                Err(e) => {
                    tracing::warn!("⚡ Fast-routed agent '{}' failed: {}", routed_agent, e);
                    // Fall through to MAMA or default
                }
            }
        }
    }

    // Step 3b: Ambiguous message → try MAMA LLM routing (slow path)
    let has_mama = {
        let orch = state.orchestrator.lock().await;
        orch.list_agents()
            .iter()
            .any(|a| a["name"].as_str() == Some("mama"))
    };

    if has_mama && target.is_none() && fast_routed.is_none() {
        let mut orch = state.orchestrator.lock().await;
        match orch.send_to("mama", content).await {
            Ok(mama_response) => {
                // Parse [ROUTE:agent-name] from MAMA's response
                if let Some(start) = mama_response.find("[ROUTE:") {
                    let after = &mama_response[start + 7..];
                    if let Some(end) = after.find(']') {
                        let routed_agent = after[..end].trim().to_string();
                        tracing::info!(
                            "👑 MAMA routed → '{}' (session={})",
                            routed_agent,
                            session_key
                        );
                        // Set session for thread isolation before delegating
                        if let Some(agent) = orch.get_agent_mut(&routed_agent) {
                            agent.set_session(&session_key);
                        }
                        match orch.send_to(&routed_agent, content).await {
                            Ok(agent_reply) => return Some(agent_reply),
                            Err(e) => {
                                tracing::warn!(
                                    "👑 MAMA routed to '{}' but failed: {}. Fallback to sales-bot.",
                                    routed_agent,
                                    e
                                );
                                if let Some(agent) = orch.get_agent_mut("sales-bot") {
                                    agent.set_session(&session_key);
                                }
                                if let Ok(fb) = orch.send_to("sales-bot", content).await {
                                    return Some(fb);
                                }
                            }
                        }
                    }
                }
                // MAMA didn't return [ROUTE:...] — use its response directly
                tracing::info!("👑 MAMA responded directly (no routing tag)");
                return Some(mama_response);
            }
            Err(e) => {
                tracing::warn!("👑 MAMA error: {}. Falling back.", e);
            }
        }
    }

    // Step 4: Fallback — channel-specific agent or default agent
    let mut orch = state.orchestrator.lock().await;

    if let Some(agent_name) = target {
        if let Some(agent) = orch.get_agent_mut(&agent_name) {
            agent.set_session(&session_key);
        }
        match orch.send_to(&agent_name, content).await {
            Ok(r) => return Some(r),
            Err(e) => {
                tracing::warn!(
                    "Failed to route to mapped agent '{}': {}. Falling back to default.",
                    agent_name,
                    e
                );
            }
        }
    }

    // Last resort: default agent with session isolation
    if let Some(default_name) = orch.default_agent_name().map(|s| s.to_string()) {
        if let Some(agent) = orch.get_agent_mut(&default_name) {
            agent.set_session(&session_key);
        }
    }
    match orch.send(content).await {
        Ok(r) => Some(r),
        Err(e) => Some(format!("⚠️ Agent error: {e}")),
    }
}

/// P1.3: Keyword-based fast intent router.
/// Returns agent name if keywords match with high confidence.
/// Returns None for ambiguous messages (routed to MAMA LLM).
fn fast_route_by_keyword(content: &str) -> Option<&'static str> {
    let lower = content.to_lowercase();

    // ── Sales signals ──
    if lower.contains("giá")
        || lower.contains("bao nhiêu")
        || lower.contains("mua")
        || lower.contains("đặt hàng")
        || lower.contains("báo giá")
        || lower.contains("order")
        || lower.contains("thanh toán")
        || lower.contains("chuyển khoản")
        || lower.contains("tư vấn")
        || lower.contains("sản phẩm")
        || lower.contains("catalogue")
        || lower.contains("bảng giá")
    {
        return Some("sales-bot");
    }

    // ── Support signals ──
    if lower.contains("lỗi")
        || lower.contains("hỏng")
        || lower.contains("không được")
        || lower.contains("crash")
        || lower.contains("bug")
        || lower.contains("sửa")
        || lower.contains("hướng dẫn")
        || lower.contains("cài đặt")
        || lower.contains("trợ giúp")
        || lower.contains("help")
        || lower.contains("ticket")
        || lower.contains("khiếu nại")
    {
        return Some("support-bot");
    }

    // ── Marketing signals ──
    if lower.contains("viết bài")
        || lower.contains("content")
        || lower.contains("quảng cáo")
        || lower.contains("marketing")
        || lower.contains("facebook")
        || lower.contains("tiktok")
        || lower.contains("social")
        || lower.contains("chiến dịch")
        || lower.contains("email marketing")
        || lower.contains("seo")
    {
        return Some("marketing-bot");
    }

    // ── Analyst signals ──
    if lower.contains("báo cáo")
        || lower.contains("thống kê")
        || lower.contains("doanh thu")
        || lower.contains("kpi")
        || lower.contains("dashboard")
        || lower.contains("phân tích")
        || lower.contains("forecast")
        || lower.contains("report")
    {
        return Some("analyst-bot");
    }

    // ── Coder signals ──
    if lower.contains("code")
        || lower.contains("debug")
        || lower.contains("api")
        || lower.contains("deploy")
        || lower.contains("lập trình")
        || lower.contains("review code")
    {
        return Some("coder-bot");
    }

    // Ambiguous — let MAMA LLM decide
    None
}

// ── Fallback: if no telegram instances, check config.toml for bot_token ──
pub async fn auto_connect_channels(state: Arc<AppState>) {
    let mut instances = load_channel_instances(&state);
    let mut connected = 0;

    // ── Fallback: if no telegram instances, check config.toml for bot_token ──
    let has_telegram_instance = instances
        .iter()
        .any(|i| i["channel_type"].as_str() == Some("telegram"));
    if !has_telegram_instance {
        // Extract telegram config data (owned) to avoid holding MutexGuard across await
        let tg_data = {
            let cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
            cfg.channel.telegram.first().and_then(|tg| {
                if tg.enabled && !tg.bot_token.is_empty() {
                    Some((tg.bot_token.clone(), tg.allowed_chat_ids.clone()))
                } else {
                    None
                }
            })
        }; // MutexGuard dropped here

        if let Some((bot_token, chat_ids)) = tg_data {
            // Find which agent to bind to — check agent-channels.json first
            let mut target_agent = resolve_agent_for_channel(&state, "telegram")
                .await
                .unwrap_or_default();
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
                tracing::info!(
                    "[auto-connect] Migrated config.toml telegram → channel instance bound to '{}'",
                    target_agent
                );
            }
        }
    }

    // ── Connect all enabled instances ──
    for inst in &instances {
        let enabled = inst["enabled"].as_bool().unwrap_or(false);
        if !enabled {
            continue;
        }
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
                    )
                    .await;
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
            "zalo" if !agent_name.is_empty() => {
                // Zalo Personal
                let enabled = inst["enabled"].as_bool().unwrap_or(false);
                if enabled && inst["config"]["mode"].as_str() != Some("official") {
                    let s = state.clone();
                    let an = agent_name.to_string();
                    let mut zalo_cfg = bizclaw_core::config::ZaloChannelConfig::default();
                    zalo_cfg.mode = "personal".into();
                    if let Some(cookie) = cfg["personal"]["cookie"].as_str() {
                        zalo_cfg.personal.cookie_path = cookie.into(); // Hack: pass raw cookie
                    }
                    tokio::spawn(async move {
                        spawn_zalo_personal_listener(s, an, zalo_cfg).await;
                    });
                    connected += 1;
                }
            }
            "webhook" if !agent_name.is_empty() => {
                // Webhook is passive — inbound via /api/v1/webhook/inbound
                // No polling needed, just log that it's ready
                tracing::info!(
                    "[webhook] Instance '{}' bound to agent '{}' — ready for inbound at /api/v1/webhook/inbound",
                    inst["name"].as_str().unwrap_or(instance_id),
                    agent_name
                );
                connected += 1;
            }
            "zalo_bot" if !agent_name.is_empty() => {
                let bot_token = cfg["bot_token"].as_str().unwrap_or("").to_string();
                let secret_token = cfg["secret_token"].as_str().unwrap_or("").to_string();
                let webhook_url = cfg["webhook_url"].as_str().unwrap_or("").to_string();
                if !bot_token.is_empty() {
                    if webhook_url.is_empty() {
                        // Polling mode (dev/local)
                        let s = state.clone();
                        let an = agent_name.to_string();
                        let iid = instance_id.to_string();
                        tokio::spawn(async move {
                            spawn_zalo_bot_polling(s, an, bot_token, secret_token, iid).await;
                        });
                    } else {
                        // Webhook mode — messages arrive at /api/v1/webhook/zalo-bot
                        tracing::info!(
                            "[zalo-bot] Instance '{}' bound to agent '{}' — webhook mode at {}",
                            inst["name"].as_str().unwrap_or(instance_id),
                            agent_name,
                            webhook_url
                        );
                    }
                    connected += 1;
                }
            }
            _ => {}
        }
    }
    if connected > 0 {
        tracing::info!("📱 Auto-connected {} channel instance(s)", connected);
    }
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
        let cfg = state.full_config.lock().unwrap_or_else(|p| p.into_inner());
        cfg.channel
            .whatsapp
            .first()
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

                            let wa_config = {
                                let cfg =
                                    state.full_config.lock().unwrap_or_else(|p| p.into_inner());
                                cfg.channel.whatsapp.first().cloned()
                            };

                            let state_clone = state.clone();
                            tokio::spawn(async move {
                                let response = dispatch_to_channel_agent(
                                    state_clone.clone(),
                                    "whatsapp",
                                    Some(&from),
                                    &text,
                                )
                                .await
                                .unwrap_or_default();
                                if response.is_empty() {
                                    return;
                                }

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

// ═══ Xiaozhi Webhook Bridge ═══

/// Xiaozhi webhook inbound — receives voice commands from Xiaozhi Server.
/// POST /api/v1/xiaozhi/webhook
pub async fn xiaozhi_webhook(
    State(state): State<Arc<AppState>>,
    _headers: axum::http::HeaderMap,
    body: String,
) -> Json<serde_json::Value> {
    let start = std::time::Instant::now();

    let req: bizclaw_channels::xiaozhi::XiaozhiRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("[security] Invalid Xiaozhi request format: {e}");
            return Json(serde_json::json!({"ok": false, "error": "Invalid request format"}));
        }
    };

    if req.content.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "'content' field required"}));
    }

    tracing::info!(
        "[xiaozhi] Device {} → '{}' ({})",
        req.device_mac,
        safe_truncate(&req.content, 80),
        req.lang
    );

    let response = dispatch_to_channel_agent(state.clone(), "xiaozhi", None, &req.content)
        .await
        .unwrap_or_default();
    let processing_ms = start.elapsed().as_millis() as u64;

    Json(serde_json::json!({
        "ok": true,
        "text": response,
        "agent": "default",
        "session_id": req.session_id,
        "device_mac": req.device_mac,
        "has_audio": false,
        "processing_ms": processing_ms,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}

// ═══════════════════════════════════════════════════════════════════════
// ZALO OA WEBHOOK — Server-side Zalo without Android bridge
// ═══════════════════════════════════════════════════════════════════════

/// POST /api/v1/webhook/zalo-oa — Receive messages from Zalo Official Account.
pub async fn zalo_oa_webhook(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Json<serde_json::Value> {
    let body_str = String::from_utf8_lossy(&body);

    tracing::info!("[zalo-oa] Webhook received: {} bytes", body.len());

    // ── 1. Validate MAC signature (optional but recommended) ──
    let cfg = state
        .full_config
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let zalo_configs = &cfg.channel.zalo;
    let oa_config = zalo_configs.iter().find(|z| z.mode == "official");

    if let Some(mac_header) = headers.get("X-ZaloOA-Signature").or(headers.get("mac"))
        && let (Some(config), Ok(mac_value)) = (oa_config, mac_header.to_str())
    {
        let app_secret = &config.official.app_secret;
        if !app_secret.is_empty() {
            use hmac::Mac;
            type HmacSha256 = hmac::Hmac<sha2::Sha256>;
            if let Ok(mut mac) = HmacSha256::new_from_slice(app_secret.as_bytes()) {
                mac.update(body_str.as_bytes());
                let expected = hex::encode(mac.finalize().into_bytes());
                if mac_value != expected {
                    tracing::warn!("[zalo-oa] Invalid MAC signature");
                    return Json(serde_json::json!({"error": "Invalid signature"}));
                }
                tracing::debug!("[zalo-oa] MAC signature validated ✓");
            }
        }
    }

    // ── 2. Parse the webhook event ──
    let event: serde_json::Value = match serde_json::from_str(&body_str) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("[zalo-oa] Failed to parse webhook body: {e}");
            return Json(serde_json::json!({"ok": false, "error": "Invalid JSON"}));
        }
    };

    let event_name = event["event_name"].as_str().unwrap_or("");
    let timestamp = event["timestamp"].as_str().unwrap_or("");

    tracing::info!("[zalo-oa] Event: {} at {}", event_name, timestamp);

    match event_name {
        "user_send_text" => {
            let sender_id = event["sender"]["id"].as_str().unwrap_or("");
            let message_text = event["message"]["text"].as_str().unwrap_or("");
            let msg_id = event["message"]["msg_id"].as_str().unwrap_or("");

            if sender_id.is_empty() || message_text.is_empty() {
                return Json(serde_json::json!({"ok": false, "error": "Missing sender or text"}));
            }

            tracing::info!(
                "[zalo-oa] Text from {}: '{}' (msg_id={})",
                sender_id,
                if message_text.len() > 50 {
                    &message_text[..50]
                } else {
                    message_text
                },
                msg_id
            );

            let agent_response_opt =
                dispatch_to_channel_agent(state.clone(), "zalo", Some(sender_id), message_text)
                    .await;

            if let Some(agent_response) = agent_response_opt {
                if let Some(config) = oa_config {
                    let access_token = &config.official.access_token;
                    if !access_token.is_empty() {
                        let reply_payload = serde_json::json!({
                            "recipient": { "user_id": sender_id },
                            "message": { "text": agent_response }
                        });

                        let client = reqwest::Client::new();
                        match client
                            .post("https://openapi.zalo.me/v3.0/oa/message/cs")
                            .header("access_token", access_token.as_str())
                            .json(&reply_payload)
                            .send()
                            .await
                        {
                            Ok(resp) => {
                                let status = resp.status();
                                let reply_body =
                                    resp.json::<serde_json::Value>().await.unwrap_or_default();
                                if status.is_success()
                                    && reply_body["error"].as_i64().unwrap_or(-1) == 0
                                {
                                    tracing::info!(
                                        "[zalo-oa] ✅ Replied to {} successfully",
                                        sender_id
                                    );
                                } else {
                                    let err_msg =
                                        reply_body["message"].as_str().unwrap_or("Unknown");
                                    tracing::error!(
                                        "[zalo-oa] Reply failed: {} (code: {})",
                                        err_msg,
                                        reply_body["error"]
                                    );
                                }
                            }
                            Err(e) => tracing::error!("[zalo-oa] Reply failed: {}", e),
                        }
                    } else {
                        tracing::warn!("[zalo-oa] No access_token configured — cannot reply");
                    }
                }
            }

            Json(serde_json::json!({
                "ok": true,
                "event": "user_send_text",
                "sender": sender_id,
                "replied": true
            }))
        }

        "user_send_image" => {
            let sender_id = event["sender"]["id"].as_str().unwrap_or("");
            let image_url = event["message"]["attachments"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|a| a["payload"]["url"].as_str())
                .unwrap_or("");

            tracing::info!("[zalo-oa] Image from {}: {}", sender_id, image_url);

            Json(serde_json::json!({
                "ok": true,
                "event": "user_send_image",
                "sender": sender_id
            }))
        }

        "follow" => {
            let follower_id = event["follower"]["id"].as_str().unwrap_or("");
            tracing::info!("[zalo-oa] 🎉 New follower: {}", follower_id);

            if let Some(config) = oa_config {
                let access_token = &config.official.access_token;
                if !access_token.is_empty() {
                    let welcome = serde_json::json!({
                        "recipient": { "user_id": follower_id },
                        "message": { "text": "Xin chào! 👋 Cảm ơn bạn đã quan tâm. Tôi là trợ lý AI, hãy gửi tin nhắn để bắt đầu trò chuyện!" }
                    });
                    let client = reqwest::Client::new();
                    let _ = client
                        .post("https://openapi.zalo.me/v3.0/oa/message/cs")
                        .header("access_token", access_token.as_str())
                        .json(&welcome)
                        .send()
                        .await;
                    tracing::info!("[zalo-oa] Welcome message sent to {}", follower_id);
                }
            }

            Json(serde_json::json!({"ok": true, "event": "follow", "follower": follower_id}))
        }

        "unfollow" => {
            let follower_id = event["follower"]["id"].as_str().unwrap_or("");
            tracing::info!("[zalo-oa] 👋 Unfollowed by: {}", follower_id);
            Json(serde_json::json!({"ok": true, "event": "unfollow"}))
        }

        _ => {
            tracing::debug!("[zalo-oa] Unhandled event: {}", event_name);
            Json(serde_json::json!({"ok": true, "event": event_name, "handled": false}))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// FACEBOOK MESSENGER WEBHOOK — Receive/Reply via Page Messaging API
// ═══════════════════════════════════════════════════════════════════════

/// GET /api/v1/webhook/messenger — Meta webhook verification.
/// Meta sends: hub.mode=subscribe&hub.verify_token=xxx&hub.challenge=yyy
pub async fn messenger_webhook_verify(
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

    // Find verify_token from channel instances
    let instances = load_channel_instances(&state);
    let expected = instances
        .iter()
        .find(|i| {
            i["channel_type"].as_str() == Some("messenger") && i["enabled"].as_bool() == Some(true)
        })
        .and_then(|i| i["config"]["verify_token"].as_str())
        .unwrap_or("");

    if mode == "subscribe" && token == expected {
        tracing::info!("[messenger] ✅ Webhook verified by Meta");
        axum::response::Response::builder()
            .status(200)
            .body(axum::body::Body::from(challenge.to_string()))
            .unwrap()
    } else {
        tracing::warn!("[messenger] ❌ Webhook verification failed (token mismatch)");
        axum::response::Response::builder()
            .status(403)
            .body(axum::body::Body::from("Forbidden"))
            .unwrap()
    }
}

/// POST /api/v1/webhook/messenger — Receive incoming messages from Facebook Messenger.
pub async fn messenger_webhook(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Json<serde_json::Value> {
    let body_str = String::from_utf8_lossy(&body);
    tracing::info!("[messenger] Webhook received: {} bytes", body.len());

    // ── 1. Find Messenger channel instance ──
    let instances = load_channel_instances(&state);
    let messenger_inst = instances.iter().find(|i| {
        i["channel_type"].as_str() == Some("messenger") && i["enabled"].as_bool() == Some(true)
    });

    let (page_access_token, app_secret, agent_name) = match messenger_inst {
        Some(inst) => {
            let token = inst["config"]["page_access_token"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let secret = inst["config"]["app_secret"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let agent = inst["agent_name"].as_str().unwrap_or("").to_string();
            (token, secret, agent)
        }
        None => {
            tracing::warn!("[messenger] No enabled Messenger channel instance found");
            return Json(serde_json::json!({"status": "ok", "note": "no_instance"}));
        }
    };

    // ── 2. Validate HMAC-SHA256 signature (X-Hub-Signature-256) ──
    if !app_secret.is_empty() {
        if let Some(sig_header) = headers.get("x-hub-signature-256") {
            let sig_str = sig_header.to_str().unwrap_or("");
            // Format: sha256=<hex>
            let expected_sig = sig_str.strip_prefix("sha256=").unwrap_or("");
            use hmac::Mac;
            type HmacSha256 = hmac::Hmac<sha2::Sha256>;
            if let Ok(mut mac) = HmacSha256::new_from_slice(app_secret.as_bytes()) {
                mac.update(&body);
                let computed = hex::encode(mac.finalize().into_bytes());
                if computed != expected_sig {
                    tracing::warn!("[messenger] Invalid HMAC signature — rejecting");
                    return Json(serde_json::json!({"error": "Invalid signature"}));
                }
                tracing::debug!("[messenger] HMAC signature validated ✓");
            }
        }
    }

    // ── 3. Parse the webhook event ──
    let event: serde_json::Value = match serde_json::from_str(&body_str) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("[messenger] Failed to parse webhook body: {e}");
            return Json(serde_json::json!({"ok": false, "error": "Invalid JSON"}));
        }
    };

    // ── 4. Process messaging events ──
    if let Some(entries) = event["entry"].as_array() {
        for entry in entries {
            if let Some(messagings) = entry["messaging"].as_array() {
                for messaging in messagings {
                    let sender_id = messaging["sender"]["id"].as_str().unwrap_or("").to_string();
                    let page_id = messaging["recipient"]["id"].as_str().unwrap_or("");

                    // Skip echo messages (sent by the page itself)
                    if messaging["message"]["is_echo"].as_bool() == Some(true) {
                        continue;
                    }

                    // Text message
                    if let Some(text) = messaging["message"]["text"].as_str() {
                        if text.is_empty() || sender_id.is_empty() {
                            continue;
                        }
                        tracing::info!(
                            "[messenger] Message from {}: '{}' (page: {})",
                            sender_id,
                            safe_truncate(text, 80),
                            page_id
                        );

                        if state.paused_threads.read().await.contains(&sender_id) {
                            tracing::info!(
                                "⏸️ Handoff active: ignoring AI reply for Messenger thread {}",
                                sender_id
                            );
                            continue;
                        }

                        // Route to agent
                        let response = if !agent_name.is_empty() {
                            let mut orch = state.orchestrator.lock().await;
                            match orch.send_to(&agent_name, text).await {
                                Ok(r) => r,
                                Err(e) => format!("⚠️ Agent error: {e}"),
                            }
                        } else {
                            dispatch_to_channel_agent(
                                state.clone(),
                                "messenger",
                                Some(&sender_id),
                                text,
                            )
                            .await
                            .unwrap_or_default()
                        };

                        // Reply via Graph API
                        if !page_access_token.is_empty() {
                            let reply_payload = serde_json::json!({
                                "recipient": { "id": sender_id },
                                "message": { "text": response },
                                "messaging_type": "RESPONSE"
                            });
                            let client = reqwest::Client::new();
                            match client
                                .post("https://graph.facebook.com/v21.0/me/messages")
                                .query(&[("access_token", &page_access_token)])
                                .json(&reply_payload)
                                .send()
                                .await
                            {
                                Ok(resp) => {
                                    let status = resp.status();
                                    if status.is_success() {
                                        tracing::info!(
                                            "[messenger] ✅ Replied to {} successfully",
                                            sender_id
                                        );
                                    } else {
                                        let err = resp.text().await.unwrap_or_default();
                                        tracing::error!(
                                            "[messenger] Reply failed ({}): {}",
                                            status,
                                            safe_truncate(&err, 200)
                                        );
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("[messenger] Reply request failed: {e}");
                                }
                            }
                        } else {
                            tracing::warn!("[messenger] No page_access_token — cannot reply");
                        }
                    }

                    // Postback (button click)
                    if let Some(pb) = messaging["postback"]["payload"].as_str() {
                        tracing::info!("[messenger] Postback from {}: {}", sender_id, pb);
                    }
                }
            }
        }
    }

    // Meta requires 200 OK within 20 seconds
    Json(serde_json::json!({"status": "ok"}))
}

// ═══════════════════════════════════════════════════════════════════════
// HANDOFF (HUMAN-IN-THE-LOOP)
// ═══════════════════════════════════════════════════════════════════════

#[derive(serde::Deserialize)]
pub struct HandoffReq {
    pub thread_id: String,
}

pub async fn handoff_pause(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(req): axum::extract::Json<HandoffReq>,
) -> Json<serde_json::Value> {
    state
        .paused_threads
        .write()
        .await
        .insert(req.thread_id.clone());
    tracing::info!(
        "⏸️ Handoff manual override activated for thread: {}",
        req.thread_id
    );
    Json(serde_json::json!({
        "ok": true,
        "thread_id": req.thread_id,
        "status": "paused"
    }))
}

pub async fn handoff_resume(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(req): axum::extract::Json<HandoffReq>,
) -> Json<serde_json::Value> {
    state.paused_threads.write().await.remove(&req.thread_id);
    tracing::info!("▶️ AI resumed for thread: {}", req.thread_id);
    Json(serde_json::json!({
        "ok": true,
        "thread_id": req.thread_id,
        "status": "active"
    }))
}

/// Spawn Zalo Personal real-time listener.
pub async fn spawn_zalo_personal_listener(
    state: Arc<AppState>,
    agent_name: String,
    config: bizclaw_core::config::ZaloChannelConfig,
) {
    use bizclaw_core::traits::Channel;
    use futures::StreamExt;

    let mut channel = bizclaw_channels::zalo::ZaloChannel::new(config.clone());
    if let Err(e) = channel.connect().await {
        tracing::error!("[zalo-personal] Connect failed: {e}");
        return;
    }
    tracing::info!("[zalo-personal] Connected → agent '{}'", agent_name);

    if let Ok(mut stream) = channel.listen().await {
        while let Some(msg) = stream.next().await {
            let thread_id = msg.thread_id.clone();
            let sender_id = msg.sender_id.clone();
            let sender_name = msg.sender_name.clone();
            let content = msg.content.clone();

            // Log to group_messages for summarization
            let db = state.db.clone();
            let group_id_clone = thread_id.clone();
            let s_id_clone = sender_id.clone();
            let s_name_clone = sender_name.clone();
            let content_clone = content.clone();
            tokio::task::spawn_blocking(move || {
                let _ = db.insert_group_message(
                    &group_id_clone,
                    &s_id_clone,
                    s_name_clone.as_deref(),
                    &content_clone,
                );
            });

            // For Zalo Personal, if sender != thread_id, it is likely a group.
            let is_group = sender_id != thread_id && !thread_id.is_empty();
            let is_mention = content.to_lowercase().contains("@agent")
                || content.to_lowercase().contains(&agent_name.to_lowercase());

            if !is_group || is_mention {
                tracing::info!(
                    "[zalo-personal] {} → agent '{}': {}",
                    sender_id,
                    agent_name,
                    crate::routes::safe_truncate(&content, 100)
                );

                // Fire and forget reply to avoid locking the stream
                let channel_clone = bizclaw_channels::zalo::ZaloChannel::new(config.clone());
                let s = state.clone();
                let an = agent_name.clone();
                tokio::spawn(async move {
                    let mut channel = channel_clone;
                    if channel.connect().await.is_ok() {
                        let response = {
                            let mut orch = s.orchestrator.lock().await;
                            match orch.send_to(&an, &content).await {
                                Ok(r) => r,
                                Err(e) => format!("⚠️ Agent error: {e}"),
                            }
                        };
                        let _ = channel
                            .send(bizclaw_core::types::OutgoingMessage {
                                thread_id: thread_id.clone(),
                                thread_type: bizclaw_core::types::ThreadType::Direct,
                                content: response,
                                reply_to: None,
                            })
                            .await;
                    }
                });
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ZALO BOT PLATFORM — Official Bot API (token-based, like Telegram)
// ═══════════════════════════════════════════════════════════════════════

/// POST /api/v1/webhook/zalo-bot — Receive messages from Zalo Bot Platform.
/// Validates X-Bot-Api-Secret-Token header against configured secret.
pub async fn zalo_bot_webhook(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // Find the zalo_bot channel instance
    let instances = load_channel_instances(&state);
    let bot_instance = instances.iter().find(|i| {
        i["channel_type"].as_str() == Some("zalo_bot")
            && i["enabled"].as_bool() == Some(true)
            && !i["agent_name"].as_str().unwrap_or("").is_empty()
    });

    let (agent_name, secret_token) = match bot_instance {
        Some(inst) => {
            let agent = inst["agent_name"].as_str().unwrap_or("").to_string();
            let secret = inst["config"]["secret_token"]
                .as_str()
                .unwrap_or("")
                .to_string();
            (agent, secret)
        }
        None => {
            return Json(serde_json::json!({
                "ok": false,
                "error": "No Zalo Bot channel bound to an agent. Create one in Dashboard → Channels."
            }));
        }
    };

    // Validate secret token
    if !secret_token.is_empty() {
        let header_token = headers
            .get("x-bot-api-secret-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if header_token != secret_token {
            tracing::warn!("[zalo-bot] Invalid secret token from webhook");
            return Json(serde_json::json!({"ok": false, "error": "Unauthorized"}));
        }
    }

    // Parse the update
    let update: bizclaw_channels::zalo_bot::ZaloBotWebhookPayload =
        match serde_json::from_value(body.clone()) {
            Ok(u) => u,
            Err(e) => {
                tracing::warn!("[zalo-bot] Invalid webhook payload: {e}");
                return Json(serde_json::json!({"ok": false, "error": "Invalid payload"}));
            }
        };

    if let Some(msg) =
        bizclaw_channels::zalo_bot::ZaloBotChannel::parse_webhook_update(&update.result)
    {
        let chat_id = msg.thread_id.clone();
        let sender = msg.sender_name.clone().unwrap_or_default();
        let text = msg.content.clone();

        tracing::info!(
            "[zalo-bot] {} ({}) → agent '{}': {}",
            sender,
            msg.sender_id,
            agent_name,
            safe_truncate(&text, 100)
        );

        // Get bot token for replying
        let bot_token = bot_instance
            .and_then(|i| i["config"]["bot_token"].as_str())
            .unwrap_or("")
            .to_string();

        let state_clone = state.clone();
        let _agent_name_clone = agent_name.clone();
        tokio::spawn(async move {
            // Send typing indicator
            let reply_bot = bizclaw_channels::zalo_bot::ZaloBotChannel::new(
                bizclaw_channels::zalo_bot::ZaloBotConfig {
                    bot_token: bot_token.clone(),
                    ..Default::default()
                },
            );
            let _ = reply_bot.send_typing(&chat_id).await;

            // Route to agent
            let response =
                dispatch_to_channel_agent(state_clone.clone(), "zalo_bot", Some(&chat_id), &text)
                    .await
                    .unwrap_or_default();
            if response.is_empty() {
                return;
            }

            // Reply via Zalo Bot API
            if let Err(e) = reply_bot.send_message(&chat_id, &response).await {
                tracing::error!("[zalo-bot] Reply failed: {e}");
            }
        });
    }

    Json(serde_json::json!({"ok": true}))
}

/// Spawn a Zalo Bot polling loop (like Telegram polling) for dev/local use.
pub async fn spawn_zalo_bot_polling(
    state: Arc<AppState>,
    agent_name: String,
    bot_token: String,
    secret_token: String,
    instance_id: String,
) {
    let bot = bizclaw_channels::zalo_bot::ZaloBotChannel::new(
        bizclaw_channels::zalo_bot::ZaloBotConfig {
            bot_token: bot_token.clone(),
            secret_token: secret_token.clone(),
            ..Default::default()
        },
    );

    // Verify bot token
    match bot.get_me().await {
        Ok(info) => {
            tracing::info!(
                "[zalo-bot] {} connected → agent '{}' (instance: {})",
                info.account_name,
                agent_name,
                instance_id
            );
        }
        Err(e) => {
            tracing::error!(
                "[zalo-bot] Bot token invalid for instance '{}': {}",
                instance_id,
                e
            );
            return;
        }
    }

    // Delete any existing webhook to enable polling
    let _ = bot.delete_webhook().await;

    let state_clone = state.clone();
    let agent_name_clone = agent_name.clone();

    tokio::spawn(async move {
        let poll_bot = bizclaw_channels::zalo_bot::ZaloBotChannel::new(
            bizclaw_channels::zalo_bot::ZaloBotConfig {
                bot_token: bot_token.clone(),
                ..Default::default()
            },
        );

        loop {
            match poll_bot.get_updates().await {
                Ok(updates) => {
                    for update in updates {
                        if let Some(msg) =
                            bizclaw_channels::zalo_bot::ZaloBotChannel::parse_webhook_update(
                                &update,
                            )
                        {
                            let chat_id = msg.thread_id.clone();
                            let sender = msg.sender_name.clone().unwrap_or_default();
                            let text = msg.content.clone();

                            // Handoff check
                            let lower = text.to_lowercase();
                            if lower.contains("gặp nhân viên")
                                || lower.contains("gap nhan vien")
                                || lower.contains("chuyển khách")
                            {
                                tracing::info!(
                                    "📞 Handoff hotword detected in Zalo Bot: '{}'",
                                    text
                                );
                                let req = crate::routes::api_handoff::HandoffRequestPayload {
                                    customer: sender.clone(),
                                    channel: Some("Zalo Bot".to_string()),
                                    reason: Some(
                                        "Khách hàng yêu cầu hỗ trợ từ nhân viên qua Zalo Bot"
                                            .to_string(),
                                    ),
                                    message: Some(text.clone()),
                                };
                                let _ = crate::routes::api_handoff::execute_handoff(
                                    state_clone.clone(),
                                    req,
                                )
                                .await;

                                let reply =
                                    crate::routes::api_handoff::load_handoff_settings(&state_clone)
                                        .greeting;
                                let _ = poll_bot.send_message(&chat_id, &reply).await;
                                continue;
                            }

                            tracing::info!(
                                "[zalo-bot] {} → agent '{}': {}",
                                sender,
                                agent_name_clone,
                                safe_truncate(&text, 100)
                            );
                            let _ = poll_bot.send_typing(&chat_id).await;

                            // Route to agent
                            let response = {
                                let mut orch = state_clone.orchestrator.lock().await;
                                match orch.send_to(&agent_name_clone, &text).await {
                                    Ok(r) => r,
                                    Err(e) => format!("⚠️ Agent error: {e}"),
                                }
                            };

                            if let Err(e) = poll_bot.send_message(&chat_id, &response).await {
                                tracing::error!("[zalo-bot] Reply failed: {e}");
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("[zalo-bot] Polling error for '{}': {e}", agent_name_clone);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    });
}
