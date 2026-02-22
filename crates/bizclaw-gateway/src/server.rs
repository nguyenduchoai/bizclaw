//! HTTP server implementation using Axum.

use axum::{Router, routing::{get, post}};
use axum::response::Html;
use bizclaw_core::config::{GatewayConfig, BizClawConfig};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Shared state for the gateway server.
#[derive(Clone)]
pub struct AppState {
    pub gateway_config: GatewayConfig,
    pub full_config: Arc<Mutex<BizClawConfig>>,
    pub config_path: PathBuf,
    pub start_time: std::time::Instant,
}

/// Serve the dashboard HTML page.
async fn dashboard_page() -> Html<&'static str> {
    Html(super::dashboard::dashboard_html())
}

/// Build the Axum router with all routes.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(dashboard_page))
        .route("/health", get(super::routes::health_check))
        .route("/api/v1/info", get(super::routes::system_info))
        .route("/api/v1/config", get(super::routes::get_config))
        .route("/api/v1/config/update", post(super::routes::update_config))
        .route("/api/v1/config/full", get(super::routes::get_full_config))
        .route("/api/v1/providers", get(super::routes::list_providers))
        .route("/api/v1/channels", get(super::routes::list_channels))
        .route("/api/v1/channels/update", post(super::routes::update_channel))
        .route("/api/v1/zalo/qr", post(super::routes::zalo_qr_code))
        .route("/ws", get(super::ws::ws_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state))
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

    let state = AppState {
        gateway_config: config.clone(),
        full_config: Arc::new(Mutex::new(full_config)),
        config_path,
        start_time: std::time::Instant::now(),
    };

    let app = build_router(state);
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🌐 Gateway server listening on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
