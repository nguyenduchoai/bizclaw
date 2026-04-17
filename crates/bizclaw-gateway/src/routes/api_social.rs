//! Single-Tenant Social Automation
//! Runs locally on the Gateway (Kiosk) to securely auto-post and reply to comments.
//!
//! Driven by `dashboard.html` Channels config:
//! - messenger: `page_access_token`
//! - instagram: `ig_user_id`, `access_token`
//! - twitter: `api_params` (consumer_key, consumer_secret, access_token, token_secret)

use crate::server::AppState;
use axum::Json;
use axum::extract::State;
use bizclaw_core::traits::Tool;
use bizclaw_tools::social_post::SocialPostTool;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub source_urls: Vec<String>,
    pub platforms: Vec<String>,
    pub ai_prompt: String,
    pub schedule: String,
    pub is_active: bool,
}

pub async fn get_pipeline_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match state.db.get_setting("social_pipeline_cfg") {
        Ok(Some(json)) => {
            if let Ok(cfg) = serde_json::from_str::<PipelineConfig>(&json) {
                Json(serde_json::json!({"ok": true, "config": cfg}))
            } else {
                Json(serde_json::json!({"ok": true, "config": serde_json::Value::Null}))
            }
        }
        _ => Json(serde_json::json!({"ok": true, "config": serde_json::Value::Null})),
    }
}

pub async fn update_pipeline_config(
    State(state): State<Arc<AppState>>,
    Json(config): Json<PipelineConfig>,
) -> Json<serde_json::Value> {
    if let Ok(json) = serde_json::to_string(&config) {
        if state.db.set_setting("social_pipeline_cfg", &json).is_ok() {
            return Json(serde_json::json!({"ok": true}));
        }
    }
    Json(serde_json::json!({"ok": false, "error": "Failed to save pipeline config"}))
}

pub async fn trigger_pipeline(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let cfg_json = state
        .db
        .get_setting("social_pipeline_cfg")
        .unwrap_or_default()
        .unwrap_or_default();
    let config: PipelineConfig = match serde_json::from_str(&cfg_json) {
        Ok(c) => c,
        Err(_) => {
            return Json(serde_json::json!({"ok": false, "error": "Pipeline not configured"}));
        }
    };

    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        social_automation_worker(state_clone, config).await;
    });

    Json(serde_json::json!({"ok": true, "status": "Pipeline triggered in background"}))
}

pub async fn social_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let instances = super::load_channel_instances(&state);

    let fb_enabled = instances
        .iter()
        .any(|i| i["channel_type"] == "messenger" && i["enabled"] == true);
    let ig_enabled = instances
        .iter()
        .any(|i| i["channel_type"] == "instagram" && i["enabled"] == true);
    let x_enabled = instances
        .iter()
        .any(|i| i["channel_type"] == "twitter" && i["enabled"] == true);

    let cfg_json = state
        .db
        .get_setting("social_pipeline_cfg")
        .unwrap_or_default()
        .unwrap_or_default();
    let pipeline_active = if let Ok(c) = serde_json::from_str::<PipelineConfig>(&cfg_json) {
        c.is_active
    } else {
        false
    };

    Json(serde_json::json!({
        "ok": true,
        "facebook_connected": fb_enabled,
        "instagram_connected": ig_enabled,
        "x_connected": x_enabled,
        "pipeline_active": pipeline_active
    }))
}

async fn social_automation_worker(state: Arc<AppState>, config: PipelineConfig) {
    if config.source_urls.is_empty() || config.platforms.is_empty() {
        return;
    }

    let url = &config.source_urls[0];
    let jina_url = format!("https://r.jina.ai/{}", url);
    let crawl_res = match reqwest::get(&jina_url).await {
        Ok(res) => res.text().await.unwrap_or_default(),
        Err(_) => String::new(),
    };
    let clean_text = if crawl_res.is_empty() {
        "No text from source".to_string()
    } else {
        crawl_res
    };

    let prompt = if config.ai_prompt.is_empty() {
        format!(
            "Hãy viết lại bài đăng mạng xã hội thật thu hút cho thông tin sau, kèm hashtag:\n{}",
            clean_text
        )
    } else {
        format!("{}\n\n{}", config.ai_prompt, clean_text)
    };

    let post_content =
        super::api_webhooks::dispatch_to_channel_agent(state.clone(), "webhook", None, &prompt)
            .await
            .unwrap_or_default();
    if post_content.is_empty() {
        return;
    }

    // Get instances for API keys
    let instances = super::load_channel_instances(&state);

    for platform in &config.platforms {
        match platform.as_str() {
            "facebook" => {
                if let Some(inst) = instances.iter().find(|i| i["channel_type"] == "messenger") {
                    let token = inst["config"]["page_access_token"]
                        .as_str()
                        .unwrap_or_default();
                    let page_id = "me"; // Basic Page Graph API fallback via /me/feed
                    if !token.is_empty() {
                        let http = reqwest::Client::new();
                        let _ = http
                            .post(format!("https://graph.facebook.com/v21.0/{}/feed", page_id))
                            .form(&[
                                ("message", &post_content),
                                ("access_token", &token.to_string()),
                            ])
                            .send()
                            .await;
                    }
                }
            }
            "instagram" => {
                if let Some(inst) = instances.iter().find(|i| i["channel_type"] == "instagram") {
                    let ig_user_id = inst["config"]["ig_user_id"].as_str().unwrap_or_default();
                    let token = inst["config"]["access_token"].as_str().unwrap_or_default();
                    if !token.is_empty() && !ig_user_id.is_empty() {
                        let tool = SocialPostTool::new();
                        let payload = serde_json::json!({
                            "action": "post",
                            "platform": "instagram",
                            "content": post_content,
                            "ig_user_id": ig_user_id,
                            "ig_access_token": token
                        });
                        let _ = tool
                            .execute(&serde_json::to_string(&payload).unwrap_or_default())
                            .await;
                    }
                }
            }
            "twitter" | "x" => {
                if let Some(inst) = instances.iter().find(|i| i["channel_type"] == "twitter") {
                    let api_params = inst["config"]["api_params"].as_str().unwrap_or_default();
                    let parts: Vec<&str> = api_params.split(',').collect();
                    if parts.len() >= 4 {
                        let tool = SocialPostTool::new();
                        let payload = serde_json::json!({
                            "action": "post",
                            "platform": "twitter",
                            "content": post_content,
                            "x_api_key": api_params
                        });
                        let arg_str = serde_json::to_string(&payload).unwrap_or_default();
                        let _ = tool.execute(&arg_str).await;
                    }
                }
            }
            _ => {}
        }
    }
}
