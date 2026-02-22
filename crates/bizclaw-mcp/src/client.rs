//! MCP Client — connects to an MCP server, discovers tools, and calls them.

use crate::transport::StdioTransport;
use crate::types::*;

/// MCP Client — manages connection to a single MCP server.
pub struct McpClient {
    pub name: String,
    config: McpServerConfig,
    transport: Option<StdioTransport>,
    tools: Vec<McpToolInfo>,
    next_id: u64,
}

impl McpClient {
    /// Create a new MCP client from config (not yet connected).
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            name: config.name.clone(),
            config,
            transport: None,
            tools: vec![],
            next_id: 1,
        }
    }

    /// Connect to the MCP server — spawn process + initialize + discover tools.
    pub async fn connect(&mut self) -> Result<(), String> {
        if !self.config.enabled {
            return Err(format!("MCP server '{}' is disabled", self.name));
        }

        tracing::info!("🔗 Connecting to MCP server '{}'...", self.name);

        // Spawn the server process
        let transport = StdioTransport::spawn(
            &self.config.command,
            &self.config.args,
            &self.config.env,
        ).await?;
        self.transport = Some(transport);

        // Initialize the MCP session
        self.initialize().await?;

        // Discover available tools
        self.discover_tools().await?;

        tracing::info!("✅ MCP server '{}' connected — {} tools available",
            self.name, self.tools.len());

        Ok(())
    }

    /// Initialize the MCP session (handshake).
    async fn initialize(&mut self) -> Result<(), String> {
        let transport = self.transport.as_mut()
            .ok_or("Not connected")?;

        let req = JsonRpcRequest::new(self.next_id(), "initialize", Some(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "bizclaw",
                "version": env!("CARGO_PKG_VERSION")
            }
        })));

        let res = transport.request(&req).await?;

        if let Some(err) = res.error {
            return Err(format!("MCP initialize error: {} (code {})", err.message, err.code));
        }

        // Send initialized notification (no response expected, but send via request for simplicity)
        let notify = JsonRpcRequest::new(self.next_id(), "notifications/initialized", None);
        // For notification, we don't wait for response strictly, but send it
        let _ = transport.request(&notify).await;

        Ok(())
    }

    /// Discover tools from the MCP server.
    async fn discover_tools(&mut self) -> Result<(), String> {
        let transport = self.transport.as_mut()
            .ok_or("Not connected")?;

        let req = JsonRpcRequest::new(self.next_id(), "tools/list", None);
        let res = transport.request(&req).await?;

        if let Some(err) = res.error {
            return Err(format!("tools/list error: {} (code {})", err.message, err.code));
        }

        if let Some(result) = res.result {
            let tools_result: ToolsListResult = serde_json::from_value(result)
                .map_err(|e| format!("Parse tools error: {e}"))?;

            self.tools = tools_result.tools.into_iter().map(|t| McpToolInfo {
                name: t.name,
                description: t.description.unwrap_or_default(),
                input_schema: t.input_schema.unwrap_or(serde_json::json!({
                    "type": "object",
                    "properties": {}
                })),
                server_name: self.name.clone(),
            }).collect();
        }

        Ok(())
    }

    /// Call a tool on the MCP server.
    pub async fn call_tool(&mut self, tool_name: &str, arguments: serde_json::Value) -> Result<String, String> {
        let transport = self.transport.as_mut()
            .ok_or("MCP server not connected")?;

        let req = JsonRpcRequest::new(self.next_id(), "tools/call", Some(serde_json::json!({
            "name": tool_name,
            "arguments": arguments
        })));

        let res = transport.request(&req).await?;

        if let Some(err) = res.error {
            return Err(format!("Tool '{}' error: {} (code {})", tool_name, err.message, err.code));
        }

        if let Some(result) = res.result {
            let call_result: ToolCallResult = serde_json::from_value(result)
                .map_err(|e| format!("Parse tool result error: {e}"))?;

            if call_result.is_error {
                let text = call_result.content.iter()
                    .filter_map(|c| c.text.as_ref())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(format!("Tool '{}' returned error: {}", tool_name, text));
            }

            // Collect text content
            let output = call_result.content.iter()
                .filter_map(|c| {
                    if c.content_type == "text" { c.text.clone() } else { None }
                })
                .collect::<Vec<_>>()
                .join("\n");

            Ok(output)
        } else {
            Ok(String::new())
        }
    }

    /// Get discovered tools.
    pub fn tools(&self) -> &[McpToolInfo] {
        &self.tools
    }

    /// Check if connected and alive.
    pub fn is_connected(&mut self) -> bool {
        self.transport.as_mut().map_or(false, |t| t.is_alive())
    }

    /// Disconnect from the MCP server.
    pub async fn disconnect(&mut self) {
        if let Some(transport) = self.transport.as_mut() {
            transport.shutdown().await;
        }
        self.transport = None;
        self.tools.clear();
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}
