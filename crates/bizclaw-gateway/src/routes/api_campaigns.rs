use axum::{
    Json,
    extract::{Path, State},
};

use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::server::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub channels: Vec<String>,
    pub segment: String,
    pub message: String,
    pub status: String,
    pub sent: i64,
    pub delivered: i64,
    pub read: i64,
    pub created_at: String,
    pub schedule_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCampaignRequest {
    pub name: String,
    pub channels: Vec<String>,
    pub segment: String,
    pub message: String,
    pub status: String,
    pub schedule_at: Option<String>,
}

pub async fn list_campaigns(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let conn = match state.db.lock_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"error": format!("DB error: {e}")})),
    };

    // Ensure table exists
    conn.execute(
        "CREATE TABLE IF NOT EXISTS campaigns (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            channels TEXT NOT NULL,
            segment TEXT NOT NULL,
            message TEXT NOT NULL,
            status TEXT NOT NULL,
            sent INTEGER DEFAULT 0,
            delivered INTEGER DEFAULT 0,
            read_count INTEGER DEFAULT 0,
            created_at TEXT NOT NULL,
            schedule_at TEXT
        )",
        [],
    )
    .unwrap();

    let mut stmt = conn.prepare("SELECT id, name, channels, segment, message, status, sent, delivered, read_count, created_at, schedule_at FROM campaigns ORDER BY created_at DESC").unwrap();

    let mut campaigns = Vec::new();
    let iter = stmt
        .query_map([], |row| {
            let channels_str: String = row.get(2)?;
            let channels: Vec<String> = serde_json::from_str(&channels_str).unwrap_or_default();
            Ok(Campaign {
                id: row.get(0)?,
                name: row.get(1)?,
                channels,
                segment: row.get(3)?,
                message: row.get(4)?,
                status: row.get(5)?,
                sent: row.get(6)?,
                delivered: row.get(7)?,
                read: row.get(8)?,
                created_at: row.get(9)?,
                schedule_at: row.get(10)?,
            })
        })
        .unwrap();

    for c in iter {
        if let Ok(cam) = c {
            campaigns.push(cam);
        }
    }

    Json(serde_json::json!({ "campaigns": campaigns }))
}

pub async fn create_campaign(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateCampaignRequest>,
) -> Json<serde_json::Value> {
    let conn = match state.db.lock_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"error": format!("DB error: {e}")})),
    };
    let id = Uuid::new_v4().to_string();
    let created_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let channels_json = serde_json::to_string(&req.channels).unwrap_or_default();

    conn.execute(
        "INSERT INTO campaigns (id, name, channels, segment, message, status, created_at, schedule_at) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, req.name, channels_json, req.segment, req.message, req.status, created_at, req.schedule_at],
    ).unwrap();

    let mut activity_log = state.activity_log.lock().unwrap();
    activity_log.push(crate::openai_compat::ActivityEvent {
        timestamp: chrono::Utc::now(),
        event_type: "campaign_created".into(),
        agent: "System".into(),
        detail: format!("Tạo chiến dịch mới: {}", req.name),
    });

    Json(serde_json::json!({ "id": id, "status": "success" }))
}

pub async fn delete_campaign(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let conn = match state.db.lock_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"error": format!("DB error: {e}")})),
    };
    conn.execute("DELETE FROM campaigns WHERE id = ?1", params![id])
        .unwrap();

    Json(serde_json::json!({ "status": "deleted" }))
}

pub async fn run_campaign(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let conn = match state.db.lock_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"error": format!("DB error: {e}")})),
    };

    // Update status to running
    conn.execute(
        "UPDATE campaigns SET status = 'running' WHERE id = ?1",
        params![id],
    )
    .unwrap();

    // and handles the AppleScript

    Json(serde_json::json!({ "status": "running" }))
}

pub async fn update_campaign(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateCampaignRequest>,
) -> Json<serde_json::Value> {
    let conn = match state.db.lock_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"error": format!("DB error: {e}")})),
    };
    let channels_json = serde_json::to_string(&req.channels).unwrap_or_default();

    conn.execute(
        "UPDATE campaigns SET name = ?1, channels = ?2, segment = ?3, message = ?4, status = ?5, schedule_at = ?6 WHERE id = ?7",
        params![req.name, channels_json, req.segment, req.message, req.status, req.schedule_at, id],
    ).unwrap();

    Json(serde_json::json!({ "id": id, "status": "updated" }))
}
