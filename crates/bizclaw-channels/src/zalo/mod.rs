//! Zalo channel module — Zalo Personal + OA (zca-js rewrite in Rust).

pub mod client;
pub mod personal;
pub mod official;

use async_trait::async_trait;
use bizclaw_core::config::ZaloChannelConfig;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Channel;
use bizclaw_core::types::{IncomingMessage, OutgoingMessage};
use tokio_stream::Stream;

/// Zalo channel implementation.
pub struct ZaloChannel {
    config: ZaloChannelConfig,
    connected: bool,
}

impl ZaloChannel {
    pub fn new(config: ZaloChannelConfig) -> Self {
        Self { config, connected: false }
    }
}

#[async_trait]
impl Channel for ZaloChannel {
    fn name(&self) -> &str { "zalo" }

    async fn connect(&mut self) -> Result<()> {
        tracing::info!("Zalo channel: connecting in {} mode...", self.config.mode);

        match self.config.mode.as_str() {
            "personal" => {
                tracing::warn!("⚠️  Zalo Personal API is unofficial. Use at your own risk.");
                // Phase 4A: implement cookie/QR login via client module
                self.connected = true;
                tracing::info!("Zalo Personal: connected (stub)");
            }
            "official" => {
                tracing::info!("Zalo OA: connecting via official API...");
                self.connected = true;
                tracing::info!("Zalo OA: connected (stub)");
            }
            _ => {
                return Err(bizclaw_core::error::BizClawError::Config(
                    format!("Unknown Zalo mode: {}", self.config.mode)
                ));
            }
        }
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        tracing::info!("Zalo channel: disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool { self.connected }

    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        // Phase 4A: WebSocket event listener
        let stream = async_stream::stream! {
            tracing::info!("Zalo listener: waiting for messages (stub)...");
            // Stub: no messages in initial build
            tokio::time::sleep(tokio::time::Duration::from_secs(86400)).await;
        };
        Ok(Box::new(Box::pin(stream)))
    }

    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        tracing::info!("Zalo: sending message to {} (stub)", message.thread_id);
        // Phase 4A: rate-limited message sending
        Ok(())
    }

    async fn send_typing(&self, thread_id: &str) -> Result<()> {
        tracing::debug!("Zalo: sending typing to {} (stub)", thread_id);
        Ok(())
    }
}
