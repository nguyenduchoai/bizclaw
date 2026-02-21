//! Admin HTTP server — REST API for the admin control plane.

use axum::{Router, Json, routing::{get, post, delete}, extract::{State, Path}};
use std::sync::{Arc, Mutex};
use crate::db::PlatformDb;
use crate::tenant::TenantManager;

/// Shared application state for the admin server.
pub struct AdminState {
    pub db: Mutex<PlatformDb>,
    pub manager: Mutex<TenantManager>,
    pub jwt_secret: String,
    pub bizclaw_bin: String,
    pub base_port: u16,
}

/// Admin API server.
pub struct AdminServer;

impl AdminServer {
    /// Build the admin router.
    pub fn router(state: Arc<AdminState>) -> Router {
        Router::new()
            // Dashboard
            .route("/api/admin/stats", get(get_stats))
            .route("/api/admin/activity", get(get_activity))
            // Tenants
            .route("/api/admin/tenants", get(list_tenants))
            .route("/api/admin/tenants", post(create_tenant))
            .route("/api/admin/tenants/{id}", get(get_tenant))
            .route("/api/admin/tenants/{id}", delete(delete_tenant))
            .route("/api/admin/tenants/{id}/start", post(start_tenant))
            .route("/api/admin/tenants/{id}/stop", post(stop_tenant))
            .route("/api/admin/tenants/{id}/restart", post(restart_tenant))
            .route("/api/admin/tenants/{id}/pairing", post(reset_pairing))
            // Users
            .route("/api/admin/users", get(list_users))
            // Auth
            .route("/api/admin/login", post(login))
            .route("/api/admin/pairing/validate", post(validate_pairing))
            // Dashboard HTML
            .route("/", get(admin_dashboard_page))
            .with_state(state)
    }

    /// Start the admin server.
    pub async fn start(state: Arc<AdminState>, port: u16) -> bizclaw_core::error::Result<()> {
        let app = Self::router(state);
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        tracing::info!("🏢 Admin platform running at http://localhost:{port}");

        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| bizclaw_core::error::BizClawError::Gateway(format!("Bind error: {e}")))?;

        axum::serve(listener, app).await
            .map_err(|e| bizclaw_core::error::BizClawError::Gateway(format!("Server error: {e}")))?;

        Ok(())
    }
}

// ── API Handlers ────────────────────────────────────

