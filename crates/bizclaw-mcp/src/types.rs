//! MCP protocol types — JSON-RPC 2.0 messages and config.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Display name for this server.
    pub name: String,
    /// Command to start the MCP server process.
    pub command: String,
    /// Arguments to the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables to set.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Whether this server is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool { true }

/// Tool information discovered from an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: serde_json::Value,
    /// Which MCP server this tool belongs to.
    #[serde(skip)]
    pub server_name: String,
}

// ── JSON-RPC 2.0 types ────────────────────────────────

/// JSON-RPC 2.0 request.
#[derive(Debug, Serialize)]
pub(crate) struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: &str, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        }
    }
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Deserialize)]
pub(crate) struct JsonRpcResponse {
    #[allow(dead_code)]
    pub jsonrpc: String,
    #[allow(dead_code)]
    pub id: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error.
#[derive(Debug, Deserialize)]
pub(crate) struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

/// MCP tools/list response.
#[derive(Debug, Deserialize)]
pub(crate) struct ToolsListResult {
    pub tools: Vec<McpToolDef>,
}

/// MCP tool definition from the server.
#[derive(Debug, Deserialize)]
pub(crate) struct McpToolDef {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "inputSchema", default)]
    pub input_schema: Option<serde_json::Value>,
}

/// MCP tools/call result.
#[derive(Debug, Deserialize)]
pub(crate) struct ToolCallResult {
    pub content: Vec<ToolCallContent>,
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

/// MCP tool call content item.
#[derive(Debug, Deserialize)]
pub(crate) struct ToolCallContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(default)]
    pub text: Option<String>,
}
