//! # BizClaw MCP Client
//!
//! Model Context Protocol (MCP) client implementation.
//! Connects to external MCP servers via stdio (JSON-RPC 2.0)
//! and exposes their tools to the BizClaw Agent.
//!
//! ## Architecture
//! ```text
//! Agent → McpClient → spawn(command, args)
//!                     ↕ JSON-RPC 2.0 (stdio)
//!                     MCP Server (any language)
//! ```

pub mod client;
pub mod transport;
pub mod types;
pub mod bridge;

pub use client::McpClient;
pub use bridge::McpToolBridge;
pub use types::{McpServerConfig, McpToolInfo};