async fn get_stats(State(state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let (total, running, stopped, error) = state.db.lock().unwrap().tenant_stats().unwrap_or((0,0,0,0));
    let users = state.db.lock().unwrap().list_users().map(|u| u.len() as u32).unwrap_or(0);
    Json(serde_json::json!({
        "total_tenants": total, "running": running, "stopped": stopped,
        "error": error, "users": users
    }))
}

async fn get_activity(State(state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let events = state.db.lock().unwrap().recent_events(20).unwrap_or_default();
    Json(serde_json::json!({ "events": events }))
}

async fn list_tenants(State(state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let tenants = state.db.lock().unwrap().list_tenants().unwrap_or_default();
    Json(serde_json::json!({ "tenants": tenants }))
}

#[derive(serde::Deserialize)]
struct CreateTenantReq {
    name: String,
    slug: String,
    provider: Option<String>,
    model: Option<String>,
    plan: Option<String>,
}

async fn create_tenant(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<CreateTenantReq>,
) -> Json<serde_json::Value> {
    let port = {
        let db = state.db.lock().unwrap();
        let used_ports = db.used_ports().unwrap_or_default();
        let mut port = state.base_port;
        while used_ports.contains(&port) {
            port += 1;
        }
        port
    };

    match state.db.lock().unwrap().create_tenant(
        &req.name, &req.slug, port,
        req.provider.as_deref().unwrap_or("openai"),
        req.model.as_deref().unwrap_or("gpt-4o-mini"),
        req.plan.as_deref().unwrap_or("free"),
    ) {
        Ok(tenant) => {
            state.db.lock().unwrap().log_event("tenant_created", "admin", &tenant.id, Some(&format!("slug={}", req.slug))).ok();
            Json(serde_json::json!({"ok": true, "tenant": tenant}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

async fn get_tenant(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.db.lock().unwrap().get_tenant(&id) {
        Ok(t) => Json(serde_json::json!({"ok": true, "tenant": t})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

async fn delete_tenant(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    state.manager.lock().unwrap().stop_tenant(&id).ok();
    match state.db.lock().unwrap().delete_tenant(&id) {
        Ok(()) => {
            state.db.lock().unwrap().log_event("tenant_deleted", "admin", &id, None).ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

async fn start_tenant(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let tenant = match state.db.lock().unwrap().get_tenant(&id) {
        Ok(t) => t,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    };

    let mut mgr = state.manager.lock().unwrap();
    match mgr.start_tenant(&tenant, &state.bizclaw_bin) {
        Ok(pid) => {
            state.db.lock().unwrap().update_tenant_status(&id, "running", Some(pid)).ok();
            state.db.lock().unwrap().log_event("tenant_started", "admin", &id, None).ok();
            Json(serde_json::json!({"ok": true, "pid": pid}))
        }
        Err(e) => {
            state.db.lock().unwrap().update_tenant_status(&id, "error", None).ok();
            Json(serde_json::json!({"ok": false, "error": e.to_string()}))
        }
    }
}

async fn stop_tenant(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    state.manager.lock().unwrap().stop_tenant(&id).ok();
    state.db.lock().unwrap().update_tenant_status(&id, "stopped", None).ok();
    state.db.lock().unwrap().log_event("tenant_stopped", "admin", &id, None).ok();
    Json(serde_json::json!({"ok": true}))
}

async fn restart_tenant(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let tenant = match state.db.lock().unwrap().get_tenant(&id) {
        Ok(t) => t,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    };

    let mut mgr = state.manager.lock().unwrap();
    let db = state.db.lock().unwrap();
    match mgr.restart_tenant(&tenant, &state.bizclaw_bin, &db) {
        Ok(pid) => Json(serde_json::json!({"ok": true, "pid": pid})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

async fn reset_pairing(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.db.lock().unwrap().reset_pairing_code(&id) {
        Ok(code) => {
            state.db.lock().unwrap().log_event("tenant_pairing_reset", "admin", &id, None).ok();
            Json(serde_json::json!({"ok": true, "pairing_code": code}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

async fn list_users(State(state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let users = state.db.lock().unwrap().list_users().unwrap_or_default();
    Json(serde_json::json!({"users": users}))
}

#[derive(serde::Deserialize)]
struct LoginReq { email: String, password: String }

async fn login(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<LoginReq>,
) -> Json<serde_json::Value> {
    match state.db.lock().unwrap().get_user_by_email(&req.email) {
        Ok(Some((id, hash, role))) => {
            if crate::auth::verify_password(&req.password, &hash) {
                match crate::auth::create_token(&id, &req.email, &role, &state.jwt_secret) {
                    Ok(token) => {
                        state.db.lock().unwrap().log_event("login_success", "user", &id, None).ok();
                        Json(serde_json::json!({"ok": true, "token": token, "role": role}))
                    }
                    Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
                }
            } else {
                Json(serde_json::json!({"ok": false, "error": "Invalid credentials"}))
            }
        }
        Ok(None) => Json(serde_json::json!({"ok": false, "error": "User not found"})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

#[derive(serde::Deserialize)]
struct PairingReq { slug: String, code: String }

async fn validate_pairing(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<PairingReq>,
) -> Json<serde_json::Value> {
    match state.db.lock().unwrap().validate_pairing(&req.slug, &req.code) {
        Ok(Some(tenant)) => {
            // Generate a session token for this tenant
            match crate::auth::create_token(&tenant.id, &tenant.slug, "tenant", &state.jwt_secret) {
                Ok(token) => {
                    state.db.lock().unwrap().log_event("pairing_success", "tenant", &tenant.id, None).ok();
                    Json(serde_json::json!({"ok": true, "token": token, "tenant": tenant}))
                }
                Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
            }
        }
        Ok(None) => Json(serde_json::json!({"ok": false, "error": "Invalid pairing code"})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

async fn admin_dashboard_page() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("admin_dashboard.html"))
}
