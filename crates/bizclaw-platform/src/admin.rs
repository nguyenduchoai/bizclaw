//! Admin HTTP server — REST API for the admin control plane.
//!
//! Phase 2: Uses tokio::sync::Mutex for db/manager to eliminate
//! poison-on-panic risk from std::sync::Mutex.

use crate::db::PlatformDb;
use crate::tenant::TenantManager;
use axum::middleware;
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    routing::{delete, get, post, put},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use axum::extract::DefaultBodyLimit;

/// Shared application state for the admin server.
pub struct AdminState {
    pub db: Mutex<PlatformDb>,
    pub manager: Mutex<TenantManager>,
    pub jwt_secret: String,
    pub bizclaw_bin: String,
    pub base_port: u16,
    /// Domain name for this platform instance (e.g. "bizclaw.vn" or "viagent.vn")
    pub domain: String,
    /// Rate limiter: email → (attempt_count, first_attempt_time)
    pub login_attempts: std::sync::Mutex<std::collections::HashMap<String, (u32, std::time::Instant)>>,
    /// Rate limiter for registration: email → (attempt_count, first_attempt_time)
    pub register_attempts: std::sync::Mutex<std::collections::HashMap<String, (u32, std::time::Instant)>>,
    /// PostgreSQL DB for enterprise features (optional — only when DATABASE_URL is set).
    /// Falls back gracefully: enterprise endpoints return 503 if None.
    pub pg_db: Option<crate::db_pg::PgDb>,
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

