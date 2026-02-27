//! # BizClaw Gateway
//! HTTP/WebSocket gateway API with embedded web dashboard.

pub mod dashboard;
pub mod db;
pub mod openai_compat;
pub mod routes;
pub mod server;
pub mod ws;

use bizclaw_core::config::GatewayConfig;

/// Start the gateway HTTP server.
pub async fn start_server(config: &GatewayConfig) -> anyhow::Result<()> {
    server::start(config).await
}
