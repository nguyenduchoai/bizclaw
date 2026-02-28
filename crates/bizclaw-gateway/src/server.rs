//! HTTP server implementation using Axum.


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
use axum::extract::DefaultBodyLimit;
use bizclaw_db::DataStore;

/// Shared state for the gateway server.
#[derive(Clone)]
pub struct AppState {
    pub gateway_config: GatewayConfig,
    pub full_config: Arc<Mutex<BizClawConfig>>,
    pub config_path: PathBuf,
    pub start_time: std::time::Instant,
    pub pairing_code: Arc<Mutex<String>>,
    /// Brute-force protection — (failed_count, last_failed_at)
    pub auth_failures: Arc<tokio::sync::Mutex<(u32, std::time::Instant)>>,
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
    /// Orchestration DataStore — delegations, teams, handoffs, traces.
    pub orch_store: Arc<dyn bizclaw_db::DataStore>,
    /// LLM call traces — records every provider call for cost/latency monitoring.
    pub traces: Arc<Mutex<Vec<super::openai_compat::LlmTrace>>>,
    /// Activity event broadcaster — sends real-time events to all connected dashboards.
    pub activity_tx: tokio::sync::broadcast::Sender<super::openai_compat::ActivityEvent>,
    /// Activity log — keeps recent events for REST polling.
    pub activity_log: Arc<Mutex<Vec<super::openai_compat::ActivityEvent>>>,
}

/// State for an active Telegram bot connected to an agent.
#[derive(Clone)]
pub struct TelegramBotState {
    pub bot_token: String,
    pub bot_username: String,
    pub abort_handle: Arc<tokio::sync::Notify>,
}

/// Serve the NEW Preact-based dashboard (no-cache to prevent stale JS after deploys).
async fn dashboard_page() -> axum::response::Response {
    axum::response::Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store, no-cache, must-revalidate")
        .header("Pragma", "no-cache")
        .body(axum::body::Body::from(super::dashboard::dashboard_v2_html()))
        .unwrap()
}

/// Serve the LEGACY monolithic dashboard at /legacy.
async fn legacy_dashboard_page() -> axum::response::Response {
    axum::response::Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store, no-cache, must-revalidate")
        .header("Pragma", "no-cache")
        .body(axum::body::Body::from(super::dashboard::dashboard_html()))
        .unwrap()
}

