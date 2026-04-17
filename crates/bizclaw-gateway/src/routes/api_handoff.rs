use axum::{
    Json,
    extract::{Path, State},
};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Arc;
use uuid::Uuid;

use crate::server::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct HandoffTicket {
    pub id: String,
    pub customer: String,
    pub channel: String,
    pub reason: String,
    pub message: String,
    pub status: String,
    pub context_summary: String,
    pub ai_attempts: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingHours {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerRouting {
    pub notify_channels: Vec<String>,
    pub assignee_group: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffSettings {
    pub enabled: bool,
    pub auto_handoff: bool,
    pub notify_channels: Vec<String>,
    pub assignee_group: Option<String>,
    pub triggers: Vec<String>,
    #[serde(default)]
    pub trigger_configs: std::collections::HashMap<String, TriggerRouting>,
    pub greeting: String,
    pub resume_greeting: String,
    pub timeout_minutes: i64,
    pub working_hours: WorkingHours,
    pub fallback_message: String,
}

impl Default for HandoffSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_handoff: true,
            notify_channels: vec!["zalo".into(), "telegram".into()],
            assignee_group: Some("general".into()),
            triggers: vec!["low_confidence".into(), "complaint".into(), "explicit_request".into()],
            trigger_configs: std::collections::HashMap::new(),
            greeting: "Dạ em xin phép chuyển cuộc trò chuyện cho đồng nghiệp hỗ trợ anh/chị tốt hơn ạ. Vui lòng đợi trong giây lát! 🙏".into(),
            resume_greeting: "AI Assistant đã quay lại phục vụ anh/chị! Nếu cần gặp nhân viên, cứ nhắn \"gặp nhân viên\" nhé 😊".into(),
            timeout_minutes: 30,
            working_hours: WorkingHours {
                start: "08:00".into(),
                end: "22:00".into()
            },
            fallback_message: "Hiện đang ngoài giờ làm việc. Tin nhắn của anh/chị đã được ghi nhận, chúng em sẽ phản hồi sớm nhất vào sáng mai ạ!".into(),
        }
    }
}

fn get_settings_path(state: &AppState) -> std::path::PathBuf {
    state
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("handoff-settings.json")
}

pub fn load_handoff_settings(state: &AppState) -> HandoffSettings {
    let path = get_settings_path(state);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str(&content) {
                return settings;
            }
        }
    }
    HandoffSettings::default()
}

pub async fn get_handoff_settings(State(state): State<Arc<AppState>>) -> Json<HandoffSettings> {
    Json(load_handoff_settings(&state))
}

pub async fn save_handoff_settings(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(settings): axum::extract::Json<HandoffSettings>,
) -> Json<serde_json::Value> {
    let path = get_settings_path(&state);
    if let Ok(content) = serde_json::to_string_pretty(&settings) {
        let _ = fs::write(path, content);
    }
    Json(serde_json::json!({"status": "ok"}))
}

pub async fn list_handoff_queue(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let conn = match state.db.lock_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"error": format!("DB error: {e}")})),
    };

    // Ensure table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS handoff_queue (
            id TEXT PRIMARY KEY,
            customer TEXT NOT NULL,
            channel TEXT NOT NULL,
            reason TEXT NOT NULL,
            message TEXT NOT NULL,
            status TEXT NOT NULL,
            context_summary TEXT NOT NULL,
            ai_attempts INTEGER DEFAULT 0,
            created_at TEXT NOT NULL
        )",
        [],
    )
    .unwrap();

    let mut stmt = conn.prepare("SELECT id, customer, channel, reason, message, status, context_summary, ai_attempts, created_at FROM handoff_queue ORDER BY created_at DESC").unwrap();

    let mut queue = Vec::new();
    let iter = stmt
        .query_map([], |row| {
            Ok(HandoffTicket {
                id: row.get(0)?,
                customer: row.get(1)?,
                channel: row.get(2)?,
                reason: row.get(3)?,
                message: row.get(4)?,
                status: row.get(5)?,
                context_summary: row.get(6)?,
                ai_attempts: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .unwrap();

    for t in iter {
        if let Ok(ticket) = t {
            queue.push(ticket);
        }
    }

    Json(serde_json::json!({ "queue": queue }))
}

pub async fn resolve_handoff(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let conn = match state.db.lock_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"error": format!("DB error: {e}")})),
    };
    conn.execute(
        "UPDATE handoff_queue SET status = 'resolved' WHERE id = ?1",
        params![id],
    )
    .unwrap();

    let mut activity_log = state.activity_log.lock().unwrap();
    activity_log.push(crate::openai_compat::ActivityEvent {
        timestamp: chrono::Utc::now(),
        event_type: "handoff_resolved".into(),
        agent: "Human".into(),
        detail: format!("Nhân viên đã xử lý ticket Handoff: {}", id),
    });

    Json(serde_json::json!({ "status": "resolved" }))
}

