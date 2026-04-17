//! Integration tests for the BizClaw Gateway API routes.
//!
//! Tests the full HTTP layer: request → handler → response.
//! Uses axum::test helpers for in-process integration testing.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

/// Build a minimal test router with only the routes we want to test.
/// This avoids needing a full AppState with real DB/agent/etc.
fn test_app() -> Router {
    use bizclaw_gateway::routes;
    use bizclaw_gateway::server::AppState;
    use std::sync::{Arc, Mutex};

    let (activity_tx, _rx) = tokio::sync::broadcast::channel(16);

    let state = Arc::new(AppState {
        gateway_config: bizclaw_core::config::GatewayConfig::default(),
        full_config: Arc::new(Mutex::new(bizclaw_core::config::BizClawConfig::default())),
        config_path: std::path::PathBuf::from("/tmp/bizclaw-test-gateway.toml"),
        start_time: std::time::Instant::now(),
        jwt_secret: String::new(),
        auth_failures: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        agent: Arc::new(tokio::sync::Mutex::new(None)),
        orchestrator: Arc::new(tokio::sync::Mutex::new(
            bizclaw_agent::orchestrator::Orchestrator::new(),
        )),
        scheduler: Arc::new(tokio::sync::Mutex::new(
            bizclaw_scheduler::SchedulerEngine::new(
                &std::env::temp_dir().join("bizclaw-test-integration"),
            ),
        )),
        knowledge: Arc::new(tokio::sync::Mutex::new(None)),
        crm: Arc::new(bizclaw_crm::CRMManager::new()),
        telegram_bots: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        db: Arc::new(
            bizclaw_gateway::db::GatewayDb::open(std::path::Path::new(":memory:")).unwrap(),
        ),
        orch_store: Arc::new(bizclaw_db::SqliteStore::in_memory().unwrap()) as _,
        traces: Arc::new(Mutex::new(Vec::new())),
        activity_tx,
        activity_log: Arc::new(Mutex::new(Vec::new())),
        rate_limiter: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        paused_threads: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
    });

    Router::new()
        .route("/health", axum::routing::get(routes::health_check))
        .route(
            "/api/v1/system/info",
            axum::routing::get(routes::system_info),
        )
        .route(
            "/api/v1/system/health",
            axum::routing::get(routes::system_health_check),
        )
        .route("/api/v1/config", axum::routing::get(routes::get_config))
        .route(
            "/api/v1/config/full",
            axum::routing::get(routes::get_full_config),
        )
        .route("/api/v1/config", axum::routing::put(routes::update_config))
        .route("/api/v1/agents", axum::routing::get(routes::list_agents))
        .route("/api/v1/agents", axum::routing::post(routes::create_agent))
        .route(
            "/api/v1/agents/{name}",
            axum::routing::delete(routes::delete_agent),
        )
        .route(
            "/api/v1/providers",
            axum::routing::get(routes::list_providers),
        )
        .route(
            "/api/v1/channels",
            axum::routing::get(routes::list_channels),
        )
        .route(
            "/api/v1/channel-instances",
            axum::routing::get(routes::list_channel_instances),
        )
        .route(
            "/api/v1/gallery/skills",
            axum::routing::get(routes::gallery_list),
        )
        .with_state(state)
}

// ── Health & System ─────────────────────────────────

#[tokio::test]
async fn test_health_endpoint() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["service"], "bizclaw-gateway");
}

#[tokio::test]
async fn test_system_info_endpoint() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/system/info")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["version"].is_string());
    assert!(json["uptime_secs"].is_number());
    assert!(json["platform"].is_string());
}

// ── Config ──────────────────────────────────────────

#[tokio::test]
async fn test_get_config_endpoint() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["default_provider"].is_string());
    assert!(json["channels"].is_object());
    assert!(json["memory"].is_object());
}

#[tokio::test]
async fn test_get_full_config_endpoint() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/config/full")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["ok"].as_bool().unwrap());
    assert!(json["toml"].is_string());
}

// ── Agents ──────────────────────────────────────────

#[tokio::test]
async fn test_agent_crud_flow() {
    let app = test_app();

    // 1. List agents — should be empty
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 0);

    // 2. Create an agent
    let create_body = json!({
        "name": "test-bot",
        "role": "assistant",
        "description": "Integration test agent"
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&create_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["ok"].as_bool().unwrap());
    assert_eq!(json["name"], "test-bot");
    assert_eq!(json["total_agents"], 1);

    // 3. List agents — should have 1
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["total"], 1);

    // 4. Delete agent
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/agents/test-bot")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["ok"].as_bool().unwrap());
}

// ── Providers & Channels ────────────────────────────

#[tokio::test]
async fn test_list_providers_endpoint() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["providers"].is_array());
    assert!(json["providers"].as_array().unwrap().len() >= 5);
}

#[tokio::test]
async fn test_list_channels_endpoint() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/channels")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["channels"].is_array());
    let channels = json["channels"].as_array().unwrap();
    assert!(channels.len() >= 3);
    // Verify structure
    let cli = channels.iter().find(|c| c["name"] == "cli").unwrap();
    assert_eq!(cli["status"], "active");
    assert_eq!(cli["configured"], true);
}

// ── Gallery ─────────────────────────────────────────

#[tokio::test]
async fn test_gallery_list_endpoint() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/gallery/skills")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["ok"].as_bool().unwrap());
    assert!(json["skills"].is_array());
    assert!(json["total"].is_number());
}
