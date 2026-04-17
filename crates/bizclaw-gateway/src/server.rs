//! HTTP server implementation using Axum.

use axum::extract::DefaultBodyLimit;
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post, put},
};
use bizclaw_core::config::{BizClawConfig, GatewayConfig};
use bizclaw_db::DataStore;
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
    // Legacy pairing_code removed — SaaS uses JWT from Platform login
    /// JWT secret — shared with Platform for token validation.
    /// When set, Gateway accepts `Authorization: Bearer <JWT>` from Platform login.
    pub jwt_secret: String,
    /// Brute-force protection — per-IP tracking: IP → (failed_count, last_failed_at)
    pub auth_failures:
        Arc<tokio::sync::Mutex<std::collections::HashMap<String, (u32, std::time::Instant)>>>,
    /// The Agent engine — handles chat with tools, memory, and all providers.
    pub agent: Arc<tokio::sync::Mutex<Option<bizclaw_agent::Agent>>>,
    /// Multi-Agent Orchestrator — manages multiple named agents.
    pub orchestrator: Arc<tokio::sync::Mutex<bizclaw_agent::orchestrator::Orchestrator>>,
    /// Scheduler engine — manages scheduled tasks and notifications.
    pub scheduler: Arc<tokio::sync::Mutex<bizclaw_scheduler::SchedulerEngine>>,
    /// Knowledge base — personal RAG with FTS5 search.
    pub knowledge: Arc<tokio::sync::Mutex<Option<bizclaw_knowledge::KnowledgeStore>>>,
    /// CRM Manager — unified inbox + customer management.
    pub crm: Arc<bizclaw_crm::CRMManager>,
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
    /// Rate limiter — IP → (count, window_start) for public endpoints.
    pub rate_limiter:
        Arc<tokio::sync::Mutex<std::collections::HashMap<String, (u32, std::time::Instant)>>>,
    /// Handoff tracking — threads that humans have taken over (AI ignores them).
    pub paused_threads: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>,
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
        .expect("static response")
}

/// Serve the LEGACY monolithic dashboard at /legacy.
async fn legacy_dashboard_page() -> axum::response::Response {
    axum::response::Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store, no-cache, must-revalidate")
        .header("Pragma", "no-cache")
        .body(axum::body::Body::from(super::dashboard::dashboard_html()))
        .expect("static response")
}

/// Serve the public Hub page at /hub (no auth required).
async fn hub_page() -> axum::response::Response {
    axum::response::Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "public, max-age=300")
        .body(axum::body::Body::from(super::dashboard::hub_html()))
        .expect("static response")
}

/// Serve the public Landing page at /landing (sales + pricing, no auth).
async fn landing_page() -> axum::response::Response {
    axum::response::Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "public, max-age=300")
        .body(axum::body::Body::from(super::dashboard::landing_html()))
        .expect("static response")
}

/// Serve the NEW Horizontal Menu Dashboard at /new.
async fn dashboard_new_page() -> axum::response::Response {
    axum::response::Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store, no-cache, must-revalidate")
        .header("Pragma", "no-cache")
        .body(axum::body::Body::from(
            super::dashboard::dashboard_new_html(),
        ))
        .expect("static response")
}

/// Serve the Visual Workflow Builder at /workflow-builder.
async fn workflow_builder_page() -> axum::response::Response {
    axum::response::Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store, no-cache, must-revalidate")
        .header("Pragma", "no-cache")
        .body(axum::body::Body::from(
            super::dashboard::workflow_builder_html(),
        ))
        .expect("static response")
}