    if let Some(token) = auth_header.strip_prefix("Bearer ")
        && let Ok(claims) = crate::auth::validate_token(token, &state.jwt_secret) {
            let mut req = req;
            req.extensions_mut().insert(claims);
            return next.run(req).await;
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
            .route("/api/admin/users", post(create_user_handler))
            .route("/api/admin/users/{id}", delete(delete_user_handler))
            .route("/api/admin/users/{id}/tenant", put(assign_tenant_handler))
            .route("/api/admin/users/{id}/password/reset", put(admin_reset_user_password))
            .route("/api/admin/users/{id}/status", put(update_user_status_handler))
            .route("/api/admin/users/{id}/role", put(update_user_role_handler))
            // Profile
            .route("/api/admin/users/me/password", put(crate::self_serve::change_password_handler))
            // ── ENTERPRISE: Multi-user RBAC per Tenant ─────────────────────
            .route("/api/admin/tenants/{id}/members", get(list_members))
            .route("/api/admin/tenants/{id}/members/invite", post(invite_member))
            .route("/api/admin/tenants/{id}/members/{uid}/role", put(update_member_role))
            .route("/api/admin/tenants/{id}/members/{uid}", delete(remove_member))
            // ── ENTERPRISE: Human Handoff ───────────────────────────────────
            .route("/api/admin/tenants/{id}/handoffs", get(list_handoffs))
            .route("/api/admin/tenants/{id}/handoffs/{hid}/claim", post(claim_handoff))
            .route("/api/admin/tenants/{id}/handoffs/{hid}/reply", post(reply_handoff))
            .route("/api/admin/tenants/{id}/handoffs/{hid}/resolve", post(resolve_handoff))
            .route("/api/admin/tenants/{id}/handoffs/{hid}/messages", get(list_handoff_messages))
            // ── ENTERPRISE: BI Analytics ────────────────────────────────────
            .route("/api/admin/tenants/{id}/analytics/summary", get(analytics_summary))
            .route("/api/admin/tenants/{id}/analytics/tokens", get(analytics_tokens))
            // ── ENTERPRISE: Budget Quota Control ───────────────────────────
            .route("/api/admin/tenants/{id}/quotas", get(list_quotas))
            .route("/api/admin/tenants/{id}/quotas/{resource}", put(set_quota))
            // ── MISSION CONTROL: Kanban Task Board ─────────────────────────
            .route("/api/admin/tasks", get(mc_list_tasks).post(mc_create_task))
            .route("/api/admin/tasks/board", get(mc_kanban_board))
            .route("/api/admin/tasks/pending-review", get(mc_pending_reviews))
            .route("/api/admin/tasks/{tid}", get(mc_get_task).put(mc_update_task).delete(mc_delete_task))
            .route("/api/admin/tasks/{tid}/comments", get(mc_list_comments).post(mc_add_comment))
            .route("/api/admin/tasks/{tid}/review", post(mc_submit_review))
            // ── MISSION CONTROL: Agent Session Monitor ─────────────────────
            .route("/api/admin/sessions", get(mc_list_sessions))
            .route("/api/admin/sessions/heartbeat", post(mc_heartbeat))
            .route("/api/admin/sessions/{key}/terminate", post(mc_terminate_session))
            // ── MISSION CONTROL: GitHub Issues Sync ────────────────────────
            .route("/api/admin/tenants/{id}/github-syncs", get(mc_list_github_syncs).post(mc_upsert_github_sync))
            .route("/api/admin/tenants/{id}/github-syncs/{repo}/trigger", post(mc_trigger_github_sync))
            // ── CLOUD: Remote Server Provisioner ────────────────────────────
            .route("/api/admin/servers", get(srv_list_servers).post(srv_provision))
            .route("/api/admin/servers/{sid}", get(srv_get_server).delete(srv_delete_server))
            .route("/api/admin/servers/{sid}/health", get(srv_health_check))
            .route("/api/admin/servers/{sid}/command", post(srv_execute_command))
            // ── MISSION CONTROL: Webhooks ───────────────────────────────────
            .route("/api/admin/tenants/{id}/webhooks", get(mc_list_webhooks))
            .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));


        // Public routes — including invitation acceptance
        let public = Router::new()
            .route("/api/admin/login", post(login))
            .route("/api/admin/pairing/validate", post(validate_pairing))
            .route("/api/admin/register", post(crate::self_serve::register_handler))
            .route("/api/admin/password-reset", post(crate::self_serve::forgot_password_handler))
            .route("/api/admin/password-reset/confirm", post(crate::self_serve::reset_password_handler))
            .route("/api/admin/invitations/{token}/accept", post(accept_invitation))
            .route("/pixel-office", get(pixel_office_page))
            .route("/", get(admin_dashboard_page));


        // SPA fallback — serve dashboard HTML for all non-API paths
        // so that /tenants, /settings, /ollama etc. all work
        let spa_fallback = Router::new().fallback(get(admin_dashboard_page));

        // CORS — configurable via BIZCLAW_CORS_ORIGINS env var
        // H4 FIX: Default to localhost-only CORS (not Any) for security
        let cors_methods = [axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE, axum::http::Method::OPTIONS];
        let cors = match std::env::var("BIZCLAW_CORS_ORIGINS") {
            Ok(origins) if !origins.is_empty() => {
                let allowed: Vec<_> = origins.split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                CorsLayer::new()
                    .allow_origin(allowed)
                    .allow_methods(cors_methods)
                    .allow_headers(Any)
            }
            _ => {
                // Dev mode: allow Any; Production: restrict to same-origin
                if std::env::var("BIZCLAW_BIND_ALL").unwrap_or_default() == "1" {
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(cors_methods)
                        .allow_headers(Any)
                } else {
                    // Production: only allow requests from same origin (no CORS header = same-origin only)
                    CorsLayer::new()
                        .allow_methods(cors_methods)
                        .allow_headers(Any)
                }
            }
        };

        protected
            .merge(public)
            .merge(spa_fallback)
            .layer(axum::middleware::from_fn(platform_security_headers))
            .layer(cors)
            .layer(DefaultBodyLimit::max(1_048_576)) // 1MB max request body
            .with_state(state)
    }

    /// Start the admin server.
    pub async fn start(state: Arc<AdminState>, port: u16) -> bizclaw_core::error::Result<()> {
        let app = Self::router(state);
        // Bind to 127.0.0.1 — only accessible via reverse proxy (Nginx)
        // Set BIZCLAW_BIND_ALL=1 to allow direct external access (dev only)
        let bind_addr = if std::env::var("BIZCLAW_BIND_ALL").unwrap_or_default() == "1" {
            [0, 0, 0, 0]
        } else {
            [127, 0, 0, 1]
        };
        let addr = std::net::SocketAddr::from((bind_addr, port));
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

// ── Security Headers (C1 FIX) ──────────────────────────

/// Security headers middleware — HSTS, CSP, X-Frame-Options, X-Content-Type-Options.
/// Matches Gateway security headers for parity.
async fn platform_security_headers(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("X-Frame-Options", "SAMEORIGIN".parse().unwrap());
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    headers.insert("Referrer-Policy", "strict-origin-when-cross-origin".parse().unwrap());
    // HSTS — only when behind reverse proxy (production)
    if std::env::var("BIZCLAW_BIND_ALL").unwrap_or_default() != "1" {
        headers.insert("Strict-Transport-Security", "max-age=31536000; includeSubDomains".parse().unwrap());
    }
    response
}

// ── Error Sanitization (H2 FIX) ────────────────────────

/// Return a sanitized error response — log the real error server-side,
/// send a generic message to the client. Prevents information disclosure.
fn internal_error(context: &str, e: impl std::fmt::Display) -> Json<serde_json::Value> {
    tracing::error!("[{context}] {e}");
    Json(serde_json::json!({"ok": false, "error": "An internal error occurred"}))
}

// ── Nginx Sync ─────────────────────────────────────


/// Regenerate /etc/nginx/conf.d/{domain}-tenants.conf from the DB
/// and reload nginx so new/removed tenants are routed correctly.
/// Runs in a background thread to avoid blocking the HTTP response.
///
/// ## Environment Variables
/// - `NGINX_SSL_CERT_DIR`: Override SSL cert directory name under `/etc/letsencrypt/live/`.
///   Default: uses the domain name. Example: `NGINX_SSL_CERT_DIR=bizclaw.vn-0001`
/// - `NGINX_CONTAINER_NAME`: Docker container name for nginx. Default: `bizclaw-nginx`
/// - `BIZCLAW_BIND_ALL`: If `1`, upstream connects to Docker service hostname; else `127.0.0.1`
async fn sync_nginx_routing(state: &AdminState) {
    let domain = state.domain.clone();
    let tenants = match state.db.lock().await.list_tenants() {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("nginx-sync[{domain}]: failed to list tenants: {e}");
            return;
        }
    };

    // Spawn background thread so HTTP response is not blocked
    std::thread::spawn(move || {
        // Use domain prefix for map variable names to avoid conflicts between domains
        let domain_slug = domain.replace('.', "_");
        let mut map_entries = String::new();

        // Reserved subdomains that should NOT be treated as tenant slugs
        let reserved_subdomains = ["apps", "www", "api", "admin", "mail", "smtp", "imap", "ftp"];

        for t in &tenants {
            // M5 FIX: Validate slug contains only safe chars before injecting into nginx config
            let safe_slug: String = t.slug.chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            if safe_slug.is_empty() || safe_slug != t.slug {
                tracing::warn!("nginx-sync[{domain}]: skipping tenant '{}' — slug contains unsafe chars", t.slug);
                continue;
            }
            // Skip reserved subdomains
            if reserved_subdomains.contains(&safe_slug.as_str()) {
                tracing::warn!("nginx-sync[{domain}]: skipping tenant '{}' — reserved subdomain", t.slug);
                continue;
            }
            map_entries.push_str(&format!("    {}      {};\n", safe_slug, t.port));
        }

        // Escape dots in domain for nginx regex
        let domain_regex = domain.replace('.', "\\.");

        // Docker mode: use container hostname; Non-Docker: use 127.0.0.1
        let upstream_host = if std::env::var("BIZCLAW_BIND_ALL").unwrap_or_default() == "1" {
            // In Docker, nginx connects to the app container via network hostname
            // The hostname matches the docker-compose service name
            "bizclaw".to_string()
        } else {
            "127.0.0.1".to_string()
        };

        // SSL cert directory: configurable via NGINX_SSL_CERT_DIR env var
        // Allows overriding when certbot creates directories like "bizclaw.vn-0001"
        let ssl_cert_dir = std::env::var("NGINX_SSL_CERT_DIR")
            .unwrap_or_else(|_| domain.clone());

        let conf = format!(
            r#"# {domain} Dynamic Tenant Routing (auto-generated)
# Generated at: {timestamp}
# Tenants: {tenant_count}

map $subdomain_{domain_slug} $tenant_port_{domain_slug} {{
    default   0;
{map_entries}}}

server {{
    listen 80;
    listen 443 ssl http2;
    server_name ~^(?<subdomain_{domain_slug}>[^.]+)\.{domain_regex}$;

    ssl_certificate /etc/letsencrypt/live/{ssl_cert_dir}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/{ssl_cert_dir}/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;

    # Reserved subdomains — let static server blocks handle them
    if ($subdomain_{domain_slug} = "apps") {{
        return 444;
    }}
    if ($subdomain_{domain_slug} = "www") {{
        return 444;
    }}

    if ($tenant_port_{domain_slug} = 0) {{
        return 404;
    }}

    location / {{
        proxy_pass http://{upstream_host}:$tenant_port_{domain_slug};
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 86400;
    }}
}}
"#,
            timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            tenant_count = tenants.len(),
        );

        // Try multiple paths: shared Docker volume first, then traditional path
        let conf_paths = [
            format!("/etc/nginx/dynamic/{domain_slug}-tenants.conf"),
            format!("/etc/nginx/conf.d/{domain_slug}-tenants.conf"),
        ];

        let mut written = false;
        for conf_path in &conf_paths {
            // Ensure directory exists
            if let Some(parent) = std::path::Path::new(conf_path).parent() {
                std::fs::create_dir_all(parent).ok();
            }
            if std::fs::write(conf_path, &conf).is_ok() {
                tracing::info!("nginx-sync[{domain}]: wrote config to {conf_path}");
                written = true;
                break;
            }
        }

        if !written {
            tracing::warn!("nginx-sync[{domain}]: failed to write to any nginx config path");
            return;
        }

        // Try to reload nginx — multiple strategies for Docker vs bare-metal
        // Strategy 0: docker exec (when bizclaw runs in a separate container from nginx)
        let nginx_container = std::env::var("NGINX_CONTAINER_NAME")
            .unwrap_or_else(|_| "bizclaw-nginx".to_string());
        if let Ok(out) = std::process::Command::new("docker")
            .args(["exec", &nginx_container, "nginx", "-s", "reload"])
            .output()
            && out.status.success() {
                tracing::info!("nginx-sync[{domain}]: {} tenants synced, nginx reloaded (docker exec {})", tenants.len(), nginx_container);
                return;
            }
        // Strategy 1: nginx -s reload (works if nginx is in same container)
        if let Ok(out) = std::process::Command::new("nginx").args(["-s", "reload"]).output()
            && out.status.success() {
                tracing::info!("nginx-sync[{domain}]: {} tenants synced, nginx reloaded (nginx -s)", tenants.len());
                return;
            }
        // Strategy 2: systemctl reload nginx (bare-metal)
        if let Ok(out) = std::process::Command::new("systemctl").args(["reload", "nginx"]).output()
            && out.status.success() {
                tracing::info!("nginx-sync[{domain}]: {} tenants synced, nginx reloaded (systemctl)", tenants.len());
                return;
            }
        // Strategy 3: send HUP to nginx master process (Docker — if pid is available)
        if let Ok(out) = std::process::Command::new("sh").args(["-c", "kill -HUP $(cat /var/run/nginx.pid 2>/dev/null) 2>/dev/null"]).output()
            && out.status.success() {
                tracing::info!("nginx-sync[{domain}]: {} tenants synced, nginx reloaded (HUP)", tenants.len());
                return;
            }
        // If none worked, config is written but nginx needs manual reload
        tracing::warn!("nginx-sync[{domain}]: config written ({} tenants) but nginx reload failed — may need manual reload", tenants.len());
    });
}

// ── RBAC Helpers ──────────────────────────────────────────

/// Check if claims represent the super-admin (platform owner).
fn is_super_admin(claims: &crate::auth::Claims) -> bool {
    claims.email == "admin@bizclaw.vn" || claims.role == "superadmin"
}

/// Check if a user can ACCESS (view) a specific tenant.
/// - superadmin: any tenant
/// - admin: only tenants where owner_id == claims.sub
/// - viewer: only the tenant assigned via JWT tenant_id
fn can_access_tenant(claims: &crate::auth::Claims, tenant_id: &str, db: &crate::db::PlatformDb) -> bool {
    if is_super_admin(claims) {
        return true;
    }
    // Admin can access tenants they own
    if claims.role == "admin" {
        if let Ok(tenant) = db.get_tenant(tenant_id) {
            return tenant.owner_id.as_deref() == Some(&claims.sub);
        }
        return false;
    }
    // Viewer can access their assigned tenant
    if claims.role == "viewer" {
        return claims.tenant_id.as_deref() == Some(tenant_id);
    }
    false
}

