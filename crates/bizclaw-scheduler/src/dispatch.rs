//! Notification dispatch — actually sends notifications to configured channels.
//! Supports: Telegram Bot API, Discord Webhook, HTTP Webhook, Dashboard WebSocket.

use super::notify::{Notification, NotifyPriority};

/// Notification target configuration.
#[derive(Debug, Clone)]
pub enum NotifyTarget {
    /// Telegram Bot API — send via `sendMessage`.
    Telegram { bot_token: String, chat_id: String },
    /// Discord Webhook URL.
    Discord { webhook_url: String },
    /// Generic HTTP webhook — POST with JSON body.
    Webhook {
        url: String,
        headers: Vec<(String, String)>,
    },
    /// Dashboard WebSocket broadcast (handled at gateway level).
    Dashboard,
}

/// Dispatch a notification to a target channel.
/// Returns Ok(()) on success, Err(reason) on failure.
pub async fn dispatch(notification: &Notification, target: &NotifyTarget) -> Result<(), String> {
    match target {
        NotifyTarget::Telegram { bot_token, chat_id } => {
            send_telegram(bot_token, chat_id, notification).await
        }
        NotifyTarget::Discord { webhook_url } => send_discord(webhook_url, notification).await,
        NotifyTarget::Webhook { url, headers } => send_webhook(url, headers, notification).await,
        NotifyTarget::Dashboard => {
            // Dashboard notifications are handled at gateway level (WebSocket broadcast).
            // This is just a marker — the gateway intercepts this.
            tracing::debug!("📊 Dashboard notification recorded: {}", notification.title);
            Ok(())
        }
    }
}

/// Send notification via Telegram Bot API.
async fn send_telegram(
    bot_token: &str,
    chat_id: &str,
    notification: &Notification,
) -> Result<(), String> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    let priority_emoji = match notification.priority {
        NotifyPriority::Urgent => "🚨",
        NotifyPriority::High => "⚠️",
        NotifyPriority::Normal => "📢",
        NotifyPriority::Low => "ℹ️",
    };

    let text = format!(
        "{} *{}*\n\n{}\n\n_Source: {} • {}_",
        priority_emoji,
        escape_markdown(&notification.title),
        escape_markdown(&notification.body),
        escape_markdown(&notification.source),
        notification.timestamp.format("%H:%M:%S UTC")
    );

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "Markdown"
        }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Telegram send failed: {e}"))?;

    if resp.status().is_success() {
        tracing::info!("✅ Telegram notification sent: {}", notification.title);
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Telegram API error {status}: {body}"))
    }
}

/// Send notification via Discord Webhook.
async fn send_discord(webhook_url: &str, notification: &Notification) -> Result<(), String> {
    let color = match notification.priority {
        NotifyPriority::Urgent => 0xFF0000, // Red
        NotifyPriority::High => 0xFF8800,   // Orange
        NotifyPriority::Normal => 0x00AAFF, // Blue
        NotifyPriority::Low => 0x888888,    // Gray
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(webhook_url)
        .json(&serde_json::json!({
            "embeds": [{
                "title": notification.title,
                "description": notification.body,
                "color": color,
                "footer": {
                    "text": format!("Source: {} • {}", notification.source, notification.timestamp.format("%H:%M:%S UTC"))
                }
            }]
        }))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Discord send failed: {e}"))?;

    if resp.status().is_success() {
        tracing::info!("✅ Discord notification sent: {}", notification.title);
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Discord webhook error {status}: {body}"))
    }
}

/// Send notification via generic HTTP webhook.
async fn send_webhook(
    url: &str,
    headers: &[(String, String)],
    notification: &Notification,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let mut req = client
        .post(url)
        .json(&serde_json::json!({
            "title": notification.title,
            "body": notification.body,
            "priority": format!("{:?}", notification.priority),
            "source": notification.source,
            "timestamp": notification.timestamp.to_rfc3339(),
        }))
        .timeout(std::time::Duration::from_secs(10));

    for (key, value) in headers {
        req = req.header(key.as_str(), value.as_str());
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("Webhook send failed: {e}"))?;

    if resp.status().is_success() {
        tracing::info!(
            "✅ Webhook notification sent to {}: {}",
            url,
            notification.title
        );
        Ok(())
    } else {
        let status = resp.status();
        Err(format!("Webhook error {status}"))
    }
}

/// Escape Telegram MarkdownV1 special characters.
fn escape_markdown(s: &str) -> String {
    s.replace('_', "\\_")
        .replace('*', "\\*")
        .replace('[', "\\[")
        .replace('`', "\\`")
}

/// Convenience: dispatch to all registered targets.
/// Returns a Vec of (target_name, Result).
pub async fn dispatch_all(
    notification: &Notification,
    targets: &[(&str, NotifyTarget)],
) -> Vec<(String, Result<(), String>)> {
    let mut results = Vec::new();
    for (name, target) in targets {
        let result = dispatch(notification, target).await;
        results.push((name.to_string(), result));
    }
    results
}

/// Build NotifyTargets from BizClaw channel config.
/// Called at server init to configure notification dispatch.
pub fn targets_from_config(
    config: &bizclaw_core::config::BizClawConfig,
) -> Vec<(String, NotifyTarget)> {
    let mut targets = Vec::new();

    // Telegram
    if let Some(tg) = &config.channel.telegram {
        if tg.enabled && !tg.bot_token.is_empty() {
            // We don't know the chat_id at init time — it's per-user.
            // For notifications, the admin should configure a notification chat_id.
            // For now, we'll store the token and use a config-based chat_id.
            let chat_id = std::env::var("BIZCLAW_NOTIFY_TELEGRAM_CHAT_ID").unwrap_or_default();
            if !chat_id.is_empty() {
                targets.push((
                    "telegram".to_string(),
                    NotifyTarget::Telegram {
                        bot_token: tg.bot_token.clone(),
                        chat_id,
                    },
                ));
            }
        }
    }

    // Webhook
    if let Some(wh) = &config.channel.webhook {
        if wh.enabled && !wh.outbound_url.is_empty() {
            targets.push((
                "webhook".to_string(),
                NotifyTarget::Webhook {
                    url: wh.outbound_url.clone(),
                    headers: vec![],
                },
            ));
        }
    }

    // Dashboard is always available
    targets.push(("dashboard".to_string(), NotifyTarget::Dashboard));

    targets
}