/// Serve embedded dashboard static files (/static/dashboard/*).
async fn dashboard_static(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> axum::response::Response {
    let full_path = format!("/static/dashboard/{}", path);
    let files = super::dashboard::dashboard_static_files();
    if let Some((content, content_type)) = files.get(full_path.as_str()) {
        // IMPORTANT: Use no-store to prevent stale JS/CSS after deploys.
        // Static files are embedded at compile time via include_str!(),
        // so the content changes with each build but the URL stays the same.
        axum::response::Response::builder()
            .header("Content-Type", *content_type)
            .header("Cache-Control", "no-store, no-cache, must-revalidate")
            .header("Pragma", "no-cache")
            .body(axum::body::Body::from(*content))
            .expect("static response")
    } else {
        axum::response::Response::builder()
            .status(axum::http::StatusCode::NOT_FOUND)
            .body(axum::body::Body::from("Not Found"))
            .expect("404 response")
    }
}

/// Auth middleware — JWT-only authentication with RBAC role injection.
/// SaaS mode: users login via Platform → get JWT → use JWT to access tenant gateway.
/// Checks: 1) Authorization: Bearer <JWT>  2) Cookie: bizclaw_token=<JWT>  3) ?token=<JWT>
/// On success, injects `AuthUser` into request extensions for downstream role checks.
async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    // Dev mode: if no JWT secret configured, allow all requests with admin role
    if state.jwt_secret.is_empty() {
        req.extensions_mut().insert(AuthUser {
            email: "dev@localhost".into(),
            role: Role::Admin,
            role_str: "admin".into(),
        });
        return next.run(req).await;
    }

    // Helper: validate JWT and inject AuthUser into request
    let inject_user = |req: &mut axum::http::Request<axum::body::Body>, claims: &JwtClaims| {
        let role = Role::from_str(&claims.role);
        req.extensions_mut().insert(AuthUser {
            email: claims.email.clone(),
            role,
            role_str: claims.role.clone(),
        });
    };

    // ── Check JWT token from multiple sources ──
    // Source 1: Authorization: Bearer <JWT>
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if let Some(token) = auth_header.strip_prefix("Bearer ")
        && let Ok(claims) = validate_jwt(token, &state.jwt_secret)
    {
        inject_user(&mut req, &claims);
        return next.run(req).await;
    }

    // Source 2: Cookie: bizclaw_token=<JWT>
    let cookie_header = req
        .headers()
        .get("Cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(token) = part.strip_prefix("bizclaw_token=")
            && let Ok(claims) = validate_jwt(token.trim(), &state.jwt_secret)
        {
            inject_user(&mut req, &claims);
            return next.run(req).await;
        }
    }

    // Source 3: Query param ?token=<JWT>
    let query_str = req.uri().query().unwrap_or("").to_string();
    for pair in query_str.split('&') {
        if let Some(token) = pair.strip_prefix("token=")
            && let Ok(claims) = validate_jwt(token, &state.jwt_secret)
        {
            inject_user(&mut req, &claims);
            return next.run(req).await;
        }
    }

    // ── Brute-force protection (per-IP) ──
    // Extract client IP from X-Forwarded-For (behind reverse proxy) or X-Real-IP
    let client_ip = req
        .headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            req.headers()
                .get("X-Real-IP")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    {
        let mut failures = state.auth_failures.lock().await;
        // Cleanup expired entries (> 120s old) to prevent memory leak
        if failures.len() > 100 {
            failures.retain(|_, (_, last)| last.elapsed().as_secs() < 120);
        }
        // Check if this IP is locked out
        if let Some((count, last)) = failures.get(&client_ip)
            && *count >= 5
            && last.elapsed().as_secs() < 60
        {
            tracing::warn!(
                "[security] Auth locked out for IP {} — {} failed attempts",
                client_ip,
                count
            );
            return axum::response::Response::builder()
                    .status(axum::http::StatusCode::TOO_MANY_REQUESTS)
                    .header("Content-Type", "application/json")
                    .header("Retry-After", "60")
                    .body(axum::body::Body::from(
                        serde_json::json!({"ok": false, "error": "Too many failed attempts. Try again in 60 seconds."}).to_string()
                    ))
                    .expect("429 response");
        }
    }

    // Track failed attempt (per-IP)
    {
        let mut failures = state.auth_failures.lock().await;
        let entry = failures
            .entry(client_ip.clone())
            .or_insert((0, std::time::Instant::now()));
        // Reset counter if previous attempts were > 60s ago
        if entry.1.elapsed().as_secs() >= 60 {
            entry.0 = 0;
        }
        entry.0 += 1;
        entry.1 = std::time::Instant::now();
        tracing::warn!(
            "[security] Failed auth attempt #{} from IP {}",
            entry.0,
            client_ip
        );
    }
    axum::response::Response::builder()
        .status(axum::http::StatusCode::UNAUTHORIZED)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::json!({"ok": false, "error": "Unauthorized — invalid or missing JWT token"}).to_string()
        ))
        .expect("401 response")
}

/// Rate-limiting middleware for public endpoints.
/// Allows 60 requests per minute per IP.
async fn rate_limit(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let ip = req
        .headers()
        .get("x-real-ip")
        .or_else(|| req.headers().get("x-forwarded-for"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .split(',')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string();

    {
        let mut limiter = state.rate_limiter.lock().await;
        let entry = limiter
            .entry(ip.clone())
            .or_insert_with(|| (0, std::time::Instant::now()));

        // Reset window after 60 seconds
        if entry.1.elapsed().as_secs() >= 60 {
            *entry = (0, std::time::Instant::now());
        }

        entry.0 += 1;

        if entry.0 > 60 {
            tracing::warn!("[rate-limit] IP {} exceeded 60 req/min", ip);
            return axum::response::Response::builder()
                .status(axum::http::StatusCode::TOO_MANY_REQUESTS)
                .header("Content-Type", "application/json")
                .header("Retry-After", "60")
                .body(axum::body::Body::from(
                    serde_json::json!({"ok": false, "error": "Rate limit exceeded. Max 60 requests per minute."}).to_string()
                ))
                .expect("429 response");
        }
    }

    next.run(req).await
}

/// Verify auth endpoint (public) — validates JWT token from Platform login.
async fn verify_auth(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if let Some(token) = body["token"].as_str()
        && !state.jwt_secret.is_empty()
        && let Ok(claims) = validate_jwt(token, &state.jwt_secret)
    {
        return Json(serde_json::json!({
            "ok": true,
            "auth": "jwt",
            "email": claims.email,
            "role": claims.role
        }));
    }
    // Dev mode fallback: no JWT secret configured
    if state.jwt_secret.is_empty() {
        return Json(serde_json::json!({"ok": true, "auth": "dev_mode"}));
    }
    Json(serde_json::json!({"ok": false, "error": "Invalid or expired token"}))
}

/// Validate JWT token using the shared secret with Platform.
fn validate_jwt(token: &str, secret: &str) -> std::result::Result<JwtClaims, String> {
    use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
    let validation = Validation::new(Algorithm::HS256);
    decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|e| format!("JWT validation failed: {e}"))
}

/// Public wrapper for JWT validation — used by openai_compat module.
pub fn validate_jwt_public(token: &str, secret: &str) -> std::result::Result<(), String> {
    validate_jwt(token, secret).map(|_| ())
}

/// JWT claims structure — mirrors Platform's auth::Claims.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct JwtClaims {
    sub: String,
    email: String,
    #[serde(default = "default_role")]
    role: String,
    #[serde(default)]
    tenant_id: Option<String>,
    exp: usize,
}

fn default_role() -> String {
    "admin".to_string() // backward compat: existing JWTs without role get admin
}

/// RBAC role hierarchy: admin > manager > user > viewer
/// Each higher role inherits permissions of lower roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Viewer = 0,
    User = 1,
    Manager = 2,
    Admin = 3,
}