/// Check if a user can WRITE (create/edit/delete/start/stop) a tenant.
/// - superadmin: any tenant
/// - admin: only tenants where owner_id == claims.sub
/// - viewer: CANNOT write
fn can_write_tenant(claims: &crate::auth::Claims, tenant_id: &str, db: &crate::db::PlatformDb) -> bool {
    if claims.role == "viewer" {
        return false;
    }
    can_access_tenant(claims, tenant_id, db)
}

// ── API Handlers ────────────────────────────────────

async fn get_stats(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
) -> Json<serde_json::Value> {
    if is_super_admin(&claims) {
        let (total, running, stopped, error) = state
            .db
            .lock().await
            .tenant_stats()
            .unwrap_or((0, 0, 0, 0));
        let users = state
            .db
            .lock().await
            .list_users()
            .map(|u| u.len() as u32)
            .unwrap_or(0);
        Json(serde_json::json!({
            "total_tenants": total, "running": running, "stopped": stopped,
            "error": error, "users": users
        }))
    } else {
        // Non-super-admin: only count their own tenants
        let my_tenants = state.db.lock().await
            .list_tenants_by_owner(&claims.sub)
            .unwrap_or_default();
        let running = my_tenants.iter().filter(|t| t.status == "running").count() as u32;
        let stopped = my_tenants.iter().filter(|t| t.status == "stopped").count() as u32;
        let error = my_tenants.iter().filter(|t| t.status == "error").count() as u32;
        Json(serde_json::json!({
            "total_tenants": my_tenants.len() as u32, "running": running, "stopped": stopped,
            "error": error, "users": 1
        }))
    }
}

async fn get_activity(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
) -> Json<serde_json::Value> {
    let events = state
        .db
        .lock().await
        .recent_events(20)
        .unwrap_or_default();
    // For non-super-admin, filter to only their events
    if is_super_admin(&claims) {
        Json(serde_json::json!({ "events": events }))
    } else {
        let filtered: Vec<_> = events.into_iter()
            .filter(|e| e.actor_id == claims.sub)
            .collect();
        Json(serde_json::json!({ "events": filtered }))
    }
}

async fn list_tenants(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
) -> Json<serde_json::Value> {
    if is_super_admin(&claims) {
        // Superadmin sees ALL tenants
        let tenants = state.db.lock().await.list_tenants().unwrap_or_default();
        Json(serde_json::json!({ "tenants": tenants }))
    } else if claims.role == "admin" {
        // Admin sees only their own tenants (owner_id match)
        let tenants = state.db.lock().await
            .list_tenants_by_owner(&claims.sub)
            .unwrap_or_default();
        Json(serde_json::json!({ "tenants": tenants }))
    } else {
        // Viewer sees only the single tenant assigned to them
        let db = state.db.lock().await;
        if let Some(tid) = &claims.tenant_id {
            match db.get_tenant(tid) {
                Ok(tenant) => Json(serde_json::json!({ "tenants": [tenant] })),
                Err(_) => Json(serde_json::json!({ "tenants": [] })),
            }
        } else {
            Json(serde_json::json!({ "tenants": [] }))
        }
    }
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
    Extension(claims): Extension<crate::auth::Claims>,
    Json(req): Json<CreateTenantReq>,
) -> Json<serde_json::Value> {
    // Role check: viewer cannot create tenants
    if claims.role == "viewer" {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền tạo tenant. Liên hệ admin để nâng cấp role."}));
    }

    // Sanitize slug: only ASCII alphanumeric + hyphens allowed
    let clean_slug = crate::self_serve::generate_safe_slug(&req.slug);
    let slug = if clean_slug.is_empty() { 
        crate::self_serve::generate_safe_slug(&req.name) 
    } else { 
        clean_slug 
    };

    let port = {
        let db = state.db.lock().await;
        let used_ports = db.used_ports().unwrap_or_default();
        let mut port = state.base_port;
        while used_ports.contains(&port) {
            port += 1;
        }
        port
    };

    // Owner is the logged-in user (unless super-admin creates for someone else)
    let owner_id = claims.sub.clone();

    // IMPORTANT: separate lock scopes to avoid Mutex deadlock
    let create_result = state.db.lock().await.create_tenant(
        &req.name,
        &slug,
        port,
        req.provider.as_deref().unwrap_or("openai"),
        req.model.as_deref().unwrap_or("gpt-4o-mini"),
        req.plan.as_deref().unwrap_or("free"),
        Some(&owner_id),
    );
    match create_result {
        Ok(tenant) => {
            state
                .db
                .lock().await
                .log_event(
                    "tenant_created",
                    "admin",
                    &tenant.id,
                    Some(&format!("slug={}", slug)),
                )
                .ok();

            // Auto-start the tenant so subdomain works immediately
            {
                let mut mgr = state.manager.lock().await;
                let db = state.db.lock().await;
                match mgr.start_tenant(&tenant, &state.bizclaw_bin, &db) {
                    Ok(pid) => {
                        drop(db);
                        state.db.lock().await
                            .update_tenant_status(&tenant.id, "running", Some(pid)).ok();
                        tracing::info!("auto-start: tenant '{}' started on port {} (pid={})", slug, port, pid);
                    }
                    Err(e) => {
                        drop(db);
                        state.db.lock().await
                            .update_tenant_status(&tenant.id, "error", None).ok();
                        tracing::warn!("auto-start: failed to start tenant '{}': {e}", slug);
                    }
                }
            }

            sync_nginx_routing(&state).await;
            Json(serde_json::json!({"ok": true, "tenant": tenant}))
        }
        Err(e) => internal_error("admin", e),
    }
}

async fn get_tenant(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().await;
    if !can_access_tenant(&claims, &id, &db) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền truy cập tenant này."}));
    }
    match db.get_tenant(&id) {
        Ok(t) => Json(serde_json::json!({"ok": true, "tenant": t})),
        Err(e) => internal_error("admin", e),
    }
}

async fn delete_tenant(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // RBAC: only superadmin or owner-admin can delete
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền xóa tenant này."}));
    }
    state.manager.lock().await.stop_tenant(&id).ok();
    // IMPORTANT: separate lock scopes to avoid Mutex deadlock.
    // delete_tenant lock must be dropped before log_event acquires it again.
    let delete_result = state.db.lock().await.delete_tenant(&id);
    match delete_result {
        Ok(()) => {
            state
                .db
                .lock().await
                .log_event("tenant_deleted", "admin", &id, None)
                .ok();
            sync_nginx_routing(&state).await;
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

async fn start_tenant(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền khởi động tenant này."}));
    }
    let tenant = match state.db.lock().await.get_tenant(&id) {
        Ok(t) => t,
        Err(e) => return internal_error("admin", e),
    };

    let mut mgr = state.manager.lock().await;
    let db = state.db.lock().await;
    match mgr.start_tenant(&tenant, &state.bizclaw_bin, &db) {
        Ok(pid) => {
            drop(db);
            state
                .db
                .lock().await
                .update_tenant_status(&id, "running", Some(pid))
                .ok();
            state
                .db
                .lock().await
                .log_event("tenant_started", "admin", &id, None)
                .ok();
            sync_nginx_routing(&state).await;
            Json(serde_json::json!({"ok": true, "pid": pid}))
        }
        Err(e) => {
            drop(db);
            state
                .db
                .lock().await
                .update_tenant_status(&id, "error", None)
                .ok();
            internal_error("start_tenant", e)
        }
    }
}

async fn stop_tenant(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền dừng tenant này."}));
    }
    state.manager.lock().await.stop_tenant(&id).ok();
    state
        .db
        .lock().await
        .update_tenant_status(&id, "stopped", None)
        .ok();
    state
        .db
        .lock().await
        .log_event("tenant_stopped", "admin", &id, None)
        .ok();
    sync_nginx_routing(&state).await;
    Json(serde_json::json!({"ok": true}))
}

