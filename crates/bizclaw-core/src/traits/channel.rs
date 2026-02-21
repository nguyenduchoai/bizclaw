//! Communication Channel trait — swappable messaging interfaces.

use async_trait::async_trait;
use tokio_stream::Stream;

use crate::error::Result;
use crate::types::{IncomingMessage, OutgoingMessage};

/// Channel trait — every communication interface implements this.
#[async_trait]
pub trait Channel: Send + Sync {
    /// Channel identifier (e.g., "cli", "telegram", "zalo").
    fn name(&self) -> &str;

    /// Connect and authenticate with the channel.
    async fn connect(&mut self) -> Result<()>;

    /// Disconnect from the channel.
    async fn disconnect(&mut self) -> Result<()>;

    /// Check if the channel is connected.
    fn is_connected(&self) -> bool;

    /// Start listening for incoming messages.
    /// Returns a stream of incoming messages.
    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>>;

    /// Send a message to a thread.
    async fn send(&self, message: OutgoingMessage) -> Result<()>;

    /// Send a typing indicator.
    async fn send_typing(&self, thread_id: &str) -> Result<()> {
        let _ = thread_id;
        Ok(()) // Default no-op
    }
}
