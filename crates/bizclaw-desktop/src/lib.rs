//! # BizClaw Desktop - Native Desktop App
//!
//! Wails-based desktop application for non-tech users.
//!
//! ## Features:
//! - Native GUI (no browser required)
//! - SQLite database (zero setup)
//! - Chat with agents (streaming, tools, media)
//! - Agent management (max 5), provider config
//! - Team tasks with Kanban board
//!
//! ## Tech Stack:
//! - **Wails v2** for native desktop framework
//! - **Preact** for UI components
//! - **SQLite** for local database

pub mod app;
pub mod window;

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DesktopApp {
    running: Arc<RwLock<bool>>,
    port: u16,
}

impl DesktopApp {
    pub fn new() -> Self {
        Self {
            running: Arc::new(RwLock::new(false)),
            port: 18789,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub async fn start(&self) -> Result<(), String> {
        let mut running = self.running.write().await;
        if *running {
            return Err("Desktop app already running".to_string());
        }
        *running = true;
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        let mut running = self.running.write().await;
        *running = false;
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

impl Default for DesktopApp {
    fn default() -> Self {
        Self::new()
    }
}
