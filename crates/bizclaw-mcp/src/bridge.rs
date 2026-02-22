//! MCP Tool Bridge — adapts MCP tools to the BizClaw Tool trait.
//!
//! This module connects MCP server tools to the Agent's ToolRegistry,
//! making external tools appear as native BizClaw tools.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};

use crate::client::McpClient;
use crate::types::McpToolInfo;

/// Bridge a single MCP tool to the BizClaw Tool trait.
/// Each MCP tool becomes one McpToolBridge instance.
pub struct McpToolBridge {
    info: McpToolInfo,
    client: Arc<Mutex<McpClient>>,
}

impl McpToolBridge {
    /// Create a new bridge for an MCP tool.
    pub fn new(info: McpToolInfo, client: Arc<Mutex<McpClient>>) -> Self {
        Self { info, client }
    }

    /// Create bridges for all tools from an MCP client.
    pub fn from_client(client: Arc<Mutex<McpClient>>, tools: &[McpToolInfo]) -> Vec<Box<dyn Tool>> {
        tools.iter().map(|tool| {
            Box::new(McpToolBridge::new(tool.clone(), client.clone())) as Box<dyn Tool>
        }).collect()
    }
}

#[async_trait]
impl Tool for McpToolBridge {
    fn name(&self) -> &str {
        &self.info.name
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.info.name.clone(),
            description: format!(
                "[MCP:{}] {}",
                self.info.server_name,
                self.info.description
            ),
            parameters: self.info.input_schema.clone(),
        }
    }

    async fn execute(&self, arguments: &str) -> Result<ToolResult> {
        // Parse arguments from JSON string
        let args: serde_json::Value = serde_json::from_str(arguments)
            .unwrap_or(serde_json::json!({}));

        // Call the MCP tool
        let mut client = self.client.lock().await;
        match client.call_tool(&self.info.name, args).await {
            Ok(output) => Ok(ToolResult {
                tool_call_id: String::new(),
                output,
                success: true,
            }),
            Err(e) => Ok(ToolResult {
                tool_call_id: String::new(),
                output: format!("MCP tool error: {e}"),
                success: false,
            }),
        }
    }
}

/// Connect all configured MCP servers and return tool bridges.
pub async fn connect_mcp_servers(
    configs: &[crate::types::McpServerConfig],
) -> Vec<(Arc<Mutex<McpClient>>, Vec<Box<dyn Tool>>)> {
    let mut results = Vec::new();

    for config in configs {
        if !config.enabled {
            tracing::debug!("⏭️ MCP server '{}' disabled, skipping", config.name);
            continue;
        }

        let mut client = McpClient::new(config.clone());
        match client.connect().await {
            Ok(()) => {
                let tools = client.tools().to_vec();
                let client_arc = Arc::new(Mutex::new(client));
                let bridges = McpToolBridge::from_client(client_arc.clone(), &tools);
                tracing::info!("🔗 MCP '{}': {} tools registered", config.name, bridges.len());
                results.push((client_arc, bridges));
            }
            Err(e) => {
                tracing::warn!("⚠️ MCP server '{}' failed to connect: {}", config.name, e);
            }
        }
    }

    results
}