async fn restart_tenant(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền khởi động lại tenant này."}));
    }
    let tenant = match state.db.lock().await.get_tenant(&id) {
        Ok(t) => t,
        Err(e) => return internal_error("admin", e),
    };

    // IMPORTANT: separate lock scopes to avoid Mutex deadlock
    let restart_result = {
        let mut mgr = state.manager.lock().await;
        let db = state.db.lock().await;
        mgr.restart_tenant(&tenant, &state.bizclaw_bin, &db)
    }; // Both locks dropped here
    match restart_result {
        Ok(pid) => {
            state.db.lock().await
                .update_tenant_status(&id, "running", Some(pid)).ok();
            state.db.lock().await
                .log_event("tenant_restarted", "admin", &id, None).ok();
            sync_nginx_routing(&state).await;
            Json(serde_json::json!({"ok": true, "pid": pid}))
        }
        Err(e) => {
            state.db.lock().await
                .update_tenant_status(&id, "error", None).ok();
            internal_error("restart_tenant", e)
        }
    }
}

async fn reset_pairing(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền reset pairing code."}));
    }
    // IMPORTANT: separate lock scopes to avoid Mutex deadlock
    let reset_result = state.db.lock().await.reset_pairing_code(&id);
    match reset_result {
        Ok(code) => {
            state
                .db
                .lock().await
                .log_event("tenant_pairing_reset", "admin", &id, None)
                .ok();
            Json(serde_json::json!({"ok": true, "pairing_code": code}))
        }
        Err(e) => internal_error("admin", e),
    }
}

async fn list_users(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
) -> Json<serde_json::Value> {
    if is_super_admin(&claims) {
        let users = state.db.lock().await.list_users().unwrap_or_default();
        Json(serde_json::json!({"users": users}))
    } else {
        // Non-super-admin can only see themselves
        let all_users = state.db.lock().await.list_users().unwrap_or_default();
        let my_users: Vec<_> = all_users.into_iter()
            .filter(|u| u.id == claims.sub)
            .collect();
        Json(serde_json::json!({"users": my_users}))
    }
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

    tracing::debug!("login: querying user {}", req.email);
    let user = state.db.lock().await.get_user_by_email(&req.email);
    match user {
        Ok(Some((id, hash, role))) => {
            // Run bcrypt in blocking thread to avoid stalling the async runtime
            let password = req.password.clone();
            let hash_clone = hash.clone();
            tracing::debug!("login: verifying password");
            let ok = tokio::task::spawn_blocking(move || {
                crate::auth::verify_password(&password, &hash_clone)
            })
            .await
            .unwrap_or(false);

            if ok {
                tracing::debug!("login: password verified, generating token");
                // Get tenant_id and status for JWT — direct query instead of list_users
                let (tenant_id, user_status) = {
                    let db = state.db.lock().await;
                    match db.get_user_by_id(&id) {
                        Ok(Some(u)) => {
                            let tid = if u.status == "pending" { None } else { u.tenant_id.clone() };
                            (tid, u.status.clone())
                        }
                        _ => (None, "active".into()),
                    }
                };
                if user_status == "pending" {
                    return Json(serde_json::json!({
                        "ok": false,
                        "error": "Tài khoản đang chờ duyệt. Vui lòng liên hệ admin để kích hoạt."
                    }));
                }
                if user_status == "suspended" {
                    return Json(serde_json::json!({
                        "ok": false,
                        "error": "Tài khoản đã bị tạm khóa. Vui lòng liên hệ admin."
                    }));
                }
                match crate::auth::create_token(&id, &req.email, &role, tenant_id.as_deref(), &state.jwt_secret) {
                    Ok(token) => {
                        state
                            .db
                            .lock().await
                            .log_event("login_success", "user", &id, None)
                            .ok();
                        Json(serde_json::json!({"ok": true, "token": token, "role": role}))
                    }
                    Err(e) => {
                        tracing::error!("login: Token error: {e}");
                        Json(serde_json::json!({"ok": false, "error": e}))
                    }
                }
            } else {
                tracing::warn!("login: Invalid credentials for {}", req.email);
                Json(serde_json::json!({"ok": false, "error": "Invalid credentials"}))
            }
        }
        Ok(None) => Json(serde_json::json!({"ok": false, "error": "Invalid credentials"})),
        Err(e) => {
            tracing::error!("login: DB error: {e}");
            Json(serde_json::json!({"ok": false, "error": "An internal error occurred. Please try again."}))
        }
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
    // IMPORTANT: separate lock scopes to avoid Mutex deadlock
    let pairing_result = state.db.lock().await.validate_pairing(&req.slug, &req.code);
    match pairing_result {
        Ok(Some(tenant)) => {
            // Generate a session token for this tenant
            match crate::auth::create_token(&tenant.id, &tenant.slug, "tenant", Some(&tenant.id), &state.jwt_secret) {
                Ok(token) => {
                    state
                        .db
                        .lock().await
                        .log_event("pairing_success", "tenant", &tenant.id, None)
                        .ok();
                    Json(serde_json::json!({"ok": true, "token": token, "tenant": tenant}))
                }
                Err(e) => Json(serde_json::json!({"ok": false, "error": e})),
            }
        }
        Ok(None) => Json(serde_json::json!({"ok": false, "error": "Invalid pairing code"})),
        Err(e) => internal_error("admin", e),
    }
}

async fn admin_dashboard_page() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("admin_dashboard.html"))
}

async fn pixel_office_page() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("../../../data/pixel_office.html"))
}

// ── Channel Configuration Handlers ────────────────────────────────────

async fn list_channels(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // IMPORTANT: separate lock scopes — can_access_tenant lock must be dropped
    // before list_channels acquires its own lock, otherwise Mutex deadlock.
    {
        let db = state.db.lock().await;
        if !can_access_tenant(&claims, &id, &db) {
            return Json(serde_json::json!({"ok": false, "error": "Không có quyền truy cập tenant này."}));
        }
    } // lock dropped here
    match state.db.lock().await.list_channels(&id) {
        Ok(channels) => Json(serde_json::json!({"ok": true, "channels": channels})),
        Err(e) => internal_error("admin", e),
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
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpsertChannelReq>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền cấu hình tenant này."}));
    }
    let config_json = serde_json::to_string(&req.config).unwrap_or_default();
    // IMPORTANT: separate lock scopes to avoid Mutex deadlock
    let upsert_result = state.db.lock().await
        .upsert_channel(&id, &req.channel_type, req.enabled, &config_json);
    match upsert_result {
        Ok(channel) => {
            state
                .db
                .lock().await
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
        Err(e) => internal_error("admin", e),
    }
}

async fn delete_channel(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((tenant_id, channel_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &tenant_id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền xóa channel."}));
    }
    // IMPORTANT: separate lock scopes to avoid Mutex deadlock
    let del_result = state.db.lock().await.delete_channel(&channel_id);
    match del_result {
        Ok(()) => {
            state
                .db
                .lock().await
                .log_event(
                    "channel_deleted",
                    "admin",
                    &tenant_id,
                    Some(&format!("channel_id={}", channel_id)),
                )
                .ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
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
        Err(e) => {
            tracing::error!("[zalo_qr] {e}");
            Json(serde_json::json!({
                "ok": false,
                "error": "Không thể tạo mã QR Zalo",
                "fallback": "Vui lòng vào chat.zalo.me → F12 → Application → Cookies → Copy toàn bộ và paste vào ô Cookie bên dưới"
            }))
        }
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
        Err(e) => {
            tracing::error!("[ollama_health] {e}");
            Json(serde_json::json!({
                "ok": false, "url": url,
                "status": "not_running",
                "error": "Ollama is not reachable",
                "install_guide": "curl -fsSL https://ollama.ai/install.sh | sh"
            }))
        }
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
        Err(e) => {
            tracing::error!("[ollama_pull] {e}");
            Json(serde_json::json!({
                "ok": false,
                "error": "Ollama is not reachable",
                "hint": "Ollama might not be installed. Run: curl -fsSL https://ollama.ai/install.sh | sh"
            }))
        }
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
        Err(e) => internal_error("admin", e),
    }
}

// ═════════════════════════════════════════════════════════════
// TENANT CONFIG (KEY-VALUE SETTINGS) — DB as source of truth
// ═════════════════════════════════════════════════════════════

/// List all config entries for a tenant.
async fn list_tenant_configs(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // IMPORTANT: separate lock scopes — can_access_tenant lock must be dropped
    // before list_configs acquires its own lock, otherwise Mutex deadlock.
    {
        let db = state.db.lock().await;
        if !can_access_tenant(&claims, &id, &db) {
            return Json(serde_json::json!({"ok": false, "error": "Không có quyền truy cập tenant này."}));
        }
    } // lock dropped here
    match state.db.lock().await.list_configs(&id) {
        Ok(configs) => {
            let mut obj = serde_json::Map::new();
            for cfg in &configs {
                obj.insert(cfg.key.clone(), serde_json::Value::String(cfg.value.clone()));
            }
            Json(serde_json::json!({"ok": true, "configs": obj}))
        }
        Err(e) => internal_error("admin", e),
    }
}

/// Set one or more config values for a tenant.
/// Body: {"configs": {"default_provider": "ollama", "default_model": "llama3.2", ...}}
async fn set_tenant_configs(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền cấu hình tenant này."}));
    }
    let configs = match body.get("configs").and_then(|c| c.as_object()) {
        Some(c) => c,
        None => return Json(serde_json::json!({"ok": false, "error": "Missing 'configs' object"})),
    };

    let db = state.db.lock().await;
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
    if let Some(provider) = configs.get("default_provider").and_then(|v| v.as_str())
        && let Some(model) = configs.get("default_model").and_then(|v| v.as_str()) {
            db.update_tenant_provider(&id, provider, model).ok();
        }

    drop(db);
    state.db.lock().await.log_event(
        "config_updated",
        "admin",
        &id,
        Some(&format!("keys={}", saved_count)),
    ).ok();

    Json(serde_json::json!({"ok": true, "saved": saved_count}))
}

// ═════════════════════════════════════════════════════════════
// TENANT AGENTS — DB-backed agent persistence
// ═════════════════════════════════════════════════════════════

/// List all agents for a tenant.
async fn list_tenant_agents(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // IMPORTANT: separate lock scopes — can_access_tenant lock must be dropped
    // before list_agents acquires its own lock, otherwise Mutex deadlock.
    {
        let db = state.db.lock().await;
        if !can_access_tenant(&claims, &id, &db) {
            return Json(serde_json::json!({"ok": false, "error": "Không có quyền truy cập tenant này."}));
        }
    } // lock dropped here
    match state.db.lock().await.list_agents(&id) {
        Ok(agents) => Json(serde_json::json!({"ok": true, "agents": agents})),
        Err(e) => internal_error("admin", e),
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
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpsertAgentReq>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền cấu hình tenant này."}));
    }
    let db = state.db.lock().await;

    // Get tenant defaults for fallback values
    let tenant = db.get_tenant(&id).ok();
    let default_provider = tenant.as_ref().map(|t| t.provider.as_str()).unwrap_or("openai");
    let default_model = tenant.as_ref().map(|t| t.model.as_str()).unwrap_or("gpt-4o-mini");

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
            state.db.lock().await.log_event(
                "agent_upserted",
                "admin",
                &id,
                Some(&format!("name={}", req.name)),
            ).ok();
            Json(serde_json::json!({"ok": true, "agent": agent}))
        }
        Err(e) => internal_error("admin", e),
    }
}

