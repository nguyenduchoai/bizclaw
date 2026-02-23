//! Admin HTTP server — REST API for the admin control plane.

use crate::db::PlatformDb;
use crate::tenant::TenantManager;
use axum::middleware;
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use std::sync::{Arc, Mutex};

/// Shared application state for the admin server.
pub struct AdminState {
    pub db: Mutex<PlatformDb>,
    pub manager: Mutex<TenantManager>,
    pub jwt_secret: String,
    pub bizclaw_bin: String,
    pub base_port: u16,
    /// Rate limiter: email → (attempt_count, first_attempt_time)
    pub login_attempts: Mutex<std::collections::HashMap<String, (u32, std::time::Instant)>>,
}

/// JWT auth middleware — validates Authorization: Bearer <token>.
async fn require_auth(
    State(state): State<Arc<AdminState>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Some(token) = auth_header.strip_prefix("Bearer ") {
        if crate::auth::validate_token(token, &state.jwt_secret).is_ok() {
            return next.run(req).await;
        }
    }

    axum::response::Response::builder()
        .status(axum::http::StatusCode::UNAUTHORIZED)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::json!({"ok": false, "error": "Unauthorized — invalid or missing JWT token"}).to_string()
        ))
        .unwrap()
}

/// Admin API server.
pub struct AdminServer;

impl AdminServer {
    /// Build the admin router.
    pub fn router(state: Arc<AdminState>) -> Router {
        // Protected routes — require valid JWT
        let protected = Router::new()
            // Dashboard data
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
            // Channel Configuration
            .route("/api/admin/tenants/{id}/channels", get(list_channels))
            .route("/api/admin/tenants/{id}/channels", post(upsert_channel))
            .route(
                "/api/admin/tenants/{id}/channels/{channel_id}",
                delete(delete_channel),
            )
            .route(
                "/api/admin/tenants/{id}/channels/zalo/qr",
                post(zalo_get_qr),
            )
            // Ollama / Brain Engine
            .route("/api/admin/ollama/models", get(ollama_list_models))
            .route("/api/admin/ollama/pull", post(ollama_pull_model))
            .route("/api/admin/ollama/delete", post(ollama_delete_model))
            .route("/api/admin/ollama/health", get(ollama_health))
            // Tenant Config (key-value settings)
            .route("/api/admin/tenants/{id}/configs", get(list_tenant_configs))
            .route("/api/admin/tenants/{id}/configs", post(set_tenant_configs))
            // Tenant Agents
            .route("/api/admin/tenants/{id}/agents", get(list_tenant_agents))
            .route("/api/admin/tenants/{id}/agents", post(upsert_tenant_agent))
            .route(
                "/api/admin/tenants/{id}/agents/{name}",
                delete(delete_tenant_agent),
            )
            // Users
            .route("/api/admin/users", get(list_users))
            .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

        // Public routes — no auth required
        let public = Router::new()
            .route("/api/admin/login", post(login))
            .route("/api/admin/pairing/validate", post(validate_pairing))
            .route("/", get(admin_dashboard_page));

        // SPA fallback — serve dashboard HTML for all non-API paths
        // so that /tenants, /settings, /ollama etc. all work
        let spa_fallback = Router::new().fallback(get(admin_dashboard_page));

        protected
            .merge(public)
            .merge(spa_fallback)
            .with_state(state)
    }

    /// Start the admin server.
    pub async fn start(state: Arc<AdminState>, port: u16) -> bizclaw_core::error::Result<()> {
        let app = Self::router(state);
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        tracing::info!("🏢 Admin platform running at http://localhost:{port}");

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| bizclaw_core::error::BizClawError::Gateway(format!("Bind error: {e}")))?;

        axum::serve(listener, app).await.map_err(|e| {
            bizclaw_core::error::BizClawError::Gateway(format!("Server error: {e}"))
        })?;

        Ok(())
    }
}

// ── API Handlers ────────────────────────────────────

