//! CLI channel — interactive terminal.

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Channel;
use bizclaw_core::types::{IncomingMessage, OutgoingMessage, ThreadType};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::Stream;

pub struct CliChannel {
    connected: bool,
}

impl CliChannel {
    pub fn new() -> Self {
        Self { connected: false }
    }
}

impl Default for CliChannel {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Channel for CliChannel {
    fn name(&self) -> &str { "cli" }

    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool { self.connected }

    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        let stream = async_stream::stream! {
            let stdin = tokio::io::stdin();
            let reader = BufReader::new(stdin);
            let mut lines = reader.lines();

            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() { continue; }
                        if line.trim() == "/quit" || line.trim() == "/exit" { break; }
                        yield IncomingMessage {
                            channel: "cli".into(),
                            thread_id: "cli-main".into(),
                            sender_id: "user".into(),
                            sender_name: Some("User".into()),
                            content: line.trim().to_string(),
                            thread_type: ThreadType::Direct,
                            timestamp: chrono::Utc::now(),
                            reply_to: None,
                        };
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        };
        Ok(Box::new(Box::pin(stream)))
    }

    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        println!("\n🤖 {}\n", message.content);
        Ok(())
    }
}