/// Delete an agent by tenant_id + name.
async fn delete_tenant_agent(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((id, name)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền xóa agent."}));
    }
    // IMPORTANT: separate lock scopes to avoid Mutex deadlock
    let del_result = state.db.lock().await.delete_agent_by_name(&id, &name);
    match del_result {
        Ok(()) => {
            state.db.lock().await.log_event(
                "agent_deleted",
                "admin",
                &id,
                Some(&format!("name={}", name)),
            ).ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

// ═════════════════════════════════════════════════════════════
// USER MANAGEMENT HANDLERS
// ═════════════════════════════════════════════════════════════

#[derive(serde::Deserialize)]
struct CreateUserReq {
    email: String,
    password: String,
    role: Option<String>,
    tenant_id: Option<String>,
}

async fn create_user_handler(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Json(req): Json<CreateUserReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({"ok": false, "error": "Chỉ Super Admin mới có quyền tạo user."}));
    }
    tracing::debug!("create_user: {}", req.email);
    if req.email.is_empty() || req.password.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Email and password are required"}));
    }

    // Hash password in blocking thread
    let password = req.password.clone();
    tracing::debug!("create_user: hashing password");
    let hash = match tokio::task::spawn_blocking(move || {
        crate::auth::hash_password(&password)
    })
    .await
    {
        Ok(Ok(h)) => h,
        Ok(Err(e)) => {
            tracing::error!("create_user: Hash error: {e}");
            return Json(serde_json::json!({"ok": false, "error": "Failed to process password"}));
        }
        Err(e) => {
            tracing::error!("create_user: Spawn error: {e}");
            return Json(serde_json::json!({"ok": false, "error": "Internal error"}));
        }
    };

    let role = req.role.as_deref().unwrap_or("admin");
    // Extracted lock to avoid deadlock with subsequent log_event lock
    let db_res = state.db.lock().await.create_user(&req.email, &hash, role, req.tenant_id.as_deref().filter(|s| !s.is_empty()));
    
    match db_res {
        Ok(id) => {
            state
                .db
                .lock().await
                .log_event("user_created", "admin", &id, Some(&format!("email={}", req.email)))
                .ok();
            Json(serde_json::json!({"ok": true, "id": id}))
        }
        Err(e) => {
            tracing::error!("create_user_handler: Error creating user: {e}");
            // Check for duplicate email
            let msg = if e.to_string().contains("UNIQUE") {
                "Email already exists"
            } else {
                "Failed to create user"
            };
            Json(serde_json::json!({"ok": false, "error": msg}))
        }
    }
}

async fn delete_user_handler(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // Only super-admin can delete users
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({"ok": false, "error": "Only super admin can delete users"}));
    }
    
    tracing::info!("delete_user_handler: Cascade deleting user {} and their tenants", id);
    
    // Stop all tenant processes first
    let tenant_ids = state.db.lock().await
        .list_tenants_by_owner(&id)
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.id)
        .collect::<Vec<_>>();
    for tid in &tenant_ids {
        state.manager.lock().await.stop_tenant(tid).ok();
    }
    
    // Cascade delete user + tenants
    let db_res = state.db.lock().await.delete_user_cascade(&id);
    
    match db_res {
        Ok(deleted_tenants) => {
            tracing::info!("delete_user_handler: Deleted user {} and {} tenants", id, deleted_tenants.len());
            state
                .db
                .lock().await
                .log_event("user_deleted_cascade", "admin", &id, Some(&format!("tenants_deleted={}", deleted_tenants.len())))
                .ok();
            if !deleted_tenants.is_empty() {
                sync_nginx_routing(&state).await;
            }
            Json(serde_json::json!({"ok": true, "tenants_deleted": deleted_tenants.len()}))
        }
        Err(e) => {
            internal_error("delete_user", e)
        }
    }
}

#[derive(serde::Deserialize)]
struct AssignTenantReq {
    tenant_id: Option<String>,
}