async fn get_stats(State(state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let (total, running, stopped, error) = state
        .db
        .lock()
        .unwrap()
        .tenant_stats()
        .unwrap_or((0, 0, 0, 0));
    let users = state
        .db
        .lock()
        .unwrap()
        .list_users()
        .map(|u| u.len() as u32)
        .unwrap_or(0);
    Json(serde_json::json!({
        "total_tenants": total, "running": running, "stopped": stopped,
        "error": error, "users": users
    }))
}

async fn get_activity(State(state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let events = state
        .db
        .lock()
        .unwrap()
        .recent_events(20)
        .unwrap_or_default();
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
        &req.name,
        &req.slug,
        port,
        req.provider.as_deref().unwrap_or("openai"),
        req.model.as_deref().unwrap_or("gpt-4o-mini"),
        req.plan.as_deref().unwrap_or("free"),
    ) {
        Ok(tenant) => {
            state
                .db
                .lock()
                .unwrap()
                .log_event(
                    "tenant_created",
                    "admin",
                    &tenant.id,
                    Some(&format!("slug={}", req.slug)),
                )
                .ok();
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
            state
                .db
                .lock()
                .unwrap()
                .log_event("tenant_deleted", "admin", &id, None)
                .ok();
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
    let db = state.db.lock().unwrap();
    match mgr.start_tenant(&tenant, &state.bizclaw_bin, &db) {
        Ok(pid) => {
            drop(db);
            state
                .db
                .lock()
                .unwrap()
                .update_tenant_status(&id, "running", Some(pid))
                .ok();
            state
                .db
                .lock()
                .unwrap()
                .log_event("tenant_started", "admin", &id, None)
                .ok();
            Json(serde_json::json!({"ok": true, "pid": pid}))
        }
        Err(e) => {
            drop(db);
            state
                .db
                .lock()
                .unwrap()
                .update_tenant_status(&id, "error", None)
                .ok();
            Json(serde_json::json!({"ok": false, "error": e.to_string()}))
        }
    }
}

async fn stop_tenant(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    state.manager.lock().unwrap().stop_tenant(&id).ok();
    state
        .db
        .lock()
        .unwrap()
        .update_tenant_status(&id, "stopped", None)
        .ok();
    state
        .db
        .lock()
        .unwrap()
        .log_event("tenant_stopped", "admin", &id, None)
        .ok();
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
            state
                .db
                .lock()
                .unwrap()
                .log_event("tenant_pairing_reset", "admin", &id, None)
                .ok();
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
struct LoginReq {
    email: String,
    password: String,
}

async fn login(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<LoginReq>,
) -> Json<serde_json::Value> {
    // Rate limiting — max 5 attempts per email per 5 minutes
    {
        let mut attempts = state.login_attempts.lock().unwrap();
        let now = std::time::Instant::now();
        if let Some((count, first_at)) = attempts.get(&req.email) {
            if now.duration_since(*first_at).as_secs() < 300 && *count >= 5 {
                return Json(serde_json::json!({
                    "ok": false,
                    "error": "Too many login attempts. Please wait 5 minutes."
                }));
            }
            // Reset if window expired
            if now.duration_since(*first_at).as_secs() >= 300 {
                attempts.remove(&req.email);
            }
        }
        // Record attempt
        let entry = attempts.entry(req.email.clone()).or_insert((0, now));
        entry.0 += 1;
    }

    let user = state.db.lock().unwrap().get_user_by_email(&req.email);
    match user {
        Ok(Some((id, hash, role))) => {
            // Run bcrypt in blocking thread to avoid stalling the async runtime
            let password = req.password.clone();
            let hash_clone = hash.clone();
            let ok = tokio::task::spawn_blocking(move || {
                crate::auth::verify_password(&password, &hash_clone)
            })
            .await
            .unwrap_or(false);

            if ok {
                match crate::auth::create_token(&id, &req.email, &role, &state.jwt_secret) {
                    Ok(token) => {
                        state
                            .db
                            .lock()
                            .unwrap()
                            .log_event("login_success", "user", &id, None)
                            .ok();
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
struct PairingReq {
    slug: String,
    code: String,
}

async fn validate_pairing(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<PairingReq>,
) -> Json<serde_json::Value> {
    match state
        .db
        .lock()
        .unwrap()
        .validate_pairing(&req.slug, &req.code)
    {
        Ok(Some(tenant)) => {
            // Generate a session token for this tenant
            match crate::auth::create_token(&tenant.id, &tenant.slug, "tenant", &state.jwt_secret) {
                Ok(token) => {
                    state
                        .db
                        .lock()
                        .unwrap()
                        .log_event("pairing_success", "tenant", &tenant.id, None)
                        .ok();
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

// ── Channel Configuration Handlers ────────────────────────────────────

async fn list_channels(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.db.lock().unwrap().list_channels(&id) {
        Ok(channels) => Json(serde_json::json!({"ok": true, "channels": channels})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

#[derive(serde::Deserialize)]
struct UpsertChannelReq {
    channel_type: String,
    enabled: bool,
    config: serde_json::Value,
}

async fn upsert_channel(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
    Json(req): Json<UpsertChannelReq>,
) -> Json<serde_json::Value> {
    let config_json = serde_json::to_string(&req.config).unwrap_or_default();
    match state
        .db
        .lock()
        .unwrap()
        .upsert_channel(&id, &req.channel_type, req.enabled, &config_json)
    {
        Ok(channel) => {
            state
                .db
                .lock()
                .unwrap()
                .log_event(
                    "channel_configured",
                    "admin",
                    &id,
                    Some(&format!(
                        "type={}, enabled={}",
                        req.channel_type, req.enabled
                    )),
                )
                .ok();
            Json(serde_json::json!({"ok": true, "channel": channel}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

async fn delete_channel(
    State(state): State<Arc<AdminState>>,
    Path((tenant_id, channel_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    match state.db.lock().unwrap().delete_channel(&channel_id) {
        Ok(()) => {
            state
                .db
                .lock()
                .unwrap()
                .log_event(
                    "channel_deleted",
                    "admin",
                    &tenant_id,
                    Some(&format!("channel_id={}", channel_id)),
                )
                .ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// Zalo QR code generation endpoint — returns QR data URL for scanning.
async fn zalo_get_qr(
    State(_state): State<Arc<AdminState>>,
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
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
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e.to_string(),
            "fallback": "Vui lòng vào chat.zalo.me → F12 → Application → Cookies → Copy toàn bộ và paste vào ô Cookie bên dưới"
        })),
    }
}

// ═══════════════════════════════════════════════════════════
// OLLAMA / BRAIN ENGINE MANAGEMENT
// ═══════════════════════════════════════════════════════════

fn ollama_url() -> String {
    std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string())
}

/// Check if Ollama is running.
async fn ollama_health(State(_state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let client = reqwest::Client::new();
    let url = ollama_url();
    match client.get(format!("{url}/api/tags")).send().await {
        Ok(r) if r.status().is_success() => {
            Json(serde_json::json!({"ok": true, "url": url, "status": "running"}))
        }
        Ok(r) => Json(serde_json::json!({
            "ok": false, "url": url,
            "status": format!("unhealthy: {}", r.status())
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false, "url": url,
            "status": "not_running",
            "error": e.to_string(),
            "install_guide": "curl -fsSL https://ollama.ai/install.sh | sh"
        })),
    }
}

/// List installed Ollama models.
async fn ollama_list_models(State(_state): State<Arc<AdminState>>) -> Json<serde_json::Value> {
    let client = reqwest::Client::new();
    let url = ollama_url();
    match client.get(format!("{url}/api/tags")).send().await {
        Ok(r) if r.status().is_success() => {
            let body: serde_json::Value = r.json().await.unwrap_or_default();
            let models: Vec<serde_json::Value> = body["models"]
                .as_array()
                .map(|arr| {
                    arr.iter().map(|m| {
                    let size_bytes = m["size"].as_u64().unwrap_or(0);
                    let size_mb = size_bytes as f64 / 1_048_576.0;
                    serde_json::json!({
                        "name": m["name"].as_str().unwrap_or(""),
                        "size": format!("{:.0} MB", size_mb),
                        "size_bytes": size_bytes,
                        "modified_at": m["modified_at"].as_str().unwrap_or(""),
                        "family": m["details"]["family"].as_str().unwrap_or(""),
                        "parameter_size": m["details"]["parameter_size"].as_str().unwrap_or(""),
                        "quantization": m["details"]["quantization_level"].as_str().unwrap_or(""),
                    })
                }).collect()
                })
                .unwrap_or_default();
            Json(serde_json::json!({"ok": true, "models": models}))
        }
        _ => Json(serde_json::json!({
            "ok": false,
            "models": [],
            "error": "Ollama not running. Install: curl -fsSL https://ollama.ai/install.sh | sh"
        })),
    }
}

/// Pull (download) a model.
async fn ollama_pull_model(
    State(_state): State<Arc<AdminState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let model = body["model"].as_str().unwrap_or("tinyllama");
    let client = reqwest::Client::new();
    let url = ollama_url();

    tracing::info!("Pulling Ollama model: {}", model);

    match client
        .post(format!("{url}/api/pull"))
        .json(&serde_json::json!({"name": model, "stream": false}))
        .timeout(std::time::Duration::from_secs(600)) // 10 min timeout
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {
            let resp: serde_json::Value = r.json().await.unwrap_or_default();
            tracing::info!("Model pulled: {}", model);
            Json(serde_json::json!({
                "ok": true,
                "model": model,
                "status": resp["status"].as_str().unwrap_or("success"),
                "message": format!("Model '{}' pulled successfully!", model)
            }))
        }
        Ok(r) => {
            let text = r.text().await.unwrap_or_default();
            Json(serde_json::json!({"ok": false, "error": text}))
        }
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": e.to_string(),
            "hint": "Ollama might not be installed. Run: curl -fsSL https://ollama.ai/install.sh | sh"
        })),
    }
}

/// Delete a model.
async fn ollama_delete_model(
    State(_state): State<Arc<AdminState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let model = body["model"].as_str().unwrap_or("");
    if model.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Missing model name"}));
    }

    let client = reqwest::Client::new();
    let url = ollama_url();

    match client
        .delete(format!("{url}/api/delete"))
        .json(&serde_json::json!({"name": model}))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {
            Json(serde_json::json!({"ok": true, "message": format!("Model '{}' deleted", model)}))
        }
        Ok(r) => {
            let text = r.text().await.unwrap_or_default();
            Json(serde_json::json!({"ok": false, "error": text}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

// ═════════════════════════════════════════════════════════════
// TENANT CONFIG (KEY-VALUE SETTINGS) — DB as source of truth
// ═════════════════════════════════════════════════════════════

/// List all config entries for a tenant.
async fn list_tenant_configs(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.db.lock().unwrap().list_configs(&id) {
        Ok(configs) => {
            // Convert Vec<TenantConfig> to a flat JSON object
            let mut obj = serde_json::Map::new();
            for cfg in &configs {
                obj.insert(
                    cfg.key.clone(),
                    serde_json::Value::String(cfg.value.clone()),
                );
            }
            Json(serde_json::json!({"ok": true, "configs": obj}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// Set one or more config values for a tenant.
/// Body: {"configs": {"default_provider": "ollama", "default_model": "llama3.2", ...}}
async fn set_tenant_configs(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let configs = match body.get("configs").and_then(|c| c.as_object()) {
        Some(c) => c,
        None => return Json(serde_json::json!({"ok": false, "error": "Missing 'configs' object"})),
    };

    let db = state.db.lock().unwrap();
    let mut saved_count = 0;
    for (key, value) in configs {
        let val_str = match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        if db.set_config(&id, key, &val_str).is_ok() {
            saved_count += 1;
        }
    }

    // Also update the tenants table provider/model for consistency
    if let Some(provider) = configs.get("default_provider").and_then(|v| v.as_str()) {
        if let Some(model) = configs.get("default_model").and_then(|v| v.as_str()) {
            db.update_tenant_provider(&id, provider, model).ok();
        }
    }

    drop(db);
    state
        .db
        .lock()
        .unwrap()
        .log_event(
            "config_updated",
            "admin",
            &id,
            Some(&format!("keys={}", saved_count)),
        )
        .ok();

    Json(serde_json::json!({"ok": true, "saved": saved_count}))
}

// ═════════════════════════════════════════════════════════════
// TENANT AGENTS — DB-backed agent persistence
// ═════════════════════════════════════════════════════════════

/// List all agents for a tenant.
async fn list_tenant_agents(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.db.lock().unwrap().list_agents(&id) {
        Ok(agents) => Json(serde_json::json!({"ok": true, "agents": agents})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

#[derive(serde::Deserialize)]
struct UpsertAgentReq {
    name: String,
    role: Option<String>,
    description: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    system_prompt: Option<String>,
}

/// Create or update an agent for a tenant.
async fn upsert_tenant_agent(
    State(state): State<Arc<AdminState>>,
    Path(id): Path<String>,
    Json(req): Json<UpsertAgentReq>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap();

    // Get tenant defaults for fallback values
    let tenant = db.get_tenant(&id).ok();
    let default_provider = tenant
        .as_ref()
        .map(|t| t.provider.as_str())
        .unwrap_or("openai");
    let default_model = tenant
        .as_ref()
        .map(|t| t.model.as_str())
        .unwrap_or("gpt-4o-mini");

    match db.upsert_agent(
        &id,
        &req.name,
        req.role.as_deref().unwrap_or("assistant"),
        req.description.as_deref().unwrap_or(""),
        req.provider.as_deref().unwrap_or(default_provider),
        req.model.as_deref().unwrap_or(default_model),
        req.system_prompt.as_deref().unwrap_or(""),
    ) {
        Ok(agent) => {
            drop(db);
            state
                .db
                .lock()
                .unwrap()
                .log_event(
                    "agent_upserted",
                    "admin",
                    &id,
                    Some(&format!("name={}", req.name)),
                )
                .ok();
            Json(serde_json::json!({"ok": true, "agent": agent}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// Delete an agent by tenant_id + name.
async fn delete_tenant_agent(
    State(state): State<Arc<AdminState>>,
    Path((id, name)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    match state.db.lock().unwrap().delete_agent_by_name(&id, &name) {
        Ok(()) => {
            state
                .db
                .lock()
                .unwrap()
                .log_event(
                    "agent_deleted",
                    "admin",
                    &id,
                    Some(&format!("name={}", name)),
                )
                .ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}
