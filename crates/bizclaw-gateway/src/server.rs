//! HTTP server implementation using Axum.

use axum::response::Html;
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post, put},
};
use bizclaw_core::config::{BizClawConfig, GatewayConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Shared state for the gateway server.
#[derive(Clone)]
pub struct AppState {
    pub gateway_config: GatewayConfig,
    pub full_config: Arc<Mutex<BizClawConfig>>,
    pub config_path: PathBuf,
    pub start_time: std::time::Instant,
    pub pairing_code: Option<String>,
    /// The Agent engine — handles chat with tools, memory, and all providers.
    pub agent: Arc<tokio::sync::Mutex<Option<bizclaw_agent::Agent>>>,
    /// Multi-Agent Orchestrator — manages multiple named agents.
    pub orchestrator: Arc<tokio::sync::Mutex<bizclaw_agent::orchestrator::Orchestrator>>,
    /// Scheduler engine — manages scheduled tasks and notifications.
    pub scheduler: Arc<tokio::sync::Mutex<bizclaw_scheduler::SchedulerEngine>>,
    /// Knowledge base — personal RAG with FTS5 search.
    pub knowledge: Arc<tokio::sync::Mutex<Option<bizclaw_knowledge::KnowledgeStore>>>,
    /// Active Telegram bot polling tasks — maps agent_name → abort handle.
    pub telegram_bots: Arc<tokio::sync::Mutex<HashMap<String, TelegramBotState>>>,
    /// Per-tenant SQLite database for persistent CRUD (providers, agents, channels, settings).
    pub db: Arc<super::db::GatewayDb>,
}

/// State for an active Telegram bot connected to an agent.
#[derive(Clone)]
pub struct TelegramBotState {
    pub bot_token: String,
    pub bot_username: String,
    pub abort_handle: Arc<tokio::sync::Notify>,
}

/// Serve the dashboard HTML page.
async fn dashboard_page() -> Html<&'static str> {
    Html(super::dashboard::dashboard_html())
}

/// Pairing code auth middleware — validates X-Pairing-Code header or ?code= query.
async fn require_pairing(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    // If no pairing code configured, allow all
    let Some(expected) = &state.pairing_code else {
        return next.run(req).await;
    };

    // Check header first
    let from_header = req
        .headers()
        .get("X-Pairing-Code")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if from_header == expected {
        return next.run(req).await;
    }

    // Check query param ?code=
    if let Some(query) = req.uri().query() {
        for pair in query.split('&') {
            if let Some(code) = pair.strip_prefix("code=") {
                if code == expected {
                    return next.run(req).await;
                }
            }
        }
    }

    axum::response::Response::builder()
        .status(axum::http::StatusCode::UNAUTHORIZED)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::json!({"ok": false, "error": "Unauthorized — invalid or missing pairing code"}).to_string()
        ))
        .unwrap()
}

/// Verify pairing code endpoint (public).
async fn verify_pairing(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let code = body["code"].as_str().unwrap_or("");
    match &state.pairing_code {
        Some(expected) if code == expected => Json(serde_json::json!({"ok": true})),
        Some(_) => Json(serde_json::json!({"ok": false, "error": "Invalid pairing code"})),
        None => Json(serde_json::json!({"ok": true})), // no code required
    }
}

