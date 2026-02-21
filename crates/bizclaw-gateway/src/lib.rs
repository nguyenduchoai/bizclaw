//! # BizClaw Gateway
//! HTTP/WebSocket gateway API.

pub mod server;
pub mod routes;

use bizclaw_core::config::GatewayConfig;

/// Start the gateway HTTP server.
pub async fn start_server(_config: &GatewayConfig) -> anyhow::Result<()> {
    tracing::info!("Gateway server starting (stub)...");
    // Phase 5: Axum server with routes
    Ok(())
}
