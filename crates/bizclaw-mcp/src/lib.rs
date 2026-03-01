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

pub mod bridge;
pub mod client;
pub mod sse_http;
pub mod transport;
pub mod types;

pub use bridge::McpToolBridge;
pub use client::McpClient;
pub use sse_http::{HttpTransport, SseTransport};
pub use types::{McpServerConfig, McpToolInfo};