/// Build the Axum router with all routes.
pub fn build_router(state: AppState) -> Router {
    let shared = Arc::new(state);

    // Protected routes — require valid pairing code
    let protected = Router::new()
        .route("/api/v1/info", get(super::routes::system_info))
        .route("/api/v1/config", get(super::routes::get_config))
        .route("/api/v1/config/update", post(super::routes::update_config))
        .route("/api/v1/config/full", get(super::routes::get_full_config))
        .route("/api/v1/providers", get(super::routes::list_providers))
        .route("/api/v1/providers", post(super::routes::create_provider))
        .route("/api/v1/providers/{name}", put(super::routes::update_provider))
        .route("/api/v1/providers/{name}", axum::routing::delete(super::routes::delete_provider))
        .route("/api/v1/providers/{name}/models", get(super::routes::fetch_provider_models))
        .route("/api/v1/channels", get(super::routes::list_channels))
        .route(
            "/api/v1/channels/update",
            post(super::routes::update_channel),
        )
        .route("/api/v1/ollama/models", get(super::routes::ollama_models))
        .route(
            "/api/v1/brain/models",
            get(super::routes::brain_scan_models),
        )
        .route("/api/v1/zalo/qr", post(super::routes::zalo_qr_code))
        // Scheduler API
        .route(
            "/api/v1/scheduler/tasks",
            get(super::routes::scheduler_list_tasks),
        )
        .route(
            "/api/v1/scheduler/tasks",
            post(super::routes::scheduler_add_task),
        )
        .route(
            "/api/v1/scheduler/tasks/{id}",
            axum::routing::delete(super::routes::scheduler_remove_task),
        )
        .route(
            "/api/v1/scheduler/notifications",
            get(super::routes::scheduler_notifications),
        )
        // Knowledge Base API
        .route(
            "/api/v1/knowledge/search",
            post(super::routes::knowledge_search),
        )
        .route(
            "/api/v1/knowledge/documents",
            get(super::routes::knowledge_list_docs),
        )
        .route(
            "/api/v1/knowledge/documents",
            post(super::routes::knowledge_add_doc),
        )
        .route(
            "/api/v1/knowledge/documents/{id}",
            axum::routing::delete(super::routes::knowledge_remove_doc),
        )
        // Multi-Agent Orchestrator API
        .route("/api/v1/agents", get(super::routes::list_agents))
        .route("/api/v1/agents", post(super::routes::create_agent))
        .route(
            "/api/v1/agents/{name}",
            axum::routing::delete(super::routes::delete_agent),
        )
        .route("/api/v1/agents/{name}", put(super::routes::update_agent))
        .route(
            "/api/v1/agents/{name}/chat",
            post(super::routes::agent_chat),
        )
        .route(
            "/api/v1/agents/broadcast",
            post(super::routes::agent_broadcast),
        )
        // Gallery API
        .route("/api/v1/gallery", get(super::routes::gallery_list))
        .route("/api/v1/gallery", post(super::routes::gallery_create))
        .route(
            "/api/v1/gallery/{id}",
            axum::routing::delete(super::routes::gallery_delete),
        )
        .route(
            "/api/v1/gallery/{id}/md",
            post(super::routes::gallery_upload_md),
        )
        .route(
            "/api/v1/gallery/{id}/md",
            get(super::routes::gallery_get_md),
        )
        // Agent-Channel Bindings
        .route(
            "/api/v1/agents/{name}/channels",
            post(super::routes::agent_bind_channels),
        )
        .route(
            "/api/v1/agents/channels",
            get(super::routes::agent_channel_bindings),
        )
        // Telegram Bot ↔ Agent API
        .route(
            "/api/v1/agents/{name}/telegram",
            post(super::routes::connect_telegram),
        )
        .route(
            "/api/v1/agents/{name}/telegram",
            axum::routing::delete(super::routes::disconnect_telegram),
        )
        .route(
            "/api/v1/agents/{name}/telegram",
            get(super::routes::telegram_status),
        )
        // Brain Workspace API
        .route("/api/v1/brain/files", get(super::routes::brain_list_files))
        .route(
            "/api/v1/brain/files/{filename}",
            get(super::routes::brain_read_file),
        )
        .route(
            "/api/v1/brain/files/{filename}",
            axum::routing::put(super::routes::brain_write_file),
        )
        .route(
            "/api/v1/brain/files/{filename}",
            axum::routing::delete(super::routes::brain_delete_file),
        )
        // Brain Personalization
        .route(
            "/api/v1/brain/personalize",
            post(super::routes::brain_personalize),
        )
        // Health Check
        .route("/api/v1/health", get(super::routes::system_health_check))
        .route("/ws", get(super::ws::ws_handler))
        .route_layer(axum::middleware::from_fn_with_state(
            shared.clone(),
            require_pairing,
        ));

    // Public routes — no auth
    let public = Router::new()
        .route("/", get(dashboard_page))
        .route("/health", get(super::routes::health_check))
        .route("/api/v1/verify-pairing", post(verify_pairing))
        // WhatsApp webhook — must be public for Meta verification
        .route(
            "/api/v1/webhook/whatsapp",
            get(super::routes::whatsapp_webhook_verify).post(super::routes::whatsapp_webhook),
        )
        // Generic webhook inbound — public, auth via HMAC signature
        .route(
            "/api/v1/webhook/inbound",
            post(super::routes::webhook_inbound),
        );

    // SPA fallback — serve dashboard HTML for all frontend routes
    // so that /dashboard, /chat, /settings etc. all work with path-based routing
    let spa_fallback = Router::new().fallback(get(dashboard_page));

    protected
        .merge(public)
        .merge(spa_fallback)
        .layer({
            let cors = CorsLayer::new()
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers(Any)
                .max_age(std::time::Duration::from_secs(3600));

            // Restrict CORS origins in production via env var
            // Example: BIZCLAW_CORS_ORIGINS=https://bizclaw.vn,https://sales.bizclaw.vn
            if let Ok(origins_str) = std::env::var("BIZCLAW_CORS_ORIGINS") {
                let origins: Vec<_> = origins_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<axum::http::HeaderValue>().ok())
                    .collect();
                cors.allow_origin(origins)
            } else {
                // Development fallback — allow all origins
                cors.allow_origin(Any)
            }
        })
        .layer(TraceLayer::new_for_http())
        .with_state(shared)
}