pub async fn delete_handoff(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let conn = match state.db.lock_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"error": format!("DB error: {e}")})),
    };
    conn.execute("DELETE FROM handoff_queue WHERE id = ?1", params![id])
        .unwrap();

    let mut activity_log = state.activity_log.lock().unwrap();
    activity_log.push(crate::openai_compat::ActivityEvent {
        timestamp: chrono::Utc::now(),
        event_type: "handoff_deleted".into(),
        agent: "Human".into(),
        detail: format!("Nhân viên đã xoá ticket Handoff: {}", id),
    });

    Json(serde_json::json!({ "status": "deleted" }))
}

#[derive(Debug, Deserialize)]
pub struct HandoffRequestPayload {
    pub customer: String,
    pub channel: Option<String>,
    pub reason: Option<String>,
    pub message: Option<String>,
}

pub async fn request_handoff(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(req): axum::extract::Json<HandoffRequestPayload>,
) -> Json<serde_json::Value> {
    execute_handoff(state, req).await
}

pub async fn execute_handoff(
    state: Arc<AppState>,
    req: HandoffRequestPayload,
) -> Json<serde_json::Value> {
    let id = Uuid::new_v4().to_string();
    let channel = req.channel.unwrap_or_else(|| "unknown".into());
    let reason = req
        .reason
        .unwrap_or_else(|| "Cần hỗ trợ từ nhân viên".into());
    let message = req.message.unwrap_or_else(|| "".into());
    let created_at = chrono::Utc::now().to_rfc3339();

    {
        let conn = match state.db.lock_conn() {
            Ok(c) => c,
            Err(e) => return Json(serde_json::json!({"error": format!("DB error: {e}")})),
        };

        conn.execute(
            "CREATE TABLE IF NOT EXISTS handoff_queue (
                id TEXT PRIMARY KEY,
                customer TEXT NOT NULL,
                channel TEXT NOT NULL,
                reason TEXT NOT NULL,
                message TEXT NOT NULL,
                status TEXT NOT NULL,
                context_summary TEXT NOT NULL,
                ai_attempts INTEGER DEFAULT 0,
                created_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();

        if let Err(e) = conn.execute(
            "INSERT INTO handoff_queue (id, customer, channel, reason, message, status, context_summary, ai_attempts, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'waiting', '', 0, ?6)",
            params![id, req.customer, channel, reason, message, created_at],
        ) {
            tracing::error!("Failed to insert into handoff_queue: {}", e);
        }
    }

    // Pause the thread for AI so it stops responding automatically
    state
        .paused_threads
        .write()
        .await
        .insert(req.customer.clone());
    tracing::info!(
        "⏸️ AI paused for thread: {} due to Hand-off request",
        req.customer
    );

    // Send Zalo Notification
    let cfg = state
        .full_config
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let mut sent_notification = false;

    // Check all Zalo channels, if any has notify_user_id and oa_access_token, dispatch.
    for zalo_cfg in cfg.channel.zalo.iter() {
        let access_token = &zalo_cfg.oa_access_token;
        let admin_id = &zalo_cfg.notify_user_id;

        if !access_token.is_empty() && !admin_id.is_empty() {
            let n_msg = format!(
                "🚨 [Có Khách Cần Hỗ Trợ]\nKhách hàng: {}\nLý do: {}\nTin nhắn: {}",
                req.customer, reason, message
            );

            tokio::spawn({
                let token = access_token.clone();
                let uid = admin_id.clone();
                async move {
                    let payload = serde_json::json!({
                        "recipient": { "user_id": uid },
                        "message": { "text": n_msg }
                    });
                    let client = reqwest::Client::new();
                    let _ = client
                        .post("https://openapi.zalo.me/v3.0/oa/message/cs")
                        .header("access_token", token)
                        .json(&payload)
                        .send()
                        .await;
                }
            });
            sent_notification = true;
            break;
        }
    }

    // Check Telegram channel admins
    for tg_cfg in cfg.channel.telegram.iter() {
        let bot_token = tg_cfg.resolve_bot_token();
        if !bot_token.is_empty() && !tg_cfg.allowed_chat_ids.is_empty() {
            let n_msg = format!(
                "🚨 <b>Có Khách Cần Hỗ Trợ</b>\nKhách hàng: <code>{}</code>\nLý do: {}\nTin nhắn: {}",
                req.customer, reason, message
            );
            for admin_id in &tg_cfg.allowed_chat_ids {
                tokio::spawn({
                    let token = bot_token.clone();
                    let chat_id = *admin_id;
                    let text = n_msg.clone();
                    async move {
                        let payload = serde_json::json!({
                            "chat_id": chat_id,
                            "text": text,
                            "parse_mode": "HTML"
                        });
                        let client = reqwest::Client::new();
                        let _ = client
                            .post(&format!(
                                "https://api.telegram.org/bot{}/sendMessage",
                                token
                            ))
                            .json(&payload)
                            .send()
                            .await;
                    }
                });
            }
            sent_notification = true;
            break;
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "id": id,
        "notified": sent_notification,
    }))
}