/// Assign or remove tenant for a user
async fn assign_tenant_handler(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<AssignTenantReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({"ok": false, "error": "Chỉ Super Admin mới có quyền gán tenant."}));
    }
    let tenant_id_str = req.tenant_id.as_deref().filter(|s| !s.is_empty());
    let db_res = state.db.lock().await.update_user_tenant(&id, tenant_id_str);
    
    match db_res {
        Ok(()) => {
            let details = format!("tenant_id={}", tenant_id_str.unwrap_or("none"));
            state
                .db
                .lock().await
                .log_event("user_assigned", "admin", &id, Some(&details))
                .ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

// ═════════════════════════════════════════════════════════════
// ADMIN RESET USER PASSWORD (Super Admin only)
// ═════════════════════════════════════════════════════════════

#[derive(serde::Deserialize)]
struct AdminResetPasswordReq {
    new_password: String,
}

/// Super Admin force-resets a user's password (no old password required).
async fn admin_reset_user_password(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<AdminResetPasswordReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({"ok": false, "error": "Chỉ Super Admin mới có quyền reset mật khẩu."}));
    }
    if req.new_password.len() < 8 {
        return Json(serde_json::json!({"ok": false, "error": "Password must be at least 8 characters"}));
    }

    let new_pwd = req.new_password.clone();
    let hash = match tokio::task::spawn_blocking(move || crate::auth::hash_password(&new_pwd)).await {
        Ok(Ok(h)) => h,
        Ok(Err(e)) => return internal_error("hash_password", e),
        Err(e) => return internal_error("hash_task", e),
    };

    let db_res = state.db.lock().await.update_user_password(&id, &hash);
    match db_res {
        Ok(()) => {
            state
                .db
                .lock().await
                .log_event("admin_password_reset", "admin", &id, Some("password force-reset by admin"))
                .ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

// ═════════════════════════════════════════════════════════════
// USER STATUS MANAGEMENT (Super Admin only)
// ═════════════════════════════════════════════════════════════

#[derive(serde::Deserialize)]
struct UpdateUserStatusReq {
    status: String, // pending, active, suspended
}

/// Super Admin approves/suspends a user account.
async fn update_user_status_handler(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserStatusReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({"ok": false, "error": "Only super admin can change user status"}));
    }

    let valid_statuses = ["pending", "active", "suspended"];
    if !valid_statuses.contains(&req.status.as_str()) {
        return Json(serde_json::json!({"ok": false, "error": "Invalid status. Must be: pending, active, suspended"}));
    }

    let db_res = state.db.lock().await.update_user_status(&id, &req.status);
    match db_res {
        Ok(()) => {
            state
                .db
                .lock().await
                .log_event("user_status_changed", "admin", &id, Some(&format!("status={}", req.status)))
                .ok();
            
            // If activating a user, try to start their stopped tenant(s)
            if req.status == "active" {
                let tenants = state.db.lock().await
                    .list_tenants_by_owner(&id)
                    .unwrap_or_default();
                let stopped: Vec<_> = tenants.iter().filter(|t| t.status == "stopped").cloned().collect();
                for tenant in &stopped {
                    // Acquire locks one at a time — never hold db + manager simultaneously
                    let start_result = {
                        let mut mgr = state.manager.lock().await;
                        let db_ref = state.db.lock().await;
                        mgr.start_tenant(tenant, &state.bizclaw_bin, &db_ref)
                    }; // Both locks dropped here
                    match start_result {
                        Ok(pid) => {
                            state.db.lock().await
                                .update_tenant_status(&tenant.id, "running", Some(pid)).ok();
                            tracing::info!("user-activate: started tenant '{}' (pid={})", tenant.slug, pid);
                        }
                        Err(e) => {
                            state.db.lock().await
                                .update_tenant_status(&tenant.id, "error", None).ok();
                            tracing::warn!("user-activate: failed to start tenant '{}': {e}", tenant.slug);
                        }
                    }
                }
                if !tenants.is_empty() {
                    sync_nginx_routing(&state).await;
                }
            }
            
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

// ═════════════════════════════════════════════════════════════
// USER ROLE MANAGEMENT (Super Admin only)
// ═════════════════════════════════════════════════════════════

#[derive(serde::Deserialize)]
struct UpdateUserRoleReq {
    role: String, // superadmin, admin, viewer
}

/// Super Admin changes a user's role.
async fn update_user_role_handler(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRoleReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({"ok": false, "error": "Chỉ Super Admin mới có quyền đổi role."}));
    }

    let valid_roles = ["superadmin", "admin", "viewer"];
    if !valid_roles.contains(&req.role.as_str()) {
        return Json(serde_json::json!({"ok": false, "error": "Role không hợp lệ. Phải là: superadmin, admin, viewer"}));
    }

    // Protect the platform owner account
    {
        let db = state.db.lock().await;
        let users = db.list_users().unwrap_or_default();
        if let Some(target) = users.iter().find(|u| u.id == id)
            && target.email == "admin@bizclaw.vn" {
                return Json(serde_json::json!({"ok": false, "error": "Không thể thay đổi role của Super Admin gốc."}));
            }
    }

    let db_res = state.db.lock().await.update_user_role(&id, &req.role);
    match db_res {
        Ok(()) => {
            state.db.lock().await
                .log_event("user_role_changed", "admin", &id, Some(&format!("role={}", req.role)))
                .ok();
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => internal_error("admin", e),
    }
}

// ════════════════════════════════════════════════════════════════════
// ENTERPRISE HANDLERS: Multi-user RBAC
// ════════════════════════════════════════════════════════════════════

async fn list_members(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền truy cập."}));
    }
    match &state.pg_db {
        Some(pg) => match pg.list_tenant_members(&id).await {
            Ok(members) => Json(serde_json::json!({"ok": true, "members": members})),
            Err(e) => internal_error("list_members", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required for enterprise features"})),
    }
}

#[derive(serde::Deserialize)]
struct InviteMemberReq { email: String, role: String }

async fn invite_member(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<InviteMemberReq>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền mời thành viên."}));
    }
    let valid_roles = ["owner", "admin", "operator", "viewer"];
    if !valid_roles.contains(&req.role.as_str()) {
        return Json(serde_json::json!({"ok": false, "error": "Role không hợp lệ. Phải là: owner, admin, operator, viewer"}));
    }
    match &state.pg_db {
        Some(pg) => match pg.create_invitation(&id, &req.email, &req.role, &claims.sub, 72).await {
            Ok(inv) => {
                let base_url = std::env::var("BIZCLAW_BASE_URL").unwrap_or_else(|_| "https://app.bizclaw.vn".into());
                let invite_url = format!("{base_url}/api/admin/invitations/{}/accept", inv.token);
                Json(serde_json::json!({
                    "ok": true,
                    "invitation_token": inv.token,
                    "invitation_url": invite_url,
                    "expires_in_hours": 72,
                }))
            }
            Err(e) => internal_error("invite_member", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

#[derive(serde::Deserialize)]
struct UpdateMemberRoleReq { role: String }

async fn update_member_role(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((id, uid)): Path<(String, String)>,
    Json(req): Json<UpdateMemberRoleReq>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    match &state.pg_db {
        Some(pg) => match pg.add_tenant_member(&id, &uid, &req.role, None).await {
            Ok(()) => Json(serde_json::json!({"ok": true})),
            Err(e) => internal_error("update_member_role", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

async fn remove_member(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((id, uid)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    match &state.pg_db {
        Some(pg) => match pg.remove_tenant_member(&id, &uid).await {
            Ok(()) => Json(serde_json::json!({"ok": true})),
            Err(e) => internal_error("remove_member", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

#[derive(serde::Deserialize)]
struct AcceptInvitationReq { user_id: String }

async fn accept_invitation(
    State(state): State<Arc<AdminState>>,
    Path(token): Path<String>,
    Json(req): Json<AcceptInvitationReq>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.accept_invitation(&token, &req.user_id).await {
            Ok((tenant_id, role)) => Json(serde_json::json!({
                "ok": true, "tenant_id": tenant_id, "role": role,
            })),
            Err(e) => internal_error("accept_invitation", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

// ════════════════════════════════════════════════════════════════════
// ENTERPRISE HANDLERS: Human Handoff
// ════════════════════════════════════════════════════════════════════

async fn list_handoffs(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    let status = params.get("status").map(|s| s.as_str());
    match &state.pg_db {
        Some(pg) => match pg.list_handoffs(&id, status).await {
            Ok(handoffs) => Json(serde_json::json!({"ok": true, "handoffs": handoffs})),
            Err(e) => internal_error("list_handoffs", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

async fn claim_handoff(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((_id, hid)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.claim_handoff(&hid, &claims.sub).await {
            Ok(()) => Json(serde_json::json!({"ok": true, "message": "Bạn đã nhận case này"})),
            Err(e) => internal_error("claim_handoff", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

#[derive(serde::Deserialize)]
struct HandoffReplyReq { content: String }

async fn reply_handoff(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((_id, hid)): Path<(String, String)>,
    Json(req): Json<HandoffReplyReq>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.add_handoff_message(&hid, "operator", Some(&claims.sub), &req.content).await {
            Ok(msg) => Json(serde_json::json!({"ok": true, "message": msg})),
            Err(e) => internal_error("reply_handoff", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

async fn resolve_handoff(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((_id, hid)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.resolve_handoff(&hid, &claims.sub).await {
            Ok(()) => Json(serde_json::json!({"ok": true, "message": "Case đã được đóng"})),
            Err(e) => internal_error("resolve_handoff", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

async fn list_handoff_messages(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((id, hid)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    match &state.pg_db {
        Some(pg) => match pg.list_handoff_messages(&hid).await {
            Ok(msgs) => Json(serde_json::json!({"ok": true, "messages": msgs})),
            Err(e) => internal_error("list_handoff_messages", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

// ════════════════════════════════════════════════════════════════════
// ENTERPRISE HANDLERS: BI Analytics
// ════════════════════════════════════════════════════════════════════

async fn analytics_summary(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    let days: i32 = params.get("days").and_then(|d| d.parse().ok()).unwrap_or(30);
    match &state.pg_db {
        Some(pg) => match pg.get_analytics_summary(&id, days).await {
            Ok(summary) => Json(serde_json::json!({"ok": true, "period_days": days, "summary": summary})),
            Err(e) => internal_error("analytics_summary", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

async fn analytics_tokens(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    let days: i32 = params.get("days").and_then(|d| d.parse().ok()).unwrap_or(30);
    match &state.pg_db {
        Some(pg) => match pg.get_token_usage_by_day(&id, days).await {
            Ok(usage) => Json(serde_json::json!({"ok": true, "period_days": days, "usage": usage})),
            Err(e) => internal_error("analytics_tokens", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

// ════════════════════════════════════════════════════════════════════
// ENTERPRISE HANDLERS: Budget Quota
// ════════════════════════════════════════════════════════════════════

async fn list_quotas(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Không có quyền."}));
    }
    match &state.pg_db {
        Some(pg) => match pg.get_quota_status(&id).await {
            Ok(quotas) => Json(serde_json::json!({"ok": true, "quotas": quotas})),
            Err(e) => internal_error("list_quotas", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

#[derive(serde::Deserialize)]
struct SetQuotaReq { limit_value: i64 }

async fn set_quota(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((id, resource)): Path<(String, String)>,
    Json(req): Json<SetQuotaReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) && !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({"ok": false, "error": "Chỉ Admin mới có thể thay đổi quota."}));
    }
    let valid = ["tokens_per_month", "messages_per_day", "handoffs_per_month"];
    if !valid.contains(&resource.as_str()) {
        return Json(serde_json::json!({"ok": false, "error": "Resource không hợp lệ."}));
    }
    match &state.pg_db {
        Some(pg) => match pg.set_quota(&id, &resource, req.limit_value).await {
            Ok(()) => Json(serde_json::json!({"ok": true, "resource": resource, "limit": req.limit_value})),
            Err(e) => internal_error("set_quota", e),
        },
        None => Json(serde_json::json!({"ok": false, "error": "PostgreSQL required"})),
    }
}

// ════════════════════════════════════════════════════════════════════
// MISSION CONTROL HANDLERS: Kanban Task Board
// ════════════════════════════════════════════════════════════════════

async fn mc_list_tasks(
    State(state): State<Arc<AdminState>>,
    Extension(_claims): Extension<crate::auth::Claims>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => {
            let tenant_id = params.get("tenant_id").map(|s| s.as_str());
            let status    = params.get("status").map(|s| s.as_str());
            let priority  = params.get("priority").map(|s| s.as_str());
            let assigned  = params.get("assigned_to").map(|s| s.as_str());
            let limit: i64 = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(100);
            match pg.list_tasks(tenant_id, status, priority, assigned, limit).await {
                Ok(tasks) => Json(serde_json::json!({ "ok": true, "tasks": tasks })),
                Err(e)    => internal_error("mc_list_tasks", e),
            }
        }
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

#[derive(serde::Deserialize)]
struct McCreateTaskReq {
    title:          String,
    description:    Option<String>,
    status:         Option<String>,
    priority:       Option<String>,
    assigned_to:    Option<String>,
    assigned_agent: Option<String>,
    tags:           Option<String>,
    due_at:         Option<String>,
    quality_gate:   Option<bool>,
    tenant_id:      Option<String>,
}

async fn mc_create_task(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Json(req): Json<McCreateTaskReq>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => {
            let create_req = crate::mission_control::CreateTaskReq {
                title: req.title, description: req.description,
                status: req.status, priority: req.priority,
                assigned_to: req.assigned_to, assigned_agent: req.assigned_agent,
                tags: req.tags, due_at: req.due_at,
                quality_gate: req.quality_gate, tenant_id: req.tenant_id,
            };
            match pg.create_task(&create_req, &claims.sub).await {
                Ok(task) => Json(serde_json::json!({ "ok": true, "task": task })),
                Err(e)   => internal_error("mc_create_task", e),
            }
        }
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_kanban_board(
    State(state): State<Arc<AdminState>>,
    Extension(_claims): Extension<crate::auth::Claims>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => {
            let tenant_id = params.get("tenant_id").map(|s| s.as_str());
            match pg.get_kanban_board(tenant_id).await {
                Ok(board) => Json(serde_json::json!({ "ok": true, "board": board })),
                Err(e)    => internal_error("mc_kanban_board", e),
            }
        }
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_pending_reviews(
    State(state): State<Arc<AdminState>>,
    Extension(_claims): Extension<crate::auth::Claims>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => {
            let tenant_id = params.get("tenant_id").map(|s| s.as_str());
            match pg.get_pending_reviews(tenant_id).await {
                Ok(tasks) => Json(serde_json::json!({ "ok": true, "tasks": tasks, "count": tasks.len() })),
                Err(e)    => internal_error("mc_pending_reviews", e),
            }
        }
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_get_task(
    State(state): State<Arc<AdminState>>,
    Path(tid): Path<String>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.get_task(&tid).await {
            Ok(task) => Json(serde_json::json!({ "ok": true, "task": task })),
            Err(e)   => internal_error("mc_get_task", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

#[derive(serde::Deserialize)]
struct McUpdateTaskReq {
    title:          Option<String>,
    description:    Option<String>,
    status:         Option<String>,
    priority:       Option<String>,
    assigned_to:    Option<String>,
    assigned_agent: Option<String>,
    position:       Option<i32>,
}

async fn mc_update_task(
    State(state): State<Arc<AdminState>>,
    Path(tid): Path<String>,
    Json(req): Json<McUpdateTaskReq>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.update_task(
            &tid,
            req.title.as_deref(), req.description.as_deref(),
            req.status.as_deref(), req.priority.as_deref(),
            req.assigned_to.as_deref(), req.assigned_agent.as_deref(),
            req.position,
        ).await {
            Ok(task) => Json(serde_json::json!({ "ok": true, "task": task })),
            Err(e)   => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_delete_task(
    State(state): State<Arc<AdminState>>,
    Path(tid): Path<String>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.delete_task(&tid).await {
            Ok(()) => Json(serde_json::json!({ "ok": true })),
            Err(e) => internal_error("mc_delete_task", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_list_comments(
    State(state): State<Arc<AdminState>>,
    Path(tid): Path<String>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.list_task_comments(&tid).await {
            Ok(comments) => Json(serde_json::json!({ "ok": true, "comments": comments })),
            Err(e)       => internal_error("mc_list_comments", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

#[derive(serde::Deserialize)]
struct McCommentReq { content: String }

async fn mc_add_comment(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(tid): Path<String>,
    Json(req): Json<McCommentReq>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.add_task_comment(&tid, Some(&claims.sub), &claims.email, &req.content).await {
            Ok(c) => Json(serde_json::json!({ "ok": true, "comment": c })),
            Err(e) => internal_error("mc_add_comment", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

#[derive(serde::Deserialize)]
struct McReviewReq {
    status: String,     // approved | rejected
    notes:  Option<String>,
}

async fn mc_submit_review(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(tid): Path<String>,
    Json(req): Json<McReviewReq>,
) -> Json<serde_json::Value> {
    let valid = ["approved", "rejected"];
    if !valid.contains(&req.status.as_str()) {
        return Json(serde_json::json!({ "ok": false, "error": "status phải là: approved | rejected" }));
    }
    match &state.pg_db {
        Some(pg) => match pg.submit_quality_review(
            &tid, Some(&claims.sub), &claims.email, &req.status, req.notes.as_deref()
        ).await {
            Ok(review) => Json(serde_json::json!({ "ok": true, "review": review })),
            Err(e)     => internal_error("mc_submit_review", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

// ════════════════════════════════════════════════════════════════════
// MISSION CONTROL HANDLERS: Agent Session Monitor
// ════════════════════════════════════════════════════════════════════

async fn mc_list_sessions(
    State(state): State<Arc<AdminState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => {
            let tenant_id = params.get("tenant_id").map(|s| s.as_str());
            match pg.list_agent_sessions(tenant_id).await {
                Ok(sessions) => Json(serde_json::json!({ "ok": true, "sessions": sessions })),
                Err(e)       => internal_error("mc_list_sessions", e),
            }
        }
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

#[derive(serde::Deserialize)]
struct McHeartbeatReq {
    agent_name:         String,
    session_key:        String,
    tenant_id:          Option<String>,
    prompt_tokens:      Option<i64>,
    completion_tokens:  Option<i64>,
    cost_usd:           Option<f32>,
    model:              Option<String>,
}

async fn mc_heartbeat(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<McHeartbeatReq>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.upsert_agent_session(
            req.tenant_id.as_deref(), &req.agent_name, &req.session_key,
            req.prompt_tokens.unwrap_or(0),
            req.completion_tokens.unwrap_or(0),
            req.cost_usd.unwrap_or(0.0),
            req.model.as_deref(),
        ).await {
            Ok(session) => Json(serde_json::json!({ "ok": true, "session": session })),
            Err(e)      => internal_error("mc_heartbeat", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_terminate_session(
    State(state): State<Arc<AdminState>>,
    Path(key): Path<String>,
) -> Json<serde_json::Value> {
    match &state.pg_db {
        Some(pg) => match pg.terminate_session(&key).await {
            Ok(()) => Json(serde_json::json!({ "ok": true })),
            Err(e) => internal_error("mc_terminate_session", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

// ════════════════════════════════════════════════════════════════════
// MISSION CONTROL HANDLERS: GitHub Sync
// ════════════════════════════════════════════════════════════════════

async fn mc_list_github_syncs(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({ "ok": false, "error": "Không có quyền." }));
    }
    match &state.pg_db {
        Some(pg) => match pg.list_github_syncs(&id).await {
            Ok(syncs) => Json(serde_json::json!({ "ok": true, "syncs": syncs })),
            Err(e)    => internal_error("mc_list_github_syncs", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

#[derive(serde::Deserialize)]
struct McGithubSyncReq {
    repo:          String,
    access_token:  Option<String>,
    label_filter:  Option<String>,
    auto_assign:   Option<String>,
}

async fn mc_upsert_github_sync(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
    Json(req): Json<McGithubSyncReq>,
) -> Json<serde_json::Value> {
    if !can_write_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({ "ok": false, "error": "Không có quyền." }));
    }
    match &state.pg_db {
        Some(pg) => match pg.upsert_github_sync(
            &id, &req.repo, req.access_token.as_deref(),
            req.label_filter.as_deref().unwrap_or(""),
            req.auto_assign.as_deref().unwrap_or(""),
        ).await {
            Ok(sync) => Json(serde_json::json!({ "ok": true, "sync": sync })),
            Err(e)   => internal_error("mc_upsert_github_sync", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn mc_trigger_github_sync(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path((id, repo)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({ "ok": false, "error": "Không có quyền." }));
    }
    // repo path param may be URL-encoded "owner/repo" → decode
    let repo_decoded = repo.replace("%2F", "/");
    match &state.pg_db {
        Some(pg) => match pg.sync_github_issues(&id, &repo_decoded).await {
            Ok(count) => Json(serde_json::json!({
                "ok": true,
                "synced": count,
                "message": format!("Đã sync {count} GitHub issues vào Task Board")
            })),
            Err(e) => internal_error("mc_trigger_github_sync", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

// ════════════════════════════════════════════════════════════════════
// MISSION CONTROL HANDLERS: Webhooks
// ════════════════════════════════════════════════════════════════════

async fn mc_list_webhooks(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if !can_access_tenant(&claims, &id, &*state.db.lock().await) {
        return Json(serde_json::json!({ "ok": false, "error": "Không có quyền." }));
    }
    match &state.pg_db {
        Some(pg) => match pg.list_webhooks(&id).await {
            Ok(webhooks) => Json(serde_json::json!({ "ok": true, "webhooks": webhooks })),
            Err(e)       => internal_error("mc_list_webhooks", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

// ════════════════════════════════════════════════════════════════
// CLOUD: Remote Server Provisioner Handlers
// ════════════════════════════════════════════════════════════════

async fn srv_list_servers(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({ "ok": false, "error": "Superadmin required" }));
    }
    match &state.pg_db {
        Some(pg) => match pg.list_servers().await {
            Ok(servers) => Json(serde_json::json!({ "ok": true, "servers": servers })),
            Err(e) => internal_error("srv_list_servers", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn srv_provision(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Json(req): Json<crate::server_provisioner::ProvisionRequest>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({ "ok": false, "error": "Superadmin required" }));
    }
    let pg = match &state.pg_db {
        Some(pg) => pg,
        None => return Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    };

    // Register server in DB
    let port = req.port.unwrap_or(3001);
    match pg.register_server(&req.name, &req.ip, req.domain.as_deref(), port).await {
        Ok(server) => {
            let server_id = server.id.clone();
            let pg_clone = pg.clone();

            // Provision asynchronously
            tokio::spawn(async move {
                match crate::server_provisioner::provision_server(&req).await {
                    Ok(result) => {
                        let status = if result.success { "online" } else { "error" };
                        let _ = pg_clone.update_server_health(
                            &server_id, status, None, 0, None, None, None,
                        ).await;
                        tracing::info!("Provision completed for {}: {}", server_id, result.message);
                    }
                    Err(e) => {
                        let _ = pg_clone.update_server_health(
                            &server_id, "error", None, 0, None, None, None,
                        ).await;
                        tracing::error!("Provision failed for {}: {}", server_id, e);
                    }
                }
            });

            Json(serde_json::json!({
                "ok": true,
                "server": server,
                "message": "Provisioning started — check status in a few minutes",
            }))
        }
        Err(e) => internal_error("srv_provision", e),
    }
}

async fn srv_get_server(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(sid): Path<String>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({ "ok": false, "error": "Superadmin required" }));
    }
    match &state.pg_db {
        Some(pg) => match pg.list_servers().await {
            Ok(servers) => match servers.into_iter().find(|s| s.id == sid) {
                Some(server) => Json(serde_json::json!({ "ok": true, "server": server })),
                None => Json(serde_json::json!({ "ok": false, "error": "Server not found" })),
            },
            Err(e) => internal_error("srv_get_server", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn srv_delete_server(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(sid): Path<String>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({ "ok": false, "error": "Superadmin required" }));
    }
    match &state.pg_db {
        Some(pg) => match pg.delete_server(&sid).await {
            Ok(_) => Json(serde_json::json!({ "ok": true, "deleted": true })),
            Err(e) => internal_error("srv_delete_server", e),
        },
        None => Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    }
}

async fn srv_health_check(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(sid): Path<String>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({ "ok": false, "error": "Superadmin required" }));
    }
    let pg = match &state.pg_db {
        Some(pg) => pg,
        None => return Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    };

    let servers = pg.list_servers().await.unwrap_or_default();
    match servers.into_iter().find(|s| s.id == sid) {
        Some(server) => {
            let health = crate::server_provisioner::health_check(&server.ip, server.port).await;
            let status = if health.online { "online" } else { "offline" };
            let _ = pg.update_server_health(
                &sid, status, health.version.as_deref(), health.tenants,
                health.cpu, health.ram, health.disk,
            ).await;
            Json(serde_json::json!({ "ok": true, "health": health }))
        }
        None => Json(serde_json::json!({ "ok": false, "error": "Server not found" })),
    }
}

#[derive(serde::Deserialize)]
struct SrvCommandReq {
    action: String,
}

async fn srv_execute_command(
    State(state): State<Arc<AdminState>>,
    Extension(claims): Extension<crate::auth::Claims>,
    Path(sid): Path<String>,
    Json(cmd): Json<SrvCommandReq>,
) -> Json<serde_json::Value> {
    if !is_super_admin(&claims) {
        return Json(serde_json::json!({ "ok": false, "error": "Superadmin required" }));
    }
    let pg = match &state.pg_db {
        Some(pg) => pg,
        None => return Json(serde_json::json!({ "ok": false, "error": "PostgreSQL required" })),
    };

    let servers = pg.list_servers().await.unwrap_or_default();
    match servers.into_iter().find(|s| s.id == sid) {
        Some(server) => {
            let remote_cmd = match cmd.action.as_str() {
                "restart" => "systemctl restart bizclaw",
                "stop" => "systemctl stop bizclaw",
                "start" => "systemctl start bizclaw",
                "status" => "systemctl status bizclaw --no-pager",
                "update" => "cd /opt/bizclaw && git pull origin master && cargo build --release && systemctl restart bizclaw",
                "logs" => "journalctl -u bizclaw -n 100 --no-pager",
                "version" => "/opt/bizclaw/bizclaw-platform --version 2>/dev/null || echo 'unknown'",
                _ => return Json(serde_json::json!({
                    "ok": false,
                    "error": format!("Unknown action: {}. Use: restart, stop, start, status, update, logs, version", cmd.action),
                })),
            };

            match crate::server_provisioner::remote_exec(&server.ip, remote_cmd).await {
                Ok(output) => Json(serde_json::json!({ "ok": true, "output": output })),
                Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
            }
        }
        None => Json(serde_json::json!({ "ok": false, "error": "Server not found" })),
    }
}