/// Start the HTTP server.
pub async fn start(config: &GatewayConfig) -> anyhow::Result<()> {
    // Load full config for settings UI
    let config_path = std::env::var("BIZCLAW_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| BizClawConfig::default_path());
    let full_config = if config_path.exists() {
        BizClawConfig::load_from(&config_path).unwrap_or_default()
    } else {
        BizClawConfig::default()
    };

    // Try to create the Agent engine (with MCP support)
    let agent: Option<bizclaw_agent::Agent> =
        match bizclaw_agent::Agent::new_with_mcp(full_config.clone()).await {
            Ok(a) => {
                let tool_count = a.tool_count();
                tracing::info!(
                    "✅ Agent engine initialized (provider={}, tools={})",
                    a.provider_name(),
                    tool_count
                );
                Some(a)
            }
            Err(e) => {
                tracing::warn!(
                    "⚠️ Agent engine not available: {e} — falling back to direct provider calls"
                );
                None
            }
        };

    // Initialize Scheduler engine
    let sched_dir = config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("scheduler");
    let scheduler = bizclaw_scheduler::SchedulerEngine::new(&sched_dir);
    let task_count = scheduler.task_count();
    if task_count > 0 {
        tracing::info!("⏰ Scheduler loaded: {} task(s)", task_count);
    }
    let scheduler = Arc::new(tokio::sync::Mutex::new(scheduler));

    // Initialize Knowledge Base
    let kb_path = config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("knowledge.db");
    let knowledge = match bizclaw_knowledge::KnowledgeStore::open(&kb_path) {
        Ok(kb) => {
            let (docs, chunks) = kb.stats();
            if docs > 0 {
                tracing::info!("📚 Knowledge base: {} documents, {} chunks", docs, chunks);
            }
            Some(kb)
        }
        Err(e) => {
            tracing::warn!("⚠️ Knowledge base not available: {e}");
            None
        }
    };

    // Initialize Gateway DB (per-tenant SQLite)
    let db_path = config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("gateway.db");
    let gateway_db = match super::db::GatewayDb::open(&db_path) {
        Ok(db) => {
            tracing::info!("💾 Gateway DB initialized: {}", db_path.display());
            db
        }
        Err(e) => {
            tracing::error!("❌ Failed to open gateway DB: {e}");
            // Create in-memory fallback
            super::db::GatewayDb::open(std::path::Path::new(":memory:")).unwrap()
        }
    };
    let gateway_db = Arc::new(gateway_db);

    // Initialize Multi-Agent Orchestrator
    let mut orchestrator = bizclaw_agent::orchestrator::Orchestrator::new();

    // Migrate from legacy agents.json if it exists AND DB is empty
    let agents_path = config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("agents.json");
    let db_agents = gateway_db.list_agents().unwrap_or_default();
    if db_agents.is_empty() && agents_path.exists() {
        // First launch with DB — migrate from flat file
        let saved_agents =
            bizclaw_agent::orchestrator::Orchestrator::load_agents_metadata(&agents_path);
        if !saved_agents.is_empty() {
            match gateway_db.migrate_from_agents_json(&saved_agents) {
                Ok(count) => tracing::info!("📦 Migrated {} agent(s) from agents.json → gateway.db", count),
                Err(e) => tracing::warn!("⚠️ Migration from agents.json failed: {e}"),
            }
        }
    }

    // Restore agents from DB (using sync Agent::new — no MCP to avoid startup hang)
    let db_agents = gateway_db.list_agents().unwrap_or_default();
    if !db_agents.is_empty() {
        tracing::info!(
            "🔄 Restoring {} agent(s) from gateway.db...",
            db_agents.len()
        );
        for agent_rec in &db_agents {
            let mut agent_cfg = full_config.clone();
            if !agent_rec.provider.is_empty() {
                agent_cfg.default_provider = agent_rec.provider.clone();
            }
            if !agent_rec.model.is_empty() {
                agent_cfg.default_model = agent_rec.model.clone();
            }
            if !agent_rec.system_prompt.is_empty() {
                agent_cfg.identity.system_prompt = agent_rec.system_prompt.clone();
            }
            agent_cfg.identity.name = agent_rec.name.clone();
            // Use sync Agent::new() for fast startup — MCP tools loaded lazily on first chat
            match bizclaw_agent::Agent::new(agent_cfg) {
                Ok(agent) => {
                    orchestrator.add_agent(&agent_rec.name, &agent_rec.role, &agent_rec.description, agent);
                    tracing::info!("  ✅ Agent '{}' restored ({})", agent_rec.name, agent_rec.role);
                }
                Err(e) => {
                    tracing::warn!("  ⚠️ Failed to restore agent '{}': {}", agent_rec.name, e);
                }
            }
        }
    }
    tracing::info!(
        "🤖 Multi-Agent Orchestrator initialized ({} agents)",
        orchestrator.agent_count()
    );

    // Wrap orchestrator in Arc for shared access
    let orchestrator_arc = Arc::new(tokio::sync::Mutex::new(orchestrator));

    // Spawn scheduler background loop with Agent integration (check every 30 seconds)
    let sched_clone = scheduler.clone();
    let orch_for_sched = orchestrator_arc.clone();
    tokio::spawn(async move {
        bizclaw_scheduler::engine::spawn_scheduler_with_agent(
            sched_clone,
            move |prompt: String| {
                let orch = orch_for_sched.clone();
                async move {
                    let mut o = orch.lock().await;
                    o.send(&prompt).await.map_err(|e| e.to_string())
                }
            },
            30,
        )
        .await;
    });

    let state = AppState {
        gateway_config: config.clone(),
        full_config: Arc::new(Mutex::new(full_config)),
        config_path: config_path.clone(),
        start_time: std::time::Instant::now(),
        pairing_code: if config.require_pairing {
            let code = std::env::var("BIZCLAW_PAIRING_CODE").ok().or_else(|| {
                config_path.parent().and_then(|d| {
                    let pc = d.join(".pairing_code");
                    std::fs::read_to_string(pc)
                        .ok()
                        .map(|s| s.trim().to_string())
                })
            });
            code
        } else {
            None
        },
        agent: Arc::new(tokio::sync::Mutex::new(agent)),
        orchestrator: orchestrator_arc.clone(),
        scheduler,
        knowledge: Arc::new(tokio::sync::Mutex::new(knowledge)),
        telegram_bots: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        db: gateway_db,
    };

    let app = build_router(state);
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🌐 Gateway server listening on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