/// Serve embedded dashboard static files (/static/dashboard/*).
async fn dashboard_static(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> axum::response::Response {
    let full_path = format!("/static/dashboard/{}", path);
    let files = super::dashboard::dashboard_static_files();
    if let Some((content, content_type)) = files.get(full_path.as_str()) {
        axum::response::Response::builder()
            .header("Content-Type", *content_type)
            .header("Cache-Control", "public, max-age=3600")
            .body(axum::body::Body::from(*content))
            .unwrap()
    } else {
        axum::response::Response::builder()
            .status(axum::http::StatusCode::NOT_FOUND)
            .body(axum::body::Body::from("Not Found"))
            .unwrap()
    }
}

/// Pairing code auth middleware — validates X-Pairing-Code header or ?code= query.
async fn require_pairing(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    // If no pairing code configured, allow all
    let expected = state.pairing_code.lock().unwrap().clone();
    if expected.is_empty() {
        return next.run(req).await;
    }

    // Brute-force protection: lock out after 5 failed attempts for 60s
    {
        let failures = state.auth_failures.lock().await;
        if failures.0 >= 5 && failures.1.elapsed().as_secs() < 60 {
            tracing::warn!("[security] Auth locked out — {} failed attempts", failures.0);
            return axum::response::Response::builder()
                .status(axum::http::StatusCode::TOO_MANY_REQUESTS)
                .header("Content-Type", "application/json")
                .header("Retry-After", "60")
                .body(axum::body::Body::from(
                    serde_json::json!({"ok": false, "error": "Too many failed attempts. Try again in 60 seconds."}).to_string()
                ))
                .unwrap();
        }
    }

    // Check header first
    let from_header = req
        .headers()
        .get("X-Pairing-Code")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if constant_time_eq(from_header, &expected) {
        // Reset failures on success
        let mut failures = state.auth_failures.lock().await;
        *failures = (0, std::time::Instant::now());
        return next.run(req).await;
    }

    // Check query param ?code=
    if let Some(query) = req.uri().query() {
        for pair in query.split('&') {
            if let Some(code) = pair.strip_prefix("code=")
                && constant_time_eq(code, &expected) {
                    return next.run(req).await;
                }
        }
    }

    // Track failed attempt
    {
        let mut failures = state.auth_failures.lock().await;
        failures.0 += 1;
        failures.1 = std::time::Instant::now();
        tracing::warn!("[security] Failed auth attempt #{} from request", failures.0);
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
    let expected = state.pairing_code.lock().unwrap().clone();
    if expected.is_empty() || constant_time_eq(code, &expected) {
        Json(serde_json::json!({"ok": true}))
    } else {
        Json(serde_json::json!({"ok": false, "error": "Invalid pairing code"}))
    }
}

/// Constant-time string comparison to prevent timing attacks (M3).
/// Does NOT short-circuit on length mismatch to avoid leaking length info.
fn constant_time_eq(a: &str, b: &str) -> bool {
    let len_eq = a.len() == b.len();
    // Always iterate over the longer string to avoid timing differences
    let max_len = a.len().max(b.len());
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let mut diff = 0u8;
    for i in 0..max_len {
        let x = if i < a_bytes.len() { a_bytes[i] } else { 0 };
        let y = if i < b_bytes.len() { b_bytes[i] } else { 0 };
        diff |= x ^ y;
    }
    len_eq && diff == 0
}

/// Security headers middleware — CSP, HSTS, XSS protection.
async fn security_headers(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    headers.insert("Referrer-Policy", "strict-origin-when-cross-origin".parse().unwrap());
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    // HSTS — tell browsers to always use HTTPS (1 year)
    headers.insert("Strict-Transport-Security", "max-age=31536000; includeSubDomains".parse().unwrap());
    // CSP — restrict script/style sources (includes esm.sh for Preact CDN)
    headers.insert("Content-Security-Policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://esm.sh https://cdn.jsdelivr.net https://cdnjs.cloudflare.com https://fonts.googleapis.com; style-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net https://cdnjs.cloudflare.com https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com https://cdnjs.cloudflare.com; img-src 'self' data: https:; connect-src 'self' ws: wss: https://esm.sh; frame-ancestors 'none'"
        .parse().unwrap());
    response
}

/// Build the Axum router with all routes.
pub fn build_router(state: AppState) -> Router {
    build_router_from_arc(Arc::new(state))
}

pub fn build_router_from_arc(shared: Arc<AppState>) -> Router {

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
        // Multi-instance channel management
        .route("/api/v1/channel-instances", get(super::routes::list_channel_instances))
        .route("/api/v1/channel-instances", post(super::routes::save_channel_instance))
        .route("/api/v1/channel-instances/{id}", axum::routing::delete(super::routes::delete_channel_instance))
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
            "/api/v1/scheduler/tasks/{id}/toggle",
            post(super::routes::scheduler_toggle_task),
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
        // Orchestration API
        .route("/api/v1/orchestration/delegate", post(super::routes::orch_delegate))
        .route("/api/v1/orchestration/handoff", post(super::routes::orch_handoff))
        .route("/api/v1/orchestration/handoff/{session_id}", axum::routing::delete(super::routes::orch_clear_handoff))
        .route("/api/v1/orchestration/evaluate", post(super::routes::orch_evaluate))
        .route("/api/v1/orchestration/links", get(super::routes::orch_list_links).post(super::routes::orch_create_link))
        .route("/api/v1/orchestration/links/{id}", axum::routing::delete(super::routes::orch_delete_link))
        .route("/api/v1/orchestration/delegations", get(super::routes::orch_list_delegations))
        .route("/api/v1/orchestration/traces", get(super::routes::orch_list_traces))
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
        // LLM Traces & Cost API
        .route("/api/v1/traces", get(super::openai_compat::list_traces))
        .route("/api/v1/traces/cost", get(super::openai_compat::cost_breakdown))
        .route("/api/v1/activity", get(super::openai_compat::list_activity))
        // MCP Servers API (stub — returns configured MCP servers)
        .route("/api/v1/mcp/servers", get(super::routes::mcp_list_servers))
        .route("/ws", get(super::ws::ws_handler))
        .route_layer(axum::middleware::from_fn_with_state(
            shared.clone(),
            require_pairing,
        ));

    // Public routes — no auth
    let public = Router::new()
        .route("/", get(dashboard_page))
        .route("/legacy", get(legacy_dashboard_page))
        .route("/static/dashboard/*path", get(dashboard_static))
        .route("/health", get(super::routes::health_check))
        .route("/api/v1/verify-pairing", post(verify_pairing))
        // WhatsApp webhook — must be public for Meta verification
        .route(
            "/api/v1/webhook/whatsapp",
            get(super::routes::whatsapp_webhook_verify).post(super::routes::whatsapp_webhook),
        )
        // Webhook inbound — public, auth via HMAC signature in header
        .route("/api/v1/webhook/inbound", post(super::routes::webhook_inbound))
        // OpenAI-Compatible API — public with own auth (Bearer token)
        .route("/v1/chat/completions", post(super::openai_compat::chat_completions))
        .route("/v1/models", get(super::openai_compat::list_models));

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
        // Security headers
        .layer(axum::middleware::from_fn(security_headers))
        // H1 FIX: Limit request body size (5MB — allows file uploads for knowledge base)
        .layer(DefaultBodyLimit::max(5_242_880))
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

    // Create the Agent engine (sync — no MCP to avoid startup hang)
    let agent: Option<bizclaw_agent::Agent> =
        match bizclaw_agent::Agent::new(full_config.clone()) {
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

    // Initialize Orchestration DataStore (SQLite — same directory as gateway.db)
    let orch_db_path = config_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("orchestration.db");
    let orch_store: Arc<dyn bizclaw_db::DataStore> = match bizclaw_db::SqliteStore::open(&orch_db_path) {
        Ok(store) => {
            let store = Arc::new(store);
            // Run migrations
            if let Err(e) = store.migrate().await {
                tracing::error!("❌ Orchestration DB migration failed: {e}");
            } else {
                tracing::info!("🔗 Orchestration DB initialized: {}", orch_db_path.display());
            }
            store
        }
        Err(e) => {
            tracing::warn!("⚠️ Orchestration DB failed, using in-memory: {e}");
            let store = Arc::new(bizclaw_db::SqliteStore::in_memory().unwrap());
            let _ = store.migrate().await;
            store
        }
    };

    // Initialize Multi-Agent Orchestrator with DataStore
    let mut orchestrator = bizclaw_agent::orchestrator::Orchestrator::with_store(orch_store.clone());

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
                // CRITICAL: sync llm.provider — create_provider() reads this FIRST
                agent_cfg.llm.provider = agent_rec.provider.clone();
            }
            if !agent_rec.model.is_empty() {
                agent_cfg.default_model = agent_rec.model.clone();
                agent_cfg.llm.model = agent_rec.model.clone();
            }
            if !agent_rec.system_prompt.is_empty() {
                agent_cfg.identity.system_prompt = agent_rec.system_prompt.clone();
            }
            agent_cfg.identity.name = agent_rec.name.clone();

            // Inject per-provider API key and base_url from DB
            // This enables agents to use different providers (e.g. Ollama, DeepSeek)
            // Must set BOTH legacy fields AND llm.* fields
            let provider_name = &agent_cfg.default_provider;
            if let Ok(db_provider) = gateway_db.get_provider(provider_name) {
                if !db_provider.api_key.is_empty() {
                    agent_cfg.api_key = db_provider.api_key.clone();
                    agent_cfg.llm.api_key = db_provider.api_key;
                }
                if db_provider.provider_type == "local" || db_provider.provider_type == "proxy" {
                    if !db_provider.base_url.is_empty() {
                        agent_cfg.api_base_url = db_provider.base_url.clone();
                        agent_cfg.llm.endpoint = db_provider.base_url;
                    }
                } else if !db_provider.base_url.is_empty() && agent_cfg.api_base_url.is_empty() {
                    agent_cfg.api_base_url = db_provider.base_url.clone();
                    agent_cfg.llm.endpoint = db_provider.base_url;
                }
            }

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

    let (activity_tx, _rx) = tokio::sync::broadcast::channel::<super::openai_compat::ActivityEvent>(256);

    let state = AppState {
        gateway_config: config.clone(),
        full_config: Arc::new(Mutex::new(full_config)),
        config_path: config_path.clone(),
        start_time: std::time::Instant::now(),
        pairing_code: Arc::new(Mutex::new(if config.require_pairing {
            std::env::var("BIZCLAW_PAIRING_CODE").ok().or_else(|| {
                config_path.parent().and_then(|d| {
                    let pc = d.join(".pairing_code");
                    std::fs::read_to_string(pc)
                        .ok()
                        .map(|s| s.trim().to_string())
                })
            }).unwrap_or_default()
        } else {
            String::new()
        })),
        auth_failures: Arc::new(tokio::sync::Mutex::new((0, std::time::Instant::now()))),
        agent: Arc::new(tokio::sync::Mutex::new(agent)),
        orchestrator: orchestrator_arc.clone(),
        scheduler,
        knowledge: Arc::new(tokio::sync::Mutex::new(knowledge)),
        telegram_bots: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        db: gateway_db,
        orch_store,
        traces: Arc::new(Mutex::new(Vec::new())),
        activity_tx: activity_tx.clone(),
        activity_log: Arc::new(Mutex::new(Vec::new())),
    };

    let state_arc = Arc::new(state);
    let app = build_router_from_arc(state_arc.clone());

    // Auto-connect saved channel instances (Telegram bots, etc.)
    let state_for_channels = state_arc.clone();
    tokio::spawn(async move {
        // Small delay to let server bind first
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        super::routes::auto_connect_channels(state_for_channels).await;
    });

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🌐 Gateway server listening on http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Listen for shutdown signals (Ctrl+C / SIGTERM) for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("🛑 Shutdown signal received — draining connections...");
}