impl Role {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "admin" | "owner" | "superadmin" => Role::Admin,
            "manager" | "editor" => Role::Manager,
            "user" | "agent" | "operator" => Role::User,
            "viewer" | "readonly" | "guest" => Role::Viewer,
            _ => Role::User, // default to User for unknown roles
        }
    }
}

/// Extension to store authenticated user info in request.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub email: String,
    pub role: Role,
    pub role_str: String,
}

/// RBAC middleware — require Admin role for sensitive operations.
/// Apply this to routes that manage system config, API keys, providers, plan limits.
async fn require_role_admin(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if let Some(user) = req.extensions().get::<AuthUser>() {
        if user.role >= Role::Admin {
            return next.run(req).await;
        }
        tracing::warn!(
            "[rbac] User '{}' (role={}) denied access to admin route",
            user.email,
            user.role_str
        );
        return axum::response::Response::builder()
            .status(axum::http::StatusCode::FORBIDDEN)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::json!({"ok": false, "error": "Forbidden — admin role required", "required_role": "admin", "your_role": user.role_str}).to_string()
            ))
            .expect("403 response");
    }
    // No auth user injected — shouldn't happen if require_auth ran first
    axum::response::Response::builder()
        .status(axum::http::StatusCode::UNAUTHORIZED)
        .body(axum::body::Body::from("Unauthorized"))
        .expect("401 response")
}

/// RBAC middleware — require Manager or higher role.
/// Apply to routes that manage agents, channels, knowledge, workflows.
async fn require_role_manager(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if let Some(user) = req.extensions().get::<AuthUser>() {
        if user.role >= Role::Manager {
            return next.run(req).await;
        }
        tracing::warn!(
            "[rbac] User '{}' (role={}) denied access to manager route",
            user.email,
            user.role_str
        );
        return axum::response::Response::builder()
            .status(axum::http::StatusCode::FORBIDDEN)
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::json!({"ok": false, "error": "Forbidden — manager role required", "required_role": "manager", "your_role": user.role_str}).to_string()
            ))
            .expect("403 response");
    }
    axum::response::Response::builder()
        .status(axum::http::StatusCode::UNAUTHORIZED)
        .body(axum::body::Body::from("Unauthorized"))
        .expect("401 response")
}

/// Security headers middleware — CSP, HSTS, XSS protection, Permissions-Policy.
async fn security_headers(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        "X-Content-Type-Options",
        "nosniff".parse().expect("static header"),
    );
    headers.insert("X-Frame-Options", "DENY".parse().expect("static header"));
    headers.insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin"
            .parse()
            .expect("static header"),
    );
    headers.insert(
        "X-XSS-Protection",
        "1; mode=block".parse().expect("static header"),
    );
    // HSTS — tell browsers to always use HTTPS (1 year)
    headers.insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains"
            .parse()
            .expect("static header"),
    );
    // CSP — restrict script/style sources (includes esm.sh for Preact CDN)
    // SECURITY: removed 'unsafe-eval' — only 'unsafe-inline' kept for embedded Preact dashboard
    headers.insert("Content-Security-Policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline' https://esm.sh https://cdn.jsdelivr.net https://cdnjs.cloudflare.com https://fonts.googleapis.com; style-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net https://cdnjs.cloudflare.com https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com https://cdnjs.cloudflare.com; img-src 'self' data: https:; connect-src 'self' ws: wss: https://esm.sh https://clawhub.ai; frame-ancestors 'none'; base-uri 'self'; form-action 'self'"
        .parse().expect("static CSP header"));
    // Permissions-Policy — restrict browser features not needed by the dashboard
    headers.insert("Permissions-Policy",
        "camera=(), microphone=(), geolocation=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()"
        .parse().expect("static header"));
    response
}

/// Build the Axum router with all routes.
pub fn build_router(state: AppState) -> Router {
    build_router_from_arc(Arc::new(state))
}

pub fn build_router_from_arc(shared: Arc<AppState>) -> Router {
    // ═══ ADMIN-only routes — require "admin" role ═══
    // Config changes, provider CRUD, API key management, plan limits, system metrics
    let admin_routes = Router::new()
        .route("/api/v1/config/update", post(super::routes::update_config))
        .route("/api/v1/config/full", get(super::routes::get_full_config))
        .route("/api/v1/providers", post(super::routes::create_provider))
        .route(
            "/api/v1/providers/{name}",
            put(super::routes::update_provider),
        )
        .route(
            "/api/v1/providers/{name}",
            axum::routing::delete(super::routes::delete_provider),
        )
        // PaaS: API Key Management (admin only)
        .route("/api/v1/api-keys", get(super::routes::list_api_keys))
        .route("/api/v1/api-keys", post(super::routes::create_api_key))
        .route(
            "/api/v1/api-keys/{id}",
            axum::routing::delete(super::routes::revoke_api_key),
        )
        // PaaS: Plan limits (admin only)
        .route(
            "/api/v1/usage/limits",
            axum::routing::put(super::routes::update_plan_limits),
        )
        // PaaS: System Metrics (admin only)
        .route("/api/v1/metrics", get(super::routes::get_system_metrics))
        // Audit Trail (admin only)
        .route("/api/v1/audit", get(super::routes::list_audit_log))
        // Backup & Restore (admin only)
        .route("/api/v1/backup", get(super::routes::export_backup))
        .route("/api/v1/restore", post(super::routes::import_restore))
        .route_layer(axum::middleware::from_fn(require_role_admin));

    // ═══ MANAGER routes — require "manager" or higher role ═══
    // Agent CRUD, channel write, knowledge write, workflow write, skills write
    let manager_routes = Router::new()
        .route("/api/v1/agents", post(super::routes::create_agent))
        .route(
            "/api/v1/agents/{name}",
            axum::routing::delete(super::routes::delete_agent),
        )
        .route("/api/v1/agents/{name}", put(super::routes::update_agent))
        // Channels write
        .route(
            "/api/v1/channels/update",
            post(super::routes::update_channel),
        )
        .route(
            "/api/v1/channel-instances",
            post(super::routes::save_channel_instance),
        )
        .route(
            "/api/v1/channel-instances/{id}",
            axum::routing::delete(super::routes::delete_channel_instance),
        )
        // Knowledge write
        .route(
            "/api/v1/knowledge/documents",
            post(super::routes::knowledge_add_doc),
        )
        .route(
            "/api/v1/knowledge/documents/{id}",
            axum::routing::delete(super::routes::knowledge_remove_doc),
        )
        .route(
            "/api/v1/knowledge/upload",
            post(super::routes::knowledge_upload_file),
        )
        .route("/api/v1/campaigns", post(super::routes::create_campaign))
        .route(
            "/api/v1/campaigns/{id}",
            axum::routing::delete(super::routes::delete_campaign)
                .put(super::routes::update_campaign),
        )
        .route(
            "/api/v1/campaigns/{id}/run",
            post(super::routes::run_campaign),
        )
        // Workflows write
        .route("/api/v1/workflows", post(super::routes::workflows_create))
        .route("/api/v1/workflows/run", post(super::routes::workflows_run))
        .route(
            "/api/v1/workflows/{id}",
            axum::routing::put(super::routes::workflows_update),
        )
        .route(
            "/api/v1/workflows/{id}",
            axum::routing::delete(super::routes::workflows_delete),
        )
        .route(
            "/api/v1/workflow-rules",
            post(super::routes::workflow_rules_add),
        )
        .route(
            "/api/v1/workflow-rules/{id}",
            axum::routing::delete(super::routes::workflow_rules_delete),
        )
        // Skills write
        .route("/api/v1/skills", post(super::routes::skills_create))
        .route(
            "/api/v1/skills/install",
            post(super::routes::skills_install),
        )
        .route(
            "/api/v1/skills/uninstall",
            post(super::routes::skills_uninstall),
        )
        .route(
            "/api/v1/skills/{id}",
            axum::routing::put(super::routes::skills_update),
        )
        .route(
            "/api/v1/skills/{id}",
            axum::routing::delete(super::routes::skills_delete),
        )
        // Tools write
        .route("/api/v1/tools", post(super::routes::tools_create))
        .route("/api/v1/handoff/pause", post(super::routes::handoff_pause))
        .route(
            "/api/v1/handoff/resume",
            post(super::routes::handoff_resume),
        )
        .route(
            "/api/v1/handoff/request",
            post(super::routes::request_handoff),
        )
        .route(
            "/api/v1/handoff/settings",
            get(super::routes::get_handoff_settings),
        )
        .route(
            "/api/v1/handoff/settings",
            post(super::routes::save_handoff_settings),
        )
        .route(
            "/api/v1/tools/{name}/toggle",
            post(super::routes::tools_toggle),
        )
        .route(
            "/api/v1/tools/{name}",
            axum::routing::delete(super::routes::tools_delete),
        )
        // Gallery write
        .route("/api/v1/gallery", post(super::routes::gallery_create))
        .route(
            "/api/v1/gallery/{id}",
            axum::routing::delete(super::routes::gallery_delete),
        )
        .route(
            "/api/v1/gallery/{id}/md",
            post(super::routes::gallery_upload_md),
        )
        // Scheduler write
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
        // Orchestration write
        .route(
            "/api/v1/handoff/resolve/{id}",
            post(super::routes::resolve_handoff),
        )
        .route(
            "/api/v1/handoff/delete/{id}",
            axum::routing::delete(super::routes::delete_handoff),
        )
        .route(
            "/api/v1/orchestration/delegate",
            post(super::routes::orch_delegate),
        )
        .route(
            "/api/v1/orchestration/handoff",
            post(super::routes::orch_handoff),
        )
        .route(
            "/api/v1/orchestration/handoff/{session_id}",
            axum::routing::delete(super::routes::orch_clear_handoff),
        )
        .route(
            "/api/v1/orchestration/evaluate",
            post(super::routes::orch_evaluate),
        )
        .route(
            "/api/v1/orchestration/links",
            post(super::routes::orch_create_link),
        )
        .route(
            "/api/v1/orchestration/links/{id}",
            axum::routing::delete(super::routes::orch_delete_link),
        )
        // Agent-Channel bindings write
        .route(
            "/api/v1/agents/{name}/channels",
            post(super::routes::agent_bind_channels),
        )
        // Telegram Bot connect/disconnect
        .route(
            "/api/v1/agents/{name}/telegram",
            post(super::routes::connect_telegram),
        )
        .route(
            "/api/v1/agents/{name}/telegram",
            axum::routing::delete(super::routes::disconnect_telegram),
        )
        // Brain workspace write
        .route(
            "/api/v1/brain/files/{filename}",
            axum::routing::put(super::routes::brain_write_file),
        )
        .route(
            "/api/v1/brain/files/{filename}",
            axum::routing::delete(super::routes::brain_delete_file),
        )
        .route(
            "/api/v1/brain/personalize",
            post(super::routes::brain_personalize),
        )
        // Traces/Activity clear
        .route(
            "/api/v1/traces",
            axum::routing::delete(super::routes::clear_traces),
        )
        .route(
            "/api/v1/activity",
            axum::routing::delete(super::routes::clear_activity),
        )
        // Product Catalog write
        .route("/api/v1/products", post(super::routes::products_upsert))
        .route(
            "/api/v1/products/{id}",
            axum::routing::delete(super::routes::products_delete),
        )
        .route(
            "/api/v1/products/sync-rag",
            post(super::routes::products_sync_rag),
        )
        // Social Automation Pipeline
        .route(
            "/api/v1/social/pipeline",
            post(super::routes::api_social::update_pipeline_config),
        )
        .route(
            "/api/v1/social/pipeline/trigger",
            post(super::routes::api_social::trigger_pipeline),
        )
        .route_layer(axum::middleware::from_fn(require_role_manager));

    // ═══ USER routes — any authenticated user (viewer, user, manager, admin) ═══
    // Read-only views, chat, search, list operations
    let user_routes = Router::new()
        .route("/api/v1/info", get(super::routes::system_info))
        .route("/api/v1/config", get(super::routes::get_config))
        .route("/api/v1/providers", get(super::routes::list_providers))
        .route(
            "/api/v1/providers/{name}/models",
            get(super::routes::fetch_provider_models),
        )
        .route("/api/v1/channels", get(super::routes::list_channels))
        .route(
            "/api/v1/channel-instances",
            get(super::routes::list_channel_instances),
        )
        .route("/api/v1/ollama/models", get(super::routes::ollama_models))
        .route(
            "/api/v1/brain/models",
            get(super::routes::brain_scan_models),
        )
        .route(
            "/api/v1/brain/models/download",
            post(super::routes::brain_download_model),
        )
        .route(
            "/api/v1/brain/models/status/{filename}",
            get(super::routes::brain_download_status),
        )
        .route(
            "/api/v1/brain/models/{filename}",
            axum::routing::delete(super::routes::brain_delete_model),
        )
        .route("/api/v1/zalo/qr", post(super::routes::zalo_qr_code))
        // Campaigns read
        .route("/api/v1/campaigns", get(super::routes::list_campaigns))
        // Scheduler read
        .route(
            "/api/v1/scheduler/tasks",
            get(super::routes::scheduler_list_tasks),
        )
        .route(
            "/api/v1/scheduler/notifications",
            get(super::routes::scheduler_notifications),
        )
        // Knowledge read + search
        .route(
            "/api/v1/knowledge/search",
            post(super::routes::knowledge_search),
        )
        .route(
            "/api/v1/knowledge/documents",
            get(super::routes::knowledge_list_docs),
        )
        .route(
            "/api/v1/knowledge/stats",
            get(super::routes::knowledge_stats),
        )
        .route(
            "/api/v1/knowledge/nudges",
            post(super::routes::knowledge_nudges),
        )
        .route(
            "/api/v1/knowledge/mcp/tools",
            get(super::routes::knowledge_mcp_tools),
        )
        .route(
            "/api/v1/knowledge/mcp/call",
            post(super::routes::knowledge_mcp_call),
        )
        .route(
            "/api/v1/knowledge/watch/scan",
            post(super::routes::knowledge_watch_scan),
        )
        .route(
            "/api/v1/knowledge/signals/stats",
            get(super::routes::knowledge_signal_stats),
        )
        .route(
            "/api/v1/knowledge/signals/feedback",
            post(super::routes::knowledge_signal_feedback),
        )
        // Agents read + chat
        .route("/api/v1/agents", get(super::routes::list_agents))
        .route(
            "/api/v1/agents/{name}/chat",
            post(super::routes::agent_chat),
        )
        .route(
            "/api/v1/agents/broadcast",
            post(super::routes::agent_broadcast),
        )
        // Orchestration read
        .route(
            "/api/v1/orchestration/links",
            get(super::routes::orch_list_links),
        )
        .route(
            "/api/v1/orchestration/delegations",
            get(super::routes::orch_list_delegations),
        )
        .route(
            "/api/v1/orchestration/traces",
            get(super::routes::orch_list_traces),
        )
        // Gallery read
        .route("/api/v1/gallery", get(super::routes::gallery_list))
        .route(
            "/api/v1/gallery/{id}/md",
            get(super::routes::gallery_get_md),
        )
        // Handoff read
        .route(
            "/api/v1/handoff/queue",
            get(super::routes::list_handoff_queue),
        )
        // Agent-Channel bindings read
        .route(
            "/api/v1/agents/channels",
            get(super::routes::agent_channel_bindings),
        )
        // Telegram status read
        .route(
            "/api/v1/agents/{name}/telegram",
            get(super::routes::telegram_status),
        )
        // Zalo session status read
        .route(
            "/api/v1/channels/zalo/session",
            get(super::routes::zalo_session_status),
        )
        // Brain workspace read
        .route("/api/v1/brain/files", get(super::routes::brain_list_files))
        .route(
            "/api/v1/brain/files/{filename}",
            get(super::routes::brain_read_file),
        )
        // Health Check
        .route("/api/v1/health", get(super::routes::system_health_check))
        // LLM Traces & Cost read
        .route("/api/v1/traces", get(super::openai_compat::list_traces))
        .route(
            "/api/v1/traces/cost",
            get(super::openai_compat::cost_breakdown),
        )
        .route("/api/v1/activity", get(super::openai_compat::list_activity))
        // Tools read
        .route("/api/v1/tools", get(super::routes::tools_list))
        // Products read
        .route("/api/v1/products", get(super::routes::products_list))
        // MCP read
        .route("/api/v1/mcp/servers", get(super::routes::mcp_list_servers))
        .route("/api/v1/mcp/catalog", get(super::routes::mcp_catalog))
        // Workflows read
        .route("/api/v1/workflows", get(super::routes::workflows_list))
        .route(
            "/api/v1/workflow-rules",
            get(super::routes::workflow_rules_list),
        )
        // Skills read
        .route("/api/v1/skills", get(super::routes::skills_list))
        .route("/api/v1/skills/{id}", get(super::routes::skills_detail))
        // TTS
        .route("/api/v1/tts/voices", get(super::routes::tts_voices))
        // Usage read
        .route("/api/v1/usage", get(super::routes::get_usage))
        .route("/api/v1/usage/daily", get(super::routes::get_usage_daily))
        .route("/api/v1/usage/limits", get(super::routes::get_plan_limits))
        // Enterprise: SSO
        .route(
            "/api/v1/sso/config",
            get(super::routes::sso_config_get).post(super::routes::sso_config_post),
        )
        // Enterprise: Analytics
        .route("/api/v1/analytics", get(super::routes::analytics_metrics))
        // Enterprise: Fine-Tuning
        .route(
            "/api/v1/fine-tuning/config",
            get(super::routes::fine_tuning_config_get),
        )
        .route(
            "/api/v1/fine-tuning/datasets",
            get(super::routes::fine_tuning_datasets),
        )
        // Enterprise: Edge IoT Gateway
        .route(
            "/api/v1/edge/status",
            get(super::routes::edge_gateway_status),
        )
        // Enterprise: Plugin Marketplace
        .route("/api/v1/plugins", get(super::routes::plugins_list))
        .route(
            "/api/v1/plugins/install",
            post(super::routes::plugin_install),
        )
        // NL Query (Text2SQL RAG)
        .route(
            "/api/v1/nl-query/status",
            get(super::routes::nl_query_status),
        )
        .route("/api/v1/nl-query/ask", post(super::routes::nl_query_ask))
        .route(
            "/api/v1/nl-query/index",
            post(super::routes::nl_query_index),
        )
        .route(
            "/api/v1/nl-query/rules/{conn_id}",
            get(super::routes::nl_query_rules_get),
        )
        .route(
            "/api/v1/nl-query/rules",
            post(super::routes::nl_query_rules_add),
        )
        .route(
            "/api/v1/nl-query/connections",
            post(super::routes::nl_query_connections_add),
        )
        .route(
            "/api/v1/nl-query/examples/{conn_id}",
            get(super::routes::nl_query_examples_get),
        )
        // Social Automation Pipeline (Read)
        .route(
            "/api/v1/social/status",
            get(super::routes::api_social::social_status),
        )
        .route(
            "/api/v1/social/pipeline",
            get(super::routes::api_social::get_pipeline_config),
        )
        // CRM routes (OmniChannel Customer Management)
        .route(
            "/api/v1/crm/contacts",
            get(super::routes::crm::list_contacts),
        )
        .route(
            "/api/v1/crm/contacts",
            post(super::routes::crm::create_contact),
        )
        .route(
            "/api/v1/crm/contacts/{id}",
            get(super::routes::crm::get_contact),
        )
        .route(
            "/api/v1/crm/contacts/{id}",
            put(super::routes::crm::update_contact),
        )
        .route(
            "/api/v1/crm/contacts/{id}/pipeline",
            put(super::routes::crm::update_pipeline),
        )
        .route(
            "/api/v1/crm/contacts/{id}/interactions",
            get(super::routes::crm::get_interactions),
        )
        .route(
            "/api/v1/crm/interactions",
            post(super::routes::crm::create_interaction),
        )
        .route(
            "/api/v1/crm/conversations",
            get(super::routes::crm::list_conversations),
        )
        .route(
            "/api/v1/crm/conversations/{id}",
            get(super::routes::crm::get_conversation),
        )
        .route(
            "/api/v1/crm/conversations/{id}/read",
            post(super::routes::crm::mark_read),
        )
        .route(
            "/api/v1/crm/dashboard",
            get(super::routes::crm::get_dashboard),
        )
        // WebSocket (chat)
        .route("/ws", get(super::ws::ws_handler));

    // Merge all RBAC layers under a single auth gate
    let protected = admin_routes
        .merge(manager_routes)
        .merge(user_routes)
        .route_layer(axum::middleware::from_fn_with_state(
            shared.clone(),
            require_auth,
        ));

    // Public routes — no auth
    let public = Router::new()
        .route("/", get(dashboard_page))
        .route("/hub", get(hub_page))
        .route("/landing", get(landing_page))
        .route("/legacy", get(legacy_dashboard_page))
        .route("/new", get(dashboard_new_page))
        .route("/workflow-builder", get(workflow_builder_page))
        .route("/static/dashboard/{*path}", get(dashboard_static))
        .route("/health", get(super::routes::health_check))
        // Prometheus metrics — public for scraper access (text/plain)
        .route("/metrics", get(super::routes::prometheus_metrics))
        .route("/api/v1/verify-pairing", post(verify_auth)) // kept same path for backward compat
        // WhatsApp webhook — must be public for Meta verification
        .route(
            "/api/v1/webhook/whatsapp",
            get(super::routes::whatsapp_webhook_verify).post(super::routes::whatsapp_webhook),
        )
        // Webhook inbound — public, auth via HMAC signature in header
        .route(
            "/api/v1/webhook/inbound",
            post(super::routes::webhook_inbound),
        )
        // Xiaozhi webhook — public, auth via header signature
        .route(
            "/api/v1/xiaozhi/webhook",
            post(super::routes::xiaozhi_webhook),
        )
        // Zalo OA webhook — public, auth via OA app_secret MAC
        .route(
            "/api/v1/webhook/zalo-oa",
            post(super::routes::zalo_oa_webhook),
        )
        // Zalo Bot Platform webhook — public, auth via X-Bot-Api-Secret-Token
        .route(
            "/api/v1/webhook/zalo-bot",
            post(super::routes::zalo_bot_webhook),
        )
        // Facebook Messenger webhook — public, auth via HMAC + verify token
        .route(
            "/api/v1/webhook/messenger",
            get(super::routes::messenger_webhook_verify).post(super::routes::messenger_webhook),
        )
        // OpenAI-Compatible API — public with own auth (Bearer token)
        .route(
            "/v1/chat/completions",
            post(super::openai_compat::chat_completions),
        )
        .route("/v1/models", get(super::openai_compat::list_models))
        // Rate limiting for all public routes
        .route_layer(axum::middleware::from_fn_with_state(
            shared.clone(),
            rate_limit,
        ));

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

            // CORS origin policy:
            // 1. BIZCLAW_CORS_ORIGINS env var → explicit whitelist (production)
            // 2. No JWT secret (dev mode) → allow all origins for convenience
            // 3. JWT secret set but no CORS env → same-origin only (secure default)
            if let Ok(origins_str) = std::env::var("BIZCLAW_CORS_ORIGINS") {
                let origins: Vec<_> = origins_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<axum::http::HeaderValue>().ok())
                    .collect();
                tracing::info!("🔒 CORS: restricted to {} origin(s)", origins.len());
                cors.allow_origin(origins)
            } else if shared.jwt_secret.is_empty() {
                // Dev mode — no auth, allow all origins
                tracing::warn!("⚠️ CORS: allow-all (dev mode, no JWT secret configured)");
                cors.allow_origin(Any)
            } else {
                // Production with JWT but no explicit CORS — restrict to same-origin
                tracing::info!(
                    "🔒 CORS: same-origin only (set BIZCLAW_CORS_ORIGINS to whitelist domains)"
                );
                cors
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
    let agent: Option<bizclaw_agent::Agent> = match bizclaw_agent::Agent::new(full_config.clone()) {
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
    let orch_store: Arc<dyn bizclaw_db::DataStore> =
        match bizclaw_db::SqliteStore::open(&orch_db_path) {
            Ok(store) => {
                let store = Arc::new(store);
                // Run migrations
                if let Err(e) = store.migrate().await {
                    tracing::error!("❌ Orchestration DB migration failed: {e}");
                } else {
                    tracing::info!(
                        "🔗 Orchestration DB initialized: {}",
                        orch_db_path.display()
                    );
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
    let mut orchestrator =
        bizclaw_agent::orchestrator::Orchestrator::with_store(orch_store.clone());

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
                Ok(count) => tracing::info!(
                    "📦 Migrated {} agent(s) from agents.json → gateway.db",
                    count
                ),
                Err(e) => tracing::warn!("⚠️ Migration from agents.json failed: {e}"),
            }
        }
    }

    // Seed demo agents if DB is completely empty (first-time install)
    // Creates 5 agents in 2 departments + orchestration links for demo
    let db_agents_after = gateway_db.list_agents().unwrap_or_default();
    if db_agents_after.is_empty() {
        let prov = full_config.default_provider.as_str();
        let model = full_config.default_model.as_str();
        match gateway_db.seed_demo_agents(prov, model) {
            Ok(count) if count > 0 => {
                tracing::info!("🎉 Demo seeded: {} agents across 2 departments", count);
                // Seed orchestration links between demo agents
                // These will be visible in Org Map page
                let demo_links: Vec<(&str, &str)> = vec![
                    // Phòng KD: Sales ↔ Marketing
                    ("sales-bot", "marketing-bot"),
                    // Phòng KT: Coder → Support, Analyst aggregates
                    ("support-bot", "coder-bot"),
                    ("analyst-bot", "sales-bot"),
                    ("analyst-bot", "marketing-bot"),
                    ("analyst-bot", "coder-bot"),
                ];
                for (src, tgt) in &demo_links {
                    let link = bizclaw_core::types::AgentLink::new(
                        src,
                        tgt,
                        bizclaw_core::types::LinkDirection::Outbound,
                    );
                    let _ = orch_store.create_link(&link).await;
                }
                tracing::info!("  🔗 Seeded {} orchestration links", demo_links.len());
            }
            Ok(_) => {} // DB already had agents, no seeding
            Err(e) => tracing::warn!("⚠️ Demo agent seeding failed: {e}"),
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
                    orchestrator.add_agent(
                        &agent_rec.name,
                        &agent_rec.role,
                        &agent_rec.description,
                        agent,
                    );
                    tracing::info!(
                        "  ✅ Agent '{}' restored ({})",
                        agent_rec.name,
                        agent_rec.role
                    );
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

    // ── P1.2 FIX: Inject shared Knowledge Base into ALL agents ──
    // Without this, agents responding via Telegram/Zalo had no RAG context
    // and would hallucinate product prices and information.
    let knowledge_arc_for_agents = Arc::new(tokio::sync::Mutex::new(knowledge));
    {
        for agent_name in orchestrator
            .list_agents()
            .iter()
            .filter_map(|a| a["name"].as_str().map(|s| s.to_string()))
            .collect::<Vec<_>>()
        {
            if let Some(agent) = orchestrator.get_agent_mut(&agent_name) {
                agent.set_knowledge(knowledge_arc_for_agents.clone());
            }
        }
        tracing::info!(
            "📚 Knowledge Base injected into {} agent(s)",
            orchestrator.agent_count()
        );
    }

    // Wrap orchestrator in Arc for shared access
    let orchestrator_arc = Arc::new(tokio::sync::Mutex::new(orchestrator));

    let (activity_tx, _rx) =
        tokio::sync::broadcast::channel::<super::openai_compat::ActivityEvent>(256);

    // Spawn scheduler background loop with Agent integration (check every 30 seconds)
    let sched_clone = scheduler.clone();
    let orch_for_sched = orchestrator_arc.clone();
    let config_for_sched = full_config.clone();
    let activity_tx_for_sched = activity_tx.clone();
    let db_for_sched = gateway_db.clone();
    tokio::spawn(async move {
        bizclaw_scheduler::engine::spawn_scheduler_with_agent(
            sched_clone,
            // Agent callback: execute prompt through orchestrator
            move |prompt: String| {
                let orch = orch_for_sched.clone();
                async move {
                    let mut o = orch.lock().await;
                    o.send(&prompt).await.map_err(|e| e.to_string())
                }
            },
            // Result callback: dispatch results to channels + activity feed
            move |task_name: String, response: String| {
                let cfg = config_for_sched.clone();
                let tx = activity_tx_for_sched.clone();
                let db = db_for_sched.clone();
                async move {
                    // 1. Broadcast to Dashboard Activity Feed
                    let _ = tx.send(super::openai_compat::ActivityEvent {
                        event_type: "hand.completed".into(),
                        agent: task_name.clone(),
                        detail: (if response.chars().count() > 150 {
                            let t: String = response.chars().take(150).collect();
                            format!("{}...", t)
                        } else {
                            response.clone()
                        })
                        .to_string(),
                        timestamp: chrono::Utc::now(),
                    });

                    // 2. Track usage
                    let _ = db.track_usage("hand_executions", 1.0);

                    // 3. Send to Telegram (all configured bots)
                    for tg_cfg in &cfg.channel.telegram {
                        if tg_cfg.enabled && !tg_cfg.bot_token.is_empty() {
                            let chat_id = std::env::var("BIZCLAW_NOTIFY_TELEGRAM_CHAT_ID")
                                .unwrap_or_default();
                            if !chat_id.is_empty() {
                                let msg = format!(
                                    "🤚 *Hand Report: {}*\n\n{}\n\n_— BizClaw Autonomous Hands_",
                                    task_name,
                                    if response.chars().count() > 3500 {
                                        let t: String = response.chars().take(3500).collect();
                                        format!("{}...", t)
                                    } else {
                                        response.clone()
                                    }
                                );
                                let url = format!(
                                    "https://api.telegram.org/bot{}/sendMessage",
                                    tg_cfg.bot_token
                                );
                                let client = reqwest::Client::new();
                                let _ = client
                                    .post(&url)
                                    .json(&serde_json::json!({
                                        "chat_id": chat_id,
                                        "text": msg,
                                        "parse_mode": "Markdown"
                                    }))
                                    .send()
                                    .await;
                                tracing::info!(
                                    "📨 Hand result sent to Telegram '{}': {} → chat {}",
                                    tg_cfg.name,
                                    task_name,
                                    chat_id
                                );
                            }
                        }
                    }

                    // 4. Send to Webhooks (all configured instances)
                    for wh_cfg in &cfg.channel.webhook {
                        if wh_cfg.enabled && !wh_cfg.outbound_url.is_empty() {
                            let client = reqwest::Client::new();
                            let _ = client
                                .post(&wh_cfg.outbound_url)
                                .json(&serde_json::json!({
                                    "event": "hand.completed",
                                    "task_name": task_name,
                                    "result": response,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                }))
                                .send()
                                .await;
                        }
                    }

                    tracing::info!("📢 Hand result dispatched: {}", task_name);
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
        // pairing_code removed — SaaS uses JWT from Platform login
        jwt_secret: std::env::var("JWT_SECRET").unwrap_or_default(),
        auth_failures: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        agent: Arc::new(tokio::sync::Mutex::new(agent)),
        orchestrator: orchestrator_arc.clone(),
        scheduler,
        knowledge: knowledge_arc_for_agents.clone(),
        crm: Arc::new(bizclaw_crm::CRMManager::new()),
        telegram_bots: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        db: gateway_db,
        orch_store,
        traces: Arc::new(Mutex::new(Vec::new())),
        activity_tx: activity_tx.clone(),
        activity_log: Arc::new(Mutex::new(Vec::new())),
        rate_limiter: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        paused_threads: Arc::new(tokio::sync::RwLock::new(std::collections::HashSet::new())),
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

    // 🚀 FLEET MANAGEMENT & TELEMETRY WORKER
    let ota_state = state_arc.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        loop {
            // Get local identifier
            let node_id = std::env::var("VPS_IP").unwrap_or_else(|_| "unknown-node".to_string());
            let version = env!("CARGO_PKG_VERSION");

            let payload = serde_json::json!({
                "node_id": node_id,
                "version": version,
                "uptime_secs": ota_state.start_time.elapsed().as_secs(),
                "status": "online"
            });

            // Telemetry ping to Central Hub (Mắt thần trung tâm)
            match client
                .post("https://hub.viagent.vn/api/v1/fleet/telemetry")
                .timeout(std::time::Duration::from_secs(5))
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) => {
                    // Check if Platform requested an OTA update
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if data["update_triggered"].as_bool().unwrap_or(false) {
                            tracing::warn!(
                                "🔄 OTA Update triggered by Central Hub! Executing update..."
                            );
                            let _ = std::process::Command::new("bash")
                                .arg("-c")
                                .arg("cd /opt/bizclaw && git pull origin main && docker-compose -f docker-compose.prod.yml up -d --build")
                                .spawn();
                        }
                    }
                }
                Err(_e) => {
                    // Fail silently, telemetry is non-critical
                }
            }
            // Ping every 60 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🌐 Gateway server listening on http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("🛑 Gateway server gracefully shutdown.");
    Ok(())
}

/// Listen for CTRL+C (SIGINT) and SIGTERM (from `kill`) to trigger graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("🔄 Received CTRL+C, starting graceful shutdown...");
        },
        _ = terminate => {
            tracing::info!("🔄 Received SIGTERM (kill), starting graceful shutdown...");
        },
    }
}
